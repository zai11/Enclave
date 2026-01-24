use libp2p::{Multiaddr, PeerId};

#[derive(Debug)]
pub struct User {
    pub id: i64,
    pub peer_id: PeerId,
    pub multiaddr: Multiaddr
}

impl User {
    pub fn new(id: i64, peer_id: PeerId, multiaddr: Multiaddr) -> Self {
        Self {
            id,
            peer_id,
            multiaddr
        }
    }
}