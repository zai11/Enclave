pub mod command_handler;
pub mod config;
pub mod event_handler;
pub mod node;
pub mod types;

use libp2p::{Multiaddr, PeerId, Transport, futures::StreamExt, swarm::SwarmEvent};
use std::collections::HashMap;
use std::sync::Arc;
use std::str::FromStr;
use tokio::sync::{mpsc, Mutex};
use crate::db::{self, models::{direct_message::DirectMessage, post::Post}};

use config::{NetworkConfig, create_swarm_behaviour};
use event_handler::EventHandler;
use command_handler::CommandHandler;
use types::{SwarmCommand};

pub use types::{FriendRequest, FriendRequestResponse, P2PMessage, P2PEvent, MyInfo};
pub use node::P2PNode;

impl P2PNode {
    pub async fn new(relay_address: Option<String>) -> anyhow::Result<(Self, mpsc::UnboundedReceiver<P2PEvent>)> {
        let config = NetworkConfig::load_or_create()?;
        log::info!("Local peer id: {}", config.peer_id);

        let (behaviour, relay_transport) = create_swarm_behaviour(&config.keypair, config.peer_id)?;
        
        let mut swarm = libp2p::SwarmBuilder::with_existing_identity(config.keypair.clone())
            .with_tokio()
            .with_tcp(
                libp2p::tcp::Config::default(),
                libp2p::noise::Config::new,
                libp2p::yamux::Config::default,
            )?
            .with_other_transport(|key| {
                relay_transport
                    .upgrade(libp2p::core::upgrade::Version::V1)
                    .authenticate(libp2p::noise::Config::new(key).unwrap())
                    .multiplex(libp2p::yamux::Config::default())
            })
            .map_err(|err| anyhow::anyhow!("Error adding relay transport: {err}"))?
            .with_behaviour(|_| behaviour)
            .map_err(|err| anyhow::anyhow!("Error adding behaviour: {err}"))?
            .with_swarm_config(|c| {
                c.with_idle_connection_timeout(std::time::Duration::from_secs(u64::MAX))
            })
            .build();

        swarm.listen_on(format!("/ip4/0.0.0.0/tcp/{}", config.port).parse()?)?;

        let topic = libp2p::gossipsub::IdentTopic::new("enclave-messages");
        swarm.behaviour_mut().gossipsub.subscribe(&topic)?;

        let (event_sender, event_receiver) = mpsc::unbounded_channel();
        let (swarm_sender, swarm_receiver) = mpsc::unbounded_channel::<SwarmCommand>();

        let listen_addresses = Arc::new(Mutex::new(Vec::new()));
        let relay_addr = Arc::new(Mutex::new(None));

        if let Some(relay_str) = relay_address {
            if let Ok(addr) = relay_str.parse::<Multiaddr>() {
                log::info!("Connecting to relay: {}", addr);
                swarm.dial(addr.clone())?;
                *relay_addr.lock().await = Some(addr);
            }
        }

        let first_address = loop {
            match swarm.select_next_some().await {
                SwarmEvent::NewListenAddr { address, .. } => {
                    log::info!("Listening on {address}");
                    break address;
                }
                _ => continue,
            }
        };
        listen_addresses.lock().await.push(first_address);

        spawn_event_loop(
            swarm,
            swarm_receiver,
            event_sender.clone(),
            listen_addresses.clone(),
            relay_addr.clone(),
        )
        .await;

        log::info!("Finished starting P2P node.");

        Ok((
            Self {
                peer_id: config.peer_id,
                keypair: config.keypair,
                listen_addresses,
                relay_address: relay_addr,
                swarm_sender,
            },
            event_receiver,
        ))
    }
}

async fn spawn_event_loop(
    mut swarm: libp2p::Swarm<config::EnclaveNetworkBehaviour>,
    mut swarm_receiver: mpsc::UnboundedReceiver<SwarmCommand>,
    event_sender: mpsc::UnboundedSender<P2PEvent>,
    listen_addresses: Arc<Mutex<Vec<Multiaddr>>>,
    relay_addr: Arc<Mutex<Option<Multiaddr>>>,
) {
    tokio::spawn(async move {
        let mut friend_list = load_friend_list(&event_sender);
        let inbound_friend_requests = load_inbound_requests(&event_sender);
        let mut direct_messages = HashMap::new();
        let mut outbound_friend_requests = HashMap::new();
        let mut pending_friend_request_responses = HashMap::new();
        let mut outbound_direct_messages = HashMap::new();

        let event_handler = EventHandler::new(event_sender.clone());

        loop {
            tokio::select! {
                event = swarm.select_next_some() => {
                    handle_swarm_event(
                        event,
                        &mut friend_list,
                        &mut direct_messages,
                        &mut outbound_direct_messages,
                        &mut outbound_friend_requests,
                        &mut pending_friend_request_responses,
                        &event_handler,
                        &mut swarm,
                        &listen_addresses,
                    )
                    .await;
                },
                Some(cmd) = swarm_receiver.recv() => {
                    handle_swarm_command(
                        cmd,
                        &mut friend_list,
                        &inbound_friend_requests,
                        &mut outbound_friend_requests,
                        &mut pending_friend_request_responses,
                        &mut direct_messages,
                        &mut outbound_direct_messages,
                        &mut swarm,
                        &listen_addresses,
                        &relay_addr,
                        &event_sender,
                    )
                    .await;
                }
            }
        }
    });
}

