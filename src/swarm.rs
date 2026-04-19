use crate::behaviour::{OrinoxBehaviour, create_behaviour};
use crate::transport::build_tcp_transport;
use libp2p::swarm::{Config, Swarm};
use std::io;

pub type OrinoxSwarm = Swarm<OrinoxBehaviour>;

pub fn create_swarm(keypair: &libp2p::identity::Keypair) -> Result<OrinoxSwarm, io::Error> {
    let peer_id = libp2p::PeerId::from_public_key(&keypair.public());
    let transport = build_tcp_transport(keypair)?;
    let behaviour = create_behaviour(keypair)?;

    Ok(Swarm::new(
        transport,
        behaviour,
        peer_id,
        Config::with_tokio_executor(),
    ))
}
