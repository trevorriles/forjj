//! Forjj Wire Protocol
//!
//! This crate implements the forjj-sync protocol for pushing and fetching
//! repositories between jj clients and the Forjj server.

pub mod framing;
pub mod messages;

pub use framing::{read_frame, write_frame, FrameError};
pub use messages::{
    Capability, FetchRequest, FetchResponse, HelloRequest, HelloResponse, PushRequest,
    PushResult, PushStatus, RefUpdate,
};

/// Protocol version
pub const PROTOCOL_VERSION: u32 = 1;
