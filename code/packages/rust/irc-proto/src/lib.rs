//! # irc-proto — Pure IRC message parsing and serialization (RFC 1459)
//!
//! This crate is the foundation of the IRC stack.  It knows nothing about
//! sockets, threads, or buffers — it only converts between the raw text
//! lines of the IRC protocol and structured [`Message`] values.
//!
//! Every other IRC crate depends on `irc-proto`'s [`Message`] type, but
//! `irc-proto` itself depends on nothing.  This is intentional: a pure
//! parsing library is easy to test exhaustively and easy to port to new
//! languages.
//!
//! ## IRC message grammar (from RFC 1459)
//!
//! ```text
//! message  = [ ":" prefix SPACE ] command [ params ] CRLF
//! prefix   = servername / ( nick [ "!" user ] [ "@" host ] )
//! command  = 1*letter / 3digit
//! params   = 0*14( SPACE middle ) [ SPACE ":" trailing ]
//!          / 14( SPACE middle ) [ SPACE [ ":" ] trailing ]
//! middle   = nospcrlfcl *( ":" / nospcrlfcl )
//! trailing = *( ":" / " " / nospcrlfcl )
//! SPACE    = 0x20
//! ```
//!
//! In practice: a message is at most 512 bytes including the final CRLF, and
//! carries a prefix, a command, and up to 15 parameters (the last of which may
//! contain spaces when prefixed by `:`).
//!
//! ## Example
//!
//! ```
//! use irc_proto::{parse, serialize, Message};
//!
//! let msg = parse("NICK alice").unwrap();
//! assert_eq!(msg.command, "NICK");
//! assert_eq!(msg.params, vec!["alice"]);
//! assert_eq!(msg.prefix, None);
//!
//! let bytes = serialize(&msg);
//! assert_eq!(bytes, b"NICK alice\r\n");
//! ```

// ──────────────────────────────────────────────────────────────────────────────
// Data model
// ──────────────────────────────────────────────────────────────────────────────

/// A single parsed IRC protocol message.
///
/// Think of this as a plain envelope with three slots:
///
/// - `prefix`  — *Who sent it?*  `None` for client-originated messages.
/// - `command` — *What kind of message is it?*  Always uppercase.
/// - `params`  — *The arguments.*  A `Vec<String>`.  The trailing param's
///   leading `:` is already stripped.
#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub prefix: Option<String>,
    pub command: String,
    pub params: Vec<String>,
}

// ──────────────────────────────────────────────────────────────────────────────
// Error type
// ──────────────────────────────────────────────────────────────────────────────

/// Error returned when a raw IRC line cannot be parsed.
#[derive(Debug)]
pub struct ParseError(pub String);

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "IRC parse error: {}", self.0)
    }
}

impl std::error::Error for ParseError {}

// RFC 1459 allows at most 15 parameters in a single message.
const MAX_PARAMS: usize = 15;

// ──────────────────────────────────────────────────────────────────────────────
// Parsing
// ──────────────────────────────────────────────────────────────────────────────

