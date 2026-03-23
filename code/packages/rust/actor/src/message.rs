//! # Message -- The Atom of Actor Communication
//!
//! A Message is the fundamental unit of data exchange in the Actor model.
//! Every piece of information that flows between actors -- a user request,
//! an agent response, a credential from the vault -- is a Message.
//!
//! ## Immutability
//!
//! Messages are **immutable**: once created, they cannot be modified. All fields
//! are set at construction time. To "modify" a message, you create a new one
//! with different values. The original is untouched.
//!
//! **Analogy:** A Message is a sealed letter. Once sealed, the contents are fixed.
//! The envelope records who sent it, when, and what kind of letter it is. You can
//! make copies of the letter, but you cannot change the original.
//!
//! ## Wire Format
//!
//! Messages serialize to a binary format that separates the **envelope** (metadata
//! as JSON) from the **payload** (raw bytes). This avoids Base64-encoding binary
//! data like images or videos, which would bloat size by 33%.
//!
//! ```text
//! +-------------------------------------------+
//! | HEADER (17 bytes, fixed)                   |
//! | magic:          4 bytes  "ACTM"            |
//! | version:        1 byte   0x01              |
//! | envelope_length: 4 bytes (big-endian u32)  |
//! | payload_length:  8 bytes (big-endian u64)  |
//! +-------------------------------------------+
//! | ENVELOPE (UTF-8 JSON, variable length)     |
//! | {"id":"...","timestamp":...,...}            |
//! +-------------------------------------------+
//! | PAYLOAD (raw bytes, variable length)       |
//! | Could be text, JSON, PNG, MP4, anything    |
//! +-------------------------------------------+
//! ```

use std::collections::HashMap;
use std::io::Read;
use std::sync::atomic::{AtomicU64, Ordering};

// ---------------------------------------------------------------------------
// Global counters
// ---------------------------------------------------------------------------

/// Global monotonic counter for generating unique message IDs.
///
/// Each message gets a unique ID by combining "msg_" with an atomically
/// incremented counter. This guarantees uniqueness within a single process
/// without needing UUIDs or external libraries.
static MSG_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Global monotonic counter for timestamps.
///
/// Timestamps are logical clock values, not wall-clock time. Each new message
/// gets a strictly increasing timestamp, which provides a total ordering of
/// messages created within a single process.
static TIMESTAMP_COUNTER: AtomicU64 = AtomicU64::new(1);

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Magic bytes at the start of every serialized message.
///
/// "ACTM" stands for "Actor Message". These 4 bytes let a reader quickly
/// verify that a byte sequence is a valid Actor message before attempting
/// to parse the rest. If the magic bytes don't match, the data is corrupt
/// or not an Actor message.
pub const WIRE_MAGIC: &[u8; 4] = b"ACTM";

/// Current wire format version.
///
/// Version 1 uses JSON for the envelope. Future versions might use Protobuf,
/// MessagePack, or other formats. The version byte tells the reader how to
/// parse the envelope. See the spec for version compatibility rules.
pub const WIRE_VERSION: u8 = 1;

/// Size of the fixed header in bytes: 4 (magic) + 1 (version) + 4 (envelope_length) + 8 (payload_length) = 17.
pub const HEADER_SIZE: usize = 17;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors that can occur when working with Actor messages.
///
/// ```text
/// +-------------------+--------------------------------------------------+
/// | Variant           | When it happens                                  |
/// +-------------------+--------------------------------------------------+
/// | InvalidFormat     | Magic bytes don't match "ACTM", or the data is  |
/// |                   | too short to contain a valid header.             |
/// +-------------------+--------------------------------------------------+
/// | VersionTooNew     | The message was written by a newer version of    |
/// |                   | the software. Upgrade to read it.                |
/// +-------------------+--------------------------------------------------+
/// | InvalidEnvelope   | The JSON envelope is malformed or missing        |
/// |                   | required fields.                                 |
/// +-------------------+--------------------------------------------------+
/// | Io                | An I/O error occurred while reading from a       |
/// |                   | stream (file or network socket).                 |
/// +-------------------+--------------------------------------------------+
/// | Eof               | Reached end of stream before reading a complete  |
/// |                   | message. This is normal at end of file.          |
/// +-------------------+--------------------------------------------------+
/// ```
#[derive(Debug)]
pub enum ActorError {
    /// The data does not start with the "ACTM" magic bytes, or is too short.
    InvalidFormat(String),
    /// The message version is higher than what this code supports.
    VersionTooNew {
        found: u8,
        max_supported: u8,
    },
    /// The JSON envelope could not be parsed or is missing required fields.
    InvalidEnvelope(String),
    /// An I/O error occurred during stream reading.
    Io(std::io::Error),
    /// End of stream reached -- no more messages to read.
    Eof,
}

impl std::fmt::Display for ActorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ActorError::InvalidFormat(msg) => write!(f, "invalid format: {}", msg),
            ActorError::VersionTooNew { found, max_supported } => {
                write!(
                    f,
                    "version {} is too new (max supported: {})",
                    found, max_supported
                )
            }
            ActorError::InvalidEnvelope(msg) => write!(f, "invalid envelope: {}", msg),
            ActorError::Io(err) => write!(f, "I/O error: {}", err),
            ActorError::Eof => write!(f, "end of stream"),
        }
    }
}

