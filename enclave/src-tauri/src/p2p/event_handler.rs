use libp2p::request_response::ResponseChannel;
use libp2p::{PeerId};
use std::collections::HashMap;
use std::str::FromStr;
use tokio::sync::mpsc;
use crate::db;
use crate::db::models::direct_message::DirectMessage;
use crate::db::models::post::Post;
use crate::p2p::{types::*};
use crate::p2p::config::EnclaveNetworkBehaviour;

pub struct EventHandler {
    pub event_sender: mpsc::UnboundedSender<P2PEvent>
}

impl EventHandler {
    pub fn new(event_sender: mpsc::UnboundedSender<P2PEvent>) -> Self {
        Self { event_sender }
    }

    pub async fn handle_connection_established(
        &self,
        peer_id: PeerId,
        endpoint: &libp2p_core::connection::ConnectedPoint,
        outbound_requests: &mut HashMap<PeerId, FriendRequest>,
        pending_responses: &mut HashMap<PeerId, P2PMessage>,
        outbound_direct_messages: &mut HashMap<PeerId, Vec<DirectMessage>>,
        swarm: &mut libp2p::Swarm<EnclaveNetworkBehaviour>
    ) {
        log::info!("Connected to peer: {peer_id}");
        let _ = self.event_sender.send(P2PEvent::PeerConnected(peer_id));

        let multiaddr = match endpoint {
            libp2p_core::connection::ConnectedPoint::Dialer { address, .. } => address.clone(),
            libp2p_core::connection::ConnectedPoint::Listener { send_back_addr, .. } => send_back_addr.clone()
        };

        if let Err(err) = db::create_user(db::DATABASE.clone(), peer_id.to_string(), multiaddr.to_string(), false) {
            let _ = self.event_sender.send(P2PEvent::Error {
                context: "create_user",
                error: err.to_string()
            });
        }

        if let Some(request) = outbound_requests.remove(&peer_id) {
            log::info!("Sending buffered friend request to {}", peer_id);
            swarm.behaviour_mut()
                .request_response
                .send_request(&peer_id, P2PMessage::FriendRequest(request));
        }

        if let Some(response) = pending_responses.remove(&peer_id) {
            log::info!("Sending buffered friend request response to {}", peer_id);
            swarm.behaviour_mut()
                .request_response
                .send_request(&peer_id, response);
        }

        if let Some(dms) = outbound_direct_messages.remove(&peer_id) {
            log::info!("Sending {} buffered direct messages to {}", dms.len(), peer_id);
            dms.iter().for_each(|dm| {
                swarm.behaviour_mut()
                    .request_response
                    .send_request(&peer_id, P2PMessage::DirectMessage(dm.to_owned()));
            });
        }
    }

    pub fn handle_friend_request(
        &self,
        peer: PeerId,
        request: FriendRequest
    ) {
        log::info!("Received friend request from {}: {}", peer, request.message);
        
        let _ = self.event_sender.send(P2PEvent::FriendRequestReceived {
            from: peer,
            request: request.clone()
        });

        let user = match db::fetch_user_by_peer_id(db::DATABASE.clone(), peer.to_string()) {
            Ok(u) => u,
            Err(err) => {
                let _ = self.event_sender.send(P2PEvent::Error {
                    context: "fetch_user_by_peer_id",
                    error: err.to_string()
                });
                return;
            }
        };

        if let Err(err) = db::create_friend_request(db::DATABASE.clone(), user.id, request.message) {
            let _ = self.event_sender.send(P2PEvent::Error {
                context: "create_friend_request",
                error: err.to_string()
            });
        }
    }

