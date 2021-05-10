use std::fmt;

use async_trait::async_trait;
use serenity::client::Context;
use serenity::model::id::GuildId;

#[derive(Clone)]
#[non_exhaustive]
pub struct ArgumentParseContext<'a> {
    pub event_ctx: &'a Context,
    pub guild_id: Option<GuildId>,
}

impl<'a> ArgumentParseContext<'a> {
    pub fn new(event_ctx: &'a Context, guild_id: Option<GuildId>) -> ArgumentParseContext<'a> {
        Self { event_ctx, guild_id }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ArgumentParseError {
    InvalidValueType,
    InvalidValueFormat,
    CannotParseInContext(String),
}

impl std::error::Error for ArgumentParseError {}

impl fmt::Display for ArgumentParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidValueType => {
                write!(f, "Unsupported value type for parser.")
            },
            Self::InvalidValueFormat => {
                write!(f, "The value doesn't match the expected format.")
            },
            Self::CannotParseInContext { .. } => {
                write!(f, "The value cannot be parsed in the current context.")
            },
        }
    }
}

#[derive(Clone, Debug)]
pub struct ArgumentParseRaw {
    pub value: String,
}

#[async_trait]
pub trait ArgumentType
where
    Self: Sized,
{
    async fn parse_argument<'a>(
        ctx: &ArgumentParseContext<'a>,
        raw: ArgumentParseRaw,
    ) -> Result<Self, ArgumentParseError>;
}
