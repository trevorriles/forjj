# Forjj Design Document

This document outlines the high-level architecture and design decisions for Forjj, a code forge specifically designed for the Jujutsu (jj) version control system.

## Table of Contents

1. [Goals and Non-Goals](#goals-and-non-goals)
2. [Architecture Overview](#architecture-overview)
3. [Data Model](#data-model)
4. [Storage Strategy](#storage-strategy)
5. [Sync and Replication](#sync-and-replication)
6. [API Design](#api-design)
7. [Review System](#review-system)
8. [Security Model](#security-model)
9. [Trade-offs and Decisions](#trade-offs-and-decisions)

---

## Goals and Non-Goals

### Goals

1. **Native jj support**: First-class support for jj change IDs, operations, and conflict handling
2. **Change-based workflow**: Similar to Gerrit/Phabricator, reviews are per-change, not per-branch
3. **CRDT-native**: Leverage jj's operation log as a CRDT for conflict-free replication
4. **Protocol support**: Push/pull via SSH and HTTPS
5. **Self-hostable**: Single-binary deployment with minimal dependencies

### Non-Goals (Initial Release)

- Full issue/ticket management system (secondary objective)
- Git compatibility layer (focus on jj-native first)
- Real-time collaborative editing
- CI/CD built-in (plugin architecture instead)

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              Forjj Architecture                              │
└─────────────────────────────────────────────────────────────────────────────┘

                                 ┌─────────────┐
                                 │   Clients   │
                                 │  (jj CLI)   │
                                 └──────┬──────┘
                                        │
                    ┌───────────────────┼───────────────────┐
                    │                   │                   │
                    ▼                   ▼                   ▼
             ┌──────────┐        ┌──────────┐        ┌──────────┐
             │   SSH    │        │  HTTPS   │        │   Web    │
             │ Protocol │        │ Protocol │        │   UI     │
             └────┬─────┘        └────┬─────┘        └────┬─────┘
                  │                   │                   │
                  └───────────────────┼───────────────────┘
                                      │
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                              API Gateway                                     │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐        │
│  │    Auth     │  │    Rate     │  │   Request   │  │   Audit     │        │
│  │  Middleware │  │   Limiter   │  │   Router    │  │    Log      │        │
│  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘        │
└─────────────────────────────────────────────────────────────────────────────┘
                                      │
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                            Core Services                                     │
│                                                                              │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐             │
│  │   Repository    │  │     Review      │  │      User       │             │
│  │    Service      │  │    Service      │  │    Service      │             │
│  └────────┬────────┘  └────────┬────────┘  └────────┬────────┘             │
│           │                    │                    │                       │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐             │
│  │   Operation     │  │    Change       │  │   Permission    │             │
│  │    Service      │  │    Service      │  │    Service      │             │
│  └────────┬────────┘  └────────┬────────┘  └────────┬────────┘             │
│           │                    │                    │                       │
└───────────┼────────────────────┼────────────────────┼───────────────────────┘
            │                    │                    │
            ▼                    ▼                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                           Storage Layer                                      │
│                                                                              │
│  ┌─────────────────────────────┐  ┌─────────────────────────────┐          │
│  │     Repository Storage      │  │     Metadata Storage        │          │
│  │  ┌───────────────────────┐  │  │  ┌───────────────────────┐  │          │
│  │  │  jj Native Backend    │  │  │  │   Review Database     │  │          │
│  │  │  (.jj/ format)        │  │  │  │   (SQLite/Postgres)   │  │          │
│  │  └───────────────────────┘  │  │  └───────────────────────┘  │          │
│  │  ┌───────────────────────┐  │  │  ┌───────────────────────┐  │          │
│  │  │  Operation Log        │  │  │  │   User/Permission     │  │          │
│  │  │  (CRDT)               │  │  │  │   Store               │  │          │
│  │  └───────────────────────┘  │  │  └───────────────────────┘  │          │
│  └─────────────────────────────┘  └─────────────────────────────┘          │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Component Breakdown

| Component | Technology | Responsibility |
|-----------|------------|----------------|
| SSH Protocol | Rust | jj-native wire protocol over SSH |
| HTTPS Protocol | Rust (axum) | jj-native wire protocol over HTTPS |
| Web UI | React 18 + Vite | Review interface, repo browsing |
| API Gateway | Rust (axum) | Request routing, auth, rate limiting |
| Repository Service | Rust | CRUD operations on repositories |
| Operation Service | Rust + jj-lib | jj operation log management |
| Change Service | Rust + jj-lib | Change ID tracking and lifecycle |
| Review Service | Rust | Code review workflow |
| Storage Layer | Rust + jj-lib | Persistent storage using jj's native backend |

---

## Data Model

### Core Entities

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            Entity Relationships                              │
└─────────────────────────────────────────────────────────────────────────────┘

  ┌────────────────┐         ┌────────────────┐         ┌────────────────┐
  │   Repository   │ 1────n  │     Change     │ 1────n  │    Commit      │
  │                │         │                │         │                │
  │  - id          │         │  - change_id   │         │  - commit_id   │
  │  - name        │         │  - repo_id     │         │  - change_id   │
  │  - owner       │         │  - status      │         │  - tree_id     │
  │  - visibility  │         │  - created_at  │         │  - parents[]   │
  │  - created_at  │         │                │         │  - predecessors│
  └────────────────┘         └───────┬────────┘         └────────────────┘
                                     │
                                     │ 1
                                     │
                                     ▼ n
                             ┌────────────────┐
                             │    Review      │
                             │                │
                             │  - id          │
                             │  - change_id   │
                             │  - status      │
                             │  - reviewers[] │
                             │  - votes{}     │
                             └───────┬────────┘
                                     │
                                     │ 1
                                     │
                                     ▼ n
                             ┌────────────────┐
                             │    Comment     │
                             │                │
                             │  - id          │
                             │  - review_id   │
                             │  - author      │
                             │  - file_path   │
                             │  - line_range  │
                             │  - content     │
                             │  - resolved    │
                             └────────────────┘
```

### jj-Specific Concepts

**Change ID vs Commit ID**: This distinction is fundamental to Forjj's design.

```
                    Change ID: stable across rewrites
                    ────────────────────────────────
                                   │
          ┌────────────────────────┼────────────────────────┐
          │                        │                        │
          ▼                        ▼                        ▼
    ┌──────────┐            ┌──────────┐            ┌──────────┐
    │ Commit A │  amend →   │ Commit B │  amend →   │ Commit C │
    │ (v1)     │            │ (v2)     │            │ (v3)     │
    └──────────┘            └──────────┘            └──────────┘

    CommitId: abc123        CommitId: def456        CommitId: ghi789
    ChangeId: xyz           ChangeId: xyz           ChangeId: xyz

    All three commits are different versions of the SAME change.
    Reviews attach to the ChangeId, not the CommitId.
```

**Operation Log**: The operation log is the key to understanding repository state transitions.

```
                         Operation DAG (CRDT)
                         ───────────────────

    Operation A (initial)
         │
         ▼
    Operation B (local amend)
         │
         ├─────────────────────┐
         │                     │
         ▼                     ▼
    Operation C            Operation D
    (user 1 push)          (user 2 push)
         │                     │
         └──────────┬──────────┘
                    │
                    ▼
              Operation E
           (merged state)

    Each operation contains a "view" - the complete repository
    state (bookmarks, heads, working copies) at that point.
```

### Database Schema (Metadata)

```sql
-- Repositories
CREATE TABLE repositories (
    id              UUID PRIMARY KEY,
    name            TEXT NOT NULL,
    owner_id        UUID NOT NULL REFERENCES users(id),
    visibility      TEXT NOT NULL DEFAULT 'private',  -- public, private, internal
    default_branch  TEXT NOT NULL DEFAULT 'main',
    created_at      TIMESTAMP NOT NULL,
    updated_at      TIMESTAMP NOT NULL,

    UNIQUE(owner_id, name)
);

-- Changes (jj change tracking)
CREATE TABLE changes (
    id              UUID PRIMARY KEY,
    repo_id         UUID NOT NULL REFERENCES repositories(id),
    change_id       TEXT NOT NULL,  -- jj change ID (hex string)
    current_commit  TEXT NOT NULL,  -- current commit ID
    status          TEXT NOT NULL DEFAULT 'open',  -- open, merged, abandoned
    created_at      TIMESTAMP NOT NULL,
    updated_at      TIMESTAMP NOT NULL,

    UNIQUE(repo_id, change_id)
);

-- Change History (predecessor tracking)
CREATE TABLE change_commits (
    id              UUID PRIMARY KEY,
    change_id       UUID NOT NULL REFERENCES changes(id),
    commit_id       TEXT NOT NULL,
    predecessor_id  TEXT,  -- previous commit ID in this change
    created_at      TIMESTAMP NOT NULL
);

-- Reviews
CREATE TABLE reviews (
    id              UUID PRIMARY KEY,
    change_id       UUID NOT NULL REFERENCES changes(id),
    status          TEXT NOT NULL DEFAULT 'pending',  -- pending, approved, rejected
    created_at      TIMESTAMP NOT NULL,
    updated_at      TIMESTAMP NOT NULL
);

-- Review Votes
CREATE TABLE review_votes (
    id              UUID PRIMARY KEY,
    review_id       UUID NOT NULL REFERENCES reviews(id),
    user_id         UUID NOT NULL REFERENCES users(id),
    label           TEXT NOT NULL,  -- 'Code-Review', 'Verified', etc.
    score           INTEGER NOT NULL,  -- -2 to +2
    created_at      TIMESTAMP NOT NULL,

    UNIQUE(review_id, user_id, label)
);

-- Comments
CREATE TABLE comments (
    id              UUID PRIMARY KEY,
    review_id       UUID NOT NULL REFERENCES reviews(id),
    author_id       UUID NOT NULL REFERENCES users(id),
    commit_id       TEXT,  -- specific version being commented on
    file_path       TEXT,
    line_start      INTEGER,
    line_end        INTEGER,
    content         TEXT NOT NULL,
    parent_id       UUID REFERENCES comments(id),
    resolved        BOOLEAN NOT NULL DEFAULT FALSE,
    created_at      TIMESTAMP NOT NULL,
    updated_at      TIMESTAMP NOT NULL
);
```

---

## Storage Strategy

### Decision: jj Native Backend vs Git Backend

**Option 1: Git Backend** (jj over Git)
- Pros: Mature, well-tested, compatible with existing Git tooling
- Cons: Requires maintaining `.git` directory, Git garbage collection complexity, extra storage layer

**Option 2: Native jj Backend** (SimpleBackend)
- Pros: Purpose-built for jj, simpler model, no Git overhead, cleaner architecture
- Cons: Less tested, smaller ecosystem, may require upstream contributions

**Decision: Native jj Backend (SimpleBackend)**

Rationale:
1. **Architectural purity**: No Git abstraction layer means simpler codebase and fewer edge cases
2. **Storage efficiency**: No duplicate storage between Git objects and jj metadata
3. **First-mover advantage**: Building a forge specifically for jj should embrace jj-native storage
4. **Upstream collaboration**: Willing to contribute fixes/improvements to jj's native backend
5. **No GC complexity**: Native backend doesn't require Git's garbage collection dance

Trade-offs accepted:
- Native backend is less tested than Git backend in jj
- Debugging may require custom tooling (no `git log` inspection)
- May encounter edge cases requiring upstream fixes
- Initial users must use jj client (no Git fallback)

### Upstream Contribution Strategy

Since the native backend is still maturing, we commit to:
1. **Bug reporting**: File detailed issues for any native backend bugs encountered
2. **Patch contributions**: Submit fixes for issues blocking Forjj development
3. **Testing feedback**: Provide real-world usage data to help stabilize the backend
4. **Documentation**: Contribute documentation for native backend internals

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    Repository Storage Layout (Native Backend)                │
└─────────────────────────────────────────────────────────────────────────────┘

/var/forjj/repos/
├── {owner}/
│   └── {repo_name}/
│       ├── .jj/
│       │   ├── repo/
│       │   │   ├── store/
│       │   │   │   ├── type              # "simple" (native backend type)
│       │   │   │   ├── commits/          # commit objects (content-addressed)
│       │   │   │   │   └── {commit_id}   # protobuf-encoded commit
│       │   │   │   ├── trees/            # tree objects (content-addressed)
│       │   │   │   │   └── {tree_id}     # protobuf-encoded tree
│       │   │   │   ├── files/            # file/blob objects (content-addressed)
│       │   │   │   │   └── {file_id}     # raw file content
│       │   │   │   ├── symlinks/         # symlink targets
│       │   │   │   │   └── {symlink_id}
│       │   │   │   └── conflicts/        # conflict objects
│       │   │   │       └── {conflict_id}
│       │   │   ├── op_store/             # operation storage
│       │   │   │   ├── type              # "simple"
│       │   │   │   ├── operations/       # operation objects (protobuf)
│       │   │   │   │   └── {op_id}
│       │   │   │   └── views/            # view objects (protobuf)
│       │   │   │       └── {view_id}
│       │   │   ├── op_heads/             # current operation head(s)
│       │   │   │   ├── type
│       │   │   │   └── heads/
│       │   │   │       └── {op_id}       # file presence = head
│       │   │   └── index/                # commit index for queries
│       │   │       ├── type
│       │   │       └── ...
│       │   └── working_copy/             # (not used server-side)
│       └── forjj.json                    # forge-specific metadata
```

### Native Backend Object Format

Objects in the native backend are serialized using Protocol Buffers. The key object types:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         Native Backend Object Types                          │
└─────────────────────────────────────────────────────────────────────────────┘

Commit Object (protobuf):
┌─────────────────────────────────────────┐
│  parents: [CommitId]                    │  # Parent commit references
│  predecessors: [CommitId]               │  # Amendment history (jj-specific)
│  root_tree: Merge<TreeId>               │  # Tree state (possibly conflicted)
│  change_id: ChangeId                    │  # Stable change identifier
│  description: string                    │  # Commit message
│  author: Signature                      │  # Author info + timestamp
│  committer: Signature                   │  # Committer info + timestamp
│  secure_sig: Option<SecureSig>          │  # Optional cryptographic signature
└─────────────────────────────────────────┘

Tree Object (protobuf):
┌─────────────────────────────────────────┐
│  entries: Map<string, TreeValue>        │  # filename -> value mapping
└─────────────────────────────────────────┘

TreeValue (enum):
┌─────────────────────────────────────────┐
│  File { id: FileId, executable: bool }  │
│  Symlink { id: SymlinkId }              │
│  Tree { id: TreeId }                    │  # Subdirectory
│  GitSubmodule { id: CommitId }          │  # For git-backend compat
│  Conflict { id: ConflictId }            │  # First-class conflict
└─────────────────────────────────────────┘

Operation Object (protobuf):
┌─────────────────────────────────────────┐
│  view_id: ViewId                        │  # Reference to view snapshot
│  parents: [OperationId]                 │  # Parent operations (DAG)
│  metadata: OperationMetadata            │  # Who, when, what command
└─────────────────────────────────────────┘

View Object (protobuf):
┌─────────────────────────────────────────┐
│  head_ids: Set<CommitId>                │  # Visible commit heads
│  bookmarks: Map<string, BookmarkTarget> │  # Named refs (branches)
│  tags: Map<string, CommitId>            │  # Tag references
│  wc_commit_ids: Map<WorkspaceId, CommitId> │ # Per-workspace state
└─────────────────────────────────────────┘
```

### Content-Addressing Scheme

All objects are content-addressed using BLAKE2b hashing, matching jj-lib's implementation:

```
Object ID = BLAKE2b-256(serialized_protobuf_bytes)

Storage path derivation:
  commits/{full_hex_id}     # No sharding initially (can add later)
  trees/{full_hex_id}
  files/{full_hex_id}
  operations/{full_hex_id}
  views/{full_hex_id}
```

**Hash Algorithm Decision: BLAKE2b**

| Consideration | Decision |
|---------------|----------|
| Algorithm | BLAKE2b-256 (32-byte output) |
| Rationale | Compatibility with jj-lib |
| Alternative considered | BLAKE3 (faster, parallel) |
| Why not BLAKE3 | jj upstream uses BLAKE2; diverging would break object compatibility |

jj maintainers have discussed potentially standardizing on a consistent hash algorithm
(with BLAKE3 mentioned as a candidate). Our strategy:

1. **Current**: Use jj-lib's `content_hash` module directly (BLAKE2b)
2. **Automatic compatibility**: Since we depend on jj-lib, hash algorithm changes are handled
3. **Participation**: Engage in upstream hash standardization discussions

**Design note**: Unlike Git's 2-character prefix sharding (`objects/ab/cdef...`), we start
with flat directories. If repository size becomes an issue, we can add sharding later.
The content-addressed nature means we can reshare objects transparently.

### Caching Strategy

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           Caching Layers                                     │
└─────────────────────────────────────────────────────────────────────────────┘

                    ┌─────────────────────────────┐
                    │      Request Cache          │
                    │   (HTTP response cache)     │
                    │   TTL: 60s for reads        │
                    └──────────────┬──────────────┘
                                   │
                    ┌──────────────▼──────────────┐
                    │      Object Cache           │
                    │   (LRU in-memory cache)     │
                    │   - 100 commits             │
                    │   - 1000 trees              │
                    │   - 10000 file entries      │
                    └──────────────┬──────────────┘
                                   │
                    ┌──────────────▼──────────────┐
                    │      Index Cache            │
                    │   (mmap'd commit index)     │
                    │   - revset queries          │
                    │   - ancestor lookups        │
                    └──────────────┬──────────────┘
                                   │
                    ┌──────────────▼──────────────┐
                    │   Native Backend Storage    │
                    │  (content-addressed files)  │
                    │  - commits/, trees/, files/ │
                    │  - operations/, views/      │
                    └─────────────────────────────┘
```

### Storage Optimization Notes

Since the native backend uses simple content-addressed files:

1. **No garbage collection needed**: Unlike Git, we don't have dangling objects from rebases.
   The operation log references all valid objects. Unreferenced objects can be pruned by
   walking the operation DAG (future optimization).

2. **Filesystem-friendly**: Each object is a separate file, enabling:
   - OS-level filesystem caching
   - Easy backup (rsync/cp)
   - Simple integrity verification (rehash any file)

3. **Future: Object deduplication**: Cross-repository object sharing is possible since
   objects are content-addressed. A shared object store could reduce storage for forks.

---

## Sync and Replication

### Local-to-Forge Sync Model

When a user pushes changes to Forjj, the following happens:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           Push Flow                                          │
└─────────────────────────────────────────────────────────────────────────────┘

Client                              Forjj Server
──────                              ────────────
   │                                     │
   │  1. Initiate push (SSH/HTTPS)       │
   ├────────────────────────────────────>│
   │                                     │
   │  2. Negotiate objects               │
   │<────────────────────────────────────┤
   │                                     │
   │  3. Send pack (commits, trees,      │
   │     blobs, operations)              │
   ├────────────────────────────────────>│
   │                                     │
   │                          ┌──────────┴──────────┐
   │                          │ 4. Validate pack    │
   │                          │    - Check perms    │
   │                          │    - Verify sigs    │
   │                          └──────────┬──────────┘
   │                                     │
   │                          ┌──────────┴──────────┐
   │                          │ 5. Merge operation  │
   │                          │    log (CRDT merge) │
   │                          └──────────┬──────────┘
   │                                     │
   │                          ┌──────────┴──────────┐
   │                          │ 6. Update refs      │
   │                          │    and indexes      │
   │                          └──────────┬──────────┘
   │                                     │
   │                          ┌──────────┴──────────┐
   │                          │ 7. Trigger hooks    │
   │                          │    (reviews, CI)    │
   │                          └──────────┬──────────┘
   │                                     │
   │  8. Push result                     │
   │<────────────────────────────────────┤
   │                                     │
```

### Operation Log Merge (CRDT)

jj's operation log is naturally CRDT-compatible. When concurrent operations occur:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    Operation Log CRDT Merge                                  │
└─────────────────────────────────────────────────────────────────────────────┘

Server State:                    Incoming Push:

Op1 ─── Op2 ─── Op3              Op1 ─── Op2 ─── Op4
                │                                │
            (server)                         (client)

                    ┌─────────────────────┐
                    │   3-way View Merge  │
                    │                     │
                    │  Base: Op2.view     │
                    │  Left: Op3.view     │
                    │  Right: Op4.view    │
                    └──────────┬──────────┘
                               │
                               ▼

            Op1 ─── Op2 ─┬─ Op3
                         │
                         └─ Op4
                              │
                              ▼
                            Op5 (merged)

Merged view contains:
- Union of visible heads
- Conflicted bookmarks (if both sides moved same bookmark)
- Merged working copy states per workspace
```

### Conflict Resolution

When view conflicts occur (e.g., same bookmark moved to different commits):

```
Bookmark 'main':
  - Server: points to commit A
  - Client: points to commit B

Result: Conflicted bookmark stored as [A, B]
        User must resolve via: jj bookmark set main -r <target>
```

### Multi-Instance Replication (Future)

For high-availability deployments:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    Multi-Instance Architecture                               │
└─────────────────────────────────────────────────────────────────────────────┘

                         ┌─────────────────┐
                         │   Load Balancer │
                         └────────┬────────┘
                                  │
              ┌───────────────────┼───────────────────┐
              │                   │                   │
              ▼                   ▼                   ▼
       ┌──────────┐        ┌──────────┐        ┌──────────┐
       │ Forjj 1  │        │ Forjj 2  │        │ Forjj 3  │
       │ (write)  │        │  (read)  │        │  (read)  │
       └────┬─────┘        └────┬─────┘        └────┬─────┘
            │                   │                   │
            └───────────────────┼───────────────────┘
                                │
                    ┌───────────▼───────────┐
                    │   Shared Storage      │
                    │   (NFS / Object Store)│
                    └───────────────────────┘
                                │
                    ┌───────────▼───────────┐
                    │   Replication Log     │
                    │   (operation stream)  │
                    └───────────────────────┘

Write path:
1. Client pushes to Forjj 1 (write primary)
2. Forjj 1 writes to shared storage
3. Forjj 1 publishes operation to replication log
4. Forjj 2, 3 consume log and update local caches

Read path:
1. Client fetches from any instance
2. Instance serves from local cache or shared storage
```

---

## API Design

### Wire Protocol (jj-native)

Since we're targeting the native jj backend, we need a jj-native wire protocol. jj currently
uses Git's protocol for `jj git push/fetch`, but for native backend sync we need a new protocol.

**Decision: Custom jj-native protocol with future jj upstream integration**

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    Forjj Wire Protocol                                       │
└─────────────────────────────────────────────────────────────────────────────┘

Protocol: forjj-sync/1.0
Transport: SSH or HTTPS
Encoding: Protocol Buffers over length-prefixed frames

Capabilities negotiation:
  Client sends:
    - protocol_version: 1
    - capabilities: [operations, thin-pack, resume]
    - client_op_heads: [OperationId...]

  Server responds:
    - protocol_version: 1
    - capabilities: [operations, thin-pack, resume]
    - server_op_heads: [OperationId...]
    - common_ancestor: OperationId (if found)
```

**Sync Operations:**

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         Fetch (Pull) Flow                                    │
└─────────────────────────────────────────────────────────────────────────────┘

1. Client → Server: FetchRequest
   {
     have_ops: [OperationId...],      # Operations client already has
     want_refs: ["main", "@"],        # Bookmarks/refs to fetch
     depth: Option<u32>,              # Shallow fetch limit (optional)
   }

2. Server → Client: FetchResponse
   {
     pack_follows: true,
     ops_to_send: [OperationId...],   # Operations client needs
     commits_to_send: u64,            # Count of commits in pack
   }

3. Server → Client: ObjectPack (streaming)
   - Operations (with views)
   - Commits (with trees, files)
   - Packfile format: length-prefixed protobuf objects

4. Client: Apply pack, merge operation log
```

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         Push Flow                                            │
└─────────────────────────────────────────────────────────────────────────────┘

1. Client → Server: PushRequest
   {
     have_ops: [OperationId...],      # Client's op heads
     updates: [
       { ref: "main", old: CommitId, new: CommitId },
       { ref: "change/xyz", old: null, new: CommitId },
     ],
   }

2. Server → Client: PushNegotiate
   {
     common_op: OperationId,          # Common ancestor operation
     need_objects: true,              # Whether pack is needed
   }

3. Client → Server: ObjectPack (streaming)
   - New operations
   - New commits, trees, files

4. Server: Validate, merge op log, update refs

5. Server → Client: PushResult
   {
     status: "ok" | "rejected" | "conflict",
     new_op_head: OperationId,
     ref_results: [{ ref: string, status: string }...],
   }
```

**Future: Upstream Integration**

This protocol is designed with eventual jj upstream contribution in mind:
- Could become `jj push/fetch` for native remotes (non-Git)
- Protocol buffer definitions can be shared with jj-lib
- Operation-aware sync is more efficient than Git ref-based sync

### REST API

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         REST API Endpoints                                   │
└─────────────────────────────────────────────────────────────────────────────┘

Repositories:
  GET    /api/v1/repos                           # List repositories
  POST   /api/v1/repos                           # Create repository
  GET    /api/v1/repos/{owner}/{name}            # Get repository
  DELETE /api/v1/repos/{owner}/{name}            # Delete repository

Changes:
  GET    /api/v1/repos/{owner}/{name}/changes                # List changes
  GET    /api/v1/repos/{owner}/{name}/changes/{change_id}    # Get change
  GET    /api/v1/repos/{owner}/{name}/changes/{change_id}/diff   # Get diff
  GET    /api/v1/repos/{owner}/{name}/changes/{change_id}/commits # Get commits

Reviews:
  GET    /api/v1/repos/{owner}/{name}/reviews                    # List reviews
  POST   /api/v1/repos/{owner}/{name}/changes/{change_id}/review # Create review
  GET    /api/v1/repos/{owner}/{name}/reviews/{id}               # Get review
  POST   /api/v1/repos/{owner}/{name}/reviews/{id}/vote          # Add vote
  POST   /api/v1/repos/{owner}/{name}/reviews/{id}/submit        # Submit change

Comments:
  GET    /api/v1/repos/{owner}/{name}/reviews/{id}/comments      # List comments
  POST   /api/v1/repos/{owner}/{name}/reviews/{id}/comments      # Add comment
  PUT    /api/v1/repos/{owner}/{name}/reviews/{id}/comments/{cid} # Update
  POST   /api/v1/repos/{owner}/{name}/reviews/{id}/comments/{cid}/resolve

Operations:
  GET    /api/v1/repos/{owner}/{name}/operations          # List operations
  GET    /api/v1/repos/{owner}/{name}/operations/{op_id}  # Get operation
  GET    /api/v1/repos/{owner}/{name}/operations/head     # Get current head(s)

Tree/File Access:
  GET    /api/v1/repos/{owner}/{name}/tree/{commit_id}/{path}    # Get tree/file
  GET    /api/v1/repos/{owner}/{name}/blame/{commit_id}/{path}   # Get blame
```

### GraphQL API (Optional)

For the web UI, a GraphQL API may be more efficient:

```graphql
type Repository {
  id: ID!
  name: String!
  owner: User!
  changes(status: ChangeStatus, first: Int, after: String): ChangeConnection!
  operations(first: Int, after: String): OperationConnection!
}

type Change {
  id: ID!
  changeId: String!  # jj change ID
  currentCommit: Commit!
  commits: [Commit!]!  # all versions of this change
  review: Review
  status: ChangeStatus!
}

type Commit {
  id: ID!
  commitId: String!
  changeId: String!
  parents: [Commit!]!
  predecessors: [Commit!]!  # jj predecessor chain
  tree: Tree!
  author: Signature!
  committer: Signature!
  message: String!
}

type Review {
  id: ID!
  change: Change!
  status: ReviewStatus!
  votes: [Vote!]!
  comments: [Comment!]!
}
```

---

## Review System

### Change-Based Reviews (Gerrit-style)

Unlike GitHub's branch-based PRs, Forjj uses change-based reviews:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    Change-Based Review Flow                                  │
└─────────────────────────────────────────────────────────────────────────────┘

Developer Workflow:

  1. Create change:     jj new main -m "Add feature X"
                                   │
                                   ▼
  2. Make edits:        (edit files)
                                   │
                                   ▼
  3. Push for review:   jj push --change @
                        (native Forjj protocol)
                                   │
                                   ▼
  4. Address feedback:  jj describe -m "Updated"
                        (jj automatically amends)
                                   │
                                   ▼
  5. Push update:       jj push --change @
                        (same change ID, new commit)


Server-side:

  Push v1                        Push v2
  (commit abc, change xyz)       (commit def, change xyz)
         │                              │
         ▼                              ▼
    ┌─────────┐                   ┌─────────┐
    │ Review  │ ◄─── same ───────►│ Review  │
    │ created │      review       │ updated │
    └─────────┘                   └─────────┘

  Comments and votes persist across change versions.
  Reviewers see diff between v1 and v2 if desired.
```

### Approval Labels

Following Gerrit's model:

| Label | Range | Meaning |
|-------|-------|---------|
| Code-Review | -2 to +2 | Code quality assessment |
| Verified | -1 to +1 | Automated testing status |

```
Submit requirements (configurable per-repo):
  - Code-Review: at least one +2, no -2
  - Verified: at least one +1, no -1
  - No unresolved comments
```

### Automatic Rebase Handling

When dependent changes are modified, jj's automatic rebase kicks in:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    Automatic Rebase on Change Update                        │
└─────────────────────────────────────────────────────────────────────────────┘

Before:                          After updating Change A:

    main                             main
      │                                │
      ▼                                ▼
  Change A (commit abc)           Change A' (commit xyz)
      │                                │
      ▼                                ▼
  Change B (commit def)           Change B' (commit uvw)
      │                         (auto-rebased by jj)
      ▼                                │
  Change C (commit ghi)                ▼
                                  Change C' (commit rst)
                                (auto-rebased by jj)

Forjj detects rebased commits via predecessor tracking and:
  1. Updates review to show new commit versions
  2. Maintains comment thread positions via predecessor chain
  3. Notifies reviewers of rebased changes
```

---

## Security Model

### Authentication

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    Authentication Flows                                      │
└─────────────────────────────────────────────────────────────────────────────┘

SSH Authentication:
  1. Client initiates SSH connection
  2. Server verifies SSH public key against user's registered keys
  3. Connection established with user identity

HTTPS Authentication:
  Option A: Personal Access Token (header or basic auth)
  Option B: OAuth2 flow (GitHub, GitLab, etc.)
  Option C: OIDC integration

Web UI Authentication:
  - Session-based with secure cookies
  - OAuth2/OIDC integration
  - Optional 2FA (TOTP)
```

### Authorization Model

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    Permission Hierarchy                                      │
└─────────────────────────────────────────────────────────────────────────────┘

                    ┌─────────────────┐
                    │  Instance Level │
                    │  (admin users)  │
                    └────────┬────────┘
                             │
              ┌──────────────┼──────────────┐
              │              │              │
              ▼              ▼              ▼
       ┌──────────┐   ┌──────────┐   ┌──────────┐
       │   Org    │   │   Org    │   │  User    │
       │  Admin   │   │  Member  │   │  Repos   │
       └────┬─────┘   └────┬─────┘   └────┬─────┘
            │              │              │
            ▼              ▼              ▼
       ┌────────────────────────────────────────┐
       │           Repository Level             │
       │                                        │
       │  admin:  full control                  │
       │  write:  push, create reviews          │
       │  read:   fetch, view reviews           │
       └────────────────────────────────────────┘

Per-branch restrictions (optional):
  refs/heads/main:
    - push: requires Code-Review +2
    - force-push: denied
  refs/heads/*:
    - push: allowed for 'write' permission
```

---

## Trade-offs and Decisions

### Decision 1: Rust as Backend Language

**Options Considered:**
1. **Rust** - Excellent for systems programming, can reuse jj-lib directly
2. **Go** - Simple concurrency, fast compile times
3. **Zig** - Low-level control, C interop, no hidden control flow

**Decision: Rust**

Rationale:
- **Direct jj-lib integration**: Can depend on jj-lib crate directly
- **Guaranteed format compatibility**: Uses same serialization code as jj
- **Mature ecosystem**: Excellent libraries for HTTP (axum), SSH (thrussh/russh), async (tokio)
- **Memory safety**: Without garbage collection pauses
- **Single binary deployment**: With static linking
- **Upstream contribution path**: Same language as jj, easier to contribute patches

Trade-offs accepted:
- Longer compile times than Go
- Steeper learning curve than Go
- Larger binary size than Zig

### jj-lib Integration Strategy

Using Rust allows direct integration with jj-lib:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    Direct jj-lib Integration                                 │
└─────────────────────────────────────────────────────────────────────────────┘

                    ┌─────────────────────────────┐
                    │   jj-vcs/jj repository      │
                    │   (upstream source of truth)│
                    └──────────────┬──────────────┘
                                   │
                    ┌──────────────▼──────────────┐
                    │         jj-lib crate        │
                    │   (Cargo dependency)        │
                    └──────────────┬──────────────┘
                                   │
                                   ▼
                            ┌──────────┐
                            │  Forjj   │
                            │  (Rust)  │
                            └──────────┘

Benefits:
- No format reimplementation needed
- Automatic compatibility with jj updates
- Can implement Backend trait directly
- Access to jj's conflict handling, revsets, etc.
```

**Integration Approach:**

1. **Cargo Dependency**
   ```toml
   [dependencies]
   jj-lib = "0.38"  # Pin to specific version
   ```

2. **Backend Trait Implementation**
   - Implement `jj_lib::backend::Backend` for server-side storage
   - Leverage existing `SimpleBackend` or `GitBackend` code as reference
   - Add server-specific features (access control, quotas)

3. **Version Pinning**
   - Pin jj-lib version in Cargo.toml
   - Test against specific jj CLI versions in CI
   - Document compatible jj version range

4. **Upstream Engagement**
   - Contribute useful abstractions back to jj-lib
   - Participate in API stability discussions
   - Report bugs found during Forjj development

### Decision 2: SQLite vs PostgreSQL for Metadata

**Options:**
1. **SQLite** - Embedded, zero-config, good for small deployments
2. **PostgreSQL** - Scalable, better concurrent access, rich features

**Decision: SQLite default, PostgreSQL option**

Rationale:
- SQLite for single-instance deployments (simpler ops)
- PostgreSQL for multi-instance or high-traffic deployments
- Abstract database layer to support both

Trade-offs:
- Two code paths to maintain
- SQLite write contention under heavy load
- PostgreSQL adds operational complexity

### Decision 3: Native jj Backend

**Options:**
1. **Git backend only** - Use jj with Git storage
2. **Native backend only** - Use jj's native protobuf format
3. **Git first, native later** - Start with Git, add native support

**Decision: Native backend only**

Rationale:
- Architectural purity: no Git abstraction layer
- Storage efficiency: single storage format, no duplication
- Aligns with project goal of being jj-native, not jj-over-git
- Willing to upstream fixes to jj's native backend as needed
- Simpler operational model (no Git GC concerns)

Trade-offs accepted:
- Native backend less battle-tested than Git backend
- No `git log` for debugging (need custom tooling)
- May require contributing upstream fixes
- Users must use jj client (no Git CLI fallback)

Mitigation strategy:
- Comprehensive test suite for storage layer
- Build debugging CLI tools for inspecting native format
- Maintain close relationship with jj upstream
- Document native format clearly for troubleshooting

### Decision 4: Monorepo Structure

```
forjj/
├── backend/           # Zig backend server
│   ├── src/
│   │   ├── main.zig
│   │   ├── api/       # HTTP/REST handlers
│   │   ├── git/       # Git protocol implementation
│   │   ├── jj/        # jj operation handling
│   │   ├── review/    # Review logic
│   │   └── storage/   # Storage abstraction
│   ├── build.zig
│   └── tests/
├── frontend/          # React frontend
│   ├── src/
│   │   ├── components/
│   │   ├── pages/
│   │   └── api/
│   ├── package.json
│   └── vite.config.ts
├── nix/               # Nix build configuration
│   ├── default.nix
│   └── shell.nix
├── docs/
├── CLAUDE.md
├── DESIGN.md
└── flake.nix
```

Rationale:
- Single repository for coordinated changes
- Shared Nix configuration for reproducible builds
- Simpler CI/CD pipeline

### Decision 5: Wire Protocol

**Options:**
1. **Full Git compatibility** - Implement Git protocol, translate internally
2. **jj-native protocol** - New protocol, require jj client with Forjj support
3. **Extended Git protocol** - Git-compatible with extensions

**Decision: jj-native protocol**

Rationale:
- Native backend requires native protocol (Git protocol assumes Git objects)
- Simpler implementation: no Git protocol complexity
- More efficient: operation-aware sync instead of ref-based
- Designed for upstream contribution to jj

Trade-offs accepted:
- Requires jj client modifications or Forjj-specific remote helper
- No Git CLI fallback (aligns with native backend decision)
- Initial adoption friction until jj gains native remote support

Implementation path:
1. Build Forjj with custom protocol
2. Create `jj-forjj` remote helper for jj CLI integration
3. Propose protocol upstream to jj for native remote support
4. Eventually: `jj push origin` works natively with Forjj

---

## Implementation Phases

### Phase 1: Storage Foundation
- Implement native jj backend reader/writer in Zig
- Protobuf encoding/decoding for jj objects
- Content-addressed object storage
- Operation log management

### Phase 2: Wire Protocol & Sync
- Implement forjj-sync protocol (SSH transport first)
- Object pack streaming
- Operation log CRDT merge
- Create jj remote helper for CLI integration

### Phase 3: Repository Management
- Repository CRUD operations
- User authentication (SSH keys)
- Basic permission model
- REST API for repo management

### Phase 4: Change Tracking & Reviews
- Change ID extraction from pushed commits
- Predecessor tracking for amendment history
- Review creation and voting
- Comment threads with line-level positioning

### Phase 5: Web UI
- Repository browser (tree, file viewer)
- Change diff viewer (with conflict visualization)
- Review interface
- HTTPS protocol support

### Phase 6: Production Readiness
- Multi-instance replication
- Plugin/webhook system for CI integration
- Performance optimization and caching
- Comprehensive documentation

---

## jj Client Integration

Since we're using a native protocol, we need a way for jj CLI users to push/fetch from Forjj.

### Option A: Remote Helper (Recommended for Phase 1)

Create a `jj-remote-forjj` helper that jj can invoke:

```
# User configuration
$ jj config set --repo remotes.origin.url "forjj://forjj.example.com/owner/repo"
$ jj config set --repo remotes.origin.helper "jj-remote-forjj"

# Usage
$ jj push origin
# jj invokes: jj-remote-forjj push forjj://forjj.example.com/owner/repo
```

The helper is a standalone binary (written in Zig or Rust) that:
1. Speaks the Forjj wire protocol
2. Reads/writes jj's native object format
3. Handles SSH/HTTPS transport and authentication

### Option B: Upstream jj Support (Long-term Goal)

Work with jj maintainers to add native remote support:

```rust
// Proposed jj-lib addition
trait Remote {
    fn fetch(&self, request: FetchRequest) -> Result<FetchResponse>;
    fn push(&self, request: PushRequest) -> Result<PushResponse>;
}

// Forjj implementation
struct ForjjRemote { ... }
impl Remote for ForjjRemote { ... }
```

This would allow:
```
$ jj remote add origin forjj://forjj.example.com/owner/repo
$ jj push origin  # Works natively
```

### Phased Approach

1. **Phase 1**: Ship `jj-remote-forjj` helper as separate download
2. **Phase 2**: Propose Remote trait upstream to jj
3. **Phase 3**: Contribute ForjjRemote implementation to jj
4. **Phase 4**: Forjj support is built into jj

---

## Open Questions

1. **jj-lib vs reimplementation**: Should we create Zig bindings to jj-lib via FFI for object parsing,
   or reimplement the protobuf schemas in Zig? FFI adds complexity but ensures compatibility.
   Reimplementation is cleaner but risks drift from jj's format.

2. **Conflict UI**: How should the web UI display jj's first-class conflicts in review diffs?
   Options: tree-of-conflicts view, inline markers, or side-by-side-by-side for 3-way conflicts.

3. **Replication consistency**: What consistency guarantees should multi-instance deployments provide?
   The CRDT nature of operation logs suggests eventual consistency is natural, but ref updates
   may need stronger guarantees.

4. **Plugin architecture**: What extension points should be exposed for CI integration, custom
   approval rules, etc.? Webhook-based (like GitHub) or in-process plugins (like Gerrit)?

5. **Remote helper distribution**: How do we distribute jj-remote-forjj? Options: standalone binary,
   cargo install, nix package, bundled with Forjj server download.

6. **Upstream relationship**: How closely should we coordinate with jj maintainers? Should we
   propose the native remote protocol before building, or build first and propose later?

---

## References

- [jj-vcs/jj Repository](https://github.com/jj-vcs/jj)
- [jj Documentation](https://docs.jj-vcs.dev/latest/)
- [jj Architecture](https://docs.jj-vcs.dev/latest/technical/architecture/)
- [jj Concurrency Model](https://docs.jj-vcs.dev/latest/technical/concurrency/)
- [Gerrit System Design](https://gerrit-review.googlesource.com/Documentation/dev-design.html)
- [Git Protocol Documentation](https://git-scm.com/docs/protocol-v2)
