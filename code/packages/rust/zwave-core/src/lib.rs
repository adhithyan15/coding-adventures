//! Z-Wave identifier, region, and Serial API frame primitives.
//!
//! This crate does not try to be a controller yet. It gives later Z-Wave
//! packages a tested byte boundary for controller serial frames, node identity,
//! command class ids, and regional profile metadata.

#![forbid(unsafe_code)]

use std::fmt;

pub const SOF: u8 = 0x01;
pub const ACK: u8 = 0x06;
pub const NAK: u8 = 0x15;
pub const CAN: u8 = 0x18;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HomeId(pub u32);

impl HomeId {
    pub fn to_be_bytes(self) -> [u8; 4] {
        self.0.to_be_bytes()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NodeId {
    Classic(u8),
    LongRange(u16),
}

impl NodeId {
    pub fn classic(value: u8) -> Result<Self, ZWaveError> {
        if value == 0 || value > 232 {
            return Err(ZWaveError::InvalidClassicNodeId(value));
        }
        Ok(Self::Classic(value))
    }

    pub fn long_range(value: u16) -> Result<Self, ZWaveError> {
        if !(1..=4_000).contains(&value) {
            return Err(ZWaveError::InvalidLongRangeNodeId(value));
        }
        Ok(Self::LongRange(value))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionProfile {
    Europe,
    UnitedStates,
    AustraliaNewZealand,
    HongKong,
    India,
    Israel,
    Russia,
    China,
    Japan,
    Korea,
    UnitedStatesLongRange,
    EuropeLongRange,
}

impl RegionProfile {
    pub fn band_description(self) -> &'static str {
        match self {
            Self::Europe => "EU sub-GHz",
            Self::UnitedStates => "US sub-GHz",
            Self::AustraliaNewZealand => "ANZ sub-GHz",
            Self::HongKong => "Hong Kong sub-GHz",
            Self::India => "India sub-GHz",
            Self::Israel => "Israel sub-GHz",
            Self::Russia => "Russia sub-GHz",
            Self::China => "China sub-GHz",
            Self::Japan => "Japan sub-GHz",
            Self::Korea => "Korea sub-GHz",
            Self::UnitedStatesLongRange => "US Z-Wave Long Range",
            Self::EuropeLongRange => "EU Z-Wave Long Range",
        }
    }

    pub fn supports_long_range(self) -> bool {
        matches!(self, Self::UnitedStatesLongRange | Self::EuropeLongRange)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommandClassId(pub u16);

impl CommandClassId {
    pub const BASIC: Self = Self(0x20);
    pub const SWITCH_BINARY: Self = Self(0x25);
    pub const SWITCH_MULTILEVEL: Self = Self(0x26);
    pub const SENSOR_BINARY: Self = Self(0x30);
    pub const SENSOR_MULTILEVEL: Self = Self(0x31);
    pub const DOOR_LOCK: Self = Self(0x62);
    pub const SECURITY_2: Self = Self(0x9f);

    pub fn encoded_len(self) -> usize {
        if self.0 <= u8::MAX as u16 {
            1
        } else {
            2
        }
    }

    pub fn encode(self, out: &mut Vec<u8>) {
        if self.0 <= u8::MAX as u16 {
            out.push(self.0 as u8);
        } else {
            out.extend_from_slice(&self.0.to_be_bytes());
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandClassFrame {
    pub command_class_id: CommandClassId,
    pub command_id: u8,
    pub payload: Vec<u8>,
}

impl CommandClassFrame {
    pub fn new(command_class_id: CommandClassId, command_id: u8, payload: Vec<u8>) -> Self {
        Self {
            command_class_id,
            command_id,
            payload,
        }
    }

    pub fn parse(bytes: &[u8]) -> Result<Self, ZWaveError> {
        let (command_class_id, command_id_offset) = parse_command_class_id(bytes)?;
        let command_id = *bytes.get(command_id_offset).ok_or(ZWaveError::Truncated {
            needed: command_id_offset + 1,
            remaining: bytes.len(),
        })?;
        let payload = bytes[command_id_offset + 1..].to_vec();

        Ok(Self {
            command_class_id,
            command_id,
            payload,
        })
    }

    pub fn encode(&self) -> Result<Vec<u8>, ZWaveError> {
        let len = self
            .command_class_id
            .encoded_len()
            .checked_add(1)
            .and_then(|len| len.checked_add(self.payload.len()))
            .ok_or(ZWaveError::CommandPayloadTooLong(self.payload.len()))?;
        if len > u8::MAX as usize {
            return Err(ZWaveError::CommandPayloadTooLong(self.payload.len()));
        }

        let mut out = Vec::with_capacity(len);
        self.command_class_id.encode(&mut out);
        out.push(self.command_id);
        out.extend_from_slice(&self.payload);
        Ok(out)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SerialFrameType {
    Request,
    Response,
}

impl SerialFrameType {
    fn from_byte(byte: u8) -> Result<Self, ZWaveError> {
        match byte {
            0x00 => Ok(Self::Request),
            0x01 => Ok(Self::Response),
            other => Err(ZWaveError::InvalidFrameType(other)),
        }
    }

    fn as_byte(self) -> u8 {
        match self {
            Self::Request => 0x00,
            Self::Response => 0x01,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerialFrame {
    pub frame_type: SerialFrameType,
    pub function_id: u8,
    pub payload: Vec<u8>,
}

impl SerialFrame {
    pub fn new(frame_type: SerialFrameType, function_id: u8, payload: Vec<u8>) -> Self {
        Self {
            frame_type,
            function_id,
            payload,
        }
    }

    pub fn parse(bytes: &[u8]) -> Result<Self, ZWaveError> {
        if bytes.len() < 5 {
            return Err(ZWaveError::Truncated {
                needed: 5,
                remaining: bytes.len(),
            });
        }
        if bytes[0] != SOF {
            return Err(ZWaveError::MissingStartOfFrame(bytes[0]));
        }

        let len = bytes[1] as usize;
        if len < 3 {
            return Err(ZWaveError::InvalidLength(len));
        }
        let frame_len = len + 2;
        if bytes.len() < frame_len {
            return Err(ZWaveError::Truncated {
                needed: frame_len,
                remaining: bytes.len(),
            });
        }

        let checksum = bytes[frame_len - 1];
        let checksum_input = &bytes[1..frame_len - 1];
        let expected = serial_checksum(checksum_input);
        if checksum != expected {
            return Err(ZWaveError::ChecksumMismatch {
                expected,
                actual: checksum,
            });
        }

        let frame_type = SerialFrameType::from_byte(bytes[2])?;
        let function_id = bytes[3];
        let payload = bytes[4..frame_len - 1].to_vec();

        Ok(Self {
            frame_type,
            function_id,
            payload,
        })
    }

    pub fn encode(&self) -> Result<Vec<u8>, ZWaveError> {
        let len = self
            .payload
            .len()
            .checked_add(3)
            .ok_or(ZWaveError::PayloadTooLong(self.payload.len()))?;
        if len > u8::MAX as usize {
            return Err(ZWaveError::PayloadTooLong(self.payload.len()));
        }

        let mut out = Vec::with_capacity(len + 2);
        out.push(SOF);
        out.push(len as u8);
        out.push(self.frame_type.as_byte());
        out.push(self.function_id);
        out.extend_from_slice(&self.payload);
        let checksum = serial_checksum(&out[1..]);
        out.push(checksum);
        Ok(out)
    }
}

pub fn serial_checksum(bytes_after_sof_before_checksum: &[u8]) -> u8 {
    bytes_after_sof_before_checksum
        .iter()
        .fold(0xff, |acc, byte| acc ^ byte)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ZWaveError {
    InvalidClassicNodeId(u8),
    InvalidLongRangeNodeId(u16),
    MissingStartOfFrame(u8),
    InvalidLength(usize),
    InvalidFrameType(u8),
    Truncated { needed: usize, remaining: usize },
    PayloadTooLong(usize),
    CommandPayloadTooLong(usize),
    ChecksumMismatch { expected: u8, actual: u8 },
}

impl fmt::Display for ZWaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidClassicNodeId(value) => {
                write!(f, "invalid classic Z-Wave node id {value}")
            }
            Self::InvalidLongRangeNodeId(value) => {
                write!(f, "invalid Z-Wave Long Range node id {value}")
            }
            Self::MissingStartOfFrame(value) => {
                write!(f, "expected Z-Wave SOF 0x01, got 0x{value:02x}")
            }
            Self::InvalidLength(value) => write!(f, "invalid Z-Wave frame length {value}"),
            Self::InvalidFrameType(value) => write!(f, "invalid Z-Wave frame type 0x{value:02x}"),
            Self::Truncated { needed, remaining } => write!(
                f,
                "truncated Z-Wave frame: needed {needed} bytes, had {remaining}"
            ),
            Self::PayloadTooLong(len) => write!(f, "Z-Wave serial payload too long: {len}"),
            Self::CommandPayloadTooLong(len) => {
                write!(f, "Z-Wave command-class payload too long: {len}")
            }
            Self::ChecksumMismatch { expected, actual } => write!(
                f,
                "Z-Wave checksum mismatch: expected 0x{expected:02x}, got 0x{actual:02x}"
            ),
        }
    }
}

impl std::error::Error for ZWaveError {}

fn parse_command_class_id(bytes: &[u8]) -> Result<(CommandClassId, usize), ZWaveError> {
    let first = *bytes.first().ok_or(ZWaveError::Truncated {
        needed: 1,
        remaining: 0,
    })?;
    if first >= 0xf0 {
        let second = *bytes.get(1).ok_or(ZWaveError::Truncated {
            needed: 2,
            remaining: bytes.len(),
        })?;
        Ok((CommandClassId(u16::from_be_bytes([first, second])), 2))
    } else {
        Ok((CommandClassId(u16::from(first)), 1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_node_id_ranges() {
        assert_eq!(NodeId::classic(1).unwrap(), NodeId::Classic(1));
        assert_eq!(NodeId::classic(0), Err(ZWaveError::InvalidClassicNodeId(0)));
        assert_eq!(NodeId::long_range(4_000).unwrap(), NodeId::LongRange(4_000));
        assert_eq!(
            NodeId::long_range(4_001),
            Err(ZWaveError::InvalidLongRangeNodeId(4_001))
        );
    }

    #[test]
    fn serial_frame_round_trips_with_checksum() {
        let frame = SerialFrame::new(SerialFrameType::Request, 0x13, vec![0x02, 0x25, 0x01]);
        let encoded = frame.encode().unwrap();

        assert_eq!(encoded[0], SOF);
        assert_eq!(SerialFrame::parse(&encoded).unwrap(), frame);
    }

    #[test]
    fn checksum_mismatch_is_rejected() {
        let frame = SerialFrame::new(SerialFrameType::Response, 0x02, vec![0x01, 0x02]);
        let mut encoded = frame.encode().unwrap();
        let checksum_index = encoded.len() - 1;
        encoded[checksum_index] ^= 0x01;

        assert!(matches!(
            SerialFrame::parse(&encoded),
            Err(ZWaveError::ChecksumMismatch { .. })
        ));
    }

    #[test]
    fn region_profiles_mark_long_range_explicitly() {
        assert!(!RegionProfile::UnitedStates.supports_long_range());
        assert!(RegionProfile::UnitedStatesLongRange.supports_long_range());
        assert_eq!(
            RegionProfile::EuropeLongRange.band_description(),
            "EU Z-Wave Long Range"
        );
    }

    #[test]
    fn common_command_class_ids_are_stable() {
        assert_eq!(CommandClassId::SWITCH_BINARY.0, 0x25);
        assert_eq!(CommandClassId::SECURITY_2.0, 0x9f);
    }

    #[test]
    fn command_class_frame_round_trips_short_id() {
        let frame = CommandClassFrame::new(CommandClassId::SWITCH_BINARY, 0x01, vec![0xff]);
        let encoded = frame.encode().unwrap();

        assert_eq!(encoded, vec![0x25, 0x01, 0xff]);
        assert_eq!(CommandClassFrame::parse(&encoded).unwrap(), frame);
    }

    #[test]
    fn command_class_frame_round_trips_extended_id() {
        let frame = CommandClassFrame::new(CommandClassId(0xf100), 0x02, vec![0x01, 0x02]);
        let encoded = frame.encode().unwrap();

        assert_eq!(CommandClassId(0xf100).encoded_len(), 2);
        assert_eq!(encoded, vec![0xf1, 0x00, 0x02, 0x01, 0x02]);
        assert_eq!(CommandClassFrame::parse(&encoded).unwrap(), frame);
    }

    #[test]
    fn command_class_frame_rejects_truncated_bytes() {
        assert_eq!(
            CommandClassFrame::parse(&[0xf1]),
            Err(ZWaveError::Truncated {
                needed: 2,
                remaining: 1,
            })
        );
        assert_eq!(
            CommandClassFrame::parse(&[0x25]),
            Err(ZWaveError::Truncated {
                needed: 2,
                remaining: 1,
            })
        );
    }
}
