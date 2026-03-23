//! # Channel -- One-Way, Append-Only Message Log
//!
//! A Channel is an ordered, immutable log of messages. It connects message
//! producers to message consumers. Messages flow in one direction only. Once
//! appended, a message cannot be removed, modified, or reordered.
//!
//! ## Analogy
//!
//! A Channel is a one-way pneumatic tube in an office building. Documents go
//! in one end and come out the other. You cannot send documents backwards.
//! The tube keeps a copy of every document that has ever passed through it
//! (the log), and each office at the receiving end has a bookmark showing
//! which documents they have already read (the offset).
//!
//! ## Why One-Way?
//!
//! Bidirectional channels create ambiguity: "who sent this message?" and
//! "can a receiver inject messages that look like they came from the sender?"
//! One-way channels eliminate both questions. If you need bidirectional
//! communication, use two channels -- one in each direction.
//!
//! ## Why Append-Only?
//!
//! If messages could be deleted or modified, crash recovery becomes impossible.
//! After a crash, the system asks: "what happened before the crash?" If the log
//! is mutable, the answer is "we don't know." If the log is append-only, the
//! answer is definitive: "here is exactly what happened, in order."
//!
//! ## Persistence
//!
//! Channels persist to disk as a **binary append log** using the same wire
//! format as individual messages. Each message is written as its header +
//! envelope + payload, concatenated end-to-end:
//!
//! ```text
//! Channel file: channels/email-summaries.log
//!
//! [ACTM][v1][env_len][pay_len] message 0 header
//! {JSON envelope}              message 0 envelope
//! <raw payload bytes>          message 0 payload
//! [ACTM][v1][env_len][pay_len] message 1 header
//! {JSON envelope}              message 1 envelope
//! <raw payload bytes>          message 1 payload
//! ...
//! ```
//!
//! This format is binary-native (no Base64 bloat), appendable, replayable,
//! and scannable without loading payloads.

use std::fs;
use std::io::Cursor;
use std::path::Path;

use crate::message::{ActorError, Message};

/// A one-way, append-only, ordered message log.
///
/// ## Fields
///
/// - `id` -- Unique identifier for this channel.
/// - `name` -- Human-readable name (e.g., "email-summaries", "vault-requests").
/// - `log` -- Ordered list of Messages. Index 0 is the first message ever written.
/// - `created_at` -- Timestamp when the channel was created (logical clock value).
///
/// ## Operations
///
/// ```text
/// +------------------+-----------------------------------------------+
/// | Operation        | Description                                   |
/// +------------------+-----------------------------------------------+
/// | append(msg)      | Add message to end of log, return seq number  |
/// | read(off, lim)   | Return up to `lim` messages from `off`        |
/// | len()            | Number of messages in the log                 |
/// | slice(start,end) | Messages from start to end (exclusive)        |
/// | persist(dir)     | Write log to disk as binary append log        |
/// | recover(dir,name)| Reconstruct channel from disk                 |
/// +------------------+-----------------------------------------------+
/// ```
#[derive(Debug)]
pub struct Channel {
    pub id: String,
    pub name: String,
    pub log: Vec<Message>,
    pub created_at: u64,
}

impl Channel {
    /// Create a new empty channel.
    ///
    /// # Arguments
    ///
    /// * `id` -- Unique identifier for this channel
    /// * `name` -- Human-readable name for discovery and debugging
    ///
    /// # Example
    ///
    /// ```
    /// use actor::Channel;
    /// let ch = Channel::new("ch_001", "email-summaries");
    /// assert_eq!(ch.name, "email-summaries");
    /// assert_eq!(ch.len(), 0);
    /// ```
    pub fn new(id: &str, name: &str) -> Self {
        Channel {
            id: id.to_string(),
            name: name.to_string(),
            log: Vec::new(),
            created_at: 0,
        }
    }

