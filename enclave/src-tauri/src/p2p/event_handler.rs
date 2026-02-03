use libp2p::{PeerId};
use std::collections::HashMap;
use tokio::sync::mpsc;
use crate::db;
use crate::p2p::types::*;
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
        swarm: &mut libp2p::Swarm<EnclaveNetworkBehaviour>
    ) {
        log::info!("Connected to peer: {peer_id}");
        let _ = self.event_sender.send(P2PEvent::PeerConnected(peer_id));

        let multiaddr = match endpoint {
            libp2p_core::connection::ConnectedPoint::Dialer { address, .. } => address.clone(),
            libp2p_core::connection::ConnectedPoint::Listener { send_back_addr, .. } => send_back_addr.clone()
        };

        if let Err(err) = db::create_user(db::DATABASE.clone(), peer_id.to_string(), multiaddr.to_string()) {
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
}