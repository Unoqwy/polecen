use syn::parse::{Parse, ParseStream};
use syn::{braced, bracketed, parenthesized, Ident, LitInt, LitStr, Token, Type};

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

pub(crate) struct CommandExpandInput {
    pub prefix: Option<Ident>,
    pub command: CommandInput,
}

impl Parse for CommandExpandInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let prefix = optional_wrapped!(input, parenthesized);
        Ok(Self { prefix, command: input.parse()? })
    }
}

pub(crate) enum CommandInput {
    CommandParent { name: Ident, aliases: Vec<LitStr>, children: Vec<CommandInput> },
    Command { name: Ident, aliases: Vec<LitStr>, arguments: Vec<ArgumentInput> },
}

impl CommandInput {
    pub fn command_name(&self) -> Ident {
        match self {
            Self::CommandParent { name, .. } => name.clone(),
            Self::Command { name, .. } => name.clone(),
        }
    }

    pub fn command_aliases(&self) -> Vec<LitStr> {
        match self {
            Self::CommandParent { aliases, .. } => aliases.clone(),
            Self::Command { aliases, .. } => aliases.clone(),
        }
    }
}

impl Parse for CommandInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name = input.parse()?;
        let mut aliases = Vec::new();
        while input.peek(Token![|]) {
            input.parse::<Token![|]>()?;
            if input.peek(Ident) {
                let ident: Ident = input.parse()?;
                aliases.push(LitStr::new(&ident.to_string(), ident.span()))
            } else {
                aliases.push(input.parse()?);
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
            Ok(Self::CommandParent { name, aliases, children })
        } else {
            let arguments = content
                .parse_terminated::<ArgumentInput, Token![;]>(ArgumentInput::parse)?
                .into_iter()
                .collect();
            Ok(Self::Command { name, aliases, arguments })
        }
    }
}

pub(crate) struct ArgumentInput {
    pub name: Ident,
    pub ty: Type,
    pub opts: Option<ArgumentOptionsInput>,
    pub description: Option<LitStr>,
}

impl Parse for ArgumentInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name = input.parse()?;
        input.parse::<Token![#]>()?;
        let ty: Type = input.parse()?;
        let opts = optional_wrapped!(input, bracketed);
        Ok(ArgumentInput { name, ty, opts, description: input.parse()? })
    }
}

pub(crate) struct ArgumentOptionsInput {
    pub required: Option<Token![*]>,
    pub span: Option<LitInt>,
}

impl Parse for ArgumentOptionsInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let required = input.parse()?;
        let span;
        if input.peek(Token![:]) {
            input.parse::<Token![:]>()?;
            span = Some(input.parse()?);
        } else {
            span = None;
        }
        Ok(Self { required, span })
    }
}
