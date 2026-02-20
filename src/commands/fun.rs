pub(crate) use crate::types::{Context, Data, Error};
use poise::serenity_prelude as serenity;
use serenity::all::Mentionable;
use crate::commands::general::{choose, remind, say};
use crate::helpers::role_colours::{is_feature_enabled, set_feature_enabled};

use crate::commands::moderation::{mod_check};

#[poise::command(prefix_command, slash_command, guild_only)]
async fn sbstatus(
    ctx: Context<'_>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().get();

    let config = ctx.data().starboard.get_starboard_config(guild_id).await?;

    let embed = serenity::CreateEmbed::default()
        .title("Starboard config:")
        .color(0x5865F2);

    let embed = match config {
        Some(config) => embed
            .field("Enabled", if config.enabled {"✅"} else { "❌"}, true)
            .field("Channel",
                   config.starboard_channel_id
                       .and_then(|id| id.parse::<u64>().ok())
                       .map(|id| format!("<#{}>", id))
                   .unwrap_or_else(|| "None".to_string()),
                   true)
            .field("Threshold", config.threshold.to_string(), true)
            .field("Self Star", if config.self_star_allowed { "Allowed"} else {"Not Allowed"},
            true),
        None => embed.description("No starboard config provided"),
    };

    let reply = poise::CreateReply::default()
        .embed(embed);

    ctx.send(reply).await?;
    Ok(())

}

#[poise::command(prefix_command, check = "mod_check")]
pub async fn rolecolours(
    ctx: Context<'_>,
    enabled: bool,
) -> Result<(), Error> {
    let guild_id = match ctx.guild_id() {
        Some(guild_id) => guild_id,
        None => return Ok(()),
    };

    let guild_id_u64 = guild_id.get();

    set_feature_enabled(&ctx.data().db, guild_id_u64, enabled).await?;
    Ok(())
}

#[poise::command(prefix_command, slash_command, guild_only)]
pub async fn rcstatus(
    ctx: Context<'_>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let is_enabled = is_feature_enabled(&ctx.data().db, guild_id.get()).await?;
    let embed = serenity::CreateEmbed::default()
        .title("Role Colour Configuration")
        .color(0x800080)
        .field(
            "Feature Status",
            if is_enabled { "**Enabled**" } else { "**Disabled**" },
            false
        )
        .footer(serenity::CreateEmbedFooter::new(format!("Server ID: {}", guild_id)));
    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}

#[poise::command(slash_command, prefix_command)] // why does it want more than one arg??? lol??
pub async fn roleset(
    ctx: Context<'_>,
    colour: String,
) -> Result<(), Error> {

    let guild_id = ctx.guild_id().ok_or(Error::from("guild not found"))?;

    if !is_feature_enabled(&ctx.data().db, guild_id.get()).await? {
        return Err(format!("Feature not enabled").into());
    }


    let clean_colour = colour.trim_start_matches('#');
    let colour_u32 = u32::from_str_radix(clean_colour, 16)
        .map_err(|_| format!("Invalid hex colour: {}", colour))?;

    let role_name = format!("role-{}", ctx.author().id);
    let user_id = ctx.author().id;

    let http = ctx.http().clone();

    let (all_roles, existing_role_info) = {
        let partial_guild = guild_id.to_partial_guild(http).await
            .map_err(|_| "discord api is poop")?;

        let role_name = format!("role-{}", ctx.author().id);

        let roles: Vec<(serenity::RoleId, String)> = partial_guild.roles
            .iter()
            .map(|(id, role)| (*id, role.name.clone()))
            .collect();

        let existing = roles
            .iter()
            .find(|(_, name)| name == &role_name)
            .map(|(id, name)| (*id, name.clone()));

        (roles, existing)
    };

    let role_id: serenity::RoleId = if let Some((role_id, role)) = existing_role_info {
        let builder = serenity::EditRole::new()
            .colour(colour_u32)
            .name(&role);

        guild_id.edit_role(&http, role_id, builder).await?;
        role_id
    } else {
        let builder = serenity::EditRole::new()
            .name(&role_name)
            .colour(colour_u32)
            .hoist(false)
            .mentionable(false);

        let new_role = guild_id.create_role(&http, builder).await?;
        new_role.id
    };

    let member = guild_id.member(&http, user_id).await?;
    member.add_role(&http, role_id).await?;

    let current_roles = member.roles.clone();

    let roles_to_cleanup: Vec<serenity::RoleId> = all_roles
        .iter()
        .filter(|(role_id, role_name_val)| {
            role_name_val.starts_with("role-") &&
                role_name_val != &role_name &&
                current_roles.contains(role_id)
        })
        .map(|(id, _)| *id)
        .collect();

    for old_role_id in roles_to_cleanup {
        let _ = member.remove_role(&http, old_role_id).await;
    }

    let role_mention = serenity::RoleId::new(role_id.get()).mention();
    let user_mention = ctx.author().mention();

    let embed = serenity::CreateEmbed::default()
        .title("Role colour added")
        .color(colour_u32)
        .field("User", user_mention.to_string(), true)
        .field("Role", role_mention.to_string(), true);

    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())


}


pub fn all_commands() -> Vec<poise::Command<Data, Error>> {
    vec![
        sbstatus(),
        rolecolours(),
        roleset(),
        rcstatus(),
        // add more here
    ]
}