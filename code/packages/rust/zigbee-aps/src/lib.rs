//! Zigbee Application Support Sublayer primitives.
//!
//! APS is where endpoint addressing, group addressing, cluster/profile ids,
//! counters, and delivery-mode flags appear before ZDO/ZCL semantics take over.
//! This crate only owns those bytes and validation rules.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt;
use zigbee_nwk::{IeeeAddress, NetworkAddress};

const FRAME_CONTROL_LEN: usize = 1;
const ENDPOINT_LEN: usize = 1;
const GROUP_ADDRESS_LEN: usize = 2;
const CLUSTER_ID_LEN: usize = 2;
const PROFILE_ID_LEN: usize = 2;
const COUNTER_LEN: usize = 1;

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
}
