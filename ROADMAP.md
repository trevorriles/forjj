# Forjj Implementation Roadmap

This document outlines the implementation plan for Forjj's backend MVP. The focus is on
storage and protocol work to enable pushing and storing code via the native jj format.

## MVP Goal

> A working server that can receive pushes from a jj client and store repositories
> in the native jj format, with basic authentication.

**Out of scope for MVP:**
- Web UI
- Review system
- Comments/voting
- Multi-user permissions (beyond basic auth)
- HTTPS protocol (SSH first)

---

## Phase 0: Project Setup

**Goal:** Establish build system, dependencies, and project structure.

### Tasks

- [ ] **0.1** Initialize Rust project with Cargo workspace
- [ ] **0.2** Set up Nix flake for reproducible builds
- [ ] **0.3** Create project directory structure:
  ```
  forjj/
  ├── crates/
  │   ├── forjj-server/     # Main server binary
  │   │   └── src/
  │   │       └── main.rs
  │   ├── forjj-storage/    # Storage layer (wraps jj-lib)
  │   │   └── src/
  │   │       └── lib.rs
  │   ├── forjj-protocol/   # Wire protocol implementation
  │   │   └── src/
  │   │       └── lib.rs
  │   └── forjj-ssh/        # SSH server
  │       └── src/
  │           └── lib.rs
  ├── Cargo.toml            # Workspace manifest
  ├── flake.nix
  └── rust-toolchain.toml
  ```
- [ ] **0.4** Add jj-lib dependency and verify it compiles
- [ ] **0.5** Set up basic CI (GitHub Actions) with Nix
- [ ] **0.6** Add development tooling (clippy, rustfmt, cargo-deny)

**Deliverable:** `cargo build` produces a binary, CI passes.

---

## Phase 1: jj-lib Integration

**Goal:** Establish working integration with jj-lib for storage operations.

### Tasks

- [ ] **1.1** Add jj-lib as Cargo dependency:
  ```toml
  [dependencies]
  jj-lib = "0.38"
  ```
- [ ] **1.2** Create wrapper types for jj object IDs:
  - `CommitId`, `ChangeId`, `TreeId`, `FileId`, `OperationId`, `ViewId`
  - Implement `Display`, `FromStr`, `Serialize`, `Deserialize`
- [ ] **1.3** Implement repository initialization using jj-lib:
  - Create new repository with native backend
  - Verify jj CLI can read the created repo
- [ ] **1.4** Implement basic repository operations:
  - Open existing repository
  - Read commits, trees, files
  - Read operation log
- [ ] **1.5** Write integration tests:
  - Create repo with jj CLI, open with Forjj
  - Create repo with Forjj, read with jj CLI
  - Round-trip tests for all object types

**Deliverable:** Can read and write jj repositories using jj-lib.

---

## Phase 2: Storage Service

**Goal:** Create storage abstraction layer on top of jj-lib.

### Tasks

- [ ] **2.1** Design storage service interface:
  ```rust
  pub trait StorageService {
      fn create_repo(&self, name: &str) -> Result<Repository>;
      fn open_repo(&self, name: &str) -> Result<Repository>;
      fn list_repos(&self) -> Result<Vec<RepoInfo>>;
      fn delete_repo(&self, name: &str) -> Result<()>;
  }
  ```
- [ ] **2.2** Implement repository path management:
  - `/var/forjj/repos/{owner}/{name}/` layout
  - Atomic repository creation
- [ ] **2.3** Implement commit operations:
  - Read commit by ID
  - Write new commit
  - List commits (with revset support via jj-lib)
- [ ] **2.4** Implement tree/file operations:
  - Read tree by ID
  - Read file content by ID
  - Tree traversal
- [ ] **2.5** Implement operation log operations:
  - Read current operation heads
  - Append new operation
  - Find common ancestor

**Deliverable:** Clean storage API abstracting jj-lib details.

---

## Phase 3: Wire Protocol

**Goal:** Define and implement the forjj-sync protocol.

### Tasks

- [ ] **3.1** Define protocol messages using protobuf or serde:
  ```rust
  pub struct FetchRequest {
      pub have_ops: Vec<OperationId>,
      pub want_refs: Vec<String>,
  }

  pub struct PushRequest {
      pub have_ops: Vec<OperationId>,
      pub updates: Vec<RefUpdate>,
  }
  ```
- [ ] **3.2** Implement length-prefixed framing:
  - `[4-byte big-endian length][message bytes]`
  - Async reader/writer using tokio
- [ ] **3.3** Implement capability negotiation:
  - Protocol version
  - Supported features
- [ ] **3.4** Implement object pack format:
  - Serialize objects for transfer
  - Deserialize and validate on receipt
- [ ] **3.5** Implement fetch logic:
  - Compute missing objects given client's op heads
  - Stream objects to client
- [ ] **3.6** Implement push logic:
  - Receive object pack from client
  - Validate and store objects
  - Update refs atomically

**Deliverable:** Protocol library that can sync repositories over a byte stream.

---

## Phase 4: Operation Log Merge (CRDT)

**Goal:** Handle concurrent pushes by merging operation logs.

### Tasks

- [ ] **4.1** Implement operation DAG traversal using jj-lib:
  - Find common ancestor of two operations
  - Walk operation history
- [ ] **4.2** Implement view 3-way merge:
  - Use jj-lib's merge functionality
  - Handle bookmark conflicts
