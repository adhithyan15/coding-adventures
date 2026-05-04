//! `Transport` trait — the pluggable wire layer.
//!
//! Every executor is reached through a `Transport`.  The runtime
//! doesn't know whether the transport is in-process, over a Unix
//! socket, over TCP, over ZeroMQ, or anything else — it only knows
//! how to call `request()`.
//!
//! V1 ships only [`LocalTransport`](crate::LocalTransport).  Future
//! crates supply additional transports (`matrix-transport-tcp`,
//! `matrix-transport-zmq`, …) without any change to this crate.
//!
//! ## Async-first
//!
//! The trait uses `async fn` so that network transports can perform
//! real I/O without restructuring.  In-process transports return
//! immediately-ready futures.  Drive them with
//! [`block_on`](crate::block_on).

use crate::messages::{ExecutorRequest, ExecutorResponse};
use core::future::Future;

/// Errors produced by transports.
#[derive(Clone, Debug)]
pub enum TransportError {
    /// Transport is closed (executor went away, socket disconnected,
    /// etc.).
    Closed,
    /// Wire-level error decoding the response.
    Wire(crate::wire::WireError),
    /// Transport-specific I/O error.  Free-form because every
    /// transport has its own underlying error type.
    Io(String),
    /// Timeout waiting for a response.
    Timeout,
}

impl From<crate::wire::WireError> for TransportError {
    fn from(e: crate::wire::WireError) -> Self {
        TransportError::Wire(e)
    }
}

/// The pluggable transport contract.
///
/// `Transport` is `Send + Sync` because the runtime may share it
/// across threads (one thread submits requests while another awaits
/// responses).  Implementations that aren't naturally thread-safe
/// can wrap themselves in a `Mutex`.
///
/// Sub-traits or wrappers may add more methods (`subscribe()` for
/// events, `flush()` for buffered transports), but the core contract
/// is just `request`.
pub trait Transport: Send + Sync {
    /// Send a request to the executor and await its response.
    ///
    /// Correlation-id management is the transport's responsibility —
    /// callers don't need to think about request ordering on the wire.
    /// In-process transports may not even bother with correlation
    /// (they call the handler directly), but the type signature is the
    /// same.
    fn request(
        &self,
        req: ExecutorRequest,
    ) -> impl Future<Output = Result<ExecutorResponse, TransportError>> + Send;
}
