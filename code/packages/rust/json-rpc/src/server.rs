//! `Server` — reads JSON-RPC messages from a stream, dispatches to handlers,
//! and writes responses.
//!
//! ## Architecture
//!
//! The server owns a `MessageReader` and a `MessageWriter`. The `serve` method
//! runs a blocking loop:
//!
//! 1. Read one message.
//! 2. Dispatch based on type:
//!    - `Request` → call the registered handler, write a `Response`.
//!    - `Notification` → call the registered handler (no response written).
//!    - `Response` → ignored (servers that only respond don't need this).
//!    - Parse/framing error → write an error response with `id: null`.
//! 3. Repeat until EOF.
//!
//! ## Handler Types
//!
//! Request handlers:
//! ```text
//! Box<dyn Fn(Value, Option<Value>) -> Result<Value, ResponseError>>
//! ```
//! - First arg: the request `id` (as a `Value`).
//! - Second arg: `params` (optional).
//! - Return `Ok(value)` → sent as `result` in the response.
//! - Return `Err(e)` → sent as `error` in the response.
//!
//! Notification handlers:
//! ```text
//! Box<dyn Fn(Option<Value>)>
//! ```
//! - Receives `params`. Return value is ignored.
//!
//! ## Thread Safety
//!
//! `serve` is single-threaded. The handlers are called sequentially — one at a
//! time. This matches the LSP protocol's single-request-at-a-time model.
//!
//! ## Usage
//!
//! ```rust,no_run
//! use coding_adventures_json_rpc::Server;
//! use std::io::{BufReader, BufWriter};
//!
//! let mut server = Server::new(
//!     BufReader::new(std::io::stdin()),
//!     BufWriter::new(std::io::stdout()),
//! );
//!
//! server.on_request("initialize", |_id, _params| {
//!     Ok(serde_json::json!({"capabilities": {}}))
//! });
//! server.on_notification("initialized", |_params| {
//!     eprintln!("server ready");
//! });
//!
//! server.serve();
//! ```

use crate::errors::ResponseError;
use crate::message::{Message, Notification, Request, Response};
use crate::reader::MessageReader;
use crate::writer::MessageWriter;
use serde_json::Value;
use std::collections::HashMap;
use std::io::{BufRead, Write};

// ---------------------------------------------------------------------------
// Handler types
// ---------------------------------------------------------------------------
//
// We box the closures so the Server can own them regardless of their concrete
// type. `Fn` (not `FnMut`) because handlers may be called multiple times.

/// A handler for `Request` messages.
///
/// Receives `(id, params)` and returns either a result value or a `ResponseError`.
pub type RequestHandler =
    Box<dyn Fn(Value, Option<Value>) -> Result<Value, ResponseError> + Send>;

/// A handler for `Notification` messages.
///
/// Receives `params`. Return value is ignored.
pub type NotificationHandler = Box<dyn Fn(Option<Value>) + Send>;

// ---------------------------------------------------------------------------
// Server struct
// ---------------------------------------------------------------------------

/// JSON-RPC 2.0 server with method dispatch.
pub struct Server<R: BufRead, W: Write> {
    reader: MessageReader<R>,
    writer: MessageWriter<W>,
    request_handlers: HashMap<String, RequestHandler>,
    notification_handlers: HashMap<String, NotificationHandler>,
}

impl<R: BufRead, W: Write> Server<R, W> {
    // -----------------------------------------------------------------------
    // Constructor
    // -----------------------------------------------------------------------

    /// Create a new `Server` that reads from `reader` and writes to `writer`.
    ///
    /// ```rust,no_run
    /// use coding_adventures_json_rpc::Server;
    /// use std::io::{BufReader, BufWriter};
    ///
    /// let server = Server::new(
    ///     BufReader::new(std::io::stdin()),
    ///     BufWriter::new(std::io::stdout()),
    /// );
    /// ```
    pub fn new(reader: R, writer: W) -> Self {
        Self {
            reader: MessageReader::new(reader),
            writer: MessageWriter::new(writer),
            request_handlers: HashMap::new(),
            notification_handlers: HashMap::new(),
        }
    }

    // -----------------------------------------------------------------------
    // on_request — register a request handler
    // -----------------------------------------------------------------------

    /// Register a handler for a named request method.
    ///
    /// The handler receives `(id, params)` and must return `Ok(value)` for
    /// success or `Err(ResponseError)` for failure.
    ///
    /// Calling `on_request` with the same method name twice replaces the
    /// previous handler.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use coding_adventures_json_rpc::Server;
    /// use std::io::{BufReader, BufWriter, Cursor};
    ///
    /// let mut server = Server::new(BufReader::new(Cursor::new(b""[..])), BufWriter::new(Cursor::new(vec![])));
    /// server.on_request("ping", |_id, _params| {
    ///     Ok(serde_json::json!("pong"))
    /// });
    /// ```
    pub fn on_request<F>(&mut self, method: &str, handler: F)
    where
        F: Fn(Value, Option<Value>) -> Result<Value, ResponseError> + Send + 'static,
    {
        self.request_handlers
            .insert(method.to_string(), Box::new(handler));
    }

