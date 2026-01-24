use chrono::{DateTime, Utc};

#[derive(Debug)]
pub struct Post {
    pub id: i64,
    pub author_user_id: i64,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub edited_at: Option<DateTime<Utc>>,
}

impl Post {
    pub fn new(id: i64, author_user_id: i64, content: String, created_at: DateTime<Utc>, edited_at: Option<DateTime<Utc>>) -> Self {
        Self {
            id,
            author_user_id,
            content,
            created_at,
            edited_at
        }
    }
}