    /// Append a message to the end of the log.
    ///
    /// Returns the 0-indexed sequence number of the appended message.
    /// This is the ONLY write operation. There is no delete, no update,
    /// no insert-at-position.
    ///
    /// # How it works
    ///
    /// ```text
    /// Before:  [m0] [m1] [m2]     (length = 3)
    /// append(m3)
    /// After:   [m0] [m1] [m2] [m3] (length = 4, returns 3)
    /// ```
    pub fn append(&mut self, msg: Message) -> usize {
        let seq = self.log.len();
        self.log.push(msg);
        seq
    }

    /// Read up to `limit` messages starting from `offset`.
    ///
    /// If offset >= log length, returns an empty list (caller is caught up).
    /// If offset + limit > log length, returns the remaining messages.
    /// This does NOT consume messages -- they remain in the log. Another
    /// reader can read the same messages independently.
    ///
    /// # Offset Tracking
    ///
    /// Each consumer independently tracks how far it has read. This is NOT
    /// managed by the channel -- it is the consumer's responsibility:
    ///
    /// ```text
    /// Channel log:   [m0] [m1] [m2] [m3] [m4] [m5]
    ///                                      ^
    /// Consumer A:    offset = 4 -----------+
    ///                      ^
    /// Consumer B:    offset = 1 (behind)
    /// ```
    pub fn read(&self, offset: usize, limit: usize) -> Vec<Message> {
        if offset >= self.log.len() {
            return Vec::new();
        }
        let end = std::cmp::min(offset + limit, self.log.len());
        self.log[offset..end].to_vec()
    }

    /// Number of messages in the log.
    pub fn len(&self) -> usize {
        self.log.len()
    }

    /// Whether the channel log is empty.
    pub fn is_empty(&self) -> bool {
        self.log.is_empty()
    }

    /// Return messages from index `start` to `end` (exclusive).
    ///
    /// Equivalent to `read(start, end - start)`.
    ///
    /// # Panics
    ///
    /// Returns empty vec if start >= log length or start >= end.
    pub fn slice(&self, start: usize, end: usize) -> Vec<Message> {
        if start >= self.log.len() || start >= end {
            return Vec::new();
        }
        let actual_end = std::cmp::min(end, self.log.len());
        self.log[start..actual_end].to_vec()
    }

    /// Persist the channel log to disk as a binary append log.
    ///
    /// Each message is written in the wire format (header + envelope + payload)
    /// concatenated end-to-end. The file name is `{name}.log` inside the
    /// given directory.
    ///
    /// ## File format
    ///
    /// ```text
    /// [ACTM][v1][envelope_len][payload_len][envelope JSON][payload bytes]
    /// [ACTM][v1][envelope_len][payload_len][envelope JSON][payload bytes]
    /// ...
    /// ```
    ///
    /// This format is:
    /// - Binary-native: no Base64 bloat for binary payloads
    /// - Appendable: just write the next message at the end
    /// - Replayable: read from the beginning, parse each message in sequence
    /// - Scannable: skip payloads by reading only headers and envelopes
    pub fn persist(&self, directory: &str) -> Result<(), ActorError> {
        let dir_path = Path::new(directory);
        fs::create_dir_all(dir_path).map_err(ActorError::Io)?;

        let file_path = dir_path.join(format!("{}.log", self.name));
        let mut data = Vec::new();
        for msg in &self.log {
            data.extend_from_slice(&msg.to_bytes());
        }
        fs::write(file_path, &data).map_err(ActorError::Io)?;
        Ok(())
    }