impl PartialEq for ActorError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ActorError::InvalidFormat(a), ActorError::InvalidFormat(b)) => a == b,
            (
                ActorError::VersionTooNew {
                    found: f1,
                    max_supported: m1,
                },
                ActorError::VersionTooNew {
                    found: f2,
                    max_supported: m2,
                },
            ) => f1 == f2 && m1 == m2,
            (ActorError::InvalidEnvelope(a), ActorError::InvalidEnvelope(b)) => a == b,
            (ActorError::Eof, ActorError::Eof) => true,
            _ => false,
        }
    }
}

impl From<std::io::Error> for ActorError {
    fn from(err: std::io::Error) -> Self {
        if err.kind() == std::io::ErrorKind::UnexpectedEof {
            ActorError::Eof
        } else {
            ActorError::Io(err)
        }
    }
}

// ---------------------------------------------------------------------------
// Message struct
// ---------------------------------------------------------------------------

/// An immutable message -- the atom of actor communication.
///
/// ## Fields
///
/// - `id` -- Unique identifier, auto-generated at creation time.
/// - `timestamp` -- Monotonic logical clock value, strictly increasing.
/// - `sender_id` -- The actor that created this message.
/// - `content_type` -- MIME-like string describing the payload format.
/// - `payload` -- The message body as raw bytes.
/// - `metadata` -- Optional key-value pairs for extensibility.
///
/// ## Why All Fields Are `pub`
///
/// Rust's ownership model already prevents mutation through shared references.
/// Making fields `pub` allows convenient read access without getter methods.
/// There are no `&mut self` methods that modify fields, so the struct is
/// effectively immutable once constructed.
#[derive(Debug, Clone)]
pub struct Message {
    pub id: String,
    pub timestamp: u64,
    pub sender_id: String,
    pub content_type: String,
    pub payload: Vec<u8>,
    pub metadata: HashMap<String, String>,
}

impl Message {
    // -----------------------------------------------------------------------
    // Constructors
    // -----------------------------------------------------------------------

    /// Create a new message with explicit content_type and raw byte payload.
    ///
    /// This is the most general constructor. The convenience methods `text()`,
    /// `json()`, and `binary()` delegate to this after setting the appropriate
    /// content_type and encoding the payload.
    ///
    /// # Arguments
    ///
    /// * `sender_id` -- The actor sending this message
    /// * `content_type` -- MIME type describing the payload (e.g. "text/plain")
    /// * `payload` -- Raw bytes of the message body
    /// * `metadata` -- Optional key-value pairs
    ///
    /// # Example
    ///
    /// ```
    /// use actor::Message;
    /// let msg = Message::new("agent", "text/plain", b"hello".to_vec(), None);
    /// assert_eq!(msg.sender_id, "agent");
    /// assert_eq!(msg.payload, b"hello");
    /// ```
    pub fn new(
        sender_id: &str,
        content_type: &str,
        payload: Vec<u8>,
        metadata: Option<HashMap<String, String>>,
    ) -> Self {
        let counter = MSG_COUNTER.fetch_add(1, Ordering::SeqCst);
        let ts = TIMESTAMP_COUNTER.fetch_add(1, Ordering::SeqCst);
        Message {
            id: format!("msg_{:016x}", counter),
            timestamp: ts,
            sender_id: sender_id.to_string(),
            content_type: content_type.to_string(),
            payload,
            metadata: metadata.unwrap_or_default(),
        }
    }

    /// Create a text message with content_type "text/plain".
    ///
    /// The text string is encoded as UTF-8 bytes for the payload. This is the
    /// most common message type for human-readable communication between actors.
    ///
    /// # Example
    ///
    /// ```
    /// use actor::Message;
    /// let msg = Message::text("agent", "hello world");
    /// assert_eq!(msg.content_type, "text/plain");
    /// assert_eq!(msg.payload_text(), "hello world");
    /// ```
    pub fn text(sender_id: &str, payload: &str) -> Self {
        Message::new(sender_id, "text/plain", payload.as_bytes().to_vec(), None)
    }

    /// Create a text message with metadata.
    pub fn text_with_metadata(
        sender_id: &str,
        payload: &str,
        metadata: HashMap<String, String>,
    ) -> Self {
        Message::new(
            sender_id,
            "text/plain",
            payload.as_bytes().to_vec(),
            Some(metadata),
        )
    }

    /// Create a JSON message with content_type "application/json".
    ///
    /// The JSON string is stored as UTF-8 bytes. Unlike the Python spec which
    /// accepts a dict/list and serializes it, the Rust version accepts a
    /// pre-formatted JSON string since we have no serde dependency.
    ///
    /// # Example
    ///
    /// ```
    /// use actor::Message;
    /// let msg = Message::json("agent", r#"{"key":"value"}"#);
    /// assert_eq!(msg.content_type, "application/json");
    /// assert_eq!(msg.payload_text(), r#"{"key":"value"}"#);
    /// ```
    pub fn json(sender_id: &str, payload: &str) -> Self {
        Message::new(
            sender_id,
            "application/json",
            payload.as_bytes().to_vec(),
            None,
        )
    }