/// Parse a single IRC message line into a [`Message`].
///
/// The `line` must already have its trailing `\r\n` stripped.
///
/// # Errors
///
/// Returns [`ParseError`] when the line is empty, whitespace-only, or has
/// no command token.
///
/// # Examples
///
/// ```
/// use irc_proto::parse;
///
/// let msg = parse("NICK alice").unwrap();
/// assert_eq!(msg.command, "NICK");
/// assert_eq!(msg.params, vec!["alice"]);
///
/// let msg = parse(":irc.local 001 alice :Welcome!").unwrap();
/// assert_eq!(msg.prefix, Some("irc.local".to_string()));
/// assert_eq!(msg.command, "001");
/// assert_eq!(msg.params, vec!["alice", "Welcome!"]);
/// ```
pub fn parse(line: &str) -> Result<Message, ParseError> {
    // Stage 0: reject empty / whitespace-only input.
    if line.is_empty() || line.trim().is_empty() {
        return Err(ParseError(format!("empty or whitespace-only line: {:?}", line)));
    }

    let mut rest = line;

    // Stage 1: optional prefix.
    // A leading colon signals that a prefix follows.
    let prefix: Option<String> = if rest.starts_with(':') {
        match rest.find(' ') {
            None => {
                return Err(ParseError(format!("line has prefix but no command: {:?}", line)));
            }
            Some(space_pos) => {
                let p = rest[1..space_pos].to_string();
                rest = &rest[space_pos + 1..];
                Some(p)
            }
        }
    } else {
        None
    };

    // Stage 2: command — first whitespace-delimited token, normalized to uppercase.
    let (command_raw, remainder) = match rest.find(' ') {
        None => (rest, ""),
        Some(pos) => (&rest[..pos], &rest[pos + 1..]),
    };

    let command = command_raw.to_uppercase();
    if command.is_empty() {
        return Err(ParseError(format!("could not extract command from line: {:?}", line)));
    }
    rest = remainder;

    // Stage 3: parameters.
    // When a token begins with `:`, it and everything after it forms the last
    // parameter (trailing param), with the `:` stripped.
    let mut params: Vec<String> = Vec::new();

    while !rest.is_empty() {
        if rest.starts_with(':') {
            // Trailing param — absorbs the rest of the line.
            params.push(rest[1..].to_string());
            break;
        }

        match rest.find(' ') {
            None => {
                params.push(rest.to_string());
                break;
            }
            Some(space_pos) => {
                let token = rest[..space_pos].to_string();
                params.push(token);
                rest = &rest[space_pos + 1..];
            }
        }

        if params.len() == MAX_PARAMS {
            break;
        }
    }

    Ok(Message { prefix, command, params })
}

// ──────────────────────────────────────────────────────────────────────────────
// Serialization
// ──────────────────────────────────────────────────────────────────────────────

/// Serialize a [`Message`] to IRC wire format (CRLF terminated).
///
/// Returns bytes ready to be written to a socket.
///
/// # Examples
///
/// ```
/// use irc_proto::{serialize, Message};
///
/// let msg = Message { prefix: None, command: "NICK".to_string(), params: vec!["alice".to_string()] };
/// assert_eq!(serialize(&msg), b"NICK alice\r\n");
///
/// let msg = Message {
///     prefix: None,
///     command: "PRIVMSG".to_string(),
///     params: vec!["#chan".to_string(), "hello world".to_string()],
/// };
/// assert_eq!(serialize(&msg), b"PRIVMSG #chan :hello world\r\n");
/// ```
pub fn serialize(msg: &Message) -> Vec<u8> {
    let mut parts: Vec<String> = Vec::new();

    // Prefix: if present, prepend with ":"
    if let Some(ref p) = msg.prefix {
        parts.push(format!(":{}", p));
    }

    // Command
    parts.push(msg.command.clone());

    // Parameters: the last param gets a ":" prefix if it contains spaces,
    // is empty, or starts with ":"
    let n = msg.params.len();
    for (i, param) in msg.params.iter().enumerate() {
        let is_last = i == n - 1;
        if is_last && (param.contains(' ') || param.is_empty() || param.starts_with(':')) {
            parts.push(format!(":{}", param));
        } else {
            parts.push(param.clone());
        }
    }

    // Join with spaces and append CRLF
    let line = parts.join(" ") + "\r\n";
    line.into_bytes()
}

