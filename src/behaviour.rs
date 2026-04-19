use libp2p::gossipsub;
use libp2p::identity::Keypair;
use std::io;

pub type OrinoxBehaviour = gossipsub::Behaviour;
pub const GOSSIPSUB_TOPIC: &str = "orinox-global";

pub fn create_behaviour(keypair: &Keypair) -> Result<OrinoxBehaviour, io::Error> {
    let config = gossipsub::ConfigBuilder::default()
        .build()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

    let mut behaviour = gossipsub::Behaviour::new(
        gossipsub::MessageAuthenticity::Signed(keypair.clone()),
        config,
    )
    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

    let topic = gossipsub::IdentTopic::new(GOSSIPSUB_TOPIC);
    behaviour
        .subscribe(&topic)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

    Ok(behaviour)
}
