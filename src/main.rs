use poise::serenity_prelude as serenity;
use sqlx::SqlitePool;
use std::env;

mod commands;
mod types;
use types::{Context, Data, Error};
use crate::commands::moderation;
use crate::commands::moderation::all_commands;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let token = env::var("DISCORD_TOKEN")
        .expect("Missing DISCORD_TOKEN");

    let db_url = env::var("DATABASE_URL")
        .expect("Missing DATABASE_URL");

    let pool = SqlitePool::connect(&db_url).await?;

    let intents =
        serenity::GatewayIntents::GUILD_MESSAGES
            | serenity::GatewayIntents::MESSAGE_CONTENT
            | serenity::GatewayIntents::GUILD_MEMBERS;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: all_commands(),
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some("n".into()),
                ..Default::default()
            },
            ..Default::default()
        })
        .setup(move |_ctx, _ready, _framework| {
            let pool = pool.clone();

            Box::pin(async move {
                Ok(moderation::Data { db: pool })
            })
        })
        .build();

    // Serenity client
    let mut client = serenity::Client::builder(token, intents)
        .framework(framework)
        .await?;

    client.start().await?;

    Ok(())
}




