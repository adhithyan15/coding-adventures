//! Zigbee APS and ZCL integration for the normalized smart-home runtime.

#![forbid(unsafe_code)]

use smart_home_core::{
    AgentId, Bridge, BridgeId, BridgeTransport, Capability, CommandResult, CommandType, Device,
    DeviceEvent, DeviceEventType, DeviceId, Entity, EntityId, EntityKind, EventId, Health,
    IntegrationId, Metadata, ProtocolFamily, ProtocolIdentifier, Value, VaultRef,
};
use smart_home_runtime::{RuntimeCommandToolRequest, RuntimeError, SmartHomeRuntime};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use zigbee_aps::{ApsAddressing, ApsError, ApsFrame, ApsFrameType, ClusterId, Endpoint, ProfileId};
use zigbee_nwk::{IeeeAddress, NetworkAddress};
use zigbee_zcl::{
    capabilities_for_cluster, move_to_color_temperature_frame, move_to_level_with_on_off_frame,
    on_off_command_frame, parse_attribute_reports, state_delta_for_report, OnOffCommand,
    ZclClusterId, ZclError, ZclFrame, ZCL_REPORT_ATTRIBUTES_COMMAND_ID,
};
use zigbee_zdo::{interview_to_device, SimpleDescriptor, ZigbeeInterviewSummary};

pub const VERSION: &str = "0.1.0";
pub const INTEGRATION_ID: &str = "zigbee";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZigbeeCoordinatorConfig {
    pub bridge_id: BridgeId,
    pub serial_path: String,
    pub pan_id: u16,
    pub channel: u8,
    pub coordinator_ieee_address: IeeeAddress,
    pub source_endpoint: Endpoint,
    pub manufacturer: String,
    pub model: String,
    pub firmware_version: Option<String>,
    pub network_key_ref: Option<VaultRef>,
}

impl ZigbeeCoordinatorConfig {
    pub fn new(
        bridge_id: BridgeId,
        serial_path: impl Into<String>,
        pan_id: u16,
        channel: u8,
        coordinator_ieee_address: IeeeAddress,
    ) -> Self {
        Self {
            bridge_id,
            serial_path: serial_path.into(),
            pan_id,
            channel,
            coordinator_ieee_address,
            source_endpoint: Endpoint::MIN_APPLICATION,
            manufacturer: "Zigbee".to_string(),
            model: "Coordinator".to_string(),
            firmware_version: None,
            network_key_ref: None,
        }
    }

    pub fn with_identity(
        mut self,
        manufacturer: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        self.manufacturer = manufacturer.into();
        self.model = model.into();
        self
    }

    pub fn with_firmware_version(mut self, firmware_version: impl Into<String>) -> Self {
        self.firmware_version = Some(firmware_version.into());
        self
    }

