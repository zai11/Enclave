#[derive(Debug)]
pub struct Friend {
    pub id: i64,
    pub user_id: i64
}

impl Friend {
    pub fn new(id: i64, user_id: i64) -> Self {
        Self {
            id,
            user_id
        }
    }
}