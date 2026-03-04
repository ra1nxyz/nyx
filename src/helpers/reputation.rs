use sqlx::{SqlitePool, QueryBuilder, Row};
use chrono::{Utc, Duration, DateTime, NaiveDate};
use serenity::all::{UserId, GuildId, ChannelId, MessageId, Cache};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tracing::{info, warn, error};

#[derive(Debug, Clone)]
pub struct InteractionWeights {
    pub reaction: f64,
    pub mention: f64,
    pub reply: f64,
    pub thread_participation: f64,
    pub message: f64,
}

impl Default for InteractionWeights {
    fn default() -> Self {
        Self {
            reaction: 0.5,
            mention: 2.0,
            reply: 1.5,
            thread_participation: 1.0,
            message: 0.1, // Base weight for just sending a message
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReputationEngine {
    pool: SqlitePool,
    weights: InteractionWeights,
}

#[derive(Debug, serde::Serialize)]
pub struct UserReputation {
    pub user_id: String,
    pub guild_id: String,
    pub reputation_score: f64,
    pub influence_score: f64,
    pub total_interactions: i64,
    pub unique_interactors: i64,
    pub toxicity_score: f64,
}

#[derive(Debug, serde::Serialize)]
pub struct InfluenceGraph {
    pub nodes: Vec<InfluenceNode>,
    pub edges: Vec<InfluenceEdge>,
}

#[derive(Debug, serde::Serialize)]
pub struct InfluenceNode {
    pub id: String,
    pub name: String,
    pub reputation: f64,
    pub size: f64,
}

#[derive(Debug, serde::Serialize)]
pub struct InfluenceEdge {
    pub source: String,
    pub target: String,
    pub weight: f64,
    pub interaction_count: i64,
}

impl ReputationEngine {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            weights: InteractionWeights::default(),
        }
    }

    pub fn with_weights(mut self, weights: InteractionWeights) -> Self {
        self.weights = weights;
        self
    }

    pub async fn track_interaction(
        &self,
        guild_id: GuildId,
        from_user: UserId,
        to_user: UserId,
        interaction_type: &str,
        channel_id: ChannelId,
        message_id: Option<MessageId>,
    ) -> Result<(), sqlx::Error> {
        let weight = match interaction_type {
            "reaction" => self.weights.reaction,
            "mention" => self.weights.mention,
            "reply" => self.weights.reply,
            "thread" => self.weights.thread_participation,
            "message" => self.weights.message,
            _ => 1.0,
        };

        sqlx::query(
            r#"
            INSERT INTO interactions (guild_id, from_user_id, to_user_id, interaction_type, channel_id, message_id, weight)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
            .bind(guild_id.to_string())
            .bind(from_user.to_string())
            .bind(to_user.to_string())
            .bind(interaction_type)
            .bind(channel_id.to_string())
            .bind(message_id.map(|id| id.to_string()))
            .bind(weight)
            .execute(&self.pool)
            .await?;

        sqlx::query(
            r#"
            INSERT INTO influence_edges (from_user_id, to_user_id, guild_id, interaction_count, total_weight, last_interaction)
            VALUES (?, ?, ?, 1, ?, CURRENT_TIMESTAMP)
            ON CONFLICT(from_user_id, to_user_id, guild_id) DO UPDATE SET
                interaction_count = interaction_count + 1,
                total_weight = total_weight + ?,
                last_interaction = CURRENT_TIMESTAMP
            "#,
        )
            .bind(from_user.to_string())
            .bind(to_user.to_string())
            .bind(guild_id.to_string())
            .bind(weight)
            .bind(weight)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn calculate_reputation(&self, guild_id: GuildId) -> Result<(), sqlx::Error> {
        let users = sqlx::query(
            r#"
            SELECT DISTINCT from_user_id as user_id FROM interactions WHERE guild_id = ?
            UNION
            SELECT DISTINCT to_user_id as user_id FROM interactions WHERE guild_id = ?
            "#,
        )
            .bind(guild_id.to_string())
            .bind(guild_id.to_string())
            .fetch_all(&self.pool)
            .await?;

        for record in users {
            let user_id: String = record.get(0);

            let stats = sqlx::query(
                r#"
                SELECT
                    COUNT(*) as total_interactions,
                    COUNT(DISTINCT from_user_id) as received_from_count,
                    COALESCE(SUM(CASE WHEN to_user_id = ? THEN weight ELSE 0 END), 0) as received_weight,
                    COALESCE(SUM(CASE WHEN from_user_id = ? THEN weight ELSE 0 END), 0) as given_weight
                FROM interactions
                WHERE guild_id = ? AND (to_user_id = ? OR from_user_id = ?)
                "#,
            )
                .bind(&user_id)
                .bind(&user_id)
                .bind(guild_id.to_string())
                .bind(&user_id)
                .bind(&user_id)
                .fetch_one(&self.pool)
                .await?;

            let total_interactions: i64 = stats.get(0);
            let unique_interactors: i64 = stats.get(1);
            let received_weight: f64 = match stats.try_get::<f64, _>(2) {
                Ok(val) => val,
                Err(_) => stats.get::<i64, _>(2) as f64,
            };

            let given_weight: f64 = match stats.try_get::<f64, _>(3) {
                Ok(val) => val,
                Err(_) => stats.get::<i64, _>(3) as f64,
            };

            // (received_weight * 0.7) + (unique_interactors * 0.3) - (toxicity_penalty)
            let toxicity = self.get_user_toxicity(&user_id, guild_id).await?;
            let toxicity_penalty = toxicity * 50.0; // multiplier

            let reputation_score = (received_weight * 0.7) + (unique_interactors as f64 * 3.0) - toxicity_penalty;

            // ratio of received to given, normalised
            let influence_score = if given_weight > 0.0 {
                (received_weight / given_weight).min(10.0)
            } else {
                received_weight / 10.0
            };

            sqlx::query(
                r#"
                INSERT INTO user_reputation (user_id, guild_id, reputation_score, influence_score, total_interactions, unique_interactors, toxicity_score, last_calculated)
                VALUES (?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
                ON CONFLICT(user_id, guild_id) DO UPDATE SET
                    reputation_score = ?,
                    influence_score = ?,
                    total_interactions = ?,
                    unique_interactors = ?,
                    toxicity_score = ?,
                    last_calculated = CURRENT_TIMESTAMP
                "#,
            )
                .bind(&user_id)
                .bind(guild_id.to_string())
                .bind(reputation_score)
                .bind(influence_score)
                .bind(total_interactions)
                .bind(unique_interactors)
                .bind(toxicity)
                .bind(reputation_score)
                .bind(influence_score)
                .bind(total_interactions)
                .bind(unique_interactors)
                .bind(toxicity)
                .execute(&self.pool)
                .await?;
        }

        Ok(())
    }

    pub async fn get_user_reputation(
        &self,
        user_id: UserId,
        guild_id: GuildId,
    ) -> Result<Option<UserReputation>, sqlx::Error> {
        let result = sqlx::query(
            r#"
            SELECT user_id, guild_id, reputation_score, influence_score, total_interactions, unique_interactors, toxicity_score
            FROM user_reputation
            WHERE user_id = ? AND guild_id = ?
            "#,
        )
            .bind(user_id.to_string())
            .bind(guild_id.to_string())
            .fetch_optional(&self.pool)
            .await?;

        Ok(result.map(|r| UserReputation {
            user_id: r.get(0),
            guild_id: r.get(1),
            reputation_score: r.get(2),
            influence_score: r.get(3),
            total_interactions: r.get(4),
            unique_interactors: r.get(5),
            toxicity_score: r.get(6),
        }))
    }

    pub async fn get_influence_graph(
        &self,
        guild_id: GuildId,
        cache: &Arc<Cache>,
        min_weight: Option<f64>,
        limit: Option<usize>,
    ) -> Result<InfluenceGraph, sqlx::Error> {
        let min_weight = min_weight.unwrap_or(1.0);
        let limit = limit.unwrap_or(50);

        // Get top edges
        let edges = sqlx::query(
            r#"
        SELECT from_user_id, to_user_id, interaction_count, total_weight
        FROM influence_edges
        WHERE guild_id = ? AND total_weight >= ?
        ORDER BY total_weight DESC
        LIMIT ?
        "#,
        )
            .bind(guild_id.to_string())
            .bind(min_weight)
            .bind(limit as i64)
            .fetch_all(&self.pool)
            .await?;

        // Collect unique users
        let mut user_ids = HashSet::new();
        let mut node_map = HashMap::new();

        for edge in &edges {
            let from_id: String = edge.get(0);
            let to_id: String = edge.get(1);
            user_ids.insert(from_id.clone());
            user_ids.insert(to_id.clone());
        }

        // Get user reputations and names
        for user_id in user_ids {
            if let Some(rep) = self.get_user_reputation(UserId::new(user_id.parse().unwrap()), guild_id).await? {
                let name = cache.user(UserId::new(user_id.parse().unwrap()))
                    .map(|u| u.name.clone())
                    .unwrap_or_else(|| format!("User {}", user_id));

                node_map.insert(user_id.clone(), (rep, name));
            }
        }

        let nodes = node_map
            .into_iter()
            .map(|(id, (rep, name))| InfluenceNode {
                id,
                name,
                reputation: rep.reputation_score,
                size: rep.influence_score * 10.0,
            })
            .collect();

        let graph_edges = edges
            .into_iter()
            .map(|e| InfluenceEdge {
                source: e.get(0),
                target: e.get(1),
                weight: e.get(3),
                interaction_count: e.get(2),
            })
            .collect();

        Ok(InfluenceGraph {
            nodes,
            edges: graph_edges,
        })
    }

    pub async fn flag_toxicity(
        &self,
        guild_id: GuildId,
        flagged_user: UserId,
        flagged_by: UserId,
        reason: String,
        message_id: Option<MessageId>,
        severity: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO toxicity_flags (guild_id, flagged_user_id, flagged_by_user_id, reason, message_id, severity)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
            .bind(guild_id.to_string())
            .bind(flagged_user.to_string())
            .bind(flagged_by.to_string())
            .bind(reason)
            .bind(message_id.map(|id| id.to_string()))
            .bind(severity)
            .execute(&self.pool)
            .await?;

        self.calculate_toxicity_score(flagged_user, guild_id).await?;

        Ok(())
    }

    async fn calculate_toxicity_score(
        &self,
        user_id: UserId,
        guild_id: GuildId,
    ) -> Result<f64, sqlx::Error> {
        let thirty_days_ago = Utc::now() - Duration::days(30);

        let flags = sqlx::query(
            r#"
            SELECT severity, created_at
            FROM toxicity_flags
            WHERE flagged_user_id = ? AND guild_id = ? AND resolved = FALSE
            AND created_at >= ?
            "#,
        )
            .bind(user_id.to_string())
            .bind(guild_id.to_string())
            .bind(thirty_days_ago)
            .fetch_all(&self.pool)
            .await?;

        if flags.is_empty() {
            sqlx::query(
                r#"
                UPDATE user_reputation
                SET toxicity_score = 0
                WHERE user_id = ? AND guild_id = ?
                "#,
            )
                .bind(user_id.to_string())
                .bind(guild_id.to_string())
                .execute(&self.pool)
                .await?;
            return Ok(0.0);
        }

        let now = Utc::now();
        let mut total_score = 0.0;
        let mut total_weight = 0.0;

        for flag in flags {
            let flag_time: DateTime<Utc> = flag.get(1);
            let days_ago = (now - flag_time).num_days() as f64;
            let recency_weight = (30.0 - days_ago) / 30.0;

            let severity: i64 = flag.get(0);
            let severity_weight = severity as f64 / 5.0;

            total_score += recency_weight * severity_weight;
            total_weight += recency_weight;
        }

        let toxicity_score = if total_weight > 0.0 {
            (total_score / total_weight).min(1.0)
        } else {
            0.0
        };

        sqlx::query(
            r#"
            UPDATE user_reputation
            SET toxicity_score = ?
            WHERE user_id = ? AND guild_id = ?
            "#,
        )
            .bind(toxicity_score)
            .bind(user_id.to_string())
            .bind(guild_id.to_string())
            .execute(&self.pool)
            .await?;

        Ok(toxicity_score)
    }

    async fn get_user_toxicity(
        &self,
        user_id: &str,
        guild_id: GuildId,
    ) -> Result<f64, sqlx::Error> {
        let result = sqlx::query(
            r#"
            SELECT toxicity_score
            FROM user_reputation
            WHERE user_id = ? AND guild_id = ?
            "#,
        )
            .bind(user_id)
            .bind(guild_id.to_string())
            .fetch_optional(&self.pool)
            .await?;

        Ok(result.map(|r| r.get(0)).unwrap_or(0.0))
    }

    pub async fn get_top_influential(
        &self,
        guild_id: GuildId,
        limit: usize,
    ) -> Result<Vec<UserReputation>, sqlx::Error> {
        let results = sqlx::query(
            r#"
            SELECT user_id, guild_id, reputation_score, influence_score, total_interactions, unique_interactors, toxicity_score
            FROM user_reputation
            WHERE guild_id = ?
            ORDER BY influence_score DESC
            LIMIT ?
            "#,
        )
            .bind(guild_id.to_string())
            .bind(limit as i64)
            .fetch_all(&self.pool)
            .await?;

        Ok(results
            .into_iter()
            .map(|r| UserReputation {
                user_id: r.get(0),
                guild_id: r.get(1),
                reputation_score: r.get(2),
                influence_score: r.get(3),
                total_interactions: r.get(4),
                unique_interactors: r.get(5),
                toxicity_score: r.get(6),
            })
            .collect())
    }

    pub async fn track_activity(
        &self,
        user_id: UserId,
        guild_id: GuildId,
    ) -> Result<(), sqlx::Error> {
        let today = Utc::now().date_naive();

        sqlx::query(
            r#"
            INSERT INTO user_activity (user_id, guild_id, last_message_time, daily_message_count, weekly_message_count, monthly_message_count, last_active_date)
            VALUES (?, ?, CURRENT_TIMESTAMP, 1, 1, 1, ?)
            ON CONFLICT(user_id, guild_id) DO UPDATE SET
                last_message_time = CURRENT_TIMESTAMP,
                daily_message_count = CASE
                    WHEN date(last_active_date) = date(?) THEN daily_message_count + 1
                    ELSE 1
                END,
                weekly_message_count = CASE
                    WHEN julianday(?) - julianday(last_active_date) <= 7 THEN weekly_message_count + 1
                    ELSE 1
                END,
                monthly_message_count = CASE
                    WHEN julianday(?) - julianday(last_active_date) <= 30 THEN monthly_message_count + 1
                    ELSE 1
                END,
                last_active_date = ?
            "#,
        )
            .bind(user_id.to_string())
            .bind(guild_id.to_string())
            .bind(today.to_string())
            .bind(today.to_string())
            .bind(today.to_string())
            .bind(today.to_string())
            .bind(today.to_string())
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn get_interaction_stats(
        &self,
        guild_id: GuildId,
    ) -> Result<serde_json::Value, sqlx::Error> {
        let total = sqlx::query(
            r#"
            SELECT
                COUNT(*) as total,
                COUNT(DISTINCT from_user_id) as unique_senders,
                COUNT(DISTINCT to_user_id) as unique_receivers,
                interaction_type,
                SUM(weight) as total_weight
            FROM interactions
            WHERE guild_id = ?
            GROUP BY interaction_type
            "#,
        )
            .bind(guild_id.to_string())
            .fetch_all(&self.pool)
            .await?;

        let mut stats = Vec::new();
        for row in total {
            stats.push(serde_json::json!({
                "type": row.get::<String, _>(3),
                "count": row.get::<i64, _>(0),
                "total_weight": row.get::<f64, _>(4),
            }));
        }

        Ok(serde_json::json!({ "interactions": stats }))
    }
}