use crate::structs::auth::*;
use poise::serenity_prelude::Mentionable;
use poise::CreateReply;
use poise::serenity_prelude as serenity;
use sqlx::{Column, Row};

use super::moderation::BOT_OWNER_ID;

pub(crate) use crate::types::{Context, Data, Error};


pub fn all_commands() -> Vec<poise::Command<Data, Error>> {
    vec![
        exec(),
        authsetup(),
        authlist(),
        authstatus(),
        authdisable(),
        authremove(),
        authenticate(),
    ]
}

#[poise::command(slash_command, prefix_command)]
pub async fn exec(
    ctx: Context<'_>,
    #[rest]
    query: String,
) -> Result<(), Error> {
    if ctx.author().id != BOT_OWNER_ID {
        return Err(format!("Bot management command ran by unprivileged user {}", ctx.author().name).into());
    }
    if query.trim().is_empty() {
        return Err("Empty query string".into());
    }
    let query_type = query.split_whitespace().next().unwrap_or("").to_lowercase();

    match query_type.as_str() {
        "select" | "pragma" | "explain" => {
            match handle_select(&ctx, &query).await {
                Ok(response) => {
                    let file = serenity::CreateAttachment::bytes(response.into_bytes(), "result.txt");
                    ctx.send(CreateReply::default()
                                 .attachment(file)
                    ).await?;
                }
                Err(err) => {
                    ctx.say(format!("Exec failed: {}", err)).await?;
                    return Err(format!("Error handling select command: {}", err).into());
                }
            }
        }
        _ => {
            match handle_update(&ctx, &query).await {
                Ok(rows_affected) => {
                    ctx.send(CreateReply::default()
                        .content(format!("Query result: Rows affected {}", rows_affected))
                    .reply(true)).await?;
                }
                Err(err) => {
                    ctx.say(format!("Exec failed: {}", err)).await?;
                    return Err(format!("Error handling update command: {}", err).into());
                }
            }
        }
    }
    Ok(())
}

async fn handle_select(ctx: &Context<'_>, query: &str) -> Result<String, sqlx::Error> {
    use sqlx::sqlite::SqliteRow;

    let rows: Vec<SqliteRow> = sqlx::query(query)
        .fetch_all(&ctx.data().db)
        .await?;

    if rows.is_empty() {
        return Ok("No results".to_string());
    }

    let mut result = String::new();

    let columns: Vec<String> = rows[0].columns()
        .iter()
        .map(|c| c.name().to_string())
        .collect();

    let mut column_width: Vec<usize> = columns.iter().map(|c| c.len()).collect();

    for row in rows.iter().take(10) { // attempt to sample table elements for widths
        for (i, _) in columns.iter().enumerate() {
            let cell_str = match row.try_get::<String, _>(i) {
                Ok(s) => s,
                Err(_) => match row.try_get::<i64, _>(i) {
                    Ok(n) => n.to_string(),
                    Err(_) => "NULL".to_string(),
                },
            };
            column_width[i] = column_width[i].max(cell_str.len());
        }
    }

    for (i, col) in columns.iter().enumerate() {
        result.push_str(&format!("{:width$} | ", col, width=column_width[i]));
    }

    result.pop();
    result.pop();
    result.push('\n');

    for &width in &column_width {
        result.push_str(&"-".repeat(width+2));
        result.push('+');
    }
    result.pop();
    result.push('\n');

    for row in rows.iter().take(10) { // limit against chat flood
        for (i, _) in columns.iter().enumerate() {
            let cell_value = match row.try_get::<String, _>(i) {
                Ok(s) => s,
                Err(_) => match row.try_get::<i64, _>(i) {
                    Ok(n) => n.to_string(),
                    Err(_) => match row.try_get::<bool, _>(i) {
                        Ok(b) => b.to_string(),
                        Err(_) => "NULL".to_string(),
                    },
                },
            };
            result.push_str(&format!("{:width$} | ", cell_value, width = column_width[i]));
        }
        result.pop();
        result.pop();
        result.push('\n');
    }

    if (rows.len() > 10) {
        result.push_str(&format!("... {} more rows", rows.len()-10))
    }
    Ok((result))
}

async fn handle_update(ctx: &Context<'_>, query: &str) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(query)
        .execute(&ctx.data().db)
        .await?;

    Ok(result.rows_affected())
}



