use libp2p::{
    Multiaddr, PeerId, StreamProtocol, Transport, dcutr, futures::StreamExt, gossipsub, identity::{self, Keypair}, noise, ping, relay, request_response as reqres, swarm::{NetworkBehaviour, SwarmEvent}, tcp, yamux
};
use libp2p_core::connection::ConnectedPoint;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr, sync::Arc, time::Duration, u64};
use tokio::sync::{Mutex, mpsc};

use crate::db;

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

#[derive(NetworkBehaviour)]
struct EnclaveNetworkBehaviour {
    gossipsub: gossipsub::Behaviour,
    request_response: reqres::cbor::Behaviour<P2PMessage, P2PMessage>,
    relay_client: relay::client::Behaviour,
    dcutr: dcutr::Behaviour,
    ping: ping::Behaviour
}

enum SwarmCommand {
    SendMessage(String),
    SendDirectMessage { peer: PeerId, content: String },
    SendFriendRequest { peer: PeerId, address: Multiaddr, message: String },
    AcceptFriendRequest(PeerId),
    DenyFriendRequest(PeerId),
    GetFriendList(tokio::sync::oneshot::Sender<Vec<PeerId>>),
    GetInboundFriendRequests(tokio::sync::oneshot::Sender<HashMap<PeerId, FriendRequest>>),
    ConnectToRelay(Multiaddr)
}

pub struct P2PNode {
    peer_id: PeerId,
    keypair: Keypair,
    listen_addresses: Arc<Mutex<Vec<Multiaddr>>>,
    relay_address: Arc<Mutex<Option<Multiaddr>>>,
    swarm_sender: mpsc::UnboundedSender<SwarmCommand>
}

impl P2PNode {
    pub async fn new(relay_address: Option<String>) -> anyhow::Result<(Self, mpsc::UnboundedReceiver<P2PEvent>)> {
        let (keypair, peer_id, port_number) = {
            if let Ok(identity_data) = db::fetch_identity(db::DATABASE.clone()) {
                log::info!("DEBUG: Called");
                let local_key = Keypair::from_protobuf_encoding(&identity_data.keypair)?;
                let peer_id = PeerId::from_str(&identity_data.peer_id)?;
                let port_number = identity_data.port_number;
                (local_key, peer_id, port_number)
            }
            else {
                let keypair = identity::Keypair::generate_ed25519();
                let peer_id = PeerId::from(keypair.clone().public());
                let port_number = rand::rng().random_range(49152..65535);

                db::create_identity(db::DATABASE.clone(), keypair.to_protobuf_encoding()?, peer_id.to_string(), port_number)?;

                (keypair, peer_id, port_number)
            }
        };

        log::info!("Local peer id: {peer_id}");

        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(1))
            .validation_mode(gossipsub::ValidationMode::Strict)
            .build()
            .map_err(|e| anyhow::anyhow!("Gossipsub config error: {e}"))?;

        let gossipsub = gossipsub::Behaviour::new(
            gossipsub::MessageAuthenticity::Signed(keypair.clone()),
            gossipsub_config
        ).map_err(|err| anyhow::anyhow!("Gossipsub behaviour error: {err}"))?;

        let request_response = reqres::cbor::Behaviour::new(
            [(StreamProtocol::new("/enclave/1.0.0"), reqres::ProtocolSupport::Full)],
            reqres::Config::default()
        );

        let (relay_transport, relay_client) = relay::client::new(peer_id);

        let dcutr = dcutr::Behaviour::new(peer_id);
        let ping = ping::Behaviour::new(ping::Config::new());

        let behaviour = EnclaveNetworkBehaviour {
            gossipsub,
            request_response,
            relay_client,
            dcutr,
            ping
        };

