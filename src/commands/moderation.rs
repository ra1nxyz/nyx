use poise::serenity_prelude as serenity;
use sqlx::SqlitePool;
use chrono::{Utc, Duration};
use serenity::framework::standard::macros::group;
use serenity::prelude::TypeMapKey;



pub struct BotData;
pub(crate) use crate::types::{Context, Data, Error};



const BOT_OWNER_ID: u64 = 1434739350993768630;
const MODERATOR_ROLE_ID: u64 = 1308891968004292618;

impl TypeMapKey for BotData {
    type Value = SqlitePool;
}


pub async fn is_moderator(ctx: &Context<'_>) -> bool {
    let author_id = ctx.author().id;
    if author_id == BOT_OWNER_ID {
        return true;
    }

    let guild_id = match ctx.guild_id() {
        Some(g) => g,
        None => return false,
    };

    let member = match guild_id.member(&ctx.http(), author_id).await {
        Ok(m) => m,
        Err(_) => return false,
    };

    member.roles.contains(&serenity::RoleId::from(MODERATOR_ROLE_ID))
}

async fn mod_check(ctx: Context<'_>) -> Result<bool, Error> {
    if is_moderator(&ctx).await {
        return Ok(true);
    }


    if let poise::Context::Prefix(prefix_ctx) = ctx {
        prefix_ctx.msg.react(ctx.http(), '❌').await?;
    }

    Ok(false)

}
pub fn all_commands() -> Vec<poise::Command<Data, Error>> {
    vec![
        kick(),
        ban(),
        unban(),
        timeout(),
        warn(),
        // add more here
    ]
}

#[poise::command(slash_command, prefix_command)]
pub async fn kick(
    ctx: Context<'_>,
    user: serenity::User,
) -> Result<(), Error> {

    let guild_id = ctx.guild_id().unwrap();
    guild_id
        .kick(&ctx.http(), user.id)
        .await?;

    ctx.say(format!("User {} has been kicked.", user.name)).await?;
    Ok(())
}


#[poise::command(slash_command, prefix_command)]
pub async fn ban(
    ctx: Context<'_>,
    user: serenity::User,
) -> Result<(), Error> {

    let guild_id = ctx.guild_id().unwrap();
    guild_id.ban(&ctx.http(), user.id, 7).await?;
    ctx.say(format!("User {} has been banned.", user.name)).await?;
    Ok(())
}

#[poise::command(slash_command, prefix_command)]
pub async fn timeout(
    ctx: Context<'_>,
    user: serenity::User,
    minutes: u64,
) -> Result<(), Error> {
    let guild = ctx.guild_id().unwrap();

    let mut member = guild.member(ctx.http(), user.id).await?;

    let until = Utc::now() + Duration::minutes(minutes as i64);

    member
        .edit(ctx.http(), serenity::EditMember::new().disable_communication_until(until.to_rfc3339()))
        .await?;

    ctx.say(format!(
        "User {} has been timed out for {} minutes.",
        user.name, minutes
    ))
        .await?;
    Ok(())
}

#[poise::command(slash_command, prefix_command)]
pub async fn warn(
    ctx: Context<'_>,
    user: serenity::User,
    severity: u8,
    reason: String,
) -> Result<(), Error> {
    if severity < 1 || severity > 5 {
        ctx.say("Severity must be between **1 and 5**.")
            .await?;
        return Ok(());
    }

    let db = &ctx.data().db;

    sqlx::query(
        "INSERT INTO warnings (user_id, severity, reason)
         VALUES (?, ?, ?)",
    )
        .bind(user.id.get() as i64)
        .bind(severity as i64)
        .bind(reason)
        .execute(db)
        .await?;

    ctx.say(format!(
        "Warned user {} with severity {}",
        user.name, severity
    ))
        .await?;
    Ok(())
}

#[poise::command(slash_command, prefix_command)]
pub async fn unban(
    ctx: Context<'_>,
    user_id: u64,
) -> Result<(), Error> {

    let guild_id = ctx.guild_id().unwrap();
    guild_id
        .unban(&ctx.http(), serenity::UserId::new(user_id))
        .await?;

    ctx.say(format!("User {} has been unbanned.", user_id)).await?;
    Ok(())
}



pub struct Moderation;