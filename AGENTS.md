# AGENTS.md

## Project Snapshot
- `orinox` is a single-binary Rust crate (`Cargo.toml`) targeting Rust 2024 edition.
- Current entrypoint is `src/main.rs`; runtime behavior is centered on local identity bootstrapping, libp2p swarm startup, and gossipsub chat relay.
- `main()` parses CLI args, initializes identity, starts TCP listening, optionally dials peers, and processes swarm + stdin events.

## Architecture and Data Flow
- CLI schema is defined with `clap` derive in `Args` and `LogLevel` (`src/main.rs`), and `Args::parse()` is called at startup.
- Identity flow (`get_or_create_identity`):
  1. Ensure `.orinox/` directory exists in the project root.
  2. Check `.orinox/identity.key`.
  3. If key exists, read bytes and decode with `libp2p::identity::Keypair::from_protobuf_encoding`.
  4. If key is missing, generate an Ed25519 keypair and persist protobuf bytes.
- Networking startup flow (`main` + swarm modules):
  1. Parse `--port`, `--connect`, and `--log-level` via `Args::parse()`.
  2. Load persisted identity and derive `PeerId`.
  3. Build swarm with `orinox::swarm::create_swarm`, which composes gossipsub behaviour + TCP/Noise/Yamux transport.
  4. Listen on `/ip4/0.0.0.0/tcp/{port}`, dial each `--connect` multiaddr, and run `tokio::select!` over `SwarmEvent` and stdin lines.
  5. Subscribe to topic `orinox-global` (`src/behaviour.rs`) and publish messages through gossipsub (including the one-time hello publish after first peer subscription).
- Local state boundary: `.orinox/identity.key` is the persistent node identity artifact; keep this behavior stable when adding network features.

## Key Files to Read First
- `Cargo.toml`: crate metadata + direct deps (`clap`, `libp2p`, `tokio`, `futures`) and `tempfile` dev-dependency (`libp2p` enables `gossipsub`).
- `src/main.rs`: CLI parsing + startup orchestration (identity, listen, dial, swarm event loop, stdin message publish).
- `src/identity.rs`: identity key lifecycle and `.orinox/identity.key` persistence format.
- `src/behaviour.rs`, `src/swarm.rs`, and `src/transport.rs`: gossipsub behaviour setup, swarm construction, and TCP/Noise/Yamux transport wiring.
- `tests/identity_lifecycle.rs`: regression coverage for key creation, reload stability, and corrupted-key handling.
- `.gitignore`: ignores `target/`, editor files, `.env`, `.orinox`, and `.agents`.

## Developer Workflow (Observed)
- Build check:
  - `cargo check` (currently passes).
- Test run:
  - `cargo test` (currently passes; includes `tests/identity_lifecycle.rs` with 3 tests).
- CLI/help:
  - `cargo run -- --help` (expected to require successful compile first).
- Manual two-node chat smoke test:
  - Terminal 1: `cargo run -- --port 9001`
  - Terminal 2: `cargo run -- --port 9002 --connect /ip4/127.0.0.1/tcp/9001`
- Optional quality gates:
  - `cargo fmt --check`
  - `cargo clippy -- -D warnings`

## Project-Specific Conventions
- Keep identity files under `.orinox/` at repo root; do not silently relocate path logic.
- Prefer `libp2p::identity::Keypair` protobuf encode/decode APIs for persistence (no custom key format).
- Preserve Clap derive style (`#[derive(Parser, ValueEnum)]`) when expanding CLI surface.
- Keep gossipsub topic naming centralized in `src/behaviour.rs` (`GOSSIPSUB_TOPIC`) and construct `IdentTopic` from that constant in `src/main.rs`.
- Keep `main.rs` orchestration thin: networking composition lives in `src/swarm.rs`, `src/transport.rs`, and `src/behaviour.rs`.

## Integration Notes
- External dependencies are minimal and direct:
  - `clap` for argument modeling.
  - `libp2p` for identity, gossipsub pub/sub, TCP transport, Noise auth, Yamux multiplexing, and `Swarm` event processing.
  - `tokio` for async runtime (`#[tokio::main]`) and async stdin line handling, plus `futures` for `StreamExt` over swarm events.
- Any new P2P behavior should use the existing persisted identity as the node key source rather than generating ephemeral keys per run.

## Agent Handoff Expectations
- Before proposing behavior changes, state whether they affect `.orinox/identity.key` lifecycle.
- When touching startup flow, reference `src/main.rs`, `src/behaviour.rs`, `src/swarm.rs`, and `src/transport.rs` and call out compile/run impact.
- If adding tests, follow the existing identity lifecycle pattern in `tests/identity_lifecycle.rs` first.

