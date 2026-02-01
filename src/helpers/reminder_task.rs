use std::sync::Arc;
use serenity::all::{CreateMessage, UserId};
use tokio::time::{sleep, Duration};
use crate::types::Data;

pub async fn reminder_task(data: Arc<Data>) {
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
                    let mut message = format!("Reminder: {}\n", reminder.reminder_message);

                    if let Some(url) = &reminder.context_message_url {
                        message.push_str(&format!("\n[Context]({})", url))
                    }

                    match user_id.create_dm_channel(&data.http_client).await {
                        Ok(dm_channel) => {
                            if let Err(e) = dm_channel
                                .send_message(&data.http_client, CreateMessage::new().content(&message))
                                .await
                            {
                            eprintln!("Error sending reminder: {} to user {}, E: {}", reminder.reminder_message, reminder.user_id, e);
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
