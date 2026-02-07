# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Forjj is a code forge/hosting platform specifically designed for the jj (Jujutsu) version control system. It provides change-based commits and review tooling similar to Gerrit or Phabricator.

## Tech Stack

- **Backend**: Rust (using jj-lib directly for storage)
- **Frontend**: TypeScript, React 18, Vite, Tailwind CSS
- **Build System**: Cargo + Nix
- **VCS**: jj (Jujutsu) native storage format via jj-lib

## Build Commands

```bash
# Build all crates
cargo build

# Run tests
cargo test

# Run the server
cargo run -p forjj-server

# Check formatting and lints
cargo fmt --check
cargo clippy

# Build with Nix
nix build
```

## Project Structure

```
forjj/
├── crates/
│   ├── forjj-server/     # Main server binary (axum HTTP + SSH)
│   ├── forjj-storage/    # Storage layer wrapping jj-lib
│   ├── forjj-protocol/   # Wire protocol for push/fetch
│   └── forjj-ssh/        # SSH server implementation
├── Cargo.toml            # Workspace manifest
├── DESIGN.md             # Architecture and design decisions
└── ROADMAP.md            # Implementation plan
```

## Architecture Principles

- **jj-lib integration**: Directly depend on jj-lib crate for storage operations
- **jj-native**: Use native `.jj` storage format (SimpleBackend)
- **CRDT-based**: jj operations are CRDTs - leverage for concurrent push handling
- **Change ID first-class**: jj change IDs (not commit hashes) are primary identifiers
- **Protocol support**: Custom forjj-sync protocol over SSH and HTTPS

## Key Dependencies

- `jj-lib` - Core jj library for storage and operations
- `axum` - HTTP server framework
- `russh` - SSH server implementation
- `tokio` - Async runtime
- `sqlx` - Database access (SQLite for metadata)

## Key Resources

- jj-vcs repository: https://github.com/jj-vcs/jj
- jj documentation: https://docs.jj-vcs.dev/latest/
- jj-lib API docs: https://docs.rs/jj-lib/

## Development Notes

- Reviews and ticket management are secondary objectives to hosting functionality
- MVP focuses on push/fetch over SSH with basic authentication
- Use jj-lib's existing merge logic for CRDT operation merging
