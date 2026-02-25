use std::collections::HashMap;

use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot::Sender;

use crate::db::models::{direct_message::DirectMessage, post::Post};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SynchRequest {
    pub since: i64,
    pub sender: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SynchResponse {
    pub created_posts: Vec<Post>,
    pub edited_posts: Vec<Post>,
    pub sender: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FriendRequest {
    pub from_peer_id: String,
    pub from_multiaddr: String,
    pub message: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FriendRequestResponse {
    pub accepted: bool,
    pub multiaddr: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MyInfo {
    pub peer_id: String,
    pub keypair: Vec<u8>,
    pub multiaddr: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum P2PMessage {
    FriendRequest(FriendRequest),
    FriendRequestResponse(FriendRequestResponse),
    DirectMessage(DirectMessage),
    SynchRequest(SynchRequest),
    SynchResponse(SynchResponse)
}

#[derive(Debug, Clone)]
pub enum P2PEvent {
    DirectMessageReceived(DirectMessage),
    DirectMessageSent(DirectMessage),
    PostRecieved(Post),
    PostSent(Post),
    PeerConnected(PeerId),
    PeerDisconnected(PeerId),
    FriendRequestReceived { from: PeerId, request: FriendRequest },
    FriendRequestAccepted { peer: PeerId },
    FriendRequestDenied { peer: PeerId },
    Error { context: &'static str, error: String },
    PostSynch
}

pub(crate) enum SwarmCommand {
    SendPost(String),
    SendDirectMessage { peer: PeerId, address: libp2p::Multiaddr, content: String },
    SendFriendRequest { peer: PeerId, address: libp2p::Multiaddr, message: String },
    AcceptFriendRequest(PeerId),
    DenyFriendRequest(PeerId),
    GetFriendList(Sender<Vec<PeerId>>),
    GetInboundFriendRequests(Sender<HashMap<PeerId, FriendRequest>>),
    GetDirectMessages { sender: Sender<Vec<DirectMessage>>, peer_id: PeerId },
    LoadFeed(Sender<Vec<Post>>),
    LoadBoard { sender: Sender<Vec<Post>>, peer_id: PeerId },
    ConnectToRelay(libp2p::Multiaddr)
}