    pub fn handle_friend_request_response(
        &self,
        peer: PeerId,
        response: FriendRequestResponse,
        friend_list: &mut Vec<PeerId>,
        swarm: &mut libp2p::Swarm<EnclaveNetworkBehaviour>
    ) {
        log::info!("Received friend request response from {}: accepted={}", peer, response.accepted);
        
        if response.accepted {
            if !friend_list.contains(&peer) {
                let user = match db::fetch_user_by_peer_id(db::DATABASE.clone(), peer.to_string()) {
                    Ok(u) => u,
                    Err(err) => {
                        let _ = self.event_sender.send(P2PEvent::Error {
                            context: "fetch_user_by_peer_id",
                            error: err.to_string()
                        });
                        return;
                    }
                };

                if let Err(err) = db::create_friend(db::DATABASE.clone(), user.id) {
                    let _ = self.event_sender.send(P2PEvent::Error {
                        context: "create_friend",
                        error: err.to_string()
                    });
                    return;
                }

                friend_list.push(peer);
                swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer);
            }

            let _ = self.event_sender.send(P2PEvent::FriendRequestAccepted { peer });
        } else {
            let _ = self.event_sender.send(P2PEvent::FriendRequestDenied { peer });
        }
    }

    pub fn handle_direct_message(
        &self,
        msg: DirectMessage,
        friend_list: &Vec<PeerId>,
        direct_messages: &mut HashMap<PeerId, Vec<DirectMessage>>
    ) {
        log::info!("Received direct message '{}' from {}", msg.content, msg.from_peer_id);

        let from_peer_id = match PeerId::from_str(&msg.from_peer_id) {
            Ok(p) => p,
            Err(err) => {
                let _ = self.event_sender.send(P2PEvent::Error { context: "PeerId::from_str", error: err.to_string() });
                return;
            }
        };

        let identity_peer_id = match db::fetch_identity(db::DATABASE.clone()) {
            Ok(id) => id.peer_id,
            Err(err) => {
                let _ = self.event_sender.send(P2PEvent::Error { context: "fetch_identity", error: err.to_string() });
                return;
            }
        };

        if friend_list.contains(&from_peer_id) {
            if let Err(err) = db::create_direct_message(db::DATABASE.clone(), msg.from_peer_id.clone(), identity_peer_id, msg.content.clone()) {
                let _ = self.event_sender.send(P2PEvent::Error { context: "create_direct_message", error: err.to_string() });
            }

            let mut current_messages = direct_messages.remove(&from_peer_id).unwrap_or(vec![]);
            current_messages.push(msg.clone());

            direct_messages.insert(from_peer_id, current_messages);

            let _ = self.event_sender.send(P2PEvent::DirectMessageReceived(msg));
        }
    }

    pub fn handle_post(
        &self,
        src_peer_id: PeerId,
        post: Post,
        friend_list: &Vec<PeerId>,
        displayed_posts: &mut Vec<Post>,
    ) {
        log::info!("Received post '{}' from {}", post.content, post.author_peer_id);

        if !friend_list.contains(&src_peer_id) {
            log::warn!("Post received from non-friend peer.");
            return;
        }

        if let Err(err) = db::create_post(db::DATABASE.clone(), post.author_peer_id.clone(), post.content.clone()) {
            let _ = self.event_sender.send(P2PEvent::Error { context: "create_post", error: err.to_string() });
            return;
        };

        displayed_posts.push(post.clone());

        let _ = self.event_sender.send(P2PEvent::PostRecieved(post));
    }

    pub fn handle_synch_request(
        &mut self, 
        since: i64, 
        sender: String,
        swarm: &mut libp2p::Swarm<EnclaveNetworkBehaviour>, 
        channel: ResponseChannel<P2PMessage>
    ) {
        log::info!("Received synch request from '{}', since: {}", sender, since);
        let posts = match db::fetch_all_posts(db::DATABASE.clone()) {
            Ok(p) => p,
            Err(err) => {
                let _ = self.event_sender.send(P2PEvent::Error { context: "fetch_all_posts", error: err.to_string() });
                vec![]
            }
        };

        let created_posts = posts.iter().filter(|&p| p.created_at >= since).cloned().collect::<Vec<Post>>();
        let edited_posts = posts.iter().filter(|&p| p.edited_at >= Some(since)).cloned().collect::<Vec<Post>>();

        let sender = swarm.local_peer_id().to_string();

        if let Err(err) = swarm.behaviour_mut().request_response.send_response(
            channel,
            P2PMessage::SynchResponse(SynchResponse { created_posts, edited_posts, sender })
        ) {
            let _ = self.event_sender.send(P2PEvent::Error { context: "send_response", error: format!("{:?}", err) });
        }
    }

    pub fn handle_synch_response(&self, created_posts: Vec<Post>, edited_posts: Vec<Post>, sender: String) {
        log::info!("Received synch response from '{}'", sender);
        log::info!("created_posts length: {}, edited_posts length: {}", created_posts.len(), edited_posts.len());
        for post in created_posts {
            if let Err(err) = db::create_post(db::DATABASE.clone(), post.author_peer_id, post.content) {
                let _ = self.event_sender.send(P2PEvent::Error { context: "create_post", error: err.to_string() });
            }
        }

        for post in edited_posts {
            if let Err(err) = db::update_post(db::DATABASE.clone(), post.id, post.content) {
                let _ = self.event_sender.send(P2PEvent::Error { context: "update_post", error: err.to_string() });
            }
        }

        let _ = self.event_sender.send(P2PEvent::PostSynch);
    }
}