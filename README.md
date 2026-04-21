# Orinox - Decentralized P2P Chat

Orinox is a single-binary Rust CLI that runs a peer-to-peer chat over libp2p gossipsub. It bootstraps a local identity, starts a TCP listener, optionally dials peers, and publishes chat messages to a shared topic.

## Features
- Simple CLI with sensible defaults
- Persistent node identity stored at `.orinox/identity.key`
- libp2p gossipsub chat over TCP/Noise/Yamux
- Interactive terminal commands (`/name`, `/peers`, `/exit`)
- JSON-encoded chat payloads with username and message

## Requirements
- Rust toolchain (edition 2024 compatible)

## Install

From source:

```bash
cd /home/likhinmn/Likhin/projects/orinox
cargo install --path . --force
```

## Quick Start

Terminal 1:

```bash
cd /home/likhinmn/Likhin/projects/orinox/node1/
cargo run -- --port 9001
```

Terminal 2:

```bash
cd /home/likhinmn/Likhin/projects/orinox/node2/
cargo run -- --port 9002 --connect /ip4/127.0.0.1/tcp/9001 --name Alice
```

## Usage

```bash
orinox [OPTIONS]
```

Run with defaults (port 9000, auto username):

```bash
orinox
```

Show help:

```bash
orinox --help
```

## Options
- `-p, --port <PORT>`: Port to listen on (default: 9000)
- `-c, --connect <ADDR>`: Connect to peer (e.g., `/ip4/127.0.0.1/tcp/9001`)
- `-n, --name <NAME>`: Username displayed in chat (default: auto-generated)
- `-l, --log-level <LEVEL>`: Logging level (`error`, `warn`, `info`, `debug`, `trace`)
- `-h, --help`: Print help
- `-V, --version`: Print version

## In-Chat Commands
- `/name <new_name>`: Change your local username
- `/peers`: List connected peers
- `/exit`: Exit the application

## Identity and Storage
Orinox stores its persistent libp2p keypair at:

```
.orinox/identity.key
```

Do not delete this file if you want your peer identity to remain stable across runs.

## Message Format
Chat messages are sent as JSON:

```json
{"username":"Alice","message":"Hello"}
```

If a message cannot be parsed as JSON, Orinox falls back to printing the raw payload.

## Logging
You can override log filters with `RUST_LOG`, for example:

```bash
RUST_LOG=orinox=debug,libp2p=debug orinox
```

## Troubleshooting
- If you cannot connect, verify both peers are listening and the `/ip4/.../tcp/...` address is correct.
- If the port is in use, choose another port with `--port`.
- If identity initialization fails, check file permissions under `.orinox/`.

## Development
Common checks:

```bash
cd /home/likhinmn/Likhin/projects/orinox
cargo fmt
cargo clippy -- -D warnings
cargo test
```

## License
See `Cargo.toml` for package metadata and license information.

