//! Object ID types for content-addressed storage.
//!
//! jj uses BLAKE2b-256 for content addressing. These types wrap the raw bytes
//! and provide convenience methods for hex encoding/decoding.

use blake2::digest::consts::U32;
use blake2::{Blake2b, Digest};

type Blake2b256 = Blake2b<U32>;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Length of object IDs in bytes (BLAKE2b-256 = 32 bytes)
pub const HASH_LEN: usize = 32;

/// Generic content-addressed object identifier.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ObjectId([u8; HASH_LEN]);

impl ObjectId {
    /// Create an ObjectId from raw bytes.
    pub fn from_bytes(bytes: [u8; HASH_LEN]) -> Self {
        Self(bytes)
    }

    /// Create an ObjectId from a byte slice.
    pub fn from_slice(slice: &[u8]) -> Result<Self, ObjectIdError> {
        if slice.len() != HASH_LEN {
            return Err(ObjectIdError::InvalidLength {
                expected: HASH_LEN,
                actual: slice.len(),
            });
        }
        let mut bytes = [0u8; HASH_LEN];
        bytes.copy_from_slice(slice);
        Ok(Self(bytes))
    }

    /// Create an ObjectId from a hex string.
    pub fn from_hex(hex: &str) -> Result<Self, ObjectIdError> {
        if hex.len() != HASH_LEN * 2 {
            return Err(ObjectIdError::InvalidHexLength {
                expected: HASH_LEN * 2,
                actual: hex.len(),
            });
        }
        let mut bytes = [0u8; HASH_LEN];
        hex::decode_to_slice(hex, &mut bytes).map_err(|_| ObjectIdError::InvalidHexCharacter)?;
        Ok(Self(bytes))
    }

    /// Get the raw bytes.
    pub fn as_bytes(&self) -> &[u8; HASH_LEN] {
        &self.0
    }

    /// Convert to hex string.
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Hash data to produce an ObjectId.
    pub fn hash(data: &[u8]) -> Self {
        let mut hasher = Blake2b256::new();
        hasher.update(data);
        let result = hasher.finalize();
        let mut bytes = [0u8; HASH_LEN];
        bytes.copy_from_slice(&result);
        Self(bytes)
    }
}

impl fmt::Display for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl fmt::Debug for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ObjectId({})", &self.to_hex()[..12])
    }
}

/// Error type for ObjectId parsing.
#[derive(Debug, thiserror::Error)]
pub enum ObjectIdError {
    #[error("invalid length: expected {expected}, got {actual}")]
    InvalidLength { expected: usize, actual: usize },

    #[error("invalid hex length: expected {expected}, got {actual}")]
    InvalidHexLength { expected: usize, actual: usize },

    #[error("invalid hex character")]
    InvalidHexCharacter,
}

// Type aliases for semantic clarity
pub type CommitId = ObjectId;
pub type ChangeId = ObjectId;
pub type TreeId = ObjectId;
pub type FileId = ObjectId;
pub type SymlinkId = ObjectId;
pub type ConflictId = ObjectId;
pub type OperationId = ObjectId;
pub type ViewId = ObjectId;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_produces_consistent_results() {
        let id1 = ObjectId::hash(b"hello, forjj!");
        let id2 = ObjectId::hash(b"hello, forjj!");
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_different_data_produces_different_hashes() {
        let id1 = ObjectId::hash(b"hello");
        let id2 = ObjectId::hash(b"world");
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_hex_roundtrip() {
        let original = ObjectId::hash(b"test data");
        let hex = original.to_hex();
        let parsed = ObjectId::from_hex(&hex).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn test_from_hex_invalid_length() {
        let result = ObjectId::from_hex("0123");
        assert!(matches!(
            result,
            Err(ObjectIdError::InvalidHexLength { .. })
        ));
    }

    #[test]
    fn test_from_hex_invalid_character() {
        let invalid = "g".repeat(64);
        let result = ObjectId::from_hex(&invalid);
        assert!(matches!(result, Err(ObjectIdError::InvalidHexCharacter)));
    }
}
