# AGENTS.md

## Project Snapshot
- `orinox` is a single-binary Rust crate (`Cargo.toml`) targeting Rust 2024 edition.
- Current entrypoint is `src/main.rs`; runtime behavior is centered on local identity bootstrapping.
- The app currently does not start networking yet: `main()` only calls `get_or_create_identity()`.

## Architecture and Data Flow
- CLI schema is defined with `clap` derive in `Args` and `LogLevel` (`src/main.rs`), but `Args::parse()` is not called yet.
- Identity flow (`get_or_create_identity`):
  1. Check for `.orinox/` directory in project root.
  2. If directory exists, check `.orinox/identity.key`.
  3. If key exists, read bytes and decode with `libp2p::identity::Keypair::from_protobuf_encoding`.
  4. If key is missing, generate an Ed25519 keypair and persist protobuf bytes.
- Local state boundary: `.orinox/identity.key` is the persistent node identity artifact; keep this behavior stable when adding network features.

## Key Files to Read First
- `Cargo.toml`: crate metadata + direct deps (`clap`, `libp2p`).
- `src/main.rs`: CLI model + identity lifecycle logic.
- `.gitignore`: ignores `target/`, editor files, and `.env`.

## Developer Workflow (Observed)
- Build check:
  - `cargo check` (currently fails on `src/main.rs` type mismatches in key encode/decode handling).
- Test run:
  - `cargo test` (no test modules currently; compile errors in main block test execution).
- CLI/help:
  - `cargo run -- --help` (expected to require successful compile first).
- Optional quality gates once compile is fixed:
  - `cargo fmt --check`
  - `cargo clippy -- -D warnings`

## Project-Specific Conventions
- Keep identity files under `.orinox/` at repo root; do not silently relocate path logic.
- Prefer `libp2p::identity::Keypair` protobuf encode/decode APIs for persistence (no custom key format).
- Preserve Clap derive style (`#[derive(Parser, ValueEnum)]`) when expanding CLI surface.
- This is a binary-focused codebase right now; if extracting modules, keep `main.rs` orchestration thin and move logic into named functions/modules.

## Integration Notes
- External dependencies are minimal and direct:
  - `clap` for argument modeling.
  - `libp2p` (identity submodule currently used; networking APIs not wired yet).
- Any new P2P behavior should use the existing persisted identity as the node key source rather than generating ephemeral keys per run.

## Agent Handoff Expectations
- Before proposing behavior changes, state whether they affect `.orinox/identity.key` lifecycle.
- When touching startup flow, reference `src/main.rs` and call out compile/run impact.
- If adding tests, prefer small unit tests around identity path/serialization decisions first.

