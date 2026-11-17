//! # Bounded TLS 1.3 handshake reassembly
//!
//! TLS record boundaries do not align with handshake-message boundaries. This
//! crate retains one message across already authenticated record-content
//! fragments in caller-owned storage. It stops exactly at the first completed
//! message so ownership and ordering of a coalesced tail remain with the caller.
//!
//! The caller is responsible for authenticating protected records and accepting
//! their inner handshake content type before passing bytes here. This crate does
//! not process records, sequence handshake states, update a transcript, parse a
//! message body, or perform cryptography or I/O.

#![forbid(unsafe_code)]

use core::fmt;
use tls13_handshake::{
    decode_tls13_handshake, decode_tls13_handshake_prefix, HandshakeErrorKind,
    Tls13HandshakeMessage, Tls13HandshakePolicy, TLS13_HANDSHAKE_HEADER_LEN,
};

/// Minimum caller-owned storage needed to retain a handshake header.
pub const TLS13_HANDSHAKE_REASSEMBLY_MIN_STORAGE_LEN: usize = TLS13_HANDSHAKE_HEADER_LEN;

/// Stable, payload-free failure categories for reassembler construction and use.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ReassemblyErrorKind {
    /// Caller-owned storage could not hold the four-byte handshake header.
    StorageTooSmall,
    /// A policy-valid declared message could not fit caller-owned storage.
    MessageTooLargeForStorage,
    /// The completed header or message failed handshake framing policy.
    HandshakeFraming(HandshakeErrorKind),
}

/// One redacted handshake-reassembly failure.
///
/// The value retains only a stable category. It never stores record content,
/// initialized storage, a wire type, a declared length, or a message body.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReassemblyError {
    kind: ReassemblyErrorKind,
}

impl ReassemblyError {
    const fn new(kind: ReassemblyErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable failure category.
    #[must_use]
    pub const fn kind(&self) -> ReassemblyErrorKind {
        self.kind
    }
}

impl fmt::Display for ReassemblyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "TLS 1.3 handshake reassembly error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for ReassemblyError {}

/// Result of consuming one authenticated record-content fragment.
///
/// `consumed` is always the exact prefix copied from the current input. A
/// complete result borrows the reassembler, preventing its storage from being
/// cleared or reused while the message is still borrowed.
#[derive(Eq, PartialEq)]
pub enum ReassemblyProgress<'message> {
    /// The current message is incomplete and the entire reported prefix is held.
    Incomplete {
        /// Bytes consumed from this call's input.
        consumed: usize,
    },
    /// One complete, policy-authorized handshake message is available.
    Complete {
        /// Bytes consumed from this call's input.
        consumed: usize,
        /// Exact header-plus-body bytes retained in caller-owned storage.
        message: Tls13HandshakeMessage<'message>,
    },
}

impl fmt::Debug for ReassemblyProgress<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Incomplete { .. } => formatter.write_str("ReassemblyProgress::Incomplete"),
            Self::Complete { .. } => formatter.write_str("ReassemblyProgress::Complete"),
        }
    }
}

impl ReassemblyProgress<'_> {
    /// Return the exact input prefix consumed by this push.
    #[must_use]
    pub const fn consumed(&self) -> usize {
        match self {
            Self::Incomplete { consumed } | Self::Complete { consumed, .. } => *consumed,
        }
    }

    /// Return whether this push completed one message.
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        matches!(self, Self::Complete { .. })
    }
}

/// Allocation-free retention for one TLS 1.3 handshake message.
///
/// The storage remains caller-owned. Only the initialized prefix is ever read,
/// and that prefix is cleared before it is abandoned or reused. The type has no
/// `Debug` implementation so accidental diagnostics cannot print buffered wire
/// bytes or length-derived state.
pub struct Tls13HandshakeReassembler<'storage> {
    policy: Tls13HandshakePolicy,
    storage: &'storage mut [u8],
    buffered_len: usize,
    target_len: Option<usize>,
    complete: bool,
}

