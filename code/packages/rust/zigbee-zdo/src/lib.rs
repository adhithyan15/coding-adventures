//! Zigbee Device Object descriptor and discovery primitives.
//!
//! ZDO is the interview layer: node descriptors, simple descriptors, active
//! endpoints, and management requests that let a coordinator learn what a node
//! is before ZCL maps clusters into normalized state.

#![forbid(unsafe_code)]

use smart_home_core::{
    BridgeId, Device, DeviceId, Health, Metadata, ProtocolFamily, ProtocolIdentifier,
};
use std::fmt;
use zigbee_aps::{ApsFrame, ClusterId, Endpoint, ProfileId};
use zigbee_nwk::{IeeeAddress, NetworkAddress};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ZdoClusterId(pub u16);

impl ZdoClusterId {
    pub const NODE_DESCRIPTOR_REQUEST: Self = Self(0x0002);
    pub const SIMPLE_DESCRIPTOR_REQUEST: Self = Self(0x0004);
    pub const ACTIVE_ENDPOINTS_REQUEST: Self = Self(0x0005);
    pub const BIND_REQUEST: Self = Self(0x0021);
    pub const MGMT_LQI_REQUEST: Self = Self(0x0031);

    pub const NODE_DESCRIPTOR_RESPONSE: Self = Self(0x8002);
    pub const SIMPLE_DESCRIPTOR_RESPONSE: Self = Self(0x8004);
    pub const ACTIVE_ENDPOINTS_RESPONSE: Self = Self(0x8005);
    pub const BIND_RESPONSE: Self = Self(0x8021);
    pub const MGMT_LQI_RESPONSE: Self = Self(0x8031);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZdoStatus {
    Success,
    InvalidRequest,
    DeviceNotFound,
    NotSupported,
    Timeout,
    NoDescriptor,
    Unknown(u8),
}

impl ZdoStatus {
    pub fn parse(raw: u8) -> Self {
        match raw {
            0x00 => Self::Success,
            0x80 => Self::InvalidRequest,
            0x81 => Self::DeviceNotFound,
            0x84 => Self::NotSupported,
            0x85 => Self::Timeout,
            0x89 => Self::NoDescriptor,
            other => Self::Unknown(other),
        }
    }