#[poise::command(prefix_command, slash_command, guild_only)]
async fn authenticate(
    ctx: Context<'_>,
    key_id: String,
) -> Result<(), Error> {
    let guild_id = match ctx.guild_id() {
        Some(id) => id,
        None => {
            return Err(format!("No guild ID found.").into());
        }
    };
    let author_id = ctx.author().id;
    let guild_id_i64 = guild_id.get() as i64;
    let user_id = ctx.author().id.get() as i64;

    if ctx.data().auth.is_user_authenticated(user_id, guild_id_i64).await? {
        let embed = serenity::CreateEmbed::default()
            .title("Already Authenticated")
            .description("You are already authenticated in this server")
            .color(0xFF0000);

        ctx.send(poise::CreateReply::default().embed(embed)).await?;
        return Ok(());
    }

    let config = match ctx.data().auth.get_config_by_key_id(&key_id).await? {
        Some(config) if config.guild_id == guild_id_i64 => config,
        Some(_) => {
            let embed = serenity::CreateEmbed::default()
                .title("Invalid Key")
                .description("This authentication key doesn't belong to this server")
                .color(0xFF0000);

            ctx.send(poise::CreateReply::default().embed(embed)).await?;
            return Ok(());
        }
        None => {
            let embed = serenity::CreateEmbed::default()
                .title("Invalid Key")
                .description("The provided authentication key is invalid")
                .color(0xFF0000);

            ctx.send(poise::CreateReply::default().embed(embed)).await?;
            return Ok(());
        }
    };

    if !config.enabled {
        let embed = serenity::CreateEmbed::default()
            .title("Authentication Disabled")
            .description("Authentication is currently disabled in this server.")
            .color(0xFF0000);

        ctx.send(poise::CreateReply::default().embed(embed)).await?;
        return Ok(());
    }


    let role_id = serenity::RoleId::new(config.role_id as u64);


    match guild_id.member(ctx.http(), author_id).await {
        Ok(mut member) => {
            if let Err(e) = member.add_role(ctx.http(), &role_id).await {
                    let embed = serenity::CreateEmbed::default()
                    .title("Failed to Add Role")
                    .description(format!("Could not add role: {}",
                    e))
                    .color(0xFF0000);

                    ctx.send(poise::CreateReply::default().embed(embed)).await?;
                    return Ok(());
            }
        }
        Err(e) => {
            // format error here later im so tired
            return Ok(())
            }
    }


    ctx.data().auth.add_authenticated_user(user_id, guild_id_i64).await?;

    let embed = serenity::CreateEmbed::default()
        .title("Authentication Successful")
        .description(format!("You have been given the {} role", role_id.mention()))
        .color(0x00FF00)
        .field("User", ctx.author().mention().to_string(), true)
        .field("Role", role_id.mention().to_string(), true);

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}

#[poise::command(prefix_command, slash_command, guild_only)]
async fn authsetup(
    ctx: Context<'_>,
    key_id: String,
    role: serenity::RoleId,
    enabled: Option<bool>,
) -> Result<(), Error> {
    if ctx.author().id != BOT_OWNER_ID {
        return Err(format!("Bot management command ran by unprivileged user {}", ctx.author().name).into());
    }
    let guild_id = ctx.guild_id().unwrap().get() as i64;
    let enabled = enabled.unwrap_or(true);

    ctx.data().auth.set_auth_config(
        guild_id,
        key_id.clone(),
        i64::from(role),
        enabled,
    ).await?;

    let embed = serenity::CreateEmbed::default()
        .title("Authentication Setup Complete")
        .color(0x00FF00)
        .field("Key ID", format!("`{}`", key_id), true)
        .field("Role", role.mention().to_string(), true)
        .field("Status", if enabled { "✅ Enabled" } else { "❌ Disabled" }, true)
        .field("Usage", format!("`{}authenticate [key-id]`", ctx.prefix()), false);

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}

#[poise::command(prefix_command, slash_command, guild_only)]
async fn authstatus(
    ctx: Context<'_>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().get() as i64;

    let config = ctx.data().auth.get_auth_config(guild_id).await?;

    let embed = serenity::CreateEmbed::default()
        .title("Authentication Configuration")
        .color(0x5865F2);

    let embed = match config {
        Some(config) => {
            let user_count = ctx.data().auth.get_authenticated_users(guild_id).await?;
            let user_count: Vec<AuthenticatedUser> = user_count;

            let user_count = user_count.len();

            embed
                .field("Enabled", if config.enabled { "✅" } else { "❌" }, true)
                .field("Key ID", format!("`{}`", config.key_id), true)
                .field("Role", format!("<@&{}>", config.role_id), true)
                .field("Authenticated Users", user_count.to_string(), true)
        },
        None => embed
            .description("No authentication configuration found for this server.")
            .color(0xFF0000),
    };

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}

