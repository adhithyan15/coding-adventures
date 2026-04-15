//! `RpcClient` — sends requests and receives responses synchronously.
//!
//! ## Design Overview
//!
//! The client is the counterpart to [`crate::server::RpcServer`]. Where the
//! server passively waits for incoming messages and dispatches them, the client
//! actively sends messages and waits for the matching reply.
//!
//! ```text
//! ┌─────────────┐   request(method, params)   ┌──────────────┐
//! │  RpcClient  │ ──────────────────────────► │  RpcServer   │
//! │             │                             │  (remote)    │
//! │             │ ◄────────────────────────── │              │
//! └─────────────┘   Response / ErrorResponse  └──────────────┘
//! ```
//!
//! ## Id Management
//!
//! The client auto-generates request ids. It maintains a monotonically
//! increasing integer counter starting at 1. Each call to `request()` uses
//! the next id.
//!
//! ```text
//! call 1 → id=1
//! call 2 → id=2
//! call 3 → id=3
//! ...
//! ```
//!
//! Ids are never reused within a single client instance. If the client is
//! dropped and recreated the counter resets to 1, but in practice the server
//! treats ids as opaque and does not care about reuse across connections.
//!
//! ## Synchronous Blocking Model
//!
//! `request()` blocks until a response with a matching id arrives. While
//! waiting it may receive:
//! - Server-push notifications (handled via `on_notification` handlers).
//! - Responses for *other* ids (ignored — not expected in the single-threaded
//!   model).
//! - EOF (the connection closed) → returns an error.
//!
//! This model is appropriate for simple CLI tools, LSP clients, and test
//! harnesses. For concurrent workloads a thread-per-request or async model
//! is needed — that is a future extension.
//!
//! ## Usage
//!
//! ```rust,no_run
//! use coding_adventures_rpc::client::RpcClient;
//! use serde_json::Value;
//!
//! // Assume `codec` and `framer` are concrete implementations:
//! // let mut client: RpcClient<Value> = RpcClient::new(
//! //     Box::new(codec),
//! //     Box::new(framer),
//! // );
//! //
//! // let result = client.request("ping", None).unwrap();
//! // client.notify("log", Some(Value::String("hello".into()))).unwrap();
//! ```

use crate::codec::RpcCodec;
use crate::errors::RpcError;
use crate::framer::RpcFramer;
use crate::message::{
    RpcErrorResponse, RpcMessage, RpcNotification, RpcRequest,
};
use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Notification handler type
// ---------------------------------------------------------------------------

/// A handler for server-push notifications received while the client is
/// waiting for a request response.
///
/// Receives `params`. Its return value is ignored.
pub type ClientNotificationHandler<V> = Box<dyn Fn(Option<V>) + Send + 'static>;

// ---------------------------------------------------------------------------
// RpcClient struct
// ---------------------------------------------------------------------------

/// Synchronous, blocking RPC client.
///
/// Sends requests to a remote server and waits for responses. Also sends
/// fire-and-forget notifications.
///
/// # Type parameter `V`
///
/// `V` is the codec's native dynamic value type (e.g. `serde_json::Value`).
/// The bounds `Clone + Send + Serialize + DeserializeOwned + 'static` are the
/// minimum required to:
/// - Clone the value for encoding.
/// - Move it across thread boundaries.
/// - Serialise/deserialise via serde (needed for the codec).
pub struct RpcClient<V>
where
    V: Clone + Send + Serialize + DeserializeOwned + 'static,
{
    /// The codec translates between `RpcMessage<V>` and raw bytes.
    codec: Box<dyn RpcCodec<V>>,
    /// The framer reads/writes discrete byte chunks from/to the stream.
    framer: Box<dyn RpcFramer>,
    /// Monotonically increasing request id counter. Starts at 1.
    next_id: u64,
    /// Handlers for server-initiated notifications received while waiting.
    notification_handlers: HashMap<String, ClientNotificationHandler<V>>,
}

