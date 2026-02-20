use sqlx::SqlitePool;
use crate::types::Error;
use crate::types::GuildConfig;

pub async fn init_role_colour_table(pool: &SqlitePool) -> Result<(), Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS guild_config (
            guild_id TEXT PRIMARY KEY,
            feature_enabled INTEGER NOT NULL DEFAULT 0
        )
        "#,
    )
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn set_feature_enabled(pool: &SqlitePool, guild_id: u64, enabled: bool) -> Result<(), Error> {
    let enabled_int = if enabled { 1 } else { 0 };

    if enabled {
        sqlx::query(
            r#"
            INSERT INTO guild_config (guild_id, feature_enabled)
            VALUES (?, ?)
            ON CONFLICT(guild_id) DO UPDATE SET feature_enabled = ?
            "#,
        )
            .bind(guild_id.to_string())
            .bind(enabled_int)
            .bind(enabled_int)
            .execute(pool)
            .await?;
    } else {
        sqlx::query(
            r#"
            DELETE FROM guild_config WHERE guild_id = ?
            "#,
        )
            .bind(guild_id.to_string())
            .execute(pool)
            .await?;
    }

    Ok(())
}

pub async fn is_feature_enabled(pool: &SqlitePool, guild_id: u64) -> Result<bool, Error> {
    let result = sqlx::query_as::<_, GuildConfig>(
        r#"
        SELECT guild_id, feature_enabled FROM guild_config WHERE guild_id = ?
        "#,
    )
        .bind(guild_id.to_string())
        .fetch_optional(pool)
        .await?;

    Ok(result.map(|c| c.feature_enabled == 1).unwrap_or(false))
}

pub async fn cleanup_old_role(pool: &SqlitePool, guild_id: u64, user_id: u64) -> Result<Option<u64>, Error> {
    // track and re role or remove old roles, dont wanna implement yet
    Ok(None)
}