    /// Create a binary message with a custom content_type.
    ///
    /// For images, videos, and arbitrary binary data. The payload is stored
    /// as-is with no encoding or transformation.
    ///
    /// # Example
    ///
    /// ```
    /// use actor::Message;
    /// let png_header = vec![0x89, 0x50, 0x4E, 0x47];
    /// let msg = Message::binary("browser", "image/png", png_header);
    /// assert_eq!(msg.content_type, "image/png");
    /// ```
    pub fn binary(sender_id: &str, content_type: &str, payload: Vec<u8>) -> Self {
        Message::new(sender_id, content_type, payload, None)
    }

    // -----------------------------------------------------------------------
    // Convenience accessors
    // -----------------------------------------------------------------------

    /// Interpret the payload as a UTF-8 string.
    ///
    /// This is a convenience method for text and JSON messages. If the payload
    /// is not valid UTF-8, this returns a lossy conversion (replacing invalid
    /// sequences with the Unicode replacement character).
    pub fn payload_text(&self) -> String {
        String::from_utf8_lossy(&self.payload).to_string()
    }

    /// Return the payload as a raw JSON string.
    ///
    /// This is identical to `payload_text()` but makes intent clearer when
    /// working with JSON messages. The caller is responsible for parsing
    /// the JSON string.
    pub fn payload_json(&self) -> String {
        self.payload_text()
    }

    // -----------------------------------------------------------------------
    // Serialization -- envelope to JSON
    // -----------------------------------------------------------------------

    /// Serialize the envelope (all fields except payload) to a JSON string.
    ///
    /// The envelope contains the message metadata: id, timestamp, sender_id,
    /// content_type, and metadata. The payload is NOT included -- it's stored
    /// as raw bytes separately. This allows indexing and searching messages
    /// without loading potentially large payloads.
    ///
    /// ## JSON structure
    ///
    /// ```json
    /// {
    ///   "id": "msg_0000000000000001",
    ///   "timestamp": 1,
    ///   "sender_id": "agent",
    ///   "content_type": "text/plain",
    ///   "metadata": {"key": "value"}
    /// }
    /// ```
    ///
    /// We implement JSON serialization manually because this crate has zero
    /// external dependencies. The envelope schema is fixed and simple, so
    /// hand-written serialization is straightforward and avoids pulling in
    /// serde + serde_json (which would add ~50KB to the binary).
    pub fn envelope_to_json(&self) -> String {
        let mut json = String::with_capacity(256);
        json.push('{');

        // "id":"..."
        json.push_str("\"id\":\"");
        json_escape_into(&mut json, &self.id);
        json.push_str("\",");

        // "timestamp":N
        json.push_str("\"timestamp\":");
        json.push_str(&self.timestamp.to_string());
        json.push(',');

        // "sender_id":"..."
        json.push_str("\"sender_id\":\"");
        json_escape_into(&mut json, &self.sender_id);
        json.push_str("\",");

        // "content_type":"..."
        json.push_str("\"content_type\":\"");
        json_escape_into(&mut json, &self.content_type);
        json.push('"');

        // "metadata":{...} -- only if non-empty
        if !self.metadata.is_empty() {
            json.push_str(",\"metadata\":{");
            let mut first = true;
            // Sort keys for deterministic output (important for tests)
            let mut keys: Vec<&String> = self.metadata.keys().collect();
            keys.sort();
            for key in keys {
                let value = &self.metadata[key];
                if !first {
                    json.push(',');
                }
                first = false;
                json.push('"');
                json_escape_into(&mut json, key);
                json.push_str("\":\"");
                json_escape_into(&mut json, value);
                json.push('"');
            }
            json.push('}');
        }

        json.push('}');
        json
    }

    // -----------------------------------------------------------------------
    // Serialization -- full wire format
    // -----------------------------------------------------------------------

    /// Serialize the message to the binary wire format.
    ///
    /// The wire format consists of three parts:
    ///
    /// 1. **Header** (17 bytes, fixed):
    ///    - Magic "ACTM" (4 bytes)
    ///    - Version (1 byte)
    ///    - Envelope length (4 bytes, big-endian u32)
    ///    - Payload length (8 bytes, big-endian u64)
    ///
    /// 2. **Envelope** (variable): JSON-encoded metadata
    ///
    /// 3. **Payload** (variable): raw bytes
    ///
    /// This format avoids Base64-encoding binary payloads. A 10MB image is
    /// 10MB on the wire, not 13.3MB.
    pub fn to_bytes(&self) -> Vec<u8> {
        let envelope = self.envelope_to_json();
        let envelope_bytes = envelope.as_bytes();
        let envelope_len = envelope_bytes.len() as u32;
        let payload_len = self.payload.len() as u64;

        let total_size = HEADER_SIZE + envelope_bytes.len() + self.payload.len();
        let mut buf = Vec::with_capacity(total_size);

        // Header: magic + version + envelope_length + payload_length
        buf.extend_from_slice(WIRE_MAGIC);
        buf.push(WIRE_VERSION);
        buf.extend_from_slice(&envelope_len.to_be_bytes());
        buf.extend_from_slice(&payload_len.to_be_bytes());

        // Envelope: JSON bytes
        buf.extend_from_slice(envelope_bytes);

        // Payload: raw bytes
        buf.extend_from_slice(&self.payload);

        buf
    }

