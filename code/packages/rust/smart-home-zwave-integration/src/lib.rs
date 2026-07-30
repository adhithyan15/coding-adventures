//! Z-Wave Serial API integration for the normalized smart-home runtime.

#![forbid(unsafe_code)]

use smart_home_core::{
    AgentId, Bridge, BridgeId, BridgeTransport, Capability, CommandId, CommandResult, CommandType,
    Device, DeviceEvent, DeviceEventType, DeviceId, Entity, EntityId, EntityKind, EventId, Health,
    IntegrationId, Metadata, ProtocolFamily, ProtocolIdentifier, StateDelta, Value,
};
use smart_home_runtime::{RuntimeCommandToolRequest, RuntimeError, SmartHomeRuntime};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;
use zwave_command_classes::{
    binary_switch_set, capabilities_for_command_class, door_lock_operation_set,
    multilevel_switch_set, parse_value_report, state_delta_for_report, zwave_bool,
    CommandClassError, ZWaveCommand, BASIC_SET, COMMAND_CLASS_METER, COMMAND_CLASS_NOTIFICATION,
};
use zwave_core::{CommandClassFrame, CommandClassId, HomeId, NodeId};
use zwave_serial_api::{
    ApplicationCommand, FunctionId, SendDataCallback, SendDataRequest, SendDataResponse,
    SendDataTransaction, SendDataTransactionState, SerialApiError, SerialMessage,
    SerialMessageKind, TransmitOptions,
};

pub const VERSION: &str = "0.1.0";
pub const INTEGRATION_ID: &str = "zwave";
pub const DEFAULT_COMMAND_TIMEOUT_MS: u64 = 5_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZWaveControllerConfig {
    pub bridge_id: BridgeId,
    pub home_id: HomeId,
    pub controller_node_id: NodeId,
    pub serial_path: String,
    pub manufacturer: String,
    pub model: String,
    pub firmware_version: Option<String>,
}

