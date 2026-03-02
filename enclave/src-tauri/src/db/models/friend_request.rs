use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriendRequest {
    pub id: i64,
    pub from_peer_id: String,
    pub from_multiaddr: String,
    pub to_peer_id: String,
    pub to_multiaddr: String,
    pub message: String,
    pub created_at: i64,
    pub pending: bool
}

impl FriendRequest {
    pub fn new(id: i64, from_peer_id: String, from_multiaddr: String, to_peer_id: String, to_multiaddr: String, message: String, created_at: i64, pending: bool) -> Self {
        Self {
            id,
            from_peer_id,
            from_multiaddr,
            to_peer_id,
            to_multiaddr,
            message,
            created_at,
            pending
        }
    }
}