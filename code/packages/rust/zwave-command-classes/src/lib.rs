//! Z-Wave command class value and D23 mapping primitives.
//!
//! This crate owns command-class payload semantics without controller I/O,
//! inclusion state, or security. It turns command-class reports into typed
//! values and the first normalized smart-home capability/state deltas.

#![forbid(unsafe_code)]

use smart_home_core::{Capability, CapabilityId, CapabilityMode, StateDelta, Value, ValueKind};
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
pub const SENSOR_BINARY_REPORT: u8 = 0x03;
pub const SENSOR_MULTILEVEL_GET: u8 = 0x04;
pub const SENSOR_MULTILEVEL_REPORT: u8 = 0x05;
pub const DOOR_LOCK_OPERATION_SET: u8 = 0x01;
pub const DOOR_LOCK_OPERATION_GET: u8 = 0x02;
pub const DOOR_LOCK_OPERATION_REPORT: u8 = 0x03;

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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoorLockMode {
    Unsecured,
    Secured,
    Unknown(u8),
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
    InvalidSensorValueSize(u8),
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
            Self::InvalidSensorValueSize(size) => {
                write!(f, "invalid Z-Wave multilevel sensor value size {size}")
            }
        }
    }
}

impl std::error::Error for CommandClassError {}

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
        _ => Err(CommandClassError::UnsupportedReport {
            command_class: command.command_class,
            command_id: command.command_id,
        }),
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
    fn set_builders_normalize_values() {
        assert_eq!(binary_switch_set(false).payload, vec![0x00]);
        assert_eq!(multilevel_switch_set(100).payload, vec![99]);
        assert_eq!(door_lock_operation_set(true).payload, vec![0xff]);
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
    fn maps_reports_to_d23_state_deltas() {
        let lock = ZWaveValueReport::DoorLock {
            mode: DoorLockMode::Secured,
        };
        let dimmer = ZWaveValueReport::MultilevelSwitch {
            current_level: 99,
            target_level: None,
            duration: None,
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
        assert!(capabilities_for_command_class(CommandClassId::BASIC).is_empty());
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
    fn invalid_extended_command_class_ids_are_rejected() {
        let command = ZWaveCommand::new(CommandClassId(0x0101), 0x01, Vec::new());

        assert_eq!(
            command.encode(),
            Err(CommandClassError::InvalidExtendedCommandClassId(0x0101))
        );
    }
}
