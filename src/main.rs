use clap::{Parser, ValueEnum};
use futures::StreamExt;
use libp2p::Multiaddr;
use libp2p::PeerId;
use libp2p::gossipsub;
use libp2p::swarm::SwarmEvent;
use orinox::behaviour::GOSSIPSUB_TOPIC;
use orinox::identity::get_or_create_identity;
use orinox::swarm::{OrinoxSwarm, create_swarm};
use std::collections::VecDeque;
use std::collections::HashMap;
use tokio::io::{self, AsyncBufReadExt, BufReader};
use tokio::time::{self, Duration, MissedTickBehavior};
use tracing_subscriber::EnvFilter;

const MSG_PREFIX: &str = "MSG|";
const NAME_PREFIX: &str = "NAME|";

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

fn encode_chat_message(name: &str, message: &str) -> String {
    format!("{MSG_PREFIX}{name}|{message}")
}

fn encode_name_update(name: &str) -> String {
    format!("{NAME_PREFIX}{name}")
}

fn parse_incoming_message(raw: &[u8]) -> Option<(Option<String>, String)> {
    let text = String::from_utf8_lossy(raw);
    if let Some(rest) = text.strip_prefix(MSG_PREFIX) {
        let mut parts = rest.splitn(2, '|');
        let name = parts.next().unwrap_or("unknown").to_string();
        let message = parts.next().unwrap_or("").to_string();
        return Some((Some(name), message));
    }

    if let Some(rest) = text.strip_prefix(NAME_PREFIX) {
        return Some((None, rest.to_string()));
    }

    Some((Some("peer".to_string()), text.to_string()))
}

fn format_peer_name(peer_id: &PeerId) -> String {
    let peer_text = peer_id.to_string();
    let short = peer_text.chars().take(8).collect::<String>();
    format!("peer-{short}")
}

fn print_system(message: &str) {
    println!("[system] {message}");
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
            print_system("Published hello message");
            true
        }
        Err(gossipsub::PublishError::InsufficientPeers) => false,
        Err(e) => {
            eprintln!("Failed to publish hello message: {e}");
            false
        }
    }
}

fn try_flush_pending_messages(
    swarm: &mut OrinoxSwarm,
    chat_topic: &gossipsub::IdentTopic,
    pending_messages: &mut VecDeque<String>,
) {
    let pending_count = pending_messages.len();
    for _ in 0..pending_count {
        let Some(message) = pending_messages.pop_front() else {
            break;
        };

        match swarm
            .behaviour_mut()
            .publish(chat_topic.clone(), message.as_bytes().to_vec())
        {
            Ok(_) => println!("[you]: {message} (sent)"),
            Err(gossipsub::PublishError::InsufficientPeers) => {
                pending_messages.push_front(message);
                break;
            }
            Err(e) => eprintln!("Failed to publish queued message: {e}"),
        }
    }
}

