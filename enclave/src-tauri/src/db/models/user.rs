use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub peer_id: String,
    pub multiaddr: String,
    pub nickname: Option<String>,
    pub is_identity: bool,
    pub created_at: i64
}

impl User {
    pub fn new(id: i64, peer_id: String, multiaddr: String, nickname: Option<String>, is_identity: bool, created_at: i64) -> Self {
        Self {
            id,
            peer_id,
            multiaddr,
            nickname,
            is_identity,
            created_at
        }
    }
}