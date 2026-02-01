use std::sync::Arc;
use poise::futures_util::lock::Mutex;
use poise::serenity_prelude as serenity;
use sqlx::SqlitePool;

pub struct Data {
    pub db: SqlitePool,
    pub last_command_success: Arc<Mutex<bool>>,
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;

pub type Context<'a> = poise::Context<'a, Data, Error>;