    /// Deserialize a message from the binary wire format.
    ///
    /// This is the inverse of `to_bytes()`. It validates the magic bytes and
    /// version, then parses the JSON envelope and extracts the raw payload.
    ///
    /// # Errors
    ///
    /// - `InvalidFormat` if the magic bytes don't match "ACTM"
    /// - `VersionTooNew` if the version byte exceeds `WIRE_VERSION`
    /// - `InvalidEnvelope` if the JSON envelope is malformed
    pub fn from_bytes(data: &[u8]) -> Result<Self, ActorError> {
        if data.len() < HEADER_SIZE {
            return Err(ActorError::InvalidFormat(
                "data too short for header".to_string(),
            ));
        }

        // Validate magic bytes
        if &data[0..4] != WIRE_MAGIC {
            return Err(ActorError::InvalidFormat(format!(
                "expected magic {:?}, got {:?}",
                WIRE_MAGIC,
                &data[0..4]
            )));
        }

        // Validate version
        let version = data[4];
        if version > WIRE_VERSION {
            return Err(ActorError::VersionTooNew {
                found: version,
                max_supported: WIRE_VERSION,
            });
        }

        // Parse lengths from header
        let envelope_len =
            u32::from_be_bytes([data[5], data[6], data[7], data[8]]) as usize;
        let payload_len =
            u64::from_be_bytes([
                data[9], data[10], data[11], data[12],
                data[13], data[14], data[15], data[16],
            ]) as usize;

        let expected_total = HEADER_SIZE + envelope_len + payload_len;
        if data.len() < expected_total {
            return Err(ActorError::InvalidFormat(format!(
                "data too short: expected {} bytes, got {}",
                expected_total,
                data.len()
            )));
        }

        // Parse envelope JSON
        let envelope_bytes = &data[HEADER_SIZE..HEADER_SIZE + envelope_len];
        let envelope_str = std::str::from_utf8(envelope_bytes)
            .map_err(|e| ActorError::InvalidEnvelope(format!("invalid UTF-8: {}", e)))?;

        // Extract payload
        let payload_start = HEADER_SIZE + envelope_len;
        let payload = data[payload_start..payload_start + payload_len].to_vec();

        // Parse the envelope JSON into message fields
        parse_envelope(envelope_str, payload)
    }

    /// Read exactly one message from a byte stream.
    ///
    /// This reads the 17-byte header first to determine the envelope and payload
    /// lengths, then reads exactly those many bytes. The stream is left positioned
    /// at the start of the next message (or at EOF).
    ///
    /// Uses `std::io::Read` so it works with files, network sockets, or any
    /// other byte source.
    ///
    /// # Errors
    ///
    /// - `Eof` if the stream has no more data
    /// - `InvalidFormat` if the magic bytes don't match
    /// - `VersionTooNew` if the version is unsupported
    /// - `Io` for underlying I/O errors
    pub fn from_reader<R: Read>(reader: &mut R) -> Result<Self, ActorError> {
        // Read the 17-byte header
        let mut header = [0u8; HEADER_SIZE];
        match reader.read_exact(&mut header) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                return Err(ActorError::Eof);
            }
            Err(e) => return Err(ActorError::Io(e)),
        }

        // Validate magic
        if &header[0..4] != WIRE_MAGIC {
            return Err(ActorError::InvalidFormat(format!(
                "expected magic {:?}, got {:?}",
                WIRE_MAGIC,
                &header[0..4]
            )));
        }

        // Validate version
        let version = header[4];
        if version > WIRE_VERSION {
            return Err(ActorError::VersionTooNew {
                found: version,
                max_supported: WIRE_VERSION,
            });
        }

        // Parse lengths
        let envelope_len =
            u32::from_be_bytes([header[5], header[6], header[7], header[8]]) as usize;
        let payload_len = u64::from_be_bytes([
            header[9], header[10], header[11], header[12],
            header[13], header[14], header[15], header[16],
        ]) as usize;

        // Read envelope
        let mut envelope_buf = vec![0u8; envelope_len];
        reader.read_exact(&mut envelope_buf)?;
        let envelope_str = std::str::from_utf8(&envelope_buf)
            .map_err(|e| ActorError::InvalidEnvelope(format!("invalid UTF-8: {}", e)))?;

        // Read payload
        let mut payload = vec![0u8; payload_len];
        reader.read_exact(&mut payload)?;

        parse_envelope(envelope_str, payload)
    }

    /// Reconstruct a message from its individual fields.
    ///
    /// This is used internally by deserialization. It bypasses the auto-generated
    /// id and timestamp, using the values from the serialized data instead.
    fn from_fields(
        id: String,
        timestamp: u64,
        sender_id: String,
        content_type: String,
        payload: Vec<u8>,
        metadata: HashMap<String, String>,
    ) -> Self {
        Message {
            id,
            timestamp,
            sender_id,
            content_type,
            payload,
            metadata,
        }
    }
}