#[poise::command(prefix_command, slash_command, guild_only)]
async fn authdisable(
    ctx: Context<'_>,
) -> Result<(), Error> {
    if ctx.author().id != BOT_OWNER_ID {
        return Err(format!("Bot management command ran by unprivileged user {}", ctx.author().name).into());
    }
    let guild_id = ctx.guild_id().unwrap().get() as i64;

    if let Some(config) = ctx.data().auth.delete_auth_config(guild_id).await? {
        let embed = serenity::CreateEmbed::default()
            .title("Authentication Disabled")
            .description("Authentication has been disabled for this server.")
            .color(0xFFA500)
            .field("Key ID", format!("`{}`", config.key_id), true)
            .field("Role", format!("<@&{}>", config.role_id), true);

        ctx.send(poise::CreateReply::default().embed(embed)).await?;
    } else {
        let embed = serenity::CreateEmbed::default()
            .title("Not Configured")
            .description("This server doesn't have authentication configured.")
            .color(0xFF0000);

        ctx.send(poise::CreateReply::default().embed(embed)).await?;
    }

    Ok(())
}

#[poise::command(prefix_command, slash_command, guild_only)]
async fn authremove(
    ctx: Context<'_>,
    #[description = "The user to remove authentication from"] user: serenity::User,
) -> Result<(), Error> {
    if ctx.author().id != BOT_OWNER_ID {
        return Err(format!("Bot management command ran by unprivileged user {}", ctx.author().name).into());
    }
    let guild_id = match ctx.guild_id() {
        Some(id) => id,
        None => {
            return Err(format!("No guild ID found.").into());
        }
    };
    let guild_id_i64 = guild_id.get() as i64;
    let user_id = user.id.get() as i64;

    if !ctx.data().auth.is_user_authenticated(user_id, guild_id_i64).await? {
        let embed = serenity::CreateEmbed::default()
            .title("Not Authenticated")
            .description(format!("{} is not authenticated in this server.", user.mention()))
            .color(0xFF0000);

        ctx.send(CreateReply::default().embed(embed)).await?;
        return Ok(());
    }

    let config = match ctx.data().auth.get_auth_config(guild_id_i64).await? {
        Some(config) => config,
        None => {
            let embed = serenity::CreateEmbed::default()
                .title("Not Configured")
                .description("This server doesn't have authentication configured.")
                .color(0xFF0000);

            ctx.send(CreateReply::default().embed(embed)).await?;
            return Ok(());
        }
    };

    match ctx.http().get_guild(guild_id).await {
        Ok(guild) => {
            if let Some(role) = guild.roles.get(&serenity::RoleId::new(config.role_id as u64)) {
                match guild_id.member(ctx.http(), user.id).await {
                    Ok(mut member) => {
                        if let Err(e) = member.remove_role(ctx.http(), role).await {
                            eprintln!("Failed to remove role: {}", e);
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to get member: {}", e);
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to fetch guild: {}", e);
        }
    }

    ctx.data().auth.remove_authenticated_user(user_id, guild_id_i64).await?;

    let embed = serenity::CreateEmbed::default()
        .title("Authentication Removed")
        .description(format!("Removed authentication from {}", user.mention()))
        .color(0x00FF00);

    ctx.send(CreateReply::default().embed(embed)).await?;
    Ok(())
}

#[poise::command(prefix_command, slash_command, guild_only)]
async fn authlist(
    ctx: Context<'_>,
) -> Result<(), Error> {
    if ctx.author().id != BOT_OWNER_ID {
        return Err(format!("Bot management command ran by unprivileged user {}", ctx.author().name).into());
    }

    let guild_id = ctx.guild_id().unwrap().get() as i64;

    let users: Vec<AuthenticatedUser> = ctx.data().auth.get_authenticated_users(guild_id).await?;

    if users.is_empty() {
        let embed = serenity::CreateEmbed::default()
            .title("Authenticated Users")
            .description("No users have been authenticated yet.")
            .color(0xFFA500);

        ctx.send(poise::CreateReply::default().embed(embed)).await?;
        return Ok(());
    }

    let user_list: String = users
        .iter()
        .take(20)
        .enumerate()
        .map(|(i, user)| {
            let timestamp = user.authenticated_at.format("%Y-%m-%d %H:%M:%S UTC");
            format!("{}. <@{}> - `{}`", i + 1, user.user_id, timestamp)
        })
        .collect::<Vec<_>>()
        .join("\n");

    let total_users = users.len();
    let embed = serenity::CreateEmbed::default()
        .title("Authenticated Users")
        .description(user_list)
        .color(0x5865F2)
        .footer(serenity::CreateEmbedFooter::new(format!(
            "Total: {} users | Showing first {}",
            total_users,
            users.len().min(20)
        )));

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}