impl<'storage> Tls13HandshakeReassembler<'storage> {
    /// Construct a reassembler over caller-owned fixed storage.
    pub fn new(
        policy: Tls13HandshakePolicy,
        storage: &'storage mut [u8],
    ) -> Result<Self, ReassemblyError> {
        if storage.len() < TLS13_HANDSHAKE_REASSEMBLY_MIN_STORAGE_LEN {
            return Err(ReassemblyError::new(ReassemblyErrorKind::StorageTooSmall));
        }
        Ok(Self {
            policy,
            storage,
            buffered_len: 0,
            target_len: None,
            complete: false,
        })
    }

    /// Consume bytes from one already authenticated handshake-content fragment.
    ///
    /// This method stops at the first complete message. The caller must retain
    /// `input[progress.consumed()..]` and submit that tail only after it has
    /// processed the returned message. Calling this method again after a
    /// complete result starts the next message and clears the previous encoding.
    ///
    /// A framing or capacity failure clears the buffered prefix and consumes the
    /// current message logically; a TLS state machine should treat that failure
    /// as fatal rather than retry attacker-controlled bytes under another policy.
    pub fn push_authenticated_record_content<'message>(
        &'message mut self,
        input: &[u8],
    ) -> Result<ReassemblyProgress<'message>, ReassemblyError> {
        if self.complete {
            self.clear_buffered();
        }

        let mut consumed = 0;
        if self.buffered_len < TLS13_HANDSHAKE_HEADER_LEN {
            let needed = TLS13_HANDSHAKE_HEADER_LEN - self.buffered_len;
            let copied = needed.min(input.len());
            let destination_end = self.buffered_len + copied;
            self.storage[self.buffered_len..destination_end].copy_from_slice(&input[..copied]);
            self.buffered_len = destination_end;
            consumed = copied;

            if self.buffered_len < TLS13_HANDSHAKE_HEADER_LEN {
                return Ok(ReassemblyProgress::Incomplete { consumed });
            }
        }

        if self.target_len.is_none() {
            if let Err(error) = decode_tls13_handshake_prefix(
                &self.policy,
                &self.storage[..TLS13_HANDSHAKE_HEADER_LEN],
            ) {
                if error.kind() != HandshakeErrorKind::TruncatedBody {
                    let kind = error.kind();
                    self.clear_buffered();
                    return Err(ReassemblyError::new(ReassemblyErrorKind::HandshakeFraming(
                        kind,
                    )));
                }
            }

            let body_len = decode_header_body_len(&self.storage[..TLS13_HANDSHAKE_HEADER_LEN]);
            let target_len = TLS13_HANDSHAKE_HEADER_LEN + body_len;
            if target_len > self.storage.len() {
                self.clear_buffered();
                return Err(ReassemblyError::new(
                    ReassemblyErrorKind::MessageTooLargeForStorage,
                ));
            }
            self.target_len = Some(target_len);
        }

        let target_len = self
            .target_len
            .expect("a complete validated header always establishes a target length");
        if self.buffered_len < target_len {
            let needed = target_len - self.buffered_len;
            let available = &input[consumed..];
            let copied = needed.min(available.len());
            let destination_end = self.buffered_len + copied;
            self.storage[self.buffered_len..destination_end].copy_from_slice(&available[..copied]);
            self.buffered_len = destination_end;
            consumed += copied;
        }

        if self.buffered_len < target_len {
            return Ok(ReassemblyProgress::Incomplete { consumed });
        }

        self.complete = true;
        let message = decode_tls13_handshake(&self.policy, &self.storage[..target_len])
            .expect("a policy-validated header plus its exact body must decode");
        Ok(ReassemblyProgress::Complete { consumed, message })
    }

    /// Abandon the current message and clear every initialized storage byte.
    pub fn reset(&mut self) {
        self.clear_buffered();
    }

    fn clear_buffered(&mut self) {
        self.storage[..self.buffered_len].fill(0);
        self.buffered_len = 0;
        self.target_len = None;
        self.complete = false;
    }
}

impl Drop for Tls13HandshakeReassembler<'_> {
    fn drop(&mut self) {
        self.clear_buffered();
    }
}

fn decode_header_body_len(header: &[u8]) -> usize {
    debug_assert!(header.len() >= TLS13_HANDSHAKE_HEADER_LEN);
    ((header[1] as usize) << 16) | ((header[2] as usize) << 8) | header[3] as usize
}