// ---------------------------------------------------------------------------
// Minimal JSON helpers -- no serde dependency
// ---------------------------------------------------------------------------

/// Escape a string for JSON output.
///
/// JSON requires certain characters to be escaped with backslashes:
/// - `"` becomes `\"`
/// - `\` becomes `\\`
/// - Control characters (U+0000 through U+001F) become `\uXXXX`
///
/// We handle the most common cases: quote, backslash, newline, tab, carriage return.
fn json_escape_into(buf: &mut String, s: &str) {
    for ch in s.chars() {
        match ch {
            '"' => buf.push_str("\\\""),
            '\\' => buf.push_str("\\\\"),
            '\n' => buf.push_str("\\n"),
            '\r' => buf.push_str("\\r"),
            '\t' => buf.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                buf.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => buf.push(c),
        }
    }
}

/// Parse a JSON envelope string into a Message.
///
/// The envelope has a fixed schema with these fields:
/// - "id": string (required)
/// - "timestamp": integer (required)
/// - "sender_id": string (required)
/// - "content_type": string (required)
/// - "metadata": object of string-to-string pairs (optional)
///
/// This is a hand-written JSON parser tailored to our specific schema.
/// It's not a general-purpose JSON parser -- it only handles the exact
/// structure we produce in `envelope_to_json()`.
fn parse_envelope(json: &str, payload: Vec<u8>) -> Result<Message, ActorError> {
    let json = json.trim();
    if !json.starts_with('{') || !json.ends_with('}') {
        return Err(ActorError::InvalidEnvelope(
            "envelope must be a JSON object".to_string(),
        ));
    }

    // Strip the outer braces
    let inner = &json[1..json.len() - 1];

    let mut id: Option<String> = None;
    let mut timestamp: Option<u64> = None;
    let mut sender_id: Option<String> = None;
    let mut content_type: Option<String> = None;
    let mut metadata: HashMap<String, String> = HashMap::new();

    // Parse top-level key-value pairs
    let mut chars = inner.chars().peekable();

    loop {
        skip_whitespace(&mut chars);
        if chars.peek().is_none() {
            break;
        }

        // Parse key
        let key = parse_json_string(&mut chars)
            .map_err(|e| ActorError::InvalidEnvelope(format!("key parse error: {}", e)))?;

        skip_whitespace(&mut chars);
        // Expect ':'
        match chars.next() {
            Some(':') => {}
            other => {
                return Err(ActorError::InvalidEnvelope(format!(
                    "expected ':', got {:?}",
                    other
                )));
            }
        }
        skip_whitespace(&mut chars);

        match key.as_str() {
            "id" => {
                id = Some(
                    parse_json_string(&mut chars)
                        .map_err(ActorError::InvalidEnvelope)?,
                );
            }
            "timestamp" => {
                timestamp = Some(
                    parse_json_number(&mut chars)
                        .map_err(ActorError::InvalidEnvelope)?,
                );
            }
            "sender_id" => {
                sender_id = Some(
                    parse_json_string(&mut chars)
                        .map_err(ActorError::InvalidEnvelope)?,
                );
            }
            "content_type" => {
                content_type = Some(
                    parse_json_string(&mut chars)
                        .map_err(ActorError::InvalidEnvelope)?,
                );
            }
            "metadata" => {
                metadata = parse_json_object(&mut chars)
                    .map_err(ActorError::InvalidEnvelope)?;
            }
            _ => {
                // Skip unknown fields (forward compatibility)
                skip_json_value(&mut chars)
                    .map_err(ActorError::InvalidEnvelope)?;
            }
        }

        skip_whitespace(&mut chars);
        // Consume optional comma
        if chars.peek() == Some(&',') {
            chars.next();
        }
    }

    let id = id.ok_or_else(|| ActorError::InvalidEnvelope("missing 'id'".to_string()))?;
    let timestamp = timestamp
        .ok_or_else(|| ActorError::InvalidEnvelope("missing 'timestamp'".to_string()))?;
    let sender_id = sender_id
        .ok_or_else(|| ActorError::InvalidEnvelope("missing 'sender_id'".to_string()))?;
    let content_type = content_type
        .ok_or_else(|| ActorError::InvalidEnvelope("missing 'content_type'".to_string()))?;

    Ok(Message::from_fields(
        id,
        timestamp,
        sender_id,
        content_type,
        payload,
        metadata,
    ))
}

/// Skip whitespace characters in a character iterator.
fn skip_whitespace(chars: &mut std::iter::Peekable<std::str::Chars>) {
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
        } else {
            break;
        }
    }
}

