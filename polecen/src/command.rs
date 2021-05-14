use std::fmt;

use async_trait::async_trait;

use crate::arguments::parse::{ArgumentParseContext, ArgumentParseError};

#[async_trait]
pub trait CommandArguments
where
    Self: Sized,
{
    async fn read_arguments<'a, I>(
        args: I,
        position: u8,
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
