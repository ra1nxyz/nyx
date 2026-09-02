use poise::serenity_prelude as serenity;
use chrono::{DateTime, Utc};
use poise::CreateReply;
pub(crate) use crate::types::{Context, Data, Error};

use rand::seq::{IndexedRandom, SliceRandom};
use rand::rng;
use serenity::{};
use serenity::all::shard_id;
use sysinfo::{RefreshKind, System};
use crate::commands::moderation::mod_check;
use crate::helpers::role_colours::is_feature_enabled;
use crate::structs::time_parse::{ParsedWhen};

pub fn all_commands() -> Vec<poise::Command<Data, Error>> {
    vec![
        say(),
        choose(),
        remind_prefix(),
        remind_slash(),
        avatar(),
        banner(),
        whois()
    ]
}

#[poise::command(slash_command, prefix_command, check = "mod_check")]
pub async fn say(
    ctx: Context<'_>,
    #[rest]
    text: String, )
-> Result<(), Error> {
    ctx.say(text).await?;
    Ok(())
}

#[poise::command(slash_command, prefix_command,)]
pub async fn choose(
    ctx: Context<'_>,
    #[rest]
    options: String,
) -> Result<(), Error> {
    let all_options: Vec<&str> = options.split(',').map(|s| s.trim()).collect();
    println!("{:?}", all_options);

    if all_options.len() < 2 {
            ctx.say("Minimum of 2 options required").await?;
            return Err(format!("Less than required arguments were given for command").into());
    }
    let choice = all_options.choose(&mut rng()).unwrap();
    ctx.say(format!("{}", choice)).await?;
    Ok(())
}

#[poise::command(slash_command, prefix_command, aliases("av", "pfp"))]
pub async fn avatar(
    ctx: Context<'_>,
    user: serenity::User,
) -> Result<(), Error> {
    if let Some(avatar_url) = user.avatar_url() {
        ctx.say(avatar_url).await?;
    }

    Ok(())
}

#[poise::command(slash_command, prefix_command)]
pub async fn banner(
    ctx: Context<'_>,
    user: serenity::User,
) -> Result<(), Error> {
    if let Some(banner_url) = user.banner_url() {
        ctx.say(banner_url).await?;
    }
    Ok(())
}


async fn remind_impl(
    ctx: Context<'_>,
    when: String,
    message: Option<String>,
) -> Result<(), Error> {
    let parsed = ParsedWhen::new(&when)
        .map_err(|e| format!("Could not parse when: {}", e))?;

    let remind_at = parsed
        .until_datetime()
        .map_err(|e| format!("Could not parse when: {}", e))?;

    let message = message.unwrap_or_else(|| "Reminder".to_string());

    let context = match ctx {
        poise::Context::Prefix(ctx) => ctx.msg.link(),
        _ => String::new(),
    };

    let remind = crate::structs::reminders::Reminder::new(
        ctx.author().id.to_string(),
        remind_at,
        message,
        Some(context),
    );

    let reminder_id = ctx.data().reminders.add_reminder(&remind).await?;

    ctx.send(
        CreateReply::default()
            .content(format!(
                "Reminder ID #{} set for <t:{}:F>",
                reminder_id,
                remind_at.timestamp()
            ))
            .reply(true),
    )
        .await?;

    Ok(())
}

#[poise::command(prefix_command, dm_only = false, rename = "remind")]
pub async fn remind_prefix(
    ctx: Context<'_>,
    when: String,
    #[rest]
    message: Option<String>,
) -> Result<(), Error> {
    remind_impl(ctx, when, message).await
}

#[poise::command(slash_command, dm_only = false, rename = "remind")]
pub async fn remind_slash(
    ctx: Context<'_>,
    when: String,
    message: Option<String>,
) -> Result<(), Error> {
    remind_impl(ctx, when, message).await
}

#[poise::command(slash_command, prefix_command)]
pub async fn whois(
    ctx: Context<'_>,
) -> Result<(), Error> {
    let version: &str = env!("CARGO_PKG_VERSION");

    let uptime_forward = ctx.data().uptime.elapsed();
    let total_secs = uptime_forward.as_secs();

    let days = total_secs / 86400;
    let hours = (total_secs % 86400) / 3600;
    let mins = (total_secs % 3600) / 60;
    let secs = total_secs % 60;

    let uptime = format!("{:02}d {:02}h {:02}m {:02}s", days, hours, mins, secs);

    let data = ctx.data();
    let runners = data.shard_manager.runners.lock().await;
    let shard_id = ctx.serenity_context().shard_id;

    let latency = runners
        .get(&shard_id)
        .and_then(|runner| runner.latency)
        .map(|d| format!("{}ms", d.as_millis()))
        .unwrap_or_else(|| "N/A".to_string());

    let mut sys = System::new_with_specifics(RefreshKind::everything());
    sys.refresh_all();

    let os = System::long_os_version().unwrap_or_else(|| "Unknown OS".to_string());
    let hostname = System::host_name().unwrap_or_else(|| "Unknown host".to_string());

    let host = format!("{os}+{hostname}");

    let embed = serenity::CreateEmbed::default()
        .title("Instance Info")
        .color(0x800080)
        .thumbnail("https://image.buggirls.xyz/bobrswmTaHAo.webp")
        .field("Version: ", format!("Nyx v{}", version), true)
        .field("Uptime: ", uptime, true)
        .field("Latency on shard: ", latency, true)
        .field("Hosted on: ", host, true)
        .footer(serenity::CreateEmbedFooter::new(format!("Shard ID: {}", shard_id)));
    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}

/*
do later i cba
#[poise::command(slash_command, prefix_command)]
pub async fn reminders(
    ctx: Context<'_>,
) -> Result<(), Error> {
    let user_id = ctx.author().id.to_string();
    let reminders = ctx.data().reminders.
}

*/