    // -----------------------------------------------------------------------
    // on_notification — register a notification handler
    // -----------------------------------------------------------------------

    /// Register a handler for a named notification method.
    ///
    /// The handler receives `params` (an `Option<Value>`). Its return value
    /// is ignored. Unknown notifications are silently dropped per the spec.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use coding_adventures_json_rpc::Server;
    /// use std::io::{BufReader, BufWriter, Cursor};
    ///
    /// let mut server = Server::new(BufReader::new(Cursor::new(b""[..])), BufWriter::new(Cursor::new(vec![])));
    /// server.on_notification("initialized", |_params| {
    ///     eprintln!("client initialised");
    /// });
    /// ```
    pub fn on_notification<F>(&mut self, method: &str, handler: F)
    where
        F: Fn(Option<Value>) + Send + 'static,
    {
        self.notification_handlers
            .insert(method.to_string(), Box::new(handler));
    }

    // -----------------------------------------------------------------------
    // serve — blocking dispatch loop
    // -----------------------------------------------------------------------

    /// Start the read-dispatch-write loop. Blocks until EOF or I/O error.
    ///
    /// For each message:
    /// - `Request` → dispatch to handler, write response.
    /// - `Notification` → dispatch to handler, no response.
    /// - `Response` → ignored.
    /// - Framing/parse error → write error response with null id.
    pub fn serve(&mut self) {
        loop {
            match self.reader.read_message() {
                None => {
                    // EOF — clean shutdown.
                    break;
                }

                Some(Ok(Message::Request(req))) => {
                    self.handle_request(req);
                }

                Some(Ok(Message::Notification(notif))) => {
                    self.handle_notification(notif);
                }

                Some(Ok(Message::Response(_))) => {
                    // Responses are for client-side use. Servers that only
                    // respond ignore incoming responses.
                }

                Some(Err(e)) => {
                    // Framing or parse error — send an error response with
                    // null id (we don't know the request id).
                    let response = Response {
                        id: Value::Null,
                        result: None,
                        error: Some(e),
                    };
                    let _ = self.writer.write_message(&Message::Response(response));
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Private: request dispatch
    // -----------------------------------------------------------------------

    fn handle_request(&mut self, req: Request) {
        let id = req.id.clone();

        let result = match self.request_handlers.get(&req.method) {
            None => {
                // No handler registered — return Method not found.
                Err(ResponseError::method_not_found(&req.method))
            }

            Some(handler) => {
                // Call the handler. We use std::panic::catch_unwind to survive
                // panicking handlers — a single bad request must not kill the
                // server process.
                //
                // Note: catch_unwind requires the closure to be UnwindSafe.
                // We use AssertUnwindSafe because the handler Box<dyn Fn>
                // does not implement UnwindSafe automatically, but we own
                // the call and can tolerate any state inconsistency.
                let params = req.params.clone();
                let handler_id = req.id.clone();
                let catch_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    handler(handler_id, params)
                }));

                match catch_result {
                    Ok(handler_result) => handler_result,
                    Err(_panic_payload) => {
                        // Handler panicked — send Internal error.
                        Err(ResponseError::internal_error(Some(Value::String(
                            "handler panicked".to_string(),
                        ))))
                    }
                }
            }
        };

        let response = match result {
            Ok(value) => Response {
                id,
                result: Some(value),
                error: None,
            },
            Err(e) => Response {
                id,
                result: None,
                error: Some(e),
            },
        };

        let _ = self.writer.write_message(&Message::Response(response));
    }

    // -----------------------------------------------------------------------
    // Private: notification dispatch
    // -----------------------------------------------------------------------

    fn handle_notification(&mut self, notif: Notification) {
        if let Some(handler) = self.notification_handlers.get(&notif.method) {
            // Call the handler. Ignore the return value. Catch panics so the
            // server stays alive even if a notification handler crashes.
            let params = notif.params.clone();
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                handler(params);
            }));
        }
        // Unknown notifications are silently dropped per the JSON-RPC spec.
        // Notifications must not generate error responses.
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{parse_message, Message, Notification, Request, Response};
    use std::io::{BufReader, BufWriter, Cursor};
    use std::sync::{Arc, Mutex};

    // Helper: build a Content-Length-framed message.
    fn frame(json: &str) -> Vec<u8> {
        let n = json.len();
        format!("Content-Length: {}\r\n\r\n{}", n, json).into_bytes()
    }

    // Helper: run the server against framed input bytes and return the output.
    fn run(input: Vec<u8>, setup: impl FnOnce(&mut Server<BufReader<Cursor<Vec<u8>>>, BufWriter<Cursor<Vec<u8>>>>)) -> Vec<u8> {
        let reader = BufReader::new(Cursor::new(input));
        let writer = BufWriter::new(Cursor::new(Vec::new()));
        let mut server = Server::new(reader, writer);
        setup(&mut server);
        server.serve();
        // Extract the written bytes from the BufWriter's inner Cursor.
        // We need to access the inner writer. We use into_inner on the writer field.
        // But we need the writer out of the server... let's extract via a helper.
        extract_output(server)
    }

    fn extract_output<R: BufRead>(server: Server<R, BufWriter<Cursor<Vec<u8>>>>) -> Vec<u8> {
        // MessageWriter::into_inner() → BufWriter<Cursor<Vec<u8>>>
        // BufWriter::into_inner() → Result<Cursor<Vec<u8>>, ...> (unwrap: flush already done)
        // Cursor::into_inner() → Vec<u8>
        server
            .writer
            .into_inner()
            .into_inner()
            .expect("BufWriter flush")
            .into_inner()
    }

    // Parse all framed messages from a byte slice.
    fn parse_all(bytes: &[u8]) -> Vec<Message> {
        let mut result = Vec::new();
        let mut rest = bytes;

        loop {
            // Find the header/payload separator.
            if let Some(sep_pos) = find_header_end(rest) {
                let header = &rest[..sep_pos];

                // Extract Content-Length.
                let header_str = std::str::from_utf8(header).unwrap();
                let cl_line = header_str.lines().find(|l| l.starts_with("Content-Length:")).unwrap();
                let n: usize = cl_line.trim_start_matches("Content-Length:").trim().parse().unwrap();

                let payload = &rest[sep_pos + 4..sep_pos + 4 + n];
                result.push(parse_message(payload).unwrap());
                rest = &rest[sep_pos + 4 + n..];

                if rest.is_empty() {
                    break;
                }
            } else {
                break;
            }
        }

        result
    }

    // Find the position of \r\n\r\n in a byte slice.
    fn find_header_end(bytes: &[u8]) -> Option<usize> {
        bytes.windows(4).position(|w| w == b"\r\n\r\n")
    }

    // -----------------------------------------------------------------------
    // Server tests (more detailed tests in the top-level tests module below)
    // -----------------------------------------------------------------------

    #[test]
    fn test_server_known_request_dispatched() {
        let input = frame(r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#);
        let output = run(input, |server| {
            server.on_request("ping", |_id, _params| {
                Ok(serde_json::json!("pong"))
            });
        });

        let messages = parse_all(&output);
        assert_eq!(messages.len(), 1);
        match &messages[0] {
            Message::Response(r) => {
                assert_eq!(r.id, serde_json::json!(1));
                assert_eq!(r.result, Some(serde_json::json!("pong")));
                assert!(r.error.is_none());
            }
            _ => panic!("expected Response"),
        }
    }

    #[test]
    fn test_server_unknown_request_method_not_found() {
        let input = frame(r#"{"jsonrpc":"2.0","id":2,"method":"unknown"}"#);
        let output = run(input, |_| {});

        let messages = parse_all(&output);
        assert_eq!(messages.len(), 1);
        match &messages[0] {
            Message::Response(r) => {
                assert_eq!(r.id, serde_json::json!(2));
                let err = r.error.as_ref().unwrap();
                assert_eq!(err.code, crate::errors::METHOD_NOT_FOUND);
            }
            _ => panic!("expected Response"),
        }
    }

    #[test]
    fn test_server_notification_no_response() {
        let notified = Arc::new(Mutex::new(false));
        let notified_clone = Arc::clone(&notified);

        let input = frame(r#"{"jsonrpc":"2.0","method":"ping"}"#);
        let output = run(input, move |server| {
            let flag = Arc::clone(&notified_clone);
            server.on_notification("ping", move |_params| {
                *flag.lock().unwrap() = true;
            });
        });

        // No response should be written for a notification.
        assert!(output.is_empty());
        assert!(*notified.lock().unwrap());
    }

    #[test]
    fn test_server_unknown_notification_silent() {
        let input = frame(r#"{"jsonrpc":"2.0","method":"unknown"}"#);
        let output = run(input, |_| {});
        assert!(output.is_empty());
    }

    #[test]
    fn test_server_handler_error_returned() {
        let input = frame(r#"{"jsonrpc":"2.0","id":3,"method":"fail"}"#);
        let output = run(input, |server| {
            server.on_request("fail", |_id, _params| {
                Err(ResponseError::invalid_params(Some(serde_json::json!("bad"))))
            });
        });

        let messages = parse_all(&output);
        assert_eq!(messages.len(), 1);
        match &messages[0] {
            Message::Response(r) => {
                let err = r.error.as_ref().unwrap();
                assert_eq!(err.code, crate::errors::INVALID_PARAMS);
            }
            _ => panic!("expected error Response"),
        }
    }
}