// ──────────────────────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_nick() {
        let msg = parse("NICK alice").unwrap();
        assert_eq!(msg.prefix, None);
        assert_eq!(msg.command, "NICK");
        assert_eq!(msg.params, vec!["alice"]);
    }

    #[test]
    fn test_parse_command_uppercased() {
        let msg = parse("join #general").unwrap();
        assert_eq!(msg.command, "JOIN");
    }

    #[test]
    fn test_parse_with_server_prefix() {
        let msg = parse(":irc.local 001 alice :Welcome to the network!").unwrap();
        assert_eq!(msg.prefix, Some("irc.local".to_string()));
        assert_eq!(msg.command, "001");
        assert_eq!(msg.params, vec!["alice", "Welcome to the network!"]);
    }

    #[test]
    fn test_parse_with_user_mask_prefix() {
        let msg = parse(":alice!alice@127.0.0.1 PRIVMSG #chan :hello world").unwrap();
        assert_eq!(msg.prefix, Some("alice!alice@127.0.0.1".to_string()));
        assert_eq!(msg.command, "PRIVMSG");
        assert_eq!(msg.params, vec!["#chan", "hello world"]);
    }

    #[test]
    fn test_parse_trailing_param_preserves_spaces() {
        let msg = parse("PRIVMSG #chan :hello   world   !").unwrap();
        assert_eq!(msg.params, vec!["#chan", "hello   world   !"]);
    }

    #[test]
    fn test_parse_no_params() {
        let msg = parse("PING").unwrap();
        assert_eq!(msg.command, "PING");
        assert!(msg.params.is_empty());
    }

    #[test]
    fn test_parse_empty_line_errors() {
        assert!(parse("").is_err());
    }

    #[test]
    fn test_parse_whitespace_only_errors() {
        assert!(parse("   ").is_err());
    }

    #[test]
    fn test_parse_prefix_only_errors() {
        assert!(parse(":irc.local").is_err());
    }

    #[test]
    fn test_parse_user_command() {
        let msg = parse("USER alice 0 * :Alice Smith").unwrap();
        assert_eq!(msg.command, "USER");
        assert_eq!(msg.params, vec!["alice", "0", "*", "Alice Smith"]);
    }

    #[test]
    fn test_parse_max_params_enforced() {
        let extra_params: Vec<&str> = (0..16).map(|_| "x").collect();
        let line = format!("CMD {}", extra_params.join(" "));
        let msg = parse(&line).unwrap();
        assert_eq!(msg.params.len(), MAX_PARAMS);
    }

    #[test]
    fn test_parse_empty_trailing() {
        let msg = parse("AWAY :").unwrap();
        assert_eq!(msg.params, vec![""]);
    }

    #[test]
    fn test_serialize_simple() {
        let msg = Message {
            prefix: None,
            command: "NICK".to_string(),
            params: vec!["alice".to_string()],
        };
        assert_eq!(serialize(&msg), b"NICK alice\r\n");
    }

    #[test]
    fn test_serialize_trailing_param_with_spaces() {
        let msg = Message {
            prefix: Some("alice!alice@host".to_string()),
            command: "PRIVMSG".to_string(),
            params: vec!["#chan".to_string(), "hello world".to_string()],
        };
        assert_eq!(serialize(&msg), b":alice!alice@host PRIVMSG #chan :hello world\r\n");
    }

    #[test]
    fn test_serialize_no_params() {
        let msg = Message {
            prefix: None,
            command: "PING".to_string(),
            params: vec![],
        };
        assert_eq!(serialize(&msg), b"PING\r\n");
    }

    #[test]
    fn test_serialize_empty_trailing_param() {
        let msg = Message {
            prefix: None,
            command: "AWAY".to_string(),
            params: vec!["".to_string()],
        };
        let output = std::str::from_utf8(&serialize(&msg)).unwrap().to_string();
        assert_eq!(output, "AWAY :\r\n");
    }

    #[test]
    fn test_round_trip_privmsg() {
        let original = ":alice!alice@host PRIVMSG #chan :hello world\r\n";
        let msg = parse(original.trim_end_matches("\r\n")).unwrap();
        let reserialized = serialize(&msg);
        assert_eq!(reserialized, original.as_bytes());
    }

    #[test]
    fn test_round_trip_nick() {
        let msg = parse("NICK alice").unwrap();
        assert_eq!(serialize(&msg), b"NICK alice\r\n");
    }

    #[test]
    fn test_numeric_command() {
        let msg = parse(":server.local 433 * nick :Nickname is already in use").unwrap();
        assert_eq!(msg.command, "433");
        assert_eq!(msg.params, vec!["*", "nick", "Nickname is already in use"]);
    }
}
