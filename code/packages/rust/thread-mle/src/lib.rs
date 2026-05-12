//! Thread Mesh Link Establishment primitives.
//!
//! MLE is the Thread control plane for roles, neighbors, parent/child attach,
//! and network data exchange. This crate starts with pure message/TLV parsing
//! and a deterministic attach-state skeleton. It intentionally performs no UDP,
//! CoAP, DTLS, radio, commissioning, or border-router I/O.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt;

const NETWORK_DATA_STABLE_FLAG: u8 = 0x80;
const NETWORK_DATA_TYPE_MASK: u8 = 0x7f;
const IPV6_PREFIX_MAX_BITS: u8 = 128;
const IPV6_PREFIX_MAX_BYTES: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceRole {
    Disabled,
    Detached,
    Child,
    Router,
    Leader,
}

impl DeviceRole {
    pub fn can_route(self) -> bool {
        matches!(self, Self::Router | Self::Leader)
    }

    pub fn is_attached(self) -> bool {
        matches!(self, Self::Child | Self::Router | Self::Leader)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MleCommand {
    LinkRequest,
    LinkAccept,
    LinkAcceptAndRequest,
    LinkReject,
    Advertisement,
    Update,
    UpdateRequest,
    DataRequest,
    DataResponse,
    ParentRequest,
    ParentResponse,
    ChildIdRequest,
    ChildIdResponse,
    ChildUpdateRequest,
    ChildUpdateResponse,
    Announce,
    DiscoveryRequest,
    DiscoveryResponse,
    Unknown(u8),
}

impl MleCommand {
    pub fn from_byte(value: u8) -> Self {
        match value {
            0 => Self::LinkRequest,
            1 => Self::LinkAccept,
            2 => Self::LinkAcceptAndRequest,
            3 => Self::LinkReject,
            4 => Self::Advertisement,
            5 => Self::Update,
            6 => Self::UpdateRequest,
            7 => Self::DataRequest,
            8 => Self::DataResponse,
            9 => Self::ParentRequest,
            10 => Self::ParentResponse,
            11 => Self::ChildIdRequest,
            12 => Self::ChildIdResponse,
            13 => Self::ChildUpdateRequest,
            14 => Self::ChildUpdateResponse,
            15 => Self::Announce,
            16 => Self::DiscoveryRequest,
            17 => Self::DiscoveryResponse,
            other => Self::Unknown(other),
        }
    }

    pub fn as_byte(self) -> u8 {
        match self {
            Self::LinkRequest => 0,
            Self::LinkAccept => 1,
            Self::LinkAcceptAndRequest => 2,
            Self::LinkReject => 3,
            Self::Advertisement => 4,
            Self::Update => 5,
            Self::UpdateRequest => 6,
            Self::DataRequest => 7,
            Self::DataResponse => 8,
            Self::ParentRequest => 9,
            Self::ParentResponse => 10,
            Self::ChildIdRequest => 11,
            Self::ChildIdResponse => 12,
            Self::ChildUpdateRequest => 13,
            Self::ChildUpdateResponse => 14,
            Self::Announce => 15,
            Self::DiscoveryRequest => 16,
            Self::DiscoveryResponse => 17,
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlvType {
    SourceAddress,
    Mode,
    Timeout,
    Challenge,
    Response,
    LinkLayerFrameCounter,
    MleFrameCounter,
    Route64,
    Address16,
    LeaderData,
    NetworkData,
    TlvRequest,
    ScanMask,
    Connectivity,
    LinkMargin,
    Status,
    Version,
    AddressRegistration,
    Channel,
    PanId,
    ActiveTimestamp,
    PendingTimestamp,
    Unknown(u8),
}

impl TlvType {
    pub fn from_byte(value: u8) -> Self {
        match value {
            0 => Self::SourceAddress,
            1 => Self::Mode,
            2 => Self::Timeout,
            3 => Self::Challenge,
            4 => Self::Response,
            5 => Self::LinkLayerFrameCounter,
            8 => Self::MleFrameCounter,
            9 => Self::Route64,
            10 => Self::Address16,
            11 => Self::LeaderData,
            12 => Self::NetworkData,
            13 => Self::TlvRequest,
            14 => Self::ScanMask,
            15 => Self::Connectivity,
            16 => Self::LinkMargin,
            17 => Self::Status,
            18 => Self::Version,
            19 => Self::AddressRegistration,
            20 => Self::Channel,
            21 => Self::PanId,
            22 => Self::ActiveTimestamp,
            23 => Self::PendingTimestamp,
            other => Self::Unknown(other),
        }
    }

    pub fn as_byte(self) -> u8 {
        match self {
            Self::SourceAddress => 0,
            Self::Mode => 1,
            Self::Timeout => 2,
            Self::Challenge => 3,
            Self::Response => 4,
            Self::LinkLayerFrameCounter => 5,
            Self::MleFrameCounter => 8,
            Self::Route64 => 9,
            Self::Address16 => 10,
            Self::LeaderData => 11,
            Self::NetworkData => 12,
            Self::TlvRequest => 13,
            Self::ScanMask => 14,
            Self::Connectivity => 15,
            Self::LinkMargin => 16,
            Self::Status => 17,
            Self::Version => 18,
            Self::AddressRegistration => 19,
            Self::Channel => 20,
            Self::PanId => 21,
            Self::ActiveTimestamp => 22,
            Self::PendingTimestamp => 23,
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tlv {
    pub tlv_type: TlvType,
    pub value: Vec<u8>,
}

impl Tlv {
    pub fn new(tlv_type: TlvType, value: Vec<u8>) -> Result<Self, MleError> {
        if value.len() > u8::MAX as usize {
            return Err(MleError::TlvTooLong(value.len()));
        }
        Ok(Self { tlv_type, value })
    }

    pub fn encode(&self, out: &mut Vec<u8>) -> Result<(), MleError> {
        if self.value.len() > u8::MAX as usize {
            return Err(MleError::TlvTooLong(self.value.len()));
        }
        out.push(self.tlv_type.as_byte());
        out.push(self.value.len() as u8);
        out.extend_from_slice(&self.value);
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LeaderData {
    pub partition_id: u32,
    pub weighting: u8,
    pub data_version: u8,
    pub stable_data_version: u8,
    pub leader_router_id: u8,
}

impl LeaderData {
    pub const ENCODED_LEN: usize = 8;

    pub fn parse(value: &[u8]) -> Result<Self, MleError> {
        if value.len() != Self::ENCODED_LEN {
            return Err(MleError::InvalidTlvLength {
                tlv_type: TlvType::LeaderData,
                expected: Self::ENCODED_LEN,
                actual: value.len(),
            });
        }
        Ok(Self {
            partition_id: u32::from_be_bytes([value[0], value[1], value[2], value[3]]),
            weighting: value[4],
            data_version: value[5],
            stable_data_version: value[6],
            leader_router_id: value[7],
        })
    }

    pub fn encode(self) -> [u8; Self::ENCODED_LEN] {
        let partition_id = self.partition_id.to_be_bytes();
        [
            partition_id[0],
            partition_id[1],
            partition_id[2],
            partition_id[3],
            self.weighting,
            self.data_version,
            self.stable_data_version,
            self.leader_router_id,
        ]
    }

    pub fn to_tlv(self) -> Tlv {
        Tlv {
            tlv_type: TlvType::LeaderData,
            value: self.encode().to_vec(),
        }
    }

    pub fn has_newer_network_data_than(self, other: Self) -> bool {
        version_is_newer(self.data_version, other.data_version)
            || version_is_newer(self.stable_data_version, other.stable_data_version)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadNetworkData {
    pub bytes: Vec<u8>,
}

impl ThreadNetworkData {
    pub fn new(bytes: Vec<u8>) -> Result<Self, MleError> {
        if bytes.len() > u8::MAX as usize {
            return Err(MleError::TlvTooLong(bytes.len()));
        }
        Ok(Self { bytes })
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn to_tlv(&self) -> Tlv {
        Tlv {
            tlv_type: TlvType::NetworkData,
            value: self.bytes.clone(),
        }
    }

    pub fn from_tlvs(tlvs: Vec<NetworkDataTlv>) -> Result<Self, MleError> {
        let mut bytes = Vec::new();
        for tlv in &tlvs {
            tlv.encode(&mut bytes)?;
        }
        Self::new(bytes)
    }

    pub fn tlvs(&self) -> Result<Vec<NetworkDataTlv>, MleError> {
        NetworkDataTlv::parse_many(&self.bytes)
    }

    pub fn prefixes(&self) -> Result<Vec<ThreadPrefixData>, MleError> {
        self.tlvs()?
            .iter()
            .filter(|tlv| tlv.tlv_type == NetworkDataTlvType::Prefix)
            .map(ThreadPrefixData::parse)
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkDataTlvType {
    HasRoute,
    Prefix,
    BorderRouter,
    LowpanId,
    CommissioningData,
    Service,
    Server,
    Context,
    Unknown(u8),
}

impl NetworkDataTlvType {
    pub fn from_byte(value: u8) -> Self {
        match value & NETWORK_DATA_TYPE_MASK {
            0 => Self::HasRoute,
            1 => Self::Prefix,
            2 => Self::BorderRouter,
            3 => Self::LowpanId,
            4 => Self::CommissioningData,
            5 => Self::Service,
            6 => Self::Server,
            7 => Self::Context,
            other => Self::Unknown(other),
        }
    }

    pub fn as_byte(self) -> u8 {
        match self {
            Self::HasRoute => 0,
            Self::Prefix => 1,
            Self::BorderRouter => 2,
            Self::LowpanId => 3,
            Self::CommissioningData => 4,
            Self::Service => 5,
            Self::Server => 6,
            Self::Context => 7,
            Self::Unknown(value) => value & NETWORK_DATA_TYPE_MASK,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkDataTlv {
    pub tlv_type: NetworkDataTlvType,
    pub stable: bool,
    pub value: Vec<u8>,
}

impl NetworkDataTlv {
    pub fn new(
        tlv_type: NetworkDataTlvType,
        stable: bool,
        value: Vec<u8>,
    ) -> Result<Self, MleError> {
        if value.len() > u8::MAX as usize {
            return Err(MleError::TlvTooLong(value.len()));
        }
        Ok(Self {
            tlv_type,
            stable,
            value,
        })
    }

    pub fn parse_many(bytes: &[u8]) -> Result<Vec<Self>, MleError> {
        let mut cursor = Cursor::new(bytes);
        let mut tlvs = Vec::new();
        while cursor.remaining() > 0 {
            let header = cursor.read_u8()?;
            let len = cursor.read_u8()? as usize;
            let value = cursor.read_bytes(len)?.to_vec();
            tlvs.push(Self {
                tlv_type: NetworkDataTlvType::from_byte(header),
                stable: header & NETWORK_DATA_STABLE_FLAG != 0,
                value,
            });
        }
        Ok(tlvs)
    }

    pub fn encode(&self, out: &mut Vec<u8>) -> Result<(), MleError> {
        if self.value.len() > u8::MAX as usize {
            return Err(MleError::TlvTooLong(self.value.len()));
        }
        let stable = if self.stable {
            NETWORK_DATA_STABLE_FLAG
        } else {
            0
        };
        out.push(stable | self.tlv_type.as_byte());
        out.push(self.value.len() as u8);
        out.extend_from_slice(&self.value);
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadPrefixData {
    pub stable: bool,
    pub domain_id: u8,
    pub prefix_length_bits: u8,
    pub prefix_bytes: Vec<u8>,
    pub sub_tlvs: Vec<NetworkDataTlv>,
}

impl ThreadPrefixData {
    pub fn new(
        stable: bool,
        domain_id: u8,
        prefix_length_bits: u8,
        prefix_bytes: Vec<u8>,
        sub_tlvs: Vec<NetworkDataTlv>,
    ) -> Result<Self, MleError> {
        validate_prefix_bytes(prefix_length_bits, prefix_bytes.len())?;
        Ok(Self {
            stable,
            domain_id,
            prefix_length_bits,
            prefix_bytes,
            sub_tlvs,
        })
    }

    pub fn parse(tlv: &NetworkDataTlv) -> Result<Self, MleError> {
        if tlv.tlv_type != NetworkDataTlvType::Prefix {
            return Err(MleError::InvalidNetworkDataTlv {
                tlv_type: tlv.tlv_type,
                reason: "expected Thread Network Data Prefix TLV",
            });
        }
        let mut cursor = Cursor::new(&tlv.value);
        let domain_id = cursor.read_u8()?;
        let prefix_length_bits = cursor.read_u8()?;
        let prefix_len = prefix_byte_len(prefix_length_bits)?;
        let prefix_bytes = cursor.read_bytes(prefix_len)?.to_vec();
        let sub_tlvs = NetworkDataTlv::parse_many(cursor.remaining_bytes())?;
        Self::new(
            tlv.stable,
            domain_id,
            prefix_length_bits,
            prefix_bytes,
            sub_tlvs,
        )
    }

    pub fn to_tlv(&self) -> Result<NetworkDataTlv, MleError> {
        validate_prefix_bytes(self.prefix_length_bits, self.prefix_bytes.len())?;
        let mut value = Vec::with_capacity(2 + self.prefix_bytes.len());
        value.push(self.domain_id);
        value.push(self.prefix_length_bits);
        value.extend_from_slice(&self.prefix_bytes);
        for sub_tlv in &self.sub_tlvs {
            sub_tlv.encode(&mut value)?;
        }
        NetworkDataTlv::new(NetworkDataTlvType::Prefix, self.stable, value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkDataAdvertisement {
    pub leader_data: Option<LeaderData>,
    pub network_data: Option<ThreadNetworkData>,
}

impl NetworkDataAdvertisement {
    pub fn from_message(message: &MleMessage) -> Result<Self, MleError> {
        Ok(Self {
            leader_data: leader_data_from_message(message)?,
            network_data: network_data_from_message(message),
        })
    }

    pub fn has_network_data(&self) -> bool {
        self.network_data
            .as_ref()
            .is_some_and(|network_data| !network_data.is_empty())
    }

    pub fn prefixes(&self) -> Result<Vec<ThreadPrefixData>, MleError> {
        self.network_data
            .as_ref()
            .map(ThreadNetworkData::prefixes)
            .unwrap_or_else(|| Ok(Vec::new()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Connectivity {
    pub parent_priority: i8,
    pub link_quality_3: u8,
    pub link_quality_2: u8,
    pub link_quality_1: u8,
    pub leader_cost: u8,
    pub id_sequence: u8,
    pub active_router_count: u8,
    pub sleepy_end_device_buffer_size: Option<u16>,
    pub sleepy_end_device_datagram_count: Option<u8>,
}

impl Connectivity {
    pub const BASE_ENCODED_LEN: usize = 7;
    pub const SLEEPY_END_DEVICE_ENCODED_LEN: usize = 10;

    pub fn parse(value: &[u8]) -> Result<Self, MleError> {
        if value.len() != Self::BASE_ENCODED_LEN
            && value.len() != Self::SLEEPY_END_DEVICE_ENCODED_LEN
        {
            return Err(MleError::InvalidTlvLength {
                tlv_type: TlvType::Connectivity,
                expected: Self::BASE_ENCODED_LEN,
                actual: value.len(),
            });
        }
        let has_sleepy_end_device_fields = value.len() == Self::SLEEPY_END_DEVICE_ENCODED_LEN;
        let sleepy_end_device_buffer_size =
            has_sleepy_end_device_fields.then(|| u16::from_be_bytes([value[7], value[8]]));
        let sleepy_end_device_datagram_count = has_sleepy_end_device_fields.then(|| value[9]);
        Ok(Self {
            parent_priority: value[0] as i8,
            link_quality_3: value[1],
            link_quality_2: value[2],
            link_quality_1: value[3],
            leader_cost: value[4],
            id_sequence: value[5],
            active_router_count: value[6],
            sleepy_end_device_buffer_size,
            sleepy_end_device_datagram_count,
        })
    }

    pub fn encode(self) -> Vec<u8> {
        let mut out = vec![
            self.parent_priority as u8,
            self.link_quality_3,
            self.link_quality_2,
            self.link_quality_1,
            self.leader_cost,
            self.id_sequence,
            self.active_router_count,
        ];
        if let (Some(buffer_size), Some(datagram_count)) = (
            self.sleepy_end_device_buffer_size,
            self.sleepy_end_device_datagram_count,
        ) {
            out.extend_from_slice(&buffer_size.to_be_bytes());
            out.push(datagram_count);
        }
        out
    }

    pub fn to_tlv(self) -> Tlv {
        Tlv {
            tlv_type: TlvType::Connectivity,
            value: self.encode(),
        }
    }

    pub fn has_sleepy_end_device_capacity(self) -> bool {
        self.sleepy_end_device_buffer_size.is_some()
            && self.sleepy_end_device_datagram_count.is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MleMessage {
    pub command: MleCommand,
    pub tlvs: Vec<Tlv>,
}

impl MleMessage {
    pub fn parse(bytes: &[u8]) -> Result<Self, MleError> {
        let Some((&command, rest)) = bytes.split_first() else {
            return Err(MleError::Truncated {
                needed: 1,
                remaining: 0,
            });
        };
        let mut cursor = Cursor::new(rest);
        let mut tlvs = Vec::new();
        while cursor.remaining() > 0 {
            let tlv_type = TlvType::from_byte(cursor.read_u8()?);
            let len = cursor.read_u8()? as usize;
            let value = cursor.read_bytes(len)?.to_vec();
            tlvs.push(Tlv { tlv_type, value });
        }
        Ok(Self {
            command: MleCommand::from_byte(command),
            tlvs,
        })
    }

    pub fn encode(&self) -> Result<Vec<u8>, MleError> {
        let mut out = Vec::new();
        out.push(self.command.as_byte());
        for tlv in &self.tlvs {
            tlv.encode(&mut out)?;
        }
        Ok(out)
    }

    pub fn find_tlv(&self, tlv_type: TlvType) -> Option<&Tlv> {
        self.tlvs.iter().find(|tlv| tlv.tlv_type == tlv_type)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanMask {
    pub routers: bool,
    pub end_devices: bool,
}

impl ScanMask {
    pub fn parse(value: u8) -> Self {
        Self {
            routers: value & 0x80 != 0,
            end_devices: value & 0x40 != 0,
        }
    }

    pub fn encode(self) -> u8 {
        ((self.routers as u8) << 7) | ((self.end_devices as u8) << 6)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mode {
    pub receiver_on_when_idle: bool,
    pub secure_data_requests: bool,
    pub full_thread_device: bool,
    pub full_network_data: bool,
}

impl Mode {
    pub fn parse(value: u8) -> Self {
        Self {
            receiver_on_when_idle: value & (1 << 3) != 0,
            secure_data_requests: value & (1 << 2) != 0,
            full_thread_device: value & (1 << 1) != 0,
            full_network_data: value & 1 != 0,
        }
    }

    pub fn encode(self) -> u8 {
        ((self.receiver_on_when_idle as u8) << 3)
            | ((self.secure_data_requests as u8) << 2)
            | ((self.full_thread_device as u8) << 1)
            | (self.full_network_data as u8)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ThreadNeighborId(pub u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NeighborRelationship {
    Parent,
    Child,
    RouterPeer,
    Leader,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LinkMetrics {
    pub link_margin: Option<u8>,
    pub incoming_link_quality: Option<u8>,
    pub outgoing_link_quality: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadNeighbor {
    pub neighbor_id: ThreadNeighborId,
    pub role: DeviceRole,
    pub relationship: NeighborRelationship,
    pub mode: Option<Mode>,
    pub metrics: LinkMetrics,
    pub last_heard_at_ms: u64,
    pub timeout_ms: u64,
}

impl ThreadNeighbor {
    pub fn new(
        neighbor_id: ThreadNeighborId,
        role: DeviceRole,
        relationship: NeighborRelationship,
        last_heard_at_ms: u64,
        timeout_ms: u64,
    ) -> Self {
        Self {
            neighbor_id,
            role,
            relationship,
            mode: None,
            metrics: LinkMetrics::default(),
            last_heard_at_ms,
            timeout_ms,
        }
    }

    pub fn with_mode(mut self, mode: Mode) -> Self {
        self.mode = Some(mode);
        self
    }

    pub fn with_link_margin(mut self, link_margin: u8) -> Self {
        self.metrics.link_margin = Some(link_margin);
        self
    }

    pub fn is_stale_at(&self, now_ms: u64) -> bool {
        now_ms >= self.last_heard_at_ms.saturating_add(self.timeout_ms)
    }

    pub fn can_route(&self) -> bool {
        self.role.can_route()
    }
}

#[derive(Debug, Clone)]
pub struct NeighborTable {
    local_role: DeviceRole,
    neighbors: BTreeMap<ThreadNeighborId, ThreadNeighbor>,
    parent: Option<ThreadNeighborId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NeighborTableSummary {
    pub local_role: DeviceRole,
    pub neighbor_count: usize,
    pub parent: Option<ThreadNeighborId>,
    pub child_count: usize,
    pub router_count: usize,
    pub stale_neighbor_count: usize,
    pub best_parent_candidate: Option<ThreadNeighborId>,
}

impl NeighborTableSummary {
    pub fn is_empty(self) -> bool {
        self.neighbor_count == 0
    }

    pub fn has_parent(self) -> bool {
        self.parent.is_some()
    }

    pub fn has_stale_neighbors(self) -> bool {
        self.stale_neighbor_count > 0
    }

    pub fn has_parent_candidate(self) -> bool {
        self.best_parent_candidate.is_some()
    }

    pub fn needs_attach(self) -> bool {
        self.local_role == DeviceRole::Detached
            || (self.local_role == DeviceRole::Child && self.parent.is_none())
    }

    pub fn has_routing_surface(self) -> bool {
        self.local_role.can_route() || self.router_count > 0
    }
}

impl NeighborTable {
    pub fn new(local_role: DeviceRole) -> Self {
        Self {
            local_role,
            neighbors: BTreeMap::new(),
            parent: None,
        }
    }

    pub fn local_role(&self) -> DeviceRole {
        self.local_role
    }

    pub fn len(&self) -> usize {
        self.neighbors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.neighbors.is_empty()
    }

    pub fn upsert(&mut self, neighbor: ThreadNeighbor) -> Option<ThreadNeighbor> {
        if neighbor.relationship == NeighborRelationship::Parent {
            self.parent = Some(neighbor.neighbor_id);
        }
        self.neighbors.insert(neighbor.neighbor_id, neighbor)
    }

    pub fn mark_parent(&mut self, neighbor_id: ThreadNeighborId) -> Result<(), MleError> {
        let neighbor = self
            .neighbors
            .get_mut(&neighbor_id)
            .ok_or(MleError::UnknownNeighbor(neighbor_id))?;
        neighbor.relationship = NeighborRelationship::Parent;
        self.parent = Some(neighbor_id);
        Ok(())
    }

    pub fn neighbor(&self, neighbor_id: ThreadNeighborId) -> Option<&ThreadNeighbor> {
        self.neighbors.get(&neighbor_id)
    }

    pub fn parent(&self) -> Option<&ThreadNeighbor> {
        self.parent.and_then(|id| self.neighbors.get(&id))
    }

    pub fn children(&self) -> impl Iterator<Item = &ThreadNeighbor> {
        self.neighbors
            .values()
            .filter(|neighbor| neighbor.relationship == NeighborRelationship::Child)
    }

    pub fn routers(&self) -> impl Iterator<Item = &ThreadNeighbor> {
        self.neighbors
            .values()
            .filter(|neighbor| neighbor.can_route())
    }

    pub fn stale_neighbors_at(&self, now_ms: u64) -> Vec<ThreadNeighborId> {
        self.neighbors
            .values()
            .filter(|neighbor| neighbor.is_stale_at(now_ms))
            .map(|neighbor| neighbor.neighbor_id)
            .collect()
    }

    pub fn expire_stale(&mut self, now_ms: u64) -> Vec<ThreadNeighborId> {
        let stale = self.stale_neighbors_at(now_ms);
        for neighbor_id in &stale {
            self.neighbors.remove(neighbor_id);
            if self.parent == Some(*neighbor_id) {
                self.parent = None;
            }
        }
        stale
    }

    pub fn best_parent_candidate(&self) -> Option<&ThreadNeighbor> {
        self.routers().max_by_key(|neighbor| {
            (
                neighbor.metrics.link_margin.unwrap_or(0),
                neighbor.last_heard_at_ms,
            )
        })
    }

    pub fn summary_at(&self, now_ms: u64) -> NeighborTableSummary {
        NeighborTableSummary {
            local_role: self.local_role,
            neighbor_count: self.len(),
            parent: self.parent().map(|neighbor| neighbor.neighbor_id),
            child_count: self.children().count(),
            router_count: self.routers().count(),
            stale_neighbor_count: self.stale_neighbors_at(now_ms).len(),
            best_parent_candidate: self
                .best_parent_candidate()
                .map(|neighbor| neighbor.neighbor_id),
        }
    }

    pub fn diagnostic_snapshot(
        &self,
        message: Option<&MleMessage>,
        captured_at_ms: u64,
    ) -> Result<ThreadDiagnosticSnapshot, MleError> {
        ThreadDiagnosticSnapshot::from_parts(self, message, captured_at_ms)
    }
}

impl Default for NeighborTable {
    fn default() -> Self {
        Self::new(DeviceRole::Detached)
    }
}

pub fn neighbor_from_parent_response(
    neighbor_id: ThreadNeighborId,
    message: &MleMessage,
    received_at_ms: u64,
    default_timeout_ms: u64,
) -> ThreadNeighbor {
    let mode = mode_from_message(message);
    let timeout_ms = timeout_ms_from_message(message).unwrap_or(default_timeout_ms);
    let link_margin = link_margin_from_message(message);
    let role = match mode {
        Some(mode) if !mode.full_thread_device => DeviceRole::Child,
        _ => DeviceRole::Router,
    };
    let mut neighbor = ThreadNeighbor::new(
        neighbor_id,
        role,
        NeighborRelationship::Parent,
        received_at_ms,
        timeout_ms,
    );
    if let Some(mode) = mode {
        neighbor = neighbor.with_mode(mode);
    }
    if let Some(link_margin) = link_margin {
        neighbor = neighbor.with_link_margin(link_margin);
    }
    neighbor
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadDiagnosticHealth {
    Offline,
    Detached,
    Degraded,
    Healthy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadSupervisionAction {
    Observe,
    EnableInterface,
    StartAttach,
    RefreshParent,
    RefreshRouterConnectivity,
}

impl ThreadSupervisionAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Observe => "observe",
            Self::EnableInterface => "enable_interface",
            Self::StartAttach => "start_attach",
            Self::RefreshParent => "refresh_parent",
            Self::RefreshRouterConnectivity => "refresh_router_connectivity",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadSupervisionPlan {
    pub health: ThreadDiagnosticHealth,
    pub action: ThreadSupervisionAction,
    pub parent: Option<ThreadNeighborId>,
    pub best_parent_candidate: Option<ThreadNeighborId>,
}

impl ThreadSupervisionPlan {
    pub fn needs_intervention(self) -> bool {
        self.action != ThreadSupervisionAction::Observe
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadDiagnosticSnapshot {
    pub captured_at_ms: u64,
    pub local_role: DeviceRole,
    pub parent: Option<ThreadNeighborId>,
    pub router_count: usize,
    pub child_count: usize,
    pub stale_neighbors: Vec<ThreadNeighborId>,
    pub best_parent_candidate: Option<ThreadNeighborId>,
    pub leader_data: Option<LeaderData>,
    pub connectivity: Option<Connectivity>,
    pub prefixes: Vec<ThreadPrefixData>,
}

impl ThreadDiagnosticSnapshot {
    pub fn from_parts(
        neighbors: &NeighborTable,
        message: Option<&MleMessage>,
        captured_at_ms: u64,
    ) -> Result<Self, MleError> {
        let advertisement = message
            .map(NetworkDataAdvertisement::from_message)
            .transpose()?;
        let connectivity = message
            .map(connectivity_from_message)
            .transpose()?
            .flatten();
        let prefixes = advertisement
            .as_ref()
            .map(NetworkDataAdvertisement::prefixes)
            .transpose()?
            .unwrap_or_default();

        Ok(Self {
            captured_at_ms,
            local_role: neighbors.local_role(),
            parent: neighbors.parent().map(|neighbor| neighbor.neighbor_id),
            router_count: neighbors.routers().count(),
            child_count: neighbors.children().count(),
            stale_neighbors: neighbors.stale_neighbors_at(captured_at_ms),
            best_parent_candidate: neighbors
                .best_parent_candidate()
                .map(|neighbor| neighbor.neighbor_id),
            leader_data: advertisement.and_then(|advertisement| advertisement.leader_data),
            connectivity,
            prefixes,
        })
    }

    pub fn partition_id(&self) -> Option<u32> {
        self.leader_data.map(|leader_data| leader_data.partition_id)
    }

    pub fn active_router_count(&self) -> Option<u8> {
        self.connectivity
            .map(|connectivity| connectivity.active_router_count)
    }

    pub fn health(&self) -> ThreadDiagnosticHealth {
        if self.local_role == DeviceRole::Disabled {
            return ThreadDiagnosticHealth::Offline;
        }
        if !self.local_role.is_attached() {
            return ThreadDiagnosticHealth::Detached;
        }
        if self.local_role == DeviceRole::Child && self.parent.is_none() {
            return ThreadDiagnosticHealth::Degraded;
        }
        if self
            .parent
            .is_some_and(|parent| self.stale_neighbors.contains(&parent))
        {
            return ThreadDiagnosticHealth::Degraded;
        }
        if self
            .connectivity
            .is_some_and(|connectivity| connectivity.active_router_count == 0)
            && self.local_role.can_route()
            && self.local_role != DeviceRole::Leader
        {
            return ThreadDiagnosticHealth::Degraded;
        }
        ThreadDiagnosticHealth::Healthy
    }

    pub fn supervision_plan(&self) -> ThreadSupervisionPlan {
        let action = if self.local_role == DeviceRole::Disabled {
            ThreadSupervisionAction::EnableInterface
        } else if !self.local_role.is_attached() {
            ThreadSupervisionAction::StartAttach
        } else if self.local_role == DeviceRole::Child && self.parent.is_none() {
            ThreadSupervisionAction::StartAttach
        } else if self
            .parent
            .is_some_and(|parent| self.stale_neighbors.contains(&parent))
        {
            ThreadSupervisionAction::RefreshParent
        } else if self
            .connectivity
            .is_some_and(|connectivity| connectivity.active_router_count == 0)
            && self.local_role.can_route()
            && self.local_role != DeviceRole::Leader
        {
            ThreadSupervisionAction::RefreshRouterConnectivity
        } else {
            ThreadSupervisionAction::Observe
        };

        ThreadSupervisionPlan {
            health: self.health(),
            action,
            parent: self.parent,
            best_parent_candidate: self.best_parent_candidate,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachState {
    Detached,
    ParentSearch,
    ParentCandidate,
    ChildIdRequestSent,
    Attached(DeviceRole),
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachAction {
    SendParentRequest,
    SendChildIdRequest,
    BecomeChild,
    BecomeRouter,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachMachine {
    state: AttachState,
}

impl AttachMachine {
    pub fn new() -> Self {
        Self {
            state: AttachState::Detached,
        }
    }

    pub fn state(&self) -> AttachState {
        self.state
    }

    pub fn start(&mut self) -> AttachAction {
        self.state = AttachState::ParentSearch;
        AttachAction::SendParentRequest
    }

    pub fn on_message(&mut self, message: &MleMessage) -> AttachAction {
        match (self.state, message.command) {
            (AttachState::ParentSearch, MleCommand::ParentResponse) => {
                self.state = AttachState::ParentCandidate;
                AttachAction::SendChildIdRequest
            }
            (AttachState::ParentCandidate, MleCommand::ChildIdResponse) => {
                let role = role_from_child_id_response(message);
                self.state = AttachState::Attached(role);
                match role {
                    DeviceRole::Router | DeviceRole::Leader => AttachAction::BecomeRouter,
                    _ => AttachAction::BecomeChild,
                }
            }
            (_, MleCommand::LinkReject) => {
                self.state = AttachState::Rejected;
                AttachAction::None
            }
            _ => AttachAction::None,
        }
    }
}

impl Default for AttachMachine {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MleError {
    Truncated {
        needed: usize,
        remaining: usize,
    },
    TlvTooLong(usize),
    InvalidTlvLength {
        tlv_type: TlvType,
        expected: usize,
        actual: usize,
    },
    InvalidNetworkDataTlv {
        tlv_type: NetworkDataTlvType,
        reason: &'static str,
    },
    UnknownNeighbor(ThreadNeighborId),
}

impl fmt::Display for MleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated { needed, remaining } => {
                write!(
                    f,
                    "truncated Thread MLE bytes: needed {needed}, had {remaining}"
                )
            }
            Self::TlvTooLong(len) => write!(f, "Thread MLE TLV too long: {len}"),
            Self::InvalidTlvLength {
                tlv_type,
                expected,
                actual,
            } => write!(
                f,
                "Thread MLE {tlv_type:?} TLV has length {actual}, expected {expected}"
            ),
            Self::InvalidNetworkDataTlv { tlv_type, reason } => {
                write!(
                    f,
                    "Thread Network Data {tlv_type:?} TLV is invalid: {reason}"
                )
            }
            Self::UnknownNeighbor(neighbor_id) => {
                write!(f, "unknown Thread neighbor 0x{:04x}", neighbor_id.0)
            }
        }
    }
}

impl std::error::Error for MleError {}

pub fn leader_data_from_message(message: &MleMessage) -> Result<Option<LeaderData>, MleError> {
    message
        .find_tlv(TlvType::LeaderData)
        .map(|tlv| LeaderData::parse(&tlv.value))
        .transpose()
}

pub fn network_data_from_message(message: &MleMessage) -> Option<ThreadNetworkData> {
    message
        .find_tlv(TlvType::NetworkData)
        .map(|tlv| ThreadNetworkData {
            bytes: tlv.value.clone(),
        })
}

pub fn connectivity_from_message(message: &MleMessage) -> Result<Option<Connectivity>, MleError> {
    message
        .find_tlv(TlvType::Connectivity)
        .map(|tlv| Connectivity::parse(&tlv.value))
        .transpose()
}

pub fn version_is_newer(candidate: u8, current: u8) -> bool {
    let distance = candidate.wrapping_sub(current);
    distance != 0 && distance < 128
}

fn role_from_child_id_response(message: &MleMessage) -> DeviceRole {
    let mode = mode_from_message(message);
    match mode {
        Some(mode) if mode.full_thread_device => DeviceRole::Router,
        _ => DeviceRole::Child,
    }
}

fn mode_from_message(message: &MleMessage) -> Option<Mode> {
    message
        .find_tlv(TlvType::Mode)
        .and_then(|tlv| tlv.value.first().copied())
        .map(Mode::parse)
}

fn link_margin_from_message(message: &MleMessage) -> Option<u8> {
    message
        .find_tlv(TlvType::LinkMargin)
        .and_then(|tlv| tlv.value.first().copied())
}

fn timeout_ms_from_message(message: &MleMessage) -> Option<u64> {
    let value = &message.find_tlv(TlvType::Timeout)?.value;
    if value.len() != 4 {
        return None;
    }
    let seconds = u32::from_be_bytes([value[0], value[1], value[2], value[3]]);
    Some(u64::from(seconds) * 1_000)
}

struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.pos)
    }

    fn read_u8(&mut self) -> Result<u8, MleError> {
        if self.remaining() < 1 {
            return Err(MleError::Truncated {
                needed: 1,
                remaining: self.remaining(),
            });
        }
        let value = self.bytes[self.pos];
        self.pos += 1;
        Ok(value)
    }

    fn read_bytes(&mut self, len: usize) -> Result<&'a [u8], MleError> {
        if self.remaining() < len {
            return Err(MleError::Truncated {
                needed: len,
                remaining: self.remaining(),
            });
        }
        let bytes = &self.bytes[self.pos..self.pos + len];
        self.pos += len;
        Ok(bytes)
    }

    fn remaining_bytes(&self) -> &'a [u8] {
        &self.bytes[self.pos..]
    }
}

fn prefix_byte_len(prefix_length_bits: u8) -> Result<usize, MleError> {
    if prefix_length_bits > IPV6_PREFIX_MAX_BITS {
        return Err(MleError::InvalidNetworkDataTlv {
            tlv_type: NetworkDataTlvType::Prefix,
            reason: "IPv6 prefix length exceeds 128 bits",
        });
    }
    Ok(usize::from(prefix_length_bits).div_ceil(8))
}

fn validate_prefix_bytes(prefix_length_bits: u8, actual_len: usize) -> Result<(), MleError> {
    let expected_len = prefix_byte_len(prefix_length_bits)?;
    if actual_len != expected_len || actual_len > IPV6_PREFIX_MAX_BYTES {
        return Err(MleError::InvalidNetworkDataTlv {
            tlv_type: NetworkDataTlvType::Prefix,
            reason: "prefix byte length does not match prefix length",
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mle_message_round_trips_tlvs() {
        let message = MleMessage {
            command: MleCommand::ParentRequest,
            tlvs: vec![
                Tlv::new(
                    TlvType::ScanMask,
                    vec![ScanMask {
                        routers: true,
                        end_devices: false,
                    }
                    .encode()],
                )
                .unwrap(),
                Tlv::new(TlvType::Version, vec![0x00, 0x04]).unwrap(),
            ],
        };

        assert_eq!(
            MleMessage::parse(&message.encode().unwrap()).unwrap(),
            message
        );
    }

    #[test]
    fn scan_mask_and_mode_bits_round_trip() {
        let scan = ScanMask {
            routers: true,
            end_devices: true,
        };
        let mode = Mode {
            receiver_on_when_idle: true,
            secure_data_requests: true,
            full_thread_device: false,
            full_network_data: true,
        };

        assert_eq!(ScanMask::parse(scan.encode()), scan);
        assert_eq!(Mode::parse(mode.encode()), mode);
    }

    #[test]
    fn leader_data_tlv_round_trips_and_compares_versions() {
        let current = LeaderData {
            partition_id: 0x0102_0304,
            weighting: 64,
            data_version: 254,
            stable_data_version: 10,
            leader_router_id: 7,
        };
        let newer = LeaderData {
            data_version: 1,
            stable_data_version: 11,
            ..current
        };

        assert_eq!(LeaderData::parse(&current.encode()).unwrap(), current);
        assert_eq!(current.to_tlv().tlv_type, TlvType::LeaderData);
        assert!(newer.has_newer_network_data_than(current));
        assert!(!current.has_newer_network_data_than(newer));
    }

    #[test]
    fn network_data_advertisement_extracts_leader_and_raw_network_data() {
        let leader_data = LeaderData {
            partition_id: 0x1122_3344,
            weighting: 16,
            data_version: 9,
            stable_data_version: 7,
            leader_router_id: 3,
        };
        let network_data = ThreadNetworkData::new(vec![0x12, 0x34, 0x56]).unwrap();
        let message = MleMessage {
            command: MleCommand::DataResponse,
            tlvs: vec![leader_data.to_tlv(), network_data.to_tlv()],
        };

        let advertisement = NetworkDataAdvertisement::from_message(&message).unwrap();

        assert_eq!(advertisement.leader_data, Some(leader_data));
        assert_eq!(
            advertisement.network_data.as_ref().unwrap().bytes,
            vec![0x12, 0x34, 0x56]
        );
        assert!(advertisement.has_network_data());
    }

    #[test]
    fn network_data_tlvs_round_trip_stable_prefixes() {
        let border_router = NetworkDataTlv::new(
            NetworkDataTlvType::BorderRouter,
            true,
            vec![0x12, 0x34, 0x80, 0x00],
        )
        .unwrap();
        let prefix = ThreadPrefixData::new(
            true,
            3,
            64,
            vec![0xfd, 0x00, 0xab, 0xcd, 0, 0, 0, 0],
            vec![border_router.clone()],
        )
        .unwrap();
        let unknown =
            NetworkDataTlv::new(NetworkDataTlvType::Unknown(42), false, vec![1, 2, 3]).unwrap();
        let network_data =
            ThreadNetworkData::from_tlvs(vec![prefix.to_tlv().unwrap(), unknown.clone()]).unwrap();

        let tlvs = network_data.tlvs().unwrap();
        let prefixes = network_data.prefixes().unwrap();

        assert_eq!(tlvs.len(), 2);
        assert_eq!(tlvs[0].tlv_type, NetworkDataTlvType::Prefix);
        assert!(tlvs[0].stable);
        assert_eq!(tlvs[1], unknown);
        assert_eq!(prefixes, vec![prefix]);
        assert_eq!(prefixes[0].sub_tlvs, vec![border_router]);
    }

    #[test]
    fn network_data_advertisement_projects_prefixes() {
        let prefix =
            ThreadPrefixData::new(false, 1, 48, vec![0xfd, 0x12, 0x34, 0, 0, 0], Vec::new())
                .unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![prefix.to_tlv().unwrap()]).unwrap();
        let advertisement = NetworkDataAdvertisement {
            leader_data: None,
            network_data: Some(network_data),
        };

        assert_eq!(advertisement.prefixes().unwrap(), vec![prefix]);
    }

    #[test]
    fn network_data_rejects_truncated_tlv_value() {
        let data =
            ThreadNetworkData::new(vec![NetworkDataTlvType::Prefix.as_byte(), 4, 0, 64]).unwrap();

        assert_eq!(
            data.tlvs(),
            Err(MleError::Truncated {
                needed: 4,
                remaining: 2,
            })
        );
    }

    #[test]
    fn prefix_tlv_rejects_mismatched_prefix_bytes() {
        assert_eq!(
            ThreadPrefixData::new(false, 0, 64, vec![0xfd], Vec::new()),
            Err(MleError::InvalidNetworkDataTlv {
                tlv_type: NetworkDataTlvType::Prefix,
                reason: "prefix byte length does not match prefix length",
            })
        );
    }

    #[test]
    fn connectivity_tlv_round_trips_base_and_sleepy_capacity_fields() {
        let connectivity = Connectivity {
            parent_priority: -1,
            link_quality_3: 3,
            link_quality_2: 2,
            link_quality_1: 1,
            leader_cost: 4,
            id_sequence: 9,
            active_router_count: 16,
            sleepy_end_device_buffer_size: Some(1_280),
            sleepy_end_device_datagram_count: Some(3),
        };

        let parsed = Connectivity::parse(&connectivity.encode()).unwrap();

        assert_eq!(parsed, connectivity);
        assert_eq!(connectivity.to_tlv().tlv_type, TlvType::Connectivity);
        assert!(parsed.has_sleepy_end_device_capacity());

        let base = Connectivity::parse(&[0, 4, 3, 2, 1, 7, 8]).unwrap();
        assert_eq!(base.parent_priority, 0);
        assert_eq!(base.leader_cost, 1);
        assert!(!base.has_sleepy_end_device_capacity());
    }

    #[test]
    fn connectivity_from_message_extracts_diagnostics_tlv() {
        let message = MleMessage {
            command: MleCommand::Advertisement,
            tlvs: vec![Tlv::new(TlvType::Connectivity, vec![1, 3, 2, 1, 5, 8, 12]).unwrap()],
        };

        let connectivity = connectivity_from_message(&message).unwrap().unwrap();

        assert_eq!(connectivity.parent_priority, 1);
        assert_eq!(connectivity.link_quality_3, 3);
        assert_eq!(connectivity.active_router_count, 12);
    }

    #[test]
    fn connectivity_rejects_unknown_length() {
        assert_eq!(
            Connectivity::parse(&[1, 2, 3]),
            Err(MleError::InvalidTlvLength {
                tlv_type: TlvType::Connectivity,
                expected: Connectivity::BASE_ENCODED_LEN,
                actual: 3,
            })
        );
    }

    #[test]
    fn leader_data_rejects_wrong_length_tlv() {
        let message = MleMessage {
            command: MleCommand::DataResponse,
            tlvs: vec![Tlv::new(TlvType::LeaderData, vec![0, 1, 2]).unwrap()],
        };

        assert_eq!(
            leader_data_from_message(&message),
            Err(MleError::InvalidTlvLength {
                tlv_type: TlvType::LeaderData,
                expected: LeaderData::ENCODED_LEN,
                actual: 3,
            })
        );
    }

    #[test]
    fn attach_machine_follows_parent_then_child_id_flow() {
        let mut machine = AttachMachine::new();
        let parent_response = MleMessage {
            command: MleCommand::ParentResponse,
            tlvs: Vec::new(),
        };
        let child_id_response = MleMessage {
            command: MleCommand::ChildIdResponse,
            tlvs: vec![Tlv::new(
                TlvType::Mode,
                vec![Mode {
                    receiver_on_when_idle: false,
                    secure_data_requests: true,
                    full_thread_device: false,
                    full_network_data: true,
                }
                .encode()],
            )
            .unwrap()],
        };

        assert_eq!(machine.start(), AttachAction::SendParentRequest);
        assert_eq!(
            machine.on_message(&parent_response),
            AttachAction::SendChildIdRequest
        );
        assert_eq!(
            machine.on_message(&child_id_response),
            AttachAction::BecomeChild
        );
        assert_eq!(machine.state(), AttachState::Attached(DeviceRole::Child));
    }

    #[test]
    fn attach_machine_can_attach_as_router_candidate() {
        let mut machine = AttachMachine::new();
        machine.start();
        machine.on_message(&MleMessage {
            command: MleCommand::ParentResponse,
            tlvs: Vec::new(),
        });

        let action = machine.on_message(&MleMessage {
            command: MleCommand::ChildIdResponse,
            tlvs: vec![Tlv::new(
                TlvType::Mode,
                vec![Mode {
                    receiver_on_when_idle: true,
                    secure_data_requests: true,
                    full_thread_device: true,
                    full_network_data: true,
                }
                .encode()],
            )
            .unwrap()],
        });

        assert_eq!(action, AttachAction::BecomeRouter);
        assert_eq!(machine.state(), AttachState::Attached(DeviceRole::Router));
    }

    #[test]
    fn malformed_tlv_reports_truncation() {
        assert_eq!(
            MleMessage::parse(&[
                MleCommand::ParentRequest.as_byte(),
                TlvType::Version.as_byte(),
                2,
                1
            ]),
            Err(MleError::Truncated {
                needed: 2,
                remaining: 1
            })
        );
    }

    #[test]
    fn neighbor_table_tracks_parent_children_and_router_candidates() {
        let mut table = NeighborTable::new(DeviceRole::Child);
        table.upsert(
            ThreadNeighbor::new(
                ThreadNeighborId(0x1000),
                DeviceRole::Router,
                NeighborRelationship::Parent,
                1_000,
                10_000,
            )
            .with_link_margin(40),
        );
        table.upsert(ThreadNeighbor::new(
            ThreadNeighborId(0x2000),
            DeviceRole::Child,
            NeighborRelationship::Child,
            1_100,
            5_000,
        ));
        table.upsert(
            ThreadNeighbor::new(
                ThreadNeighborId(0x3000),
                DeviceRole::Router,
                NeighborRelationship::RouterPeer,
                1_200,
                10_000,
            )
            .with_link_margin(60),
        );

        assert_eq!(table.local_role(), DeviceRole::Child);
        assert_eq!(
            table.parent().unwrap().neighbor_id,
            ThreadNeighborId(0x1000)
        );
        assert_eq!(table.children().count(), 1);
        assert_eq!(
            table.best_parent_candidate().unwrap().neighbor_id,
            ThreadNeighborId(0x3000)
        );

        let summary = table.summary_at(1_250);
        assert_eq!(
            summary,
            NeighborTableSummary {
                local_role: DeviceRole::Child,
                neighbor_count: 3,
                parent: Some(ThreadNeighborId(0x1000)),
                child_count: 1,
                router_count: 2,
                stale_neighbor_count: 0,
                best_parent_candidate: Some(ThreadNeighborId(0x3000)),
            }
        );
        assert!(!summary.is_empty());
        assert!(summary.has_parent());
        assert!(!summary.has_stale_neighbors());
        assert!(summary.has_parent_candidate());
        assert!(!summary.needs_attach());
        assert!(summary.has_routing_surface());
    }

    #[test]
    fn diagnostic_snapshot_combines_neighbors_and_mle_data() {
        let mut table = NeighborTable::new(DeviceRole::Child);
        table.upsert(
            ThreadNeighbor::new(
                ThreadNeighborId(0x1000),
                DeviceRole::Router,
                NeighborRelationship::Parent,
                1_000,
                10_000,
            )
            .with_link_margin(50),
        );
        table.upsert(
            ThreadNeighbor::new(
                ThreadNeighborId(0x3000),
                DeviceRole::Router,
                NeighborRelationship::RouterPeer,
                1_100,
                10_000,
            )
            .with_link_margin(60),
        );
        table.upsert(ThreadNeighbor::new(
            ThreadNeighborId(0x4000),
            DeviceRole::Child,
            NeighborRelationship::Child,
            1_200,
            10_000,
        ));

        let leader_data = LeaderData {
            partition_id: 0x0102_0304,
            weighting: 32,
            data_version: 4,
            stable_data_version: 3,
            leader_router_id: 1,
        };
        let prefix = ThreadPrefixData::new(
            true,
            0,
            64,
            vec![0xfd, 0, 0xab, 0xcd, 0, 0, 0, 0],
            Vec::new(),
        )
        .unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![prefix.to_tlv().unwrap()]).unwrap();
        let connectivity = Connectivity {
            parent_priority: 0,
            link_quality_3: 3,
            link_quality_2: 2,
            link_quality_1: 1,
            leader_cost: 2,
            id_sequence: 9,
            active_router_count: 12,
            sleepy_end_device_buffer_size: None,
            sleepy_end_device_datagram_count: None,
        };
        let message = MleMessage {
            command: MleCommand::Advertisement,
            tlvs: vec![
                leader_data.to_tlv(),
                network_data.to_tlv(),
                connectivity.to_tlv(),
            ],
        };

        let snapshot = table.diagnostic_snapshot(Some(&message), 2_000).unwrap();

        assert_eq!(snapshot.local_role, DeviceRole::Child);
        assert_eq!(snapshot.parent, Some(ThreadNeighborId(0x1000)));
        assert_eq!(snapshot.router_count, 2);
        assert_eq!(snapshot.child_count, 1);
        assert!(snapshot.stale_neighbors.is_empty());
        assert_eq!(
            snapshot.best_parent_candidate,
            Some(ThreadNeighborId(0x3000))
        );
        assert_eq!(snapshot.partition_id(), Some(0x0102_0304));
        assert_eq!(snapshot.active_router_count(), Some(12));
        assert_eq!(snapshot.prefixes, vec![prefix]);
        assert_eq!(snapshot.health(), ThreadDiagnosticHealth::Healthy);
        assert_eq!(
            snapshot.supervision_plan(),
            ThreadSupervisionPlan {
                health: ThreadDiagnosticHealth::Healthy,
                action: ThreadSupervisionAction::Observe,
                parent: Some(ThreadNeighborId(0x1000)),
                best_parent_candidate: Some(ThreadNeighborId(0x3000)),
            }
        );
        assert!(!snapshot.supervision_plan().needs_intervention());
        assert_eq!(ThreadSupervisionAction::Observe.as_str(), "observe");
    }

    #[test]
    fn diagnostic_snapshot_flags_detached_and_stale_parent_states() {
        let detached = NeighborTable::new(DeviceRole::Detached);
        let detached_snapshot = detached.diagnostic_snapshot(None, 10_000).unwrap();

        assert_eq!(detached_snapshot.health(), ThreadDiagnosticHealth::Detached);

        let mut child = NeighborTable::new(DeviceRole::Child);
        child.upsert(ThreadNeighbor::new(
            ThreadNeighborId(0x1000),
            DeviceRole::Router,
            NeighborRelationship::Parent,
            1_000,
            500,
        ));

        let stale_snapshot = child.diagnostic_snapshot(None, 2_000).unwrap();

        assert_eq!(
            stale_snapshot.stale_neighbors,
            vec![ThreadNeighborId(0x1000)]
        );
        assert_eq!(stale_snapshot.health(), ThreadDiagnosticHealth::Degraded);
    }

    #[test]
    fn supervision_plan_projects_repair_actions_from_diagnostics() {
        let disabled = NeighborTable::new(DeviceRole::Disabled);
        let disabled_plan = disabled
            .diagnostic_snapshot(None, 10_000)
            .unwrap()
            .supervision_plan();
        assert_eq!(disabled_plan.health, ThreadDiagnosticHealth::Offline);
        assert_eq!(
            disabled_plan.action,
            ThreadSupervisionAction::EnableInterface
        );
        assert_eq!(
            ThreadSupervisionAction::EnableInterface.as_str(),
            "enable_interface"
        );
        assert!(disabled_plan.needs_intervention());

        let detached = NeighborTable::new(DeviceRole::Detached);
        let detached_plan = detached
            .diagnostic_snapshot(None, 10_000)
            .unwrap()
            .supervision_plan();
        assert_eq!(detached_plan.action, ThreadSupervisionAction::StartAttach);

        let child_without_parent = NeighborTable::new(DeviceRole::Child);
        let child_plan = child_without_parent
            .diagnostic_snapshot(None, 10_000)
            .unwrap()
            .supervision_plan();
        assert_eq!(child_plan.action, ThreadSupervisionAction::StartAttach);

        let mut child_with_stale_parent = NeighborTable::new(DeviceRole::Child);
        child_with_stale_parent.upsert(ThreadNeighbor::new(
            ThreadNeighborId(0x1000),
            DeviceRole::Router,
            NeighborRelationship::Parent,
            1_000,
            500,
        ));
        let stale_parent_plan = child_with_stale_parent
            .diagnostic_snapshot(None, 2_000)
            .unwrap()
            .supervision_plan();
        assert_eq!(
            stale_parent_plan.action,
            ThreadSupervisionAction::RefreshParent
        );

        let router = NeighborTable::new(DeviceRole::Router);
        let message = MleMessage {
            command: MleCommand::Advertisement,
            tlvs: vec![Connectivity {
                parent_priority: 0,
                link_quality_3: 0,
                link_quality_2: 0,
                link_quality_1: 0,
                leader_cost: 0,
                id_sequence: 1,
                active_router_count: 0,
                sleepy_end_device_buffer_size: None,
                sleepy_end_device_datagram_count: None,
            }
            .to_tlv()],
        };
        let router_plan = router
            .diagnostic_snapshot(Some(&message), 10_000)
            .unwrap()
            .supervision_plan();
        assert_eq!(
            router_plan.action,
            ThreadSupervisionAction::RefreshRouterConnectivity
        );
    }

    #[test]
    fn neighbor_table_expires_stale_neighbors_and_clears_parent() {
        let mut table = NeighborTable::new(DeviceRole::Child);
        table.upsert(ThreadNeighbor::new(
            ThreadNeighborId(0x1000),
            DeviceRole::Router,
            NeighborRelationship::Parent,
            1_000,
            500,
        ));

        assert!(table.stale_neighbors_at(1_499).is_empty());
        assert_eq!(table.summary_at(1_500).stale_neighbor_count, 1);
        assert!(table.summary_at(1_500).has_stale_neighbors());
        assert_eq!(table.expire_stale(1_500), vec![ThreadNeighborId(0x1000)]);
        assert!(table.parent().is_none());
        assert!(table.is_empty());
        assert!(table.summary_at(1_500).is_empty());
    }

    #[test]
    fn parent_response_builds_neighbor_from_mle_tlvs() {
        let message = MleMessage {
            command: MleCommand::ParentResponse,
            tlvs: vec![
                Tlv::new(
                    TlvType::Mode,
                    vec![Mode {
                        receiver_on_when_idle: true,
                        secure_data_requests: true,
                        full_thread_device: true,
                        full_network_data: true,
                    }
                    .encode()],
                )
                .unwrap(),
                Tlv::new(TlvType::LinkMargin, vec![73]).unwrap(),
                Tlv::new(TlvType::Timeout, 30_u32.to_be_bytes().to_vec()).unwrap(),
            ],
        };

        let neighbor =
            neighbor_from_parent_response(ThreadNeighborId(0x1234), &message, 9_000, 5_000);

        assert_eq!(neighbor.role, DeviceRole::Router);
        assert_eq!(neighbor.relationship, NeighborRelationship::Parent);
        assert_eq!(neighbor.metrics.link_margin, Some(73));
        assert_eq!(neighbor.timeout_ms, 30_000);
    }

    #[test]
    fn parent_response_without_mode_still_tracks_router_parent() {
        let message = MleMessage {
            command: MleCommand::ParentResponse,
            tlvs: vec![Tlv::new(TlvType::LinkMargin, vec![55]).unwrap()],
        };

        let neighbor =
            neighbor_from_parent_response(ThreadNeighborId(0x1234), &message, 9_000, 5_000);

        assert_eq!(neighbor.role, DeviceRole::Router);
        assert_eq!(neighbor.relationship, NeighborRelationship::Parent);
        assert_eq!(neighbor.metrics.link_margin, Some(55));
        assert_eq!(neighbor.timeout_ms, 5_000);
    }
}
