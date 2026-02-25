use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Friend {
    pub id: i64,
    pub user_id: i64,
    pub created_at: i64,
    pub last_synch: i64
}

impl Friend {
    pub fn new(id: i64, user_id: i64, created_at: i64, last_synch: i64) -> Self {
        Self {
            id,
            user_id,
            created_at,
            last_synch
        }
    }
}