use sqlx::{SqlitePool, Sqlite, QueryBuilder};
use crate::structs::auth::{AuthConfig, AuthenticatedUser};

#[derive(Clone)]
pub struct AuthDatabase {
    pool: SqlitePool,
}

impl AuthDatabase {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create_tables(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS auth_configs (
                guild_id INTEGER PRIMARY KEY,
                key_id TEXT NOT NULL,
                role_id INTEGER NOT NULL,
                enabled BOOLEAN NOT NULL DEFAULT 1,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#
        )
            .execute(&self.pool)
            .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS authenticated_users (
                user_id INTEGER NOT NULL,
                guild_id INTEGER NOT NULL,
                authenticated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY (user_id, guild_id)
            )
            "#
        )
            .execute(&self.pool)
            .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_authenticated_users_guild
            ON authenticated_users(guild_id)
            "#
        )
            .execute(&self.pool)
            .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_auth_configs_key_id
            ON auth_configs(key_id)
            "#
        )
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn get_auth_config(&self, guild_id: i64) -> Result<Option<AuthConfig>, sqlx::Error> {
        sqlx::query_as::<_, AuthConfig>(
            r#"
            SELECT * FROM auth_configs
            WHERE guild_id = ?
            "#
        )
            .bind(guild_id)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn set_auth_config(
        &self,
        guild_id: i64,
        key_id: String,
        role_id: i64,
        enabled: bool,
    ) -> Result<AuthConfig, sqlx::Error> {
        let enabled_int = if enabled { 1 } else { 0 };

        sqlx::query_as::<_, AuthConfig>(
            r#"
            INSERT INTO auth_configs (guild_id, key_id, role_id, enabled, created_at, updated_at)
            VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
            ON CONFLICT(guild_id) DO UPDATE SET
                key_id = excluded.key_id,
                role_id = excluded.role_id,
                enabled = excluded.enabled,
                updated_at = CURRENT_TIMESTAMP
            RETURNING *
            "#
        )
            .bind(guild_id)
            .bind(key_id)
            .bind(role_id)
            .bind(enabled_int)
            .fetch_one(&self.pool)
            .await
    }

    pub async fn delete_auth_config(&self, guild_id: i64) -> Result<Option<AuthConfig>, sqlx::Error> {
        sqlx::query_as::<_, AuthConfig>(
            r#"
            DELETE FROM auth_configs
            WHERE guild_id = ?
            RETURNING *
            "#
        )
            .bind(guild_id)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn is_user_authenticated(&self, user_id: i64, guild_id: i64) -> Result<bool, sqlx::Error> {
        let result: Option<(i64,)> = sqlx::query_as(
            r#"
            SELECT 1 FROM authenticated_users
            WHERE user_id = ? AND guild_id = ?
            "#
        )
            .bind(user_id)
            .bind(guild_id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(result.is_some())
    }

    pub async fn add_authenticated_user(
        &self,
        user_id: i64,
        guild_id: i64,
    ) -> Result<AuthenticatedUser, sqlx::Error> {
        sqlx::query_as::<_, AuthenticatedUser>(
            r#"
            INSERT INTO authenticated_users (user_id, guild_id, authenticated_at)
            VALUES (?, ?, CURRENT_TIMESTAMP)
            ON CONFLICT(user_id, guild_id) DO NOTHING
            RETURNING *
            "#
        )
            .bind(user_id)
            .bind(guild_id)
            .fetch_one(&self.pool)
            .await
    }

    pub async fn get_authenticated_users(&self, guild_id: i64) -> Result<Vec<AuthenticatedUser>, sqlx::Error> {
        sqlx::query_as::<_, AuthenticatedUser>(
            r#"
            SELECT * FROM authenticated_users
            WHERE guild_id = ?
            ORDER BY authenticated_at DESC
            "#
        )
            .bind(guild_id)
            .fetch_all(&self.pool)
            .await
    }

    pub async fn remove_authenticated_user(
        &self,
        user_id: i64,
        guild_id: i64,
    ) -> Result<Option<AuthenticatedUser>, sqlx::Error> {
        sqlx::query_as::<_, AuthenticatedUser>(
            r#"
            DELETE FROM authenticated_users
            WHERE user_id = ? AND guild_id = ?
            RETURNING *
            "#
        )
            .bind(user_id)
            .bind(guild_id)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn get_config_by_key_id(&self, key_id: &str) -> Result<Option<AuthConfig>, sqlx::Error> {
        sqlx::query_as::<_, AuthConfig>(
            r#"
            SELECT * FROM auth_configs
            WHERE key_id = ? AND enabled = 1
            "#
        )
            .bind(key_id)
            .fetch_optional(&self.pool)
            .await
    }
}