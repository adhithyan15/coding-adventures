//! Thread Mesh Link Establishment primitives.
//!
//! MLE is the Thread control plane for roles, neighbors, parent/child attach,
//! and network data exchange. This crate starts with pure message/TLV parsing
//! and a deterministic attach-state skeleton. It intentionally performs no UDP,
//! CoAP, DTLS, radio, commissioning, or border-router I/O.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt;

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
        assert_eq!(table.expire_stale(1_500), vec![ThreadNeighborId(0x1000)]);
        assert!(table.parent().is_none());
        assert!(table.is_empty());
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
