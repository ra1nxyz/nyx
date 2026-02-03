use sqlx::{sqlite::SqlitePool, prelude::*};
pub(crate) use crate::structs::starboard_message::{StarboardConfig, StarredMessage};

#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = SqlitePool::connect(database_url).await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS starboard_messages (
                id INTEGER PRIMARY KEY,
                original_message_id TEXT NOT NULL UNIQUE,
                original_channel_id TEXT NOT NULL,
                starboard_message_id TEXT,
                starboard_channel_id TEXT,
                stars INTEGER DEFAULT 1,
                starred_by TEXT NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )
            "#
        ).execute(&pool).await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS starboard_reactions (
                id INTEGER PRIMARY KEY,
                message_id TEXT NOT NULL,
                user_id TEXT NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(message_id, user_id)
            )
            "#
        ).execute(&pool).await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS starboard_config (
                guild_id TEXT PRIMARY KEY,
                starboard_channel_id TEXT,
                threshold INTEGER DEFAULT 2,
                star_emoji TEXT DEFAULT '⭐',
                self_star_allowed BOOLEAN DEFAULT FALSE,
                enabled BOOLEAN DEFAULT TRUE
            )
            "#
        ).execute(&pool).await?;

        Ok(Self { pool })
    }

    pub async fn get_starboard_config(&self, guild_id: u64) -> Result<Option<StarboardConfig>, sqlx::Error> {
        sqlx::query_as::<_, StarboardConfig>(
            "SELECT * FROM starboard_config WHERE guild_id = ?"
        )
            .bind(guild_id.to_string())
            .fetch_optional(&self.pool)
            .await
    }
    /*
        pub async fn set_starboard_channel(&self, guild_id: u64, channel_id: u64) -> Result<(), sqlx::Error> {
            sqlx::query(
                "INSERT OR REPLACE INTO starboard_config (guild_id, starboard_channel_id) VALUES (?, ?)"
            )
                .bind(guild_id.to_string())
                .bind(channel_id.to_string())
                .execute(&self.pool)
                .await?;
            Ok(())
        }

        pub async fn set_starboard_threshold(&self, guild_id: u64, threshold: i64) -> Result<(), sqlx::Error> {
            sqlx::query(
                "INSERT OR REPLACE INTO starboard_config (guild_id, threshold) VALUES (?, ?)"
            )
                .bind(guild_id.to_string())
                .bind(threshold)
                .execute(&self.pool)
                .await?;
            Ok(())
        }

        // dont think this is necessary since exec exists, backup tho
        pub async fn update_starboard_config(&self, config: &StarboardConfig) -> Result<(), sqlx::Error> {
            sqlx::query(
                "INSERT OR REPLACE INTO starboard_config (guild_id, starboard_channel_id, threshold, star_emoji, self_star_allowed, enabled)
                 VALUES (?, ?, ?, ?, ?, ?)"
            )
                .bind(&config.guild_id)
                .bind(&config.starboard_channel_id)
                .bind(config.threshold)
                .bind(&config.star_emoji)
                .bind(config.self_star_allowed)
                .bind(config.enabled)
                .execute(&self.pool)
                .await?;
            Ok(())
        }
    */
    pub async fn get_starred_message(&self, message_id: u64) -> Result<Option<StarredMessage>, sqlx::Error> {
        sqlx::query_as::<_, StarredMessage>(
            "SELECT * FROM starboard_messages WHERE original_message_id = ?"
        )
            .bind(message_id.to_string())
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn add_starred_message(&self, message: &StarredMessage) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO starboard_messages (original_message_id, original_channel_id, starboard_message_id, starboard_channel_id, stars, starred_by)
             VALUES (?, ?, ?, ?, ?, ?)"
        )
            .bind(&message.original_message_id)
            .bind(&message.original_channel_id)
            .bind(&message.starboard_message_id)
            .bind(&message.starboard_channel_id)
            .bind(message.stars)
            .bind(&message.starred_by)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_starred_message(&self, message: &StarredMessage) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE starboard_messages SET stars = ?, starboard_message_id = ? WHERE original_message_id = ?"
        )
            .bind(message.stars)
            .bind(&message.starboard_message_id)
            .bind(&message.original_message_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete_starred_message(&self, message_id: u64) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM starboard_messages WHERE original_message_id = ?")
            .bind(message_id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn add_star_reaction(&self, message_id: u64, user_id: u64) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT OR IGNORE INTO starboard_reactions (message_id, user_id) VALUES (?, ?)"
        )
            .bind(message_id.to_string())
            .bind(user_id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn remove_star_reaction(&self, message_id: u64, user_id: u64) -> Result<(), sqlx::Error> {
        sqlx::query(
            "DELETE FROM starboard_reactions WHERE message_id = ? AND user_id = ?"
        )
            .bind(message_id.to_string())
            .bind(user_id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn count_star_reactions(&self, message_id: u64) -> Result<i64, sqlx::Error> {
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM starboard_reactions WHERE message_id = ?"
        )
            .bind(message_id.to_string())
            .fetch_one(&self.pool)
            .await?;
        Ok(count.0)
    }

    // no functionality its way too late to do it now
    pub async fn has_user_starred(&self, message_id: u64, user_id: u64) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "SELECT 1 FROM starboard_reactions WHERE message_id = ? AND user_id = ? LIMIT 1"
        )
            .bind(message_id.to_string())
            .bind(user_id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        Ok(result.is_some())
    }
}

