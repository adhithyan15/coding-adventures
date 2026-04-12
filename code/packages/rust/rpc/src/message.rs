//! Codec-agnostic RPC message types.
//!
//! ## What Is a Message?
//!
//! Every RPC system exchanges four kinds of messages:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │  Message type      │ Has id? │ Has method? │ Expects reply?         │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │  Request           │  yes    │  yes        │  yes                   │
//! │  Response          │  yes    │  no         │  no (IS the reply)     │
//! │  ErrorResponse     │  maybe  │  no         │  no (IS the reply)     │
//! │  Notification      │  no     │  yes        │  no                    │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## The `V` Type Parameter
//!
//! All message types are parameterised by `V`, the codec's native *value* type:
//!
//! - **JSON codec**: `V = serde_json::Value`
//! - **MessagePack codec**: `V = rmpv::Value`
//! - **In tests**: `V = serde_json::Value` (convenient and widely available)
//!
//! The RPC layer never inspects `V` — it only passes it between the codec and
//! the handler. This is a key design goal: the dispatch logic, panic recovery,
//! and error handling are the same regardless of the serialisation format.
//!
//! ## `RpcId`
//!
//! An RPC id uniquely correlates a response to its request. The id is generated
//! by the client and echoed verbatim by the server. Valid id values are:
//!
//! - A JSON string: `"abc-123"`
//! - A JSON integer: `42`
//! - JSON null: only in `RpcErrorResponse` when the request was so malformed
//!   the server could not extract the original id.
//!
//! We represent `RpcId` as `serde_json::Value` because it natively covers
//! all three cases without requiring a separate enum.
//!
//! ```text
//! RpcId = String | Integer | Null
//!                            ^^^^
//!                            only in RpcErrorResponse when id was unknown
//! ```

use serde_json::Value;

// ---------------------------------------------------------------------------
// RpcId type alias
// ---------------------------------------------------------------------------

/// The id type for RPC messages — a JSON string, integer, or null.
///
/// We use `serde_json::Value` because it covers `String`, `Number`, and `Null`
/// without any custom enum boilerplate. The RPC layer never inspects the
/// concrete variant — only the codec and client need to care about what kind
/// of id was used.
///
/// # Conventions
///
/// | Scenario                              | Value to use             |
/// |---------------------------------------|--------------------------|
/// | Integer id (most common)              | `Value::Number(1.into())`|
/// | String id                             | `Value::String("x".into())`|
/// | Unknown id in error response          | `Value::Null`            |
pub type RpcId = Value;

// ---------------------------------------------------------------------------
// RpcRequest
// ---------------------------------------------------------------------------

/// A client-to-server call that expects a response.
///
/// The server must send back an [`RpcResponse`] or [`RpcErrorResponse`] with
/// the same `id`. If the server sends nothing, the client will wait forever
/// (in the synchronous model) or the pending-request will leak (in async).
///
/// # Example (JSON representation)
///
/// ```json
/// {"jsonrpc":"2.0","id":1,"method":"textDocument/hover","params":{"line":10}}
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct RpcRequest<V> {
    /// Correlates the response back to this request.
    ///
    /// Must not be null for requests. The server echoes this value back
    /// unchanged in the corresponding `RpcResponse`.
    pub id: RpcId,

    /// The name of the procedure to call, e.g. `"textDocument/hover"`.
    ///
    /// Method names are case-sensitive. The server's dispatch table does an
    /// exact string match.
    pub method: String,

    /// Optional parameters to the procedure.
    ///
    /// The codec is responsible for deserialising these into the handler's
    /// expected type. If the handler expects named params it will receive an
    /// object; if it expects positional params it will receive an array.
    pub params: Option<V>,
}

// ---------------------------------------------------------------------------
// RpcResponse
// ---------------------------------------------------------------------------

