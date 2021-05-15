//! # Default Argument Types implementations
//! Available with the feature `default_parsers`, this mod provides implementations
//! of ArgumentType for many std types and serenity models.

#[cfg(default_parsers_models)]
pub use models::*;
#[cfg(default_parsers_primitives)]
pub use primitives::*;
#[cfg(default_parsers_time)]
pub use time::*;

macro_rules! default_impl {
    ($ty:ty, $ctx:ident, $raw:ident, $inside:tt) => {
        #[async_trait]
        impl ArgumentType for $ty {
            async fn parse_argument<'a>(
                $ctx: &ArgumentParseContext<'a>,
                $raw: ArgumentParseRaw,
            ) -> Result<Self, ArgumentParseError> {
                $inside
            }
        }
    };
    ($raw:ident) => {
        (match $raw {
            ArgumentParseRaw::String(value) => value.parse(),
            #[cfg(feature = "interactions")]
            ArgumentParseRaw::InteractionData(serde_json::Value::String(value)) => value.parse(),
            #[cfg(feature = "interactions")]
            // FIXME: get numerical value from Number directly instead of using to_string
            ArgumentParseRaw::InteractionData(serde_json::Value::Number(value)) => value.to_string().parse(),
            #[cfg(feature = "interactions")]
            _ => return Err(ArgumentParseError::InvalidValueFormat)
        }).map_err(|_| ArgumentParseError::InvalidValueFormat)
    };
    ($($ty:ty $(=> $middleman:ty)?),+) => {
        $( default_impl!(> $ty, _ctx, raw $(=> $middleman)?); )+
    };
    (> $ty:ty, $ctx:ident, $raw:ident) => {
        default_impl!($ty, $ctx, $raw, {
            Ok(default_impl!($raw)?)
        });
    };
    (> $ty:ty, $ctx:ident, $raw:ident => $middleman:ty) => {
        default_impl!($ty, $ctx, $raw, {
            let value: $middleman = default_impl!($raw)?;
            Ok(value.into())
        });
    };
}

#[cfg(default_parsers_primitives)]
mod primitives {
    use async_trait::async_trait;

    use crate::arguments::parse::*;

    #[async_trait]
    impl ArgumentType for String {
        async fn parse_argument<'a>(
            _ctx: &ArgumentParseContext<'a>,
            raw: ArgumentParseRaw,
        ) -> Result<Self, ArgumentParseError> {
            match raw {
                ArgumentParseRaw::String(value) => Ok(value),
                #[cfg(feature = "interactions")]
                ArgumentParseRaw::InteractionData(serde_json::Value::String(value)) => Ok(value),
                #[cfg(feature = "interactions")]
                _ => Err(ArgumentParseError::InvalidValueType),
            }
        }
    }

    #[async_trait]
    impl ArgumentType for bool {
        async fn parse_argument<'a>(
            _ctx: &ArgumentParseContext<'a>,
            raw: ArgumentParseRaw,
        ) -> Result<Self, ArgumentParseError> {
            match raw {
                ArgumentParseRaw::String(value) => {
                    Ok(value.parse().map_err(|_| ArgumentParseError::InvalidValueFormat)?)
                },
                #[cfg(feature = "interactions")]
                ArgumentParseRaw::InteractionData(serde_json::Value::Bool(value)) => Ok(value),
                #[cfg(feature = "interactions")]
                _ => Err(ArgumentParseError::InvalidValueType),
            }
        }
    }

    default_impl!(char, f32, f64, i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);
}

#[cfg(default_parsers_time)]
mod time {
    use std::time::{Duration, SystemTime};

    use async_trait::async_trait;

    use crate::arguments::parse::*;

    default_impl!(Duration => humantime::Duration, SystemTime => humantime::Timestamp);
}

#[cfg(default_parsers_models)]
mod models {
    use async_trait::async_trait;
    use serenity::model::channel::{Channel, GuildChannel};
    use serenity::model::guild::{Member, Role};
    use serenity::model::id::{ChannelId, RoleId, UserId};
    use serenity::model::prelude::User;

    use crate::arguments::parse::*;

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

    macro_rules! str_or_obj {
        ($ty:ty, fn($ctx:ident, $value:ident) $from_str:tt) => {
            #[async_trait]
            impl ArgumentType for $ty {
                async fn parse_argument<'a>(
                    $ctx: &ArgumentParseContext<'a>,
                    raw: ArgumentParseRaw,
                ) -> Result<Self, ArgumentParseError> {
                    match raw {
                        ArgumentParseRaw::String($value) => $from_str,
                        #[cfg(feature = "interactions")]
                        ArgumentParseRaw::InteractionData(value @ serde_json::Value::Object(_)) => {
                            Ok(serde_json::from_value(value)
                                .map_err(|_| ArgumentParseError::InvalidValueFormat)?)
                        },
                        #[cfg(feature = "interactions")]
                        _ => Err(ArgumentParseError::InvalidValueType),
                    }
                }
            }
        };
    }

    str_or_obj!(User, fn(ctx, value) {
        let user = parse_id!(value, UserId)
            .to_user(&ctx.event_ctx.http)
            .await
            .map_err(ArgumentParseError::from)?;
        Ok(user)
    });

    #[async_trait]
    impl ArgumentType for Member {
        async fn parse_argument<'a>(
            ctx: &ArgumentParseContext<'a>,
            raw: ArgumentParseRaw,
        ) -> Result<Self, ArgumentParseError> {
            if let Some(guild_id) = ctx.guild_id {
                let user_id = match raw {
                    ArgumentParseRaw::String(value) => parse_id!(value, UserId),
                    #[cfg(feature = "interactions")]
                    ArgumentParseRaw::InteractionData(serde_json::Value::Object(object)) => {
                        parse_id!(
                            object
                                .get("id")
                                .map(serde_json::Value::as_str)
                                .flatten()
                                .ok_or(ArgumentParseError::InvalidValueFormat)?
                                .to_owned(),
                            UserId
                        )
                    },
                    #[cfg(feature = "interactions")]
                    _ => return Err(ArgumentParseError::InvalidValueType),
                };
                let member = guild_id
                    .member(&ctx.event_ctx.http, user_id)
                    .await
                    .map_err(ArgumentParseError::from)?;
                Ok(member)
            } else {
                Err(ArgumentParseError::CannotParseInContext("Expected guild".to_owned()))
            }
        }
    }

    str_or_obj!(Channel, fn(ctx, value) {
        let channel = parse_id!(value, ChannelId)
            .to_channel(&ctx.event_ctx.http)
            .await
            .map_err(ArgumentParseError::from)?;
        Ok(channel)
    });

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

    str_or_obj!(Role, fn(ctx, value) {
        if let Some(guild_id) = ctx.guild_id {
            let role_id = parse_id!(value, RoleId);
            if let Some(role) = ctx
                .event_ctx
                    .cache
                    .guild_field(&guild_id, |guild| guild.roles.get(&role_id).map(|r| r.clone()))
                    .await
                    .ok_or(ArgumentParseError::CannotParseInContext(
                            "Guild not in cache".to_owned(),
                    ))?
            {
                Ok(role)
            } else {
                Err(ArgumentParseError::CannotParseInContext(
                        "Role does not exist in guild".to_owned(),
                ))
            }
        } else {
            Err(ArgumentParseError::CannotParseInContext("Expected guild".to_owned()))
        }
    });
}
