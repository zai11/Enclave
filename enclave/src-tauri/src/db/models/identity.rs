use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    pub id: i64,
    pub keypair: Vec<u8>,
    pub peer_id: String,
    pub port_number: i64,
    pub created_at: i64
}

impl Identity {
    pub fn new(id: i64, keypair: Vec<u8>, peer_id: String, port_number: i64, created_at: i64) -> Self {
        Self {
            id,
            keypair,    
            peer_id,
            port_number,
            created_at
        }
    }
}