use clap::{Parser, ValueEnum};
use futures::StreamExt;
use libp2p::gossipsub;
use libp2p::swarm::SwarmEvent;
use libp2p::Multiaddr;
use libp2p::PeerId;
use orinox::behaviour::GOSSIPSUB_TOPIC;
use orinox::identity::get_or_create_identity;
use orinox::swarm::{create_swarm, OrinoxSwarm};

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

fn try_publish_hello(
    swarm: &mut OrinoxSwarm,
    chat_topic: &gossipsub::IdentTopic,
    hello_message: &str,
) -> bool {
    match swarm
        .behaviour_mut()
        .publish(chat_topic.clone(), hello_message.as_bytes())
    {
        Ok(_) => {
            println!("Published hello message");
            true
        }
        Err(gossipsub::PublishError::InsufficientPeers) => false,
        Err(e) => {
            eprintln!("Failed to publish hello message: {e}");
            false
        }
    }
}


#[tokio::main]
async fn main() {
    let args = Args::parse();
    let port = args.port;
    let connect_urls = args.connect;
    let log_level = args.log_level;
    println!("Starting orinox with port: {port}");
    println!("Starting orinox connection urls: {connect_urls:?}");
    println!("Starting orinox logs: {log_level:?}");

    let keypair = match get_or_create_identity() {
        Ok(keypair) => keypair,
        Err(e) => {
            eprintln!("Failed to initialize identity: {e}");
            std::process::exit(1);
        }
    };

    let local_peer_id = PeerId::from_public_key(&keypair.public());
    println!("Local peer id: {local_peer_id}");

    let chat_topic = gossipsub::IdentTopic::new(GOSSIPSUB_TOPIC);
    let chat_topic_hash = chat_topic.hash();
    let hello_message = format!("Hello from {local_peer_id}");
    let mut hello_published = false;

    let mut swarm = match create_swarm(&keypair) {
        Ok(swarm) => swarm,
        Err(e) => {
            eprintln!("Failed to create swarm: {e}");
            std::process::exit(1);
        }
    };

    let listen_addr: Multiaddr = match format!("/ip4/0.0.0.0/tcp/{port}").parse() {
        Ok(addr) => addr,
        Err(e) => {
            eprintln!("Invalid listen address for port {port}: {e}");
            std::process::exit(1);
        }
    };

    if let Err(e) = swarm.listen_on(listen_addr.clone()) {
        eprintln!("Failed to start listening on {listen_addr}: {e}");
        std::process::exit(1);
    }
    println!("Swarm listening on {listen_addr}");

    for connect_url in connect_urls {
        let remote_addr: Multiaddr = match connect_url.parse() {
            Ok(addr) => addr,
            Err(e) => {
                eprintln!("Skipping invalid --connect address '{connect_url}': {e}");
                continue;
            }
        };

        println!("Dialing {remote_addr}");
        if let Err(e) = swarm.dial(remote_addr.clone()) {
            eprintln!("Failed to dial {remote_addr}: {e}");
        }
    }

    loop {
        match swarm.next().await {
            Some(SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. }) => {
                println!("Connection established with {peer_id} via {endpoint:?}");
            }
            Some(SwarmEvent::Behaviour(gossipsub::Event::Subscribed { peer_id, topic })) => {
                println!("Peer {peer_id} subscribed to {topic}");
                if !hello_published && topic == chat_topic_hash {
                    hello_published = try_publish_hello(&mut swarm, &chat_topic, &hello_message);
                }
            }
            Some(SwarmEvent::Behaviour(gossipsub::Event::Message {
                propagation_source,
                message,
                ..
            })) => {
                let sender = message
                    .source
                    .map(|peer_id| peer_id.to_string())
                    .unwrap_or_else(|| propagation_source.to_string());
                let content = String::from_utf8_lossy(&message.data);
                println!("Received from {sender}: {content}");
            }
            Some(SwarmEvent::OutgoingConnectionError { peer_id, error, .. }) => {
                match peer_id {
                    Some(peer_id) => eprintln!("Outgoing connection error to {peer_id}: {error}"),
                    None => eprintln!("Outgoing connection error: {error}"),
                }
            }
            Some(SwarmEvent::IncomingConnectionError { send_back_addr, error, .. }) => {
                eprintln!("Incoming connection error from {send_back_addr}: {error}");
            }
            Some(SwarmEvent::NewListenAddr { address, .. }) => {
                println!("Listening on {address}");
            }
            Some(_) => {}
            None => {
                eprintln!("Swarm event stream ended");
                break;
            }
        }
    }


}