impl<V: Clone + Send + Serialize + DeserializeOwned + 'static> RpcClient<V> {
    // -----------------------------------------------------------------------
    // Constructor
    // -----------------------------------------------------------------------

    /// Create a new `RpcClient`.
    ///
    /// The id counter starts at 1. Handlers can be registered before the first
    /// call to `request()` or `notify()`.
    pub fn new(codec: Box<dyn RpcCodec<V>>, framer: Box<dyn RpcFramer>) -> Self {
        Self {
            codec,
            framer,
            next_id: 1,
            notification_handlers: HashMap::new(),
        }
    }

    // -----------------------------------------------------------------------
    // on_notification — register a server-push notification handler
    // -----------------------------------------------------------------------

    /// Register a handler for server-initiated notifications.
    ///
    /// While the client is blocked in `request()` waiting for a response, the
    /// server may push notifications (e.g., `"textDocument/publishDiagnostics"`
    /// in LSP). This handler is called for those notifications. The client then
    /// continues waiting for the response.
    ///
    /// Returns `&mut Self` for method chaining.
    pub fn on_notification<F>(&mut self, method: &str, handler: F) -> &mut Self
    where
        F: Fn(Option<V>) + Send + 'static,
    {
        self.notification_handlers
            .insert(method.to_string(), Box::new(handler));
        self
    }

    // -----------------------------------------------------------------------
    // request — send a request and wait for the matching response
    // -----------------------------------------------------------------------

    /// Send a request to the server and wait (blocking) for the response.
    ///
    /// 1. Generates the next request id (`next_id++`).
    /// 2. Encodes `RpcRequest { id, method, params }` and writes the frame.
    /// 3. Reads frames in a loop until a response with the matching id arrives.
    ///    - Server-push notifications are handled via registered handlers.
    ///    - Responses for other ids are ignored (not expected in the sync model).
    ///    - EOF → returns `Err(RpcErrorResponse { code: INTERNAL_ERROR, ... })`.
    ///
    /// # Errors
    ///
    /// Returns `Err(RpcErrorResponse<V>)` when:
    /// - The server sent an error response.
    /// - The connection was closed before a response arrived.
    /// - The codec failed to encode the request.
    pub fn request(
        &mut self,
        method: &str,
        params: Option<V>,
    ) -> Result<V, RpcErrorResponse<V>> {
        // Step 1: Generate the next id.
        let id_num = self.next_id;
        self.next_id += 1;
        let id = serde_json::Value::Number(id_num.into());

        // Step 2: Encode and send the request.
        let req_msg = RpcMessage::Request(RpcRequest {
            id: id.clone(),
            method: method.to_string(),
            params,
        });
        let bytes = self.codec.encode(&req_msg).map_err(|e| RpcErrorResponse {
            id: Some(id.clone()),
            code: crate::errors::INTERNAL_ERROR,
            message: format!("encode error: {}", e),
            data: None,
        })?;
        self.framer.write_frame(&bytes).map_err(|e| RpcErrorResponse {
            id: Some(id.clone()),
            code: crate::errors::INTERNAL_ERROR,
            message: format!("write error: {}", e),
            data: None,
        })?;

        // Step 3: Read frames until we get a response with our id.
        loop {
            let frame = match self.framer.read_frame() {
                None => {
                    // Connection closed — we cannot get a response.
                    return Err(RpcErrorResponse {
                        id: Some(id),
                        code: crate::errors::INTERNAL_ERROR,
                        message: "connection closed before response".to_string(),
                        data: None,
                    });
                }
                Some(Ok(bytes)) => bytes,
                Some(Err(e)) => {
                    // Framing error while waiting — propagate as internal error.
                    return Err(RpcErrorResponse {
                        id: Some(id),
                        code: crate::errors::PARSE_ERROR,
                        message: format!("framing error: {}", e),
                        data: None,
                    });
                }
            };

            let msg = match self.codec.decode(&frame) {
                Ok(msg) => msg,
                Err(e) => {
                    // Decode error — not our response, but malformed bytes.
                    // Treat as internal error and return.
                    return Err(RpcErrorResponse {
                        id: Some(id),
                        code: e.code,
                        message: e.message,
                        data: e.data,
                    });
                }
            };

            match msg {
                RpcMessage::Response(resp) if resp.id == id => {
                    return Ok(resp.result);
                }
                RpcMessage::ErrorResponse(err) if err.id.as_ref() == Some(&id) => {
                    return Err(err);
                }
                RpcMessage::Notification(notif) => {
                    // Server-push notification received while we wait.
                    // Dispatch to the registered handler (if any) and continue
                    // waiting for our response.
                    self.dispatch_notification(notif);
                }
                _ => {
                    // Response for a different id, or unexpected message type.
                    // In the single-threaded model this shouldn't happen, but
                    // we skip it gracefully rather than panicking.
                    continue;
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // notify — send a fire-and-forget notification
    // -----------------------------------------------------------------------

    /// Send a notification to the server.
    ///
    /// Unlike `request()`, this method does not wait for any reply. The server
    /// must not send a response to a notification.
    ///
    /// # Errors
    ///
    /// Returns `Err(RpcError)` if encoding or writing fails.
    pub fn notify(&mut self, method: &str, params: Option<V>) -> Result<(), RpcError> {
        let notif_msg = RpcMessage::Notification(RpcNotification {
            method: method.to_string(),
            params,
        });
        let bytes = self.codec.encode(&notif_msg)?;
        self.framer.write_frame(&bytes)
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    /// Dispatch a server-push notification to the registered handler (if any).
    fn dispatch_notification(&self, notif: RpcNotification<V>) {
        if let Some(handler) = self.notification_handlers.get(&notif.method) {
            // Catch panics — a crashing notification handler should not
            // interrupt the client's wait for a response.
            let params = notif.params;
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                handler(params);
            }));
        }
        // Unknown notifications silently dropped.
    }
}
