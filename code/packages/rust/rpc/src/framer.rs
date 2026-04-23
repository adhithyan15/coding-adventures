//! The `RpcFramer` trait — splits a raw byte stream into discrete frames.
//!
//! ## What Is a Framer?
//!
//! A raw byte stream is just a sequence of bytes with no inherent message
//! boundaries. The framer's job is to impose those boundaries. Think of the
//! stream as a long piece of ticker tape and the framer as scissors: it knows
//! exactly where to cut.
//!
//! Different protocols use different "cut rules":
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │  Framing scheme        │ How boundaries are marked              │
//! ├─────────────────────────────────────────────────────────────────┤
//! │  Content-Length        │ Header: "Content-Length: N\r\n\r\n"   │
//! │  Length-prefix (4B)    │ First 4 bytes = big-endian uint32      │
//! │  Newline-delimited     │ '\n' after each message                │
//! │  WebSocket frames      │ WebSocket framing protocol             │
//! │  Passthrough           │ Entire stream is one message           │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Relationship to Codec and Transport
//!
//! ```text
//! ┌─────────────────────────────────┐
//! │  RpcServer / RpcClient          │
//! │    ↕  RpcMessage<V>             │
//! ├─────────────────────────────────┤
//! │  RpcCodec   (serialisation)     │
//! │    ↕  Vec<u8>  (payload bytes)  │
//! ├─────────────────────────────────┤
//! │  RpcFramer  (this module)       │  ← knows how to cut the stream
//! │    ↕  raw byte stream           │
//! └─────────────────────────────────┘
//! ```
//!
//! The framer owns the transport (the underlying `Read`/`Write` stream).
//! This is intentional: the framer is the layer that interacts with I/O,
//! so it naturally holds the connection.
//!
//! ## Ownership of State
//!
//! Unlike the codec, a framer *is* stateful — it buffers bytes between calls
//! to `read_frame`. This is why `read_frame` takes `&mut self`.
//!
//! ## EOF Handling
//!
//! `read_frame` returns `None` on clean EOF — the remote end closed the
//! connection gracefully. The server's `serve()` loop treats `None` as a
//! normal shutdown signal.
//!
//! ## Example: NewlineFramer sketch
//!
//! ```rust,ignore
//! use coding_adventures_rpc::framer::RpcFramer;
//! use coding_adventures_rpc::errors::RpcError;
//! use std::io::{BufRead, Write};
//!
//! struct NewlineFramer<R: BufRead, W: Write> {
//!     reader: R,
//!     writer: W,
//! }
//!
//! impl<R: BufRead, W: Write> RpcFramer for NewlineFramer<R, W> {
//!     fn read_frame(&mut self) -> Option<Result<Vec<u8>, RpcError>> {
//!         let mut line = String::new();
//!         match self.reader.read_line(&mut line) {
//!             Ok(0) => None,   // EOF
//!             Ok(_) => Some(Ok(line.trim_end_matches('\n').as_bytes().to_vec())),
//!             Err(e) => Some(Err(RpcError::new(e.to_string()))),
//!         }
//!     }
//!     fn write_frame(&mut self, data: &[u8]) -> Result<(), RpcError> {
//!         self.writer.write_all(data).map_err(|e| RpcError::new(e.to_string()))?;
//!         self.writer.write_all(b"\n").map_err(|e| RpcError::new(e.to_string()))
//!     }
//! }
//! ```

use crate::errors::RpcError;

/// Split a raw byte stream into discrete payload frames.
///
/// The framer knows about frame boundaries; it does not know what the bytes
/// mean. Interpreting the payload bytes is the [`crate::codec::RpcCodec`]'s
/// responsibility.
///
/// Implementations must hold the underlying transport (the `Read`/`Write`
/// stream) and any per-connection buffering state.
///
/// # `read_frame` return values
///
/// | Return value          | Meaning                                      |
/// |-----------------------|----------------------------------------------|
/// | `None`                | Clean EOF — remote end closed the connection |
/// | `Some(Ok(bytes))`     | One complete payload frame                   |
/// | `Some(Err(e))`        | Framing error (malformed header, I/O error)  |
///
/// The `None` / `Some` distinction is important: `None` means "done, stop the
/// loop"; `Some(Err(...))` means "something went wrong but the connection may
/// still be alive — send an error response and continue".
pub trait RpcFramer {
    /// Read the next frame from the underlying stream.
    ///
    /// Blocks until a complete frame is available, EOF is reached, or an
    /// error occurs.
    ///
    /// Returns:
    /// - `None` — clean EOF, the remote end closed the connection.
    /// - `Some(Ok(bytes))` — one complete payload (no framing envelope).
    /// - `Some(Err(e))` — a framing error occurred.
    fn read_frame(&mut self) -> Option<Result<Vec<u8>, RpcError>>;

    /// Write a payload frame to the underlying stream.
    ///
    /// The implementation adds whatever framing envelope is appropriate
    /// (e.g., `Content-Length: N\r\n\r\n`) before writing `data`.
    ///
    /// # Errors
    ///
    /// Returns `Err(RpcError)` if writing fails (I/O error, broken pipe, etc.).
    fn write_frame(&mut self, data: &[u8]) -> Result<(), RpcError>;
}
