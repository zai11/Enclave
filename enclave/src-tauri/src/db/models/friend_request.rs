use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriendRequest {
    pub id: i64,
    pub from_user_id: i64,
    pub message: String,
    pub created_at: i64
}

impl FriendRequest {
    pub fn new(id: i64, from_user_id: i64, message: String, created_at: i64) -> Self {
        Self {
            id,
            from_user_id,
            message,
            created_at
        }
    }
}