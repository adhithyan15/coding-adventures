//! Matter application-layer primitives for D23 smart-home integrations.
//!
//! Matter-over-Thread enters the smart-home runtime through Matter clusters,
//! attributes, and commands. This crate deliberately avoids networking,
//! commissioning, certificates, secure sessions, and fabric storage. It only
//! owns the typed identifiers and projection helpers that map Matter reports
//! into `smart-home-core`.

#![forbid(unsafe_code)]

use smart_home_core::{
    Capability, CapabilityId, CommandType, DeviceCommand, EntityKind, StateDelta, Value,
};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatterError {
    EmptyIdentifier {
        kind: &'static str,
    },
    InvalidEndpoint {
        endpoint_id: u16,
    },
    InvalidLevel {
        value: u16,
    },
    InvalidHumidity {
        value: u32,
    },
    UnsupportedAttribute {
        cluster_id: MatterClusterId,
        attribute_id: MatterAttributeId,
    },
    UnsupportedCommand {
        command_type: CommandType,
    },
    UnexpectedCommandArgument {
        command_type: CommandType,
        expected: &'static str,
    },
    UnexpectedValueKind {
        cluster_id: MatterClusterId,
        attribute_id: MatterAttributeId,
        expected: &'static str,
    },
}

impl fmt::Display for MatterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyIdentifier { kind } => write!(f, "{kind} must not be empty"),
            Self::InvalidEndpoint { endpoint_id } => {
                write!(f, "Matter endpoint id {endpoint_id} is outside 1..=65534")
            }
            Self::InvalidLevel { value } => {
                write!(f, "Matter level {value} is outside 0..=254")
            }
            Self::InvalidHumidity { value } => {
                write!(f, "Matter humidity {value} is outside 0..=10000 hundredths percent")
            }
            Self::UnsupportedAttribute {
                cluster_id,
                attribute_id,
            } => write!(
                f,
                "Matter cluster {cluster_id} attribute {attribute_id} does not map to a D23 state delta"
            ),
            Self::UnsupportedCommand { command_type } => {
                write!(f, "smart-home command {command_type:?} does not map to a Matter command")
            }
            Self::UnexpectedCommandArgument {
                command_type,
                expected,
            } => write!(
                f,
                "smart-home command {command_type:?} expected Matter command argument {expected}"
            ),
            Self::UnexpectedValueKind {
                cluster_id,
                attribute_id,
                expected,
            } => write!(
                f,
                "Matter cluster {cluster_id} attribute {attribute_id} expected {expected}"
            ),
        }
    }
}

impl std::error::Error for MatterError {}

macro_rules! id_type {
    ($name:ident, $inner:ty, $kind:literal) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name($inner);

        impl $name {
            pub const fn new(value: $inner) -> Self {
                Self(value)
            }

            pub const fn value(self) -> $inner {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, concat!($kind, "({:#x})"), self.0)
            }
        }
    };
}

id_type!(MatterFabricId, u64, "fabric");
id_type!(MatterNodeId, u64, "node");
id_type!(MatterClusterId, u32, "cluster");
id_type!(MatterAttributeId, u32, "attribute");
id_type!(MatterCommandId, u32, "command");

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MatterEndpointId(u16);

impl MatterEndpointId {
    pub fn new(endpoint_id: u16) -> Result<Self, MatterError> {
        if endpoint_id == 0 || endpoint_id == u16::MAX {
            return Err(MatterError::InvalidEndpoint { endpoint_id });
        }
        Ok(Self(endpoint_id))
    }

    pub const fn trusted(endpoint_id: u16) -> Self {
        Self(endpoint_id)
    }

    pub const fn value(self) -> u16 {
        self.0
    }
}

impl fmt::Display for MatterEndpointId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "endpoint({})", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatterCluster {
    Descriptor,
    Groups,
    Scenes,
    OnOff,
    LevelControl,
    DoorLock,
    Thermostat,
    ColorControl,
    IlluminanceMeasurement,
    TemperatureMeasurement,
    RelativeHumidityMeasurement,
    OccupancySensing,
    Switch,
    Unknown(MatterClusterId),
}