- [ ] **4.3** Create merge operation:
  - Parents: both divergent op heads
  - View: merged view
  - Metadata: record merge source
- [ ] **4.4** Test concurrent push scenarios:
  - Two clients push different commits
  - Two clients move same bookmark
  - Push while server has new operations

**Deliverable:** Server correctly merges concurrent operations.

---

## Phase 5: SSH Server

**Goal:** Accept SSH connections and handle push/fetch.

### Tasks

- [ ] **5.1** Add SSH server dependency:
  ```toml
  [dependencies]
  russh = "0.44"
  russh-keys = "0.44"
  ```
- [ ] **5.2** Implement SSH server setup:
  - Generate/load host keys
  - Configure listening address
- [ ] **5.3** Implement SSH authentication:
  - Public key authentication
  - User lookup from config file
- [ ] **5.4** Implement SSH channel handling:
  - Parse exec command: `forjj-receive-pack` / `forjj-upload-pack`
  - Route to protocol handler
- [ ] **5.5** Implement repository path resolution:
  - `owner/repo` → `/var/forjj/repos/owner/repo`
  - Permission checking
- [ ] **5.6** Wire up protocol to SSH channel:
  - Async bridge between SSH channel and protocol
- [ ] **5.7** End-to-end test:
  - Configure jj with forjj remote helper
  - `jj push` to Forjj server over SSH
  - `jj fetch` from Forjj server

**Deliverable:** Can push/fetch over SSH.

---

## Phase 6: jj Remote Helper

**Goal:** Create helper binary so jj CLI can talk to Forjj.

### Tasks

- [ ] **6.1** Create `jj-remote-forjj` binary crate
- [ ] **6.2** Implement remote helper protocol:
  - Parse jj's remote helper invocation
  - Translate to forjj-sync protocol
- [ ] **6.3** Implement SSH transport in helper:
  - Connect to Forjj server
  - Handle authentication (SSH agent, key files)
- [ ] **6.4** Package helper for distribution:
  - Standalone binary via cargo-dist
  - Nix package
- [ ] **6.5** Write user documentation:
  - How to install remote helper
  - How to configure jj remote
  - Example workflows

**Deliverable:** Users can `jj push`/`jj fetch` to Forjj with minimal setup.

---

## Phase 7: Basic Repository Management

**Goal:** Minimal REST API for repository administration.

### Tasks

- [ ] **7.1** Add HTTP server dependencies:
  ```toml
  [dependencies]
  axum = "0.7"
  tokio = { version = "1", features = ["full"] }
  tower-http = "0.5"
  ```
- [ ] **7.2** Implement REST endpoints:
  - `POST /api/v1/repos` - Create repository
  - `GET /api/v1/repos/{owner}/{name}` - Get repo info
  - `GET /api/v1/repos` - List repositories
  - `DELETE /api/v1/repos/{owner}/{name}` - Delete repo
- [ ] **7.3** Implement API authentication:
  - Bearer token authentication
  - Token storage in config file
- [ ] **7.4** Implement repository metadata:
  - Store in `forjj.json` per repo
  - Owner, visibility, created_at, description

**Deliverable:** Can create/list/delete repositories via REST API.

---

## Success Criteria for MVP

1. **Create repository** via REST API
2. **Push commits** from jj CLI over SSH
3. **Fetch commits** from jj CLI over SSH
4. **Concurrent pushes** merge correctly (no data loss)
5. **Round-trip compatibility** - repos work with both jj CLI and Forjj
6. **Basic auth** - SSH keys for push/fetch, API tokens for management

---

## Timeline Estimate

| Phase | Estimated Effort | Dependencies |
|-------|------------------|--------------|
| Phase 0: Setup | 1 day | None |
| Phase 1: jj-lib Integration | 2-3 days | Phase 0 |
| Phase 2: Storage Service | 2-3 days | Phase 1 |
| Phase 3: Wire Protocol | 3-4 days | Phase 2 |
| Phase 4: CRDT Merge | 2-3 days | Phase 3 |
| Phase 5: SSH Server | 3-4 days | Phase 3 |
| Phase 6: Remote Helper | 2-3 days | Phase 5 |
| Phase 7: REST API | 2-3 days | Phase 2 |

**Total: ~3-4 weeks for MVP**

---

## Key Dependencies

```toml
[workspace.dependencies]
# jj integration
jj-lib = "0.38"

# Async runtime
tokio = { version = "1", features = ["full"] }

# HTTP server
axum = "0.7"
tower = "0.4"
tower-http = "0.5"

# SSH server
russh = "0.44"
russh-keys = "0.44"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Error handling
anyhow = "1"
thiserror = "1"

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

# Database (for metadata)
sqlx = { version = "0.8", features = ["sqlite", "runtime-tokio"] }
```

---

## Risk Register

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| jj-lib API changes | Medium | Medium | Pin version, test on upgrade, engage upstream |
| SSH library complexity | Low | Medium | russh is mature, fallback to openssh subprocess |
| CRDT merge edge cases | Low | High | Leverage jj-lib's battle-tested merge code |
| Performance at scale | Low | Medium | Profile early, jj-lib is already optimized |

---

## Next Steps

1. Start Phase 0: Initialize Cargo workspace
2. Add jj-lib dependency and verify build
3. Create first integration test: read a jj repository
4. Set up CI with Nix