async fn handle_swarm_event(
    event: SwarmEvent<config::EnclaveNetworkBehaviourEvent>,
    friend_list: &mut Vec<PeerId>,
    direct_messages: &mut HashMap<PeerId, Vec<DirectMessage>>,
    outbound_direct_messages: &mut HashMap<PeerId, Vec<DirectMessage>>,
    outbound_requests: &mut HashMap<PeerId, FriendRequest>,
    pending_responses: &mut HashMap<PeerId, P2PMessage>,
    event_handler: &EventHandler,
    swarm: &mut libp2p::Swarm<config::EnclaveNetworkBehaviour>,
    listen_addresses: &Arc<Mutex<Vec<Multiaddr>>>
) {
    use config::EnclaveNetworkBehaviourEvent;
    
    match event {
        SwarmEvent::Behaviour(EnclaveNetworkBehaviourEvent::Gossipsub(gossip_event)) => {
            if let libp2p::gossipsub::Event::Message { propagation_source, message, .. } = gossip_event {
                if friend_list.contains(&propagation_source) {
                    if let Ok(msg) = serde_json::from_slice::<DirectMessage>(&message.data) {
                        let _ = event_handler.event_sender.send(P2PEvent::DirectMessageReceived(msg));
                    }
                }
            }
        },
        SwarmEvent::Behaviour(EnclaveNetworkBehaviourEvent::RequestResponse(req_event)) => {
            use libp2p::request_response as reqres;
            
            match req_event {
                reqres::Event::Message { peer, message, .. } => {
                    if let reqres::Message::Request { request, channel, .. } = message {
                        match request {
                            P2PMessage::FriendRequest(req) => {
                                event_handler.handle_friend_request(peer, req);
                                
                                /*let ack = P2PMessage::DirectMessage(DirectMessage {
                                    from: "system".into(),
                                    content: "request_received".into(),
                                    timestamp: 0
                                });
                                let _ = swarm.behaviour_mut().request_response.send_response(channel, ack);*/
                            },
                            P2PMessage::FriendRequestResponse(response) => {
                                event_handler.handle_friend_request_response(peer, response, friend_list, swarm);
                            },
                            P2PMessage::DirectMessage(msg) => {
                                event_handler.handle_direct_message(msg, friend_list, direct_messages);
                            }
                        }
                    }
                },
                reqres::Event::OutboundFailure { peer, request_id, error, .. } => {
                    log::info!("Outbound request {:?} to {} failed {:?}", request_id, peer, error);
                },
                reqres::Event::InboundFailure { peer, request_id, error, .. } => {
                    log::info!("Inbound request {:?} from {} failed {:?}", request_id, peer, error);
                },
                _ => {}
            }
        },
        SwarmEvent::Behaviour(EnclaveNetworkBehaviourEvent::Ping(event)) => {
            log::info!("Ping event {:?}", event);
        },
        SwarmEvent::Behaviour(EnclaveNetworkBehaviourEvent::RelayClient(event)) => {
            log::info!("Relay client event: {:?}", event);
        },
        SwarmEvent::Behaviour(EnclaveNetworkBehaviourEvent::Dcutr(event)) => {
            log::info!("DCUTR event {:?}", event);
        },
        SwarmEvent::NewListenAddr { address, .. } => {
            log::info!("Listening on {address}");
            listen_addresses.lock().await.push(address);
        },
        SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
            event_handler
                .handle_connection_established(
                    peer_id,
                    &endpoint,
                    outbound_requests,
                    pending_responses,
                    outbound_direct_messages,
                    swarm
                )
                .await;
        },
        SwarmEvent::ConnectionClosed { peer_id, .. } => {
            log::info!("Disconnected from peer: {peer_id}");
            let _ = event_handler.event_sender.send(P2PEvent::PeerDisconnected(peer_id));
        },
        _ => {}
    }
}

