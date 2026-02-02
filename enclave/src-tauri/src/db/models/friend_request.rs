#[derive(Debug)]
pub struct FriendRequest {
    pub id: i64,
    pub user_id: i64,
    pub message: String
}

impl FriendRequest {
    pub fn new(id: i64, user_id: i64, message: String) -> Self {
        Self {
            id,
            user_id,
            message
        }
    }
}