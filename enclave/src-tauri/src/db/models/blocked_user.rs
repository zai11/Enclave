use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockedUser {
    pub id: i64,
    pub user_id: i64,
    pub blocked_at: i64
}

impl BlockedUser {
    pub fn new(id: i64, user_id: i64, blocked_at: i64) -> Self {
        Self {
            id,
            user_id,
            blocked_at
        }
    }
}