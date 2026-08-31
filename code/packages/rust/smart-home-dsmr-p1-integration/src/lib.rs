//! Bounded supervised DSMR P1 serial telemetry integration for D23.

#![forbid(unsafe_code)]

use dsmr_p1_protocol::{parse_telegram_with_limit, DsmrP1Error, DsmrP1Telegram};
use smart_home_core::{
    AgentId, Bridge, BridgeId, BridgeTransport, Capability, CapabilityId, CapabilityMode, Device,
    DeviceId, Entity, EntityId, EntityKind, EventId, Health, IntegrationId, Metadata,
    ProtocolFamily, ProtocolIdentifier, SmartHomeTool, StateConfidence, StateSnapshot, StateSource,
    Value, ValueKind,
};
use smart_home_event_streams::{
    EventStreamCheckpoint, EventStreamSpec, EventStreamState, EventStreamTransport,
};
use smart_home_runtime::{RuntimeError, SmartHomeRuntime};
use std::fmt;
use std::io::{self, Read};
use std::time::Duration;

pub const VERSION: &str = "0.1.0";
pub const INTEGRATION_ID: &str = "dsmr_p1";
pub const PROTOCOL_ID: &str = "dsmr_p1_5_0_2";
pub const SERIAL_BAUD_RATE: u32 = 115_200;
pub const DEFAULT_MAX_TELEGRAM_BYTES: usize = 8 * 1024;
pub const DEFAULT_STALE_AFTER_MS: u64 = 5_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DsmrP1Config {
    pub bridge_id: BridgeId,
    pub serial_path: String,
    pub display_name: String,
    pub timeout: Duration,
    pub max_telegram_bytes: usize,
    pub stale_after_ms: u64,
}

impl DsmrP1Config {
    pub fn new(
        bridge_id: BridgeId,
        serial_path: impl Into<String>,
    ) -> Result<Self, DsmrP1IntegrationError> {
        let config = Self {
            bridge_id,
            serial_path: serial_path.into(),
            display_name: "DSMR P1 Meter".to_string(),
            timeout: Duration::from_secs(3),
            max_telegram_bytes: DEFAULT_MAX_TELEGRAM_BYTES,
            stale_after_ms: DEFAULT_STALE_AFTER_MS,
        };
        config.validate()?;
        Ok(config)
    }

    pub fn with_display_name(mut self, display_name: impl Into<String>) -> Self {
        let display_name = display_name.into();
        if !display_name.trim().is_empty() {
            self.display_name = display_name;
        }
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout.max(Duration::from_millis(1));
        self
    }

    pub fn with_max_telegram_bytes(
        mut self,
        max_telegram_bytes: usize,
    ) -> Result<Self, DsmrP1IntegrationError> {
        self.max_telegram_bytes = max_telegram_bytes;
        self.validate()?;
        Ok(self)
    }

    fn validate(&self) -> Result<(), DsmrP1IntegrationError> {
        if self.serial_path.trim().is_empty()
            || self.serial_path.len() > 512
            || self.serial_path.contains(['\r', '\n', '\0'])
        {
            return Err(DsmrP1IntegrationError::Validation(
                "serial path must be explicit, non-empty, and at most 512 safe characters"
                    .to_string(),
            ));
        }
        if !(256..=64 * 1024).contains(&self.max_telegram_bytes) {
            return Err(DsmrP1IntegrationError::Validation(
                "maximum telegram size must be between 256 and 65536 bytes".to_string(),
            ));
        }
        Ok(())
    }

    fn stream_spec(&self) -> EventStreamSpec {
        EventStreamSpec::new(
            IntegrationId::trusted(INTEGRATION_ID),
            self.bridge_id.clone(),
            EventStreamTransport::SerialFrames,
        )
        .with_endpoint(self.serial_path.clone())
        .with_heartbeat_timeout(self.stale_after_ms)
        .with_stale_after(self.stale_after_ms)
        .with_metadata(Metadata::new(
            "dsmr.serial_baud",
            SERIAL_BAUD_RATE.to_string(),
        ))
        .with_metadata(Metadata::new("dsmr.serial_format", "8N1"))
    }
}

