use std::sync::Arc;
use poise::futures_util::lock::Mutex;
use poise::serenity_prelude as serenity;
use sqlx::SqlitePool;


#[derive(Debug, sqlx::FromRow)]
pub struct GuildConfig {
    pub guild_id: String,
    pub feature_enabled: i64,
}

#[derive(Debug)]
pub struct ColorRole {
    pub user_id: u64,
    pub role_id: u64,
    pub guild_id: u64,
    pub color_hex: String,
}

pub struct Data {
    pub db: SqlitePool,
    pub last_command_success: Arc<Mutex<bool>>,
    pub reminders: crate::helpers::reminder::ReminderStore,
    pub starboard: crate::helpers::starboard::Database,
    pub starboard_lock: Mutex<()>,
    pub http_client: Arc<serenity::Http>,
    pub auth: Arc<crate::helpers::auth::AuthDatabase>
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;

pub type Context<'a> = poise::Context<'a, Data, Error>;
