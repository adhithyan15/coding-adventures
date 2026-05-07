//! Zigbee Cluster Library frame, attribute, and D23 mapping primitives.
//!
//! ZCL sits above APS and below smart-home modeling. This crate owns cluster
//! ids, foundation command frames, typed attribute reports, and the first
//! capability/state-delta projection into `smart-home-core`.

#![forbid(unsafe_code)]

use smart_home_core::{Capability, CapabilityId, CapabilityMode, StateDelta, Value, ValueKind};
use std::fmt;
use zigbee_nwk::NetworkAddress;

pub const ZCL_READ_ATTRIBUTES_COMMAND_ID: u8 = 0x00;
pub const ZCL_REPORT_ATTRIBUTES_COMMAND_ID: u8 = 0x0a;
pub const ZCL_DEFAULT_RESPONSE_COMMAND_ID: u8 = 0x0b;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ZclClusterId(pub u16);

impl ZclClusterId {
    pub const BASIC: Self = Self(0x0000);
    pub const IDENTIFY: Self = Self(0x0003);
    pub const GROUPS: Self = Self(0x0004);
    pub const SCENES: Self = Self(0x0005);
    pub const ON_OFF: Self = Self(0x0006);
    pub const LEVEL_CONTROL: Self = Self(0x0008);
    pub const DOOR_LOCK: Self = Self(0x0101);
    pub const THERMOSTAT: Self = Self(0x0201);
    pub const COLOR_CONTROL: Self = Self(0x0300);
    pub const TEMPERATURE_MEASUREMENT: Self = Self(0x0402);
    pub const ILLUMINANCE_MEASUREMENT: Self = Self(0x0400);
    pub const OCCUPANCY_SENSING: Self = Self(0x0406);

