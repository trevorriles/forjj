//! Forjj Storage Layer
//!
//! This crate provides the storage abstraction layer for Forjj, wrapping jj-lib
//! to provide repository management, object storage, and operation log handling.

pub mod object_id;
pub mod repository;

pub use object_id::{ChangeId, CommitId, FileId, ObjectId, OperationId, TreeId, ViewId};

/// Re-export jj-lib for direct access when needed
pub use jj_lib;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert!(true);
    }
}
