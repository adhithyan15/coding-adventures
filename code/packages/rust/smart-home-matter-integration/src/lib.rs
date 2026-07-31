//! Matter application integration for the normalized smart-home runtime.

#![forbid(unsafe_code)]

use matter_core::{
    capabilities_for_cluster, percentage_to_level, state_delta_for_attribute_report,
    MatterAttributeReport, MatterCluster, MatterClusterId, MatterCommand, MatterCommandInvocation,
    MatterEndpointId, MatterError, MatterFabricId, MatterNodeId, MatterValue,
};
use smart_home_core::{
    AgentId, Bridge, BridgeId, BridgeTransport, Capability, CommandResult, CommandType, Device,
    DeviceEvent, DeviceEventType, DeviceId, Entity, EntityId, EntityKind, EventId, Health,
    IntegrationId, Metadata, ProtocolFamily, ProtocolIdentifier, Value, VaultRef,
};
use smart_home_runtime::{RuntimeCommandToolRequest, RuntimeError, SmartHomeRuntime};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const VERSION: &str = "0.1.0";
pub const INTEGRATION_ID: &str = "matter";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatterControllerConfig {
    pub bridge_id: BridgeId,
    pub fabric_id: MatterFabricId,
    pub controller_node_id: MatterNodeId,
    pub host_endpoint: String,
    pub manufacturer: String,
    pub model: String,
    pub firmware_version: Option<String>,
    pub fabric_credential_ref: VaultRef,
}

