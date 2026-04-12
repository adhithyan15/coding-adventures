//! # `coding-adventures-rpc` — Codec-agnostic RPC primitive
//!
//! This crate defines the abstract RPC layer that sits below protocol-specific
//! packages like `json-rpc` and `msgpack-rpc`. It captures the *semantics* of
//! remote procedure calls — method dispatch, id correlation, error codes, panic
//! recovery, handler registration — without coupling to any particular
//! serialisation format or framing scheme.
//!
//! ## The Three-Layer Model
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │  Application  (your LSP server, CLI tool, test client, …)           │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │  RpcServer / RpcClient          (this crate)                        │
//! │  — method dispatch, id correlation, error handling, panic recovery  │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │  RpcCodec  (implement the trait in a downstream crate)              │
//! │  — serialises/deserialises RpcMessage<V> ↔ bytes                   │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │  RpcFramer  (implement the trait in a downstream crate)             │
//! │  — splits the raw byte stream into discrete message frames          │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │  Transport  (raw byte stream — stdin/stdout, TCP, Unix socket, …)   │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! Each layer knows only about the layer immediately below it:
//! - The application registers handlers on `RpcServer` and calls methods on
//!   `RpcClient`. It never touches bytes.
//! - `RpcServer`/`RpcClient` call `RpcCodec::encode`/`decode` and
//!   `RpcFramer::read_frame`/`write_frame`. They never touch the wire format.
//! - `RpcCodec` receives/produces payload bytes. It never knows about framing.
//! - `RpcFramer` reads/writes raw bytes from/to the transport. It never knows
//!   about message content.
//!
//! ## Quick Example
//!
//! ```rust,no_run
//! use coding_adventures_rpc::server::RpcServer;
//! use coding_adventures_rpc::message::RpcErrorResponse;
//! use serde_json::Value;
//!
//! // Bring your own codec and framer (e.g. from coding-adventures-json-rpc):
//! // let mut server = RpcServer::new(
//! //     Box::new(JsonCodec::new()),
//! //     Box::new(ContentLengthFramer::new(stdin, stdout)),
//! // );
//! //
//! // server.on_request("ping", |_id, _params| {
//! //     Ok(Value::String("pong".into()))
//! // });
//! // server.serve();
//! ```
//!
//! ## Module Map
//!
//! | Module    | Contents                                                   |
//! |-----------|------------------------------------------------------------|
//! | `errors`  | `RpcError` + standard error code constants                 |
//! | `message` | `RpcMessage`, `RpcRequest`, `RpcResponse`, `RpcNotification`, `RpcId` |
//! | `codec`   | `RpcCodec` trait                                           |
//! | `framer`  | `RpcFramer` trait                                          |
//! | `server`  | `RpcServer` — dispatch loop                                |
//! | `client`  | `RpcClient` — blocking synchronous client                  |

pub mod client;
pub mod codec;
pub mod errors;
pub mod framer;
pub mod message;
pub mod server;

// Re-export the most commonly used items at the crate root for ergonomics.
pub use errors::{RpcError, INTERNAL_ERROR, INVALID_PARAMS, INVALID_REQUEST, METHOD_NOT_FOUND, PARSE_ERROR};
pub use message::{RpcErrorResponse, RpcId, RpcMessage, RpcNotification, RpcRequest, RpcResponse};
pub use codec::RpcCodec;
pub use framer::RpcFramer;
pub use server::RpcServer;
pub use client::RpcClient;
