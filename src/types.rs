use std::sync::Arc;
use poise::futures_util::lock::Mutex;
use poise::serenity_prelude as serenity;
use sqlx::SqlitePool;


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