#[derive(Debug)]
pub enum DsmrP1IntegrationError {
    Validation(String),
    Open(String),
    Io(io::Error),
    TruncatedTelegram,
    Protocol(DsmrP1Error),
    Runtime(RuntimeError),
}

impl fmt::Display for DsmrP1IntegrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => write!(formatter, "invalid DSMR P1 input: {message}"),
            Self::Open(message) => {
                write!(formatter, "failed to open DSMR P1 serial port: {message}")
            }
            Self::Io(error) => write!(formatter, "DSMR P1 serial I/O failed: {error}"),
            Self::TruncatedTelegram => {
                formatter.write_str("DSMR P1 serial stream ended before one complete telegram")
            }
            Self::Protocol(error) => error.fmt(formatter),
            Self::Runtime(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for DsmrP1IntegrationError {}

impl From<io::Error> for DsmrP1IntegrationError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<DsmrP1Error> for DsmrP1IntegrationError {
    fn from(error: DsmrP1Error) -> Self {
        Self::Protocol(error)
    }
}

impl From<RuntimeError> for DsmrP1IntegrationError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

pub trait DsmrP1PortOpener {
    fn open(&mut self, config: &DsmrP1Config) -> Result<Box<dyn Read>, DsmrP1IntegrationError>;
}

#[derive(Debug, Default)]
pub struct SerialPortOpener;

impl DsmrP1PortOpener for SerialPortOpener {
    fn open(&mut self, config: &DsmrP1Config) -> Result<Box<dyn Read>, DsmrP1IntegrationError> {
        config.validate()?;
        let port = serialport::new(&config.serial_path, SERIAL_BAUD_RATE)
            .timeout(config.timeout)
            .data_bits(serialport::DataBits::Eight)
            .flow_control(serialport::FlowControl::None)
            .parity(serialport::Parity::None)
            .stop_bits(serialport::StopBits::One)
            .open()
            .map_err(|error| DsmrP1IntegrationError::Open(error.to_string()))?;
        Ok(Box::new(SerialReader(port)))
    }
}

struct SerialReader(Box<dyn serialport::SerialPort>);

impl Read for SerialReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.0.read(buffer)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct InstalledDsmrP1Meter {
    pub bridge_id: BridgeId,
    pub device_id: DeviceId,
    pub entity_ids: Vec<EntityId>,
    pub checkpoint: EventStreamCheckpoint,
}

pub struct DsmrP1StreamSupervisor<O> {
    config: DsmrP1Config,
    opener: O,
    stream_state: EventStreamState,
}

impl<O: DsmrP1PortOpener> DsmrP1StreamSupervisor<O> {
    pub fn new(config: DsmrP1Config, opener: O, now_ms: u64) -> Self {
        let stream_state = EventStreamState::new(config.stream_spec(), now_ms);
        Self {
            config,
            opener,
            stream_state,
        }
    }

    pub fn config(&self) -> &DsmrP1Config {
        &self.config
    }

    pub fn stream_state(&self) -> &EventStreamState {
        &self.stream_state
    }

    pub fn opener(&self) -> &O {
        &self.opener
    }

    pub fn sample_and_install_authorized(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        principal_id: AgentId,
        observed_at_ms: u64,
    ) -> Result<InstalledDsmrP1Meter, DsmrP1IntegrationError> {
        authorize_read(runtime, principal_id, observed_at_ms)?;
        self.stream_state.mark_connecting();
        let mut reader = match self.opener.open(&self.config) {
            Ok(reader) => reader,
            Err(error) => {
                self.stream_state.mark_disconnected(observed_at_ms);
                return Err(error);
            }
        };
        self.stream_state.mark_connected(observed_at_ms);
        let result =
            read_one_telegram(&mut reader, self.config.max_telegram_bytes).and_then(|bytes| {
                parse_telegram_with_limit(&bytes, self.config.max_telegram_bytes)
                    .map_err(Into::into)
            });
        let telegram = match result {
            Ok(telegram) => telegram,
            Err(error) => {
                self.stream_state.mark_disconnected(observed_at_ms);
                return Err(error);
            }
        };
        let checkpoint = self.stream_state.record_event(
            EventId::trusted(format!("dsmr:{}", telegram.timestamp)),
            Some(telegram.timestamp.clone()),
            observed_at_ms,
        );
        install_telegram(runtime, &self.config, &telegram, checkpoint, observed_at_ms)
    }
}

pub fn read_one_telegram(
    reader: &mut dyn Read,
    max_bytes: usize,
) -> Result<Vec<u8>, DsmrP1IntegrationError> {
    if !(256..=64 * 1024).contains(&max_bytes) {
        return Err(DsmrP1IntegrationError::Validation(
            "maximum telegram size must be between 256 and 65536 bytes".to_string(),
        ));
    }
    let mut frame = Vec::with_capacity(max_bytes.min(4096));
    let mut scanned = 0_usize;
    let mut byte = [0_u8; 1];
    loop {
        let read = reader.read(&mut byte)?;
        if read == 0 {
            return Err(DsmrP1IntegrationError::TruncatedTelegram);
        }
        scanned = scanned.saturating_add(1);
        if frame.is_empty() && byte[0] != b'/' {
            if scanned >= max_bytes {
                return Err(DsmrP1IntegrationError::Protocol(DsmrP1Error::TooLarge {
                    limit: max_bytes,
                }));
            }
            continue;
        }
        frame.push(byte[0]);
        if frame.len() > max_bytes {
            return Err(DsmrP1IntegrationError::Protocol(DsmrP1Error::TooLarge {
                limit: max_bytes,
            }));
        }
        if frame.len() >= 7 {
            let tail = &frame[frame.len() - 7..];
            if tail[0] == b'!'
                && tail[1..5].iter().all(u8::is_ascii_hexdigit)
                && &tail[5..] == b"\r\n"
            {
                return Ok(frame);
            }
        }
    }
}

fn install_telegram(
    runtime: &mut SmartHomeRuntime,
    config: &DsmrP1Config,
    telegram: &DsmrP1Telegram,
    checkpoint: EventStreamCheckpoint,
    observed_at_ms: u64,
) -> Result<InstalledDsmrP1Meter, DsmrP1IntegrationError> {
    let native_id = stable_component(&telegram.equipment_id);
    if native_id.is_empty() {
        return Err(DsmrP1IntegrationError::Validation(
            "equipment identifier does not contain a stable component".to_string(),
        ));
    }
    let device_id = DeviceId::trusted(format!("dsmr:{native_id}"));
    let mut bridge = Bridge::new(
        config.bridge_id.clone(),
        IntegrationId::trusted(INTEGRATION_ID),
        BridgeTransport::Serial,
    );
    bridge.address = Some(config.serial_path.clone());
    bridge.hardware_model = Some("DSMR P1".to_string());
    bridge.firmware_version = Some(telegram.version.clone());
    bridge.health = Health::Online;
    bridge.last_seen_at_ms = Some(observed_at_ms);
    bridge.identifiers = vec![protocol_identifier("serial_path", &config.serial_path)?];
    bridge.metadata = vec![
        Metadata::new("dsmr.transport", "serial_frames"),
        Metadata::new("dsmr.serial_baud", SERIAL_BAUD_RATE.to_string()),
        Metadata::new("dsmr.serial_format", "8N1"),
    ];
    runtime.upsert_bridge(bridge)?;

    let measurements = measurements(telegram);
    let entities = measurements
        .into_iter()
        .map(|measurement| {
            let entity_id =
                EntityId::trusted(format!("dsmr:{native_id}:sensor:{}", measurement.id));
            Entity {
                entity_id: entity_id.clone(),
                device_id: device_id.clone(),
                kind: EntityKind::Sensor,
                name: format!("{} {}", config.display_name, measurement.name),
                capabilities: vec![Capability::new(
                    CapabilityId::trusted("sensor.measurement"),
                    CapabilityMode::Observe,
                    ValueKind::Object,
                )],
                state: Some(StateSnapshot {
                    entity_id,
                    value: Value::Object(vec![
                        ("value".to_string(), measurement.value),
                        (
                            "unit".to_string(),
                            Value::Text(measurement.unit.to_string()),
                        ),
                    ]),
                    source: StateSource::EventStream,
                    observed_at_ms,
                    received_at_ms: observed_at_ms,
                    expires_at_ms: Some(observed_at_ms.saturating_add(config.stale_after_ms)),
                    confidence: StateConfidence::Confirmed,
                }),
                metadata: vec![
                    Metadata::new("dsmr.obis", measurement.obis),
                    Metadata::new("dsmr.telegram_timestamp", telegram.timestamp.clone()),
                ],
            }
        })
        .collect::<Vec<_>>();
    let entity_ids = entities
        .iter()
        .map(|entity| entity.entity_id.clone())
        .collect::<Vec<_>>();
    runtime.upsert_device(Device {
        device_id: device_id.clone(),
        bridge_id: config.bridge_id.clone(),
        manufacturer: telegram.header.chars().take(3).collect(),
        model: telegram
            .header
            .chars()
            .skip(3)
            .collect::<String>()
            .trim()
            .to_string(),
        name: config.display_name.clone(),
        serial: Some(telegram.equipment_id.clone()),
        firmware_version: Some(telegram.version.clone()),
        room_id: None,
        entity_ids: entity_ids.clone(),
        identifiers: vec![protocol_identifier("equipment_id", &telegram.equipment_id)?],
        health: Health::Online,
        metadata: vec![
            Metadata::new("dsmr.output_version", telegram.version.clone()),
            Metadata::new("dsmr.last_timestamp", telegram.timestamp.clone()),
        ],
    })?;
    for entity in entities {
        runtime.upsert_entity(entity)?;
    }
    Ok(InstalledDsmrP1Meter {
        bridge_id: config.bridge_id.clone(),
        device_id,
        entity_ids,
        checkpoint,
    })
}

struct Measurement {
    id: String,
    name: String,
    obis: String,
    value: Value,
    unit: &'static str,
}

fn measurements(telegram: &DsmrP1Telegram) -> Vec<Measurement> {
    let mut values = vec![
        number_measurement(
            "electricity-import-tariff-1",
            "Electricity import tariff 1",
            "1-0:1.8.1",
            telegram.electricity_import_tariff_1_kwh,
            "kWh",
        ),
        number_measurement(
            "electricity-import-tariff-2",
            "Electricity import tariff 2",
            "1-0:1.8.2",
            telegram.electricity_import_tariff_2_kwh,
            "kWh",
        ),
        number_measurement(
            "electricity-export-tariff-1",
            "Electricity export tariff 1",
            "1-0:2.8.1",
            telegram.electricity_export_tariff_1_kwh,
            "kWh",
        ),
        number_measurement(
            "electricity-export-tariff-2",
            "Electricity export tariff 2",
            "1-0:2.8.2",
            telegram.electricity_export_tariff_2_kwh,
            "kWh",
        ),
        Measurement {
            id: "active-tariff".to_string(),
            name: "Active tariff".to_string(),
            obis: "0-0:96.14.0".to_string(),
            value: Value::Text(telegram.active_tariff.clone()),
            unit: "tariff",
        },
        number_measurement(
            "electricity-import-power",
            "Electricity import power",
            "1-0:1.7.0",
            telegram.electricity_import_kw,
            "kW",
        ),
        number_measurement(
            "electricity-export-power",
            "Electricity export power",
            "1-0:2.7.0",
            telegram.electricity_export_kw,
            "kW",
        ),
    ];
    append_phases(
        &mut values,
        "voltage",
        "Voltage",
        ["1-0:32.7.0", "1-0:52.7.0", "1-0:72.7.0"],
        telegram.phase_voltage_v,
        "V",
    );
    append_phases(
        &mut values,
        "current",
        "Current",
        ["1-0:31.7.0", "1-0:51.7.0", "1-0:71.7.0"],
        telegram.phase_current_a,
        "A",
    );
    append_phases(
        &mut values,
        "import-power",
        "Import power",
        ["1-0:21.7.0", "1-0:41.7.0", "1-0:61.7.0"],
        telegram.phase_import_kw,
        "kW",
    );
    append_phases(
        &mut values,
        "export-power",
        "Export power",
        ["1-0:22.7.0", "1-0:42.7.0", "1-0:62.7.0"],
        telegram.phase_export_kw,
        "kW",
    );
    if let Some(gas) = &telegram.gas {
        values.push(number_measurement(
            "gas-delivered",
            "Gas delivered",
            "0-n:24.2.1",
            gas.cubic_metres,
            "m3",
        ));
    }
    values
}

fn append_phases(
    output: &mut Vec<Measurement>,
    id: &str,
    name: &str,
    obis: [&str; 3],
    values: [Option<f64>; 3],
    unit: &'static str,
) {
    for (index, value) in values.into_iter().enumerate() {
        if let Some(value) = value {
            output.push(number_measurement(
                &format!("{id}-l{}", index + 1),
                &format!("{name} L{}", index + 1),
                obis[index],
                value,
                unit,
            ));
        }
    }
}

fn number_measurement(
    id: &str,
    name: &str,
    obis: &str,
    value: f64,
    unit: &'static str,
) -> Measurement {
    Measurement {
        id: id.to_string(),
        name: name.to_string(),
        obis: obis.to_string(),
        value: Value::Number(value),
        unit,
    }
}

fn authorize_read(
    runtime: &mut SmartHomeRuntime,
    principal_id: AgentId,
    now_ms: u64,
) -> Result<(), DsmrP1IntegrationError> {
    let tool = SmartHomeTool::GetState;
    let decision = runtime.authorize_tool_for_principal(principal_id.clone(), tool, now_ms);
    if decision.missing_capabilities.is_empty() {
        Ok(())
    } else {
        Err(DsmrP1IntegrationError::Runtime(
            RuntimeError::UnauthorizedTool {
                principal_id,
                tool,
                missing_capabilities: decision.missing_capabilities,
            },
        ))
    }
}

fn protocol_identifier(
    kind: &str,
    value: &str,
) -> Result<ProtocolIdentifier, DsmrP1IntegrationError> {
    ProtocolIdentifier::new(ProtocolFamily::Vendor(PROTOCOL_ID.to_string()), kind, value)
        .map_err(|error| DsmrP1IntegrationError::Validation(error.to_string()))
}

fn stable_component(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use dsmr_p1_protocol::crc16;
    use smart_home_core::{CapabilityGrant, CapabilityGrantId, PrivilegeTier};
    use smart_home_event_streams::EventStreamStatus;
    use std::cell::Cell;
    use std::io::Cursor;
    use std::rc::Rc;

    fn telegram() -> Vec<u8> {
        let body = "/ISK5\\2MT382-1000\r\n\r\n1-3:0.2.8(50)\r\n0-0:1.0.0(101209113020W)\r\n0-0:96.1.1(4B384547303034303436333935353037)\r\n1-0:1.8.1(123456.789*kWh)\r\n1-0:1.8.2(123456.790*kWh)\r\n1-0:2.8.1(000001.001*kWh)\r\n1-0:2.8.2(000002.002*kWh)\r\n0-0:96.14.0(0002)\r\n1-0:1.7.0(01.193*kW)\r\n1-0:2.7.0(00.000*kW)\r\n1-0:32.7.0(220.1*V)\r\n1-0:31.7.0(001*A)\r\n0-1:24.2.1(101209112500W)(12785.123*m3)\r\n";
        let mut bytes = body.as_bytes().to_vec();
        bytes.push(b'!');
        let checksum = crc16(&bytes);
        bytes.extend_from_slice(format!("{checksum:04X}\r\n").as_bytes());
        bytes
    }

    #[derive(Clone)]
    struct FakeOpener {
        calls: Rc<Cell<usize>>,
        bytes: Vec<u8>,
    }

    impl DsmrP1PortOpener for FakeOpener {
        fn open(
            &mut self,
            _config: &DsmrP1Config,
        ) -> Result<Box<dyn Read>, DsmrP1IntegrationError> {
            self.calls.set(self.calls.get() + 1);
            Ok(Box::new(Cursor::new(self.bytes.clone())))
        }
    }

    fn config() -> DsmrP1Config {
        DsmrP1Config::new(BridgeId::trusted("dsmr.test"), "/dev/ttyUSB0")
            .unwrap()
            .with_display_name("Utility Meter")
    }

    fn grant(runtime: &mut SmartHomeRuntime, principal: &AgentId) {
        let _ = runtime.registry_mut().upsert_capability_grant(
            CapabilityGrant::for_all_smart_home(
                CapabilityGrantId::trusted("grant:dsmr-test"),
                principal.clone(),
                PrivilegeTier::LowRisk,
                "test",
                1_000,
            )
            .with_expiry(20_000),
        );
    }

    #[test]
    fn bounded_reader_resynchronizes_and_returns_one_exact_frame() {
        let expected = telegram();
        let mut bytes = b"discarded noise".to_vec();
        bytes.extend_from_slice(&expected);
        bytes.extend_from_slice(b"second frame is untouched");
        let actual = read_one_telegram(&mut Cursor::new(bytes), 4096).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn authorization_precedes_serial_open() {
        let calls = Rc::new(Cell::new(0));
        let opener = FakeOpener {
            calls: Rc::clone(&calls),
            bytes: telegram(),
        };
        let mut supervisor = DsmrP1StreamSupervisor::new(config(), opener, 1_000);
        let result = supervisor.sample_and_install_authorized(
            &mut SmartHomeRuntime::new(),
            AgentId::trusted("agent:denied"),
            5_000,
        );
        assert!(matches!(result, Err(DsmrP1IntegrationError::Runtime(_))));
        assert_eq!(calls.get(), 0);
        assert_eq!(supervisor.stream_state().status, EventStreamStatus::Idle);
    }

    #[test]
    fn supervised_sample_installs_event_stream_measurements_without_raw_frame() {
        let calls = Rc::new(Cell::new(0));
        let opener = FakeOpener {
            calls: Rc::clone(&calls),
            bytes: telegram(),
        };
        let mut supervisor = DsmrP1StreamSupervisor::new(config(), opener, 1_000);
        let principal = AgentId::trusted("agent:dsmr-read");
        let mut runtime = SmartHomeRuntime::new();
        grant(&mut runtime, &principal);
        let installed = supervisor
            .sample_and_install_authorized(&mut runtime, principal, 5_000)
            .unwrap();
        assert_eq!(calls.get(), 1);
        assert_eq!(supervisor.stream_state().status, EventStreamStatus::Healthy);
        assert_eq!(installed.checkpoint.cursor.sequence, 1);
        assert_eq!(installed.entity_ids.len(), 10);
        let bridge = runtime.registry().bridge(&installed.bridge_id).unwrap();
        assert_eq!(bridge.transport, BridgeTransport::Serial);
        assert_eq!(bridge.address.as_deref(), Some("/dev/ttyUSB0"));
        let gas = runtime
            .registry()
            .entity(&EntityId::trusted(format!(
                "{}:sensor:gas-delivered",
                installed.device_id.as_str()
            )))
            .unwrap();
        assert_eq!(gas.state.as_ref().unwrap().source, StateSource::EventStream);
        assert_eq!(gas.state.as_ref().unwrap().expires_at_ms, Some(10_000));
        let debug = format!("{gas:?}");
        assert!(!debug.contains("ISK5"));
        assert!(!debug.contains("EF2F"));
    }

    #[test]
    fn malformed_frame_disconnects_supervision() {
        let opener = FakeOpener {
            calls: Rc::new(Cell::new(0)),
            bytes: b"/bad\r\n".to_vec(),
        };
        let mut supervisor = DsmrP1StreamSupervisor::new(config(), opener, 1_000);
        let principal = AgentId::trusted("agent:dsmr-read");
        let mut runtime = SmartHomeRuntime::new();
        grant(&mut runtime, &principal);
        assert!(supervisor
            .sample_and_install_authorized(&mut runtime, principal, 5_000)
            .is_err());
        assert_eq!(
            supervisor.stream_state().status,
            EventStreamStatus::Disconnected
        );
        assert!(supervisor.stream_state().restart_plan_at(5_000).is_some());
    }

    #[test]
    fn config_rejects_unsafe_paths_and_unbounded_frames() {
        assert!(DsmrP1Config::new(BridgeId::trusted("dsmr.test"), "\n").is_err());
        assert!(config().with_max_telegram_bytes(128).is_err());
    }
}