async fn handle_swarm_command(
    cmd: SwarmCommand,
    friend_list: &mut Vec<PeerId>,
    inbound_requests: &HashMap<PeerId, FriendRequest>,
    outbound_requests: &mut HashMap<PeerId, FriendRequest>,
    pending_responses: &mut HashMap<PeerId, P2PMessage>,
    direct_messages: &mut HashMap<PeerId, Vec<DirectMessage>>,
    outbound_direct_messages: &mut HashMap<PeerId, Vec<DirectMessage>>,
    swarm: &mut libp2p::Swarm<config::EnclaveNetworkBehaviour>,
    listen_addresses: &Arc<Mutex<Vec<Multiaddr>>>,
    relay_addr: &Arc<Mutex<Option<Multiaddr>>>,
    event_sender: &mpsc::UnboundedSender<P2PEvent>
) {
    match cmd {
        SwarmCommand::SendMessage(content) => {
            /*let topic = libp2p::gossipsub::IdentTopic::new("enclave-posts");

            if let Ok(data) = serde_json::to_vec(&post) {
                let _ = swarm.behaviour_mut().gossipsub.publish(topic, data);
            }*/
        },
        SwarmCommand::SendDirectMessage { peer, address, content } => {
            CommandHandler::handle_send_direct_message(
                peer, 
                address, 
                content, 
                friend_list, 
                outbound_direct_messages, 
                swarm,
                event_sender
            )
            .await;
        },
        SwarmCommand::SendFriendRequest { peer, address, message } => {
            CommandHandler::handle_send_friend_request(
                peer,
                address,
                message,
                outbound_requests,
                listen_addresses,
                relay_addr,
                swarm
            )
            .await;
        },
        SwarmCommand::AcceptFriendRequest(peer) => {
            CommandHandler::handle_accept_friend_request(
                peer,
                friend_list,
                pending_responses,
                listen_addresses,
                relay_addr,
                swarm,
                event_sender
            )
            .await;
        },
        SwarmCommand::DenyFriendRequest(peer) => {
            let user = match db::fetch_user_by_peer_id(db::DATABASE.clone(), peer.to_string()) {
                Ok(u) => u,
                Err(err) => {
                    let _ = event_sender.send(P2PEvent::Error {
                        context: "fetch_user_by_peer_id",
                        error: err.to_string(),
                    });
                    return;
                }
            };

            if let Ok(friend_request) = db::fetch_friend_request_by_from_user_id(db::DATABASE.clone(), user.id) {
                let _ = db::delete_friend_request(db::DATABASE.clone(), friend_request.id);
            }

            let response = P2PMessage::FriendRequestResponse(FriendRequestResponse {
                accepted: false,
                multiaddr: String::new()
            });

            swarm.behaviour_mut().request_response.send_request(&peer, response);
        },
        SwarmCommand::GetFriendList(sender) => {
            let _ = sender.send(friend_list.clone());
        },
        SwarmCommand::GetInboundFriendRequests(sender) => {
            let _ = sender.send(inbound_requests.clone());
        },
        SwarmCommand::GetDirectMessages { sender, peer_id } => {
            let direct_messages_with_peer = match db::fetch_direct_messages_with_peer(db::DATABASE.clone(), peer_id.to_string()) {
                Ok(dms) => dms,
                Err(err) => {
                    let _ = event_sender.send(P2PEvent::Error { context: "fetch_direct_message_with_user", error: err.to_string() });
                    vec![]
                }
            };
            direct_messages.insert(peer_id, direct_messages_with_peer);
            let peer_direct_messages = match direct_messages.get(&peer_id) {
                Some(dms) => dms.to_owned(),
                None => {
                    log::warn!("Attempted to get direct messages from Peer '{}' but none were found.", peer_id.to_string());
                    vec![]
                }
            };
            
            let _ = sender.send(peer_direct_messages);
        }
        SwarmCommand::ConnectToRelay(address) => {
            log::info!("Connecting to relay: {}", address);
            let _ = swarm.dial(address.clone());
            *relay_addr.lock().await = Some(address);
        }
    }
}

fn load_friend_list(event_sender: &mpsc::UnboundedSender<P2PEvent>) -> Vec<PeerId> {
    db::fetch_all_friends(db::DATABASE.clone())
        .unwrap_or_else(|err| {
            let _ = event_sender.send(P2PEvent::Error {
                context: "fetch_all_friends",
                error: err.to_string()
            });
            Vec::new()
        })
        .into_iter()
        .filter_map(|friend| {
            db::fetch_user_by_id(db::DATABASE.clone(), friend.user_id)
                .ok()
                .and_then(|user| PeerId::from_str(&user.peer_id).ok())
        })
        .collect()
}

fn load_inbound_requests(event_sender: &mpsc::UnboundedSender<P2PEvent>) -> HashMap<PeerId, FriendRequest> {
    db::fetch_all_friend_requests(db::DATABASE.clone())
        .unwrap_or_else(|err| {
            let _ = event_sender.send(P2PEvent::Error {
                context: "fetch_all_friend_requests",
                error: err.to_string()
            });
            Vec::new()
        })
        .into_iter()
        .filter_map(|req| {
            db::fetch_user_by_id(db::DATABASE.clone(), req.from_user_id)
                .ok()
                .and_then(|user| {
                    PeerId::from_str(&user.peer_id)
                        .ok()
                        .map(|peer_id| {
                            (
                                peer_id,
                                FriendRequest {
                                    from_peer_id: user.peer_id,
                                    from_multiaddr: user.multiaddr,
                                    message: req.message
                                }
                            )
                        })
                })
        })
        .collect()
}