    pub fn name(self) -> &'static str {
        match self {
            Self::BASIC => "basic",
            Self::IDENTIFY => "identify",
            Self::GROUPS => "groups",
            Self::SCENES => "scenes",
            Self::ON_OFF => "on_off",
            Self::LEVEL_CONTROL => "level_control",
            Self::DOOR_LOCK => "door_lock",
            Self::THERMOSTAT => "thermostat",
            Self::COLOR_CONTROL => "color_control",
            Self::TEMPERATURE_MEASUREMENT => "temperature_measurement",
            Self::ILLUMINANCE_MEASUREMENT => "illuminance_measurement",
            Self::OCCUPANCY_SENSING => "occupancy_sensing",
            _ => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ZclAttributeId(pub u16);

impl ZclAttributeId {
    pub const ON_OFF: Self = Self(0x0000);
    pub const CURRENT_LEVEL: Self = Self(0x0000);
    pub const LOCK_STATE: Self = Self(0x0000);
    pub const LOCAL_TEMPERATURE: Self = Self(0x0000);
    pub const OCCUPANCY: Self = Self(0x0000);
    pub const COLOR_TEMPERATURE_MIREK: Self = Self(0x0007);
    pub const MANUFACTURER_NAME: Self = Self(0x0004);
    pub const MODEL_IDENTIFIER: Self = Self(0x0005);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZclFrameType {
    Foundation,
    ClusterSpecific,
    Reserved(u8),
}

impl ZclFrameType {
    fn from_bits(bits: u8) -> Self {
        match bits & 0b11 {
            0 => Self::Foundation,
            1 => Self::ClusterSpecific,
            other => Self::Reserved(other),
        }
    }

    fn bits(self) -> u8 {
        match self {
            Self::Foundation => 0,
            Self::ClusterSpecific => 1,
            Self::Reserved(bits) => bits & 0b11,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZclDirection {
    ClientToServer,
    ServerToClient,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZclFrameControl {
    pub frame_type: ZclFrameType,
    pub manufacturer_specific: bool,
    pub direction: ZclDirection,
    pub disable_default_response: bool,
}

impl ZclFrameControl {
    pub fn parse(raw: u8) -> Self {
        Self {
            frame_type: ZclFrameType::from_bits(raw),
            manufacturer_specific: raw & (1 << 2) != 0,
            direction: if raw & (1 << 3) == 0 {
                ZclDirection::ClientToServer
            } else {
                ZclDirection::ServerToClient
            },
            disable_default_response: raw & (1 << 4) != 0,
        }
    }

    pub fn encode(self) -> u8 {
        let mut raw = self.frame_type.bits();
        raw |= (self.manufacturer_specific as u8) << 2;
        raw |= (matches!(self.direction, ZclDirection::ServerToClient) as u8) << 3;
        raw |= (self.disable_default_response as u8) << 4;
        raw
    }

    pub fn foundation_client_to_server() -> Self {
        Self {
            frame_type: ZclFrameType::Foundation,
            manufacturer_specific: false,
            direction: ZclDirection::ClientToServer,
            disable_default_response: true,
        }
    }

    pub fn cluster_client_to_server() -> Self {
        Self {
            frame_type: ZclFrameType::ClusterSpecific,
            manufacturer_specific: false,
            direction: ZclDirection::ClientToServer,
            disable_default_response: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZclFrame {
    pub frame_control: ZclFrameControl,
    pub manufacturer_code: Option<u16>,
    pub transaction_sequence_number: u8,
    pub command_id: u8,
    pub payload: Vec<u8>,
}

impl ZclFrame {
    pub fn parse(bytes: &[u8]) -> Result<Self, ZclError> {
        let mut cursor = Cursor::new(bytes);
        let frame_control = ZclFrameControl::parse(cursor.read_u8()?);
        let manufacturer_code = if frame_control.manufacturer_specific {
            Some(cursor.read_u16_le()?)
        } else {
            None
        };
        let transaction_sequence_number = cursor.read_u8()?;
        let command_id = cursor.read_u8()?;
        let payload = cursor.remaining_bytes().to_vec();
        Ok(Self {
            frame_control,
            manufacturer_code,
            transaction_sequence_number,
            command_id,
            payload,
        })
    }

    pub fn encode(&self) -> Result<Vec<u8>, ZclError> {
        if self.frame_control.manufacturer_specific != self.manufacturer_code.is_some() {
            return Err(ZclError::ManufacturerCodeMismatch);
        }
        let mut out = Vec::with_capacity(3 + self.payload.len());
        out.push(self.frame_control.encode());
        if let Some(code) = self.manufacturer_code {
            out.extend_from_slice(&code.to_le_bytes());
        }
        out.push(self.transaction_sequence_number);
        out.push(self.command_id);
        out.extend_from_slice(&self.payload);
        Ok(out)
    }

    pub fn foundation_command(
        transaction_sequence_number: u8,
        command_id: u8,
        payload: Vec<u8>,
    ) -> Self {
        Self {
            frame_control: ZclFrameControl::foundation_client_to_server(),
            manufacturer_code: None,
            transaction_sequence_number,
            command_id,
            payload,
        }
    }

    pub fn cluster_command(
        transaction_sequence_number: u8,
        command_id: u8,
        payload: Vec<u8>,
    ) -> Self {
        Self {
            frame_control: ZclFrameControl::cluster_client_to_server(),
            manufacturer_code: None,
            transaction_sequence_number,
            command_id,
            payload,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnOffCommand {
    Off,
    On,
    Toggle,
}

impl OnOffCommand {
    pub fn command_id(self) -> u8 {
        match self {
            Self::Off => 0x00,
            Self::On => 0x01,
            Self::Toggle => 0x02,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZigbeeEndpointRef {
    pub network_address: NetworkAddress,
    pub endpoint: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZclDataType {
    Bool,
    Bitmap8,
    U8,
    U16,
    U32,
    I16,
    Enum8,
    CharacterString,
    Unknown(u8),
}

impl ZclDataType {
    pub fn parse(raw: u8) -> Self {
        match raw {
            0x10 => Self::Bool,
            0x18 => Self::Bitmap8,
            0x20 => Self::U8,
            0x21 => Self::U16,
            0x23 => Self::U32,
            0x29 => Self::I16,
            0x30 => Self::Enum8,
            0x42 => Self::CharacterString,
            other => Self::Unknown(other),
        }
    }

    pub fn encode(self) -> u8 {
        match self {
            Self::Bool => 0x10,
            Self::Bitmap8 => 0x18,
            Self::U8 => 0x20,
            Self::U16 => 0x21,
            Self::U32 => 0x23,
            Self::I16 => 0x29,
            Self::Enum8 => 0x30,
            Self::CharacterString => 0x42,
            Self::Unknown(raw) => raw,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ZclValue {
    Bool(bool),
    Bitmap8(u8),
    U8(u8),
    U16(u16),
    U32(u32),
    I16(i16),
    Enum8(u8),
    CharacterString(String),
    Raw(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZclAttributeReport {
    pub cluster_id: ZclClusterId,
    pub attribute_id: ZclAttributeId,
    pub data_type: ZclDataType,
    pub value: ZclValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ZclError {
    Truncated { needed: usize, remaining: usize },
    ManufacturerCodeMismatch,
    InvalidString,
}

impl fmt::Display for ZclError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated { needed, remaining } => write!(
                f,
                "truncated Zigbee ZCL frame: needed {needed} bytes, had {remaining}"
            ),
            Self::ManufacturerCodeMismatch => {
                write!(f, "manufacturer-specific flag does not match code field")
            }
            Self::InvalidString => write!(f, "ZCL character string is not valid UTF-8"),
        }
    }
}

impl std::error::Error for ZclError {}

pub fn read_attributes_frame(
    transaction_sequence_number: u8,
    attribute_ids: &[ZclAttributeId],
) -> ZclFrame {
    let mut payload = Vec::with_capacity(attribute_ids.len() * 2);
    for attribute_id in attribute_ids {
        payload.extend_from_slice(&attribute_id.0.to_le_bytes());
    }
    ZclFrame::foundation_command(
        transaction_sequence_number,
        ZCL_READ_ATTRIBUTES_COMMAND_ID,
        payload,
    )
}

pub fn on_off_command_frame(transaction_sequence_number: u8, command: OnOffCommand) -> ZclFrame {
    ZclFrame::cluster_command(
        transaction_sequence_number,
        command.command_id(),
        Vec::new(),
    )
}

pub fn parse_attribute_reports(
    cluster_id: ZclClusterId,
    payload: &[u8],
) -> Result<Vec<ZclAttributeReport>, ZclError> {
    let mut cursor = Cursor::new(payload);
    let mut reports = Vec::new();
    while cursor.remaining_len() > 0 {
        let attribute_id = ZclAttributeId(cursor.read_u16_le()?);
        let data_type = ZclDataType::parse(cursor.read_u8()?);
        let value = read_zcl_value(&mut cursor, data_type)?;
        reports.push(ZclAttributeReport {
            cluster_id,
            attribute_id,
            data_type,
            value,
        });
    }
    Ok(reports)
}

pub fn capabilities_for_cluster(cluster_id: ZclClusterId) -> Vec<Capability> {
    match cluster_id {
        ZclClusterId::ON_OFF => vec![Capability::light_on_off()],
        ZclClusterId::LEVEL_CONTROL => vec![Capability::light_brightness()],
        ZclClusterId::COLOR_CONTROL => vec![Capability::light_color_temperature()],
        ZclClusterId::OCCUPANCY_SENSING => vec![Capability::sensor_occupancy()],
        ZclClusterId::DOOR_LOCK => vec![Capability::new(
            CapabilityId::trusted("lock.state"),
            CapabilityMode::ObserveAndCommand,
            ValueKind::Text,
        )],
        ZclClusterId::THERMOSTAT => vec![Capability::new(
            CapabilityId::trusted("climate.setpoint"),
            CapabilityMode::ObserveAndCommand,
            ValueKind::Number,
        )],
        _ => Vec::new(),
    }
}

pub fn state_delta_for_report(report: &ZclAttributeReport) -> Option<StateDelta> {
    match (report.cluster_id, report.attribute_id, &report.value) {
        (ZclClusterId::ON_OFF, ZclAttributeId::ON_OFF, ZclValue::Bool(on)) => Some(StateDelta {
            capability_id: CapabilityId::trusted("light.on_off"),
            value: Value::Bool(*on),
        }),
        (ZclClusterId::LEVEL_CONTROL, ZclAttributeId::CURRENT_LEVEL, ZclValue::U8(level)) => {
            Some(StateDelta {
                capability_id: CapabilityId::trusted("light.brightness"),
                value: Value::Percentage(level_to_percentage(*level)),
            })
        }
        (
            ZclClusterId::COLOR_CONTROL,
            ZclAttributeId::COLOR_TEMPERATURE_MIREK,
            ZclValue::U16(mirek),
        ) => Some(StateDelta {
            capability_id: CapabilityId::trusted("light.color_temperature"),
            value: Value::Integer(i64::from(*mirek)),
        }),
        (ZclClusterId::OCCUPANCY_SENSING, ZclAttributeId::OCCUPANCY, ZclValue::Bitmap8(bits)) => {
            Some(StateDelta {
                capability_id: CapabilityId::trusted("sensor.occupancy"),
                value: Value::Bool(bits & 0x01 != 0),
            })
        }
        (ZclClusterId::DOOR_LOCK, ZclAttributeId::LOCK_STATE, ZclValue::Enum8(state)) => {
            Some(StateDelta {
                capability_id: CapabilityId::trusted("lock.state"),
                value: Value::Text(lock_state_name(*state).to_string()),
            })
        }
        _ => None,
    }
}

pub fn level_to_percentage(level: u8) -> u8 {
    ((u16::from(level) * 100 + 127) / 254).min(100) as u8
}

pub fn lock_state_name(value: u8) -> &'static str {
    match value {
        0x00 => "not_fully_locked",
        0x01 => "locked",
        0x02 => "unlocked",
        _ => "unknown",
    }
}

fn read_zcl_value(cursor: &mut Cursor<'_>, data_type: ZclDataType) -> Result<ZclValue, ZclError> {
    match data_type {
        ZclDataType::Bool => Ok(ZclValue::Bool(cursor.read_u8()? != 0)),
        ZclDataType::Bitmap8 => Ok(ZclValue::Bitmap8(cursor.read_u8()?)),
        ZclDataType::U8 => Ok(ZclValue::U8(cursor.read_u8()?)),
        ZclDataType::U16 => Ok(ZclValue::U16(cursor.read_u16_le()?)),
        ZclDataType::U32 => Ok(ZclValue::U32(cursor.read_u32_le()?)),
        ZclDataType::I16 => Ok(ZclValue::I16(cursor.read_i16_le()?)),
        ZclDataType::Enum8 => Ok(ZclValue::Enum8(cursor.read_u8()?)),
        ZclDataType::CharacterString => {
            let len = cursor.read_u8()? as usize;
            let bytes = cursor.read_bytes(len)?;
            let value = std::str::from_utf8(bytes).map_err(|_| ZclError::InvalidString)?;
            Ok(ZclValue::CharacterString(value.to_string()))
        }
        ZclDataType::Unknown(_) => Ok(ZclValue::Raw(cursor.remaining_bytes().to_vec())),
    }
}

struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    fn remaining_len(&self) -> usize {
        self.bytes.len().saturating_sub(self.pos)
    }

    fn remaining_bytes(&self) -> &'a [u8] {
        &self.bytes[self.pos..]
    }

    fn read_u8(&mut self) -> Result<u8, ZclError> {
        if self.remaining_len() < 1 {
            return Err(ZclError::Truncated {
                needed: 1,
                remaining: self.remaining_len(),
            });
        }
        let value = self.bytes[self.pos];
        self.pos += 1;
        Ok(value)
    }

    fn read_u16_le(&mut self) -> Result<u16, ZclError> {
        let bytes = self.read_array::<2>()?;
        Ok(u16::from_le_bytes(bytes))
    }

    fn read_i16_le(&mut self) -> Result<i16, ZclError> {
        let bytes = self.read_array::<2>()?;
        Ok(i16::from_le_bytes(bytes))
    }

    fn read_u32_le(&mut self) -> Result<u32, ZclError> {
        let bytes = self.read_array::<4>()?;
        Ok(u32::from_le_bytes(bytes))
    }

    fn read_bytes(&mut self, len: usize) -> Result<&'a [u8], ZclError> {
        if self.remaining_len() < len {
            return Err(ZclError::Truncated {
                needed: len,
                remaining: self.remaining_len(),
            });
        }
        let start = self.pos;
        self.pos += len;
        Ok(&self.bytes[start..self.pos])
    }

    fn read_array<const N: usize>(&mut self) -> Result<[u8; N], ZclError> {
        let bytes = self.read_bytes(N)?;
        let mut out = [0_u8; N];
        out.copy_from_slice(bytes);
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_control_round_trips_foundation_flags() {
        let control = ZclFrameControl {
            frame_type: ZclFrameType::Foundation,
            manufacturer_specific: true,
            direction: ZclDirection::ServerToClient,
            disable_default_response: true,
        };

        assert_eq!(ZclFrameControl::parse(control.encode()), control);
    }

    #[test]
    fn read_attributes_frame_encodes_foundation_command() {
        let frame = read_attributes_frame(
            0x22,
            &[
                ZclAttributeId::MANUFACTURER_NAME,
                ZclAttributeId::MODEL_IDENTIFIER,
            ],
        );

        assert_eq!(frame.command_id, ZCL_READ_ATTRIBUTES_COMMAND_ID);
        assert_eq!(
            frame.encode().unwrap(),
            vec![0x10, 0x22, 0x00, 0x04, 0x00, 0x05, 0x00]
        );
    }

    #[test]
    fn on_command_encodes_cluster_specific_frame() {
        let frame = on_off_command_frame(0x33, OnOffCommand::On);

        assert_eq!(frame.command_id, 0x01);
        assert_eq!(frame.encode().unwrap(), vec![0x11, 0x33, 0x01]);
    }

    #[test]
    fn parses_on_off_attribute_report_and_maps_to_state_delta() {
        let reports = parse_attribute_reports(
            ZclClusterId::ON_OFF,
            &[0x00, 0x00, ZclDataType::Bool.encode(), 0x01],
        )
        .unwrap();
        let delta = state_delta_for_report(&reports[0]).unwrap();

        assert_eq!(reports[0].value, ZclValue::Bool(true));
        assert_eq!(delta.capability_id, CapabilityId::trusted("light.on_off"));
        assert_eq!(delta.value, Value::Bool(true));
    }

    #[test]
    fn parses_character_string_attribute_report() {
        let reports = parse_attribute_reports(
            ZclClusterId::BASIC,
            &[
                0x04,
                0x00,
                ZclDataType::CharacterString.encode(),
                0x07,
                b'S',
                b'i',
                b'g',
                b'n',
                b'i',
                b'f',
                b'y',
            ],
        )
        .unwrap();

        assert_eq!(
            reports[0],
            ZclAttributeReport {
                cluster_id: ZclClusterId::BASIC,
                attribute_id: ZclAttributeId::MANUFACTURER_NAME,
                data_type: ZclDataType::CharacterString,
                value: ZclValue::CharacterString("Signify".to_string()),
            }
        );
    }

    #[test]
    fn maps_level_and_occupancy_reports_to_d23_deltas() {
        let level = ZclAttributeReport {
            cluster_id: ZclClusterId::LEVEL_CONTROL,
            attribute_id: ZclAttributeId::CURRENT_LEVEL,
            data_type: ZclDataType::U8,
            value: ZclValue::U8(254),
        };
        let occupancy = ZclAttributeReport {
            cluster_id: ZclClusterId::OCCUPANCY_SENSING,
            attribute_id: ZclAttributeId::OCCUPANCY,
            data_type: ZclDataType::Bitmap8,
            value: ZclValue::Bitmap8(0x01),
        };

        assert_eq!(
            state_delta_for_report(&level).unwrap().value,
            Value::Percentage(100)
        );
        assert_eq!(
            state_delta_for_report(&occupancy).unwrap().value,
            Value::Bool(true)
        );
    }

    #[test]
    fn common_clusters_project_capabilities() {
        assert_eq!(
            capabilities_for_cluster(ZclClusterId::ON_OFF)[0].capability_id,
            CapabilityId::trusted("light.on_off")
        );
        assert_eq!(
            capabilities_for_cluster(ZclClusterId::DOOR_LOCK)[0].capability_id,
            CapabilityId::trusted("lock.state")
        );
        assert!(capabilities_for_cluster(ZclClusterId::BASIC).is_empty());
    }

    #[test]
    fn endpoint_refs_use_nwk_addresses() {
        let endpoint = ZigbeeEndpointRef {
            network_address: NetworkAddress(0x1234),
            endpoint: 11,
        };

        assert_eq!(endpoint.network_address, NetworkAddress(0x1234));
        assert_eq!(endpoint.endpoint, 11);
    }
}
