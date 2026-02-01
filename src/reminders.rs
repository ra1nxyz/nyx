use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;


#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Reminder {
    pub id: i64,
    pub user_id: String,
    pub context_message_url: Option<String>,
    pub remind_at: DateTime<Utc>,
    pub reminder_message: String,
    pub sent: bool,
    pub created_at: Option<DateTime<Utc>>,
}

impl Reminder {
    pub fn new(
        user_id: String,
        remind_at: DateTime<Utc>,
        reminder_message: String,
        context_message_url: Option<String>,
    ) -> Self {
        Self {
            id: 0,
            user_id,
            context_message_url,
            remind_at,
            reminder_message,
            sent: false,
            created_at: None,
        }
    }

    pub fn is_due(&self) -> bool {
        self.remind_at <= Utc::now() && !self.sent
    }
}