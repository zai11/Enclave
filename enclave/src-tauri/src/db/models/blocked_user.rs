#[derive(Debug)]
pub struct BlockedUser {
    pub id: i64,
    pub user_id: i64
}

impl BlockedUser {
    pub fn new(id: i64, user_id: i64) -> Self {
        Self {
            id,
            user_id
        }
    }
}