impl MatterCluster {
    pub const DESCRIPTOR: MatterClusterId = MatterClusterId::new(0x001d);
    pub const GROUPS: MatterClusterId = MatterClusterId::new(0x0004);
    pub const SCENES: MatterClusterId = MatterClusterId::new(0x0005);
    pub const ON_OFF: MatterClusterId = MatterClusterId::new(0x0006);
    pub const LEVEL_CONTROL: MatterClusterId = MatterClusterId::new(0x0008);
    pub const DOOR_LOCK: MatterClusterId = MatterClusterId::new(0x0101);
    pub const THERMOSTAT: MatterClusterId = MatterClusterId::new(0x0201);
    pub const COLOR_CONTROL: MatterClusterId = MatterClusterId::new(0x0300);
    pub const ILLUMINANCE_MEASUREMENT: MatterClusterId = MatterClusterId::new(0x0400);
    pub const TEMPERATURE_MEASUREMENT: MatterClusterId = MatterClusterId::new(0x0402);
    pub const RELATIVE_HUMIDITY_MEASUREMENT: MatterClusterId = MatterClusterId::new(0x0405);
    pub const OCCUPANCY_SENSING: MatterClusterId = MatterClusterId::new(0x0406);
    pub const SWITCH: MatterClusterId = MatterClusterId::new(0x003b);

    pub fn from_id(cluster_id: MatterClusterId) -> Self {
        match cluster_id {
            Self::DESCRIPTOR => Self::Descriptor,
            Self::GROUPS => Self::Groups,
            Self::SCENES => Self::Scenes,
            Self::ON_OFF => Self::OnOff,
            Self::LEVEL_CONTROL => Self::LevelControl,
            Self::DOOR_LOCK => Self::DoorLock,
            Self::THERMOSTAT => Self::Thermostat,
            Self::COLOR_CONTROL => Self::ColorControl,
            Self::ILLUMINANCE_MEASUREMENT => Self::IlluminanceMeasurement,
            Self::TEMPERATURE_MEASUREMENT => Self::TemperatureMeasurement,
            Self::RELATIVE_HUMIDITY_MEASUREMENT => Self::RelativeHumidityMeasurement,
            Self::OCCUPANCY_SENSING => Self::OccupancySensing,
            Self::SWITCH => Self::Switch,
            other => Self::Unknown(other),
        }
    }

    pub fn id(self) -> MatterClusterId {
        match self {
            Self::Descriptor => Self::DESCRIPTOR,
            Self::Groups => Self::GROUPS,
            Self::Scenes => Self::SCENES,
            Self::OnOff => Self::ON_OFF,
            Self::LevelControl => Self::LEVEL_CONTROL,
            Self::DoorLock => Self::DOOR_LOCK,
            Self::Thermostat => Self::THERMOSTAT,
            Self::ColorControl => Self::COLOR_CONTROL,
            Self::IlluminanceMeasurement => Self::ILLUMINANCE_MEASUREMENT,
            Self::TemperatureMeasurement => Self::TEMPERATURE_MEASUREMENT,
            Self::RelativeHumidityMeasurement => Self::RELATIVE_HUMIDITY_MEASUREMENT,
            Self::OccupancySensing => Self::OCCUPANCY_SENSING,
            Self::Switch => Self::SWITCH,
            Self::Unknown(cluster_id) => cluster_id,
        }
    }

