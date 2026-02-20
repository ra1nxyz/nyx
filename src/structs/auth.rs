use sqlx::{FromRow, SqlitePool};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AuthConfig {
    pub guild_id: i64,
    pub key_id: String,
    pub role_id: i64,
    pub enabled: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AuthenticatedUser {
    pub user_id: i64,
    pub guild_id: i64,
    pub authenticated_at: chrono::DateTime<chrono::Utc>,
}