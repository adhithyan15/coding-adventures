//! Z-Wave command class value and D23 mapping primitives.
//!
//! This crate owns command-class payload semantics without controller I/O,
//! inclusion state, or security. It turns command-class reports into typed
//! values and the first normalized smart-home capability/state deltas.

#![forbid(unsafe_code)]

use smart_home_core::{Capability, CapabilityId, CapabilityMode, StateDelta, Value, ValueKind};
use std::collections::BTreeSet;
use std::fmt;
use zwave_core::CommandClassId;

pub const BASIC_SET: u8 = 0x01;
pub const BASIC_GET: u8 = 0x02;
pub const BASIC_REPORT: u8 = 0x03;
pub const SWITCH_BINARY_SET: u8 = 0x01;
pub const SWITCH_BINARY_GET: u8 = 0x02;
pub const SWITCH_BINARY_REPORT: u8 = 0x03;
pub const SWITCH_MULTILEVEL_SET: u8 = 0x01;
pub const SWITCH_MULTILEVEL_GET: u8 = 0x02;
pub const SWITCH_MULTILEVEL_REPORT: u8 = 0x03;
pub const SENSOR_BINARY_GET: u8 = 0x02;
pub const SENSOR_BINARY_REPORT: u8 = 0x03;
pub const SENSOR_MULTILEVEL_GET: u8 = 0x04;
pub const SENSOR_MULTILEVEL_REPORT: u8 = 0x05;
pub const DOOR_LOCK_OPERATION_SET: u8 = 0x01;
pub const DOOR_LOCK_OPERATION_GET: u8 = 0x02;
pub const DOOR_LOCK_OPERATION_REPORT: u8 = 0x03;
pub const BATTERY_GET: u8 = 0x02;
pub const BATTERY_REPORT: u8 = 0x03;
pub const BATTERY_LOW_WARNING: u8 = 0xff;
pub const METER_GET: u8 = 0x01;
pub const METER_REPORT: u8 = 0x02;
pub const NOTIFICATION_GET: u8 = 0x04;
pub const NOTIFICATION_REPORT: u8 = 0x05;
pub const COMMAND_CLASS_METER: CommandClassId = CommandClassId(0x32);
pub const COMMAND_CLASS_NOTIFICATION: CommandClassId = CommandClassId(0x71);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZWaveCommand {
    pub command_class: CommandClassId,
    pub command_id: u8,
    pub payload: Vec<u8>,
}

impl ZWaveCommand {
    pub fn new(command_class: CommandClassId, command_id: u8, payload: Vec<u8>) -> Self {
        Self {
            command_class,
            command_id,
            payload,
        }
    }

    pub fn summary(&self) -> ZWaveCommandSummary {
        let class_len = encoded_command_class_len(self.command_class);
        ZWaveCommandSummary {
            command_class: self.command_class,
            command_id: self.command_id,
            command_kind: command_kind(self.command_class, self.command_id),
            payload_len: self.payload.len(),
            uses_extended_command_class: self.command_class.0 > u8::MAX as u16,
            can_encode: class_len.is_some(),
            encoded_len: class_len.map(|len| len + 1 + self.payload.len()),
        }
    }

    pub fn parse(bytes: &[u8]) -> Result<Self, CommandClassError> {
        if bytes.len() < 2 {
            return Err(CommandClassError::Truncated {
                needed: 2,
                remaining: bytes.len(),
            });
        }

        let (command_class, command_offset) = if bytes[0] >= 0xf1 {
            if bytes.len() < 3 {
                return Err(CommandClassError::Truncated {
                    needed: 3,
                    remaining: bytes.len(),
                });
            }
            (CommandClassId(u16::from_be_bytes([bytes[0], bytes[1]])), 2)
        } else {
            (CommandClassId(u16::from(bytes[0])), 1)
        };

        Ok(Self {
            command_class,
            command_id: bytes[command_offset],
            payload: bytes[command_offset + 1..].to_vec(),
        })
    }