/// Parse a JSON string value (including the surrounding quotes).
///
/// Handles escape sequences: \", \\, \n, \r, \t, \uXXXX.
fn parse_json_string(
    chars: &mut std::iter::Peekable<std::str::Chars>,
) -> Result<String, String> {
    match chars.next() {
        Some('"') => {}
        other => return Err(format!("expected '\"', got {:?}", other)),
    }

    let mut result = String::new();
    loop {
        match chars.next() {
            Some('"') => return Ok(result),
            Some('\\') => {
                match chars.next() {
                    Some('"') => result.push('"'),
                    Some('\\') => result.push('\\'),
                    Some('n') => result.push('\n'),
                    Some('r') => result.push('\r'),
                    Some('t') => result.push('\t'),
                    Some('/') => result.push('/'),
                    Some('u') => {
                        // Parse 4 hex digits
                        let mut hex = String::with_capacity(4);
                        for _ in 0..4 {
                            match chars.next() {
                                Some(c) => hex.push(c),
                                None => return Err("unexpected end in \\u escape".to_string()),
                            }
                        }
                        let code = u32::from_str_radix(&hex, 16)
                            .map_err(|_| format!("invalid unicode escape: \\u{}", hex))?;
                        if let Some(c) = char::from_u32(code) {
                            result.push(c);
                        }
                    }
                    other => return Err(format!("unknown escape: {:?}", other)),
                }
            }
            Some(c) => result.push(c),
            None => return Err("unterminated string".to_string()),
        }
    }
}

/// Parse a JSON number (unsigned integer only -- timestamps are u64).
fn parse_json_number(
    chars: &mut std::iter::Peekable<std::str::Chars>,
) -> Result<u64, String> {
    let mut num_str = String::new();
    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() {
            num_str.push(c);
            chars.next();
        } else {
            break;
        }
    }
    if num_str.is_empty() {
        return Err("expected number".to_string());
    }
    num_str
        .parse::<u64>()
        .map_err(|e| format!("invalid number '{}': {}", num_str, e))
}

/// Parse a JSON object with string keys and string values.
///
/// This is used for the metadata field, which is always `Map<String, String>`.
fn parse_json_object(
    chars: &mut std::iter::Peekable<std::str::Chars>,
) -> Result<HashMap<String, String>, String> {
    match chars.next() {
        Some('{') => {}
        other => return Err(format!("expected '{{', got {:?}", other)),
    }

    let mut map = HashMap::new();
    skip_whitespace(chars);

    if chars.peek() == Some(&'}') {
        chars.next();
        return Ok(map);
    }

    loop {
        skip_whitespace(chars);
        let key = parse_json_string(chars)?;
        skip_whitespace(chars);
        match chars.next() {
            Some(':') => {}
            other => return Err(format!("expected ':', got {:?}", other)),
        }
        skip_whitespace(chars);
        let value = parse_json_string(chars)?;
        map.insert(key, value);

        skip_whitespace(chars);
        match chars.peek() {
            Some(&',') => {
                chars.next();
            }
            Some(&'}') => {
                chars.next();
                return Ok(map);
            }
            other => return Err(format!("expected ',' or '}}', got {:?}", other)),
        }
    }
}

