use std::sync::Arc;
use tokio::time;
use tracing::{info, error, warn, debug};
use serenity::all::GuildId;

use crate::types::Data;

pub async fn reputation_task(data: Arc<Data>) {
    info!("Starting reputation calculation task");

    // Run every 6 hours
    let mut interval = time::interval(time::Duration::from_secs(6 * 60 * 60));

    info!("Running initial reputation calculation on startup");
    if let Err(e) = run_reputation_calculation(&data).await {
        error!("Initial reputation calculation failed: {}", e);
    }

    loop {
        interval.tick().await;
        info!("⏰ Running scheduled reputation calculation");

        if let Err(e) = run_reputation_calculation(&data).await {
            error!("Scheduled reputation calculation failed: {}", e);
        }
    }
}

async fn run_reputation_calculation(data: &Data) -> Result<(), sqlx::Error> {
    info!("Accessing cache to get guilds");

    let guilds: Vec<GuildId> = data.cache.guilds().into_iter().collect();

    info!("Found {} guilds in cache", guilds.len());

    if guilds.is_empty() {
        warn!("No guilds found in cache bot just started");
        return Ok(());
    }

    info!("Calculating reputation for {} guilds", guilds.len());

    let mut success_count = 0;
    let mut fail_count = 0;

    for (idx, guild_id) in guilds.iter().enumerate() {
        debug!("Processing guild {} of {}: {}", idx + 1, guilds.len(), guild_id);

        match data.reputation.calculate_reputation(*guild_id).await {
            Ok(_) => {
                success_count += 1;
                debug!("Successfully calculated reputation for guild {}", guild_id);
            }
            Err(e) => {
                fail_count += 1;
                error!("Failed to calculate reputation for guild {}: {}", guild_id, e);
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    info!("Reputation calculation completed - Success: {}, Failed: {}", success_count, fail_count);

    info!("Cleaning up old interactions");
    let deleted = sqlx::query(
        r#"
        DELETE FROM interactions
        WHERE created_at < datetime('now', '-90 days')
        "#
    )
        .execute(&data.db)
        .await?;

    if deleted.rows_affected() > 0 {
        info!("Cleaned up {} old interaction records", deleted.rows_affected());
    } else {
        debug!("No old interactions to clean up");
    }

    Ok(())
}

async fn cleanup_old_interactions(data: &Data) -> Result<(), sqlx::Error> {
    let ninety_days_ago = chrono::Utc::now() - chrono::Duration::days(90);

    let deleted = sqlx::query(
        r#"
        DELETE FROM interactions 
        WHERE created_at < ?
        "#
    )
        .bind(ninety_days_ago)
        .execute(&data.db)
        .await?;

    info!("Cleaned up {} old interaction records", deleted.rows_affected());
    Ok(())
}

pub async fn activity_streak_task(data: Arc<Data>) {
    let mut interval = time::interval(time::Duration::from_secs(60 * 60));

    loop {
        interval.tick().await;

        if let Err(e) = reset_activity_streaks(&data).await {
            error!("Failed to reset activity streaks: {}", e);
        }
    }
}

async fn reset_activity_streaks(data: &Data) -> Result<(), sqlx::Error> {
    let yesterday = (chrono::Utc::now() - chrono::Duration::days(1)).date_naive();

    sqlx::query(
        r#"
        UPDATE user_activity
        SET daily_message_count = 0
        WHERE last_active_date < ?
        "#
    )
        .bind(yesterday.to_string())
        .execute(&data.db)
        .await?;

    Ok(())
}