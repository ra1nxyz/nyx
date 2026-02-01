use poise::serenity_prelude as serenity;
use sqlx::SqlitePool;
use chrono::{Utc, Duration};
use serenity::framework::standard::macros::group;
use serenity::prelude::TypeMapKey;

use crate::time_parse::{ParsedDuration, TimeParseError};
pub(crate) use crate::types::{Context, Data, Error};

pub(crate) const BOT_OWNER_ID: serenity::UserId = serenity::UserId::new(1434739350993768630);

// refactor later to run query and cache instead of multiple queries, yayayaya
pub async fn is_moderator(ctx: &Context<'_>) -> bool {
    let author_id = ctx.author().id;
    if author_id == BOT_OWNER_ID {
        return true;
    }

    let is_user_elevated = sqlx::query("SELECT 1 FROM moderator_users WHERE user_id = ?")
        .bind(author_id.to_string())
        .fetch_optional(&ctx.data().db)
        .await
        .map(|row| row.is_some())
        .unwrap_or(false);

    if is_user_elevated {
        return true;
    }

    let guild_id = match ctx.guild_id() {
        Some(gid) => gid,
        None => return false,
    };

    let member = match guild_id.member(&ctx.http(), author_id).await {
        Ok(m) => m,
        Err(_) => return false,
    };

    for role_id in &member.roles {
        let is_privilged_role = sqlx::query("SELECT role_id FROM moderator_roles WHERE role_id = ?")
        .bind(role_id.to_string())
            .fetch_optional(&ctx.data().db)
        .await
        .map(|row| row.is_some())
        .unwrap_or(false);

        if is_privilged_role {
            return true;
        }
    }
    return false;
}


pub async fn mod_check(ctx: poise::Context<'_, Data, Error>) -> Result<bool, Error> {
    if is_moderator(&ctx).await {
        return Ok(true);
    }


    //match ctx {
    //    poise::Context::Prefix(ctx) => {
    //        ctx.msg.react(&ctx.serenity_context().http, '❌').await?;
    //    }
    //    _ => return Ok(false),
    //}

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

#[poise::command(slash_command, prefix_command, check = "mod_check")]
pub async fn kick(
    ctx: Context<'_>,
    user: serenity::User,
) -> Result<(), Error> {

    let guild_id = ctx.guild_id().unwrap();
    guild_id
        .kick(&ctx.http(), user.id)
        .await?;

    Ok(())
}


#[poise::command(slash_command, prefix_command, check = "mod_check")]
pub async fn ban(
    ctx: Context<'_>,
    user: serenity::User,
) -> Result<(), Error> {

    let guild_id = ctx.guild_id().unwrap();
    guild_id.ban(&ctx.http(), user.id, 7).await?;
    Ok(())
}

#[poise::command(slash_command, prefix_command, check = "mod_check")]
pub async fn timeout(
    ctx: crate::Context<'_>,
    user: serenity::User,
    duration_str: String,
) -> Result<(), crate::Error> {
    let parsed_duration = match ParsedDuration::new(&duration_str) {
        Ok(d) => d,
        Err(e) => {
            ctx.say(format!("Duration parse error: {}", e)).await?;
            return Err(format!("Error parsing duration: {}", e).into());
        }
    };
    const MAX_DURATION: i64 = 28;

    if parsed_duration.duration.num_days() > MAX_DURATION {
        ctx.say(format!("Timeout duration is over discord maximum ({} days)", MAX_DURATION)).await?;
        return Err(format!("Maximum timeout duration exceeded in command ({})", MAX_DURATION).into());
    }
    let guild = ctx.guild_id().unwrap();
    let mut member = guild.member(&ctx.http(), user.id).await?;

    let until = parsed_duration.until_datetime();

    member
        .edit(ctx.http(), serenity::EditMember::new()
        .disable_communication_until(until.to_rfc3339()))
        .await?;

    Ok(())
}

#[poise::command(slash_command, prefix_command, check = "mod_check")]
pub async fn warn(
    ctx: Context<'_>,
    user: serenity::User,
    severity: u8,
    reason: String,
) -> Result<(), Error> {

    if severity < 1 || severity > 5 {
        ctx.say("Severity must be between **1 and 5**.")
            .await?;
        return Err(format!("Error in severity: {}", severity).into());

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
    Ok(())
}

#[poise::command(slash_command, prefix_command, check = "mod_check")]
pub async fn unban(
    ctx: Context<'_>,
    user_id: u64,
) -> Result<(), Error> {

    let guild_id = ctx.guild_id().unwrap();
    guild_id
        .unban(&ctx.http(), serenity::UserId::new(user_id))
        .await?;
    Ok(())
}



pub struct Moderation;