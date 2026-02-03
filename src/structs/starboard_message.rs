use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct StarboardConfig {
    pub guild_id: String,
    pub starboard_channel_id: Option<String>,
    pub threshold: i64,
    pub star_emoji: String,
    pub self_star_allowed: bool,
    pub enabled: bool,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct StarredMessage {
    pub id: i64,
    pub original_message_id: String,
    pub original_channel_id: String,
    pub starboard_message_id: Option<String>,
    pub starboard_channel_id: Option<String>,
    pub stars: i64,
    pub starred_by: String,
    pub created_at: Option<String>,
}