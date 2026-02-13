use std::ptr::null;
use std::sync::Arc;
use serenity::all::{Colour, CreateMessage, UserId};
use poise::serenity_prelude as serenity;
use tokio::time::{sleep, Duration};
use crate::types::Data;

pub async fn reminder_task(data: Arc<Data>) -> Result<serenity::CreateEmbed, Box<dyn std::error::Error + Send + Sync>> {
    loop {
        sleep(Duration::from_secs(60)).await;

        match data.reminders.get_dues().await {
            Ok(reminders) => {
                for reminder in reminders {
                    let user_id = match reminder.user_id.parse::<u64>() {
                        Ok(id) => UserId::new(id),
                        Err(e) => {
                            eprintln!("Invalid user ID caught in reminder feedback loop for user {}, E: {}", reminder.user_id, e);
                            continue;
                        }
                    };

                    let readable = reminder.created_at.map(|t| format!("<t:{}:F>", t.timestamp()))
                        .unwrap_or_else(|| "Unknown time".into());

                    //let footer = serenity::CreateEmbedFooter::new(format!("Reminder created on {}", readable));

                    let embed = serenity::CreateEmbed::default()
                        .colour(Colour::new(0x800080))
                        .title(format!("Reminder set at {}:", readable))
                        .description(reminder.reminder_message)
                        .field("Context", format!("{}", &reminder.context_message_url.unwrap().to_string()), true);

                    match user_id.create_dm_channel(&data.http_client).await {
                        Ok(dm_channel) => {
                            if let Err(e) = dm_channel
                                .send_message(&data.http_client, CreateMessage::new().content("").embed(embed))
                                .await
                            {
                            eprintln!("Error sending reminder E: {}", e);
                        } else {
                            if let Err(e) = data.reminders.mark_due(reminder.id).await {
                            eprintln!("Error marking reminders: {}", e);}
                            }
                        }
                        Err(e) => {
                            eprintln!("Error creating dm_channel for reminder: {}", e);
                        }
                    }

                }
            }
            Err(e) => {
                eprintln!("Error fetching dm_channel for reminders: {}", e);
            }
        }
    }
}
