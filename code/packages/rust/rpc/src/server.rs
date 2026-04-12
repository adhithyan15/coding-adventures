//! `RpcServer` — codec-agnostic request dispatch loop.
//!
//! ## Architecture
//!
//! The server owns:
//! - A **codec** (`Box<dyn RpcCodec<V>>`): translates bytes ↔ `RpcMessage<V>`.
//! - A **framer** (`Box<dyn RpcFramer>`): splits the stream into byte chunks.
//! - Two **dispatch tables**: one for request methods, one for notifications.
//!
//! The `serve()` method drives the loop:
//!
//! ```text
//! loop:
//!   bytes = framer.read_frame()
//!   if bytes == None: break                 // clean EOF — normal shutdown
//!
//!   msg = codec.decode(bytes)
//!   if msg is Err(error_resp):
//!     framer.write_frame(codec.encode(error_resp))
//!     continue                              // recoverable — keep going
//!
//!   match msg:
//!     Request(req):
//!       handler = dispatch_table[req.method]
//!       if no handler: send METHOD_NOT_FOUND
//!       else: result = catch_panic(handler); send result
//!
//!     Notification(notif):
//!       handler = dispatch_table[notif.method]
//!       if handler: catch_panic(handler)    // unknown notifs silently dropped
//!       // Never send a response to a notification
//!
//!     Response | ErrorResponse:
//!       ignored                             // servers don't handle incoming responses
//! ```
//!
//! ## Panic Safety
//!
//! Handler panics are caught with `std::panic::catch_unwind`. A single bad
//! request cannot kill the server process. The recovered panic sends an
//! `INTERNAL_ERROR` response to the caller so they know something went wrong.
//!
//! ## Usage
//!
//! ```rust,no_run
//! use coding_adventures_rpc::server::RpcServer;
//! use coding_adventures_rpc::message::RpcErrorResponse;
//! use serde_json::Value;
//!
//! // Assume `codec` and `framer` are concrete implementations:
//! // let mut server = RpcServer::new(Box::new(codec), Box::new(framer));
//! //
//! // server.on_request("ping", |_id, _params| {
//! //     Ok(Value::String("pong".into()))
//! // });
//! // server.on_notification("log", |params| {
//! //     eprintln!("log: {:?}", params);
//! // });
//! // server.serve();
//! ```

use crate::codec::RpcCodec;
use crate::errors::{RpcError, INTERNAL_ERROR, METHOD_NOT_FOUND};
use crate::framer::RpcFramer;
use crate::message::{RpcErrorResponse, RpcId, RpcMessage, RpcResponse};
use std::collections::HashMap;
use std::io::{Read, Write};

// ---------------------------------------------------------------------------
// Handler types
// ---------------------------------------------------------------------------
//
// We use `Box<dyn Fn(...)>` so the server can own closures of any concrete
// type. `Fn` (not `FnMut`) because handlers may be called multiple times —
// once per matching message.
//
// `Send + 'static` bounds are required because the handler boxes are stored
// in the server and the server may (in principle) be moved across threads.

/// A handler for `RpcRequest` messages.
///
/// Receives `(id, params)` and returns either `Ok(result)` (sent as a success
/// response) or `Err(RpcErrorResponse)` (sent as an error response).
pub type RequestHandler<V> =
    Box<dyn Fn(RpcId, Option<V>) -> Result<V, RpcErrorResponse<V>> + Send + 'static>;

/// A handler for `RpcNotification` messages.
///
/// Receives `params`. Its return value is ignored. Notifications never
/// generate a response, even if the handler returns an error.
pub type NotificationHandler<V> = Box<dyn Fn(Option<V>) + Send + 'static>;

// ---------------------------------------------------------------------------
// RpcServer struct
// ---------------------------------------------------------------------------

/// Codec-agnostic RPC server.
///
/// Owns a codec, a framer, and handler tables. Call [`RpcServer::serve`] to
/// start the blocking read-dispatch-write loop.
///
/// # Type parameters
///
/// - `R`: the `Read` type of the underlying transport (unused structurally, kept
///   for future extension and to match the spec API).
/// - `W`: the `Write` type (same).
/// - `V`: the codec's native value type. Must be `Clone + Send + 'static`.
///   `Clone` is needed because handlers may need to inspect params and the
///   server needs to clone the id for the response.
pub struct RpcServer<R, W, V>
where
    V: Clone + Send + 'static,
{
    /// The codec translates between `RpcMessage<V>` and raw bytes.
    codec: Box<dyn RpcCodec<V>>,
    /// The framer reads/writes discrete byte chunks from/to the stream.
    framer: Box<dyn RpcFramer>,
    /// Dispatch table for request methods.
    request_handlers: HashMap<String, RequestHandler<V>>,
    /// Dispatch table for notification methods.
    notification_handlers: HashMap<String, NotificationHandler<V>>,
    // PhantomData is needed so the R and W type parameters are "used".
    // We store the framer as a trait object, so R and W are not directly
    // held. We keep them in the API for consistency with the spec.
    _phantom: std::marker::PhantomData<(R, W)>,
}

