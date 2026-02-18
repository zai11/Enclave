use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
    pub id: i64,
    pub author_peer_id: String,
    pub content: String,
    pub created_at: i64,
    pub edited_at: Option<i64>,
}

impl Post {
    pub fn new(id: i64, author_peer_id: String, content: String, created_at: i64, edited_at: Option<i64>) -> Self {
        Self {
            id,
            author_peer_id,
            content,
            created_at,
            edited_at
        }
    }
}