//! Top-level message frame.  See spec MX03 §"Wire layout: top-level frame".
//!
//! Every message — request, response, or event — is wrapped in the
//! same outer envelope:
//!
//! ```text
//! u8           format_version       (= FRAME_VERSION)
//! u8           message_kind          (0=Request, 1=Response, 2=Event)
//! u64          correlation_id        (request id; responses echo it; events use 0)
//! uv64         payload_length
//! payload_length raw bytes (the encoded message)
//! ```
//!
//! Why a frame:
//!
//! - **Versioning at the byte level.**  A V2 reader that sees a V3
//!   frame errors immediately rather than misinterpreting the payload.
//! - **Multiplexing.**  A single transport carries multiple in-flight
//!   requests; correlation ids match responses to their requests.
//! - **Self-delimitation.**  `payload_length` lets a stream-oriented
//!   transport (TCP, Unix socket) frame messages without another layer.

/// Wire frame version.  V1 = 1.  Bumped only when the *frame layout*
/// changes incompatibly; payload version is independent.
pub const FRAME_VERSION: u8 = 1;

/// Whether a frame carries a request, response, or event.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum MessageKind {
    /// Runtime → executor.
    Request,
    /// Executor → runtime, in reply to a request.
    Response,
    /// Executor → runtime, unsolicited.
    Event,
}

impl MessageKind {
    /// Wire byte for this kind.  Stable.
    pub const fn wire_byte(self) -> u8 {
        match self {
            MessageKind::Request => 0,
            MessageKind::Response => 1,
            MessageKind::Event => 2,
        }
    }

    /// Decode from the wire byte.  Returns `None` for unknown values.
    pub const fn from_wire_byte(b: u8) -> Option<MessageKind> {
        match b {
            0 => Some(MessageKind::Request),
            1 => Some(MessageKind::Response),
            2 => Some(MessageKind::Event),
            _ => None,
        }
    }
}

/// A framed protocol message.  Carries the frame header plus the
/// already-encoded payload bytes.  Use [`MessageFrame::request`],
/// [`MessageFrame::response`], or [`MessageFrame::event`] to build
/// frames from typed messages.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct MessageFrame {
    /// Always [`FRAME_VERSION`] when produced by this crate.  Decoded
    /// frames carry whatever was on the wire (which the decoder
    /// validates).
    pub format_version: u8,
    /// Whether this is a request, response, or event.
    pub kind: MessageKind,
    /// Request id; responses echo it; events use 0.
    pub correlation_id: u64,
    /// Encoded payload bytes (the message variant).
    pub payload: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_kind_round_trip() {
        for k in [MessageKind::Request, MessageKind::Response, MessageKind::Event] {
            let b = k.wire_byte();
            assert_eq!(MessageKind::from_wire_byte(b), Some(k));
        }
        assert_eq!(MessageKind::from_wire_byte(99), None);
    }
}