    /// Recover a channel from a persisted binary log on disk.
    ///
    /// Reads the file `{directory}/{name}.log` and reconstructs all messages.
    /// If the file does not exist, returns an empty channel.
    ///
    /// ## Crash Recovery
    ///
    /// If the file ends with a partial/truncated message (e.g., a crash
    /// happened mid-write), the incomplete message is silently discarded.
    /// All complete messages before it are recovered. The 17-byte header
    /// acts as a frame boundary -- if a crash happens mid-write, the next
    /// recovery sees a truncated header or incomplete payload and cleanly
    /// discards only the partial message.
    ///
    /// ```text
    /// [complete msg 0] [complete msg 1] [truncated msg 2...]
    ///                                    ↑ discarded on recovery
    /// ```
    pub fn recover(directory: &str, name: &str) -> Result<Channel, ActorError> {
        let file_path = Path::new(directory).join(format!("{}.log", name));

        if !file_path.exists() {
            // No file means empty channel -- this is normal, not an error
            return Ok(Channel::new("", name));
        }

        let data = fs::read(&file_path).map_err(ActorError::Io)?;
        let mut cursor = Cursor::new(data);
        let mut channel = Channel::new("", name);

        loop {
            match Message::from_reader(&mut cursor) {
                Ok(msg) => {
                    channel.log.push(msg);
                }
                Err(ActorError::Eof) => break,
                Err(ActorError::InvalidFormat(_)) => {
                    // Truncated message at end of file -- discard and stop
                    break;
                }
                Err(e) => return Err(e),
            }
        }

        Ok(channel)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    /// Helper: create a text message with a given payload.
    fn text_msg(payload: &str) -> Message {
        Message::text("test_sender", payload)
    }

    /// Test 20: Create channel, verify id and name.
    #[test]
    fn test_create_channel() {
        let ch = Channel::new("ch_001", "greetings");
        assert_eq!(ch.id, "ch_001");
        assert_eq!(ch.name, "greetings");
        assert_eq!(ch.len(), 0);
    }

    /// Test 21: Append 3 messages, verify length.
    #[test]
    fn test_append_and_length() {
        let mut ch = Channel::new("ch_001", "test");
        ch.append(text_msg("a"));
        ch.append(text_msg("b"));
        ch.append(text_msg("c"));
        assert_eq!(ch.len(), 3);
    }

    /// Test 22: Append returns correct sequence numbers (0, 1, 2).
    #[test]
    fn test_append_returns_sequence_number() {
        let mut ch = Channel::new("ch_001", "test");
        assert_eq!(ch.append(text_msg("a")), 0);
        assert_eq!(ch.append(text_msg("b")), 1);
        assert_eq!(ch.append(text_msg("c")), 2);
    }

    /// Test 23: Read from beginning returns all messages in order.
    #[test]
    fn test_read_from_beginning() {
        let mut ch = Channel::new("ch_001", "test");
        for i in 0..5 {
            ch.append(text_msg(&format!("msg_{}", i)));
        }
        let msgs = ch.read(0, 5);
        assert_eq!(msgs.len(), 5);
        for i in 0..5 {
            assert_eq!(msgs[i].payload_text(), format!("msg_{}", i));
        }
    }

    /// Test 24: Read with offset returns correct subset.
    #[test]
    fn test_read_with_offset() {
        let mut ch = Channel::new("ch_001", "test");
        for i in 0..5 {
            ch.append(text_msg(&format!("msg_{}", i)));
        }
        let msgs = ch.read(2, 3);
        assert_eq!(msgs.len(), 3);
        assert_eq!(msgs[0].payload_text(), "msg_2");
        assert_eq!(msgs[1].payload_text(), "msg_3");
        assert_eq!(msgs[2].payload_text(), "msg_4");
    }

    /// Test 25: Read past end returns empty list.
    #[test]
    fn test_read_past_end() {
        let mut ch = Channel::new("ch_001", "test");
        ch.append(text_msg("a"));
        ch.append(text_msg("b"));
        ch.append(text_msg("c"));
        let msgs = ch.read(5, 10);
        assert!(msgs.is_empty());
    }

    /// Test 26: Read with limit returns only that many.
    #[test]
    fn test_read_with_limit() {
        let mut ch = Channel::new("ch_001", "test");
        for i in 0..10 {
            ch.append(text_msg(&format!("msg_{}", i)));
        }
        let msgs = ch.read(0, 3);
        assert_eq!(msgs.len(), 3);
    }

    /// Test 27: Slice returns correct range (exclusive end).
    #[test]
    fn test_slice() {
        let mut ch = Channel::new("ch_001", "test");
        for i in 0..5 {
            ch.append(text_msg(&format!("msg_{}", i)));
        }
        let msgs = ch.slice(1, 4);
        assert_eq!(msgs.len(), 3);
        assert_eq!(msgs[0].payload_text(), "msg_1");
        assert_eq!(msgs[1].payload_text(), "msg_2");
        assert_eq!(msgs[2].payload_text(), "msg_3");
    }

    /// Test 28: Two consumers read the same channel independently.
    #[test]
    fn test_independent_readers() {
        let mut ch = Channel::new("ch_001", "test");
        for i in 0..6 {
            ch.append(text_msg(&format!("msg_{}", i)));
        }

        // Consumer A reads from offset 4
        let batch_a = ch.read(4, 10);
        assert_eq!(batch_a.len(), 2);
        assert_eq!(batch_a[0].payload_text(), "msg_4");

        // Consumer B reads from offset 1
        let batch_b = ch.read(1, 2);
        assert_eq!(batch_b.len(), 2);
        assert_eq!(batch_b[0].payload_text(), "msg_1");

        // Both reads are independent -- channel still has all messages
        assert_eq!(ch.len(), 6);
    }

    /// Test 29: Channel is append-only (no delete/update methods exist).
    ///
    /// This test verifies by construction: Channel only exposes `append` as
    /// a write operation. There is no `delete`, `update`, `remove`, or
    /// `insert_at` method. Rust's type system enforces this.
    #[test]
    fn test_append_only() {
        let mut ch = Channel::new("ch_001", "test");
        ch.append(text_msg("first"));
        ch.append(text_msg("second"));
        // Verify we can read them back unchanged
        assert_eq!(ch.read(0, 2)[0].payload_text(), "first");
        assert_eq!(ch.read(0, 2)[1].payload_text(), "second");
    }

    /// Test 30: Binary persistence -- file starts with ACTM magic.
    #[test]
    fn test_binary_persistence() {
        let dir = tempdir("test_persist_magic");
        let mut ch = Channel::new("ch_001", "test-persist");
        ch.append(text_msg("hello"));
        ch.append(Message::binary("agent", "image/png", vec![0x89, 0x50, 0x4E, 0x47]));
        ch.persist(&dir).unwrap();

        let file_path = Path::new(&dir).join("test-persist.log");
        let data = fs::read(file_path).unwrap();
        assert_eq!(&data[0..4], b"ACTM");
        cleanup_tempdir(&dir);
    }

    /// Test 31: Persist and recover -- all messages restored.
    #[test]
    fn test_recovery() {
        let dir = tempdir("test_recovery");
        let mut ch = Channel::new("ch_001", "recovery-test");
        let orig_msg = text_msg("hello recovery");
        let orig_id = orig_msg.id.clone();
        ch.append(orig_msg);
        ch.append(Message::binary("agent", "image/png", vec![0x89, 0x50, 0x4E, 0x47]));
        ch.persist(&dir).unwrap();

        let recovered = Channel::recover(&dir, "recovery-test").unwrap();
        assert_eq!(recovered.len(), 2);
        assert_eq!(recovered.log[0].id, orig_id);
        assert_eq!(recovered.log[0].payload_text(), "hello recovery");
        assert_eq!(recovered.log[1].content_type, "image/png");
        assert_eq!(recovered.log[1].payload, vec![0x89, 0x50, 0x4E, 0x47]);
        cleanup_tempdir(&dir);
    }

    /// Test 32: Recovery preserves message order (100 messages).
    #[test]
    fn test_recovery_preserves_order() {
        let dir = tempdir("test_order");
        let mut ch = Channel::new("ch_001", "order-test");
        for i in 0..100 {
            ch.append(text_msg(&format!("msg_{}", i)));
        }
        ch.persist(&dir).unwrap();

        let recovered = Channel::recover(&dir, "order-test").unwrap();
        assert_eq!(recovered.len(), 100);
        for i in 0..100 {
            assert_eq!(recovered.log[i].payload_text(), format!("msg_{}", i));
        }
        cleanup_tempdir(&dir);
    }

    /// Test 33: Recovery from non-existent file returns empty channel.
    #[test]
    fn test_empty_channel_recovery() {
        let dir = tempdir("test_nonexistent");
        let recovered = Channel::recover(&dir, "does-not-exist").unwrap();
        assert_eq!(recovered.len(), 0);
        cleanup_tempdir(&dir);
    }

    /// Test 34: Mixed content types persist and recover correctly.
    #[test]
    fn test_mixed_content_recovery() {
        let dir = tempdir("test_mixed");
        let mut ch = Channel::new("ch_001", "mixed-test");
        ch.append(text_msg("plain text"));
        ch.append(Message::json("agent", r#"{"key":"value"}"#));
        ch.append(Message::binary(
            "agent",
            "image/png",
            vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A],
        ));
        ch.persist(&dir).unwrap();

        let recovered = Channel::recover(&dir, "mixed-test").unwrap();
        assert_eq!(recovered.len(), 3);
        assert_eq!(recovered.log[0].content_type, "text/plain");
        assert_eq!(recovered.log[0].payload_text(), "plain text");
        assert_eq!(recovered.log[1].content_type, "application/json");
        assert_eq!(recovered.log[1].payload_text(), r#"{"key":"value"}"#);
        assert_eq!(recovered.log[2].content_type, "image/png");
        assert_eq!(
            recovered.log[2].payload,
            vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]
        );
        cleanup_tempdir(&dir);
    }

    /// Test 35: Truncated file recovery discards partial message.
    #[test]
    fn test_truncated_write_recovery() {
        let dir = tempdir("test_truncated");
        let mut ch = Channel::new("ch_001", "truncated-test");
        ch.append(text_msg("complete message 1"));
        ch.append(text_msg("complete message 2"));
        ch.persist(&dir).unwrap();

        // Simulate crash by appending a partial message (truncated header)
        let file_path = Path::new(&dir).join("truncated-test.log");
        let mut data = fs::read(&file_path).unwrap();
        // Append a partial header (only 10 of 17 bytes)
        data.extend_from_slice(&b"ACTM\x01\x00\x00"[..]);
        fs::write(&file_path, &data).unwrap();

        let recovered = Channel::recover(&dir, "truncated-test").unwrap();
        assert_eq!(recovered.len(), 2); // Only complete messages recovered
        assert_eq!(recovered.log[0].payload_text(), "complete message 1");
        assert_eq!(recovered.log[1].payload_text(), "complete message 2");
        cleanup_tempdir(&dir);
    }

    /// Test 36: Mixed version recovery (simulated).
    ///
    /// Since we only support v1 currently, this test writes v1 messages and
    /// verifies they can be read. When v2 is added, this test would be updated
    /// to write a mix of v1 and v2 messages.
    #[test]
    fn test_mixed_version_recovery() {
        let dir = tempdir("test_mixed_version");
        let mut ch = Channel::new("ch_001", "version-test");
        ch.append(text_msg("v1 message 1"));
        ch.append(text_msg("v1 message 2"));
        ch.persist(&dir).unwrap();

        let recovered = Channel::recover(&dir, "version-test").unwrap();
        assert_eq!(recovered.len(), 2);
        // All messages are v1 -- they round-trip correctly
        assert_eq!(recovered.log[0].payload_text(), "v1 message 1");
        assert_eq!(recovered.log[1].payload_text(), "v1 message 2");
        cleanup_tempdir(&dir);
    }

    // -----------------------------------------------------------------------
    // Temp directory helpers
    // -----------------------------------------------------------------------

    /// Create a unique temporary directory for tests.
    fn tempdir(prefix: &str) -> String {
        let dir = std::env::temp_dir().join(format!(
            "actor_test_{}_{:x}",
            prefix,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir.to_string_lossy().to_string()
    }

    /// Remove a temporary directory.
    fn cleanup_tempdir(dir: &str) {
        let _ = fs::remove_dir_all(dir);
    }
}
