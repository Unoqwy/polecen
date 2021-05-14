use proc_macro::TokenStream as StdTokenStream;
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::{parse_macro_input, LitStr};

use crate::input::{ArgumentInput, CommandInput};

mod input;
mod utils;

macro_rules! metavar {
    ($ty:ty; $s:expr) => {
        <$ty>::new($s, Span::call_site())
    };
    ($ty:ty; MERGE $lhs:expr, $rhs:expr) => {
        <$ty>::new(&format!("{}{}", $lhs, $rhs), Span::call_site())
    };
}

macro_rules! ident {
    ($s:expr) => {
        metavar!(Ident; $s);
    };
    (MERGE $lhs:expr, $rhs:expr) => {
        metavar!(Ident; MERGE $lhs, $rhs);
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

/// Generate argument structures and reader implementations in current scope.
///
/// FIXME: examples
#[proc_macro]
pub fn expand_command_here(tokens: StdTokenStream) -> StdTokenStream {
    let input = parse_macro_input!(tokens as CommandInput);

    let mut structs = Vec::new();
    generate_arguments(&mut structs, &input, None, &ident!("ctx"), &ident!("args"));
    if structs.is_empty() {
        return StdTokenStream::new();
    }

    (quote! { #(#structs)* }).into()
}

/// Generate argument structures and readers.
/// Returns ident of top-level structure (input) name.
///
/// # Arguments
///
/// * `structs` - Output, structures and parser impls are wrriten to this vector
/// * `input` - Command input
/// * `prefix` - Name of the upper-level input, if None this is a top-level call
/// * `ctx_ident` - Ident of the ctx variable in scope
/// * `args_ident` - Ident of the args variable in scope
pub(crate) fn generate_arguments(
    structs: &mut Vec<TokenStream>,
    input: &CommandInput,
    prefix: Option<String>,
    ctx_ident: &Ident,
    args_ident: &Ident,
) -> Ident {
    let err = quote!(::polecen::command::CommandArgumentsReadError);

    let parent_name = if let Some(prefix) = prefix {
        ident!(MERGE prefix, input.struct_name())
    } else {
        ident!(&input.struct_name().to_string())
    };

    let mut entries = Vec::new();
    let reader = match input {
        CommandInput::CommandParent { children, .. } => {
            let mut children_arms = Vec::new();
            for child in children {
                let child_name = child.struct_name();
                let pattern = child.command_pattern();
                if let CommandInput::Command { arguments, .. } = child {
                    if arguments.is_empty() {
                        entries.push(quote! { #child_name });
                        children_arms.push(quote! { #(#pattern)|* => { Self::#child_name } });
                        continue;
                    }
                }

                let child_struct = generate_arguments(
                    structs,
                    child,
                    Some(parent_name.to_string()),
                    ctx_ident,
                    args_ident,
                );
                entries.push(quote! { #child_name(#child_struct) });
                children_arms.push(quote! { #(#pattern)|* => {
                    Self::#child_name(#child_struct::read_arguments(args, position + 1, ctx).await?)
                } });
            }

            quote! {
                if let Some(subcommand) = #args_ident.next() {
                    match subcommand {
                        #(#children_arms),*
                        s => {
                            return Err(#err::UnknownSubcommand {
                                position: position,
                                given: s.to_owned(),
                            });
                        },
                    }
                } else {
                    return Err(#err::MissingSubcommand {
                        position: position,
                    });
                }
            }
        },
        CommandInput::Command { arguments, .. } => {
            let mut fields = Vec::new();
            for (i, argument) in arguments.iter().enumerate() {
                let i = i as u8;
                let ArgumentInput { name: field, ty, .. } = &argument;

                let required = arg_opt!(argument, required).is_none();
                entries.push(if required {
                    quote! { pub #field: #ty }
                } else {
                    quote! { pub #field: Option<#ty> }
                });

                let inner_parse = quote! {
                    #ty::parse_argument(
                        &#ctx_ident,
                        ::polecen::arguments::parse::ArgumentParseRaw {
                            value: arg.to_owned(),
                        },
                    )
                    .await
                    .map_err(|e| #err::ValueParseError { position: position + #i, inner: e })?
                };

                let (parse, err_handler) = if required {
                    let name = metavar!(LitStr; &argument.name.to_string());
                    (inner_parse, quote! {
                        return Err(#err::RequiredArgumentMissing {
                            position: position + #i,
                            name: String::from(#name),
                        });
                    })
                } else {
                    (quote! { Some(#inner_parse) }, quote! { None })
                };
                fields.push(quote! {
                    #field: if let Some(arg) = #args_ident.next() {
                        #parse
                    } else {
                        #err_handler
                    }
                });
            }

            quote! {
                Self {
                    #(#fields),*
                }
            }
        },
    };

    let struct_type = match input {
        CommandInput::CommandParent { .. } => quote! { enum },
        CommandInput::Command { .. } => quote! { struct },
    };
    structs.push(quote! {
        #[derive(Clone, Debug)]
        pub #struct_type #parent_name {
            #(#entries),*
        }

        #[::polecen::async_trait]
        impl ::polecen::command::CommandArguments for #parent_name {
            async fn read_arguments<'a, I>(
                mut args: I,
                position: u8,
                ctx: ::polecen::arguments::parse::ArgumentParseContext<'a>,
            ) -> Result<Self, ::polecen::command::CommandArgumentsReadError>
            where
                I: Iterator<Item = &'a str> + Send
            {
                Ok(#reader)
            }
        }
    });

    parent_name
}
