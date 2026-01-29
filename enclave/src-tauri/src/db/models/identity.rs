use chrono::{DateTime, Utc};

#[derive(Debug)]
pub struct Identity {
    pub id: i64,
    pub keypair: Vec<u8>,
    pub peer_id: String,
    pub created_at: DateTime<Utc>
}

impl Identity {
    pub fn new(id: i64, keypair: Vec<u8>, peer_id: String, created_at: DateTime<Utc>) -> Self {
        Self {
            id,
            keypair,    
            peer_id,
            created_at
        }
    }
}