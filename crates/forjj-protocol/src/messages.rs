//! Protocol message definitions for forjj-sync/1.0

use forjj_storage::OperationId;
use serde::{Deserialize, Serialize};

/// Capabilities that can be negotiated between client and server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    /// Include operation log in sync
    Operations,
    /// Delta compression for object packs
    ThinPack,
    /// Resumable transfers
    Resumable,
}

/// Initial handshake from client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelloRequest {
    pub protocol_version: u32,
    pub capabilities: Vec<Capability>,
    pub client_op_heads: Vec<OperationId>,
}

/// Server response to handshake.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelloResponse {
    pub protocol_version: u32,
    pub capabilities: Vec<Capability>,
    pub server_op_heads: Vec<OperationId>,
    pub common_ancestor: Option<OperationId>,
}

/// Fetch request from client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchRequest {
    /// Operations the client already has
    pub have_ops: Vec<OperationId>,
    /// Bookmark names to fetch
    pub want_refs: Vec<String>,
    /// Shallow fetch limit (optional)
    pub depth: Option<u32>,
}

/// Fetch response header.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchResponse {
    /// Whether an object pack follows this message
    pub pack_follows: bool,
    /// Operations that will be sent
    pub ops_to_send: Vec<OperationId>,
    /// Number of commits in the pack
    pub commit_count: u64,
}

/// Push request from client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushRequest {
    /// Operations the client has
    pub have_ops: Vec<OperationId>,
    /// Reference updates to apply
    pub updates: Vec<RefUpdate>,
}

/// Reference update in a push.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefUpdate {
    /// Name of the reference (bookmark)
    pub ref_name: String,
    /// Expected current value (None for create)
    pub old_id: Option<String>,
    /// New value (None for delete)
    pub new_id: Option<String>,
}

/// Server response to push negotiation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushNegotiate {
    /// Common ancestor operation
    pub common_op: Option<OperationId>,
    /// Whether the server needs objects
    pub need_objects: bool,
}

/// Final push result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushResult {
    /// Overall status
    pub status: PushStatus,
    /// New operation head after push
    pub new_op_head: Option<OperationId>,
    /// Per-reference results
    pub ref_results: Vec<RefResult>,
}

/// Push status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PushStatus {
    Ok,
    Rejected,
    Conflict,
}

/// Result for a single reference update.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefResult {
    pub ref_name: String,
    pub status: RefStatus,
    pub message: Option<String>,
}

/// Status for a single reference update.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefStatus {
    Ok,
    Rejected,
    /// Fast-forward required
    Stale,
    /// Bookmark conflict
    Conflict,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hello_request_serialization() {
        let request = HelloRequest {
            protocol_version: 1,
            capabilities: vec![Capability::Operations],
            client_op_heads: vec![],
        };

        let json = serde_json::to_string(&request).unwrap();
        let parsed: HelloRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.protocol_version, 1);
        assert_eq!(parsed.capabilities, vec![Capability::Operations]);
    }
}
