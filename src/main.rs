use poise::{insert_owners_from_http, serenity_prelude as serenity};
use sqlx::SqlitePool;
use std::env;
use std::sync::Arc;
use std::time::Instant;
use poise::futures_util::lock::Mutex;
use serenity::all::{FullEvent, UserId};

mod commands;
mod helpers;

mod types;
mod structs;
mod tooling;

use types::{Context, Data, Error};

use crate::commands::all_commands;
use crate::helpers::auth::AuthDatabase;
use crate::helpers::starboard;
use crate::helpers::reminder::ReminderStore;
use crate::helpers::reminder_task::reminder_task;
use crate::helpers::starboard::Database;


use crate::helpers::starboard_manager::{
    handle_reaction_add,
    handle_reaction_remove,
    handle_reaction_remove_all
};
use crate::tooling::osu::api::OsuClient;
// split this whole file later down the line

async fn on_error(error: poise::FrameworkError<'_, Data, Error>)
{

    match &error {
        poise::FrameworkError::Setup { error, ..} => panic!("Failed to start bot: {}", error),
        poise::FrameworkError::NotAnOwner { ctx, .. } => {
            let message = format!("{} is not in the sudoers file. This incident will be reported.", ctx.author().name);
            let _ = ctx.say(message).await;
        }
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
    let token = env::var("DISCORD_TOKEN")?;
    let db_url = env::var("DATABASE_URL")?;
    let client_id: u32 = env::var("OSU_CLIENT_ID")?.parse()?;
    let client_oauth = env::var("OSU_CLIENT_OAUTH")?;

    let osu = OsuClient::new(
        client_id,
        &client_oauth,
    ).await?;

    let pool = SqlitePool::connect(&db_url).await?;
    let http_client = Arc::new(serenity::Http::new(&token));

    let reminders = ReminderStore::new(pool.clone());
    let starboard = Database::new(&db_url).await?;
    let auth = Arc::new(AuthDatabase::new(pool.clone()));
    auth.create_tables().await?;
    helpers::role_colours::init_role_colour_table(&pool).await?;

    sqlx::query("PRAGMA journal_mode = WAL;").execute(&pool).await?;
    sqlx::query("PRAGMA synchronous = NORMAL;").execute(&pool).await?;

    let owners = std::env::var("OWNERS")
        .expect("Missing OWNERS")
        .split(',')
        .filter_map(|id| id.parse::<u64>().ok())
        .map(UserId::new)
        .collect();

    let intents = serenity::GatewayIntents::GUILD_MESSAGES
        | serenity::GatewayIntents::MESSAGE_CONTENT
        | serenity::GatewayIntents::GUILD_MEMBERS
        | serenity::GatewayIntents::GUILD_MESSAGE_REACTIONS
        | serenity::GatewayIntents::DIRECT_MESSAGES;

    let shard_manager_holder = Arc::new(tokio::sync::Mutex::new(None));
    let shard_manager_holder_clone = shard_manager_holder.clone();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: all_commands(),
            owners: owners,
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
                            poise::Context::Prefix(prefix_ctx) => {
                                if let Err(e) = prefix_ctx.msg.react(&prefix_ctx.serenity_context().http, '✅').await {
                                    eprintln!("Error sending message: {:?}", e);
                                }
                            }
                            poise::Context::Application(_) => {}
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
            let reminders = reminders.clone();
            let starboard = starboard.clone();
            let auth = auth.clone();
            let shard_manager_holder = shard_manager_holder_clone.clone();



            Box::pin(async move {
                poise::builtins::register_globally(_ctx, &_framework.options().commands).await?;
                let shard_manager = loop {
                    if let Some(sm) = shard_manager_holder.lock().await.clone() {
                        break sm;
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                };

                let data = Data {
                    shard_manager,
                    db: pool.clone(),
                    last_command_success: Arc::from(Mutex::new(true)),
                    reminders: reminders.clone(),
                    http_client: Arc::clone(&http_client),
                    starboard: starboard.clone(),
                    starboard_lock: Mutex::new(()),
                    auth: auth.clone(),
                    uptime: Instant::now(),
                    osu,
                };

                let task_data = Data {
                    shard_manager: data.shard_manager.clone(),
                    db: pool,
                    last_command_success: Arc::new(Default::default()),
                    reminders,
                    http_client,
                    starboard,
                    starboard_lock: Mutex::new(()),
                    auth,
                    uptime: Instant::now(),
                    osu: OsuClient::new( //lazy
                        client_id,
                        &client_oauth,
                    ).await?
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

    let shard_manager = client.shard_manager.clone();
    *shard_manager_holder.lock().await = Some(shard_manager);

    client.start().await?;

    Ok(())
}




