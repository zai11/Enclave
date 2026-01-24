use chrono::{DateTime, Utc};

#[derive(Debug)]
pub struct DirectMessage {
    pub id: i64,
    pub user_id: i64,
    pub from_me: bool,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub edited_at: Option<DateTime<Utc>>,
    pub read: bool
}

impl DirectMessage {
    pub fn new(id: i64, user_id: i64, from_me: bool, content: String, created_at: DateTime<Utc>, edited_at: Option<DateTime<Utc>>, read: bool) -> Self {
        Self {
            id,
            user_id,
            from_me,
            content,
            created_at,
            edited_at,
            read
        }
    }
}