use convert_case::Case;
use proc_macro2::Span;
use syn::parse::{Parse, ParseStream};
use syn::{braced, bracketed, parenthesized, Ident, LitInt, LitStr, Token, Type};

use crate::utils::ConvertCase;

mod kw {
    syn::custom_keyword!(Option);
}

macro_rules! optional_wrapped {
    ($input:ident, $wrapper:ident) => {
        if let Some(opt_input) = (|| {
            let content;
            $wrapper!(content in $input);
            Ok(content)
        })()
        .ok()
        {
            Some(opt_input.parse()?)
        } else {
            None
        }
    };
}

macro_rules! optional {
    ($input:ident) => {
        if $input.parse::<Option<Token![,]>>()?.is_some() {
            $input.parse()?
        } else {
            None
        }
    };
}

pub(crate) enum CommandInput {
    CommandParent { struct_name: Ident, pattern: Vec<LitStr>, children: Vec<CommandInput> },
    Command { struct_name: Ident, pattern: Vec<LitStr>, arguments: Vec<ArgumentInput> },
}

impl CommandInput {
    pub fn struct_name(&self) -> Ident {
        match self {
            Self::CommandParent { struct_name, .. } => struct_name.clone(),
            Self::Command { struct_name, .. } => struct_name.clone(),
        }
    }

    pub fn command_pattern(&self) -> Vec<LitStr> {
        match self {
            Self::CommandParent { pattern, .. } => pattern.clone(),
            Self::Command { pattern, .. } => pattern.clone(),
        }
    }
}

impl Parse for CommandInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let struct_name: Option<Ident> = optional_wrapped!(input, parenthesized);
        let name: Ident = input.parse()?;
        let struct_name = if let Some(struct_name) = struct_name {
            struct_name
        } else {
            name.clone().to_case(Case::Pascal)
        };

        let mut pattern = Vec::new();
        pattern.push(LitStr::new(&name.to_string(), Span::call_site()));
        while input.peek(Token![|]) {
            input.parse::<Token![|]>()?;
            if input.peek(Ident) {
                let ident: Ident = input.parse()?;
                pattern.push(LitStr::new(&ident.to_string(), ident.span()))
            } else {
                pattern.push(input.parse()?);
            }
        }

        input.parse::<Token![=>]>()?;
        let is_parent = input.peek(Token![match]);
        if is_parent {
            input.parse::<Token![match]>()?;
        }

        let content;
        braced!(content in input);
        if is_parent {
            let children = content
                .parse_terminated::<CommandInput, Token![,]>(Self::parse)?
                .into_iter()
                .collect();
            Ok(Self::CommandParent { struct_name, pattern, children })
        } else {
            let arguments = content
                .parse_terminated::<ArgumentInput, Token![;]>(ArgumentInput::parse)?
                .into_iter()
                .collect();
            Ok(Self::Command { struct_name, pattern, arguments })
        }
    }
}

pub(crate) struct ArgumentInput {
    pub name: Ident,
    pub ty: Type,
    pub required: bool,
    pub opts: Option<ArgumentOptionsInput>,
    pub description: Option<LitStr>,
}

impl Parse for ArgumentInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name = input.parse()?;
        input.parse::<Token![:]>()?;

        let (ty, optional) = if input.peek(kw::Option) {
            input.parse::<kw::Option>()?;
            input.parse::<Token![<]>()?;
            let ty = input.parse()?;
            input.parse::<Token![>]>()?;
            (ty, true)
        } else {
            (input.parse()?, false)
        };

        let opts = optional_wrapped!(input, bracketed);
        Ok(ArgumentInput { name, ty, required: !optional, opts, description: optional!(input) })
    }
}

pub(crate) struct ArgumentOptionsInput {
    pub span: Option<LitInt>,
}

impl Parse for ArgumentOptionsInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let span = input.parse()?;
        Ok(Self { span })
    }
}
