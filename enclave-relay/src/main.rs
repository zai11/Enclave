use std::{fs, path::Path};

use libp2p::{
    PeerId, SwarmBuilder, futures::StreamExt, identity, noise, relay, swarm::SwarmEvent, tcp, yamux
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let key_file = "relay_key.bin";

    let local_key = if Path::new(key_file).exists() {
        println!("Loading existing keypair...");
        let bytes = fs::read(key_file)?;
        identity::Keypair::from_protobuf_encoding(&bytes)?
    } else {
        println!("Generating new keypair...");
        let key = identity::Keypair::generate_ed25519();
        let bytes = key.to_protobuf_encoding()?;
        fs::write(key_file, bytes)?;
        key
    };

    let local_peer_id = PeerId::from(local_key.public());

    println!("Relay Peer ID: {}", local_peer_id);

    let relay_behaviour = relay::Behaviour::new(local_peer_id, Default::default());

    let mut swarm = SwarmBuilder::with_existing_identity(local_key)
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default
        )?
        .with_behaviour(|_| relay_behaviour)?
        .build();

    swarm.listen_on("/ip4/0.0.0.0/tcp/4001".parse()?)?;

    println!("Relay server started");

    loop {
        match swarm.select_next_some().await {
            SwarmEvent::NewListenAddr { address, .. } => {
                println!("Listening on {}", address);
            },
            SwarmEvent::Behaviour(event) => {
                println!("Relay event: {:?}", event);
            },
            _ => {}
        }
    }
}