/// Skip over a JSON value without parsing it.
///
/// Used for forward compatibility -- if the envelope contains fields we don't
/// recognize, we skip them rather than failing.
fn skip_json_value(
    chars: &mut std::iter::Peekable<std::str::Chars>,
) -> Result<(), String> {
    skip_whitespace(chars);
    match chars.peek() {
        Some(&'"') => {
            parse_json_string(chars)?;
            Ok(())
        }
        Some(&'{') => {
            parse_json_object(chars)?;
            Ok(())
        }
        Some(&'[') => {
            // Skip array
            chars.next();
            let mut depth = 1;
            while depth > 0 {
                match chars.next() {
                    Some('[') => depth += 1,
                    Some(']') => depth -= 1,
                    Some('"') => {
                        // Skip string contents including escaped chars
                        loop {
                            match chars.next() {
                                Some('\\') => { chars.next(); }
                                Some('"') => break,
                                None => return Err("unterminated string in array".to_string()),
                                _ => {}
                            }
                        }
                    }
                    None => return Err("unterminated array".to_string()),
                    _ => {}
                }
            }
            Ok(())
        }
        Some(c) if c.is_ascii_digit() || *c == '-' => {
            // Skip number
            while let Some(&c) = chars.peek() {
                if c.is_ascii_digit() || c == '.' || c == '-' || c == 'e' || c == 'E' || c == '+' {
                    chars.next();
                } else {
                    break;
                }
            }
            Ok(())
        }
        Some(&'t') | Some(&'f') | Some(&'n') => {
            // Skip true/false/null
            while let Some(&c) = chars.peek() {
                if c.is_ascii_alphabetic() {
                    chars.next();
                } else {
                    break;
                }
            }
            Ok(())
        }
        other => Err(format!("unexpected character: {:?}", other)),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// Test 1: Create a Message with all fields and verify properties.
    #[test]
    fn test_create_message() {
        let mut meta = HashMap::new();
        meta.insert("key".to_string(), "value".to_string());
        let msg = Message::new("agent", "text/plain", b"hello".to_vec(), Some(meta));

        assert!(msg.id.starts_with("msg_"));
        assert!(msg.timestamp > 0);
        assert_eq!(msg.sender_id, "agent");
        assert_eq!(msg.content_type, "text/plain");
        assert_eq!(msg.payload, b"hello");
        assert_eq!(msg.metadata.get("key").unwrap(), "value");
    }

    /// Test 2: Immutability -- Message has no setter methods.
    ///
    /// In Rust, immutability is enforced by the type system. If you have a
    /// `Message` (not `&mut Message`), you cannot modify its fields. This test
    /// verifies that we can read all fields without needing mutable access.
    #[test]
    fn test_immutability_via_read_access() {
        let msg = Message::text("agent", "hello");
        // All fields are readable without &mut self
        let _id = &msg.id;
        let _ts = msg.timestamp;
        let _sender = &msg.sender_id;
        let _ct = &msg.content_type;
        let _payload = &msg.payload;
        let _meta = &msg.metadata;
        // If this compiles, immutability is enforced by Rust's ownership model.
    }

    /// Test 3: Unique IDs -- 1000 messages all have distinct IDs.
    #[test]
    fn test_unique_ids() {
        let mut ids = std::collections::HashSet::new();
        for _ in 0..1000 {
            let msg = Message::text("agent", "test");
            assert!(ids.insert(msg.id.clone()), "duplicate id: {}", msg.id);
        }
        assert_eq!(ids.len(), 1000);
    }

    /// Test 4: Timestamps are strictly increasing.
    #[test]
    fn test_timestamp_ordering() {
        let msgs: Vec<Message> = (0..100)
            .map(|_| Message::text("agent", "test"))
            .collect();
        for i in 1..msgs.len() {
            assert!(
                msgs[i].timestamp > msgs[i - 1].timestamp,
                "timestamp {} should be > {}",
                msgs[i].timestamp,
                msgs[i - 1].timestamp
            );
        }
    }

    /// Test 5: Wire format round-trip for text messages.
    #[test]
    fn test_wire_roundtrip_text() {
        let msg = Message::text("agent", "hello world");
        let bytes = msg.to_bytes();
        let restored = Message::from_bytes(&bytes).unwrap();

        assert_eq!(restored.id, msg.id);
        assert_eq!(restored.timestamp, msg.timestamp);
        assert_eq!(restored.sender_id, msg.sender_id);
        assert_eq!(restored.content_type, msg.content_type);
        assert_eq!(restored.payload, msg.payload);
    }

    /// Test 6: Wire format round-trip for binary messages (PNG header).
    #[test]
    fn test_wire_roundtrip_binary() {
        let png_header = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        let msg = Message::binary("browser", "image/png", png_header.clone());
        let bytes = msg.to_bytes();
        let restored = Message::from_bytes(&bytes).unwrap();

        assert_eq!(restored.payload, png_header);
        assert_eq!(restored.content_type, "image/png");
    }

    /// Test 7: Metadata is preserved across serialization.
    #[test]
    fn test_metadata_passthrough() {
        let mut meta = HashMap::new();
        meta.insert("correlation_id".to_string(), "req_abc123".to_string());
        meta.insert("priority".to_string(), "high".to_string());
        let msg = Message::text_with_metadata("agent", "hello", meta.clone());
        let bytes = msg.to_bytes();
        let restored = Message::from_bytes(&bytes).unwrap();

        assert_eq!(restored.metadata, meta);
    }

    /// Test 8: Empty payload works correctly.
    #[test]
    fn test_empty_payload() {
        let msg = Message::new("agent", "text/plain", vec![], None);
        assert!(msg.payload.is_empty());

        let bytes = msg.to_bytes();
        let restored = Message::from_bytes(&bytes).unwrap();
        assert!(restored.payload.is_empty());
    }

    /// Test 9: Large payload (1MB) serializes correctly.
    #[test]
    fn test_large_payload() {
        let payload = vec![0xAB; 1_000_000]; // 1MB of 0xAB bytes
        let msg = Message::binary("agent", "application/octet-stream", payload.clone());
        let bytes = msg.to_bytes();
        let restored = Message::from_bytes(&bytes).unwrap();

        assert_eq!(restored.payload.len(), 1_000_000);
        assert_eq!(restored.payload, payload);
    }

    /// Test 10: content_type is preserved across serialization.
    #[test]
    fn test_content_type_preservation() {
        for ct in &[
            "text/plain",
            "application/json",
            "image/png",
            "video/mp4",
            "application/octet-stream",
        ] {
            let msg = Message::new("agent", ct, b"data".to_vec(), None);
            let bytes = msg.to_bytes();
            let restored = Message::from_bytes(&bytes).unwrap();
            assert_eq!(restored.content_type, *ct);
        }
    }

    /// Test 11: Convenience constructors produce correct content_type.
    #[test]
    fn test_convenience_constructors() {
        let text_msg = Message::text("a", "hello");
        assert_eq!(text_msg.content_type, "text/plain");
        assert_eq!(text_msg.payload, b"hello");

        let json_msg = Message::json("a", r#"{"key":"value"}"#);
        assert_eq!(json_msg.content_type, "application/json");
        assert_eq!(json_msg.payload, br#"{"key":"value"}"#);

        let bin_msg = Message::binary("a", "image/png", vec![1, 2, 3]);
        assert_eq!(bin_msg.content_type, "image/png");
        assert_eq!(bin_msg.payload, vec![1, 2, 3]);
    }

    /// Test 12: payload_text returns decoded string.
    #[test]
    fn test_payload_text() {
        let msg = Message::text("agent", "hello world");
        assert_eq!(msg.payload_text(), "hello world");
    }

    /// Test 13: payload_json returns raw JSON string.
    #[test]
    fn test_payload_json() {
        let msg = Message::json("agent", r#"{"key":"value"}"#);
        assert_eq!(msg.payload_json(), r#"{"key":"value"}"#);
    }

    /// Test 14: envelope_to_json produces JSON without payload.
    #[test]
    fn test_envelope_to_json() {
        let msg = Message::text("agent", "hello");
        let json = msg.envelope_to_json();

        assert!(json.contains("\"id\""));
        assert!(json.contains("\"timestamp\""));
        assert!(json.contains("\"sender_id\":\"agent\""));
        assert!(json.contains("\"content_type\":\"text/plain\""));
        // Payload should NOT appear in the envelope
        assert!(!json.contains("hello"));
    }

    /// Test 15: to_bytes starts with "ACTM" magic bytes.
    #[test]
    fn test_wire_magic() {
        let msg = Message::text("agent", "test");
        let bytes = msg.to_bytes();
        assert_eq!(&bytes[0..4], b"ACTM");
    }

    /// Test 16: to_bytes contains correct version byte.
    #[test]
    fn test_wire_version() {
        let msg = Message::text("agent", "test");
        let bytes = msg.to_bytes();
        assert_eq!(bytes[4], WIRE_VERSION);
    }

    /// Test 17: from_bytes rejects future versions with VersionTooNew.
    #[test]
    fn test_future_version_rejection() {
        let msg = Message::text("agent", "test");
        let mut bytes = msg.to_bytes();
        // Tamper with version byte
        bytes[4] = WIRE_VERSION + 1;

        match Message::from_bytes(&bytes) {
            Err(ActorError::VersionTooNew {
                found,
                max_supported,
            }) => {
                assert_eq!(found, WIRE_VERSION + 1);
                assert_eq!(max_supported, WIRE_VERSION);
            }
            other => panic!("expected VersionTooNew, got {:?}", other),
        }
    }

    /// Test 18: from_bytes rejects corrupt magic bytes.
    #[test]
    fn test_corrupt_magic_rejection() {
        let msg = Message::text("agent", "test");
        let mut bytes = msg.to_bytes();
        bytes[0] = b'X'; // Corrupt the magic

        match Message::from_bytes(&bytes) {
            Err(ActorError::InvalidFormat(_)) => {} // Expected
            other => panic!("expected InvalidFormat, got {:?}", other),
        }
    }

    /// Test 19: from_reader reads exactly one message from a stream.
    #[test]
    fn test_stream_reading() {
        let msg1 = Message::text("agent", "first");
        let msg2 = Message::text("agent", "second");

        // Concatenate two messages into a single stream
        let mut stream_data = msg1.to_bytes();
        stream_data.extend_from_slice(&msg2.to_bytes());

        let mut cursor = Cursor::new(stream_data);

        // Read first message
        let read1 = Message::from_reader(&mut cursor).unwrap();
        assert_eq!(read1.id, msg1.id);
        assert_eq!(read1.payload_text(), "first");

        // Read second message
        let read2 = Message::from_reader(&mut cursor).unwrap();
        assert_eq!(read2.id, msg2.id);
        assert_eq!(read2.payload_text(), "second");

        // No more messages -- should get Eof
        match Message::from_reader(&mut cursor) {
            Err(ActorError::Eof) => {} // Expected
            other => panic!("expected Eof, got {:?}", other),
        }
    }

    /// Test: JSON escaping works for special characters.
    #[test]
    fn test_json_escape_roundtrip() {
        let msg = Message::text_with_metadata(
            "agent\"with\\quotes",
            "hello\nworld",
            {
                let mut m = HashMap::new();
                m.insert("key\"1".to_string(), "val\tue".to_string());
                m
            },
        );
        let bytes = msg.to_bytes();
        let restored = Message::from_bytes(&bytes).unwrap();
        assert_eq!(restored.sender_id, "agent\"with\\quotes");
        assert_eq!(restored.metadata.get("key\"1").unwrap(), "val\tue");
    }

    /// Test: from_bytes with too-short data returns InvalidFormat.
    #[test]
    fn test_too_short_data() {
        match Message::from_bytes(&[0u8; 5]) {
            Err(ActorError::InvalidFormat(_)) => {}
            other => panic!("expected InvalidFormat, got {:?}", other),
        }
    }
}
