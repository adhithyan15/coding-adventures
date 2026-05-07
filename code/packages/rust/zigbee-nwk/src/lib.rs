//! Zigbee network-layer primitives built above IEEE 802.15.4.
//!
//! This crate starts with the NWK byte boundary: network addresses, frame
//! control bits, optional extended addresses, radius/sequence fields, and
//! payload extraction. Joining, routing tables, APS, ZDO, and ZCL live in later
//! crates.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt;

const NWK_FRAME_CONTROL_LEN: usize = 2;
const NWK_ADDR_LEN: usize = 2;
const IEEE_ADDR_LEN: usize = 8;
const NWK_BASE_HEADER_LEN: usize = NWK_FRAME_CONTROL_LEN + (NWK_ADDR_LEN * 2) + 2;
const SOURCE_ROUTE_FIXED_LEN: usize = 2;

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
pub enum NwkDeviceRole {
    Coordinator,
    Router,
    EndDevice,
    Unknown,
}

impl NwkDeviceRole {
    pub fn can_route(self) -> bool {
        matches!(self, Self::Coordinator | Self::Router)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NeighborRelationship {
    Parent,
    Child,
    Sibling,
    PreviousChild,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NeighborEntry {
    pub network_address: NetworkAddress,
    pub ieee_address: Option<IeeeAddress>,
    pub role: NwkDeviceRole,
    pub relationship: NeighborRelationship,
    pub depth: Option<u8>,
    pub lqi: Option<u8>,
    pub outgoing_cost: Option<u8>,
    pub last_seen_at_ms: u64,
    pub timeout_ms: u64,
}

impl NeighborEntry {
    pub fn new(
        network_address: NetworkAddress,
        role: NwkDeviceRole,
        relationship: NeighborRelationship,
        last_seen_at_ms: u64,
        timeout_ms: u64,
    ) -> Self {
        Self {
            network_address,
            ieee_address: None,
            role,
            relationship,
            depth: None,
            lqi: None,
            outgoing_cost: None,
            last_seen_at_ms,
            timeout_ms,
        }
    }

    pub fn with_ieee_address(mut self, ieee_address: IeeeAddress) -> Self {
        self.ieee_address = Some(ieee_address);
        self
    }

    pub fn with_link_metrics(mut self, lqi: u8, outgoing_cost: u8) -> Self {
        self.lqi = Some(lqi);
        self.outgoing_cost = Some(outgoing_cost);
        self
    }

    pub fn is_stale_at(&self, now_ms: u64) -> bool {
        now_ms >= self.last_seen_at_ms.saturating_add(self.timeout_ms)
    }

    pub fn can_route(&self) -> bool {
        self.role.can_route()
    }
}

#[derive(Debug, Clone, Default)]
pub struct NeighborTable {
    neighbors: BTreeMap<NetworkAddress, NeighborEntry>,
    ieee_index: BTreeMap<IeeeAddress, NetworkAddress>,
}

impl NeighborTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.neighbors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.neighbors.is_empty()
    }

    pub fn upsert(&mut self, entry: NeighborEntry) -> Option<NeighborEntry> {
        if let Some(old) = self.neighbors.get(&entry.network_address) {
            if let Some(old_ieee) = old.ieee_address {
                self.ieee_index.remove(&old_ieee);
            }
        }
        if let Some(ieee_address) = entry.ieee_address {
            self.ieee_index.insert(ieee_address, entry.network_address);
        }
        self.neighbors.insert(entry.network_address, entry)
    }

    pub fn neighbor(&self, network_address: NetworkAddress) -> Option<&NeighborEntry> {
        self.neighbors.get(&network_address)
    }

    pub fn neighbor_by_ieee(&self, ieee_address: IeeeAddress) -> Option<&NeighborEntry> {
        self.ieee_index
            .get(&ieee_address)
            .and_then(|network_address| self.neighbors.get(network_address))
    }

    pub fn routers(&self) -> impl Iterator<Item = &NeighborEntry> {
        self.neighbors.values().filter(|entry| entry.can_route())
    }

    pub fn children(&self) -> impl Iterator<Item = &NeighborEntry> {
        self.neighbors
            .values()
            .filter(|entry| entry.relationship == NeighborRelationship::Child)
    }

    pub fn stale_neighbors_at(&self, now_ms: u64) -> Vec<NetworkAddress> {
        self.neighbors
            .values()
            .filter(|entry| entry.is_stale_at(now_ms))
            .map(|entry| entry.network_address)
            .collect()
    }

    pub fn expire_stale(&mut self, now_ms: u64) -> Vec<NeighborEntry> {
        let stale = self.stale_neighbors_at(now_ms);
        stale
            .into_iter()
            .filter_map(|network_address| self.remove(network_address))
            .collect()
    }

    pub fn remove(&mut self, network_address: NetworkAddress) -> Option<NeighborEntry> {
        let removed = self.neighbors.remove(&network_address)?;
        if let Some(ieee_address) = removed.ieee_address {
            self.ieee_index.remove(&ieee_address);
        }
        Some(removed)
    }

    pub fn best_router_candidate(&self) -> Option<&NeighborEntry> {
        self.routers().max_by_key(|entry| {
            (
                entry.lqi.unwrap_or(0),
                u8::MAX.saturating_sub(entry.outgoing_cost.unwrap_or(u8::MAX)),
                entry.last_seen_at_ms,
            )
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteStatus {
    Active,
    DiscoveryUnderway,
    DiscoveryFailed,
    Inactive,
}

impl RouteStatus {
    pub fn is_usable(self) -> bool {
        self == Self::Active
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteEntry {
    pub destination: NetworkAddress,
    pub next_hop: NetworkAddress,
    pub status: RouteStatus,
    pub route_record_required: bool,
    pub many_to_one: bool,
    pub last_updated_at_ms: u64,
}

impl RouteEntry {
    pub fn active(
        destination: NetworkAddress,
        next_hop: NetworkAddress,
        last_updated_at_ms: u64,
    ) -> Self {
        Self {
            destination,
            next_hop,
            status: RouteStatus::Active,
            route_record_required: false,
            many_to_one: false,
            last_updated_at_ms,
        }
    }

    pub fn is_usable(&self) -> bool {
        self.status.is_usable()
    }
}

#[derive(Debug, Clone, Default)]
pub struct RouteTable {
    routes: BTreeMap<NetworkAddress, RouteEntry>,
}

impl RouteTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.routes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.routes.is_empty()
    }

    pub fn upsert(&mut self, entry: RouteEntry) -> Option<RouteEntry> {
        self.routes.insert(entry.destination, entry)
    }

    pub fn route_to(&self, destination: NetworkAddress) -> Option<&RouteEntry> {
        self.routes.get(&destination)
    }

    pub fn next_hop_for(&self, destination: NetworkAddress) -> Option<NetworkAddress> {
        self.route_to(destination)
            .filter(|entry| entry.is_usable())
            .map(|entry| entry.next_hop)
    }

    pub fn routes_via(&self, next_hop: NetworkAddress) -> impl Iterator<Item = &RouteEntry> {
        self.routes
            .values()
            .filter(move |entry| entry.next_hop == next_hop)
    }

    pub fn remove(&mut self, destination: NetworkAddress) -> Option<RouteEntry> {
        self.routes.remove(&destination)
    }

    pub fn mark_inactive(&mut self, destination: NetworkAddress) -> Option<&RouteEntry> {
        let entry = self.routes.get_mut(&destination)?;
        entry.status = RouteStatus::Inactive;
        Some(entry)
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
pub struct SourceRoute {
    pub relay_index: u8,
    pub relays: Vec<NetworkAddress>,
}

impl SourceRoute {
    pub fn new(relay_index: u8, relays: Vec<NetworkAddress>) -> Result<Self, NwkError> {
        if relays.len() > u8::MAX as usize {
            return Err(NwkError::TooManySourceRouteRelays {
                count: relays.len(),
            });
        }
        Ok(Self {
            relay_index,
            relays,
        })
    }

    pub fn relay_count(&self) -> usize {
        self.relays.len()
    }

    pub fn is_empty(&self) -> bool {
        self.relays.is_empty()
    }

    pub fn next_relay(&self) -> Option<NetworkAddress> {
        self.relays.get(self.relay_index as usize).copied()
    }

    fn parse(cursor: &mut Cursor<'_>) -> Result<Self, NwkError> {
        let relay_count = cursor.read_u8()?;
        let relay_index = cursor.read_u8()?;
        let mut relays = Vec::with_capacity(relay_count as usize);
        for _ in 0..relay_count {
            relays.push(NetworkAddress(cursor.read_u16_le()?));
        }
        Ok(Self {
            relay_index,
            relays,
        })
    }

    fn encode_into(&self, out: &mut Vec<u8>) -> Result<(), NwkError> {
        if self.relays.len() > u8::MAX as usize {
            return Err(NwkError::TooManySourceRouteRelays {
                count: self.relays.len(),
            });
        }

        out.reserve(SOURCE_ROUTE_FIXED_LEN + (self.relays.len() * NWK_ADDR_LEN));
        out.push(self.relays.len() as u8);
        out.push(self.relay_index);
        for relay in &self.relays {
            out.extend_from_slice(&relay.0.to_le_bytes());
        }
        Ok(())
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
    pub source_route: Option<SourceRoute>,
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
        let source_route = if frame_control.source_route {
            Some(SourceRoute::parse(&mut cursor)?)
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
            source_route,
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
        if self.frame_control.source_route != self.source_route.is_some() {
            return Err(NwkError::SourceRouteMismatch);
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
        if let Some(source_route) = &self.source_route {
            source_route.encode_into(&mut out)?;
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
            source_route: None,
            payload,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NwkError {
    Truncated { needed: usize, remaining: usize },
    ExtendedAddressMismatch { field: &'static str },
    MulticastControlMismatch,
    SourceRouteMismatch,
    TooManySourceRouteRelays { count: usize },
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
            Self::SourceRouteMismatch => {
                write!(
                    f,
                    "source-route flag does not match source-route relay subframe"
                )
            }
            Self::TooManySourceRouteRelays { count } => write!(
                f,
                "source-route relay count {count} exceeds the NWK u8 relay count field"
            ),
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
            source_route: None,
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
    fn source_route_subframe_round_trips() {
        let mut control = NwkFrameControl::zigbee_pro_2007(NwkFrameType::Data);
        control.source_route = true;
        let frame = NwkFrame {
            frame_control: control,
            destination: NetworkAddress(0x3000),
            source: NetworkAddress(0x0000),
            radius: 30,
            sequence_number: 42,
            destination_ieee: None,
            source_ieee: None,
            multicast_control: None,
            source_route: Some(
                SourceRoute::new(
                    1,
                    vec![
                        NetworkAddress(0x1001),
                        NetworkAddress(0x1002),
                        NetworkAddress(0x1003),
                    ],
                )
                .unwrap(),
            ),
            payload: vec![0xaa, 0xbb],
        };

        let parsed = NwkFrame::parse(&frame.encode().unwrap()).unwrap();

        assert_eq!(parsed, frame);
        assert_eq!(
            parsed.source_route.as_ref().unwrap().next_relay(),
            Some(NetworkAddress(0x1002))
        );
    }

    #[test]
    fn rejects_source_route_flag_without_subframe() {
        let mut frame =
            NwkFrame::plain_data(NetworkAddress(1), NetworkAddress(2), 1, 1, Vec::new());
        frame.frame_control.source_route = true;

        assert_eq!(frame.encode(), Err(NwkError::SourceRouteMismatch));
    }

    #[test]
    fn rejects_source_routes_that_exceed_wire_count_field() {
        assert_eq!(
            SourceRoute::new(0, vec![NetworkAddress(0x1001); 256]),
            Err(NwkError::TooManySourceRouteRelays { count: 256 })
        );
    }

    #[test]
    fn broadcast_addresses_are_identified() {
        assert!(NetworkAddress::BROADCAST_ALL_DEVICES.is_broadcast());
        assert!(!NetworkAddress(0x1234).is_broadcast());
    }

    #[test]
    fn neighbor_table_tracks_indexes_and_router_candidates() {
        let mut table = NeighborTable::new();
        table.upsert(
            NeighborEntry::new(
                NetworkAddress(0x1001),
                NwkDeviceRole::Router,
                NeighborRelationship::Parent,
                1_000,
                10_000,
            )
            .with_ieee_address(IeeeAddress(0x0012_4b00_0000_0001))
            .with_link_metrics(180, 3),
        );
        table.upsert(
            NeighborEntry::new(
                NetworkAddress(0x1002),
                NwkDeviceRole::Router,
                NeighborRelationship::Sibling,
                1_100,
                10_000,
            )
            .with_link_metrics(200, 1),
        );
        table.upsert(NeighborEntry::new(
            NetworkAddress(0x1003),
            NwkDeviceRole::EndDevice,
            NeighborRelationship::Child,
            1_200,
            10_000,
        ));

        assert_eq!(table.len(), 3);
        assert_eq!(table.children().count(), 1);
        assert_eq!(
            table
                .neighbor_by_ieee(IeeeAddress(0x0012_4b00_0000_0001))
                .unwrap()
                .network_address,
            NetworkAddress(0x1001)
        );
        assert_eq!(
            table.best_router_candidate().unwrap().network_address,
            NetworkAddress(0x1002)
        );
    }

    #[test]
    fn neighbor_table_expires_stale_entries_and_ieee_index() {
        let mut table = NeighborTable::new();
        table.upsert(
            NeighborEntry::new(
                NetworkAddress(0x1001),
                NwkDeviceRole::Router,
                NeighborRelationship::Parent,
                1_000,
                500,
            )
            .with_ieee_address(IeeeAddress(0x0012_4b00_0000_0001)),
        );

        assert!(table.stale_neighbors_at(1_499).is_empty());
        let expired = table.expire_stale(1_500);

        assert_eq!(expired.len(), 1);
        assert!(table.neighbor(NetworkAddress(0x1001)).is_none());
        assert!(table
            .neighbor_by_ieee(IeeeAddress(0x0012_4b00_0000_0001))
            .is_none());
    }

    #[test]
    fn route_table_tracks_active_next_hops() {
        let mut table = RouteTable::new();
        table.upsert(RouteEntry::active(
            NetworkAddress(0x2001),
            NetworkAddress(0x1001),
            1_000,
        ));
        table.upsert(RouteEntry {
            destination: NetworkAddress(0x2002),
            next_hop: NetworkAddress(0x1001),
            status: RouteStatus::DiscoveryUnderway,
            route_record_required: true,
            many_to_one: false,
            last_updated_at_ms: 1_100,
        });

        assert_eq!(
            table.next_hop_for(NetworkAddress(0x2001)),
            Some(NetworkAddress(0x1001))
        );
        assert_eq!(table.next_hop_for(NetworkAddress(0x2002)), None);
        assert_eq!(table.routes_via(NetworkAddress(0x1001)).count(), 2);

        table.mark_inactive(NetworkAddress(0x2001)).unwrap();
        assert_eq!(table.next_hop_for(NetworkAddress(0x2001)), None);
    }
}
