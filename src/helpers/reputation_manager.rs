use serenity::all::{Context, Message, Reaction, UserId};
use crate::types::Data;
use crate::Error;

pub async fn handle_message(ctx: &Context, new_message: &Message, data: &Data) -> Result<(), Error> {
    if new_message.author.bot || new_message.guild_id.is_none() {
        return Ok(());
    }

    let guild_id = new_message.guild_id.unwrap();

    data.reputation.track_activity(new_message.author.id, guild_id).await?;

    for user in &new_message.mentions {
        if user.id != new_message.author.id && !user.bot {
            data.reputation.track_interaction(
                guild_id,
                new_message.author.id,
                user.id,
                "mention",
                new_message.channel_id,
                Some(new_message.id),
            ).await?;
        }
    }

    if let Some(referenced) = &new_message.referenced_message {
        if referenced.author.id != new_message.author.id && !referenced.author.bot {
            data.reputation.track_interaction(
                guild_id,
                new_message.author.id,
                referenced.author.id,
                "reply",
                new_message.channel_id,
                Some(new_message.id),
            ).await?;
        }
    }

    Ok(())
}

pub async fn handle_reaction(ctx: &Context, reaction: &Reaction, data: &Data) -> Result<(), Error> {
    if reaction.user_id == reaction.message_author_id || reaction.user_id.is_none() {
        return Ok(());
    }

    let guild_id = match reaction.guild_id {
        Some(id) => id,
        None => return Ok(()),
    };

    let message_author_id = match reaction.message_author_id {
        Some(id) => id,
        None => return Ok(()),
    };

    data.reputation.track_interaction(
        guild_id,
        reaction.user_id.unwrap(),
        message_author_id,
        "reaction",
        reaction.channel_id,
        Some(reaction.message_id),
    ).await?;

    Ok(())
}