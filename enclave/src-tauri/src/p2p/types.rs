use libp2p::PeerId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectMessage {
    pub from: String,
    pub content: String,
    pub timestamp: u64
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriendRequest {
    pub from_peer_id: String,
    pub from_multiaddr: String,
    pub message: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriendRequestResponse {
    pub accepted: bool,
    pub multiaddr: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyInfo {
    pub peer_id: String,
    pub keypair: Vec<u8>,
    pub multiaddr: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum P2PMessage {
    FriendRequest(FriendRequest),
    FriendRequestResponse(FriendRequestResponse),
    DirectMessage(DirectMessage)
}

#[derive(Debug, Clone)]
pub enum P2PEvent {
    MessageReceived(DirectMessage),
    PeerConnected(PeerId),
    PeerDisconnected(PeerId),
    FriendRequestReceived { from: PeerId, request: FriendRequest },
    FriendRequestAccepted { peer: PeerId },
    FriendRequestDenied { peer: PeerId },
    Error { context: &'static str, error: String }
}

pub(crate) enum SwarmCommand {
    SendMessage(String),
    SendDirectMessage { peer: PeerId, content: String },
    SendFriendRequest { peer: PeerId, address: libp2p::Multiaddr, message: String },
    AcceptFriendRequest(PeerId),
    DenyFriendRequest(PeerId),
    GetFriendList(tokio::sync::oneshot::Sender<Vec<PeerId>>),
    GetInboundFriendRequests(tokio::sync::oneshot::Sender<std::collections::HashMap<PeerId, FriendRequest>>),
    ConnectToRelay(libp2p::Multiaddr)
}