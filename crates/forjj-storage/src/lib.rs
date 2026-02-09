//! Forjj Storage Layer
//!
//! This crate provides the storage abstraction layer for Forjj, wrapping jj-lib
//! to provide repository management, object storage, and operation log handling.

pub mod object_id;
pub mod repository;

pub use object_id::{ChangeId, CommitId, FileId, ObjectId, OperationId, TreeId, ViewId};
pub use repository::{BackendType, RepoInfo, Repository, RepositoryManager, StorageConfig};

/// Re-export jj-lib for direct access when needed
pub use jj_lib;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn object_id_hash_is_deterministic() {
        let id1 = ObjectId::hash(b"test");
        let id2 = ObjectId::hash(b"test");
        assert_eq!(id1, id2);
    }
}
