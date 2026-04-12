//! The `RpcCodec` trait — translates between `RpcMessage<V>` and raw bytes.
//!
//! ## What Is a Codec?
//!
//! Think of a codec like a human interpreter in a meeting room. The meeting
//! room is the RPC server. The *agenda items* (requests, responses,
//! notifications) are the `RpcMessage` values. The interpreter's job is to
//! translate them to and from a specific language — English for JSON, a compact
//! binary notation for MessagePack, and so on.
//!
//! The meeting room (the server) does not care which language is being spoken.
//! It only cares about the *agenda items*. The interpreter (the codec) does not
//! care about the agenda — it only knows how to translate words.
//!
//! ```text
//! ┌─────────────────────────────┐
//! │  RpcServer (dispatch)       │
//! │    ↕  RpcMessage<V>         │
//! ├─────────────────────────────┤
//! │  RpcCodec<V>  (this module) │
//! │    ↕  Vec<u8>               │
//! ├─────────────────────────────┤
//! │  RpcFramer    (see framer)  │
//! │    ↕  raw stream            │
//! └─────────────────────────────┘
//! ```
//!
//! ## Statelessness
//!
//! A codec is stateless. Every call to `encode` and `decode` is independent.
//! There is no session state, no sequence counter, no buffer. The framer holds
//! any per-connection state.
//!
//! ## The `V` Type Parameter
//!
//! `V` is the codec's native *dynamic value* type:
//! - **JSON**: `V = serde_json::Value` — a JSON number, string, object, etc.
//! - **MessagePack**: `V = rmpv::Value` — a compact binary equivalent.
//! - **Tests**: `V = serde_json::Value` — easy to construct literals with `json!`.
//!
//! ## Example (in-crate test codec)
//!
//! ```rust
//! use coding_adventures_rpc::codec::RpcCodec;
//! use coding_adventures_rpc::message::{RpcMessage, RpcRequest};
//! use coding_adventures_rpc::errors::RpcError;
//! use serde_json::Value;
//!
//! // A trivial codec that serialises RpcMessage<Value> as JSON.
//! struct TrivialJsonCodec;
//!
//! impl RpcCodec<Value> for TrivialJsonCodec {
//!     fn encode(&self, msg: &RpcMessage<Value>) -> Result<Vec<u8>, RpcError> {
//!         // In a real codec, this would build the protocol-specific envelope.
//!         Ok(b"encoded".to_vec())
//!     }
//!     fn decode(&self, _data: &[u8]) -> Result<RpcMessage<Value>,
//!             coding_adventures_rpc::message::RpcErrorResponse<Value>> {
//!         // In a real codec, this would parse the bytes.
//!         Err(coding_adventures_rpc::message::RpcErrorResponse {
//!             id: None, code: -32700,
//!             message: "not implemented".into(), data: None,
//!         })
//!     }
//! }
//! ```

use crate::errors::RpcError;
use crate::message::{RpcErrorResponse, RpcMessage};

/// Translate between `RpcMessage<V>` and raw bytes.
///
/// Implementors are responsible for the serialisation format only — not framing
/// (that is [`crate::framer::RpcFramer`]'s job), and not dispatch (that is
/// [`crate::server::RpcServer`]'s job).
///
/// A codec is expected to be **stateless** and **cheap to clone or share**.
/// Multiple calls to `encode`/`decode` on the same codec instance must be
/// independent.
///
/// # Error handling
///
/// - `encode` returns `Err(RpcError)` only for internal encoding failures
///   (e.g., a value that cannot be represented in the target format).
///   In practice this is rare — most formats can represent arbitrary values.
///
/// - `decode` returns `Err(RpcErrorResponse<V>)` when the bytes cannot be
///   parsed. The error response is fully formed and ready to send back to the
///   caller:
///   - **Parse error (-32700)**: bytes are not valid for the format at all.
///   - **Invalid request (-32600)**: bytes are valid format but not a valid
///     RPC message shape (e.g., valid JSON but missing `method`/`result`).
///
/// # Type parameters
///
/// - `V`: the codec's native dynamic value type (e.g. `serde_json::Value`).
pub trait RpcCodec<V> {
    /// Encode an `RpcMessage` to bytes ready for the framer.
    ///
    /// The returned bytes are the *payload only* — no framing envelope.
    /// The framer will wrap them before writing to the stream.
    ///
    /// # Errors
    ///
    /// Returns `Err(RpcError)` if encoding fails. This should be rare —
    /// report the situation as an [`RpcError`] with a descriptive message.
    fn encode(&self, msg: &RpcMessage<V>) -> Result<Vec<u8>, RpcError>;

    /// Decode a byte slice from the framer into a typed `RpcMessage`.
    ///
    /// The `data` slice is the *payload only* — the framer has already
    /// stripped any envelope (Content-Length header, length prefix, etc.).
    ///
    /// # Errors
    ///
    /// Returns `Err(RpcErrorResponse<V>)` when decoding fails. The error
    /// response is pre-built and ready to send over the wire:
    ///
    /// - `code = PARSE_ERROR (-32700)` — bytes are not valid for this format.
    /// - `code = INVALID_REQUEST (-32600)` — valid format but not an RPC message.
    fn decode(&self, data: &[u8]) -> Result<RpcMessage<V>, RpcErrorResponse<V>>;
}