    pub fn is_success(self) -> bool {
        self == Self::Success
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicalType {
    Coordinator,
    Router,
    EndDevice,
    Unknown(u8),
}

impl LogicalType {
    pub fn parse(bits: u8) -> Self {
        match bits & 0b111 {
            0 => Self::Coordinator,
            1 => Self::Router,
            2 => Self::EndDevice,
            other => Self::Unknown(other),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Coordinator => "coordinator",
            Self::Router => "router",
            Self::EndDevice => "end_device",
            Self::Unknown(_) => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MacCapabilityFlags {
    pub alternate_pan_coordinator: bool,
    pub full_function_device: bool,
    pub mains_powered: bool,
    pub receiver_on_when_idle: bool,
    pub security_capable: bool,
    pub allocate_address: bool,
}

impl MacCapabilityFlags {
    pub fn parse(raw: u8) -> Self {
        Self {
            alternate_pan_coordinator: raw & 0x01 != 0,
            full_function_device: raw & 0x02 != 0,
            mains_powered: raw & 0x04 != 0,
            receiver_on_when_idle: raw & 0x08 != 0,
            security_capable: raw & 0x40 != 0,
            allocate_address: raw & 0x80 != 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeDescriptor {
    pub logical_type: LogicalType,
    pub complex_descriptor_available: bool,
    pub user_descriptor_available: bool,
    pub aps_flags: u8,
    pub frequency_band: u8,
    pub mac_capability_flags: MacCapabilityFlags,
    pub manufacturer_code: u16,
    pub maximum_buffer_size: u8,
    pub maximum_incoming_transfer_size: u16,
    pub server_mask: u16,
    pub maximum_outgoing_transfer_size: u16,
    pub descriptor_capability_field: u8,
}

impl NodeDescriptor {
    pub fn parse(bytes: &[u8]) -> Result<Self, ZdoError> {
        let mut cursor = Cursor::new(bytes);
        let node_flags = cursor.read_u8()?;
        let aps_and_frequency = cursor.read_u8()?;
        Ok(Self {
            logical_type: LogicalType::parse(node_flags),
            complex_descriptor_available: node_flags & 0x08 != 0,
            user_descriptor_available: node_flags & 0x10 != 0,
            aps_flags: aps_and_frequency & 0b111,
            frequency_band: (aps_and_frequency >> 3) & 0b1_1111,
            mac_capability_flags: MacCapabilityFlags::parse(cursor.read_u8()?),
            manufacturer_code: cursor.read_u16_le()?,
            maximum_buffer_size: cursor.read_u8()?,
            maximum_incoming_transfer_size: cursor.read_u16_le()?,
            server_mask: cursor.read_u16_le()?,
            maximum_outgoing_transfer_size: cursor.read_u16_le()?,
            descriptor_capability_field: cursor.read_u8()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimpleDescriptor {
    pub endpoint: Endpoint,
    pub profile_id: ProfileId,
    pub device_id: u16,
    pub device_version: u8,
    pub input_clusters: Vec<ClusterId>,
    pub output_clusters: Vec<ClusterId>,
}

impl SimpleDescriptor {
    pub fn parse(bytes: &[u8]) -> Result<Self, ZdoError> {
        let mut cursor = Cursor::new(bytes);
        let endpoint = Endpoint(cursor.read_u8()?);
        let profile_id = ProfileId(cursor.read_u16_le()?);
        let device_id = cursor.read_u16_le()?;
        let device_version = cursor.read_u8()? & 0x0f;
        let input_clusters = cursor.read_cluster_list()?;
        let output_clusters = cursor.read_cluster_list()?;
        Ok(Self {
            endpoint,
            profile_id,
            device_id,
            device_version,
            input_clusters,
            output_clusters,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeDescriptorResponse {
    pub transaction_sequence_number: u8,
    pub status: ZdoStatus,
    pub network_address: NetworkAddress,
    pub descriptor: Option<NodeDescriptor>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimpleDescriptorResponse {
    pub transaction_sequence_number: u8,
    pub status: ZdoStatus,
    pub network_address: NetworkAddress,
    pub descriptor: Option<SimpleDescriptor>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveEndpointsResponse {
    pub transaction_sequence_number: u8,
    pub status: ZdoStatus,
    pub network_address: NetworkAddress,
    pub endpoints: Vec<Endpoint>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZigbeeInterviewSummary {
    pub network_address: NetworkAddress,
    pub ieee_address: Option<IeeeAddress>,
    pub node_descriptor: Option<NodeDescriptor>,
    pub simple_descriptors: Vec<SimpleDescriptor>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ZdoError {
    Truncated { needed: usize, remaining: usize },
    InvalidDescriptorLength { declared: usize, remaining: usize },
}

impl fmt::Display for ZdoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated { needed, remaining } => write!(
                f,
                "truncated Zigbee ZDO payload: needed {needed} bytes, had {remaining}"
            ),
            Self::InvalidDescriptorLength {
                declared,
                remaining,
            } => write!(
                f,
                "ZDO descriptor declared {declared} bytes but only {remaining} remain"
            ),
        }
    }
}

impl std::error::Error for ZdoError {}

pub fn node_descriptor_request(
    aps_counter: u8,
    transaction_sequence_number: u8,
    target: NetworkAddress,
) -> ApsFrame {
    zdo_request_frame(
        ZdoClusterId::NODE_DESCRIPTOR_REQUEST,
        aps_counter,
        transaction_sequence_number,
        target.0.to_le_bytes().to_vec(),
    )
}

pub fn simple_descriptor_request(
    aps_counter: u8,
    transaction_sequence_number: u8,
    target: NetworkAddress,
    endpoint: Endpoint,
) -> ApsFrame {
    let mut payload = target.0.to_le_bytes().to_vec();
    payload.push(endpoint.0);
    zdo_request_frame(
        ZdoClusterId::SIMPLE_DESCRIPTOR_REQUEST,
        aps_counter,
        transaction_sequence_number,
        payload,
    )
}

pub fn active_endpoints_request(
    aps_counter: u8,
    transaction_sequence_number: u8,
    target: NetworkAddress,
) -> ApsFrame {
    zdo_request_frame(
        ZdoClusterId::ACTIVE_ENDPOINTS_REQUEST,
        aps_counter,
        transaction_sequence_number,
        target.0.to_le_bytes().to_vec(),
    )
}

pub fn parse_node_descriptor_response(payload: &[u8]) -> Result<NodeDescriptorResponse, ZdoError> {
    let mut cursor = Cursor::new(payload);
    let transaction_sequence_number = cursor.read_u8()?;
    let status = ZdoStatus::parse(cursor.read_u8()?);
    let network_address = NetworkAddress(cursor.read_u16_le()?);
    let descriptor = if status.is_success() {
        Some(NodeDescriptor::parse(cursor.remaining_bytes())?)
    } else {
        None
    };
    Ok(NodeDescriptorResponse {
        transaction_sequence_number,
        status,
        network_address,
        descriptor,
    })
}

pub fn parse_simple_descriptor_response(
    payload: &[u8],
) -> Result<SimpleDescriptorResponse, ZdoError> {
    let mut cursor = Cursor::new(payload);
    let transaction_sequence_number = cursor.read_u8()?;
    let status = ZdoStatus::parse(cursor.read_u8()?);
    let network_address = NetworkAddress(cursor.read_u16_le()?);
    let descriptor = if status.is_success() {
        let declared_len = cursor.read_u8()? as usize;
        if cursor.remaining_len() < declared_len {
            return Err(ZdoError::InvalidDescriptorLength {
                declared: declared_len,
                remaining: cursor.remaining_len(),
            });
        }
        Some(SimpleDescriptor::parse(cursor.read_bytes(declared_len)?)?)
    } else {
        None
    };
    Ok(SimpleDescriptorResponse {
        transaction_sequence_number,
        status,
        network_address,
        descriptor,
    })
}

pub fn parse_active_endpoints_response(
    payload: &[u8],
) -> Result<ActiveEndpointsResponse, ZdoError> {
    let mut cursor = Cursor::new(payload);
    let transaction_sequence_number = cursor.read_u8()?;
    let status = ZdoStatus::parse(cursor.read_u8()?);
    let network_address = NetworkAddress(cursor.read_u16_le()?);
    let endpoint_count = if status.is_success() {
        cursor.read_u8()? as usize
    } else {
        0
    };
    let mut endpoints = Vec::with_capacity(endpoint_count);
    for _ in 0..endpoint_count {
        endpoints.push(Endpoint(cursor.read_u8()?));
    }
    Ok(ActiveEndpointsResponse {
        transaction_sequence_number,
        status,
        network_address,
        endpoints,
    })
}

pub fn interview_to_device(bridge_id: &BridgeId, summary: &ZigbeeInterviewSummary) -> Device {
    let node_hex = format!("0x{:04x}", summary.network_address.0);
    let mut identifiers = vec![
        ProtocolIdentifier::new(ProtocolFamily::Zigbee, "nwk", &node_hex)
            .expect("formatted Zigbee NWK identifier is non-empty"),
    ];
    if let Some(ieee) = summary.ieee_address {
        identifiers.push(
            ProtocolIdentifier::new(ProtocolFamily::Zigbee, "ieee", format!("0x{:016x}", ieee.0))
                .expect("formatted Zigbee IEEE identifier is non-empty"),
        );
    }

    let logical_type = summary
        .node_descriptor
        .as_ref()
        .map(|descriptor| descriptor.logical_type)
        .unwrap_or(LogicalType::Unknown(0xff));
    let manufacturer = summary
        .node_descriptor
        .as_ref()
        .map(|descriptor| format!("Zigbee manufacturer 0x{:04x}", descriptor.manufacturer_code))
        .unwrap_or_else(|| "Zigbee".to_string());

    Device {
        device_id: DeviceId::trusted(format!("zigbee.device.{bridge_id}.{node_hex}")),
        bridge_id: bridge_id.clone(),
        manufacturer,
        model: logical_type.name().to_string(),
        name: format!("Zigbee node {node_hex}"),
        serial: summary
            .ieee_address
            .map(|ieee| format!("0x{:016x}", ieee.0)),
        firmware_version: None,
        room_id: None,
        entity_ids: Vec::new(),
        identifiers,
        health: Health::Unknown,
        metadata: vec![
            Metadata::new("zigbee.logical_type", logical_type.name()),
            Metadata::new(
                "zigbee.endpoint_count",
                summary.simple_descriptors.len().to_string(),
            ),
        ],
    }
}

fn zdo_request_frame(
    cluster_id: ZdoClusterId,
    aps_counter: u8,
    transaction_sequence_number: u8,
    mut payload: Vec<u8>,
) -> ApsFrame {
    let mut zdo_payload = Vec::with_capacity(1 + payload.len());
    zdo_payload.push(transaction_sequence_number);
    zdo_payload.append(&mut payload);
    ApsFrame::unicast_data(
        Endpoint::ZDO,
        Endpoint::ZDO,
        ClusterId(cluster_id.0),
        ProfileId::ZIGBEE_DEVICE_PROFILE,
        aps_counter,
        zdo_payload,
    )
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

    fn read_u8(&mut self) -> Result<u8, ZdoError> {
        if self.remaining_len() < 1 {
            return Err(ZdoError::Truncated {
                needed: 1,
                remaining: self.remaining_len(),
            });
        }
        let value = self.bytes[self.pos];
        self.pos += 1;
        Ok(value)
    }

    fn read_u16_le(&mut self) -> Result<u16, ZdoError> {
        if self.remaining_len() < 2 {
            return Err(ZdoError::Truncated {
                needed: 2,
                remaining: self.remaining_len(),
            });
        }
        let value = u16::from_le_bytes([self.bytes[self.pos], self.bytes[self.pos + 1]]);
        self.pos += 2;
        Ok(value)
    }

    fn read_bytes(&mut self, len: usize) -> Result<&'a [u8], ZdoError> {
        if self.remaining_len() < len {
            return Err(ZdoError::Truncated {
                needed: len,
                remaining: self.remaining_len(),
            });
        }
        let start = self.pos;
        self.pos += len;
        Ok(&self.bytes[start..self.pos])
    }

    fn read_cluster_list(&mut self) -> Result<Vec<ClusterId>, ZdoError> {
        let count = self.read_u8()? as usize;
        let mut clusters = Vec::with_capacity(count);
        for _ in 0..count {
            clusters.push(ClusterId(self.read_u16_le()?));
        }
        Ok(clusters)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node_descriptor_bytes() -> [u8; 13] {
        [
            0x01, 0x08, 0x8e, 0x34, 0x12, 82, 0x40, 0x00, 0x2c, 0x11, 0x40, 0x00, 0x00,
        ]
    }

    fn simple_descriptor_bytes() -> Vec<u8> {
        vec![
            1, 0x04, 0x01, 0x00, 0x01, 0x02, 2, 0x00, 0x00, 0x06, 0x00, 1, 0x03, 0x00,
        ]
    }

    #[test]
    fn parses_node_descriptor_response() {
        let mut payload = vec![0xaa, 0x00, 0x34, 0x12];
        payload.extend_from_slice(&node_descriptor_bytes());

        let response = parse_node_descriptor_response(&payload).unwrap();
        let descriptor = response.descriptor.unwrap();

        assert_eq!(response.transaction_sequence_number, 0xaa);
        assert_eq!(response.network_address, NetworkAddress(0x1234));
        assert_eq!(descriptor.logical_type, LogicalType::Router);
        assert_eq!(descriptor.manufacturer_code, 0x1234);
        assert!(descriptor.mac_capability_flags.receiver_on_when_idle);
    }

    #[test]
    fn parses_simple_descriptor_response() {
        let descriptor = simple_descriptor_bytes();
        let mut payload = vec![0xbb, 0x00, 0x34, 0x12, descriptor.len() as u8];
        payload.extend_from_slice(&descriptor);

        let response = parse_simple_descriptor_response(&payload).unwrap();
        let descriptor = response.descriptor.unwrap();

        assert_eq!(descriptor.endpoint, Endpoint(1));
        assert_eq!(descriptor.profile_id, ProfileId::HOME_AUTOMATION);
        assert_eq!(descriptor.device_id, 0x0100);
        assert_eq!(
            descriptor.input_clusters,
            vec![ClusterId::BASIC, ClusterId::ON_OFF]
        );
        assert_eq!(descriptor.output_clusters, vec![ClusterId(0x0003)]);
    }

    #[test]
    fn parses_active_endpoint_response() {
        let response =
            parse_active_endpoints_response(&[0xcc, 0x00, 0x34, 0x12, 2, 1, 11]).unwrap();

        assert_eq!(response.status, ZdoStatus::Success);
        assert_eq!(response.endpoints, vec![Endpoint(1), Endpoint(11)]);
    }

    #[test]
    fn descriptor_requests_are_zdo_aps_frames() {
        let frame = node_descriptor_request(7, 0xaa, NetworkAddress(0x1234));

        assert_eq!(frame.cluster_id, ClusterId(0x0002));
        assert_eq!(frame.profile_id, ProfileId::ZIGBEE_DEVICE_PROFILE);
        assert_eq!(frame.counter, 7);
        assert_eq!(frame.payload, vec![0xaa, 0x34, 0x12]);
    }

    #[test]
    fn simple_descriptor_request_includes_endpoint() {
        let frame = simple_descriptor_request(8, 0xbb, NetworkAddress(0x1234), Endpoint(11));

        assert_eq!(frame.cluster_id, ClusterId(0x0004));
        assert_eq!(frame.payload, vec![0xbb, 0x34, 0x12, 11]);
    }

    #[test]
    fn interview_summary_projects_to_core_device() {
        let node_descriptor = NodeDescriptor::parse(&node_descriptor_bytes()).unwrap();
        let simple_descriptor = SimpleDescriptor::parse(&simple_descriptor_bytes()).unwrap();
        let device = interview_to_device(
            &BridgeId::trusted("zigbee.bridge.1"),
            &ZigbeeInterviewSummary {
                network_address: NetworkAddress(0x1234),
                ieee_address: Some(IeeeAddress(0x0012_4b00_24c8_abcd)),
                node_descriptor: Some(node_descriptor),
                simple_descriptors: vec![simple_descriptor],
            },
        );

        assert_eq!(device.bridge_id, BridgeId::trusted("zigbee.bridge.1"));
        assert_eq!(device.model, "router");
        assert_eq!(device.serial.as_deref(), Some("0x00124b0024c8abcd"));
        assert_eq!(device.identifiers.len(), 2);
        assert_eq!(device.metadata[1].value, "1");
    }

    #[test]
    fn failed_simple_descriptor_response_omits_descriptor() {
        let response = parse_simple_descriptor_response(&[0xdd, 0x89, 0x34, 0x12]).unwrap();

        assert_eq!(response.status, ZdoStatus::NoDescriptor);
        assert!(response.descriptor.is_none());
    }
}
