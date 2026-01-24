use chrono::{DateTime, Utc};
use libp2p::PeerId;

#[derive(Debug)]
pub struct Identity {
    pub id: i64,
    pub keypair: Vec<u8>,
    pub peer_id: PeerId,
    pub created_at: DateTime<Utc>
}

impl Identity {
    pub fn new(id: i64, keypair: Vec<u8>, peer_id: PeerId, created_at: DateTime<Utc>) -> Self {
        Self {
            id,
            keypair,    
            peer_id,
            created_at
        }
    }
}