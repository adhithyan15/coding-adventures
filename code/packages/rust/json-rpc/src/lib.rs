//! # JSON-RPC 2.0 — transport layer for LSP servers
//!
//! This crate implements [JSON-RPC 2.0](https://www.jsonrpc.org/specification)
//! over byte streams (stdin/stdout) using Content-Length framing. It is the
//! foundation that every Language Server Protocol (LSP) server in this repo
//! is built on.
//!
//! ## Why JSON-RPC before LSP?
//!
//! The LSP spec sits on top of JSON-RPC. The LSP server for Brainfuck (and
//! every future language) delegates all message framing, dispatch, and error
//! handling to this layer. Implementing JSON-RPC first:
//!
//! 1. Keeps the LSP layer thin — it only knows about LSP-specific methods.
//! 2. Lets us test the transport independently of any language server logic.
//! 3. Gives us a reusable library for any future RPC-based protocol (DAP).
//!
//! ## Content-Length Framing
//!
//! Messages are framed like HTTP headers:
//!
//! ```text
//! Content-Length: 47\r\n
//! \r\n
//! {"jsonrpc":"2.0","id":1,"method":"initialize"}
//! ```
//!
//! The receiver reads the header, learns the exact byte count, and reads
//! exactly that many bytes — no heuristics, no scanning for delimiters.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use coding_adventures_json_rpc::{Server, errors};
//! use std::io::{BufReader, BufWriter};
//!
//! let stdin = BufReader::new(std::io::stdin());
//! let stdout = BufWriter::new(std::io::stdout());
//!
//! let mut server = Server::new(stdin, stdout);
//! server.on_request("initialize", |_id, _params| {
//!     Ok(serde_json::json!({"capabilities": {"hoverProvider": true}}))
//! });
//! server.on_notification("textDocument/didOpen", |params| {
//!     eprintln!("document opened: {:?}", params);
//! });
//! server.serve();
//! ```
//!
//! ## Module Map
//!
//! | Module    | Role                                        |
//! |-----------|---------------------------------------------|
//! | `message` | Message enum, structs, parse + serialize    |
//! | `errors`  | Standard error codes + ResponseError type  |
//! | `reader`  | `MessageReader<R>` — reads framed messages  |
//! | `writer`  | `MessageWriter<W>` — writes framed messages |
//! | `server`  | `Server` — dispatch loop                    |

pub mod errors;
pub mod message;
pub mod reader;
pub mod server;
pub mod writer;

// Re-export the most commonly used types at the crate root for ergonomic use.
pub use errors::ResponseError;
pub use message::{Message, Notification, Request, Response};
pub use reader::MessageReader;
pub use server::Server;
pub use writer::MessageWriter;
