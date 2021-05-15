use std::env;
use std::time::{Duration, SystemTime};

use polecen::arguments::prelude::*;
use polecen::read_args;
use serenity::client::{Context, EventHandler};
use serenity::framework::standard::macros::{command, group};
use serenity::framework::standard::{Args, CommandResult};
use serenity::framework::StandardFramework;
use serenity::model::channel::Message;
use serenity::model::prelude::Ready;
use serenity::Client;

struct Handler;

#[serenity::async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

polecen::expand_command_here!((DateCommandArgs) date => {
    duration: Option<Duration>;
});

#[command]
async fn date(ctx: &Context, message: &Message, args: Args) -> CommandResult {
    match read_args!(DateCommandArgs, args.raw_quoted(), ctx, [M] message) {
        Ok(args) => {
            let time = if let Some(add) = args.duration {
                SystemTime::now() + add
            } else {
                SystemTime::now()
            };
            message
                .channel_id
                .say(
                    &ctx.http,
                    format!(":date: Date: `{}`.", humantime::format_rfc3339_seconds(time)),
                )
                .await?;
        },
        Err(e) => {
            message.channel_id.say(&ctx.http, e.to_string()).await?;
        },
    }

    Ok(())
}

#[group]
#[commands(date)]
struct General;

#[tokio::main]
async fn main() {
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let framework = StandardFramework::new()
        .configure(|c| c.with_whitespace(true).prefix("!"))
        .group(&GENERAL_GROUP);
    let mut client = Client::builder(&token)
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("Could not create client");

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
