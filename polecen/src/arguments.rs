use std::fmt;

use async_trait::async_trait;
#[cfg(feature = "patterns")]
use fancy_regex::Regex;
use serenity::client::Context;
use serenity::model::guild::Member;
use serenity::model::id::{GuildId, UserId};

// TODO: convert fancy regex to native rust parsing
// benefits: performance, detecting invalid formatted input, and knowing where it is invalid
// foreseeable breaking change: split_args will return Result

#[cfg(feature = "patterns")]
lazy_static::lazy_static! {
    static ref SPLIT_ARGUMENTS_PATTERN: Regex =
        Regex::new(r#"(?<!\\)"(?:\\.|[^"\\])*?"|(?<!")(?:\\.|[^"\s])+(?!")"#).unwrap();
}

#[cfg(feature = "patterns")]
pub fn split_args<'a>(text: &'a str) -> Box<dyn Iterator<Item = &'a str> + Send + 'a> {
    Box::new(
        SPLIT_ARGUMENTS_PATTERN
            .captures_iter(text)
            .filter_map(|c| c.ok())
            .filter_map(|c| c.get(0))
            .map(|g| g.as_str())
            .filter(|s| !s.trim().is_empty())
            .map(|s| {
                if s.starts_with("\"") && s.ends_with("\"") {
                    s.trim_matches(|c| c == '"')
                } else {
                    s
                }
            }),
    )
}

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

#[derive(Clone, Debug)]
pub struct ArgumentParseRaw {
    pub value: serde_json::Value,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ArgumentParseError {
    InvalidValueType,
    InvalidValueFormat,
}

impl std::error::Error for ArgumentParseError {}

impl fmt::Display for ArgumentParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidValueType => {
                write!(f, "Unsupported value type for parser.")
            },
            Self::InvalidValueFormat => {
                write!(f, "The value doesn't match the expected format.",)
            },
        }
    }
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

// # Argument Type implementations

// ## PRIMITIVES

#[async_trait]
impl ArgumentType for String {
    async fn parse_argument<'a>(
        _ctx: &ArgumentParseContext<'a>,
        raw: ArgumentParseRaw,
    ) -> Result<Self, ArgumentParseError> {
        match raw.value {
            serde_json::Value::String(s) => Ok(s),
            _ => Err(ArgumentParseError::InvalidValueType),
        }
    }
}

// ## SERENITY

macro_rules! parse_id_from_mention {
    ($s:ident, $prefix:literal $(($trim:expr))? $suffix:literal) => {
        (|| $s.strip_prefix($prefix)?.trim_start_matches('!').strip_suffix($suffix)?.parse().ok())()
    };
}

#[async_trait]
impl ArgumentType for Member {
    async fn parse_argument<'a>(
        ctx: &ArgumentParseContext<'a>,
        raw: ArgumentParseRaw,
    ) -> Result<Self, ArgumentParseError> {
        let user_id = match raw.value {
            serde_json::Value::String(s) => UserId(
                parse_id_from_mention!(s, "<@" ('!') ">")
                    .ok_or(ArgumentParseError::InvalidValueFormat)?,
            ),
            serde_json::Value::Object(o) => unimplemented!(),
            _ => return Err(ArgumentParseError::InvalidValueType),
        };

        if let Some(guild_id) = ctx.guild_id {
            // FIXME: unwrap
            let member = guild_id.member(&ctx.event_ctx.http, user_id).await.unwrap();
            Ok(member)
        } else {
            unimplemented!();
        }
    }
}
