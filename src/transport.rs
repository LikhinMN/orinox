use libp2p::core::muxing::StreamMuxerBox;
use libp2p::core::transport::Boxed;
use libp2p::core::upgrade;
use libp2p::identity::Keypair;
use libp2p::{PeerId, Transport, noise, tcp, yamux};
use std::io;

pub fn build_tcp_transport(
    keypair: &Keypair,
) -> Result<Boxed<(PeerId, StreamMuxerBox)>, io::Error> {
    let noise_config = noise::Config::new(keypair)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

    let transport = tcp::tokio::Transport::new(tcp::Config::default().nodelay(true))
        .upgrade(upgrade::Version::V1)
        .authenticate(noise_config)
        .multiplex(yamux::Config::default())
        .boxed();

    Ok(transport)
}
