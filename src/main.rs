use poise::serenity_prelude as serenity;
use sqlx::SqlitePool;
use std::env;
use std::sync::Arc;
use poise::futures_util::lock::Mutex;
use serenity::all::FullEvent;

mod commands;
mod helpers;

mod types;
mod structs;
use types::{Context, Data, Error};

use crate::commands::all_commands;
use crate::helpers::starboard;
use crate::helpers::reminder::ReminderStore;
use crate::helpers::reminder_task::reminder_task;
use crate::helpers::starboard::Database;

use crate::helpers::starboard_manager::{
    handle_reaction_add,
    handle_reaction_remove,
    handle_reaction_remove_all
};

// split this whole file later down the line

async fn on_error(error: poise::FrameworkError<'_, Data, Error>)
{

    match &error {
        poise::FrameworkError::Setup { error, ..} => panic!("Failed to start bot: {}", error),
        poise::FrameworkError::Command { ctx, error, .. } |
        poise::FrameworkError::ArgumentParse { ctx, error, .. } => {
            println!("Command failed: `{}`: {:?}", ctx.command().name, error);

            match ctx {
                poise::Context::Prefix(prefix_ctx) => {
                    let _ = prefix_ctx.msg.react(&prefix_ctx.serenity_context().http, '❌').await;
                }
                _ => {}
            }
        }
        poise::FrameworkError::CommandCheckFailed { ctx, .. } => {
            let guild_name = ctx
                .guild_id()
                .and_then(|guild_id| ctx.cache().guild(guild_id))
                .map(|guild| guild.name.clone())
                .unwrap_or_else(|| "Unknown Guild".to_string());  // defaults to unknown guild despite valid id? fix later

            println!("Command permissions failed: `{}` ran by {} in {}", ctx.command().name, ctx.author().name, guild_name);

            match ctx {
                poise::Context::Prefix(prefix_ctx) => {
                    let _ = prefix_ctx.msg.react(&prefix_ctx.serenity_context().http, '❌').await;
                }
                _ => {}
            }
        }
        _ => {
            if let Err(e) = poise::builtins::on_error(error).await {
                println!("Unknown error {}", e)
            }
        }
    }

}

async fn event_handler(
    ctx: &serenity::Context,
    event: &FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    match event {
        FullEvent::ReactionAdd { add_reaction} => {
            handle_reaction_add(ctx, add_reaction, data).await?;
        }
        FullEvent::ReactionRemove { removed_reaction} => {
            handle_reaction_remove(ctx, removed_reaction, data).await?;
        }
        FullEvent::ReactionRemoveAll {channel_id, removed_from_message_id} => {
            handle_reaction_remove_all(ctx, *channel_id, *removed_from_message_id, data).await?;
        }
        _ => {}
    }
    Ok(())
}


#[tokio::main]
async fn main() -> Result<(), Error> {
    let token = env::var("DISCORD_TOKEN")
        .expect("Missing DISCORD_TOKEN");

    let db_url = env::var("DATABASE_URL")
        .expect("Missing DATABASE_URL");

    let pool = SqlitePool::connect(&db_url).await?;

    let http_client = Arc::new(serenity::Http::new(&token));

    let intents =
        serenity::GatewayIntents::GUILD_MESSAGES
            | serenity::GatewayIntents::MESSAGE_CONTENT
            | serenity::GatewayIntents::GUILD_MEMBERS
            | serenity::GatewayIntents::GUILD_MESSAGE_REACTIONS;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: all_commands(),
            event_handler: |ctx, event, framework, data| {
                Box::pin(event_handler(ctx, event, framework, data))
            },
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some("n".into()),
                ..Default::default()
            },
            pre_command: |ctx| {
                Box::pin(async move {
                    let data = ctx.data();
                    println!("{}", *data.last_command_success.lock().await);
                    *data.last_command_success.lock().await = true;
                })
            },
            on_error: |error| Box::pin(on_error(error)),
            post_command: |ctx| {
                Box::pin(async move {
                    let data = ctx.data();
                    let success = *data.last_command_success.lock().await;
                    println!("Success: {:?}", success);
                    if success {
                        println!("Command {} ran", ctx.command().qualified_name);
                        match ctx {
                            poise::Context::Prefix(prefix_ctx) =>
                                {
                                    if let Err(e) = prefix_ctx.msg.react(&prefix_ctx.serenity_context().http, '✅').await
                                    {
                                        eprintln!("Error sending message: {:?}", e);
                                    }
                                }
                            poise::Context::Application(_) => {

                            }
                        }
                    } else {
                        println!("Command {} failed", ctx.command().qualified_name);

                    }
                    *data.last_command_success.lock().await = true;
                })
            },
            ..Default::default()
        })
        .setup(move |_ctx, _ready, _framework| {
            let pool = pool.clone();
            let http_client = Arc::clone(&http_client);

            Box::pin(async move {
                let reminders = ReminderStore::new(pool.clone());
                let starboard = Database::new(&db_url).await?;

                let data = Data {
                    db: pool.clone(),
                    last_command_success: Arc::from(Mutex::new(true)),
                    reminders: reminders.clone(),
                    http_client: Arc::clone(&http_client),
                    starboard: starboard.clone(),
                    starboard_lock: Mutex::new(()),

                };

                let task_data = Data {
                    db: pool,
                    last_command_success: Arc::new(Default::default()),
                    reminders,
                    http_client,
                    starboard,
                    starboard_lock: Mutex::new(()),
                };

                tokio::spawn(async move {
                    reminder_task(Arc::from(task_data)).await;
                });

                Ok(data)
            })
        })
        .build();

    let mut client = serenity::Client::builder(token, intents)
        .framework(framework)
        .await?;



    client.start().await?;

    Ok(())
}




