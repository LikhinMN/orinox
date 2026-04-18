use clap::{Parser, ValueEnum};
use libp2p::PeerId;
use orinox::identity::get_or_create_identity;

#[derive(ValueEnum, Debug, Clone)]
enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

#[derive(Parser, Debug)]
#[command(version, about = "Orinox - P2P Networking Engine", long_about = None)]
struct Args {
    /// Port to listen on
    #[arg(short, long)]
    port: u16,

    /// Peer addresses to connect to (multiaddr format)
    #[arg(short, long)]
    connect: Vec<String>,

    /// Logging level
    #[arg(short = 'l', long, value_enum, default_value_t = LogLevel::Info)]
    log_level: LogLevel,
}


fn main() {
    let keypair = match get_or_create_identity() {
        Ok(keypair) => keypair,
        Err(e) => {
            eprintln!("Failed to initialize identity: {e}");
            std::process::exit(1);
        }
    };

    let peer_id = PeerId::from_public_key(&keypair.public());
    println!("Local peer id: {peer_id}");
}
