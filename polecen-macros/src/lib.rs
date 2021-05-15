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

// macro_rules! arg_opt {
//     ($arg:ident, $($field:ident).+, $def:expr) => {
//         if let Some(opts) = &$arg.opts { opts.$($field).+ } else { $def }
//     };
//     ($arg:ident, $($field:ident).+) => {
//         arg_opt!($arg, $($field).+, None)
//     };
// }

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

macro_rules! subcommand_reader {
    ($children_arms:ident, $fn_reader:literal, [$($unpack:tt)+] = [$($set:tt)+]) => {{
        let arms: Vec<TokenStream> = $children_arms
            .iter()
            .map(|c| c(&ident!($fn_reader)))
            .collect();
        quote! {
            if let $($unpack)+ = $($set)+ {
                match subcommand {
                    #(#arms),*
                    s => {
                        return Err(::polecen::command::CommandArgumentsReadError::UnknownSubcommand {
                            position: position,
                            given: s.to_owned(),
                        });
                    },
                }
            } else {
                return Err(::polecen::command::CommandArgumentsReadError::MissingSubcommand {
                    position: position,
                });
            }
        }
    }};
}

macro_rules! struct_reader {
    ($fields_readers:ident, [$($unpack:tt)+] = [$($set:tt)+] -> $($parse:tt)+) => {{
        let unpack = quote! { $($unpack)+ };
        let set = quote! { $($set)+ };
        let parse = quote! { $($parse)+ };
        let fields: Vec<TokenStream> = $fields_readers
            .iter()
            .map(|c| c(&unpack, &set, &parse))
            .collect();
        quote! {
            Self {
                #(#fields),*
            }
        }
    }}
}

#[derive(Clone, Debug)]
struct Readers<T> {
    str_reader: T,
    #[cfg(feature = "interactions")]
    interaction_reader: T,
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
    let parent_name = if let Some(prefix) = prefix {
        ident!(MERGE prefix, input.struct_name())
    } else {
        ident!(&input.struct_name().to_string())
    };

    let mut entries = Vec::new();
    let readers = match input {
        CommandInput::CommandParent { children, .. } => {
            let mut children_arms: Vec<Box<dyn Fn(&Ident) -> TokenStream>> = Vec::new();
            for child in children {
                let child_name = child.struct_name();
                let pattern = child.command_pattern();
                if let CommandInput::Command { arguments, .. } = child {
                    if arguments.is_empty() {
                        entries.push(quote! { #child_name });
                        children_arms.push(Box::new(
                            move |_| quote! { #(#pattern)|* => { Self::#child_name } },
                        ));
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
                children_arms.push(Box::new(move |fn_reader| {
                    quote! { #(#pattern)|* => {
                        Self::#child_name(#child_struct::#fn_reader(args, position + 1, ctx).await?)
                    } }
                }));
            }

            Readers {
                str_reader: subcommand_reader!(children_arms, "read_str_arguments",
                    [Some(subcommand)] = [#args_ident.next()]
                ),
                #[cfg(feature = "interactions")]
                interaction_reader: subcommand_reader!(children_arms, "read_command_interaction_data",
                    [Some(polecen::serde_json::Value::String(subcommand))] = [#args_ident.value] // FIXME; make it work
                ),
            }
        },
        CommandInput::Command { arguments, .. } => {
            let mut fields_readers: Vec<
                Box<dyn Fn(&TokenStream, &TokenStream, &TokenStream) -> TokenStream>,
            > = Vec::new();
            for (i, argument) in arguments.iter().enumerate() {
                let i = i as u8;
                let ArgumentInput { name: field, ty, required, .. } = &argument;
                let required = *required;

                entries.push(if required {
                    quote! { pub #field: #ty }
                } else {
                    quote! { pub #field: Option<#ty> }
                });

                let inner_parse: Box<dyn Fn(&TokenStream) -> TokenStream> = Box::new(
                    move |parser| {
                        quote! {
                            #ty::parse_argument(
                                &#ctx_ident,
                                ::polecen::arguments::parse::ArgumentParseRaw::#parser
                            )
                            .await
                            .map_err(|e| ::polecen::command::CommandArgumentsReadError::ValueParseError { position: position + #i, inner: e })?
                        }
                    },
                );

                let (parse, err_handler): (Box<dyn Fn(&TokenStream) -> TokenStream>, TokenStream) =
                    if required {
                        let name = metavar!(LitStr; &argument.name.to_string());
                        (inner_parse, quote! {
                            return Err(::polecen::command::CommandArgumentsReadError::RequiredArgumentMissing {
                                position: position + #i,
                                name: String::from(#name),
                            });
                        })
                    } else {
                        (
                            Box::new(move |parser| {
                                let inner_parse = inner_parse(parser);
                                quote! { Some(#inner_parse) }
                            }),
                            quote! { None },
                        )
                    };
                fields_readers.push(Box::new(move |unpack, set, parser| {
                    let parse = parse(&parser);
                    quote! {
                        #field: if let #unpack = #set {
                            #parse
                        } else {
                            #err_handler
                        }
                    }
                }));
            }

            Readers {
                str_reader: struct_reader!(fields_readers, [Some(arg)] = [#args_ident.next()] -> String(arg.to_owned())),
                #[cfg(feature = "interactions")]
                interaction_reader: quote! {}, // FIXME
            }
        },
    };

    let struct_type = match input {
        CommandInput::CommandParent { .. } => quote! { enum },
        CommandInput::Command { .. } => quote! { struct },
    };

    let Readers { str_reader, .. } = readers;
    #[cfg(feature = "interactions")]
    let interaction_reader = readers.interaction_reader;
    let fns = vec![
        quote! {
            async fn read_str_arguments<'a, I>(
                mut args: I,
                position: u8,
                ctx: ::polecen::arguments::parse::ArgumentParseContext<'a>,
            ) -> Result<Self, ::polecen::command::CommandArgumentsReadError>
            where
                I: Iterator<Item = &'a str> + Send
            {
                Ok(#str_reader)
            }
        },
        #[cfg(feature = "interactions")]
        quote! {
            async fn read_command_interaction_data<'a>(
                args: serenity::model::interactions::ApplicationCommandInteractionData,
                position: u8,
                ctx: ArgumentParseContext<'a>,
            ) -> Result<Self> {
                Ok(#interaction_reader)
            }
        },
    ];

    structs.push(quote! {
        #[derive(Clone, Debug)]
        pub #struct_type #parent_name {
            #(#entries),*
        }

        #[::polecen::async_trait]
        impl ::polecen::command::CommandArguments for #parent_name {
            #(#fns)*
        }
    });

    parent_name
}
