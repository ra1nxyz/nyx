use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::{SqlitePool, Error, Row};

use crate::structs::reminders::Reminder;

#[derive(Clone)]
pub struct ReminderStore {
    pool: SqlitePool,
}



// was gonna refactor to use timeparse.rs but i realise i cba to do all that rn :wilted_rose:
impl ReminderStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn add_reminder(&self, reminder: &Reminder) -> Result<i64, Error> {
        let convertformat = reminder.remind_at.format("%Y-%m-%d %H:%M:%S").to_string();

        let query = sqlx::query(
            r#"INSERT INTO reminders (user_id, context_message_url, remind_at, reminder_message)
            VALUES (?, ?, ?, ?)"#,
        )
            .bind(&reminder.user_id)
            .bind(&reminder.context_message_url)
            .bind(&convertformat)
            .bind(&reminder.reminder_message)
            .execute(&self.pool)
            .await?;

        Ok(query.last_insert_rowid())
    }

    pub async fn get_dues(&self) -> Result<Vec<Reminder>, Error> {
        let convertformat = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        let rows = sqlx::query(
            r#"SELECT * FROM reminders
            WHERE sent = FALSE
            AND remind_at <= ?
            ORDER BY remind_at
            "#,
        )
            .bind(&convertformat)
            .fetch_all(&self.pool)
            .await?;

        let mut reminders = Vec::new();
        for row in rows {
            let remind_at_str: String = row.try_get("remind_at")?;
            let remind_at = NaiveDateTime::parse_from_str(&remind_at_str, "%Y-%m-%d %H:%M:%S")
                .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc))
                .map_err(|e| sqlx::Error::Decode(Box::new(e)))?;

            let created_at_str: Option<String> = row.try_get("created_at")?;
            let created_at = match created_at_str {
                Some(s) => {
                    Some(NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S")
                        .map(|ndt| DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc))
                        .map_err(|e| sqlx::Error::Decode(Box::new(e)))?)
                }
                None => None,
            };

            reminders.push(Reminder {
                id: row.try_get("id")?,
                user_id: row.try_get("user_id")?,
                context_message_url: row.try_get("context_message_url")?,
                remind_at,
                reminder_message: row.try_get("reminder_message")?,
                sent: row.try_get("sent")?,
                created_at,
            });
        }

        Ok(reminders)
    }

    pub async fn mark_due(&self, reminder_id: i64) -> Result<(), Error> {
        sqlx::query(
            r#"UPDATE reminders
            SET sent = TRUE
            WHERE id = ?"#,
        )
            .bind(reminder_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}