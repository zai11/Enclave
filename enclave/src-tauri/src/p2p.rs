use libp2p::{
    Multiaddr, PeerId, StreamProtocol, Transport, dcutr, futures::StreamExt, gossipsub, identity, noise, relay, request_response as reqres, swarm::{NetworkBehaviour, SwarmEvent}, tcp, yamux
};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};
use tokio::sync::{Mutex, mpsc};

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
pub struct FriendInfo {
    pub peer_id: String,
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
    FriendRequestDenied { peer: PeerId }
}

#[derive(NetworkBehaviour)]
struct EnclaveNetworkBehaviour {
    gossipsub: gossipsub::Behaviour,
    request_response: reqres::cbor::Behaviour<P2PMessage, P2PMessage>,
    relay_client: relay::client::Behaviour,
    dcutr: dcutr::Behaviour
}

enum SwarmCommand {
    SendMessage(String),
    SendDirectMessage { peer: PeerId, content: String },
    SendFriendRequest { peer: PeerId, address: Multiaddr, message: String },
    AcceptFriendRequest(PeerId),
    DenyFriendRequest(PeerId),
    GetFriendList(tokio::sync::oneshot::Sender<Vec<PeerId>>),
    ConnectToRelay(Multiaddr)
}

pub struct P2PNode {
    peer_id: PeerId,
    listen_addresses: Arc<Mutex<Vec<Multiaddr>>>,
    relay_address: Arc<Mutex<Option<Multiaddr>>>,
    swarm_sender: mpsc::UnboundedSender<SwarmCommand>
}

impl P2PNode {
    pub async fn new(relay_address: Option<String>) -> anyhow::Result<(Self, mpsc::UnboundedReceiver<P2PEvent>)> {
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());
        println!("Local peer id: {local_peer_id}");

        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(1))
            .validation_mode(gossipsub::ValidationMode::Strict)
            .build()
            .map_err(|e| anyhow::anyhow!("Gossipsub config error: {e}"))?;

        let gossipsub = gossipsub::Behaviour::new(
            gossipsub::MessageAuthenticity::Signed(local_key.clone()),
            gossipsub_config
        ).map_err(|err| anyhow::anyhow!("Gossipsub behaviour error: {err}"))?;

        let request_response = reqres::cbor::Behaviour::new(
            [(StreamProtocol::new("/enclave/1.0.0"), reqres::ProtocolSupport::Full)],
            reqres::Config::default()
        );

        let (relay_transport, relay_client) = relay::client::new(local_peer_id);

        let dcutr = dcutr::Behaviour::new(local_peer_id);

        let behaviour = EnclaveNetworkBehaviour {
            gossipsub,
            request_response,
            relay_client,
            dcutr
        };

        let mut swarm = libp2p::SwarmBuilder::with_existing_identity(local_key)
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
            .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
            .build();

        swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

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
                println!("Connecting to relay: {}", addr);
                swarm.dial(addr.clone())?;
                *relay_addr_clone.lock().await = Some(addr);
            }
        }

        // Wait for first listening address before continuing:
        let first_address = loop {
            match swarm.select_next_some().await {
                SwarmEvent::NewListenAddr { address, .. } => {
                    println!("Listening on {address}");
                    break address;
                },
                _ => continue,
            }
        };

        listen_addresses_clone.lock().await.push(first_address);

        tokio::spawn(async move {
            let mut friend_list: Vec<PeerId> = Vec::new();

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
                                if let reqres::Event::Message { peer, message, .. } = req_event {
                                    if let reqres::Message::Request { request, .. } = message {
                                        match request {
                                            P2PMessage::FriendRequest(req) => {
                                                let _ = event_sender.send(P2PEvent::FriendRequestReceived {
                                                    from: peer,
                                                    request: req
                                                });
                                            },
                                            P2PMessage::FriendRequestResponse(response) => {
                                                if response.accepted {
                                                    if !friend_list.contains(&peer) {
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
                                                if !friend_list.contains(&peer) {
                                                    continue;
                                                }
                                                let _ = event_sender.send(P2PEvent::MessageReceived(msg));
                                            }
                                        }
                                    }
                                }
                            },
                            SwarmEvent::Behaviour(EnclaveNetworkBehaviourEvent::RelayClient(event)) => {
                                println!("Relay client event: {:?}", event);
                            },
                            SwarmEvent::Behaviour(EnclaveNetworkBehaviourEvent::Dcutr(event)) => {
                                println!("DCUTR event {:?}", event);
                            }
                            SwarmEvent::NewListenAddr { address, .. } => {
                                println!("Listening on {address}");
                                listen_addresses_clone.lock().await.push(address);
                            },
                            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                                println!("Connected to peer: {peer_id}");
                                let _ = event_sender.send(P2PEvent::PeerConnected(peer_id));
                            },
                            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                                println!("Disconnected from peer: {peer_id}");
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
                                let _ = swarm.dial(address);
                                let local_addresses = listen_addresses_clone.lock().await;
                                let relay_addr_opt = relay_addr_clone.lock().await;

                                let address_to_send = if let Some(relay) = relay_addr_opt.as_ref() {
                                    format!("{}/p2p-circuit/p2p/{}", relay, swarm.local_peer_id())
                                } else {
                                    local_addresses.first().map(|a| a.to_string()).unwrap_or_default()
                                };

                                let request = P2PMessage::FriendRequest(
                                    FriendRequest {
                                        from_peer_id: swarm.local_peer_id().to_string(),
                                        from_multiaddr: address_to_send,
                                        message
                                    }
                                );
                                swarm.behaviour_mut().request_response.send_request(&peer, request);
                            },
                            SwarmCommand::AcceptFriendRequest(peer) => {
                                if !friend_list.contains(&peer) {
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
                                swarm.behaviour_mut().request_response.send_request(&peer, response);
                            },
                            SwarmCommand::DenyFriendRequest(peer) => {
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
                            SwarmCommand::ConnectToRelay(address) => {
                                println!("Connecting to relay: {}", address);
                                let _ = swarm.dial(address.clone());
                                *relay_addr_clone.lock().await = Some(address);
                            }
                        }
                    }
                }
            }
        });

        Ok((
            Self {
                peer_id: local_peer_id,
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

    pub fn connect_to_relay(&self, address: Multiaddr) -> anyhow::Result<()> {
        self.swarm_sender.send(SwarmCommand::ConnectToRelay(address))?;
        Ok(())
    }
}