impl MatterControllerConfig {
    pub fn new(
        bridge_id: BridgeId,
        fabric_id: MatterFabricId,
        controller_node_id: MatterNodeId,
        host_endpoint: impl Into<String>,
        fabric_credential_ref: VaultRef,
    ) -> Self {
        Self {
            bridge_id,
            fabric_id,
            controller_node_id,
            host_endpoint: host_endpoint.into(),
            manufacturer: "Matter".to_string(),
            model: "Controller Host".to_string(),
            firmware_version: None,
            fabric_credential_ref,
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatterEndpointDescriptor {
    pub endpoint_id: MatterEndpointId,
    pub cluster_ids: Vec<MatterClusterId>,
}

impl MatterEndpointDescriptor {
    pub fn new(
        endpoint_id: MatterEndpointId,
        cluster_ids: impl IntoIterator<Item = MatterClusterId>,
    ) -> Self {
        Self {
            endpoint_id,
            cluster_ids: cluster_ids.into_iter().collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatterNodeInstallation {
    pub node_id: MatterNodeId,
    pub name: String,
    pub manufacturer: String,
    pub model: String,
    pub room_id: Option<String>,
    pub endpoints: Vec<MatterEndpointDescriptor>,
}

impl MatterNodeInstallation {
    pub fn new(
        node_id: MatterNodeId,
        name: impl Into<String>,
        endpoints: impl IntoIterator<Item = MatterEndpointDescriptor>,
    ) -> Self {
        Self {
            node_id,
            name: name.into(),
            manufacturer: "Matter".to_string(),
            model: "Matter Node".to_string(),
            room_id: None,
            endpoints: endpoints.into_iter().collect(),
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

    pub fn in_room(mut self, room_id: impl Into<String>) -> Self {
        self.room_id = Some(room_id.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledMatterEndpoint {
    pub node_id: MatterNodeId,
    pub endpoint_id: MatterEndpointId,
    pub entity_id: EntityId,
    pub entity_kind: EntityKind,
    pub capability_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledMatterNode {
    pub node_id: MatterNodeId,
    pub device_id: DeviceId,
    pub endpoints: Vec<InstalledMatterEndpoint>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatterCommandDispatch {
    pub command_result: CommandResult,
    pub invocation: MatterCommandInvocation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EndpointBinding {
    node_id: MatterNodeId,
    endpoint_id: MatterEndpointId,
    device_id: DeviceId,
    entity_id: EntityId,
    cluster_ids: BTreeSet<MatterClusterId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatterRuntimeIntegration {
    config: MatterControllerConfig,
    endpoints: BTreeMap<(MatterNodeId, MatterEndpointId), EndpointBinding>,
    next_event_sequence: u64,
}

impl MatterRuntimeIntegration {
    pub fn new(config: MatterControllerConfig) -> Result<Self, MatterIntegrationError> {
        if config.host_endpoint.trim().is_empty() {
            return Err(MatterIntegrationError::Validation(
                "host endpoint must not be empty".to_string(),
            ));
        }
        Ok(Self {
            config,
            endpoints: BTreeMap::new(),
            next_event_sequence: 1,
        })
    }

    pub fn config(&self) -> &MatterControllerConfig {
        &self.config
    }

    pub fn install_controller(
        &self,
        runtime: &mut SmartHomeRuntime,
        observed_at_ms: u64,
    ) -> Result<Option<Bridge>, MatterIntegrationError> {
        let mut bridge = Bridge::new(
            self.config.bridge_id.clone(),
            IntegrationId::trusted(INTEGRATION_ID),
            BridgeTransport::LocalProcess,
        );
        bridge.address = Some(self.config.host_endpoint.clone());
        bridge.hardware_model = Some(self.config.model.clone());
        bridge.firmware_version = self.config.firmware_version.clone();
        bridge.auth_ref = Some(self.config.fabric_credential_ref.clone());
        bridge.health = Health::Online;
        bridge.last_seen_at_ms = Some(observed_at_ms);
        bridge.identifiers = vec![
            protocol_identifier(
                "fabric_id",
                format!("0x{:016x}", self.config.fabric_id.value()),
            )?,
            protocol_identifier(
                "controller_node_id",
                format!("0x{:016x}", self.config.controller_node_id.value()),
            )?,
        ];
        bridge.metadata = vec![
            Metadata::new("matter.controller.manufacturer", &self.config.manufacturer),
            Metadata::new("matter.controller.model", &self.config.model),
            Metadata::new("matter.host_boundary", "commissioned_secure_session"),
        ];
        runtime.upsert_bridge(bridge).map_err(Into::into)
    }

    pub fn install_node(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        installation: MatterNodeInstallation,
    ) -> Result<InstalledMatterNode, MatterIntegrationError> {
        if runtime.registry().bridge(&self.config.bridge_id).is_none() {
            return Err(MatterIntegrationError::ControllerNotInstalled(
                self.config.bridge_id.clone(),
            ));
        }
        if installation.endpoints.is_empty() {
            return Err(MatterIntegrationError::NoEndpoints(installation.node_id));
        }

        let device_id = device_id(self.config.fabric_id, installation.node_id);
        let mut device = Device {
            device_id: device_id.clone(),
            bridge_id: self.config.bridge_id.clone(),
            manufacturer: installation.manufacturer,
            model: installation.model,
            name: installation.name.clone(),
            serial: Some(format!("0x{:016x}", installation.node_id.value())),
            firmware_version: None,
            room_id: installation.room_id,
            entity_ids: Vec::new(),
            identifiers: vec![
                protocol_identifier(
                    "fabric_node",
                    format!(
                        "0x{:016x}:0x{:016x}",
                        self.config.fabric_id.value(),
                        installation.node_id.value()
                    ),
                )?,
                protocol_identifier(
                    "node_id",
                    format!("0x{:016x}", installation.node_id.value()),
                )?,
            ],
            health: Health::Online,
            metadata: vec![Metadata::new(
                "matter.endpoint_count",
                installation.endpoints.len().to_string(),
            )],
        };

        let mut entities = Vec::new();
        let mut installed_endpoints = Vec::new();
        for descriptor in installation.endpoints {
            let Some((entity, binding, installed)) = endpoint_projection(
                &device,
                installation.node_id,
                descriptor,
                &installation.name,
            ) else {
                continue;
            };
            device.entity_ids.push(entity.entity_id.clone());
            self.endpoints
                .insert((installation.node_id, binding.endpoint_id), binding);
            installed_endpoints.push(installed);
            entities.push(entity);
        }
        if installed_endpoints.is_empty() {
            return Err(MatterIntegrationError::NoSupportedEndpoints(
                installation.node_id,
            ));
        }

        runtime.upsert_device(device)?;
        for entity in entities {
            runtime.upsert_entity(entity)?;
        }
        Ok(InstalledMatterNode {
            node_id: installation.node_id,
            device_id,
            endpoints: installed_endpoints,
        })
    }

    pub fn ingest_attribute_reports(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        reports: &[MatterAttributeReport],
        observed_at_ms: u64,
    ) -> Result<Vec<DeviceEvent>, MatterIntegrationError> {
        if reports.is_empty() {
            return Err(MatterIntegrationError::EmptyReportBatch);
        }
        let mut events = Vec::with_capacity(reports.len());
        for (index, report) in reports.iter().enumerate() {
            let binding = self
                .endpoints
                .get(&(report.node_id, report.endpoint_id))
                .cloned()
                .ok_or(MatterIntegrationError::UnknownEndpoint {
                    node_id: report.node_id,
                    endpoint_id: report.endpoint_id,
                })?;
            if !binding.cluster_ids.contains(&report.cluster_id) {
                return Err(MatterIntegrationError::UnexpectedCluster {
                    endpoint_id: report.endpoint_id,
                    cluster_id: report.cluster_id,
                });
            }
            let state_delta = state_delta_for_attribute_report(report)?;
            let event = DeviceEvent {
                event_id: EventId::trusted(format!(
                    "matter-event:{:016x}:{:016x}:{}:{}",
                    self.config.fabric_id.value(),
                    report.node_id.value(),
                    self.next_event_sequence,
                    index
                )),
                bridge_id: self.config.bridge_id.clone(),
                device_id: Some(binding.device_id),
                entity_id: Some(binding.entity_id),
                observed_at_ms,
                received_at_ms: observed_at_ms,
                event_type: DeviceEventType::Updated,
                state_delta: Some(state_delta),
                raw_ref: Some(format!(
                    "matter://fabric/0x{:016x}/node/0x{:016x}/endpoint/{}/cluster/0x{:08x}",
                    self.config.fabric_id.value(),
                    report.node_id.value(),
                    report.endpoint_id.value(),
                    report.cluster_id.value()
                )),
                correlation_id: None,
                metadata: vec![
                    Metadata::new("matter.endpoint_id", report.endpoint_id.value().to_string()),
                    Metadata::new(
                        "matter.cluster_id",
                        format!("0x{:08x}", report.cluster_id.value()),
                    ),
                    Metadata::new(
                        "matter.attribute_id",
                        format!("0x{:08x}", report.attribute_id.value()),
                    ),
                ],
            };
            runtime.apply_device_event(event.clone())?;
            events.push(event);
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
    ) -> Result<MatterCommandDispatch, MatterIntegrationError> {
        let binding = self
            .endpoints
            .values()
            .find(|binding| binding.entity_id == request.entity_id)
            .cloned()
            .ok_or_else(|| MatterIntegrationError::UnknownEntity(request.entity_id.clone()))?;
        let plan = command_plan(&binding, &request)?;

        // Authorization and durable command audit happen before an invocation exists.
        let command_result = runtime.execute_command_tool(principal_id, request, now_ms)?;
        Ok(MatterCommandDispatch {
            command_result,
            invocation: plan.into_invocation(binding.node_id, binding.endpoint_id),
        })
    }
}

fn endpoint_projection(
    device: &Device,
    node_id: MatterNodeId,
    descriptor: MatterEndpointDescriptor,
    device_name: &str,
) -> Option<(Entity, EndpointBinding, InstalledMatterEndpoint)> {
    let cluster_ids = descriptor.cluster_ids.into_iter().collect::<BTreeSet<_>>();
    let mut capabilities = BTreeMap::new();
    for cluster_id in &cluster_ids {
        for capability in capabilities_for_cluster(*cluster_id) {
            capabilities
                .entry(capability.capability_id.as_str().to_string())
                .or_insert(capability);
        }
    }
    let capabilities = capabilities.into_values().collect::<Vec<Capability>>();
    if capabilities.is_empty() {
        return None;
    }
    let entity_kind = entity_kind_for_clusters(&cluster_ids);
    let entity_id = EntityId::trusted(format!(
        "matter.entity.{:016x}.{}",
        node_id.value(),
        descriptor.endpoint_id.value()
    ));
    let capability_ids = capabilities
        .iter()
        .map(|capability| capability.capability_id.as_str().to_string())
        .collect();
    let cluster_list = cluster_ids
        .iter()
        .map(|cluster| format!("0x{:08x}", cluster.value()))
        .collect::<Vec<_>>()
        .join(",");
    let entity = Entity {
        entity_id: entity_id.clone(),
        device_id: device.device_id.clone(),
        kind: entity_kind,
        name: if device.entity_ids.is_empty() {
            device_name.to_string()
        } else {
            format!("{device_name} endpoint {}", descriptor.endpoint_id.value())
        },
        capabilities,
        state: None,
        metadata: vec![
            Metadata::new(
                "matter.endpoint_id",
                descriptor.endpoint_id.value().to_string(),
            ),
            Metadata::new("matter.server_clusters", cluster_list),
        ],
    };
    let binding = EndpointBinding {
        node_id,
        endpoint_id: descriptor.endpoint_id,
        device_id: device.device_id.clone(),
        entity_id: entity_id.clone(),
        cluster_ids,
    };
    let installed = InstalledMatterEndpoint {
        node_id,
        endpoint_id: descriptor.endpoint_id,
        entity_id,
        entity_kind,
        capability_ids,
    };
    Some((entity, binding, installed))
}

fn entity_kind_for_clusters(clusters: &BTreeSet<MatterClusterId>) -> EntityKind {
    for preferred in [
        MatterCluster::DOOR_LOCK,
        MatterCluster::THERMOSTAT,
        MatterCluster::ON_OFF,
        MatterCluster::LEVEL_CONTROL,
        MatterCluster::COLOR_CONTROL,
        MatterCluster::TEMPERATURE_MEASUREMENT,
    ] {
        if clusters.contains(&preferred) {
            if let Some(kind) = MatterCluster::from_id(preferred).entity_kind() {
                return kind;
            }
        }
    }
    clusters
        .iter()
        .find_map(|cluster| MatterCluster::from_id(*cluster).entity_kind())
        .unwrap_or(EntityKind::Unknown)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MatterCommandPlan {
    Off,
    On,
    Level(u8),
    ColorTemperature(u16),
    Lock,
    Unlock,
}

impl MatterCommandPlan {
    fn cluster_id(self) -> MatterClusterId {
        match self {
            Self::Off | Self::On => MatterCluster::ON_OFF,
            Self::Level(_) => MatterCluster::LEVEL_CONTROL,
            Self::ColorTemperature(_) => MatterCluster::COLOR_CONTROL,
            Self::Lock | Self::Unlock => MatterCluster::DOOR_LOCK,
        }
    }

    fn into_invocation(
        self,
        node_id: MatterNodeId,
        endpoint_id: MatterEndpointId,
    ) -> MatterCommandInvocation {
        match self {
            Self::Off => MatterCommandInvocation::command(node_id, endpoint_id, MatterCommand::Off),
            Self::On => MatterCommandInvocation::command(node_id, endpoint_id, MatterCommand::On),
            Self::Level(percent) => MatterCommandInvocation::new(
                node_id,
                endpoint_id,
                MatterCommand::MoveToLevelWithOnOff,
                vec![
                    (
                        "level".to_string(),
                        MatterValue::U64(u64::from(percentage_to_level(percent))),
                    ),
                    ("transition_time_ds".to_string(), MatterValue::U64(0)),
                ],
            ),
            Self::ColorTemperature(mirek) => MatterCommandInvocation::new(
                node_id,
                endpoint_id,
                MatterCommand::MoveToColorTemperature,
                vec![
                    (
                        "color_temperature_mireds".to_string(),
                        MatterValue::U64(u64::from(mirek)),
                    ),
                    ("transition_time_ds".to_string(), MatterValue::U64(0)),
                ],
            ),
            Self::Lock => {
                MatterCommandInvocation::command(node_id, endpoint_id, MatterCommand::LockDoor)
            }
            Self::Unlock => {
                MatterCommandInvocation::command(node_id, endpoint_id, MatterCommand::UnlockDoor)
            }
        }
    }
}

fn command_plan(
    binding: &EndpointBinding,
    request: &RuntimeCommandToolRequest,
) -> Result<MatterCommandPlan, MatterIntegrationError> {
    let plan = match request.command_type {
        CommandType::TurnOn => MatterCommandPlan::On,
        CommandType::TurnOff => MatterCommandPlan::Off,
        CommandType::SetBrightness => {
            let Value::Percentage(percent) = &request.arguments else {
                return Err(MatterIntegrationError::InvalidCommandArguments {
                    command_type: request.command_type,
                    expected: "percentage",
                });
            };
            MatterCommandPlan::Level(*percent)
        }
        CommandType::SetColorTemperature => {
            let Value::Integer(mirek) = &request.arguments else {
                return Err(MatterIntegrationError::InvalidCommandArguments {
                    command_type: request.command_type,
                    expected: "positive integer mirek",
                });
            };
            MatterCommandPlan::ColorTemperature(u16::try_from(*mirek).map_err(|_| {
                MatterIntegrationError::InvalidCommandArguments {
                    command_type: request.command_type,
                    expected: "positive integer mirek",
                }
            })?)
        }
        CommandType::SetLock => match &request.arguments {
            Value::Bool(true) => MatterCommandPlan::Lock,
            Value::Bool(false) => MatterCommandPlan::Unlock,
            Value::Text(state) if state.eq_ignore_ascii_case("locked") => MatterCommandPlan::Lock,
            Value::Text(state) if state.eq_ignore_ascii_case("unlocked") => {
                MatterCommandPlan::Unlock
            }
            _ => {
                return Err(MatterIntegrationError::InvalidCommandArguments {
                    command_type: request.command_type,
                    expected: "`locked`, `unlocked`, or boolean",
                });
            }
        },
        _ => {
            return Err(MatterIntegrationError::UnsupportedCommand {
                entity_id: request.entity_id.clone(),
                command_type: request.command_type,
            });
        }
    };
    if !binding.cluster_ids.contains(&plan.cluster_id()) {
        return Err(MatterIntegrationError::UnsupportedCommand {
            entity_id: request.entity_id.clone(),
            command_type: request.command_type,
        });
    }
    Ok(plan)
}

fn device_id(fabric_id: MatterFabricId, node_id: MatterNodeId) -> DeviceId {
    DeviceId::trusted(format!(
        "matter.device.{:016x}.{:016x}",
        fabric_id.value(),
        node_id.value()
    ))
}

fn protocol_identifier(
    kind: impl Into<String>,
    value: impl Into<String>,
) -> Result<ProtocolIdentifier, MatterIntegrationError> {
    ProtocolIdentifier::new(ProtocolFamily::Matter, kind, value)
        .map_err(|error| MatterIntegrationError::Validation(error.to_string()))
}

#[derive(Debug)]
pub enum MatterIntegrationError {
    Validation(String),
    ControllerNotInstalled(BridgeId),
    NoEndpoints(MatterNodeId),
    NoSupportedEndpoints(MatterNodeId),
    EmptyReportBatch,
    UnknownEndpoint {
        node_id: MatterNodeId,
        endpoint_id: MatterEndpointId,
    },
    UnknownEntity(EntityId),
    UnexpectedCluster {
        endpoint_id: MatterEndpointId,
        cluster_id: MatterClusterId,
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
    Matter(MatterError),
}

impl fmt::Display for MatterIntegrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => write!(f, "invalid Matter integration: {message}"),
            Self::ControllerNotInstalled(bridge_id) => {
                write!(f, "Matter controller {bridge_id} is not installed")
            }
            Self::NoEndpoints(node_id) => write!(f, "Matter node {node_id} has no endpoints"),
            Self::NoSupportedEndpoints(node_id) => {
                write!(
                    f,
                    "Matter node {node_id} has no supported endpoint clusters"
                )
            }
            Self::EmptyReportBatch => write!(f, "Matter attribute report batch is empty"),
            Self::UnknownEndpoint {
                node_id,
                endpoint_id,
            } => write!(f, "unknown Matter endpoint {node_id}/{endpoint_id}"),
            Self::UnknownEntity(entity_id) => write!(f, "unknown Matter entity {entity_id}"),
            Self::UnexpectedCluster {
                endpoint_id,
                cluster_id,
            } => write!(
                f,
                "Matter cluster {cluster_id} was not installed on {endpoint_id}"
            ),
            Self::UnsupportedCommand {
                entity_id,
                command_type,
            } => write!(
                f,
                "Matter entity {entity_id} does not support {command_type:?}"
            ),
            Self::InvalidCommandArguments {
                command_type,
                expected,
            } => write!(f, "invalid {command_type:?} arguments; expected {expected}"),
            Self::Runtime(error) => error.fmt(f),
            Self::Matter(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for MatterIntegrationError {}

impl From<RuntimeError> for MatterIntegrationError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

impl From<MatterError> for MatterIntegrationError {
    fn from(error: MatterError) -> Self {
        Self::Matter(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use matter_core::{MatterAttribute, MatterValue};
    use smart_home_core::{
        CapabilityGrant, CapabilityGrantId, PrivilegeTier, StateConfidence, StateSource,
    };

    const FABRIC: MatterFabricId = MatterFabricId::new(0x1111);
    const NODE: MatterNodeId = MatterNodeId::new(0x2222);

    fn integration() -> MatterRuntimeIntegration {
        MatterRuntimeIntegration::new(
            MatterControllerConfig::new(
                BridgeId::trusted("matter-controller-1"),
                FABRIC,
                MatterNodeId::new(1),
                "unix:///var/run/matter-controller.sock",
                VaultRef::trusted("vault:matter/fabric-1111"),
            )
            .with_identity("Open Home", "Matter Controller")
            .with_firmware_version("2026.07"),
        )
        .unwrap()
    }

    fn installation() -> MatterNodeInstallation {
        MatterNodeInstallation::new(
            NODE,
            "Entry Matter Device",
            [
                MatterEndpointDescriptor::new(
                    MatterEndpointId::trusted(1),
                    [
                        MatterCluster::ON_OFF,
                        MatterCluster::LEVEL_CONTROL,
                        MatterCluster::COLOR_CONTROL,
                    ],
                ),
                MatterEndpointDescriptor::new(
                    MatterEndpointId::trusted(2),
                    [MatterCluster::DOOR_LOCK],
                ),
                MatterEndpointDescriptor::new(
                    MatterEndpointId::trusted(3),
                    [
                        MatterCluster::TEMPERATURE_MEASUREMENT,
                        MatterCluster::RELATIVE_HUMIDITY_MEASUREMENT,
                    ],
                ),
            ],
        )
        .with_identity("Acme", "Matter Combo")
        .in_room("entry")
    }

    fn install(
        integration: &mut MatterRuntimeIntegration,
        runtime: &mut SmartHomeRuntime,
    ) -> InstalledMatterNode {
        integration.install_controller(runtime, 1_000).unwrap();
        integration.install_node(runtime, installation()).unwrap()
    }

    fn grant(runtime: &mut SmartHomeRuntime, principal: &AgentId) {
        runtime
            .registry_mut()
            .upsert_capability_grant(CapabilityGrant::for_all_smart_home(
                CapabilityGrantId::trusted("grant-matter-test"),
                principal.clone(),
                PrivilegeTier::HighRisk,
                "test",
                0,
            ));
    }

    #[test]
    fn installs_controller_and_commissioned_endpoint_topology() {
        let mut integration = integration();
        let mut runtime = SmartHomeRuntime::new();
        let installed = install(&mut integration, &mut runtime);

        assert_eq!(runtime.topology_summary().bridges, 1);
        assert_eq!(runtime.topology_summary().devices, 1);
        assert_eq!(runtime.topology_summary().entities, 3);
        assert_eq!(installed.endpoints[0].entity_kind, EntityKind::Light);
        assert_eq!(installed.endpoints[1].entity_kind, EntityKind::Lock);
        assert_eq!(installed.endpoints[2].entity_kind, EntityKind::Sensor);
        let bridge = runtime
            .registry()
            .bridge(&BridgeId::trusted("matter-controller-1"))
            .unwrap();
        assert_eq!(
            bridge.auth_ref.as_ref().map(VaultRef::as_str),
            Some("vault:matter/fabric-1111")
        );
        assert!(!format!("{bridge:?}").contains("certificate_bytes"));
    }

    #[test]
    fn typed_reports_update_confirmed_runtime_state() {
        let mut integration = integration();
        let mut runtime = SmartHomeRuntime::new();
        let installed = install(&mut integration, &mut runtime);
        let reports = [
            MatterAttributeReport::new(
                NODE,
                MatterEndpointId::trusted(1),
                MatterCluster::ON_OFF,
                MatterAttribute::ON_OFF,
                MatterValue::Bool(true),
            ),
            MatterAttributeReport::new(
                NODE,
                MatterEndpointId::trusted(3),
                MatterCluster::TEMPERATURE_MEASUREMENT,
                MatterAttribute::MEASURED_VALUE,
                MatterValue::I64(2_125),
            ),
        ];

        let events = integration
            .ingest_attribute_reports(&mut runtime, &reports, 2_000)
            .unwrap();

        assert_eq!(events.len(), 2);
        assert_eq!(
            events[1].state_delta.as_ref().unwrap().value,
            Value::Number(21.25)
        );
        let state = runtime
            .registry()
            .state(&installed.endpoints[2].entity_id)
            .unwrap();
        assert_eq!(state.source, StateSource::EventStream);
        assert_eq!(state.confidence, StateConfidence::Confirmed);
    }

    #[test]
    fn authorized_command_creates_matter_invocation_after_audit() {
        let mut integration = integration();
        let mut runtime = SmartHomeRuntime::new();
        let installed = install(&mut integration, &mut runtime);
        let principal = AgentId::trusted("agent:matter-test");
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
                .with_idempotency_key("entry-matter-42"),
                3_000,
            )
            .unwrap();

        assert_eq!(dispatch.invocation.node_id, NODE);
        assert_eq!(
            dispatch.invocation.endpoint_id,
            MatterEndpointId::trusted(1)
        );
        assert_eq!(dispatch.invocation.cluster_id, MatterCluster::LEVEL_CONTROL);
        assert_eq!(
            dispatch.invocation.argument("level"),
            Some(&MatterValue::U64(107))
        );
        assert_eq!(
            dispatch.command_result.status,
            smart_home_core::CommandStatus::Accepted
        );
        assert_eq!(runtime.registry().counts().authorization_decisions, 2);
    }

    #[test]
    fn lock_command_uses_high_risk_runtime_authorization() {
        let mut integration = integration();
        let mut runtime = SmartHomeRuntime::new();
        let installed = install(&mut integration, &mut runtime);
        let principal = AgentId::trusted("agent:matter-lock-test");
        grant(&mut runtime, &principal);

        let dispatch = integration
            .dispatch_command(
                &mut runtime,
                principal,
                RuntimeCommandToolRequest::new(
                    installed.endpoints[1].entity_id.clone(),
                    CommandType::SetLock,
                    Value::Text("locked".to_string()),
                ),
                3_100,
            )
            .unwrap();
        assert_eq!(dispatch.invocation.cluster_id, MatterCluster::DOOR_LOCK);
        assert_eq!(dispatch.invocation.command_id, MatterCommand::LOCK_DOOR);
    }

    #[test]
    fn denied_command_produces_no_invocation_and_records_denial() {
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
            MatterIntegrationError::Runtime(RuntimeError::UnauthorizedTool { .. })
        ));
        assert_eq!(runtime.registry().counts().authorization_decisions, 1);
        assert!(runtime
            .registry()
            .state(&installed.endpoints[0].entity_id)
            .is_none());
    }

    #[test]
    fn validates_host_installation_and_report_boundaries() {
        let bad = MatterControllerConfig::new(
            BridgeId::trusted("bad"),
            FABRIC,
            MatterNodeId::new(1),
            "",
            VaultRef::trusted("vault:matter/bad"),
        );
        assert!(matches!(
            MatterRuntimeIntegration::new(bad),
            Err(MatterIntegrationError::Validation(_))
        ));

        let mut integration = integration();
        let mut runtime = SmartHomeRuntime::new();
        install(&mut integration, &mut runtime);
        let report = MatterAttributeReport::new(
            NODE,
            MatterEndpointId::trusted(3),
            MatterCluster::OCCUPANCY_SENSING,
            MatterAttribute::OCCUPANCY,
            MatterValue::U64(1),
        );
        assert!(matches!(
            integration.ingest_attribute_reports(&mut runtime, &[report], 5_000),
            Err(MatterIntegrationError::UnexpectedCluster { .. })
        ));
    }
}
