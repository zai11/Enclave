use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
    pub id: i64,
    pub author_user_id: i64,
    pub content: String,
    pub created_at: i64,
    pub edited_at: Option<i64>,
}

impl Post {
    pub fn new(id: i64, author_user_id: i64, content: String, created_at: i64, edited_at: Option<i64>) -> Self {
        Self {
            id,
            author_user_id,
            content,
            created_at,
            edited_at
        }
    }
}