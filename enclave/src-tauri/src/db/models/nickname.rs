#[derive(Debug)]
pub struct Nickname {
    pub id: i64,
    pub user_id: i64,
    pub nickname: String
}

impl Nickname {
    pub fn new(id: i64, user_id: i64, nickname: String) -> Self {
        Self {
            id,
            user_id,
            nickname
        }
    }
}