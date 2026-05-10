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
const FRAME_COUNTER_32_LEN: usize = 4;
const FRAME_COUNTER_40_LEN: usize = 5;
const SUPERFRAME_SPEC_LEN: usize = 2;
const U8_FIELD_LEN: usize = 1;

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
    pub auxiliary_security_header: Option<AuxiliarySecurityHeader>,
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

        if self.frame_control.security_enabled {
            self.auxiliary_security_header
                .as_ref()
                .ok_or(MacFrameError::MissingAuxiliarySecurityHeader)?
                .encode(&mut out);
        } else if self.auxiliary_security_header.is_some() {
            return Err(MacFrameError::UnexpectedAuxiliarySecurityHeader);
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

        let auxiliary_security_header = if frame_control.security_enabled {
            Some(AuxiliarySecurityHeader::parse(&mut cursor)?)
        } else {
            None
        };

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
            auxiliary_security_header,
            payload,
            fcs,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SuperframeSpecification {
    raw: u16,
}

impl SuperframeSpecification {
    pub fn new(raw: u16) -> Self {
        Self { raw }
    }

    pub fn raw(self) -> u16 {
        self.raw
    }

    pub fn beacon_order(self) -> u8 {
        (self.raw & 0x000f) as u8
    }

    pub fn superframe_order(self) -> u8 {
        ((self.raw >> 4) & 0x000f) as u8
    }

    pub fn final_cap_slot(self) -> u8 {
        ((self.raw >> 8) & 0x000f) as u8
    }

    pub fn battery_life_extension(self) -> bool {
        self.raw & (1 << 12) != 0
    }

    pub fn pan_coordinator(self) -> bool {
        self.raw & (1 << 14) != 0
    }

    pub fn association_permit(self) -> bool {
        self.raw & (1 << 15) != 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GtsFields {
    pub descriptor_count: u8,
    pub permit: bool,
    pub directions: Option<u8>,
    pub descriptors: Vec<GtsDescriptor>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GtsDescriptor {
    pub short_address: u16,
    pub starting_slot: u8,
    pub length: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingAddressFields {
    pub short_addresses: Vec<u16>,
    pub extended_addresses: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeaconPayload {
    pub superframe: SuperframeSpecification,
    pub gts: GtsFields,
    pub pending_addresses: PendingAddressFields,
    pub payload: Vec<u8>,
}

impl BeaconPayload {
    pub fn parse(bytes: &[u8]) -> Result<Self, BeaconPayloadError> {
        let mut offset = 0;
        let raw_superframe = read_beacon_u16_le(bytes, &mut offset, "superframe specification")?;
        let superframe = SuperframeSpecification::new(raw_superframe);

        let gts_spec = read_beacon_u8(bytes, &mut offset, "GTS specification")?;
        let descriptor_count = gts_spec & 0b0000_0111;
        let permit = gts_spec & 0b1000_0000 != 0;
        let directions = if descriptor_count == 0 {
            None
        } else {
            Some(read_beacon_u8(bytes, &mut offset, "GTS directions")?)
        };

        let mut descriptors = Vec::with_capacity(descriptor_count as usize);
        for _ in 0..descriptor_count {
            let short_address = read_beacon_u16_le(bytes, &mut offset, "GTS descriptor")?;
            let slot_and_length = read_beacon_u8(bytes, &mut offset, "GTS descriptor")?;
            descriptors.push(GtsDescriptor {
                short_address,
                starting_slot: slot_and_length & 0x0f,
                length: (slot_and_length >> 4) & 0x0f,
            });
        }

        let pending_spec = read_beacon_u8(bytes, &mut offset, "pending address specification")?;
        let short_count = pending_spec & 0b0000_0111;
        let extended_count = (pending_spec >> 4) & 0b0000_0111;
        let mut short_addresses = Vec::with_capacity(short_count as usize);
        let mut extended_addresses = Vec::with_capacity(extended_count as usize);

        for _ in 0..short_count {
            short_addresses.push(read_beacon_u16_le(
                bytes,
                &mut offset,
                "pending short address",
            )?);
        }

        for _ in 0..extended_count {
            extended_addresses.push(read_beacon_u64_le(
                bytes,
                &mut offset,
                "pending extended address",
            )?);
        }

        Ok(Self {
            superframe,
            gts: GtsFields {
                descriptor_count,
                permit,
                directions,
                descriptors,
            },
            pending_addresses: PendingAddressFields {
                short_addresses,
                extended_addresses,
            },
            payload: bytes[offset..].to_vec(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanDescriptor {
    pub coordinator_pan_id: u16,
    pub coordinator_address: Address,
    pub channel: u8,
    pub channel_page: u8,
    pub link_quality: u8,
    pub beacon: BeaconPayload,
}

impl PanDescriptor {
    pub fn from_beacon_frame(
        frame: &MacFrame,
        channel: u8,
        channel_page: u8,
        link_quality: u8,
    ) -> Result<Self, BeaconPayloadError> {
        if frame.frame_control.frame_type != FrameType::Beacon {
            return Err(BeaconPayloadError::ExpectedBeaconFrame);
        }

        let coordinator_address = frame
            .source
            .ok_or(BeaconPayloadError::MissingBeaconSourceAddress)?;
        let coordinator_pan_id = frame
            .source_pan_id
            .or(frame.destination_pan_id)
            .ok_or(BeaconPayloadError::MissingBeaconPanId)?;
        let beacon = BeaconPayload::parse(&frame.payload)?;

        Ok(Self {
            coordinator_pan_id,
            coordinator_address,
            channel,
            channel_page,
            link_quality,
            beacon,
        })
    }

    pub fn association_permitted(&self) -> bool {
        self.beacon.superframe.association_permit()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanScanSummary {
    pub scanned_at_ms: u64,
    pub descriptors: Vec<PanDescriptor>,
}

impl PanScanSummary {
    pub fn new(scanned_at_ms: u64, descriptors: Vec<PanDescriptor>) -> Self {
        Self {
            scanned_at_ms,
            descriptors,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.descriptors.is_empty()
    }

    pub fn len(&self) -> usize {
        self.descriptors.len()
    }

    pub fn descriptors_for_channel(&self, channel: u8) -> Vec<&PanDescriptor> {
        self.descriptors
            .iter()
            .filter(|descriptor| descriptor.channel == channel)
            .collect()
    }

    pub fn association_candidates(&self) -> Vec<&PanDescriptor> {
        self.descriptors
            .iter()
            .filter(|descriptor| descriptor.association_permitted())
            .collect()
    }

    pub fn best_association_candidate(&self) -> Option<&PanDescriptor> {
        self.descriptors
            .iter()
            .filter(|descriptor| descriptor.association_permitted())
            .max_by_key(|descriptor| {
                (
                    descriptor.link_quality,
                    descriptor.beacon.superframe.pan_coordinator(),
                )
            })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BeaconPayloadError {
    ExpectedBeaconFrame,
    MissingBeaconSourceAddress,
    MissingBeaconPanId,
    TruncatedField {
        field: &'static str,
        needed: usize,
        remaining: usize,
    },
}

impl fmt::Display for BeaconPayloadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ExpectedBeaconFrame => write!(f, "expected an IEEE 802.15.4 beacon frame"),
            Self::MissingBeaconSourceAddress => {
                write!(f, "beacon frame is missing coordinator source address")
            }
            Self::MissingBeaconPanId => write!(f, "beacon frame is missing coordinator PAN id"),
            Self::TruncatedField {
                field,
                needed,
                remaining,
            } => write!(
                f,
                "truncated IEEE 802.15.4 beacon payload field {field}: needed {needed} bytes, had {remaining}"
            ),
        }
    }
}

impl std::error::Error for BeaconPayloadError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityLevel {
    None,
    Mic32,
    Mic64,
    Mic128,
    Enc,
    EncMic32,
    EncMic64,
    EncMic128,
}

impl SecurityLevel {
    fn from_bits(bits: u8) -> Self {
        match bits {
            0 => Self::None,
            1 => Self::Mic32,
            2 => Self::Mic64,
            3 => Self::Mic128,
            4 => Self::Enc,
            5 => Self::EncMic32,
            6 => Self::EncMic64,
            _ => Self::EncMic128,
        }
    }

    fn bits(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Mic32 => 1,
            Self::Mic64 => 2,
            Self::Mic128 => 3,
            Self::Enc => 4,
            Self::EncMic32 => 5,
            Self::EncMic64 => 6,
            Self::EncMic128 => 7,
        }
    }

    pub fn encrypts(self) -> bool {
        matches!(
            self,
            Self::Enc | Self::EncMic32 | Self::EncMic64 | Self::EncMic128
        )
    }

    pub fn mic_len(self) -> usize {
        match self {
            Self::None | Self::Enc => 0,
            Self::Mic32 | Self::EncMic32 => 4,
            Self::Mic64 | Self::EncMic64 => 8,
            Self::Mic128 | Self::EncMic128 => 16,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyIdentifierMode {
    Implicit,
    KeyIndex,
    KeySource4,
    KeySource8,
}

impl KeyIdentifierMode {
    fn from_bits(bits: u8) -> Self {
        match bits {
            0 => Self::Implicit,
            1 => Self::KeyIndex,
            2 => Self::KeySource4,
            _ => Self::KeySource8,
        }
    }

    fn bits(self) -> u8 {
        match self {
            Self::Implicit => 0,
            Self::KeyIndex => 1,
            Self::KeySource4 => 2,
            Self::KeySource8 => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SecurityControl {
    pub security_level: SecurityLevel,
    pub key_identifier_mode: KeyIdentifierMode,
    pub frame_counter_suppression: bool,
    pub frame_counter_size_5: bool,
}

impl SecurityControl {
    pub fn parse(raw: u8) -> Self {
        Self {
            security_level: SecurityLevel::from_bits(raw & 0b111),
            key_identifier_mode: KeyIdentifierMode::from_bits((raw >> 3) & 0b11),
            frame_counter_suppression: raw & (1 << 5) != 0,
            frame_counter_size_5: raw & (1 << 6) != 0,
        }
    }

    pub fn encode(self) -> u8 {
        let mut raw = self.security_level.bits();
        raw |= self.key_identifier_mode.bits() << 3;
        raw |= (self.frame_counter_suppression as u8) << 5;
        raw |= (self.frame_counter_size_5 as u8) << 6;
        raw
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameCounter {
    Counter32(u32),
    Counter40(u64),
}

impl FrameCounter {
    pub fn value(self) -> u64 {
        match self {
            Self::Counter32(value) => value as u64,
            Self::Counter40(value) => value,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyIdentifier {
    Implicit,
    KeyIndex(u8),
    KeySource4 { source: [u8; 4], index: u8 },
    KeySource8 { source: [u8; 8], index: u8 },
}

impl KeyIdentifier {
    pub fn mode(self) -> KeyIdentifierMode {
        match self {
            Self::Implicit => KeyIdentifierMode::Implicit,
            Self::KeyIndex(_) => KeyIdentifierMode::KeyIndex,
            Self::KeySource4 { .. } => KeyIdentifierMode::KeySource4,
            Self::KeySource8 { .. } => KeyIdentifierMode::KeySource8,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuxiliarySecurityHeader {
    pub security_control: SecurityControl,
    pub frame_counter: Option<FrameCounter>,
    pub key_identifier: KeyIdentifier,
}

impl AuxiliarySecurityHeader {
    fn parse(cursor: &mut Cursor<'_>) -> Result<Self, MacFrameError> {
        let security_control = SecurityControl::parse(cursor.read_u8()?);

        let frame_counter = if security_control.frame_counter_suppression {
            None
        } else if security_control.frame_counter_size_5 {
            Some(FrameCounter::Counter40(cursor.read_u40_le()?))
        } else {
            Some(FrameCounter::Counter32(cursor.read_u32_le()?))
        };

        let key_identifier = match security_control.key_identifier_mode {
            KeyIdentifierMode::Implicit => KeyIdentifier::Implicit,
            KeyIdentifierMode::KeyIndex => KeyIdentifier::KeyIndex(cursor.read_u8()?),
            KeyIdentifierMode::KeySource4 => {
                let source = cursor.read_array_4()?;
                let index = cursor.read_u8()?;
                KeyIdentifier::KeySource4 { source, index }
            }
            KeyIdentifierMode::KeySource8 => {
                let source = cursor.read_array_8()?;
                let index = cursor.read_u8()?;
                KeyIdentifier::KeySource8 { source, index }
            }
        };

        Ok(Self {
            security_control,
            frame_counter,
            key_identifier,
        })
    }

    fn encode(&self, out: &mut Vec<u8>) {
        out.push(self.security_control.encode());

        match self.frame_counter {
            Some(FrameCounter::Counter32(value)) => out.extend_from_slice(&value.to_le_bytes()),
            Some(FrameCounter::Counter40(value)) => {
                let bytes = value.to_le_bytes();
                out.extend_from_slice(&bytes[..FRAME_COUNTER_40_LEN]);
            }
            None => {}
        }

        match self.key_identifier {
            KeyIdentifier::Implicit => {}
            KeyIdentifier::KeyIndex(index) => out.push(index),
            KeyIdentifier::KeySource4 { source, index } => {
                out.extend_from_slice(&source);
                out.push(index);
            }
            KeyIdentifier::KeySource8 { source, index } => {
                out.extend_from_slice(&source);
                out.push(index);
            }
        }
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
    MissingAuxiliarySecurityHeader,
    UnexpectedAuxiliarySecurityHeader,
    MissingFrameCounter,
    UnexpectedFrameCounter,
    FrameCounterSizeMismatch,
    FrameCounterOutOfRange,
    KeyIdentifierModeMismatch,
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
            Self::MissingAuxiliarySecurityHeader => {
                write!(f, "missing auxiliary security header")
            }
            Self::UnexpectedAuxiliarySecurityHeader => {
                write!(f, "unexpected auxiliary security header")
            }
            Self::MissingFrameCounter => write!(f, "missing frame counter"),
            Self::UnexpectedFrameCounter => write!(f, "unexpected frame counter"),
            Self::FrameCounterSizeMismatch => write!(f, "frame counter size mismatch"),
            Self::FrameCounterOutOfRange => write!(f, "40-bit frame counter is out of range"),
            Self::KeyIdentifierModeMismatch => write!(f, "key identifier mode mismatch"),
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

    match (
        frame.frame_control.security_enabled,
        &frame.auxiliary_security_header,
    ) {
        (true, Some(header)) => validate_auxiliary_security_header(header)?,
        (true, None) => return Err(MacFrameError::MissingAuxiliarySecurityHeader),
        (false, Some(_)) => return Err(MacFrameError::UnexpectedAuxiliarySecurityHeader),
        (false, None) => {}
    }

    Ok(())
}

fn validate_auxiliary_security_header(
    header: &AuxiliarySecurityHeader,
) -> Result<(), MacFrameError> {
    if header.key_identifier.mode() != header.security_control.key_identifier_mode {
        return Err(MacFrameError::KeyIdentifierModeMismatch);
    }

    match (
        header.security_control.frame_counter_suppression,
        header.security_control.frame_counter_size_5,
        header.frame_counter,
    ) {
        (true, _, None) => Ok(()),
        (true, _, Some(_)) => Err(MacFrameError::UnexpectedFrameCounter),
        (false, false, Some(FrameCounter::Counter32(_))) => Ok(()),
        (false, true, Some(FrameCounter::Counter40(value))) if value <= 0x00ff_ffff_ffff => Ok(()),
        (false, _, None) => Err(MacFrameError::MissingFrameCounter),
        (false, false, Some(FrameCounter::Counter40(_))) => {
            Err(MacFrameError::FrameCounterSizeMismatch)
        }
        (false, true, Some(FrameCounter::Counter32(_))) => {
            Err(MacFrameError::FrameCounterSizeMismatch)
        }
        (false, true, Some(FrameCounter::Counter40(_))) => {
            Err(MacFrameError::FrameCounterOutOfRange)
        }
    }
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

fn read_beacon_u8(
    bytes: &[u8],
    offset: &mut usize,
    field: &'static str,
) -> Result<u8, BeaconPayloadError> {
    Ok(read_beacon_bytes(bytes, offset, U8_FIELD_LEN, field)?[0])
}

fn read_beacon_u16_le(
    bytes: &[u8],
    offset: &mut usize,
    field: &'static str,
) -> Result<u16, BeaconPayloadError> {
    let bytes = read_beacon_bytes(bytes, offset, SUPERFRAME_SPEC_LEN, field)?;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn read_beacon_u64_le(
    bytes: &[u8],
    offset: &mut usize,
    field: &'static str,
) -> Result<u64, BeaconPayloadError> {
    let bytes = read_beacon_bytes(bytes, offset, EXTENDED_ADDR_LEN, field)?;
    Ok(u64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ]))
}

fn read_beacon_bytes<'a>(
    bytes: &'a [u8],
    offset: &mut usize,
    len: usize,
    field: &'static str,
) -> Result<&'a [u8], BeaconPayloadError> {
    let remaining = bytes.len().saturating_sub(*offset);
    if remaining < len {
        return Err(BeaconPayloadError::TruncatedField {
            field,
            needed: len,
            remaining,
        });
    }

    let start = *offset;
    *offset += len;
    Ok(&bytes[start..*offset])
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

    fn read_u32_le(&mut self) -> Result<u32, MacFrameError> {
        let bytes = self.read_bytes(FRAME_COUNTER_32_LEN)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn read_u40_le(&mut self) -> Result<u64, MacFrameError> {
        let bytes = self.read_bytes(FRAME_COUNTER_40_LEN)?;
        Ok(u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], 0, 0, 0,
        ]))
    }

    fn read_u64_le(&mut self) -> Result<u64, MacFrameError> {
        let bytes = self.read_bytes(EXTENDED_ADDR_LEN)?;
        Ok(u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    fn read_array_4(&mut self) -> Result<[u8; 4], MacFrameError> {
        let bytes = self.read_bytes(4)?;
        Ok([bytes[0], bytes[1], bytes[2], bytes[3]])
    }

    fn read_array_8(&mut self) -> Result<[u8; 8], MacFrameError> {
        let bytes = self.read_bytes(8)?;
        Ok([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ])
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
        assert_eq!(frame.auxiliary_security_header, None);
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
            auxiliary_security_header: None,
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
    fn parses_auxiliary_security_header_with_key_index() {
        let bytes = [
            0x49, 0x98, // data frame plus security enabled
            0x07, // sequence number
            0x34, 0x12, // destination PAN id
            0x78, 0x56, // destination short address
            0xbc, 0x9a, // source short address
            0x0d, // EncMic32 plus key-index mode
            0x44, 0x33, 0x22, 0x11, // frame counter
            0x02, // key index
            0xaa, 0xbb, // encrypted payload bytes, not decrypted here
        ];

        let frame = MacFrame::parse_without_fcs(&bytes).unwrap();

        assert_eq!(
            frame.auxiliary_security_header,
            Some(AuxiliarySecurityHeader {
                security_control: SecurityControl {
                    security_level: SecurityLevel::EncMic32,
                    key_identifier_mode: KeyIdentifierMode::KeyIndex,
                    frame_counter_suppression: false,
                    frame_counter_size_5: false,
                },
                frame_counter: Some(FrameCounter::Counter32(0x1122_3344)),
                key_identifier: KeyIdentifier::KeyIndex(2),
            })
        );
        assert_eq!(frame.payload, vec![0xaa, 0xbb]);
    }

    #[test]
    fn encodes_auxiliary_security_header_with_key_source8() {
        let frame = MacFrame {
            frame_control: FrameControl {
                security_enabled: true,
                ..data_frame_control()
            },
            sequence_number: Some(7),
            destination_pan_id: Some(0x1234),
            destination: Some(Address::Short(0x5678)),
            source_pan_id: Some(0x1234),
            source: Some(Address::Short(0x9abc)),
            auxiliary_security_header: Some(AuxiliarySecurityHeader {
                security_control: SecurityControl {
                    security_level: SecurityLevel::EncMic64,
                    key_identifier_mode: KeyIdentifierMode::KeySource8,
                    frame_counter_suppression: false,
                    frame_counter_size_5: true,
                },
                frame_counter: Some(FrameCounter::Counter40(0x0001_0203_0405)),
                key_identifier: KeyIdentifier::KeySource8 {
                    source: [0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17],
                    index: 0x22,
                },
            }),
            payload: vec![0xaa],
            fcs: None,
        };

        assert_eq!(
            frame.encode().unwrap(),
            vec![
                0x49, 0x98, 0x07, 0x34, 0x12, 0x78, 0x56, 0xbc, 0x9a, 0x5e, 0x05, 0x04, 0x03, 0x02,
                0x01, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x22, 0xaa,
            ]
        );
    }

    #[test]
    fn rejects_security_enabled_encode_without_auxiliary_header() {
        let frame = MacFrame {
            frame_control: FrameControl {
                security_enabled: true,
                ..data_frame_control()
            },
            sequence_number: Some(7),
            destination_pan_id: Some(0x1234),
            destination: Some(Address::Short(0x5678)),
            source_pan_id: Some(0x1234),
            source: Some(Address::Short(0x9abc)),
            auxiliary_security_header: None,
            payload: vec![],
            fcs: None,
        };

        assert_eq!(
            frame.encode(),
            Err(MacFrameError::MissingAuxiliarySecurityHeader)
        );
    }

    #[test]
    fn security_level_reports_mic_length_and_encryption() {
        assert!(!SecurityLevel::Mic64.encrypts());
        assert_eq!(SecurityLevel::Mic64.mic_len(), 8);
        assert!(SecurityLevel::EncMic128.encrypts());
        assert_eq!(SecurityLevel::EncMic128.mic_len(), 16);
    }

    #[test]
    fn parses_beacon_payload_with_pending_addresses() {
        let bytes = [
            0xff,
            0xdf, // superframe spec: BO/SO/FCS=15, BLE, PAN coordinator, association permit
            0x80, // GTS permit, no descriptors
            0x11, // one short and one extended pending address
            0x34, 0x12, // pending short address
            0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, // pending extended address
            0xaa, 0xbb, // beacon payload
        ];

        let beacon = BeaconPayload::parse(&bytes).unwrap();

        assert_eq!(beacon.superframe.raw(), 0xdfff);
        assert_eq!(beacon.superframe.beacon_order(), 15);
        assert_eq!(beacon.superframe.superframe_order(), 15);
        assert_eq!(beacon.superframe.final_cap_slot(), 15);
        assert!(beacon.superframe.battery_life_extension());
        assert!(beacon.superframe.pan_coordinator());
        assert!(beacon.superframe.association_permit());
        assert_eq!(beacon.gts.descriptor_count, 0);
        assert!(beacon.gts.permit);
        assert_eq!(beacon.gts.directions, None);
        assert!(beacon.gts.descriptors.is_empty());
        assert_eq!(beacon.pending_addresses.short_addresses, vec![0x1234]);
        assert_eq!(
            beacon.pending_addresses.extended_addresses,
            vec![0x8877_6655_4433_2211]
        );
        assert_eq!(beacon.payload, vec![0xaa, 0xbb]);
    }

    #[test]
    fn parses_beacon_payload_with_gts_descriptors() {
        let bytes = [
            0xcf, 0x0f, // superframe spec: BO=15, SO=12, final CAP slot=15
            0x81, // GTS permit and one descriptor
            0x01, // first descriptor is receive direction
            0x67, 0x45, // GTS short address
            0x35, // starting slot 5, length 3
            0x00, // no pending addresses
        ];

        let beacon = BeaconPayload::parse(&bytes).unwrap();

        assert_eq!(beacon.superframe.beacon_order(), 15);
        assert_eq!(beacon.superframe.superframe_order(), 12);
        assert_eq!(beacon.gts.descriptor_count, 1);
        assert_eq!(beacon.gts.directions, Some(0x01));
        assert_eq!(
            beacon.gts.descriptors,
            vec![GtsDescriptor {
                short_address: 0x4567,
                starting_slot: 5,
                length: 3,
            }]
        );
        assert!(beacon.pending_addresses.short_addresses.is_empty());
        assert!(beacon.payload.is_empty());
    }

    #[test]
    fn derives_pan_descriptor_from_beacon_frame() {
        let frame = MacFrame {
            frame_control: FrameControl {
                frame_type: FrameType::Beacon,
                security_enabled: false,
                frame_pending: false,
                ack_request: false,
                pan_id_compression: false,
                sequence_number_suppression: false,
                information_elements_present: false,
                destination_address_mode: AddressMode::None,
                frame_version: FrameVersion::Ieee8021542006,
                source_address_mode: AddressMode::Extended,
            },
            sequence_number: Some(0x2a),
            destination_pan_id: None,
            destination: None,
            source_pan_id: Some(0x1234),
            source: Some(Address::Extended(0x8877_6655_4433_2211)),
            auxiliary_security_header: None,
            payload: vec![0xff, 0xdf, 0x00, 0x00],
            fcs: None,
        };

        let descriptor = PanDescriptor::from_beacon_frame(&frame, 15, 0, 244).unwrap();

        assert_eq!(descriptor.coordinator_pan_id, 0x1234);
        assert_eq!(
            descriptor.coordinator_address,
            Address::Extended(0x8877_6655_4433_2211)
        );
        assert_eq!(descriptor.channel, 15);
        assert_eq!(descriptor.channel_page, 0);
        assert_eq!(descriptor.link_quality, 244);
        assert!(descriptor.association_permitted());
    }

    #[test]
    fn rejects_truncated_beacon_payload() {
        let bytes = [
            0xff, 0xdf, // superframe spec
            0x00, // no GTS descriptors
            0x10, // one extended pending address, but no address bytes
        ];

        assert_eq!(
            BeaconPayload::parse(&bytes),
            Err(BeaconPayloadError::TruncatedField {
                field: "pending extended address",
                needed: 8,
                remaining: 0,
            })
        );
    }

    #[test]
    fn rejects_pan_descriptor_from_non_beacon_frame() {
        let frame = MacFrame {
            frame_control: data_frame_control(),
            sequence_number: Some(0x2a),
            destination_pan_id: Some(0x1234),
            destination: Some(Address::Short(0xffff)),
            source_pan_id: Some(0x1234),
            source: Some(Address::Short(0x0001)),
            auxiliary_security_header: None,
            payload: vec![],
            fcs: None,
        };

        assert_eq!(
            PanDescriptor::from_beacon_frame(&frame, 11, 0, 128),
            Err(BeaconPayloadError::ExpectedBeaconFrame)
        );
    }

    #[test]
    fn pan_scan_summary_filters_and_ranks_association_candidates() {
        let closed = pan_descriptor(0x1001, 11, 180, false);
        let weak = pan_descriptor(0x1002, 12, 80, true);
        let strong = pan_descriptor(0x1003, 12, 220, true);
        let summary = PanScanSummary::new(5_000, vec![closed, weak.clone(), strong.clone()]);

        assert_eq!(summary.scanned_at_ms, 5_000);
        assert_eq!(summary.len(), 3);
        assert!(!summary.is_empty());
        assert_eq!(summary.descriptors_for_channel(12).len(), 2);
        assert_eq!(summary.association_candidates(), vec![&weak, &strong]);
        assert_eq!(summary.best_association_candidate(), Some(&strong));
    }

    #[test]
    fn pan_scan_summary_returns_none_without_open_candidate() {
        let summary = PanScanSummary::new(5_000, vec![pan_descriptor(0x1001, 11, 240, false)]);

        assert_eq!(summary.association_candidates().len(), 0);
        assert_eq!(summary.best_association_candidate(), None);
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

    fn pan_descriptor(
        pan_id: u16,
        channel: u8,
        link_quality: u8,
        association_permit: bool,
    ) -> PanDescriptor {
        let association_bit = if association_permit { 1 << 15 } else { 0 };
        PanDescriptor {
            coordinator_pan_id: pan_id,
            coordinator_address: Address::Extended(0x8877_6655_4433_2211 + u64::from(pan_id)),
            channel,
            channel_page: 0,
            link_quality,
            beacon: BeaconPayload {
                superframe: SuperframeSpecification::new(0x4000 | association_bit),
                gts: GtsFields {
                    descriptor_count: 0,
                    permit: false,
                    directions: None,
                    descriptors: Vec::new(),
                },
                pending_addresses: PendingAddressFields {
                    short_addresses: Vec::new(),
                    extended_addresses: Vec::new(),
                },
                payload: Vec::new(),
            },
        }
    }
}