        let mut swarm = libp2p::SwarmBuilder::with_existing_identity(keypair.clone())
            .with_tokio()
            .with_tcp(
                tcp::Config::default(),
                noise::Config::new,
                yamux::Config::default
            )?
            .with_other_transport(|key| {
                relay_transport
                    .upgrade(libp2p::core::upgrade::Version::V1)
                    .authenticate(noise::Config::new(key).unwrap())
                    .multiplex(yamux::Config::default())
            })
            .map_err(|err| anyhow::anyhow!("Error adding relay transport to swarm: {err}"))?
            .with_behaviour(|_| behaviour)
            .map_err(|err| anyhow::anyhow!("Error adding behaviour to swarm: {err}"))?
            .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(u64::MAX)))
            .build();

        swarm.listen_on(format!("/ip4/0.0.0.0/tcp/{port_number}").parse()?)?;

        let topic = gossipsub::IdentTopic::new("enclave-messages");
        swarm.behaviour_mut().gossipsub.subscribe(&topic)?;

        let (event_sender, event_receiver) = mpsc::unbounded_channel();
        let (swarm_sender, mut swarm_receiver) = mpsc::unbounded_channel::<SwarmCommand>();

        let listen_addresses = Arc::new(Mutex::new(Vec::new()));
        let listen_addresses_clone = listen_addresses.clone();
        let relay_addr = Arc::new(Mutex::new(None));
        let relay_addr_clone = relay_addr.clone();

        if let Some(relay_str) = relay_address {
            if let Ok(addr) = relay_str.parse::<Multiaddr>() {
                log::info!("Connecting to relay: {}", addr);
                swarm.dial(addr.clone())?;
                *relay_addr_clone.lock().await = Some(addr);
            }
        }

        // Wait for first listening address before continuing:
        let first_address = loop {
            match swarm.select_next_some().await {
                SwarmEvent::NewListenAddr { address, .. } => {
                    log::info!("Listening on {address}");
                    break address;
                },
                _ => continue,
            }
        };

        listen_addresses_clone.lock().await.push(first_address);

        tokio::spawn(async move {
            let mut friend_list: Vec<PeerId> = db::fetch_all_friends(db::DATABASE.clone())
                .unwrap_or_else(|err| {
                    let _ = event_sender.send(P2PEvent::Error {
                        context: "fetch_all_friends",
                        error: err.to_string(),
                    });
                    Vec::new()
                })
                .into_iter()
                .filter_map(|friend| {
                    match db::fetch_user_by_id(db::DATABASE.clone(), friend.user_id) {
                        Ok(user) => match PeerId::from_str(&user.peer_id) {
                            Ok(peer_id) => Some(peer_id),
                            Err(err) => {
                                let _ = event_sender.send(P2PEvent::Error {
                                    context: "parse_peer_id",
                                    error: err.to_string(),
                                });
                                None
                            }
                        },
                        Err(err) => {
                            let _ = event_sender.send(P2PEvent::Error {
                                context: "fetch_user_by_id",
                                error: err.to_string(),
                            });
                            None
                        }
                    }
                })
                .collect();

            let inbound_friend_requests: HashMap<PeerId, FriendRequest> = db::fetch_all_friend_requests(db::DATABASE.clone())
                .unwrap_or_else(|err| {
                    let _ = event_sender.send(P2PEvent::Error {
                        context: "fetch_all_friend_requests",
                        error: err.to_string(),
                    });
                    Vec::new()
                })
                .into_iter()
                .filter_map(|req| {
                    match db::fetch_user_by_id(db::DATABASE.clone(), req.user_id) {
                        Ok(user) => {
                            let peer_id = PeerId::from_str(user.peer_id.as_str()).unwrap();
                            let req = FriendRequest {
                                from_peer_id: user.peer_id,
                                from_multiaddr: user.multiaddr,
                                message: req.message
                            };
                            Some((peer_id, req))
                        },
                        Err(err) => {
                            let _ = event_sender.send(P2PEvent::Error {
                                context: "fetch_user_by_id",
                                error: err.to_string(),
                            });
                            None
                        }
                    }
                })
                .collect();

            let mut outbound_friend_requests = HashMap::new();
            let mut pending_friend_request_responses = HashMap::new();

            loop {
                tokio::select! {
                    event = swarm.select_next_some() => {
                        match event {
                            SwarmEvent::Behaviour(EnclaveNetworkBehaviourEvent::Gossipsub(gossip_event)) => {
                                if let gossipsub::Event::Message { propagation_source, message, .. } = gossip_event {
                                    if !friend_list.contains(&propagation_source) {
                                        continue;
                                    }

                                    if let Ok(msg) = serde_json::from_slice::<DirectMessage>(&message.data) {
                                        let _ = event_sender.send(P2PEvent::MessageReceived(msg));
                                    }
                                }
                            },
                            SwarmEvent::Behaviour(EnclaveNetworkBehaviourEvent::RequestResponse(req_event)) => {
                                match req_event {
                                    reqres::Event::Message { peer, message, .. } => {
                                        match message {
                                            reqres::Message::Request { request, channel, .. } => {
                                                match request {
                                                    P2PMessage::FriendRequest(req) => {
                                                        log::info!("Received friend request from {}: {}", peer, req.message);
                                                        let _ = event_sender.send(P2PEvent::FriendRequestReceived {
                                                            from: peer,
                                                            request: req.clone()
                                                        });

                                                        let user = match db::fetch_user_by_peer_id(db::DATABASE.clone(), peer.to_string()) {
                                                            Ok(u) => u,
                                                            Err(err) => {
                                                                let _ = event_sender.send(P2PEvent::Error { 
                                                                    context: "fetch_user_by_peer_id", 
                                                                    error: err.to_string() 
                                                                });
                                                                continue;
                                                            }
                                                        };

                                                        if let Err(err) = db::create_friend_request(db::DATABASE.clone(), user.id, req.message) {
                                                            let _ = event_sender.send(P2PEvent::Error {
                                                                context: "create_friend_request",
                                                                error: err.to_string()
                                                            });
                                                            continue;
                                                        }

                                                        // Send back an empty acknowledgement so the request doesn't hang
                                                        // But don't auto-accept - wait for user action
                                                        let ack = P2PMessage::DirectMessage(DirectMessage {
                                                            from: "system".into(),
                                                            content: "request_received".into(),
                                                            timestamp: 0
                                                        });

                                                        let _ = swarm.behaviour_mut().request_response.send_response(channel, ack);
                                                    },
                                                    P2PMessage::FriendRequestResponse(response) => {
                                                        log::info!("Received friend request response from {}: accepted={}", peer, response.accepted);
                                                        if response.accepted {
                                                            if !friend_list.contains(&peer) {
                                                                let friend_count = match db::fetch_all_friends(db::DATABASE.clone()) {
                                                                    Ok(f) => f.len(),
                                                                    Err(_) => 0
                                                                };
                                                                log::info!("Before friend count: {friend_count}");
                                                                
                                                                let user = match db::fetch_user_by_peer_id(db::DATABASE.clone(), peer.to_string()) {
                                                                    Ok(u) => u,
                                                                    Err(err) => {
                                                                        let _ = event_sender.send(P2PEvent::Error { 
                                                                            context: "fetch_user_by_peer_id", 
                                                                            error: err.to_string() 
                                                                        });
                                                                        continue;
                                                                    }
                                                                };

                                                                if let Err(err) = db::create_friend(db::DATABASE.clone(), user.id) {
                                                                    let _ = event_sender.send(P2PEvent::Error {
                                                                        context: "create_friend",
                                                                        error: err.to_string()
                                                                    });
                                                                    continue;
                                                                }

                                                                let friend_count = match db::fetch_all_friends(db::DATABASE.clone()) {
                                                                    Ok(f) => f.len(),
                                                                    Err(_) => 0
                                                                };
                                                                log::info!("After friend count: {friend_count}");

                                                                friend_list.push(peer);
                                                                swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer);
                                                            }
                                                            
                                                            let _ = event_sender.send(P2PEvent::FriendRequestAccepted { peer });
                                                        }
                                                        else {
                                                            let _ = event_sender.send(P2PEvent::FriendRequestDenied { peer });
                                                        }
                                                    },
                                                    P2PMessage::DirectMessage(msg) => {
                                                        if msg.from == "system" {
                                                            return;
                                                        }

                                                        if !friend_list.contains(&peer) {
                                                            log::info!("Ignoring DM from non-friend: {}", peer);
                                                            return;
                                                        }

                                                        let _ = event_sender.send(P2PEvent::MessageReceived(msg));
                                                    }
                                                }
                                            },
                                            reqres::Message::Response { request_id, response } => {
                                                log::info!("Received response for request {:?}: {:?}", request_id, response);
                                            }
                                        }
                                    },
                                    reqres::Event::OutboundFailure { peer, request_id, error, .. } => {
                                        log::info!("Outbound request {:?} to {} failed {:?}", request_id, peer, error);
                                    },
                                    reqres::Event::InboundFailure { peer, request_id, error, .. } => {
                                        log::info!("Inbound request {:?} to {} failed {:?}", request_id, peer, error);
                                    },
                                    _ => {}
                                }
                            },
                            SwarmEvent::Behaviour(EnclaveNetworkBehaviourEvent::RelayClient(event)) => {
                                log::info!("Relay client event: {:?}", event);
                            },
                            SwarmEvent::Behaviour(EnclaveNetworkBehaviourEvent::Dcutr(event)) => {
                                log::info!("DCUTR event {:?}", event);
                            },
                            SwarmEvent::Behaviour(EnclaveNetworkBehaviourEvent::Ping(event)) => {
                                log::info!("Ping event {:?}", event);
                            },
                            SwarmEvent::NewListenAddr { address, .. } => {
                                log::info!("Listening on {address}");
                                listen_addresses_clone.lock().await.push(address);
                            },
                            SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
                                log::info!("Connected to peer: {peer_id}");
                                let _ = event_sender.send(P2PEvent::PeerConnected(peer_id));

                                let multiaddr = match &endpoint {
                                    ConnectedPoint::Dialer { address, .. } => address.clone(),
                                    ConnectedPoint::Listener { send_back_addr, .. } => send_back_addr.clone()
                                };

                                if let Err(err) = db::create_user(db::DATABASE.clone(), peer_id.to_string(), multiaddr.to_string()) {
                                    let _ = event_sender.send(P2PEvent::Error {
                                        context: "create_user",
                                        error: err.to_string()
                                    });
                                }

                                if let Some(pending_request) = outbound_friend_requests.remove(&peer_id) {
                                    log::info!("Sending buffered friend request to {}", peer_id);
                                    let request = P2PMessage::FriendRequest(pending_request);
                                    swarm.behaviour_mut().request_response.send_request(&peer_id, request);
                                }

                                if let Some(pending_response) = pending_friend_request_responses.remove(&peer_id) {
                                    log::info!("Sending buffered friend request response to {}", peer_id);
                                    swarm.behaviour_mut().request_response.send_request(&peer_id, pending_response);
                                }
                            },
                            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                                log::info!("Disconnected from peer: {peer_id}");
                                let _ = event_sender.send(P2PEvent::PeerDisconnected(peer_id));
                            },
                            _ => {}
                        }
                    }

                    Some(cmd) = swarm_receiver.recv() => {
                        match cmd {
                            SwarmCommand::SendDirectMessage { peer, content } => {
                                // It may be good to accept direct messages from non-friend users but have them quarantined
                                // separate from messages from friends. In that case, we can remove this condition.
                                if !friend_list.contains(&peer) {
                                    continue;
                                }

                                let message = P2PMessage::DirectMessage(
                                    DirectMessage {
                                        from: swarm.local_peer_id().to_string(),
                                        content,
                                        timestamp: std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .unwrap()
                                            .as_secs()
                                    }
                                );

                                swarm.behaviour_mut().request_response.send_request(&peer, message);
                            }
                            SwarmCommand::SendMessage(content) => {
                                let topic = gossipsub::IdentTopic::new("enclave-messages");
                                let message = DirectMessage {
                                    from: swarm.local_peer_id().to_string(),
                                    content,
                                    timestamp: std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap()
                                        .as_secs()
                                };

                                if let Ok(data) = serde_json::to_vec(&message) {
                                    let _ = swarm.behaviour_mut().gossipsub.publish(topic, data);
                                }
                            },
                            SwarmCommand::SendFriendRequest { peer, address, message } => {
                                log::info!("Buffering friend request to: {peer} at: {address}");

                                let local_addresses = listen_addresses_clone.lock().await;
                                let relay_addr_opt = relay_addr_clone.lock().await;

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

                                outbound_friend_requests.insert(peer, request);

                                if let Err(err) = swarm.dial(address) {
                                    log::error!("Failed to dial peer {}: {}", peer, err);
                                    outbound_friend_requests.remove(&peer);
                                }
                            },
                            SwarmCommand::AcceptFriendRequest(peer) => {
                                log::info!("Accepting friend request from: {}", peer);
                                if !friend_list.contains(&peer) {
                                    let user = match db::fetch_user_by_peer_id(db::DATABASE.clone(), peer.to_string()) {
                                        Ok(u) => u,
                                        Err(err) => {
                                            let _ = event_sender.send(P2PEvent::Error { 
                                                context: "fetch_user_by_peer_id", 
                                                error: err.to_string() 
                                            });
                                            continue;
                                        }
                                    };

                                    log::info!("{}, {}", peer.to_string(), user.peer_id);

                                    if let Err(err) = db::create_friend(db::DATABASE.clone(), user.id) {
                                        let _ = event_sender.send(P2PEvent::Error {
                                            context: "create_friend",
                                            error: err.to_string()
                                        });
                                        continue;
                                    }

                                    let friend_request = match db::fetch_friend_request_by_user_id(db::DATABASE.clone(), user.id) {
                                        Ok(fr) => fr,
                                        Err(err) => {
                                            let _ = event_sender.send(P2PEvent::Error {
                                                context: "fetch_friend_request_by_user_id",
                                                error: err.to_string()
                                            });
                                            continue;
                                        }
                                    };

                                    if let Err(err) = db::delete_friend_request(db::DATABASE.clone(), friend_request.id) {
                                        let _ = event_sender.send(P2PEvent::Error {
                                            context: "delete_friend_request",
                                            error: err.to_string()
                                        });
                                        continue;
                                    }

                                    friend_list.push(peer);
                                    swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer);
                                }
                                let local_addresses = listen_addresses_clone.lock().await;
                                let relay_addr_opt = relay_addr_clone.lock().await;

                                let address_to_send = if let Some(relay) = relay_addr_opt.as_ref() {
                                    format!("{}/p2p-circuit/p2p/{}", relay, swarm.local_peer_id())
                                } else {
                                    local_addresses.first().map(|a| a.to_string()).unwrap_or_default()
                                };

                                let response = P2PMessage::FriendRequestResponse(
                                    FriendRequestResponse{
                                        accepted: true,
                                        multiaddr: address_to_send
                                    }
                                );

                                if swarm.is_connected(&peer) {
                                    log::info!("Already connected to {}, sending acceptance immediately", peer);
                                    swarm.behaviour_mut().request_response.send_request(&peer, response);
                                } else {
                                    log::info!("Not connected to {}, dialing before sending acceptance", peer);

                                    let user = match db::fetch_user_by_peer_id(db::DATABASE.clone(), peer.to_string()) {
                                        Ok(u) => u,
                                        Err(err) => {
                                            let _ = event_sender.send(P2PEvent::Error { 
                                                context: "fetch_user_by_peer_id", 
                                                error: err.to_string() 
                                            });
                                            continue;
                                        }
                                    };

                                    let peer_addr = match user.multiaddr.parse::<Multiaddr>() {
                                        Ok(addr) => addr,
                                        Err(err) => {
                                            let _ = event_sender.send(P2PEvent::Error {
                                                context: "parse::<Multiaddress>",
                                                error: err.to_string()
                                            });
                                            continue;
                                        }
                                    };

                                    pending_friend_request_responses.insert(peer, response);

                                    if let Err(err) = swarm.dial(peer_addr) {
                                        let _ = event_sender.send(P2PEvent::Error {
                                            context: "swarm.dial",
                                            error: err.to_string()
                                        });
                                        pending_friend_request_responses.remove(&peer);
                                    }
                                }
                            },
                            SwarmCommand::DenyFriendRequest(peer) => {
                                let user = match db::fetch_user_by_peer_id(db::DATABASE.clone(), peer.to_string()) {
                                    Ok(u) => u,
                                    Err(err) => {
                                        let _ = event_sender.send(P2PEvent::Error { 
                                            context: "fetch_user_by_peer_id", 
                                            error: err.to_string() 
                                        });
                                        continue;
                                    }
                                };

                                let friend_request = match db::fetch_friend_request_by_user_id(db::DATABASE.clone(), user.id) {
                                    Ok(fr) => fr,
                                    Err(err) => {
                                        let _ = event_sender.send(P2PEvent::Error {
                                            context: "fetch_friend_request_by_user_id",
                                            error: err.to_string()
                                        });
                                        continue;
                                    }
                                };

                                if let Err(err) = db::delete_friend_request(db::DATABASE.clone(), friend_request.id) {
                                    let _ = event_sender.send(P2PEvent::Error {
                                        context: "delete_friend_request",
                                        error: err.to_string()
                                    });
                                    continue;
                                }

                                let response = P2PMessage::FriendRequestResponse(
                                    FriendRequestResponse {
                                        accepted: false,
                                        multiaddr: String::new()
                                    }
                                );
                                swarm.behaviour_mut().request_response.send_request(&peer, response);
                            },
                            SwarmCommand::GetFriendList(sender) => {
                                let _ = sender.send(friend_list.clone());
                            },
                            SwarmCommand::GetInboundFriendRequests(sender) => {
                                let _ = sender.send(inbound_friend_requests.clone());
                            }
                            SwarmCommand::ConnectToRelay(address) => {
                                log::info!("Connecting to relay: {}", address);
                                let _ = swarm.dial(address.clone());
                                *relay_addr_clone.lock().await = Some(address);
                            }
                        }
                    }
                }
            }
        });

        log::info!("Finished starting P2P node.");

        Ok((
            Self {
                peer_id: peer_id,
                keypair: keypair,
                listen_addresses,
                relay_address: relay_addr,
                swarm_sender
            },
            event_receiver
        ))
    }

    pub fn get_peer_id(&self) -> PeerId {
        self.peer_id
    }

    pub fn get_keypair(&self) -> Keypair {
        self.keypair.clone()
    }

    pub async fn get_listen_addresses(&self) -> Vec<Multiaddr> {
        let mut addresses = self.listen_addresses.lock().await.clone();

        if let Some(relay) = self.relay_address.lock().await.as_ref() {
            let relay_circuit = format!("{}/p2p-circuit/p2p/{}", relay, self.peer_id)
                .parse()
                .ok();
            if let Some(circuit_address) = relay_circuit {
                addresses.push(circuit_address);
            }
        }

        addresses
    }

    pub fn send_direct_message(&self, peer: PeerId, content: String) -> anyhow::Result<()> {
        self.swarm_sender.send(SwarmCommand::SendDirectMessage { peer, content })?;
        Ok(())
    }

    pub fn send_message(&self, content: String) -> anyhow::Result<()> {
        self.swarm_sender.send(SwarmCommand::SendMessage(content))?;
        Ok(())
    }

    pub fn send_friend_request(&self, peer: PeerId, address: Multiaddr, message: String) -> anyhow::Result<()> {
        self.swarm_sender.send(SwarmCommand::SendFriendRequest { peer, address, message })?;
        Ok(())
    }

    pub fn accept_friend_request(&self, peer: PeerId) -> anyhow::Result<()> {
        self.swarm_sender.send(SwarmCommand::AcceptFriendRequest(peer))?;
        Ok(())
    }

    pub fn deny_friend_request(&self, peer: PeerId) -> anyhow::Result<()> {
        self.swarm_sender.send(SwarmCommand::DenyFriendRequest(peer))?;
        Ok(())
    }

    pub async fn get_friend_list(&self) -> anyhow::Result<Vec<PeerId>> {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        self.swarm_sender.send(SwarmCommand::GetFriendList(sender))?;
        Ok(receiver.await?)
    }

    pub async fn get_inbound_friend_requests(&self) -> anyhow::Result<HashMap<PeerId, FriendRequest>> {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        self.swarm_sender.send(SwarmCommand::GetInboundFriendRequests(sender))?;
        Ok(receiver.await?)
    }

    pub fn connect_to_relay(&self, address: Multiaddr) -> anyhow::Result<()> {
        self.swarm_sender.send(SwarmCommand::ConnectToRelay(address))?;
        Ok(())
    }
}