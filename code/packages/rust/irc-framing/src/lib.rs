//! # irc-framing — Stateful byte-stream-to-line-frame converter
//!
//! ## The Problem: TCP delivers a byte stream, not messages
//!
//! When you call `read()` on a TCP socket, the OS hands you however many bytes
//! happen to be available — half a message, one complete message, or several.
//!
//! IRC solves this with a simple framing convention: every message ends with
//! `\r\n` (CRLF).  The framer's job is to absorb raw byte chunks and emit
//! complete, `\r\n`-stripped lines to the layer above.
//!
//! ## Layer diagram
//!
//! ```text
//! irc-proto   ← receives complete \r\n-stripped bytes
//!      ↑
//! irc-framing ← THIS CRATE: feed(raw_bytes) / frames() → Vec<Vec<u8>>
//!      ↑
//! irc-net     ← calls read() and feeds raw bytes upward
//! ```
//!
//! ## RFC 1459 maximum line length
//!
//! RFC 1459 §2.3: maximum 512 bytes per message including `\r\n`, leaving at
//! most 510 bytes of content.  Lines exceeding 510 bytes are silently discarded.
//!
//! ## Usage
//!
//! ```
//! use irc_framing::Framer;
//!
//! let mut framer = Framer::new();
//! framer.feed(b"NICK alice\r\nUSER alice 0 * :Al");
//! framer.feed(b"ice Smith\r\n");
//!
//! let frames = framer.frames();
//! assert_eq!(frames.len(), 2);
//! assert_eq!(frames[0], b"NICK alice");
//! assert_eq!(frames[1], b"USER alice 0 * :Alice Smith");
//! ```

/// RFC 1459 §2.3: maximum line content is 510 bytes (512 - CRLF).
const MAX_CONTENT_BYTES: usize = 510;

/// Stateful byte-stream-to-line-frame converter.
///
/// Call [`feed`](Framer::feed) with raw bytes from the socket.
/// Call [`frames`](Framer::frames) to get all complete CRLF-stripped lines.
///
/// The `Framer` is **not thread-safe**.  Each connection should own its own
/// `Framer` instance.
pub struct Framer {
    buf: Vec<u8>,
}

impl Framer {
    /// Create a new `Framer` with an empty internal buffer.
    pub fn new() -> Self {
        Framer { buf: Vec::new() }
    }

    /// Append `data` to the internal buffer.
    ///
    /// Passing an empty slice is a safe no-op.
    pub fn feed(&mut self, data: &[u8]) {
        self.buf.extend_from_slice(data);
    }

    /// Drain all complete frames from the buffer, with CRLF stripped.
    ///
    /// Lines exceeding 510 bytes are silently discarded.
    pub fn frames(&mut self) -> Vec<Vec<u8>> {
        let mut result = Vec::new();

        loop {
            // Find the first LF byte.
            let lf_pos = match self.buf.iter().position(|&b| b == b'\n') {
                None => break,
                Some(pos) => pos,
            };

            // If there's a CR immediately before the LF, exclude it too.
            let content_end = if lf_pos > 0 && self.buf[lf_pos - 1] == b'\r' {
                lf_pos - 1
            } else {
                lf_pos
            };

            // Extract the frame content (excluding CR/LF).
            let line: Vec<u8> = self.buf[..content_end].to_vec();

            // Remove everything up to and including the LF.
            self.buf.drain(..=lf_pos);

            // Discard overlong lines (RFC 1459 §2.3).
            if line.len() > MAX_CONTENT_BYTES {
                continue;
            }

            result.push(line);
        }

        result
    }

    /// Discard all buffered data.
    pub fn reset(&mut self) {
        self.buf = Vec::new();
    }

    /// Number of bytes currently held in the internal buffer.
    pub fn buffer_size(&self) -> usize {
        self.buf.len()
    }
}

impl Default for Framer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_complete_crlf_message() {
        let mut f = Framer::new();
        f.feed(b"NICK alice\r\n");
        let frames = f.frames();
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0], b"NICK alice");
    }

    #[test]
    fn test_single_lf_only_message() {
        let mut f = Framer::new();
        f.feed(b"NICK alice\n");
        let frames = f.frames();
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0], b"NICK alice");
    }

    #[test]
    fn test_multiple_messages_in_one_feed() {
        let mut f = Framer::new();
        f.feed(b"NICK alice\r\nUSER alice 0 * :Alice\r\n");
        let frames = f.frames();
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0], b"NICK alice");
        assert_eq!(frames[1], b"USER alice 0 * :Alice");
    }

    #[test]
    fn test_partial_message_buffered() {
        let mut f = Framer::new();
        f.feed(b"NICK al");
        assert!(f.frames().is_empty());
        assert_eq!(f.buffer_size(), 7);

        f.feed(b"ice\r\n");
        let frames = f.frames();
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0], b"NICK alice");
        assert_eq!(f.buffer_size(), 0);
    }

    #[test]
    fn test_split_across_crlf_boundary() {
        let mut f = Framer::new();
        f.feed(b"NICK alice\r");
        assert!(f.frames().is_empty());
        f.feed(b"\n");
        let frames = f.frames();
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0], b"NICK alice");
    }

    #[test]
    fn test_empty_feed() {
        let mut f = Framer::new();
        f.feed(b"");
        assert_eq!(f.buffer_size(), 0);
        assert!(f.frames().is_empty());
    }

    #[test]
    fn test_just_crlf() {
        let mut f = Framer::new();
        f.feed(b"\r\n");
        let frames = f.frames();
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0], b"");
    }

    #[test]
    fn test_overlong_line_discarded() {
        let overlong = vec![b'A'; 511];
        let mut f = Framer::new();
        f.feed(&overlong);
        f.feed(b"\r\n");
        assert!(f.frames().is_empty());
    }

    #[test]
    fn test_exact_max_length_accepted() {
        let exact = vec![b'A'; 510];
        let mut f = Framer::new();
        f.feed(&exact);
        f.feed(b"\r\n");
        let frames = f.frames();
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].len(), 510);
    }

    #[test]
    fn test_overlong_followed_by_valid() {
        let mut overlong = vec![b'X'; 511];
        overlong.extend_from_slice(b"\r\n");
        overlong.extend_from_slice(b"NICK alice\r\n");
        let mut f = Framer::new();
        f.feed(&overlong);
        let frames = f.frames();
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0], b"NICK alice");
    }

    #[test]
    fn test_reset_clears_buffer() {
        let mut f = Framer::new();
        f.feed(b"partial data");
        f.reset();
        assert_eq!(f.buffer_size(), 0);
        assert!(f.frames().is_empty());
    }

    #[test]
    fn test_default_produces_empty_framer() {
        let f = Framer::default();
        assert_eq!(f.buffer_size(), 0);
    }

    #[test]
    fn test_three_messages_split_across_feeds() {
        let mut f = Framer::new();
        f.feed(b"JOIN #one\r\nJOIN");
        let f1 = f.frames();
        assert_eq!(f1.len(), 1);
        assert_eq!(f1[0], b"JOIN #one");

        f.feed(b" #two\r\nJOIN #three\r\n");
        let f2 = f.frames();
        assert_eq!(f2.len(), 2);
        assert_eq!(f2[0], b"JOIN #two");
        assert_eq!(f2[1], b"JOIN #three");
    }
}
