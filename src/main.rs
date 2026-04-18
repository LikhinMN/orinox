use clap::{Parser, ValueEnum};
use libp2p::PeerId;
use orinox::behaviour::create_behaviour;
use orinox::identity::get_or_create_identity;
use orinox::transport::build_tcp_transport;

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
    let args = Args::parse();
    let port = args.port;
    let connect_rul = args.connect;
    let log_level = args.log_level;
    println!("Starting orinox with port: {port}");
    println!("Starting orinox connection urls: {connect_rul:?}");
    println!("Starting orinox logs: {log_level:?}");

    let keypair = match get_or_create_identity() {
        Ok(keypair) => keypair,
        Err(e) => {
            eprintln!("Failed to initialize identity: {e}");
            std::process::exit(1);
        }
    };

    let peer_id = PeerId::from_public_key(&keypair.public());
    println!("Local peer id: {peer_id}");

    if let Err(e) = build_tcp_transport(&keypair) {
        eprintln!("Failed to build TCP transport: {e}");
        std::process::exit(1);
    }

    println!("TCP transport initialized");

    let _behaviour = create_behaviour();
    println!("Behaviour initialized");


}
