use clap::{Parser, ValueEnum};
use futures::StreamExt;
use libp2p::Multiaddr;
use libp2p::PeerId;
use libp2p::gossipsub;
use libp2p::swarm::SwarmEvent;
use orinox::behaviour::GOSSIPSUB_TOPIC;
use orinox::identity::get_or_create_identity;
use orinox::swarm::{OrinoxSwarm, create_swarm};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::io::Write;
use tokio::io::{self, AsyncBufReadExt, BufReader};
use tokio::time::{self, Duration, MissedTickBehavior};
use tracing_subscriber::EnvFilter;

#[derive(ValueEnum, Debug, Clone)]
enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

#[derive(Parser, Debug)]
#[command(
    version,
    about = "Orinox - Decentralized P2P Chat",
    propagate_version = true,
    long_about = None
)]
struct Args {
    /// Port to listen on
    #[arg(short, long, default_value_t = 9000, value_name = "PORT")]
    port: u16,

    /// Connect to peer (e.g., /ip4/127.0.0.1/tcp/9001)
    #[arg(short, long, value_name = "ADDR")]
    connect: Vec<String>,

    /// Logging level
    #[arg(short = 'l', long, value_enum, default_value_t = LogLevel::Info, value_name = "LEVEL")]
    log_level: LogLevel,

    /// Username displayed in chat (default: auto-generated)
    #[arg(short = 'n', long, value_name = "NAME")]
    name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatMessage {
    username: String,
    message: String,
}

struct PendingMessage {
    payload: String,
    display: String,
}

fn format_peer_name(peer_id: &PeerId) -> String {
    let peer_text = peer_id.to_string();
    let short = peer_text.chars().take(8).collect::<String>();
    format!("user_{short}")
}

fn build_chat_payload(username: &str, message: &str) -> Result<String, serde_json::Error> {
    serde_json::to_string(&ChatMessage {
        username: username.to_string(),
        message: message.to_string(),
    })
}

fn print_system(message: &str) {
    println!("[system] {message}");
}

fn print_prompt() {
    print!("> ");
    let _ = std::io::stdout().flush();
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
    pending_messages: &mut VecDeque<PendingMessage>,
) {
    let pending_count = pending_messages.len();
    for _ in 0..pending_count {
        let Some(message) = pending_messages.pop_front() else {
            break;
        };

        match swarm
            .behaviour_mut()
            .publish(chat_topic.clone(), message.payload.as_bytes().to_vec())
        {
            Ok(_) => println!("[you]: {} (sent)", message.display),
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
    let requested_name = args.name;
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
    let mut local_username = requested_name.unwrap_or_else(|| format_peer_name(&local_peer_id));
    let hello_payload = match build_chat_payload(
        &local_username,
        &format!("Hello from {local_username}"),
    ) {
        Ok(payload) => payload,
        Err(e) => {
            eprintln!("Failed to build hello payload: {e}");
            std::process::exit(1);
        }
    };
    let mut hello_published = false;
    let mut pending_messages = VecDeque::new();

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
    print_prompt();

    loop {
        tokio::select! {
            event = swarm.select_next_some() => {
                match event {
                    SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
                        print_system(&format!("Connection established with {peer_id} via {endpoint:?}"));
                    }
                    SwarmEvent::ConnectionClosed { peer_id, .. } => {
                        print_system(&format!("Connection closed with {peer_id}"));
                    }
                    SwarmEvent::Behaviour(gossipsub::Event::Subscribed { peer_id, topic }) => {
                        print_system(&format!("Peer {peer_id} subscribed to {topic}"));
                        if topic == chat_topic_hash {
                            if !hello_published {
                                hello_published = try_publish_hello(&mut swarm, &chat_topic, &hello_payload);
                            }
                        }
                    }
                    SwarmEvent::Behaviour(gossipsub::Event::Message {
                        propagation_source,
                        message,
                        ..
                    }) => {
                        let sender = message.source.unwrap_or(propagation_source);
                        if sender == local_peer_id {
                            continue;
                        }

                        match serde_json::from_slice::<ChatMessage>(&message.data) {
                            Ok(chat) => {
                                println!("[{}]: {}", chat.username, chat.message);
                            }
                            Err(_) => {
                                let fallback = String::from_utf8_lossy(&message.data);
                                println!("[{sender}]: {fallback}");
                            }
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
                if !stdin_closed {
                    print_prompt();
                }
            }
            line_result = stdin_lines.next_line(), if !stdin_closed => {
                match line_result {
                    Ok(Some(line)) => {
                        let text = line.trim();
                        if text.is_empty() {
                            print_prompt();
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
                                print_prompt();
                                continue;
                            }

                            local_username = new_name.to_string();
                            print_system(&format!("Username changed to {local_username}"));
                            print_prompt();
                             continue;
                        }

                        if text == "/peers" {
                            print_system("Connected peers:");
                            for peer_id in swarm.connected_peers() {
                                println!("- {peer_id}");
                            }
                            print_prompt();
                             continue;
                        }

                        let payload = match build_chat_payload(&local_username, text) {
                            Ok(payload) => payload,
                            Err(e) => {
                                eprintln!("Failed to serialize message: {e}");
                                print_prompt();
                                continue;
                            }
                        };
                        match swarm
                            .behaviour_mut()
                            .publish(chat_topic.clone(), payload.as_bytes().to_vec())
                        {
                            Ok(_) => println!("[you]: {text}"),
                            Err(gossipsub::PublishError::InsufficientPeers) => {
                                eprintln!("Waiting for peers to join... Message queued.");
                                pending_messages.push_back(PendingMessage {
                                    payload,
                                    display: text.to_string(),
                                });
                            }
                            Err(e) => eprintln!("Failed to publish message: {e}"),
                        }
                        print_prompt();
                    }
                    Ok(None) => {
                        stdin_closed = true;
                        eprintln!("Standard input closed; continuing to process swarm events");
                    }
                    Err(e) => {
                        eprintln!("Failed to read input: {e}");
                        print_prompt();
                    }
                }
            }
            _ = retry_interval.tick(), if !pending_messages.is_empty() => {
                try_flush_pending_messages(&mut swarm, &chat_topic, &mut pending_messages);
                if !stdin_closed {
                    print_prompt();
                }
            }
        }
    }
}
