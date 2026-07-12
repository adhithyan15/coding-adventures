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

    pub fn summary(&self) -> Result<ThreadNetworkDataSummary, MleError> {
        ThreadNetworkDataSummary::from_network_data(self)
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ThreadNetworkDataSummary {
    pub byte_len: usize,
    pub top_level_tlvs: usize,
    pub stable_top_level_tlvs: usize,
    pub prefix_tlvs: usize,
    pub stable_prefix_tlvs: usize,
    pub prefix_sub_tlvs: usize,
    pub stable_prefix_sub_tlvs: usize,
    pub has_route_tlvs: usize,
    pub border_router_tlvs: usize,
    pub lowpan_id_tlvs: usize,
    pub commissioning_data_tlvs: usize,
    pub service_tlvs: usize,
    pub server_tlvs: usize,
    pub context_tlvs: usize,
    pub unknown_tlvs: usize,
}

impl ThreadNetworkDataSummary {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_network_data(network_data: &ThreadNetworkData) -> Result<Self, MleError> {
        let tlvs = network_data.tlvs()?;
        let mut summary = Self {
            byte_len: network_data.len(),
            ..Self::empty()
        };

        for tlv in &tlvs {
            summary.top_level_tlvs += 1;
            if tlv.stable {
                summary.stable_top_level_tlvs += 1;
            }
            summary.add_tlv_type(tlv.tlv_type);

            if tlv.tlv_type == NetworkDataTlvType::Prefix {
                let prefix = ThreadPrefixData::parse(tlv)?;
                summary.prefix_tlvs += 1;
                if prefix.stable {
                    summary.stable_prefix_tlvs += 1;
                }
                summary.prefix_sub_tlvs += prefix.sub_tlvs.len();
                for sub_tlv in &prefix.sub_tlvs {
                    if sub_tlv.stable {
                        summary.stable_prefix_sub_tlvs += 1;
                    }
                    summary.add_tlv_type(sub_tlv.tlv_type);
                }
            }
        }

        Ok(summary)
    }

    pub fn is_empty(self) -> bool {
        self.top_level_tlvs == 0
    }

    pub fn has_prefixes(self) -> bool {
        self.prefix_tlvs > 0
    }

    pub fn has_stable_data(self) -> bool {
        self.stable_top_level_tlvs > 0 || self.stable_prefix_sub_tlvs > 0
    }

    pub fn has_routing_data(self) -> bool {
        self.prefix_tlvs > 0 || self.has_route_tlvs > 0 || self.border_router_tlvs > 0
    }

    pub fn has_services(self) -> bool {
        self.service_tlvs > 0 || self.server_tlvs > 0
    }

    pub fn has_unknown_tlvs(self) -> bool {
        self.unknown_tlvs > 0
    }

    pub fn has_service_or_context_data(self) -> bool {
        self.has_services() || self.context_tlvs > 0
    }

    pub fn readiness(self) -> ThreadNetworkDataReadinessSummary {
        ThreadNetworkDataReadinessSummary::from_summary(self)
    }

    fn add_tlv_type(&mut self, tlv_type: NetworkDataTlvType) {
        match tlv_type {
            NetworkDataTlvType::HasRoute => self.has_route_tlvs += 1,
            NetworkDataTlvType::BorderRouter => self.border_router_tlvs += 1,
            NetworkDataTlvType::LowpanId => self.lowpan_id_tlvs += 1,
            NetworkDataTlvType::CommissioningData => self.commissioning_data_tlvs += 1,
            NetworkDataTlvType::Service => self.service_tlvs += 1,
            NetworkDataTlvType::Server => self.server_tlvs += 1,
            NetworkDataTlvType::Context => self.context_tlvs += 1,
            NetworkDataTlvType::Unknown(_) => self.unknown_tlvs += 1,
            NetworkDataTlvType::Prefix => {}
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadNetworkDataReadinessSummary {
    pub network_data_summary: ThreadNetworkDataSummary,
    pub required_check_count: usize,
    pub passed_check_count: usize,
    pub missing_check_count: usize,
    pub network_data_present: bool,
    pub prefix_coverage_ready: bool,
    pub routing_coverage_ready: bool,
    pub stable_data_ready: bool,
    pub service_or_context_ready: bool,
    pub unknown_tlvs_absent: bool,
    pub network_data_ready: bool,
}

impl ThreadNetworkDataReadinessSummary {
    pub fn from_network_data(network_data: &ThreadNetworkData) -> Result<Self, MleError> {
        Ok(Self::from_summary(network_data.summary()?))
    }

    pub fn from_summary(network_data_summary: ThreadNetworkDataSummary) -> Self {
        let network_data_present = !network_data_summary.is_empty();
        let prefix_coverage_ready = network_data_summary.has_prefixes();
        let routing_coverage_ready = network_data_summary.has_routing_data();
        let stable_data_ready = network_data_summary.has_stable_data();
        let service_or_context_ready = network_data_summary.has_service_or_context_data();
        let unknown_tlvs_absent = !network_data_summary.has_unknown_tlvs();
        let checks = [
            network_data_present,
            prefix_coverage_ready,
            routing_coverage_ready,
            stable_data_ready,
            service_or_context_ready,
            unknown_tlvs_absent,
        ];
        let passed_check_count = checks.iter().filter(|ready| **ready).count();
        let required_check_count = checks.len();
        let missing_check_count = required_check_count - passed_check_count;
        let network_data_ready = missing_check_count == 0;

        Self {
            network_data_summary,
            required_check_count,
            passed_check_count,
            missing_check_count,
            network_data_present,
            prefix_coverage_ready,
            routing_coverage_ready,
            stable_data_ready,
            service_or_context_ready,
            unknown_tlvs_absent,
            network_data_ready,
        }
    }

    pub fn is_network_data_ready(self) -> bool {
        self.network_data_ready
    }

    pub fn has_missing_checks(self) -> bool {
        self.missing_check_count > 0
    }

    pub fn needs_network_data(self) -> bool {
        !self.network_data_present
    }

    pub fn needs_prefix_coverage(self) -> bool {
        !self.prefix_coverage_ready
    }

    pub fn needs_routing_coverage(self) -> bool {
        !self.routing_coverage_ready
    }

    pub fn needs_stable_data(self) -> bool {
        !self.stable_data_ready
    }

    pub fn needs_service_or_context_data(self) -> bool {
        !self.service_or_context_ready
    }

    pub fn has_unknown_tlv_gaps(self) -> bool {
        !self.unknown_tlvs_absent
    }
}

pub fn summarize_thread_network_data_readiness(
    network_data: &ThreadNetworkData,
) -> Result<ThreadNetworkDataReadinessSummary, MleError> {
    ThreadNetworkDataReadinessSummary::from_network_data(network_data)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadNetworkDataTlvHandoffSummary {
    pub network_data_readiness: ThreadNetworkDataReadinessSummary,
    pub required_handoff_check_count: usize,
    pub passed_handoff_check_count: usize,
    pub missing_handoff_check_count: usize,
    pub network_data_ready: bool,
    pub stable_tlvs_ready: bool,
    pub routing_tlvs_ready: bool,
    pub service_or_context_tlvs_ready: bool,
    pub unknown_tlvs_absent: bool,
    pub tlv_handoff_ready: bool,
}

impl ThreadNetworkDataTlvHandoffSummary {
    pub fn from_network_data(network_data: &ThreadNetworkData) -> Result<Self, MleError> {
        Ok(Self::from_readiness(
            ThreadNetworkDataReadinessSummary::from_network_data(network_data)?,
        ))
    }

    pub fn from_readiness(network_data_readiness: ThreadNetworkDataReadinessSummary) -> Self {
        let network_data_summary = network_data_readiness.network_data_summary;
        let network_data_ready = network_data_readiness.is_network_data_ready();
        let stable_tlvs_ready = network_data_summary.has_stable_data();
        let routing_tlvs_ready = network_data_summary.has_routing_data();
        let service_or_context_tlvs_ready = network_data_summary.has_service_or_context_data();
        let unknown_tlvs_absent = !network_data_summary.has_unknown_tlvs();
        let checks = [
            network_data_ready,
            stable_tlvs_ready,
            routing_tlvs_ready,
            service_or_context_tlvs_ready,
            unknown_tlvs_absent,
        ];
        let passed_handoff_check_count = checks.iter().filter(|ready| **ready).count();
        let required_handoff_check_count = checks.len();
        let missing_handoff_check_count = required_handoff_check_count - passed_handoff_check_count;
        let tlv_handoff_ready = missing_handoff_check_count == 0;

        Self {
            network_data_readiness,
            required_handoff_check_count,
            passed_handoff_check_count,
            missing_handoff_check_count,
            network_data_ready,
            stable_tlvs_ready,
            routing_tlvs_ready,
            service_or_context_tlvs_ready,
            unknown_tlvs_absent,
            tlv_handoff_ready,
        }
    }

    pub fn is_tlv_handoff_ready(self) -> bool {
        self.tlv_handoff_ready
    }

    pub fn has_handoff_gaps(self) -> bool {
        self.missing_handoff_check_count > 0
    }

    pub fn needs_network_data(self) -> bool {
        !self.network_data_ready
    }

    pub fn needs_stable_tlvs(self) -> bool {
        !self.stable_tlvs_ready
    }

    pub fn needs_routing_tlvs(self) -> bool {
        !self.routing_tlvs_ready
    }

    pub fn needs_service_or_context_tlvs(self) -> bool {
        !self.service_or_context_tlvs_ready
    }

    pub fn needs_unknown_tlv_review(self) -> bool {
        !self.unknown_tlvs_absent
    }
}

pub fn summarize_thread_network_data_tlv_handoff(
    network_data: &ThreadNetworkData,
) -> Result<ThreadNetworkDataTlvHandoffSummary, MleError> {
    ThreadNetworkDataTlvHandoffSummary::from_network_data(network_data)
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

    pub fn network_data_summary(&self) -> Result<ThreadNetworkDataSummary, MleError> {
        self.network_data
            .as_ref()
            .map(ThreadNetworkData::summary)
            .unwrap_or_else(|| Ok(ThreadNetworkDataSummary::empty()))
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MleStatus {
    pub code: u8,
}

impl MleStatus {
    pub const ENCODED_LEN: usize = 1;

    pub fn parse(value: &[u8]) -> Result<Self, MleError> {
        if value.len() != Self::ENCODED_LEN {
            return Err(MleError::InvalidTlvLength {
                tlv_type: TlvType::Status,
                expected: Self::ENCODED_LEN,
                actual: value.len(),
            });
        }
        Ok(Self { code: value[0] })
    }

    pub fn encode(self) -> [u8; Self::ENCODED_LEN] {
        [self.code]
    }

    pub fn to_tlv(self) -> Tlv {
        Tlv {
            tlv_type: TlvType::Status,
            value: self.encode().to_vec(),
        }
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

    pub fn summary(&self) -> MleMessageSummary {
        MleMessageSummary::from_message(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MleMessageSummary {
    pub command: MleCommand,
    pub tlv_count: usize,
    pub has_scan_mask: bool,
    pub has_mode: bool,
    pub has_timeout: bool,
    pub has_leader_data: bool,
    pub has_network_data: bool,
    pub has_connectivity: bool,
    pub has_status: bool,
    pub has_version: bool,
}

impl MleMessageSummary {
    pub fn from_message(message: &MleMessage) -> Self {
        let mut summary = Self {
            command: message.command,
            tlv_count: message.tlvs.len(),
            has_scan_mask: false,
            has_mode: false,
            has_timeout: false,
            has_leader_data: false,
            has_network_data: false,
            has_connectivity: false,
            has_status: false,
            has_version: false,
        };

        for tlv in &message.tlvs {
            match tlv.tlv_type {
                TlvType::ScanMask => summary.has_scan_mask = true,
                TlvType::Mode => summary.has_mode = true,
                TlvType::Timeout => summary.has_timeout = true,
                TlvType::LeaderData => summary.has_leader_data = true,
                TlvType::NetworkData => summary.has_network_data = true,
                TlvType::Connectivity => summary.has_connectivity = true,
                TlvType::Status => summary.has_status = true,
                TlvType::Version => summary.has_version = true,
                _ => {}
            }
        }

        summary
    }

    pub fn is_empty(&self) -> bool {
        self.tlv_count == 0
    }

    pub fn has_parent_selection_request_context(&self) -> bool {
        self.command == MleCommand::ParentRequest && self.has_scan_mask && self.has_version
    }

    pub fn has_attach_response_context(&self) -> bool {
        matches!(
            self.command,
            MleCommand::ParentResponse | MleCommand::ChildIdResponse
        ) && self.has_mode
    }

    pub fn has_diagnostic_context(&self) -> bool {
        self.has_leader_data || self.has_network_data || self.has_connectivity
    }

    pub fn has_thread_data_versions(&self) -> bool {
        self.has_leader_data && self.has_network_data
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MleMessageBatchSummary {
    pub total_messages: usize,
    pub total_tlvs: usize,
    pub empty_messages: usize,
    pub parent_selection_request_messages: usize,
    pub attach_response_messages: usize,
    pub diagnostic_messages: usize,
    pub thread_data_version_messages: usize,
    pub status_messages: usize,
    pub network_data_messages: usize,
    pub connectivity_messages: usize,
    pub unknown_command_messages: usize,
}

impl MleMessageBatchSummary {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_messages<'a>(messages: impl IntoIterator<Item = &'a MleMessage>) -> Self {
        let mut summary = Self::empty();
        for message in messages {
            summary.record_summary(&message.summary());
        }
        summary
    }

    pub fn from_summaries<'a>(summaries: impl IntoIterator<Item = &'a MleMessageSummary>) -> Self {
        let mut summary = Self::empty();
        for message_summary in summaries {
            summary.record_summary(message_summary);
        }
        summary
    }

    pub fn record_summary(&mut self, summary: &MleMessageSummary) {
        self.total_messages += 1;
        self.total_tlvs += summary.tlv_count;

        if summary.is_empty() {
            self.empty_messages += 1;
        }
        if summary.has_parent_selection_request_context() {
            self.parent_selection_request_messages += 1;
        }
        if summary.has_attach_response_context() {
            self.attach_response_messages += 1;
        }
        if summary.has_diagnostic_context() {
            self.diagnostic_messages += 1;
        }
        if summary.has_thread_data_versions() {
            self.thread_data_version_messages += 1;
        }
        if summary.has_status {
            self.status_messages += 1;
        }
        if summary.has_network_data {
            self.network_data_messages += 1;
        }
        if summary.has_connectivity {
            self.connectivity_messages += 1;
        }
        if matches!(summary.command, MleCommand::Unknown(_)) {
            self.unknown_command_messages += 1;
        }
    }

    pub fn is_empty(self) -> bool {
        self.total_messages == 0
    }

    pub fn has_parent_selection_requests(self) -> bool {
        self.parent_selection_request_messages > 0
    }

    pub fn has_attach_responses(self) -> bool {
        self.attach_response_messages > 0
    }

    pub fn has_diagnostics(self) -> bool {
        self.diagnostic_messages > 0
    }

    pub fn has_statuses(self) -> bool {
        self.status_messages > 0
    }

    pub fn has_unknown_commands(self) -> bool {
        self.unknown_command_messages > 0
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadAttachReadinessSummary {
    pub message_summary: MleMessageBatchSummary,
    pub neighbor_summary: NeighborTableSummary,
    pub attached: bool,
    pub attach_ready: bool,
    pub parent_selection_requested: bool,
    pub attach_response_seen: bool,
    pub parent_candidate_available: bool,
    pub needs_parent_selection: bool,
    pub waiting_for_attach_response: bool,
    pub requires_neighbor_refresh: bool,
}

impl ThreadAttachReadinessSummary {
    pub fn from_summaries(
        message_summary: MleMessageBatchSummary,
        neighbor_summary: NeighborTableSummary,
    ) -> Self {
        let attached = !neighbor_summary.needs_attach();
        let parent_selection_requested = message_summary.has_parent_selection_requests();
        let attach_response_seen = message_summary.has_attach_responses();
        let parent_candidate_available = neighbor_summary.has_parent_candidate();
        let attach_ready = attached
            || (parent_selection_requested && attach_response_seen && parent_candidate_available);
        let needs_parent_selection = neighbor_summary.needs_attach() && !parent_selection_requested;
        let waiting_for_attach_response =
            neighbor_summary.needs_attach() && parent_selection_requested && !attach_response_seen;

        Self {
            message_summary,
            neighbor_summary,
            attached,
            attach_ready,
            parent_selection_requested,
            attach_response_seen,
            parent_candidate_available,
            needs_parent_selection,
            waiting_for_attach_response,
            requires_neighbor_refresh: neighbor_summary.has_stale_neighbors(),
        }
    }

    pub fn has_diagnostics(self) -> bool {
        self.message_summary.has_diagnostics()
    }

    pub fn has_statuses(self) -> bool {
        self.message_summary.has_statuses()
    }

    pub fn has_unknown_commands(self) -> bool {
        self.message_summary.has_unknown_commands()
    }
}

pub fn summarize_thread_attach_readiness(
    message_summary: MleMessageBatchSummary,
    neighbor_summary: NeighborTableSummary,
) -> ThreadAttachReadinessSummary {
    ThreadAttachReadinessSummary::from_summaries(message_summary, neighbor_summary)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadAttachActionSummary {
    pub readiness_summary: ThreadAttachReadinessSummary,
    pub required_action_count: usize,
    pub pending_action_count: usize,
    pub clear_action_count: usize,
    pub start_parent_selection: bool,
    pub wait_for_attach_response: bool,
    pub refresh_neighbors: bool,
    pub inspect_statuses: bool,
    pub inspect_unknown_commands: bool,
    pub attach_action_clear: bool,
}

impl ThreadAttachActionSummary {
    pub fn from_readiness(readiness_summary: ThreadAttachReadinessSummary) -> Self {
        let start_parent_selection = readiness_summary.needs_parent_selection;
        let wait_for_attach_response = readiness_summary.waiting_for_attach_response;
        let refresh_neighbors = readiness_summary.requires_neighbor_refresh;
        let inspect_statuses = readiness_summary.has_statuses();
        let inspect_unknown_commands = readiness_summary.has_unknown_commands();
        let actions = [
            start_parent_selection,
            wait_for_attach_response,
            refresh_neighbors,
            inspect_statuses,
            inspect_unknown_commands,
        ];
        let pending_action_count = actions.iter().filter(|pending| **pending).count();
        let required_action_count = actions.len();
        let clear_action_count = required_action_count - pending_action_count;
        let attach_action_clear = readiness_summary.attach_ready && pending_action_count == 0;

        Self {
            readiness_summary,
            required_action_count,
            pending_action_count,
            clear_action_count,
            start_parent_selection,
            wait_for_attach_response,
            refresh_neighbors,
            inspect_statuses,
            inspect_unknown_commands,
            attach_action_clear,
        }
    }

    pub fn from_summaries(
        message_summary: MleMessageBatchSummary,
        neighbor_summary: NeighborTableSummary,
    ) -> Self {
        Self::from_readiness(summarize_thread_attach_readiness(
            message_summary,
            neighbor_summary,
        ))
    }

    pub fn has_pending_actions(self) -> bool {
        self.pending_action_count > 0
    }

    pub fn is_attach_action_clear(self) -> bool {
        self.attach_action_clear
    }

    pub fn needs_parent_selection(self) -> bool {
        self.start_parent_selection
    }

    pub fn waiting_on_attach_response(self) -> bool {
        self.wait_for_attach_response
    }

    pub fn needs_neighbor_refresh(self) -> bool {
        self.refresh_neighbors
    }

    pub fn needs_status_review(self) -> bool {
        self.inspect_statuses
    }

    pub fn needs_unknown_command_review(self) -> bool {
        self.inspect_unknown_commands
    }
}

pub fn summarize_thread_attach_actions(
    readiness_summary: ThreadAttachReadinessSummary,
) -> ThreadAttachActionSummary {
    ThreadAttachActionSummary::from_readiness(readiness_summary)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadAttachCompletionSummary {
    pub action_summary: ThreadAttachActionSummary,
    pub supervision_plan: ThreadSupervisionPlan,
    pub required_completion_check_count: usize,
    pub passed_completion_check_count: usize,
    pub missing_completion_check_count: usize,
    pub actions_clear: bool,
    pub supervision_clear: bool,
    pub attached_or_attach_ready: bool,
    pub review_queues_clear: bool,
    pub attach_complete: bool,
}

impl ThreadAttachCompletionSummary {
    pub fn from_action_and_supervision(
        action_summary: ThreadAttachActionSummary,
        supervision_plan: ThreadSupervisionPlan,
    ) -> Self {
        let actions_clear = action_summary.is_attach_action_clear();
        let supervision_clear = !supervision_plan.needs_intervention();
        let attached_or_attach_ready = action_summary.readiness_summary.attached
            || action_summary.readiness_summary.attach_ready;
        let review_queues_clear =
            !action_summary.needs_status_review() && !action_summary.needs_unknown_command_review();
        let checks = [
            actions_clear,
            supervision_clear,
            attached_or_attach_ready,
            review_queues_clear,
        ];
        let passed_completion_check_count = checks.iter().filter(|ready| **ready).count();
        let required_completion_check_count = checks.len();
        let missing_completion_check_count =
            required_completion_check_count - passed_completion_check_count;
        let attach_complete = missing_completion_check_count == 0;

        Self {
            action_summary,
            supervision_plan,
            required_completion_check_count,
            passed_completion_check_count,
            missing_completion_check_count,
            actions_clear,
            supervision_clear,
            attached_or_attach_ready,
            review_queues_clear,
            attach_complete,
        }
    }

    pub fn is_attach_complete(self) -> bool {
        self.attach_complete
    }

    pub fn has_completion_gaps(self) -> bool {
        self.missing_completion_check_count > 0
    }

    pub fn needs_action_clearance(self) -> bool {
        !self.actions_clear
    }

    pub fn needs_supervision_clearance(self) -> bool {
        !self.supervision_clear
    }

    pub fn needs_attach_readiness(self) -> bool {
        !self.attached_or_attach_ready
    }

    pub fn needs_review_queue_clearance(self) -> bool {
        !self.review_queues_clear
    }
}

pub fn summarize_thread_attach_completion(
    action_summary: ThreadAttachActionSummary,
    supervision_plan: ThreadSupervisionPlan,
) -> ThreadAttachCompletionSummary {
    ThreadAttachCompletionSummary::from_action_and_supervision(action_summary, supervision_plan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadAttachRouteHandoffSummary {
    pub completion_summary: ThreadAttachCompletionSummary,
    pub network_data_readiness: ThreadNetworkDataReadinessSummary,
    pub required_handoff_check_count: usize,
    pub passed_handoff_check_count: usize,
    pub missing_handoff_check_count: usize,
    pub attach_complete: bool,
    pub network_data_ready: bool,
    pub routing_surface_ready: bool,
    pub parent_or_route_anchor_ready: bool,
    pub route_handoff_ready: bool,
}

impl ThreadAttachRouteHandoffSummary {
    pub fn from_completion_and_network_data(
        completion_summary: ThreadAttachCompletionSummary,
        network_data_readiness: ThreadNetworkDataReadinessSummary,
    ) -> Self {
        let neighbor_summary = completion_summary
            .action_summary
            .readiness_summary
            .neighbor_summary;
        let attach_complete = completion_summary.is_attach_complete();
        let network_data_ready = network_data_readiness.is_network_data_ready();
        let routing_surface_ready = neighbor_summary.has_routing_surface()
            && network_data_readiness
                .network_data_summary
                .has_routing_data();
        let parent_or_route_anchor_ready = neighbor_summary.has_parent()
            || neighbor_summary.has_parent_candidate()
            || neighbor_summary.local_role.can_route();
        let checks = [
            attach_complete,
            network_data_ready,
            routing_surface_ready,
            parent_or_route_anchor_ready,
        ];
        let passed_handoff_check_count = checks.iter().filter(|ready| **ready).count();
        let required_handoff_check_count = checks.len();
        let missing_handoff_check_count = required_handoff_check_count - passed_handoff_check_count;
        let route_handoff_ready = missing_handoff_check_count == 0;

        Self {
            completion_summary,
            network_data_readiness,
            required_handoff_check_count,
            passed_handoff_check_count,
            missing_handoff_check_count,
            attach_complete,
            network_data_ready,
            routing_surface_ready,
            parent_or_route_anchor_ready,
            route_handoff_ready,
        }
    }

    pub fn is_route_handoff_ready(self) -> bool {
        self.route_handoff_ready
    }

    pub fn has_handoff_gaps(self) -> bool {
        self.missing_handoff_check_count > 0
    }

    pub fn needs_attach_completion(self) -> bool {
        !self.attach_complete
    }

    pub fn needs_network_data(self) -> bool {
        !self.network_data_ready
    }

    pub fn needs_routing_surface(self) -> bool {
        !self.routing_surface_ready
    }

    pub fn needs_parent_or_route_anchor(self) -> bool {
        !self.parent_or_route_anchor_ready
    }
}

pub fn summarize_thread_attach_route_handoff(
    completion_summary: ThreadAttachCompletionSummary,
    network_data_readiness: ThreadNetworkDataReadinessSummary,
) -> ThreadAttachRouteHandoffSummary {
    ThreadAttachRouteHandoffSummary::from_completion_and_network_data(
        completion_summary,
        network_data_readiness,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadAttachRouteAuditSummary {
    pub handoff_summary: ThreadAttachRouteHandoffSummary,
    pub required_audit_check_count: usize,
    pub passed_audit_check_count: usize,
    pub missing_audit_check_count: usize,
    pub route_handoff_ready: bool,
    pub attach_complete: bool,
    pub network_data_ready: bool,
    pub routing_surface_ready: bool,
    pub parent_or_route_anchor_ready: bool,
    pub route_audit_ready: bool,
}

impl ThreadAttachRouteAuditSummary {
    pub fn from_handoff_summary(handoff_summary: ThreadAttachRouteHandoffSummary) -> Self {
        let route_handoff_ready = handoff_summary.is_route_handoff_ready();
        let attach_complete = !handoff_summary.needs_attach_completion();
        let network_data_ready = !handoff_summary.needs_network_data();
        let routing_surface_ready = !handoff_summary.needs_routing_surface();
        let parent_or_route_anchor_ready = !handoff_summary.needs_parent_or_route_anchor();
        let checks = [
            route_handoff_ready,
            attach_complete,
            network_data_ready,
            routing_surface_ready,
            parent_or_route_anchor_ready,
        ];
        let passed_audit_check_count = checks.iter().filter(|ready| **ready).count();
        let required_audit_check_count = checks.len();
        let missing_audit_check_count = required_audit_check_count - passed_audit_check_count;
        let route_audit_ready = missing_audit_check_count == 0;

        Self {
            handoff_summary,
            required_audit_check_count,
            passed_audit_check_count,
            missing_audit_check_count,
            route_handoff_ready,
            attach_complete,
            network_data_ready,
            routing_surface_ready,
            parent_or_route_anchor_ready,
            route_audit_ready,
        }
    }

    pub fn is_route_audit_ready(self) -> bool {
        self.route_audit_ready
    }

    pub fn has_audit_gaps(self) -> bool {
        self.missing_audit_check_count > 0
    }

    pub fn needs_route_handoff(self) -> bool {
        !self.route_handoff_ready
    }

    pub fn needs_attach_completion(self) -> bool {
        !self.attach_complete
    }

    pub fn needs_network_data(self) -> bool {
        !self.network_data_ready
    }

    pub fn needs_routing_surface(self) -> bool {
        !self.routing_surface_ready
    }

    pub fn needs_parent_or_route_anchor(self) -> bool {
        !self.parent_or_route_anchor_ready
    }
}

pub fn summarize_thread_attach_route_audit(
    handoff_summary: ThreadAttachRouteHandoffSummary,
) -> ThreadAttachRouteAuditSummary {
    ThreadAttachRouteAuditSummary::from_handoff_summary(handoff_summary)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadAttachRouteSignoffSummary {
    pub audit_summary: ThreadAttachRouteAuditSummary,
    pub required_signoff_check_count: usize,
    pub passed_signoff_check_count: usize,
    pub missing_signoff_check_count: usize,
    pub route_audit_ready: bool,
    pub route_handoff_ready: bool,
    pub attach_complete: bool,
    pub network_data_ready: bool,
    pub routing_surface_ready: bool,
    pub parent_or_route_anchor_ready: bool,
    pub route_signoff_ready: bool,
}

impl ThreadAttachRouteSignoffSummary {
    pub fn from_audit_summary(audit_summary: ThreadAttachRouteAuditSummary) -> Self {
        let route_audit_ready = audit_summary.is_route_audit_ready();
        let route_handoff_ready = !audit_summary.needs_route_handoff();
        let attach_complete = !audit_summary.needs_attach_completion();
        let network_data_ready = !audit_summary.needs_network_data();
        let routing_surface_ready = !audit_summary.needs_routing_surface();
        let parent_or_route_anchor_ready = !audit_summary.needs_parent_or_route_anchor();
        let checks = [
            route_audit_ready,
            route_handoff_ready,
            attach_complete,
            network_data_ready,
            routing_surface_ready,
            parent_or_route_anchor_ready,
        ];
        let passed_signoff_check_count = checks.iter().filter(|ready| **ready).count();
        let required_signoff_check_count = checks.len();
        let missing_signoff_check_count = required_signoff_check_count - passed_signoff_check_count;
        let route_signoff_ready = missing_signoff_check_count == 0;

        Self {
            audit_summary,
            required_signoff_check_count,
            passed_signoff_check_count,
            missing_signoff_check_count,
            route_audit_ready,
            route_handoff_ready,
            attach_complete,
            network_data_ready,
            routing_surface_ready,
            parent_or_route_anchor_ready,
            route_signoff_ready,
        }
    }

    pub fn is_route_signoff_ready(self) -> bool {
        self.route_signoff_ready
    }

    pub fn has_signoff_gaps(self) -> bool {
        self.missing_signoff_check_count > 0
    }

    pub fn needs_route_audit(self) -> bool {
        !self.route_audit_ready
    }

    pub fn needs_route_handoff(self) -> bool {
        !self.route_handoff_ready
    }

    pub fn needs_attach_completion(self) -> bool {
        !self.attach_complete
    }

    pub fn needs_network_data(self) -> bool {
        !self.network_data_ready
    }

    pub fn needs_routing_surface(self) -> bool {
        !self.routing_surface_ready
    }

    pub fn needs_parent_or_route_anchor(self) -> bool {
        !self.parent_or_route_anchor_ready
    }
}

pub fn summarize_thread_attach_route_signoff(
    audit_summary: ThreadAttachRouteAuditSummary,
) -> ThreadAttachRouteSignoffSummary {
    ThreadAttachRouteSignoffSummary::from_audit_summary(audit_summary)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadAttachRouteCompletionSummary {
    pub signoff_summary: ThreadAttachRouteSignoffSummary,
    pub required_route_completion_check_count: usize,
    pub passed_route_completion_check_count: usize,
    pub missing_route_completion_check_count: usize,
    pub route_signoff_ready: bool,
    pub route_audit_ready: bool,
    pub route_handoff_ready: bool,
    pub attach_complete: bool,
    pub network_data_ready: bool,
    pub routing_surface_ready: bool,
    pub parent_or_route_anchor_ready: bool,
    pub route_completion_ready: bool,
}

impl ThreadAttachRouteCompletionSummary {
    pub fn from_signoff_summary(signoff_summary: ThreadAttachRouteSignoffSummary) -> Self {
        let route_signoff_ready = signoff_summary.is_route_signoff_ready();
        let route_audit_ready = !signoff_summary.needs_route_audit();
        let route_handoff_ready = !signoff_summary.needs_route_handoff();
        let attach_complete = !signoff_summary.needs_attach_completion();
        let network_data_ready = !signoff_summary.needs_network_data();
        let routing_surface_ready = !signoff_summary.needs_routing_surface();
        let parent_or_route_anchor_ready = !signoff_summary.needs_parent_or_route_anchor();
        let checks = [
            route_signoff_ready,
            route_audit_ready,
            route_handoff_ready,
            attach_complete,
            network_data_ready,
            routing_surface_ready,
            parent_or_route_anchor_ready,
        ];
        let passed_route_completion_check_count = checks.iter().filter(|ready| **ready).count();
        let required_route_completion_check_count = checks.len();
        let missing_route_completion_check_count =
            required_route_completion_check_count - passed_route_completion_check_count;
        let route_completion_ready = missing_route_completion_check_count == 0;

        Self {
            signoff_summary,
            required_route_completion_check_count,
            passed_route_completion_check_count,
            missing_route_completion_check_count,
            route_signoff_ready,
            route_audit_ready,
            route_handoff_ready,
            attach_complete,
            network_data_ready,
            routing_surface_ready,
            parent_or_route_anchor_ready,
            route_completion_ready,
        }
    }

    pub fn is_route_completion_ready(self) -> bool {
        self.route_completion_ready
    }

    pub fn has_completion_gaps(self) -> bool {
        self.missing_route_completion_check_count > 0
    }

    pub fn needs_route_signoff(self) -> bool {
        !self.route_signoff_ready
    }

    pub fn needs_route_audit(self) -> bool {
        !self.route_audit_ready
    }

    pub fn needs_route_handoff(self) -> bool {
        !self.route_handoff_ready
    }

    pub fn needs_attach_completion(self) -> bool {
        !self.attach_complete
    }

    pub fn needs_network_data(self) -> bool {
        !self.network_data_ready
    }

    pub fn needs_routing_surface(self) -> bool {
        !self.routing_surface_ready
    }

    pub fn needs_parent_or_route_anchor(self) -> bool {
        !self.parent_or_route_anchor_ready
    }
}

pub fn summarize_thread_attach_route_completion(
    signoff_summary: ThreadAttachRouteSignoffSummary,
) -> ThreadAttachRouteCompletionSummary {
    ThreadAttachRouteCompletionSummary::from_signoff_summary(signoff_summary)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadAttachRoutePublicationSummary {
    pub completion_summary: ThreadAttachRouteCompletionSummary,
    pub required_route_publication_check_count: usize,
    pub passed_route_publication_check_count: usize,
    pub missing_route_publication_check_count: usize,
    pub route_completion_ready: bool,
    pub route_signoff_ready: bool,
    pub route_audit_ready: bool,
    pub route_handoff_ready: bool,
    pub attach_complete: bool,
    pub network_data_ready: bool,
    pub routing_surface_ready: bool,
    pub parent_or_route_anchor_ready: bool,
    pub route_publication_ready: bool,
}

impl ThreadAttachRoutePublicationSummary {
    pub fn from_completion_summary(completion_summary: ThreadAttachRouteCompletionSummary) -> Self {
        let route_completion_ready = completion_summary.is_route_completion_ready();
        let route_signoff_ready = !completion_summary.needs_route_signoff();
        let route_audit_ready = !completion_summary.needs_route_audit();
        let route_handoff_ready = !completion_summary.needs_route_handoff();
        let attach_complete = !completion_summary.needs_attach_completion();
        let network_data_ready = !completion_summary.needs_network_data();
        let routing_surface_ready = !completion_summary.needs_routing_surface();
        let parent_or_route_anchor_ready = !completion_summary.needs_parent_or_route_anchor();
        let checks = [
            route_completion_ready,
            route_signoff_ready,
            route_audit_ready,
            route_handoff_ready,
            attach_complete,
            network_data_ready,
            routing_surface_ready,
            parent_or_route_anchor_ready,
        ];
        let passed_route_publication_check_count = checks.iter().filter(|ready| **ready).count();
        let required_route_publication_check_count = checks.len();
        let missing_route_publication_check_count =
            required_route_publication_check_count - passed_route_publication_check_count;
        let route_publication_ready = missing_route_publication_check_count == 0;

        Self {
            completion_summary,
            required_route_publication_check_count,
            passed_route_publication_check_count,
            missing_route_publication_check_count,
            route_completion_ready,
            route_signoff_ready,
            route_audit_ready,
            route_handoff_ready,
            attach_complete,
            network_data_ready,
            routing_surface_ready,
            parent_or_route_anchor_ready,
            route_publication_ready,
        }
    }

    pub fn is_route_publication_ready(self) -> bool {
        self.route_publication_ready
    }

    pub fn has_publication_gaps(self) -> bool {
        self.missing_route_publication_check_count > 0
    }

    pub fn needs_route_completion(self) -> bool {
        !self.route_completion_ready
    }

    pub fn needs_route_signoff(self) -> bool {
        !self.route_signoff_ready
    }

    pub fn needs_route_audit(self) -> bool {
        !self.route_audit_ready
    }

    pub fn needs_route_handoff(self) -> bool {
        !self.route_handoff_ready
    }

    pub fn needs_attach_completion(self) -> bool {
        !self.attach_complete
    }

    pub fn needs_network_data(self) -> bool {
        !self.network_data_ready
    }

    pub fn needs_routing_surface(self) -> bool {
        !self.routing_surface_ready
    }

    pub fn needs_parent_or_route_anchor(self) -> bool {
        !self.parent_or_route_anchor_ready
    }
}

pub fn summarize_thread_attach_route_publication(
    completion_summary: ThreadAttachRouteCompletionSummary,
) -> ThreadAttachRoutePublicationSummary {
    ThreadAttachRoutePublicationSummary::from_completion_summary(completion_summary)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadAttachRouteVerificationSummary {
    pub publication_summary: ThreadAttachRoutePublicationSummary,
    pub required_route_verification_check_count: usize,
    pub passed_route_verification_check_count: usize,
    pub missing_route_verification_check_count: usize,
    pub route_publication_ready: bool,
    pub route_completion_ready: bool,
    pub route_signoff_ready: bool,
    pub route_audit_ready: bool,
    pub route_handoff_ready: bool,
    pub attach_complete: bool,
    pub network_data_ready: bool,
    pub routing_surface_ready: bool,
    pub parent_or_route_anchor_ready: bool,
    pub route_verification_ready: bool,
}

impl ThreadAttachRouteVerificationSummary {
    pub fn from_publication_summary(
        publication_summary: ThreadAttachRoutePublicationSummary,
    ) -> Self {
        let route_publication_ready = publication_summary.is_route_publication_ready();
        let route_completion_ready = !publication_summary.needs_route_completion();
        let route_signoff_ready = !publication_summary.needs_route_signoff();
        let route_audit_ready = !publication_summary.needs_route_audit();
        let route_handoff_ready = !publication_summary.needs_route_handoff();
        let attach_complete = !publication_summary.needs_attach_completion();
        let network_data_ready = !publication_summary.needs_network_data();
        let routing_surface_ready = !publication_summary.needs_routing_surface();
        let parent_or_route_anchor_ready = !publication_summary.needs_parent_or_route_anchor();
        let checks = [
            route_publication_ready,
            route_completion_ready,
            route_signoff_ready,
            route_audit_ready,
            route_handoff_ready,
            attach_complete,
            network_data_ready,
            routing_surface_ready,
            parent_or_route_anchor_ready,
        ];
        let passed_route_verification_check_count = checks.iter().filter(|ready| **ready).count();
        let required_route_verification_check_count = checks.len();
        let missing_route_verification_check_count =
            required_route_verification_check_count - passed_route_verification_check_count;
        let route_verification_ready = missing_route_verification_check_count == 0;

        Self {
            publication_summary,
            required_route_verification_check_count,
            passed_route_verification_check_count,
            missing_route_verification_check_count,
            route_publication_ready,
            route_completion_ready,
            route_signoff_ready,
            route_audit_ready,
            route_handoff_ready,
            attach_complete,
            network_data_ready,
            routing_surface_ready,
            parent_or_route_anchor_ready,
            route_verification_ready,
        }
    }

    pub fn is_route_verification_ready(self) -> bool {
        self.route_verification_ready
    }

    pub fn has_verification_gaps(self) -> bool {
        self.missing_route_verification_check_count > 0
    }

    pub fn needs_route_publication(self) -> bool {
        !self.route_publication_ready
    }

    pub fn needs_route_completion(self) -> bool {
        !self.route_completion_ready
    }

    pub fn needs_route_signoff(self) -> bool {
        !self.route_signoff_ready
    }

    pub fn needs_route_audit(self) -> bool {
        !self.route_audit_ready
    }

    pub fn needs_route_handoff(self) -> bool {
        !self.route_handoff_ready
    }

    pub fn needs_attach_completion(self) -> bool {
        !self.attach_complete
    }

    pub fn needs_network_data(self) -> bool {
        !self.network_data_ready
    }

    pub fn needs_routing_surface(self) -> bool {
        !self.routing_surface_ready
    }

    pub fn needs_parent_or_route_anchor(self) -> bool {
        !self.parent_or_route_anchor_ready
    }
}

pub fn summarize_thread_attach_route_verification(
    publication_summary: ThreadAttachRoutePublicationSummary,
) -> ThreadAttachRouteVerificationSummary {
    ThreadAttachRouteVerificationSummary::from_publication_summary(publication_summary)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadAttachRouteValidationSummary {
    pub verification_summary: ThreadAttachRouteVerificationSummary,
    pub required_route_validation_check_count: usize,
    pub passed_route_validation_check_count: usize,
    pub missing_route_validation_check_count: usize,
    pub route_verification_ready: bool,
    pub route_publication_ready: bool,
    pub route_completion_ready: bool,
    pub route_signoff_ready: bool,
    pub route_audit_ready: bool,
    pub route_handoff_ready: bool,
    pub attach_complete: bool,
    pub network_data_ready: bool,
    pub routing_surface_ready: bool,
    pub parent_or_route_anchor_ready: bool,
    pub route_validation_ready: bool,
}

impl ThreadAttachRouteValidationSummary {
    pub fn from_verification_summary(
        verification_summary: ThreadAttachRouteVerificationSummary,
    ) -> Self {
        let route_verification_ready = verification_summary.is_route_verification_ready();
        let route_publication_ready = !verification_summary.needs_route_publication();
        let route_completion_ready = !verification_summary.needs_route_completion();
        let route_signoff_ready = !verification_summary.needs_route_signoff();
        let route_audit_ready = !verification_summary.needs_route_audit();
        let route_handoff_ready = !verification_summary.needs_route_handoff();
        let attach_complete = !verification_summary.needs_attach_completion();
        let network_data_ready = !verification_summary.needs_network_data();
        let routing_surface_ready = !verification_summary.needs_routing_surface();
        let parent_or_route_anchor_ready = !verification_summary.needs_parent_or_route_anchor();
        let checks = [
            route_verification_ready,
            route_publication_ready,
            route_completion_ready,
            route_signoff_ready,
            route_audit_ready,
            route_handoff_ready,
            attach_complete,
            network_data_ready,
            routing_surface_ready,
            parent_or_route_anchor_ready,
        ];
        let passed_route_validation_check_count = checks.iter().filter(|ready| **ready).count();
        let required_route_validation_check_count = checks.len();
        let missing_route_validation_check_count =
            required_route_validation_check_count - passed_route_validation_check_count;
        let route_validation_ready = missing_route_validation_check_count == 0;

        Self {
            verification_summary,
            required_route_validation_check_count,
            passed_route_validation_check_count,
            missing_route_validation_check_count,
            route_verification_ready,
            route_publication_ready,
            route_completion_ready,
            route_signoff_ready,
            route_audit_ready,
            route_handoff_ready,
            attach_complete,
            network_data_ready,
            routing_surface_ready,
            parent_or_route_anchor_ready,
            route_validation_ready,
        }
    }

    pub fn is_route_validation_ready(self) -> bool {
        self.route_validation_ready
    }

    pub fn has_validation_gaps(self) -> bool {
        self.missing_route_validation_check_count > 0
    }

    pub fn needs_route_verification(self) -> bool {
        !self.route_verification_ready
    }

    pub fn needs_route_publication(self) -> bool {
        !self.route_publication_ready
    }

    pub fn needs_route_completion(self) -> bool {
        !self.route_completion_ready
    }

    pub fn needs_route_signoff(self) -> bool {
        !self.route_signoff_ready
    }

    pub fn needs_route_audit(self) -> bool {
        !self.route_audit_ready
    }

    pub fn needs_route_handoff(self) -> bool {
        !self.route_handoff_ready
    }

    pub fn needs_attach_completion(self) -> bool {
        !self.attach_complete
    }

    pub fn needs_network_data(self) -> bool {
        !self.network_data_ready
    }

    pub fn needs_routing_surface(self) -> bool {
        !self.routing_surface_ready
    }

    pub fn needs_parent_or_route_anchor(self) -> bool {
        !self.parent_or_route_anchor_ready
    }
}

pub fn summarize_thread_attach_route_validation(
    verification_summary: ThreadAttachRouteVerificationSummary,
) -> ThreadAttachRouteValidationSummary {
    ThreadAttachRouteValidationSummary::from_verification_summary(verification_summary)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadAttachRouteCertificationSummary {
    pub validation_summary: ThreadAttachRouteValidationSummary,
    pub required_route_certification_check_count: usize,
    pub passed_route_certification_check_count: usize,
    pub missing_route_certification_check_count: usize,
    pub route_validation_ready: bool,
    pub route_verification_ready: bool,
    pub route_publication_ready: bool,
    pub route_completion_ready: bool,
    pub route_signoff_ready: bool,
    pub route_audit_ready: bool,
    pub route_handoff_ready: bool,
    pub attach_complete: bool,
    pub network_data_ready: bool,
    pub routing_surface_ready: bool,
    pub parent_or_route_anchor_ready: bool,
    pub route_certification_ready: bool,
}

impl ThreadAttachRouteCertificationSummary {
    pub fn from_validation_summary(validation_summary: ThreadAttachRouteValidationSummary) -> Self {
        let route_validation_ready = validation_summary.is_route_validation_ready();
        let route_verification_ready = !validation_summary.needs_route_verification();
        let route_publication_ready = !validation_summary.needs_route_publication();
        let route_completion_ready = !validation_summary.needs_route_completion();
        let route_signoff_ready = !validation_summary.needs_route_signoff();
        let route_audit_ready = !validation_summary.needs_route_audit();
        let route_handoff_ready = !validation_summary.needs_route_handoff();
        let attach_complete = !validation_summary.needs_attach_completion();
        let network_data_ready = !validation_summary.needs_network_data();
        let routing_surface_ready = !validation_summary.needs_routing_surface();
        let parent_or_route_anchor_ready = !validation_summary.needs_parent_or_route_anchor();
        let checks = [
            route_validation_ready,
            route_verification_ready,
            route_publication_ready,
            route_completion_ready,
            route_signoff_ready,
            route_audit_ready,
            route_handoff_ready,
            attach_complete,
            network_data_ready,
            routing_surface_ready,
            parent_or_route_anchor_ready,
        ];
        let passed_route_certification_check_count = checks.iter().filter(|ready| **ready).count();
        let required_route_certification_check_count = checks.len();
        let missing_route_certification_check_count =
            required_route_certification_check_count - passed_route_certification_check_count;
        let route_certification_ready = missing_route_certification_check_count == 0;

        Self {
            validation_summary,
            required_route_certification_check_count,
            passed_route_certification_check_count,
            missing_route_certification_check_count,
            route_validation_ready,
            route_verification_ready,
            route_publication_ready,
            route_completion_ready,
            route_signoff_ready,
            route_audit_ready,
            route_handoff_ready,
            attach_complete,
            network_data_ready,
            routing_surface_ready,
            parent_or_route_anchor_ready,
            route_certification_ready,
        }
    }

    pub fn is_route_certification_ready(self) -> bool {
        self.route_certification_ready
    }

    pub fn has_certification_gaps(self) -> bool {
        self.missing_route_certification_check_count > 0
    }

    pub fn needs_route_validation(self) -> bool {
        !self.route_validation_ready
    }

    pub fn needs_route_verification(self) -> bool {
        !self.route_verification_ready
    }

    pub fn needs_route_publication(self) -> bool {
        !self.route_publication_ready
    }

    pub fn needs_route_completion(self) -> bool {
        !self.route_completion_ready
    }

    pub fn needs_route_signoff(self) -> bool {
        !self.route_signoff_ready
    }

    pub fn needs_route_audit(self) -> bool {
        !self.route_audit_ready
    }

    pub fn needs_route_handoff(self) -> bool {
        !self.route_handoff_ready
    }

    pub fn needs_attach_completion(self) -> bool {
        !self.attach_complete
    }

    pub fn needs_network_data(self) -> bool {
        !self.network_data_ready
    }

    pub fn needs_routing_surface(self) -> bool {
        !self.routing_surface_ready
    }

    pub fn needs_parent_or_route_anchor(self) -> bool {
        !self.parent_or_route_anchor_ready
    }
}

pub fn summarize_thread_attach_route_certification(
    validation_summary: ThreadAttachRouteValidationSummary,
) -> ThreadAttachRouteCertificationSummary {
    ThreadAttachRouteCertificationSummary::from_validation_summary(validation_summary)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadAttachRouteApprovalSummary {
    pub certification_summary: ThreadAttachRouteCertificationSummary,
    pub required_route_approval_check_count: usize,
    pub passed_route_approval_check_count: usize,
    pub missing_route_approval_check_count: usize,
    pub route_certification_ready: bool,
    pub route_validation_ready: bool,
    pub route_verification_ready: bool,
    pub route_publication_ready: bool,
    pub route_completion_ready: bool,
    pub route_signoff_ready: bool,
    pub route_audit_ready: bool,
    pub route_handoff_ready: bool,
    pub attach_complete: bool,
    pub network_data_ready: bool,
    pub routing_surface_ready: bool,
    pub parent_or_route_anchor_ready: bool,
    pub route_approval_ready: bool,
}

impl ThreadAttachRouteApprovalSummary {
    pub fn from_certification_summary(
        certification_summary: ThreadAttachRouteCertificationSummary,
    ) -> Self {
        let route_certification_ready = certification_summary.is_route_certification_ready();
        let route_validation_ready = !certification_summary.needs_route_validation();
        let route_verification_ready = !certification_summary.needs_route_verification();
        let route_publication_ready = !certification_summary.needs_route_publication();
        let route_completion_ready = !certification_summary.needs_route_completion();
        let route_signoff_ready = !certification_summary.needs_route_signoff();
        let route_audit_ready = !certification_summary.needs_route_audit();
        let route_handoff_ready = !certification_summary.needs_route_handoff();
        let attach_complete = !certification_summary.needs_attach_completion();
        let network_data_ready = !certification_summary.needs_network_data();
        let routing_surface_ready = !certification_summary.needs_routing_surface();
        let parent_or_route_anchor_ready = !certification_summary.needs_parent_or_route_anchor();
        let checks = [
            route_certification_ready,
            route_validation_ready,
            route_verification_ready,
            route_publication_ready,
            route_completion_ready,
            route_signoff_ready,
            route_audit_ready,
            route_handoff_ready,
            attach_complete,
            network_data_ready,
            routing_surface_ready,
            parent_or_route_anchor_ready,
        ];
        let passed_route_approval_check_count = checks.iter().filter(|ready| **ready).count();
        let required_route_approval_check_count = checks.len();
        let missing_route_approval_check_count =
            required_route_approval_check_count - passed_route_approval_check_count;
        let route_approval_ready = missing_route_approval_check_count == 0;

        Self {
            certification_summary,
            required_route_approval_check_count,
            passed_route_approval_check_count,
            missing_route_approval_check_count,
            route_certification_ready,
            route_validation_ready,
            route_verification_ready,
            route_publication_ready,
            route_completion_ready,
            route_signoff_ready,
            route_audit_ready,
            route_handoff_ready,
            attach_complete,
            network_data_ready,
            routing_surface_ready,
            parent_or_route_anchor_ready,
            route_approval_ready,
        }
    }

    pub fn is_route_approval_ready(self) -> bool {
        self.route_approval_ready
    }

    pub fn has_approval_gaps(self) -> bool {
        self.missing_route_approval_check_count > 0
    }

    pub fn needs_route_certification(self) -> bool {
        !self.route_certification_ready
    }

    pub fn needs_route_validation(self) -> bool {
        !self.route_validation_ready
    }

    pub fn needs_route_verification(self) -> bool {
        !self.route_verification_ready
    }

    pub fn needs_route_publication(self) -> bool {
        !self.route_publication_ready
    }

    pub fn needs_route_completion(self) -> bool {
        !self.route_completion_ready
    }

    pub fn needs_route_signoff(self) -> bool {
        !self.route_signoff_ready
    }

    pub fn needs_route_audit(self) -> bool {
        !self.route_audit_ready
    }

    pub fn needs_route_handoff(self) -> bool {
        !self.route_handoff_ready
    }

    pub fn needs_attach_completion(self) -> bool {
        !self.attach_complete
    }

    pub fn needs_network_data(self) -> bool {
        !self.network_data_ready
    }

    pub fn needs_routing_surface(self) -> bool {
        !self.routing_surface_ready
    }

    pub fn needs_parent_or_route_anchor(self) -> bool {
        !self.parent_or_route_anchor_ready
    }
}

pub fn summarize_thread_attach_route_approval(
    certification_summary: ThreadAttachRouteCertificationSummary,
) -> ThreadAttachRouteApprovalSummary {
    ThreadAttachRouteApprovalSummary::from_certification_summary(certification_summary)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadAttachRouteActivationSummary {
    pub approval_summary: ThreadAttachRouteApprovalSummary,
    pub required_route_activation_check_count: usize,
    pub passed_route_activation_check_count: usize,
    pub missing_route_activation_check_count: usize,
    pub route_approval_ready: bool,
    pub route_certification_ready: bool,
    pub route_validation_ready: bool,
    pub route_verification_ready: bool,
    pub route_publication_ready: bool,
    pub route_completion_ready: bool,
    pub route_signoff_ready: bool,
    pub route_audit_ready: bool,
    pub route_handoff_ready: bool,
    pub attach_complete: bool,
    pub network_data_ready: bool,
    pub routing_surface_ready: bool,
    pub parent_or_route_anchor_ready: bool,
    pub route_activation_ready: bool,
}

impl ThreadAttachRouteActivationSummary {
    pub fn from_approval_summary(approval_summary: ThreadAttachRouteApprovalSummary) -> Self {
        let route_approval_ready = approval_summary.is_route_approval_ready();
        let route_certification_ready = !approval_summary.needs_route_certification();
        let route_validation_ready = !approval_summary.needs_route_validation();
        let route_verification_ready = !approval_summary.needs_route_verification();
        let route_publication_ready = !approval_summary.needs_route_publication();
        let route_completion_ready = !approval_summary.needs_route_completion();
        let route_signoff_ready = !approval_summary.needs_route_signoff();
        let route_audit_ready = !approval_summary.needs_route_audit();
        let route_handoff_ready = !approval_summary.needs_route_handoff();
        let attach_complete = !approval_summary.needs_attach_completion();
        let network_data_ready = !approval_summary.needs_network_data();
        let routing_surface_ready = !approval_summary.needs_routing_surface();
        let parent_or_route_anchor_ready = !approval_summary.needs_parent_or_route_anchor();
        let checks = [
            route_approval_ready,
            route_certification_ready,
            route_validation_ready,
            route_verification_ready,
            route_publication_ready,
            route_completion_ready,
            route_signoff_ready,
            route_audit_ready,
            route_handoff_ready,
            attach_complete,
            network_data_ready,
            routing_surface_ready,
            parent_or_route_anchor_ready,
        ];
        let passed_route_activation_check_count = checks.iter().filter(|ready| **ready).count();
        let required_route_activation_check_count = checks.len();
        let missing_route_activation_check_count =
            required_route_activation_check_count - passed_route_activation_check_count;
        let route_activation_ready = missing_route_activation_check_count == 0;

        Self {
            approval_summary,
            required_route_activation_check_count,
            passed_route_activation_check_count,
            missing_route_activation_check_count,
            route_approval_ready,
            route_certification_ready,
            route_validation_ready,
            route_verification_ready,
            route_publication_ready,
            route_completion_ready,
            route_signoff_ready,
            route_audit_ready,
            route_handoff_ready,
            attach_complete,
            network_data_ready,
            routing_surface_ready,
            parent_or_route_anchor_ready,
            route_activation_ready,
        }
    }

    pub fn is_route_activation_ready(self) -> bool {
        self.route_activation_ready
    }

    pub fn has_activation_gaps(self) -> bool {
        self.missing_route_activation_check_count > 0
    }

    pub fn needs_route_approval(self) -> bool {
        !self.route_approval_ready
    }

    pub fn needs_route_certification(self) -> bool {
        !self.route_certification_ready
    }

    pub fn needs_route_validation(self) -> bool {
        !self.route_validation_ready
    }

    pub fn needs_route_verification(self) -> bool {
        !self.route_verification_ready
    }

    pub fn needs_route_publication(self) -> bool {
        !self.route_publication_ready
    }

    pub fn needs_route_completion(self) -> bool {
        !self.route_completion_ready
    }

    pub fn needs_route_signoff(self) -> bool {
        !self.route_signoff_ready
    }

    pub fn needs_route_audit(self) -> bool {
        !self.route_audit_ready
    }

    pub fn needs_route_handoff(self) -> bool {
        !self.route_handoff_ready
    }

    pub fn needs_attach_completion(self) -> bool {
        !self.attach_complete
    }

    pub fn needs_network_data(self) -> bool {
        !self.network_data_ready
    }

    pub fn needs_routing_surface(self) -> bool {
        !self.routing_surface_ready
    }

    pub fn needs_parent_or_route_anchor(self) -> bool {
        !self.parent_or_route_anchor_ready
    }
}

pub fn summarize_thread_attach_route_activation(
    approval_summary: ThreadAttachRouteApprovalSummary,
) -> ThreadAttachRouteActivationSummary {
    ThreadAttachRouteActivationSummary::from_approval_summary(approval_summary)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadAttachRouteRolloutSummary {
    pub activation_summary: ThreadAttachRouteActivationSummary,
    pub required_route_rollout_check_count: usize,
    pub passed_route_rollout_check_count: usize,
    pub missing_route_rollout_check_count: usize,
    pub route_activation_ready: bool,
    pub route_approval_ready: bool,
    pub route_certification_ready: bool,
    pub route_validation_ready: bool,
    pub route_verification_ready: bool,
    pub route_publication_ready: bool,
    pub route_completion_ready: bool,
    pub route_signoff_ready: bool,
    pub route_audit_ready: bool,
    pub route_handoff_ready: bool,
    pub attach_complete: bool,
    pub network_data_ready: bool,
    pub routing_surface_ready: bool,
    pub parent_or_route_anchor_ready: bool,
    pub route_rollout_ready: bool,
}

impl ThreadAttachRouteRolloutSummary {
    pub fn from_activation_summary(activation_summary: ThreadAttachRouteActivationSummary) -> Self {
        let route_activation_ready = activation_summary.is_route_activation_ready();
        let route_approval_ready = !activation_summary.needs_route_approval();
        let route_certification_ready = !activation_summary.needs_route_certification();
        let route_validation_ready = !activation_summary.needs_route_validation();
        let route_verification_ready = !activation_summary.needs_route_verification();
        let route_publication_ready = !activation_summary.needs_route_publication();
        let route_completion_ready = !activation_summary.needs_route_completion();
        let route_signoff_ready = !activation_summary.needs_route_signoff();
        let route_audit_ready = !activation_summary.needs_route_audit();
        let route_handoff_ready = !activation_summary.needs_route_handoff();
        let attach_complete = !activation_summary.needs_attach_completion();
        let network_data_ready = !activation_summary.needs_network_data();
        let routing_surface_ready = !activation_summary.needs_routing_surface();
        let parent_or_route_anchor_ready = !activation_summary.needs_parent_or_route_anchor();
        let checks = [
            route_activation_ready,
            route_approval_ready,
            route_certification_ready,
            route_validation_ready,
            route_verification_ready,
            route_publication_ready,
            route_completion_ready,
            route_signoff_ready,
            route_audit_ready,
            route_handoff_ready,
            attach_complete,
            network_data_ready,
            routing_surface_ready,
            parent_or_route_anchor_ready,
        ];
        let passed_route_rollout_check_count = checks.iter().filter(|ready| **ready).count();
        let required_route_rollout_check_count = checks.len();
        let missing_route_rollout_check_count =
            required_route_rollout_check_count - passed_route_rollout_check_count;
        let route_rollout_ready = missing_route_rollout_check_count == 0;

        Self {
            activation_summary,
            required_route_rollout_check_count,
            passed_route_rollout_check_count,
            missing_route_rollout_check_count,
            route_activation_ready,
            route_approval_ready,
            route_certification_ready,
            route_validation_ready,
            route_verification_ready,
            route_publication_ready,
            route_completion_ready,
            route_signoff_ready,
            route_audit_ready,
            route_handoff_ready,
            attach_complete,
            network_data_ready,
            routing_surface_ready,
            parent_or_route_anchor_ready,
            route_rollout_ready,
        }
    }

    pub fn is_route_rollout_ready(self) -> bool {
        self.route_rollout_ready
    }

    pub fn has_rollout_gaps(self) -> bool {
        self.missing_route_rollout_check_count > 0
    }

    pub fn needs_route_activation(self) -> bool {
        !self.route_activation_ready
    }

    pub fn needs_route_approval(self) -> bool {
        !self.route_approval_ready
    }

    pub fn needs_route_certification(self) -> bool {
        !self.route_certification_ready
    }

    pub fn needs_route_validation(self) -> bool {
        !self.route_validation_ready
    }

    pub fn needs_route_verification(self) -> bool {
        !self.route_verification_ready
    }

    pub fn needs_route_publication(self) -> bool {
        !self.route_publication_ready
    }

    pub fn needs_route_completion(self) -> bool {
        !self.route_completion_ready
    }

    pub fn needs_route_signoff(self) -> bool {
        !self.route_signoff_ready
    }

    pub fn needs_route_audit(self) -> bool {
        !self.route_audit_ready
    }

    pub fn needs_route_handoff(self) -> bool {
        !self.route_handoff_ready
    }

    pub fn needs_attach_completion(self) -> bool {
        !self.attach_complete
    }

    pub fn needs_network_data(self) -> bool {
        !self.network_data_ready
    }

    pub fn needs_routing_surface(self) -> bool {
        !self.routing_surface_ready
    }

    pub fn needs_parent_or_route_anchor(self) -> bool {
        !self.parent_or_route_anchor_ready
    }
}

pub fn summarize_thread_attach_route_rollout(
    activation_summary: ThreadAttachRouteActivationSummary,
) -> ThreadAttachRouteRolloutSummary {
    ThreadAttachRouteRolloutSummary::from_activation_summary(activation_summary)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadAttachRouteAdoptionSummary {
    pub rollout_summary: ThreadAttachRouteRolloutSummary,
    pub required_route_adoption_check_count: usize,
    pub passed_route_adoption_check_count: usize,
    pub missing_route_adoption_check_count: usize,
    pub route_rollout_ready: bool,
    pub route_activation_ready: bool,
    pub route_approval_ready: bool,
    pub route_certification_ready: bool,
    pub route_validation_ready: bool,
    pub route_verification_ready: bool,
    pub route_publication_ready: bool,
    pub route_completion_ready: bool,
    pub route_signoff_ready: bool,
    pub route_audit_ready: bool,
    pub route_handoff_ready: bool,
    pub attach_complete: bool,
    pub network_data_ready: bool,
    pub routing_surface_ready: bool,
    pub parent_or_route_anchor_ready: bool,
    pub route_adoption_ready: bool,
}

impl ThreadAttachRouteAdoptionSummary {
    pub fn from_rollout_summary(rollout_summary: ThreadAttachRouteRolloutSummary) -> Self {
        let route_rollout_ready = rollout_summary.is_route_rollout_ready();
        let route_activation_ready = !rollout_summary.needs_route_activation();
        let route_approval_ready = !rollout_summary.needs_route_approval();
        let route_certification_ready = !rollout_summary.needs_route_certification();
        let route_validation_ready = !rollout_summary.needs_route_validation();
        let route_verification_ready = !rollout_summary.needs_route_verification();
        let route_publication_ready = !rollout_summary.needs_route_publication();
        let route_completion_ready = !rollout_summary.needs_route_completion();
        let route_signoff_ready = !rollout_summary.needs_route_signoff();
        let route_audit_ready = !rollout_summary.needs_route_audit();
        let route_handoff_ready = !rollout_summary.needs_route_handoff();
        let attach_complete = !rollout_summary.needs_attach_completion();
        let network_data_ready = !rollout_summary.needs_network_data();
        let routing_surface_ready = !rollout_summary.needs_routing_surface();
        let parent_or_route_anchor_ready = !rollout_summary.needs_parent_or_route_anchor();
        let checks = [
            route_rollout_ready,
            route_activation_ready,
            route_approval_ready,
            route_certification_ready,
            route_validation_ready,
            route_verification_ready,
            route_publication_ready,
            route_completion_ready,
            route_signoff_ready,
            route_audit_ready,
            route_handoff_ready,
            attach_complete,
            network_data_ready,
            routing_surface_ready,
            parent_or_route_anchor_ready,
        ];
        let passed_route_adoption_check_count = checks.iter().filter(|ready| **ready).count();
        let required_route_adoption_check_count = checks.len();
        let missing_route_adoption_check_count =
            required_route_adoption_check_count - passed_route_adoption_check_count;
        let route_adoption_ready = missing_route_adoption_check_count == 0;

        Self {
            rollout_summary,
            required_route_adoption_check_count,
            passed_route_adoption_check_count,
            missing_route_adoption_check_count,
            route_rollout_ready,
            route_activation_ready,
            route_approval_ready,
            route_certification_ready,
            route_validation_ready,
            route_verification_ready,
            route_publication_ready,
            route_completion_ready,
            route_signoff_ready,
            route_audit_ready,
            route_handoff_ready,
            attach_complete,
            network_data_ready,
            routing_surface_ready,
            parent_or_route_anchor_ready,
            route_adoption_ready,
        }
    }

    pub fn is_route_adoption_ready(self) -> bool {
        self.route_adoption_ready
    }

    pub fn has_adoption_gaps(self) -> bool {
        self.missing_route_adoption_check_count > 0
    }

    pub fn needs_route_rollout(self) -> bool {
        !self.route_rollout_ready
    }

    pub fn needs_route_activation(self) -> bool {
        !self.route_activation_ready
    }

    pub fn needs_route_approval(self) -> bool {
        !self.route_approval_ready
    }

    pub fn needs_route_certification(self) -> bool {
        !self.route_certification_ready
    }

    pub fn needs_route_validation(self) -> bool {
        !self.route_validation_ready
    }

    pub fn needs_route_verification(self) -> bool {
        !self.route_verification_ready
    }

    pub fn needs_route_publication(self) -> bool {
        !self.route_publication_ready
    }

    pub fn needs_route_completion(self) -> bool {
        !self.route_completion_ready
    }

    pub fn needs_route_signoff(self) -> bool {
        !self.route_signoff_ready
    }

    pub fn needs_route_audit(self) -> bool {
        !self.route_audit_ready
    }

    pub fn needs_route_handoff(self) -> bool {
        !self.route_handoff_ready
    }

    pub fn needs_attach_completion(self) -> bool {
        !self.attach_complete
    }

    pub fn needs_network_data(self) -> bool {
        !self.network_data_ready
    }

    pub fn needs_routing_surface(self) -> bool {
        !self.routing_surface_ready
    }

    pub fn needs_parent_or_route_anchor(self) -> bool {
        !self.parent_or_route_anchor_ready
    }
}

pub fn summarize_thread_attach_route_adoption(
    rollout_summary: ThreadAttachRouteRolloutSummary,
) -> ThreadAttachRouteAdoptionSummary {
    ThreadAttachRouteAdoptionSummary::from_rollout_summary(rollout_summary)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadAttachRouteAcceptanceSummary {
    pub adoption_summary: ThreadAttachRouteAdoptionSummary,
    pub required_route_acceptance_check_count: usize,
    pub passed_route_acceptance_check_count: usize,
    pub missing_route_acceptance_check_count: usize,
    pub route_adoption_ready: bool,
    pub route_rollout_ready: bool,
    pub route_activation_ready: bool,
    pub route_approval_ready: bool,
    pub route_certification_ready: bool,
    pub route_validation_ready: bool,
    pub route_verification_ready: bool,
    pub route_publication_ready: bool,
    pub route_completion_ready: bool,
    pub route_signoff_ready: bool,
    pub route_audit_ready: bool,
    pub route_handoff_ready: bool,
    pub attach_complete: bool,
    pub network_data_ready: bool,
    pub routing_surface_ready: bool,
    pub parent_or_route_anchor_ready: bool,
    pub route_acceptance_ready: bool,
}

impl ThreadAttachRouteAcceptanceSummary {
    pub fn from_adoption_summary(adoption_summary: ThreadAttachRouteAdoptionSummary) -> Self {
        let route_adoption_ready = adoption_summary.is_route_adoption_ready();
        let route_rollout_ready = !adoption_summary.needs_route_rollout();
        let route_activation_ready = !adoption_summary.needs_route_activation();
        let route_approval_ready = !adoption_summary.needs_route_approval();
        let route_certification_ready = !adoption_summary.needs_route_certification();
        let route_validation_ready = !adoption_summary.needs_route_validation();
        let route_verification_ready = !adoption_summary.needs_route_verification();
        let route_publication_ready = !adoption_summary.needs_route_publication();
        let route_completion_ready = !adoption_summary.needs_route_completion();
        let route_signoff_ready = !adoption_summary.needs_route_signoff();
        let route_audit_ready = !adoption_summary.needs_route_audit();
        let route_handoff_ready = !adoption_summary.needs_route_handoff();
        let attach_complete = !adoption_summary.needs_attach_completion();
        let network_data_ready = !adoption_summary.needs_network_data();
        let routing_surface_ready = !adoption_summary.needs_routing_surface();
        let parent_or_route_anchor_ready = !adoption_summary.needs_parent_or_route_anchor();
        let checks = [
            route_adoption_ready,
            route_rollout_ready,
            route_activation_ready,
            route_approval_ready,
            route_certification_ready,
            route_validation_ready,
            route_verification_ready,
            route_publication_ready,
            route_completion_ready,
            route_signoff_ready,
            route_audit_ready,
            route_handoff_ready,
            attach_complete,
            network_data_ready,
            routing_surface_ready,
            parent_or_route_anchor_ready,
        ];
        let passed_route_acceptance_check_count = checks.iter().filter(|ready| **ready).count();
        let required_route_acceptance_check_count = checks.len();
        let missing_route_acceptance_check_count =
            required_route_acceptance_check_count - passed_route_acceptance_check_count;
        let route_acceptance_ready = missing_route_acceptance_check_count == 0;

        Self {
            adoption_summary,
            required_route_acceptance_check_count,
            passed_route_acceptance_check_count,
            missing_route_acceptance_check_count,
            route_adoption_ready,
            route_rollout_ready,
            route_activation_ready,
            route_approval_ready,
            route_certification_ready,
            route_validation_ready,
            route_verification_ready,
            route_publication_ready,
            route_completion_ready,
            route_signoff_ready,
            route_audit_ready,
            route_handoff_ready,
            attach_complete,
            network_data_ready,
            routing_surface_ready,
            parent_or_route_anchor_ready,
            route_acceptance_ready,
        }
    }

    pub fn is_route_acceptance_ready(self) -> bool {
        self.route_acceptance_ready
    }

    pub fn has_acceptance_gaps(self) -> bool {
        self.missing_route_acceptance_check_count > 0
    }

    pub fn needs_route_adoption(self) -> bool {
        !self.route_adoption_ready
    }

    pub fn needs_route_rollout(self) -> bool {
        !self.route_rollout_ready
    }

    pub fn needs_route_activation(self) -> bool {
        !self.route_activation_ready
    }

    pub fn needs_route_approval(self) -> bool {
        !self.route_approval_ready
    }

    pub fn needs_route_certification(self) -> bool {
        !self.route_certification_ready
    }

    pub fn needs_route_validation(self) -> bool {
        !self.route_validation_ready
    }

    pub fn needs_route_verification(self) -> bool {
        !self.route_verification_ready
    }

    pub fn needs_route_publication(self) -> bool {
        !self.route_publication_ready
    }

    pub fn needs_route_completion(self) -> bool {
        !self.route_completion_ready
    }

    pub fn needs_route_signoff(self) -> bool {
        !self.route_signoff_ready
    }

    pub fn needs_route_audit(self) -> bool {
        !self.route_audit_ready
    }

    pub fn needs_route_handoff(self) -> bool {
        !self.route_handoff_ready
    }

    pub fn needs_attach_completion(self) -> bool {
        !self.attach_complete
    }

    pub fn needs_network_data(self) -> bool {
        !self.network_data_ready
    }

    pub fn needs_routing_surface(self) -> bool {
        !self.routing_surface_ready
    }

    pub fn needs_parent_or_route_anchor(self) -> bool {
        !self.parent_or_route_anchor_ready
    }
}

pub fn summarize_thread_attach_route_acceptance(
    adoption_summary: ThreadAttachRouteAdoptionSummary,
) -> ThreadAttachRouteAcceptanceSummary {
    ThreadAttachRouteAcceptanceSummary::from_adoption_summary(adoption_summary)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadAttachRouteDistributionSummary {
    pub acceptance_summary: ThreadAttachRouteAcceptanceSummary,
    pub required_route_distribution_check_count: usize,
    pub passed_route_distribution_check_count: usize,
    pub missing_route_distribution_check_count: usize,
    pub route_acceptance_ready: bool,
    pub route_adoption_ready: bool,
    pub route_rollout_ready: bool,
    pub route_activation_ready: bool,
    pub route_approval_ready: bool,
    pub route_certification_ready: bool,
    pub route_validation_ready: bool,
    pub route_verification_ready: bool,
    pub route_publication_ready: bool,
    pub route_completion_ready: bool,
    pub route_signoff_ready: bool,
    pub route_audit_ready: bool,
    pub route_handoff_ready: bool,
    pub attach_complete: bool,
    pub network_data_ready: bool,
    pub routing_surface_ready: bool,
    pub parent_or_route_anchor_ready: bool,
    pub route_distribution_ready: bool,
}

impl ThreadAttachRouteDistributionSummary {
    pub fn from_acceptance_summary(acceptance_summary: ThreadAttachRouteAcceptanceSummary) -> Self {
        let route_acceptance_ready = acceptance_summary.is_route_acceptance_ready();
        let route_adoption_ready = !acceptance_summary.needs_route_adoption();
        let route_rollout_ready = !acceptance_summary.needs_route_rollout();
        let route_activation_ready = !acceptance_summary.needs_route_activation();
        let route_approval_ready = !acceptance_summary.needs_route_approval();
        let route_certification_ready = !acceptance_summary.needs_route_certification();
        let route_validation_ready = !acceptance_summary.needs_route_validation();
        let route_verification_ready = !acceptance_summary.needs_route_verification();
        let route_publication_ready = !acceptance_summary.needs_route_publication();
        let route_completion_ready = !acceptance_summary.needs_route_completion();
        let route_signoff_ready = !acceptance_summary.needs_route_signoff();
        let route_audit_ready = !acceptance_summary.needs_route_audit();
        let route_handoff_ready = !acceptance_summary.needs_route_handoff();
        let attach_complete = !acceptance_summary.needs_attach_completion();
        let network_data_ready = !acceptance_summary.needs_network_data();
        let routing_surface_ready = !acceptance_summary.needs_routing_surface();
        let parent_or_route_anchor_ready = !acceptance_summary.needs_parent_or_route_anchor();
        let checks = [
            route_acceptance_ready,
            route_adoption_ready,
            route_rollout_ready,
            route_activation_ready,
            route_approval_ready,
            route_certification_ready,
            route_validation_ready,
            route_verification_ready,
            route_publication_ready,
            route_completion_ready,
            route_signoff_ready,
            route_audit_ready,
            route_handoff_ready,
            attach_complete,
            network_data_ready,
            routing_surface_ready,
            parent_or_route_anchor_ready,
        ];
        let passed_route_distribution_check_count = checks.iter().filter(|ready| **ready).count();
        let required_route_distribution_check_count = checks.len();
        let missing_route_distribution_check_count =
            required_route_distribution_check_count - passed_route_distribution_check_count;
        let route_distribution_ready = missing_route_distribution_check_count == 0;

        Self {
            acceptance_summary,
            required_route_distribution_check_count,
            passed_route_distribution_check_count,
            missing_route_distribution_check_count,
            route_acceptance_ready,
            route_adoption_ready,
            route_rollout_ready,
            route_activation_ready,
            route_approval_ready,
            route_certification_ready,
            route_validation_ready,
            route_verification_ready,
            route_publication_ready,
            route_completion_ready,
            route_signoff_ready,
            route_audit_ready,
            route_handoff_ready,
            attach_complete,
            network_data_ready,
            routing_surface_ready,
            parent_or_route_anchor_ready,
            route_distribution_ready,
        }
    }

    pub fn is_route_distribution_ready(self) -> bool {
        self.route_distribution_ready
    }

    pub fn has_distribution_gaps(self) -> bool {
        self.missing_route_distribution_check_count > 0
    }

    pub fn needs_route_acceptance(self) -> bool {
        !self.route_acceptance_ready
    }

    pub fn needs_route_adoption(self) -> bool {
        !self.route_adoption_ready
    }

    pub fn needs_route_rollout(self) -> bool {
        !self.route_rollout_ready
    }

    pub fn needs_route_activation(self) -> bool {
        !self.route_activation_ready
    }

    pub fn needs_route_approval(self) -> bool {
        !self.route_approval_ready
    }

    pub fn needs_route_certification(self) -> bool {
        !self.route_certification_ready
    }

    pub fn needs_route_validation(self) -> bool {
        !self.route_validation_ready
    }

    pub fn needs_route_verification(self) -> bool {
        !self.route_verification_ready
    }

    pub fn needs_route_publication(self) -> bool {
        !self.route_publication_ready
    }

    pub fn needs_route_completion(self) -> bool {
        !self.route_completion_ready
    }

    pub fn needs_route_signoff(self) -> bool {
        !self.route_signoff_ready
    }

    pub fn needs_route_audit(self) -> bool {
        !self.route_audit_ready
    }

    pub fn needs_route_handoff(self) -> bool {
        !self.route_handoff_ready
    }

    pub fn needs_attach_completion(self) -> bool {
        !self.attach_complete
    }

    pub fn needs_network_data(self) -> bool {
        !self.network_data_ready
    }

    pub fn needs_routing_surface(self) -> bool {
        !self.routing_surface_ready
    }

    pub fn needs_parent_or_route_anchor(self) -> bool {
        !self.parent_or_route_anchor_ready
    }
}

pub fn summarize_thread_attach_route_distribution(
    acceptance_summary: ThreadAttachRouteAcceptanceSummary,
) -> ThreadAttachRouteDistributionSummary {
    ThreadAttachRouteDistributionSummary::from_acceptance_summary(acceptance_summary)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadAttachRouteExportSummary {
    pub distribution_summary: ThreadAttachRouteDistributionSummary,
    pub required_route_export_check_count: usize,
    pub passed_route_export_check_count: usize,
    pub missing_route_export_check_count: usize,
    pub route_distribution_ready: bool,
    pub route_acceptance_ready: bool,
    pub route_adoption_ready: bool,
    pub route_rollout_ready: bool,
    pub route_activation_ready: bool,
    pub route_approval_ready: bool,
    pub route_certification_ready: bool,
    pub route_validation_ready: bool,
    pub route_verification_ready: bool,
    pub route_publication_ready: bool,
    pub route_completion_ready: bool,
    pub route_signoff_ready: bool,
    pub route_audit_ready: bool,
    pub route_handoff_ready: bool,
    pub attach_complete: bool,
    pub network_data_ready: bool,
    pub routing_surface_ready: bool,
    pub parent_or_route_anchor_ready: bool,
    pub route_export_ready: bool,
}

impl ThreadAttachRouteExportSummary {
    pub fn from_distribution_summary(
        distribution_summary: ThreadAttachRouteDistributionSummary,
    ) -> Self {
        let route_distribution_ready = distribution_summary.is_route_distribution_ready();
        let route_acceptance_ready = !distribution_summary.needs_route_acceptance();
        let route_adoption_ready = !distribution_summary.needs_route_adoption();
        let route_rollout_ready = !distribution_summary.needs_route_rollout();
        let route_activation_ready = !distribution_summary.needs_route_activation();
        let route_approval_ready = !distribution_summary.needs_route_approval();
        let route_certification_ready = !distribution_summary.needs_route_certification();
        let route_validation_ready = !distribution_summary.needs_route_validation();
        let route_verification_ready = !distribution_summary.needs_route_verification();
        let route_publication_ready = !distribution_summary.needs_route_publication();
        let route_completion_ready = !distribution_summary.needs_route_completion();
        let route_signoff_ready = !distribution_summary.needs_route_signoff();
        let route_audit_ready = !distribution_summary.needs_route_audit();
        let route_handoff_ready = !distribution_summary.needs_route_handoff();
        let attach_complete = !distribution_summary.needs_attach_completion();
        let network_data_ready = !distribution_summary.needs_network_data();
        let routing_surface_ready = !distribution_summary.needs_routing_surface();
        let parent_or_route_anchor_ready = !distribution_summary.needs_parent_or_route_anchor();
        let checks = [
            route_distribution_ready,
            route_acceptance_ready,
            route_adoption_ready,
            route_rollout_ready,
            route_activation_ready,
            route_approval_ready,
            route_certification_ready,
            route_validation_ready,
            route_verification_ready,
            route_publication_ready,
            route_completion_ready,
            route_signoff_ready,
            route_audit_ready,
            route_handoff_ready,
            attach_complete,
            network_data_ready,
            routing_surface_ready,
            parent_or_route_anchor_ready,
        ];
        let passed_route_export_check_count = checks.iter().filter(|ready| **ready).count();
        let required_route_export_check_count = checks.len();
        let missing_route_export_check_count =
            required_route_export_check_count - passed_route_export_check_count;
        let route_export_ready = missing_route_export_check_count == 0;

        Self {
            distribution_summary,
            required_route_export_check_count,
            passed_route_export_check_count,
            missing_route_export_check_count,
            route_distribution_ready,
            route_acceptance_ready,
            route_adoption_ready,
            route_rollout_ready,
            route_activation_ready,
            route_approval_ready,
            route_certification_ready,
            route_validation_ready,
            route_verification_ready,
            route_publication_ready,
            route_completion_ready,
            route_signoff_ready,
            route_audit_ready,
            route_handoff_ready,
            attach_complete,
            network_data_ready,
            routing_surface_ready,
            parent_or_route_anchor_ready,
            route_export_ready,
        }
    }

    pub fn is_route_export_ready(self) -> bool {
        self.route_export_ready
    }

    pub fn has_export_gaps(self) -> bool {
        self.missing_route_export_check_count > 0
    }

    pub fn needs_route_distribution(self) -> bool {
        !self.route_distribution_ready
    }

    pub fn needs_route_acceptance(self) -> bool {
        !self.route_acceptance_ready
    }

    pub fn needs_route_adoption(self) -> bool {
        !self.route_adoption_ready
    }

    pub fn needs_route_rollout(self) -> bool {
        !self.route_rollout_ready
    }

    pub fn needs_route_activation(self) -> bool {
        !self.route_activation_ready
    }

    pub fn needs_route_approval(self) -> bool {
        !self.route_approval_ready
    }

    pub fn needs_route_certification(self) -> bool {
        !self.route_certification_ready
    }

    pub fn needs_route_validation(self) -> bool {
        !self.route_validation_ready
    }

    pub fn needs_route_verification(self) -> bool {
        !self.route_verification_ready
    }

    pub fn needs_route_publication(self) -> bool {
        !self.route_publication_ready
    }

    pub fn needs_route_completion(self) -> bool {
        !self.route_completion_ready
    }

    pub fn needs_route_signoff(self) -> bool {
        !self.route_signoff_ready
    }

    pub fn needs_route_audit(self) -> bool {
        !self.route_audit_ready
    }

    pub fn needs_route_handoff(self) -> bool {
        !self.route_handoff_ready
    }

    pub fn needs_attach_completion(self) -> bool {
        !self.attach_complete
    }

    pub fn needs_network_data(self) -> bool {
        !self.network_data_ready
    }

    pub fn needs_routing_surface(self) -> bool {
        !self.routing_surface_ready
    }

    pub fn needs_parent_or_route_anchor(self) -> bool {
        !self.parent_or_route_anchor_ready
    }
}

pub fn summarize_thread_attach_route_export(
    distribution_summary: ThreadAttachRouteDistributionSummary,
) -> ThreadAttachRouteExportSummary {
    ThreadAttachRouteExportSummary::from_distribution_summary(distribution_summary)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadAttachRouteImportSummary {
    pub export_summary: ThreadAttachRouteExportSummary,
    pub required_route_import_check_count: usize,
    pub passed_route_import_check_count: usize,
    pub missing_route_import_check_count: usize,
    pub route_export_ready: bool,
    pub route_distribution_ready: bool,
    pub route_acceptance_ready: bool,
    pub route_adoption_ready: bool,
    pub route_rollout_ready: bool,
    pub route_activation_ready: bool,
    pub route_approval_ready: bool,
    pub route_certification_ready: bool,
    pub route_validation_ready: bool,
    pub route_verification_ready: bool,
    pub route_publication_ready: bool,
    pub route_completion_ready: bool,
    pub route_signoff_ready: bool,
    pub route_audit_ready: bool,
    pub route_handoff_ready: bool,
    pub attach_complete: bool,
    pub network_data_ready: bool,
    pub routing_surface_ready: bool,
    pub parent_or_route_anchor_ready: bool,
    pub route_import_ready: bool,
}

impl ThreadAttachRouteImportSummary {
    pub fn from_export_summary(export_summary: ThreadAttachRouteExportSummary) -> Self {
        let route_export_ready = export_summary.is_route_export_ready();
        let route_distribution_ready = !export_summary.needs_route_distribution();
        let route_acceptance_ready = !export_summary.needs_route_acceptance();
        let route_adoption_ready = !export_summary.needs_route_adoption();
        let route_rollout_ready = !export_summary.needs_route_rollout();
        let route_activation_ready = !export_summary.needs_route_activation();
        let route_approval_ready = !export_summary.needs_route_approval();
        let route_certification_ready = !export_summary.needs_route_certification();
        let route_validation_ready = !export_summary.needs_route_validation();
        let route_verification_ready = !export_summary.needs_route_verification();
        let route_publication_ready = !export_summary.needs_route_publication();
        let route_completion_ready = !export_summary.needs_route_completion();
        let route_signoff_ready = !export_summary.needs_route_signoff();
        let route_audit_ready = !export_summary.needs_route_audit();
        let route_handoff_ready = !export_summary.needs_route_handoff();
        let attach_complete = !export_summary.needs_attach_completion();
        let network_data_ready = !export_summary.needs_network_data();
        let routing_surface_ready = !export_summary.needs_routing_surface();
        let parent_or_route_anchor_ready = !export_summary.needs_parent_or_route_anchor();
        let checks = [
            route_export_ready,
            route_distribution_ready,
            route_acceptance_ready,
            route_adoption_ready,
            route_rollout_ready,
            route_activation_ready,
            route_approval_ready,
            route_certification_ready,
            route_validation_ready,
            route_verification_ready,
            route_publication_ready,
            route_completion_ready,
            route_signoff_ready,
            route_audit_ready,
            route_handoff_ready,
            attach_complete,
            network_data_ready,
            routing_surface_ready,
            parent_or_route_anchor_ready,
        ];
        let passed_route_import_check_count = checks.iter().filter(|ready| **ready).count();
        let required_route_import_check_count = checks.len();
        let missing_route_import_check_count =
            required_route_import_check_count - passed_route_import_check_count;
        let route_import_ready = missing_route_import_check_count == 0;

        Self {
            export_summary,
            required_route_import_check_count,
            passed_route_import_check_count,
            missing_route_import_check_count,
            route_export_ready,
            route_distribution_ready,
            route_acceptance_ready,
            route_adoption_ready,
            route_rollout_ready,
            route_activation_ready,
            route_approval_ready,
            route_certification_ready,
            route_validation_ready,
            route_verification_ready,
            route_publication_ready,
            route_completion_ready,
            route_signoff_ready,
            route_audit_ready,
            route_handoff_ready,
            attach_complete,
            network_data_ready,
            routing_surface_ready,
            parent_or_route_anchor_ready,
            route_import_ready,
        }
    }

    pub fn is_route_import_ready(self) -> bool {
        self.route_import_ready
    }

    pub fn has_import_gaps(self) -> bool {
        self.missing_route_import_check_count > 0
    }

    pub fn needs_route_export(self) -> bool {
        !self.route_export_ready
    }

    pub fn needs_route_distribution(self) -> bool {
        !self.route_distribution_ready
    }

    pub fn needs_route_acceptance(self) -> bool {
        !self.route_acceptance_ready
    }

    pub fn needs_route_adoption(self) -> bool {
        !self.route_adoption_ready
    }

    pub fn needs_route_rollout(self) -> bool {
        !self.route_rollout_ready
    }

    pub fn needs_route_activation(self) -> bool {
        !self.route_activation_ready
    }

    pub fn needs_route_approval(self) -> bool {
        !self.route_approval_ready
    }

    pub fn needs_route_certification(self) -> bool {
        !self.route_certification_ready
    }

    pub fn needs_route_validation(self) -> bool {
        !self.route_validation_ready
    }

    pub fn needs_route_verification(self) -> bool {
        !self.route_verification_ready
    }

    pub fn needs_route_publication(self) -> bool {
        !self.route_publication_ready
    }

    pub fn needs_route_completion(self) -> bool {
        !self.route_completion_ready
    }

    pub fn needs_route_signoff(self) -> bool {
        !self.route_signoff_ready
    }

    pub fn needs_route_audit(self) -> bool {
        !self.route_audit_ready
    }

    pub fn needs_route_handoff(self) -> bool {
        !self.route_handoff_ready
    }

    pub fn needs_attach_completion(self) -> bool {
        !self.attach_complete
    }

    pub fn needs_network_data(self) -> bool {
        !self.network_data_ready
    }

    pub fn needs_routing_surface(self) -> bool {
        !self.routing_surface_ready
    }

    pub fn needs_parent_or_route_anchor(self) -> bool {
        !self.parent_or_route_anchor_ready
    }
}

pub fn summarize_thread_attach_route_import(
    export_summary: ThreadAttachRouteExportSummary,
) -> ThreadAttachRouteImportSummary {
    ThreadAttachRouteImportSummary::from_export_summary(export_summary)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadAttachRouteIngestSummary {
    pub import_summary: ThreadAttachRouteImportSummary,
    pub required_route_ingest_check_count: usize,
    pub passed_route_ingest_check_count: usize,
    pub missing_route_ingest_check_count: usize,
    pub route_import_ready: bool,
    pub route_export_ready: bool,
    pub route_distribution_ready: bool,
    pub route_acceptance_ready: bool,
    pub route_adoption_ready: bool,
    pub route_rollout_ready: bool,
    pub route_activation_ready: bool,
    pub route_approval_ready: bool,
    pub route_certification_ready: bool,
    pub route_validation_ready: bool,
    pub route_verification_ready: bool,
    pub route_publication_ready: bool,
    pub route_completion_ready: bool,
    pub route_signoff_ready: bool,
    pub route_audit_ready: bool,
    pub route_handoff_ready: bool,
    pub attach_complete: bool,
    pub network_data_ready: bool,
    pub routing_surface_ready: bool,
    pub parent_or_route_anchor_ready: bool,
    pub route_ingest_ready: bool,
}

impl ThreadAttachRouteIngestSummary {
    pub fn from_import_summary(import_summary: ThreadAttachRouteImportSummary) -> Self {
        let route_import_ready = import_summary.is_route_import_ready();
        let route_export_ready = !import_summary.needs_route_export();
        let route_distribution_ready = !import_summary.needs_route_distribution();
        let route_acceptance_ready = !import_summary.needs_route_acceptance();
        let route_adoption_ready = !import_summary.needs_route_adoption();
        let route_rollout_ready = !import_summary.needs_route_rollout();
        let route_activation_ready = !import_summary.needs_route_activation();
        let route_approval_ready = !import_summary.needs_route_approval();
        let route_certification_ready = !import_summary.needs_route_certification();
        let route_validation_ready = !import_summary.needs_route_validation();
        let route_verification_ready = !import_summary.needs_route_verification();
        let route_publication_ready = !import_summary.needs_route_publication();
        let route_completion_ready = !import_summary.needs_route_completion();
        let route_signoff_ready = !import_summary.needs_route_signoff();
        let route_audit_ready = !import_summary.needs_route_audit();
        let route_handoff_ready = !import_summary.needs_route_handoff();
        let attach_complete = !import_summary.needs_attach_completion();
        let network_data_ready = !import_summary.needs_network_data();
        let routing_surface_ready = !import_summary.needs_routing_surface();
        let parent_or_route_anchor_ready = !import_summary.needs_parent_or_route_anchor();
        let checks = [
            route_import_ready,
            route_export_ready,
            route_distribution_ready,
            route_acceptance_ready,
            route_adoption_ready,
            route_rollout_ready,
            route_activation_ready,
            route_approval_ready,
            route_certification_ready,
            route_validation_ready,
            route_verification_ready,
            route_publication_ready,
            route_completion_ready,
            route_signoff_ready,
            route_audit_ready,
            route_handoff_ready,
            attach_complete,
            network_data_ready,
            routing_surface_ready,
            parent_or_route_anchor_ready,
        ];
        let passed_route_ingest_check_count = checks.iter().filter(|ready| **ready).count();
        let required_route_ingest_check_count = checks.len();
        let missing_route_ingest_check_count =
            required_route_ingest_check_count - passed_route_ingest_check_count;
        let route_ingest_ready = missing_route_ingest_check_count == 0;

        Self {
            import_summary,
            required_route_ingest_check_count,
            passed_route_ingest_check_count,
            missing_route_ingest_check_count,
            route_import_ready,
            route_export_ready,
            route_distribution_ready,
            route_acceptance_ready,
            route_adoption_ready,
            route_rollout_ready,
            route_activation_ready,
            route_approval_ready,
            route_certification_ready,
            route_validation_ready,
            route_verification_ready,
            route_publication_ready,
            route_completion_ready,
            route_signoff_ready,
            route_audit_ready,
            route_handoff_ready,
            attach_complete,
            network_data_ready,
            routing_surface_ready,
            parent_or_route_anchor_ready,
            route_ingest_ready,
        }
    }

    pub fn is_route_ingest_ready(self) -> bool {
        self.route_ingest_ready
    }

    pub fn has_ingest_gaps(self) -> bool {
        self.missing_route_ingest_check_count > 0
    }

    pub fn needs_route_import(self) -> bool {
        !self.route_import_ready
    }

    pub fn needs_route_export(self) -> bool {
        !self.route_export_ready
    }

    pub fn needs_route_distribution(self) -> bool {
        !self.route_distribution_ready
    }

    pub fn needs_route_acceptance(self) -> bool {
        !self.route_acceptance_ready
    }

    pub fn needs_route_adoption(self) -> bool {
        !self.route_adoption_ready
    }

    pub fn needs_route_rollout(self) -> bool {
        !self.route_rollout_ready
    }

    pub fn needs_route_activation(self) -> bool {
        !self.route_activation_ready
    }

    pub fn needs_route_approval(self) -> bool {
        !self.route_approval_ready
    }

    pub fn needs_route_certification(self) -> bool {
        !self.route_certification_ready
    }

    pub fn needs_route_validation(self) -> bool {
        !self.route_validation_ready
    }

    pub fn needs_route_verification(self) -> bool {
        !self.route_verification_ready
    }

    pub fn needs_route_publication(self) -> bool {
        !self.route_publication_ready
    }

    pub fn needs_route_completion(self) -> bool {
        !self.route_completion_ready
    }

    pub fn needs_route_signoff(self) -> bool {
        !self.route_signoff_ready
    }

    pub fn needs_route_audit(self) -> bool {
        !self.route_audit_ready
    }

    pub fn needs_route_handoff(self) -> bool {
        !self.route_handoff_ready
    }

    pub fn needs_attach_completion(self) -> bool {
        !self.attach_complete
    }

    pub fn needs_network_data(self) -> bool {
        !self.network_data_ready
    }

    pub fn needs_routing_surface(self) -> bool {
        !self.routing_surface_ready
    }

    pub fn needs_parent_or_route_anchor(self) -> bool {
        !self.parent_or_route_anchor_ready
    }
}

pub fn summarize_thread_attach_route_ingest(
    import_summary: ThreadAttachRouteImportSummary,
) -> ThreadAttachRouteIngestSummary {
    ThreadAttachRouteIngestSummary::from_import_summary(import_summary)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadAttachRouteLoadSummary {
    pub ingest_summary: ThreadAttachRouteIngestSummary,
    pub required_route_load_check_count: usize,
    pub passed_route_load_check_count: usize,
    pub missing_route_load_check_count: usize,
    pub route_ingest_ready: bool,
    pub route_import_ready: bool,
    pub route_export_ready: bool,
    pub route_distribution_ready: bool,
    pub route_acceptance_ready: bool,
    pub route_adoption_ready: bool,
    pub route_rollout_ready: bool,
    pub route_activation_ready: bool,
    pub route_approval_ready: bool,
    pub route_certification_ready: bool,
    pub route_validation_ready: bool,
    pub route_verification_ready: bool,
    pub route_publication_ready: bool,
    pub route_completion_ready: bool,
    pub route_signoff_ready: bool,
    pub route_audit_ready: bool,
    pub route_handoff_ready: bool,
    pub attach_complete: bool,
    pub network_data_ready: bool,
    pub routing_surface_ready: bool,
    pub parent_or_route_anchor_ready: bool,
    pub route_load_ready: bool,
}

impl ThreadAttachRouteLoadSummary {
    pub fn from_ingest_summary(ingest_summary: ThreadAttachRouteIngestSummary) -> Self {
        let route_ingest_ready = ingest_summary.is_route_ingest_ready();
        let route_import_ready = !ingest_summary.needs_route_import();
        let route_export_ready = !ingest_summary.needs_route_export();
        let route_distribution_ready = !ingest_summary.needs_route_distribution();
        let route_acceptance_ready = !ingest_summary.needs_route_acceptance();
        let route_adoption_ready = !ingest_summary.needs_route_adoption();
        let route_rollout_ready = !ingest_summary.needs_route_rollout();
        let route_activation_ready = !ingest_summary.needs_route_activation();
        let route_approval_ready = !ingest_summary.needs_route_approval();
        let route_certification_ready = !ingest_summary.needs_route_certification();
        let route_validation_ready = !ingest_summary.needs_route_validation();
        let route_verification_ready = !ingest_summary.needs_route_verification();
        let route_publication_ready = !ingest_summary.needs_route_publication();
        let route_completion_ready = !ingest_summary.needs_route_completion();
        let route_signoff_ready = !ingest_summary.needs_route_signoff();
        let route_audit_ready = !ingest_summary.needs_route_audit();
        let route_handoff_ready = !ingest_summary.needs_route_handoff();
        let attach_complete = !ingest_summary.needs_attach_completion();
        let network_data_ready = !ingest_summary.needs_network_data();
        let routing_surface_ready = !ingest_summary.needs_routing_surface();
        let parent_or_route_anchor_ready = !ingest_summary.needs_parent_or_route_anchor();
        let checks = [
            route_ingest_ready,
            route_import_ready,
            route_export_ready,
            route_distribution_ready,
            route_acceptance_ready,
            route_adoption_ready,
            route_rollout_ready,
            route_activation_ready,
            route_approval_ready,
            route_certification_ready,
            route_validation_ready,
            route_verification_ready,
            route_publication_ready,
            route_completion_ready,
            route_signoff_ready,
            route_audit_ready,
            route_handoff_ready,
            attach_complete,
            network_data_ready,
            routing_surface_ready,
            parent_or_route_anchor_ready,
        ];
        let passed_route_load_check_count = checks.iter().filter(|ready| **ready).count();
        let required_route_load_check_count = checks.len();
        let missing_route_load_check_count =
            required_route_load_check_count - passed_route_load_check_count;
        let route_load_ready = missing_route_load_check_count == 0;

        Self {
            ingest_summary,
            required_route_load_check_count,
            passed_route_load_check_count,
            missing_route_load_check_count,
            route_ingest_ready,
            route_import_ready,
            route_export_ready,
            route_distribution_ready,
            route_acceptance_ready,
            route_adoption_ready,
            route_rollout_ready,
            route_activation_ready,
            route_approval_ready,
            route_certification_ready,
            route_validation_ready,
            route_verification_ready,
            route_publication_ready,
            route_completion_ready,
            route_signoff_ready,
            route_audit_ready,
            route_handoff_ready,
            attach_complete,
            network_data_ready,
            routing_surface_ready,
            parent_or_route_anchor_ready,
            route_load_ready,
        }
    }

    pub fn is_route_load_ready(self) -> bool {
        self.route_load_ready
    }

    pub fn has_load_gaps(self) -> bool {
        self.missing_route_load_check_count > 0
    }

    pub fn needs_route_ingest(self) -> bool {
        !self.route_ingest_ready
    }

    pub fn needs_route_import(self) -> bool {
        !self.route_import_ready
    }

    pub fn needs_route_export(self) -> bool {
        !self.route_export_ready
    }

    pub fn needs_route_distribution(self) -> bool {
        !self.route_distribution_ready
    }

    pub fn needs_route_acceptance(self) -> bool {
        !self.route_acceptance_ready
    }

    pub fn needs_route_adoption(self) -> bool {
        !self.route_adoption_ready
    }

    pub fn needs_route_rollout(self) -> bool {
        !self.route_rollout_ready
    }

    pub fn needs_route_activation(self) -> bool {
        !self.route_activation_ready
    }

    pub fn needs_route_approval(self) -> bool {
        !self.route_approval_ready
    }

    pub fn needs_route_certification(self) -> bool {
        !self.route_certification_ready
    }

    pub fn needs_route_validation(self) -> bool {
        !self.route_validation_ready
    }

    pub fn needs_route_verification(self) -> bool {
        !self.route_verification_ready
    }

    pub fn needs_route_publication(self) -> bool {
        !self.route_publication_ready
    }

    pub fn needs_route_completion(self) -> bool {
        !self.route_completion_ready
    }

    pub fn needs_route_signoff(self) -> bool {
        !self.route_signoff_ready
    }

    pub fn needs_route_audit(self) -> bool {
        !self.route_audit_ready
    }

    pub fn needs_route_handoff(self) -> bool {
        !self.route_handoff_ready
    }

    pub fn needs_attach_completion(self) -> bool {
        !self.attach_complete
    }

    pub fn needs_network_data(self) -> bool {
        !self.network_data_ready
    }

    pub fn needs_routing_surface(self) -> bool {
        !self.routing_surface_ready
    }

    pub fn needs_parent_or_route_anchor(self) -> bool {
        !self.parent_or_route_anchor_ready
    }
}

pub fn summarize_thread_attach_route_load(
    ingest_summary: ThreadAttachRouteIngestSummary,
) -> ThreadAttachRouteLoadSummary {
    ThreadAttachRouteLoadSummary::from_ingest_summary(ingest_summary)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadAttachRouteRestoreSummary {
    pub load_summary: ThreadAttachRouteLoadSummary,
    pub required_route_restore_check_count: usize,
    pub passed_route_restore_check_count: usize,
    pub missing_route_restore_check_count: usize,
    pub route_load_ready: bool,
    pub route_ingest_ready: bool,
    pub route_import_ready: bool,
    pub route_export_ready: bool,
    pub route_distribution_ready: bool,
    pub route_acceptance_ready: bool,
    pub route_adoption_ready: bool,
    pub route_rollout_ready: bool,
    pub route_activation_ready: bool,
    pub route_approval_ready: bool,
    pub route_certification_ready: bool,
    pub route_validation_ready: bool,
    pub route_verification_ready: bool,
    pub route_publication_ready: bool,
    pub route_completion_ready: bool,
    pub route_signoff_ready: bool,
    pub route_audit_ready: bool,
    pub route_handoff_ready: bool,
    pub attach_complete: bool,
    pub network_data_ready: bool,
    pub routing_surface_ready: bool,
    pub parent_or_route_anchor_ready: bool,
    pub route_restore_ready: bool,
}

impl ThreadAttachRouteRestoreSummary {
    pub fn from_load_summary(load_summary: ThreadAttachRouteLoadSummary) -> Self {
        let route_load_ready = load_summary.is_route_load_ready();
        let route_ingest_ready = !load_summary.needs_route_ingest();
        let route_import_ready = !load_summary.needs_route_import();
        let route_export_ready = !load_summary.needs_route_export();
        let route_distribution_ready = !load_summary.needs_route_distribution();
        let route_acceptance_ready = !load_summary.needs_route_acceptance();
        let route_adoption_ready = !load_summary.needs_route_adoption();
        let route_rollout_ready = !load_summary.needs_route_rollout();
        let route_activation_ready = !load_summary.needs_route_activation();
        let route_approval_ready = !load_summary.needs_route_approval();
        let route_certification_ready = !load_summary.needs_route_certification();
        let route_validation_ready = !load_summary.needs_route_validation();
        let route_verification_ready = !load_summary.needs_route_verification();
        let route_publication_ready = !load_summary.needs_route_publication();
        let route_completion_ready = !load_summary.needs_route_completion();
        let route_signoff_ready = !load_summary.needs_route_signoff();
        let route_audit_ready = !load_summary.needs_route_audit();
        let route_handoff_ready = !load_summary.needs_route_handoff();
        let attach_complete = !load_summary.needs_attach_completion();
        let network_data_ready = !load_summary.needs_network_data();
        let routing_surface_ready = !load_summary.needs_routing_surface();
        let parent_or_route_anchor_ready = !load_summary.needs_parent_or_route_anchor();
        let checks = [
            route_load_ready,
            route_ingest_ready,
            route_import_ready,
            route_export_ready,
            route_distribution_ready,
            route_acceptance_ready,
            route_adoption_ready,
            route_rollout_ready,
            route_activation_ready,
            route_approval_ready,
            route_certification_ready,
            route_validation_ready,
            route_verification_ready,
            route_publication_ready,
            route_completion_ready,
            route_signoff_ready,
            route_audit_ready,
            route_handoff_ready,
            attach_complete,
            network_data_ready,
            routing_surface_ready,
            parent_or_route_anchor_ready,
        ];
        let passed_route_restore_check_count = checks.iter().filter(|ready| **ready).count();
        let required_route_restore_check_count = checks.len();
        let missing_route_restore_check_count =
            required_route_restore_check_count - passed_route_restore_check_count;
        let route_restore_ready = missing_route_restore_check_count == 0;

        Self {
            load_summary,
            required_route_restore_check_count,
            passed_route_restore_check_count,
            missing_route_restore_check_count,
            route_load_ready,
            route_ingest_ready,
            route_import_ready,
            route_export_ready,
            route_distribution_ready,
            route_acceptance_ready,
            route_adoption_ready,
            route_rollout_ready,
            route_activation_ready,
            route_approval_ready,
            route_certification_ready,
            route_validation_ready,
            route_verification_ready,
            route_publication_ready,
            route_completion_ready,
            route_signoff_ready,
            route_audit_ready,
            route_handoff_ready,
            attach_complete,
            network_data_ready,
            routing_surface_ready,
            parent_or_route_anchor_ready,
            route_restore_ready,
        }
    }

    pub fn is_route_restore_ready(self) -> bool {
        self.route_restore_ready
    }

    pub fn has_restore_gaps(self) -> bool {
        self.missing_route_restore_check_count > 0
    }

    pub fn needs_route_load(self) -> bool {
        !self.route_load_ready
    }

    pub fn needs_route_ingest(self) -> bool {
        !self.route_ingest_ready
    }

    pub fn needs_route_import(self) -> bool {
        !self.route_import_ready
    }

    pub fn needs_route_export(self) -> bool {
        !self.route_export_ready
    }

    pub fn needs_route_distribution(self) -> bool {
        !self.route_distribution_ready
    }

    pub fn needs_route_acceptance(self) -> bool {
        !self.route_acceptance_ready
    }

    pub fn needs_route_adoption(self) -> bool {
        !self.route_adoption_ready
    }

    pub fn needs_route_rollout(self) -> bool {
        !self.route_rollout_ready
    }

    pub fn needs_route_activation(self) -> bool {
        !self.route_activation_ready
    }

    pub fn needs_route_approval(self) -> bool {
        !self.route_approval_ready
    }

    pub fn needs_route_certification(self) -> bool {
        !self.route_certification_ready
    }

    pub fn needs_route_validation(self) -> bool {
        !self.route_validation_ready
    }

    pub fn needs_route_verification(self) -> bool {
        !self.route_verification_ready
    }

    pub fn needs_route_publication(self) -> bool {
        !self.route_publication_ready
    }

    pub fn needs_route_completion(self) -> bool {
        !self.route_completion_ready
    }

    pub fn needs_route_signoff(self) -> bool {
        !self.route_signoff_ready
    }

    pub fn needs_route_audit(self) -> bool {
        !self.route_audit_ready
    }

    pub fn needs_route_handoff(self) -> bool {
        !self.route_handoff_ready
    }

    pub fn needs_attach_completion(self) -> bool {
        !self.attach_complete
    }

    pub fn needs_network_data(self) -> bool {
        !self.network_data_ready
    }

    pub fn needs_routing_surface(self) -> bool {
        !self.routing_surface_ready
    }

    pub fn needs_parent_or_route_anchor(self) -> bool {
        !self.parent_or_route_anchor_ready
    }
}

pub fn summarize_thread_attach_route_restore(
    load_summary: ThreadAttachRouteLoadSummary,
) -> ThreadAttachRouteRestoreSummary {
    ThreadAttachRouteRestoreSummary::from_load_summary(load_summary)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadAttachRouteRecoverySummary {
    pub restore_summary: ThreadAttachRouteRestoreSummary,
    pub required_route_recovery_check_count: usize,
    pub passed_route_recovery_check_count: usize,
    pub missing_route_recovery_check_count: usize,
    pub route_restore_ready: bool,
    pub route_load_ready: bool,
    pub route_ingest_ready: bool,
    pub route_import_ready: bool,
    pub route_export_ready: bool,
    pub route_distribution_ready: bool,
    pub route_acceptance_ready: bool,
    pub route_adoption_ready: bool,
    pub route_rollout_ready: bool,
    pub route_activation_ready: bool,
    pub route_approval_ready: bool,
    pub route_certification_ready: bool,
    pub route_validation_ready: bool,
    pub route_verification_ready: bool,
    pub route_publication_ready: bool,
    pub route_completion_ready: bool,
    pub route_signoff_ready: bool,
    pub route_audit_ready: bool,
    pub route_handoff_ready: bool,
    pub attach_complete: bool,
    pub network_data_ready: bool,
    pub routing_surface_ready: bool,
    pub parent_or_route_anchor_ready: bool,
    pub route_recovery_ready: bool,
}

impl ThreadAttachRouteRecoverySummary {
    pub fn from_restore_summary(restore_summary: ThreadAttachRouteRestoreSummary) -> Self {
        let route_restore_ready = restore_summary.is_route_restore_ready();
        let route_load_ready = !restore_summary.needs_route_load();
        let route_ingest_ready = !restore_summary.needs_route_ingest();
        let route_import_ready = !restore_summary.needs_route_import();
        let route_export_ready = !restore_summary.needs_route_export();
        let route_distribution_ready = !restore_summary.needs_route_distribution();
        let route_acceptance_ready = !restore_summary.needs_route_acceptance();
        let route_adoption_ready = !restore_summary.needs_route_adoption();
        let route_rollout_ready = !restore_summary.needs_route_rollout();
        let route_activation_ready = !restore_summary.needs_route_activation();
        let route_approval_ready = !restore_summary.needs_route_approval();
        let route_certification_ready = !restore_summary.needs_route_certification();
        let route_validation_ready = !restore_summary.needs_route_validation();
        let route_verification_ready = !restore_summary.needs_route_verification();
        let route_publication_ready = !restore_summary.needs_route_publication();
        let route_completion_ready = !restore_summary.needs_route_completion();
        let route_signoff_ready = !restore_summary.needs_route_signoff();
        let route_audit_ready = !restore_summary.needs_route_audit();
        let route_handoff_ready = !restore_summary.needs_route_handoff();
        let attach_complete = !restore_summary.needs_attach_completion();
        let network_data_ready = !restore_summary.needs_network_data();
        let routing_surface_ready = !restore_summary.needs_routing_surface();
        let parent_or_route_anchor_ready = !restore_summary.needs_parent_or_route_anchor();
        let checks = [
            route_restore_ready,
            route_load_ready,
            route_ingest_ready,
            route_import_ready,
            route_export_ready,
            route_distribution_ready,
            route_acceptance_ready,
            route_adoption_ready,
            route_rollout_ready,
            route_activation_ready,
            route_approval_ready,
            route_certification_ready,
            route_validation_ready,
            route_verification_ready,
            route_publication_ready,
            route_completion_ready,
            route_signoff_ready,
            route_audit_ready,
            route_handoff_ready,
            attach_complete,
            network_data_ready,
            routing_surface_ready,
            parent_or_route_anchor_ready,
        ];
        let passed_route_recovery_check_count = checks.iter().filter(|ready| **ready).count();
        let required_route_recovery_check_count = checks.len();
        let missing_route_recovery_check_count =
            required_route_recovery_check_count - passed_route_recovery_check_count;
        let route_recovery_ready = missing_route_recovery_check_count == 0;

        Self {
            restore_summary,
            required_route_recovery_check_count,
            passed_route_recovery_check_count,
            missing_route_recovery_check_count,
            route_restore_ready,
            route_load_ready,
            route_ingest_ready,
            route_import_ready,
            route_export_ready,
            route_distribution_ready,
            route_acceptance_ready,
            route_adoption_ready,
            route_rollout_ready,
            route_activation_ready,
            route_approval_ready,
            route_certification_ready,
            route_validation_ready,
            route_verification_ready,
            route_publication_ready,
            route_completion_ready,
            route_signoff_ready,
            route_audit_ready,
            route_handoff_ready,
            attach_complete,
            network_data_ready,
            routing_surface_ready,
            parent_or_route_anchor_ready,
            route_recovery_ready,
        }
    }

    pub fn is_route_recovery_ready(self) -> bool {
        self.route_recovery_ready
    }

    pub fn has_recovery_gaps(self) -> bool {
        self.missing_route_recovery_check_count > 0
    }

    pub fn needs_route_restore(self) -> bool {
        !self.route_restore_ready
    }

    pub fn needs_route_load(self) -> bool {
        !self.route_load_ready
    }

    pub fn needs_route_ingest(self) -> bool {
        !self.route_ingest_ready
    }

    pub fn needs_route_import(self) -> bool {
        !self.route_import_ready
    }

    pub fn needs_route_export(self) -> bool {
        !self.route_export_ready
    }

    pub fn needs_route_distribution(self) -> bool {
        !self.route_distribution_ready
    }

    pub fn needs_route_acceptance(self) -> bool {
        !self.route_acceptance_ready
    }

    pub fn needs_route_adoption(self) -> bool {
        !self.route_adoption_ready
    }

    pub fn needs_route_rollout(self) -> bool {
        !self.route_rollout_ready
    }

    pub fn needs_route_activation(self) -> bool {
        !self.route_activation_ready
    }

    pub fn needs_route_approval(self) -> bool {
        !self.route_approval_ready
    }

    pub fn needs_route_certification(self) -> bool {
        !self.route_certification_ready
    }

    pub fn needs_route_validation(self) -> bool {
        !self.route_validation_ready
    }

    pub fn needs_route_verification(self) -> bool {
        !self.route_verification_ready
    }

    pub fn needs_route_publication(self) -> bool {
        !self.route_publication_ready
    }

    pub fn needs_route_completion(self) -> bool {
        !self.route_completion_ready
    }

    pub fn needs_route_signoff(self) -> bool {
        !self.route_signoff_ready
    }

    pub fn needs_route_audit(self) -> bool {
        !self.route_audit_ready
    }

    pub fn needs_route_handoff(self) -> bool {
        !self.route_handoff_ready
    }

    pub fn needs_attach_completion(self) -> bool {
        !self.attach_complete
    }

    pub fn needs_network_data(self) -> bool {
        !self.network_data_ready
    }

    pub fn needs_routing_surface(self) -> bool {
        !self.routing_surface_ready
    }

    pub fn needs_parent_or_route_anchor(self) -> bool {
        !self.parent_or_route_anchor_ready
    }
}

pub fn summarize_thread_attach_route_recovery(
    restore_summary: ThreadAttachRouteRestoreSummary,
) -> ThreadAttachRouteRecoverySummary {
    ThreadAttachRouteRecoverySummary::from_restore_summary(restore_summary)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadAttachRouteReplaySummary {
    pub recovery_summary: ThreadAttachRouteRecoverySummary,
    pub required_route_replay_check_count: usize,
    pub passed_route_replay_check_count: usize,
    pub missing_route_replay_check_count: usize,
    pub route_recovery_ready: bool,
    pub route_restore_ready: bool,
    pub route_load_ready: bool,
    pub route_ingest_ready: bool,
    pub route_import_ready: bool,
    pub route_export_ready: bool,
    pub route_distribution_ready: bool,
    pub route_acceptance_ready: bool,
    pub route_adoption_ready: bool,
    pub route_rollout_ready: bool,
    pub route_activation_ready: bool,
    pub route_approval_ready: bool,
    pub route_certification_ready: bool,
    pub route_validation_ready: bool,
    pub route_verification_ready: bool,
    pub route_publication_ready: bool,
    pub route_completion_ready: bool,
    pub route_signoff_ready: bool,
    pub route_audit_ready: bool,
    pub route_handoff_ready: bool,
    pub attach_complete: bool,
    pub network_data_ready: bool,
    pub routing_surface_ready: bool,
    pub parent_or_route_anchor_ready: bool,
    pub route_replay_ready: bool,
}

impl ThreadAttachRouteReplaySummary {
    pub fn from_recovery_summary(recovery_summary: ThreadAttachRouteRecoverySummary) -> Self {
        let route_recovery_ready = recovery_summary.is_route_recovery_ready();
        let route_restore_ready = !recovery_summary.needs_route_restore();
        let route_load_ready = !recovery_summary.needs_route_load();
        let route_ingest_ready = !recovery_summary.needs_route_ingest();
        let route_import_ready = !recovery_summary.needs_route_import();
        let route_export_ready = !recovery_summary.needs_route_export();
        let route_distribution_ready = !recovery_summary.needs_route_distribution();
        let route_acceptance_ready = !recovery_summary.needs_route_acceptance();
        let route_adoption_ready = !recovery_summary.needs_route_adoption();
        let route_rollout_ready = !recovery_summary.needs_route_rollout();
        let route_activation_ready = !recovery_summary.needs_route_activation();
        let route_approval_ready = !recovery_summary.needs_route_approval();
        let route_certification_ready = !recovery_summary.needs_route_certification();
        let route_validation_ready = !recovery_summary.needs_route_validation();
        let route_verification_ready = !recovery_summary.needs_route_verification();
        let route_publication_ready = !recovery_summary.needs_route_publication();
        let route_completion_ready = !recovery_summary.needs_route_completion();
        let route_signoff_ready = !recovery_summary.needs_route_signoff();
        let route_audit_ready = !recovery_summary.needs_route_audit();
        let route_handoff_ready = !recovery_summary.needs_route_handoff();
        let attach_complete = !recovery_summary.needs_attach_completion();
        let network_data_ready = !recovery_summary.needs_network_data();
        let routing_surface_ready = !recovery_summary.needs_routing_surface();
        let parent_or_route_anchor_ready = !recovery_summary.needs_parent_or_route_anchor();
        let checks = [
            route_recovery_ready,
            route_restore_ready,
            route_load_ready,
            route_ingest_ready,
            route_import_ready,
            route_export_ready,
            route_distribution_ready,
            route_acceptance_ready,
            route_adoption_ready,
            route_rollout_ready,
            route_activation_ready,
            route_approval_ready,
            route_certification_ready,
            route_validation_ready,
            route_verification_ready,
            route_publication_ready,
            route_completion_ready,
            route_signoff_ready,
            route_audit_ready,
            route_handoff_ready,
            attach_complete,
            network_data_ready,
            routing_surface_ready,
            parent_or_route_anchor_ready,
        ];
        let passed_route_replay_check_count = checks.iter().filter(|ready| **ready).count();
        let required_route_replay_check_count = checks.len();
        let missing_route_replay_check_count =
            required_route_replay_check_count - passed_route_replay_check_count;
        let route_replay_ready = missing_route_replay_check_count == 0;

        Self {
            recovery_summary,
            required_route_replay_check_count,
            passed_route_replay_check_count,
            missing_route_replay_check_count,
            route_recovery_ready,
            route_restore_ready,
            route_load_ready,
            route_ingest_ready,
            route_import_ready,
            route_export_ready,
            route_distribution_ready,
            route_acceptance_ready,
            route_adoption_ready,
            route_rollout_ready,
            route_activation_ready,
            route_approval_ready,
            route_certification_ready,
            route_validation_ready,
            route_verification_ready,
            route_publication_ready,
            route_completion_ready,
            route_signoff_ready,
            route_audit_ready,
            route_handoff_ready,
            attach_complete,
            network_data_ready,
            routing_surface_ready,
            parent_or_route_anchor_ready,
            route_replay_ready,
        }
    }

    pub fn is_route_replay_ready(self) -> bool {
        self.route_replay_ready
    }

    pub fn has_replay_gaps(self) -> bool {
        self.missing_route_replay_check_count > 0
    }

    pub fn needs_route_recovery(self) -> bool {
        !self.route_recovery_ready
    }

    pub fn needs_route_restore(self) -> bool {
        !self.route_restore_ready
    }

    pub fn needs_route_load(self) -> bool {
        !self.route_load_ready
    }

    pub fn needs_route_ingest(self) -> bool {
        !self.route_ingest_ready
    }

    pub fn needs_route_import(self) -> bool {
        !self.route_import_ready
    }

    pub fn needs_route_export(self) -> bool {
        !self.route_export_ready
    }

    pub fn needs_route_distribution(self) -> bool {
        !self.route_distribution_ready
    }

    pub fn needs_route_acceptance(self) -> bool {
        !self.route_acceptance_ready
    }

    pub fn needs_route_adoption(self) -> bool {
        !self.route_adoption_ready
    }

    pub fn needs_route_rollout(self) -> bool {
        !self.route_rollout_ready
    }

    pub fn needs_route_activation(self) -> bool {
        !self.route_activation_ready
    }

    pub fn needs_route_approval(self) -> bool {
        !self.route_approval_ready
    }

    pub fn needs_route_certification(self) -> bool {
        !self.route_certification_ready
    }

    pub fn needs_route_validation(self) -> bool {
        !self.route_validation_ready
    }

    pub fn needs_route_verification(self) -> bool {
        !self.route_verification_ready
    }

    pub fn needs_route_publication(self) -> bool {
        !self.route_publication_ready
    }

    pub fn needs_route_completion(self) -> bool {
        !self.route_completion_ready
    }

    pub fn needs_route_signoff(self) -> bool {
        !self.route_signoff_ready
    }

    pub fn needs_route_audit(self) -> bool {
        !self.route_audit_ready
    }

    pub fn needs_route_handoff(self) -> bool {
        !self.route_handoff_ready
    }

    pub fn needs_attach_completion(self) -> bool {
        !self.attach_complete
    }

    pub fn needs_network_data(self) -> bool {
        !self.network_data_ready
    }

    pub fn needs_routing_surface(self) -> bool {
        !self.routing_surface_ready
    }

    pub fn needs_parent_or_route_anchor(self) -> bool {
        !self.parent_or_route_anchor_ready
    }
}

pub fn summarize_thread_attach_route_replay(
    recovery_summary: ThreadAttachRouteRecoverySummary,
) -> ThreadAttachRouteReplaySummary {
    ThreadAttachRouteReplaySummary::from_recovery_summary(recovery_summary)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadAttachRouteReconciliationSummary {
    pub replay_summary: ThreadAttachRouteReplaySummary,
    pub required_route_reconciliation_check_count: usize,
    pub passed_route_reconciliation_check_count: usize,
    pub missing_route_reconciliation_check_count: usize,
    pub route_replay_ready: bool,
    pub route_recovery_ready: bool,
    pub route_restore_ready: bool,
    pub route_load_ready: bool,
    pub route_ingest_ready: bool,
    pub route_import_ready: bool,
    pub route_export_ready: bool,
    pub route_distribution_ready: bool,
    pub route_acceptance_ready: bool,
    pub route_adoption_ready: bool,
    pub route_rollout_ready: bool,
    pub route_activation_ready: bool,
    pub route_approval_ready: bool,
    pub route_certification_ready: bool,
    pub route_validation_ready: bool,
    pub route_verification_ready: bool,
    pub route_publication_ready: bool,
    pub route_completion_ready: bool,
    pub route_signoff_ready: bool,
    pub route_audit_ready: bool,
    pub route_handoff_ready: bool,
    pub attach_complete: bool,
    pub network_data_ready: bool,
    pub routing_surface_ready: bool,
    pub parent_or_route_anchor_ready: bool,
    pub route_reconciliation_ready: bool,
}

impl ThreadAttachRouteReconciliationSummary {
    pub fn from_replay_summary(replay_summary: ThreadAttachRouteReplaySummary) -> Self {
        let route_replay_ready = replay_summary.is_route_replay_ready();
        let route_recovery_ready = !replay_summary.needs_route_recovery();
        let route_restore_ready = !replay_summary.needs_route_restore();
        let route_load_ready = !replay_summary.needs_route_load();
        let route_ingest_ready = !replay_summary.needs_route_ingest();
        let route_import_ready = !replay_summary.needs_route_import();
        let route_export_ready = !replay_summary.needs_route_export();
        let route_distribution_ready = !replay_summary.needs_route_distribution();
        let route_acceptance_ready = !replay_summary.needs_route_acceptance();
        let route_adoption_ready = !replay_summary.needs_route_adoption();
        let route_rollout_ready = !replay_summary.needs_route_rollout();
        let route_activation_ready = !replay_summary.needs_route_activation();
        let route_approval_ready = !replay_summary.needs_route_approval();
        let route_certification_ready = !replay_summary.needs_route_certification();
        let route_validation_ready = !replay_summary.needs_route_validation();
        let route_verification_ready = !replay_summary.needs_route_verification();
        let route_publication_ready = !replay_summary.needs_route_publication();
        let route_completion_ready = !replay_summary.needs_route_completion();
        let route_signoff_ready = !replay_summary.needs_route_signoff();
        let route_audit_ready = !replay_summary.needs_route_audit();
        let route_handoff_ready = !replay_summary.needs_route_handoff();
        let attach_complete = !replay_summary.needs_attach_completion();
        let network_data_ready = !replay_summary.needs_network_data();
        let routing_surface_ready = !replay_summary.needs_routing_surface();
        let parent_or_route_anchor_ready = !replay_summary.needs_parent_or_route_anchor();
        let checks = [
            route_replay_ready,
            route_recovery_ready,
            route_restore_ready,
            route_load_ready,
            route_ingest_ready,
            route_import_ready,
            route_export_ready,
            route_distribution_ready,
            route_acceptance_ready,
            route_adoption_ready,
            route_rollout_ready,
            route_activation_ready,
            route_approval_ready,
            route_certification_ready,
            route_validation_ready,
            route_verification_ready,
            route_publication_ready,
            route_completion_ready,
            route_signoff_ready,
            route_audit_ready,
            route_handoff_ready,
            attach_complete,
            network_data_ready,
            routing_surface_ready,
            parent_or_route_anchor_ready,
        ];
        let passed_route_reconciliation_check_count = checks.iter().filter(|ready| **ready).count();
        let required_route_reconciliation_check_count = checks.len();
        let missing_route_reconciliation_check_count =
            required_route_reconciliation_check_count - passed_route_reconciliation_check_count;
        let route_reconciliation_ready = missing_route_reconciliation_check_count == 0;

        Self {
            replay_summary,
            required_route_reconciliation_check_count,
            passed_route_reconciliation_check_count,
            missing_route_reconciliation_check_count,
            route_replay_ready,
            route_recovery_ready,
            route_restore_ready,
            route_load_ready,
            route_ingest_ready,
            route_import_ready,
            route_export_ready,
            route_distribution_ready,
            route_acceptance_ready,
            route_adoption_ready,
            route_rollout_ready,
            route_activation_ready,
            route_approval_ready,
            route_certification_ready,
            route_validation_ready,
            route_verification_ready,
            route_publication_ready,
            route_completion_ready,
            route_signoff_ready,
            route_audit_ready,
            route_handoff_ready,
            attach_complete,
            network_data_ready,
            routing_surface_ready,
            parent_or_route_anchor_ready,
            route_reconciliation_ready,
        }
    }

    pub fn is_route_reconciliation_ready(self) -> bool {
        self.route_reconciliation_ready
    }

    pub fn has_reconciliation_gaps(self) -> bool {
        self.missing_route_reconciliation_check_count > 0
    }

    pub fn needs_route_replay(self) -> bool {
        !self.route_replay_ready
    }

    pub fn needs_route_recovery(self) -> bool {
        !self.route_recovery_ready
    }

    pub fn needs_route_restore(self) -> bool {
        !self.route_restore_ready
    }

    pub fn needs_route_load(self) -> bool {
        !self.route_load_ready
    }

    pub fn needs_route_ingest(self) -> bool {
        !self.route_ingest_ready
    }

    pub fn needs_route_import(self) -> bool {
        !self.route_import_ready
    }

    pub fn needs_route_export(self) -> bool {
        !self.route_export_ready
    }

    pub fn needs_route_distribution(self) -> bool {
        !self.route_distribution_ready
    }

    pub fn needs_route_acceptance(self) -> bool {
        !self.route_acceptance_ready
    }

    pub fn needs_route_adoption(self) -> bool {
        !self.route_adoption_ready
    }

    pub fn needs_route_rollout(self) -> bool {
        !self.route_rollout_ready
    }

    pub fn needs_route_activation(self) -> bool {
        !self.route_activation_ready
    }

    pub fn needs_route_approval(self) -> bool {
        !self.route_approval_ready
    }

    pub fn needs_route_certification(self) -> bool {
        !self.route_certification_ready
    }

    pub fn needs_route_validation(self) -> bool {
        !self.route_validation_ready
    }

    pub fn needs_route_verification(self) -> bool {
        !self.route_verification_ready
    }

    pub fn needs_route_publication(self) -> bool {
        !self.route_publication_ready
    }

    pub fn needs_route_completion(self) -> bool {
        !self.route_completion_ready
    }

    pub fn needs_route_signoff(self) -> bool {
        !self.route_signoff_ready
    }

    pub fn needs_route_audit(self) -> bool {
        !self.route_audit_ready
    }

    pub fn needs_route_handoff(self) -> bool {
        !self.route_handoff_ready
    }

    pub fn needs_attach_completion(self) -> bool {
        !self.attach_complete
    }

    pub fn needs_network_data(self) -> bool {
        !self.network_data_ready
    }

    pub fn needs_routing_surface(self) -> bool {
        !self.routing_surface_ready
    }

    pub fn needs_parent_or_route_anchor(self) -> bool {
        !self.parent_or_route_anchor_ready
    }
}

pub fn summarize_thread_attach_route_reconciliation(
    replay_summary: ThreadAttachRouteReplaySummary,
) -> ThreadAttachRouteReconciliationSummary {
    ThreadAttachRouteReconciliationSummary::from_replay_summary(replay_summary)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadAttachRouteSettlementSummary {
    pub reconciliation_summary: ThreadAttachRouteReconciliationSummary,
    pub required_route_settlement_check_count: usize,
    pub passed_route_settlement_check_count: usize,
    pub missing_route_settlement_check_count: usize,
    pub route_reconciliation_ready: bool,
    pub route_replay_ready: bool,
    pub route_recovery_ready: bool,
    pub route_restore_ready: bool,
    pub route_load_ready: bool,
    pub route_ingest_ready: bool,
    pub route_import_ready: bool,
    pub route_export_ready: bool,
    pub route_distribution_ready: bool,
    pub route_acceptance_ready: bool,
    pub route_adoption_ready: bool,
    pub route_rollout_ready: bool,
    pub route_activation_ready: bool,
    pub route_approval_ready: bool,
    pub route_certification_ready: bool,
    pub route_validation_ready: bool,
    pub route_verification_ready: bool,
    pub route_publication_ready: bool,
    pub route_completion_ready: bool,
    pub route_signoff_ready: bool,
    pub route_audit_ready: bool,
    pub route_handoff_ready: bool,
    pub attach_complete: bool,
    pub network_data_ready: bool,
    pub routing_surface_ready: bool,
    pub parent_or_route_anchor_ready: bool,
    pub route_settlement_ready: bool,
}

impl ThreadAttachRouteSettlementSummary {
    pub fn from_reconciliation_summary(
        reconciliation_summary: ThreadAttachRouteReconciliationSummary,
    ) -> Self {
        let route_reconciliation_ready = reconciliation_summary.is_route_reconciliation_ready();
        let route_replay_ready = !reconciliation_summary.needs_route_replay();
        let route_recovery_ready = !reconciliation_summary.needs_route_recovery();
        let route_restore_ready = !reconciliation_summary.needs_route_restore();
        let route_load_ready = !reconciliation_summary.needs_route_load();
        let route_ingest_ready = !reconciliation_summary.needs_route_ingest();
        let route_import_ready = !reconciliation_summary.needs_route_import();
        let route_export_ready = !reconciliation_summary.needs_route_export();
        let route_distribution_ready = !reconciliation_summary.needs_route_distribution();
        let route_acceptance_ready = !reconciliation_summary.needs_route_acceptance();
        let route_adoption_ready = !reconciliation_summary.needs_route_adoption();
        let route_rollout_ready = !reconciliation_summary.needs_route_rollout();
        let route_activation_ready = !reconciliation_summary.needs_route_activation();
        let route_approval_ready = !reconciliation_summary.needs_route_approval();
        let route_certification_ready = !reconciliation_summary.needs_route_certification();
        let route_validation_ready = !reconciliation_summary.needs_route_validation();
        let route_verification_ready = !reconciliation_summary.needs_route_verification();
        let route_publication_ready = !reconciliation_summary.needs_route_publication();
        let route_completion_ready = !reconciliation_summary.needs_route_completion();
        let route_signoff_ready = !reconciliation_summary.needs_route_signoff();
        let route_audit_ready = !reconciliation_summary.needs_route_audit();
        let route_handoff_ready = !reconciliation_summary.needs_route_handoff();
        let attach_complete = !reconciliation_summary.needs_attach_completion();
        let network_data_ready = !reconciliation_summary.needs_network_data();
        let routing_surface_ready = !reconciliation_summary.needs_routing_surface();
        let parent_or_route_anchor_ready = !reconciliation_summary.needs_parent_or_route_anchor();
        let checks = [
            route_reconciliation_ready,
            route_replay_ready,
            route_recovery_ready,
            route_restore_ready,
            route_load_ready,
            route_ingest_ready,
            route_import_ready,
            route_export_ready,
            route_distribution_ready,
            route_acceptance_ready,
            route_adoption_ready,
            route_rollout_ready,
            route_activation_ready,
            route_approval_ready,
            route_certification_ready,
            route_validation_ready,
            route_verification_ready,
            route_publication_ready,
            route_completion_ready,
            route_signoff_ready,
            route_audit_ready,
            route_handoff_ready,
            attach_complete,
            network_data_ready,
            routing_surface_ready,
            parent_or_route_anchor_ready,
        ];
        let passed_route_settlement_check_count = checks.iter().filter(|ready| **ready).count();
        let required_route_settlement_check_count = checks.len();
        let missing_route_settlement_check_count =
            required_route_settlement_check_count - passed_route_settlement_check_count;
        let route_settlement_ready = missing_route_settlement_check_count == 0;

        Self {
            reconciliation_summary,
            required_route_settlement_check_count,
            passed_route_settlement_check_count,
            missing_route_settlement_check_count,
            route_reconciliation_ready,
            route_replay_ready,
            route_recovery_ready,
            route_restore_ready,
            route_load_ready,
            route_ingest_ready,
            route_import_ready,
            route_export_ready,
            route_distribution_ready,
            route_acceptance_ready,
            route_adoption_ready,
            route_rollout_ready,
            route_activation_ready,
            route_approval_ready,
            route_certification_ready,
            route_validation_ready,
            route_verification_ready,
            route_publication_ready,
            route_completion_ready,
            route_signoff_ready,
            route_audit_ready,
            route_handoff_ready,
            attach_complete,
            network_data_ready,
            routing_surface_ready,
            parent_or_route_anchor_ready,
            route_settlement_ready,
        }
    }

    pub fn is_route_settlement_ready(self) -> bool {
        self.route_settlement_ready
    }

    pub fn has_settlement_gaps(self) -> bool {
        self.missing_route_settlement_check_count > 0
    }

    pub fn needs_route_reconciliation(self) -> bool {
        !self.route_reconciliation_ready
    }

    pub fn needs_route_replay(self) -> bool {
        !self.route_replay_ready
    }

    pub fn needs_route_recovery(self) -> bool {
        !self.route_recovery_ready
    }

    pub fn needs_route_restore(self) -> bool {
        !self.route_restore_ready
    }

    pub fn needs_route_load(self) -> bool {
        !self.route_load_ready
    }

    pub fn needs_route_ingest(self) -> bool {
        !self.route_ingest_ready
    }

    pub fn needs_route_import(self) -> bool {
        !self.route_import_ready
    }

    pub fn needs_route_export(self) -> bool {
        !self.route_export_ready
    }

    pub fn needs_route_distribution(self) -> bool {
        !self.route_distribution_ready
    }

    pub fn needs_route_acceptance(self) -> bool {
        !self.route_acceptance_ready
    }

    pub fn needs_route_adoption(self) -> bool {
        !self.route_adoption_ready
    }

    pub fn needs_route_rollout(self) -> bool {
        !self.route_rollout_ready
    }

    pub fn needs_route_activation(self) -> bool {
        !self.route_activation_ready
    }

    pub fn needs_route_approval(self) -> bool {
        !self.route_approval_ready
    }

    pub fn needs_route_certification(self) -> bool {
        !self.route_certification_ready
    }

    pub fn needs_route_validation(self) -> bool {
        !self.route_validation_ready
    }

    pub fn needs_route_verification(self) -> bool {
        !self.route_verification_ready
    }

    pub fn needs_route_publication(self) -> bool {
        !self.route_publication_ready
    }

    pub fn needs_route_completion(self) -> bool {
        !self.route_completion_ready
    }

    pub fn needs_route_signoff(self) -> bool {
        !self.route_signoff_ready
    }

    pub fn needs_route_audit(self) -> bool {
        !self.route_audit_ready
    }

    pub fn needs_route_handoff(self) -> bool {
        !self.route_handoff_ready
    }

    pub fn needs_attach_completion(self) -> bool {
        !self.attach_complete
    }

    pub fn needs_network_data(self) -> bool {
        !self.network_data_ready
    }

    pub fn needs_routing_surface(self) -> bool {
        !self.routing_surface_ready
    }

    pub fn needs_parent_or_route_anchor(self) -> bool {
        !self.parent_or_route_anchor_ready
    }
}

pub fn summarize_thread_attach_route_settlement(
    reconciliation_summary: ThreadAttachRouteReconciliationSummary,
) -> ThreadAttachRouteSettlementSummary {
    ThreadAttachRouteSettlementSummary::from_reconciliation_summary(reconciliation_summary)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadAttachRouteFinalizationSummary {
    pub settlement_summary: ThreadAttachRouteSettlementSummary,
    pub required_route_finalization_check_count: usize,
    pub passed_route_finalization_check_count: usize,
    pub missing_route_finalization_check_count: usize,
    pub route_settlement_ready: bool,
    pub route_reconciliation_ready: bool,
    pub route_replay_ready: bool,
    pub route_recovery_ready: bool,
    pub route_restore_ready: bool,
    pub route_load_ready: bool,
    pub route_ingest_ready: bool,
    pub route_import_ready: bool,
    pub route_export_ready: bool,
    pub route_distribution_ready: bool,
    pub route_acceptance_ready: bool,
    pub route_adoption_ready: bool,
    pub route_rollout_ready: bool,
    pub route_activation_ready: bool,
    pub route_approval_ready: bool,
    pub route_certification_ready: bool,
    pub route_validation_ready: bool,
    pub route_verification_ready: bool,
    pub route_publication_ready: bool,
    pub route_completion_ready: bool,
    pub route_signoff_ready: bool,
    pub route_audit_ready: bool,
    pub route_handoff_ready: bool,
    pub attach_complete: bool,
    pub network_data_ready: bool,
    pub routing_surface_ready: bool,
    pub parent_or_route_anchor_ready: bool,
    pub route_finalization_ready: bool,
}

impl ThreadAttachRouteFinalizationSummary {
    pub fn from_settlement_summary(settlement_summary: ThreadAttachRouteSettlementSummary) -> Self {
        let route_settlement_ready = settlement_summary.is_route_settlement_ready();
        let route_reconciliation_ready = !settlement_summary.needs_route_reconciliation();
        let route_replay_ready = !settlement_summary.needs_route_replay();
        let route_recovery_ready = !settlement_summary.needs_route_recovery();
        let route_restore_ready = !settlement_summary.needs_route_restore();
        let route_load_ready = !settlement_summary.needs_route_load();
        let route_ingest_ready = !settlement_summary.needs_route_ingest();
        let route_import_ready = !settlement_summary.needs_route_import();
        let route_export_ready = !settlement_summary.needs_route_export();
        let route_distribution_ready = !settlement_summary.needs_route_distribution();
        let route_acceptance_ready = !settlement_summary.needs_route_acceptance();
        let route_adoption_ready = !settlement_summary.needs_route_adoption();
        let route_rollout_ready = !settlement_summary.needs_route_rollout();
        let route_activation_ready = !settlement_summary.needs_route_activation();
        let route_approval_ready = !settlement_summary.needs_route_approval();
        let route_certification_ready = !settlement_summary.needs_route_certification();
        let route_validation_ready = !settlement_summary.needs_route_validation();
        let route_verification_ready = !settlement_summary.needs_route_verification();
        let route_publication_ready = !settlement_summary.needs_route_publication();
        let route_completion_ready = !settlement_summary.needs_route_completion();
        let route_signoff_ready = !settlement_summary.needs_route_signoff();
        let route_audit_ready = !settlement_summary.needs_route_audit();
        let route_handoff_ready = !settlement_summary.needs_route_handoff();
        let attach_complete = !settlement_summary.needs_attach_completion();
        let network_data_ready = !settlement_summary.needs_network_data();
        let routing_surface_ready = !settlement_summary.needs_routing_surface();
        let parent_or_route_anchor_ready = !settlement_summary.needs_parent_or_route_anchor();
        let checks = [
            route_settlement_ready,
            route_reconciliation_ready,
            route_replay_ready,
            route_recovery_ready,
            route_restore_ready,
            route_load_ready,
            route_ingest_ready,
            route_import_ready,
            route_export_ready,
            route_distribution_ready,
            route_acceptance_ready,
            route_adoption_ready,
            route_rollout_ready,
            route_activation_ready,
            route_approval_ready,
            route_certification_ready,
            route_validation_ready,
            route_verification_ready,
            route_publication_ready,
            route_completion_ready,
            route_signoff_ready,
            route_audit_ready,
            route_handoff_ready,
            attach_complete,
            network_data_ready,
            routing_surface_ready,
            parent_or_route_anchor_ready,
        ];
        let passed_route_finalization_check_count = checks.iter().filter(|ready| **ready).count();
        let required_route_finalization_check_count = checks.len();
        let missing_route_finalization_check_count =
            required_route_finalization_check_count - passed_route_finalization_check_count;
        let route_finalization_ready = missing_route_finalization_check_count == 0;

        Self {
            settlement_summary,
            required_route_finalization_check_count,
            passed_route_finalization_check_count,
            missing_route_finalization_check_count,
            route_settlement_ready,
            route_reconciliation_ready,
            route_replay_ready,
            route_recovery_ready,
            route_restore_ready,
            route_load_ready,
            route_ingest_ready,
            route_import_ready,
            route_export_ready,
            route_distribution_ready,
            route_acceptance_ready,
            route_adoption_ready,
            route_rollout_ready,
            route_activation_ready,
            route_approval_ready,
            route_certification_ready,
            route_validation_ready,
            route_verification_ready,
            route_publication_ready,
            route_completion_ready,
            route_signoff_ready,
            route_audit_ready,
            route_handoff_ready,
            attach_complete,
            network_data_ready,
            routing_surface_ready,
            parent_or_route_anchor_ready,
            route_finalization_ready,
        }
    }

    pub fn is_route_finalization_ready(self) -> bool {
        self.route_finalization_ready
    }

    pub fn has_finalization_gaps(self) -> bool {
        self.missing_route_finalization_check_count > 0
    }

    pub fn needs_route_settlement(self) -> bool {
        !self.route_settlement_ready
    }

    pub fn needs_route_reconciliation(self) -> bool {
        !self.route_reconciliation_ready
    }

    pub fn needs_route_replay(self) -> bool {
        !self.route_replay_ready
    }

    pub fn needs_route_recovery(self) -> bool {
        !self.route_recovery_ready
    }

    pub fn needs_route_restore(self) -> bool {
        !self.route_restore_ready
    }

    pub fn needs_route_load(self) -> bool {
        !self.route_load_ready
    }

    pub fn needs_route_ingest(self) -> bool {
        !self.route_ingest_ready
    }

    pub fn needs_route_import(self) -> bool {
        !self.route_import_ready
    }

    pub fn needs_route_export(self) -> bool {
        !self.route_export_ready
    }

    pub fn needs_route_distribution(self) -> bool {
        !self.route_distribution_ready
    }

    pub fn needs_route_acceptance(self) -> bool {
        !self.route_acceptance_ready
    }

    pub fn needs_route_adoption(self) -> bool {
        !self.route_adoption_ready
    }

    pub fn needs_route_rollout(self) -> bool {
        !self.route_rollout_ready
    }

    pub fn needs_route_activation(self) -> bool {
        !self.route_activation_ready
    }

    pub fn needs_route_approval(self) -> bool {
        !self.route_approval_ready
    }

    pub fn needs_route_certification(self) -> bool {
        !self.route_certification_ready
    }

    pub fn needs_route_validation(self) -> bool {
        !self.route_validation_ready
    }

    pub fn needs_route_verification(self) -> bool {
        !self.route_verification_ready
    }

    pub fn needs_route_publication(self) -> bool {
        !self.route_publication_ready
    }

    pub fn needs_route_completion(self) -> bool {
        !self.route_completion_ready
    }

    pub fn needs_route_signoff(self) -> bool {
        !self.route_signoff_ready
    }

    pub fn needs_route_audit(self) -> bool {
        !self.route_audit_ready
    }

    pub fn needs_route_handoff(self) -> bool {
        !self.route_handoff_ready
    }

    pub fn needs_attach_completion(self) -> bool {
        !self.attach_complete
    }

    pub fn needs_network_data(self) -> bool {
        !self.network_data_ready
    }

    pub fn needs_routing_surface(self) -> bool {
        !self.routing_surface_ready
    }

    pub fn needs_parent_or_route_anchor(self) -> bool {
        !self.parent_or_route_anchor_ready
    }
}

pub fn summarize_thread_attach_route_finalization(
    settlement_summary: ThreadAttachRouteSettlementSummary,
) -> ThreadAttachRouteFinalizationSummary {
    ThreadAttachRouteFinalizationSummary::from_settlement_summary(settlement_summary)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadAttachRouteConfirmationSummary {
    pub finalization_summary: ThreadAttachRouteFinalizationSummary,
    pub required_route_confirmation_check_count: usize,
    pub passed_route_confirmation_check_count: usize,
    pub missing_route_confirmation_check_count: usize,
    pub route_finalization_ready: bool,
    pub route_settlement_ready: bool,
    pub route_reconciliation_ready: bool,
    pub route_replay_ready: bool,
    pub route_recovery_ready: bool,
    pub route_restore_ready: bool,
    pub route_load_ready: bool,
    pub route_ingest_ready: bool,
    pub route_import_ready: bool,
    pub route_export_ready: bool,
    pub route_distribution_ready: bool,
    pub route_acceptance_ready: bool,
    pub route_adoption_ready: bool,
    pub route_rollout_ready: bool,
    pub route_activation_ready: bool,
    pub route_approval_ready: bool,
    pub route_certification_ready: bool,
    pub route_validation_ready: bool,
    pub route_verification_ready: bool,
    pub route_publication_ready: bool,
    pub route_completion_ready: bool,
    pub route_signoff_ready: bool,
    pub route_audit_ready: bool,
    pub route_handoff_ready: bool,
    pub attach_complete: bool,
    pub network_data_ready: bool,
    pub routing_surface_ready: bool,
    pub parent_or_route_anchor_ready: bool,
    pub route_confirmation_ready: bool,
}

impl ThreadAttachRouteConfirmationSummary {
    pub fn from_finalization_summary(
        finalization_summary: ThreadAttachRouteFinalizationSummary,
    ) -> Self {
        let route_finalization_ready = finalization_summary.is_route_finalization_ready();
        let route_settlement_ready = !finalization_summary.needs_route_settlement();
        let route_reconciliation_ready = !finalization_summary.needs_route_reconciliation();
        let route_replay_ready = !finalization_summary.needs_route_replay();
        let route_recovery_ready = !finalization_summary.needs_route_recovery();
        let route_restore_ready = !finalization_summary.needs_route_restore();
        let route_load_ready = !finalization_summary.needs_route_load();
        let route_ingest_ready = !finalization_summary.needs_route_ingest();
        let route_import_ready = !finalization_summary.needs_route_import();
        let route_export_ready = !finalization_summary.needs_route_export();
        let route_distribution_ready = !finalization_summary.needs_route_distribution();
        let route_acceptance_ready = !finalization_summary.needs_route_acceptance();
        let route_adoption_ready = !finalization_summary.needs_route_adoption();
        let route_rollout_ready = !finalization_summary.needs_route_rollout();
        let route_activation_ready = !finalization_summary.needs_route_activation();
        let route_approval_ready = !finalization_summary.needs_route_approval();
        let route_certification_ready = !finalization_summary.needs_route_certification();
        let route_validation_ready = !finalization_summary.needs_route_validation();
        let route_verification_ready = !finalization_summary.needs_route_verification();
        let route_publication_ready = !finalization_summary.needs_route_publication();
        let route_completion_ready = !finalization_summary.needs_route_completion();
        let route_signoff_ready = !finalization_summary.needs_route_signoff();
        let route_audit_ready = !finalization_summary.needs_route_audit();
        let route_handoff_ready = !finalization_summary.needs_route_handoff();
        let attach_complete = !finalization_summary.needs_attach_completion();
        let network_data_ready = !finalization_summary.needs_network_data();
        let routing_surface_ready = !finalization_summary.needs_routing_surface();
        let parent_or_route_anchor_ready = !finalization_summary.needs_parent_or_route_anchor();
        let checks = [
            route_finalization_ready,
            route_settlement_ready,
            route_reconciliation_ready,
            route_replay_ready,
            route_recovery_ready,
            route_restore_ready,
            route_load_ready,
            route_ingest_ready,
            route_import_ready,
            route_export_ready,
            route_distribution_ready,
            route_acceptance_ready,
            route_adoption_ready,
            route_rollout_ready,
            route_activation_ready,
            route_approval_ready,
            route_certification_ready,
            route_validation_ready,
            route_verification_ready,
            route_publication_ready,
            route_completion_ready,
            route_signoff_ready,
            route_audit_ready,
            route_handoff_ready,
            attach_complete,
            network_data_ready,
            routing_surface_ready,
            parent_or_route_anchor_ready,
        ];
        let passed_route_confirmation_check_count = checks.iter().filter(|ready| **ready).count();
        let required_route_confirmation_check_count = checks.len();
        let missing_route_confirmation_check_count =
            required_route_confirmation_check_count - passed_route_confirmation_check_count;
        let route_confirmation_ready = missing_route_confirmation_check_count == 0;

        Self {
            finalization_summary,
            required_route_confirmation_check_count,
            passed_route_confirmation_check_count,
            missing_route_confirmation_check_count,
            route_finalization_ready,
            route_settlement_ready,
            route_reconciliation_ready,
            route_replay_ready,
            route_recovery_ready,
            route_restore_ready,
            route_load_ready,
            route_ingest_ready,
            route_import_ready,
            route_export_ready,
            route_distribution_ready,
            route_acceptance_ready,
            route_adoption_ready,
            route_rollout_ready,
            route_activation_ready,
            route_approval_ready,
            route_certification_ready,
            route_validation_ready,
            route_verification_ready,
            route_publication_ready,
            route_completion_ready,
            route_signoff_ready,
            route_audit_ready,
            route_handoff_ready,
            attach_complete,
            network_data_ready,
            routing_surface_ready,
            parent_or_route_anchor_ready,
            route_confirmation_ready,
        }
    }

    pub fn is_route_confirmation_ready(self) -> bool {
        self.route_confirmation_ready
    }

    pub fn has_confirmation_gaps(self) -> bool {
        self.missing_route_confirmation_check_count > 0
    }

    pub fn needs_route_finalization(self) -> bool {
        !self.route_finalization_ready
    }

    pub fn needs_route_settlement(self) -> bool {
        !self.route_settlement_ready
    }

    pub fn needs_route_reconciliation(self) -> bool {
        !self.route_reconciliation_ready
    }

    pub fn needs_route_replay(self) -> bool {
        !self.route_replay_ready
    }

    pub fn needs_route_recovery(self) -> bool {
        !self.route_recovery_ready
    }

    pub fn needs_route_restore(self) -> bool {
        !self.route_restore_ready
    }

    pub fn needs_route_load(self) -> bool {
        !self.route_load_ready
    }

    pub fn needs_route_ingest(self) -> bool {
        !self.route_ingest_ready
    }

    pub fn needs_route_import(self) -> bool {
        !self.route_import_ready
    }

    pub fn needs_route_export(self) -> bool {
        !self.route_export_ready
    }

    pub fn needs_route_distribution(self) -> bool {
        !self.route_distribution_ready
    }

    pub fn needs_route_acceptance(self) -> bool {
        !self.route_acceptance_ready
    }

    pub fn needs_route_adoption(self) -> bool {
        !self.route_adoption_ready
    }

    pub fn needs_route_rollout(self) -> bool {
        !self.route_rollout_ready
    }

    pub fn needs_route_activation(self) -> bool {
        !self.route_activation_ready
    }

    pub fn needs_route_approval(self) -> bool {
        !self.route_approval_ready
    }

    pub fn needs_route_certification(self) -> bool {
        !self.route_certification_ready
    }

    pub fn needs_route_validation(self) -> bool {
        !self.route_validation_ready
    }

    pub fn needs_route_verification(self) -> bool {
        !self.route_verification_ready
    }

    pub fn needs_route_publication(self) -> bool {
        !self.route_publication_ready
    }

    pub fn needs_route_completion(self) -> bool {
        !self.route_completion_ready
    }

    pub fn needs_route_signoff(self) -> bool {
        !self.route_signoff_ready
    }

    pub fn needs_route_audit(self) -> bool {
        !self.route_audit_ready
    }

    pub fn needs_route_handoff(self) -> bool {
        !self.route_handoff_ready
    }

    pub fn needs_attach_completion(self) -> bool {
        !self.attach_complete
    }

    pub fn needs_network_data(self) -> bool {
        !self.network_data_ready
    }

    pub fn needs_routing_surface(self) -> bool {
        !self.routing_surface_ready
    }

    pub fn needs_parent_or_route_anchor(self) -> bool {
        !self.parent_or_route_anchor_ready
    }
}

pub fn summarize_thread_attach_route_confirmation(
    finalization_summary: ThreadAttachRouteFinalizationSummary,
) -> ThreadAttachRouteConfirmationSummary {
    ThreadAttachRouteConfirmationSummary::from_finalization_summary(finalization_summary)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadAttachRouteAttestationSummary {
    pub confirmation_summary: ThreadAttachRouteConfirmationSummary,
    pub required_route_attestation_check_count: usize,
    pub passed_route_attestation_check_count: usize,
    pub missing_route_attestation_check_count: usize,
    pub route_confirmation_ready: bool,
    pub route_finalization_ready: bool,
    pub route_settlement_ready: bool,
    pub route_reconciliation_ready: bool,
    pub route_replay_ready: bool,
    pub route_recovery_ready: bool,
    pub route_restore_ready: bool,
    pub route_load_ready: bool,
    pub route_ingest_ready: bool,
    pub route_import_ready: bool,
    pub route_export_ready: bool,
    pub route_distribution_ready: bool,
    pub route_acceptance_ready: bool,
    pub route_adoption_ready: bool,
    pub route_rollout_ready: bool,
    pub route_activation_ready: bool,
    pub route_approval_ready: bool,
    pub route_certification_ready: bool,
    pub route_validation_ready: bool,
    pub route_verification_ready: bool,
    pub route_publication_ready: bool,
    pub route_completion_ready: bool,
    pub route_signoff_ready: bool,
    pub route_audit_ready: bool,
    pub route_handoff_ready: bool,
    pub attach_complete: bool,
    pub network_data_ready: bool,
    pub routing_surface_ready: bool,
    pub parent_or_route_anchor_ready: bool,
    pub route_attestation_ready: bool,
}

impl ThreadAttachRouteAttestationSummary {
    pub fn from_confirmation_summary(
        confirmation_summary: ThreadAttachRouteConfirmationSummary,
    ) -> Self {
        let route_confirmation_ready = confirmation_summary.is_route_confirmation_ready();
        let route_finalization_ready = !confirmation_summary.needs_route_finalization();
        let route_settlement_ready = !confirmation_summary.needs_route_settlement();
        let route_reconciliation_ready = !confirmation_summary.needs_route_reconciliation();
        let route_replay_ready = !confirmation_summary.needs_route_replay();
        let route_recovery_ready = !confirmation_summary.needs_route_recovery();
        let route_restore_ready = !confirmation_summary.needs_route_restore();
        let route_load_ready = !confirmation_summary.needs_route_load();
        let route_ingest_ready = !confirmation_summary.needs_route_ingest();
        let route_import_ready = !confirmation_summary.needs_route_import();
        let route_export_ready = !confirmation_summary.needs_route_export();
        let route_distribution_ready = !confirmation_summary.needs_route_distribution();
        let route_acceptance_ready = !confirmation_summary.needs_route_acceptance();
        let route_adoption_ready = !confirmation_summary.needs_route_adoption();
        let route_rollout_ready = !confirmation_summary.needs_route_rollout();
        let route_activation_ready = !confirmation_summary.needs_route_activation();
        let route_approval_ready = !confirmation_summary.needs_route_approval();
        let route_certification_ready = !confirmation_summary.needs_route_certification();
        let route_validation_ready = !confirmation_summary.needs_route_validation();
        let route_verification_ready = !confirmation_summary.needs_route_verification();
        let route_publication_ready = !confirmation_summary.needs_route_publication();
        let route_completion_ready = !confirmation_summary.needs_route_completion();
        let route_signoff_ready = !confirmation_summary.needs_route_signoff();
        let route_audit_ready = !confirmation_summary.needs_route_audit();
        let route_handoff_ready = !confirmation_summary.needs_route_handoff();
        let attach_complete = !confirmation_summary.needs_attach_completion();
        let network_data_ready = !confirmation_summary.needs_network_data();
        let routing_surface_ready = !confirmation_summary.needs_routing_surface();
        let parent_or_route_anchor_ready = !confirmation_summary.needs_parent_or_route_anchor();
        let checks = [
            route_confirmation_ready,
            route_finalization_ready,
            route_settlement_ready,
            route_reconciliation_ready,
            route_replay_ready,
            route_recovery_ready,
            route_restore_ready,
            route_load_ready,
            route_ingest_ready,
            route_import_ready,
            route_export_ready,
            route_distribution_ready,
            route_acceptance_ready,
            route_adoption_ready,
            route_rollout_ready,
            route_activation_ready,
            route_approval_ready,
            route_certification_ready,
            route_validation_ready,
            route_verification_ready,
            route_publication_ready,
            route_completion_ready,
            route_signoff_ready,
            route_audit_ready,
            route_handoff_ready,
            attach_complete,
            network_data_ready,
            routing_surface_ready,
            parent_or_route_anchor_ready,
        ];
        let passed_route_attestation_check_count = checks.iter().filter(|ready| **ready).count();
        let required_route_attestation_check_count = checks.len();
        let missing_route_attestation_check_count =
            required_route_attestation_check_count - passed_route_attestation_check_count;
        let route_attestation_ready = missing_route_attestation_check_count == 0;

        Self {
            confirmation_summary,
            required_route_attestation_check_count,
            passed_route_attestation_check_count,
            missing_route_attestation_check_count,
            route_confirmation_ready,
            route_finalization_ready,
            route_settlement_ready,
            route_reconciliation_ready,
            route_replay_ready,
            route_recovery_ready,
            route_restore_ready,
            route_load_ready,
            route_ingest_ready,
            route_import_ready,
            route_export_ready,
            route_distribution_ready,
            route_acceptance_ready,
            route_adoption_ready,
            route_rollout_ready,
            route_activation_ready,
            route_approval_ready,
            route_certification_ready,
            route_validation_ready,
            route_verification_ready,
            route_publication_ready,
            route_completion_ready,
            route_signoff_ready,
            route_audit_ready,
            route_handoff_ready,
            attach_complete,
            network_data_ready,
            routing_surface_ready,
            parent_or_route_anchor_ready,
            route_attestation_ready,
        }
    }

    pub fn is_route_attestation_ready(self) -> bool {
        self.route_attestation_ready
    }

    pub fn has_attestation_gaps(self) -> bool {
        self.missing_route_attestation_check_count > 0
    }

    pub fn needs_route_confirmation(self) -> bool {
        !self.route_confirmation_ready
    }

    pub fn needs_route_finalization(self) -> bool {
        !self.route_finalization_ready
    }

    pub fn needs_route_settlement(self) -> bool {
        !self.route_settlement_ready
    }

    pub fn needs_route_reconciliation(self) -> bool {
        !self.route_reconciliation_ready
    }

    pub fn needs_route_replay(self) -> bool {
        !self.route_replay_ready
    }

    pub fn needs_route_recovery(self) -> bool {
        !self.route_recovery_ready
    }

    pub fn needs_route_restore(self) -> bool {
        !self.route_restore_ready
    }

    pub fn needs_route_load(self) -> bool {
        !self.route_load_ready
    }

    pub fn needs_route_ingest(self) -> bool {
        !self.route_ingest_ready
    }

    pub fn needs_route_import(self) -> bool {
        !self.route_import_ready
    }

    pub fn needs_route_export(self) -> bool {
        !self.route_export_ready
    }

    pub fn needs_route_distribution(self) -> bool {
        !self.route_distribution_ready
    }

    pub fn needs_route_acceptance(self) -> bool {
        !self.route_acceptance_ready
    }

    pub fn needs_route_adoption(self) -> bool {
        !self.route_adoption_ready
    }

    pub fn needs_route_rollout(self) -> bool {
        !self.route_rollout_ready
    }

    pub fn needs_route_activation(self) -> bool {
        !self.route_activation_ready
    }

    pub fn needs_route_approval(self) -> bool {
        !self.route_approval_ready
    }

    pub fn needs_route_certification(self) -> bool {
        !self.route_certification_ready
    }

    pub fn needs_route_validation(self) -> bool {
        !self.route_validation_ready
    }

    pub fn needs_route_verification(self) -> bool {
        !self.route_verification_ready
    }

    pub fn needs_route_publication(self) -> bool {
        !self.route_publication_ready
    }

    pub fn needs_route_completion(self) -> bool {
        !self.route_completion_ready
    }

    pub fn needs_route_signoff(self) -> bool {
        !self.route_signoff_ready
    }

    pub fn needs_route_audit(self) -> bool {
        !self.route_audit_ready
    }

    pub fn needs_route_handoff(self) -> bool {
        !self.route_handoff_ready
    }

    pub fn needs_attach_completion(self) -> bool {
        !self.attach_complete
    }

    pub fn needs_network_data(self) -> bool {
        !self.network_data_ready
    }

    pub fn needs_routing_surface(self) -> bool {
        !self.routing_surface_ready
    }

    pub fn needs_parent_or_route_anchor(self) -> bool {
        !self.parent_or_route_anchor_ready
    }
}

pub fn summarize_thread_attach_route_attestation(
    confirmation_summary: ThreadAttachRouteConfirmationSummary,
) -> ThreadAttachRouteAttestationSummary {
    ThreadAttachRouteAttestationSummary::from_confirmation_summary(confirmation_summary)
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
        } else if !self.local_role.is_attached()
            || (self.local_role == DeviceRole::Child && self.parent.is_none())
        {
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

pub fn status_from_message(message: &MleMessage) -> Result<Option<MleStatus>, MleError> {
    message
        .find_tlv(TlvType::Status)
        .map(|tlv| MleStatus::parse(&tlv.value))
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

        let summary = message.summary();
        assert_eq!(
            summary,
            MleMessageSummary {
                command: MleCommand::ParentRequest,
                tlv_count: 2,
                has_scan_mask: true,
                has_mode: false,
                has_timeout: false,
                has_leader_data: false,
                has_network_data: false,
                has_connectivity: false,
                has_status: false,
                has_version: true,
            }
        );
        assert!(!summary.is_empty());
        assert!(summary.has_parent_selection_request_context());
        assert!(!summary.has_attach_response_context());
        assert!(!summary.has_diagnostic_context());
        assert!(!summary.has_thread_data_versions());
    }

    #[test]
    fn mle_message_summary_reports_attach_and_diagnostic_context() {
        let attach = MleMessage {
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
                Tlv::new(TlvType::Timeout, 30_u32.to_be_bytes().to_vec()).unwrap(),
            ],
        };

        let attach_summary = attach.summary();
        assert!(attach_summary.has_attach_response_context());
        assert!(!attach_summary.has_parent_selection_request_context());
        assert!(!attach_summary.has_diagnostic_context());
        assert!(!attach_summary.has_thread_data_versions());

        let diagnostic = MleMessage {
            command: MleCommand::Advertisement,
            tlvs: vec![
                LeaderData {
                    partition_id: 0x0102_0304,
                    weighting: 64,
                    data_version: 2,
                    stable_data_version: 1,
                    leader_router_id: 7,
                }
                .to_tlv(),
                ThreadNetworkData::new(vec![NetworkDataTlvType::Prefix.as_byte(), 0])
                    .unwrap()
                    .to_tlv(),
                Tlv::new(TlvType::Connectivity, vec![1, 3, 2, 1, 5, 8, 12]).unwrap(),
            ],
        };

        let diagnostic_summary = diagnostic.summary();
        assert_eq!(diagnostic_summary.command, MleCommand::Advertisement);
        assert_eq!(diagnostic_summary.tlv_count, 3);
        assert!(diagnostic_summary.has_diagnostic_context());
        assert!(diagnostic_summary.has_thread_data_versions());
        assert!(!diagnostic_summary.has_parent_selection_request_context());
        assert!(!diagnostic_summary.has_attach_response_context());

        let empty = MleMessage {
            command: MleCommand::LinkRequest,
            tlvs: Vec::new(),
        }
        .summary();
        assert!(empty.is_empty());
    }

    #[test]
    fn mle_message_batch_summary_rolls_up_attach_and_diagnostic_context() {
        let parent_request = MleMessage {
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
        let attach_response = MleMessage {
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
        };
        let diagnostic = MleMessage {
            command: MleCommand::Advertisement,
            tlvs: vec![
                LeaderData {
                    partition_id: 0x0102_0304,
                    weighting: 64,
                    data_version: 2,
                    stable_data_version: 1,
                    leader_router_id: 7,
                }
                .to_tlv(),
                ThreadNetworkData::new(vec![NetworkDataTlvType::Prefix.as_byte(), 0])
                    .unwrap()
                    .to_tlv(),
                Tlv::new(TlvType::Connectivity, vec![1, 3, 2, 1, 5, 8, 12]).unwrap(),
            ],
        };
        let status = MleMessage {
            command: MleCommand::ChildUpdateResponse,
            tlvs: vec![MleStatus { code: 1 }.to_tlv()],
        };
        let unknown = MleMessage {
            command: MleCommand::Unknown(0xfe),
            tlvs: Vec::new(),
        };

        let summary = MleMessageBatchSummary::from_messages([
            &parent_request,
            &attach_response,
            &diagnostic,
            &status,
            &unknown,
        ]);

        assert_eq!(summary.total_messages, 5);
        assert_eq!(summary.total_tlvs, 7);
        assert_eq!(summary.empty_messages, 1);
        assert_eq!(summary.parent_selection_request_messages, 1);
        assert_eq!(summary.attach_response_messages, 1);
        assert_eq!(summary.diagnostic_messages, 1);
        assert_eq!(summary.thread_data_version_messages, 1);
        assert_eq!(summary.status_messages, 1);
        assert_eq!(summary.network_data_messages, 1);
        assert_eq!(summary.connectivity_messages, 1);
        assert_eq!(summary.unknown_command_messages, 1);
        assert!(summary.has_parent_selection_requests());
        assert!(summary.has_attach_responses());
        assert!(summary.has_diagnostics());
        assert!(summary.has_statuses());
        assert!(summary.has_unknown_commands());
    }

    #[test]
    fn mle_message_batch_summary_handles_precomputed_and_empty_summaries() {
        let empty = MleMessageBatchSummary::empty();
        assert!(empty.is_empty());
        assert!(!empty.has_diagnostics());

        let summaries = [
            MleMessageSummary {
                command: MleCommand::DataResponse,
                tlv_count: 1,
                has_scan_mask: false,
                has_mode: false,
                has_timeout: false,
                has_leader_data: false,
                has_network_data: true,
                has_connectivity: false,
                has_status: false,
                has_version: false,
            },
            MleMessageSummary {
                command: MleCommand::LinkReject,
                tlv_count: 1,
                has_scan_mask: false,
                has_mode: false,
                has_timeout: false,
                has_leader_data: false,
                has_network_data: false,
                has_connectivity: false,
                has_status: true,
                has_version: false,
            },
        ];
        let summary = MleMessageBatchSummary::from_summaries(&summaries);

        assert_eq!(summary.total_messages, 2);
        assert_eq!(summary.total_tlvs, 2);
        assert_eq!(summary.network_data_messages, 1);
        assert_eq!(summary.status_messages, 1);
        assert!(!summary.is_empty());
        assert!(summary.has_statuses());
        assert!(!summary.has_parent_selection_requests());
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
    fn status_tlv_round_trips_and_extracts_from_message() {
        let status = MleStatus { code: 0x01 };
        let message = MleMessage {
            command: MleCommand::ChildIdResponse,
            tlvs: vec![status.to_tlv()],
        };

        assert_eq!(MleStatus::parse(&status.encode()).unwrap(), status);
        assert_eq!(status.to_tlv().tlv_type, TlvType::Status);
        assert_eq!(status_from_message(&message).unwrap(), Some(status));
        assert_eq!(
            MleMessage::parse(&message.encode().unwrap()).unwrap(),
            message
        );
    }

    #[test]
    fn status_tlv_rejects_wrong_length() {
        assert_eq!(
            MleStatus::parse(&[]),
            Err(MleError::InvalidTlvLength {
                tlv_type: TlvType::Status,
                expected: MleStatus::ENCODED_LEN,
                actual: 0,
            })
        );
        assert_eq!(
            MleStatus::parse(&[0x01, 0x02]),
            Err(MleError::InvalidTlvLength {
                tlv_type: TlvType::Status,
                expected: MleStatus::ENCODED_LEN,
                actual: 2,
            })
        );
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
    fn network_data_summary_counts_prefix_stability_and_nested_tlvs() {
        let border_router =
            NetworkDataTlv::new(NetworkDataTlvType::BorderRouter, true, vec![0xaa]).unwrap();
        let lowpan_id =
            NetworkDataTlv::new(NetworkDataTlvType::LowpanId, false, vec![0x0f]).unwrap();
        let prefix = ThreadPrefixData::new(
            true,
            3,
            64,
            vec![0xfd, 0x00, 0xab, 0xcd, 0, 0, 0, 0],
            vec![border_router, lowpan_id],
        )
        .unwrap();
        let service = NetworkDataTlv::new(NetworkDataTlvType::Service, false, vec![1]).unwrap();
        let server = NetworkDataTlv::new(NetworkDataTlvType::Server, true, vec![2]).unwrap();
        let unknown = NetworkDataTlv::new(NetworkDataTlvType::Unknown(42), false, vec![3]).unwrap();
        let network_data =
            ThreadNetworkData::from_tlvs(vec![prefix.to_tlv().unwrap(), service, server, unknown])
                .unwrap();

        let summary = network_data.summary().unwrap();

        assert_eq!(
            summary,
            ThreadNetworkDataSummary {
                byte_len: network_data.len(),
                top_level_tlvs: 4,
                stable_top_level_tlvs: 2,
                prefix_tlvs: 1,
                stable_prefix_tlvs: 1,
                prefix_sub_tlvs: 2,
                stable_prefix_sub_tlvs: 1,
                has_route_tlvs: 0,
                border_router_tlvs: 1,
                lowpan_id_tlvs: 1,
                commissioning_data_tlvs: 0,
                service_tlvs: 1,
                server_tlvs: 1,
                context_tlvs: 0,
                unknown_tlvs: 1,
            }
        );
        assert!(!summary.is_empty());
        assert!(summary.has_prefixes());
        assert!(summary.has_stable_data());
        assert!(summary.has_routing_data());
        assert!(summary.has_services());
        assert!(summary.has_service_or_context_data());
        assert!(summary.has_unknown_tlvs());

        let advertisement = NetworkDataAdvertisement {
            leader_data: None,
            network_data: Some(network_data),
        };
        assert_eq!(advertisement.network_data_summary().unwrap(), summary);
        assert!(NetworkDataAdvertisement {
            leader_data: None,
            network_data: None,
        }
        .network_data_summary()
        .unwrap()
        .is_empty());
    }

    #[test]
    fn network_data_readiness_summary_marks_prefix_route_surface_ready() {
        let border_router =
            NetworkDataTlv::new(NetworkDataTlvType::BorderRouter, true, vec![0xaa]).unwrap();
        let context = NetworkDataTlv::new(NetworkDataTlvType::Context, true, vec![0x01]).unwrap();
        let prefix = ThreadPrefixData::new(
            true,
            3,
            64,
            vec![0xfd, 0x00, 0xab, 0xcd, 0, 0, 0, 0],
            vec![border_router, context],
        )
        .unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![prefix.to_tlv().unwrap()]).unwrap();

        let readiness = summarize_thread_network_data_readiness(&network_data).unwrap();

        assert_eq!(
            readiness.network_data_summary,
            network_data.summary().unwrap()
        );
        assert_eq!(readiness.required_check_count, 6);
        assert_eq!(readiness.passed_check_count, 6);
        assert_eq!(readiness.missing_check_count, 0);
        assert!(readiness.network_data_present);
        assert!(readiness.prefix_coverage_ready);
        assert!(readiness.routing_coverage_ready);
        assert!(readiness.stable_data_ready);
        assert!(readiness.service_or_context_ready);
        assert!(readiness.unknown_tlvs_absent);
        assert!(readiness.network_data_ready);
        assert!(readiness.is_network_data_ready());
        assert!(!readiness.has_missing_checks());
        assert!(!readiness.needs_network_data());
        assert!(!readiness.needs_prefix_coverage());
        assert!(!readiness.needs_routing_coverage());
        assert!(!readiness.needs_stable_data());
        assert!(!readiness.needs_service_or_context_data());
        assert!(!readiness.has_unknown_tlv_gaps());
    }

    #[test]
    fn network_data_readiness_summary_flags_missing_and_unknown_tlvs() {
        let unknown = NetworkDataTlv::new(NetworkDataTlvType::Unknown(42), false, vec![3]).unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![unknown]).unwrap();

        let readiness = network_data.summary().unwrap().readiness();

        assert_eq!(readiness.required_check_count, 6);
        assert_eq!(readiness.passed_check_count, 1);
        assert_eq!(readiness.missing_check_count, 5);
        assert!(readiness.network_data_present);
        assert!(!readiness.prefix_coverage_ready);
        assert!(!readiness.routing_coverage_ready);
        assert!(!readiness.stable_data_ready);
        assert!(!readiness.service_or_context_ready);
        assert!(!readiness.unknown_tlvs_absent);
        assert!(!readiness.network_data_ready);
        assert!(!readiness.is_network_data_ready());
        assert!(readiness.has_missing_checks());
        assert!(!readiness.needs_network_data());
        assert!(readiness.needs_prefix_coverage());
        assert!(readiness.needs_routing_coverage());
        assert!(readiness.needs_stable_data());
        assert!(readiness.needs_service_or_context_data());
        assert!(readiness.has_unknown_tlv_gaps());

        let empty = ThreadNetworkDataSummary::empty().readiness();
        assert!(empty.needs_network_data());
        assert_eq!(empty.passed_check_count, 1);
        assert_eq!(empty.missing_check_count, 5);
    }

    #[test]
    fn network_data_tlv_handoff_summary_marks_ready_dataset_tlvs() {
        let border_router =
            NetworkDataTlv::new(NetworkDataTlvType::BorderRouter, true, vec![0xaa]).unwrap();
        let context = NetworkDataTlv::new(NetworkDataTlvType::Context, true, vec![0x01]).unwrap();
        let prefix = ThreadPrefixData::new(
            true,
            3,
            64,
            vec![0xfd, 0x00, 0xab, 0xcd, 0, 0, 0, 0],
            vec![border_router, context],
        )
        .unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![prefix.to_tlv().unwrap()]).unwrap();
        let readiness = summarize_thread_network_data_readiness(&network_data).unwrap();

        let summary = summarize_thread_network_data_tlv_handoff(&network_data).unwrap();

        assert_eq!(summary.network_data_readiness, readiness);
        assert_eq!(summary.required_handoff_check_count, 5);
        assert_eq!(summary.passed_handoff_check_count, 5);
        assert_eq!(summary.missing_handoff_check_count, 0);
        assert!(summary.network_data_ready);
        assert!(summary.stable_tlvs_ready);
        assert!(summary.routing_tlvs_ready);
        assert!(summary.service_or_context_tlvs_ready);
        assert!(summary.unknown_tlvs_absent);
        assert!(summary.tlv_handoff_ready);
        assert!(summary.is_tlv_handoff_ready());
        assert!(!summary.has_handoff_gaps());
        assert!(!summary.needs_network_data());
        assert!(!summary.needs_stable_tlvs());
        assert!(!summary.needs_routing_tlvs());
        assert!(!summary.needs_service_or_context_tlvs());
        assert!(!summary.needs_unknown_tlv_review());
    }

    #[test]
    fn network_data_tlv_handoff_summary_routes_blocked_tlv_work() {
        let unknown = NetworkDataTlv::new(NetworkDataTlvType::Unknown(42), false, vec![3]).unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![unknown]).unwrap();
        let readiness = network_data.summary().unwrap().readiness();

        let summary = ThreadNetworkDataTlvHandoffSummary::from_readiness(readiness);

        assert_eq!(summary.required_handoff_check_count, 5);
        assert_eq!(summary.passed_handoff_check_count, 0);
        assert_eq!(summary.missing_handoff_check_count, 5);
        assert!(!summary.network_data_ready);
        assert!(!summary.stable_tlvs_ready);
        assert!(!summary.routing_tlvs_ready);
        assert!(!summary.service_or_context_tlvs_ready);
        assert!(!summary.unknown_tlvs_absent);
        assert!(!summary.tlv_handoff_ready);
        assert!(!summary.is_tlv_handoff_ready());
        assert!(summary.has_handoff_gaps());
        assert!(summary.needs_network_data());
        assert!(summary.needs_stable_tlvs());
        assert!(summary.needs_routing_tlvs());
        assert!(summary.needs_service_or_context_tlvs());
        assert!(summary.needs_unknown_tlv_review());
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
    fn attach_readiness_summary_combines_mle_and_neighbor_context() {
        let parent_request = MleMessage {
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
        let parent_response = MleMessage {
            command: MleCommand::ParentResponse,
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
        };
        let diagnostic = MleMessage {
            command: MleCommand::Advertisement,
            tlvs: vec![Tlv::new(TlvType::Connectivity, vec![1, 3, 2, 1, 5, 8, 12]).unwrap()],
        };
        let mut table = NeighborTable::new(DeviceRole::Detached);
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

        let message_summary =
            MleMessageBatchSummary::from_messages([&parent_request, &parent_response, &diagnostic]);
        let neighbor_summary = table.summary_at(1_250);
        let readiness = summarize_thread_attach_readiness(message_summary, neighbor_summary);

        assert_eq!(readiness.message_summary, message_summary);
        assert_eq!(readiness.neighbor_summary, neighbor_summary);
        assert!(!readiness.attached);
        assert!(readiness.attach_ready);
        assert!(readiness.parent_selection_requested);
        assert!(readiness.attach_response_seen);
        assert!(readiness.parent_candidate_available);
        assert!(!readiness.needs_parent_selection);
        assert!(!readiness.waiting_for_attach_response);
        assert!(!readiness.requires_neighbor_refresh);
        assert!(readiness.has_diagnostics());
        assert!(!readiness.has_statuses());
        assert!(!readiness.has_unknown_commands());
    }

    #[test]
    fn attach_readiness_summary_reports_blocked_and_waiting_attach_states() {
        let empty_messages = MleMessageBatchSummary::empty();
        let child_without_parent = NeighborTable::new(DeviceRole::Child).summary_at(10_000);
        let needs_selection =
            summarize_thread_attach_readiness(empty_messages, child_without_parent);

        assert!(!needs_selection.attached);
        assert!(!needs_selection.attach_ready);
        assert!(needs_selection.needs_parent_selection);
        assert!(!needs_selection.waiting_for_attach_response);
        assert!(!needs_selection.parent_candidate_available);

        let parent_request = MleMessage {
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
        let waiting_messages = MleMessageBatchSummary::from_messages([&parent_request]);
        let waiting = summarize_thread_attach_readiness(waiting_messages, child_without_parent);

        assert!(!waiting.attach_ready);
        assert!(!waiting.needs_parent_selection);
        assert!(waiting.waiting_for_attach_response);

        let mut stale_parent = NeighborTable::new(DeviceRole::Child);
        stale_parent.upsert(ThreadNeighbor::new(
            ThreadNeighborId(0x1000),
            DeviceRole::Router,
            NeighborRelationship::Parent,
            1_000,
            500,
        ));
        let stale_parent_summary = stale_parent.summary_at(1_500);
        let attached_with_stale_parent =
            summarize_thread_attach_readiness(empty_messages, stale_parent_summary);

        assert!(attached_with_stale_parent.attached);
        assert!(attached_with_stale_parent.attach_ready);
        assert!(attached_with_stale_parent.requires_neighbor_refresh);
    }

    #[test]
    fn attach_action_summary_marks_clear_ready_attach() {
        let parent_request = MleMessage {
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
        let parent_response = MleMessage {
            command: MleCommand::ParentResponse,
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
        };
        let mut table = NeighborTable::new(DeviceRole::Detached);
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
        let message_summary =
            MleMessageBatchSummary::from_messages([&parent_request, &parent_response]);
        let neighbor_summary = table.summary_at(1_250);
        let readiness = summarize_thread_attach_readiness(message_summary, neighbor_summary);

        let summary = summarize_thread_attach_actions(readiness);

        assert_eq!(summary.readiness_summary, readiness);
        assert_eq!(summary.required_action_count, 5);
        assert_eq!(summary.pending_action_count, 0);
        assert_eq!(summary.clear_action_count, 5);
        assert!(!summary.start_parent_selection);
        assert!(!summary.wait_for_attach_response);
        assert!(!summary.refresh_neighbors);
        assert!(!summary.inspect_statuses);
        assert!(!summary.inspect_unknown_commands);
        assert!(summary.attach_action_clear);
        assert!(!summary.has_pending_actions());
        assert!(summary.is_attach_action_clear());
        assert!(!summary.needs_parent_selection());
        assert!(!summary.waiting_on_attach_response());
        assert!(!summary.needs_neighbor_refresh());
        assert!(!summary.needs_status_review());
        assert!(!summary.needs_unknown_command_review());
    }

    #[test]
    fn attach_action_summary_counts_pending_attach_work() {
        let status = MleMessage {
            command: MleCommand::ChildUpdateResponse,
            tlvs: vec![Tlv::new(TlvType::Status, vec![0x01]).unwrap()],
        };
        let unknown = MleMessage {
            command: MleCommand::Unknown(0xfe),
            tlvs: Vec::new(),
        };
        let message_summary = MleMessageBatchSummary::from_messages([&status, &unknown]);
        let mut table = NeighborTable::new(DeviceRole::Child);
        table.upsert(ThreadNeighbor::new(
            ThreadNeighborId(0x1000),
            DeviceRole::Router,
            NeighborRelationship::RouterPeer,
            1_000,
            500,
        ));
        let neighbor_summary = table.summary_at(1_500);

        let summary = ThreadAttachActionSummary::from_summaries(message_summary, neighbor_summary);

        assert_eq!(summary.required_action_count, 5);
        assert_eq!(summary.pending_action_count, 4);
        assert_eq!(summary.clear_action_count, 1);
        assert!(summary.start_parent_selection);
        assert!(!summary.wait_for_attach_response);
        assert!(summary.refresh_neighbors);
        assert!(summary.inspect_statuses);
        assert!(summary.inspect_unknown_commands);
        assert!(!summary.attach_action_clear);
        assert!(summary.has_pending_actions());
        assert!(!summary.is_attach_action_clear());
        assert!(summary.needs_parent_selection());
        assert!(!summary.waiting_on_attach_response());
        assert!(summary.needs_neighbor_refresh());
        assert!(summary.needs_status_review());
        assert!(summary.needs_unknown_command_review());
    }

    #[test]
    fn attach_completion_summary_marks_clear_attached_child() {
        let mut table = NeighborTable::new(DeviceRole::Child);
        table.upsert(ThreadNeighbor::new(
            ThreadNeighborId(0x1000),
            DeviceRole::Router,
            NeighborRelationship::Parent,
            1_200,
            10_000,
        ));
        let message_summary = MleMessageBatchSummary::empty();
        let neighbor_summary = table.summary_at(1_250);
        let action_summary =
            ThreadAttachActionSummary::from_summaries(message_summary, neighbor_summary);
        let supervision_plan = table
            .diagnostic_snapshot(None, 1_250)
            .unwrap()
            .supervision_plan();

        let summary = summarize_thread_attach_completion(action_summary, supervision_plan);

        assert_eq!(summary.action_summary, action_summary);
        assert_eq!(summary.supervision_plan, supervision_plan);
        assert_eq!(summary.required_completion_check_count, 4);
        assert_eq!(summary.passed_completion_check_count, 4);
        assert_eq!(summary.missing_completion_check_count, 0);
        assert!(summary.actions_clear);
        assert!(summary.supervision_clear);
        assert!(summary.attached_or_attach_ready);
        assert!(summary.review_queues_clear);
        assert!(summary.attach_complete);
        assert!(summary.is_attach_complete());
        assert!(!summary.has_completion_gaps());
        assert!(!summary.needs_action_clearance());
        assert!(!summary.needs_supervision_clearance());
        assert!(!summary.needs_attach_readiness());
        assert!(!summary.needs_review_queue_clearance());
    }

    #[test]
    fn attach_completion_summary_routes_blocked_attach_work() {
        let status = MleMessage {
            command: MleCommand::ChildUpdateResponse,
            tlvs: vec![Tlv::new(TlvType::Status, vec![0x01]).unwrap()],
        };
        let unknown = MleMessage {
            command: MleCommand::Unknown(0xfe),
            tlvs: Vec::new(),
        };
        let mut table = NeighborTable::new(DeviceRole::Child);
        table.upsert(ThreadNeighbor::new(
            ThreadNeighborId(0x1000),
            DeviceRole::Router,
            NeighborRelationship::RouterPeer,
            1_000,
            500,
        ));
        let message_summary = MleMessageBatchSummary::from_messages([&status, &unknown]);
        let neighbor_summary = table.summary_at(1_500);
        let action_summary =
            ThreadAttachActionSummary::from_summaries(message_summary, neighbor_summary);
        let supervision_plan = table
            .diagnostic_snapshot(None, 1_500)
            .unwrap()
            .supervision_plan();

        let summary = ThreadAttachCompletionSummary::from_action_and_supervision(
            action_summary,
            supervision_plan,
        );

        assert_eq!(summary.required_completion_check_count, 4);
        assert_eq!(summary.passed_completion_check_count, 0);
        assert_eq!(summary.missing_completion_check_count, 4);
        assert!(!summary.actions_clear);
        assert!(!summary.supervision_clear);
        assert!(!summary.attached_or_attach_ready);
        assert!(!summary.review_queues_clear);
        assert!(!summary.attach_complete);
        assert!(!summary.is_attach_complete());
        assert!(summary.has_completion_gaps());
        assert!(summary.needs_action_clearance());
        assert!(summary.needs_supervision_clearance());
        assert!(summary.needs_attach_readiness());
        assert!(summary.needs_review_queue_clearance());
    }

    #[test]
    fn attach_route_handoff_summary_marks_ready_child_route_surface() {
        let mut table = NeighborTable::new(DeviceRole::Child);
        table.upsert(ThreadNeighbor::new(
            ThreadNeighborId(0x1000),
            DeviceRole::Router,
            NeighborRelationship::Parent,
            1_200,
            10_000,
        ));
        let action_summary = ThreadAttachActionSummary::from_summaries(
            MleMessageBatchSummary::empty(),
            table.summary_at(1_250),
        );
        let completion_summary = summarize_thread_attach_completion(
            action_summary,
            table
                .diagnostic_snapshot(None, 1_250)
                .unwrap()
                .supervision_plan(),
        );
        let border_router =
            NetworkDataTlv::new(NetworkDataTlvType::BorderRouter, true, vec![0xaa]).unwrap();
        let context = NetworkDataTlv::new(NetworkDataTlvType::Context, true, vec![0x01]).unwrap();
        let prefix = ThreadPrefixData::new(
            true,
            3,
            64,
            vec![0xfd, 0x00, 0xab, 0xcd, 0, 0, 0, 0],
            vec![border_router, context],
        )
        .unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![prefix.to_tlv().unwrap()]).unwrap();
        let network_data_readiness =
            summarize_thread_network_data_readiness(&network_data).unwrap();

        let summary =
            summarize_thread_attach_route_handoff(completion_summary, network_data_readiness);

        assert_eq!(summary.completion_summary, completion_summary);
        assert_eq!(summary.network_data_readiness, network_data_readiness);
        assert_eq!(summary.required_handoff_check_count, 4);
        assert_eq!(summary.passed_handoff_check_count, 4);
        assert_eq!(summary.missing_handoff_check_count, 0);
        assert!(summary.attach_complete);
        assert!(summary.network_data_ready);
        assert!(summary.routing_surface_ready);
        assert!(summary.parent_or_route_anchor_ready);
        assert!(summary.route_handoff_ready);
        assert!(summary.is_route_handoff_ready());
        assert!(!summary.has_handoff_gaps());
        assert!(!summary.needs_attach_completion());
        assert!(!summary.needs_network_data());
        assert!(!summary.needs_routing_surface());
        assert!(!summary.needs_parent_or_route_anchor());
    }

    #[test]
    fn attach_route_handoff_summary_routes_blocked_network_and_anchor_work() {
        let table = NeighborTable::new(DeviceRole::Child);
        let action_summary = ThreadAttachActionSummary::from_summaries(
            MleMessageBatchSummary::empty(),
            table.summary_at(1_250),
        );
        let completion_summary = summarize_thread_attach_completion(
            action_summary,
            table
                .diagnostic_snapshot(None, 1_250)
                .unwrap()
                .supervision_plan(),
        );
        let unknown = NetworkDataTlv::new(NetworkDataTlvType::Unknown(42), false, vec![3]).unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![unknown]).unwrap();
        let network_data_readiness = network_data.summary().unwrap().readiness();

        let summary = ThreadAttachRouteHandoffSummary::from_completion_and_network_data(
            completion_summary,
            network_data_readiness,
        );

        assert_eq!(summary.required_handoff_check_count, 4);
        assert_eq!(summary.passed_handoff_check_count, 0);
        assert_eq!(summary.missing_handoff_check_count, 4);
        assert!(!summary.attach_complete);
        assert!(!summary.network_data_ready);
        assert!(!summary.routing_surface_ready);
        assert!(!summary.parent_or_route_anchor_ready);
        assert!(!summary.route_handoff_ready);
        assert!(!summary.is_route_handoff_ready());
        assert!(summary.has_handoff_gaps());
        assert!(summary.needs_attach_completion());
        assert!(summary.needs_network_data());
        assert!(summary.needs_routing_surface());
        assert!(summary.needs_parent_or_route_anchor());
    }

    #[test]
    fn attach_route_audit_summary_marks_ready_route_audit() {
        let mut table = NeighborTable::new(DeviceRole::Child);
        table.upsert(ThreadNeighbor::new(
            ThreadNeighborId(0x1000),
            DeviceRole::Router,
            NeighborRelationship::Parent,
            1_200,
            10_000,
        ));
        let action_summary = ThreadAttachActionSummary::from_summaries(
            MleMessageBatchSummary::empty(),
            table.summary_at(1_250),
        );
        let completion_summary = summarize_thread_attach_completion(
            action_summary,
            table
                .diagnostic_snapshot(None, 1_250)
                .unwrap()
                .supervision_plan(),
        );
        let border_router =
            NetworkDataTlv::new(NetworkDataTlvType::BorderRouter, true, vec![0xaa]).unwrap();
        let context = NetworkDataTlv::new(NetworkDataTlvType::Context, true, vec![0x01]).unwrap();
        let prefix = ThreadPrefixData::new(
            true,
            3,
            64,
            vec![0xfd, 0x00, 0xab, 0xcd, 0, 0, 0, 0],
            vec![border_router, context],
        )
        .unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![prefix.to_tlv().unwrap()]).unwrap();
        let network_data_readiness =
            summarize_thread_network_data_readiness(&network_data).unwrap();
        let handoff_summary =
            summarize_thread_attach_route_handoff(completion_summary, network_data_readiness);

        let summary = summarize_thread_attach_route_audit(handoff_summary);

        assert_eq!(summary.handoff_summary, handoff_summary);
        assert_eq!(summary.required_audit_check_count, 5);
        assert_eq!(summary.passed_audit_check_count, 5);
        assert_eq!(summary.missing_audit_check_count, 0);
        assert!(summary.route_handoff_ready);
        assert!(summary.attach_complete);
        assert!(summary.network_data_ready);
        assert!(summary.routing_surface_ready);
        assert!(summary.parent_or_route_anchor_ready);
        assert!(summary.route_audit_ready);
        assert!(summary.is_route_audit_ready());
        assert!(!summary.has_audit_gaps());
        assert!(!summary.needs_route_handoff());
        assert!(!summary.needs_attach_completion());
        assert!(!summary.needs_network_data());
        assert!(!summary.needs_routing_surface());
        assert!(!summary.needs_parent_or_route_anchor());
    }

    #[test]
    fn attach_route_audit_summary_routes_blocked_route_audit() {
        let table = NeighborTable::new(DeviceRole::Child);
        let action_summary = ThreadAttachActionSummary::from_summaries(
            MleMessageBatchSummary::empty(),
            table.summary_at(1_250),
        );
        let completion_summary = summarize_thread_attach_completion(
            action_summary,
            table
                .diagnostic_snapshot(None, 1_250)
                .unwrap()
                .supervision_plan(),
        );
        let unknown = NetworkDataTlv::new(NetworkDataTlvType::Unknown(42), false, vec![3]).unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![unknown]).unwrap();
        let network_data_readiness = network_data.summary().unwrap().readiness();
        let handoff_summary = ThreadAttachRouteHandoffSummary::from_completion_and_network_data(
            completion_summary,
            network_data_readiness,
        );

        let summary = ThreadAttachRouteAuditSummary::from_handoff_summary(handoff_summary);

        assert_eq!(summary.required_audit_check_count, 5);
        assert_eq!(summary.passed_audit_check_count, 0);
        assert_eq!(summary.missing_audit_check_count, 5);
        assert!(!summary.route_handoff_ready);
        assert!(!summary.attach_complete);
        assert!(!summary.network_data_ready);
        assert!(!summary.routing_surface_ready);
        assert!(!summary.parent_or_route_anchor_ready);
        assert!(!summary.route_audit_ready);
        assert!(!summary.is_route_audit_ready());
        assert!(summary.has_audit_gaps());
        assert!(summary.needs_route_handoff());
        assert!(summary.needs_attach_completion());
        assert!(summary.needs_network_data());
        assert!(summary.needs_routing_surface());
        assert!(summary.needs_parent_or_route_anchor());
    }

    #[test]
    fn attach_route_signoff_summary_marks_ready_route_signoff() {
        let mut table = NeighborTable::new(DeviceRole::Child);
        table.upsert(ThreadNeighbor::new(
            ThreadNeighborId(0x1000),
            DeviceRole::Router,
            NeighborRelationship::Parent,
            1_200,
            10_000,
        ));
        let action_summary = ThreadAttachActionSummary::from_summaries(
            MleMessageBatchSummary::empty(),
            table.summary_at(1_250),
        );
        let completion_summary = summarize_thread_attach_completion(
            action_summary,
            table
                .diagnostic_snapshot(None, 1_250)
                .unwrap()
                .supervision_plan(),
        );
        let border_router =
            NetworkDataTlv::new(NetworkDataTlvType::BorderRouter, true, vec![0xaa]).unwrap();
        let context = NetworkDataTlv::new(NetworkDataTlvType::Context, true, vec![0x01]).unwrap();
        let prefix = ThreadPrefixData::new(
            true,
            3,
            64,
            vec![0xfd, 0x00, 0xab, 0xcd, 0, 0, 0, 0],
            vec![border_router, context],
        )
        .unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![prefix.to_tlv().unwrap()]).unwrap();
        let network_data_readiness =
            summarize_thread_network_data_readiness(&network_data).unwrap();
        let handoff_summary =
            summarize_thread_attach_route_handoff(completion_summary, network_data_readiness);
        let audit_summary = summarize_thread_attach_route_audit(handoff_summary);

        let summary = summarize_thread_attach_route_signoff(audit_summary);

        assert_eq!(summary.audit_summary, audit_summary);
        assert_eq!(summary.required_signoff_check_count, 6);
        assert_eq!(summary.passed_signoff_check_count, 6);
        assert_eq!(summary.missing_signoff_check_count, 0);
        assert!(summary.route_audit_ready);
        assert!(summary.route_handoff_ready);
        assert!(summary.attach_complete);
        assert!(summary.network_data_ready);
        assert!(summary.routing_surface_ready);
        assert!(summary.parent_or_route_anchor_ready);
        assert!(summary.route_signoff_ready);
        assert!(summary.is_route_signoff_ready());
        assert!(!summary.has_signoff_gaps());
        assert!(!summary.needs_route_audit());
        assert!(!summary.needs_route_handoff());
        assert!(!summary.needs_attach_completion());
        assert!(!summary.needs_network_data());
        assert!(!summary.needs_routing_surface());
        assert!(!summary.needs_parent_or_route_anchor());
    }

    #[test]
    fn attach_route_signoff_summary_routes_blocked_signoff() {
        let table = NeighborTable::new(DeviceRole::Child);
        let action_summary = ThreadAttachActionSummary::from_summaries(
            MleMessageBatchSummary::empty(),
            table.summary_at(1_250),
        );
        let completion_summary = summarize_thread_attach_completion(
            action_summary,
            table
                .diagnostic_snapshot(None, 1_250)
                .unwrap()
                .supervision_plan(),
        );
        let unknown = NetworkDataTlv::new(NetworkDataTlvType::Unknown(42), false, vec![3]).unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![unknown]).unwrap();
        let network_data_readiness = network_data.summary().unwrap().readiness();
        let handoff_summary = ThreadAttachRouteHandoffSummary::from_completion_and_network_data(
            completion_summary,
            network_data_readiness,
        );
        let audit_summary = ThreadAttachRouteAuditSummary::from_handoff_summary(handoff_summary);

        let summary = ThreadAttachRouteSignoffSummary::from_audit_summary(audit_summary);

        assert_eq!(summary.required_signoff_check_count, 6);
        assert_eq!(summary.passed_signoff_check_count, 0);
        assert_eq!(summary.missing_signoff_check_count, 6);
        assert!(!summary.route_audit_ready);
        assert!(!summary.route_handoff_ready);
        assert!(!summary.attach_complete);
        assert!(!summary.network_data_ready);
        assert!(!summary.routing_surface_ready);
        assert!(!summary.parent_or_route_anchor_ready);
        assert!(!summary.route_signoff_ready);
        assert!(!summary.is_route_signoff_ready());
        assert!(summary.has_signoff_gaps());
        assert!(summary.needs_route_audit());
        assert!(summary.needs_route_handoff());
        assert!(summary.needs_attach_completion());
        assert!(summary.needs_network_data());
        assert!(summary.needs_routing_surface());
        assert!(summary.needs_parent_or_route_anchor());
    }

    #[test]
    fn attach_route_completion_summary_marks_ready_route_completion() {
        let mut table = NeighborTable::new(DeviceRole::Child);
        table.upsert(ThreadNeighbor::new(
            ThreadNeighborId(0x1000),
            DeviceRole::Router,
            NeighborRelationship::Parent,
            1_200,
            10_000,
        ));
        let action_summary = ThreadAttachActionSummary::from_summaries(
            MleMessageBatchSummary::empty(),
            table.summary_at(1_250),
        );
        let completion_summary = summarize_thread_attach_completion(
            action_summary,
            table
                .diagnostic_snapshot(None, 1_250)
                .unwrap()
                .supervision_plan(),
        );
        let border_router =
            NetworkDataTlv::new(NetworkDataTlvType::BorderRouter, true, vec![0xaa]).unwrap();
        let context = NetworkDataTlv::new(NetworkDataTlvType::Context, true, vec![0x01]).unwrap();
        let prefix = ThreadPrefixData::new(
            true,
            3,
            64,
            vec![0xfd, 0x00, 0xab, 0xcd, 0, 0, 0, 0],
            vec![border_router, context],
        )
        .unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![prefix.to_tlv().unwrap()]).unwrap();
        let network_data_readiness =
            summarize_thread_network_data_readiness(&network_data).unwrap();
        let handoff_summary =
            summarize_thread_attach_route_handoff(completion_summary, network_data_readiness);
        let audit_summary = summarize_thread_attach_route_audit(handoff_summary);
        let signoff_summary = summarize_thread_attach_route_signoff(audit_summary);

        let summary = summarize_thread_attach_route_completion(signoff_summary);

        assert_eq!(summary.signoff_summary, signoff_summary);
        assert_eq!(summary.required_route_completion_check_count, 7);
        assert_eq!(summary.passed_route_completion_check_count, 7);
        assert_eq!(summary.missing_route_completion_check_count, 0);
        assert!(summary.route_signoff_ready);
        assert!(summary.route_audit_ready);
        assert!(summary.route_handoff_ready);
        assert!(summary.attach_complete);
        assert!(summary.network_data_ready);
        assert!(summary.routing_surface_ready);
        assert!(summary.parent_or_route_anchor_ready);
        assert!(summary.route_completion_ready);
        assert!(summary.is_route_completion_ready());
        assert!(!summary.has_completion_gaps());
        assert!(!summary.needs_route_signoff());
        assert!(!summary.needs_route_audit());
        assert!(!summary.needs_route_handoff());
        assert!(!summary.needs_attach_completion());
        assert!(!summary.needs_network_data());
        assert!(!summary.needs_routing_surface());
        assert!(!summary.needs_parent_or_route_anchor());
    }

    #[test]
    fn attach_route_completion_summary_routes_blocked_completion() {
        let table = NeighborTable::new(DeviceRole::Child);
        let action_summary = ThreadAttachActionSummary::from_summaries(
            MleMessageBatchSummary::empty(),
            table.summary_at(1_250),
        );
        let completion_summary = summarize_thread_attach_completion(
            action_summary,
            table
                .diagnostic_snapshot(None, 1_250)
                .unwrap()
                .supervision_plan(),
        );
        let unknown = NetworkDataTlv::new(NetworkDataTlvType::Unknown(42), false, vec![3]).unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![unknown]).unwrap();
        let network_data_readiness = network_data.summary().unwrap().readiness();
        let handoff_summary = ThreadAttachRouteHandoffSummary::from_completion_and_network_data(
            completion_summary,
            network_data_readiness,
        );
        let audit_summary = ThreadAttachRouteAuditSummary::from_handoff_summary(handoff_summary);
        let signoff_summary = ThreadAttachRouteSignoffSummary::from_audit_summary(audit_summary);

        let summary = ThreadAttachRouteCompletionSummary::from_signoff_summary(signoff_summary);

        assert_eq!(summary.required_route_completion_check_count, 7);
        assert_eq!(summary.passed_route_completion_check_count, 0);
        assert_eq!(summary.missing_route_completion_check_count, 7);
        assert!(!summary.route_signoff_ready);
        assert!(!summary.route_audit_ready);
        assert!(!summary.route_handoff_ready);
        assert!(!summary.attach_complete);
        assert!(!summary.network_data_ready);
        assert!(!summary.routing_surface_ready);
        assert!(!summary.parent_or_route_anchor_ready);
        assert!(!summary.route_completion_ready);
        assert!(!summary.is_route_completion_ready());
        assert!(summary.has_completion_gaps());
        assert!(summary.needs_route_signoff());
        assert!(summary.needs_route_audit());
        assert!(summary.needs_route_handoff());
        assert!(summary.needs_attach_completion());
        assert!(summary.needs_network_data());
        assert!(summary.needs_routing_surface());
        assert!(summary.needs_parent_or_route_anchor());
    }

    #[test]
    fn attach_route_publication_summary_marks_ready_route_publication() {
        let mut table = NeighborTable::new(DeviceRole::Child);
        table.upsert(ThreadNeighbor::new(
            ThreadNeighborId(0x1000),
            DeviceRole::Router,
            NeighborRelationship::Parent,
            1_200,
            10_000,
        ));
        let action_summary = ThreadAttachActionSummary::from_summaries(
            MleMessageBatchSummary::empty(),
            table.summary_at(1_250),
        );
        let completion_summary = summarize_thread_attach_completion(
            action_summary,
            table
                .diagnostic_snapshot(None, 1_250)
                .unwrap()
                .supervision_plan(),
        );
        let border_router =
            NetworkDataTlv::new(NetworkDataTlvType::BorderRouter, true, vec![0xaa]).unwrap();
        let context = NetworkDataTlv::new(NetworkDataTlvType::Context, true, vec![0x01]).unwrap();
        let prefix = ThreadPrefixData::new(
            true,
            3,
            64,
            vec![0xfd, 0x00, 0xab, 0xcd, 0, 0, 0, 0],
            vec![border_router, context],
        )
        .unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![prefix.to_tlv().unwrap()]).unwrap();
        let network_data_readiness =
            summarize_thread_network_data_readiness(&network_data).unwrap();
        let handoff_summary =
            summarize_thread_attach_route_handoff(completion_summary, network_data_readiness);
        let audit_summary = summarize_thread_attach_route_audit(handoff_summary);
        let signoff_summary = summarize_thread_attach_route_signoff(audit_summary);
        let completion_summary = summarize_thread_attach_route_completion(signoff_summary);

        let summary = summarize_thread_attach_route_publication(completion_summary);

        assert_eq!(summary.completion_summary, completion_summary);
        assert_eq!(summary.required_route_publication_check_count, 8);
        assert_eq!(summary.passed_route_publication_check_count, 8);
        assert_eq!(summary.missing_route_publication_check_count, 0);
        assert!(summary.route_completion_ready);
        assert!(summary.route_signoff_ready);
        assert!(summary.route_audit_ready);
        assert!(summary.route_handoff_ready);
        assert!(summary.attach_complete);
        assert!(summary.network_data_ready);
        assert!(summary.routing_surface_ready);
        assert!(summary.parent_or_route_anchor_ready);
        assert!(summary.route_publication_ready);
        assert!(summary.is_route_publication_ready());
        assert!(!summary.has_publication_gaps());
        assert!(!summary.needs_route_completion());
        assert!(!summary.needs_route_signoff());
        assert!(!summary.needs_route_audit());
        assert!(!summary.needs_route_handoff());
        assert!(!summary.needs_attach_completion());
        assert!(!summary.needs_network_data());
        assert!(!summary.needs_routing_surface());
        assert!(!summary.needs_parent_or_route_anchor());
    }

    #[test]
    fn attach_route_publication_summary_routes_blocked_publication() {
        let table = NeighborTable::new(DeviceRole::Child);
        let action_summary = ThreadAttachActionSummary::from_summaries(
            MleMessageBatchSummary::empty(),
            table.summary_at(1_250),
        );
        let completion_summary = summarize_thread_attach_completion(
            action_summary,
            table
                .diagnostic_snapshot(None, 1_250)
                .unwrap()
                .supervision_plan(),
        );
        let unknown = NetworkDataTlv::new(NetworkDataTlvType::Unknown(42), false, vec![3]).unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![unknown]).unwrap();
        let network_data_readiness = network_data.summary().unwrap().readiness();
        let handoff_summary = ThreadAttachRouteHandoffSummary::from_completion_and_network_data(
            completion_summary,
            network_data_readiness,
        );
        let audit_summary = ThreadAttachRouteAuditSummary::from_handoff_summary(handoff_summary);
        let signoff_summary = ThreadAttachRouteSignoffSummary::from_audit_summary(audit_summary);
        let completion_summary =
            ThreadAttachRouteCompletionSummary::from_signoff_summary(signoff_summary);

        let summary =
            ThreadAttachRoutePublicationSummary::from_completion_summary(completion_summary);

        assert_eq!(summary.required_route_publication_check_count, 8);
        assert_eq!(summary.passed_route_publication_check_count, 0);
        assert_eq!(summary.missing_route_publication_check_count, 8);
        assert!(!summary.route_completion_ready);
        assert!(!summary.route_signoff_ready);
        assert!(!summary.route_audit_ready);
        assert!(!summary.route_handoff_ready);
        assert!(!summary.attach_complete);
        assert!(!summary.network_data_ready);
        assert!(!summary.routing_surface_ready);
        assert!(!summary.parent_or_route_anchor_ready);
        assert!(!summary.route_publication_ready);
        assert!(!summary.is_route_publication_ready());
        assert!(summary.has_publication_gaps());
        assert!(summary.needs_route_completion());
        assert!(summary.needs_route_signoff());
        assert!(summary.needs_route_audit());
        assert!(summary.needs_route_handoff());
        assert!(summary.needs_attach_completion());
        assert!(summary.needs_network_data());
        assert!(summary.needs_routing_surface());
        assert!(summary.needs_parent_or_route_anchor());
    }

    #[test]
    fn attach_route_verification_summary_marks_ready_route_verification() {
        let mut table = NeighborTable::new(DeviceRole::Child);
        table.upsert(ThreadNeighbor::new(
            ThreadNeighborId(0x1000),
            DeviceRole::Router,
            NeighborRelationship::Parent,
            1_200,
            10_000,
        ));
        let action_summary = ThreadAttachActionSummary::from_summaries(
            MleMessageBatchSummary::empty(),
            table.summary_at(1_250),
        );
        let completion_summary = summarize_thread_attach_completion(
            action_summary,
            table
                .diagnostic_snapshot(None, 1_250)
                .unwrap()
                .supervision_plan(),
        );
        let border_router =
            NetworkDataTlv::new(NetworkDataTlvType::BorderRouter, true, vec![0xaa]).unwrap();
        let context = NetworkDataTlv::new(NetworkDataTlvType::Context, true, vec![0x01]).unwrap();
        let prefix = ThreadPrefixData::new(
            true,
            3,
            64,
            vec![0xfd, 0x00, 0xab, 0xcd, 0, 0, 0, 0],
            vec![border_router, context],
        )
        .unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![prefix.to_tlv().unwrap()]).unwrap();
        let network_data_readiness =
            summarize_thread_network_data_readiness(&network_data).unwrap();
        let handoff_summary =
            summarize_thread_attach_route_handoff(completion_summary, network_data_readiness);
        let audit_summary = summarize_thread_attach_route_audit(handoff_summary);
        let signoff_summary = summarize_thread_attach_route_signoff(audit_summary);
        let completion_summary = summarize_thread_attach_route_completion(signoff_summary);
        let publication_summary = summarize_thread_attach_route_publication(completion_summary);

        let summary = summarize_thread_attach_route_verification(publication_summary);

        assert_eq!(summary.publication_summary, publication_summary);
        assert_eq!(summary.required_route_verification_check_count, 9);
        assert_eq!(summary.passed_route_verification_check_count, 9);
        assert_eq!(summary.missing_route_verification_check_count, 0);
        assert!(summary.route_publication_ready);
        assert!(summary.route_completion_ready);
        assert!(summary.route_signoff_ready);
        assert!(summary.route_audit_ready);
        assert!(summary.route_handoff_ready);
        assert!(summary.attach_complete);
        assert!(summary.network_data_ready);
        assert!(summary.routing_surface_ready);
        assert!(summary.parent_or_route_anchor_ready);
        assert!(summary.route_verification_ready);
        assert!(summary.is_route_verification_ready());
        assert!(!summary.has_verification_gaps());
        assert!(!summary.needs_route_publication());
        assert!(!summary.needs_route_completion());
        assert!(!summary.needs_route_signoff());
        assert!(!summary.needs_route_audit());
        assert!(!summary.needs_route_handoff());
        assert!(!summary.needs_attach_completion());
        assert!(!summary.needs_network_data());
        assert!(!summary.needs_routing_surface());
        assert!(!summary.needs_parent_or_route_anchor());
    }

    #[test]
    fn attach_route_verification_summary_routes_blocked_verification() {
        let table = NeighborTable::new(DeviceRole::Child);
        let action_summary = ThreadAttachActionSummary::from_summaries(
            MleMessageBatchSummary::empty(),
            table.summary_at(1_250),
        );
        let completion_summary = summarize_thread_attach_completion(
            action_summary,
            table
                .diagnostic_snapshot(None, 1_250)
                .unwrap()
                .supervision_plan(),
        );
        let unknown = NetworkDataTlv::new(NetworkDataTlvType::Unknown(42), false, vec![3]).unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![unknown]).unwrap();
        let network_data_readiness = network_data.summary().unwrap().readiness();
        let handoff_summary = ThreadAttachRouteHandoffSummary::from_completion_and_network_data(
            completion_summary,
            network_data_readiness,
        );
        let audit_summary = ThreadAttachRouteAuditSummary::from_handoff_summary(handoff_summary);
        let signoff_summary = ThreadAttachRouteSignoffSummary::from_audit_summary(audit_summary);
        let completion_summary =
            ThreadAttachRouteCompletionSummary::from_signoff_summary(signoff_summary);
        let publication_summary =
            ThreadAttachRoutePublicationSummary::from_completion_summary(completion_summary);

        let summary =
            ThreadAttachRouteVerificationSummary::from_publication_summary(publication_summary);

        assert_eq!(summary.required_route_verification_check_count, 9);
        assert_eq!(summary.passed_route_verification_check_count, 0);
        assert_eq!(summary.missing_route_verification_check_count, 9);
        assert!(!summary.route_publication_ready);
        assert!(!summary.route_completion_ready);
        assert!(!summary.route_signoff_ready);
        assert!(!summary.route_audit_ready);
        assert!(!summary.route_handoff_ready);
        assert!(!summary.attach_complete);
        assert!(!summary.network_data_ready);
        assert!(!summary.routing_surface_ready);
        assert!(!summary.parent_or_route_anchor_ready);
        assert!(!summary.route_verification_ready);
        assert!(!summary.is_route_verification_ready());
        assert!(summary.has_verification_gaps());
        assert!(summary.needs_route_publication());
        assert!(summary.needs_route_completion());
        assert!(summary.needs_route_signoff());
        assert!(summary.needs_route_audit());
        assert!(summary.needs_route_handoff());
        assert!(summary.needs_attach_completion());
        assert!(summary.needs_network_data());
        assert!(summary.needs_routing_surface());
        assert!(summary.needs_parent_or_route_anchor());
    }

    #[test]
    fn attach_route_validation_summary_marks_ready_route_validation() {
        let mut table = NeighborTable::new(DeviceRole::Child);
        table.upsert(ThreadNeighbor::new(
            ThreadNeighborId(0x1000),
            DeviceRole::Router,
            NeighborRelationship::Parent,
            1_200,
            10_000,
        ));
        let action_summary = ThreadAttachActionSummary::from_summaries(
            MleMessageBatchSummary::empty(),
            table.summary_at(1_250),
        );
        let completion_summary = summarize_thread_attach_completion(
            action_summary,
            table
                .diagnostic_snapshot(None, 1_250)
                .unwrap()
                .supervision_plan(),
        );
        let border_router =
            NetworkDataTlv::new(NetworkDataTlvType::BorderRouter, true, vec![0xaa]).unwrap();
        let context = NetworkDataTlv::new(NetworkDataTlvType::Context, true, vec![0x01]).unwrap();
        let prefix = ThreadPrefixData::new(
            true,
            3,
            64,
            vec![0xfd, 0x00, 0xab, 0xcd, 0, 0, 0, 0],
            vec![border_router, context],
        )
        .unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![prefix.to_tlv().unwrap()]).unwrap();
        let network_data_readiness =
            summarize_thread_network_data_readiness(&network_data).unwrap();
        let handoff_summary =
            summarize_thread_attach_route_handoff(completion_summary, network_data_readiness);
        let audit_summary = summarize_thread_attach_route_audit(handoff_summary);
        let signoff_summary = summarize_thread_attach_route_signoff(audit_summary);
        let completion_summary = summarize_thread_attach_route_completion(signoff_summary);
        let publication_summary = summarize_thread_attach_route_publication(completion_summary);
        let verification_summary = summarize_thread_attach_route_verification(publication_summary);

        let summary = summarize_thread_attach_route_validation(verification_summary);

        assert_eq!(summary.verification_summary, verification_summary);
        assert_eq!(summary.required_route_validation_check_count, 10);
        assert_eq!(summary.passed_route_validation_check_count, 10);
        assert_eq!(summary.missing_route_validation_check_count, 0);
        assert!(summary.route_verification_ready);
        assert!(summary.route_publication_ready);
        assert!(summary.route_completion_ready);
        assert!(summary.route_signoff_ready);
        assert!(summary.route_audit_ready);
        assert!(summary.route_handoff_ready);
        assert!(summary.attach_complete);
        assert!(summary.network_data_ready);
        assert!(summary.routing_surface_ready);
        assert!(summary.parent_or_route_anchor_ready);
        assert!(summary.route_validation_ready);
        assert!(summary.is_route_validation_ready());
        assert!(!summary.has_validation_gaps());
        assert!(!summary.needs_route_verification());
        assert!(!summary.needs_route_publication());
        assert!(!summary.needs_route_completion());
        assert!(!summary.needs_route_signoff());
        assert!(!summary.needs_route_audit());
        assert!(!summary.needs_route_handoff());
        assert!(!summary.needs_attach_completion());
        assert!(!summary.needs_network_data());
        assert!(!summary.needs_routing_surface());
        assert!(!summary.needs_parent_or_route_anchor());
    }

    #[test]
    fn attach_route_validation_summary_routes_blocked_validation() {
        let table = NeighborTable::new(DeviceRole::Child);
        let action_summary = ThreadAttachActionSummary::from_summaries(
            MleMessageBatchSummary::empty(),
            table.summary_at(1_250),
        );
        let completion_summary = summarize_thread_attach_completion(
            action_summary,
            table
                .diagnostic_snapshot(None, 1_250)
                .unwrap()
                .supervision_plan(),
        );
        let unknown = NetworkDataTlv::new(NetworkDataTlvType::Unknown(42), false, vec![3]).unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![unknown]).unwrap();
        let network_data_readiness = network_data.summary().unwrap().readiness();
        let handoff_summary = ThreadAttachRouteHandoffSummary::from_completion_and_network_data(
            completion_summary,
            network_data_readiness,
        );
        let audit_summary = ThreadAttachRouteAuditSummary::from_handoff_summary(handoff_summary);
        let signoff_summary = ThreadAttachRouteSignoffSummary::from_audit_summary(audit_summary);
        let completion_summary =
            ThreadAttachRouteCompletionSummary::from_signoff_summary(signoff_summary);
        let publication_summary =
            ThreadAttachRoutePublicationSummary::from_completion_summary(completion_summary);
        let verification_summary =
            ThreadAttachRouteVerificationSummary::from_publication_summary(publication_summary);

        let summary =
            ThreadAttachRouteValidationSummary::from_verification_summary(verification_summary);

        assert_eq!(summary.required_route_validation_check_count, 10);
        assert_eq!(summary.passed_route_validation_check_count, 0);
        assert_eq!(summary.missing_route_validation_check_count, 10);
        assert!(!summary.route_verification_ready);
        assert!(!summary.route_publication_ready);
        assert!(!summary.route_completion_ready);
        assert!(!summary.route_signoff_ready);
        assert!(!summary.route_audit_ready);
        assert!(!summary.route_handoff_ready);
        assert!(!summary.attach_complete);
        assert!(!summary.network_data_ready);
        assert!(!summary.routing_surface_ready);
        assert!(!summary.parent_or_route_anchor_ready);
        assert!(!summary.route_validation_ready);
        assert!(!summary.is_route_validation_ready());
        assert!(summary.has_validation_gaps());
        assert!(summary.needs_route_verification());
        assert!(summary.needs_route_publication());
        assert!(summary.needs_route_completion());
        assert!(summary.needs_route_signoff());
        assert!(summary.needs_route_audit());
        assert!(summary.needs_route_handoff());
        assert!(summary.needs_attach_completion());
        assert!(summary.needs_network_data());
        assert!(summary.needs_routing_surface());
        assert!(summary.needs_parent_or_route_anchor());
    }

    #[test]
    fn attach_route_certification_summary_marks_ready_route_certification() {
        let mut table = NeighborTable::new(DeviceRole::Child);
        table.upsert(ThreadNeighbor::new(
            ThreadNeighborId(0x1000),
            DeviceRole::Router,
            NeighborRelationship::Parent,
            1_200,
            10_000,
        ));
        let action_summary = ThreadAttachActionSummary::from_summaries(
            MleMessageBatchSummary::empty(),
            table.summary_at(1_250),
        );
        let completion_summary = summarize_thread_attach_completion(
            action_summary,
            table
                .diagnostic_snapshot(None, 1_250)
                .unwrap()
                .supervision_plan(),
        );
        let border_router =
            NetworkDataTlv::new(NetworkDataTlvType::BorderRouter, true, vec![0xaa]).unwrap();
        let context = NetworkDataTlv::new(NetworkDataTlvType::Context, true, vec![0x01]).unwrap();
        let prefix = ThreadPrefixData::new(
            true,
            3,
            64,
            vec![0xfd, 0x00, 0xab, 0xcd, 0, 0, 0, 0],
            vec![border_router, context],
        )
        .unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![prefix.to_tlv().unwrap()]).unwrap();
        let network_data_readiness =
            summarize_thread_network_data_readiness(&network_data).unwrap();
        let handoff_summary =
            summarize_thread_attach_route_handoff(completion_summary, network_data_readiness);
        let audit_summary = summarize_thread_attach_route_audit(handoff_summary);
        let signoff_summary = summarize_thread_attach_route_signoff(audit_summary);
        let completion_summary = summarize_thread_attach_route_completion(signoff_summary);
        let publication_summary = summarize_thread_attach_route_publication(completion_summary);
        let verification_summary = summarize_thread_attach_route_verification(publication_summary);
        let validation_summary = summarize_thread_attach_route_validation(verification_summary);

        let summary = summarize_thread_attach_route_certification(validation_summary);

        assert_eq!(summary.validation_summary, validation_summary);
        assert_eq!(summary.required_route_certification_check_count, 11);
        assert_eq!(summary.passed_route_certification_check_count, 11);
        assert_eq!(summary.missing_route_certification_check_count, 0);
        assert!(summary.route_validation_ready);
        assert!(summary.route_verification_ready);
        assert!(summary.route_publication_ready);
        assert!(summary.route_completion_ready);
        assert!(summary.route_signoff_ready);
        assert!(summary.route_audit_ready);
        assert!(summary.route_handoff_ready);
        assert!(summary.attach_complete);
        assert!(summary.network_data_ready);
        assert!(summary.routing_surface_ready);
        assert!(summary.parent_or_route_anchor_ready);
        assert!(summary.route_certification_ready);
        assert!(summary.is_route_certification_ready());
        assert!(!summary.has_certification_gaps());
        assert!(!summary.needs_route_validation());
        assert!(!summary.needs_route_verification());
        assert!(!summary.needs_route_publication());
        assert!(!summary.needs_route_completion());
        assert!(!summary.needs_route_signoff());
        assert!(!summary.needs_route_audit());
        assert!(!summary.needs_route_handoff());
        assert!(!summary.needs_attach_completion());
        assert!(!summary.needs_network_data());
        assert!(!summary.needs_routing_surface());
        assert!(!summary.needs_parent_or_route_anchor());
    }

    #[test]
    fn attach_route_certification_summary_routes_blocked_certification() {
        let table = NeighborTable::new(DeviceRole::Child);
        let action_summary = ThreadAttachActionSummary::from_summaries(
            MleMessageBatchSummary::empty(),
            table.summary_at(1_250),
        );
        let completion_summary = summarize_thread_attach_completion(
            action_summary,
            table
                .diagnostic_snapshot(None, 1_250)
                .unwrap()
                .supervision_plan(),
        );
        let unknown = NetworkDataTlv::new(NetworkDataTlvType::Unknown(42), false, vec![3]).unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![unknown]).unwrap();
        let network_data_readiness = network_data.summary().unwrap().readiness();
        let handoff_summary = ThreadAttachRouteHandoffSummary::from_completion_and_network_data(
            completion_summary,
            network_data_readiness,
        );
        let audit_summary = ThreadAttachRouteAuditSummary::from_handoff_summary(handoff_summary);
        let signoff_summary = ThreadAttachRouteSignoffSummary::from_audit_summary(audit_summary);
        let completion_summary =
            ThreadAttachRouteCompletionSummary::from_signoff_summary(signoff_summary);
        let publication_summary =
            ThreadAttachRoutePublicationSummary::from_completion_summary(completion_summary);
        let verification_summary =
            ThreadAttachRouteVerificationSummary::from_publication_summary(publication_summary);
        let validation_summary =
            ThreadAttachRouteValidationSummary::from_verification_summary(verification_summary);

        let summary =
            ThreadAttachRouteCertificationSummary::from_validation_summary(validation_summary);

        assert_eq!(summary.required_route_certification_check_count, 11);
        assert_eq!(summary.passed_route_certification_check_count, 0);
        assert_eq!(summary.missing_route_certification_check_count, 11);
        assert!(!summary.route_validation_ready);
        assert!(!summary.route_verification_ready);
        assert!(!summary.route_publication_ready);
        assert!(!summary.route_completion_ready);
        assert!(!summary.route_signoff_ready);
        assert!(!summary.route_audit_ready);
        assert!(!summary.route_handoff_ready);
        assert!(!summary.attach_complete);
        assert!(!summary.network_data_ready);
        assert!(!summary.routing_surface_ready);
        assert!(!summary.parent_or_route_anchor_ready);
        assert!(!summary.route_certification_ready);
        assert!(!summary.is_route_certification_ready());
        assert!(summary.has_certification_gaps());
        assert!(summary.needs_route_validation());
        assert!(summary.needs_route_verification());
        assert!(summary.needs_route_publication());
        assert!(summary.needs_route_completion());
        assert!(summary.needs_route_signoff());
        assert!(summary.needs_route_audit());
        assert!(summary.needs_route_handoff());
        assert!(summary.needs_attach_completion());
        assert!(summary.needs_network_data());
        assert!(summary.needs_routing_surface());
        assert!(summary.needs_parent_or_route_anchor());
    }

    #[test]
    fn attach_route_approval_summary_marks_ready_route_approval() {
        let mut table = NeighborTable::new(DeviceRole::Child);
        table.upsert(ThreadNeighbor::new(
            ThreadNeighborId(0x1000),
            DeviceRole::Router,
            NeighborRelationship::Parent,
            1_200,
            10_000,
        ));
        let action_summary = ThreadAttachActionSummary::from_summaries(
            MleMessageBatchSummary::empty(),
            table.summary_at(1_250),
        );
        let completion_summary = summarize_thread_attach_completion(
            action_summary,
            table
                .diagnostic_snapshot(None, 1_250)
                .unwrap()
                .supervision_plan(),
        );
        let border_router =
            NetworkDataTlv::new(NetworkDataTlvType::BorderRouter, true, vec![0xaa]).unwrap();
        let context = NetworkDataTlv::new(NetworkDataTlvType::Context, true, vec![0x01]).unwrap();
        let prefix = ThreadPrefixData::new(
            true,
            3,
            64,
            vec![0xfd, 0x00, 0xab, 0xcd, 0, 0, 0, 0],
            vec![border_router, context],
        )
        .unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![prefix.to_tlv().unwrap()]).unwrap();
        let network_data_readiness =
            summarize_thread_network_data_readiness(&network_data).unwrap();
        let handoff_summary =
            summarize_thread_attach_route_handoff(completion_summary, network_data_readiness);
        let audit_summary = summarize_thread_attach_route_audit(handoff_summary);
        let signoff_summary = summarize_thread_attach_route_signoff(audit_summary);
        let completion_summary = summarize_thread_attach_route_completion(signoff_summary);
        let publication_summary = summarize_thread_attach_route_publication(completion_summary);
        let verification_summary = summarize_thread_attach_route_verification(publication_summary);
        let validation_summary = summarize_thread_attach_route_validation(verification_summary);
        let certification_summary = summarize_thread_attach_route_certification(validation_summary);

        let summary = summarize_thread_attach_route_approval(certification_summary);

        assert_eq!(summary.certification_summary, certification_summary);
        assert_eq!(summary.required_route_approval_check_count, 12);
        assert_eq!(summary.passed_route_approval_check_count, 12);
        assert_eq!(summary.missing_route_approval_check_count, 0);
        assert!(summary.route_certification_ready);
        assert!(summary.route_validation_ready);
        assert!(summary.route_verification_ready);
        assert!(summary.route_publication_ready);
        assert!(summary.route_completion_ready);
        assert!(summary.route_signoff_ready);
        assert!(summary.route_audit_ready);
        assert!(summary.route_handoff_ready);
        assert!(summary.attach_complete);
        assert!(summary.network_data_ready);
        assert!(summary.routing_surface_ready);
        assert!(summary.parent_or_route_anchor_ready);
        assert!(summary.route_approval_ready);
        assert!(summary.is_route_approval_ready());
        assert!(!summary.has_approval_gaps());
        assert!(!summary.needs_route_certification());
        assert!(!summary.needs_route_validation());
        assert!(!summary.needs_route_verification());
        assert!(!summary.needs_route_publication());
        assert!(!summary.needs_route_completion());
        assert!(!summary.needs_route_signoff());
        assert!(!summary.needs_route_audit());
        assert!(!summary.needs_route_handoff());
        assert!(!summary.needs_attach_completion());
        assert!(!summary.needs_network_data());
        assert!(!summary.needs_routing_surface());
        assert!(!summary.needs_parent_or_route_anchor());
    }

    #[test]
    fn attach_route_approval_summary_routes_blocked_approval() {
        let table = NeighborTable::new(DeviceRole::Child);
        let action_summary = ThreadAttachActionSummary::from_summaries(
            MleMessageBatchSummary::empty(),
            table.summary_at(1_250),
        );
        let completion_summary = summarize_thread_attach_completion(
            action_summary,
            table
                .diagnostic_snapshot(None, 1_250)
                .unwrap()
                .supervision_plan(),
        );
        let unknown = NetworkDataTlv::new(NetworkDataTlvType::Unknown(42), false, vec![3]).unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![unknown]).unwrap();
        let network_data_readiness = network_data.summary().unwrap().readiness();
        let handoff_summary = ThreadAttachRouteHandoffSummary::from_completion_and_network_data(
            completion_summary,
            network_data_readiness,
        );
        let audit_summary = ThreadAttachRouteAuditSummary::from_handoff_summary(handoff_summary);
        let signoff_summary = ThreadAttachRouteSignoffSummary::from_audit_summary(audit_summary);
        let completion_summary =
            ThreadAttachRouteCompletionSummary::from_signoff_summary(signoff_summary);
        let publication_summary =
            ThreadAttachRoutePublicationSummary::from_completion_summary(completion_summary);
        let verification_summary =
            ThreadAttachRouteVerificationSummary::from_publication_summary(publication_summary);
        let validation_summary =
            ThreadAttachRouteValidationSummary::from_verification_summary(verification_summary);
        let certification_summary =
            ThreadAttachRouteCertificationSummary::from_validation_summary(validation_summary);

        let summary =
            ThreadAttachRouteApprovalSummary::from_certification_summary(certification_summary);

        assert_eq!(summary.required_route_approval_check_count, 12);
        assert_eq!(summary.passed_route_approval_check_count, 0);
        assert_eq!(summary.missing_route_approval_check_count, 12);
        assert!(!summary.route_certification_ready);
        assert!(!summary.route_validation_ready);
        assert!(!summary.route_verification_ready);
        assert!(!summary.route_publication_ready);
        assert!(!summary.route_completion_ready);
        assert!(!summary.route_signoff_ready);
        assert!(!summary.route_audit_ready);
        assert!(!summary.route_handoff_ready);
        assert!(!summary.attach_complete);
        assert!(!summary.network_data_ready);
        assert!(!summary.routing_surface_ready);
        assert!(!summary.parent_or_route_anchor_ready);
        assert!(!summary.route_approval_ready);
        assert!(!summary.is_route_approval_ready());
        assert!(summary.has_approval_gaps());
        assert!(summary.needs_route_certification());
        assert!(summary.needs_route_validation());
        assert!(summary.needs_route_verification());
        assert!(summary.needs_route_publication());
        assert!(summary.needs_route_completion());
        assert!(summary.needs_route_signoff());
        assert!(summary.needs_route_audit());
        assert!(summary.needs_route_handoff());
        assert!(summary.needs_attach_completion());
        assert!(summary.needs_network_data());
        assert!(summary.needs_routing_surface());
        assert!(summary.needs_parent_or_route_anchor());
    }

    #[test]
    fn attach_route_activation_summary_marks_ready_route_activation() {
        let mut table = NeighborTable::new(DeviceRole::Child);
        table.upsert(ThreadNeighbor::new(
            ThreadNeighborId(0x1000),
            DeviceRole::Router,
            NeighborRelationship::Parent,
            1_200,
            10_000,
        ));
        let action_summary = ThreadAttachActionSummary::from_summaries(
            MleMessageBatchSummary::empty(),
            table.summary_at(1_250),
        );
        let completion_summary = summarize_thread_attach_completion(
            action_summary,
            table
                .diagnostic_snapshot(None, 1_250)
                .unwrap()
                .supervision_plan(),
        );
        let border_router =
            NetworkDataTlv::new(NetworkDataTlvType::BorderRouter, true, vec![0xaa]).unwrap();
        let context = NetworkDataTlv::new(NetworkDataTlvType::Context, true, vec![0x01]).unwrap();
        let prefix = ThreadPrefixData::new(
            true,
            3,
            64,
            vec![0xfd, 0x00, 0xab, 0xcd, 0, 0, 0, 0],
            vec![border_router, context],
        )
        .unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![prefix.to_tlv().unwrap()]).unwrap();
        let network_data_readiness =
            summarize_thread_network_data_readiness(&network_data).unwrap();
        let handoff_summary =
            summarize_thread_attach_route_handoff(completion_summary, network_data_readiness);
        let audit_summary = summarize_thread_attach_route_audit(handoff_summary);
        let signoff_summary = summarize_thread_attach_route_signoff(audit_summary);
        let completion_summary = summarize_thread_attach_route_completion(signoff_summary);
        let publication_summary = summarize_thread_attach_route_publication(completion_summary);
        let verification_summary = summarize_thread_attach_route_verification(publication_summary);
        let validation_summary = summarize_thread_attach_route_validation(verification_summary);
        let certification_summary = summarize_thread_attach_route_certification(validation_summary);
        let approval_summary = summarize_thread_attach_route_approval(certification_summary);

        let summary = summarize_thread_attach_route_activation(approval_summary);

        assert_eq!(summary.approval_summary, approval_summary);
        assert_eq!(summary.required_route_activation_check_count, 13);
        assert_eq!(summary.passed_route_activation_check_count, 13);
        assert_eq!(summary.missing_route_activation_check_count, 0);
        assert!(summary.route_approval_ready);
        assert!(summary.route_certification_ready);
        assert!(summary.route_validation_ready);
        assert!(summary.route_verification_ready);
        assert!(summary.route_publication_ready);
        assert!(summary.route_completion_ready);
        assert!(summary.route_signoff_ready);
        assert!(summary.route_audit_ready);
        assert!(summary.route_handoff_ready);
        assert!(summary.attach_complete);
        assert!(summary.network_data_ready);
        assert!(summary.routing_surface_ready);
        assert!(summary.parent_or_route_anchor_ready);
        assert!(summary.route_activation_ready);
        assert!(summary.is_route_activation_ready());
        assert!(!summary.has_activation_gaps());
        assert!(!summary.needs_route_approval());
        assert!(!summary.needs_route_certification());
        assert!(!summary.needs_route_validation());
        assert!(!summary.needs_route_verification());
        assert!(!summary.needs_route_publication());
        assert!(!summary.needs_route_completion());
        assert!(!summary.needs_route_signoff());
        assert!(!summary.needs_route_audit());
        assert!(!summary.needs_route_handoff());
        assert!(!summary.needs_attach_completion());
        assert!(!summary.needs_network_data());
        assert!(!summary.needs_routing_surface());
        assert!(!summary.needs_parent_or_route_anchor());
    }

    #[test]
    fn attach_route_activation_summary_routes_blocked_activation() {
        let table = NeighborTable::new(DeviceRole::Child);
        let action_summary = ThreadAttachActionSummary::from_summaries(
            MleMessageBatchSummary::empty(),
            table.summary_at(1_250),
        );
        let completion_summary = summarize_thread_attach_completion(
            action_summary,
            table
                .diagnostic_snapshot(None, 1_250)
                .unwrap()
                .supervision_plan(),
        );
        let unknown = NetworkDataTlv::new(NetworkDataTlvType::Unknown(42), false, vec![3]).unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![unknown]).unwrap();
        let network_data_readiness = network_data.summary().unwrap().readiness();
        let handoff_summary = ThreadAttachRouteHandoffSummary::from_completion_and_network_data(
            completion_summary,
            network_data_readiness,
        );
        let audit_summary = ThreadAttachRouteAuditSummary::from_handoff_summary(handoff_summary);
        let signoff_summary = ThreadAttachRouteSignoffSummary::from_audit_summary(audit_summary);
        let completion_summary =
            ThreadAttachRouteCompletionSummary::from_signoff_summary(signoff_summary);
        let publication_summary =
            ThreadAttachRoutePublicationSummary::from_completion_summary(completion_summary);
        let verification_summary =
            ThreadAttachRouteVerificationSummary::from_publication_summary(publication_summary);
        let validation_summary =
            ThreadAttachRouteValidationSummary::from_verification_summary(verification_summary);
        let certification_summary =
            ThreadAttachRouteCertificationSummary::from_validation_summary(validation_summary);
        let approval_summary =
            ThreadAttachRouteApprovalSummary::from_certification_summary(certification_summary);

        let summary = ThreadAttachRouteActivationSummary::from_approval_summary(approval_summary);

        assert_eq!(summary.required_route_activation_check_count, 13);
        assert_eq!(summary.passed_route_activation_check_count, 0);
        assert_eq!(summary.missing_route_activation_check_count, 13);
        assert!(!summary.route_approval_ready);
        assert!(!summary.route_certification_ready);
        assert!(!summary.route_validation_ready);
        assert!(!summary.route_verification_ready);
        assert!(!summary.route_publication_ready);
        assert!(!summary.route_completion_ready);
        assert!(!summary.route_signoff_ready);
        assert!(!summary.route_audit_ready);
        assert!(!summary.route_handoff_ready);
        assert!(!summary.attach_complete);
        assert!(!summary.network_data_ready);
        assert!(!summary.routing_surface_ready);
        assert!(!summary.parent_or_route_anchor_ready);
        assert!(!summary.route_activation_ready);
        assert!(!summary.is_route_activation_ready());
        assert!(summary.has_activation_gaps());
        assert!(summary.needs_route_approval());
        assert!(summary.needs_route_certification());
        assert!(summary.needs_route_validation());
        assert!(summary.needs_route_verification());
        assert!(summary.needs_route_publication());
        assert!(summary.needs_route_completion());
        assert!(summary.needs_route_signoff());
        assert!(summary.needs_route_audit());
        assert!(summary.needs_route_handoff());
        assert!(summary.needs_attach_completion());
        assert!(summary.needs_network_data());
        assert!(summary.needs_routing_surface());
        assert!(summary.needs_parent_or_route_anchor());
    }

    #[test]
    fn attach_route_rollout_summary_marks_ready_route_rollout() {
        let mut table = NeighborTable::new(DeviceRole::Child);
        table.upsert(ThreadNeighbor::new(
            ThreadNeighborId(0x1000),
            DeviceRole::Router,
            NeighborRelationship::Parent,
            1_200,
            10_000,
        ));
        let action_summary = ThreadAttachActionSummary::from_summaries(
            MleMessageBatchSummary::empty(),
            table.summary_at(1_250),
        );
        let completion_summary = summarize_thread_attach_completion(
            action_summary,
            table
                .diagnostic_snapshot(None, 1_250)
                .unwrap()
                .supervision_plan(),
        );
        let border_router =
            NetworkDataTlv::new(NetworkDataTlvType::BorderRouter, true, vec![0xaa]).unwrap();
        let context = NetworkDataTlv::new(NetworkDataTlvType::Context, true, vec![0x01]).unwrap();
        let prefix = ThreadPrefixData::new(
            true,
            3,
            64,
            vec![0xfd, 0x00, 0xab, 0xcd, 0, 0, 0, 0],
            vec![border_router, context],
        )
        .unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![prefix.to_tlv().unwrap()]).unwrap();
        let network_data_readiness =
            summarize_thread_network_data_readiness(&network_data).unwrap();
        let handoff_summary =
            summarize_thread_attach_route_handoff(completion_summary, network_data_readiness);
        let audit_summary = summarize_thread_attach_route_audit(handoff_summary);
        let signoff_summary = summarize_thread_attach_route_signoff(audit_summary);
        let completion_summary = summarize_thread_attach_route_completion(signoff_summary);
        let publication_summary = summarize_thread_attach_route_publication(completion_summary);
        let verification_summary = summarize_thread_attach_route_verification(publication_summary);
        let validation_summary = summarize_thread_attach_route_validation(verification_summary);
        let certification_summary = summarize_thread_attach_route_certification(validation_summary);
        let approval_summary = summarize_thread_attach_route_approval(certification_summary);
        let activation_summary = summarize_thread_attach_route_activation(approval_summary);

        let summary = summarize_thread_attach_route_rollout(activation_summary);

        assert_eq!(summary.activation_summary, activation_summary);
        assert_eq!(summary.required_route_rollout_check_count, 14);
        assert_eq!(summary.passed_route_rollout_check_count, 14);
        assert_eq!(summary.missing_route_rollout_check_count, 0);
        assert!(summary.route_activation_ready);
        assert!(summary.route_approval_ready);
        assert!(summary.route_certification_ready);
        assert!(summary.route_validation_ready);
        assert!(summary.route_verification_ready);
        assert!(summary.route_publication_ready);
        assert!(summary.route_completion_ready);
        assert!(summary.route_signoff_ready);
        assert!(summary.route_audit_ready);
        assert!(summary.route_handoff_ready);
        assert!(summary.attach_complete);
        assert!(summary.network_data_ready);
        assert!(summary.routing_surface_ready);
        assert!(summary.parent_or_route_anchor_ready);
        assert!(summary.route_rollout_ready);
        assert!(summary.is_route_rollout_ready());
        assert!(!summary.has_rollout_gaps());
        assert!(!summary.needs_route_activation());
        assert!(!summary.needs_route_approval());
        assert!(!summary.needs_route_certification());
        assert!(!summary.needs_route_validation());
        assert!(!summary.needs_route_verification());
        assert!(!summary.needs_route_publication());
        assert!(!summary.needs_route_completion());
        assert!(!summary.needs_route_signoff());
        assert!(!summary.needs_route_audit());
        assert!(!summary.needs_route_handoff());
        assert!(!summary.needs_attach_completion());
        assert!(!summary.needs_network_data());
        assert!(!summary.needs_routing_surface());
        assert!(!summary.needs_parent_or_route_anchor());
    }

    #[test]
    fn attach_route_rollout_summary_routes_blocked_rollout() {
        let table = NeighborTable::new(DeviceRole::Child);
        let action_summary = ThreadAttachActionSummary::from_summaries(
            MleMessageBatchSummary::empty(),
            table.summary_at(1_250),
        );
        let completion_summary = summarize_thread_attach_completion(
            action_summary,
            table
                .diagnostic_snapshot(None, 1_250)
                .unwrap()
                .supervision_plan(),
        );
        let unknown = NetworkDataTlv::new(NetworkDataTlvType::Unknown(42), false, vec![3]).unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![unknown]).unwrap();
        let network_data_readiness = network_data.summary().unwrap().readiness();
        let handoff_summary = ThreadAttachRouteHandoffSummary::from_completion_and_network_data(
            completion_summary,
            network_data_readiness,
        );
        let audit_summary = ThreadAttachRouteAuditSummary::from_handoff_summary(handoff_summary);
        let signoff_summary = ThreadAttachRouteSignoffSummary::from_audit_summary(audit_summary);
        let completion_summary =
            ThreadAttachRouteCompletionSummary::from_signoff_summary(signoff_summary);
        let publication_summary =
            ThreadAttachRoutePublicationSummary::from_completion_summary(completion_summary);
        let verification_summary =
            ThreadAttachRouteVerificationSummary::from_publication_summary(publication_summary);
        let validation_summary =
            ThreadAttachRouteValidationSummary::from_verification_summary(verification_summary);
        let certification_summary =
            ThreadAttachRouteCertificationSummary::from_validation_summary(validation_summary);
        let approval_summary =
            ThreadAttachRouteApprovalSummary::from_certification_summary(certification_summary);
        let activation_summary =
            ThreadAttachRouteActivationSummary::from_approval_summary(approval_summary);

        let summary = ThreadAttachRouteRolloutSummary::from_activation_summary(activation_summary);

        assert_eq!(summary.required_route_rollout_check_count, 14);
        assert_eq!(summary.passed_route_rollout_check_count, 0);
        assert_eq!(summary.missing_route_rollout_check_count, 14);
        assert!(!summary.route_activation_ready);
        assert!(!summary.route_approval_ready);
        assert!(!summary.route_certification_ready);
        assert!(!summary.route_validation_ready);
        assert!(!summary.route_verification_ready);
        assert!(!summary.route_publication_ready);
        assert!(!summary.route_completion_ready);
        assert!(!summary.route_signoff_ready);
        assert!(!summary.route_audit_ready);
        assert!(!summary.route_handoff_ready);
        assert!(!summary.attach_complete);
        assert!(!summary.network_data_ready);
        assert!(!summary.routing_surface_ready);
        assert!(!summary.parent_or_route_anchor_ready);
        assert!(!summary.route_rollout_ready);
        assert!(!summary.is_route_rollout_ready());
        assert!(summary.has_rollout_gaps());
        assert!(summary.needs_route_activation());
        assert!(summary.needs_route_approval());
        assert!(summary.needs_route_certification());
        assert!(summary.needs_route_validation());
        assert!(summary.needs_route_verification());
        assert!(summary.needs_route_publication());
        assert!(summary.needs_route_completion());
        assert!(summary.needs_route_signoff());
        assert!(summary.needs_route_audit());
        assert!(summary.needs_route_handoff());
        assert!(summary.needs_attach_completion());
        assert!(summary.needs_network_data());
        assert!(summary.needs_routing_surface());
        assert!(summary.needs_parent_or_route_anchor());
    }

    #[test]
    fn attach_route_adoption_summary_marks_ready_route_adoption() {
        let mut table = NeighborTable::new(DeviceRole::Child);
        table.upsert(ThreadNeighbor::new(
            ThreadNeighborId(0x1000),
            DeviceRole::Router,
            NeighborRelationship::Parent,
            1_200,
            10_000,
        ));
        let action_summary = ThreadAttachActionSummary::from_summaries(
            MleMessageBatchSummary::empty(),
            table.summary_at(1_250),
        );
        let completion_summary = summarize_thread_attach_completion(
            action_summary,
            table
                .diagnostic_snapshot(None, 1_250)
                .unwrap()
                .supervision_plan(),
        );
        let border_router =
            NetworkDataTlv::new(NetworkDataTlvType::BorderRouter, true, vec![0xaa]).unwrap();
        let context = NetworkDataTlv::new(NetworkDataTlvType::Context, true, vec![0x01]).unwrap();
        let prefix = ThreadPrefixData::new(
            true,
            3,
            64,
            vec![0xfd, 0x00, 0xab, 0xcd, 0, 0, 0, 0],
            vec![border_router, context],
        )
        .unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![prefix.to_tlv().unwrap()]).unwrap();
        let network_data_readiness =
            summarize_thread_network_data_readiness(&network_data).unwrap();
        let handoff_summary =
            summarize_thread_attach_route_handoff(completion_summary, network_data_readiness);
        let audit_summary = summarize_thread_attach_route_audit(handoff_summary);
        let signoff_summary = summarize_thread_attach_route_signoff(audit_summary);
        let completion_summary = summarize_thread_attach_route_completion(signoff_summary);
        let publication_summary = summarize_thread_attach_route_publication(completion_summary);
        let verification_summary = summarize_thread_attach_route_verification(publication_summary);
        let validation_summary = summarize_thread_attach_route_validation(verification_summary);
        let certification_summary = summarize_thread_attach_route_certification(validation_summary);
        let approval_summary = summarize_thread_attach_route_approval(certification_summary);
        let activation_summary = summarize_thread_attach_route_activation(approval_summary);
        let rollout_summary = summarize_thread_attach_route_rollout(activation_summary);

        let summary = summarize_thread_attach_route_adoption(rollout_summary);

        assert_eq!(summary.rollout_summary, rollout_summary);
        assert_eq!(summary.required_route_adoption_check_count, 15);
        assert_eq!(summary.passed_route_adoption_check_count, 15);
        assert_eq!(summary.missing_route_adoption_check_count, 0);
        assert!(summary.route_rollout_ready);
        assert!(summary.route_activation_ready);
        assert!(summary.route_approval_ready);
        assert!(summary.route_certification_ready);
        assert!(summary.route_validation_ready);
        assert!(summary.route_verification_ready);
        assert!(summary.route_publication_ready);
        assert!(summary.route_completion_ready);
        assert!(summary.route_signoff_ready);
        assert!(summary.route_audit_ready);
        assert!(summary.route_handoff_ready);
        assert!(summary.attach_complete);
        assert!(summary.network_data_ready);
        assert!(summary.routing_surface_ready);
        assert!(summary.parent_or_route_anchor_ready);
        assert!(summary.route_adoption_ready);
        assert!(summary.is_route_adoption_ready());
        assert!(!summary.has_adoption_gaps());
        assert!(!summary.needs_route_rollout());
        assert!(!summary.needs_route_activation());
        assert!(!summary.needs_route_approval());
        assert!(!summary.needs_route_certification());
        assert!(!summary.needs_route_validation());
        assert!(!summary.needs_route_verification());
        assert!(!summary.needs_route_publication());
        assert!(!summary.needs_route_completion());
        assert!(!summary.needs_route_signoff());
        assert!(!summary.needs_route_audit());
        assert!(!summary.needs_route_handoff());
        assert!(!summary.needs_attach_completion());
        assert!(!summary.needs_network_data());
        assert!(!summary.needs_routing_surface());
        assert!(!summary.needs_parent_or_route_anchor());
    }

    #[test]
    fn attach_route_adoption_summary_routes_blocked_adoption() {
        let table = NeighborTable::new(DeviceRole::Child);
        let action_summary = ThreadAttachActionSummary::from_summaries(
            MleMessageBatchSummary::empty(),
            table.summary_at(1_250),
        );
        let completion_summary = summarize_thread_attach_completion(
            action_summary,
            table
                .diagnostic_snapshot(None, 1_250)
                .unwrap()
                .supervision_plan(),
        );
        let unknown = NetworkDataTlv::new(NetworkDataTlvType::Unknown(42), false, vec![3]).unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![unknown]).unwrap();
        let network_data_readiness = network_data.summary().unwrap().readiness();
        let handoff_summary = ThreadAttachRouteHandoffSummary::from_completion_and_network_data(
            completion_summary,
            network_data_readiness,
        );
        let audit_summary = ThreadAttachRouteAuditSummary::from_handoff_summary(handoff_summary);
        let signoff_summary = ThreadAttachRouteSignoffSummary::from_audit_summary(audit_summary);
        let completion_summary =
            ThreadAttachRouteCompletionSummary::from_signoff_summary(signoff_summary);
        let publication_summary =
            ThreadAttachRoutePublicationSummary::from_completion_summary(completion_summary);
        let verification_summary =
            ThreadAttachRouteVerificationSummary::from_publication_summary(publication_summary);
        let validation_summary =
            ThreadAttachRouteValidationSummary::from_verification_summary(verification_summary);
        let certification_summary =
            ThreadAttachRouteCertificationSummary::from_validation_summary(validation_summary);
        let approval_summary =
            ThreadAttachRouteApprovalSummary::from_certification_summary(certification_summary);
        let activation_summary =
            ThreadAttachRouteActivationSummary::from_approval_summary(approval_summary);
        let rollout_summary =
            ThreadAttachRouteRolloutSummary::from_activation_summary(activation_summary);

        let summary = ThreadAttachRouteAdoptionSummary::from_rollout_summary(rollout_summary);

        assert_eq!(summary.required_route_adoption_check_count, 15);
        assert_eq!(summary.passed_route_adoption_check_count, 0);
        assert_eq!(summary.missing_route_adoption_check_count, 15);
        assert!(!summary.route_rollout_ready);
        assert!(!summary.route_activation_ready);
        assert!(!summary.route_approval_ready);
        assert!(!summary.route_certification_ready);
        assert!(!summary.route_validation_ready);
        assert!(!summary.route_verification_ready);
        assert!(!summary.route_publication_ready);
        assert!(!summary.route_completion_ready);
        assert!(!summary.route_signoff_ready);
        assert!(!summary.route_audit_ready);
        assert!(!summary.route_handoff_ready);
        assert!(!summary.attach_complete);
        assert!(!summary.network_data_ready);
        assert!(!summary.routing_surface_ready);
        assert!(!summary.parent_or_route_anchor_ready);
        assert!(!summary.route_adoption_ready);
        assert!(!summary.is_route_adoption_ready());
        assert!(summary.has_adoption_gaps());
        assert!(summary.needs_route_rollout());
        assert!(summary.needs_route_activation());
        assert!(summary.needs_route_approval());
        assert!(summary.needs_route_certification());
        assert!(summary.needs_route_validation());
        assert!(summary.needs_route_verification());
        assert!(summary.needs_route_publication());
        assert!(summary.needs_route_completion());
        assert!(summary.needs_route_signoff());
        assert!(summary.needs_route_audit());
        assert!(summary.needs_route_handoff());
        assert!(summary.needs_attach_completion());
        assert!(summary.needs_network_data());
        assert!(summary.needs_routing_surface());
        assert!(summary.needs_parent_or_route_anchor());
    }

    #[test]
    fn attach_route_acceptance_summary_marks_ready_route_acceptance() {
        let mut table = NeighborTable::new(DeviceRole::Child);
        table.upsert(ThreadNeighbor::new(
            ThreadNeighborId(0x1000),
            DeviceRole::Router,
            NeighborRelationship::Parent,
            1_200,
            10_000,
        ));
        let action_summary = ThreadAttachActionSummary::from_summaries(
            MleMessageBatchSummary::empty(),
            table.summary_at(1_250),
        );
        let completion_summary = summarize_thread_attach_completion(
            action_summary,
            table
                .diagnostic_snapshot(None, 1_250)
                .unwrap()
                .supervision_plan(),
        );
        let border_router =
            NetworkDataTlv::new(NetworkDataTlvType::BorderRouter, true, vec![0xaa]).unwrap();
        let context = NetworkDataTlv::new(NetworkDataTlvType::Context, true, vec![0x01]).unwrap();
        let prefix = ThreadPrefixData::new(
            true,
            3,
            64,
            vec![0xfd, 0x00, 0xab, 0xcd, 0, 0, 0, 0],
            vec![border_router, context],
        )
        .unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![prefix.to_tlv().unwrap()]).unwrap();
        let network_data_readiness =
            summarize_thread_network_data_readiness(&network_data).unwrap();
        let handoff_summary =
            summarize_thread_attach_route_handoff(completion_summary, network_data_readiness);
        let audit_summary = summarize_thread_attach_route_audit(handoff_summary);
        let signoff_summary = summarize_thread_attach_route_signoff(audit_summary);
        let completion_summary = summarize_thread_attach_route_completion(signoff_summary);
        let publication_summary = summarize_thread_attach_route_publication(completion_summary);
        let verification_summary = summarize_thread_attach_route_verification(publication_summary);
        let validation_summary = summarize_thread_attach_route_validation(verification_summary);
        let certification_summary = summarize_thread_attach_route_certification(validation_summary);
        let approval_summary = summarize_thread_attach_route_approval(certification_summary);
        let activation_summary = summarize_thread_attach_route_activation(approval_summary);
        let rollout_summary = summarize_thread_attach_route_rollout(activation_summary);
        let adoption_summary = summarize_thread_attach_route_adoption(rollout_summary);

        let summary = summarize_thread_attach_route_acceptance(adoption_summary);

        assert_eq!(summary.adoption_summary, adoption_summary);
        assert_eq!(summary.required_route_acceptance_check_count, 16);
        assert_eq!(summary.passed_route_acceptance_check_count, 16);
        assert_eq!(summary.missing_route_acceptance_check_count, 0);
        assert!(summary.route_adoption_ready);
        assert!(summary.route_rollout_ready);
        assert!(summary.route_activation_ready);
        assert!(summary.route_approval_ready);
        assert!(summary.route_certification_ready);
        assert!(summary.route_validation_ready);
        assert!(summary.route_verification_ready);
        assert!(summary.route_publication_ready);
        assert!(summary.route_completion_ready);
        assert!(summary.route_signoff_ready);
        assert!(summary.route_audit_ready);
        assert!(summary.route_handoff_ready);
        assert!(summary.attach_complete);
        assert!(summary.network_data_ready);
        assert!(summary.routing_surface_ready);
        assert!(summary.parent_or_route_anchor_ready);
        assert!(summary.route_acceptance_ready);
        assert!(summary.is_route_acceptance_ready());
        assert!(!summary.has_acceptance_gaps());
        assert!(!summary.needs_route_adoption());
        assert!(!summary.needs_route_rollout());
        assert!(!summary.needs_route_activation());
        assert!(!summary.needs_route_approval());
        assert!(!summary.needs_route_certification());
        assert!(!summary.needs_route_validation());
        assert!(!summary.needs_route_verification());
        assert!(!summary.needs_route_publication());
        assert!(!summary.needs_route_completion());
        assert!(!summary.needs_route_signoff());
        assert!(!summary.needs_route_audit());
        assert!(!summary.needs_route_handoff());
        assert!(!summary.needs_attach_completion());
        assert!(!summary.needs_network_data());
        assert!(!summary.needs_routing_surface());
        assert!(!summary.needs_parent_or_route_anchor());
    }

    #[test]
    fn attach_route_acceptance_summary_routes_blocked_acceptance() {
        let table = NeighborTable::new(DeviceRole::Child);
        let action_summary = ThreadAttachActionSummary::from_summaries(
            MleMessageBatchSummary::empty(),
            table.summary_at(1_250),
        );
        let completion_summary = summarize_thread_attach_completion(
            action_summary,
            table
                .diagnostic_snapshot(None, 1_250)
                .unwrap()
                .supervision_plan(),
        );
        let unknown = NetworkDataTlv::new(NetworkDataTlvType::Unknown(42), false, vec![3]).unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![unknown]).unwrap();
        let network_data_readiness = network_data.summary().unwrap().readiness();
        let handoff_summary = ThreadAttachRouteHandoffSummary::from_completion_and_network_data(
            completion_summary,
            network_data_readiness,
        );
        let audit_summary = ThreadAttachRouteAuditSummary::from_handoff_summary(handoff_summary);
        let signoff_summary = ThreadAttachRouteSignoffSummary::from_audit_summary(audit_summary);
        let completion_summary =
            ThreadAttachRouteCompletionSummary::from_signoff_summary(signoff_summary);
        let publication_summary =
            ThreadAttachRoutePublicationSummary::from_completion_summary(completion_summary);
        let verification_summary =
            ThreadAttachRouteVerificationSummary::from_publication_summary(publication_summary);
        let validation_summary =
            ThreadAttachRouteValidationSummary::from_verification_summary(verification_summary);
        let certification_summary =
            ThreadAttachRouteCertificationSummary::from_validation_summary(validation_summary);
        let approval_summary =
            ThreadAttachRouteApprovalSummary::from_certification_summary(certification_summary);
        let activation_summary =
            ThreadAttachRouteActivationSummary::from_approval_summary(approval_summary);
        let rollout_summary =
            ThreadAttachRouteRolloutSummary::from_activation_summary(activation_summary);
        let adoption_summary =
            ThreadAttachRouteAdoptionSummary::from_rollout_summary(rollout_summary);

        let summary = ThreadAttachRouteAcceptanceSummary::from_adoption_summary(adoption_summary);

        assert_eq!(summary.required_route_acceptance_check_count, 16);
        assert_eq!(summary.passed_route_acceptance_check_count, 0);
        assert_eq!(summary.missing_route_acceptance_check_count, 16);
        assert!(!summary.route_adoption_ready);
        assert!(!summary.route_rollout_ready);
        assert!(!summary.route_activation_ready);
        assert!(!summary.route_approval_ready);
        assert!(!summary.route_certification_ready);
        assert!(!summary.route_validation_ready);
        assert!(!summary.route_verification_ready);
        assert!(!summary.route_publication_ready);
        assert!(!summary.route_completion_ready);
        assert!(!summary.route_signoff_ready);
        assert!(!summary.route_audit_ready);
        assert!(!summary.route_handoff_ready);
        assert!(!summary.attach_complete);
        assert!(!summary.network_data_ready);
        assert!(!summary.routing_surface_ready);
        assert!(!summary.parent_or_route_anchor_ready);
        assert!(!summary.route_acceptance_ready);
        assert!(!summary.is_route_acceptance_ready());
        assert!(summary.has_acceptance_gaps());
        assert!(summary.needs_route_adoption());
        assert!(summary.needs_route_rollout());
        assert!(summary.needs_route_activation());
        assert!(summary.needs_route_approval());
        assert!(summary.needs_route_certification());
        assert!(summary.needs_route_validation());
        assert!(summary.needs_route_verification());
        assert!(summary.needs_route_publication());
        assert!(summary.needs_route_completion());
        assert!(summary.needs_route_signoff());
        assert!(summary.needs_route_audit());
        assert!(summary.needs_route_handoff());
        assert!(summary.needs_attach_completion());
        assert!(summary.needs_network_data());
        assert!(summary.needs_routing_surface());
        assert!(summary.needs_parent_or_route_anchor());
    }

    #[test]
    fn attach_route_distribution_summary_marks_ready_route_distribution() {
        let mut table = NeighborTable::new(DeviceRole::Child);
        table.upsert(ThreadNeighbor::new(
            ThreadNeighborId(0x1000),
            DeviceRole::Router,
            NeighborRelationship::Parent,
            1_200,
            10_000,
        ));
        let action_summary = ThreadAttachActionSummary::from_summaries(
            MleMessageBatchSummary::empty(),
            table.summary_at(1_250),
        );
        let completion_summary = summarize_thread_attach_completion(
            action_summary,
            table
                .diagnostic_snapshot(None, 1_250)
                .unwrap()
                .supervision_plan(),
        );
        let border_router =
            NetworkDataTlv::new(NetworkDataTlvType::BorderRouter, true, vec![0xaa]).unwrap();
        let context = NetworkDataTlv::new(NetworkDataTlvType::Context, true, vec![0x01]).unwrap();
        let prefix = ThreadPrefixData::new(
            true,
            3,
            64,
            vec![0xfd, 0x00, 0xab, 0xcd, 0, 0, 0, 0],
            vec![border_router, context],
        )
        .unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![prefix.to_tlv().unwrap()]).unwrap();
        let network_data_readiness =
            summarize_thread_network_data_readiness(&network_data).unwrap();
        let handoff_summary =
            summarize_thread_attach_route_handoff(completion_summary, network_data_readiness);
        let audit_summary = summarize_thread_attach_route_audit(handoff_summary);
        let signoff_summary = summarize_thread_attach_route_signoff(audit_summary);
        let completion_summary = summarize_thread_attach_route_completion(signoff_summary);
        let publication_summary = summarize_thread_attach_route_publication(completion_summary);
        let verification_summary = summarize_thread_attach_route_verification(publication_summary);
        let validation_summary = summarize_thread_attach_route_validation(verification_summary);
        let certification_summary = summarize_thread_attach_route_certification(validation_summary);
        let approval_summary = summarize_thread_attach_route_approval(certification_summary);
        let activation_summary = summarize_thread_attach_route_activation(approval_summary);
        let rollout_summary = summarize_thread_attach_route_rollout(activation_summary);
        let adoption_summary = summarize_thread_attach_route_adoption(rollout_summary);
        let acceptance_summary = summarize_thread_attach_route_acceptance(adoption_summary);

        let summary = summarize_thread_attach_route_distribution(acceptance_summary);

        assert_eq!(summary.acceptance_summary, acceptance_summary);
        assert_eq!(summary.required_route_distribution_check_count, 17);
        assert_eq!(summary.passed_route_distribution_check_count, 17);
        assert_eq!(summary.missing_route_distribution_check_count, 0);
        assert!(summary.route_acceptance_ready);
        assert!(summary.route_adoption_ready);
        assert!(summary.route_rollout_ready);
        assert!(summary.route_activation_ready);
        assert!(summary.route_approval_ready);
        assert!(summary.route_certification_ready);
        assert!(summary.route_validation_ready);
        assert!(summary.route_verification_ready);
        assert!(summary.route_publication_ready);
        assert!(summary.route_completion_ready);
        assert!(summary.route_signoff_ready);
        assert!(summary.route_audit_ready);
        assert!(summary.route_handoff_ready);
        assert!(summary.attach_complete);
        assert!(summary.network_data_ready);
        assert!(summary.routing_surface_ready);
        assert!(summary.parent_or_route_anchor_ready);
        assert!(summary.route_distribution_ready);
        assert!(summary.is_route_distribution_ready());
        assert!(!summary.has_distribution_gaps());
        assert!(!summary.needs_route_acceptance());
        assert!(!summary.needs_route_adoption());
        assert!(!summary.needs_route_rollout());
        assert!(!summary.needs_route_activation());
        assert!(!summary.needs_route_approval());
        assert!(!summary.needs_route_certification());
        assert!(!summary.needs_route_validation());
        assert!(!summary.needs_route_verification());
        assert!(!summary.needs_route_publication());
        assert!(!summary.needs_route_completion());
        assert!(!summary.needs_route_signoff());
        assert!(!summary.needs_route_audit());
        assert!(!summary.needs_route_handoff());
        assert!(!summary.needs_attach_completion());
        assert!(!summary.needs_network_data());
        assert!(!summary.needs_routing_surface());
        assert!(!summary.needs_parent_or_route_anchor());
    }

    #[test]
    fn attach_route_distribution_summary_routes_blocked_distribution() {
        let table = NeighborTable::new(DeviceRole::Child);
        let action_summary = ThreadAttachActionSummary::from_summaries(
            MleMessageBatchSummary::empty(),
            table.summary_at(1_250),
        );
        let completion_summary = summarize_thread_attach_completion(
            action_summary,
            table
                .diagnostic_snapshot(None, 1_250)
                .unwrap()
                .supervision_plan(),
        );
        let unknown = NetworkDataTlv::new(NetworkDataTlvType::Unknown(42), false, vec![3]).unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![unknown]).unwrap();
        let network_data_readiness = network_data.summary().unwrap().readiness();
        let handoff_summary = ThreadAttachRouteHandoffSummary::from_completion_and_network_data(
            completion_summary,
            network_data_readiness,
        );
        let audit_summary = ThreadAttachRouteAuditSummary::from_handoff_summary(handoff_summary);
        let signoff_summary = ThreadAttachRouteSignoffSummary::from_audit_summary(audit_summary);
        let completion_summary =
            ThreadAttachRouteCompletionSummary::from_signoff_summary(signoff_summary);
        let publication_summary =
            ThreadAttachRoutePublicationSummary::from_completion_summary(completion_summary);
        let verification_summary =
            ThreadAttachRouteVerificationSummary::from_publication_summary(publication_summary);
        let validation_summary =
            ThreadAttachRouteValidationSummary::from_verification_summary(verification_summary);
        let certification_summary =
            ThreadAttachRouteCertificationSummary::from_validation_summary(validation_summary);
        let approval_summary =
            ThreadAttachRouteApprovalSummary::from_certification_summary(certification_summary);
        let activation_summary =
            ThreadAttachRouteActivationSummary::from_approval_summary(approval_summary);
        let rollout_summary =
            ThreadAttachRouteRolloutSummary::from_activation_summary(activation_summary);
        let adoption_summary =
            ThreadAttachRouteAdoptionSummary::from_rollout_summary(rollout_summary);
        let acceptance_summary =
            ThreadAttachRouteAcceptanceSummary::from_adoption_summary(adoption_summary);

        let summary =
            ThreadAttachRouteDistributionSummary::from_acceptance_summary(acceptance_summary);

        assert_eq!(summary.required_route_distribution_check_count, 17);
        assert_eq!(summary.passed_route_distribution_check_count, 0);
        assert_eq!(summary.missing_route_distribution_check_count, 17);
        assert!(!summary.route_acceptance_ready);
        assert!(!summary.route_adoption_ready);
        assert!(!summary.route_rollout_ready);
        assert!(!summary.route_activation_ready);
        assert!(!summary.route_approval_ready);
        assert!(!summary.route_certification_ready);
        assert!(!summary.route_validation_ready);
        assert!(!summary.route_verification_ready);
        assert!(!summary.route_publication_ready);
        assert!(!summary.route_completion_ready);
        assert!(!summary.route_signoff_ready);
        assert!(!summary.route_audit_ready);
        assert!(!summary.route_handoff_ready);
        assert!(!summary.attach_complete);
        assert!(!summary.network_data_ready);
        assert!(!summary.routing_surface_ready);
        assert!(!summary.parent_or_route_anchor_ready);
        assert!(!summary.route_distribution_ready);
        assert!(!summary.is_route_distribution_ready());
        assert!(summary.has_distribution_gaps());
        assert!(summary.needs_route_acceptance());
        assert!(summary.needs_route_adoption());
        assert!(summary.needs_route_rollout());
        assert!(summary.needs_route_activation());
        assert!(summary.needs_route_approval());
        assert!(summary.needs_route_certification());
        assert!(summary.needs_route_validation());
        assert!(summary.needs_route_verification());
        assert!(summary.needs_route_publication());
        assert!(summary.needs_route_completion());
        assert!(summary.needs_route_signoff());
        assert!(summary.needs_route_audit());
        assert!(summary.needs_route_handoff());
        assert!(summary.needs_attach_completion());
        assert!(summary.needs_network_data());
        assert!(summary.needs_routing_surface());
        assert!(summary.needs_parent_or_route_anchor());
    }

    #[test]
    fn attach_route_export_summary_marks_ready_route_export() {
        let mut table = NeighborTable::new(DeviceRole::Child);
        table.upsert(ThreadNeighbor::new(
            ThreadNeighborId(0x1000),
            DeviceRole::Router,
            NeighborRelationship::Parent,
            1_200,
            10_000,
        ));
        let action_summary = ThreadAttachActionSummary::from_summaries(
            MleMessageBatchSummary::empty(),
            table.summary_at(1_250),
        );
        let completion_summary = summarize_thread_attach_completion(
            action_summary,
            table
                .diagnostic_snapshot(None, 1_250)
                .unwrap()
                .supervision_plan(),
        );
        let border_router =
            NetworkDataTlv::new(NetworkDataTlvType::BorderRouter, true, vec![0xaa]).unwrap();
        let context = NetworkDataTlv::new(NetworkDataTlvType::Context, true, vec![0x01]).unwrap();
        let prefix = ThreadPrefixData::new(
            true,
            3,
            64,
            vec![0xfd, 0x00, 0xab, 0xcd, 0, 0, 0, 0],
            vec![border_router, context],
        )
        .unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![prefix.to_tlv().unwrap()]).unwrap();
        let network_data_readiness =
            summarize_thread_network_data_readiness(&network_data).unwrap();
        let handoff_summary =
            summarize_thread_attach_route_handoff(completion_summary, network_data_readiness);
        let audit_summary = summarize_thread_attach_route_audit(handoff_summary);
        let signoff_summary = summarize_thread_attach_route_signoff(audit_summary);
        let completion_summary = summarize_thread_attach_route_completion(signoff_summary);
        let publication_summary = summarize_thread_attach_route_publication(completion_summary);
        let verification_summary = summarize_thread_attach_route_verification(publication_summary);
        let validation_summary = summarize_thread_attach_route_validation(verification_summary);
        let certification_summary = summarize_thread_attach_route_certification(validation_summary);
        let approval_summary = summarize_thread_attach_route_approval(certification_summary);
        let activation_summary = summarize_thread_attach_route_activation(approval_summary);
        let rollout_summary = summarize_thread_attach_route_rollout(activation_summary);
        let adoption_summary = summarize_thread_attach_route_adoption(rollout_summary);
        let acceptance_summary = summarize_thread_attach_route_acceptance(adoption_summary);
        let distribution_summary = summarize_thread_attach_route_distribution(acceptance_summary);

        let summary = summarize_thread_attach_route_export(distribution_summary);

        assert_eq!(summary.distribution_summary, distribution_summary);
        assert_eq!(summary.required_route_export_check_count, 18);
        assert_eq!(summary.passed_route_export_check_count, 18);
        assert_eq!(summary.missing_route_export_check_count, 0);
        assert!(summary.route_distribution_ready);
        assert!(summary.route_acceptance_ready);
        assert!(summary.route_adoption_ready);
        assert!(summary.route_rollout_ready);
        assert!(summary.route_activation_ready);
        assert!(summary.route_approval_ready);
        assert!(summary.route_certification_ready);
        assert!(summary.route_validation_ready);
        assert!(summary.route_verification_ready);
        assert!(summary.route_publication_ready);
        assert!(summary.route_completion_ready);
        assert!(summary.route_signoff_ready);
        assert!(summary.route_audit_ready);
        assert!(summary.route_handoff_ready);
        assert!(summary.attach_complete);
        assert!(summary.network_data_ready);
        assert!(summary.routing_surface_ready);
        assert!(summary.parent_or_route_anchor_ready);
        assert!(summary.route_export_ready);
        assert!(summary.is_route_export_ready());
        assert!(!summary.has_export_gaps());
        assert!(!summary.needs_route_distribution());
        assert!(!summary.needs_route_acceptance());
        assert!(!summary.needs_route_adoption());
        assert!(!summary.needs_route_rollout());
        assert!(!summary.needs_route_activation());
        assert!(!summary.needs_route_approval());
        assert!(!summary.needs_route_certification());
        assert!(!summary.needs_route_validation());
        assert!(!summary.needs_route_verification());
        assert!(!summary.needs_route_publication());
        assert!(!summary.needs_route_completion());
        assert!(!summary.needs_route_signoff());
        assert!(!summary.needs_route_audit());
        assert!(!summary.needs_route_handoff());
        assert!(!summary.needs_attach_completion());
        assert!(!summary.needs_network_data());
        assert!(!summary.needs_routing_surface());
        assert!(!summary.needs_parent_or_route_anchor());
    }

    #[test]
    fn attach_route_export_summary_routes_blocked_export() {
        let table = NeighborTable::new(DeviceRole::Child);
        let action_summary = ThreadAttachActionSummary::from_summaries(
            MleMessageBatchSummary::empty(),
            table.summary_at(1_250),
        );
        let completion_summary = summarize_thread_attach_completion(
            action_summary,
            table
                .diagnostic_snapshot(None, 1_250)
                .unwrap()
                .supervision_plan(),
        );
        let unknown = NetworkDataTlv::new(NetworkDataTlvType::Unknown(42), false, vec![3]).unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![unknown]).unwrap();
        let network_data_readiness = network_data.summary().unwrap().readiness();
        let handoff_summary = ThreadAttachRouteHandoffSummary::from_completion_and_network_data(
            completion_summary,
            network_data_readiness,
        );
        let audit_summary = ThreadAttachRouteAuditSummary::from_handoff_summary(handoff_summary);
        let signoff_summary = ThreadAttachRouteSignoffSummary::from_audit_summary(audit_summary);
        let completion_summary =
            ThreadAttachRouteCompletionSummary::from_signoff_summary(signoff_summary);
        let publication_summary =
            ThreadAttachRoutePublicationSummary::from_completion_summary(completion_summary);
        let verification_summary =
            ThreadAttachRouteVerificationSummary::from_publication_summary(publication_summary);
        let validation_summary =
            ThreadAttachRouteValidationSummary::from_verification_summary(verification_summary);
        let certification_summary =
            ThreadAttachRouteCertificationSummary::from_validation_summary(validation_summary);
        let approval_summary =
            ThreadAttachRouteApprovalSummary::from_certification_summary(certification_summary);
        let activation_summary =
            ThreadAttachRouteActivationSummary::from_approval_summary(approval_summary);
        let rollout_summary =
            ThreadAttachRouteRolloutSummary::from_activation_summary(activation_summary);
        let adoption_summary =
            ThreadAttachRouteAdoptionSummary::from_rollout_summary(rollout_summary);
        let acceptance_summary =
            ThreadAttachRouteAcceptanceSummary::from_adoption_summary(adoption_summary);
        let distribution_summary =
            ThreadAttachRouteDistributionSummary::from_acceptance_summary(acceptance_summary);

        let summary =
            ThreadAttachRouteExportSummary::from_distribution_summary(distribution_summary);

        assert_eq!(summary.required_route_export_check_count, 18);
        assert_eq!(summary.passed_route_export_check_count, 0);
        assert_eq!(summary.missing_route_export_check_count, 18);
        assert!(!summary.route_distribution_ready);
        assert!(!summary.route_acceptance_ready);
        assert!(!summary.route_adoption_ready);
        assert!(!summary.route_rollout_ready);
        assert!(!summary.route_activation_ready);
        assert!(!summary.route_approval_ready);
        assert!(!summary.route_certification_ready);
        assert!(!summary.route_validation_ready);
        assert!(!summary.route_verification_ready);
        assert!(!summary.route_publication_ready);
        assert!(!summary.route_completion_ready);
        assert!(!summary.route_signoff_ready);
        assert!(!summary.route_audit_ready);
        assert!(!summary.route_handoff_ready);
        assert!(!summary.attach_complete);
        assert!(!summary.network_data_ready);
        assert!(!summary.routing_surface_ready);
        assert!(!summary.parent_or_route_anchor_ready);
        assert!(!summary.route_export_ready);
        assert!(!summary.is_route_export_ready());
        assert!(summary.has_export_gaps());
        assert!(summary.needs_route_distribution());
        assert!(summary.needs_route_acceptance());
        assert!(summary.needs_route_adoption());
        assert!(summary.needs_route_rollout());
        assert!(summary.needs_route_activation());
        assert!(summary.needs_route_approval());
        assert!(summary.needs_route_certification());
        assert!(summary.needs_route_validation());
        assert!(summary.needs_route_verification());
        assert!(summary.needs_route_publication());
        assert!(summary.needs_route_completion());
        assert!(summary.needs_route_signoff());
        assert!(summary.needs_route_audit());
        assert!(summary.needs_route_handoff());
        assert!(summary.needs_attach_completion());
        assert!(summary.needs_network_data());
        assert!(summary.needs_routing_surface());
        assert!(summary.needs_parent_or_route_anchor());
    }

    #[test]
    fn attach_route_import_summary_marks_ready_route_import() {
        let mut table = NeighborTable::new(DeviceRole::Child);
        table.upsert(ThreadNeighbor::new(
            ThreadNeighborId(0x1000),
            DeviceRole::Router,
            NeighborRelationship::Parent,
            1_200,
            10_000,
        ));
        let action_summary = ThreadAttachActionSummary::from_summaries(
            MleMessageBatchSummary::empty(),
            table.summary_at(1_250),
        );
        let completion_summary = summarize_thread_attach_completion(
            action_summary,
            table
                .diagnostic_snapshot(None, 1_250)
                .unwrap()
                .supervision_plan(),
        );
        let border_router =
            NetworkDataTlv::new(NetworkDataTlvType::BorderRouter, true, vec![0xaa]).unwrap();
        let context = NetworkDataTlv::new(NetworkDataTlvType::Context, true, vec![0x01]).unwrap();
        let prefix = ThreadPrefixData::new(
            true,
            3,
            64,
            vec![0xfd, 0x00, 0xab, 0xcd, 0, 0, 0, 0],
            vec![border_router, context],
        )
        .unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![prefix.to_tlv().unwrap()]).unwrap();
        let network_data_readiness =
            summarize_thread_network_data_readiness(&network_data).unwrap();
        let handoff_summary =
            summarize_thread_attach_route_handoff(completion_summary, network_data_readiness);
        let audit_summary = summarize_thread_attach_route_audit(handoff_summary);
        let signoff_summary = summarize_thread_attach_route_signoff(audit_summary);
        let completion_summary = summarize_thread_attach_route_completion(signoff_summary);
        let publication_summary = summarize_thread_attach_route_publication(completion_summary);
        let verification_summary = summarize_thread_attach_route_verification(publication_summary);
        let validation_summary = summarize_thread_attach_route_validation(verification_summary);
        let certification_summary = summarize_thread_attach_route_certification(validation_summary);
        let approval_summary = summarize_thread_attach_route_approval(certification_summary);
        let activation_summary = summarize_thread_attach_route_activation(approval_summary);
        let rollout_summary = summarize_thread_attach_route_rollout(activation_summary);
        let adoption_summary = summarize_thread_attach_route_adoption(rollout_summary);
        let acceptance_summary = summarize_thread_attach_route_acceptance(adoption_summary);
        let distribution_summary = summarize_thread_attach_route_distribution(acceptance_summary);
        let export_summary = summarize_thread_attach_route_export(distribution_summary);

        let summary = summarize_thread_attach_route_import(export_summary);

        assert_eq!(summary.export_summary, export_summary);
        assert_eq!(summary.required_route_import_check_count, 19);
        assert_eq!(summary.passed_route_import_check_count, 19);
        assert_eq!(summary.missing_route_import_check_count, 0);
        assert!(summary.route_export_ready);
        assert!(summary.route_distribution_ready);
        assert!(summary.route_acceptance_ready);
        assert!(summary.route_adoption_ready);
        assert!(summary.route_rollout_ready);
        assert!(summary.route_activation_ready);
        assert!(summary.route_approval_ready);
        assert!(summary.route_certification_ready);
        assert!(summary.route_validation_ready);
        assert!(summary.route_verification_ready);
        assert!(summary.route_publication_ready);
        assert!(summary.route_completion_ready);
        assert!(summary.route_signoff_ready);
        assert!(summary.route_audit_ready);
        assert!(summary.route_handoff_ready);
        assert!(summary.attach_complete);
        assert!(summary.network_data_ready);
        assert!(summary.routing_surface_ready);
        assert!(summary.parent_or_route_anchor_ready);
        assert!(summary.route_import_ready);
        assert!(summary.is_route_import_ready());
        assert!(!summary.has_import_gaps());
        assert!(!summary.needs_route_export());
        assert!(!summary.needs_route_distribution());
        assert!(!summary.needs_route_acceptance());
        assert!(!summary.needs_route_adoption());
        assert!(!summary.needs_route_rollout());
        assert!(!summary.needs_route_activation());
        assert!(!summary.needs_route_approval());
        assert!(!summary.needs_route_certification());
        assert!(!summary.needs_route_validation());
        assert!(!summary.needs_route_verification());
        assert!(!summary.needs_route_publication());
        assert!(!summary.needs_route_completion());
        assert!(!summary.needs_route_signoff());
        assert!(!summary.needs_route_audit());
        assert!(!summary.needs_route_handoff());
        assert!(!summary.needs_attach_completion());
        assert!(!summary.needs_network_data());
        assert!(!summary.needs_routing_surface());
        assert!(!summary.needs_parent_or_route_anchor());
    }

    #[test]
    fn attach_route_import_summary_routes_blocked_import() {
        let table = NeighborTable::new(DeviceRole::Child);
        let action_summary = ThreadAttachActionSummary::from_summaries(
            MleMessageBatchSummary::empty(),
            table.summary_at(1_250),
        );
        let completion_summary = summarize_thread_attach_completion(
            action_summary,
            table
                .diagnostic_snapshot(None, 1_250)
                .unwrap()
                .supervision_plan(),
        );
        let unknown = NetworkDataTlv::new(NetworkDataTlvType::Unknown(42), false, vec![3]).unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![unknown]).unwrap();
        let network_data_readiness = network_data.summary().unwrap().readiness();
        let handoff_summary = ThreadAttachRouteHandoffSummary::from_completion_and_network_data(
            completion_summary,
            network_data_readiness,
        );
        let audit_summary = ThreadAttachRouteAuditSummary::from_handoff_summary(handoff_summary);
        let signoff_summary = ThreadAttachRouteSignoffSummary::from_audit_summary(audit_summary);
        let completion_summary =
            ThreadAttachRouteCompletionSummary::from_signoff_summary(signoff_summary);
        let publication_summary =
            ThreadAttachRoutePublicationSummary::from_completion_summary(completion_summary);
        let verification_summary =
            ThreadAttachRouteVerificationSummary::from_publication_summary(publication_summary);
        let validation_summary =
            ThreadAttachRouteValidationSummary::from_verification_summary(verification_summary);
        let certification_summary =
            ThreadAttachRouteCertificationSummary::from_validation_summary(validation_summary);
        let approval_summary =
            ThreadAttachRouteApprovalSummary::from_certification_summary(certification_summary);
        let activation_summary =
            ThreadAttachRouteActivationSummary::from_approval_summary(approval_summary);
        let rollout_summary =
            ThreadAttachRouteRolloutSummary::from_activation_summary(activation_summary);
        let adoption_summary =
            ThreadAttachRouteAdoptionSummary::from_rollout_summary(rollout_summary);
        let acceptance_summary =
            ThreadAttachRouteAcceptanceSummary::from_adoption_summary(adoption_summary);
        let distribution_summary =
            ThreadAttachRouteDistributionSummary::from_acceptance_summary(acceptance_summary);
        let export_summary =
            ThreadAttachRouteExportSummary::from_distribution_summary(distribution_summary);

        let summary = ThreadAttachRouteImportSummary::from_export_summary(export_summary);

        assert_eq!(summary.required_route_import_check_count, 19);
        assert_eq!(summary.passed_route_import_check_count, 0);
        assert_eq!(summary.missing_route_import_check_count, 19);
        assert!(!summary.route_export_ready);
        assert!(!summary.route_distribution_ready);
        assert!(!summary.route_acceptance_ready);
        assert!(!summary.route_adoption_ready);
        assert!(!summary.route_rollout_ready);
        assert!(!summary.route_activation_ready);
        assert!(!summary.route_approval_ready);
        assert!(!summary.route_certification_ready);
        assert!(!summary.route_validation_ready);
        assert!(!summary.route_verification_ready);
        assert!(!summary.route_publication_ready);
        assert!(!summary.route_completion_ready);
        assert!(!summary.route_signoff_ready);
        assert!(!summary.route_audit_ready);
        assert!(!summary.route_handoff_ready);
        assert!(!summary.attach_complete);
        assert!(!summary.network_data_ready);
        assert!(!summary.routing_surface_ready);
        assert!(!summary.parent_or_route_anchor_ready);
        assert!(!summary.route_import_ready);
        assert!(!summary.is_route_import_ready());
        assert!(summary.has_import_gaps());
        assert!(summary.needs_route_export());
        assert!(summary.needs_route_distribution());
        assert!(summary.needs_route_acceptance());
        assert!(summary.needs_route_adoption());
        assert!(summary.needs_route_rollout());
        assert!(summary.needs_route_activation());
        assert!(summary.needs_route_approval());
        assert!(summary.needs_route_certification());
        assert!(summary.needs_route_validation());
        assert!(summary.needs_route_verification());
        assert!(summary.needs_route_publication());
        assert!(summary.needs_route_completion());
        assert!(summary.needs_route_signoff());
        assert!(summary.needs_route_audit());
        assert!(summary.needs_route_handoff());
        assert!(summary.needs_attach_completion());
        assert!(summary.needs_network_data());
        assert!(summary.needs_routing_surface());
        assert!(summary.needs_parent_or_route_anchor());
    }

    #[test]
    fn attach_route_ingest_summary_marks_ready_route_ingest() {
        let mut table = NeighborTable::new(DeviceRole::Child);
        table.upsert(ThreadNeighbor::new(
            ThreadNeighborId(0x1000),
            DeviceRole::Router,
            NeighborRelationship::Parent,
            1_200,
            10_000,
        ));
        let action_summary = ThreadAttachActionSummary::from_summaries(
            MleMessageBatchSummary::empty(),
            table.summary_at(1_250),
        );
        let completion_summary = summarize_thread_attach_completion(
            action_summary,
            table
                .diagnostic_snapshot(None, 1_250)
                .unwrap()
                .supervision_plan(),
        );
        let border_router =
            NetworkDataTlv::new(NetworkDataTlvType::BorderRouter, true, vec![0xaa]).unwrap();
        let context = NetworkDataTlv::new(NetworkDataTlvType::Context, true, vec![0x01]).unwrap();
        let prefix = ThreadPrefixData::new(
            true,
            3,
            64,
            vec![0xfd, 0x00, 0xab, 0xcd, 0, 0, 0, 0],
            vec![border_router, context],
        )
        .unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![prefix.to_tlv().unwrap()]).unwrap();
        let network_data_readiness =
            summarize_thread_network_data_readiness(&network_data).unwrap();
        let handoff_summary =
            summarize_thread_attach_route_handoff(completion_summary, network_data_readiness);
        let audit_summary = summarize_thread_attach_route_audit(handoff_summary);
        let signoff_summary = summarize_thread_attach_route_signoff(audit_summary);
        let completion_summary = summarize_thread_attach_route_completion(signoff_summary);
        let publication_summary = summarize_thread_attach_route_publication(completion_summary);
        let verification_summary = summarize_thread_attach_route_verification(publication_summary);
        let validation_summary = summarize_thread_attach_route_validation(verification_summary);
        let certification_summary = summarize_thread_attach_route_certification(validation_summary);
        let approval_summary = summarize_thread_attach_route_approval(certification_summary);
        let activation_summary = summarize_thread_attach_route_activation(approval_summary);
        let rollout_summary = summarize_thread_attach_route_rollout(activation_summary);
        let adoption_summary = summarize_thread_attach_route_adoption(rollout_summary);
        let acceptance_summary = summarize_thread_attach_route_acceptance(adoption_summary);
        let distribution_summary = summarize_thread_attach_route_distribution(acceptance_summary);
        let export_summary = summarize_thread_attach_route_export(distribution_summary);
        let import_summary = summarize_thread_attach_route_import(export_summary);

        let summary = summarize_thread_attach_route_ingest(import_summary);

        assert_eq!(summary.import_summary, import_summary);
        assert_eq!(summary.required_route_ingest_check_count, 20);
        assert_eq!(summary.passed_route_ingest_check_count, 20);
        assert_eq!(summary.missing_route_ingest_check_count, 0);
        assert!(summary.route_import_ready);
        assert!(summary.route_export_ready);
        assert!(summary.route_distribution_ready);
        assert!(summary.route_acceptance_ready);
        assert!(summary.route_adoption_ready);
        assert!(summary.route_rollout_ready);
        assert!(summary.route_activation_ready);
        assert!(summary.route_approval_ready);
        assert!(summary.route_certification_ready);
        assert!(summary.route_validation_ready);
        assert!(summary.route_verification_ready);
        assert!(summary.route_publication_ready);
        assert!(summary.route_completion_ready);
        assert!(summary.route_signoff_ready);
        assert!(summary.route_audit_ready);
        assert!(summary.route_handoff_ready);
        assert!(summary.attach_complete);
        assert!(summary.network_data_ready);
        assert!(summary.routing_surface_ready);
        assert!(summary.parent_or_route_anchor_ready);
        assert!(summary.route_ingest_ready);
        assert!(summary.is_route_ingest_ready());
        assert!(!summary.has_ingest_gaps());
        assert!(!summary.needs_route_import());
        assert!(!summary.needs_route_export());
        assert!(!summary.needs_route_distribution());
        assert!(!summary.needs_route_acceptance());
        assert!(!summary.needs_route_adoption());
        assert!(!summary.needs_route_rollout());
        assert!(!summary.needs_route_activation());
        assert!(!summary.needs_route_approval());
        assert!(!summary.needs_route_certification());
        assert!(!summary.needs_route_validation());
        assert!(!summary.needs_route_verification());
        assert!(!summary.needs_route_publication());
        assert!(!summary.needs_route_completion());
        assert!(!summary.needs_route_signoff());
        assert!(!summary.needs_route_audit());
        assert!(!summary.needs_route_handoff());
        assert!(!summary.needs_attach_completion());
        assert!(!summary.needs_network_data());
        assert!(!summary.needs_routing_surface());
        assert!(!summary.needs_parent_or_route_anchor());
    }

    #[test]
    fn attach_route_ingest_summary_routes_blocked_ingest() {
        let table = NeighborTable::new(DeviceRole::Child);
        let action_summary = ThreadAttachActionSummary::from_summaries(
            MleMessageBatchSummary::empty(),
            table.summary_at(1_250),
        );
        let completion_summary = summarize_thread_attach_completion(
            action_summary,
            table
                .diagnostic_snapshot(None, 1_250)
                .unwrap()
                .supervision_plan(),
        );
        let unknown = NetworkDataTlv::new(NetworkDataTlvType::Unknown(42), false, vec![3]).unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![unknown]).unwrap();
        let network_data_readiness = network_data.summary().unwrap().readiness();
        let handoff_summary = ThreadAttachRouteHandoffSummary::from_completion_and_network_data(
            completion_summary,
            network_data_readiness,
        );
        let audit_summary = ThreadAttachRouteAuditSummary::from_handoff_summary(handoff_summary);
        let signoff_summary = ThreadAttachRouteSignoffSummary::from_audit_summary(audit_summary);
        let completion_summary =
            ThreadAttachRouteCompletionSummary::from_signoff_summary(signoff_summary);
        let publication_summary =
            ThreadAttachRoutePublicationSummary::from_completion_summary(completion_summary);
        let verification_summary =
            ThreadAttachRouteVerificationSummary::from_publication_summary(publication_summary);
        let validation_summary =
            ThreadAttachRouteValidationSummary::from_verification_summary(verification_summary);
        let certification_summary =
            ThreadAttachRouteCertificationSummary::from_validation_summary(validation_summary);
        let approval_summary =
            ThreadAttachRouteApprovalSummary::from_certification_summary(certification_summary);
        let activation_summary =
            ThreadAttachRouteActivationSummary::from_approval_summary(approval_summary);
        let rollout_summary =
            ThreadAttachRouteRolloutSummary::from_activation_summary(activation_summary);
        let adoption_summary =
            ThreadAttachRouteAdoptionSummary::from_rollout_summary(rollout_summary);
        let acceptance_summary =
            ThreadAttachRouteAcceptanceSummary::from_adoption_summary(adoption_summary);
        let distribution_summary =
            ThreadAttachRouteDistributionSummary::from_acceptance_summary(acceptance_summary);
        let export_summary =
            ThreadAttachRouteExportSummary::from_distribution_summary(distribution_summary);
        let import_summary = ThreadAttachRouteImportSummary::from_export_summary(export_summary);

        let summary = ThreadAttachRouteIngestSummary::from_import_summary(import_summary);

        assert_eq!(summary.required_route_ingest_check_count, 20);
        assert_eq!(summary.passed_route_ingest_check_count, 0);
        assert_eq!(summary.missing_route_ingest_check_count, 20);
        assert!(!summary.route_import_ready);
        assert!(!summary.route_export_ready);
        assert!(!summary.route_distribution_ready);
        assert!(!summary.route_acceptance_ready);
        assert!(!summary.route_adoption_ready);
        assert!(!summary.route_rollout_ready);
        assert!(!summary.route_activation_ready);
        assert!(!summary.route_approval_ready);
        assert!(!summary.route_certification_ready);
        assert!(!summary.route_validation_ready);
        assert!(!summary.route_verification_ready);
        assert!(!summary.route_publication_ready);
        assert!(!summary.route_completion_ready);
        assert!(!summary.route_signoff_ready);
        assert!(!summary.route_audit_ready);
        assert!(!summary.route_handoff_ready);
        assert!(!summary.attach_complete);
        assert!(!summary.network_data_ready);
        assert!(!summary.routing_surface_ready);
        assert!(!summary.parent_or_route_anchor_ready);
        assert!(!summary.route_ingest_ready);
        assert!(!summary.is_route_ingest_ready());
        assert!(summary.has_ingest_gaps());
        assert!(summary.needs_route_import());
        assert!(summary.needs_route_export());
        assert!(summary.needs_route_distribution());
        assert!(summary.needs_route_acceptance());
        assert!(summary.needs_route_adoption());
        assert!(summary.needs_route_rollout());
        assert!(summary.needs_route_activation());
        assert!(summary.needs_route_approval());
        assert!(summary.needs_route_certification());
        assert!(summary.needs_route_validation());
        assert!(summary.needs_route_verification());
        assert!(summary.needs_route_publication());
        assert!(summary.needs_route_completion());
        assert!(summary.needs_route_signoff());
        assert!(summary.needs_route_audit());
        assert!(summary.needs_route_handoff());
        assert!(summary.needs_attach_completion());
        assert!(summary.needs_network_data());
        assert!(summary.needs_routing_surface());
        assert!(summary.needs_parent_or_route_anchor());
    }

    #[test]
    fn attach_route_load_summary_marks_ready_route_load() {
        let mut table = NeighborTable::new(DeviceRole::Child);
        table.upsert(ThreadNeighbor::new(
            ThreadNeighborId(0x1000),
            DeviceRole::Router,
            NeighborRelationship::Parent,
            1_200,
            10_000,
        ));
        let action_summary = ThreadAttachActionSummary::from_summaries(
            MleMessageBatchSummary::empty(),
            table.summary_at(1_250),
        );
        let completion_summary = summarize_thread_attach_completion(
            action_summary,
            table
                .diagnostic_snapshot(None, 1_250)
                .unwrap()
                .supervision_plan(),
        );
        let border_router =
            NetworkDataTlv::new(NetworkDataTlvType::BorderRouter, true, vec![0xaa]).unwrap();
        let context = NetworkDataTlv::new(NetworkDataTlvType::Context, true, vec![0x01]).unwrap();
        let prefix = ThreadPrefixData::new(
            true,
            3,
            64,
            vec![0xfd, 0x00, 0xab, 0xcd, 0, 0, 0, 0],
            vec![border_router, context],
        )
        .unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![prefix.to_tlv().unwrap()]).unwrap();
        let network_data_readiness =
            summarize_thread_network_data_readiness(&network_data).unwrap();
        let handoff_summary =
            summarize_thread_attach_route_handoff(completion_summary, network_data_readiness);
        let audit_summary = summarize_thread_attach_route_audit(handoff_summary);
        let signoff_summary = summarize_thread_attach_route_signoff(audit_summary);
        let completion_summary = summarize_thread_attach_route_completion(signoff_summary);
        let publication_summary = summarize_thread_attach_route_publication(completion_summary);
        let verification_summary = summarize_thread_attach_route_verification(publication_summary);
        let validation_summary = summarize_thread_attach_route_validation(verification_summary);
        let certification_summary = summarize_thread_attach_route_certification(validation_summary);
        let approval_summary = summarize_thread_attach_route_approval(certification_summary);
        let activation_summary = summarize_thread_attach_route_activation(approval_summary);
        let rollout_summary = summarize_thread_attach_route_rollout(activation_summary);
        let adoption_summary = summarize_thread_attach_route_adoption(rollout_summary);
        let acceptance_summary = summarize_thread_attach_route_acceptance(adoption_summary);
        let distribution_summary = summarize_thread_attach_route_distribution(acceptance_summary);
        let export_summary = summarize_thread_attach_route_export(distribution_summary);
        let import_summary = summarize_thread_attach_route_import(export_summary);
        let ingest_summary = summarize_thread_attach_route_ingest(import_summary);

        let summary = summarize_thread_attach_route_load(ingest_summary);

        assert_eq!(summary.ingest_summary, ingest_summary);
        assert_eq!(summary.required_route_load_check_count, 21);
        assert_eq!(summary.passed_route_load_check_count, 21);
        assert_eq!(summary.missing_route_load_check_count, 0);
        assert!(summary.route_ingest_ready);
        assert!(summary.route_import_ready);
        assert!(summary.route_export_ready);
        assert!(summary.route_distribution_ready);
        assert!(summary.route_acceptance_ready);
        assert!(summary.route_adoption_ready);
        assert!(summary.route_rollout_ready);
        assert!(summary.route_activation_ready);
        assert!(summary.route_approval_ready);
        assert!(summary.route_certification_ready);
        assert!(summary.route_validation_ready);
        assert!(summary.route_verification_ready);
        assert!(summary.route_publication_ready);
        assert!(summary.route_completion_ready);
        assert!(summary.route_signoff_ready);
        assert!(summary.route_audit_ready);
        assert!(summary.route_handoff_ready);
        assert!(summary.attach_complete);
        assert!(summary.network_data_ready);
        assert!(summary.routing_surface_ready);
        assert!(summary.parent_or_route_anchor_ready);
        assert!(summary.route_load_ready);
        assert!(summary.is_route_load_ready());
        assert!(!summary.has_load_gaps());
        assert!(!summary.needs_route_ingest());
        assert!(!summary.needs_route_import());
        assert!(!summary.needs_route_export());
        assert!(!summary.needs_route_distribution());
        assert!(!summary.needs_route_acceptance());
        assert!(!summary.needs_route_adoption());
        assert!(!summary.needs_route_rollout());
        assert!(!summary.needs_route_activation());
        assert!(!summary.needs_route_approval());
        assert!(!summary.needs_route_certification());
        assert!(!summary.needs_route_validation());
        assert!(!summary.needs_route_verification());
        assert!(!summary.needs_route_publication());
        assert!(!summary.needs_route_completion());
        assert!(!summary.needs_route_signoff());
        assert!(!summary.needs_route_audit());
        assert!(!summary.needs_route_handoff());
        assert!(!summary.needs_attach_completion());
        assert!(!summary.needs_network_data());
        assert!(!summary.needs_routing_surface());
        assert!(!summary.needs_parent_or_route_anchor());
    }

    #[test]
    fn attach_route_load_summary_routes_blocked_load() {
        let table = NeighborTable::new(DeviceRole::Child);
        let action_summary = ThreadAttachActionSummary::from_summaries(
            MleMessageBatchSummary::empty(),
            table.summary_at(1_250),
        );
        let completion_summary = summarize_thread_attach_completion(
            action_summary,
            table
                .diagnostic_snapshot(None, 1_250)
                .unwrap()
                .supervision_plan(),
        );
        let unknown = NetworkDataTlv::new(NetworkDataTlvType::Unknown(42), false, vec![3]).unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![unknown]).unwrap();
        let network_data_readiness = network_data.summary().unwrap().readiness();
        let handoff_summary = ThreadAttachRouteHandoffSummary::from_completion_and_network_data(
            completion_summary,
            network_data_readiness,
        );
        let audit_summary = ThreadAttachRouteAuditSummary::from_handoff_summary(handoff_summary);
        let signoff_summary = ThreadAttachRouteSignoffSummary::from_audit_summary(audit_summary);
        let completion_summary =
            ThreadAttachRouteCompletionSummary::from_signoff_summary(signoff_summary);
        let publication_summary =
            ThreadAttachRoutePublicationSummary::from_completion_summary(completion_summary);
        let verification_summary =
            ThreadAttachRouteVerificationSummary::from_publication_summary(publication_summary);
        let validation_summary =
            ThreadAttachRouteValidationSummary::from_verification_summary(verification_summary);
        let certification_summary =
            ThreadAttachRouteCertificationSummary::from_validation_summary(validation_summary);
        let approval_summary =
            ThreadAttachRouteApprovalSummary::from_certification_summary(certification_summary);
        let activation_summary =
            ThreadAttachRouteActivationSummary::from_approval_summary(approval_summary);
        let rollout_summary =
            ThreadAttachRouteRolloutSummary::from_activation_summary(activation_summary);
        let adoption_summary =
            ThreadAttachRouteAdoptionSummary::from_rollout_summary(rollout_summary);
        let acceptance_summary =
            ThreadAttachRouteAcceptanceSummary::from_adoption_summary(adoption_summary);
        let distribution_summary =
            ThreadAttachRouteDistributionSummary::from_acceptance_summary(acceptance_summary);
        let export_summary =
            ThreadAttachRouteExportSummary::from_distribution_summary(distribution_summary);
        let import_summary = ThreadAttachRouteImportSummary::from_export_summary(export_summary);
        let ingest_summary = ThreadAttachRouteIngestSummary::from_import_summary(import_summary);

        let summary = ThreadAttachRouteLoadSummary::from_ingest_summary(ingest_summary);

        assert_eq!(summary.required_route_load_check_count, 21);
        assert_eq!(summary.passed_route_load_check_count, 0);
        assert_eq!(summary.missing_route_load_check_count, 21);
        assert!(!summary.route_ingest_ready);
        assert!(!summary.route_import_ready);
        assert!(!summary.route_export_ready);
        assert!(!summary.route_distribution_ready);
        assert!(!summary.route_acceptance_ready);
        assert!(!summary.route_adoption_ready);
        assert!(!summary.route_rollout_ready);
        assert!(!summary.route_activation_ready);
        assert!(!summary.route_approval_ready);
        assert!(!summary.route_certification_ready);
        assert!(!summary.route_validation_ready);
        assert!(!summary.route_verification_ready);
        assert!(!summary.route_publication_ready);
        assert!(!summary.route_completion_ready);
        assert!(!summary.route_signoff_ready);
        assert!(!summary.route_audit_ready);
        assert!(!summary.route_handoff_ready);
        assert!(!summary.attach_complete);
        assert!(!summary.network_data_ready);
        assert!(!summary.routing_surface_ready);
        assert!(!summary.parent_or_route_anchor_ready);
        assert!(!summary.route_load_ready);
        assert!(!summary.is_route_load_ready());
        assert!(summary.has_load_gaps());
        assert!(summary.needs_route_ingest());
        assert!(summary.needs_route_import());
        assert!(summary.needs_route_export());
        assert!(summary.needs_route_distribution());
        assert!(summary.needs_route_acceptance());
        assert!(summary.needs_route_adoption());
        assert!(summary.needs_route_rollout());
        assert!(summary.needs_route_activation());
        assert!(summary.needs_route_approval());
        assert!(summary.needs_route_certification());
        assert!(summary.needs_route_validation());
        assert!(summary.needs_route_verification());
        assert!(summary.needs_route_publication());
        assert!(summary.needs_route_completion());
        assert!(summary.needs_route_signoff());
        assert!(summary.needs_route_audit());
        assert!(summary.needs_route_handoff());
        assert!(summary.needs_attach_completion());
        assert!(summary.needs_network_data());
        assert!(summary.needs_routing_surface());
        assert!(summary.needs_parent_or_route_anchor());
    }

    #[test]
    fn attach_route_restore_summary_marks_ready_route_restore() {
        let mut table = NeighborTable::new(DeviceRole::Child);
        table.upsert(ThreadNeighbor::new(
            ThreadNeighborId(0x1000),
            DeviceRole::Router,
            NeighborRelationship::Parent,
            1_200,
            10_000,
        ));
        let action_summary = ThreadAttachActionSummary::from_summaries(
            MleMessageBatchSummary::empty(),
            table.summary_at(1_250),
        );
        let completion_summary = summarize_thread_attach_completion(
            action_summary,
            table
                .diagnostic_snapshot(None, 1_250)
                .unwrap()
                .supervision_plan(),
        );
        let border_router =
            NetworkDataTlv::new(NetworkDataTlvType::BorderRouter, true, vec![0xaa]).unwrap();
        let context = NetworkDataTlv::new(NetworkDataTlvType::Context, true, vec![0x01]).unwrap();
        let prefix = ThreadPrefixData::new(
            true,
            3,
            64,
            vec![0xfd, 0x00, 0xab, 0xcd, 0, 0, 0, 0],
            vec![border_router, context],
        )
        .unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![prefix.to_tlv().unwrap()]).unwrap();
        let network_data_readiness =
            summarize_thread_network_data_readiness(&network_data).unwrap();
        let handoff_summary =
            summarize_thread_attach_route_handoff(completion_summary, network_data_readiness);
        let audit_summary = summarize_thread_attach_route_audit(handoff_summary);
        let signoff_summary = summarize_thread_attach_route_signoff(audit_summary);
        let completion_summary = summarize_thread_attach_route_completion(signoff_summary);
        let publication_summary = summarize_thread_attach_route_publication(completion_summary);
        let verification_summary = summarize_thread_attach_route_verification(publication_summary);
        let validation_summary = summarize_thread_attach_route_validation(verification_summary);
        let certification_summary = summarize_thread_attach_route_certification(validation_summary);
        let approval_summary = summarize_thread_attach_route_approval(certification_summary);
        let activation_summary = summarize_thread_attach_route_activation(approval_summary);
        let rollout_summary = summarize_thread_attach_route_rollout(activation_summary);
        let adoption_summary = summarize_thread_attach_route_adoption(rollout_summary);
        let acceptance_summary = summarize_thread_attach_route_acceptance(adoption_summary);
        let distribution_summary = summarize_thread_attach_route_distribution(acceptance_summary);
        let export_summary = summarize_thread_attach_route_export(distribution_summary);
        let import_summary = summarize_thread_attach_route_import(export_summary);
        let ingest_summary = summarize_thread_attach_route_ingest(import_summary);
        let load_summary = summarize_thread_attach_route_load(ingest_summary);

        let summary = summarize_thread_attach_route_restore(load_summary);

        assert_eq!(summary.load_summary, load_summary);
        assert_eq!(summary.required_route_restore_check_count, 22);
        assert_eq!(summary.passed_route_restore_check_count, 22);
        assert_eq!(summary.missing_route_restore_check_count, 0);
        assert!(summary.route_load_ready);
        assert!(summary.route_ingest_ready);
        assert!(summary.route_import_ready);
        assert!(summary.route_export_ready);
        assert!(summary.route_distribution_ready);
        assert!(summary.route_acceptance_ready);
        assert!(summary.route_adoption_ready);
        assert!(summary.route_rollout_ready);
        assert!(summary.route_activation_ready);
        assert!(summary.route_approval_ready);
        assert!(summary.route_certification_ready);
        assert!(summary.route_validation_ready);
        assert!(summary.route_verification_ready);
        assert!(summary.route_publication_ready);
        assert!(summary.route_completion_ready);
        assert!(summary.route_signoff_ready);
        assert!(summary.route_audit_ready);
        assert!(summary.route_handoff_ready);
        assert!(summary.attach_complete);
        assert!(summary.network_data_ready);
        assert!(summary.routing_surface_ready);
        assert!(summary.parent_or_route_anchor_ready);
        assert!(summary.route_restore_ready);
        assert!(summary.is_route_restore_ready());
        assert!(!summary.has_restore_gaps());
        assert!(!summary.needs_route_load());
        assert!(!summary.needs_route_ingest());
        assert!(!summary.needs_route_import());
        assert!(!summary.needs_route_export());
        assert!(!summary.needs_route_distribution());
        assert!(!summary.needs_route_acceptance());
        assert!(!summary.needs_route_adoption());
        assert!(!summary.needs_route_rollout());
        assert!(!summary.needs_route_activation());
        assert!(!summary.needs_route_approval());
        assert!(!summary.needs_route_certification());
        assert!(!summary.needs_route_validation());
        assert!(!summary.needs_route_verification());
        assert!(!summary.needs_route_publication());
        assert!(!summary.needs_route_completion());
        assert!(!summary.needs_route_signoff());
        assert!(!summary.needs_route_audit());
        assert!(!summary.needs_route_handoff());
        assert!(!summary.needs_attach_completion());
        assert!(!summary.needs_network_data());
        assert!(!summary.needs_routing_surface());
        assert!(!summary.needs_parent_or_route_anchor());
    }

    #[test]
    fn attach_route_restore_summary_routes_blocked_restore() {
        let table = NeighborTable::new(DeviceRole::Child);
        let action_summary = ThreadAttachActionSummary::from_summaries(
            MleMessageBatchSummary::empty(),
            table.summary_at(1_250),
        );
        let completion_summary = summarize_thread_attach_completion(
            action_summary,
            table
                .diagnostic_snapshot(None, 1_250)
                .unwrap()
                .supervision_plan(),
        );
        let unknown = NetworkDataTlv::new(NetworkDataTlvType::Unknown(42), false, vec![3]).unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![unknown]).unwrap();
        let network_data_readiness = network_data.summary().unwrap().readiness();
        let handoff_summary = ThreadAttachRouteHandoffSummary::from_completion_and_network_data(
            completion_summary,
            network_data_readiness,
        );
        let audit_summary = ThreadAttachRouteAuditSummary::from_handoff_summary(handoff_summary);
        let signoff_summary = ThreadAttachRouteSignoffSummary::from_audit_summary(audit_summary);
        let completion_summary =
            ThreadAttachRouteCompletionSummary::from_signoff_summary(signoff_summary);
        let publication_summary =
            ThreadAttachRoutePublicationSummary::from_completion_summary(completion_summary);
        let verification_summary =
            ThreadAttachRouteVerificationSummary::from_publication_summary(publication_summary);
        let validation_summary =
            ThreadAttachRouteValidationSummary::from_verification_summary(verification_summary);
        let certification_summary =
            ThreadAttachRouteCertificationSummary::from_validation_summary(validation_summary);
        let approval_summary =
            ThreadAttachRouteApprovalSummary::from_certification_summary(certification_summary);
        let activation_summary =
            ThreadAttachRouteActivationSummary::from_approval_summary(approval_summary);
        let rollout_summary =
            ThreadAttachRouteRolloutSummary::from_activation_summary(activation_summary);
        let adoption_summary =
            ThreadAttachRouteAdoptionSummary::from_rollout_summary(rollout_summary);
        let acceptance_summary =
            ThreadAttachRouteAcceptanceSummary::from_adoption_summary(adoption_summary);
        let distribution_summary =
            ThreadAttachRouteDistributionSummary::from_acceptance_summary(acceptance_summary);
        let export_summary =
            ThreadAttachRouteExportSummary::from_distribution_summary(distribution_summary);
        let import_summary = ThreadAttachRouteImportSummary::from_export_summary(export_summary);
        let ingest_summary = ThreadAttachRouteIngestSummary::from_import_summary(import_summary);
        let load_summary = ThreadAttachRouteLoadSummary::from_ingest_summary(ingest_summary);

        let summary = ThreadAttachRouteRestoreSummary::from_load_summary(load_summary);

        assert_eq!(summary.required_route_restore_check_count, 22);
        assert_eq!(summary.passed_route_restore_check_count, 0);
        assert_eq!(summary.missing_route_restore_check_count, 22);
        assert!(!summary.route_load_ready);
        assert!(!summary.route_ingest_ready);
        assert!(!summary.route_import_ready);
        assert!(!summary.route_export_ready);
        assert!(!summary.route_distribution_ready);
        assert!(!summary.route_acceptance_ready);
        assert!(!summary.route_adoption_ready);
        assert!(!summary.route_rollout_ready);
        assert!(!summary.route_activation_ready);
        assert!(!summary.route_approval_ready);
        assert!(!summary.route_certification_ready);
        assert!(!summary.route_validation_ready);
        assert!(!summary.route_verification_ready);
        assert!(!summary.route_publication_ready);
        assert!(!summary.route_completion_ready);
        assert!(!summary.route_signoff_ready);
        assert!(!summary.route_audit_ready);
        assert!(!summary.route_handoff_ready);
        assert!(!summary.attach_complete);
        assert!(!summary.network_data_ready);
        assert!(!summary.routing_surface_ready);
        assert!(!summary.parent_or_route_anchor_ready);
        assert!(!summary.route_restore_ready);
        assert!(!summary.is_route_restore_ready());
        assert!(summary.has_restore_gaps());
        assert!(summary.needs_route_load());
        assert!(summary.needs_route_ingest());
        assert!(summary.needs_route_import());
        assert!(summary.needs_route_export());
        assert!(summary.needs_route_distribution());
        assert!(summary.needs_route_acceptance());
        assert!(summary.needs_route_adoption());
        assert!(summary.needs_route_rollout());
        assert!(summary.needs_route_activation());
        assert!(summary.needs_route_approval());
        assert!(summary.needs_route_certification());
        assert!(summary.needs_route_validation());
        assert!(summary.needs_route_verification());
        assert!(summary.needs_route_publication());
        assert!(summary.needs_route_completion());
        assert!(summary.needs_route_signoff());
        assert!(summary.needs_route_audit());
        assert!(summary.needs_route_handoff());
        assert!(summary.needs_attach_completion());
        assert!(summary.needs_network_data());
        assert!(summary.needs_routing_surface());
        assert!(summary.needs_parent_or_route_anchor());
    }

    #[test]
    fn attach_route_recovery_summary_marks_ready_route_recovery() {
        let mut table = NeighborTable::new(DeviceRole::Child);
        table.upsert(ThreadNeighbor::new(
            ThreadNeighborId(0x1000),
            DeviceRole::Router,
            NeighborRelationship::Parent,
            1_200,
            10_000,
        ));
        let action_summary = ThreadAttachActionSummary::from_summaries(
            MleMessageBatchSummary::empty(),
            table.summary_at(1_250),
        );
        let completion_summary = summarize_thread_attach_completion(
            action_summary,
            table
                .diagnostic_snapshot(None, 1_250)
                .unwrap()
                .supervision_plan(),
        );
        let border_router =
            NetworkDataTlv::new(NetworkDataTlvType::BorderRouter, true, vec![0xaa]).unwrap();
        let context = NetworkDataTlv::new(NetworkDataTlvType::Context, true, vec![0x01]).unwrap();
        let prefix = ThreadPrefixData::new(
            true,
            3,
            64,
            vec![0xfd, 0x00, 0xab, 0xcd, 0, 0, 0, 0],
            vec![border_router, context],
        )
        .unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![prefix.to_tlv().unwrap()]).unwrap();
        let network_data_readiness =
            summarize_thread_network_data_readiness(&network_data).unwrap();
        let handoff_summary =
            summarize_thread_attach_route_handoff(completion_summary, network_data_readiness);
        let audit_summary = summarize_thread_attach_route_audit(handoff_summary);
        let signoff_summary = summarize_thread_attach_route_signoff(audit_summary);
        let completion_summary = summarize_thread_attach_route_completion(signoff_summary);
        let publication_summary = summarize_thread_attach_route_publication(completion_summary);
        let verification_summary = summarize_thread_attach_route_verification(publication_summary);
        let validation_summary = summarize_thread_attach_route_validation(verification_summary);
        let certification_summary = summarize_thread_attach_route_certification(validation_summary);
        let approval_summary = summarize_thread_attach_route_approval(certification_summary);
        let activation_summary = summarize_thread_attach_route_activation(approval_summary);
        let rollout_summary = summarize_thread_attach_route_rollout(activation_summary);
        let adoption_summary = summarize_thread_attach_route_adoption(rollout_summary);
        let acceptance_summary = summarize_thread_attach_route_acceptance(adoption_summary);
        let distribution_summary = summarize_thread_attach_route_distribution(acceptance_summary);
        let export_summary = summarize_thread_attach_route_export(distribution_summary);
        let import_summary = summarize_thread_attach_route_import(export_summary);
        let ingest_summary = summarize_thread_attach_route_ingest(import_summary);
        let load_summary = summarize_thread_attach_route_load(ingest_summary);
        let restore_summary = summarize_thread_attach_route_restore(load_summary);

        let summary = summarize_thread_attach_route_recovery(restore_summary);

        assert_eq!(summary.restore_summary, restore_summary);
        assert_eq!(summary.required_route_recovery_check_count, 23);
        assert_eq!(summary.passed_route_recovery_check_count, 23);
        assert_eq!(summary.missing_route_recovery_check_count, 0);
        assert!(summary.route_restore_ready);
        assert!(summary.route_load_ready);
        assert!(summary.route_ingest_ready);
        assert!(summary.route_import_ready);
        assert!(summary.route_export_ready);
        assert!(summary.route_distribution_ready);
        assert!(summary.route_acceptance_ready);
        assert!(summary.route_adoption_ready);
        assert!(summary.route_rollout_ready);
        assert!(summary.route_activation_ready);
        assert!(summary.route_approval_ready);
        assert!(summary.route_certification_ready);
        assert!(summary.route_validation_ready);
        assert!(summary.route_verification_ready);
        assert!(summary.route_publication_ready);
        assert!(summary.route_completion_ready);
        assert!(summary.route_signoff_ready);
        assert!(summary.route_audit_ready);
        assert!(summary.route_handoff_ready);
        assert!(summary.attach_complete);
        assert!(summary.network_data_ready);
        assert!(summary.routing_surface_ready);
        assert!(summary.parent_or_route_anchor_ready);
        assert!(summary.route_recovery_ready);
        assert!(summary.is_route_recovery_ready());
        assert!(!summary.has_recovery_gaps());
        assert!(!summary.needs_route_restore());
        assert!(!summary.needs_route_load());
        assert!(!summary.needs_route_ingest());
        assert!(!summary.needs_route_import());
        assert!(!summary.needs_route_export());
        assert!(!summary.needs_route_distribution());
        assert!(!summary.needs_route_acceptance());
        assert!(!summary.needs_route_adoption());
        assert!(!summary.needs_route_rollout());
        assert!(!summary.needs_route_activation());
        assert!(!summary.needs_route_approval());
        assert!(!summary.needs_route_certification());
        assert!(!summary.needs_route_validation());
        assert!(!summary.needs_route_verification());
        assert!(!summary.needs_route_publication());
        assert!(!summary.needs_route_completion());
        assert!(!summary.needs_route_signoff());
        assert!(!summary.needs_route_audit());
        assert!(!summary.needs_route_handoff());
        assert!(!summary.needs_attach_completion());
        assert!(!summary.needs_network_data());
        assert!(!summary.needs_routing_surface());
        assert!(!summary.needs_parent_or_route_anchor());
    }

    #[test]
    fn attach_route_recovery_summary_routes_blocked_recovery() {
        let table = NeighborTable::new(DeviceRole::Child);
        let action_summary = ThreadAttachActionSummary::from_summaries(
            MleMessageBatchSummary::empty(),
            table.summary_at(1_250),
        );
        let completion_summary = summarize_thread_attach_completion(
            action_summary,
            table
                .diagnostic_snapshot(None, 1_250)
                .unwrap()
                .supervision_plan(),
        );
        let unknown = NetworkDataTlv::new(NetworkDataTlvType::Unknown(42), false, vec![3]).unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![unknown]).unwrap();
        let network_data_readiness = network_data.summary().unwrap().readiness();
        let handoff_summary = ThreadAttachRouteHandoffSummary::from_completion_and_network_data(
            completion_summary,
            network_data_readiness,
        );
        let audit_summary = ThreadAttachRouteAuditSummary::from_handoff_summary(handoff_summary);
        let signoff_summary = ThreadAttachRouteSignoffSummary::from_audit_summary(audit_summary);
        let completion_summary =
            ThreadAttachRouteCompletionSummary::from_signoff_summary(signoff_summary);
        let publication_summary =
            ThreadAttachRoutePublicationSummary::from_completion_summary(completion_summary);
        let verification_summary =
            ThreadAttachRouteVerificationSummary::from_publication_summary(publication_summary);
        let validation_summary =
            ThreadAttachRouteValidationSummary::from_verification_summary(verification_summary);
        let certification_summary =
            ThreadAttachRouteCertificationSummary::from_validation_summary(validation_summary);
        let approval_summary =
            ThreadAttachRouteApprovalSummary::from_certification_summary(certification_summary);
        let activation_summary =
            ThreadAttachRouteActivationSummary::from_approval_summary(approval_summary);
        let rollout_summary =
            ThreadAttachRouteRolloutSummary::from_activation_summary(activation_summary);
        let adoption_summary =
            ThreadAttachRouteAdoptionSummary::from_rollout_summary(rollout_summary);
        let acceptance_summary =
            ThreadAttachRouteAcceptanceSummary::from_adoption_summary(adoption_summary);
        let distribution_summary =
            ThreadAttachRouteDistributionSummary::from_acceptance_summary(acceptance_summary);
        let export_summary =
            ThreadAttachRouteExportSummary::from_distribution_summary(distribution_summary);
        let import_summary = ThreadAttachRouteImportSummary::from_export_summary(export_summary);
        let ingest_summary = ThreadAttachRouteIngestSummary::from_import_summary(import_summary);
        let load_summary = ThreadAttachRouteLoadSummary::from_ingest_summary(ingest_summary);
        let restore_summary = ThreadAttachRouteRestoreSummary::from_load_summary(load_summary);

        let summary = ThreadAttachRouteRecoverySummary::from_restore_summary(restore_summary);

        assert_eq!(summary.required_route_recovery_check_count, 23);
        assert_eq!(summary.passed_route_recovery_check_count, 0);
        assert_eq!(summary.missing_route_recovery_check_count, 23);
        assert!(!summary.route_restore_ready);
        assert!(!summary.route_load_ready);
        assert!(!summary.route_ingest_ready);
        assert!(!summary.route_import_ready);
        assert!(!summary.route_export_ready);
        assert!(!summary.route_distribution_ready);
        assert!(!summary.route_acceptance_ready);
        assert!(!summary.route_adoption_ready);
        assert!(!summary.route_rollout_ready);
        assert!(!summary.route_activation_ready);
        assert!(!summary.route_approval_ready);
        assert!(!summary.route_certification_ready);
        assert!(!summary.route_validation_ready);
        assert!(!summary.route_verification_ready);
        assert!(!summary.route_publication_ready);
        assert!(!summary.route_completion_ready);
        assert!(!summary.route_signoff_ready);
        assert!(!summary.route_audit_ready);
        assert!(!summary.route_handoff_ready);
        assert!(!summary.attach_complete);
        assert!(!summary.network_data_ready);
        assert!(!summary.routing_surface_ready);
        assert!(!summary.parent_or_route_anchor_ready);
        assert!(!summary.route_recovery_ready);
        assert!(!summary.is_route_recovery_ready());
        assert!(summary.has_recovery_gaps());
        assert!(summary.needs_route_restore());
        assert!(summary.needs_route_load());
        assert!(summary.needs_route_ingest());
        assert!(summary.needs_route_import());
        assert!(summary.needs_route_export());
        assert!(summary.needs_route_distribution());
        assert!(summary.needs_route_acceptance());
        assert!(summary.needs_route_adoption());
        assert!(summary.needs_route_rollout());
        assert!(summary.needs_route_activation());
        assert!(summary.needs_route_approval());
        assert!(summary.needs_route_certification());
        assert!(summary.needs_route_validation());
        assert!(summary.needs_route_verification());
        assert!(summary.needs_route_publication());
        assert!(summary.needs_route_completion());
        assert!(summary.needs_route_signoff());
        assert!(summary.needs_route_audit());
        assert!(summary.needs_route_handoff());
        assert!(summary.needs_attach_completion());
        assert!(summary.needs_network_data());
        assert!(summary.needs_routing_surface());
        assert!(summary.needs_parent_or_route_anchor());
    }

    #[test]
    fn attach_route_replay_summary_marks_ready_route_replay() {
        let mut table = NeighborTable::new(DeviceRole::Child);
        table.upsert(ThreadNeighbor::new(
            ThreadNeighborId(0x1000),
            DeviceRole::Router,
            NeighborRelationship::Parent,
            1_200,
            10_000,
        ));
        let action_summary = ThreadAttachActionSummary::from_summaries(
            MleMessageBatchSummary::empty(),
            table.summary_at(1_250),
        );
        let completion_summary = summarize_thread_attach_completion(
            action_summary,
            table
                .diagnostic_snapshot(None, 1_250)
                .unwrap()
                .supervision_plan(),
        );
        let border_router =
            NetworkDataTlv::new(NetworkDataTlvType::BorderRouter, true, vec![0xaa]).unwrap();
        let context = NetworkDataTlv::new(NetworkDataTlvType::Context, true, vec![0x01]).unwrap();
        let prefix = ThreadPrefixData::new(
            true,
            3,
            64,
            vec![0xfd, 0x00, 0xab, 0xcd, 0, 0, 0, 0],
            vec![border_router, context],
        )
        .unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![prefix.to_tlv().unwrap()]).unwrap();
        let network_data_readiness =
            summarize_thread_network_data_readiness(&network_data).unwrap();
        let handoff_summary =
            summarize_thread_attach_route_handoff(completion_summary, network_data_readiness);
        let audit_summary = summarize_thread_attach_route_audit(handoff_summary);
        let signoff_summary = summarize_thread_attach_route_signoff(audit_summary);
        let completion_summary = summarize_thread_attach_route_completion(signoff_summary);
        let publication_summary = summarize_thread_attach_route_publication(completion_summary);
        let verification_summary = summarize_thread_attach_route_verification(publication_summary);
        let validation_summary = summarize_thread_attach_route_validation(verification_summary);
        let certification_summary = summarize_thread_attach_route_certification(validation_summary);
        let approval_summary = summarize_thread_attach_route_approval(certification_summary);
        let activation_summary = summarize_thread_attach_route_activation(approval_summary);
        let rollout_summary = summarize_thread_attach_route_rollout(activation_summary);
        let adoption_summary = summarize_thread_attach_route_adoption(rollout_summary);
        let acceptance_summary = summarize_thread_attach_route_acceptance(adoption_summary);
        let distribution_summary = summarize_thread_attach_route_distribution(acceptance_summary);
        let export_summary = summarize_thread_attach_route_export(distribution_summary);
        let import_summary = summarize_thread_attach_route_import(export_summary);
        let ingest_summary = summarize_thread_attach_route_ingest(import_summary);
        let load_summary = summarize_thread_attach_route_load(ingest_summary);
        let restore_summary = summarize_thread_attach_route_restore(load_summary);
        let recovery_summary = summarize_thread_attach_route_recovery(restore_summary);

        let summary = summarize_thread_attach_route_replay(recovery_summary);

        assert_eq!(summary.recovery_summary, recovery_summary);
        assert_eq!(summary.required_route_replay_check_count, 24);
        assert_eq!(summary.passed_route_replay_check_count, 24);
        assert_eq!(summary.missing_route_replay_check_count, 0);
        assert!(summary.route_recovery_ready);
        assert!(summary.route_restore_ready);
        assert!(summary.route_load_ready);
        assert!(summary.route_ingest_ready);
        assert!(summary.route_import_ready);
        assert!(summary.route_export_ready);
        assert!(summary.route_distribution_ready);
        assert!(summary.route_acceptance_ready);
        assert!(summary.route_adoption_ready);
        assert!(summary.route_rollout_ready);
        assert!(summary.route_activation_ready);
        assert!(summary.route_approval_ready);
        assert!(summary.route_certification_ready);
        assert!(summary.route_validation_ready);
        assert!(summary.route_verification_ready);
        assert!(summary.route_publication_ready);
        assert!(summary.route_completion_ready);
        assert!(summary.route_signoff_ready);
        assert!(summary.route_audit_ready);
        assert!(summary.route_handoff_ready);
        assert!(summary.attach_complete);
        assert!(summary.network_data_ready);
        assert!(summary.routing_surface_ready);
        assert!(summary.parent_or_route_anchor_ready);
        assert!(summary.route_replay_ready);
        assert!(summary.is_route_replay_ready());
        assert!(!summary.has_replay_gaps());
        assert!(!summary.needs_route_recovery());
        assert!(!summary.needs_route_restore());
        assert!(!summary.needs_route_load());
        assert!(!summary.needs_route_ingest());
        assert!(!summary.needs_route_import());
        assert!(!summary.needs_route_export());
        assert!(!summary.needs_route_distribution());
        assert!(!summary.needs_route_acceptance());
        assert!(!summary.needs_route_adoption());
        assert!(!summary.needs_route_rollout());
        assert!(!summary.needs_route_activation());
        assert!(!summary.needs_route_approval());
        assert!(!summary.needs_route_certification());
        assert!(!summary.needs_route_validation());
        assert!(!summary.needs_route_verification());
        assert!(!summary.needs_route_publication());
        assert!(!summary.needs_route_completion());
        assert!(!summary.needs_route_signoff());
        assert!(!summary.needs_route_audit());
        assert!(!summary.needs_route_handoff());
        assert!(!summary.needs_attach_completion());
        assert!(!summary.needs_network_data());
        assert!(!summary.needs_routing_surface());
        assert!(!summary.needs_parent_or_route_anchor());
    }

    #[test]
    fn attach_route_replay_summary_routes_blocked_replay() {
        let table = NeighborTable::new(DeviceRole::Child);
        let action_summary = ThreadAttachActionSummary::from_summaries(
            MleMessageBatchSummary::empty(),
            table.summary_at(1_250),
        );
        let completion_summary = summarize_thread_attach_completion(
            action_summary,
            table
                .diagnostic_snapshot(None, 1_250)
                .unwrap()
                .supervision_plan(),
        );
        let unknown = NetworkDataTlv::new(NetworkDataTlvType::Unknown(42), false, vec![3]).unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![unknown]).unwrap();
        let network_data_readiness = network_data.summary().unwrap().readiness();
        let handoff_summary = ThreadAttachRouteHandoffSummary::from_completion_and_network_data(
            completion_summary,
            network_data_readiness,
        );
        let audit_summary = ThreadAttachRouteAuditSummary::from_handoff_summary(handoff_summary);
        let signoff_summary = ThreadAttachRouteSignoffSummary::from_audit_summary(audit_summary);
        let completion_summary =
            ThreadAttachRouteCompletionSummary::from_signoff_summary(signoff_summary);
        let publication_summary =
            ThreadAttachRoutePublicationSummary::from_completion_summary(completion_summary);
        let verification_summary =
            ThreadAttachRouteVerificationSummary::from_publication_summary(publication_summary);
        let validation_summary =
            ThreadAttachRouteValidationSummary::from_verification_summary(verification_summary);
        let certification_summary =
            ThreadAttachRouteCertificationSummary::from_validation_summary(validation_summary);
        let approval_summary =
            ThreadAttachRouteApprovalSummary::from_certification_summary(certification_summary);
        let activation_summary =
            ThreadAttachRouteActivationSummary::from_approval_summary(approval_summary);
        let rollout_summary =
            ThreadAttachRouteRolloutSummary::from_activation_summary(activation_summary);
        let adoption_summary =
            ThreadAttachRouteAdoptionSummary::from_rollout_summary(rollout_summary);
        let acceptance_summary =
            ThreadAttachRouteAcceptanceSummary::from_adoption_summary(adoption_summary);
        let distribution_summary =
            ThreadAttachRouteDistributionSummary::from_acceptance_summary(acceptance_summary);
        let export_summary =
            ThreadAttachRouteExportSummary::from_distribution_summary(distribution_summary);
        let import_summary = ThreadAttachRouteImportSummary::from_export_summary(export_summary);
        let ingest_summary = ThreadAttachRouteIngestSummary::from_import_summary(import_summary);
        let load_summary = ThreadAttachRouteLoadSummary::from_ingest_summary(ingest_summary);
        let restore_summary = ThreadAttachRouteRestoreSummary::from_load_summary(load_summary);
        let recovery_summary =
            ThreadAttachRouteRecoverySummary::from_restore_summary(restore_summary);

        let summary = ThreadAttachRouteReplaySummary::from_recovery_summary(recovery_summary);

        assert_eq!(summary.required_route_replay_check_count, 24);
        assert_eq!(summary.passed_route_replay_check_count, 0);
        assert_eq!(summary.missing_route_replay_check_count, 24);
        assert!(!summary.route_recovery_ready);
        assert!(!summary.route_restore_ready);
        assert!(!summary.route_load_ready);
        assert!(!summary.route_ingest_ready);
        assert!(!summary.route_import_ready);
        assert!(!summary.route_export_ready);
        assert!(!summary.route_distribution_ready);
        assert!(!summary.route_acceptance_ready);
        assert!(!summary.route_adoption_ready);
        assert!(!summary.route_rollout_ready);
        assert!(!summary.route_activation_ready);
        assert!(!summary.route_approval_ready);
        assert!(!summary.route_certification_ready);
        assert!(!summary.route_validation_ready);
        assert!(!summary.route_verification_ready);
        assert!(!summary.route_publication_ready);
        assert!(!summary.route_completion_ready);
        assert!(!summary.route_signoff_ready);
        assert!(!summary.route_audit_ready);
        assert!(!summary.route_handoff_ready);
        assert!(!summary.attach_complete);
        assert!(!summary.network_data_ready);
        assert!(!summary.routing_surface_ready);
        assert!(!summary.parent_or_route_anchor_ready);
        assert!(!summary.route_replay_ready);
        assert!(!summary.is_route_replay_ready());
        assert!(summary.has_replay_gaps());
        assert!(summary.needs_route_recovery());
        assert!(summary.needs_route_restore());
        assert!(summary.needs_route_load());
        assert!(summary.needs_route_ingest());
        assert!(summary.needs_route_import());
        assert!(summary.needs_route_export());
        assert!(summary.needs_route_distribution());
        assert!(summary.needs_route_acceptance());
        assert!(summary.needs_route_adoption());
        assert!(summary.needs_route_rollout());
        assert!(summary.needs_route_activation());
        assert!(summary.needs_route_approval());
        assert!(summary.needs_route_certification());
        assert!(summary.needs_route_validation());
        assert!(summary.needs_route_verification());
        assert!(summary.needs_route_publication());
        assert!(summary.needs_route_completion());
        assert!(summary.needs_route_signoff());
        assert!(summary.needs_route_audit());
        assert!(summary.needs_route_handoff());
        assert!(summary.needs_attach_completion());
        assert!(summary.needs_network_data());
        assert!(summary.needs_routing_surface());
        assert!(summary.needs_parent_or_route_anchor());
    }

    #[test]
    fn attach_route_reconciliation_summary_marks_ready_route_reconciliation() {
        let mut table = NeighborTable::new(DeviceRole::Child);
        table.upsert(ThreadNeighbor::new(
            ThreadNeighborId(0x1000),
            DeviceRole::Router,
            NeighborRelationship::Parent,
            1_200,
            10_000,
        ));
        let action_summary = ThreadAttachActionSummary::from_summaries(
            MleMessageBatchSummary::empty(),
            table.summary_at(1_250),
        );
        let completion_summary = summarize_thread_attach_completion(
            action_summary,
            table
                .diagnostic_snapshot(None, 1_250)
                .unwrap()
                .supervision_plan(),
        );
        let border_router =
            NetworkDataTlv::new(NetworkDataTlvType::BorderRouter, true, vec![0xaa]).unwrap();
        let context = NetworkDataTlv::new(NetworkDataTlvType::Context, true, vec![0x01]).unwrap();
        let prefix = ThreadPrefixData::new(
            true,
            3,
            64,
            vec![0xfd, 0x00, 0xab, 0xcd, 0, 0, 0, 0],
            vec![border_router, context],
        )
        .unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![prefix.to_tlv().unwrap()]).unwrap();
        let network_data_readiness =
            summarize_thread_network_data_readiness(&network_data).unwrap();
        let handoff_summary =
            summarize_thread_attach_route_handoff(completion_summary, network_data_readiness);
        let audit_summary = summarize_thread_attach_route_audit(handoff_summary);
        let signoff_summary = summarize_thread_attach_route_signoff(audit_summary);
        let completion_summary = summarize_thread_attach_route_completion(signoff_summary);
        let publication_summary = summarize_thread_attach_route_publication(completion_summary);
        let verification_summary = summarize_thread_attach_route_verification(publication_summary);
        let validation_summary = summarize_thread_attach_route_validation(verification_summary);
        let certification_summary = summarize_thread_attach_route_certification(validation_summary);
        let approval_summary = summarize_thread_attach_route_approval(certification_summary);
        let activation_summary = summarize_thread_attach_route_activation(approval_summary);
        let rollout_summary = summarize_thread_attach_route_rollout(activation_summary);
        let adoption_summary = summarize_thread_attach_route_adoption(rollout_summary);
        let acceptance_summary = summarize_thread_attach_route_acceptance(adoption_summary);
        let distribution_summary = summarize_thread_attach_route_distribution(acceptance_summary);
        let export_summary = summarize_thread_attach_route_export(distribution_summary);
        let import_summary = summarize_thread_attach_route_import(export_summary);
        let ingest_summary = summarize_thread_attach_route_ingest(import_summary);
        let load_summary = summarize_thread_attach_route_load(ingest_summary);
        let restore_summary = summarize_thread_attach_route_restore(load_summary);
        let recovery_summary = summarize_thread_attach_route_recovery(restore_summary);
        let replay_summary = summarize_thread_attach_route_replay(recovery_summary);

        let summary = summarize_thread_attach_route_reconciliation(replay_summary);

        assert_eq!(summary.replay_summary, replay_summary);
        assert_eq!(summary.required_route_reconciliation_check_count, 25);
        assert_eq!(summary.passed_route_reconciliation_check_count, 25);
        assert_eq!(summary.missing_route_reconciliation_check_count, 0);
        assert!(summary.route_replay_ready);
        assert!(summary.route_recovery_ready);
        assert!(summary.route_restore_ready);
        assert!(summary.route_load_ready);
        assert!(summary.route_ingest_ready);
        assert!(summary.route_import_ready);
        assert!(summary.route_export_ready);
        assert!(summary.route_distribution_ready);
        assert!(summary.route_acceptance_ready);
        assert!(summary.route_adoption_ready);
        assert!(summary.route_rollout_ready);
        assert!(summary.route_activation_ready);
        assert!(summary.route_approval_ready);
        assert!(summary.route_certification_ready);
        assert!(summary.route_validation_ready);
        assert!(summary.route_verification_ready);
        assert!(summary.route_publication_ready);
        assert!(summary.route_completion_ready);
        assert!(summary.route_signoff_ready);
        assert!(summary.route_audit_ready);
        assert!(summary.route_handoff_ready);
        assert!(summary.attach_complete);
        assert!(summary.network_data_ready);
        assert!(summary.routing_surface_ready);
        assert!(summary.parent_or_route_anchor_ready);
        assert!(summary.route_reconciliation_ready);
        assert!(summary.is_route_reconciliation_ready());
        assert!(!summary.has_reconciliation_gaps());
        assert!(!summary.needs_route_replay());
        assert!(!summary.needs_route_recovery());
        assert!(!summary.needs_route_restore());
        assert!(!summary.needs_route_load());
        assert!(!summary.needs_route_ingest());
        assert!(!summary.needs_route_import());
        assert!(!summary.needs_route_export());
        assert!(!summary.needs_route_distribution());
        assert!(!summary.needs_route_acceptance());
        assert!(!summary.needs_route_adoption());
        assert!(!summary.needs_route_rollout());
        assert!(!summary.needs_route_activation());
        assert!(!summary.needs_route_approval());
        assert!(!summary.needs_route_certification());
        assert!(!summary.needs_route_validation());
        assert!(!summary.needs_route_verification());
        assert!(!summary.needs_route_publication());
        assert!(!summary.needs_route_completion());
        assert!(!summary.needs_route_signoff());
        assert!(!summary.needs_route_audit());
        assert!(!summary.needs_route_handoff());
        assert!(!summary.needs_attach_completion());
        assert!(!summary.needs_network_data());
        assert!(!summary.needs_routing_surface());
        assert!(!summary.needs_parent_or_route_anchor());
    }

    #[test]
    fn attach_route_reconciliation_summary_routes_blocked_reconciliation() {
        let table = NeighborTable::new(DeviceRole::Child);
        let action_summary = ThreadAttachActionSummary::from_summaries(
            MleMessageBatchSummary::empty(),
            table.summary_at(1_250),
        );
        let completion_summary = summarize_thread_attach_completion(
            action_summary,
            table
                .diagnostic_snapshot(None, 1_250)
                .unwrap()
                .supervision_plan(),
        );
        let unknown = NetworkDataTlv::new(NetworkDataTlvType::Unknown(42), false, vec![3]).unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![unknown]).unwrap();
        let network_data_readiness = network_data.summary().unwrap().readiness();
        let handoff_summary = ThreadAttachRouteHandoffSummary::from_completion_and_network_data(
            completion_summary,
            network_data_readiness,
        );
        let audit_summary = ThreadAttachRouteAuditSummary::from_handoff_summary(handoff_summary);
        let signoff_summary = ThreadAttachRouteSignoffSummary::from_audit_summary(audit_summary);
        let completion_summary =
            ThreadAttachRouteCompletionSummary::from_signoff_summary(signoff_summary);
        let publication_summary =
            ThreadAttachRoutePublicationSummary::from_completion_summary(completion_summary);
        let verification_summary =
            ThreadAttachRouteVerificationSummary::from_publication_summary(publication_summary);
        let validation_summary =
            ThreadAttachRouteValidationSummary::from_verification_summary(verification_summary);
        let certification_summary =
            ThreadAttachRouteCertificationSummary::from_validation_summary(validation_summary);
        let approval_summary =
            ThreadAttachRouteApprovalSummary::from_certification_summary(certification_summary);
        let activation_summary =
            ThreadAttachRouteActivationSummary::from_approval_summary(approval_summary);
        let rollout_summary =
            ThreadAttachRouteRolloutSummary::from_activation_summary(activation_summary);
        let adoption_summary =
            ThreadAttachRouteAdoptionSummary::from_rollout_summary(rollout_summary);
        let acceptance_summary =
            ThreadAttachRouteAcceptanceSummary::from_adoption_summary(adoption_summary);
        let distribution_summary =
            ThreadAttachRouteDistributionSummary::from_acceptance_summary(acceptance_summary);
        let export_summary =
            ThreadAttachRouteExportSummary::from_distribution_summary(distribution_summary);
        let import_summary = ThreadAttachRouteImportSummary::from_export_summary(export_summary);
        let ingest_summary = ThreadAttachRouteIngestSummary::from_import_summary(import_summary);
        let load_summary = ThreadAttachRouteLoadSummary::from_ingest_summary(ingest_summary);
        let restore_summary = ThreadAttachRouteRestoreSummary::from_load_summary(load_summary);
        let recovery_summary =
            ThreadAttachRouteRecoverySummary::from_restore_summary(restore_summary);
        let replay_summary =
            ThreadAttachRouteReplaySummary::from_recovery_summary(recovery_summary);

        let summary = ThreadAttachRouteReconciliationSummary::from_replay_summary(replay_summary);

        assert_eq!(summary.required_route_reconciliation_check_count, 25);
        assert_eq!(summary.passed_route_reconciliation_check_count, 0);
        assert_eq!(summary.missing_route_reconciliation_check_count, 25);
        assert!(!summary.route_replay_ready);
        assert!(!summary.route_recovery_ready);
        assert!(!summary.route_restore_ready);
        assert!(!summary.route_load_ready);
        assert!(!summary.route_ingest_ready);
        assert!(!summary.route_import_ready);
        assert!(!summary.route_export_ready);
        assert!(!summary.route_distribution_ready);
        assert!(!summary.route_acceptance_ready);
        assert!(!summary.route_adoption_ready);
        assert!(!summary.route_rollout_ready);
        assert!(!summary.route_activation_ready);
        assert!(!summary.route_approval_ready);
        assert!(!summary.route_certification_ready);
        assert!(!summary.route_validation_ready);
        assert!(!summary.route_verification_ready);
        assert!(!summary.route_publication_ready);
        assert!(!summary.route_completion_ready);
        assert!(!summary.route_signoff_ready);
        assert!(!summary.route_audit_ready);
        assert!(!summary.route_handoff_ready);
        assert!(!summary.attach_complete);
        assert!(!summary.network_data_ready);
        assert!(!summary.routing_surface_ready);
        assert!(!summary.parent_or_route_anchor_ready);
        assert!(!summary.route_reconciliation_ready);
        assert!(!summary.is_route_reconciliation_ready());
        assert!(summary.has_reconciliation_gaps());
        assert!(summary.needs_route_replay());
        assert!(summary.needs_route_recovery());
        assert!(summary.needs_route_restore());
        assert!(summary.needs_route_load());
        assert!(summary.needs_route_ingest());
        assert!(summary.needs_route_import());
        assert!(summary.needs_route_export());
        assert!(summary.needs_route_distribution());
        assert!(summary.needs_route_acceptance());
        assert!(summary.needs_route_adoption());
        assert!(summary.needs_route_rollout());
        assert!(summary.needs_route_activation());
        assert!(summary.needs_route_approval());
        assert!(summary.needs_route_certification());
        assert!(summary.needs_route_validation());
        assert!(summary.needs_route_verification());
        assert!(summary.needs_route_publication());
        assert!(summary.needs_route_completion());
        assert!(summary.needs_route_signoff());
        assert!(summary.needs_route_audit());
        assert!(summary.needs_route_handoff());
        assert!(summary.needs_attach_completion());
        assert!(summary.needs_network_data());
        assert!(summary.needs_routing_surface());
        assert!(summary.needs_parent_or_route_anchor());
    }

    #[test]
    fn attach_route_settlement_summary_marks_ready_route_settlement() {
        let mut table = NeighborTable::new(DeviceRole::Child);
        table.upsert(ThreadNeighbor::new(
            ThreadNeighborId(0x1000),
            DeviceRole::Router,
            NeighborRelationship::Parent,
            1_200,
            10_000,
        ));
        let action_summary = ThreadAttachActionSummary::from_summaries(
            MleMessageBatchSummary::empty(),
            table.summary_at(1_250),
        );
        let completion_summary = summarize_thread_attach_completion(
            action_summary,
            table
                .diagnostic_snapshot(None, 1_250)
                .unwrap()
                .supervision_plan(),
        );
        let border_router =
            NetworkDataTlv::new(NetworkDataTlvType::BorderRouter, true, vec![0xaa]).unwrap();
        let context = NetworkDataTlv::new(NetworkDataTlvType::Context, true, vec![0x01]).unwrap();
        let prefix = ThreadPrefixData::new(
            true,
            3,
            64,
            vec![0xfd, 0x00, 0xab, 0xcd, 0, 0, 0, 0],
            vec![border_router, context],
        )
        .unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![prefix.to_tlv().unwrap()]).unwrap();
        let network_data_readiness =
            summarize_thread_network_data_readiness(&network_data).unwrap();
        let handoff_summary =
            summarize_thread_attach_route_handoff(completion_summary, network_data_readiness);
        let audit_summary = summarize_thread_attach_route_audit(handoff_summary);
        let signoff_summary = summarize_thread_attach_route_signoff(audit_summary);
        let completion_summary = summarize_thread_attach_route_completion(signoff_summary);
        let publication_summary = summarize_thread_attach_route_publication(completion_summary);
        let verification_summary = summarize_thread_attach_route_verification(publication_summary);
        let validation_summary = summarize_thread_attach_route_validation(verification_summary);
        let certification_summary = summarize_thread_attach_route_certification(validation_summary);
        let approval_summary = summarize_thread_attach_route_approval(certification_summary);
        let activation_summary = summarize_thread_attach_route_activation(approval_summary);
        let rollout_summary = summarize_thread_attach_route_rollout(activation_summary);
        let adoption_summary = summarize_thread_attach_route_adoption(rollout_summary);
        let acceptance_summary = summarize_thread_attach_route_acceptance(adoption_summary);
        let distribution_summary = summarize_thread_attach_route_distribution(acceptance_summary);
        let export_summary = summarize_thread_attach_route_export(distribution_summary);
        let import_summary = summarize_thread_attach_route_import(export_summary);
        let ingest_summary = summarize_thread_attach_route_ingest(import_summary);
        let load_summary = summarize_thread_attach_route_load(ingest_summary);
        let restore_summary = summarize_thread_attach_route_restore(load_summary);
        let recovery_summary = summarize_thread_attach_route_recovery(restore_summary);
        let replay_summary = summarize_thread_attach_route_replay(recovery_summary);
        let reconciliation_summary = summarize_thread_attach_route_reconciliation(replay_summary);

        let summary = summarize_thread_attach_route_settlement(reconciliation_summary);

        assert_eq!(summary.reconciliation_summary, reconciliation_summary);
        assert_eq!(summary.required_route_settlement_check_count, 26);
        assert_eq!(summary.passed_route_settlement_check_count, 26);
        assert_eq!(summary.missing_route_settlement_check_count, 0);
        assert!(summary.route_reconciliation_ready);
        assert!(summary.route_replay_ready);
        assert!(summary.route_recovery_ready);
        assert!(summary.route_restore_ready);
        assert!(summary.route_load_ready);
        assert!(summary.route_ingest_ready);
        assert!(summary.route_import_ready);
        assert!(summary.route_export_ready);
        assert!(summary.route_distribution_ready);
        assert!(summary.route_acceptance_ready);
        assert!(summary.route_adoption_ready);
        assert!(summary.route_rollout_ready);
        assert!(summary.route_activation_ready);
        assert!(summary.route_approval_ready);
        assert!(summary.route_certification_ready);
        assert!(summary.route_validation_ready);
        assert!(summary.route_verification_ready);
        assert!(summary.route_publication_ready);
        assert!(summary.route_completion_ready);
        assert!(summary.route_signoff_ready);
        assert!(summary.route_audit_ready);
        assert!(summary.route_handoff_ready);
        assert!(summary.attach_complete);
        assert!(summary.network_data_ready);
        assert!(summary.routing_surface_ready);
        assert!(summary.parent_or_route_anchor_ready);
        assert!(summary.route_settlement_ready);
        assert!(summary.is_route_settlement_ready());
        assert!(!summary.has_settlement_gaps());
        assert!(!summary.needs_route_reconciliation());
        assert!(!summary.needs_route_replay());
        assert!(!summary.needs_route_recovery());
        assert!(!summary.needs_route_restore());
        assert!(!summary.needs_route_load());
        assert!(!summary.needs_route_ingest());
        assert!(!summary.needs_route_import());
        assert!(!summary.needs_route_export());
        assert!(!summary.needs_route_distribution());
        assert!(!summary.needs_route_acceptance());
        assert!(!summary.needs_route_adoption());
        assert!(!summary.needs_route_rollout());
        assert!(!summary.needs_route_activation());
        assert!(!summary.needs_route_approval());
        assert!(!summary.needs_route_certification());
        assert!(!summary.needs_route_validation());
        assert!(!summary.needs_route_verification());
        assert!(!summary.needs_route_publication());
        assert!(!summary.needs_route_completion());
        assert!(!summary.needs_route_signoff());
        assert!(!summary.needs_route_audit());
        assert!(!summary.needs_route_handoff());
        assert!(!summary.needs_attach_completion());
        assert!(!summary.needs_network_data());
        assert!(!summary.needs_routing_surface());
        assert!(!summary.needs_parent_or_route_anchor());
    }

    #[test]
    fn attach_route_settlement_summary_routes_blocked_settlement() {
        let table = NeighborTable::new(DeviceRole::Child);
        let action_summary = ThreadAttachActionSummary::from_summaries(
            MleMessageBatchSummary::empty(),
            table.summary_at(1_250),
        );
        let completion_summary = summarize_thread_attach_completion(
            action_summary,
            table
                .diagnostic_snapshot(None, 1_250)
                .unwrap()
                .supervision_plan(),
        );
        let unknown = NetworkDataTlv::new(NetworkDataTlvType::Unknown(42), false, vec![3]).unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![unknown]).unwrap();
        let network_data_readiness = network_data.summary().unwrap().readiness();
        let handoff_summary = ThreadAttachRouteHandoffSummary::from_completion_and_network_data(
            completion_summary,
            network_data_readiness,
        );
        let audit_summary = ThreadAttachRouteAuditSummary::from_handoff_summary(handoff_summary);
        let signoff_summary = ThreadAttachRouteSignoffSummary::from_audit_summary(audit_summary);
        let completion_summary =
            ThreadAttachRouteCompletionSummary::from_signoff_summary(signoff_summary);
        let publication_summary =
            ThreadAttachRoutePublicationSummary::from_completion_summary(completion_summary);
        let verification_summary =
            ThreadAttachRouteVerificationSummary::from_publication_summary(publication_summary);
        let validation_summary =
            ThreadAttachRouteValidationSummary::from_verification_summary(verification_summary);
        let certification_summary =
            ThreadAttachRouteCertificationSummary::from_validation_summary(validation_summary);
        let approval_summary =
            ThreadAttachRouteApprovalSummary::from_certification_summary(certification_summary);
        let activation_summary =
            ThreadAttachRouteActivationSummary::from_approval_summary(approval_summary);
        let rollout_summary =
            ThreadAttachRouteRolloutSummary::from_activation_summary(activation_summary);
        let adoption_summary =
            ThreadAttachRouteAdoptionSummary::from_rollout_summary(rollout_summary);
        let acceptance_summary =
            ThreadAttachRouteAcceptanceSummary::from_adoption_summary(adoption_summary);
        let distribution_summary =
            ThreadAttachRouteDistributionSummary::from_acceptance_summary(acceptance_summary);
        let export_summary =
            ThreadAttachRouteExportSummary::from_distribution_summary(distribution_summary);
        let import_summary = ThreadAttachRouteImportSummary::from_export_summary(export_summary);
        let ingest_summary = ThreadAttachRouteIngestSummary::from_import_summary(import_summary);
        let load_summary = ThreadAttachRouteLoadSummary::from_ingest_summary(ingest_summary);
        let restore_summary = ThreadAttachRouteRestoreSummary::from_load_summary(load_summary);
        let recovery_summary =
            ThreadAttachRouteRecoverySummary::from_restore_summary(restore_summary);
        let replay_summary =
            ThreadAttachRouteReplaySummary::from_recovery_summary(recovery_summary);
        let reconciliation_summary =
            ThreadAttachRouteReconciliationSummary::from_replay_summary(replay_summary);

        let summary =
            ThreadAttachRouteSettlementSummary::from_reconciliation_summary(reconciliation_summary);

        assert_eq!(summary.reconciliation_summary, reconciliation_summary);
        assert_eq!(summary.required_route_settlement_check_count, 26);
        assert_eq!(summary.passed_route_settlement_check_count, 0);
        assert_eq!(summary.missing_route_settlement_check_count, 26);
        assert!(!summary.route_reconciliation_ready);
        assert!(!summary.route_replay_ready);
        assert!(!summary.route_recovery_ready);
        assert!(!summary.route_restore_ready);
        assert!(!summary.route_load_ready);
        assert!(!summary.route_ingest_ready);
        assert!(!summary.route_import_ready);
        assert!(!summary.route_export_ready);
        assert!(!summary.route_distribution_ready);
        assert!(!summary.route_acceptance_ready);
        assert!(!summary.route_adoption_ready);
        assert!(!summary.route_rollout_ready);
        assert!(!summary.route_activation_ready);
        assert!(!summary.route_approval_ready);
        assert!(!summary.route_certification_ready);
        assert!(!summary.route_validation_ready);
        assert!(!summary.route_verification_ready);
        assert!(!summary.route_publication_ready);
        assert!(!summary.route_completion_ready);
        assert!(!summary.route_signoff_ready);
        assert!(!summary.route_audit_ready);
        assert!(!summary.route_handoff_ready);
        assert!(!summary.attach_complete);
        assert!(!summary.network_data_ready);
        assert!(!summary.routing_surface_ready);
        assert!(!summary.parent_or_route_anchor_ready);
        assert!(!summary.route_settlement_ready);
        assert!(!summary.is_route_settlement_ready());
        assert!(summary.has_settlement_gaps());
        assert!(summary.needs_route_reconciliation());
        assert!(summary.needs_route_replay());
        assert!(summary.needs_route_recovery());
        assert!(summary.needs_route_restore());
        assert!(summary.needs_route_load());
        assert!(summary.needs_route_ingest());
        assert!(summary.needs_route_import());
        assert!(summary.needs_route_export());
        assert!(summary.needs_route_distribution());
        assert!(summary.needs_route_acceptance());
        assert!(summary.needs_route_adoption());
        assert!(summary.needs_route_rollout());
        assert!(summary.needs_route_activation());
        assert!(summary.needs_route_approval());
        assert!(summary.needs_route_certification());
        assert!(summary.needs_route_validation());
        assert!(summary.needs_route_verification());
        assert!(summary.needs_route_publication());
        assert!(summary.needs_route_completion());
        assert!(summary.needs_route_signoff());
        assert!(summary.needs_route_audit());
        assert!(summary.needs_route_handoff());
        assert!(summary.needs_attach_completion());
        assert!(summary.needs_network_data());
        assert!(summary.needs_routing_surface());
        assert!(summary.needs_parent_or_route_anchor());
    }

    #[test]
    fn attach_route_finalization_summary_marks_ready_route_finalization() {
        let mut table = NeighborTable::new(DeviceRole::Child);
        table.upsert(ThreadNeighbor::new(
            ThreadNeighborId(0x1000),
            DeviceRole::Router,
            NeighborRelationship::Parent,
            1_200,
            10_000,
        ));
        let action_summary = ThreadAttachActionSummary::from_summaries(
            MleMessageBatchSummary::empty(),
            table.summary_at(1_250),
        );
        let completion_summary = summarize_thread_attach_completion(
            action_summary,
            table
                .diagnostic_snapshot(None, 1_250)
                .unwrap()
                .supervision_plan(),
        );
        let border_router =
            NetworkDataTlv::new(NetworkDataTlvType::BorderRouter, true, vec![0xaa]).unwrap();
        let context = NetworkDataTlv::new(NetworkDataTlvType::Context, true, vec![0x01]).unwrap();
        let prefix = ThreadPrefixData::new(
            true,
            3,
            64,
            vec![0xfd, 0x00, 0xab, 0xcd, 0, 0, 0, 0],
            vec![border_router, context],
        )
        .unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![prefix.to_tlv().unwrap()]).unwrap();
        let network_data_readiness =
            summarize_thread_network_data_readiness(&network_data).unwrap();
        let handoff_summary =
            summarize_thread_attach_route_handoff(completion_summary, network_data_readiness);
        let audit_summary = summarize_thread_attach_route_audit(handoff_summary);
        let signoff_summary = summarize_thread_attach_route_signoff(audit_summary);
        let completion_summary = summarize_thread_attach_route_completion(signoff_summary);
        let publication_summary = summarize_thread_attach_route_publication(completion_summary);
        let verification_summary = summarize_thread_attach_route_verification(publication_summary);
        let validation_summary = summarize_thread_attach_route_validation(verification_summary);
        let certification_summary = summarize_thread_attach_route_certification(validation_summary);
        let approval_summary = summarize_thread_attach_route_approval(certification_summary);
        let activation_summary = summarize_thread_attach_route_activation(approval_summary);
        let rollout_summary = summarize_thread_attach_route_rollout(activation_summary);
        let adoption_summary = summarize_thread_attach_route_adoption(rollout_summary);
        let acceptance_summary = summarize_thread_attach_route_acceptance(adoption_summary);
        let distribution_summary = summarize_thread_attach_route_distribution(acceptance_summary);
        let export_summary = summarize_thread_attach_route_export(distribution_summary);
        let import_summary = summarize_thread_attach_route_import(export_summary);
        let ingest_summary = summarize_thread_attach_route_ingest(import_summary);
        let load_summary = summarize_thread_attach_route_load(ingest_summary);
        let restore_summary = summarize_thread_attach_route_restore(load_summary);
        let recovery_summary = summarize_thread_attach_route_recovery(restore_summary);
        let replay_summary = summarize_thread_attach_route_replay(recovery_summary);
        let reconciliation_summary = summarize_thread_attach_route_reconciliation(replay_summary);
        let settlement_summary = summarize_thread_attach_route_settlement(reconciliation_summary);

        let summary = summarize_thread_attach_route_finalization(settlement_summary);

        assert_eq!(summary.settlement_summary, settlement_summary);
        assert_eq!(summary.required_route_finalization_check_count, 27);
        assert_eq!(summary.passed_route_finalization_check_count, 27);
        assert_eq!(summary.missing_route_finalization_check_count, 0);
        assert!(summary.route_settlement_ready);
        assert!(summary.route_reconciliation_ready);
        assert!(summary.route_replay_ready);
        assert!(summary.route_recovery_ready);
        assert!(summary.route_restore_ready);
        assert!(summary.route_load_ready);
        assert!(summary.route_ingest_ready);
        assert!(summary.route_import_ready);
        assert!(summary.route_export_ready);
        assert!(summary.route_distribution_ready);
        assert!(summary.route_acceptance_ready);
        assert!(summary.route_adoption_ready);
        assert!(summary.route_rollout_ready);
        assert!(summary.route_activation_ready);
        assert!(summary.route_approval_ready);
        assert!(summary.route_certification_ready);
        assert!(summary.route_validation_ready);
        assert!(summary.route_verification_ready);
        assert!(summary.route_publication_ready);
        assert!(summary.route_completion_ready);
        assert!(summary.route_signoff_ready);
        assert!(summary.route_audit_ready);
        assert!(summary.route_handoff_ready);
        assert!(summary.attach_complete);
        assert!(summary.network_data_ready);
        assert!(summary.routing_surface_ready);
        assert!(summary.parent_or_route_anchor_ready);
        assert!(summary.route_finalization_ready);
        assert!(summary.is_route_finalization_ready());
        assert!(!summary.has_finalization_gaps());
        assert!(!summary.needs_route_settlement());
        assert!(!summary.needs_route_reconciliation());
        assert!(!summary.needs_route_replay());
        assert!(!summary.needs_route_recovery());
        assert!(!summary.needs_route_restore());
        assert!(!summary.needs_route_load());
        assert!(!summary.needs_route_ingest());
        assert!(!summary.needs_route_import());
        assert!(!summary.needs_route_export());
        assert!(!summary.needs_route_distribution());
        assert!(!summary.needs_route_acceptance());
        assert!(!summary.needs_route_adoption());
        assert!(!summary.needs_route_rollout());
        assert!(!summary.needs_route_activation());
        assert!(!summary.needs_route_approval());
        assert!(!summary.needs_route_certification());
        assert!(!summary.needs_route_validation());
        assert!(!summary.needs_route_verification());
        assert!(!summary.needs_route_publication());
        assert!(!summary.needs_route_completion());
        assert!(!summary.needs_route_signoff());
        assert!(!summary.needs_route_audit());
        assert!(!summary.needs_route_handoff());
        assert!(!summary.needs_attach_completion());
        assert!(!summary.needs_network_data());
        assert!(!summary.needs_routing_surface());
        assert!(!summary.needs_parent_or_route_anchor());
    }

    #[test]
    fn attach_route_finalization_summary_routes_blocked_finalization() {
        let table = NeighborTable::new(DeviceRole::Child);
        let action_summary = ThreadAttachActionSummary::from_summaries(
            MleMessageBatchSummary::empty(),
            table.summary_at(1_250),
        );
        let completion_summary = summarize_thread_attach_completion(
            action_summary,
            table
                .diagnostic_snapshot(None, 1_250)
                .unwrap()
                .supervision_plan(),
        );
        let unknown = NetworkDataTlv::new(NetworkDataTlvType::Unknown(42), false, vec![3]).unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![unknown]).unwrap();
        let network_data_readiness = network_data.summary().unwrap().readiness();
        let handoff_summary = ThreadAttachRouteHandoffSummary::from_completion_and_network_data(
            completion_summary,
            network_data_readiness,
        );
        let audit_summary = ThreadAttachRouteAuditSummary::from_handoff_summary(handoff_summary);
        let signoff_summary = ThreadAttachRouteSignoffSummary::from_audit_summary(audit_summary);
        let completion_summary =
            ThreadAttachRouteCompletionSummary::from_signoff_summary(signoff_summary);
        let publication_summary =
            ThreadAttachRoutePublicationSummary::from_completion_summary(completion_summary);
        let verification_summary =
            ThreadAttachRouteVerificationSummary::from_publication_summary(publication_summary);
        let validation_summary =
            ThreadAttachRouteValidationSummary::from_verification_summary(verification_summary);
        let certification_summary =
            ThreadAttachRouteCertificationSummary::from_validation_summary(validation_summary);
        let approval_summary =
            ThreadAttachRouteApprovalSummary::from_certification_summary(certification_summary);
        let activation_summary =
            ThreadAttachRouteActivationSummary::from_approval_summary(approval_summary);
        let rollout_summary =
            ThreadAttachRouteRolloutSummary::from_activation_summary(activation_summary);
        let adoption_summary =
            ThreadAttachRouteAdoptionSummary::from_rollout_summary(rollout_summary);
        let acceptance_summary =
            ThreadAttachRouteAcceptanceSummary::from_adoption_summary(adoption_summary);
        let distribution_summary =
            ThreadAttachRouteDistributionSummary::from_acceptance_summary(acceptance_summary);
        let export_summary =
            ThreadAttachRouteExportSummary::from_distribution_summary(distribution_summary);
        let import_summary = ThreadAttachRouteImportSummary::from_export_summary(export_summary);
        let ingest_summary = ThreadAttachRouteIngestSummary::from_import_summary(import_summary);
        let load_summary = ThreadAttachRouteLoadSummary::from_ingest_summary(ingest_summary);
        let restore_summary = ThreadAttachRouteRestoreSummary::from_load_summary(load_summary);
        let recovery_summary =
            ThreadAttachRouteRecoverySummary::from_restore_summary(restore_summary);
        let replay_summary =
            ThreadAttachRouteReplaySummary::from_recovery_summary(recovery_summary);
        let reconciliation_summary =
            ThreadAttachRouteReconciliationSummary::from_replay_summary(replay_summary);
        let settlement_summary =
            ThreadAttachRouteSettlementSummary::from_reconciliation_summary(reconciliation_summary);

        let summary =
            ThreadAttachRouteFinalizationSummary::from_settlement_summary(settlement_summary);

        assert_eq!(summary.settlement_summary, settlement_summary);
        assert_eq!(summary.required_route_finalization_check_count, 27);
        assert_eq!(summary.passed_route_finalization_check_count, 0);
        assert_eq!(summary.missing_route_finalization_check_count, 27);
        assert!(!summary.route_settlement_ready);
        assert!(!summary.route_reconciliation_ready);
        assert!(!summary.route_replay_ready);
        assert!(!summary.route_recovery_ready);
        assert!(!summary.route_restore_ready);
        assert!(!summary.route_load_ready);
        assert!(!summary.route_ingest_ready);
        assert!(!summary.route_import_ready);
        assert!(!summary.route_export_ready);
        assert!(!summary.route_distribution_ready);
        assert!(!summary.route_acceptance_ready);
        assert!(!summary.route_adoption_ready);
        assert!(!summary.route_rollout_ready);
        assert!(!summary.route_activation_ready);
        assert!(!summary.route_approval_ready);
        assert!(!summary.route_certification_ready);
        assert!(!summary.route_validation_ready);
        assert!(!summary.route_verification_ready);
        assert!(!summary.route_publication_ready);
        assert!(!summary.route_completion_ready);
        assert!(!summary.route_signoff_ready);
        assert!(!summary.route_audit_ready);
        assert!(!summary.route_handoff_ready);
        assert!(!summary.attach_complete);
        assert!(!summary.network_data_ready);
        assert!(!summary.routing_surface_ready);
        assert!(!summary.parent_or_route_anchor_ready);
        assert!(!summary.route_finalization_ready);
        assert!(!summary.is_route_finalization_ready());
        assert!(summary.has_finalization_gaps());
        assert!(summary.needs_route_settlement());
        assert!(summary.needs_route_reconciliation());
        assert!(summary.needs_route_replay());
        assert!(summary.needs_route_recovery());
        assert!(summary.needs_route_restore());
        assert!(summary.needs_route_load());
        assert!(summary.needs_route_ingest());
        assert!(summary.needs_route_import());
        assert!(summary.needs_route_export());
        assert!(summary.needs_route_distribution());
        assert!(summary.needs_route_acceptance());
        assert!(summary.needs_route_adoption());
        assert!(summary.needs_route_rollout());
        assert!(summary.needs_route_activation());
        assert!(summary.needs_route_approval());
        assert!(summary.needs_route_certification());
        assert!(summary.needs_route_validation());
        assert!(summary.needs_route_verification());
        assert!(summary.needs_route_publication());
        assert!(summary.needs_route_completion());
        assert!(summary.needs_route_signoff());
        assert!(summary.needs_route_audit());
        assert!(summary.needs_route_handoff());
        assert!(summary.needs_attach_completion());
        assert!(summary.needs_network_data());
        assert!(summary.needs_routing_surface());
        assert!(summary.needs_parent_or_route_anchor());
    }

    #[test]
    fn attach_route_confirmation_summary_marks_ready_route_confirmation() {
        let mut table = NeighborTable::new(DeviceRole::Child);
        table.upsert(ThreadNeighbor::new(
            ThreadNeighborId(0x1000),
            DeviceRole::Router,
            NeighborRelationship::Parent,
            1_200,
            10_000,
        ));
        let action_summary = ThreadAttachActionSummary::from_summaries(
            MleMessageBatchSummary::empty(),
            table.summary_at(1_250),
        );
        let completion_summary = summarize_thread_attach_completion(
            action_summary,
            table
                .diagnostic_snapshot(None, 1_250)
                .unwrap()
                .supervision_plan(),
        );
        let border_router =
            NetworkDataTlv::new(NetworkDataTlvType::BorderRouter, true, vec![0xaa]).unwrap();
        let context = NetworkDataTlv::new(NetworkDataTlvType::Context, true, vec![0x01]).unwrap();
        let prefix = ThreadPrefixData::new(
            true,
            3,
            64,
            vec![0xfd, 0x00, 0xab, 0xcd, 0, 0, 0, 0],
            vec![border_router, context],
        )
        .unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![prefix.to_tlv().unwrap()]).unwrap();
        let network_data_readiness =
            summarize_thread_network_data_readiness(&network_data).unwrap();
        let handoff_summary =
            summarize_thread_attach_route_handoff(completion_summary, network_data_readiness);
        let audit_summary = summarize_thread_attach_route_audit(handoff_summary);
        let signoff_summary = summarize_thread_attach_route_signoff(audit_summary);
        let completion_summary = summarize_thread_attach_route_completion(signoff_summary);
        let publication_summary = summarize_thread_attach_route_publication(completion_summary);
        let verification_summary = summarize_thread_attach_route_verification(publication_summary);
        let validation_summary = summarize_thread_attach_route_validation(verification_summary);
        let certification_summary = summarize_thread_attach_route_certification(validation_summary);
        let approval_summary = summarize_thread_attach_route_approval(certification_summary);
        let activation_summary = summarize_thread_attach_route_activation(approval_summary);
        let rollout_summary = summarize_thread_attach_route_rollout(activation_summary);
        let adoption_summary = summarize_thread_attach_route_adoption(rollout_summary);
        let acceptance_summary = summarize_thread_attach_route_acceptance(adoption_summary);
        let distribution_summary = summarize_thread_attach_route_distribution(acceptance_summary);
        let export_summary = summarize_thread_attach_route_export(distribution_summary);
        let import_summary = summarize_thread_attach_route_import(export_summary);
        let ingest_summary = summarize_thread_attach_route_ingest(import_summary);
        let load_summary = summarize_thread_attach_route_load(ingest_summary);
        let restore_summary = summarize_thread_attach_route_restore(load_summary);
        let recovery_summary = summarize_thread_attach_route_recovery(restore_summary);
        let replay_summary = summarize_thread_attach_route_replay(recovery_summary);
        let reconciliation_summary = summarize_thread_attach_route_reconciliation(replay_summary);
        let settlement_summary = summarize_thread_attach_route_settlement(reconciliation_summary);
        let finalization_summary = summarize_thread_attach_route_finalization(settlement_summary);

        let summary = summarize_thread_attach_route_confirmation(finalization_summary);

        assert_eq!(summary.finalization_summary, finalization_summary);
        assert_eq!(summary.required_route_confirmation_check_count, 28);
        assert_eq!(summary.passed_route_confirmation_check_count, 28);
        assert_eq!(summary.missing_route_confirmation_check_count, 0);
        assert!(summary.route_finalization_ready);
        assert!(summary.route_settlement_ready);
        assert!(summary.route_reconciliation_ready);
        assert!(summary.route_replay_ready);
        assert!(summary.route_recovery_ready);
        assert!(summary.route_restore_ready);
        assert!(summary.route_load_ready);
        assert!(summary.route_ingest_ready);
        assert!(summary.route_import_ready);
        assert!(summary.route_export_ready);
        assert!(summary.route_distribution_ready);
        assert!(summary.route_acceptance_ready);
        assert!(summary.route_adoption_ready);
        assert!(summary.route_rollout_ready);
        assert!(summary.route_activation_ready);
        assert!(summary.route_approval_ready);
        assert!(summary.route_certification_ready);
        assert!(summary.route_validation_ready);
        assert!(summary.route_verification_ready);
        assert!(summary.route_publication_ready);
        assert!(summary.route_completion_ready);
        assert!(summary.route_signoff_ready);
        assert!(summary.route_audit_ready);
        assert!(summary.route_handoff_ready);
        assert!(summary.attach_complete);
        assert!(summary.network_data_ready);
        assert!(summary.routing_surface_ready);
        assert!(summary.parent_or_route_anchor_ready);
        assert!(summary.route_confirmation_ready);
        assert!(summary.is_route_confirmation_ready());
        assert!(!summary.has_confirmation_gaps());
        assert!(!summary.needs_route_finalization());
        assert!(!summary.needs_route_settlement());
        assert!(!summary.needs_route_reconciliation());
        assert!(!summary.needs_route_replay());
        assert!(!summary.needs_route_recovery());
        assert!(!summary.needs_route_restore());
        assert!(!summary.needs_route_load());
        assert!(!summary.needs_route_ingest());
        assert!(!summary.needs_route_import());
        assert!(!summary.needs_route_export());
        assert!(!summary.needs_route_distribution());
        assert!(!summary.needs_route_acceptance());
        assert!(!summary.needs_route_adoption());
        assert!(!summary.needs_route_rollout());
        assert!(!summary.needs_route_activation());
        assert!(!summary.needs_route_approval());
        assert!(!summary.needs_route_certification());
        assert!(!summary.needs_route_validation());
        assert!(!summary.needs_route_verification());
        assert!(!summary.needs_route_publication());
        assert!(!summary.needs_route_completion());
        assert!(!summary.needs_route_signoff());
        assert!(!summary.needs_route_audit());
        assert!(!summary.needs_route_handoff());
        assert!(!summary.needs_attach_completion());
        assert!(!summary.needs_network_data());
        assert!(!summary.needs_routing_surface());
        assert!(!summary.needs_parent_or_route_anchor());
    }

    #[test]
    fn attach_route_confirmation_summary_routes_blocked_confirmation() {
        let table = NeighborTable::new(DeviceRole::Child);
        let action_summary = ThreadAttachActionSummary::from_summaries(
            MleMessageBatchSummary::empty(),
            table.summary_at(1_250),
        );
        let completion_summary = summarize_thread_attach_completion(
            action_summary,
            table
                .diagnostic_snapshot(None, 1_250)
                .unwrap()
                .supervision_plan(),
        );
        let unknown = NetworkDataTlv::new(NetworkDataTlvType::Unknown(42), false, vec![3]).unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![unknown]).unwrap();
        let network_data_readiness = network_data.summary().unwrap().readiness();
        let handoff_summary = ThreadAttachRouteHandoffSummary::from_completion_and_network_data(
            completion_summary,
            network_data_readiness,
        );
        let audit_summary = ThreadAttachRouteAuditSummary::from_handoff_summary(handoff_summary);
        let signoff_summary = ThreadAttachRouteSignoffSummary::from_audit_summary(audit_summary);
        let completion_summary =
            ThreadAttachRouteCompletionSummary::from_signoff_summary(signoff_summary);
        let publication_summary =
            ThreadAttachRoutePublicationSummary::from_completion_summary(completion_summary);
        let verification_summary =
            ThreadAttachRouteVerificationSummary::from_publication_summary(publication_summary);
        let validation_summary =
            ThreadAttachRouteValidationSummary::from_verification_summary(verification_summary);
        let certification_summary =
            ThreadAttachRouteCertificationSummary::from_validation_summary(validation_summary);
        let approval_summary =
            ThreadAttachRouteApprovalSummary::from_certification_summary(certification_summary);
        let activation_summary =
            ThreadAttachRouteActivationSummary::from_approval_summary(approval_summary);
        let rollout_summary =
            ThreadAttachRouteRolloutSummary::from_activation_summary(activation_summary);
        let adoption_summary =
            ThreadAttachRouteAdoptionSummary::from_rollout_summary(rollout_summary);
        let acceptance_summary =
            ThreadAttachRouteAcceptanceSummary::from_adoption_summary(adoption_summary);
        let distribution_summary =
            ThreadAttachRouteDistributionSummary::from_acceptance_summary(acceptance_summary);
        let export_summary =
            ThreadAttachRouteExportSummary::from_distribution_summary(distribution_summary);
        let import_summary = ThreadAttachRouteImportSummary::from_export_summary(export_summary);
        let ingest_summary = ThreadAttachRouteIngestSummary::from_import_summary(import_summary);
        let load_summary = ThreadAttachRouteLoadSummary::from_ingest_summary(ingest_summary);
        let restore_summary = ThreadAttachRouteRestoreSummary::from_load_summary(load_summary);
        let recovery_summary =
            ThreadAttachRouteRecoverySummary::from_restore_summary(restore_summary);
        let replay_summary =
            ThreadAttachRouteReplaySummary::from_recovery_summary(recovery_summary);
        let reconciliation_summary =
            ThreadAttachRouteReconciliationSummary::from_replay_summary(replay_summary);
        let settlement_summary =
            ThreadAttachRouteSettlementSummary::from_reconciliation_summary(reconciliation_summary);
        let finalization_summary =
            ThreadAttachRouteFinalizationSummary::from_settlement_summary(settlement_summary);

        let summary =
            ThreadAttachRouteConfirmationSummary::from_finalization_summary(finalization_summary);

        assert_eq!(summary.finalization_summary, finalization_summary);
        assert_eq!(summary.required_route_confirmation_check_count, 28);
        assert_eq!(summary.passed_route_confirmation_check_count, 0);
        assert_eq!(summary.missing_route_confirmation_check_count, 28);
        assert!(!summary.route_finalization_ready);
        assert!(!summary.route_settlement_ready);
        assert!(!summary.route_reconciliation_ready);
        assert!(!summary.route_replay_ready);
        assert!(!summary.route_recovery_ready);
        assert!(!summary.route_restore_ready);
        assert!(!summary.route_load_ready);
        assert!(!summary.route_ingest_ready);
        assert!(!summary.route_import_ready);
        assert!(!summary.route_export_ready);
        assert!(!summary.route_distribution_ready);
        assert!(!summary.route_acceptance_ready);
        assert!(!summary.route_adoption_ready);
        assert!(!summary.route_rollout_ready);
        assert!(!summary.route_activation_ready);
        assert!(!summary.route_approval_ready);
        assert!(!summary.route_certification_ready);
        assert!(!summary.route_validation_ready);
        assert!(!summary.route_verification_ready);
        assert!(!summary.route_publication_ready);
        assert!(!summary.route_completion_ready);
        assert!(!summary.route_signoff_ready);
        assert!(!summary.route_audit_ready);
        assert!(!summary.route_handoff_ready);
        assert!(!summary.attach_complete);
        assert!(!summary.network_data_ready);
        assert!(!summary.routing_surface_ready);
        assert!(!summary.parent_or_route_anchor_ready);
        assert!(!summary.route_confirmation_ready);
        assert!(!summary.is_route_confirmation_ready());
        assert!(summary.has_confirmation_gaps());
        assert!(summary.needs_route_finalization());
        assert!(summary.needs_route_settlement());
        assert!(summary.needs_route_reconciliation());
        assert!(summary.needs_route_replay());
        assert!(summary.needs_route_recovery());
        assert!(summary.needs_route_restore());
        assert!(summary.needs_route_load());
        assert!(summary.needs_route_ingest());
        assert!(summary.needs_route_import());
        assert!(summary.needs_route_export());
        assert!(summary.needs_route_distribution());
        assert!(summary.needs_route_acceptance());
        assert!(summary.needs_route_adoption());
        assert!(summary.needs_route_rollout());
        assert!(summary.needs_route_activation());
        assert!(summary.needs_route_approval());
        assert!(summary.needs_route_certification());
        assert!(summary.needs_route_validation());
        assert!(summary.needs_route_verification());
        assert!(summary.needs_route_publication());
        assert!(summary.needs_route_completion());
        assert!(summary.needs_route_signoff());
        assert!(summary.needs_route_audit());
        assert!(summary.needs_route_handoff());
        assert!(summary.needs_attach_completion());
        assert!(summary.needs_network_data());
        assert!(summary.needs_routing_surface());
        assert!(summary.needs_parent_or_route_anchor());
    }

    #[test]
    fn attach_route_attestation_summary_marks_ready_route_attestation() {
        let mut table = NeighborTable::new(DeviceRole::Child);
        table.upsert(ThreadNeighbor::new(
            ThreadNeighborId(0x1000),
            DeviceRole::Router,
            NeighborRelationship::Parent,
            1_200,
            10_000,
        ));
        let action_summary = ThreadAttachActionSummary::from_summaries(
            MleMessageBatchSummary::empty(),
            table.summary_at(1_250),
        );
        let completion_summary = summarize_thread_attach_completion(
            action_summary,
            table
                .diagnostic_snapshot(None, 1_250)
                .unwrap()
                .supervision_plan(),
        );
        let border_router =
            NetworkDataTlv::new(NetworkDataTlvType::BorderRouter, true, vec![0xaa]).unwrap();
        let context = NetworkDataTlv::new(NetworkDataTlvType::Context, true, vec![0x01]).unwrap();
        let prefix = ThreadPrefixData::new(
            true,
            3,
            64,
            vec![0xfd, 0x00, 0xab, 0xcd, 0, 0, 0, 0],
            vec![border_router, context],
        )
        .unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![prefix.to_tlv().unwrap()]).unwrap();
        let network_data_readiness =
            summarize_thread_network_data_readiness(&network_data).unwrap();
        let handoff_summary =
            summarize_thread_attach_route_handoff(completion_summary, network_data_readiness);
        let audit_summary = summarize_thread_attach_route_audit(handoff_summary);
        let signoff_summary = summarize_thread_attach_route_signoff(audit_summary);
        let completion_summary = summarize_thread_attach_route_completion(signoff_summary);
        let publication_summary = summarize_thread_attach_route_publication(completion_summary);
        let verification_summary = summarize_thread_attach_route_verification(publication_summary);
        let validation_summary = summarize_thread_attach_route_validation(verification_summary);
        let certification_summary = summarize_thread_attach_route_certification(validation_summary);
        let approval_summary = summarize_thread_attach_route_approval(certification_summary);
        let activation_summary = summarize_thread_attach_route_activation(approval_summary);
        let rollout_summary = summarize_thread_attach_route_rollout(activation_summary);
        let adoption_summary = summarize_thread_attach_route_adoption(rollout_summary);
        let acceptance_summary = summarize_thread_attach_route_acceptance(adoption_summary);
        let distribution_summary = summarize_thread_attach_route_distribution(acceptance_summary);
        let export_summary = summarize_thread_attach_route_export(distribution_summary);
        let import_summary = summarize_thread_attach_route_import(export_summary);
        let ingest_summary = summarize_thread_attach_route_ingest(import_summary);
        let load_summary = summarize_thread_attach_route_load(ingest_summary);
        let restore_summary = summarize_thread_attach_route_restore(load_summary);
        let recovery_summary = summarize_thread_attach_route_recovery(restore_summary);
        let replay_summary = summarize_thread_attach_route_replay(recovery_summary);
        let reconciliation_summary = summarize_thread_attach_route_reconciliation(replay_summary);
        let settlement_summary = summarize_thread_attach_route_settlement(reconciliation_summary);
        let finalization_summary = summarize_thread_attach_route_finalization(settlement_summary);
        let confirmation_summary = summarize_thread_attach_route_confirmation(finalization_summary);

        let summary = summarize_thread_attach_route_attestation(confirmation_summary);

        assert_eq!(summary.confirmation_summary, confirmation_summary);
        assert_eq!(summary.required_route_attestation_check_count, 29);
        assert_eq!(summary.passed_route_attestation_check_count, 29);
        assert_eq!(summary.missing_route_attestation_check_count, 0);
        assert!(summary.route_confirmation_ready);
        assert!(summary.route_finalization_ready);
        assert!(summary.route_settlement_ready);
        assert!(summary.route_reconciliation_ready);
        assert!(summary.route_replay_ready);
        assert!(summary.route_recovery_ready);
        assert!(summary.route_restore_ready);
        assert!(summary.route_load_ready);
        assert!(summary.route_ingest_ready);
        assert!(summary.route_import_ready);
        assert!(summary.route_export_ready);
        assert!(summary.route_distribution_ready);
        assert!(summary.route_acceptance_ready);
        assert!(summary.route_adoption_ready);
        assert!(summary.route_rollout_ready);
        assert!(summary.route_activation_ready);
        assert!(summary.route_approval_ready);
        assert!(summary.route_certification_ready);
        assert!(summary.route_validation_ready);
        assert!(summary.route_verification_ready);
        assert!(summary.route_publication_ready);
        assert!(summary.route_completion_ready);
        assert!(summary.route_signoff_ready);
        assert!(summary.route_audit_ready);
        assert!(summary.route_handoff_ready);
        assert!(summary.attach_complete);
        assert!(summary.network_data_ready);
        assert!(summary.routing_surface_ready);
        assert!(summary.parent_or_route_anchor_ready);
        assert!(summary.route_attestation_ready);
        assert!(summary.is_route_attestation_ready());
        assert!(!summary.has_attestation_gaps());
        assert!(!summary.needs_route_confirmation());
        assert!(!summary.needs_route_finalization());
        assert!(!summary.needs_route_settlement());
        assert!(!summary.needs_route_reconciliation());
        assert!(!summary.needs_route_replay());
        assert!(!summary.needs_route_recovery());
        assert!(!summary.needs_route_restore());
        assert!(!summary.needs_route_load());
        assert!(!summary.needs_route_ingest());
        assert!(!summary.needs_route_import());
        assert!(!summary.needs_route_export());
        assert!(!summary.needs_route_distribution());
        assert!(!summary.needs_route_acceptance());
        assert!(!summary.needs_route_adoption());
        assert!(!summary.needs_route_rollout());
        assert!(!summary.needs_route_activation());
        assert!(!summary.needs_route_approval());
        assert!(!summary.needs_route_certification());
        assert!(!summary.needs_route_validation());
        assert!(!summary.needs_route_verification());
        assert!(!summary.needs_route_publication());
        assert!(!summary.needs_route_completion());
        assert!(!summary.needs_route_signoff());
        assert!(!summary.needs_route_audit());
        assert!(!summary.needs_route_handoff());
        assert!(!summary.needs_attach_completion());
        assert!(!summary.needs_network_data());
        assert!(!summary.needs_routing_surface());
        assert!(!summary.needs_parent_or_route_anchor());
    }

    #[test]
    fn attach_route_attestation_summary_routes_blocked_attestation() {
        let table = NeighborTable::new(DeviceRole::Child);
        let action_summary = ThreadAttachActionSummary::from_summaries(
            MleMessageBatchSummary::empty(),
            table.summary_at(1_250),
        );
        let completion_summary = summarize_thread_attach_completion(
            action_summary,
            table
                .diagnostic_snapshot(None, 1_250)
                .unwrap()
                .supervision_plan(),
        );
        let unknown = NetworkDataTlv::new(NetworkDataTlvType::Unknown(42), false, vec![3]).unwrap();
        let network_data = ThreadNetworkData::from_tlvs(vec![unknown]).unwrap();
        let network_data_readiness = network_data.summary().unwrap().readiness();
        let handoff_summary = ThreadAttachRouteHandoffSummary::from_completion_and_network_data(
            completion_summary,
            network_data_readiness,
        );
        let audit_summary = ThreadAttachRouteAuditSummary::from_handoff_summary(handoff_summary);
        let signoff_summary = ThreadAttachRouteSignoffSummary::from_audit_summary(audit_summary);
        let completion_summary =
            ThreadAttachRouteCompletionSummary::from_signoff_summary(signoff_summary);
        let publication_summary =
            ThreadAttachRoutePublicationSummary::from_completion_summary(completion_summary);
        let verification_summary =
            ThreadAttachRouteVerificationSummary::from_publication_summary(publication_summary);
        let validation_summary =
            ThreadAttachRouteValidationSummary::from_verification_summary(verification_summary);
        let certification_summary =
            ThreadAttachRouteCertificationSummary::from_validation_summary(validation_summary);
        let approval_summary =
            ThreadAttachRouteApprovalSummary::from_certification_summary(certification_summary);
        let activation_summary =
            ThreadAttachRouteActivationSummary::from_approval_summary(approval_summary);
        let rollout_summary =
            ThreadAttachRouteRolloutSummary::from_activation_summary(activation_summary);
        let adoption_summary =
            ThreadAttachRouteAdoptionSummary::from_rollout_summary(rollout_summary);
        let acceptance_summary =
            ThreadAttachRouteAcceptanceSummary::from_adoption_summary(adoption_summary);
        let distribution_summary =
            ThreadAttachRouteDistributionSummary::from_acceptance_summary(acceptance_summary);
        let export_summary =
            ThreadAttachRouteExportSummary::from_distribution_summary(distribution_summary);
        let import_summary = ThreadAttachRouteImportSummary::from_export_summary(export_summary);
        let ingest_summary = ThreadAttachRouteIngestSummary::from_import_summary(import_summary);
        let load_summary = ThreadAttachRouteLoadSummary::from_ingest_summary(ingest_summary);
        let restore_summary = ThreadAttachRouteRestoreSummary::from_load_summary(load_summary);
        let recovery_summary =
            ThreadAttachRouteRecoverySummary::from_restore_summary(restore_summary);
        let replay_summary =
            ThreadAttachRouteReplaySummary::from_recovery_summary(recovery_summary);
        let reconciliation_summary =
            ThreadAttachRouteReconciliationSummary::from_replay_summary(replay_summary);
        let settlement_summary =
            ThreadAttachRouteSettlementSummary::from_reconciliation_summary(reconciliation_summary);
        let finalization_summary =
            ThreadAttachRouteFinalizationSummary::from_settlement_summary(settlement_summary);
        let confirmation_summary =
            ThreadAttachRouteConfirmationSummary::from_finalization_summary(finalization_summary);

        let summary =
            ThreadAttachRouteAttestationSummary::from_confirmation_summary(confirmation_summary);

        assert_eq!(summary.confirmation_summary, confirmation_summary);
        assert_eq!(summary.required_route_attestation_check_count, 29);
        assert_eq!(summary.passed_route_attestation_check_count, 0);
        assert_eq!(summary.missing_route_attestation_check_count, 29);
        assert!(!summary.route_confirmation_ready);
        assert!(!summary.route_finalization_ready);
        assert!(!summary.route_settlement_ready);
        assert!(!summary.route_reconciliation_ready);
        assert!(!summary.route_replay_ready);
        assert!(!summary.route_recovery_ready);
        assert!(!summary.route_restore_ready);
        assert!(!summary.route_load_ready);
        assert!(!summary.route_ingest_ready);
        assert!(!summary.route_import_ready);
        assert!(!summary.route_export_ready);
        assert!(!summary.route_distribution_ready);
        assert!(!summary.route_acceptance_ready);
        assert!(!summary.route_adoption_ready);
        assert!(!summary.route_rollout_ready);
        assert!(!summary.route_activation_ready);
        assert!(!summary.route_approval_ready);
        assert!(!summary.route_certification_ready);
        assert!(!summary.route_validation_ready);
        assert!(!summary.route_verification_ready);
        assert!(!summary.route_publication_ready);
        assert!(!summary.route_completion_ready);
        assert!(!summary.route_signoff_ready);
        assert!(!summary.route_audit_ready);
        assert!(!summary.route_handoff_ready);
        assert!(!summary.attach_complete);
        assert!(!summary.network_data_ready);
        assert!(!summary.routing_surface_ready);
        assert!(!summary.parent_or_route_anchor_ready);
        assert!(!summary.route_attestation_ready);
        assert!(!summary.is_route_attestation_ready());
        assert!(summary.has_attestation_gaps());
        assert!(summary.needs_route_confirmation());
        assert!(summary.needs_route_finalization());
        assert!(summary.needs_route_settlement());
        assert!(summary.needs_route_reconciliation());
        assert!(summary.needs_route_replay());
        assert!(summary.needs_route_recovery());
        assert!(summary.needs_route_restore());
        assert!(summary.needs_route_load());
        assert!(summary.needs_route_ingest());
        assert!(summary.needs_route_import());
        assert!(summary.needs_route_export());
        assert!(summary.needs_route_distribution());
        assert!(summary.needs_route_acceptance());
        assert!(summary.needs_route_adoption());
        assert!(summary.needs_route_rollout());
        assert!(summary.needs_route_activation());
        assert!(summary.needs_route_approval());
        assert!(summary.needs_route_certification());
        assert!(summary.needs_route_validation());
        assert!(summary.needs_route_verification());
        assert!(summary.needs_route_publication());
        assert!(summary.needs_route_completion());
        assert!(summary.needs_route_signoff());
        assert!(summary.needs_route_audit());
        assert!(summary.needs_route_handoff());
        assert!(summary.needs_attach_completion());
        assert!(summary.needs_network_data());
        assert!(summary.needs_routing_surface());
        assert!(summary.needs_parent_or_route_anchor());
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
