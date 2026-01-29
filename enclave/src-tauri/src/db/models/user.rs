#[derive(Debug)]
pub struct User {
    pub id: i64,
    pub peer_id: String,
    pub multiaddr: String
}

impl User {
    pub fn new(id: i64, peer_id: String, multiaddr: String) -> Self {
        Self {
            id,
            peer_id,
            multiaddr
        }
    }
}