impl<R: Read, W: Write, V: Clone + Send + 'static> RpcServer<R, W, V> {
    // -----------------------------------------------------------------------
    // Constructor
    // -----------------------------------------------------------------------

    /// Create a new `RpcServer`.
    ///
    /// The codec and framer are passed as boxed trait objects so the server
    /// is generic over the concrete implementations.
    ///
    /// ```rust,no_run
    /// use coding_adventures_rpc::server::RpcServer;
    /// use coding_adventures_rpc::codec::RpcCodec;
    /// use coding_adventures_rpc::framer::RpcFramer;
    /// use serde_json::Value;
    ///
    /// // let server: RpcServer<_, _, Value> = RpcServer::new(
    /// //     Box::new(my_codec),
    /// //     Box::new(my_framer),
    /// // );
    /// ```
    pub fn new(codec: Box<dyn RpcCodec<V>>, framer: Box<dyn RpcFramer>) -> Self {
        Self {
            codec,
            framer,
            request_handlers: HashMap::new(),
            notification_handlers: HashMap::new(),
            _phantom: std::marker::PhantomData,
        }
    }

    // -----------------------------------------------------------------------
    // on_request — register a request handler
    // -----------------------------------------------------------------------

    /// Register a handler for a named request method.
    ///
    /// The handler is called whenever the server receives a request with this
    /// method name. It receives `(id, params)` and should return:
    /// - `Ok(V)` — the success result, sent back as an `RpcResponse`.
    /// - `Err(RpcErrorResponse<V>)` — an application error, sent back as an
    ///   error response.
    ///
    /// Registering the same method twice replaces the earlier handler.
    ///
    /// Returns `&mut Self` so calls can be chained:
    ///
    /// ```rust,no_run
    /// # use coding_adventures_rpc::server::RpcServer;
    /// # use serde_json::Value;
    /// # fn example(server: &mut RpcServer<std::io::Cursor<Vec<u8>>, std::io::Cursor<Vec<u8>>, Value>) {
    /// server
    ///     .on_request("ping", |_id, _params| Ok(Value::String("pong".into())))
    ///     .on_request("echo", |_id, params| Ok(params.unwrap_or(Value::Null)));
    /// # }
    /// ```
    pub fn on_request<F>(&mut self, method: &str, handler: F) -> &mut Self
    where
        F: Fn(RpcId, Option<V>) -> Result<V, RpcErrorResponse<V>> + Send + 'static,
    {
        self.request_handlers
            .insert(method.to_string(), Box::new(handler));
        self
    }

    // -----------------------------------------------------------------------
    // on_notification — register a notification handler
    // -----------------------------------------------------------------------

    /// Register a handler for a named notification method.
    ///
    /// The handler is called whenever the server receives a notification with
    /// this method name. It receives `params`. Its return value is discarded.
    ///
    /// Unknown notifications (no registered handler) are silently dropped —
    /// the spec forbids sending error responses to notifications.
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
    // serve — blocking dispatch loop
    // -----------------------------------------------------------------------

    /// Start the blocking read-dispatch-write loop.
    ///
    /// Reads messages from the framer, dispatches to handlers, and writes
    /// responses. Runs until:
    /// - The framer returns `None` (clean EOF / connection closed).
    /// - An unrecoverable I/O error occurs while writing (silently discarded —
    ///   there is nothing useful we can do if the write stream is broken).
    ///
    /// Recoverable errors (decode failures, handler errors, panics) generate
    /// error responses and the loop continues.
    pub fn serve(&mut self) {
        loop {
            // Step 1: Read the next frame from the framer.
            let frame = match self.framer.read_frame() {
                None => {
                    // Clean EOF — remote end closed the connection. Normal
                    // shutdown. Break out of the loop.
                    break;
                }
                Some(Ok(bytes)) => bytes,
                Some(Err(e)) => {
                    // The framer could not produce a valid frame (e.g. malformed
                    // Content-Length header). Send a PARSE_ERROR response with
                    // null id and continue — the stream may still be usable.
                    self.send_error(None, crate::errors::PARSE_ERROR, e.message().to_string(), None);
                    continue;
                }
            };

            // Step 2: Decode the frame bytes into an RpcMessage.
            let msg = match self.codec.decode(&frame) {
                Ok(msg) => msg,
                Err(err_resp) => {
                    // The codec failed — bytes are not a valid message.
                    // The codec pre-built the error response for us.
                    let _ = self.write_error_response(err_resp);
                    continue;
                }
            };

            // Step 3: Dispatch based on message type.
            match msg {
                RpcMessage::Request(req) => {
                    self.handle_request(req);
                }
                RpcMessage::Notification(notif) => {
                    self.handle_notification(notif);
                }
                RpcMessage::Response(_) | RpcMessage::ErrorResponse(_) => {
                    // Servers that only respond ignore incoming responses.
                    // In a bidirectional peer these would be routed to a
                    // pending-request table.
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Private: request dispatch
    // -----------------------------------------------------------------------

    fn handle_request(&mut self, req: crate::message::RpcRequest<V>) {
        let id = req.id.clone();

        let result = match self.request_handlers.get(&req.method) {
            None => {
                // No handler registered → METHOD_NOT_FOUND.
                Err(RpcErrorResponse {
                    id: Some(id.clone()),
                    code: METHOD_NOT_FOUND,
                    message: "Method not found".to_string(),
                    data: None,
                })
            }
            Some(handler) => {
                // Call the handler inside catch_unwind so a panicking handler
                // cannot kill the server process. We use AssertUnwindSafe
                // because Box<dyn Fn> does not implement UnwindSafe, but we
                // accept the risk: if the handler left shared state corrupted,
                // the next request will surface the corruption.
                let params = req.params.clone();
                let handler_id = req.id.clone();

                let catch_result =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        handler(handler_id, params)
                    }));

                match catch_result {
                    Ok(handler_result) => handler_result.map_err(|mut e| {
                        // Ensure the error response carries the request id.
                        e.id = Some(id.clone());
                        e
                    }),
                    Err(_panic_payload) => {
                        // Handler panicked — INTERNAL_ERROR.
                        Err(RpcErrorResponse {
                            id: Some(id.clone()),
                            code: INTERNAL_ERROR,
                            message: "Internal error".to_string(),
                            data: None,
                        })
                    }
                }
            }
        };

        match result {
            Ok(value) => {
                let resp = RpcMessage::Response(RpcResponse {
                    id,
                    result: value,
                });
                // Ignore encode/write errors — we cannot send an error
                // response if writing itself is broken.
                let _ = self.codec.encode(&resp).and_then(|bytes| {
                    self.framer.write_frame(&bytes)
                });
            }
            Err(err_resp) => {
                let _ = self.write_error_response(err_resp);
            }
        }
    }

    // -----------------------------------------------------------------------
    // Private: notification dispatch
    // -----------------------------------------------------------------------

    fn handle_notification(&mut self, notif: crate::message::RpcNotification<V>) {
        if let Some(handler) = self.notification_handlers.get(&notif.method) {
            // Catch panics — a crashing notification handler should not
            // kill the server. Per spec, we still never send a response.
            let params = notif.params.clone();
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                handler(params);
            }));
        }
        // Unknown notifications are silently dropped — the spec forbids
        // sending error responses to notifications.
    }

    // -----------------------------------------------------------------------
    // Private: helpers
    // -----------------------------------------------------------------------

    /// Build and write an `RpcErrorResponse` via the codec + framer.
    fn write_error_response(&mut self, err_resp: RpcErrorResponse<V>) -> Result<(), RpcError> {
        let msg = RpcMessage::ErrorResponse(err_resp);
        let bytes = self.codec.encode(&msg)?;
        self.framer.write_frame(&bytes)
    }

    /// Construct and send an error response from raw components.
    ///
    /// Used for infrastructure errors (framing failures) where we do not
    /// have a pre-built `RpcErrorResponse`.
    fn send_error(&mut self, id: Option<RpcId>, code: i64, message: String, data: Option<V>) {
        let err_resp = RpcErrorResponse { id, code, message, data };
        let _ = self.write_error_response(err_resp);
    }
}
