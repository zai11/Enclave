use libp2p::{Multiaddr, PeerId, identity::Keypair};
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::{mpsc, Mutex};
use crate::p2p::types::*;

pub struct P2PNode {
    pub peer_id: PeerId,
    pub keypair: Keypair,
    pub listen_addresses: Arc<Mutex<Vec<Multiaddr>>>,
    pub relay_address: Arc<Mutex<Option<Multiaddr>>>,
    pub swarm_sender: mpsc::UnboundedSender<SwarmCommand>
}

impl P2PNode {
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