/// A successful server-to-client reply.
///
/// Sent when the handler returns `Ok(result)`. The `id` must match the
/// originating [`RpcRequest`].
///
/// # Example (JSON representation)
///
/// ```json
/// {"jsonrpc":"2.0","id":1,"result":{"contents":"**INC** — Increment register"}}
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct RpcResponse<V> {
    /// The id from the originating [`RpcRequest`].
    pub id: RpcId,

    /// The successful result value.
    ///
    /// The codec encodes this as whatever the wire format uses for values.
    pub result: V,
}

// ---------------------------------------------------------------------------
// RpcErrorResponse
// ---------------------------------------------------------------------------

/// An error reply — sent when the handler returns `Err` or when the
/// infrastructure encounters a protocol-level error.
///
/// Three distinct situations produce an `RpcErrorResponse`:
///
/// 1. **Application error**: the handler called `on_request` returned
///    `Err(RpcErrorResponse { code: INVALID_PARAMS, ... })`.
/// 2. **Infrastructure error**: the codec failed to decode the frame bytes
///    (`PARSE_ERROR`) or the request was well-formed but structurally invalid
///    (`INVALID_REQUEST`).
/// 3. **Method not found**: no handler is registered for the method
///    (`METHOD_NOT_FOUND`).
/// 4. **Panic recovery**: the handler panicked; `serve()` catches the panic
///    and sends `INTERNAL_ERROR`.
///
/// # Example (JSON representation)
///
/// ```json
/// {"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"Method not found","data":"foo"}}
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct RpcErrorResponse<V> {
    /// The id from the originating request.
    ///
    /// This is `Option<RpcId>` because when the frame is so malformed that the
    /// server cannot extract an id, it must still send an error — with `id:
    /// null`. `None` here serialises to JSON `null`.
    pub id: Option<RpcId>,

    /// Integer error code. Use the constants in [`crate::errors`].
    pub code: i64,

    /// Short, stable, human-readable description.
    ///
    /// Should be stable across versions — clients may display it directly or
    /// match on it as a fallback.
    pub message: String,

    /// Optional structured data providing extra context.
    ///
    /// For `METHOD_NOT_FOUND` this is typically the unknown method name.
    /// For `PARSE_ERROR` it might be the raw error from the codec.
    pub data: Option<V>,
}

// ---------------------------------------------------------------------------
// RpcNotification
// ---------------------------------------------------------------------------

/// A one-way message with no response.
///
/// Notifications are fire-and-forget: the sender does not expect any reply and
/// the receiver must not send one. This makes notifications suitable for
/// events, logging, and progress updates.
///
/// # Example (JSON representation)
///
/// ```json
/// {"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"uri":"file:///foo.rs"}}
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct RpcNotification<V> {
    /// The event name, e.g. `"textDocument/didOpen"`.
    pub method: String,

    /// Optional event parameters.
    pub params: Option<V>,
}

// ---------------------------------------------------------------------------
// RpcMessage — the discriminated union
// ---------------------------------------------------------------------------

/// All four RPC message types in one enum.
///
/// Use pattern matching to handle each case:
///
/// ```rust
/// use coding_adventures_rpc::message::{RpcMessage, RpcRequest};
/// use serde_json::Value;
///
/// fn handle(msg: RpcMessage<Value>) {
///     match msg {
///         RpcMessage::Request(req) => println!("request: {}", req.method),
///         RpcMessage::Response(resp) => println!("response id: {:?}", resp.id),
///         RpcMessage::ErrorResponse(err) => println!("error: {}", err.message),
///         RpcMessage::Notification(notif) => println!("notification: {}", notif.method),
///     }
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum RpcMessage<V> {
    /// A client-to-server call that expects a response.
    Request(RpcRequest<V>),

    /// A successful server-to-client reply.
    Response(RpcResponse<V>),

    /// An error server-to-client reply.
    ErrorResponse(RpcErrorResponse<V>),

    /// A one-way fire-and-forget message.
    Notification(RpcNotification<V>),
}
