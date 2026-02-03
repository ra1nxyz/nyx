pub(crate) use crate::types::{Context, Data, Error};
use poise::serenity_prelude as serenity;
use crate::commands::general::{choose, remind, say};

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


pub fn all_commands() -> Vec<poise::Command<Data, Error>> {
    vec![
        sbstatus(),
        // add more here
    ]
}