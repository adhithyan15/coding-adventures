//! 6LoWPAN adaptation primitives for Thread over IEEE 802.15.4.
//!
//! The first useful Thread foundation is not an MLE state machine. It is the
//! small set of dispatch bytes, fragmentation headers, and IPHC header bits
//! that let higher layers classify and replay captured packets deterministically.

#![forbid(unsafe_code)]

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dispatch {
    Ipv6,
    LowpanHc1,
    Iphc,
    FragmentFirst,
    FragmentNext,
    Mesh,
    Broadcast,
    Unknown(u8),
}

impl Dispatch {
    pub fn parse(byte: u8) -> Self {
        match byte {
            0x41 => Self::Ipv6,
            0x42 => Self::LowpanHc1,
            value if value & 0b1110_0000 == 0b0110_0000 => Self::Iphc,
            value if value & 0b1111_1000 == 0b1100_0000 => Self::FragmentFirst,
            value if value & 0b1111_1000 == 0b1110_0000 => Self::FragmentNext,
            value if value & 0b1100_0000 == 0b1000_0000 => Self::Mesh,
            value if value & 0b1111_0000 == 0b0101_0000 => Self::Broadcast,
            other => Self::Unknown(other),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IphcTrafficClassFlowLabel {
    Inline,
    FlowLabelInline,
    TrafficClassInline,
    Elided,
}

impl IphcTrafficClassFlowLabel {
    fn from_bits(bits: u8) -> Self {
        match bits & 0b11 {
            0 => Self::Inline,
            1 => Self::FlowLabelInline,
            2 => Self::TrafficClassInline,
            _ => Self::Elided,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IphcHopLimit {
    Inline,
    One,
    SixtyFour,
    TwoHundredFiftyFive,
}

impl IphcHopLimit {
    fn from_bits(bits: u8) -> Self {
        match bits & 0b11 {
            0 => Self::Inline,
            1 => Self::One,
            2 => Self::SixtyFour,
            _ => Self::TwoHundredFiftyFive,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IphcAddressMode {
    Inline128,
    Compressed64,
    Compressed16,
    Elided,
}

impl IphcAddressMode {
    fn from_bits(bits: u8) -> Self {
        match bits & 0b11 {
            0 => Self::Inline128,
            1 => Self::Compressed64,
            2 => Self::Compressed16,
            _ => Self::Elided,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IphcEncoding {
    pub traffic_class_flow_label: IphcTrafficClassFlowLabel,
    pub next_header_compressed: bool,
    pub hop_limit: IphcHopLimit,
    pub context_identifier_extension: bool,
    pub source_address_compression: bool,
    pub source_address_mode: IphcAddressMode,
    pub multicast_destination: bool,
    pub destination_address_compression: bool,
    pub destination_address_mode: IphcAddressMode,
}

impl IphcEncoding {
    pub fn parse(first: u8, second: u8) -> Result<Self, SixlowpanError> {
        if Dispatch::parse(first) != Dispatch::Iphc {
            return Err(SixlowpanError::NotIphc(first));
        }

        Ok(Self {
            traffic_class_flow_label: IphcTrafficClassFlowLabel::from_bits(first >> 3),
            next_header_compressed: first & (1 << 2) != 0,
            hop_limit: IphcHopLimit::from_bits(first),
            context_identifier_extension: second & (1 << 7) != 0,
            source_address_compression: second & (1 << 6) != 0,
            source_address_mode: IphcAddressMode::from_bits(second >> 4),
            multicast_destination: second & (1 << 3) != 0,
            destination_address_compression: second & (1 << 2) != 0,
            destination_address_mode: IphcAddressMode::from_bits(second),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FragmentKind {
    First,
    Next,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FragmentHeader {
    pub kind: FragmentKind,
    pub datagram_size: u16,
    pub datagram_tag: u16,
    pub datagram_offset: Option<u8>,
}

impl FragmentHeader {
    pub fn first(datagram_size: u16, datagram_tag: u16) -> Result<Self, SixlowpanError> {
        validate_datagram_size(datagram_size)?;
        Ok(Self {
            kind: FragmentKind::First,
            datagram_size,
            datagram_tag,
            datagram_offset: None,
        })
    }

    pub fn next(
        datagram_size: u16,
        datagram_tag: u16,
        datagram_offset: u8,
    ) -> Result<Self, SixlowpanError> {
        validate_datagram_size(datagram_size)?;
        Ok(Self {
            kind: FragmentKind::Next,
            datagram_size,
            datagram_tag,
            datagram_offset: Some(datagram_offset),
        })
    }

    pub fn parse(bytes: &[u8]) -> Result<Self, SixlowpanError> {
        if bytes.len() < 4 {
            return Err(SixlowpanError::Truncated {
                needed: 4,
                remaining: bytes.len(),
            });
        }
        let dispatch = Dispatch::parse(bytes[0]);
        let datagram_size = (((bytes[0] & 0b0000_0111) as u16) << 8) | bytes[1] as u16;
        let datagram_tag = u16::from_be_bytes([bytes[2], bytes[3]]);
        match dispatch {
            Dispatch::FragmentFirst => Self::first(datagram_size, datagram_tag),
            Dispatch::FragmentNext => {
                if bytes.len() < 5 {
                    return Err(SixlowpanError::Truncated {
                        needed: 5,
                        remaining: bytes.len(),
                    });
                }
                Self::next(datagram_size, datagram_tag, bytes[4])
            }
            _ => Err(SixlowpanError::NotFragment(bytes[0])),
        }
    }

    pub fn encode(self) -> Vec<u8> {
        let mut out = Vec::with_capacity(match self.kind {
            FragmentKind::First => 4,
            FragmentKind::Next => 5,
        });
        let prefix = match self.kind {
            FragmentKind::First => 0b1100_0000,
            FragmentKind::Next => 0b1110_0000,
        };
        out.push(prefix | ((self.datagram_size >> 8) as u8 & 0b0000_0111));
        out.push(self.datagram_size as u8);
        out.extend_from_slice(&self.datagram_tag.to_be_bytes());
        if let Some(offset) = self.datagram_offset {
            out.push(offset);
        }
        out
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LowpanFrame {
    pub dispatch: Dispatch,
    pub payload: Vec<u8>,
}

impl LowpanFrame {
    pub fn parse(bytes: &[u8]) -> Result<Self, SixlowpanError> {
        let Some((&first, payload)) = bytes.split_first() else {
            return Err(SixlowpanError::Truncated {
                needed: 1,
                remaining: 0,
            });
        };
        Ok(Self {
            dispatch: Dispatch::parse(first),
            payload: payload.to_vec(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SixlowpanError {
    Truncated { needed: usize, remaining: usize },
    NotIphc(u8),
    NotFragment(u8),
    DatagramSizeTooLarge(u16),
}

impl fmt::Display for SixlowpanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated { needed, remaining } => write!(
                f,
                "truncated 6LoWPAN frame: needed {needed} bytes, had {remaining}"
            ),
            Self::NotIphc(value) => write!(f, "dispatch 0x{value:02x} is not LOWPAN_IPHC"),
            Self::NotFragment(value) => {
                write!(f, "dispatch 0x{value:02x} is not a 6LoWPAN fragment header")
            }
            Self::DatagramSizeTooLarge(value) => {
                write!(f, "6LoWPAN datagram size {value} exceeds 11-bit field")
            }
        }
    }
}

impl std::error::Error for SixlowpanError {}

fn validate_datagram_size(datagram_size: u16) -> Result<(), SixlowpanError> {
    if datagram_size > 0x07ff {
        return Err(SixlowpanError::DatagramSizeTooLarge(datagram_size));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_classifies_common_headers() {
        assert_eq!(Dispatch::parse(0x41), Dispatch::Ipv6);
        assert_eq!(Dispatch::parse(0x60), Dispatch::Iphc);
        assert_eq!(Dispatch::parse(0xc2), Dispatch::FragmentFirst);
        assert_eq!(Dispatch::parse(0xe2), Dispatch::FragmentNext);
        assert_eq!(Dispatch::parse(0x80), Dispatch::Mesh);
    }

    #[test]
    fn iphc_encoding_parses_first_two_header_bytes() {
        let encoding = IphcEncoding::parse(0b0111_1110, 0b1111_1111).unwrap();

        assert_eq!(
            encoding.traffic_class_flow_label,
            IphcTrafficClassFlowLabel::Elided
        );
        assert!(encoding.next_header_compressed);
        assert_eq!(encoding.hop_limit, IphcHopLimit::SixtyFour);
        assert!(encoding.context_identifier_extension);
        assert_eq!(encoding.source_address_mode, IphcAddressMode::Elided);
        assert!(encoding.multicast_destination);
        assert_eq!(encoding.destination_address_mode, IphcAddressMode::Elided);
    }

    #[test]
    fn fragment_headers_round_trip() {
        let first = FragmentHeader::first(1_280, 0x3344).unwrap();
        let next = FragmentHeader::next(1_280, 0x3344, 16).unwrap();

        assert_eq!(FragmentHeader::parse(&first.encode()).unwrap(), first);
        assert_eq!(FragmentHeader::parse(&next.encode()).unwrap(), next);
    }

    #[test]
    fn fragment_size_is_limited_to_eleven_bits() {
        assert_eq!(
            FragmentHeader::first(0x0800, 1),
            Err(SixlowpanError::DatagramSizeTooLarge(0x0800))
        );
    }

    #[test]
    fn lowpan_frame_keeps_payload_bytes() {
        let frame = LowpanFrame::parse(&[0x41, 0xaa, 0xbb]).unwrap();

        assert_eq!(frame.dispatch, Dispatch::Ipv6);
        assert_eq!(frame.payload, vec![0xaa, 0xbb]);
    }
}
