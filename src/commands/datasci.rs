use poise::serenity_prelude as serenity;
use serenity::all::{Colour, CreateEmbed, CreateEmbedFooter};
use crate::types::{Context, Data, Error};
use serenity::UserId;
use tracing::info;

pub fn all_commands() -> Vec<poise::Command<Data, Error>> {
    vec![
        reputation(),
        topinfluential(),
        flagtoxic(), // manual thing is stupid but i needed an example, use something to estimate based on msgs later
        influencegraph(),
        repostats(),
        forcerecalc(),
    ]
}

#[poise::command(slash_command, prefix_command)]
pub async fn reputation(
    ctx: Context<'_>,
    user: Option<serenity::User>,
) -> Result<(), Error> {
    let target_user = user.as_ref().unwrap_or_else(|| ctx.author());
    let guild_id = match ctx.guild_id() {
        Some(id) => id,
        None => {
            ctx.say("This command can only be used in servers").await?;
            return Ok(());
        }
    };

    let reputation = ctx.data().reputation.get_user_reputation(target_user.id, guild_id).await?;

    match reputation {
        Some(rep) => {
            let embed = CreateEmbed::default()
                .title(format!("Reputation Profile"))
                .thumbnail(target_user.face())
                .color(0x12b0dc)
                .field("User", format!("<@{}>", target_user.id), true)
                .field("Reputation Score", format!("{:.2}", rep.reputation_score), true)
                .field("Influence Score", format!("{:.2}", rep.influence_score), true)
                .field("Stats", format!(
                    "**Total Interactions:** {}\n**Unique Interactors:** {}",
                    rep.total_interactions, rep.unique_interactors
                ), false)
                .field("Toxicity", format!("{:.2}%", rep.toxicity_score * 100.0), true)
                .footer(CreateEmbedFooter::new(format!("User ID: {}", target_user.id)))
                .timestamp(chrono::Utc::now());

            ctx.send(poise::CreateReply::default().embed(embed)).await?;
        }
        None => {
            let embed = CreateEmbed::default()
                .title("No Reputation Data")
                .description(format!("<@{}> has no reputation data yet.", target_user.id))
                .color(Colour::from_rgb(255, 160, 0))
                .thumbnail(target_user.face());

            ctx.send(poise::CreateReply::default().embed(embed)).await?;
        }
    }

    Ok(())
}

#[poise::command(slash_command, prefix_command)]
pub async fn topinfluential(
    ctx: Context<'_>,
    limit: Option<usize>,
) -> Result<(), Error> {
    let limit = limit.unwrap_or(10).min(25);
    let guild_id = match ctx.guild_id() {
        Some(id) => id,
        None => {
            ctx.say("This command can only be used in servers").await?;
            return Ok(());
        }
    };

    let top_users = ctx.data().reputation.get_top_influential(guild_id, limit).await?;

    if top_users.is_empty() {
        let embed = CreateEmbed::default()
            .title("Top Influential Users")
            .description("No reputation data available yet.")
            .color(Colour::from_rgb(255, 165, 0));

        ctx.send(poise::CreateReply::default().embed(embed)).await?;
        return Ok(());
    }

    let mut description = String::new();

    for (i, user) in top_users.iter().enumerate() {
        let user_id = UserId::new(user.user_id.parse().unwrap());
        let mention = format!("<@{}>", user_id);

        description.push_str(&format!(
            "{}\n└ Influence: {} \n",
            mention,
            user.influence_score,
        ));

        description.push_str(&format!(
            "└ Reputation: {}\n\n",
            user.reputation_score,
        ))

    }

    let guild = ctx.guild();
    let thumbnail = guild.and_then(|g| g.icon_url()).unwrap_or_default();

    let embed = CreateEmbed::default()
        .title("Top Influential Users")
        .description(description)
        .color(0x5865F2)
        .thumbnail(thumbnail)
        .footer(CreateEmbedFooter::new(format!(
            "Total users ranked • Showing top {} • Updated periodically",
            top_users.len()
        )))
        .timestamp(chrono::Utc::now());

    ctx.send(poise::CreateReply::default().embed(embed)).await?;
    Ok(())
}

#[poise::command(slash_command, prefix_command, required_permissions = "MODERATE_MEMBERS")]
pub async fn flagtoxic(
    ctx: Context<'_>,
    user: serenity::User,
    reason: String,
    severity: Option<i64>,
) -> Result<(), Error> {
    let severity = severity.unwrap_or(3).clamp(1, 5);
    let guild_id = ctx.guild_id().unwrap();

    ctx.data().reputation.flag_toxicity(
        guild_id,
        user.id,
        ctx.author().id,
        reason,
        None,
        severity,
    ).await?;

    ctx.say(format!("Flagged {} for toxic behavior (Severity: {}/5)", user.name, severity)).await?;
    Ok(())
}

#[poise::command(slash_command, prefix_command, owners_only)]
pub async fn influencegraph(
    ctx: Context<'_>,
    min_weight: Option<f64>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();

    let cache = &ctx.data().cache;
    let graph = ctx.data().reputation
        .get_influence_graph(guild_id, cache, min_weight, Some(50))
        .await?;

    let response = format!(
        "Influence Graph Data:\n\
        Nodes: {}\n\
        Edges: {}\n\
        \n*use some graphing thing for this later*",
        graph.nodes.len(),
        graph.edges.len()
    );

    ctx.say(response).await?;

    // let json = serde_json::to_string_pretty(&graph)?;
    // ctx.say(format!("```json\n{}\n```", &json[..1900])).await?;
    // json? maybe? idk

    Ok(())
}

#[poise::command(slash_command, prefix_command, required_permissions = "ADMINISTRATOR")]
pub async fn forcerecalc(
    ctx: Context<'_>,
) -> Result<(), Error> {
    info!("test");
    let guild_id = match ctx.guild_id() {
        Some(id) => id,
        None => {
            ctx.say("This command can only be used in servers").await?;
            return Ok(());
        }
    };

    ctx.defer().await?;

    info!("Manual reputation recalculation triggered for guild {}", guild_id);

    match ctx.data().reputation.calculate_reputation(guild_id).await {
        Ok(_) => { return Ok(()) }
        Err(e) => { return Err(e.into()); }
    }
}

#[poise::command(slash_command, prefix_command)]
pub async fn repostats(
    ctx: Context<'_>,
) -> Result<(), Error> {
    let guild_id = match ctx.guild_id() {
        Some(id) => id,
        None => {
            ctx.say("This command can only be used in servers").await?;
            return Ok(());
        }
    };

    let stats = ctx.data().reputation.get_interaction_stats(guild_id).await?;

    let response = format!(
        "**Interaction Statistics**\n```json\n{}\n```",
        serde_json::to_string_pretty(&stats).unwrap_or_default()
    );

    let response = if response.len() > 1900 {
        format!("{}...", &response[..1900])
    } else {
        response
    };

    ctx.say(response).await?;

    Ok(())
}