    pub fn with_network_key_ref(mut self, network_key_ref: VaultRef) -> Self {
        self.network_key_ref = Some(network_key_ref);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZigbeeDeviceInstallation {
    pub interview: ZigbeeInterviewSummary,
    pub name: String,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub room_id: Option<String>,
}

impl ZigbeeDeviceInstallation {
    pub fn new(interview: ZigbeeInterviewSummary, name: impl Into<String>) -> Self {
        Self {
            interview,
            name: name.into(),
            manufacturer: None,
            model: None,
            room_id: None,
        }
    }

    pub fn with_identity(
        mut self,
        manufacturer: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        self.manufacturer = Some(manufacturer.into());
        self.model = Some(model.into());
        self
    }

    pub fn in_room(mut self, room_id: impl Into<String>) -> Self {
        self.room_id = Some(room_id.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledZigbeeEndpoint {
    pub network_address: NetworkAddress,
    pub endpoint: Endpoint,
    pub entity_id: EntityId,
    pub entity_kind: EntityKind,
    pub capability_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledZigbeeDevice {
    pub network_address: NetworkAddress,
    pub device_id: DeviceId,
    pub endpoints: Vec<InstalledZigbeeEndpoint>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZigbeeCommandDispatch {
    pub command_result: CommandResult,
    pub network_address: NetworkAddress,
    pub destination_endpoint: Endpoint,
    pub cluster_id: ClusterId,
    pub zcl_frame: ZclFrame,
    pub aps_frame: ApsFrame,
    pub aps_bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EndpointBinding {
    network_address: NetworkAddress,
    endpoint: Endpoint,
    device_id: DeviceId,
    entity_id: EntityId,
    input_clusters: BTreeSet<ClusterId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZigbeeRuntimeIntegration {
    config: ZigbeeCoordinatorConfig,
    endpoints: BTreeMap<(NetworkAddress, Endpoint), EndpointBinding>,
    next_aps_counter: u8,
    next_zcl_sequence: u8,
    next_event_sequence: u64,
}

impl ZigbeeRuntimeIntegration {
    pub fn new(config: ZigbeeCoordinatorConfig) -> Result<Self, ZigbeeIntegrationError> {
        if config.serial_path.trim().is_empty() {
            return Err(ZigbeeIntegrationError::Validation(
                "serial path must not be empty".to_string(),
            ));
        }
        if !(11..=26).contains(&config.channel) {
            return Err(ZigbeeIntegrationError::Validation(
                "channel must be between 11 and 26".to_string(),
            ));
        }
        if !config.source_endpoint.is_application() {
            return Err(ZigbeeIntegrationError::Validation(
                "coordinator source endpoint must be an application endpoint".to_string(),
            ));
        }
        Ok(Self {
            config,
            endpoints: BTreeMap::new(),
            next_aps_counter: 0,
            next_zcl_sequence: 0,
            next_event_sequence: 1,
        })
    }

    pub fn config(&self) -> &ZigbeeCoordinatorConfig {
        &self.config
    }

    pub fn install_coordinator(
        &self,
        runtime: &mut SmartHomeRuntime,
        observed_at_ms: u64,
    ) -> Result<Option<Bridge>, ZigbeeIntegrationError> {
        let mut bridge = Bridge::new(
            self.config.bridge_id.clone(),
            IntegrationId::trusted(INTEGRATION_ID),
            BridgeTransport::Serial,
        );
        bridge.address = Some(self.config.serial_path.clone());
        bridge.hardware_model = Some(self.config.model.clone());
        bridge.firmware_version = self.config.firmware_version.clone();
        bridge.auth_ref = self.config.network_key_ref.clone();
        bridge.health = Health::Online;
        bridge.last_seen_at_ms = Some(observed_at_ms);
        bridge.identifiers = vec![
            protocol_identifier("pan_id", format!("0x{:04x}", self.config.pan_id))?,
            protocol_identifier(
                "coordinator_ieee",
                format!("0x{:016x}", self.config.coordinator_ieee_address.0),
            )?,
        ];
        bridge.metadata = vec![
            Metadata::new("zigbee.coordinator.manufacturer", &self.config.manufacturer),
            Metadata::new("zigbee.coordinator.model", &self.config.model),
            Metadata::new("zigbee.channel", self.config.channel.to_string()),
            Metadata::new("zigbee.transport_boundary", "aps"),
        ];
        runtime.upsert_bridge(bridge).map_err(Into::into)
    }

    pub fn install_device(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        installation: ZigbeeDeviceInstallation,
    ) -> Result<InstalledZigbeeDevice, ZigbeeIntegrationError> {
        if runtime.registry().bridge(&self.config.bridge_id).is_none() {
            return Err(ZigbeeIntegrationError::CoordinatorNotInstalled(
                self.config.bridge_id.clone(),
            ));
        }
        validate_interview(&installation.interview)?;

        let network_address = installation.interview.network_address;
        let mut device = interview_to_device(&self.config.bridge_id, &installation.interview);
        device.name = installation.name.clone();
        if let Some(manufacturer) = installation.manufacturer {
            device.manufacturer = manufacturer;
        }
        if let Some(model) = installation.model {
            device.model = model;
        }
        device.room_id = installation.room_id;
        device.health = Health::Online;

        let mut installed_endpoints = Vec::new();
        let mut entities = Vec::new();
        for descriptor in &installation.interview.simple_descriptors {
            let Some((entity, binding, installed)) =
                endpoint_projection(&device, descriptor, network_address, &installation.name)
            else {
                continue;
            };
            device.entity_ids.push(entity.entity_id.clone());
            entities.push(entity);
            self.endpoints
                .insert((network_address, descriptor.endpoint), binding);
            installed_endpoints.push(installed);
        }
        if installed_endpoints.is_empty() {
            return Err(ZigbeeIntegrationError::NoSupportedEndpoints(
                network_address,
            ));
        }

        runtime.upsert_device(device)?;
        for entity in entities {
            runtime.upsert_entity(entity)?;
        }
        Ok(InstalledZigbeeDevice {
            network_address,
            device_id: DeviceId::trusted(format!(
                "zigbee.device.{}.0x{:04x}",
                self.config.bridge_id, network_address.0
            )),
            endpoints: installed_endpoints,
        })
    }

    pub fn ingest_aps_frame(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        source_network_address: NetworkAddress,
        bytes: &[u8],
        observed_at_ms: u64,
    ) -> Result<Vec<DeviceEvent>, ZigbeeIntegrationError> {
        let aps_frame = ApsFrame::parse(bytes)?;
        if aps_frame.frame_control.frame_type != ApsFrameType::Data {
            return Err(ZigbeeIntegrationError::UnsupportedApsFrameType);
        }
        if aps_frame.profile_id != ProfileId::HOME_AUTOMATION {
            return Err(ZigbeeIntegrationError::UnsupportedProfile(
                aps_frame.profile_id,
            ));
        }
        let source_endpoint = source_endpoint(&aps_frame.addressing);
        let binding = self
            .endpoints
            .get(&(source_network_address, source_endpoint))
            .cloned()
            .ok_or(ZigbeeIntegrationError::UnknownEndpoint {
                network_address: source_network_address,
                endpoint: source_endpoint,
            })?;
        if !binding.input_clusters.contains(&aps_frame.cluster_id) {
            return Err(ZigbeeIntegrationError::UnexpectedCluster {
                endpoint: source_endpoint,
                cluster_id: aps_frame.cluster_id,
            });
        }

        let zcl_frame = ZclFrame::parse(&aps_frame.payload)?;
        let zcl_summary = zcl_frame.summary();
        if !zcl_summary.is_report_attributes()
            || !zcl_summary.is_server_to_client()
            || zcl_frame.command_id != ZCL_REPORT_ATTRIBUTES_COMMAND_ID
        {
            return Err(ZigbeeIntegrationError::UnsupportedZclFrame);
        }
        let zcl_cluster_id = ZclClusterId(aps_frame.cluster_id.0);
        let reports = parse_attribute_reports(zcl_cluster_id, &zcl_frame.payload)?;
        let mut events = Vec::new();
        for (report_index, state_delta) in reports
            .iter()
            .filter_map(state_delta_for_report)
            .enumerate()
        {
            let event = DeviceEvent {
                event_id: EventId::trusted(format!(
                    "zigbee-event:{}:{:04x}:{}:{}",
                    self.config.pan_id,
                    source_network_address.0,
                    self.next_event_sequence,
                    report_index
                )),
                bridge_id: self.config.bridge_id.clone(),
                device_id: Some(binding.device_id.clone()),
                entity_id: Some(binding.entity_id.clone()),
                observed_at_ms,
                received_at_ms: observed_at_ms,
                event_type: DeviceEventType::Updated,
                state_delta: Some(state_delta),
                raw_ref: Some(format!(
                    "zigbee-aps://0x{:04x}/endpoint/{}/counter/{}",
                    source_network_address.0, source_endpoint.0, aps_frame.counter
                )),
                correlation_id: None,
                metadata: vec![
                    Metadata::new(
                        "zigbee.cluster_id",
                        format!("0x{:04x}", aps_frame.cluster_id.0),
                    ),
                    Metadata::new(
                        "zigbee.zcl_transaction_sequence",
                        zcl_frame.transaction_sequence_number.to_string(),
                    ),
                    Metadata::new("zigbee.aps_counter", aps_frame.counter.to_string()),
                ],
            };
            runtime.apply_device_event(event.clone())?;
            events.push(event);
        }
        if events.is_empty() {
            return Err(ZigbeeIntegrationError::NoMappedAttributeReports {
                cluster_id: aps_frame.cluster_id,
            });
        }
        self.next_event_sequence = self.next_event_sequence.saturating_add(1);
        Ok(events)
    }

    pub fn dispatch_command(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        principal_id: AgentId,
        request: RuntimeCommandToolRequest,
        now_ms: u64,
    ) -> Result<ZigbeeCommandDispatch, ZigbeeIntegrationError> {
        let binding = self
            .endpoints
            .values()
            .find(|binding| binding.entity_id == request.entity_id)
            .cloned()
            .ok_or_else(|| ZigbeeIntegrationError::UnknownEntity(request.entity_id.clone()))?;
        let plan = command_plan(&binding, &request)?;

        // Authorization and durable command audit happen before wire bytes exist.
        let command_result = runtime.execute_command_tool(principal_id, request, now_ms)?;
        let zcl_frame = plan.to_zcl_frame(self.next_zcl_sequence);
        let zcl_bytes = zcl_frame.encode()?;
        let mut aps_frame = ApsFrame::unicast_data(
            binding.endpoint,
            self.config.source_endpoint,
            plan.cluster_id(),
            ProfileId::HOME_AUTOMATION,
            self.next_aps_counter,
            zcl_bytes,
        );
        aps_frame.frame_control.ack_request = true;
        let aps_bytes = aps_frame.encode()?;
        self.next_zcl_sequence = self.next_zcl_sequence.wrapping_add(1);
        self.next_aps_counter = self.next_aps_counter.wrapping_add(1);

        Ok(ZigbeeCommandDispatch {
            command_result,
            network_address: binding.network_address,
            destination_endpoint: binding.endpoint,
            cluster_id: plan.cluster_id(),
            zcl_frame,
            aps_frame,
            aps_bytes,
        })
    }
}

fn validate_interview(interview: &ZigbeeInterviewSummary) -> Result<(), ZigbeeIntegrationError> {
    if interview.ieee_address.is_none() {
        return Err(ZigbeeIntegrationError::IncompleteInterview(
            "IEEE address is missing",
        ));
    }
    if interview.simple_descriptors.is_empty() {
        return Err(ZigbeeIntegrationError::IncompleteInterview(
            "simple descriptors are missing",
        ));
    }
    Ok(())
}

fn endpoint_projection(
    device: &Device,
    descriptor: &SimpleDescriptor,
    network_address: NetworkAddress,
    device_name: &str,
) -> Option<(Entity, EndpointBinding, InstalledZigbeeEndpoint)> {
    if descriptor.profile_id != ProfileId::HOME_AUTOMATION || !descriptor.endpoint.is_application()
    {
        return None;
    }
    let input_clusters = descriptor
        .input_clusters
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let mut capabilities = BTreeMap::new();
    for cluster_id in &input_clusters {
        for capability in capabilities_for_cluster(ZclClusterId(cluster_id.0)) {
            capabilities
                .entry(capability.capability_id.as_str().to_string())
                .or_insert(capability);
        }
    }
    let capabilities = capabilities.into_values().collect::<Vec<Capability>>();
    if capabilities.is_empty() {
        return None;
    }
    let entity_kind = entity_kind_for_clusters(&input_clusters);
    let entity_id = EntityId::trusted(format!(
        "zigbee.entity.0x{:04x}.{}",
        network_address.0, descriptor.endpoint.0
    ));
    let capability_ids = capabilities
        .iter()
        .map(|capability| capability.capability_id.as_str().to_string())
        .collect();
    let cluster_list = input_clusters
        .iter()
        .map(|cluster| format!("0x{:04x}", cluster.0))
        .collect::<Vec<_>>()
        .join(",");
    let entity = Entity {
        entity_id: entity_id.clone(),
        device_id: device.device_id.clone(),
        kind: entity_kind,
        name: if device.entity_ids.is_empty() {
            device_name.to_string()
        } else {
            format!("{device_name} endpoint {}", descriptor.endpoint.0)
        },
        capabilities,
        state: None,
        metadata: vec![
            Metadata::new("zigbee.endpoint", descriptor.endpoint.0.to_string()),
            Metadata::new("zigbee.profile_id", "0x0104"),
            Metadata::new(
                "zigbee.device_id",
                format!("0x{:04x}", descriptor.device_id),
            ),
            Metadata::new("zigbee.input_clusters", cluster_list),
        ],
    };
    let binding = EndpointBinding {
        network_address,
        endpoint: descriptor.endpoint,
        device_id: device.device_id.clone(),
        entity_id: entity_id.clone(),
        input_clusters,
    };
    let installed = InstalledZigbeeEndpoint {
        network_address,
        endpoint: descriptor.endpoint,
        entity_id,
        entity_kind,
        capability_ids,
    };
    Some((entity, binding, installed))
}

fn entity_kind_for_clusters(clusters: &BTreeSet<ClusterId>) -> EntityKind {
    if clusters.contains(&ClusterId(ZclClusterId::DOOR_LOCK.0)) {
        EntityKind::Lock
    } else if clusters.contains(&ClusterId(ZclClusterId::THERMOSTAT.0)) {
        EntityKind::Thermostat
    } else if clusters.contains(&ClusterId::ON_OFF)
        || clusters.contains(&ClusterId::LEVEL_CONTROL)
        || clusters.contains(&ClusterId(ZclClusterId::COLOR_CONTROL.0))
    {
        EntityKind::Light
    } else {
        EntityKind::Sensor
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ZigbeeCommandPlan {
    OnOff(OnOffCommand),
    Brightness(u8),
    ColorTemperature(u16),
}

impl ZigbeeCommandPlan {
    fn cluster_id(self) -> ClusterId {
        match self {
            Self::OnOff(_) => ClusterId::ON_OFF,
            Self::Brightness(_) => ClusterId::LEVEL_CONTROL,
            Self::ColorTemperature(_) => ClusterId(ZclClusterId::COLOR_CONTROL.0),
        }
    }

    fn to_zcl_frame(self, transaction_sequence_number: u8) -> ZclFrame {
        match self {
            Self::OnOff(command) => on_off_command_frame(transaction_sequence_number, command),
            Self::Brightness(percent) => {
                move_to_level_with_on_off_frame(transaction_sequence_number, percent, 0)
            }
            Self::ColorTemperature(mirek) => {
                move_to_color_temperature_frame(transaction_sequence_number, mirek, 0)
            }
        }
    }
}

fn command_plan(
    binding: &EndpointBinding,
    request: &RuntimeCommandToolRequest,
) -> Result<ZigbeeCommandPlan, ZigbeeIntegrationError> {
    let plan = match request.command_type {
        CommandType::TurnOn => ZigbeeCommandPlan::OnOff(OnOffCommand::On),
        CommandType::TurnOff => ZigbeeCommandPlan::OnOff(OnOffCommand::Off),
        CommandType::SetBrightness => {
            let Value::Percentage(percent) = &request.arguments else {
                return Err(ZigbeeIntegrationError::InvalidCommandArguments {
                    command_type: request.command_type,
                    expected: "percentage",
                });
            };
            ZigbeeCommandPlan::Brightness(*percent)
        }
        CommandType::SetColorTemperature => {
            let Value::Integer(mirek) = &request.arguments else {
                return Err(ZigbeeIntegrationError::InvalidCommandArguments {
                    command_type: request.command_type,
                    expected: "positive integer mirek",
                });
            };
            let mirek = u16::try_from(*mirek).map_err(|_| {
                ZigbeeIntegrationError::InvalidCommandArguments {
                    command_type: request.command_type,
                    expected: "positive integer mirek",
                }
            })?;
            ZigbeeCommandPlan::ColorTemperature(mirek)
        }
        _ => {
            return Err(ZigbeeIntegrationError::UnsupportedCommand {
                entity_id: request.entity_id.clone(),
                command_type: request.command_type,
            });
        }
    };
    if !binding.input_clusters.contains(&plan.cluster_id()) {
        return Err(ZigbeeIntegrationError::UnsupportedCommand {
            entity_id: request.entity_id.clone(),
            command_type: request.command_type,
        });
    }
    Ok(plan)
}

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

fn protocol_identifier(
    kind: impl Into<String>,
    value: impl Into<String>,
) -> Result<ProtocolIdentifier, ZigbeeIntegrationError> {
    ProtocolIdentifier::new(ProtocolFamily::Zigbee, kind, value)
        .map_err(|error| ZigbeeIntegrationError::Validation(error.to_string()))
}

#[derive(Debug)]
pub enum ZigbeeIntegrationError {
    Validation(String),
    CoordinatorNotInstalled(BridgeId),
    IncompleteInterview(&'static str),
    NoSupportedEndpoints(NetworkAddress),
    UnknownEndpoint {
        network_address: NetworkAddress,
        endpoint: Endpoint,
    },
    UnknownEntity(EntityId),
    UnsupportedApsFrameType,
    UnsupportedProfile(ProfileId),
    UnexpectedCluster {
        endpoint: Endpoint,
        cluster_id: ClusterId,
    },
    UnsupportedZclFrame,
    NoMappedAttributeReports {
        cluster_id: ClusterId,
    },
    UnsupportedCommand {
        entity_id: EntityId,
        command_type: CommandType,
    },
    InvalidCommandArguments {
        command_type: CommandType,
        expected: &'static str,
    },
    Runtime(RuntimeError),
    Aps(ApsError),
    Zcl(ZclError),
}

impl fmt::Display for ZigbeeIntegrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => write!(f, "invalid Zigbee integration: {message}"),
            Self::CoordinatorNotInstalled(bridge_id) => {
                write!(f, "Zigbee coordinator {bridge_id} is not installed")
            }
            Self::IncompleteInterview(message) => {
                write!(f, "incomplete Zigbee interview: {message}")
            }
            Self::NoSupportedEndpoints(address) => {
                write!(
                    f,
                    "Zigbee node 0x{:04x} has no supported endpoints",
                    address.0
                )
            }
            Self::UnknownEndpoint {
                network_address,
                endpoint,
            } => write!(
                f,
                "unknown Zigbee endpoint 0x{:04x}/{}",
                network_address.0, endpoint.0
            ),
            Self::UnknownEntity(entity_id) => write!(f, "unknown Zigbee entity {entity_id}"),
            Self::UnsupportedApsFrameType => {
                write!(f, "APS frame is not an application data frame")
            }
            Self::UnsupportedProfile(profile_id) => {
                write!(f, "unsupported Zigbee profile 0x{:04x}", profile_id.0)
            }
            Self::UnexpectedCluster {
                endpoint,
                cluster_id,
            } => write!(
                f,
                "cluster 0x{:04x} was not interviewed on endpoint {}",
                cluster_id.0, endpoint.0
            ),
            Self::UnsupportedZclFrame => write!(f, "ZCL frame is not an attribute report"),
            Self::NoMappedAttributeReports { cluster_id } => write!(
                f,
                "ZCL report for cluster 0x{:04x} has no normalized state mapping",
                cluster_id.0
            ),
            Self::UnsupportedCommand {
                entity_id,
                command_type,
            } => write!(
                f,
                "Zigbee entity {entity_id} does not support {command_type:?}"
            ),
            Self::InvalidCommandArguments {
                command_type,
                expected,
            } => write!(f, "invalid {command_type:?} arguments; expected {expected}"),
            Self::Runtime(error) => error.fmt(f),
            Self::Aps(error) => error.fmt(f),
            Self::Zcl(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for ZigbeeIntegrationError {}

impl From<RuntimeError> for ZigbeeIntegrationError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

impl From<ApsError> for ZigbeeIntegrationError {
    fn from(error: ApsError) -> Self {
        Self::Aps(error)
    }
}

impl From<ZclError> for ZigbeeIntegrationError {
    fn from(error: ZclError) -> Self {
        Self::Zcl(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smart_home_core::{
        CapabilityGrant, CapabilityGrantId, PrivilegeTier, StateConfidence, StateSource,
    };
    use zigbee_zcl::{
        encode_attribute_reports, ZclAttributeId, ZclAttributeReport, ZclDataType, ZclValue,
    };

    fn integration() -> ZigbeeRuntimeIntegration {
        ZigbeeRuntimeIntegration::new(
            ZigbeeCoordinatorConfig::new(
                BridgeId::trusted("zigbee-coordinator-1"),
                "/dev/ttyUSB1",
                0x1a62,
                15,
                IeeeAddress(0x0012_4b00_01aa_55ff),
            )
            .with_identity("Texas Instruments", "CC2652P")
            .with_firmware_version("2026.07")
            .with_network_key_ref(VaultRef::trusted("vault:zigbee/home")),
        )
        .unwrap()
    }

    fn interview() -> ZigbeeInterviewSummary {
        ZigbeeInterviewSummary {
            network_address: NetworkAddress(0x1234),
            ieee_address: Some(IeeeAddress(0x0015_8d00_0455_6677)),
            node_descriptor: None,
            simple_descriptors: vec![
                SimpleDescriptor {
                    endpoint: Endpoint(1),
                    profile_id: ProfileId::HOME_AUTOMATION,
                    device_id: 0x0101,
                    device_version: 1,
                    input_clusters: vec![
                        ClusterId::BASIC,
                        ClusterId::ON_OFF,
                        ClusterId::LEVEL_CONTROL,
                        ClusterId(ZclClusterId::COLOR_CONTROL.0),
                    ],
                    output_clusters: vec![],
                },
                SimpleDescriptor {
                    endpoint: Endpoint(2),
                    profile_id: ProfileId::HOME_AUTOMATION,
                    device_id: 0x0302,
                    device_version: 1,
                    input_clusters: vec![
                        ClusterId::TEMPERATURE_MEASUREMENT,
                        ClusterId(ZclClusterId::RELATIVE_HUMIDITY_MEASUREMENT.0),
                    ],
                    output_clusters: vec![],
                },
            ],
        }
    }

    fn install(
        integration: &mut ZigbeeRuntimeIntegration,
        runtime: &mut SmartHomeRuntime,
    ) -> InstalledZigbeeDevice {
        integration.install_coordinator(runtime, 1_000).unwrap();
        integration
            .install_device(
                runtime,
                ZigbeeDeviceInstallation::new(interview(), "Kitchen Zigbee Device")
                    .with_identity("IKEA", "TRADFRI combo")
                    .in_room("kitchen"),
            )
            .unwrap()
    }

    fn grant(runtime: &mut SmartHomeRuntime, principal: &AgentId) {
        runtime
            .registry_mut()
            .upsert_capability_grant(CapabilityGrant::for_all_smart_home(
                CapabilityGrantId::trusted("grant-zigbee-test"),
                principal.clone(),
                PrivilegeTier::HighRisk,
                "test",
                0,
            ));
    }

    fn report_aps_frame(
        source_endpoint: Endpoint,
        cluster_id: ZclClusterId,
        reports: &[ZclAttributeReport],
    ) -> Vec<u8> {
        let zcl = ZclFrame::foundation_response(
            7,
            ZCL_REPORT_ATTRIBUTES_COMMAND_ID,
            encode_attribute_reports(reports).unwrap(),
        );
        ApsFrame::unicast_data(
            Endpoint(1),
            source_endpoint,
            ClusterId(cluster_id.0),
            ProfileId::HOME_AUTOMATION,
            11,
            zcl.encode().unwrap(),
        )
        .encode()
        .unwrap()
    }

    #[test]
    fn installs_coordinator_and_interviewed_endpoints() {
        let mut integration = integration();
        let mut runtime = SmartHomeRuntime::new();
        let installed = install(&mut integration, &mut runtime);

        assert_eq!(runtime.topology_summary().bridges, 1);
        assert_eq!(runtime.topology_summary().devices, 1);
        assert_eq!(runtime.topology_summary().entities, 2);
        assert_eq!(installed.endpoints[0].entity_kind, EntityKind::Light);
        assert_eq!(
            installed.endpoints[0].capability_ids,
            vec![
                "light.brightness",
                "light.color_temperature",
                "light.on_off"
            ]
        );
        assert_eq!(installed.endpoints[1].entity_kind, EntityKind::Sensor);
        let bridge = runtime
            .registry()
            .bridge(&BridgeId::trusted("zigbee-coordinator-1"))
            .unwrap();
        assert_eq!(
            bridge.auth_ref.as_ref().map(VaultRef::as_str),
            Some("vault:zigbee/home")
        );
        assert!(!format!("{bridge:?}").contains("network_key"));
    }

    #[test]
    fn aps_and_zcl_report_bytes_update_normalized_state() {
        let mut integration = integration();
        let mut runtime = SmartHomeRuntime::new();
        let installed = install(&mut integration, &mut runtime);
        let bytes = report_aps_frame(
            Endpoint(2),
            ZclClusterId::TEMPERATURE_MEASUREMENT,
            &[ZclAttributeReport {
                cluster_id: ZclClusterId::TEMPERATURE_MEASUREMENT,
                attribute_id: ZclAttributeId::MEASURED_VALUE,
                data_type: ZclDataType::I16,
                value: ZclValue::I16(2_175),
            }],
        );

        let events = integration
            .ingest_aps_frame(&mut runtime, NetworkAddress(0x1234), &bytes, 2_000)
            .unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].state_delta.as_ref().unwrap().value,
            Value::Number(21.75)
        );
        let state = runtime
            .registry()
            .state(&installed.endpoints[1].entity_id)
            .unwrap();
        assert_eq!(state.source, StateSource::EventStream);
        assert_eq!(state.confidence, StateConfidence::Confirmed);
    }

    #[test]
    fn authorized_command_emits_round_trippable_aps_and_zcl_bytes() {
        let mut integration = integration();
        let mut runtime = SmartHomeRuntime::new();
        let installed = install(&mut integration, &mut runtime);
        let principal = AgentId::trusted("agent:zigbee-test");
        grant(&mut runtime, &principal);

        let dispatch = integration
            .dispatch_command(
                &mut runtime,
                principal,
                RuntimeCommandToolRequest::new(
                    installed.endpoints[0].entity_id.clone(),
                    CommandType::SetBrightness,
                    Value::Percentage(42),
                )
                .with_idempotency_key("kitchen-zigbee-42"),
                3_000,
            )
            .unwrap();

        assert_eq!(dispatch.network_address, NetworkAddress(0x1234));
        assert_eq!(dispatch.cluster_id, ClusterId::LEVEL_CONTROL);
        assert_eq!(dispatch.zcl_frame.payload, vec![107, 0, 0]);
        let parsed_aps = ApsFrame::parse(&dispatch.aps_bytes).unwrap();
        assert!(parsed_aps.frame_control.ack_request);
        assert_eq!(parsed_aps.cluster_id, ClusterId::LEVEL_CONTROL);
        assert_eq!(
            ZclFrame::parse(&parsed_aps.payload).unwrap(),
            dispatch.zcl_frame
        );
        assert_eq!(
            dispatch.command_result.status,
            smart_home_core::CommandStatus::Accepted
        );
        assert_eq!(runtime.registry().counts().authorization_decisions, 2);
    }

    #[test]
    fn unauthorized_command_produces_no_wire_frame_and_records_denial() {
        let mut integration = integration();
        let mut runtime = SmartHomeRuntime::new();
        let installed = install(&mut integration, &mut runtime);

        let error = integration
            .dispatch_command(
                &mut runtime,
                AgentId::trusted("agent:unauthorized"),
                RuntimeCommandToolRequest::new(
                    installed.endpoints[0].entity_id.clone(),
                    CommandType::TurnOn,
                    Value::Null,
                ),
                4_000,
            )
            .unwrap_err();

        assert!(matches!(
            error,
            ZigbeeIntegrationError::Runtime(RuntimeError::UnauthorizedTool { .. })
        ));
        assert_eq!(runtime.registry().counts().authorization_decisions, 1);
        assert!(runtime
            .registry()
            .state(&EntityId::trusted("zigbee.entity.0x1234.1"))
            .is_none());
        assert_eq!(integration.next_aps_counter, 0);
        assert_eq!(integration.next_zcl_sequence, 0);
    }

    #[test]
    fn rejects_reports_from_uninterviewed_clusters() {
        let mut integration = integration();
        let mut runtime = SmartHomeRuntime::new();
        install(&mut integration, &mut runtime);
        let bytes = report_aps_frame(
            Endpoint(2),
            ZclClusterId::OCCUPANCY_SENSING,
            &[ZclAttributeReport {
                cluster_id: ZclClusterId::OCCUPANCY_SENSING,
                attribute_id: ZclAttributeId::OCCUPANCY,
                data_type: ZclDataType::Bitmap8,
                value: ZclValue::Bitmap8(1),
            }],
        );

        let error = integration
            .ingest_aps_frame(&mut runtime, NetworkAddress(0x1234), &bytes, 5_000)
            .unwrap_err();
        assert!(matches!(
            error,
            ZigbeeIntegrationError::UnexpectedCluster { .. }
        ));
    }

    #[test]
    fn validates_coordinator_and_interview_boundaries() {
        let bad = ZigbeeCoordinatorConfig::new(BridgeId::trusted("bad"), "", 1, 27, IeeeAddress(1));
        assert!(matches!(
            ZigbeeRuntimeIntegration::new(bad),
            Err(ZigbeeIntegrationError::Validation(_))
        ));

        let mut integration = integration();
        let mut runtime = SmartHomeRuntime::new();
        integration
            .install_coordinator(&mut runtime, 1_000)
            .unwrap();
        let mut incomplete = interview();
        incomplete.ieee_address = None;
        assert!(matches!(
            integration.install_device(
                &mut runtime,
                ZigbeeDeviceInstallation::new(incomplete, "Incomplete")
            ),
            Err(ZigbeeIntegrationError::IncompleteInterview(_))
        ));
    }
}
