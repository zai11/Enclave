use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectMessage {
    pub id: i64,
    pub from_peer_id: String,
    pub to_peer_id: String,
    pub content: String,
    pub created_at: i64,
    pub edited_at: Option<i64>,
    pub read: bool
}

impl DirectMessage {
    pub fn new(id: i64, from_peer_id: String, to_peer_id: String, content: String, created_at: i64, edited_at: Option<i64>, read: bool) -> Self {
        Self {
            id,
            from_peer_id,
            to_peer_id,
            content,
            created_at,
            edited_at,
            read
        }
    }
}