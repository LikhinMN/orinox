use libp2p::gossipsub;
use libp2p::identity::Keypair;
use std::io;

pub type OrinoxBehaviour = gossipsub::Behaviour;

pub fn create_behaviour(keypair: &Keypair) -> Result<OrinoxBehaviour, io::Error> {
    let config = gossipsub::ConfigBuilder::default()
        .build()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

    gossipsub::Behaviour::new(gossipsub::MessageAuthenticity::Signed(keypair.clone()), config)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
}

