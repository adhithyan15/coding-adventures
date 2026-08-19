//! Length-prefixed framing over any `Read`/`Write` byte stream.
//!
//! A frame is `[4-byte big-endian length][payload]`. This module knows
//! nothing about what a payload means — that is
//! `coding_adventures_vault_pm_agent_protocol`'s job — it only guarantees
//! that a caller never allocates a buffer larger than the ceiling it was
//! given, and that the length prefix is checked *before* any payload byte is
//! read.

use coding_adventures_zeroize::Zeroizing;
use std::io::{self, Read, Write};

/// Byte width of the length prefix.
const LENGTH_PREFIX_BYTES: usize = 4;

/// Framing failed: the stream, or the frame itself.
#[derive(Debug)]
pub enum TransportError {
    /// The declared payload length exceeded the caller's ceiling.
    FrameTooLarge,
    /// The underlying stream failed or closed early.
    Io(io::Error),
}

impl From<io::Error> for TransportError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// Write one length-prefixed frame and flush it.
///
/// # Errors
///
/// Returns [`TransportError::FrameTooLarge`] if `payload` exceeds `max_len`,
/// checked before anything is written, and [`TransportError::Io`] on any
/// stream failure.
pub fn write_frame(
    writer: &mut impl Write,
    payload: &[u8],
    max_len: usize,
) -> Result<(), TransportError> {
    if payload.len() > max_len {
        return Err(TransportError::FrameTooLarge);
    }
    #[allow(clippy::cast_possible_truncation)]
    let length = payload.len() as u32;
    writer.write_all(&length.to_be_bytes())?;
    writer.write_all(payload)?;
    writer.flush()?;
    Ok(())
}

/// Read one length-prefixed frame, refusing to allocate past `max_len`.
///
/// The length prefix is read and checked against `max_len` *before* the
/// payload buffer is allocated, so a peer cannot make this function reserve
/// an unbounded amount of memory merely by sending a large length and
/// nothing else.
///
/// # Errors
///
/// Returns [`TransportError::FrameTooLarge`] if the declared length exceeds
/// `max_len`, and [`TransportError::Io`] on a short read, a closed
/// connection, or a stream failure (including a configured read timeout
/// elapsing, which the caller is expected to have set on the underlying
/// stream — this function performs no timing of its own).
///
/// The returned buffer is [`Zeroizing`] unconditionally. This module has no
/// way to know in advance whether a given frame carries a passphrase (an
/// `Unlock` request or a `Passphrase` response both do; every other message
/// does not) — the wire format is `vault-pm-agent-protocol`'s concern, not
/// this one's — so every frame this function reads is wiped on drop, at a
/// cost that is negligible at these sizes.
pub fn read_frame(
    reader: &mut impl Read,
    max_len: usize,
) -> Result<Zeroizing<Vec<u8>>, TransportError> {
    let mut length_bytes = [0_u8; LENGTH_PREFIX_BYTES];
    reader.read_exact(&mut length_bytes)?;
    let length = u32::from_be_bytes(length_bytes) as usize;
    if length > max_len {
        return Err(TransportError::FrameTooLarge);
    }
    let mut payload = Zeroizing::new(vec![0_u8; length]);
    reader.read_exact(&mut payload)?;
    Ok(payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn round_trips_a_frame() {
        let mut buffer = Vec::new();
        write_frame(&mut buffer, b"hello agent", 64).unwrap();
        let mut cursor = Cursor::new(buffer);
        assert_eq!(&*read_frame(&mut cursor, 64).unwrap(), b"hello agent");
    }

    #[test]
    fn refuses_to_write_past_the_ceiling() {
        let mut buffer = Vec::new();
        assert!(matches!(
            write_frame(&mut buffer, b"too long", 4),
            Err(TransportError::FrameTooLarge)
        ));
        assert!(buffer.is_empty(), "nothing is written on a rejected frame");
    }

    #[test]
    fn refuses_to_read_past_the_ceiling_without_allocating_the_payload() {
        let mut oversized = Vec::new();
        oversized.extend_from_slice(&100_000_u32.to_be_bytes());
        // No payload bytes follow: if this function tried to allocate first,
        // the subsequent `read_exact` would hang or fail differently. It must
        // fail on the length check alone.
        let mut cursor = Cursor::new(oversized);
        assert!(matches!(
            read_frame(&mut cursor, 64),
            Err(TransportError::FrameTooLarge)
        ));
    }

    #[test]
    fn a_truncated_frame_is_an_io_error() {
        let mut cursor = Cursor::new(vec![0, 0, 0, 5, b'h', b'i']);
        assert!(matches!(
            read_frame(&mut cursor, 64),
            Err(TransportError::Io(_))
        ));
    }
}
