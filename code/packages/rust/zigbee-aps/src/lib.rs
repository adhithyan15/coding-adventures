//! Zigbee Application Support Sublayer primitives.
//!
//! APS is where endpoint addressing, group addressing, cluster/profile ids,
//! counters, and delivery-mode flags appear before ZDO/ZCL semantics take over.
//! This crate only owns those bytes and validation rules.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use zigbee_nwk::{IeeeAddress, NetworkAddress};

const FRAME_CONTROL_LEN: usize = 1;
const ENDPOINT_LEN: usize = 1;
const GROUP_ADDRESS_LEN: usize = 2;
const CLUSTER_ID_LEN: usize = 2;
const PROFILE_ID_LEN: usize = 2;
const COUNTER_LEN: usize = 1;
const APS_COMMAND_ID_LEN: usize = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Endpoint(pub u8);

impl Endpoint {
    pub const ZDO: Self = Self(0);
    pub const MIN_APPLICATION: Self = Self(1);
    pub const MAX_APPLICATION: Self = Self(240);

    pub fn is_application(self) -> bool {
        (Self::MIN_APPLICATION.0..=Self::MAX_APPLICATION.0).contains(&self.0)
    }

    pub fn is_zdo(self) -> bool {
        self == Self::ZDO
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GroupAddress(pub u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClusterId(pub u16);

impl ClusterId {
    pub const BASIC: Self = Self(0x0000);
    pub const ON_OFF: Self = Self(0x0006);
    pub const LEVEL_CONTROL: Self = Self(0x0008);
    pub const TEMPERATURE_MEASUREMENT: Self = Self(0x0402);
    pub const OCCUPANCY_SENSING: Self = Self(0x0406);

    pub fn kind(self) -> ClusterKind {
        match self {
            Self::BASIC | Self::ON_OFF | Self::LEVEL_CONTROL => ClusterKind::General,
            Self::TEMPERATURE_MEASUREMENT | Self::OCCUPANCY_SENSING => {
                ClusterKind::MeasurementAndSensing
            }
            Self(0xfc00..=0xffff) => ClusterKind::ManufacturerSpecific,
            _ => ClusterKind::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProfileId(pub u16);

impl ProfileId {
    pub const ZIGBEE_DEVICE_PROFILE: Self = Self(0x0000);
    pub const HOME_AUTOMATION: Self = Self(0x0104);

    pub fn kind(self) -> ProfileKind {
        match self {
            Self::ZIGBEE_DEVICE_PROFILE => ProfileKind::ZigbeeDeviceProfile,
            Self::HOME_AUTOMATION => ProfileKind::HomeAutomation,
            Self(0xc000..=0xffff) => ProfileKind::ManufacturerSpecific,
            _ => ProfileKind::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileKind {
    ZigbeeDeviceProfile,
    HomeAutomation,
    ManufacturerSpecific,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClusterKind {
    General,
    MeasurementAndSensing,
    ManufacturerSpecific,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApsFrameType {
    Data,
    Command,
    Ack,
    InterPan,
}

impl ApsFrameType {
    fn from_bits(bits: u8) -> Self {
        match bits & 0b11 {
            0 => Self::Data,
            1 => Self::Command,
            2 => Self::Ack,
            _ => Self::InterPan,
        }
    }

    fn bits(self) -> u8 {
        match self {
            Self::Data => 0,
            Self::Command => 1,
            Self::Ack => 2,
            Self::InterPan => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryMode {
    Unicast,
    Indirect,
    Broadcast,
    Group,
}

impl DeliveryMode {
    fn from_bits(bits: u8) -> Self {
        match bits & 0b11 {
            0 => Self::Unicast,
            1 => Self::Indirect,
            2 => Self::Broadcast,
            _ => Self::Group,
        }
    }

    fn bits(self) -> u8 {
        match self {
            Self::Unicast => 0,
            Self::Indirect => 1,
            Self::Broadcast => 2,
            Self::Group => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApsFrameControl {
    pub frame_type: ApsFrameType,
    pub delivery_mode: DeliveryMode,
    pub ack_format: bool,
    pub security: bool,
    pub ack_request: bool,
    pub extended_header: bool,
}

impl ApsFrameControl {
    pub fn parse(raw: u8) -> Self {
        Self {
            frame_type: ApsFrameType::from_bits(raw),
            delivery_mode: DeliveryMode::from_bits(raw >> 2),
            ack_format: raw & (1 << 4) != 0,
            security: raw & (1 << 5) != 0,
            ack_request: raw & (1 << 6) != 0,
            extended_header: raw & (1 << 7) != 0,
        }
    }

    pub fn encode(self) -> u8 {
        self.frame_type.bits()
            | (self.delivery_mode.bits() << 2)
            | ((self.ack_format as u8) << 4)
            | ((self.security as u8) << 5)
            | ((self.ack_request as u8) << 6)
            | ((self.extended_header as u8) << 7)
    }

    pub fn data_unicast() -> Self {
        Self {
            frame_type: ApsFrameType::Data,
            delivery_mode: DeliveryMode::Unicast,
            ack_format: false,
            security: false,
            ack_request: false,
            extended_header: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApsAddressing {
    Unicast {
        destination_endpoint: Endpoint,
        source_endpoint: Endpoint,
    },
    Group {
        group: GroupAddress,
        source_endpoint: Endpoint,
    },
    Broadcast {
        destination_endpoint: Endpoint,
        source_endpoint: Endpoint,
    },
    Indirect {
        source_endpoint: Endpoint,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApsFrame {
    pub frame_control: ApsFrameControl,
    pub addressing: ApsAddressing,
    pub cluster_id: ClusterId,
    pub profile_id: ProfileId,
    pub counter: u8,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApsFrameSummary {
    pub frame_type: ApsFrameType,
    pub delivery_mode: DeliveryMode,
    pub profile_kind: ProfileKind,
    pub cluster_kind: ClusterKind,
    pub source_endpoint: Endpoint,
    pub destination_endpoint: Option<Endpoint>,
    pub group: Option<GroupAddress>,
    pub counter: u8,
    pub payload_len: usize,
    pub ack_request: bool,
    pub security: bool,
}

impl ApsFrameSummary {
    pub fn is_home_automation(self) -> bool {
        self.profile_kind == ProfileKind::HomeAutomation
    }

    pub fn is_group_delivery(self) -> bool {
        self.delivery_mode == DeliveryMode::Group
    }

    pub fn requires_ack(self) -> bool {
        self.ack_request
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ApsFrameBatchSummary {
    pub total_frames: usize,
    pub data_frames: usize,
    pub command_frames: usize,
    pub ack_frames: usize,
    pub inter_pan_frames: usize,
    pub unicast_frames: usize,
    pub indirect_frames: usize,
    pub broadcast_frames: usize,
    pub group_frames: usize,
    pub home_automation_frames: usize,
    pub zdo_profile_frames: usize,
    pub manufacturer_specific_profile_frames: usize,
    pub unknown_profile_frames: usize,
    pub general_cluster_frames: usize,
    pub measurement_and_sensing_frames: usize,
    pub manufacturer_specific_cluster_frames: usize,
    pub unknown_cluster_frames: usize,
    pub ack_request_frames: usize,
    pub secured_frames: usize,
    pub payload_bytes: usize,
}

impl ApsFrameBatchSummary {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_frames<'a>(frames: impl IntoIterator<Item = &'a ApsFrame>) -> Self {
        let mut summary = Self::empty();
        for frame in frames {
            summary.record_summary(frame.summary());
        }
        summary
    }

    pub fn from_summaries(summaries: impl IntoIterator<Item = ApsFrameSummary>) -> Self {
        let mut summary = Self::empty();
        for frame_summary in summaries {
            summary.record_summary(frame_summary);
        }
        summary
    }

    pub fn record_summary(&mut self, summary: ApsFrameSummary) {
        self.total_frames += 1;
        self.payload_bytes += summary.payload_len;

        match summary.frame_type {
            ApsFrameType::Data => self.data_frames += 1,
            ApsFrameType::Command => self.command_frames += 1,
            ApsFrameType::Ack => self.ack_frames += 1,
            ApsFrameType::InterPan => self.inter_pan_frames += 1,
        }

        match summary.delivery_mode {
            DeliveryMode::Unicast => self.unicast_frames += 1,
            DeliveryMode::Indirect => self.indirect_frames += 1,
            DeliveryMode::Broadcast => self.broadcast_frames += 1,
            DeliveryMode::Group => self.group_frames += 1,
        }

        match summary.profile_kind {
            ProfileKind::ZigbeeDeviceProfile => self.zdo_profile_frames += 1,
            ProfileKind::HomeAutomation => self.home_automation_frames += 1,
            ProfileKind::ManufacturerSpecific => self.manufacturer_specific_profile_frames += 1,
            ProfileKind::Unknown => self.unknown_profile_frames += 1,
        }

        match summary.cluster_kind {
            ClusterKind::General => self.general_cluster_frames += 1,
            ClusterKind::MeasurementAndSensing => self.measurement_and_sensing_frames += 1,
            ClusterKind::ManufacturerSpecific => self.manufacturer_specific_cluster_frames += 1,
            ClusterKind::Unknown => self.unknown_cluster_frames += 1,
        }

        if summary.ack_request {
            self.ack_request_frames += 1;
        }
        if summary.security {
            self.secured_frames += 1;
        }
    }

    pub fn is_empty(self) -> bool {
        self.total_frames == 0
    }

    pub fn has_group_delivery(self) -> bool {
        self.group_frames > 0
    }

    pub fn has_broadcast_delivery(self) -> bool {
        self.broadcast_frames > 0
    }

    pub fn has_ack_requests(self) -> bool {
        self.ack_request_frames > 0
    }

    pub fn has_secured_frames(self) -> bool {
        self.secured_frames > 0
    }

    pub fn carries_payloads(self) -> bool {
        self.payload_bytes > 0
    }

    pub fn has_application_delivery(self) -> bool {
        self.unicast_frames > 0 || self.group_frames > 0
    }

    pub fn has_home_automation_context(self) -> bool {
        self.home_automation_frames > 0
    }

    pub fn has_cluster_context(self) -> bool {
        self.general_cluster_frames > 0 || self.measurement_and_sensing_frames > 0
    }

    pub fn readiness(self) -> ApsFrameBatchReadinessSummary {
        ApsFrameBatchReadinessSummary::from_summary(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApsFrameBatchReadinessSummary {
    pub batch_summary: ApsFrameBatchSummary,
    pub required_check_count: usize,
    pub passed_check_count: usize,
    pub missing_check_count: usize,
    pub frames_present: bool,
    pub data_frames_present: bool,
    pub application_delivery_ready: bool,
    pub home_automation_context_ready: bool,
    pub cluster_context_ready: bool,
    pub payload_context_ready: bool,
    pub frame_batch_ready: bool,
}

impl ApsFrameBatchReadinessSummary {
    pub fn from_summary(batch_summary: ApsFrameBatchSummary) -> Self {
        let frames_present = !batch_summary.is_empty();
        let data_frames_present = batch_summary.data_frames > 0;
        let application_delivery_ready = batch_summary.has_application_delivery();
        let home_automation_context_ready = batch_summary.has_home_automation_context();
        let cluster_context_ready = batch_summary.has_cluster_context();
        let payload_context_ready = batch_summary.carries_payloads();
        let checks = [
            frames_present,
            data_frames_present,
            application_delivery_ready,
            home_automation_context_ready,
            cluster_context_ready,
            payload_context_ready,
        ];
        let passed_check_count = checks.iter().filter(|ready| **ready).count();
        let required_check_count = checks.len();
        let missing_check_count = required_check_count - passed_check_count;
        let frame_batch_ready = missing_check_count == 0;

        Self {
            batch_summary,
            required_check_count,
            passed_check_count,
            missing_check_count,
            frames_present,
            data_frames_present,
            application_delivery_ready,
            home_automation_context_ready,
            cluster_context_ready,
            payload_context_ready,
            frame_batch_ready,
        }
    }

    pub fn is_frame_batch_ready(self) -> bool {
        self.frame_batch_ready
    }

    pub fn has_missing_checks(self) -> bool {
        self.missing_check_count > 0
    }

    pub fn needs_frames(self) -> bool {
        !self.frames_present
    }

    pub fn needs_data_frames(self) -> bool {
        !self.data_frames_present
    }

    pub fn needs_application_delivery(self) -> bool {
        !self.application_delivery_ready
    }

    pub fn needs_home_automation_context(self) -> bool {
        !self.home_automation_context_ready
    }

    pub fn needs_cluster_context(self) -> bool {
        !self.cluster_context_ready
    }

    pub fn needs_payload_context(self) -> bool {
        !self.payload_context_ready
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApsDeliveryHandoffSummary {
    pub readiness_summary: ApsFrameBatchReadinessSummary,
    pub required_handoff_check_count: usize,
    pub passed_handoff_check_count: usize,
    pub blocked_handoff_check_count: usize,
    pub frame_batch_ready: bool,
    pub application_delivery_ready: bool,
    pub payload_context_ready: bool,
    pub security_or_ack_context_present: bool,
    pub delivery_handoff_ready: bool,
}

impl ApsDeliveryHandoffSummary {
    pub fn from_readiness(readiness_summary: ApsFrameBatchReadinessSummary) -> Self {
        let frame_batch_ready = readiness_summary.is_frame_batch_ready();
        let application_delivery_ready = !readiness_summary.needs_application_delivery();
        let payload_context_ready = !readiness_summary.needs_payload_context();
        let security_or_ack_context_present = readiness_summary.batch_summary.has_secured_frames()
            || readiness_summary.batch_summary.has_ack_requests();
        let checks = [
            frame_batch_ready,
            application_delivery_ready,
            payload_context_ready,
            security_or_ack_context_present,
        ];
        let passed_handoff_check_count = checks.iter().filter(|ready| **ready).count();
        let required_handoff_check_count = checks.len();
        let blocked_handoff_check_count = required_handoff_check_count - passed_handoff_check_count;
        let delivery_handoff_ready = blocked_handoff_check_count == 0;

        Self {
            readiness_summary,
            required_handoff_check_count,
            passed_handoff_check_count,
            blocked_handoff_check_count,
            frame_batch_ready,
            application_delivery_ready,
            payload_context_ready,
            security_or_ack_context_present,
            delivery_handoff_ready,
        }
    }

    pub fn from_batch_summary(batch_summary: ApsFrameBatchSummary) -> Self {
        Self::from_readiness(batch_summary.readiness())
    }

    pub fn is_delivery_handoff_ready(self) -> bool {
        self.delivery_handoff_ready
    }

    pub fn has_blocked_handoff_checks(self) -> bool {
        self.blocked_handoff_check_count > 0
    }

    pub fn needs_frame_batch(self) -> bool {
        !self.frame_batch_ready
    }

    pub fn needs_application_delivery(self) -> bool {
        !self.application_delivery_ready
    }

    pub fn needs_payload_context(self) -> bool {
        !self.payload_context_ready
    }

    pub fn needs_security_or_ack_context(self) -> bool {
        !self.security_or_ack_context_present
    }
}

pub fn summarize_aps_delivery_handoff(
    readiness_summary: ApsFrameBatchReadinessSummary,
) -> ApsDeliveryHandoffSummary {
    ApsDeliveryHandoffSummary::from_readiness(readiness_summary)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ApsCommandId(pub u8);

impl ApsCommandId {
    pub const TRANSPORT_KEY: Self = Self(0x05);
    pub const UPDATE_DEVICE: Self = Self(0x06);
    pub const REMOVE_DEVICE: Self = Self(0x07);
    pub const REQUEST_KEY: Self = Self(0x08);
    pub const SWITCH_KEY: Self = Self(0x09);

    pub fn is_key_management(self) -> bool {
        matches!(
            self,
            Self::TRANSPORT_KEY
                | Self::UPDATE_DEVICE
                | Self::REMOVE_DEVICE
                | Self::REQUEST_KEY
                | Self::SWITCH_KEY
        )
    }

    pub fn name(self) -> Option<&'static str> {
        match self {
            Self::TRANSPORT_KEY => Some("transport-key"),
            Self::UPDATE_DEVICE => Some("update-device"),
            Self::REMOVE_DEVICE => Some("remove-device"),
            Self::REQUEST_KEY => Some("request-key"),
            Self::SWITCH_KEY => Some("switch-key"),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApsCommandFrame {
    pub command_id: ApsCommandId,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApsCommandSummary {
    pub command_id: ApsCommandId,
    pub command_name: Option<&'static str>,
    pub payload_len: usize,
    pub key_management: bool,
}

impl ApsCommandFrame {
    pub fn new(command_id: ApsCommandId, payload: Vec<u8>) -> Self {
        Self {
            command_id,
            payload,
        }
    }

    pub fn parse(bytes: &[u8]) -> Result<Self, ApsError> {
        let mut cursor = Cursor::new(bytes);
        let command_id = ApsCommandId(cursor.read_u8()?);
        Ok(Self {
            command_id,
            payload: cursor.remaining_bytes().to_vec(),
        })
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(APS_COMMAND_ID_LEN + self.payload.len());
        out.push(self.command_id.0);
        out.extend_from_slice(&self.payload);
        out
    }

    pub fn summary(&self) -> ApsCommandSummary {
        ApsCommandSummary {
            command_id: self.command_id,
            command_name: self.command_id.name(),
            payload_len: self.payload.len(),
            key_management: self.command_id.is_key_management(),
        }
    }
}

impl ApsCommandSummary {
    pub fn is_known(self) -> bool {
        self.command_name.is_some()
    }

    pub fn is_key_management(self) -> bool {
        self.key_management
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EndpointAddress {
    pub network_address: NetworkAddress,
    pub endpoint: Endpoint,
}

impl EndpointAddress {
    pub fn new(network_address: NetworkAddress, endpoint: Endpoint) -> Self {
        Self {
            network_address,
            endpoint,
        }
    }

    pub fn coordinator_zdo() -> Self {
        Self::new(NetworkAddress::COORDINATOR, Endpoint::ZDO)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClusterEndpoint {
    pub endpoint: Endpoint,
    pub profile_id: ProfileId,
    pub cluster_id: ClusterId,
}

impl ClusterEndpoint {
    pub fn new(endpoint: Endpoint, profile_id: ProfileId, cluster_id: ClusterId) -> Self {
        Self {
            endpoint,
            profile_id,
            cluster_id,
        }
    }

    pub fn is_home_automation(self) -> bool {
        self.profile_id.kind() == ProfileKind::HomeAutomation
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BindingSource {
    pub ieee_address: IeeeAddress,
    pub endpoint: Endpoint,
}

impl BindingSource {
    pub fn new(ieee_address: IeeeAddress, endpoint: Endpoint) -> Self {
        Self {
            ieee_address,
            endpoint,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BindingDestination {
    Group(GroupAddress),
    Device {
        ieee_address: IeeeAddress,
        endpoint: Endpoint,
    },
}

impl BindingDestination {
    pub fn device(ieee_address: IeeeAddress, endpoint: Endpoint) -> Self {
        Self::Device {
            ieee_address,
            endpoint,
        }
    }

    pub fn group(group: GroupAddress) -> Self {
        Self::Group(group)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingEntry {
    pub source: BindingSource,
    pub cluster_id: ClusterId,
    pub destination: BindingDestination,
}

impl BindingEntry {
    pub fn new(
        source: BindingSource,
        cluster_id: ClusterId,
        destination: BindingDestination,
    ) -> Self {
        Self {
            source,
            cluster_id,
            destination,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct BindingTable {
    entries: BTreeMap<BindingKey, BindingEntry>,
}

impl BindingTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn upsert(&mut self, entry: BindingEntry) -> Option<BindingEntry> {
        self.entries.insert(BindingKey::from(&entry), entry)
    }

    pub fn remove(&mut self, entry: &BindingEntry) -> Option<BindingEntry> {
        self.entries.remove(&BindingKey::from(entry))
    }

    pub fn entries(&self) -> impl Iterator<Item = &BindingEntry> {
        self.entries.values()
    }

    pub fn bindings_for(
        &self,
        source: BindingSource,
        cluster_id: ClusterId,
    ) -> impl Iterator<Item = &BindingEntry> {
        self.entries
            .values()
            .filter(move |entry| entry.source == source && entry.cluster_id == cluster_id)
    }

    pub fn destinations_for(
        &self,
        source: BindingSource,
        cluster_id: ClusterId,
    ) -> Vec<BindingDestination> {
        self.bindings_for(source, cluster_id)
            .map(|entry| entry.destination)
            .collect()
    }

    pub fn groups_for(&self, source: BindingSource, cluster_id: ClusterId) -> Vec<GroupAddress> {
        self.destinations_for(source, cluster_id)
            .into_iter()
            .filter_map(|destination| match destination {
                BindingDestination::Group(group) => Some(group),
                BindingDestination::Device { .. } => None,
            })
            .collect()
    }

    pub fn device_destinations_for(
        &self,
        source: BindingSource,
        cluster_id: ClusterId,
    ) -> Vec<(IeeeAddress, Endpoint)> {
        self.destinations_for(source, cluster_id)
            .into_iter()
            .filter_map(|destination| match destination {
                BindingDestination::Device {
                    ieee_address,
                    endpoint,
                } => Some((ieee_address, endpoint)),
                BindingDestination::Group(_) => None,
            })
            .collect()
    }

    pub fn summary(&self) -> BindingTableSummary {
        let mut unique_sources = BTreeSet::new();
        let mut unique_clusters = BTreeSet::new();
        let mut unique_groups = BTreeSet::new();
        let mut unique_device_destinations = BTreeSet::new();
        let mut summary = BindingTableSummary {
            total_bindings: self.entries.len(),
            group_bindings: 0,
            device_bindings: 0,
            general_cluster_bindings: 0,
            measurement_and_sensing_bindings: 0,
            manufacturer_specific_cluster_bindings: 0,
            unknown_cluster_bindings: 0,
            unique_sources: 0,
            unique_clusters: 0,
            unique_groups: 0,
            unique_device_destinations: 0,
            zdo_source_bindings: 0,
            application_source_bindings: 0,
            non_application_source_bindings: 0,
        };

        for entry in self.entries.values() {
            unique_sources.insert(entry.source);
            unique_clusters.insert(entry.cluster_id);
            match entry.destination {
                BindingDestination::Group(group) => {
                    summary.group_bindings += 1;
                    unique_groups.insert(group);
                }
                BindingDestination::Device {
                    ieee_address,
                    endpoint,
                } => {
                    summary.device_bindings += 1;
                    unique_device_destinations.insert((ieee_address, endpoint));
                }
            }

            match entry.cluster_id.kind() {
                ClusterKind::General => summary.general_cluster_bindings += 1,
                ClusterKind::MeasurementAndSensing => {
                    summary.measurement_and_sensing_bindings += 1;
                }
                ClusterKind::ManufacturerSpecific => {
                    summary.manufacturer_specific_cluster_bindings += 1;
                }
                ClusterKind::Unknown => summary.unknown_cluster_bindings += 1,
            }

            if entry.source.endpoint.is_zdo() {
                summary.zdo_source_bindings += 1;
            } else if entry.source.endpoint.is_application() {
                summary.application_source_bindings += 1;
            } else {
                summary.non_application_source_bindings += 1;
            }
        }

        summary.unique_sources = unique_sources.len();
        summary.unique_clusters = unique_clusters.len();
        summary.unique_groups = unique_groups.len();
        summary.unique_device_destinations = unique_device_destinations.len();
        summary
    }

    pub fn readiness_summary(&self) -> BindingTableReadinessSummary {
        self.summary().readiness()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct BindingKey {
    source: BindingSource,
    cluster_id: ClusterId,
    destination: BindingDestination,
}

impl From<&BindingEntry> for BindingKey {
    fn from(entry: &BindingEntry) -> Self {
        Self {
            source: entry.source,
            cluster_id: entry.cluster_id,
            destination: entry.destination,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BindingTableSummary {
    pub total_bindings: usize,
    pub group_bindings: usize,
    pub device_bindings: usize,
    pub general_cluster_bindings: usize,
    pub measurement_and_sensing_bindings: usize,
    pub manufacturer_specific_cluster_bindings: usize,
    pub unknown_cluster_bindings: usize,
    pub unique_sources: usize,
    pub unique_clusters: usize,
    pub unique_groups: usize,
    pub unique_device_destinations: usize,
    pub zdo_source_bindings: usize,
    pub application_source_bindings: usize,
    pub non_application_source_bindings: usize,
}

impl BindingTableSummary {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn is_empty(self) -> bool {
        self.total_bindings == 0
    }

    pub fn has_bindings(self) -> bool {
        self.total_bindings > 0
    }

    pub fn has_group_bindings(self) -> bool {
        self.group_bindings > 0
    }

    pub fn has_device_bindings(self) -> bool {
        self.device_bindings > 0
    }

    pub fn has_manufacturer_specific_clusters(self) -> bool {
        self.manufacturer_specific_cluster_bindings > 0
    }

    pub fn has_zdo_sources(self) -> bool {
        self.zdo_source_bindings > 0
    }

    pub fn has_application_sources(self) -> bool {
        self.application_source_bindings > 0
    }

    pub fn has_non_application_sources(self) -> bool {
        self.non_application_source_bindings > 0
    }

    pub fn has_destination_coverage(self) -> bool {
        self.has_group_bindings() && self.has_device_bindings()
    }

    pub fn has_cluster_coverage(self) -> bool {
        self.general_cluster_bindings > 0 && self.measurement_and_sensing_bindings > 0
    }

    pub fn readiness(self) -> BindingTableReadinessSummary {
        BindingTableReadinessSummary::from_summary(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BindingTableReadinessSummary {
    pub binding_summary: BindingTableSummary,
    pub required_check_count: usize,
    pub passed_check_count: usize,
    pub missing_check_count: usize,
    pub bindings_present: bool,
    pub application_sources_ready: bool,
    pub destination_coverage_ready: bool,
    pub cluster_coverage_ready: bool,
    pub zdo_sources_absent: bool,
    pub non_application_sources_absent: bool,
    pub binding_ready: bool,
}

impl BindingTableReadinessSummary {
    pub fn from_summary(binding_summary: BindingTableSummary) -> Self {
        let bindings_present = binding_summary.has_bindings();
        let application_sources_ready = binding_summary.has_application_sources();
        let destination_coverage_ready = binding_summary.has_destination_coverage();
        let cluster_coverage_ready = binding_summary.has_cluster_coverage();
        let zdo_sources_absent = !binding_summary.has_zdo_sources();
        let non_application_sources_absent = !binding_summary.has_non_application_sources();
        let checks = [
            bindings_present,
            application_sources_ready,
            destination_coverage_ready,
            cluster_coverage_ready,
            zdo_sources_absent,
            non_application_sources_absent,
        ];
        let passed_check_count = checks.iter().filter(|ready| **ready).count();
        let required_check_count = checks.len();
        let missing_check_count = required_check_count - passed_check_count;
        let binding_ready = missing_check_count == 0;

        Self {
            binding_summary,
            required_check_count,
            passed_check_count,
            missing_check_count,
            bindings_present,
            application_sources_ready,
            destination_coverage_ready,
            cluster_coverage_ready,
            zdo_sources_absent,
            non_application_sources_absent,
            binding_ready,
        }
    }

    pub fn is_binding_ready(self) -> bool {
        self.binding_ready
    }

    pub fn has_missing_checks(self) -> bool {
        self.missing_check_count > 0
    }

    pub fn needs_binding_discovery(self) -> bool {
        !self.bindings_present
    }

    pub fn needs_application_source_binding(self) -> bool {
        !self.application_sources_ready
    }

    pub fn needs_destination_coverage(self) -> bool {
        !self.destination_coverage_ready
    }

    pub fn needs_cluster_coverage(self) -> bool {
        !self.cluster_coverage_ready
    }

    pub fn has_source_endpoint_issues(self) -> bool {
        !self.zdo_sources_absent || !self.non_application_sources_absent
    }
}

impl ApsFrame {
    pub fn unicast_data(
        destination_endpoint: Endpoint,
        source_endpoint: Endpoint,
        cluster_id: ClusterId,
        profile_id: ProfileId,
        counter: u8,
        payload: Vec<u8>,
    ) -> Self {
        Self {
            frame_control: ApsFrameControl::data_unicast(),
            addressing: ApsAddressing::Unicast {
                destination_endpoint,
                source_endpoint,
            },
            cluster_id,
            profile_id,
            counter,
            payload,
        }
    }

    pub fn parse(bytes: &[u8]) -> Result<Self, ApsError> {
        let mut cursor = Cursor::new(bytes);
        let frame_control = ApsFrameControl::parse(cursor.read_u8()?);
        let addressing = match frame_control.delivery_mode {
            DeliveryMode::Unicast => ApsAddressing::Unicast {
                destination_endpoint: Endpoint(cursor.read_u8()?),
                source_endpoint: Endpoint::ZDO,
            },
            DeliveryMode::Group => ApsAddressing::Group {
                group: GroupAddress(cursor.read_u16_le()?),
                source_endpoint: Endpoint::ZDO,
            },
            DeliveryMode::Broadcast => ApsAddressing::Broadcast {
                destination_endpoint: Endpoint(cursor.read_u8()?),
                source_endpoint: Endpoint::ZDO,
            },
            DeliveryMode::Indirect => ApsAddressing::Indirect {
                source_endpoint: Endpoint::ZDO,
            },
        };

        let cluster_id = ClusterId(cursor.read_u16_le()?);
        let profile_id = ProfileId(cursor.read_u16_le()?);
        let mut addressing = addressing;
        let counter = match &mut addressing {
            ApsAddressing::Unicast {
                source_endpoint, ..
            }
            | ApsAddressing::Group {
                source_endpoint, ..
            }
            | ApsAddressing::Broadcast {
                source_endpoint, ..
            }
            | ApsAddressing::Indirect { source_endpoint } => {
                *source_endpoint = Endpoint(cursor.read_u8()?);
                cursor.read_u8()?
            }
        };
        let payload = cursor.remaining_bytes().to_vec();

        Ok(Self {
            frame_control,
            addressing,
            cluster_id,
            profile_id,
            counter,
            payload,
        })
    }

    pub fn encode(&self) -> Result<Vec<u8>, ApsError> {
        validate_addressing(self.frame_control.delivery_mode, &self.addressing)?;

        let mut out = Vec::with_capacity(
            FRAME_CONTROL_LEN
                + GROUP_ADDRESS_LEN
                + (ENDPOINT_LEN * 2)
                + CLUSTER_ID_LEN
                + PROFILE_ID_LEN
                + COUNTER_LEN
                + self.payload.len(),
        );
        out.push(self.frame_control.encode());
        match self.addressing {
            ApsAddressing::Unicast {
                destination_endpoint,
                ..
            }
            | ApsAddressing::Broadcast {
                destination_endpoint,
                ..
            } => out.push(destination_endpoint.0),
            ApsAddressing::Group { group, .. } => out.extend_from_slice(&group.0.to_le_bytes()),
            ApsAddressing::Indirect { .. } => {}
        }
        out.extend_from_slice(&self.cluster_id.0.to_le_bytes());
        out.extend_from_slice(&self.profile_id.0.to_le_bytes());
        out.push(source_endpoint(&self.addressing).0);
        out.push(self.counter);
        out.extend_from_slice(&self.payload);
        Ok(out)
    }

    pub fn summary(&self) -> ApsFrameSummary {
        let (destination_endpoint, group) = match self.addressing {
            ApsAddressing::Unicast {
                destination_endpoint,
                ..
            }
            | ApsAddressing::Broadcast {
                destination_endpoint,
                ..
            } => (Some(destination_endpoint), None),
            ApsAddressing::Group { group, .. } => (None, Some(group)),
            ApsAddressing::Indirect { .. } => (None, None),
        };

        ApsFrameSummary {
            frame_type: self.frame_control.frame_type,
            delivery_mode: self.frame_control.delivery_mode,
            profile_kind: self.profile_id.kind(),
            cluster_kind: self.cluster_id.kind(),
            source_endpoint: source_endpoint(&self.addressing),
            destination_endpoint,
            group,
            counter: self.counter,
            payload_len: self.payload.len(),
            ack_request: self.frame_control.ack_request,
            security: self.frame_control.security,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApsError {
    Truncated { needed: usize, remaining: usize },
    DeliveryModeMismatch,
}

impl fmt::Display for ApsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated { needed, remaining } => write!(
                f,
                "truncated Zigbee APS frame: needed {needed} bytes, had {remaining}"
            ),
            Self::DeliveryModeMismatch => write!(f, "APS delivery mode does not match addressing"),
        }
    }
}

impl std::error::Error for ApsError {}

fn source_endpoint(addressing: &ApsAddressing) -> Endpoint {
    match addressing {
        ApsAddressing::Unicast {
            source_endpoint, ..
        }
        | ApsAddressing::Group {
            source_endpoint, ..
        }
        | ApsAddressing::Broadcast {
            source_endpoint, ..
        }
        | ApsAddressing::Indirect { source_endpoint } => *source_endpoint,
    }
}

fn validate_addressing(
    delivery_mode: DeliveryMode,
    addressing: &ApsAddressing,
) -> Result<(), ApsError> {
    let ok = matches!(
        (delivery_mode, addressing),
        (DeliveryMode::Unicast, ApsAddressing::Unicast { .. })
            | (DeliveryMode::Group, ApsAddressing::Group { .. })
            | (DeliveryMode::Broadcast, ApsAddressing::Broadcast { .. })
            | (DeliveryMode::Indirect, ApsAddressing::Indirect { .. })
    );
    if ok {
        Ok(())
    } else {
        Err(ApsError::DeliveryModeMismatch)
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

    fn read_u8(&mut self) -> Result<u8, ApsError> {
        let remaining = self.bytes.len().saturating_sub(self.pos);
        if remaining < 1 {
            return Err(ApsError::Truncated {
                needed: 1,
                remaining,
            });
        }
        let value = self.bytes[self.pos];
        self.pos += 1;
        Ok(value)
    }

    fn read_u16_le(&mut self) -> Result<u16, ApsError> {
        let remaining = self.bytes.len().saturating_sub(self.pos);
        if remaining < 2 {
            return Err(ApsError::Truncated {
                needed: 2,
                remaining,
            });
        }
        let value = u16::from_le_bytes([self.bytes[self.pos], self.bytes[self.pos + 1]]);
        self.pos += 2;
        Ok(value)
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
        let control = ApsFrameControl {
            frame_type: ApsFrameType::Data,
            delivery_mode: DeliveryMode::Group,
            ack_format: true,
            security: true,
            ack_request: true,
            extended_header: false,
        };

        assert_eq!(ApsFrameControl::parse(control.encode()), control);
    }

    #[test]
    fn unicast_data_frame_round_trips() {
        let frame = ApsFrame::unicast_data(
            Endpoint(1),
            Endpoint(2),
            ClusterId::ON_OFF,
            ProfileId::HOME_AUTOMATION,
            7,
            vec![0x01, 0x02],
        );

        assert_eq!(ApsFrame::parse(&frame.encode().unwrap()).unwrap(), frame);
    }

    #[test]
    fn aps_command_frame_round_trips_key_management_payloads() {
        let frame = ApsCommandFrame::new(ApsCommandId::TRANSPORT_KEY, vec![0xaa, 0xbb, 0xcc]);

        let parsed = ApsCommandFrame::parse(&frame.encode()).unwrap();
        let summary = parsed.summary();

        assert_eq!(parsed, frame);
        assert_eq!(summary.command_id, ApsCommandId::TRANSPORT_KEY);
        assert_eq!(summary.command_name, Some("transport-key"));
        assert_eq!(summary.payload_len, 3);
        assert!(summary.is_known());
        assert!(summary.is_key_management());
    }

    #[test]
    fn aps_command_frame_preserves_unknown_command_payloads() {
        let parsed = ApsCommandFrame::parse(&[0xfe, 0x01, 0x02]).unwrap();
        let summary = parsed.summary();

        assert_eq!(parsed.command_id, ApsCommandId(0xfe));
        assert_eq!(parsed.payload, vec![0x01, 0x02]);
        assert_eq!(parsed.encode(), vec![0xfe, 0x01, 0x02]);
        assert_eq!(summary.command_name, None);
        assert!(!summary.is_known());
        assert!(!summary.is_key_management());
    }

    #[test]
    fn aps_command_frame_rejects_empty_payloads() {
        assert_eq!(
            ApsCommandFrame::parse(&[]),
            Err(ApsError::Truncated {
                needed: 1,
                remaining: 0
            })
        );
    }

    #[test]
    fn group_frame_round_trips() {
        let mut control = ApsFrameControl::data_unicast();
        control.delivery_mode = DeliveryMode::Group;
        let frame = ApsFrame {
            frame_control: control,
            addressing: ApsAddressing::Group {
                group: GroupAddress(0x1234),
                source_endpoint: Endpoint(1),
            },
            cluster_id: ClusterId::LEVEL_CONTROL,
            profile_id: ProfileId::HOME_AUTOMATION,
            counter: 9,
            payload: vec![0x05],
        };

        assert_eq!(ApsFrame::parse(&frame.encode().unwrap()).unwrap(), frame);
    }

    #[test]
    fn rejects_delivery_mode_addressing_mismatch() {
        let mut frame = ApsFrame::unicast_data(
            Endpoint(1),
            Endpoint(2),
            ClusterId::ON_OFF,
            ProfileId::HOME_AUTOMATION,
            1,
            Vec::new(),
        );
        frame.frame_control.delivery_mode = DeliveryMode::Group;

        assert_eq!(frame.encode(), Err(ApsError::DeliveryModeMismatch));
    }

    #[test]
    fn endpoint_knows_application_range() {
        assert!(!Endpoint::ZDO.is_application());
        assert!(Endpoint::ZDO.is_zdo());
        assert!(Endpoint(1).is_application());
        assert!(Endpoint(240).is_application());
        assert!(!Endpoint(241).is_application());
    }

    #[test]
    fn profile_and_cluster_ids_are_classified() {
        assert_eq!(
            ProfileId::ZIGBEE_DEVICE_PROFILE.kind(),
            ProfileKind::ZigbeeDeviceProfile
        );
        assert_eq!(
            ProfileId::HOME_AUTOMATION.kind(),
            ProfileKind::HomeAutomation
        );
        assert_eq!(ProfileId(0xc001).kind(), ProfileKind::ManufacturerSpecific);
        assert_eq!(ClusterId::ON_OFF.kind(), ClusterKind::General);
        assert_eq!(
            ClusterId::OCCUPANCY_SENSING.kind(),
            ClusterKind::MeasurementAndSensing
        );
        assert_eq!(ClusterId(0xfc00).kind(), ClusterKind::ManufacturerSpecific);
    }

    #[test]
    fn endpoint_addresses_keep_nwk_and_aps_identity_together() {
        let address = EndpointAddress::new(NetworkAddress(0x1234), Endpoint(11));
        assert_eq!(address.network_address, NetworkAddress(0x1234));
        assert_eq!(address.endpoint, Endpoint(11));
        assert_eq!(EndpointAddress::coordinator_zdo().endpoint, Endpoint::ZDO);

        let cluster_endpoint =
            ClusterEndpoint::new(Endpoint(1), ProfileId::HOME_AUTOMATION, ClusterId::ON_OFF);
        assert!(cluster_endpoint.is_home_automation());
    }

    #[test]
    fn binding_table_tracks_device_and_group_destinations() {
        let source = BindingSource::new(IeeeAddress(0x0012_4b00_0000_0001), Endpoint(1));
        let device_destination =
            BindingDestination::device(IeeeAddress(0x0012_4b00_0000_0002), Endpoint(2));
        let group_destination = BindingDestination::group(GroupAddress(0x1234));
        let mut table = BindingTable::new();

        assert!(table.is_empty());
        assert_eq!(
            table.upsert(BindingEntry::new(
                source,
                ClusterId::ON_OFF,
                device_destination
            )),
            None
        );
        assert_eq!(
            table.upsert(BindingEntry::new(
                source,
                ClusterId::ON_OFF,
                group_destination
            )),
            None
        );

        assert_eq!(table.len(), 2);
        assert_eq!(
            table.destinations_for(source, ClusterId::ON_OFF),
            vec![group_destination, device_destination]
        );
        assert_eq!(
            table.groups_for(source, ClusterId::ON_OFF),
            vec![GroupAddress(0x1234)]
        );
        assert_eq!(
            table.device_destinations_for(source, ClusterId::ON_OFF),
            vec![(IeeeAddress(0x0012_4b00_0000_0002), Endpoint(2))]
        );

        let removed = table
            .remove(&BindingEntry::new(
                source,
                ClusterId::ON_OFF,
                device_destination,
            ))
            .unwrap();
        assert_eq!(removed.destination, device_destination);
        assert_eq!(table.len(), 1);
    }

    #[test]
    fn aps_frame_summary_omits_payload_but_keeps_routing_fields() {
        let mut control = ApsFrameControl::data_unicast();
        control.delivery_mode = DeliveryMode::Group;
        control.ack_request = true;
        control.security = true;
        let frame = ApsFrame {
            frame_control: control,
            addressing: ApsAddressing::Group {
                group: GroupAddress(0x1234),
                source_endpoint: Endpoint(1),
            },
            cluster_id: ClusterId::LEVEL_CONTROL,
            profile_id: ProfileId::HOME_AUTOMATION,
            counter: 9,
            payload: vec![0x05, 0x06],
        };

        let summary = frame.summary();

        assert_eq!(summary.frame_type, ApsFrameType::Data);
        assert_eq!(summary.delivery_mode, DeliveryMode::Group);
        assert_eq!(summary.profile_kind, ProfileKind::HomeAutomation);
        assert_eq!(summary.cluster_kind, ClusterKind::General);
        assert_eq!(summary.source_endpoint, Endpoint(1));
        assert_eq!(summary.destination_endpoint, None);
        assert_eq!(summary.group, Some(GroupAddress(0x1234)));
        assert_eq!(summary.counter, 9);
        assert_eq!(summary.payload_len, 2);
        assert!(summary.is_home_automation());
        assert!(summary.is_group_delivery());
        assert!(summary.requires_ack());
        assert!(summary.security);
    }

    #[test]
    fn aps_frame_batch_summary_rolls_up_delivery_and_payload_context() {
        let unicast = ApsFrame::unicast_data(
            Endpoint(1),
            Endpoint(2),
            ClusterId::ON_OFF,
            ProfileId::HOME_AUTOMATION,
            7,
            vec![0x01, 0x02],
        );
        let mut group_control = ApsFrameControl::data_unicast();
        group_control.delivery_mode = DeliveryMode::Group;
        group_control.ack_request = true;
        group_control.security = true;
        let group = ApsFrame {
            frame_control: group_control,
            addressing: ApsAddressing::Group {
                group: GroupAddress(0x1234),
                source_endpoint: Endpoint(3),
            },
            cluster_id: ClusterId::TEMPERATURE_MEASUREMENT,
            profile_id: ProfileId::HOME_AUTOMATION,
            counter: 8,
            payload: vec![0x03],
        };
        let mut command_control = ApsFrameControl::data_unicast();
        command_control.frame_type = ApsFrameType::Command;
        command_control.delivery_mode = DeliveryMode::Broadcast;
        let command = ApsFrame {
            frame_control: command_control,
            addressing: ApsAddressing::Broadcast {
                destination_endpoint: Endpoint(255),
                source_endpoint: Endpoint::ZDO,
            },
            cluster_id: ClusterId(0xfc00),
            profile_id: ProfileId::ZIGBEE_DEVICE_PROFILE,
            counter: 9,
            payload: Vec::new(),
        };

        let summary = ApsFrameBatchSummary::from_frames([&unicast, &group, &command]);

        assert_eq!(summary.total_frames, 3);
        assert_eq!(summary.data_frames, 2);
        assert_eq!(summary.command_frames, 1);
        assert_eq!(summary.unicast_frames, 1);
        assert_eq!(summary.broadcast_frames, 1);
        assert_eq!(summary.group_frames, 1);
        assert_eq!(summary.home_automation_frames, 2);
        assert_eq!(summary.zdo_profile_frames, 1);
        assert_eq!(summary.general_cluster_frames, 1);
        assert_eq!(summary.measurement_and_sensing_frames, 1);
        assert_eq!(summary.manufacturer_specific_cluster_frames, 1);
        assert_eq!(summary.ack_request_frames, 1);
        assert_eq!(summary.secured_frames, 1);
        assert_eq!(summary.payload_bytes, 3);
        assert!(summary.has_group_delivery());
        assert!(summary.has_broadcast_delivery());
        assert!(summary.has_ack_requests());
        assert!(summary.has_secured_frames());
        assert!(summary.carries_payloads());
    }

    #[test]
    fn aps_frame_batch_summary_handles_precomputed_and_empty_summaries() {
        let empty = ApsFrameBatchSummary::empty();
        assert!(empty.is_empty());
        assert!(!empty.carries_payloads());

        let summary = ApsFrameBatchSummary::from_summaries([
            ApsFrameSummary {
                frame_type: ApsFrameType::Ack,
                delivery_mode: DeliveryMode::Indirect,
                profile_kind: ProfileKind::ManufacturerSpecific,
                cluster_kind: ClusterKind::Unknown,
                source_endpoint: Endpoint(2),
                destination_endpoint: None,
                group: None,
                counter: 1,
                payload_len: 0,
                ack_request: false,
                security: false,
            },
            ApsFrameSummary {
                frame_type: ApsFrameType::InterPan,
                delivery_mode: DeliveryMode::Unicast,
                profile_kind: ProfileKind::Unknown,
                cluster_kind: ClusterKind::ManufacturerSpecific,
                source_endpoint: Endpoint(3),
                destination_endpoint: Some(Endpoint(4)),
                group: None,
                counter: 2,
                payload_len: 4,
                ack_request: true,
                security: false,
            },
        ]);

        assert_eq!(summary.total_frames, 2);
        assert_eq!(summary.ack_frames, 1);
        assert_eq!(summary.inter_pan_frames, 1);
        assert_eq!(summary.indirect_frames, 1);
        assert_eq!(summary.unicast_frames, 1);
        assert_eq!(summary.manufacturer_specific_profile_frames, 1);
        assert_eq!(summary.unknown_profile_frames, 1);
        assert_eq!(summary.manufacturer_specific_cluster_frames, 1);
        assert_eq!(summary.unknown_cluster_frames, 1);
        assert_eq!(summary.payload_bytes, 4);
        assert!(summary.has_ack_requests());
        assert!(!summary.has_secured_frames());
    }

    #[test]
    fn aps_frame_batch_readiness_summary_marks_application_capture_ready() {
        let unicast = ApsFrame::unicast_data(
            Endpoint(1),
            Endpoint(2),
            ClusterId::ON_OFF,
            ProfileId::HOME_AUTOMATION,
            7,
            vec![0x01],
        );
        let group = ApsFrame {
            frame_control: ApsFrameControl {
                delivery_mode: DeliveryMode::Group,
                ..ApsFrameControl::data_unicast()
            },
            addressing: ApsAddressing::Group {
                group: GroupAddress(0x1234),
                source_endpoint: Endpoint(3),
            },
            cluster_id: ClusterId::TEMPERATURE_MEASUREMENT,
            profile_id: ProfileId::HOME_AUTOMATION,
            counter: 8,
            payload: vec![0x02, 0x03],
        };
        let summary = ApsFrameBatchSummary::from_frames([&unicast, &group]);

        let readiness = summary.readiness();

        assert_eq!(readiness.batch_summary, summary);
        assert_eq!(readiness.required_check_count, 6);
        assert_eq!(readiness.passed_check_count, 6);
        assert_eq!(readiness.missing_check_count, 0);
        assert!(readiness.frames_present);
        assert!(readiness.data_frames_present);
        assert!(readiness.application_delivery_ready);
        assert!(readiness.home_automation_context_ready);
        assert!(readiness.cluster_context_ready);
        assert!(readiness.payload_context_ready);
        assert!(readiness.frame_batch_ready);
        assert!(readiness.is_frame_batch_ready());
        assert!(!readiness.has_missing_checks());
        assert!(!readiness.needs_frames());
        assert!(!readiness.needs_data_frames());
        assert!(!readiness.needs_application_delivery());
        assert!(!readiness.needs_home_automation_context());
        assert!(!readiness.needs_cluster_context());
        assert!(!readiness.needs_payload_context());
    }

    #[test]
    fn aps_frame_batch_readiness_summary_routes_sparse_capture_gaps() {
        let command_only = ApsFrameSummary {
            frame_type: ApsFrameType::Command,
            delivery_mode: DeliveryMode::Broadcast,
            profile_kind: ProfileKind::ZigbeeDeviceProfile,
            cluster_kind: ClusterKind::ManufacturerSpecific,
            source_endpoint: Endpoint::ZDO,
            destination_endpoint: Some(Endpoint(255)),
            group: None,
            counter: 1,
            payload_len: 0,
            ack_request: false,
            security: false,
        };

        let readiness = ApsFrameBatchSummary::from_summaries([command_only]).readiness();

        assert_eq!(readiness.required_check_count, 6);
        assert_eq!(readiness.passed_check_count, 1);
        assert_eq!(readiness.missing_check_count, 5);
        assert!(readiness.frames_present);
        assert!(!readiness.data_frames_present);
        assert!(!readiness.application_delivery_ready);
        assert!(!readiness.home_automation_context_ready);
        assert!(!readiness.cluster_context_ready);
        assert!(!readiness.payload_context_ready);
        assert!(!readiness.frame_batch_ready);
        assert!(!readiness.needs_frames());
        assert!(readiness.needs_data_frames());
        assert!(readiness.needs_application_delivery());
        assert!(readiness.needs_home_automation_context());
        assert!(readiness.needs_cluster_context());
        assert!(readiness.needs_payload_context());

        let empty = ApsFrameBatchSummary::empty().readiness();
        assert_eq!(empty.passed_check_count, 0);
        assert!(empty.needs_frames());
    }

    #[test]
    fn aps_delivery_handoff_summary_marks_ready_delivery() {
        let frame = ApsFrame {
            frame_control: ApsFrameControl {
                ack_request: true,
                ..ApsFrameControl::data_unicast()
            },
            addressing: ApsAddressing::Unicast {
                destination_endpoint: Endpoint(1),
                source_endpoint: Endpoint(2),
            },
            cluster_id: ClusterId::ON_OFF,
            profile_id: ProfileId::HOME_AUTOMATION,
            counter: 9,
            payload: vec![0x01, 0x00],
        };
        let batch_summary = ApsFrameBatchSummary::from_frames([&frame]);
        let readiness = batch_summary.readiness();

        let summary = summarize_aps_delivery_handoff(readiness);

        assert_eq!(summary.readiness_summary, readiness);
        assert_eq!(summary.required_handoff_check_count, 4);
        assert_eq!(summary.passed_handoff_check_count, 4);
        assert_eq!(summary.blocked_handoff_check_count, 0);
        assert!(summary.frame_batch_ready);
        assert!(summary.application_delivery_ready);
        assert!(summary.payload_context_ready);
        assert!(summary.security_or_ack_context_present);
        assert!(summary.delivery_handoff_ready);
        assert!(summary.is_delivery_handoff_ready());
        assert!(!summary.has_blocked_handoff_checks());
        assert!(!summary.needs_frame_batch());
        assert!(!summary.needs_application_delivery());
        assert!(!summary.needs_payload_context());
        assert!(!summary.needs_security_or_ack_context());
    }

    #[test]
    fn aps_delivery_handoff_summary_routes_blocked_delivery() {
        let command_only = ApsFrameSummary {
            frame_type: ApsFrameType::Command,
            delivery_mode: DeliveryMode::Broadcast,
            profile_kind: ProfileKind::ZigbeeDeviceProfile,
            cluster_kind: ClusterKind::ManufacturerSpecific,
            source_endpoint: Endpoint::ZDO,
            destination_endpoint: Some(Endpoint(255)),
            group: None,
            counter: 1,
            payload_len: 0,
            ack_request: false,
            security: false,
        };
        let batch_summary = ApsFrameBatchSummary::from_summaries([command_only]);

        let summary = ApsDeliveryHandoffSummary::from_batch_summary(batch_summary);

        assert_eq!(summary.required_handoff_check_count, 4);
        assert_eq!(summary.passed_handoff_check_count, 0);
        assert_eq!(summary.blocked_handoff_check_count, 4);
        assert!(!summary.frame_batch_ready);
        assert!(!summary.application_delivery_ready);
        assert!(!summary.payload_context_ready);
        assert!(!summary.security_or_ack_context_present);
        assert!(!summary.delivery_handoff_ready);
        assert!(!summary.is_delivery_handoff_ready());
        assert!(summary.has_blocked_handoff_checks());
        assert!(summary.needs_frame_batch());
        assert!(summary.needs_application_delivery());
        assert!(summary.needs_payload_context());
        assert!(summary.needs_security_or_ack_context());
    }

    #[test]
    fn binding_table_summary_counts_destinations_and_cluster_kinds() {
        let source = BindingSource::new(IeeeAddress(0x0012_4b00_0000_0001), Endpoint(1));
        let mut table = BindingTable::new();
        table.upsert(BindingEntry::new(
            source,
            ClusterId::ON_OFF,
            BindingDestination::group(GroupAddress(0x1234)),
        ));
        table.upsert(BindingEntry::new(
            source,
            ClusterId::TEMPERATURE_MEASUREMENT,
            BindingDestination::device(IeeeAddress(0x0012_4b00_0000_0002), Endpoint(2)),
        ));
        table.upsert(BindingEntry::new(
            BindingSource::new(IeeeAddress(0x0012_4b00_0000_0003), Endpoint(3)),
            ClusterId(0xfc00),
            BindingDestination::group(GroupAddress(0x2345)),
        ));

        let summary = table.summary();

        assert_eq!(summary.total_bindings, 3);
        assert_eq!(summary.group_bindings, 2);
        assert_eq!(summary.device_bindings, 1);
        assert_eq!(summary.general_cluster_bindings, 1);
        assert_eq!(summary.measurement_and_sensing_bindings, 1);
        assert_eq!(summary.manufacturer_specific_cluster_bindings, 1);
        assert_eq!(summary.unknown_cluster_bindings, 0);
        assert_eq!(summary.unique_sources, 2);
        assert_eq!(summary.unique_clusters, 3);
        assert_eq!(summary.unique_groups, 2);
        assert_eq!(summary.unique_device_destinations, 1);
        assert_eq!(summary.zdo_source_bindings, 0);
        assert_eq!(summary.application_source_bindings, 3);
        assert_eq!(summary.non_application_source_bindings, 0);
        assert!(!summary.is_empty());
        assert!(summary.has_bindings());
        assert!(summary.has_group_bindings());
        assert!(summary.has_device_bindings());
        assert!(summary.has_manufacturer_specific_clusters());
        assert!(summary.has_application_sources());
        assert!(!summary.has_zdo_sources());
        assert!(!summary.has_non_application_sources());
    }

    #[test]
    fn binding_table_summary_classifies_source_endpoint_shapes() {
        let mut table = BindingTable::new();
        let empty = table.summary();
        assert_eq!(empty, BindingTableSummary::empty());
        assert!(empty.is_empty());
        table.upsert(BindingEntry::new(
            BindingSource::new(IeeeAddress(0x0012_4b00_0000_0010), Endpoint::ZDO),
            ClusterId::BASIC,
            BindingDestination::group(GroupAddress(0x1000)),
        ));
        table.upsert(BindingEntry::new(
            BindingSource::new(IeeeAddress(0x0012_4b00_0000_0011), Endpoint(241)),
            ClusterId(0x1234),
            BindingDestination::device(IeeeAddress(0x0012_4b00_0000_0012), Endpoint(3)),
        ));

        let summary = table.summary();

        assert_eq!(summary.total_bindings, 2);
        assert_eq!(summary.zdo_source_bindings, 1);
        assert_eq!(summary.application_source_bindings, 0);
        assert_eq!(summary.non_application_source_bindings, 1);
        assert_eq!(summary.unique_sources, 2);
        assert_eq!(summary.unique_clusters, 2);
        assert!(summary.has_zdo_sources());
        assert!(!summary.has_application_sources());
        assert!(summary.has_non_application_sources());
    }

    #[test]
    fn binding_table_readiness_summary_marks_application_binding_surface_ready() {
        let source = BindingSource::new(IeeeAddress(0x0012_4b00_0000_0001), Endpoint(1));
        let mut table = BindingTable::new();
        table.upsert(BindingEntry::new(
            source,
            ClusterId::ON_OFF,
            BindingDestination::group(GroupAddress(0x1234)),
        ));
        table.upsert(BindingEntry::new(
            source,
            ClusterId::TEMPERATURE_MEASUREMENT,
            BindingDestination::device(IeeeAddress(0x0012_4b00_0000_0002), Endpoint(2)),
        ));

        let readiness = table.readiness_summary();

        assert_eq!(readiness.binding_summary, table.summary());
        assert_eq!(readiness.required_check_count, 6);
        assert_eq!(readiness.passed_check_count, 6);
        assert_eq!(readiness.missing_check_count, 0);
        assert!(readiness.bindings_present);
        assert!(readiness.application_sources_ready);
        assert!(readiness.destination_coverage_ready);
        assert!(readiness.cluster_coverage_ready);
        assert!(readiness.zdo_sources_absent);
        assert!(readiness.non_application_sources_absent);
        assert!(readiness.binding_ready);
        assert!(readiness.is_binding_ready());
        assert!(!readiness.has_missing_checks());
        assert!(!readiness.needs_binding_discovery());
        assert!(!readiness.needs_application_source_binding());
        assert!(!readiness.needs_destination_coverage());
        assert!(!readiness.needs_cluster_coverage());
        assert!(!readiness.has_source_endpoint_issues());
    }

    #[test]
    fn binding_table_readiness_summary_flags_endpoint_and_cluster_gaps() {
        let mut table = BindingTable::new();
        table.upsert(BindingEntry::new(
            BindingSource::new(IeeeAddress(0x0012_4b00_0000_0010), Endpoint::ZDO),
            ClusterId::BASIC,
            BindingDestination::group(GroupAddress(0x1000)),
        ));
        table.upsert(BindingEntry::new(
            BindingSource::new(IeeeAddress(0x0012_4b00_0000_0011), Endpoint(241)),
            ClusterId(0x1234),
            BindingDestination::device(IeeeAddress(0x0012_4b00_0000_0012), Endpoint(3)),
        ));

        let readiness = BindingTableReadinessSummary::from_summary(table.summary());

        assert_eq!(readiness.required_check_count, 6);
        assert_eq!(readiness.passed_check_count, 2);
        assert_eq!(readiness.missing_check_count, 4);
        assert!(readiness.bindings_present);
        assert!(!readiness.application_sources_ready);
        assert!(readiness.destination_coverage_ready);
        assert!(!readiness.cluster_coverage_ready);
        assert!(!readiness.zdo_sources_absent);
        assert!(!readiness.non_application_sources_absent);
        assert!(!readiness.binding_ready);
        assert!(!readiness.is_binding_ready());
        assert!(readiness.has_missing_checks());
        assert!(!readiness.needs_binding_discovery());
        assert!(readiness.needs_application_source_binding());
        assert!(!readiness.needs_destination_coverage());
        assert!(readiness.needs_cluster_coverage());
        assert!(readiness.has_source_endpoint_issues());
    }
}
