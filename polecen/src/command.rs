use std::fmt;

use async_trait::async_trait;

use crate::arguments::{ArgumentParseContext, ArgumentParseError};

#[async_trait]
pub trait CommandArguments
where
    Self: Sized,
{
    async fn read_arguments<'a, I>(
        args: I,
        ctx: ArgumentParseContext<'a>,
    ) -> Result<Self, CommandArgumentsReadError>
    where
        I: Iterator<Item = &'a str> + Send;
}

#[derive(Clone, Debug)]
pub enum CommandArgumentsReadError {
    RequiredArgumentMissing { position: u8, name: String },
    MissingSubcommand { position: u8 },
    UnknownSubcommand { position: u8, given: String },
    ValueParseError { position: u8, inner: ArgumentParseError },
}

impl std::error::Error for CommandArgumentsReadError {}

impl fmt::Display for CommandArgumentsReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)?;
        Ok(())
    }
}

// #[derive(Clone, Debug)]
// pub struct Command {
//     pub name: String,
//     pub description: Option<String>,
//     pub values: Vec<CommandField>,
// }
//
// #[derive(Clone, Debug)]
// pub struct CommandField {
//     pub name: String,
//     pub description: Option<String>,
//     pub required: bool,
//     pub span: Option<u16>,
// }
