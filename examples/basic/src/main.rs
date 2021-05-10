use std::env;
use std::str::FromStr;

use polecen::arguments::prelude::*;
use serenity::client::{Context, EventHandler};
use serenity::model::channel::Message;
use serenity::model::guild::Member;
use serenity::model::prelude::Ready;
use serenity::{async_trait, Client};

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, message: Message) {
        if let Some(content) = message.content.strip_prefix("!") {
            if let Err(e) = run_command(&ctx, &message, content.to_owned()).await {
                match e {
                    CommandArgumentsReadError::RequiredArgumentMissing { position, .. } => {
                        message
                            .channel_id
                            .say(&ctx.http, format!("Missing argument at position `{}`!", position))
                            .await
                            .unwrap();
                    },
                    _ => {
                        message
                            .channel_id
                            .say(&ctx.http, format!(":x: Raw error: {}", e))
                            .await
                            .unwrap();
                    },
                }
            }
        }
    }

    async fn ready(&self, _ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

#[derive(Clone)]
enum Operator {
    ADD,
    SUBSTRACT,
}

impl Operator {
    fn calc(&self, lhs: i32, rhs: i32) -> i32 {
        match self {
            Self::ADD => lhs + rhs,
            Self::SUBSTRACT => lhs - rhs,
        }
    }
}

impl FromStr for Operator {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "+" | "plus" | "add" => Ok(Self::ADD),
            "-" | "minus" | "substract" => Ok(Self::SUBSTRACT),
            _ => Err(()),
        }
    }
}

polecen::expand_command_here!((PolecenCommandArgs) polecen => match {
    perform => {
        target#Member "Target member";
        action#String "Action to perform";
        reason#String [*] "Reason";
    },
    calc => {
        lhs#i32 "Integer";
        op#String "Operator";
        rhs#i32 "Integer";
    },
    version | ver | "?" => {}
});

async fn run_command(
    ctx: &Context,
    message: &Message,
    content: String,
) -> Result<(), CommandArgumentsReadError> {
    let mut args = content.split_whitespace();
    if let Some(command) = args.next() {
        match command {
            "polecen" => {
                let args = polecen::read_args!(PolecenCommandArgs, args, ctx, [M] message)?;
                exec_polecen_command(ctx, message, args).await.unwrap();
            },
            _ => {},
        }
    }
    Ok(())
}

type CommandResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

async fn exec_polecen_command(
    ctx: &Context,
    message: &Message,
    args: PolecenCommandArgs,
) -> CommandResult {
    match &args {
        PolecenCommandArgs::Perform(args) => {
            let PolecenCommandArgsPerform { target, .. } = args;
            message
                .channel_id
                .send_message(&ctx.http, |m| {
                    m.embed(|e| {
                        e.title("Action required");
                        e.color(0xff0000);

                        e.author(|a| {
                            a.name(target.display_name());
                            if let Some(icon_url) = target.user.avatar_url() {
                                a.icon_url(icon_url);
                            }
                            a
                        });

                        e.field(
                            "Information",
                            format!(
                            "You are required to `{}`. Please complete it in the shortest delay.",
                            args.action
                        ),
                            false,
                        );

                        if let Some(reason) = &args.reason {
                            e.field("Reason", reason, false);
                        }

                        e
                    });

                    m
                })
                .await?;
        },
        PolecenCommandArgs::Calc(args) => {
            let PolecenCommandArgsCalc { lhs, rhs, .. } = args;
            if let Ok(op) = args.op.parse::<Operator>() {
                message
                    .channel_id
                    .say(&ctx.http, format!(":white_check_mark: Result: `{}`", op.calc(*lhs, *rhs)))
                    .await?;
            } else {
                message.channel_id.say(&ctx.http, "Unknown operator. Available: `+`, `-`.").await?;
            }
        },
        PolecenCommandArgs::Version(_) => {
            message
                .channel_id
                .say(
                    &ctx.http,
                    format!(":information_source: Version: {}", env!("CARGO_PKG_VERSION")),
                )
                .await?;
        },
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let mut client =
        Client::builder(&token).event_handler(Handler).await.expect("Could not create client");

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
