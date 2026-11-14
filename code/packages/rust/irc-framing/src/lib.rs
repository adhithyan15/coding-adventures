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
//! RFC 2812 §2.3 keeps the same 512-byte limit.  A line still lacking its LF
//! after 511 bytes (510 plus a CR) is dropped early and the framer skips to the
//! next LF, so a peer that never sends LF cannot make the buffer grow (SEC-011).
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

/// RFC 1459 §2.3 / RFC 2812 §2.3: a message is at most 512 bytes *including*
/// the trailing CRLF, so the content is at most 510 bytes (512 - CRLF).
const MAX_CONTENT_BYTES: usize = 510;

/// The most bytes we will hold for one line that has not yet seen its LF.
///
/// A legal line can be 510 content bytes plus a CR whose LF has not arrived
/// yet, i.e. 511 bytes.  One more byte with no LF and the line can no longer
/// be legal (SEC-011), so we stop remembering it.
const MAX_PENDING_BYTES: usize = MAX_CONTENT_BYTES + 1;

/// Stateful byte-stream-to-line-frame converter.
///
/// Call [`feed`](Framer::feed) with raw bytes from the socket.
/// Call [`frames`](Framer::frames) to get all complete CRLF-stripped lines.
///
/// The `Framer` is **not thread-safe**.  Each connection should own its own
/// `Framer` instance.
pub struct Framer {
    buf: Vec<u8>,
    /// Length of the unterminated tail of `buf` (bytes after the last LF).
    /// Invariant: `tail_len <= MAX_PENDING_BYTES` after every `feed`.
    tail_len: usize,
    /// True while we are skipping the rest of an overlong line: everything up
    /// to and including the next LF is thrown away, unread.
    discarding: bool,
}

impl Framer {
    /// Create a new `Framer` with an empty internal buffer.
    pub fn new() -> Self {
        Framer {
            buf: Vec::new(),
            tail_len: 0,
            discarding: false,
        }
    }

    /// Append `data` to the internal buffer.
    ///
    /// Passing an empty slice is a safe no-op.
    ///
    /// ## Memory bound (SEC-011)
    ///
    /// The unterminated tail (bytes after the last LF) never exceeds
    /// [`MAX_PENDING_BYTES`] = 511.  A peer that sends bytes forever without an
    /// LF would otherwise grow the buffer without limit (CWE-400/770).  When
    /// the tail passes the cap, the partial line is dropped and the framer
    /// *discards input up to the next LF* -- fail closed, the same silent
    /// discard that already applies to a complete overlong line, so the API is
    /// unchanged.  Complete lines already buffered are kept; the caller bounds
    /// those by calling [`frames`](Framer::frames) after each read.
    ///
    /// ```text
    /// data = [ ...complete lines... LF ][ unterminated tail ]
    ///                                   ^ only this part can be flooded
    /// ```
    pub fn feed(&mut self, mut data: &[u8]) {
        // Step 1: while discarding an overlong line, skip to just past its LF.
        if self.discarding {
            match data.iter().position(|&b| b == b'\n') {
                None => return, // still inside the overlong line
                Some(pos) => {
                    self.discarding = false;
                    data = &data[pos + 1..];
                }
            }
        }

        // Step 2: split `data` at its last LF.  The head (through that LF)
        // completes lines and is stored; only the tail can be flooded.
        let (head, tail) = match data.iter().rposition(|&b| b == b'\n') {
            Some(pos) => data.split_at(pos + 1),
            None => (&data[..0], data),
        };
        if !head.is_empty() {
            self.buf.extend_from_slice(head);
            self.tail_len = 0;
        }

        // Step 3: store the tail only if the pending line still fits.  We
        // check *before* extending so a huge single feed is never copied.
        if self.tail_len + tail.len() > MAX_PENDING_BYTES {
            let keep = self.buf.len() - self.tail_len;
            self.buf.truncate(keep); // drop the partial line
            self.tail_len = 0;
            self.discarding = true;
        } else {
            self.buf.extend_from_slice(tail);
            self.tail_len += tail.len();
        }
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

        // Only the unterminated tail remains in the buffer.
        self.tail_len = self.buf.len();
        result
    }

    /// Discard all buffered data.
    pub fn reset(&mut self) {
        self.buf = Vec::new();
        self.tail_len = 0;
        self.discarding = false;
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

    // -----------------------------------------------------------------
    // SEC-011: a peer that never sends LF must not grow the buffer.
    // -----------------------------------------------------------------

    const SIXTEEN_MIB: usize = 16 * 1024 * 1024;

    #[test]
    fn test_no_lf_in_chunks_is_bounded() {
        let mut f = Framer::new();
        let chunk = [b'A'; 4096];
        let mut worst = 0;
        for _ in 0..(SIXTEEN_MIB / chunk.len()) {
            f.feed(&chunk);
            worst = worst.max(f.buffer_size());
        }
        assert!(worst <= 512, "buffer grew to {worst} bytes with no LF");
        assert!(f.frames().is_empty());
    }

    #[test]
    fn test_no_lf_in_one_feed_is_bounded() {
        let mut f = Framer::new();
        f.feed(&vec![b'A'; SIXTEEN_MIB]);
        assert!(f.buffer_size() <= 512, "buffer grew to {}", f.buffer_size());
        assert!(f.frames().is_empty());
    }

    #[test]
    fn test_framer_recovers_after_flood() {
        // The flooded line is dropped whole, including bytes that arrive
        // after the cap, up to its LF; the next line frames normally.
        let mut f = Framer::new();
        f.feed(&vec![b'A'; 4096]);
        f.feed(&vec![b'B'; 4096]);
        f.feed(b"tail\r\nNICK alice\r\n");
        assert_eq!(f.frames(), vec![b"NICK alice".to_vec()]);
        assert_eq!(f.buffer_size(), 0);
    }

    #[test]
    fn test_pending_line_at_limit_is_kept() {
        // 510 content bytes plus a CR is a legal line still waiting for its LF.
        let mut f = Framer::new();
        let mut data = vec![b'A'; 510];
        data.push(b'\r');
        f.feed(&data);
        assert_eq!(f.buffer_size(), 511);
        f.feed(b"\n");
        assert_eq!(f.frames(), vec![vec![b'A'; 510]]);
    }

    #[test]
    fn test_pending_line_over_limit_is_dropped() {
        let mut f = Framer::new();
        f.feed(&vec![b'A'; 512]);
        assert_eq!(f.buffer_size(), 0);
        f.feed(b"\r\nPING x\r\n");
        assert_eq!(f.frames(), vec![b"PING x".to_vec()]);
    }

    #[test]
    fn test_complete_lines_before_flood_survive() {
        let mut f = Framer::new();
        let mut data = b"NICK a\r\n".to_vec();
        data.extend(vec![b'A'; 2000]);
        f.feed(&data);
        assert_eq!(f.frames(), vec![b"NICK a".to_vec()]);
        assert_eq!(f.buffer_size(), 0);
    }

    #[test]
    fn test_reset_clears_discard_mode() {
        let mut f = Framer::new();
        f.feed(&vec![b'A'; 2000]);
        f.reset();
        f.feed(b"PING y\r\n");
        assert_eq!(f.frames(), vec![b"PING y".to_vec()]);
    }
}
