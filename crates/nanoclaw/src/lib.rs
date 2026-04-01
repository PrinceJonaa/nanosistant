//! `nstn-nanoclaw` — Edge-tier crate for the Nanosistant project.
//!
//! Provides:
//! - [`edge`]: Edge runtime that orchestrates local + remote resolution.
//! - [`grpc_client`]: gRPC client stub for the `RuFlo` brain tier.
//! - [`local`]: Local deterministic execution (delegates to `nstn-common`).
//! - [`sync`]: Offline queue for messages queued while the network is down.

pub mod edge;
pub mod grpc_client;
pub mod local;
pub mod sync;

pub use edge::{EdgeError, EdgeRequest, EdgeResponse, EdgeRuntime, SessionContext};
pub use grpc_client::{GrpcClient, GrpcError};
pub use local::LocalExecutor;
pub use sync::{OfflineQueue, QueuedMessage};