fn init_logging(log_level: &LogLevel) {
    let filter = match std::env::var("RUST_LOG") {
        Ok(value) => EnvFilter::new(value),
        Err(_) => {
            let level = match log_level {
                LogLevel::Error => "error",
                LogLevel::Warn => "warn",
                LogLevel::Info => "info",
                LogLevel::Debug => "debug",
                LogLevel::Trace => "trace",
            };
            EnvFilter::new(format!("orinox={level},libp2p={level}"))
        }
    };

    tracing_subscriber::fmt().with_env_filter(filter).init();
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let port = args.port;
    let connect_urls = args.connect;
    let log_level = args.log_level;
    init_logging(&log_level);
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
    let mut local_name = format_peer_name(&local_peer_id);
    let hello_message = encode_chat_message(&local_name, &format!("Hello from {local_name}"));
    let mut hello_published = false;
    let mut pending_messages = VecDeque::new();
    let mut peer_names: HashMap<PeerId, String> = HashMap::new();

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

    let stdin = io::stdin();
    let mut stdin_lines = BufReader::new(stdin).lines();
    let mut stdin_closed = false;
    let mut retry_interval = time::interval(Duration::from_millis(500));
    retry_interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
    print_system(&format!(
        "Type a message and press Enter to publish to {GOSSIPSUB_TOPIC} (commands: /name, /peers, /exit)"
    ));

    loop {
        tokio::select! {
            event = swarm.select_next_some() => {
                match event {
                    SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
                        print_system(&format!("Connection established with {peer_id} via {endpoint:?}"));
                        peer_names.entry(peer_id).or_insert_with(|| format_peer_name(&peer_id));
                    }
                    SwarmEvent::ConnectionClosed { peer_id, .. } => {
                        print_system(&format!("Connection closed with {peer_id}"));
                        peer_names.remove(&peer_id);
                    }
                    SwarmEvent::Behaviour(gossipsub::Event::Subscribed { peer_id, topic }) => {
                        print_system(&format!("Peer {peer_id} subscribed to {topic}"));
                        if topic == chat_topic_hash {
                            if !hello_published {
                                hello_published = try_publish_hello(&mut swarm, &chat_topic, &hello_message);
                            }
                        }
                    }
                    SwarmEvent::Behaviour(gossipsub::Event::Message {
                        propagation_source,
                        message,
                        ..
                    }) => {
                        if message.source.as_ref() == Some(&local_peer_id) {
                            continue;
                        }

                        let sender = message
                            .source
                            .unwrap_or(propagation_source);

                        match parse_incoming_message(&message.data) {
                            Some((Some(name), content)) => {
                                peer_names.insert(sender, name.clone());
                                if !content.is_empty() {
                                    println!("[{name}]: {content}");
                                }
                            }
                            Some((None, name)) => {
                                if !name.is_empty() {
                                    peer_names.insert(sender, name.clone());
                                    print_system(&format!("{sender} is now known as {name}"));
                                }
                            }
                            None => {}
                        }
                    }
                    SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                        match peer_id {
                            Some(peer_id) => eprintln!("Outgoing connection error to {peer_id}: {error}"),
                            None => eprintln!("Outgoing connection error: {error}"),
                        }
                    }
                    SwarmEvent::IncomingConnectionError { send_back_addr, error, .. } => {
                        eprintln!("Incoming connection error from {send_back_addr}: {error}");
                    }
                    SwarmEvent::NewListenAddr { address, .. } => {
                        print_system(&format!("Listening on {address}"));
                    }
                    _ => {}
                }

                try_flush_pending_messages(&mut swarm, &chat_topic, &mut pending_messages);
            }
            line_result = stdin_lines.next_line(), if !stdin_closed => {
                match line_result {
                    Ok(Some(line)) => {
                        let text = line.trim();
                        if text.is_empty() {
                            continue;
                        }

                        if text == "/exit" {
                            print_system("Exiting...");
                            break;
                        }

                        if let Some(new_name) = text.strip_prefix("/name ") {
                            let new_name = new_name.trim();
                            if new_name.is_empty() {
                                print_system("Usage: /name <your_name>");
                                continue;
                            }

                            local_name = new_name.to_string();
                            print_system(&format!("Your name is now {local_name}"));
                            let update = encode_name_update(&local_name);
                            if let Err(e) = swarm
                                .behaviour_mut()
                                .publish(chat_topic.clone(), update.as_bytes().to_vec())
                            {
                                eprintln!("Failed to publish name update: {e}");
                            }
                            continue;
                        }

                        if text == "/peers" {
                            let peers: Vec<String> = swarm
                                .connected_peers()
                                .map(|peer_id| {
                                    peer_names
                                        .get(peer_id)
                                        .cloned()
                                        .unwrap_or_else(|| format_peer_name(peer_id))
                                })
                                .collect();
                            print_system(&format!("Connected peers: {}", peers.join(", ")));
                            continue;
                        }

                        let payload = encode_chat_message(&local_name, text);
                        match swarm
                            .behaviour_mut()
                            .publish(chat_topic.clone(), payload.as_bytes().to_vec())
                        {
                            Ok(_) => println!("[you]: {text}"),
                            Err(gossipsub::PublishError::InsufficientPeers) => {
                                eprintln!("Waiting for peers to join... Message queued.");
                                pending_messages.push_back(payload);
                            }
                            Err(e) => eprintln!("Failed to publish message: {e}"),
                        }
                    }
                    Ok(None) => {
                        stdin_closed = true;
                        eprintln!("Standard input closed; continuing to process swarm events");
                    }
                    Err(e) => {
                        eprintln!("Failed to read input: {e}");
                    }
                }
            }
            _ = retry_interval.tick(), if !pending_messages.is_empty() => {
                try_flush_pending_messages(&mut swarm, &chat_topic, &mut pending_messages);
            }
        }
    }
}
