//! `MessageReader` — reads Content-Length-framed JSON-RPC messages from a
//! `BufRead` byte stream.
//!
//! ## Reading Algorithm
//!
//! Each message is preceded by an HTTP-style header block:
//!
//! ```text
//! Content-Length: 47\r\n
//! \r\n
//! {"jsonrpc":"2.0","id":1,"method":"initialize"}
//! ```
//!
//! The reader:
//! 1. Reads lines (via `BufRead::read_line`) until it sees a blank line.
//! 2. Extracts the `Content-Length` value from the headers.
//! 3. Reads exactly that many bytes for the payload using `Read::read_exact`.
//! 4. Parses the payload as a JSON-RPC message.
//!
//! ## Why `BufRead`?
//!
//! `BufRead::read_line` reads one line at a time without copying the entire
//! stream into memory. For stdin it is backed by an OS read buffer; for tests
//! it is backed by `std::io::Cursor`. The same `MessageReader` code works for
//! both with no changes — that's the power of trait-based generics.
//!
//! ## EOF Handling
//!
//! When `read_line` returns `Ok(0)`, the stream has ended cleanly. We return
//! `None` to signal EOF to the caller. The server loop treats `None` as a
//! normal shutdown.
//!
//! ## Usage
//!
//! ```rust,no_run
//! use coding_adventures_json_rpc::MessageReader;
//! use std::io::BufReader;
//!
//! let reader = MessageReader::new(BufReader::new(std::io::stdin()));
//! while let Some(result) = reader.read_message() {
//!     match result {
//!         Ok(msg) => println!("got: {:?}", msg),
//!         Err(e)  => eprintln!("error: {}", e),
//!     }
//! }
//! ```

use crate::errors::ResponseError;
use crate::message::{parse_message, Message};
use serde_json::Value;
use std::io::{BufRead, Read};

/// Reads Content-Length-framed JSON-RPC messages from a `BufRead` source.
///
/// The generic parameter `R` must implement both `BufRead` (for line reading)
/// and `Read` (for exact-byte-count payload reading). `BufReader<R>` satisfies
/// both.
pub struct MessageReader<R: BufRead> {
    /// The underlying buffered reader.
    ///
    /// We use `std::cell::RefCell` so that `read_message` can take `&self`
    /// instead of `&mut self` — this makes the API easier to use in the server
    /// loop where the reader is stored in a struct alongside other state.
    reader: std::cell::RefCell<R>,
}

impl<R: BufRead> MessageReader<R> {
    // -----------------------------------------------------------------------
    // Constructor
    // -----------------------------------------------------------------------

    /// Create a new `MessageReader` wrapping the given buffered reader.
    ///
    /// ```rust
    /// use coding_adventures_json_rpc::MessageReader;
    /// use std::io::{BufReader, Cursor};
    ///
    /// let data = b"Content-Length: 3\r\n\r\n{}\n".to_vec();
    /// let reader = MessageReader::new(BufReader::new(Cursor::new(data)));
    /// ```
    pub fn new(reader: R) -> Self {
        Self {
            reader: std::cell::RefCell::new(reader),
        }
    }

    // -----------------------------------------------------------------------
    // read_message — read and parse the next message
    // -----------------------------------------------------------------------

    /// Read and parse the next framed message.
    ///
    /// Returns:
    /// - `None` — EOF, the stream ended cleanly.
    /// - `Some(Ok(message))` — a successfully parsed message.
    /// - `Some(Err(e))` — a framing or parse error.
    ///
    /// # Example
    ///
    /// ```rust
    /// use coding_adventures_json_rpc::{MessageReader, message::Message};
    /// use std::io::{BufReader, Cursor};
    ///
    /// let json = r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#;
    /// let framed = format!("Content-Length: {}\r\n\r\n{}", json.len(), json);
    /// let reader = MessageReader::new(BufReader::new(Cursor::new(framed)));
    ///
    /// let msg = reader.read_message().unwrap().unwrap();
    /// assert!(matches!(msg, Message::Request(_)));
    /// ```
    pub fn read_message(&self) -> Option<Result<Message, ResponseError>> {
        match self.read_raw() {
            None => None,
            Some(Ok(bytes)) => Some(parse_message(&bytes)),
            Some(Err(e)) => Some(Err(e)),
        }
    }

    // -----------------------------------------------------------------------
    // read_raw — read the next payload without parsing
    // -----------------------------------------------------------------------

    /// Read the next raw JSON payload bytes without parsing them into a message.
    ///
    /// Useful for testing, logging, or when the caller wants to control parsing.
    ///
    /// Returns `None` on EOF, `Some(Ok(bytes))` on success, or `Some(Err(e))`
    /// on framing error.
    pub fn read_raw(&self) -> Option<Result<Vec<u8>, ResponseError>> {
        let mut reader = self.reader.borrow_mut();

        // Step 1: Read header lines until the blank line.
        let content_length = match read_headers(&mut *reader) {
            None => return None, // EOF before headers
            Some(Ok(n)) => n,
            Some(Err(e)) => return Some(Err(e)),
        };

        // Step 2: Read exactly content_length bytes for the payload.
        let mut payload = vec![0u8; content_length];
        match reader.read_exact(&mut payload) {
            Ok(()) => Some(Ok(payload)),
            Err(e) => Some(Err(ResponseError::parse_error(Some(Value::String(
                format!("I/O error reading payload: {}", e),
            ))))),
        }
    }
}

// ---------------------------------------------------------------------------
// Private: header reading
// ---------------------------------------------------------------------------
//
// Reads lines from the buffered reader until it encounters a blank line
// (`\r\n` or just `\n`). Extracts the `Content-Length` value.
//
// Returns:
//   None               — EOF before any header line (clean shutdown)
//   Some(Ok(n))        — content length n
//   Some(Err(e))       — framing error

fn read_headers<R: BufRead>(reader: &mut R) -> Option<Result<usize, ResponseError>> {
    let mut content_length: Option<usize> = None;
    let mut first_line = true;

    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Err(e) => {
                return Some(Err(ResponseError::parse_error(Some(Value::String(
                    format!("I/O error reading header: {}", e),
                )))))
            }
            Ok(0) => {
                // EOF.
                if first_line {
                    return None; // Clean EOF before any message — normal shutdown.
                }
                return Some(Err(ResponseError::parse_error(Some(Value::String(
                    "EOF in header block".to_string(),
                )))));
            }
            Ok(_) => {}
        }

        first_line = false;

        // Strip the CRLF or LF terminator.
        let stripped = line.trim_end_matches('\n').trim_end_matches('\r');

        if stripped.is_empty() {
            // Blank line — end of header block.
            match content_length {
                Some(n) => return Some(Ok(n)),
                None => {
                    return Some(Err(ResponseError::parse_error(Some(Value::String(
                        "missing Content-Length header".to_string(),
                    )))))
                }
            }
        }

        // Parse Content-Length header (case-sensitive, per LSP spec).
        // Other headers (Content-Type etc.) are ignored.
        if let Some(value_str) = stripped.strip_prefix("Content-Length: ") {
            match value_str.trim().parse::<usize>() {
                Ok(n) => content_length = Some(n),
                Err(_) => {
                    return Some(Err(ResponseError::parse_error(Some(Value::String(
                        format!("invalid Content-Length value: '{}'", value_str),
                    )))))
                }
            }
        }
        // else: some other header — ignore and continue.
    }
}
