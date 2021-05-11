use convert_case::Case;
use proc_macro::TokenStream as StdTokenStream;
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::{parse_macro_input, LitStr};

use crate::input::{ArgumentInput, CommandExpandInput, CommandInput};
use crate::utils::ConvertCase;

mod input;
mod utils;

macro_rules! parent_name {
    ($prefix:ident, $depth:expr, $input:ident) => {
        if $depth == 0 {
            Ident::new(&$prefix.clone().unwrap_or("Args".to_owned()), Span::call_site())
        } else if let Some(prefix) = &$prefix {
            let ident = $input.command_name();
            Ident::new(&format!("{}{}", prefix, ident.to_case(Case::Pascal)), ident.span())
        } else {
            $input.command_name().to_case(Case::Pascal)
        }
    };
}

macro_rules! arg_opt {
    ($arg:ident, $($field:ident).+, $def:expr) => {
        if let Some(opts) = &$arg.opts { opts.$($field).+ } else { $def }
    };
    ($arg:ident, $($field:ident).+) => {
        arg_opt!($arg, $($field).+, None)
    };
}

macro_rules! ident {
    ($s:expr) => {
        Ident::new($s, Span::call_site())
    };
    (MERGE $lhs:expr, $rhs:expr) => {
        Ident::new(&format!("{}{}", $lhs, $rhs), Span::call_site())
    };
}

#[proc_macro]
pub fn expand_command_here(tokens: StdTokenStream) -> StdTokenStream {
    let input = parse_macro_input!(tokens as CommandExpandInput);
    let prefix = input.prefix.map(|ident| ident.to_string());

    let command = input.command;
    let structs = generate_structs(prefix.clone(), 0, &command);
    if structs.is_empty() {
        return StdTokenStream::new();
    }

    let main_struct = parent_name!(prefix, 0, command);
    let parser = generate_parser(prefix.clone(), 0, &command, &ident!("ctx"), &ident!("args"));

    (quote! {
        #(#structs)*

        #[::polecen::async_trait]
        impl ::polecen::command::CommandArguments for #main_struct {
            async fn read_arguments<'a, I>(
                mut args: I,
                ctx: ::polecen::arguments::parse::ArgumentParseContext<'a>,
            ) -> Result<Self, ::polecen::command::CommandArgumentsReadError>
            where
                I: Iterator<Item = &'a str> + Send
            {
                Ok(#parser)
            }
        }
    })
    .into()
}

fn generate_structs(prefix: Option<String>, depth: u8, input: &CommandInput) -> Vec<TokenStream> {
    let parent_name = parent_name!(prefix, depth, input);

    let (mut structs, mut entries) = (Vec::new(), Vec::new());
    let struct_type;
    match input {
        CommandInput::CommandParent { children, .. } => {
            struct_type = quote! { enum };

            for child in children {
                let child_name = child.command_name().to_case(Case::Pascal);
                if let CommandInput::Command { arguments, .. } = child {
                    if arguments.is_empty() {
                        entries.push(quote! { #child_name });
                        continue;
                    }
                }

                let child_args = ident!(MERGE parent_name, child_name);
                entries.push(quote! { #child_name(#child_args) });
                structs.extend(generate_structs(Some(parent_name.to_string()), depth + 1, child));
            }
        },
        CommandInput::Command { arguments, .. } => {
            struct_type = quote! { struct };

            for argument in arguments {
                let ArgumentInput { name, ty, .. } = &argument;
                let required = arg_opt!(argument, required).is_none();
                entries.push(if required {
                    quote! { pub #name: #ty }
                } else {
                    quote! { pub #name: Option<#ty> }
                });
            }
        },
    }

    structs.push(quote! {
        #[derive(Clone, Debug)]
        pub #struct_type #parent_name {
            #(#entries),*
        }
    });
    structs
}

// TODO: check if Derive macros could be used for this purpose
// and whether converting to a derive macro wuold actually make sense

fn generate_parser(
    prefix: Option<String>,
    mut position: u8,
    input: &CommandInput,
    ctx_ident: &Ident,
    args_ident: &Ident,
) -> TokenStream {
    let err = quote!(::polecen::command::CommandArgumentsReadError);

    let parent_name = parent_name!(prefix, position, input);
    match input {
        CommandInput::CommandParent { children, .. } => {
            let mut children_arms = Vec::new();
            for child in children {
                let child_name = child.command_name().to_case(Case::Pascal);
                let mut aliases = child.command_aliases().clone();
                aliases.push(LitStr::new(&child.command_name().to_string(), Span::call_site()));

                if let CommandInput::Command { arguments, .. } = child {
                    if arguments.is_empty() {
                        children_arms
                            .push(quote! { #(#aliases)|* => { #parent_name::#child_name } });
                        continue;
                    }
                }

                let child_parser = generate_parser(
                    Some(parent_name.to_string()),
                    position + 1,
                    child,
                    ctx_ident,
                    args_ident,
                );
                children_arms
                    .push(quote! { #(#aliases)|* => { #parent_name::#child_name(#child_parser) } });
            }

            quote! {
                if let Some(subcommand) = #args_ident.next() {
                    match subcommand {
                        #(#children_arms),*
                        s => {
                            return Err(#err::UnknownSubcommand {
                                position: #position,
                                given: s.to_owned(),
                            });
                        },
                    }
                } else {
                    return Err(#err::MissingSubcommand {
                        position: #position,
                    });
                }
            }
        },
        CommandInput::Command { arguments, .. } => {
            let mut fields = Vec::new();
            for argument in arguments {
                let name = LitStr::new(&argument.name.to_string(), Span::call_site());
                let ArgumentInput { ty, .. } = &argument;

                let name_ident = &argument.name;
                let required = arg_opt!(argument, required).is_none();

                let inner_parse = quote! {
                    #ty::parse_argument(
                        &#ctx_ident,
                        ::polecen::arguments::parse::ArgumentParseRaw {
                            value: arg.to_owned(),
                        },
                    )
                    .await
                    .map_err(|e| #err::ValueParseError { position: #position, inner: e })?
                };
                let parse = if required {
                    inner_parse
                } else {
                    quote! { Some(#inner_parse) }
                };
                let err_handler = if required {
                    quote! {
                        return Err(#err::RequiredArgumentMissing {
                            position: #position,
                            name: String::from(#name),
                        });
                    }
                } else {
                    quote! { None }
                };

                fields.push(quote! {
                    #name_ident: if let Some(arg) = #args_ident.next() {
                        #parse
                    } else {
                        #err_handler
                    }
                });
                position += 1;
            }

            quote! {
                #parent_name {
                    #(#fields),*
                }
            }
        },
    }
}
