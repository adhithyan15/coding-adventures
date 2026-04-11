//! `MessageWriter` — writes Content-Length-framed JSON-RPC messages to a
//! `Write` byte stream.
//!
//! ## Writing Algorithm
//!
//! For each message:
//! 1. Serialise the `Message` to a JSON byte vector (`serde_json::to_vec`).
//! 2. Compute `payload.len()` — the byte length (NOT the character count).
//!    UTF-8 characters can be 1–4 bytes; `len()` gives the correct value.
//! 3. Write `"Content-Length: {n}\r\n\r\n"` followed by the payload bytes.
//! 4. Flush the writer so the bytes reach the OS buffer immediately.
//!
//! ## Why Flush?
//!
//! If the writer is a `BufWriter<Stdout>`, bytes are accumulated in a
//! user-space buffer until the buffer fills or the writer is explicitly
//! flushed. Without flushing after each message, the client may wait
//! indefinitely for the response. We flush after every write.
//!
//! ## Usage
//!
//! ```rust,no_run
//! use coding_adventures_json_rpc::{MessageWriter, message::{Message, Response}};
//! use std::io::{BufWriter, stdout};
//!
//! let mut writer = MessageWriter::new(BufWriter::new(stdout()));
//! let response = Response {
//!     id: serde_json::json!(1),
//!     result: Some(serde_json::json!({"ok": true})),
//!     error: None,
//! };
//! writer.write_message(&Message::Response(response)).unwrap();
//! ```

use crate::message::{message_to_value, Message};
use std::io::{self, Write};

/// Writes Content-Length-framed JSON-RPC messages to a `Write` destination.
pub struct MessageWriter<W: Write> {
    writer: W,
}

impl<W: Write> MessageWriter<W> {
    // -----------------------------------------------------------------------
    // Constructor
    // -----------------------------------------------------------------------

    /// Create a new `MessageWriter` wrapping the given writer.
    ///
    /// ```rust
    /// use coding_adventures_json_rpc::MessageWriter;
    /// use std::io::Cursor;
    ///
    /// let writer = MessageWriter::new(Cursor::new(Vec::new()));
    /// ```
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    // -----------------------------------------------------------------------
    // write_message — serialise and write a typed message
    // -----------------------------------------------------------------------

    /// Serialise a `Message` and write it with Content-Length framing.
    ///
    /// Flushes the underlying writer after writing so the bytes reach the
    /// client immediately.
    ///
    /// # Errors
    ///
    /// Returns `Err(io::Error)` if serialisation or writing fails.
    ///
    /// # Example
    ///
    /// ```rust
    /// use coding_adventures_json_rpc::{MessageWriter, message::{Message, Notification}};
    /// use std::io::Cursor;
    ///
    /// let mut out = Cursor::new(Vec::new());
    /// let mut writer = MessageWriter::new(&mut out);
    /// let notif = Notification { method: "initialized".to_string(), params: None };
    /// writer.write_message(&Message::Notification(notif)).unwrap();
    ///
    /// let bytes = out.into_inner();
    /// let text = std::str::from_utf8(&bytes).unwrap();
    /// assert!(text.starts_with("Content-Length: "));
    /// ```
    pub fn write_message(&mut self, message: &Message) -> io::Result<()> {
        // Convert the typed message to a serde_json Value, then serialise it.
        let value = message_to_value(message);
        let payload = serde_json::to_vec(&value)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        self.write_raw(&payload)
    }

    // -----------------------------------------------------------------------
    // write_raw — write a raw byte payload with framing
    // -----------------------------------------------------------------------

    /// Write a raw byte payload with Content-Length framing.
    ///
    /// Useful when the caller has already serialised the payload or wants to
    /// write a custom JSON body.
    ///
    /// ```rust
    /// use coding_adventures_json_rpc::MessageWriter;
    /// use std::io::Cursor;
    ///
    /// let mut out = Cursor::new(Vec::new());
    /// let mut writer = MessageWriter::new(&mut out);
    /// let json = br#"{"jsonrpc":"2.0","method":"ping"}"#;
    /// writer.write_raw(json).unwrap();
    ///
    /// let bytes = out.into_inner();
    /// assert!(bytes.starts_with(b"Content-Length: "));
    /// ```
    pub fn write_raw(&mut self, payload: &[u8]) -> io::Result<()> {
        // byte_size, not character count — crucial for UTF-8 payloads with
        // multi-byte characters (e.g. file paths with Unicode).
        let n = payload.len();

        // Write the header: "Content-Length: N\r\n\r\n"
        write!(self.writer, "Content-Length: {}\r\n\r\n", n)?;

        // Write the payload bytes.
        self.writer.write_all(payload)?;

        // Flush so the bytes are immediately visible to the reader.
        self.writer.flush()
    }

    // -----------------------------------------------------------------------
    // into_inner — extract the underlying writer
    // -----------------------------------------------------------------------

    /// Consume the `MessageWriter` and return the underlying writer.
    ///
    /// Useful in tests to inspect the written bytes.
    pub fn into_inner(self) -> W {
        self.writer
    }
}
