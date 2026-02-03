use libp2p::relay::client::Transport;
use libp2p::{identity::Keypair, PeerId, StreamProtocol, gossipsub, relay, dcutr, ping, request_response as reqres, swarm::NetworkBehaviour};
use rand::Rng;
use std::str::FromStr;
use std::time::Duration;
use crate::db;
use crate::p2p::types::P2PMessage;

#[derive(NetworkBehaviour)]
pub struct EnclaveNetworkBehaviour {
    pub gossipsub: gossipsub::Behaviour,
    pub request_response: reqres::cbor::Behaviour<P2PMessage, P2PMessage>,
    pub relay_client: relay::client::Behaviour,
    pub dcutr: dcutr::Behaviour,
    pub ping: ping::Behaviour
}

pub struct NetworkConfig {
    pub keypair: Keypair,
    pub peer_id: PeerId,
    pub port: i64
}

impl NetworkConfig {
    pub fn load_or_create() -> anyhow::Result<Self> {
        if let Ok(identity_data) = db::fetch_identity(db::DATABASE.clone()) {
            log::info!("Loading existing identity");
            let keypair = Keypair::from_protobuf_encoding(&identity_data.keypair)?;
            let peer_id = PeerId::from_str(&identity_data.peer_id)?;
            let port = identity_data.port_number;
            Ok(Self { keypair, peer_id, port })
        } else {
            log::info!("Creating new identity");
            let keypair = libp2p::identity::Keypair::generate_ed25519();
            let peer_id = PeerId::from(keypair.public());
            let port = rand::rng().random_range(49152..65535);
            
            db::create_identity(
                db::DATABASE.clone(),
                keypair.to_protobuf_encoding()?,
                peer_id.to_string(),
                port
            )?;
            
            Ok(Self { keypair, peer_id, port })
        }
    }
}

pub fn create_swarm_behaviour(keypair: &Keypair, peer_id: PeerId) -> anyhow::Result<(EnclaveNetworkBehaviour, Transport)> {
    let gossipsub_config = gossipsub::ConfigBuilder::default()
        .heartbeat_interval(Duration::from_secs(1))
        .validation_mode(gossipsub::ValidationMode::Strict)
        .build()
        .map_err(|e| anyhow::anyhow!("Gossipsub config error: {e}"))?;

    let gossipsub = gossipsub::Behaviour::new(
        gossipsub::MessageAuthenticity::Signed(keypair.clone()),
        gossipsub_config
    ).map_err(|err| anyhow::anyhow!(err.to_string()))?;

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

    Ok((behaviour, relay_transport))
}