    pub fn encode(&self) -> Result<Vec<u8>, CommandClassError> {
        let mut out = Vec::with_capacity(2 + self.payload.len());
        if self.command_class.0 <= u8::MAX as u16 {
            out.push(self.command_class.0 as u8);
        } else if self.command_class.0 < 0xf100 {
            return Err(CommandClassError::InvalidExtendedCommandClassId(
                self.command_class.0,
            ));
        } else {
            out.extend_from_slice(&self.command_class.0.to_be_bytes());
        }
        out.push(self.command_id);
        out.extend_from_slice(&self.payload);
        Ok(out)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZWaveCommandSummary {
    pub command_class: CommandClassId,
    pub command_id: u8,
    pub command_kind: ZWaveCommandKind,
    pub payload_len: usize,
    pub uses_extended_command_class: bool,
    pub can_encode: bool,
    pub encoded_len: Option<usize>,
}

impl ZWaveCommandSummary {
    pub fn has_payload(self) -> bool {
        self.payload_len > 0
    }

    pub fn is_payload_free(self) -> bool {
        self.payload_len == 0
    }

    pub fn is_get(self) -> bool {
        self.command_kind == ZWaveCommandKind::Get
    }

    pub fn is_set(self) -> bool {
        self.command_kind == ZWaveCommandKind::Set
    }

    pub fn is_report(self) -> bool {
        self.command_kind == ZWaveCommandKind::Report
    }

    pub fn is_request(self) -> bool {
        matches!(
            self.command_kind,
            ZWaveCommandKind::Get | ZWaveCommandKind::Set
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZWaveCommandBatchSummary {
    pub total_commands: usize,
    pub get_commands: usize,
    pub set_commands: usize,
    pub report_commands: usize,
    pub other_commands: usize,
    pub payload_free_commands: usize,
    pub commands_with_payload: usize,
    pub extended_command_classes: usize,
    pub encodable_commands: usize,
    pub unencodable_commands: usize,
    pub total_payload_bytes: usize,
    pub total_encoded_bytes: Option<usize>,
    pub unique_command_classes: usize,
}

impl ZWaveCommandBatchSummary {
    pub fn from_commands<'a>(commands: impl IntoIterator<Item = &'a ZWaveCommand>) -> Self {
        Self::from_summaries(commands.into_iter().map(ZWaveCommand::summary))
    }

    pub fn from_summaries(summaries: impl IntoIterator<Item = ZWaveCommandSummary>) -> Self {
        let mut batch = Self {
            total_commands: 0,
            get_commands: 0,
            set_commands: 0,
            report_commands: 0,
            other_commands: 0,
            payload_free_commands: 0,
            commands_with_payload: 0,
            extended_command_classes: 0,
            encodable_commands: 0,
            unencodable_commands: 0,
            total_payload_bytes: 0,
            total_encoded_bytes: Some(0),
            unique_command_classes: 0,
        };
        let mut command_classes = BTreeSet::new();

        for summary in summaries {
            batch.record_summary(summary);
            command_classes.insert(summary.command_class);
        }

        batch.unique_command_classes = command_classes.len();
        batch
    }

    pub fn record_summary(&mut self, summary: ZWaveCommandSummary) {
        self.total_commands += 1;
        match summary.command_kind {
            ZWaveCommandKind::Get => self.get_commands += 1,
            ZWaveCommandKind::Set => self.set_commands += 1,
            ZWaveCommandKind::Report => self.report_commands += 1,
            ZWaveCommandKind::Other => self.other_commands += 1,
        }
        if summary.has_payload() {
            self.commands_with_payload += 1;
        } else {
            self.payload_free_commands += 1;
        }
        if summary.uses_extended_command_class {
            self.extended_command_classes += 1;
        }
        if summary.can_encode {
            self.encodable_commands += 1;
        } else {
            self.unencodable_commands += 1;
        }
        self.total_payload_bytes += summary.payload_len;
        self.total_encoded_bytes = match (self.total_encoded_bytes, summary.encoded_len) {
            (Some(total), Some(encoded_len)) => Some(total + encoded_len),
            _ => None,
        };
    }

    pub fn is_empty(self) -> bool {
        self.total_commands == 0
    }

    pub fn has_requests(self) -> bool {
        self.get_commands + self.set_commands > 0
    }

    pub fn has_reports(self) -> bool {
        self.report_commands > 0
    }

    pub fn has_payloads(self) -> bool {
        self.commands_with_payload > 0
    }

    pub fn has_unencodable_commands(self) -> bool {
        self.unencodable_commands > 0
    }
}

pub fn summarize_zwave_commands<'a>(
    commands: impl IntoIterator<Item = &'a ZWaveCommand>,
) -> ZWaveCommandBatchSummary {
    ZWaveCommandBatchSummary::from_commands(commands)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZWaveCommandKind {
    Get,
    Set,
    Report,
    Other,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CommandClassInterviewDescriptor {
    pub command_class: CommandClassId,
    pub commands: Vec<ZWaveCommand>,
    pub capabilities: Vec<Capability>,
}

impl CommandClassInterviewDescriptor {
    pub fn can_query_state(&self) -> bool {
        !self.commands.is_empty()
    }

    pub fn projects_capabilities(&self) -> bool {
        !self.capabilities.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ZWaveValueReport {
    Basic {
        value: u8,
    },
    BinarySwitch {
        current_value: bool,
    },
    MultilevelSwitch {
        current_level: u8,
        target_level: Option<u8>,
        duration: Option<u8>,
    },
    BinarySensor {
        detected: bool,
        sensor_type: Option<u8>,
    },
    MultilevelSensor {
        sensor_type: u8,
        scale: u8,
        precision: u8,
        raw_value: i32,
    },
    DoorLock {
        mode: DoorLockMode,
    },
    Battery {
        level: BatteryLevel,
    },
    Meter {
        meter_type: u8,
        scale: u8,
        precision: u8,
        raw_value: i32,
    },
    Notification(NotificationReport),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoorLockMode {
    Unsecured,
    Secured,
    Unknown(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatteryLevel {
    Percentage(u8),
    LowWarning,
    Reserved(u8),
}

impl BatteryLevel {
    pub fn parse(value: u8) -> Self {
        match value {
            0..=100 => Self::Percentage(value),
            BATTERY_LOW_WARNING => Self::LowWarning,
            reserved => Self::Reserved(reserved),
        }
    }

    pub fn normalized_percentage(self) -> u8 {
        match self {
            Self::Percentage(value) => value,
            Self::LowWarning => 0,
            Self::Reserved(value) => value.min(100),
        }
    }

    pub fn is_low_warning(self) -> bool {
        matches!(self, Self::LowWarning)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationType {
    Smoke,
    AccessControl,
    HomeSecurity,
    Water,
    Heat,
    CarbonMonoxide,
    CarbonDioxide,
    Unknown(u8),
}

impl NotificationType {
    pub fn parse(value: u8) -> Self {
        match value {
            0x01 => Self::Smoke,
            0x06 => Self::AccessControl,
            0x07 => Self::HomeSecurity,
            0x05 => Self::Water,
            0x04 => Self::Heat,
            0x02 => Self::CarbonMonoxide,
            0x03 => Self::CarbonDioxide,
            other => Self::Unknown(other),
        }
    }

    pub fn as_byte(self) -> u8 {
        match self {
            Self::Smoke => 0x01,
            Self::CarbonMonoxide => 0x02,
            Self::CarbonDioxide => 0x03,
            Self::Heat => 0x04,
            Self::Water => 0x05,
            Self::AccessControl => 0x06,
            Self::HomeSecurity => 0x07,
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationReport {
    pub v1_alarm_type: u8,
    pub v1_alarm_level: u8,
    pub notification_status: u8,
    pub notification_type: NotificationType,
    pub event: u8,
    pub event_parameters: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationState {
    Idle,
    MotionDetected,
    DoorOpen,
    DoorClosed,
    Locked,
    Unlocked,
    SmokeDetected,
    WaterLeakDetected,
    Alarm,
    Unknown {
        notification_type: NotificationType,
        event: u8,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandClassProjectionSummary {
    pub command_class_entries: usize,
    pub unique_command_classes: usize,
    pub projected_command_classes: usize,
    pub commandable_command_classes: usize,
    pub sensor_command_classes: usize,
    pub projected_capabilities: usize,
    pub observe_only_capabilities: usize,
    pub commandable_capabilities: usize,
}

impl CommandClassProjectionSummary {
    pub fn from_command_classes<I>(command_classes: I) -> Self
    where
        I: IntoIterator<Item = CommandClassId>,
    {
        let mut summary = Self {
            command_class_entries: 0,
            unique_command_classes: 0,
            projected_command_classes: 0,
            commandable_command_classes: 0,
            sensor_command_classes: 0,
            projected_capabilities: 0,
            observe_only_capabilities: 0,
            commandable_capabilities: 0,
        };

        let mut unique_command_classes = BTreeSet::new();
        for command_class in command_classes {
            summary.command_class_entries += 1;
            unique_command_classes.insert(command_class);
        }
        summary.unique_command_classes = unique_command_classes.len();

        for command_class in unique_command_classes {
            let capabilities = capabilities_for_command_class(command_class);
            if capabilities.is_empty() {
                continue;
            }

            summary.projected_command_classes += 1;
            if capabilities.iter().any(is_commandable_capability) {
                summary.commandable_command_classes += 1;
            }
            if capabilities.iter().any(is_sensor_capability) {
                summary.sensor_command_classes += 1;
            }

            for capability in capabilities {
                summary.projected_capabilities += 1;
                match capability.mode {
                    CapabilityMode::Observe => summary.observe_only_capabilities += 1,
                    CapabilityMode::Command | CapabilityMode::ObserveAndCommand => {
                        summary.commandable_capabilities += 1;
                    }
                }
            }
        }

        summary
    }

    pub fn has_projected_capabilities(self) -> bool {
        self.projected_capabilities > 0
    }

    pub fn has_command_surface(self) -> bool {
        self.commandable_capabilities > 0
    }

    pub fn has_sensor_surface(self) -> bool {
        self.sensor_command_classes > 0
    }

    pub fn has_observe_only_surface(self) -> bool {
        self.observe_only_capabilities > 0
    }

    pub fn readiness(self) -> CommandClassProjectionReadinessSummary {
        CommandClassProjectionReadinessSummary::from_summary(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandClassProjectionReadinessSummary {
    pub projection_summary: CommandClassProjectionSummary,
    pub required_projection_check_count: usize,
    pub passed_projection_check_count: usize,
    pub missing_projection_check_count: usize,
    pub command_classes_present: bool,
    pub capability_projection_ready: bool,
    pub command_surface_ready: bool,
    pub sensor_surface_ready: bool,
    pub observe_only_surface_ready: bool,
    pub projection_ready: bool,
}

impl CommandClassProjectionReadinessSummary {
    pub fn from_command_classes<I>(command_classes: I) -> Self
    where
        I: IntoIterator<Item = CommandClassId>,
    {
        Self::from_summary(CommandClassProjectionSummary::from_command_classes(
            command_classes,
        ))
    }

    pub fn from_summary(projection_summary: CommandClassProjectionSummary) -> Self {
        let command_classes_present = projection_summary.unique_command_classes > 0;
        let capability_projection_ready = projection_summary.has_projected_capabilities();
        let command_surface_ready = projection_summary.has_command_surface();
        let sensor_surface_ready = projection_summary.has_sensor_surface();
        let observe_only_surface_ready = projection_summary.has_observe_only_surface();
        let checks = [
            command_classes_present,
            capability_projection_ready,
            command_surface_ready,
            sensor_surface_ready,
            observe_only_surface_ready,
        ];
        let passed_projection_check_count = checks.iter().filter(|ready| **ready).count();
        let required_projection_check_count = checks.len();
        let missing_projection_check_count =
            required_projection_check_count - passed_projection_check_count;
        let projection_ready = missing_projection_check_count == 0;

        Self {
            projection_summary,
            required_projection_check_count,
            passed_projection_check_count,
            missing_projection_check_count,
            command_classes_present,
            capability_projection_ready,
            command_surface_ready,
            sensor_surface_ready,
            observe_only_surface_ready,
            projection_ready,
        }
    }

    pub fn is_projection_ready(self) -> bool {
        self.projection_ready
    }

    pub fn has_missing_projection_checks(self) -> bool {
        self.missing_projection_check_count > 0
    }

    pub fn needs_command_class_inventory(self) -> bool {
        !self.command_classes_present
    }

    pub fn needs_capability_projection(self) -> bool {
        !self.capability_projection_ready
    }

    pub fn needs_command_surface(self) -> bool {
        !self.command_surface_ready
    }

    pub fn needs_sensor_surface(self) -> bool {
        !self.sensor_surface_ready
    }

    pub fn needs_observe_only_surface(self) -> bool {
        !self.observe_only_surface_ready
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandClassProjectionSignoffSummary {
    pub readiness_summary: CommandClassProjectionReadinessSummary,
    pub required_signoff_check_count: usize,
    pub passed_signoff_check_count: usize,
    pub missing_signoff_check_count: usize,
    pub projection_ready: bool,
    pub command_classes_present: bool,
    pub capability_projection_ready: bool,
    pub command_surface_ready: bool,
    pub sensor_surface_ready: bool,
    pub observe_only_surface_ready: bool,
    pub signoff_ready: bool,
}

impl CommandClassProjectionSignoffSummary {
    pub fn from_command_classes<I>(command_classes: I) -> Self
    where
        I: IntoIterator<Item = CommandClassId>,
    {
        Self::from_readiness_summary(
            CommandClassProjectionReadinessSummary::from_command_classes(command_classes),
        )
    }

    pub fn from_readiness_summary(
        readiness_summary: CommandClassProjectionReadinessSummary,
    ) -> Self {
        let projection_ready = readiness_summary.is_projection_ready();
        let command_classes_present = !readiness_summary.needs_command_class_inventory();
        let capability_projection_ready = !readiness_summary.needs_capability_projection();
        let command_surface_ready = !readiness_summary.needs_command_surface();
        let sensor_surface_ready = !readiness_summary.needs_sensor_surface();
        let observe_only_surface_ready = !readiness_summary.needs_observe_only_surface();
        let checks = [
            projection_ready,
            command_classes_present,
            capability_projection_ready,
            command_surface_ready,
            sensor_surface_ready,
            observe_only_surface_ready,
        ];
        let passed_signoff_check_count = checks.iter().filter(|ready| **ready).count();
        let required_signoff_check_count = checks.len();
        let missing_signoff_check_count = required_signoff_check_count - passed_signoff_check_count;
        let signoff_ready = missing_signoff_check_count == 0;

        Self {
            readiness_summary,
            required_signoff_check_count,
            passed_signoff_check_count,
            missing_signoff_check_count,
            projection_ready,
            command_classes_present,
            capability_projection_ready,
            command_surface_ready,
            sensor_surface_ready,
            observe_only_surface_ready,
            signoff_ready,
        }
    }

    pub fn is_signoff_ready(self) -> bool {
        self.signoff_ready
    }

    pub fn has_missing_signoff_checks(self) -> bool {
        self.missing_signoff_check_count > 0
    }

    pub fn needs_projection_readiness(self) -> bool {
        !self.projection_ready
    }

    pub fn needs_command_class_inventory(self) -> bool {
        !self.command_classes_present
    }

    pub fn needs_capability_projection(self) -> bool {
        !self.capability_projection_ready
    }

    pub fn needs_command_surface(self) -> bool {
        !self.command_surface_ready
    }

    pub fn needs_sensor_surface(self) -> bool {
        !self.sensor_surface_ready
    }

    pub fn needs_observe_only_surface(self) -> bool {
        !self.observe_only_surface_ready
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandClassProjectionClosureSummary {
    pub signoff_summary: CommandClassProjectionSignoffSummary,
    pub required_closure_check_count: usize,
    pub passed_closure_check_count: usize,
    pub missing_closure_check_count: usize,
    pub signoff_ready: bool,
    pub projection_ready: bool,
    pub command_classes_present: bool,
    pub capability_projection_ready: bool,
    pub command_surface_ready: bool,
    pub sensor_surface_ready: bool,
    pub observe_only_surface_ready: bool,
    pub closure_ready: bool,
}

impl CommandClassProjectionClosureSummary {
    pub fn from_command_classes<I>(command_classes: I) -> Self
    where
        I: IntoIterator<Item = CommandClassId>,
    {
        Self::from_signoff_summary(CommandClassProjectionSignoffSummary::from_command_classes(
            command_classes,
        ))
    }

    pub fn from_signoff_summary(signoff_summary: CommandClassProjectionSignoffSummary) -> Self {
        let signoff_ready = signoff_summary.is_signoff_ready();
        let projection_ready = !signoff_summary.needs_projection_readiness();
        let command_classes_present = !signoff_summary.needs_command_class_inventory();
        let capability_projection_ready = !signoff_summary.needs_capability_projection();
        let command_surface_ready = !signoff_summary.needs_command_surface();
        let sensor_surface_ready = !signoff_summary.needs_sensor_surface();
        let observe_only_surface_ready = !signoff_summary.needs_observe_only_surface();
        let checks = [
            signoff_ready,
            projection_ready,
            command_classes_present,
            capability_projection_ready,
            command_surface_ready,
            sensor_surface_ready,
            observe_only_surface_ready,
        ];
        let passed_closure_check_count = checks.iter().filter(|ready| **ready).count();
        let required_closure_check_count = checks.len();
        let missing_closure_check_count = required_closure_check_count - passed_closure_check_count;
        let closure_ready = missing_closure_check_count == 0;

        Self {
            signoff_summary,
            required_closure_check_count,
            passed_closure_check_count,
            missing_closure_check_count,
            signoff_ready,
            projection_ready,
            command_classes_present,
            capability_projection_ready,
            command_surface_ready,
            sensor_surface_ready,
            observe_only_surface_ready,
            closure_ready,
        }
    }

    pub fn is_closure_ready(self) -> bool {
        self.closure_ready
    }

    pub fn has_missing_closure_checks(self) -> bool {
        self.missing_closure_check_count > 0
    }

    pub fn needs_projection_signoff(self) -> bool {
        !self.signoff_ready
    }

    pub fn needs_projection_readiness(self) -> bool {
        !self.projection_ready
    }

    pub fn needs_command_class_inventory(self) -> bool {
        !self.command_classes_present
    }

    pub fn needs_capability_projection(self) -> bool {
        !self.capability_projection_ready
    }

    pub fn needs_command_surface(self) -> bool {
        !self.command_surface_ready
    }

    pub fn needs_sensor_surface(self) -> bool {
        !self.sensor_surface_ready
    }

    pub fn needs_observe_only_surface(self) -> bool {
        !self.observe_only_surface_ready
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandClassError {
    Truncated {
        needed: usize,
        remaining: usize,
    },
    UnsupportedReport {
        command_class: CommandClassId,
        command_id: u8,
    },
    InvalidExtendedCommandClassId(u16),
    InvalidReportValue(&'static str),
    InvalidSensorValueSize(u8),
    InvalidMeterValueSize(u8),
}

impl fmt::Display for CommandClassError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated { needed, remaining } => write!(
                f,
                "truncated Z-Wave command class payload: needed {needed} bytes, had {remaining}"
            ),
            Self::UnsupportedReport {
                command_class,
                command_id,
            } => write!(
                f,
                "unsupported Z-Wave report command class 0x{:02x} command 0x{command_id:02x}",
                command_class.0
            ),
            Self::InvalidExtendedCommandClassId(id) => {
                write!(f, "invalid extended Z-Wave command class id 0x{id:04x}")
            }
            Self::InvalidReportValue(reason) => write!(f, "invalid Z-Wave report value: {reason}"),
            Self::InvalidSensorValueSize(size) => {
                write!(f, "invalid Z-Wave multilevel sensor value size {size}")
            }
            Self::InvalidMeterValueSize(size) => {
                write!(f, "invalid Z-Wave meter value size {size}")
            }
        }
    }
}

impl std::error::Error for CommandClassError {}

pub fn basic_get() -> ZWaveCommand {
    ZWaveCommand::new(CommandClassId::BASIC, BASIC_GET, Vec::new())
}

pub fn binary_switch_get() -> ZWaveCommand {
    ZWaveCommand::new(CommandClassId::SWITCH_BINARY, SWITCH_BINARY_GET, Vec::new())
}

pub fn binary_switch_set(on: bool) -> ZWaveCommand {
    ZWaveCommand::new(
        CommandClassId::SWITCH_BINARY,
        SWITCH_BINARY_SET,
        vec![zwave_bool(on)],
    )
}

pub fn multilevel_switch_get() -> ZWaveCommand {
    ZWaveCommand::new(
        CommandClassId::SWITCH_MULTILEVEL,
        SWITCH_MULTILEVEL_GET,
        Vec::new(),
    )
}

pub fn multilevel_switch_set(percent: u8) -> ZWaveCommand {
    ZWaveCommand::new(
        CommandClassId::SWITCH_MULTILEVEL,
        SWITCH_MULTILEVEL_SET,
        vec![percentage_to_zwave_level(percent)],
    )
}

pub fn door_lock_operation_get() -> ZWaveCommand {
    ZWaveCommand::new(
        CommandClassId::DOOR_LOCK,
        DOOR_LOCK_OPERATION_GET,
        Vec::new(),
    )
}

pub fn door_lock_operation_set(secured: bool) -> ZWaveCommand {
    ZWaveCommand::new(
        CommandClassId::DOOR_LOCK,
        DOOR_LOCK_OPERATION_SET,
        vec![if secured { 0xff } else { 0x00 }],
    )
}

pub fn binary_sensor_get() -> ZWaveCommand {
    ZWaveCommand::new(CommandClassId::SENSOR_BINARY, SENSOR_BINARY_GET, Vec::new())
}

pub fn multilevel_sensor_get() -> ZWaveCommand {
    ZWaveCommand::new(
        CommandClassId::SENSOR_MULTILEVEL,
        SENSOR_MULTILEVEL_GET,
        Vec::new(),
    )
}

pub fn battery_get() -> ZWaveCommand {
    ZWaveCommand::new(CommandClassId::BATTERY, BATTERY_GET, Vec::new())
}

pub fn meter_get() -> ZWaveCommand {
    ZWaveCommand::new(COMMAND_CLASS_METER, METER_GET, Vec::new())
}

pub fn interview_commands_for_command_class(command_class: CommandClassId) -> Vec<ZWaveCommand> {
    match command_class {
        CommandClassId::BASIC => vec![basic_get()],
        CommandClassId::SWITCH_BINARY => vec![binary_switch_get()],
        CommandClassId::SWITCH_MULTILEVEL => vec![multilevel_switch_get()],
        CommandClassId::SENSOR_BINARY => vec![binary_sensor_get()],
        CommandClassId::SENSOR_MULTILEVEL => vec![multilevel_sensor_get()],
        CommandClassId::DOOR_LOCK => vec![door_lock_operation_get()],
        CommandClassId::BATTERY => vec![battery_get()],
        COMMAND_CLASS_METER => vec![meter_get()],
        _ => Vec::new(),
    }
}

pub fn interview_descriptor_for_command_class(
    command_class: CommandClassId,
) -> Option<CommandClassInterviewDescriptor> {
    let commands = interview_commands_for_command_class(command_class);
    let capabilities = capabilities_for_command_class(command_class);
    if commands.is_empty() && capabilities.is_empty() {
        return None;
    }
    Some(CommandClassInterviewDescriptor {
        command_class,
        commands,
        capabilities,
    })
}

pub fn interview_descriptors_for_command_classes(
    command_classes: impl IntoIterator<Item = CommandClassId>,
) -> Vec<CommandClassInterviewDescriptor> {
    command_classes
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter_map(interview_descriptor_for_command_class)
        .collect()
}

pub fn parse_value_report(command: &ZWaveCommand) -> Result<ZWaveValueReport, CommandClassError> {
    match (command.command_class, command.command_id) {
        (CommandClassId::BASIC, BASIC_REPORT) => {
            require_len(&command.payload, 1)?;
            Ok(ZWaveValueReport::Basic {
                value: command.payload[0],
            })
        }
        (CommandClassId::SWITCH_BINARY, SWITCH_BINARY_REPORT) => {
            require_len(&command.payload, 1)?;
            Ok(ZWaveValueReport::BinarySwitch {
                current_value: zwave_value_to_bool(command.payload[0]),
            })
        }
        (CommandClassId::SWITCH_MULTILEVEL, SWITCH_MULTILEVEL_REPORT) => {
            require_len(&command.payload, 1)?;
            Ok(ZWaveValueReport::MultilevelSwitch {
                current_level: command.payload[0],
                target_level: command.payload.get(1).copied(),
                duration: command.payload.get(2).copied(),
            })
        }
        (CommandClassId::SENSOR_BINARY, SENSOR_BINARY_REPORT) => {
            require_len(&command.payload, 1)?;
            Ok(ZWaveValueReport::BinarySensor {
                detected: zwave_value_to_bool(command.payload[0]),
                sensor_type: command.payload.get(1).copied(),
            })
        }
        (CommandClassId::SENSOR_MULTILEVEL, SENSOR_MULTILEVEL_REPORT) => {
            parse_multilevel_sensor_report(&command.payload)
        }
        (CommandClassId::DOOR_LOCK, DOOR_LOCK_OPERATION_REPORT) => {
            require_len(&command.payload, 1)?;
            Ok(ZWaveValueReport::DoorLock {
                mode: door_lock_mode(command.payload[0]),
            })
        }
        (CommandClassId::BATTERY, BATTERY_REPORT) => {
            require_len(&command.payload, 1)?;
            Ok(ZWaveValueReport::Battery {
                level: BatteryLevel::parse(command.payload[0]),
            })
        }
        (COMMAND_CLASS_METER, METER_REPORT) => parse_meter_report(&command.payload),
        (COMMAND_CLASS_NOTIFICATION, NOTIFICATION_REPORT) => Ok(ZWaveValueReport::Notification(
            parse_notification_report(&command.payload)?,
        )),
        _ => Err(CommandClassError::UnsupportedReport {
            command_class: command.command_class,
            command_id: command.command_id,
        }),
    }
}

pub fn encode_value_report(report: &ZWaveValueReport) -> Result<ZWaveCommand, CommandClassError> {
    match report {
        ZWaveValueReport::Basic { value } => Ok(ZWaveCommand::new(
            CommandClassId::BASIC,
            BASIC_REPORT,
            vec![*value],
        )),
        ZWaveValueReport::BinarySwitch { current_value } => Ok(ZWaveCommand::new(
            CommandClassId::SWITCH_BINARY,
            SWITCH_BINARY_REPORT,
            vec![zwave_bool(*current_value)],
        )),
        ZWaveValueReport::MultilevelSwitch {
            current_level,
            target_level,
            duration,
        } => {
            let mut payload = vec![*current_level];
            if let Some(target_level) = target_level {
                payload.push(*target_level);
            }
            if let Some(duration) = duration {
                if target_level.is_none() {
                    payload.push(*current_level);
                }
                payload.push(*duration);
            }
            Ok(ZWaveCommand::new(
                CommandClassId::SWITCH_MULTILEVEL,
                SWITCH_MULTILEVEL_REPORT,
                payload,
            ))
        }
        ZWaveValueReport::BinarySensor {
            detected,
            sensor_type,
        } => {
            let mut payload = vec![zwave_bool(*detected)];
            if let Some(sensor_type) = sensor_type {
                payload.push(*sensor_type);
            }
            Ok(ZWaveCommand::new(
                CommandClassId::SENSOR_BINARY,
                SENSOR_BINARY_REPORT,
                payload,
            ))
        }
        ZWaveValueReport::MultilevelSensor {
            sensor_type,
            scale,
            precision,
            raw_value,
        } => {
            let mut payload = vec![*sensor_type];
            encode_scaled_value_properties(*precision, *scale, *raw_value, &mut payload)?;
            Ok(ZWaveCommand::new(
                CommandClassId::SENSOR_MULTILEVEL,
                SENSOR_MULTILEVEL_REPORT,
                payload,
            ))
        }
        ZWaveValueReport::DoorLock { mode } => Ok(ZWaveCommand::new(
            CommandClassId::DOOR_LOCK,
            DOOR_LOCK_OPERATION_REPORT,
            vec![door_lock_mode_byte(*mode)],
        )),
        ZWaveValueReport::Battery { level } => Ok(ZWaveCommand::new(
            CommandClassId::BATTERY,
            BATTERY_REPORT,
            vec![battery_level_byte(*level)],
        )),
        ZWaveValueReport::Meter {
            meter_type,
            scale,
            precision,
            raw_value,
        } => {
            if *meter_type > 0b0001_1111 {
                return Err(CommandClassError::InvalidReportValue(
                    "meter type must fit in five bits",
                ));
            }
            let mut payload = vec![*meter_type];
            encode_scaled_value_properties(*precision, *scale, *raw_value, &mut payload)?;
            Ok(ZWaveCommand::new(
                COMMAND_CLASS_METER,
                METER_REPORT,
                payload,
            ))
        }
        ZWaveValueReport::Notification(report) => encode_notification_report(report),
    }
}

pub fn capabilities_for_command_class(command_class: CommandClassId) -> Vec<Capability> {
    match command_class {
        CommandClassId::SWITCH_BINARY => vec![Capability::light_on_off()],
        CommandClassId::SWITCH_MULTILEVEL => {
            vec![Capability::light_on_off(), Capability::light_brightness()]
        }
        CommandClassId::SENSOR_BINARY => vec![Capability::sensor_occupancy()],
        CommandClassId::SENSOR_MULTILEVEL => vec![Capability::new(
            CapabilityId::trusted("sensor.value"),
            CapabilityMode::Observe,
            ValueKind::Number,
        )],
        CommandClassId::DOOR_LOCK => vec![Capability::new(
            CapabilityId::trusted("lock.state"),
            CapabilityMode::ObserveAndCommand,
            ValueKind::Text,
        )],
        CommandClassId::BATTERY => vec![Capability::sensor_battery()],
        COMMAND_CLASS_METER => vec![
            Capability::new(
                CapabilityId::trusted("sensor.energy"),
                CapabilityMode::Observe,
                ValueKind::Number,
            )
            .with_unit("kWh"),
            Capability::new(
                CapabilityId::trusted("sensor.power"),
                CapabilityMode::Observe,
                ValueKind::Number,
            )
            .with_unit("W"),
        ],
        COMMAND_CLASS_NOTIFICATION => vec![
            Capability::sensor_occupancy(),
            Capability::new(
                CapabilityId::trusted("sensor.contact"),
                CapabilityMode::Observe,
                ValueKind::Boolean,
            ),
            Capability::new(
                CapabilityId::trusted("sensor.alarm"),
                CapabilityMode::Observe,
                ValueKind::Text,
            ),
        ],
        _ => Vec::new(),
    }
}

pub fn state_delta_for_report(report: &ZWaveValueReport) -> StateDelta {
    match report {
        ZWaveValueReport::Basic { value } => StateDelta {
            capability_id: CapabilityId::trusted("light.on_off"),
            value: Value::Bool(zwave_value_to_bool(*value)),
        },
        ZWaveValueReport::BinarySwitch { current_value } => StateDelta {
            capability_id: CapabilityId::trusted("light.on_off"),
            value: Value::Bool(*current_value),
        },
        ZWaveValueReport::MultilevelSwitch { current_level, .. } => StateDelta {
            capability_id: CapabilityId::trusted("light.brightness"),
            value: Value::Percentage(zwave_level_to_percentage(*current_level)),
        },
        ZWaveValueReport::BinarySensor { detected, .. } => StateDelta {
            capability_id: CapabilityId::trusted("sensor.occupancy"),
            value: Value::Bool(*detected),
        },
        ZWaveValueReport::MultilevelSensor {
            sensor_type,
            precision,
            raw_value,
            ..
        } => StateDelta {
            capability_id: multilevel_sensor_capability_id(*sensor_type),
            value: Value::Number(scaled_sensor_value(*raw_value, *precision)),
        },
        ZWaveValueReport::DoorLock { mode } => StateDelta {
            capability_id: CapabilityId::trusted("lock.state"),
            value: Value::Text(door_lock_state_name(*mode).to_string()),
        },
        ZWaveValueReport::Battery { level } => StateDelta {
            capability_id: CapabilityId::trusted("sensor.battery"),
            value: Value::Percentage(level.normalized_percentage()),
        },
        ZWaveValueReport::Meter {
            meter_type,
            scale,
            precision,
            raw_value,
        } => StateDelta {
            capability_id: meter_capability_id(*meter_type, *scale),
            value: Value::Number(scaled_sensor_value(*raw_value, *precision)),
        },
        ZWaveValueReport::Notification(report) => state_delta_for_notification(report),
    }
}

pub fn zwave_bool(value: bool) -> u8 {
    if value {
        0xff
    } else {
        0x00
    }
}

pub fn zwave_value_to_bool(value: u8) -> bool {
    value != 0x00
}

pub fn percentage_to_zwave_level(percent: u8) -> u8 {
    match percent {
        0 => 0,
        100..=u8::MAX => 99,
        value => value.min(99),
    }
}

pub fn zwave_level_to_percentage(level: u8) -> u8 {
    match level {
        0x00 => 0,
        0xff => 100,
        value => ((u16::from(value.min(99)) * 100 + 49) / 99) as u8,
    }
}

pub fn door_lock_state_name(mode: DoorLockMode) -> &'static str {
    match mode {
        DoorLockMode::Secured => "locked",
        DoorLockMode::Unsecured => "unlocked",
        DoorLockMode::Unknown(_) => "unknown",
    }
}

pub fn scaled_sensor_value(raw_value: i32, precision: u8) -> f64 {
    let scale = 10_f64.powi(i32::from(precision));
    f64::from(raw_value) / scale
}

pub fn multilevel_sensor_capability_id(sensor_type: u8) -> CapabilityId {
    match sensor_type {
        0x01 => CapabilityId::trusted("sensor.temperature"),
        0x03 => CapabilityId::trusted("sensor.illuminance"),
        0x05 => CapabilityId::trusted("sensor.humidity"),
        _ => CapabilityId::trusted("sensor.value"),
    }
}

pub fn meter_capability_id(meter_type: u8, scale: u8) -> CapabilityId {
    match (meter_type, scale) {
        (0x01, 0x00 | 0x01) => CapabilityId::trusted("sensor.energy"),
        (0x01, 0x02) => CapabilityId::trusted("sensor.power"),
        (0x01, 0x04) => CapabilityId::trusted("sensor.voltage"),
        (0x01, 0x05) => CapabilityId::trusted("sensor.current"),
        (0x01, 0x06) => CapabilityId::trusted("sensor.power_factor"),
        (0x02, 0x00 | 0x01) => CapabilityId::trusted("sensor.gas"),
        (0x03, 0x00..=0x02) => CapabilityId::trusted("sensor.water"),
        _ => CapabilityId::trusted("sensor.meter"),
    }
}

pub fn notification_state(report: &NotificationReport) -> NotificationState {
    match (report.notification_type, report.event) {
        (_, 0x00) => NotificationState::Idle,
        (NotificationType::HomeSecurity, 0x07..=0x09) => NotificationState::MotionDetected,
        (NotificationType::AccessControl, 0x16) => NotificationState::DoorOpen,
        (NotificationType::AccessControl, 0x17) => NotificationState::DoorClosed,
        (NotificationType::AccessControl, 0x01 | 0x03 | 0x05) => NotificationState::Locked,
        (NotificationType::AccessControl, 0x02 | 0x04 | 0x06) => NotificationState::Unlocked,
        (NotificationType::Smoke, _) => NotificationState::SmokeDetected,
        (NotificationType::Water, _) => NotificationState::WaterLeakDetected,
        (
            NotificationType::Heat
            | NotificationType::CarbonMonoxide
            | NotificationType::CarbonDioxide,
            _,
        ) => NotificationState::Alarm,
        (notification_type, event) => NotificationState::Unknown {
            notification_type,
            event,
        },
    }
}

pub fn state_delta_for_notification(report: &NotificationReport) -> StateDelta {
    match notification_state(report) {
        NotificationState::Idle => StateDelta {
            capability_id: CapabilityId::trusted("sensor.occupancy"),
            value: Value::Bool(false),
        },
        NotificationState::MotionDetected => StateDelta {
            capability_id: CapabilityId::trusted("sensor.occupancy"),
            value: Value::Bool(true),
        },
        NotificationState::DoorOpen => StateDelta {
            capability_id: CapabilityId::trusted("sensor.contact"),
            value: Value::Bool(true),
        },
        NotificationState::DoorClosed => StateDelta {
            capability_id: CapabilityId::trusted("sensor.contact"),
            value: Value::Bool(false),
        },
        NotificationState::Locked => StateDelta {
            capability_id: CapabilityId::trusted("lock.state"),
            value: Value::Text("locked".to_string()),
        },
        NotificationState::Unlocked => StateDelta {
            capability_id: CapabilityId::trusted("lock.state"),
            value: Value::Text("unlocked".to_string()),
        },
        NotificationState::SmokeDetected => StateDelta {
            capability_id: CapabilityId::trusted("sensor.alarm"),
            value: Value::Text("smoke".to_string()),
        },
        NotificationState::WaterLeakDetected => StateDelta {
            capability_id: CapabilityId::trusted("sensor.alarm"),
            value: Value::Text("water_leak".to_string()),
        },
        NotificationState::Alarm => StateDelta {
            capability_id: CapabilityId::trusted("sensor.alarm"),
            value: Value::Text("alarm".to_string()),
        },
        NotificationState::Unknown {
            notification_type,
            event,
        } => StateDelta {
            capability_id: CapabilityId::trusted("sensor.alarm"),
            value: Value::Text(format!(
                "notification.{:02x}.{:02x}",
                notification_type.as_byte(),
                event
            )),
        },
    }
}

fn is_commandable_capability(capability: &Capability) -> bool {
    matches!(
        capability.mode,
        CapabilityMode::Command | CapabilityMode::ObserveAndCommand
    )
}

fn is_sensor_capability(capability: &Capability) -> bool {
    capability.capability_id.as_str().starts_with("sensor.")
}

fn parse_notification_report(payload: &[u8]) -> Result<NotificationReport, CommandClassError> {
    require_len(payload, 6)?;
    let event_parameters = if let Some(parameter_len) = payload.get(6).copied() {
        let parameter_len = usize::from(parameter_len);
        require_len(payload, 7 + parameter_len)?;
        payload[7..7 + parameter_len].to_vec()
    } else {
        Vec::new()
    };
    Ok(NotificationReport {
        v1_alarm_type: payload[0],
        v1_alarm_level: payload[1],
        notification_status: payload[3],
        notification_type: NotificationType::parse(payload[4]),
        event: payload[5],
        event_parameters,
    })
}

fn parse_multilevel_sensor_report(payload: &[u8]) -> Result<ZWaveValueReport, CommandClassError> {
    require_len(payload, 2)?;
    let sensor_type = payload[0];
    let level = payload[1];
    let precision = (level >> 5) & 0b111;
    let scale = (level >> 3) & 0b11;
    let size = level & 0b111;
    if !matches!(size, 1 | 2 | 4) {
        return Err(CommandClassError::InvalidSensorValueSize(size));
    }
    require_len(payload, 2 + usize::from(size))?;
    let raw_value = signed_be_value(&payload[2..2 + usize::from(size)], size);
    Ok(ZWaveValueReport::MultilevelSensor {
        sensor_type,
        scale,
        precision,
        raw_value,
    })
}

fn parse_meter_report(payload: &[u8]) -> Result<ZWaveValueReport, CommandClassError> {
    require_len(payload, 2)?;
    let meter_type = payload[0] & 0b0001_1111;
    let properties = payload[1];
    let precision = (properties >> 5) & 0b111;
    let scale = (properties >> 3) & 0b11;
    let size = properties & 0b111;
    if !matches!(size, 1 | 2 | 4) {
        return Err(CommandClassError::InvalidMeterValueSize(size));
    }
    require_len(payload, 2 + usize::from(size))?;
    let raw_value = signed_be_value(&payload[2..2 + usize::from(size)], size);
    Ok(ZWaveValueReport::Meter {
        meter_type,
        scale,
        precision,
        raw_value,
    })
}

fn encode_notification_report(
    report: &NotificationReport,
) -> Result<ZWaveCommand, CommandClassError> {
    if report.event_parameters.len() > u8::MAX as usize {
        return Err(CommandClassError::InvalidReportValue(
            "notification event parameters must fit in one length byte",
        ));
    }

    let mut payload = vec![
        report.v1_alarm_type,
        report.v1_alarm_level,
        0x00,
        report.notification_status,
        report.notification_type.as_byte(),
        report.event,
    ];
    if !report.event_parameters.is_empty() {
        payload.push(report.event_parameters.len() as u8);
        payload.extend_from_slice(&report.event_parameters);
    }

    Ok(ZWaveCommand::new(
        COMMAND_CLASS_NOTIFICATION,
        NOTIFICATION_REPORT,
        payload,
    ))
}

fn encode_scaled_value_properties(
    precision: u8,
    scale: u8,
    raw_value: i32,
    payload: &mut Vec<u8>,
) -> Result<(), CommandClassError> {
    if precision > 0b111 {
        return Err(CommandClassError::InvalidReportValue(
            "precision must fit in three bits",
        ));
    }
    if scale > 0b11 {
        return Err(CommandClassError::InvalidReportValue(
            "scale must fit in two bits",
        ));
    }

    let value_bytes = compact_signed_be_bytes(raw_value);
    let size = value_bytes.len() as u8;
    payload.push((precision << 5) | (scale << 3) | size);
    payload.extend_from_slice(&value_bytes);
    Ok(())
}

fn compact_signed_be_bytes(value: i32) -> Vec<u8> {
    if let Ok(value) = i8::try_from(value) {
        value.to_be_bytes().to_vec()
    } else if let Ok(value) = i16::try_from(value) {
        value.to_be_bytes().to_vec()
    } else {
        value.to_be_bytes().to_vec()
    }
}

fn signed_be_value(bytes: &[u8], size: u8) -> i32 {
    match size {
        1 => i8::from_be_bytes([bytes[0]]) as i32,
        2 => i16::from_be_bytes([bytes[0], bytes[1]]) as i32,
        4 => i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        _ => 0,
    }
}

fn door_lock_mode(value: u8) -> DoorLockMode {
    match value {
        0x00 => DoorLockMode::Unsecured,
        0xff => DoorLockMode::Secured,
        other => DoorLockMode::Unknown(other),
    }
}

fn door_lock_mode_byte(mode: DoorLockMode) -> u8 {
    match mode {
        DoorLockMode::Unsecured => 0x00,
        DoorLockMode::Secured => 0xff,
        DoorLockMode::Unknown(value) => value,
    }
}

fn battery_level_byte(level: BatteryLevel) -> u8 {
    match level {
        BatteryLevel::Percentage(value) => value,
        BatteryLevel::LowWarning => BATTERY_LOW_WARNING,
        BatteryLevel::Reserved(value) => value,
    }
}

fn encoded_command_class_len(command_class: CommandClassId) -> Option<usize> {
    if command_class.0 <= u8::MAX as u16 {
        Some(1)
    } else if command_class.0 >= 0xf100 {
        Some(2)
    } else {
        None
    }
}

fn command_kind(command_class: CommandClassId, command_id: u8) -> ZWaveCommandKind {
    match (command_class, command_id) {
        (CommandClassId::BASIC, BASIC_GET)
        | (CommandClassId::SWITCH_BINARY, SWITCH_BINARY_GET)
        | (CommandClassId::SWITCH_MULTILEVEL, SWITCH_MULTILEVEL_GET)
        | (CommandClassId::SENSOR_BINARY, SENSOR_BINARY_GET)
        | (CommandClassId::SENSOR_MULTILEVEL, SENSOR_MULTILEVEL_GET)
        | (CommandClassId::DOOR_LOCK, DOOR_LOCK_OPERATION_GET)
        | (CommandClassId::BATTERY, BATTERY_GET)
        | (COMMAND_CLASS_METER, METER_GET)
        | (COMMAND_CLASS_NOTIFICATION, NOTIFICATION_GET) => ZWaveCommandKind::Get,
        (CommandClassId::BASIC, BASIC_SET)
        | (CommandClassId::SWITCH_BINARY, SWITCH_BINARY_SET)
        | (CommandClassId::SWITCH_MULTILEVEL, SWITCH_MULTILEVEL_SET)
        | (CommandClassId::DOOR_LOCK, DOOR_LOCK_OPERATION_SET) => ZWaveCommandKind::Set,
        (CommandClassId::BASIC, BASIC_REPORT)
        | (CommandClassId::SWITCH_BINARY, SWITCH_BINARY_REPORT)
        | (CommandClassId::SWITCH_MULTILEVEL, SWITCH_MULTILEVEL_REPORT)
        | (CommandClassId::SENSOR_BINARY, SENSOR_BINARY_REPORT)
        | (CommandClassId::SENSOR_MULTILEVEL, SENSOR_MULTILEVEL_REPORT)
        | (CommandClassId::DOOR_LOCK, DOOR_LOCK_OPERATION_REPORT)
        | (CommandClassId::BATTERY, BATTERY_REPORT)
        | (COMMAND_CLASS_METER, METER_REPORT)
        | (COMMAND_CLASS_NOTIFICATION, NOTIFICATION_REPORT) => ZWaveCommandKind::Report,
        _ => ZWaveCommandKind::Other,
    }
}

fn require_len(bytes: &[u8], needed: usize) -> Result<(), CommandClassError> {
    if bytes.len() < needed {
        return Err(CommandClassError::Truncated {
            needed,
            remaining: bytes.len(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_payloads_round_trip() {
        let command = binary_switch_set(true);
        let encoded = command.encode().unwrap();

        assert_eq!(encoded, vec![0x25, 0x01, 0xff]);
        assert_eq!(ZWaveCommand::parse(&encoded).unwrap(), command);
    }

    #[test]
    fn command_summary_reports_shape_without_payload_bytes() {
        let command = binary_switch_set(true);

        let summary = command.summary();

        assert_eq!(
            summary,
            ZWaveCommandSummary {
                command_class: CommandClassId::SWITCH_BINARY,
                command_id: SWITCH_BINARY_SET,
                command_kind: ZWaveCommandKind::Set,
                payload_len: 1,
                uses_extended_command_class: false,
                can_encode: true,
                encoded_len: Some(3),
            }
        );
        assert!(summary.has_payload());
        assert!(!summary.is_payload_free());
        assert!(!summary.is_get());
        assert!(summary.is_set());
        assert!(summary.is_request());
        assert!(!summary.is_report());
    }

    #[test]
    fn command_summary_handles_extended_and_invalid_class_ids() {
        let extended = ZWaveCommand::new(CommandClassId(0xf102), 0x09, vec![0xaa, 0xbb]);
        let invalid = ZWaveCommand::new(CommandClassId(0x0101), 0x01, Vec::new());

        assert_eq!(
            extended.summary(),
            ZWaveCommandSummary {
                command_class: CommandClassId(0xf102),
                command_id: 0x09,
                command_kind: ZWaveCommandKind::Other,
                payload_len: 2,
                uses_extended_command_class: true,
                can_encode: true,
                encoded_len: Some(5),
            }
        );
        assert_eq!(
            invalid.summary(),
            ZWaveCommandSummary {
                command_class: CommandClassId(0x0101),
                command_id: 0x01,
                command_kind: ZWaveCommandKind::Other,
                payload_len: 0,
                uses_extended_command_class: true,
                can_encode: false,
                encoded_len: None,
            }
        );
    }

    #[test]
    fn command_summary_classifies_known_reports() {
        let command = ZWaveCommand::new(CommandClassId::BATTERY, BATTERY_REPORT, vec![87]);

        let summary = command.summary();

        assert_eq!(summary.command_kind, ZWaveCommandKind::Report);
        assert!(!summary.is_get());
        assert!(!summary.is_set());
        assert!(summary.is_report());
        assert!(!summary.is_request());
    }

    #[test]
    fn command_batch_summary_rolls_up_command_shapes() {
        let get = binary_switch_get();
        let set = binary_switch_set(true);
        let report = ZWaveCommand::new(CommandClassId::BATTERY, BATTERY_REPORT, vec![87]);
        let extended = ZWaveCommand::new(CommandClassId(0xf102), 0x09, vec![0xaa, 0xbb]);

        let summary = summarize_zwave_commands([&get, &set, &report, &extended]);

        assert_eq!(
            summary,
            ZWaveCommandBatchSummary {
                total_commands: 4,
                get_commands: 1,
                set_commands: 1,
                report_commands: 1,
                other_commands: 1,
                payload_free_commands: 1,
                commands_with_payload: 3,
                extended_command_classes: 1,
                encodable_commands: 4,
                unencodable_commands: 0,
                total_payload_bytes: 4,
                total_encoded_bytes: Some(13),
                unique_command_classes: 3,
            }
        );
        assert!(!summary.is_empty());
        assert!(summary.has_requests());
        assert!(summary.has_reports());
        assert!(summary.has_payloads());
        assert!(!summary.has_unencodable_commands());
    }

    #[test]
    fn command_batch_summary_tracks_unencodable_commands() {
        let invalid = ZWaveCommand::new(CommandClassId(0x0101), 0x01, Vec::new());

        let summary = ZWaveCommandBatchSummary::from_summaries([invalid.summary()]);

        assert_eq!(summary.total_commands, 1);
        assert_eq!(summary.other_commands, 1);
        assert_eq!(summary.payload_free_commands, 1);
        assert_eq!(summary.extended_command_classes, 1);
        assert_eq!(summary.encodable_commands, 0);
        assert_eq!(summary.unencodable_commands, 1);
        assert_eq!(summary.total_encoded_bytes, None);
        assert_eq!(summary.unique_command_classes, 1);
        assert!(!summary.has_requests());
        assert!(!summary.has_reports());
        assert!(summary.has_unencodable_commands());

        let empty = ZWaveCommandBatchSummary::from_summaries([]);
        assert!(empty.is_empty());
        assert_eq!(empty.total_encoded_bytes, Some(0));
    }

    #[test]
    fn set_builders_normalize_values() {
        let basic_summary = basic_get().summary();
        assert_eq!(
            basic_get().encode().unwrap(),
            vec![CommandClassId::BASIC.0 as u8, BASIC_GET]
        );
        assert!(basic_summary.is_get());
        assert!(!basic_summary.is_set());
        assert!(basic_summary.is_payload_free());
        assert_eq!(binary_switch_set(false).payload, vec![0x00]);
        assert_eq!(multilevel_switch_set(100).payload, vec![99]);
        assert_eq!(door_lock_operation_set(true).payload, vec![0xff]);
        assert_eq!(
            binary_sensor_get().encode().unwrap(),
            vec![CommandClassId::SENSOR_BINARY.0 as u8, SENSOR_BINARY_GET]
        );
        assert_eq!(
            multilevel_sensor_get().encode().unwrap(),
            vec![
                CommandClassId::SENSOR_MULTILEVEL.0 as u8,
                SENSOR_MULTILEVEL_GET
            ]
        );
        assert_eq!(
            battery_get().encode().unwrap(),
            vec![CommandClassId::BATTERY.0 as u8, BATTERY_GET]
        );
        assert_eq!(
            meter_get().encode().unwrap(),
            vec![COMMAND_CLASS_METER.0 as u8, METER_GET]
        );
    }

    #[test]
    fn command_class_interview_descriptors_project_queries_and_capabilities() {
        let descriptors = interview_descriptors_for_command_classes([
            CommandClassId::BATTERY,
            CommandClassId::SWITCH_BINARY,
            CommandClassId::BATTERY,
            COMMAND_CLASS_METER,
            COMMAND_CLASS_NOTIFICATION,
            CommandClassId(0xfe),
        ]);

        assert_eq!(
            descriptors
                .iter()
                .map(|descriptor| descriptor.command_class)
                .collect::<Vec<_>>(),
            vec![
                CommandClassId::SWITCH_BINARY,
                COMMAND_CLASS_METER,
                COMMAND_CLASS_NOTIFICATION,
                CommandClassId::BATTERY,
            ]
        );

        let switch = descriptors
            .iter()
            .find(|descriptor| descriptor.command_class == CommandClassId::SWITCH_BINARY)
            .unwrap();
        assert_eq!(switch.commands, vec![binary_switch_get()]);
        assert_eq!(
            switch.capabilities[0].capability_id,
            CapabilityId::trusted("light.on_off")
        );
        assert!(switch.can_query_state());
        assert!(switch.projects_capabilities());

        let notification = descriptors
            .iter()
            .find(|descriptor| descriptor.command_class == COMMAND_CLASS_NOTIFICATION)
            .unwrap();
        assert!(notification.commands.is_empty());
        assert!(!notification.can_query_state());
        assert!(notification.projects_capabilities());

        let meter = descriptors
            .iter()
            .find(|descriptor| descriptor.command_class == COMMAND_CLASS_METER)
            .unwrap();
        assert_eq!(meter.commands, vec![meter_get()]);
        assert!(meter.capabilities.iter().any(|capability| {
            capability.capability_id == CapabilityId::trusted("sensor.energy")
        }));
    }

    #[test]
    fn parses_binary_and_multilevel_switch_reports() {
        let binary = ZWaveCommand::new(
            CommandClassId::SWITCH_BINARY,
            SWITCH_BINARY_REPORT,
            vec![0xff],
        );
        let multilevel = ZWaveCommand::new(
            CommandClassId::SWITCH_MULTILEVEL,
            SWITCH_MULTILEVEL_REPORT,
            vec![99, 50, 0],
        );

        assert_eq!(
            parse_value_report(&binary).unwrap(),
            ZWaveValueReport::BinarySwitch {
                current_value: true
            }
        );
        assert_eq!(
            parse_value_report(&multilevel).unwrap(),
            ZWaveValueReport::MultilevelSwitch {
                current_level: 99,
                target_level: Some(50),
                duration: Some(0),
            }
        );
    }

    #[test]
    fn parses_multilevel_sensor_signed_scaled_values() {
        let report = ZWaveCommand::new(
            CommandClassId::SENSOR_MULTILEVEL,
            SENSOR_MULTILEVEL_REPORT,
            vec![0x01, 0b0010_1010, 0x09, 0xc4],
        );

        assert_eq!(
            parse_value_report(&report).unwrap(),
            ZWaveValueReport::MultilevelSensor {
                sensor_type: 0x01,
                scale: 1,
                precision: 1,
                raw_value: 2500,
            }
        );
        assert_eq!(scaled_sensor_value(2500, 1), 250.0);
    }

    #[test]
    fn parses_battery_reports_and_low_warning() {
        let percentage = ZWaveCommand::new(CommandClassId::BATTERY, BATTERY_REPORT, vec![87]);
        let low_warning = ZWaveCommand::new(
            CommandClassId::BATTERY,
            BATTERY_REPORT,
            vec![BATTERY_LOW_WARNING],
        );

        assert_eq!(
            parse_value_report(&percentage).unwrap(),
            ZWaveValueReport::Battery {
                level: BatteryLevel::Percentage(87),
            }
        );
        assert_eq!(
            parse_value_report(&low_warning).unwrap(),
            ZWaveValueReport::Battery {
                level: BatteryLevel::LowWarning,
            }
        );
        assert!(BatteryLevel::LowWarning.is_low_warning());
        assert_eq!(BatteryLevel::LowWarning.normalized_percentage(), 0);
        assert_eq!(BatteryLevel::Reserved(200).normalized_percentage(), 100);
    }

    #[test]
    fn parses_meter_reports_and_projects_energy_or_power() {
        let energy = ZWaveCommand::new(
            COMMAND_CLASS_METER,
            METER_REPORT,
            vec![0x01, 0b0100_0010, 0x04, 0xd2],
        );
        let power = ZWaveCommand::new(
            COMMAND_CLASS_METER,
            METER_REPORT,
            vec![0x01, 0b0001_0010, 0x01, 0xc2],
        );

        assert_eq!(
            parse_value_report(&energy).unwrap(),
            ZWaveValueReport::Meter {
                meter_type: 0x01,
                scale: 0,
                precision: 2,
                raw_value: 1234,
            }
        );

        let energy_delta = state_delta_for_report(&parse_value_report(&energy).unwrap());
        assert_eq!(
            energy_delta.capability_id,
            CapabilityId::trusted("sensor.energy")
        );
        match energy_delta.value {
            Value::Number(value) => assert!((value - 12.34).abs() < f64::EPSILON),
            other => panic!("expected numeric energy value, got {other:?}"),
        }

        let power_delta = state_delta_for_report(&parse_value_report(&power).unwrap());
        assert_eq!(
            power_delta.capability_id,
            CapabilityId::trusted("sensor.power")
        );
        assert_eq!(power_delta.value, Value::Number(450.0));
    }

    #[test]
    fn encodes_common_value_reports_for_round_trip_fixtures() {
        let reports = vec![
            ZWaveValueReport::Basic { value: 0xff },
            ZWaveValueReport::BinarySwitch {
                current_value: true,
            },
            ZWaveValueReport::MultilevelSwitch {
                current_level: 99,
                target_level: Some(50),
                duration: Some(0),
            },
            ZWaveValueReport::BinarySensor {
                detected: true,
                sensor_type: Some(0x0c),
            },
            ZWaveValueReport::DoorLock {
                mode: DoorLockMode::Secured,
            },
            ZWaveValueReport::Battery {
                level: BatteryLevel::LowWarning,
            },
        ];

        for report in reports {
            let command = encode_value_report(&report).unwrap();
            assert_eq!(parse_value_report(&command).unwrap(), report);
            assert!(command.summary().is_report());
        }
    }

    #[test]
    fn encodes_scaled_sensor_meter_and_notification_reports() {
        let sensor = ZWaveValueReport::MultilevelSensor {
            sensor_type: 0x01,
            scale: 1,
            precision: 1,
            raw_value: 2500,
        };
        let meter = ZWaveValueReport::Meter {
            meter_type: 0x01,
            scale: 0,
            precision: 2,
            raw_value: 1234,
        };
        let notification = ZWaveValueReport::Notification(NotificationReport {
            v1_alarm_type: 0x00,
            v1_alarm_level: 0x00,
            notification_status: 0xff,
            notification_type: NotificationType::HomeSecurity,
            event: 0x08,
            event_parameters: vec![0xaa, 0xbb],
        });

        assert_eq!(
            encode_value_report(&sensor).unwrap().payload,
            vec![0x01, 0b0010_1010, 0x09, 0xc4]
        );
        assert_eq!(
            encode_value_report(&meter).unwrap().payload,
            vec![0x01, 0b0100_0010, 0x04, 0xd2]
        );
        assert_eq!(
            encode_value_report(&notification).unwrap().payload,
            vec![0x00, 0x00, 0x00, 0xff, 0x07, 0x08, 0x02, 0xaa, 0xbb]
        );

        for report in [sensor, meter, notification] {
            let command = encode_value_report(&report).unwrap();
            assert_eq!(parse_value_report(&command).unwrap(), report);
        }
    }

    #[test]
    fn report_encoder_rejects_unrepresentable_packed_fields() {
        let bad_precision = ZWaveValueReport::MultilevelSensor {
            sensor_type: 0x01,
            scale: 0,
            precision: 8,
            raw_value: 1,
        };
        let bad_scale = ZWaveValueReport::Meter {
            meter_type: 0x01,
            scale: 4,
            precision: 0,
            raw_value: 1,
        };
        let bad_meter_type = ZWaveValueReport::Meter {
            meter_type: 0x20,
            scale: 0,
            precision: 0,
            raw_value: 1,
        };

        assert!(matches!(
            encode_value_report(&bad_precision),
            Err(CommandClassError::InvalidReportValue(_))
        ));
        assert!(matches!(
            encode_value_report(&bad_scale),
            Err(CommandClassError::InvalidReportValue(_))
        ));
        assert!(matches!(
            encode_value_report(&bad_meter_type),
            Err(CommandClassError::InvalidReportValue(_))
        ));
    }

    #[test]
    fn maps_reports_to_d23_state_deltas() {
        let lock = ZWaveValueReport::DoorLock {
            mode: DoorLockMode::Secured,
        };
        let dimmer = ZWaveValueReport::MultilevelSwitch {
            current_level: 99,
            target_level: None,
            duration: None,
        };
        let battery = ZWaveValueReport::Battery {
            level: BatteryLevel::Percentage(72),
        };

        assert_eq!(
            state_delta_for_report(&lock),
            StateDelta {
                capability_id: CapabilityId::trusted("lock.state"),
                value: Value::Text("locked".to_string()),
            }
        );
        assert_eq!(
            state_delta_for_report(&dimmer),
            StateDelta {
                capability_id: CapabilityId::trusted("light.brightness"),
                value: Value::Percentage(100),
            }
        );
        assert_eq!(
            state_delta_for_report(&battery),
            StateDelta {
                capability_id: CapabilityId::trusted("sensor.battery"),
                value: Value::Percentage(72),
            }
        );
    }

    #[test]
    fn parses_notification_reports_with_event_parameters() {
        let report = ZWaveCommand::new(
            COMMAND_CLASS_NOTIFICATION,
            NOTIFICATION_REPORT,
            vec![0x00, 0x00, 0x00, 0xff, 0x07, 0x08, 0x02, 0xaa, 0xbb],
        );

        assert_eq!(
            parse_value_report(&report).unwrap(),
            ZWaveValueReport::Notification(NotificationReport {
                v1_alarm_type: 0x00,
                v1_alarm_level: 0x00,
                notification_status: 0xff,
                notification_type: NotificationType::HomeSecurity,
                event: 0x08,
                event_parameters: vec![0xaa, 0xbb],
            })
        );
    }

    #[test]
    fn notification_reports_project_common_states() {
        let motion = NotificationReport {
            v1_alarm_type: 0,
            v1_alarm_level: 0,
            notification_status: 0xff,
            notification_type: NotificationType::HomeSecurity,
            event: 0x08,
            event_parameters: Vec::new(),
        };
        let door_closed = NotificationReport {
            notification_type: NotificationType::AccessControl,
            event: 0x17,
            ..motion.clone()
        };
        let unlocked = NotificationReport {
            notification_type: NotificationType::AccessControl,
            event: 0x02,
            ..motion.clone()
        };

        assert_eq!(
            state_delta_for_notification(&motion),
            StateDelta {
                capability_id: CapabilityId::trusted("sensor.occupancy"),
                value: Value::Bool(true),
            }
        );
        assert_eq!(
            state_delta_for_notification(&door_closed),
            StateDelta {
                capability_id: CapabilityId::trusted("sensor.contact"),
                value: Value::Bool(false),
            }
        );
        assert_eq!(
            state_delta_for_notification(&unlocked),
            StateDelta {
                capability_id: CapabilityId::trusted("lock.state"),
                value: Value::Text("unlocked".to_string()),
            }
        );
    }

    #[test]
    fn command_classes_project_capabilities() {
        assert_eq!(
            capabilities_for_command_class(CommandClassId::SWITCH_BINARY)[0].capability_id,
            CapabilityId::trusted("light.on_off")
        );
        assert_eq!(
            capabilities_for_command_class(CommandClassId::DOOR_LOCK)[0].capability_id,
            CapabilityId::trusted("lock.state")
        );
        assert_eq!(
            capabilities_for_command_class(CommandClassId::BATTERY)[0].capability_id,
            CapabilityId::trusted("sensor.battery")
        );
        assert!(capabilities_for_command_class(COMMAND_CLASS_METER)
            .iter()
            .any(|capability| capability.capability_id == CapabilityId::trusted("sensor.power")));
        assert!(capabilities_for_command_class(COMMAND_CLASS_NOTIFICATION)
            .iter()
            .any(|capability| capability.capability_id == CapabilityId::trusted("sensor.alarm")));
        assert!(capabilities_for_command_class(CommandClassId::BASIC).is_empty());
    }

    #[test]
    fn command_class_projection_summary_counts_d23_surfaces() {
        let summary = CommandClassProjectionSummary::from_command_classes([
            CommandClassId::SWITCH_BINARY,
            CommandClassId::SWITCH_BINARY,
            CommandClassId::SWITCH_MULTILEVEL,
            CommandClassId::SENSOR_BINARY,
            COMMAND_CLASS_NOTIFICATION,
            CommandClassId::BASIC,
        ]);

        assert_eq!(
            summary,
            CommandClassProjectionSummary {
                command_class_entries: 6,
                unique_command_classes: 5,
                projected_command_classes: 4,
                commandable_command_classes: 2,
                sensor_command_classes: 2,
                projected_capabilities: 7,
                observe_only_capabilities: 4,
                commandable_capabilities: 3,
            }
        );
        assert!(summary.has_projected_capabilities());
        assert!(summary.has_command_surface());
        assert!(summary.has_sensor_surface());

        let empty = CommandClassProjectionSummary::from_command_classes([CommandClassId::BASIC]);
        assert_eq!(empty.command_class_entries, 1);
        assert_eq!(empty.unique_command_classes, 1);
        assert!(!empty.has_projected_capabilities());
        assert!(!empty.has_command_surface());
        assert!(!empty.has_sensor_surface());
    }

    #[test]
    fn command_class_projection_readiness_marks_mixed_surface_ready() {
        let summary = CommandClassProjectionSummary::from_command_classes([
            CommandClassId::SWITCH_BINARY,
            CommandClassId::DOOR_LOCK,
            CommandClassId::BATTERY,
            COMMAND_CLASS_METER,
            COMMAND_CLASS_NOTIFICATION,
        ]);

        let readiness = summary.readiness();

        assert_eq!(readiness.projection_summary, summary);
        assert_eq!(readiness.required_projection_check_count, 5);
        assert_eq!(readiness.passed_projection_check_count, 5);
        assert_eq!(readiness.missing_projection_check_count, 0);
        assert!(readiness.command_classes_present);
        assert!(readiness.capability_projection_ready);
        assert!(readiness.command_surface_ready);
        assert!(readiness.sensor_surface_ready);
        assert!(readiness.observe_only_surface_ready);
        assert!(readiness.projection_ready);
        assert!(readiness.is_projection_ready());
        assert!(!readiness.has_missing_projection_checks());
        assert!(!readiness.needs_command_class_inventory());
        assert!(!readiness.needs_capability_projection());
        assert!(!readiness.needs_command_surface());
        assert!(!readiness.needs_sensor_surface());
        assert!(!readiness.needs_observe_only_surface());
    }

    #[test]
    fn command_class_projection_readiness_routes_sparse_inventory_gaps() {
        let basic_only = CommandClassProjectionReadinessSummary::from_command_classes([
            CommandClassId::BASIC,
            CommandClassId::BASIC,
        ]);

        assert_eq!(basic_only.required_projection_check_count, 5);
        assert_eq!(basic_only.passed_projection_check_count, 1);
        assert_eq!(basic_only.missing_projection_check_count, 4);
        assert!(basic_only.command_classes_present);
        assert!(!basic_only.capability_projection_ready);
        assert!(!basic_only.command_surface_ready);
        assert!(!basic_only.sensor_surface_ready);
        assert!(!basic_only.observe_only_surface_ready);
        assert!(!basic_only.projection_ready);
        assert!(!basic_only.needs_command_class_inventory());
        assert!(basic_only.needs_capability_projection());
        assert!(basic_only.needs_command_surface());
        assert!(basic_only.needs_sensor_surface());
        assert!(basic_only.needs_observe_only_surface());

        let empty = CommandClassProjectionReadinessSummary::from_command_classes([]);
        assert_eq!(empty.passed_projection_check_count, 0);
        assert!(empty.needs_command_class_inventory());
        assert!(empty.has_missing_projection_checks());
    }

    #[test]
    fn command_class_projection_signoff_marks_mixed_surface_ready() {
        let readiness = CommandClassProjectionReadinessSummary::from_command_classes([
            CommandClassId::SWITCH_BINARY,
            CommandClassId::DOOR_LOCK,
            CommandClassId::BATTERY,
            COMMAND_CLASS_METER,
            COMMAND_CLASS_NOTIFICATION,
        ]);

        let signoff = CommandClassProjectionSignoffSummary::from_readiness_summary(readiness);

        assert_eq!(signoff.readiness_summary, readiness);
        assert_eq!(signoff.required_signoff_check_count, 6);
        assert_eq!(signoff.passed_signoff_check_count, 6);
        assert_eq!(signoff.missing_signoff_check_count, 0);
        assert!(signoff.projection_ready);
        assert!(signoff.command_classes_present);
        assert!(signoff.capability_projection_ready);
        assert!(signoff.command_surface_ready);
        assert!(signoff.sensor_surface_ready);
        assert!(signoff.observe_only_surface_ready);
        assert!(signoff.signoff_ready);
        assert!(signoff.is_signoff_ready());
        assert!(!signoff.has_missing_signoff_checks());
        assert!(!signoff.needs_projection_readiness());
        assert!(!signoff.needs_command_class_inventory());
        assert!(!signoff.needs_capability_projection());
        assert!(!signoff.needs_command_surface());
        assert!(!signoff.needs_sensor_surface());
        assert!(!signoff.needs_observe_only_surface());
    }

    #[test]
    fn command_class_projection_signoff_routes_sparse_inventory_gaps() {
        let basic_only = CommandClassProjectionSignoffSummary::from_command_classes([
            CommandClassId::BASIC,
            CommandClassId::BASIC,
        ]);

        assert_eq!(basic_only.required_signoff_check_count, 6);
        assert_eq!(basic_only.passed_signoff_check_count, 1);
        assert_eq!(basic_only.missing_signoff_check_count, 5);
        assert!(!basic_only.projection_ready);
        assert!(basic_only.command_classes_present);
        assert!(!basic_only.capability_projection_ready);
        assert!(!basic_only.command_surface_ready);
        assert!(!basic_only.sensor_surface_ready);
        assert!(!basic_only.observe_only_surface_ready);
        assert!(!basic_only.signoff_ready);
        assert!(!basic_only.is_signoff_ready());
        assert!(basic_only.has_missing_signoff_checks());
        assert!(basic_only.needs_projection_readiness());
        assert!(!basic_only.needs_command_class_inventory());
        assert!(basic_only.needs_capability_projection());
        assert!(basic_only.needs_command_surface());
        assert!(basic_only.needs_sensor_surface());
        assert!(basic_only.needs_observe_only_surface());

        let empty = CommandClassProjectionSignoffSummary::from_command_classes([]);
        assert_eq!(empty.passed_signoff_check_count, 0);
        assert!(empty.needs_command_class_inventory());
        assert!(empty.has_missing_signoff_checks());
    }

    #[test]
    fn command_class_projection_closure_marks_mixed_surface_ready() {
        let signoff = CommandClassProjectionSignoffSummary::from_command_classes([
            CommandClassId::SWITCH_BINARY,
            CommandClassId::DOOR_LOCK,
            CommandClassId::BATTERY,
            COMMAND_CLASS_METER,
            COMMAND_CLASS_NOTIFICATION,
        ]);

        let closure = CommandClassProjectionClosureSummary::from_signoff_summary(signoff);

        assert_eq!(closure.signoff_summary, signoff);
        assert_eq!(closure.required_closure_check_count, 7);
        assert_eq!(closure.passed_closure_check_count, 7);
        assert_eq!(closure.missing_closure_check_count, 0);
        assert!(closure.signoff_ready);
        assert!(closure.projection_ready);
        assert!(closure.command_classes_present);
        assert!(closure.capability_projection_ready);
        assert!(closure.command_surface_ready);
        assert!(closure.sensor_surface_ready);
        assert!(closure.observe_only_surface_ready);
        assert!(closure.closure_ready);
        assert!(closure.is_closure_ready());
        assert!(!closure.has_missing_closure_checks());
        assert!(!closure.needs_projection_signoff());
        assert!(!closure.needs_projection_readiness());
        assert!(!closure.needs_command_class_inventory());
        assert!(!closure.needs_capability_projection());
        assert!(!closure.needs_command_surface());
        assert!(!closure.needs_sensor_surface());
        assert!(!closure.needs_observe_only_surface());
    }

    #[test]
    fn command_class_projection_closure_routes_sparse_inventory_gaps() {
        let basic_only = CommandClassProjectionClosureSummary::from_command_classes([
            CommandClassId::BASIC,
            CommandClassId::BASIC,
        ]);

        assert_eq!(basic_only.required_closure_check_count, 7);
        assert_eq!(basic_only.passed_closure_check_count, 1);
        assert_eq!(basic_only.missing_closure_check_count, 6);
        assert!(!basic_only.signoff_ready);
        assert!(!basic_only.projection_ready);
        assert!(basic_only.command_classes_present);
        assert!(!basic_only.capability_projection_ready);
        assert!(!basic_only.command_surface_ready);
        assert!(!basic_only.sensor_surface_ready);
        assert!(!basic_only.observe_only_surface_ready);
        assert!(!basic_only.closure_ready);
        assert!(!basic_only.is_closure_ready());
        assert!(basic_only.has_missing_closure_checks());
        assert!(basic_only.needs_projection_signoff());
        assert!(basic_only.needs_projection_readiness());
        assert!(!basic_only.needs_command_class_inventory());
        assert!(basic_only.needs_capability_projection());
        assert!(basic_only.needs_command_surface());
        assert!(basic_only.needs_sensor_surface());
        assert!(basic_only.needs_observe_only_surface());

        let empty = CommandClassProjectionClosureSummary::from_command_classes([]);
        assert_eq!(empty.passed_closure_check_count, 0);
        assert!(empty.needs_command_class_inventory());
        assert!(empty.has_missing_closure_checks());
    }

    #[test]
    fn sensor_report_rejects_invalid_value_size() {
        let report = ZWaveCommand::new(
            CommandClassId::SENSOR_MULTILEVEL,
            SENSOR_MULTILEVEL_REPORT,
            vec![0x01, 0x03],
        );

        assert_eq!(
            parse_value_report(&report),
            Err(CommandClassError::InvalidSensorValueSize(3))
        );
    }

    #[test]
    fn meter_report_rejects_invalid_value_size() {
        let report = ZWaveCommand::new(COMMAND_CLASS_METER, METER_REPORT, vec![0x01, 0x03]);

        assert_eq!(
            parse_value_report(&report),
            Err(CommandClassError::InvalidMeterValueSize(3))
        );
    }

    #[test]
    fn invalid_extended_command_class_ids_are_rejected() {
        let command = ZWaveCommand::new(CommandClassId(0x0101), 0x01, Vec::new());

        assert_eq!(
            command.encode(),
            Err(CommandClassError::InvalidExtendedCommandClassId(0x0101))
        );
    }
}
