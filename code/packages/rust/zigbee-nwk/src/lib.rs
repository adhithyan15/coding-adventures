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
const ROUTE_REQUEST_FIXED_LEN: usize = 5;
const ROUTE_REPLY_FIXED_LEN: usize = 7;
const NETWORK_STATUS_LEN: usize = 3;
const ROUTE_REQUEST_MANY_TO_ONE_MASK: u8 = 0b0001_1000;
const ROUTE_REQUEST_MANY_TO_ONE_SHIFT: u8 = 3;
const ROUTE_REQUEST_DESTINATION_IEEE_FLAG: u8 = 1 << 5;
const ROUTE_REQUEST_MULTICAST_FLAG: u8 = 1 << 6;
const ROUTE_REPLY_ORIGINATOR_IEEE_FLAG: u8 = 1 << 4;
const ROUTE_REPLY_RESPONDER_IEEE_FLAG: u8 = 1 << 5;
const ROUTE_REPLY_MULTICAST_FLAG: u8 = 1 << 6;

pub const NWK_COMMAND_ROUTE_REQUEST: u8 = 0x01;
pub const NWK_COMMAND_ROUTE_REPLY: u8 = 0x02;
pub const NWK_COMMAND_NETWORK_STATUS: u8 = 0x03;
pub const NWK_COMMAND_ROUTE_RECORD: u8 = 0x05;

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

    pub fn summary_at(&self, now_ms: u64) -> NeighborTableSummary {
        let mut summary = NeighborTableSummary {
            total_neighbors: self.neighbors.len(),
            coordinator_neighbors: 0,
            router_neighbors: 0,
            end_device_neighbors: 0,
            unknown_role_neighbors: 0,
            route_capable_neighbors: 0,
            parent_neighbors: 0,
            child_neighbors: 0,
            sibling_neighbors: 0,
            previous_child_neighbors: 0,
            unknown_relationship_neighbors: 0,
            stale_neighbors: 0,
            ieee_address_neighbors: 0,
            link_metric_neighbors: 0,
            best_router_candidate: self
                .best_router_candidate()
                .map(|entry| entry.network_address),
        };

        for entry in self.neighbors.values() {
            match entry.role {
                NwkDeviceRole::Coordinator => summary.coordinator_neighbors += 1,
                NwkDeviceRole::Router => summary.router_neighbors += 1,
                NwkDeviceRole::EndDevice => summary.end_device_neighbors += 1,
                NwkDeviceRole::Unknown => summary.unknown_role_neighbors += 1,
            }
            if entry.can_route() {
                summary.route_capable_neighbors += 1;
            }
            match entry.relationship {
                NeighborRelationship::Parent => summary.parent_neighbors += 1,
                NeighborRelationship::Child => summary.child_neighbors += 1,
                NeighborRelationship::Sibling => summary.sibling_neighbors += 1,
                NeighborRelationship::PreviousChild => summary.previous_child_neighbors += 1,
                NeighborRelationship::Unknown => summary.unknown_relationship_neighbors += 1,
            }
            if entry.is_stale_at(now_ms) {
                summary.stale_neighbors += 1;
            }
            if entry.ieee_address.is_some() {
                summary.ieee_address_neighbors += 1;
            }
            if entry.lqi.is_some() || entry.outgoing_cost.is_some() {
                summary.link_metric_neighbors += 1;
            }
        }

        summary
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NeighborTableSummary {
    pub total_neighbors: usize,
    pub coordinator_neighbors: usize,
    pub router_neighbors: usize,
    pub end_device_neighbors: usize,
    pub unknown_role_neighbors: usize,
    pub route_capable_neighbors: usize,
    pub parent_neighbors: usize,
    pub child_neighbors: usize,
    pub sibling_neighbors: usize,
    pub previous_child_neighbors: usize,
    pub unknown_relationship_neighbors: usize,
    pub stale_neighbors: usize,
    pub ieee_address_neighbors: usize,
    pub link_metric_neighbors: usize,
    pub best_router_candidate: Option<NetworkAddress>,
}

impl NeighborTableSummary {
    pub fn has_stale_neighbors(self) -> bool {
        self.stale_neighbors > 0
    }

    pub fn has_router_candidate(self) -> bool {
        self.best_router_candidate.is_some()
    }

    pub fn has_missing_ieee_addresses(self) -> bool {
        self.ieee_address_neighbors < self.total_neighbors
    }

    pub fn has_link_metrics(self) -> bool {
        self.link_metric_neighbors > 0
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

    pub fn summary(&self) -> RouteTableSummary {
        let mut summary = RouteTableSummary {
            total_routes: self.routes.len(),
            usable_routes: 0,
            discovery_underway_routes: 0,
            discovery_failed_routes: 0,
            inactive_routes: 0,
            many_to_one_routes: 0,
            route_record_required_routes: 0,
        };

        for entry in self.routes.values() {
            match entry.status {
                RouteStatus::Active => summary.usable_routes += 1,
                RouteStatus::DiscoveryUnderway => summary.discovery_underway_routes += 1,
                RouteStatus::DiscoveryFailed => summary.discovery_failed_routes += 1,
                RouteStatus::Inactive => summary.inactive_routes += 1,
            }
            if entry.many_to_one {
                summary.many_to_one_routes += 1;
            }
            if entry.route_record_required {
                summary.route_record_required_routes += 1;
            }
        }

        summary
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouteTableSummary {
    pub total_routes: usize,
    pub usable_routes: usize,
    pub discovery_underway_routes: usize,
    pub discovery_failed_routes: usize,
    pub inactive_routes: usize,
    pub many_to_one_routes: usize,
    pub route_record_required_routes: usize,
}

impl RouteTableSummary {
    pub fn has_discovery_failures(self) -> bool {
        self.discovery_failed_routes > 0
    }

    pub fn has_usable_routes(self) -> bool {
        self.usable_routes > 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NwkTopologySummary {
    pub generated_at_ms: u64,
    pub neighbors: NeighborTableSummary,
    pub routes: RouteTableSummary,
}

impl NwkTopologySummary {
    pub fn new(generated_at_ms: u64, neighbors: &NeighborTable, routes: &RouteTable) -> Self {
        Self {
            generated_at_ms,
            neighbors: neighbors.summary_at(generated_at_ms),
            routes: routes.summary(),
        }
    }

    pub fn needs_supervision(self) -> bool {
        self.neighbors.has_stale_neighbors() || self.routes.has_discovery_failures()
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
pub enum NwkCommandId {
    RouteRequest,
    RouteReply,
    NetworkStatus,
    RouteRecord,
    Unknown(u8),
}

impl NwkCommandId {
    pub fn from_byte(value: u8) -> Self {
        match value {
            NWK_COMMAND_ROUTE_REQUEST => Self::RouteRequest,
            NWK_COMMAND_ROUTE_REPLY => Self::RouteReply,
            NWK_COMMAND_NETWORK_STATUS => Self::NetworkStatus,
            NWK_COMMAND_ROUTE_RECORD => Self::RouteRecord,
            other => Self::Unknown(other),
        }
    }

    pub fn as_byte(self) -> u8 {
        match self {
            Self::RouteRequest => NWK_COMMAND_ROUTE_REQUEST,
            Self::RouteReply => NWK_COMMAND_ROUTE_REPLY,
            Self::NetworkStatus => NWK_COMMAND_NETWORK_STATUS,
            Self::RouteRecord => NWK_COMMAND_ROUTE_RECORD,
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RouteCommandOptions(u8);

impl RouteCommandOptions {
    pub fn new(raw: u8) -> Self {
        Self(raw)
    }

    pub fn raw(self) -> u8 {
        self.0
    }

    pub fn route_request_many_to_one(self) -> u8 {
        (self.0 & ROUTE_REQUEST_MANY_TO_ONE_MASK) >> ROUTE_REQUEST_MANY_TO_ONE_SHIFT
    }

    pub fn route_request_destination_ieee_present(self) -> bool {
        self.has_flag(ROUTE_REQUEST_DESTINATION_IEEE_FLAG)
    }

    pub fn route_request_multicast(self) -> bool {
        self.has_flag(ROUTE_REQUEST_MULTICAST_FLAG)
    }

    pub fn with_route_request_destination_ieee_present(self, present: bool) -> Self {
        self.with_flag(ROUTE_REQUEST_DESTINATION_IEEE_FLAG, present)
    }

    pub fn route_reply_originator_ieee_present(self) -> bool {
        self.has_flag(ROUTE_REPLY_ORIGINATOR_IEEE_FLAG)
    }

    pub fn route_reply_responder_ieee_present(self) -> bool {
        self.has_flag(ROUTE_REPLY_RESPONDER_IEEE_FLAG)
    }

    pub fn route_reply_multicast(self) -> bool {
        self.has_flag(ROUTE_REPLY_MULTICAST_FLAG)
    }

    pub fn with_route_reply_originator_ieee_present(self, present: bool) -> Self {
        self.with_flag(ROUTE_REPLY_ORIGINATOR_IEEE_FLAG, present)
    }

    pub fn with_route_reply_responder_ieee_present(self, present: bool) -> Self {
        self.with_flag(ROUTE_REPLY_RESPONDER_IEEE_FLAG, present)
    }

    fn has_flag(self, flag: u8) -> bool {
        self.0 & flag != 0
    }

    fn with_flag(mut self, flag: u8, present: bool) -> Self {
        if present {
            self.0 |= flag;
        } else {
            self.0 &= !flag;
        }
        self
    }
}

impl From<u8> for RouteCommandOptions {
    fn from(value: u8) -> Self {
        Self::new(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteRequest {
    pub options: RouteCommandOptions,
    pub request_id: u8,
    pub destination: NetworkAddress,
    pub path_cost: u8,
    pub destination_ieee: Option<IeeeAddress>,
}

impl RouteRequest {
    pub fn new(request_id: u8, destination: NetworkAddress, path_cost: u8) -> Self {
        Self {
            options: RouteCommandOptions::default(),
            request_id,
            destination,
            path_cost,
            destination_ieee: None,
        }
    }

    pub fn with_options(mut self, options: RouteCommandOptions) -> Self {
        self.options = options;
        self
    }

    pub fn with_destination_ieee(mut self, destination_ieee: IeeeAddress) -> Self {
        self.destination_ieee = Some(destination_ieee);
        self.options = self
            .options
            .with_route_request_destination_ieee_present(true);
        self
    }

    fn parse(cursor: &mut Cursor<'_>) -> Result<Self, NwkError> {
        let options = RouteCommandOptions::new(cursor.read_u8()?);
        let request_id = cursor.read_u8()?;
        let destination = NetworkAddress(cursor.read_u16_le()?);
        let path_cost = cursor.read_u8()?;
        let destination_ieee = if options.route_request_destination_ieee_present() {
            Some(IeeeAddress(cursor.read_u64_le()?))
        } else {
            None
        };
        ensure_no_command_tail(cursor, NWK_COMMAND_ROUTE_REQUEST)?;

        Ok(Self {
            options,
            request_id,
            destination,
            path_cost,
            destination_ieee,
        })
    }

    fn encode_into(&self, out: &mut Vec<u8>) -> Result<(), NwkError> {
        if self.options.route_request_destination_ieee_present() != self.destination_ieee.is_some()
        {
            return Err(NwkError::InvalidCommandPayload {
                command_id: NWK_COMMAND_ROUTE_REQUEST,
                reason: "route request destination IEEE flag does not match address field",
            });
        }

        out.reserve(ROUTE_REQUEST_FIXED_LEN + self.destination_ieee.map_or(0, |_| IEEE_ADDR_LEN));
        out.push(self.options.raw());
        out.push(self.request_id);
        out.extend_from_slice(&self.destination.0.to_le_bytes());
        out.push(self.path_cost);
        if let Some(address) = self.destination_ieee {
            out.extend_from_slice(&address.to_le_bytes());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteReply {
    pub options: RouteCommandOptions,
    pub request_id: u8,
    pub originator: NetworkAddress,
    pub responder: NetworkAddress,
    pub path_cost: u8,
    pub originator_ieee: Option<IeeeAddress>,
    pub responder_ieee: Option<IeeeAddress>,
}

impl RouteReply {
    pub fn new(
        request_id: u8,
        originator: NetworkAddress,
        responder: NetworkAddress,
        path_cost: u8,
    ) -> Self {
        Self {
            options: RouteCommandOptions::default(),
            request_id,
            originator,
            responder,
            path_cost,
            originator_ieee: None,
            responder_ieee: None,
        }
    }

    pub fn with_options(mut self, options: RouteCommandOptions) -> Self {
        self.options = options;
        self
    }

    pub fn with_originator_ieee(mut self, originator_ieee: IeeeAddress) -> Self {
        self.originator_ieee = Some(originator_ieee);
        self.options = self.options.with_route_reply_originator_ieee_present(true);
        self
    }

    pub fn with_responder_ieee(mut self, responder_ieee: IeeeAddress) -> Self {
        self.responder_ieee = Some(responder_ieee);
        self.options = self.options.with_route_reply_responder_ieee_present(true);
        self
    }

    fn parse(cursor: &mut Cursor<'_>) -> Result<Self, NwkError> {
        let options = RouteCommandOptions::new(cursor.read_u8()?);
        let request_id = cursor.read_u8()?;
        let originator = NetworkAddress(cursor.read_u16_le()?);
        let responder = NetworkAddress(cursor.read_u16_le()?);
        let path_cost = cursor.read_u8()?;
        let originator_ieee = if options.route_reply_originator_ieee_present() {
            Some(IeeeAddress(cursor.read_u64_le()?))
        } else {
            None
        };
        let responder_ieee = if options.route_reply_responder_ieee_present() {
            Some(IeeeAddress(cursor.read_u64_le()?))
        } else {
            None
        };
        ensure_no_command_tail(cursor, NWK_COMMAND_ROUTE_REPLY)?;

        Ok(Self {
            options,
            request_id,
            originator,
            responder,
            path_cost,
            originator_ieee,
            responder_ieee,
        })
    }

    fn encode_into(&self, out: &mut Vec<u8>) -> Result<(), NwkError> {
        if self.options.route_reply_originator_ieee_present() != self.originator_ieee.is_some() {
            return Err(NwkError::InvalidCommandPayload {
                command_id: NWK_COMMAND_ROUTE_REPLY,
                reason: "route reply originator IEEE flag does not match address field",
            });
        }
        if self.options.route_reply_responder_ieee_present() != self.responder_ieee.is_some() {
            return Err(NwkError::InvalidCommandPayload {
                command_id: NWK_COMMAND_ROUTE_REPLY,
                reason: "route reply responder IEEE flag does not match address field",
            });
        }

        out.reserve(
            ROUTE_REPLY_FIXED_LEN
                + self.originator_ieee.map_or(0, |_| IEEE_ADDR_LEN)
                + self.responder_ieee.map_or(0, |_| IEEE_ADDR_LEN),
        );
        out.push(self.options.raw());
        out.push(self.request_id);
        out.extend_from_slice(&self.originator.0.to_le_bytes());
        out.extend_from_slice(&self.responder.0.to_le_bytes());
        out.push(self.path_cost);
        if let Some(address) = self.originator_ieee {
            out.extend_from_slice(&address.to_le_bytes());
        }
        if let Some(address) = self.responder_ieee {
            out.extend_from_slice(&address.to_le_bytes());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkStatusCode {
    NoRouteAvailable,
    TreeLinkFailure,
    NonTreeLinkFailure,
    LowBattery,
    NoRoutingCapacity,
    NoIndirectCapacity,
    IndirectTransactionExpiry,
    TargetDeviceUnavailable,
    TargetAddressUnallocated,
    ParentLinkFailure,
    ValidateRoute,
    SourceRouteFailure,
    ManyToOneRouteFailure,
    AddressConflict,
    VerifyAddresses,
    PanIdentifierUpdate,
    NetworkAddressUpdate,
    BadFrameCounter,
    BadKeySequenceNumber,
    Unknown(u8),
}

impl NetworkStatusCode {
    pub fn from_byte(value: u8) -> Self {
        match value {
            0x00 => Self::NoRouteAvailable,
            0x01 => Self::TreeLinkFailure,
            0x02 => Self::NonTreeLinkFailure,
            0x03 => Self::LowBattery,
            0x04 => Self::NoRoutingCapacity,
            0x05 => Self::NoIndirectCapacity,
            0x06 => Self::IndirectTransactionExpiry,
            0x07 => Self::TargetDeviceUnavailable,
            0x08 => Self::TargetAddressUnallocated,
            0x09 => Self::ParentLinkFailure,
            0x0a => Self::ValidateRoute,
            0x0b => Self::SourceRouteFailure,
            0x0c => Self::ManyToOneRouteFailure,
            0x0d => Self::AddressConflict,
            0x0e => Self::VerifyAddresses,
            0x0f => Self::PanIdentifierUpdate,
            0x10 => Self::NetworkAddressUpdate,
            0x11 => Self::BadFrameCounter,
            0x12 => Self::BadKeySequenceNumber,
            other => Self::Unknown(other),
        }
    }

    pub fn as_byte(self) -> u8 {
        match self {
            Self::NoRouteAvailable => 0x00,
            Self::TreeLinkFailure => 0x01,
            Self::NonTreeLinkFailure => 0x02,
            Self::LowBattery => 0x03,
            Self::NoRoutingCapacity => 0x04,
            Self::NoIndirectCapacity => 0x05,
            Self::IndirectTransactionExpiry => 0x06,
            Self::TargetDeviceUnavailable => 0x07,
            Self::TargetAddressUnallocated => 0x08,
            Self::ParentLinkFailure => 0x09,
            Self::ValidateRoute => 0x0a,
            Self::SourceRouteFailure => 0x0b,
            Self::ManyToOneRouteFailure => 0x0c,
            Self::AddressConflict => 0x0d,
            Self::VerifyAddresses => 0x0e,
            Self::PanIdentifierUpdate => 0x0f,
            Self::NetworkAddressUpdate => 0x10,
            Self::BadFrameCounter => 0x11,
            Self::BadKeySequenceNumber => 0x12,
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NetworkStatus {
    pub status: NetworkStatusCode,
    pub destination: NetworkAddress,
}

impl NetworkStatus {
    pub fn new(status: NetworkStatusCode, destination: NetworkAddress) -> Self {
        Self {
            status,
            destination,
        }
    }

    fn parse(cursor: &mut Cursor<'_>) -> Result<Self, NwkError> {
        let status = NetworkStatusCode::from_byte(cursor.read_u8()?);
        let destination = NetworkAddress(cursor.read_u16_le()?);
        ensure_no_command_tail(cursor, NWK_COMMAND_NETWORK_STATUS)?;
        Ok(Self {
            status,
            destination,
        })
    }

    fn encode_into(self, out: &mut Vec<u8>) {
        out.reserve(NETWORK_STATUS_LEN);
        out.push(self.status.as_byte());
        out.extend_from_slice(&self.destination.0.to_le_bytes());
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteRecord {
    pub relays: Vec<NetworkAddress>,
}

impl RouteRecord {
    pub fn new(relays: Vec<NetworkAddress>) -> Result<Self, NwkError> {
        if relays.len() > u8::MAX as usize {
            return Err(NwkError::TooManyRouteRecordRelays {
                count: relays.len(),
            });
        }
        Ok(Self { relays })
    }

    pub fn relay_count(&self) -> usize {
        self.relays.len()
    }

    pub fn is_empty(&self) -> bool {
        self.relays.is_empty()
    }

    fn parse(cursor: &mut Cursor<'_>) -> Result<Self, NwkError> {
        let relay_count = cursor.read_u8()? as usize;
        let mut relays = Vec::with_capacity(relay_count);
        for _ in 0..relay_count {
            relays.push(NetworkAddress(cursor.read_u16_le()?));
        }
        ensure_no_command_tail(cursor, NWK_COMMAND_ROUTE_RECORD)?;
        Ok(Self { relays })
    }

    fn encode_into(&self, out: &mut Vec<u8>) -> Result<(), NwkError> {
        if self.relays.len() > u8::MAX as usize {
            return Err(NwkError::TooManyRouteRecordRelays {
                count: self.relays.len(),
            });
        }

        out.reserve(1 + (self.relays.len() * NWK_ADDR_LEN));
        out.push(self.relays.len() as u8);
        for relay in &self.relays {
            out.extend_from_slice(&relay.0.to_le_bytes());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NwkCommand {
    RouteRequest(RouteRequest),
    RouteReply(RouteReply),
    NetworkStatus(NetworkStatus),
    RouteRecord(RouteRecord),
    Unknown { command_id: u8, payload: Vec<u8> },
}

impl NwkCommand {
    pub fn command_id(&self) -> NwkCommandId {
        match self {
            Self::RouteRequest(_) => NwkCommandId::RouteRequest,
            Self::RouteReply(_) => NwkCommandId::RouteReply,
            Self::NetworkStatus(_) => NwkCommandId::NetworkStatus,
            Self::RouteRecord(_) => NwkCommandId::RouteRecord,
            Self::Unknown { command_id, .. } => NwkCommandId::Unknown(*command_id),
        }
    }

    pub fn parse(bytes: &[u8]) -> Result<Self, NwkError> {
        let mut cursor = Cursor::new(bytes);
        let command_id = cursor.read_u8()?;
        match NwkCommandId::from_byte(command_id) {
            NwkCommandId::RouteRequest => Ok(Self::RouteRequest(RouteRequest::parse(&mut cursor)?)),
            NwkCommandId::RouteReply => Ok(Self::RouteReply(RouteReply::parse(&mut cursor)?)),
            NwkCommandId::NetworkStatus => {
                Ok(Self::NetworkStatus(NetworkStatus::parse(&mut cursor)?))
            }
            NwkCommandId::RouteRecord => Ok(Self::RouteRecord(RouteRecord::parse(&mut cursor)?)),
            NwkCommandId::Unknown(command_id) => Ok(Self::Unknown {
                command_id,
                payload: cursor.remaining_bytes().to_vec(),
            }),
        }
    }

    pub fn encode(&self) -> Result<Vec<u8>, NwkError> {
        let mut out = Vec::new();
        out.push(self.command_id().as_byte());
        match self {
            Self::RouteRequest(command) => command.encode_into(&mut out)?,
            Self::RouteReply(command) => command.encode_into(&mut out)?,
            Self::NetworkStatus(command) => command.encode_into(&mut out),
            Self::RouteRecord(command) => command.encode_into(&mut out)?,
            Self::Unknown { payload, .. } => out.extend_from_slice(payload),
        }
        Ok(out)
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

    pub fn plain_command(
        destination: NetworkAddress,
        source: NetworkAddress,
        radius: u8,
        sequence_number: u8,
        command: NwkCommand,
    ) -> Result<Self, NwkError> {
        Ok(Self {
            frame_control: NwkFrameControl::zigbee_pro_2007(NwkFrameType::Command),
            destination,
            source,
            radius,
            sequence_number,
            destination_ieee: None,
            source_ieee: None,
            multicast_control: None,
            source_route: None,
            payload: command.encode()?,
        })
    }

    pub fn parse_command(&self) -> Result<NwkCommand, NwkError> {
        if self.frame_control.frame_type != NwkFrameType::Command {
            return Err(NwkError::NotCommandFrame {
                frame_type: self.frame_control.frame_type,
            });
        }
        NwkCommand::parse(&self.payload)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NwkError {
    Truncated {
        needed: usize,
        remaining: usize,
    },
    NotCommandFrame {
        frame_type: NwkFrameType,
    },
    InvalidCommandPayload {
        command_id: u8,
        reason: &'static str,
    },
    ExtendedAddressMismatch {
        field: &'static str,
    },
    MulticastControlMismatch,
    SourceRouteMismatch,
    TooManySourceRouteRelays {
        count: usize,
    },
    TooManyRouteRecordRelays {
        count: usize,
    },
}

impl fmt::Display for NwkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated { needed, remaining } => write!(
                f,
                "truncated Zigbee NWK frame: needed {needed} bytes, had {remaining}"
            ),
            Self::NotCommandFrame { frame_type } => {
                write!(f, "expected Zigbee NWK command frame, got {frame_type:?}")
            }
            Self::InvalidCommandPayload { command_id, reason } => write!(
                f,
                "invalid Zigbee NWK command 0x{command_id:02x} payload: {reason}"
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
            Self::TooManyRouteRecordRelays { count } => write!(
                f,
                "route-record relay count {count} exceeds the NWK u8 relay count field"
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

fn ensure_no_command_tail(cursor: &Cursor<'_>, command_id: u8) -> Result<(), NwkError> {
    if !cursor.remaining_bytes().is_empty() {
        return Err(NwkError::InvalidCommandPayload {
            command_id,
            reason: "unexpected trailing bytes",
        });
    }
    Ok(())
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

    #[test]
    fn neighbor_table_summary_tracks_freshness_and_router_candidates() {
        let mut table = NeighborTable::new();
        table.upsert(
            NeighborEntry::new(
                NetworkAddress(0x1000),
                NwkDeviceRole::Coordinator,
                NeighborRelationship::Parent,
                1_100,
                10_000,
            )
            .with_ieee_address(IeeeAddress(0x0012_4b00_0000_1000))
            .with_link_metrics(160, 2),
        );
        table.upsert(
            NeighborEntry::new(
                NetworkAddress(0x1001),
                NwkDeviceRole::Router,
                NeighborRelationship::Sibling,
                1_000,
                500,
            )
            .with_link_metrics(180, 3),
        );
        table.upsert(
            NeighborEntry::new(
                NetworkAddress(0x1002),
                NwkDeviceRole::Router,
                NeighborRelationship::PreviousChild,
                1_200,
                10_000,
            )
            .with_link_metrics(210, 1),
        );
        table.upsert(NeighborEntry::new(
            NetworkAddress(0x1003),
            NwkDeviceRole::EndDevice,
            NeighborRelationship::Child,
            1_300,
            10_000,
        ));
        table.upsert(
            NeighborEntry::new(
                NetworkAddress(0x1004),
                NwkDeviceRole::Unknown,
                NeighborRelationship::Unknown,
                1_400,
                10_000,
            )
            .with_ieee_address(IeeeAddress(0x0012_4b00_0000_1004)),
        );

        let summary = table.summary_at(1_500);

        assert_eq!(summary.total_neighbors, 5);
        assert_eq!(summary.coordinator_neighbors, 1);
        assert_eq!(summary.router_neighbors, 2);
        assert_eq!(summary.end_device_neighbors, 1);
        assert_eq!(summary.unknown_role_neighbors, 1);
        assert_eq!(summary.route_capable_neighbors, 3);
        assert_eq!(summary.parent_neighbors, 1);
        assert_eq!(summary.child_neighbors, 1);
        assert_eq!(summary.sibling_neighbors, 1);
        assert_eq!(summary.previous_child_neighbors, 1);
        assert_eq!(summary.unknown_relationship_neighbors, 1);
        assert_eq!(summary.stale_neighbors, 1);
        assert_eq!(summary.ieee_address_neighbors, 2);
        assert_eq!(summary.link_metric_neighbors, 3);
        assert_eq!(summary.best_router_candidate, Some(NetworkAddress(0x1002)));
        assert!(summary.has_stale_neighbors());
        assert!(summary.has_router_candidate());
        assert!(summary.has_missing_ieee_addresses());
        assert!(summary.has_link_metrics());
    }

    #[test]
    fn route_table_summary_tracks_route_health() {
        let mut table = RouteTable::new();
        table.upsert(RouteEntry::active(
            NetworkAddress(0x2001),
            NetworkAddress(0x1001),
            1_000,
        ));
        table.upsert(RouteEntry {
            destination: NetworkAddress(0x2002),
            next_hop: NetworkAddress(0x1002),
            status: RouteStatus::DiscoveryUnderway,
            route_record_required: true,
            many_to_one: false,
            last_updated_at_ms: 1_100,
        });
        table.upsert(RouteEntry {
            destination: NetworkAddress(0x2003),
            next_hop: NetworkAddress(0x1003),
            status: RouteStatus::DiscoveryFailed,
            route_record_required: false,
            many_to_one: true,
            last_updated_at_ms: 1_200,
        });
        table.upsert(RouteEntry {
            destination: NetworkAddress(0x2004),
            next_hop: NetworkAddress(0x1004),
            status: RouteStatus::Inactive,
            route_record_required: false,
            many_to_one: false,
            last_updated_at_ms: 1_300,
        });

        let summary = table.summary();

        assert_eq!(summary.total_routes, 4);
        assert_eq!(summary.usable_routes, 1);
        assert_eq!(summary.discovery_underway_routes, 1);
        assert_eq!(summary.discovery_failed_routes, 1);
        assert_eq!(summary.inactive_routes, 1);
        assert_eq!(summary.many_to_one_routes, 1);
        assert_eq!(summary.route_record_required_routes, 1);
        assert!(summary.has_usable_routes());
        assert!(summary.has_discovery_failures());
    }

    #[test]
    fn topology_summary_marks_supervision_needs() {
        let mut neighbors = NeighborTable::new();
        neighbors.upsert(NeighborEntry::new(
            NetworkAddress(0x1001),
            NwkDeviceRole::Router,
            NeighborRelationship::Parent,
            1_000,
            500,
        ));

        let mut routes = RouteTable::new();
        routes.upsert(RouteEntry {
            destination: NetworkAddress(0x2001),
            next_hop: NetworkAddress(0x1001),
            status: RouteStatus::DiscoveryFailed,
            route_record_required: false,
            many_to_one: false,
            last_updated_at_ms: 1_100,
        });

        let summary = NwkTopologySummary::new(1_500, &neighbors, &routes);

        assert_eq!(summary.generated_at_ms, 1_500);
        assert_eq!(summary.neighbors.stale_neighbors, 1);
        assert_eq!(summary.routes.discovery_failed_routes, 1);
        assert!(summary.needs_supervision());
    }

    #[test]
    fn route_request_command_round_trips_with_destination_ieee() {
        let command = NwkCommand::RouteRequest(
            RouteRequest::new(7, NetworkAddress(0x3344), 12)
                .with_options(RouteCommandOptions::new(0x08))
                .with_destination_ieee(IeeeAddress(0x0012_4b00_0000_abcd)),
        );

        let encoded = command.encode().unwrap();
        assert_eq!(encoded[0], NWK_COMMAND_ROUTE_REQUEST);
        assert_eq!(
            encoded[1],
            ROUTE_REQUEST_DESTINATION_IEEE_FLAG | (1 << ROUTE_REQUEST_MANY_TO_ONE_SHIFT)
        );
        if let NwkCommand::RouteRequest(request) = &command {
            assert_eq!(request.options.route_request_many_to_one(), 1);
            assert!(request.options.route_request_destination_ieee_present());
        }
        assert_eq!(NwkCommand::parse(&encoded).unwrap(), command);
    }

    #[test]
    fn route_reply_command_round_trips_with_extended_addresses() {
        let command = NwkCommand::RouteReply(
            RouteReply::new(7, NetworkAddress(0x0000), NetworkAddress(0x3344), 9)
                .with_originator_ieee(IeeeAddress(0x0012_4b00_0000_0001))
                .with_responder_ieee(IeeeAddress(0x0012_4b00_0000_0002)),
        );

        let encoded = command.encode().unwrap();

        assert_eq!(encoded[0], NWK_COMMAND_ROUTE_REPLY);
        assert_eq!(
            encoded[1],
            ROUTE_REPLY_ORIGINATOR_IEEE_FLAG | ROUTE_REPLY_RESPONDER_IEEE_FLAG
        );
        assert_eq!(NwkCommand::parse(&encoded).unwrap(), command);
    }

    #[test]
    fn route_reply_round_trips_responder_ieee_without_originator_ieee() {
        let command = NwkCommand::RouteReply(
            RouteReply::new(7, NetworkAddress(0x0000), NetworkAddress(0x3344), 9)
                .with_responder_ieee(IeeeAddress(0x0012_4b00_0000_0002)),
        );
        let encoded = command.encode().unwrap();

        assert_eq!(encoded[1], ROUTE_REPLY_RESPONDER_IEEE_FLAG);
        assert_eq!(NwkCommand::parse(&encoded).unwrap(), command);
    }

    #[test]
    fn route_reply_rejects_option_flag_without_matching_address() {
        let command = NwkCommand::RouteReply(
            RouteReply::new(7, NetworkAddress(0x0000), NetworkAddress(0x3344), 9)
                .with_options(RouteCommandOptions::new(ROUTE_REPLY_ORIGINATOR_IEEE_FLAG)),
        );

        assert_eq!(
            command.encode(),
            Err(NwkError::InvalidCommandPayload {
                command_id: NWK_COMMAND_ROUTE_REPLY,
                reason: "route reply originator IEEE flag does not match address field",
            })
        );
    }

    #[test]
    fn network_status_command_round_trips() {
        let command = NwkCommand::NetworkStatus(NetworkStatus::new(
            NetworkStatusCode::SourceRouteFailure,
            NetworkAddress(0x3344),
        ));

        let encoded = command.encode().unwrap();

        assert_eq!(encoded, vec![NWK_COMMAND_NETWORK_STATUS, 0x0b, 0x44, 0x33]);
        assert_eq!(NwkCommand::parse(&encoded).unwrap(), command);
    }

    #[test]
    fn route_record_command_round_trips_relays() {
        let command = NwkCommand::RouteRecord(
            RouteRecord::new(vec![
                NetworkAddress(0x1001),
                NetworkAddress(0x1002),
                NetworkAddress(0x1003),
            ])
            .unwrap(),
        );

        let encoded = command.encode().unwrap();
        let parsed = NwkCommand::parse(&encoded).unwrap();

        assert_eq!(encoded[0], NWK_COMMAND_ROUTE_RECORD);
        assert_eq!(parsed, command);
        if let NwkCommand::RouteRecord(record) = parsed {
            assert_eq!(record.relay_count(), 3);
        } else {
            panic!("expected route record command");
        }
    }

    #[test]
    fn rejects_route_records_that_exceed_wire_count_field() {
        assert_eq!(
            RouteRecord::new(vec![NetworkAddress(0x1001); 256]),
            Err(NwkError::TooManyRouteRecordRelays { count: 256 })
        );
    }

    #[test]
    fn command_frames_parse_typed_payloads() {
        let frame = NwkFrame::plain_command(
            NetworkAddress::BROADCAST_RX_ON_WHEN_IDLE,
            NetworkAddress(0x0000),
            30,
            44,
            NwkCommand::RouteRequest(RouteRequest::new(5, NetworkAddress(0x3344), 1)),
        )
        .unwrap();
        let encoded = frame.encode().unwrap();
        let parsed = NwkFrame::parse(&encoded).unwrap();

        assert_eq!(
            parsed.parse_command().unwrap(),
            NwkCommand::RouteRequest(RouteRequest::new(5, NetworkAddress(0x3344), 1))
        );
    }

    #[test]
    fn data_frames_reject_typed_command_parsing() {
        let frame = NwkFrame::plain_data(
            NetworkAddress(0x1234),
            NetworkAddress(0x0000),
            30,
            7,
            vec![0x01],
        );

        assert_eq!(
            frame.parse_command(),
            Err(NwkError::NotCommandFrame {
                frame_type: NwkFrameType::Data,
            })
        );
    }
}
