// lib.rs -- IEEE 802.15.4 MAC frame primitives
// ================================================================
//
// This package starts the smart-home radio work at the byte boundary. Zigbee
// and Thread both build on IEEE 802.15.4, so the first reusable primitive is a
// small, dependency-free MAC frame parser/encoder.

use std::fmt;

const SEQ_LEN: usize = 1;
const PAN_ID_LEN: usize = 2;
const SHORT_ADDR_LEN: usize = 2;
const EXTENDED_ADDR_LEN: usize = 8;
const FCS_LEN: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameType {
    Beacon,
    Data,
    Acknowledgment,
    MacCommand,
    Multipurpose,
    Fragment,
    Extended,
    Reserved(u8),
}

impl FrameType {
    fn from_bits(bits: u8) -> Self {
        match bits {
            0 => Self::Beacon,
            1 => Self::Data,
            2 => Self::Acknowledgment,
            3 => Self::MacCommand,
            5 => Self::Multipurpose,
            6 => Self::Fragment,
            7 => Self::Extended,
            other => Self::Reserved(other),
        }
    }

    fn bits(self) -> u16 {
        match self {
            Self::Beacon => 0,
            Self::Data => 1,
            Self::Acknowledgment => 2,
            Self::MacCommand => 3,
            Self::Reserved(bits) => bits as u16,
            Self::Multipurpose => 5,
            Self::Fragment => 6,
            Self::Extended => 7,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressMode {
    None,
    Reserved,
    Short,
    Extended,
}

impl AddressMode {
    fn from_bits(bits: u8) -> Self {
        match bits {
            0 => Self::None,
            2 => Self::Short,
            3 => Self::Extended,
            _ => Self::Reserved,
        }
    }

    fn bits(self) -> u16 {
        match self {
            Self::None => 0,
            Self::Reserved => 1,
            Self::Short => 2,
            Self::Extended => 3,
        }
    }

    pub fn encoded_len(self) -> usize {
        match self {
            Self::None => 0,
            Self::Reserved => 0,
            Self::Short => SHORT_ADDR_LEN,
            Self::Extended => EXTENDED_ADDR_LEN,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameVersion {
    Ieee8021542003,
    Ieee8021542006,
    Ieee8021542015,
    Reserved,
}

impl FrameVersion {
    fn from_bits(bits: u8) -> Self {
        match bits {
            0 => Self::Ieee8021542003,
            1 => Self::Ieee8021542006,
            2 => Self::Ieee8021542015,
            _ => Self::Reserved,
        }
    }

    fn bits(self) -> u16 {
        match self {
            Self::Ieee8021542003 => 0,
            Self::Ieee8021542006 => 1,
            Self::Ieee8021542015 => 2,
            Self::Reserved => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Address {
    Short(u16),
    Extended(u64),
}

impl Address {
    fn mode(self) -> AddressMode {
        match self {
            Self::Short(_) => AddressMode::Short,
            Self::Extended(_) => AddressMode::Extended,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameControl {
    pub frame_type: FrameType,
    pub security_enabled: bool,
    pub frame_pending: bool,
    pub ack_request: bool,
    pub pan_id_compression: bool,
    pub sequence_number_suppression: bool,
    pub information_elements_present: bool,
    pub destination_address_mode: AddressMode,
    pub frame_version: FrameVersion,
    pub source_address_mode: AddressMode,
}

impl FrameControl {
    pub fn parse(raw: u16) -> Self {
        Self {
            frame_type: FrameType::from_bits((raw & 0b111) as u8),
            security_enabled: raw & (1 << 3) != 0,
            frame_pending: raw & (1 << 4) != 0,
            ack_request: raw & (1 << 5) != 0,
            pan_id_compression: raw & (1 << 6) != 0,
            sequence_number_suppression: raw & (1 << 8) != 0,
            information_elements_present: raw & (1 << 9) != 0,
            destination_address_mode: AddressMode::from_bits(((raw >> 10) & 0b11) as u8),
            frame_version: FrameVersion::from_bits(((raw >> 12) & 0b11) as u8),
            source_address_mode: AddressMode::from_bits(((raw >> 14) & 0b11) as u8),
        }
    }

    pub fn encode(self) -> u16 {
        let mut raw = self.frame_type.bits();
        raw |= (self.security_enabled as u16) << 3;
        raw |= (self.frame_pending as u16) << 4;
        raw |= (self.ack_request as u16) << 5;
        raw |= (self.pan_id_compression as u16) << 6;
        raw |= (self.sequence_number_suppression as u16) << 8;
        raw |= (self.information_elements_present as u16) << 9;
        raw |= self.destination_address_mode.bits() << 10;
        raw |= self.frame_version.bits() << 12;
        raw |= self.source_address_mode.bits() << 14;
        raw
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MacFrame {
    pub frame_control: FrameControl,
    pub sequence_number: Option<u8>,
    pub destination_pan_id: Option<u16>,
    pub destination: Option<Address>,
    pub source_pan_id: Option<u16>,
    pub source: Option<Address>,
    pub payload: Vec<u8>,
    pub fcs: Option<u16>,
}

impl MacFrame {
    pub fn parse_without_fcs(bytes: &[u8]) -> Result<Self, MacFrameError> {
        Self::parse(bytes, false)
    }

    pub fn parse_with_fcs(bytes: &[u8]) -> Result<Self, MacFrameError> {
        Self::parse(bytes, true)
    }

    pub fn encode(&self) -> Result<Vec<u8>, MacFrameError> {
        validate_modes(self)?;

        let mut out = Vec::new();
        out.extend_from_slice(&self.frame_control.encode().to_le_bytes());

        if !self.frame_control.sequence_number_suppression {
            out.push(
                self.sequence_number
                    .ok_or(MacFrameError::MissingSequenceNumber)?,
            );
        }

        if let Some(destination) = self.destination {
            out.extend_from_slice(
                &self
                    .destination_pan_id
                    .ok_or(MacFrameError::MissingDestinationPanId)?
                    .to_le_bytes(),
            );
            encode_address(destination, &mut out);
        }

        if let Some(source) = self.source {
            if !self.frame_control.pan_id_compression || self.destination_pan_id.is_none() {
                out.extend_from_slice(
                    &self
                        .source_pan_id
                        .ok_or(MacFrameError::MissingSourcePanId)?
                        .to_le_bytes(),
                );
            }
            encode_address(source, &mut out);
        }

        out.extend_from_slice(&self.payload);

        if let Some(fcs) = self.fcs {
            out.extend_from_slice(&fcs.to_le_bytes());
        }

        Ok(out)
    }

    fn parse(bytes: &[u8], has_fcs: bool) -> Result<Self, MacFrameError> {
        let mut cursor = Cursor::new(bytes);
        let raw_fcf = cursor.read_u16_le()?;
        let frame_control = FrameControl::parse(raw_fcf);

        reject_reserved_address_modes(frame_control)?;

        if frame_control.security_enabled {
            return Err(MacFrameError::SecurityHeaderNotYetSupported);
        }

        let sequence_number = if frame_control.sequence_number_suppression {
            None
        } else {
            Some(cursor.read_u8()?)
        };

        let (destination_pan_id, destination) =
            read_pan_and_address(&mut cursor, frame_control.destination_address_mode)?;

        let source_pan_id = if frame_control.source_address_mode == AddressMode::None {
            None
        } else if frame_control.pan_id_compression && destination_pan_id.is_some() {
            destination_pan_id
        } else {
            Some(cursor.read_u16_le()?)
        };

        let source = read_address(&mut cursor, frame_control.source_address_mode)?;

        let remaining = cursor.remaining();
        let payload_len = if has_fcs {
            remaining
                .checked_sub(FCS_LEN)
                .ok_or(MacFrameError::TruncatedFrame {
                    needed: FCS_LEN,
                    remaining,
                })?
        } else {
            remaining
        };

        let payload = cursor.read_bytes(payload_len)?.to_vec();
        let fcs = if has_fcs {
            Some(cursor.read_u16_le()?)
        } else {
            None
        };

        Ok(Self {
            frame_control,
            sequence_number,
            destination_pan_id,
            destination,
            source_pan_id,
            source,
            payload,
            fcs,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MacFrameError {
    TruncatedFrame { needed: usize, remaining: usize },
    ReservedAddressMode { field: &'static str },
    AddressModeMismatch { field: &'static str },
    MissingSequenceNumber,
    MissingDestinationPanId,
    MissingSourcePanId,
    SecurityHeaderNotYetSupported,
}

impl fmt::Display for MacFrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TruncatedFrame { needed, remaining } => write!(
                f,
                "truncated IEEE 802.15.4 frame: needed {needed} bytes, had {remaining}"
            ),
            Self::ReservedAddressMode { field } => {
                write!(f, "reserved address mode in {field} field")
            }
            Self::AddressModeMismatch { field } => {
                write!(f, "frame-control address mode does not match {field}")
            }
            Self::MissingSequenceNumber => write!(f, "missing sequence number"),
            Self::MissingDestinationPanId => write!(f, "missing destination PAN id"),
            Self::MissingSourcePanId => write!(f, "missing source PAN id"),
            Self::SecurityHeaderNotYetSupported => {
                write!(
                    f,
                    "auxiliary security header parsing is not implemented yet"
                )
            }
        }
    }
}

impl std::error::Error for MacFrameError {}

fn reject_reserved_address_modes(frame_control: FrameControl) -> Result<(), MacFrameError> {
    if frame_control.destination_address_mode == AddressMode::Reserved {
        return Err(MacFrameError::ReservedAddressMode {
            field: "destination",
        });
    }
    if frame_control.source_address_mode == AddressMode::Reserved {
        return Err(MacFrameError::ReservedAddressMode { field: "source" });
    }
    Ok(())
}

fn validate_modes(frame: &MacFrame) -> Result<(), MacFrameError> {
    if frame.frame_control.destination_address_mode == AddressMode::Reserved {
        return Err(MacFrameError::ReservedAddressMode {
            field: "destination",
        });
    }
    if frame.frame_control.source_address_mode == AddressMode::Reserved {
        return Err(MacFrameError::ReservedAddressMode { field: "source" });
    }

    let destination_mode = frame
        .destination
        .map(Address::mode)
        .unwrap_or(AddressMode::None);
    if destination_mode != frame.frame_control.destination_address_mode {
        return Err(MacFrameError::AddressModeMismatch {
            field: "destination",
        });
    }

    let source_mode = frame.source.map(Address::mode).unwrap_or(AddressMode::None);
    if source_mode != frame.frame_control.source_address_mode {
        return Err(MacFrameError::AddressModeMismatch { field: "source" });
    }

    Ok(())
}

fn read_pan_and_address(
    cursor: &mut Cursor<'_>,
    mode: AddressMode,
) -> Result<(Option<u16>, Option<Address>), MacFrameError> {
    if mode == AddressMode::None {
        return Ok((None, None));
    }

    let pan_id = cursor.read_u16_le()?;
    let address = read_address(cursor, mode)?;
    Ok((Some(pan_id), address))
}

fn read_address(
    cursor: &mut Cursor<'_>,
    mode: AddressMode,
) -> Result<Option<Address>, MacFrameError> {
    match mode {
        AddressMode::None => Ok(None),
        AddressMode::Reserved => Err(MacFrameError::ReservedAddressMode { field: "address" }),
        AddressMode::Short => Ok(Some(Address::Short(cursor.read_u16_le()?))),
        AddressMode::Extended => Ok(Some(Address::Extended(cursor.read_u64_le()?))),
    }
}

fn encode_address(address: Address, out: &mut Vec<u8>) {
    match address {
        Address::Short(value) => out.extend_from_slice(&value.to_le_bytes()),
        Address::Extended(value) => out.extend_from_slice(&value.to_le_bytes()),
    }
}

struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.offset)
    }

    fn read_u8(&mut self) -> Result<u8, MacFrameError> {
        Ok(self.read_bytes(SEQ_LEN)?[0])
    }

    fn read_u16_le(&mut self) -> Result<u16, MacFrameError> {
        let bytes = self.read_bytes(PAN_ID_LEN)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    fn read_u64_le(&mut self) -> Result<u64, MacFrameError> {
        let bytes = self.read_bytes(EXTENDED_ADDR_LEN)?;
        Ok(u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    fn read_bytes(&mut self, len: usize) -> Result<&'a [u8], MacFrameError> {
        let remaining = self.remaining();
        if remaining < len {
            return Err(MacFrameError::TruncatedFrame {
                needed: len,
                remaining,
            });
        }

        let start = self.offset;
        self.offset += len;
        Ok(&self.bytes[start..start + len])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data_frame_control() -> FrameControl {
        FrameControl {
            frame_type: FrameType::Data,
            security_enabled: false,
            frame_pending: false,
            ack_request: false,
            pan_id_compression: true,
            sequence_number_suppression: false,
            information_elements_present: false,
            destination_address_mode: AddressMode::Short,
            frame_version: FrameVersion::Ieee8021542006,
            source_address_mode: AddressMode::Short,
        }
    }

    #[test]
    fn parses_short_address_data_frame_without_fcs() {
        let bytes = [
            0x41, 0x98, // frame control
            0x07, // sequence number
            0x34, 0x12, // destination PAN id
            0x78, 0x56, // destination short address
            0xbc, 0x9a, // source short address, PAN compressed
            0x01, 0x02, // payload
        ];

        let frame = MacFrame::parse_without_fcs(&bytes).unwrap();

        assert_eq!(frame.frame_control, data_frame_control());
        assert_eq!(frame.sequence_number, Some(7));
        assert_eq!(frame.destination_pan_id, Some(0x1234));
        assert_eq!(frame.destination, Some(Address::Short(0x5678)));
        assert_eq!(frame.source_pan_id, Some(0x1234));
        assert_eq!(frame.source, Some(Address::Short(0x9abc)));
        assert_eq!(frame.payload, vec![0x01, 0x02]);
        assert_eq!(frame.fcs, None);
    }

    #[test]
    fn encodes_short_address_data_frame_without_fcs() {
        let frame = MacFrame {
            frame_control: data_frame_control(),
            sequence_number: Some(7),
            destination_pan_id: Some(0x1234),
            destination: Some(Address::Short(0x5678)),
            source_pan_id: Some(0x1234),
            source: Some(Address::Short(0x9abc)),
            payload: vec![0x01, 0x02],
            fcs: None,
        };

        assert_eq!(
            frame.encode().unwrap(),
            vec![0x41, 0x98, 0x07, 0x34, 0x12, 0x78, 0x56, 0xbc, 0x9a, 0x01, 0x02]
        );
    }

    #[test]
    fn parses_ack_frame() {
        let bytes = [0x02, 0x00, 0x2a];

        let frame = MacFrame::parse_without_fcs(&bytes).unwrap();

        assert_eq!(frame.frame_control.frame_type, FrameType::Acknowledgment);
        assert_eq!(frame.sequence_number, Some(0x2a));
        assert_eq!(frame.destination, None);
        assert_eq!(frame.source, None);
        assert!(frame.payload.is_empty());
    }

    #[test]
    fn parses_frame_with_fcs() {
        let bytes = [0x02, 0x00, 0x2a, 0xef, 0xbe];

        let frame = MacFrame::parse_with_fcs(&bytes).unwrap();

        assert_eq!(frame.sequence_number, Some(0x2a));
        assert_eq!(frame.fcs, Some(0xbeef));
        assert!(frame.payload.is_empty());
    }

    #[test]
    fn supports_sequence_number_suppression() {
        let control = FrameControl {
            sequence_number_suppression: true,
            ..data_frame_control()
        };
        let bytes = [
            0x41, 0x99, // same data frame with sequence suppression bit set
            0x34, 0x12, 0x78, 0x56, 0xbc, 0x9a,
        ];

        let frame = MacFrame::parse_without_fcs(&bytes).unwrap();

        assert_eq!(frame.frame_control, control);
        assert_eq!(frame.sequence_number, None);
        assert!(frame.payload.is_empty());
    }

    #[test]
    fn rejects_reserved_address_mode() {
        let bytes = [
            0x01, 0x04, // data frame with reserved destination address mode
            0x07,
        ];

        assert_eq!(
            MacFrame::parse_without_fcs(&bytes),
            Err(MacFrameError::ReservedAddressMode {
                field: "destination"
            })
        );
    }

    #[test]
    fn rejects_security_until_aux_header_is_implemented() {
        let bytes = [
            0x49, 0x98, // data frame plus security enabled
            0x07, 0x34, 0x12, 0x78, 0x56, 0xbc, 0x9a,
        ];

        assert_eq!(
            MacFrame::parse_without_fcs(&bytes),
            Err(MacFrameError::SecurityHeaderNotYetSupported)
        );
    }

    #[test]
    fn reports_truncated_frame() {
        let bytes = [0x41];

        assert_eq!(
            MacFrame::parse_without_fcs(&bytes),
            Err(MacFrameError::TruncatedFrame {
                needed: 2,
                remaining: 1
            })
        );
    }

    #[test]
    fn address_mode_knows_encoded_length() {
        assert_eq!(AddressMode::None.encoded_len(), 0);
        assert_eq!(AddressMode::Short.encoded_len(), 2);
        assert_eq!(AddressMode::Extended.encoded_len(), 8);
    }
}
