//! Zigbee network-layer primitives built above IEEE 802.15.4.
//!
//! This crate starts with the NWK byte boundary: network addresses, frame
//! control bits, optional extended addresses, radius/sequence fields, and
//! payload extraction. Joining, routing tables, APS, ZDO, and ZCL live in later
//! crates.

#![forbid(unsafe_code)]

use std::fmt;

const NWK_FRAME_CONTROL_LEN: usize = 2;
const NWK_ADDR_LEN: usize = 2;
const IEEE_ADDR_LEN: usize = 8;
const NWK_BASE_HEADER_LEN: usize = NWK_FRAME_CONTROL_LEN + (NWK_ADDR_LEN * 2) + 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NetworkAddress(pub u16);

impl NetworkAddress {
    pub const COORDINATOR: Self = Self(0x0000);
    pub const BROADCAST_ALL_DEVICES: Self = Self(0xffff);
    pub const BROADCAST_RX_ON_WHEN_IDLE: Self = Self(0xfffd);

    pub fn is_broadcast(self) -> bool {
        self.0 >= 0xfff8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IeeeAddress(pub u64);

impl IeeeAddress {
    pub fn to_le_bytes(self) -> [u8; IEEE_ADDR_LEN] {
        self.0.to_le_bytes()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NwkFrameType {
    Data,
    Command,
    InterPan,
    Reserved,
}

impl NwkFrameType {
    fn from_bits(bits: u16) -> Self {
        match bits & 0b11 {
            0 => Self::Data,
            1 => Self::Command,
            3 => Self::InterPan,
            _ => Self::Reserved,
        }
    }

    fn bits(self) -> u16 {
        match self {
            Self::Data => 0,
            Self::Command => 1,
            Self::Reserved => 2,
            Self::InterPan => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscoverRoute {
    Suppress,
    Enable,
    Force,
    Reserved,
}

impl DiscoverRoute {
    fn from_bits(bits: u16) -> Self {
        match bits & 0b11 {
            0 => Self::Suppress,
            1 => Self::Enable,
            2 => Self::Force,
            _ => Self::Reserved,
        }
    }

    fn bits(self) -> u16 {
        match self {
            Self::Suppress => 0,
            Self::Enable => 1,
            Self::Force => 2,
            Self::Reserved => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NwkFrameControl {
    pub frame_type: NwkFrameType,
    pub protocol_version: u8,
    pub discover_route: DiscoverRoute,
    pub multicast: bool,
    pub security: bool,
    pub source_route: bool,
    pub extended_destination: bool,
    pub extended_source: bool,
    pub end_device_initiator: bool,
}

impl NwkFrameControl {
    pub fn parse(raw: u16) -> Self {
        Self {
            frame_type: NwkFrameType::from_bits(raw),
            protocol_version: ((raw >> 2) & 0b1111) as u8,
            discover_route: DiscoverRoute::from_bits(raw >> 6),
            multicast: raw & (1 << 8) != 0,
            security: raw & (1 << 9) != 0,
            source_route: raw & (1 << 10) != 0,
            extended_destination: raw & (1 << 11) != 0,
            extended_source: raw & (1 << 12) != 0,
            end_device_initiator: raw & (1 << 13) != 0,
        }
    }

    pub fn encode(self) -> u16 {
        let mut raw = self.frame_type.bits();
        raw |= ((self.protocol_version as u16) & 0b1111) << 2;
        raw |= self.discover_route.bits() << 6;
        raw |= (self.multicast as u16) << 8;
        raw |= (self.security as u16) << 9;
        raw |= (self.source_route as u16) << 10;
        raw |= (self.extended_destination as u16) << 11;
        raw |= (self.extended_source as u16) << 12;
        raw |= (self.end_device_initiator as u16) << 13;
        raw
    }

    pub fn zigbee_pro_2007(frame_type: NwkFrameType) -> Self {
        Self {
            frame_type,
            protocol_version: 2,
            discover_route: DiscoverRoute::Suppress,
            multicast: false,
            security: false,
            source_route: false,
            extended_destination: false,
            extended_source: false,
            end_device_initiator: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NwkFrame {
    pub frame_control: NwkFrameControl,
    pub destination: NetworkAddress,
    pub source: NetworkAddress,
    pub radius: u8,
    pub sequence_number: u8,
    pub destination_ieee: Option<IeeeAddress>,
    pub source_ieee: Option<IeeeAddress>,
    pub multicast_control: Option<u8>,
    pub payload: Vec<u8>,
}

impl NwkFrame {
    pub fn parse(bytes: &[u8]) -> Result<Self, NwkError> {
        if bytes.len() < NWK_BASE_HEADER_LEN {
            return Err(NwkError::Truncated {
                needed: NWK_BASE_HEADER_LEN,
                remaining: bytes.len(),
            });
        }

        let mut cursor = Cursor::new(bytes);
        let frame_control = NwkFrameControl::parse(cursor.read_u16_le()?);
        let destination = NetworkAddress(cursor.read_u16_le()?);
        let source = NetworkAddress(cursor.read_u16_le()?);
        let radius = cursor.read_u8()?;
        let sequence_number = cursor.read_u8()?;

        let destination_ieee = if frame_control.extended_destination {
            Some(IeeeAddress(cursor.read_u64_le()?))
        } else {
            None
        };
        let source_ieee = if frame_control.extended_source {
            Some(IeeeAddress(cursor.read_u64_le()?))
        } else {
            None
        };
        let multicast_control = if frame_control.multicast {
            Some(cursor.read_u8()?)
        } else {
            None
        };
        let payload = cursor.remaining_bytes().to_vec();

        Ok(Self {
            frame_control,
            destination,
            source,
            radius,
            sequence_number,
            destination_ieee,
            source_ieee,
            multicast_control,
            payload,
        })
    }

    pub fn encode(&self) -> Result<Vec<u8>, NwkError> {
        if self.frame_control.extended_destination != self.destination_ieee.is_some() {
            return Err(NwkError::ExtendedAddressMismatch {
                field: "destination",
            });
        }
        if self.frame_control.extended_source != self.source_ieee.is_some() {
            return Err(NwkError::ExtendedAddressMismatch { field: "source" });
        }
        if self.frame_control.multicast != self.multicast_control.is_some() {
            return Err(NwkError::MulticastControlMismatch);
        }

        let mut out = Vec::with_capacity(NWK_BASE_HEADER_LEN + self.payload.len());
        out.extend_from_slice(&self.frame_control.encode().to_le_bytes());
        out.extend_from_slice(&self.destination.0.to_le_bytes());
        out.extend_from_slice(&self.source.0.to_le_bytes());
        out.push(self.radius);
        out.push(self.sequence_number);
        if let Some(address) = self.destination_ieee {
            out.extend_from_slice(&address.to_le_bytes());
        }
        if let Some(address) = self.source_ieee {
            out.extend_from_slice(&address.to_le_bytes());
        }
        if let Some(multicast_control) = self.multicast_control {
            out.push(multicast_control);
        }
        out.extend_from_slice(&self.payload);
        Ok(out)
    }

    pub fn plain_data(
        destination: NetworkAddress,
        source: NetworkAddress,
        radius: u8,
        sequence_number: u8,
        payload: Vec<u8>,
    ) -> Self {
        Self {
            frame_control: NwkFrameControl::zigbee_pro_2007(NwkFrameType::Data),
            destination,
            source,
            radius,
            sequence_number,
            destination_ieee: None,
            source_ieee: None,
            multicast_control: None,
            payload,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NwkError {
    Truncated { needed: usize, remaining: usize },
    ExtendedAddressMismatch { field: &'static str },
    MulticastControlMismatch,
}

impl fmt::Display for NwkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated { needed, remaining } => write!(
                f,
                "truncated Zigbee NWK frame: needed {needed} bytes, had {remaining}"
            ),
            Self::ExtendedAddressMismatch { field } => {
                write!(
                    f,
                    "extended {field} address flag does not match address field"
                )
            }
            Self::MulticastControlMismatch => {
                write!(f, "multicast flag does not match multicast control field")
            }
        }
    }
}

impl std::error::Error for NwkError {}

struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    fn read_u8(&mut self) -> Result<u8, NwkError> {
        if self.pos >= self.bytes.len() {
            return Err(NwkError::Truncated {
                needed: 1,
                remaining: 0,
            });
        }
        let value = self.bytes[self.pos];
        self.pos += 1;
        Ok(value)
    }

    fn read_u16_le(&mut self) -> Result<u16, NwkError> {
        let bytes = self.read_array::<2>()?;
        Ok(u16::from_le_bytes(bytes))
    }

    fn read_u64_le(&mut self) -> Result<u64, NwkError> {
        let bytes = self.read_array::<8>()?;
        Ok(u64::from_le_bytes(bytes))
    }

    fn read_array<const N: usize>(&mut self) -> Result<[u8; N], NwkError> {
        let remaining = self.bytes.len().saturating_sub(self.pos);
        if remaining < N {
            return Err(NwkError::Truncated {
                needed: N,
                remaining,
            });
        }
        let mut out = [0u8; N];
        out.copy_from_slice(&self.bytes[self.pos..self.pos + N]);
        self.pos += N;
        Ok(out)
    }

    fn remaining_bytes(&self) -> &'a [u8] {
        &self.bytes[self.pos..]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_control_round_trips() {
        let control = NwkFrameControl {
            frame_type: NwkFrameType::Command,
            protocol_version: 2,
            discover_route: DiscoverRoute::Enable,
            multicast: true,
            security: true,
            source_route: false,
            extended_destination: true,
            extended_source: true,
            end_device_initiator: false,
        };

        assert_eq!(NwkFrameControl::parse(control.encode()), control);
    }

    #[test]
    fn plain_data_frame_round_trips() {
        let frame = NwkFrame::plain_data(
            NetworkAddress(0x1234),
            NetworkAddress(0x0000),
            30,
            7,
            vec![0x01, 0x02, 0x03],
        );
        let encoded = frame.encode().unwrap();

        assert_eq!(NwkFrame::parse(&encoded).unwrap(), frame);
    }

    #[test]
    fn extended_addresses_are_parsed_when_flags_are_set() {
        let mut control = NwkFrameControl::zigbee_pro_2007(NwkFrameType::Data);
        control.extended_destination = true;
        control.extended_source = true;
        let frame = NwkFrame {
            frame_control: control,
            destination: NetworkAddress(0xfffc),
            source: NetworkAddress(0x3344),
            radius: 10,
            sequence_number: 9,
            destination_ieee: Some(IeeeAddress(0x8877_6655_4433_2211)),
            source_ieee: Some(IeeeAddress(0x1100_ffee_ddcc_bbaa)),
            multicast_control: None,
            payload: vec![0xaa],
        };

        let parsed = NwkFrame::parse(&frame.encode().unwrap()).unwrap();

        assert_eq!(parsed.destination_ieee, frame.destination_ieee);
        assert_eq!(parsed.source_ieee, frame.source_ieee);
        assert_eq!(parsed.payload, vec![0xaa]);
    }

    #[test]
    fn rejects_extended_flag_without_address() {
        let mut frame =
            NwkFrame::plain_data(NetworkAddress(1), NetworkAddress(2), 1, 1, Vec::new());
        frame.frame_control.extended_source = true;

        assert_eq!(
            frame.encode(),
            Err(NwkError::ExtendedAddressMismatch { field: "source" })
        );
    }

    #[test]
    fn broadcast_addresses_are_identified() {
        assert!(NetworkAddress::BROADCAST_ALL_DEVICES.is_broadcast());
        assert!(!NetworkAddress(0x1234).is_broadcast());
    }
}
