use crate::helpers::starboard::{Database, StarboardConfig, StarredMessage};
use poise::serenity_prelude as serenity;

pub(crate) async fn handle_reaction_add(
    ctx: &serenity::Context,
    add_reaction: &serenity::Reaction,
    data: &crate::Data,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let guild_id = match add_reaction.guild_id {
        Some(id) => id.get() ,
        None => return Ok(()),
    };


    let config = match data.starboard.get_starboard_config(guild_id.into()).await? {
        Some(config) => config,
        None => return Ok(()),
    };

    if !config.enabled {
        return Ok(());
    }

    let emoji_string = add_reaction.emoji.to_string();
    if emoji_string != config.star_emoji {
        return Ok(());
    }

    let message_id = add_reaction.message_id.into();
    let user_id = add_reaction.user_id.unwrap().into();


    let message = add_reaction.message(&ctx.http).await?;
    if message.author.id.get() == user_id && !config.self_star_allowed {
        add_reaction.delete(&ctx.http).await?;
        return Ok(());
    }

    data.starboard.add_star_reaction(message_id, user_id).await?;

    let star_count = data.starboard.count_star_reactions(message_id).await?;

    if star_count >= config.threshold {
        let _lock = data.starboard_lock.lock().await;
        let star_count = data.starboard.count_star_reactions(message_id).await?;
        if star_count >= config.threshold {
            update_starboard_message(ctx, &data.starboard, &config, &message, guild_id, star_count).await?;
        }
    }

    Ok(())
}

pub(crate) async fn handle_reaction_remove(
    ctx: &serenity::Context,
    removed_reaction: &serenity::Reaction,
    data: &crate::Data,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let guild_id = match removed_reaction.guild_id {
        Some(id) => id,
        None => return Ok(()),
    };

    let config = match data.starboard.get_starboard_config(guild_id.into()).await? {
        Some(config) => config,
        None => return Ok(()),
    };

    if !config.enabled {
        return Ok(());
    }

    let emoji_string = removed_reaction.emoji.to_string();
    if emoji_string != config.star_emoji {
        return Ok(());
    }

    let message_id = removed_reaction.message_id.into();
    let user_id = removed_reaction.user_id.unwrap().into();

    data.starboard.remove_star_reaction(message_id, user_id).await?;

    let star_count = data.starboard.count_star_reactions(message_id).await?;

    let starred_message = data.starboard.get_starred_message(message_id).await?;

    match starred_message {
        Some(mut starred) => {
            if star_count >= config.threshold {
                starred.stars = star_count;
                data.starboard.update_starred_message(&starred).await?;

                if let (Some(starboard_channel_id), Some(starboard_message_id)) =
                    (&starred.starboard_channel_id, &starred.starboard_message_id)
                {
                    let starboard_channel = starboard_channel_id.parse::<u64>()?;
                    let starboard_message_id = starboard_message_id.parse::<u64>()?;

                    update_existing_starboard_message(
                        ctx,
                        starboard_channel,
                        starboard_message_id,
                        star_count,
                        &config.star_emoji,
                    ).await?;
                }
            } else {
                // If below threshold, delete from starboard and database
                if let (Some(starboard_channel_id), Some(starboard_message_id)) =
                    (&starred.starboard_channel_id, &starred.starboard_message_id)
                {
                    let starboard_channel = starboard_channel_id.parse::<u64>()?;
                    let starboard_message_id = starboard_message_id.parse::<u64>()?;

                    delete_starboard_message(
                        ctx,
                        starboard_channel,
                        starboard_message_id,
                    ).await?;
                }

                data.starboard.delete_starred_message(message_id).await?;
            }
        }
        None => {
        }
    }

    Ok(())
}

pub(crate) async fn handle_reaction_remove_all(
    ctx: &serenity::Context,
    channel_id: serenity::ChannelId,
    message_id: serenity::MessageId,
    data: &crate::Data,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let starred_message = data.starboard.get_starred_message(message_id.into()).await?;

    if let Some(starred) = starred_message {
        if let (Some(starboard_channel_id), Some(starboard_message_id)) =
            (&starred.starboard_channel_id, &starred.starboard_message_id)
        {
            let starboard_channel = starboard_channel_id.parse::<u64>()?;
            let starboard_message_id = starboard_message_id.parse::<u64>()?;

            delete_starboard_message(
                ctx,
                starboard_channel,
                starboard_message_id,
            ).await?;
        }

        data.starboard.delete_starred_message(message_id.into()).await?;
    }

    Ok(())
}