impl ZWaveControllerConfig {
    pub fn new(
        bridge_id: BridgeId,
        home_id: HomeId,
        controller_node_id: NodeId,
        serial_path: impl Into<String>,
    ) -> Self {
        Self {
            bridge_id,
            home_id,
            controller_node_id,
            serial_path: serial_path.into(),
            manufacturer: "Z-Wave".to_string(),
            model: "Serial API Controller".to_string(),
            firmware_version: None,
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
pub struct ZWaveNodeInterview {
    pub node_id: NodeId,
    pub name: String,
    pub manufacturer: String,
    pub model: String,
    pub room_id: Option<String>,
    pub command_classes: Vec<CommandClassId>,
}

impl ZWaveNodeInterview {
    pub fn new(
        node_id: NodeId,
        name: impl Into<String>,
        command_classes: impl IntoIterator<Item = CommandClassId>,
    ) -> Self {
        Self {
            node_id,
            name: name.into(),
            manufacturer: "Unknown".to_string(),
            model: "Unknown".to_string(),
            room_id: None,
            command_classes: command_classes.into_iter().collect(),
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
pub struct InstalledZWaveNode {
    pub node_id: NodeId,
    pub device_id: DeviceId,
    pub entity_id: EntityId,
    pub entity_kind: EntityKind,
    pub capability_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ZWaveNodeBinding {
    node_id: NodeId,
    device_id: DeviceId,
    entity_id: EntityId,
    command_classes: BTreeSet<CommandClassId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZWaveCommandDispatch {
    pub command_result: CommandResult,
    pub callback_id: u8,
    pub command: ZWaveCommand,
    pub send_data_request: SendDataRequest,
    pub serial_message: SerialMessage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZWaveDispatchState {
    pub callback_id: u8,
    pub command_id: CommandId,
    pub state: SendDataTransactionState,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ZWaveSerialOutcome {
    StateEvent(Box<DeviceEvent>),
    DispatchState(ZWaveDispatchState),
    Ignored,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingDispatch {
    command_id: CommandId,
    transaction: SendDataTransaction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZWaveRuntimeIntegration {
    config: ZWaveControllerConfig,
    nodes: BTreeMap<NodeId, ZWaveNodeBinding>,
    pending_dispatches: BTreeMap<u8, PendingDispatch>,
    awaiting_responses: VecDeque<u8>,
    next_callback_id: u8,
    next_event_sequence: u64,
}

impl ZWaveRuntimeIntegration {
    pub fn new(config: ZWaveControllerConfig) -> Result<Self, ZWaveIntegrationError> {
        if config.serial_path.trim().is_empty() {
            return Err(ZWaveIntegrationError::Validation(
                "serial path must not be empty".to_string(),
            ));
        }
        Ok(Self {
            config,
            nodes: BTreeMap::new(),
            pending_dispatches: BTreeMap::new(),
            awaiting_responses: VecDeque::new(),
            next_callback_id: 1,
            next_event_sequence: 1,
        })
    }

    pub fn config(&self) -> &ZWaveControllerConfig {
        &self.config
    }

    pub fn install_controller(
        &self,
        runtime: &mut SmartHomeRuntime,
        observed_at_ms: u64,
    ) -> Result<Option<Bridge>, ZWaveIntegrationError> {
        let mut bridge = Bridge::new(
            self.config.bridge_id.clone(),
            IntegrationId::trusted(INTEGRATION_ID),
            BridgeTransport::Serial,
        );
        bridge.address = Some(self.config.serial_path.clone());
        bridge.hardware_model = Some(self.config.model.clone());
        bridge.firmware_version = self.config.firmware_version.clone();
        bridge.health = Health::Online;
        bridge.last_seen_at_ms = Some(observed_at_ms);
        bridge.identifiers = vec![
            ProtocolIdentifier::new(
                ProtocolFamily::ZWave,
                "home_id",
                home_id_string(self.config.home_id),
            )
            .map_err(|error| ZWaveIntegrationError::Validation(error.to_string()))?,
            ProtocolIdentifier::new(
                ProtocolFamily::ZWave,
                "controller_node_id",
                node_id_string(self.config.controller_node_id),
            )
            .map_err(|error| ZWaveIntegrationError::Validation(error.to_string()))?,
        ];
        bridge.metadata = vec![
            Metadata::new("zwave.controller.manufacturer", &self.config.manufacturer),
            Metadata::new("zwave.controller.model", &self.config.model),
        ];
        runtime
            .upsert_bridge(bridge)
            .map_err(ZWaveIntegrationError::Runtime)
    }

    pub fn install_node(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        interview: ZWaveNodeInterview,
    ) -> Result<InstalledZWaveNode, ZWaveIntegrationError> {
        if runtime.registry().bridge(&self.config.bridge_id).is_none() {
            return Err(ZWaveIntegrationError::ControllerNotInstalled(
                self.config.bridge_id.clone(),
            ));
        }

        let command_classes = interview
            .command_classes
            .into_iter()
            .collect::<BTreeSet<_>>();
        let capabilities = capabilities_for_node(&command_classes);
        let entity_kind = entity_kind_for_node(&command_classes);
        let node_key = format!(
            "{}:{}",
            home_id_string(self.config.home_id),
            node_id_string(interview.node_id)
        );
        let device_id = DeviceId::trusted(format!("zwave-device:{node_key}"));
        let entity_id = EntityId::trusted(format!("zwave-entity:{node_key}"));
        let identifiers = vec![
            ProtocolIdentifier::new(ProtocolFamily::ZWave, "home_node", node_key.clone())
                .map_err(|error| ZWaveIntegrationError::Validation(error.to_string()))?,
            ProtocolIdentifier::new(
                ProtocolFamily::ZWave,
                "node_id",
                node_id_string(interview.node_id),
            )
            .map_err(|error| ZWaveIntegrationError::Validation(error.to_string()))?,
        ];
        let command_class_list = command_classes
            .iter()
            .map(|command_class| format!("0x{:02x}", command_class.0))
            .collect::<Vec<_>>()
            .join(",");
        let device = Device {
            device_id: device_id.clone(),
            bridge_id: self.config.bridge_id.clone(),
            manufacturer: interview.manufacturer,
            model: interview.model,
            name: interview.name.clone(),
            serial: None,
            firmware_version: None,
            room_id: interview.room_id,
            entity_ids: vec![entity_id.clone()],
            identifiers,
            health: Health::Online,
            metadata: vec![Metadata::new(
                "zwave.command_classes",
                command_class_list.clone(),
            )],
        };
        let entity = Entity {
            entity_id: entity_id.clone(),
            device_id: device_id.clone(),
            kind: entity_kind,
            name: interview.name,
            capabilities: capabilities.clone(),
            state: None,
            metadata: vec![
                Metadata::new("zwave.node_id", node_id_string(interview.node_id)),
                Metadata::new("zwave.command_classes", command_class_list),
            ],
        };
        runtime.upsert_device(device)?;
        runtime.upsert_entity(entity)?;
        self.nodes.insert(
            interview.node_id,
            ZWaveNodeBinding {
                node_id: interview.node_id,
                device_id: device_id.clone(),
                entity_id: entity_id.clone(),
                command_classes,
            },
        );

        Ok(InstalledZWaveNode {
            node_id: interview.node_id,
            device_id,
            entity_id,
            entity_kind,
            capability_ids: capabilities
                .iter()
                .map(|capability| capability.capability_id.as_str().to_string())
                .collect(),
        })
    }

    pub fn ingest_application_command(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        message: &SerialMessage,
        observed_at_ms: u64,
    ) -> Result<DeviceEvent, ZWaveIntegrationError> {
        let application_command = ApplicationCommand::from_message(message)?;
        let binding = self
            .nodes
            .get(&application_command.source_node)
            .cloned()
            .ok_or(ZWaveIntegrationError::UnknownNode(
                application_command.source_node,
            ))?;
        let command = ZWaveCommand::parse(&application_command.command)?;
        let report = parse_value_report(&command)?;
        let state_delta = state_delta_for_report(&report);
        ensure_entity_capability(runtime, &binding.entity_id, &state_delta)?;

        let event = DeviceEvent {
            event_id: EventId::trusted(format!(
                "zwave-event:{}:{}:{}",
                home_id_string(self.config.home_id),
                node_id_string(application_command.source_node),
                self.next_event_sequence
            )),
            bridge_id: self.config.bridge_id.clone(),
            device_id: Some(binding.device_id),
            entity_id: Some(binding.entity_id),
            observed_at_ms,
            received_at_ms: observed_at_ms,
            event_type: DeviceEventType::Updated,
            state_delta: Some(state_delta),
            raw_ref: Some(format!(
                "zwave-serial://{}/node/{}",
                self.config.serial_path,
                node_id_string(application_command.source_node)
            )),
            correlation_id: None,
            metadata: vec![
                Metadata::new(
                    "zwave.command_class",
                    format!("0x{:02x}", command.command_class.0),
                ),
                Metadata::new("zwave.command_id", format!("0x{:02x}", command.command_id)),
                Metadata::new(
                    "zwave.rx_status",
                    format!("0x{:02x}", application_command.rx_status),
                ),
            ],
        };
        self.next_event_sequence = self.next_event_sequence.saturating_add(1);
        runtime.apply_device_event(event.clone())?;
        Ok(event)
    }

    pub fn dispatch_command(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        principal_id: AgentId,
        request: RuntimeCommandToolRequest,
        now_ms: u64,
    ) -> Result<ZWaveCommandDispatch, ZWaveIntegrationError> {
        let binding = self
            .nodes
            .values()
            .find(|binding| binding.entity_id == request.entity_id)
            .cloned()
            .ok_or_else(|| ZWaveIntegrationError::UnknownEntity(request.entity_id.clone()))?;
        let command = command_for_request(&binding, &request)?;
        let timeout_ms = request.timeout_ms.unwrap_or(DEFAULT_COMMAND_TIMEOUT_MS);
        let callback_id = self.next_available_callback_id()?;
        let send_data_request = SendDataRequest::new(
            binding.node_id,
            CommandClassFrame::new(
                command.command_class,
                command.command_id,
                command.payload.clone(),
            ),
            TransmitOptions::reliable(),
            callback_id,
        );
        let serial_message = send_data_request.to_message()?;
        let command_result = runtime.execute_command_tool(principal_id, request, now_ms)?;
        let transaction = SendDataTransaction::new(&send_data_request, now_ms, timeout_ms);
        self.next_callback_id = callback_id.wrapping_add(1).max(1);
        self.pending_dispatches.insert(
            callback_id,
            PendingDispatch {
                command_id: command_result.command_id.clone(),
                transaction,
            },
        );
        self.awaiting_responses.push_back(callback_id);

        Ok(ZWaveCommandDispatch {
            command_result,
            callback_id,
            command,
            send_data_request,
            serial_message,
        })
    }

    pub fn handle_serial_message(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        message: &SerialMessage,
        observed_at_ms: u64,
    ) -> Result<ZWaveSerialOutcome, ZWaveIntegrationError> {
        match (message.function_id, message.kind) {
            (FunctionId::APPLICATION_COMMAND_HANDLER, _) => self
                .ingest_application_command(runtime, message, observed_at_ms)
                .map(Box::new)
                .map(ZWaveSerialOutcome::StateEvent),
            (FunctionId::SEND_DATA, SerialMessageKind::Response) => {
                let response = SendDataResponse::from_message(message)?;
                let callback_id = self
                    .awaiting_responses
                    .pop_front()
                    .ok_or(ZWaveIntegrationError::UnexpectedSendDataResponse)?;
                let pending = self.pending_dispatch_mut(callback_id)?;
                let state = pending.transaction.on_response(response);
                Ok(ZWaveSerialOutcome::DispatchState(ZWaveDispatchState {
                    callback_id,
                    command_id: pending.command_id.clone(),
                    state,
                }))
            }
            (FunctionId::SEND_DATA, SerialMessageKind::Callback) => {
                let callback = SendDataCallback::from_message(message)?;
                let pending = self.pending_dispatch_mut(callback.callback_id)?;
                let state = pending.transaction.on_callback(callback)?;
                Ok(ZWaveSerialOutcome::DispatchState(ZWaveDispatchState {
                    callback_id: callback.callback_id,
                    command_id: pending.command_id.clone(),
                    state,
                }))
            }
            _ => Ok(ZWaveSerialOutcome::Ignored),
        }
    }

    pub fn expire_dispatches(&mut self, now_ms: u64) -> Vec<ZWaveDispatchState> {
        let mut expired = Vec::new();
        for (callback_id, pending) in &mut self.pending_dispatches {
            if pending.transaction.has_timed_out_at(now_ms) {
                expired.push(ZWaveDispatchState {
                    callback_id: *callback_id,
                    command_id: pending.command_id.clone(),
                    state: pending.transaction.expire_at(now_ms),
                });
            }
        }
        expired
    }

    pub fn dispatch_state(&self, callback_id: u8) -> Option<ZWaveDispatchState> {
        self.pending_dispatches
            .get(&callback_id)
            .map(|pending| ZWaveDispatchState {
                callback_id,
                command_id: pending.command_id.clone(),
                state: pending.transaction.state(),
            })
    }

    pub fn pending_dispatch_count(&self) -> usize {
        self.pending_dispatches
            .values()
            .filter(|pending| !pending.transaction.is_terminal())
            .count()
    }

    fn next_available_callback_id(&self) -> Result<u8, ZWaveIntegrationError> {
        let mut callback_id = self.next_callback_id.max(1);
        for _ in 0..u8::MAX {
            let available = match self.pending_dispatches.get(&callback_id) {
                None => true,
                Some(pending) => {
                    pending.transaction.is_terminal()
                        && pending.transaction.state() != SendDataTransactionState::TimedOut
                        && !self.awaiting_responses.contains(&callback_id)
                }
            };
            if available {
                return Ok(callback_id);
            }
            callback_id = callback_id.wrapping_add(1).max(1);
        }
        Err(ZWaveIntegrationError::CallbackIdsExhausted)
    }

    fn pending_dispatch_mut(
        &mut self,
        callback_id: u8,
    ) -> Result<&mut PendingDispatch, ZWaveIntegrationError> {
        self.pending_dispatches
            .get_mut(&callback_id)
            .ok_or(ZWaveIntegrationError::UnknownCallbackId(callback_id))
    }
}

fn capabilities_for_node(command_classes: &BTreeSet<CommandClassId>) -> Vec<Capability> {
    let mut capabilities = BTreeMap::new();
    for command_class in command_classes {
        let projected = if *command_class == CommandClassId::BASIC {
            vec![Capability::light_on_off()]
        } else {
            capabilities_for_command_class(*command_class)
        };
        for capability in projected {
            capabilities
                .entry(capability.capability_id.as_str().to_string())
                .or_insert(capability);
        }
    }
    capabilities.into_values().collect()
}

fn entity_kind_for_node(command_classes: &BTreeSet<CommandClassId>) -> EntityKind {
    if command_classes.contains(&CommandClassId::DOOR_LOCK) {
        EntityKind::Lock
    } else if command_classes.contains(&CommandClassId::SWITCH_BINARY)
        || command_classes.contains(&CommandClassId::SWITCH_MULTILEVEL)
        || command_classes.contains(&CommandClassId::BASIC)
    {
        EntityKind::Light
    } else if command_classes.contains(&CommandClassId::SENSOR_BINARY)
        || command_classes.contains(&CommandClassId::SENSOR_MULTILEVEL)
        || command_classes.contains(&CommandClassId::BATTERY)
        || command_classes.contains(&COMMAND_CLASS_METER)
        || command_classes.contains(&COMMAND_CLASS_NOTIFICATION)
    {
        EntityKind::Sensor
    } else {
        EntityKind::Unknown
    }
}

fn ensure_entity_capability(
    runtime: &mut SmartHomeRuntime,
    entity_id: &EntityId,
    state_delta: &StateDelta,
) -> Result<(), ZWaveIntegrationError> {
    let mut entity = runtime
        .registry()
        .entity(entity_id)
        .cloned()
        .ok_or_else(|| ZWaveIntegrationError::UnknownEntity(entity_id.clone()))?;
    if entity
        .capabilities
        .iter()
        .any(|capability| capability.capability_id == state_delta.capability_id)
    {
        return Ok(());
    }
    let capability = smart_home_core::canonical_capability_catalog()
        .into_iter()
        .find(|capability| capability.capability_id == state_delta.capability_id)
        .unwrap_or_else(|| {
            Capability::new(
                state_delta.capability_id.clone(),
                smart_home_core::CapabilityMode::Observe,
                value_kind(&state_delta.value),
            )
        });
    entity.capabilities.push(capability);
    entity.capabilities.sort_by(|left, right| {
        left.capability_id
            .as_str()
            .cmp(right.capability_id.as_str())
    });
    runtime.upsert_entity(entity)?;
    Ok(())
}

fn value_kind(value: &Value) -> smart_home_core::ValueKind {
    match value {
        Value::Null => smart_home_core::ValueKind::Null,
        Value::Bool(_) => smart_home_core::ValueKind::Boolean,
        Value::Integer(_) => smart_home_core::ValueKind::Integer,
        Value::Number(_) => smart_home_core::ValueKind::Number,
        Value::Percentage(_) => smart_home_core::ValueKind::Percentage,
        Value::Text(_) => smart_home_core::ValueKind::Text,
        Value::Object(_) => smart_home_core::ValueKind::Object,
        Value::Array(_) => smart_home_core::ValueKind::Array,
    }
}

fn command_for_request(
    binding: &ZWaveNodeBinding,
    request: &RuntimeCommandToolRequest,
) -> Result<ZWaveCommand, ZWaveIntegrationError> {
    match request.command_type {
        CommandType::TurnOn | CommandType::TurnOff => {
            let on = request.command_type == CommandType::TurnOn;
            if binding
                .command_classes
                .contains(&CommandClassId::SWITCH_BINARY)
            {
                Ok(binary_switch_set(on))
            } else if binding.command_classes.contains(&CommandClassId::BASIC) {
                Ok(ZWaveCommand::new(
                    CommandClassId::BASIC,
                    BASIC_SET,
                    vec![zwave_bool(on)],
                ))
            } else {
                Err(unsupported_command(request))
            }
        }
        CommandType::SetBrightness
            if binding
                .command_classes
                .contains(&CommandClassId::SWITCH_MULTILEVEL) =>
        {
            let Value::Percentage(percentage) = &request.arguments else {
                return Err(ZWaveIntegrationError::InvalidCommandArguments {
                    command_type: request.command_type,
                    expected: "percentage",
                });
            };
            Ok(multilevel_switch_set(*percentage))
        }
        CommandType::SetLock if binding.command_classes.contains(&CommandClassId::DOOR_LOCK) => {
            let secured = match &request.arguments {
                Value::Text(state) if state.eq_ignore_ascii_case("locked") => true,
                Value::Text(state) if state.eq_ignore_ascii_case("unlocked") => false,
                Value::Bool(secured) => *secured,
                _ => {
                    return Err(ZWaveIntegrationError::InvalidCommandArguments {
                        command_type: request.command_type,
                        expected: "`locked`, `unlocked`, or boolean",
                    });
                }
            };
            Ok(door_lock_operation_set(secured))
        }
        _ => Err(unsupported_command(request)),
    }
}

fn unsupported_command(request: &RuntimeCommandToolRequest) -> ZWaveIntegrationError {
    ZWaveIntegrationError::UnsupportedCommand {
        entity_id: request.entity_id.clone(),
        command_type: request.command_type,
    }
}

fn home_id_string(home_id: HomeId) -> String {
    format!("{:08x}", home_id.0)
}

fn node_id_string(node_id: NodeId) -> String {
    match node_id {
        NodeId::Classic(value) => value.to_string(),
        NodeId::LongRange(value) => format!("lr-{value}"),
    }
}

#[derive(Debug)]
pub enum ZWaveIntegrationError {
    Validation(String),
    ControllerNotInstalled(BridgeId),
    UnknownNode(NodeId),
    UnknownEntity(EntityId),
    UnsupportedCommand {
        entity_id: EntityId,
        command_type: CommandType,
    },
    InvalidCommandArguments {
        command_type: CommandType,
        expected: &'static str,
    },
    UnexpectedSendDataResponse,
    UnknownCallbackId(u8),
    CallbackIdsExhausted,
    Runtime(RuntimeError),
    CommandClass(CommandClassError),
    SerialApi(SerialApiError),
}

impl fmt::Display for ZWaveIntegrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => write!(f, "invalid Z-Wave integration: {message}"),
            Self::ControllerNotInstalled(bridge_id) => {
                write!(f, "Z-Wave controller {bridge_id} is not installed")
            }
            Self::UnknownNode(node_id) => {
                write!(f, "unknown Z-Wave node {}", node_id_string(*node_id))
            }
            Self::UnknownEntity(entity_id) => write!(f, "unknown Z-Wave entity {entity_id}"),
            Self::UnsupportedCommand {
                entity_id,
                command_type,
            } => write!(
                f,
                "Z-Wave entity {entity_id} does not support {command_type:?}"
            ),
            Self::InvalidCommandArguments {
                command_type,
                expected,
            } => write!(f, "invalid {command_type:?} arguments; expected {expected}"),
            Self::UnexpectedSendDataResponse => {
                write!(f, "received a SendData response without a pending request")
            }
            Self::UnknownCallbackId(callback_id) => {
                write!(f, "unknown SendData callback id {callback_id}")
            }
            Self::CallbackIdsExhausted => write!(f, "all SendData callback ids are in use"),
            Self::Runtime(error) => error.fmt(f),
            Self::CommandClass(error) => error.fmt(f),
            Self::SerialApi(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for ZWaveIntegrationError {}

impl From<RuntimeError> for ZWaveIntegrationError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

impl From<CommandClassError> for ZWaveIntegrationError {
    fn from(error: CommandClassError) -> Self {
        Self::CommandClass(error)
    }
}

impl From<SerialApiError> for ZWaveIntegrationError {
    fn from(error: SerialApiError) -> Self {
        Self::SerialApi(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smart_home_core::{
        CapabilityGrant, CapabilityGrantId, PrivilegeTier, StateConfidence, StateSource,
    };
    use zwave_command_classes::{
        encode_value_report, BatteryLevel, DoorLockMode, ZWaveValueReport,
    };

    fn integration() -> ZWaveRuntimeIntegration {
        ZWaveRuntimeIntegration::new(
            ZWaveControllerConfig::new(
                BridgeId::trusted("zwave-controller-1"),
                HomeId(0xdead_beef),
                NodeId::classic(1).unwrap(),
                "/dev/ttyUSB0",
            )
            .with_identity("Zooz", "ZST39")
            .with_firmware_version("1.2.3"),
        )
        .unwrap()
    }

    fn install_light(
        integration: &mut ZWaveRuntimeIntegration,
        runtime: &mut SmartHomeRuntime,
    ) -> InstalledZWaveNode {
        integration.install_controller(runtime, 1_000).unwrap();
        integration
            .install_node(
                runtime,
                ZWaveNodeInterview::new(
                    NodeId::classic(5).unwrap(),
                    "Kitchen Dimmer",
                    [
                        CommandClassId::SWITCH_BINARY,
                        CommandClassId::SWITCH_MULTILEVEL,
                        CommandClassId::BATTERY,
                    ],
                )
                .with_identity("Inovelli", "LZW31-SN")
                .in_room("kitchen"),
            )
            .unwrap()
    }

    fn grant(runtime: &mut SmartHomeRuntime, principal: &AgentId) {
        runtime
            .registry_mut()
            .upsert_capability_grant(CapabilityGrant::for_all_smart_home(
                CapabilityGrantId::trusted("grant-zwave-test"),
                principal.clone(),
                PrivilegeTier::HighRisk,
                "test",
                0,
            ));
    }

    fn application_report(node_id: u8, report: ZWaveValueReport) -> SerialMessage {
        let command = encode_value_report(&report).unwrap().encode().unwrap();
        let mut payload = vec![0x01, node_id, command.len() as u8];
        payload.extend(command);
        SerialMessage {
            kind: SerialMessageKind::Request,
            function_id: FunctionId::APPLICATION_COMMAND_HANDLER,
            callback_id: None,
            payload,
        }
    }

    #[test]
    fn installs_controller_and_interviewed_node_in_normalized_runtime() {
        let mut integration = integration();
        let mut runtime = SmartHomeRuntime::new();
        let installed = install_light(&mut integration, &mut runtime);

        assert_eq!(runtime.topology_summary().bridges, 1);
        assert_eq!(runtime.topology_summary().devices, 1);
        assert_eq!(runtime.topology_summary().entities, 1);
        assert_eq!(installed.entity_kind, EntityKind::Light);
        assert_eq!(
            installed.capability_ids,
            vec![
                "light.brightness".to_string(),
                "light.on_off".to_string(),
                "sensor.battery".to_string(),
            ]
        );
        let bridge = runtime
            .registry()
            .bridge(&BridgeId::trusted("zwave-controller-1"))
            .unwrap();
        assert_eq!(bridge.transport, BridgeTransport::Serial);
        assert_eq!(bridge.address.as_deref(), Some("/dev/ttyUSB0"));
        assert_eq!(bridge.health, Health::Online);
    }

    #[test]
    fn application_reports_update_state_and_add_specific_sensor_capabilities() {
        let mut integration = integration();
        let mut runtime = SmartHomeRuntime::new();
        integration.install_controller(&mut runtime, 1_000).unwrap();
        let installed = integration
            .install_node(
                &mut runtime,
                ZWaveNodeInterview::new(
                    NodeId::classic(9).unwrap(),
                    "Hall Sensor",
                    [CommandClassId::SENSOR_MULTILEVEL, CommandClassId::BATTERY],
                ),
            )
            .unwrap();
        let report = application_report(
            9,
            ZWaveValueReport::MultilevelSensor {
                sensor_type: 0x01,
                scale: 0,
                precision: 1,
                raw_value: 217,
            },
        );

        let event = integration
            .ingest_application_command(&mut runtime, &report, 2_000)
            .unwrap();

        assert_eq!(
            event.state_delta,
            Some(StateDelta {
                capability_id: smart_home_core::CapabilityId::trusted("sensor.temperature"),
                value: Value::Number(21.7),
            })
        );
        let entity = runtime.registry().entity(&installed.entity_id).unwrap();
        assert!(entity
            .capabilities
            .iter()
            .any(|capability| { capability.capability_id.as_str() == "sensor.temperature" }));
        let state = runtime.registry().state(&installed.entity_id).unwrap();
        assert_eq!(state.source, StateSource::EventStream);
        assert_eq!(state.confidence, StateConfidence::Confirmed);
    }

    #[test]
    fn authorized_command_builds_send_data_and_tracks_successful_callback() {
        let mut integration = integration();
        let mut runtime = SmartHomeRuntime::new();
        let installed = install_light(&mut integration, &mut runtime);
        let principal = AgentId::trusted("agent:zwave-test");
        grant(&mut runtime, &principal);

        let dispatch = integration
            .dispatch_command(
                &mut runtime,
                principal,
                RuntimeCommandToolRequest::new(
                    installed.entity_id,
                    CommandType::SetBrightness,
                    Value::Percentage(42),
                )
                .with_idempotency_key("kitchen-42")
                .with_timeout_ms(2_000),
                3_000,
            )
            .unwrap();

        assert_eq!(dispatch.command, multilevel_switch_set(42));
        assert_eq!(
            dispatch.command_result.status,
            smart_home_core::CommandStatus::Accepted
        );
        assert_eq!(dispatch.serial_message.function_id, FunctionId::SEND_DATA);
        assert_eq!(
            dispatch.serial_message.payload,
            vec![5, 3, 0x26, 0x01, 42, 0x25, dispatch.callback_id]
        );
        assert_eq!(integration.pending_dispatch_count(), 1);

        let malformed_response = SerialMessage {
            kind: SerialMessageKind::Response,
            function_id: FunctionId::SEND_DATA,
            callback_id: None,
            payload: vec![],
        };
        assert!(matches!(
            integration.handle_serial_message(&mut runtime, &malformed_response, 3_005),
            Err(ZWaveIntegrationError::SerialApi(_))
        ));

        let response = SerialMessage {
            kind: SerialMessageKind::Response,
            function_id: FunctionId::SEND_DATA,
            callback_id: None,
            payload: vec![1],
        };
        let response_outcome = integration
            .handle_serial_message(&mut runtime, &response, 3_010)
            .unwrap();
        assert!(matches!(
            response_outcome,
            ZWaveSerialOutcome::DispatchState(ZWaveDispatchState {
                state: SendDataTransactionState::AwaitingCallback,
                ..
            })
        ));

        let callback = SerialMessage {
            kind: SerialMessageKind::Callback,
            function_id: FunctionId::SEND_DATA,
            callback_id: Some(dispatch.callback_id),
            payload: vec![dispatch.callback_id, 0],
        };
        let callback_outcome = integration
            .handle_serial_message(&mut runtime, &callback, 3_020)
            .unwrap();
        assert!(matches!(
            callback_outcome,
            ZWaveSerialOutcome::DispatchState(ZWaveDispatchState {
                state: SendDataTransactionState::Succeeded,
                ..
            })
        ));
        assert_eq!(integration.pending_dispatch_count(), 0);
    }

    #[test]
    fn unauthorized_commands_do_not_create_serial_dispatches() {
        let mut integration = integration();
        let mut runtime = SmartHomeRuntime::new();
        let installed = install_light(&mut integration, &mut runtime);

        let error = integration
            .dispatch_command(
                &mut runtime,
                AgentId::trusted("agent:unauthorized"),
                RuntimeCommandToolRequest::new(
                    installed.entity_id,
                    CommandType::TurnOn,
                    Value::Null,
                ),
                4_000,
            )
            .unwrap_err();

        assert!(matches!(
            error,
            ZWaveIntegrationError::Runtime(RuntimeError::UnauthorizedTool { .. })
        ));
        assert_eq!(integration.pending_dispatch_count(), 0);
    }

    #[test]
    fn lock_commands_and_battery_reports_share_the_runtime_boundary() {
        let mut integration = integration();
        let mut runtime = SmartHomeRuntime::new();
        integration.install_controller(&mut runtime, 1_000).unwrap();
        let installed = integration
            .install_node(
                &mut runtime,
                ZWaveNodeInterview::new(
                    NodeId::classic(12).unwrap(),
                    "Front Door",
                    [CommandClassId::DOOR_LOCK, CommandClassId::BATTERY],
                ),
            )
            .unwrap();
        let principal = AgentId::trusted("agent:lock-test");
        grant(&mut runtime, &principal);

        let dispatch = integration
            .dispatch_command(
                &mut runtime,
                principal,
                RuntimeCommandToolRequest::new(
                    installed.entity_id.clone(),
                    CommandType::SetLock,
                    Value::Text("locked".to_string()),
                ),
                5_000,
            )
            .unwrap();
        assert_eq!(dispatch.command, door_lock_operation_set(true));

        let battery = application_report(
            12,
            ZWaveValueReport::Battery {
                level: BatteryLevel::Percentage(87),
            },
        );
        integration
            .ingest_application_command(&mut runtime, &battery, 5_100)
            .unwrap();
        assert_eq!(
            runtime
                .registry()
                .state(&installed.entity_id)
                .unwrap()
                .value,
            Value::Object(vec![("sensor.battery".to_string(), Value::Percentage(87))])
        );

        let lock = application_report(
            12,
            ZWaveValueReport::DoorLock {
                mode: DoorLockMode::Secured,
            },
        );
        integration
            .ingest_application_command(&mut runtime, &lock, 5_200)
            .unwrap();
        assert_eq!(
            runtime
                .registry()
                .state(&installed.entity_id)
                .unwrap()
                .value,
            Value::Object(vec![(
                "lock.state".to_string(),
                Value::Text("locked".to_string())
            )])
        );
    }

    #[test]
    fn dispatch_timeout_is_reported_once() {
        let mut integration = integration();
        let mut runtime = SmartHomeRuntime::new();
        let installed = install_light(&mut integration, &mut runtime);
        let principal = AgentId::trusted("agent:timeout-test");
        grant(&mut runtime, &principal);
        let dispatch = integration
            .dispatch_command(
                &mut runtime,
                principal,
                RuntimeCommandToolRequest::new(
                    installed.entity_id,
                    CommandType::TurnOff,
                    Value::Null,
                )
                .with_timeout_ms(100),
                6_000,
            )
            .unwrap();

        assert!(integration.expire_dispatches(6_099).is_empty());
        let expired = integration.expire_dispatches(6_100);
        assert_eq!(
            expired,
            vec![ZWaveDispatchState {
                callback_id: dispatch.callback_id,
                command_id: dispatch.command_result.command_id,
                state: SendDataTransactionState::TimedOut,
            }]
        );
        assert!(integration.expire_dispatches(6_101).is_empty());
    }

    #[test]
    fn timed_out_callback_ids_are_quarantined_from_reuse() {
        let mut integration = integration();
        let mut runtime = SmartHomeRuntime::new();
        let installed = install_light(&mut integration, &mut runtime);
        let principal = AgentId::trusted("agent:callback-quarantine-test");
        grant(&mut runtime, &principal);
        let timed_out = integration
            .dispatch_command(
                &mut runtime,
                principal.clone(),
                RuntimeCommandToolRequest::new(
                    installed.entity_id.clone(),
                    CommandType::TurnOff,
                    Value::Null,
                )
                .with_timeout_ms(100),
                6_000,
            )
            .unwrap();
        assert_eq!(integration.expire_dispatches(6_100).len(), 1);

        integration.next_callback_id = timed_out.callback_id;
        let next = integration
            .dispatch_command(
                &mut runtime,
                principal,
                RuntimeCommandToolRequest::new(
                    installed.entity_id,
                    CommandType::TurnOn,
                    Value::Null,
                ),
                6_200,
            )
            .unwrap();

        assert_ne!(next.callback_id, timed_out.callback_id);
    }

    #[test]
    fn basic_nodes_use_basic_set_fallback() {
        let mut integration = integration();
        let mut runtime = SmartHomeRuntime::new();
        integration.install_controller(&mut runtime, 1_000).unwrap();
        let installed = integration
            .install_node(
                &mut runtime,
                ZWaveNodeInterview::new(
                    NodeId::classic(20).unwrap(),
                    "Legacy Switch",
                    [CommandClassId::BASIC],
                ),
            )
            .unwrap();
        let principal = AgentId::trusted("agent:basic-test");
        grant(&mut runtime, &principal);

        let dispatch = integration
            .dispatch_command(
                &mut runtime,
                principal,
                RuntimeCommandToolRequest::new(
                    installed.entity_id,
                    CommandType::TurnOn,
                    Value::Null,
                ),
                7_000,
            )
            .unwrap();

        assert_eq!(
            dispatch.command,
            ZWaveCommand::new(CommandClassId::BASIC, BASIC_SET, vec![0xff])
        );
    }
}