    pub fn entity_kind(self) -> Option<EntityKind> {
        match self {
            Self::OnOff | Self::LevelControl | Self::ColorControl => Some(EntityKind::Light),
            Self::DoorLock => Some(EntityKind::Lock),
            Self::Thermostat => Some(EntityKind::Thermostat),
            Self::IlluminanceMeasurement
            | Self::TemperatureMeasurement
            | Self::RelativeHumidityMeasurement
            | Self::OccupancySensing => Some(EntityKind::Sensor),
            Self::Scenes => Some(EntityKind::Scene),
            Self::Switch => Some(EntityKind::Input),
            Self::Descriptor | Self::Groups | Self::Unknown(_) => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatterAttribute {
    OnOff,
    CurrentLevel,
    MeasuredValue,
    Occupancy,
    LockState,
    LocalTemperature,
    OccupiedHeatingSetpoint,
    OccupiedCoolingSetpoint,
    Unknown(MatterAttributeId),
}

impl MatterAttribute {
    pub const ON_OFF: MatterAttributeId = MatterAttributeId::new(0x0000);
    pub const CURRENT_LEVEL: MatterAttributeId = MatterAttributeId::new(0x0000);
    pub const MEASURED_VALUE: MatterAttributeId = MatterAttributeId::new(0x0000);
    pub const OCCUPANCY: MatterAttributeId = MatterAttributeId::new(0x0000);
    pub const LOCK_STATE: MatterAttributeId = MatterAttributeId::new(0x0000);
    pub const LOCAL_TEMPERATURE: MatterAttributeId = MatterAttributeId::new(0x0000);
    pub const OCCUPIED_HEATING_SETPOINT: MatterAttributeId = MatterAttributeId::new(0x0012);
    pub const OCCUPIED_COOLING_SETPOINT: MatterAttributeId = MatterAttributeId::new(0x0011);
}

#[derive(Debug, Clone, PartialEq)]
pub enum MatterValue {
    Null,
    Bool(bool),
    I64(i64),
    U64(u64),
    F64(f64),
    Text(String),
}

impl MatterValue {
    fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            _ => None,
        }
    }

    fn as_u64(&self) -> Option<u64> {
        match self {
            Self::U64(value) => Some(*value),
            Self::I64(value) if *value >= 0 => Some(*value as u64),
            _ => None,
        }
    }

    fn as_i64(&self) -> Option<i64> {
        match self {
            Self::I64(value) => Some(*value),
            Self::U64(value) if *value <= i64::MAX as u64 => Some(*value as i64),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatterAttributeReport {
    pub node_id: MatterNodeId,
    pub endpoint_id: MatterEndpointId,
    pub cluster_id: MatterClusterId,
    pub attribute_id: MatterAttributeId,
    pub value: MatterValue,
}

impl MatterAttributeReport {
    pub fn new(
        node_id: MatterNodeId,
        endpoint_id: MatterEndpointId,
        cluster_id: MatterClusterId,
        attribute_id: MatterAttributeId,
        value: MatterValue,
    ) -> Self {
        Self {
            node_id,
            endpoint_id,
            cluster_id,
            attribute_id,
            value,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatterCommand {
    Off,
    On,
    MoveToLevelWithOnOff,
    MoveToColorTemperature,
    LockDoor,
    UnlockDoor,
}

impl MatterCommand {
    pub const OFF: MatterCommandId = MatterCommandId::new(0x0000);
    pub const ON: MatterCommandId = MatterCommandId::new(0x0001);
    pub const MOVE_TO_LEVEL_WITH_ON_OFF: MatterCommandId = MatterCommandId::new(0x0004);
    pub const MOVE_TO_COLOR_TEMPERATURE: MatterCommandId = MatterCommandId::new(0x000a);
    pub const LOCK_DOOR: MatterCommandId = MatterCommandId::new(0x0000);
    pub const UNLOCK_DOOR: MatterCommandId = MatterCommandId::new(0x0001);

    pub fn cluster_id(self) -> MatterClusterId {
        match self {
            Self::Off | Self::On => MatterCluster::ON_OFF,
            Self::MoveToLevelWithOnOff => MatterCluster::LEVEL_CONTROL,
            Self::MoveToColorTemperature => MatterCluster::COLOR_CONTROL,
            Self::LockDoor | Self::UnlockDoor => MatterCluster::DOOR_LOCK,
        }
    }

    pub fn command_id(self) -> MatterCommandId {
        match self {
            Self::Off => Self::OFF,
            Self::On => Self::ON,
            Self::MoveToLevelWithOnOff => Self::MOVE_TO_LEVEL_WITH_ON_OFF,
            Self::MoveToColorTemperature => Self::MOVE_TO_COLOR_TEMPERATURE,
            Self::LockDoor => Self::LOCK_DOOR,
            Self::UnlockDoor => Self::UNLOCK_DOOR,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatterCommandInvocation {
    pub node_id: MatterNodeId,
    pub endpoint_id: MatterEndpointId,
    pub cluster_id: MatterClusterId,
    pub command_id: MatterCommandId,
    pub arguments: Vec<(String, MatterValue)>,
}

impl MatterCommandInvocation {
    pub fn new(
        node_id: MatterNodeId,
        endpoint_id: MatterEndpointId,
        command: MatterCommand,
        arguments: Vec<(String, MatterValue)>,
    ) -> Self {
        Self {
            node_id,
            endpoint_id,
            cluster_id: command.cluster_id(),
            command_id: command.command_id(),
            arguments,
        }
    }

    pub fn command(
        node_id: MatterNodeId,
        endpoint_id: MatterEndpointId,
        command: MatterCommand,
    ) -> Self {
        Self::new(node_id, endpoint_id, command, Vec::new())
    }

    pub fn argument(&self, name: &str) -> Option<&MatterValue> {
        self.arguments
            .iter()
            .find(|(candidate, _)| candidate == name)
            .map(|(_, value)| value)
    }
}

pub fn matter_command_for_device_command(
    node_id: MatterNodeId,
    endpoint_id: MatterEndpointId,
    command: &DeviceCommand,
) -> Result<MatterCommandInvocation, MatterError> {
    match command.command_type {
        CommandType::TurnOn => Ok(MatterCommandInvocation::command(
            node_id,
            endpoint_id,
            MatterCommand::On,
        )),
        CommandType::TurnOff => Ok(MatterCommandInvocation::command(
            node_id,
            endpoint_id,
            MatterCommand::Off,
        )),
        CommandType::SetBrightness => Ok(MatterCommandInvocation::new(
            node_id,
            endpoint_id,
            MatterCommand::MoveToLevelWithOnOff,
            vec![
                (
                    "level".to_string(),
                    MatterValue::U64(u64::from(percentage_to_level(expect_command_percentage(
                        command,
                    )?))),
                ),
                ("transition_time_ds".to_string(), MatterValue::U64(0)),
            ],
        )),
        CommandType::SetColorTemperature => Ok(MatterCommandInvocation::new(
            node_id,
            endpoint_id,
            MatterCommand::MoveToColorTemperature,
            vec![
                (
                    "color_temperature_mireds".to_string(),
                    MatterValue::U64(u64::from(expect_command_u16(command)?)),
                ),
                ("transition_time_ds".to_string(), MatterValue::U64(0)),
            ],
        )),
        CommandType::SetLock => Ok(MatterCommandInvocation::command(
            node_id,
            endpoint_id,
            expect_lock_command(command)?,
        )),
        CommandType::SetColor | CommandType::RecallScene | CommandType::SetThermostatSetpoint => {
            Err(MatterError::UnsupportedCommand {
                command_type: command.command_type,
            })
        }
    }
}

pub fn percentage_to_level(percentage: u8) -> u8 {
    ((u32::from(percentage) * 254 + 50) / 100) as u8
}

pub fn capabilities_for_cluster(cluster_id: MatterClusterId) -> Vec<Capability> {
    match MatterCluster::from_id(cluster_id) {
        MatterCluster::OnOff => vec![Capability::light_on_off()],
        MatterCluster::LevelControl => vec![Capability::light_brightness()],
        MatterCluster::ColorControl => {
            vec![
                Capability::light_color(),
                Capability::light_color_temperature(),
            ]
        }
        MatterCluster::Scenes => vec![Capability::scene_recall()],
        MatterCluster::DoorLock => vec![Capability::lock_state()],
        MatterCluster::Thermostat => {
            vec![
                Capability::climate_setpoint(),
                Capability::sensor_temperature(),
            ]
        }
        MatterCluster::IlluminanceMeasurement => vec![Capability::sensor_illuminance()],
        MatterCluster::TemperatureMeasurement => vec![Capability::sensor_temperature()],
        MatterCluster::RelativeHumidityMeasurement => vec![Capability::sensor_humidity()],
        MatterCluster::OccupancySensing => vec![Capability::sensor_occupancy()],
        MatterCluster::Switch => vec![Capability::input_button()],
        MatterCluster::Descriptor | MatterCluster::Groups | MatterCluster::Unknown(_) => Vec::new(),
    }
}

pub fn capability_ids_for_cluster(cluster_id: MatterClusterId) -> Vec<CapabilityId> {
    capabilities_for_cluster(cluster_id)
        .into_iter()
        .map(|capability| capability.capability_id)
        .collect()
}

pub fn state_delta_for_attribute_report(
    report: &MatterAttributeReport,
) -> Result<StateDelta, MatterError> {
    match (
        MatterCluster::from_id(report.cluster_id),
        report.attribute_id,
    ) {
        (MatterCluster::OnOff, MatterAttribute::ON_OFF) => Ok(StateDelta {
            capability_id: CapabilityId::trusted("light.on_off"),
            value: Value::Bool(expect_bool(report)?),
        }),
        (MatterCluster::LevelControl, MatterAttribute::CURRENT_LEVEL) => Ok(StateDelta {
            capability_id: CapabilityId::trusted("light.brightness"),
            value: Value::Percentage(level_to_percentage(expect_u16(report)?)?),
        }),
        (MatterCluster::TemperatureMeasurement, MatterAttribute::MEASURED_VALUE) => {
            Ok(StateDelta {
                capability_id: CapabilityId::trusted("sensor.temperature"),
                value: Value::Number(centi_units_to_number(expect_i64(report)?)),
            })
        }
        (MatterCluster::RelativeHumidityMeasurement, MatterAttribute::MEASURED_VALUE) => {
            Ok(StateDelta {
                capability_id: CapabilityId::trusted("sensor.humidity"),
                value: Value::Percentage(hundredths_percent_to_percentage(expect_u32(report)?)?),
            })
        }
        (MatterCluster::OccupancySensing, MatterAttribute::OCCUPANCY) => Ok(StateDelta {
            capability_id: CapabilityId::trusted("sensor.occupancy"),
            value: Value::Bool((expect_u64(report)? & 0x01) != 0),
        }),
        (MatterCluster::DoorLock, MatterAttribute::LOCK_STATE) => Ok(StateDelta {
            capability_id: CapabilityId::trusted("lock.state"),
            value: Value::Text(lock_state_label(expect_u8(report)?).to_string()),
        }),
        (MatterCluster::Thermostat, MatterAttribute::LOCAL_TEMPERATURE) => Ok(StateDelta {
            capability_id: CapabilityId::trusted("sensor.temperature"),
            value: Value::Number(centi_units_to_number(expect_i64(report)?)),
        }),
        (
            MatterCluster::Thermostat,
            MatterAttribute::OCCUPIED_HEATING_SETPOINT | MatterAttribute::OCCUPIED_COOLING_SETPOINT,
        ) => Ok(StateDelta {
            capability_id: CapabilityId::trusted("climate.setpoint"),
            value: Value::Number(centi_units_to_number(expect_i64(report)?)),
        }),
        _ => Err(MatterError::UnsupportedAttribute {
            cluster_id: report.cluster_id,
            attribute_id: report.attribute_id,
        }),
    }
}

pub fn level_to_percentage(level: u16) -> Result<u8, MatterError> {
    if level > 254 {
        return Err(MatterError::InvalidLevel { value: level });
    }
    Ok(((u32::from(level) * 100) / 254) as u8)
}

pub fn hundredths_percent_to_percentage(value: u32) -> Result<u8, MatterError> {
    if value > 10_000 {
        return Err(MatterError::InvalidHumidity { value });
    }
    Ok((value / 100) as u8)
}

pub fn centi_units_to_number(value: i64) -> f64 {
    value as f64 / 100.0
}

pub fn lock_state_label(value: u8) -> &'static str {
    match value {
        0 => "not_fully_locked",
        1 => "locked",
        2 => "unlocked",
        _ => "unknown",
    }
}

fn expect_bool(report: &MatterAttributeReport) -> Result<bool, MatterError> {
    report
        .value
        .as_bool()
        .ok_or_else(|| unexpected(report, "bool"))
}

fn expect_u8(report: &MatterAttributeReport) -> Result<u8, MatterError> {
    let value = expect_u64(report)?;
    if value <= u8::MAX as u64 {
        Ok(value as u8)
    } else {
        Err(unexpected(report, "u8"))
    }
}

fn expect_u16(report: &MatterAttributeReport) -> Result<u16, MatterError> {
    let value = expect_u64(report)?;
    if value <= u16::MAX as u64 {
        Ok(value as u16)
    } else {
        Err(unexpected(report, "u16"))
    }
}

fn expect_u32(report: &MatterAttributeReport) -> Result<u32, MatterError> {
    let value = expect_u64(report)?;
    if value <= u32::MAX as u64 {
        Ok(value as u32)
    } else {
        Err(unexpected(report, "u32"))
    }
}

fn expect_u64(report: &MatterAttributeReport) -> Result<u64, MatterError> {
    report
        .value
        .as_u64()
        .ok_or_else(|| unexpected(report, "u64"))
}

fn expect_i64(report: &MatterAttributeReport) -> Result<i64, MatterError> {
    report
        .value
        .as_i64()
        .ok_or_else(|| unexpected(report, "i64"))
}

fn unexpected(report: &MatterAttributeReport, expected: &'static str) -> MatterError {
    MatterError::UnexpectedValueKind {
        cluster_id: report.cluster_id,
        attribute_id: report.attribute_id,
        expected,
    }
}

fn expect_command_percentage(command: &DeviceCommand) -> Result<u8, MatterError> {
    match &command.arguments {
        Value::Percentage(value) => Ok(*value),
        _ => Err(MatterError::UnexpectedCommandArgument {
            command_type: command.command_type,
            expected: "percentage",
        }),
    }
}

fn expect_command_u16(command: &DeviceCommand) -> Result<u16, MatterError> {
    match &command.arguments {
        Value::Integer(value) if (0..=u16::MAX as i64).contains(value) => Ok(*value as u16),
        _ => Err(MatterError::UnexpectedCommandArgument {
            command_type: command.command_type,
            expected: "u16 integer",
        }),
    }
}

fn expect_lock_command(command: &DeviceCommand) -> Result<MatterCommand, MatterError> {
    match &command.arguments {
        Value::Bool(true) => Ok(MatterCommand::LockDoor),
        Value::Bool(false) => Ok(MatterCommand::UnlockDoor),
        Value::Text(value) if value == "locked" || value == "lock" => Ok(MatterCommand::LockDoor),
        Value::Text(value) if value == "unlocked" || value == "unlock" => {
            Ok(MatterCommand::UnlockDoor)
        }
        _ => Err(MatterError::UnexpectedCommandArgument {
            command_type: command.command_type,
            expected: "lock state boolean or locked/unlocked text",
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smart_home_core::{CommandId, CorrelationId, EntityId};

    fn report(
        cluster_id: MatterClusterId,
        attribute_id: MatterAttributeId,
        value: MatterValue,
    ) -> MatterAttributeReport {
        MatterAttributeReport::new(
            MatterNodeId::new(0x1234),
            MatterEndpointId::new(1).unwrap(),
            cluster_id,
            attribute_id,
            value,
        )
    }

    fn device_command(command_type: CommandType, arguments: Value) -> DeviceCommand {
        DeviceCommand::new(
            CommandId::trusted("cmd-1"),
            EntityId::trusted("entity-1"),
            command_type,
            arguments,
            "agent-1",
            CorrelationId::trusted("corr-1"),
        )
        .unwrap()
    }

    #[test]
    fn endpoint_ids_reject_reserved_values() {
        assert_eq!(
            MatterEndpointId::new(0),
            Err(MatterError::InvalidEndpoint { endpoint_id: 0 })
        );
        assert_eq!(
            MatterEndpointId::new(u16::MAX),
            Err(MatterError::InvalidEndpoint {
                endpoint_id: u16::MAX
            })
        );
        assert_eq!(MatterEndpointId::new(1).unwrap().value(), 1);
    }

    #[test]
    fn cluster_ids_map_to_entity_kinds_and_capabilities() {
        assert_eq!(
            MatterCluster::from_id(MatterCluster::DOOR_LOCK).entity_kind(),
            Some(EntityKind::Lock)
        );
        assert_eq!(
            MatterCluster::from_id(MatterCluster::THERMOSTAT).entity_kind(),
            Some(EntityKind::Thermostat)
        );

        let color_ids = capability_ids_for_cluster(MatterCluster::COLOR_CONTROL);
        assert_eq!(
            color_ids,
            vec![
                CapabilityId::trusted("light.color"),
                CapabilityId::trusted("light.color_temperature")
            ]
        );

        let thermostat_ids = capability_ids_for_cluster(MatterCluster::THERMOSTAT);
        assert!(thermostat_ids.contains(&CapabilityId::trusted("climate.setpoint")));
        assert!(thermostat_ids.contains(&CapabilityId::trusted("sensor.temperature")));
    }

    #[test]
    fn attribute_reports_project_to_d23_state_deltas() {
        let on_off = state_delta_for_attribute_report(&report(
            MatterCluster::ON_OFF,
            MatterAttribute::ON_OFF,
            MatterValue::Bool(true),
        ))
        .unwrap();
        assert_eq!(on_off.capability_id, CapabilityId::trusted("light.on_off"));
        assert_eq!(on_off.value, Value::Bool(true));

        let brightness = state_delta_for_attribute_report(&report(
            MatterCluster::LEVEL_CONTROL,
            MatterAttribute::CURRENT_LEVEL,
            MatterValue::U64(127),
        ))
        .unwrap();
        assert_eq!(
            brightness,
            StateDelta {
                capability_id: CapabilityId::trusted("light.brightness"),
                value: Value::Percentage(50),
            }
        );

        let occupancy = state_delta_for_attribute_report(&report(
            MatterCluster::OCCUPANCY_SENSING,
            MatterAttribute::OCCUPANCY,
            MatterValue::U64(0x01),
        ))
        .unwrap();
        assert_eq!(occupancy.value, Value::Bool(true));
    }

    #[test]
    fn device_commands_project_to_matter_on_off_and_level_commands() {
        let node_id = MatterNodeId::new(0x1234);
        let endpoint_id = MatterEndpointId::new(1).unwrap();

        let on = matter_command_for_device_command(
            node_id,
            endpoint_id,
            &device_command(CommandType::TurnOn, Value::Null),
        )
        .unwrap();
        assert_eq!(on.cluster_id, MatterCluster::ON_OFF);
        assert_eq!(on.command_id, MatterCommand::ON);
        assert!(on.arguments.is_empty());

        let brightness = matter_command_for_device_command(
            node_id,
            endpoint_id,
            &device_command(CommandType::SetBrightness, Value::Percentage(50)),
        )
        .unwrap();
        assert_eq!(brightness.cluster_id, MatterCluster::LEVEL_CONTROL);
        assert_eq!(
            brightness.command_id,
            MatterCommand::MOVE_TO_LEVEL_WITH_ON_OFF
        );
        assert_eq!(brightness.argument("level"), Some(&MatterValue::U64(127)));
        assert_eq!(
            brightness.argument("transition_time_ds"),
            Some(&MatterValue::U64(0))
        );
        assert_eq!(percentage_to_level(100), 254);
    }

    #[test]
    fn device_commands_project_to_matter_lock_and_color_temperature_commands() {
        let node_id = MatterNodeId::new(0x1234);
        let endpoint_id = MatterEndpointId::new(1).unwrap();

        let lock = matter_command_for_device_command(
            node_id,
            endpoint_id,
            &device_command(CommandType::SetLock, Value::Text("locked".to_string())),
        )
        .unwrap();
        assert_eq!(lock.cluster_id, MatterCluster::DOOR_LOCK);
        assert_eq!(lock.command_id, MatterCommand::LOCK_DOOR);

        let unlock = matter_command_for_device_command(
            node_id,
            endpoint_id,
            &device_command(CommandType::SetLock, Value::Bool(false)),
        )
        .unwrap();
        assert_eq!(unlock.command_id, MatterCommand::UNLOCK_DOOR);

        let color_temp = matter_command_for_device_command(
            node_id,
            endpoint_id,
            &device_command(CommandType::SetColorTemperature, Value::Integer(250)),
        )
        .unwrap();
        assert_eq!(color_temp.cluster_id, MatterCluster::COLOR_CONTROL);
        assert_eq!(
            color_temp.command_id,
            MatterCommand::MOVE_TO_COLOR_TEMPERATURE
        );
        assert_eq!(
            color_temp.argument("color_temperature_mireds"),
            Some(&MatterValue::U64(250))
        );
    }

    #[test]
    fn device_command_projection_rejects_unsupported_or_malformed_commands() {
        let node_id = MatterNodeId::new(0x1234);
        let endpoint_id = MatterEndpointId::new(1).unwrap();

        let bad_brightness = matter_command_for_device_command(
            node_id,
            endpoint_id,
            &device_command(CommandType::SetBrightness, Value::Integer(50)),
        )
        .unwrap_err();
        assert!(matches!(
            bad_brightness,
            MatterError::UnexpectedCommandArgument { .. }
        ));

        let unsupported = matter_command_for_device_command(
            node_id,
            endpoint_id,
            &device_command(CommandType::RecallScene, Value::Null),
        )
        .unwrap_err();
        assert_eq!(
            unsupported,
            MatterError::UnsupportedCommand {
                command_type: CommandType::RecallScene
            }
        );
    }

    #[test]
    fn environmental_and_climate_reports_scale_values() {
        let temperature = state_delta_for_attribute_report(&report(
            MatterCluster::TEMPERATURE_MEASUREMENT,
            MatterAttribute::MEASURED_VALUE,
            MatterValue::I64(2_135),
        ))
        .unwrap();
        assert_eq!(temperature.value, Value::Number(21.35));

        let humidity = state_delta_for_attribute_report(&report(
            MatterCluster::RELATIVE_HUMIDITY_MEASUREMENT,
            MatterAttribute::MEASURED_VALUE,
            MatterValue::U64(4_255),
        ))
        .unwrap();
        assert_eq!(humidity.value, Value::Percentage(42));

        let setpoint = state_delta_for_attribute_report(&report(
            MatterCluster::THERMOSTAT,
            MatterAttribute::OCCUPIED_HEATING_SETPOINT,
            MatterValue::I64(1_950),
        ))
        .unwrap();
        assert_eq!(
            setpoint.capability_id,
            CapabilityId::trusted("climate.setpoint")
        );
        assert_eq!(setpoint.value, Value::Number(19.5));
    }

    #[test]
    fn lock_reports_map_to_text_state() {
        let locked = state_delta_for_attribute_report(&report(
            MatterCluster::DOOR_LOCK,
            MatterAttribute::LOCK_STATE,
            MatterValue::U64(1),
        ))
        .unwrap();
        assert_eq!(locked.capability_id, CapabilityId::trusted("lock.state"));
        assert_eq!(locked.value, Value::Text("locked".to_string()));
        assert_eq!(lock_state_label(2), "unlocked");
        assert_eq!(lock_state_label(99), "unknown");
    }

    #[test]
    fn invalid_or_unsupported_reports_are_rejected() {
        let bad_level = state_delta_for_attribute_report(&report(
            MatterCluster::LEVEL_CONTROL,
            MatterAttribute::CURRENT_LEVEL,
            MatterValue::U64(255),
        ))
        .unwrap_err();
        assert_eq!(bad_level, MatterError::InvalidLevel { value: 255 });

        let bad_kind = state_delta_for_attribute_report(&report(
            MatterCluster::ON_OFF,
            MatterAttribute::ON_OFF,
            MatterValue::Text("true".to_string()),
        ))
        .unwrap_err();
        assert!(matches!(bad_kind, MatterError::UnexpectedValueKind { .. }));

        let unsupported = state_delta_for_attribute_report(&report(
            MatterCluster::GROUPS,
            MatterAttributeId::new(0x0000),
            MatterValue::Null,
        ))
        .unwrap_err();
        assert!(matches!(
            unsupported,
            MatterError::UnsupportedAttribute { .. }
        ));
    }
}