async fn update_starboard_message(
    ctx: &serenity::Context,
    starboard: &Database,
    config: &StarboardConfig,
    original_message: &serenity::Message,
    guild_id: u64,
    star_count: i64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let starboard_channel_id = match &config.starboard_channel_id {
        Some(id) => id.parse::<u64>()?,
        None => return Ok(()),
    };

    let starboard_channel = serenity::ChannelId::new(starboard_channel_id);
    let original_message_id: u64 = original_message.id.into();

    let existing = starboard.get_starred_message(original_message_id).await?;


    match existing {
        Some(mut starred_message) => {
            starred_message.stars = star_count;
            starboard.update_starred_message(&starred_message).await?;

            if let Some(starboard_message_id) = &starred_message.starboard_message_id {
                let starboard_message_id = starboard_message_id.parse::<u64>()?;
                update_existing_starboard_message(
                    ctx,
                    starboard_channel.into(),
                    starboard_message_id,
                    star_count,
                    &config.star_emoji,
                ).await?;
            }
        }
        None => {
            let embed = create_starboard_embed(original_message, guild_id, star_count, &config.star_emoji).await?;

            let message_builder = serenity::CreateMessage::new()
                .embed(embed);

            let starboard_message = starboard_channel
                .send_message(&ctx.http, message_builder)
                .await?;

            let starred_message = StarredMessage {
                id: 0,
                original_message_id: original_message_id.to_string(),
                original_channel_id: original_message.channel_id.get().to_string(),
                starboard_message_id: Some(starboard_message.id.get().to_string()),
                starboard_channel_id: Some(starboard_channel_id.to_string()),
                stars: star_count,
                starred_by: "".to_string(),
                created_at: None,
            };
            starboard.add_starred_message(&starred_message).await?;
        }
    }

    Ok(())
}

async fn create_starboard_embed(
    message: &serenity::Message,
    guild_id: u64,
    star_count: i64,
    star_emoji: &str,
) -> Result<serenity::CreateEmbed, Box<dyn std::error::Error + Send + Sync>> {

    let author = serenity::CreateEmbedAuthor::new(&message.author.name)
        .icon_url(&message.author.face());


    let footer = serenity::CreateEmbedFooter::new(format!("{} {}", star_emoji, star_count));

    let channel_id = message.channel_id.get();
    let message_id = message.id.get();

    let mut embed = serenity::CreateEmbed::default()
        .author(author)
        .description(&message.content)
        .field("Original", format!("https://discord.com/channels/{}/{}/{}",
                                   guild_id, channel_id, message_id),
                            false)
        .footer(footer)
        .timestamp(message.timestamp);

    if !message.attachments.is_empty() {
        if let Some(attachment) = message.attachments.first() {
            if attachment.width.is_some() && attachment.height.is_some() {
                embed = embed.image(&attachment.url);
            } else {
                embed = embed.field("Attachment", format!("[{}]({})", &attachment.filename, &attachment.url), false);
            }
        }
    }


    Ok(embed)
}

async fn update_existing_starboard_message(
    ctx: &serenity::Context,
    starboard_channel_id: u64,
    starboard_message_id: u64,
    star_count: i64,
    star_emoji: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let channel = serenity::ChannelId::new(starboard_channel_id);
    let message = serenity::MessageId::new(starboard_message_id);

    let existing_message = channel.message(&ctx.http, message).await?;

    if let Some(existing_embed) = existing_message.embeds.first() {
        let mut new_embed = serenity::CreateEmbed::default();

        // clone embed manually, i cant get the init thing right, i dont get it ah
        if let Some(ref title) = existing_embed.title {
            new_embed = new_embed.title(title);
        }

        if let Some(ref description) = existing_embed.description {
            new_embed = new_embed.description(description);
        }

        if let Some(ref author) = existing_embed.author {
            let mut new_author = serenity::CreateEmbedAuthor::new(&author.name);

            if let Some(ref url) = author.icon_url {
                new_author = new_author.icon_url(url);
            }

            new_embed = new_embed.author(new_author);
        }


        for fields in &existing_embed.fields {
            new_embed = new_embed.field(&fields.name, &fields.value, fields.inline);
        }

        let new_footer = serenity::CreateEmbedFooter::new(format!("{} {}", star_emoji, star_count));
        new_embed = new_embed.footer(new_footer);

        if let Some(ref timestamp) = existing_embed.timestamp {
            new_embed = new_embed.timestamp(*timestamp);
        }

        if let Some(ref color) = existing_embed.colour {
            new_embed = new_embed.color(*color);
        }

        if let Some(ref image) = existing_embed.image {
            new_embed = new_embed.image(image.url.clone());
        }

        if let Some(ref thumbnail) = existing_embed.thumbnail {
            new_embed = new_embed.thumbnail(thumbnail.url.clone());
        }

        let edit_builder = serenity::EditMessage::new().embed(new_embed);
        channel.edit_message(&ctx.http, message, edit_builder).await?;
    }

    Ok(())
}

async fn delete_starboard_message(
    ctx: &serenity::Context,
    starboard_channel_id: u64,
    starboard_message_id: u64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let channel = serenity::ChannelId::new(starboard_channel_id);
    let message = serenity::MessageId::new(starboard_message_id);

    channel.delete_message(&ctx.http, message).await?;

    Ok(())
}