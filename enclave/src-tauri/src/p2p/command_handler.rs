use libp2p::{PeerId, Multiaddr};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::db;
use crate::db::models::direct_message::DirectMessage;
use crate::p2p::types::*;
use crate::p2p::config::EnclaveNetworkBehaviour;

pub struct CommandHandler;

impl CommandHandler {
    pub async fn handle_send_friend_request(
        peer: PeerId,
        address: Multiaddr,
        message: String,
        outbound_requests: &mut HashMap<PeerId, FriendRequest>,
        listen_addrs: &Arc<Mutex<Vec<Multiaddr>>>,
        relay_addr: &Arc<Mutex<Option<Multiaddr>>>,
        swarm: &mut libp2p::Swarm<EnclaveNetworkBehaviour>
    ) {
        log::info!("Buffering friend request to: {peer} at: {address}");

        let local_addresses = listen_addrs.lock().await;
        let relay_addr_opt = relay_addr.lock().await;

        let address_to_send = if let Some(relay) = relay_addr_opt.as_ref() {
            format!("{}/p2p-circuit/p2p/{}", relay, swarm.local_peer_id())
        } else {
            local_addresses.first().map(|a| a.to_string()).unwrap_or_default()
        };

        let request = FriendRequest {
            from_peer_id: swarm.local_peer_id().to_string(),
            from_multiaddr: address_to_send,
            message
        };

        outbound_requests.insert(peer, request);

        if let Err(err) = swarm.dial(address) {
            log::error!("Failed to dial peer {}: {}", peer, err);
            outbound_requests.remove(&peer);
        }
    }

    pub async fn handle_accept_friend_request(
        peer: PeerId,
        friend_list: &mut Vec<PeerId>,
        pending_responses: &mut HashMap<PeerId, P2PMessage>,
        listen_addrs: &Arc<Mutex<Vec<Multiaddr>>>,
        relay_addr: &Arc<Mutex<Option<Multiaddr>>>,
        swarm: &mut libp2p::Swarm<EnclaveNetworkBehaviour>,
        event_sender: &tokio::sync::mpsc::UnboundedSender<P2PEvent>
    ) {
        log::info!("Accepting friend request from: {}", peer);

        if !friend_list.contains(&peer) {
            let user = match db::fetch_user_by_peer_id(db::DATABASE.clone(), peer.to_string()) {
                Ok(u) => u,
                Err(err) => {
                    let _ = event_sender.send(P2PEvent::Error {
                        context: "fetch_user_by_peer_id",
                        error: err.to_string()
                    });
                    return;
                }
            };

            if let Err(err) = db::create_friend(db::DATABASE.clone(), user.id) {
                let _ = event_sender.send(P2PEvent::Error {
                    context: "create_friend",
                    error: err.to_string()
                });
                return;
            }

            if let Ok(friend_request) = db::fetch_friend_request_by_from_user_id(db::DATABASE.clone(), user.id) {
                let _ = db::delete_friend_request(db::DATABASE.clone(), friend_request.id);
            }

            friend_list.push(peer);
            swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer);
        }

        let local_addresses = listen_addrs.lock().await;
        let relay_addr_opt = relay_addr.lock().await;

        let address_to_send = if let Some(relay) = relay_addr_opt.as_ref() {
            format!("{}/p2p-circuit/p2p/{}", relay, swarm.local_peer_id())
        } else {
            local_addresses.first().map(|a| a.to_string()).unwrap_or_default()
        };

        let response = P2PMessage::FriendRequestResponse(FriendRequestResponse {
            accepted: true,
            multiaddr: address_to_send
        });

        if swarm.is_connected(&peer) {
            log::info!("Already connected, sending acceptance immediately");
            swarm.behaviour_mut().request_response.send_request(&peer, response);
        } else {
            log::info!("Not connected, dialing before sending acceptance");
            
            let user = match db::fetch_user_by_peer_id(db::DATABASE.clone(), peer.to_string()) {
                Ok(u) => u,
                Err(err) => {
                    let _ = event_sender.send(P2PEvent::Error {
                        context: "fetch_user_by_peer_id",
                        error: err.to_string()
                    });
                    return;
                }
            };

            if let Ok(peer_addr) = user.multiaddr.parse::<Multiaddr>() {
                pending_responses.insert(peer, response);
                if let Err(err) = swarm.dial(peer_addr) {
                    let _ = event_sender.send(P2PEvent::Error {
                        context: "swarm.dial",
                        error: err.to_string()
                    });
                    pending_responses.remove(&peer);
                }
            }
        }
    }

    pub async fn handle_send_direct_message(
        peer_id: PeerId,
        address: Multiaddr,
        content: String,
        friend_list: &mut Vec<PeerId>,
        outbound_direct_messages: &mut HashMap<PeerId, Vec<DirectMessage>>,
        swarm: &mut libp2p::Swarm<EnclaveNetworkBehaviour>,
        event_sender: &tokio::sync::mpsc::UnboundedSender<P2PEvent>
    ) {
        log::info!("Sending direct message '{}' to {}", content, peer_id);
        if !friend_list.contains(&peer_id) {
            return;
        }

        let direct_message_id = match db::create_direct_message(db::DATABASE.clone(), swarm.local_peer_id().to_string(), peer_id.to_string(), content) {
            Ok(id) => id,
            Err(err) => {
                let _ = event_sender.send(P2PEvent::Error { context: "create_direct_message", error: err.to_string() });
                return;
            }
        };

        let message = match db::fetch_direct_message_by_id(db::DATABASE.clone(), direct_message_id) {
            Ok(dm) => dm,
            Err(err) => {
                let _ = event_sender.send(P2PEvent::Error { context: "fetch_direct_message_by_id", error: err.to_string() });
                return;
            }
        };

        let _ = event_sender.send(P2PEvent::DirectMessageSent(message.clone()));

        if swarm.is_connected(&peer_id) {
            log::info!("Already connected, sending direct message immediately");
            swarm.behaviour_mut().request_response.send_request(&peer_id, P2PMessage::DirectMessage(message));
        } else {
            log::info!("Not connected, dialing before sending acceptance");

            match outbound_direct_messages.get_mut(&peer_id) {
                Some(dms) => dms.push(message),
                None => {
                    outbound_direct_messages.insert(peer_id, vec![message]);
                }
            };
            if let Err(err) = swarm.dial(address) {
                let _ = event_sender.send(P2PEvent::Error {
                    context: "swarm.dial",
                    error: err.to_string()
                });
                match outbound_direct_messages.get_mut(&peer_id) {
                    Some(dms) => {
                        dms.pop();
                    },
                    None => ()
                }
            }
        }
    }
}