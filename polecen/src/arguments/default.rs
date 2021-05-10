//! # Default Argument Types implementations
//! Available with the feature `default_parsers`, this mod provides implementations
//! of ArgumentType for many std types and serenity models.

use async_trait::async_trait;
use serenity::model::channel::{Channel, GuildChannel};
use serenity::model::guild::Member;
use serenity::model::id::{ChannelId, UserId};
use serenity::model::prelude::User;

use crate::arguments::parse::{
    ArgumentParseContext,
    ArgumentParseError,
    ArgumentParseRaw,
    ArgumentType,
};

// Std

macro_rules! std_parse {
    ($($ty:ty),+) => {
        $(std_parse!(> $ty);)+
    };
    (> $ty:ty) => {
        #[async_trait]
        impl ArgumentType for $ty {
            async fn parse_argument<'a>(
                _ctx: &ArgumentParseContext<'a>,
                raw: ArgumentParseRaw,
            ) -> Result<Self, ArgumentParseError> {
                Ok(raw.value.parse().map_err(|_| ArgumentParseError::InvalidValueFormat)?)
            }
        }
    };
}

#[async_trait]
impl ArgumentType for String {
    async fn parse_argument<'a>(
        _ctx: &ArgumentParseContext<'a>,
        raw: ArgumentParseRaw,
    ) -> Result<Self, ArgumentParseError> {
        Ok(raw.value)
    }
}

std_parse!(bool, char, f32, f64, i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);

// Serenity

macro_rules! parse_id {
    ($value:expr, $ty:ty) => {
        $value.parse::<$ty>().map_err(|_| ArgumentParseError::InvalidValueFormat)?
    };
}

impl From<serenity::Error> for ArgumentParseError {
    fn from(err: serenity::Error) -> Self {
        Self::CannotParseInContext(format!("{}", err))
    }
}

#[async_trait]
impl ArgumentType for User {
    async fn parse_argument<'a>(
        ctx: &ArgumentParseContext<'a>,
        raw: ArgumentParseRaw,
    ) -> Result<Self, ArgumentParseError> {
        let user = parse_id!(raw.value, UserId)
            .to_user(&ctx.event_ctx.http)
            .await
            .map_err(ArgumentParseError::from)?;
        Ok(user)
    }
}

#[async_trait]
impl ArgumentType for Member {
    async fn parse_argument<'a>(
        ctx: &ArgumentParseContext<'a>,
        raw: ArgumentParseRaw,
    ) -> Result<Self, ArgumentParseError> {
        if let Some(guild_id) = ctx.guild_id {
            let member = guild_id
                .member(&ctx.event_ctx.http, parse_id!(raw.value, UserId))
                .await
                .map_err(ArgumentParseError::from)?;
            Ok(member)
        } else {
            Err(ArgumentParseError::CannotParseInContext("Expected guild".to_owned()))
        }
    }
}

#[async_trait]
impl ArgumentType for Channel {
    async fn parse_argument<'a>(
        ctx: &ArgumentParseContext<'a>,
        raw: ArgumentParseRaw,
    ) -> Result<Self, ArgumentParseError> {
        let channel = parse_id!(raw.value, ChannelId)
            .to_channel(&ctx.event_ctx.http)
            .await
            .map_err(ArgumentParseError::from)?;
        Ok(channel)
    }
}

#[async_trait]
impl ArgumentType for GuildChannel {
    async fn parse_argument<'a>(
        ctx: &ArgumentParseContext<'a>,
        raw: ArgumentParseRaw,
    ) -> Result<Self, ArgumentParseError> {
        let channel = Channel::parse_argument(ctx, raw).await?.guild().ok_or(
            ArgumentParseError::CannotParseInContext(
                "Channel does not belong to a guild".to_owned(),
            )
            .to_owned(),
        )?;
        Ok(channel)
    }
}
