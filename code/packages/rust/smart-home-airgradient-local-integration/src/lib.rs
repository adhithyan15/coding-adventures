//! AirGradient local monitor telemetry integration for D23.

#![forbid(unsafe_code)]

use http1::{parse_response_head, Http1ParseError};
use http_core::BodyKind;
use serde_json::{Map as JsonMap, Value as JsonValue};
use smart_home_core::{
    AgentId, Bridge, BridgeId, BridgeTransport, Capability, CapabilityId, CapabilityMode, Device,
    CommandResult, CommandType, DeviceControlCommandType, DeviceId, Entity, EntityId, EntityKind,
    Health, IntegrationId, Metadata, ProtocolFamily, ProtocolIdentifier, SmartHomeTool,
    StateConfidence, StateSnapshot, StateSource, Value, ValueKind,
};
use smart_home_discovery::{
    DiscoveryConfidence, DiscoveryRecord, DiscoverySource, PairingRequirement,
};
use smart_home_local_http::{
    LocalHttpEndpoint, LocalHttpError, LocalHttpMethod, LocalHttpRequestPlan,
    LocalHttpRequestTemplate, LocalHttpScheme,
};
use smart_home_runtime::{RuntimeCommandToolRequest, RuntimeError, SmartHomeRuntime};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io::{self, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;
use url_parser::{Url, UrlError};

pub const VERSION: &str = "0.2.0";
pub const INTEGRATION_ID: &str = "airgradient";
pub const PROTOCOL_ID: &str = "airgradient_local_api";
pub const MEASUREMENT_PATH: &str = "/measures/current";
pub const CONFIGURATION_PATH: &str = "/config";
pub const DEFAULT_PORT: u16 = 80;
pub const DEFAULT_MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug)]
pub enum AirGradientError {
    Validation(String),
    LocalHttp(LocalHttpError),
    Url(UrlError),
    Io(String),
    Http(String),
    HttpStatus(u16),
    ResponseTooLarge { limit: usize },
    TruncatedBody { expected: usize, actual: usize },
    Json(serde_json::Error),
    MissingField(&'static str),
    NoMeasurements,
    UnknownEntity(EntityId),
    UnsupportedCommand(CommandType),
    InvalidCommandArguments {
        command_type: CommandType,
        expected: &'static str,
    },
    CloudConfigurationConflict,
    VerificationFailed(&'static str),
    Runtime(RuntimeError),
}

impl fmt::Display for AirGradientError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => write!(formatter, "invalid AirGradient input: {message}"),
            Self::LocalHttp(error) => error.fmt(formatter),
            Self::Url(error) => write!(formatter, "invalid AirGradient URL: {error}"),
            Self::Io(message) => write!(formatter, "AirGradient LAN I/O failed: {message}"),
            Self::Http(message) => {
                write!(formatter, "invalid AirGradient HTTP response: {message}")
            }
            Self::HttpStatus(status) => {
                write!(formatter, "AirGradient endpoint returned HTTP {status}")
            }
            Self::ResponseTooLarge { limit } => {
                write!(formatter, "AirGradient response exceeds {limit} bytes")
            }
            Self::TruncatedBody { expected, actual } => write!(
                formatter,
                "AirGradient response is truncated: expected {expected} bytes, got {actual}"
            ),
            Self::Json(error) => write!(formatter, "invalid AirGradient JSON: {error}"),
            Self::MissingField(field) => {
                write!(formatter, "AirGradient response is missing {field}")
            }
            Self::NoMeasurements => {
                formatter.write_str("AirGradient response contains no numeric measurements")
            }
            Self::UnknownEntity(entity_id) => {
                write!(formatter, "unknown AirGradient entity {entity_id}")
            }
            Self::UnsupportedCommand(command_type) => {
                write!(formatter, "unsupported AirGradient command {command_type:?}")
            }
            Self::InvalidCommandArguments {
                command_type,
                expected,
            } => write!(
                formatter,
                "invalid arguments for AirGradient command {command_type:?}; expected {expected}"
            ),
            Self::CloudConfigurationConflict => formatter.write_str(
                "AirGradient configurationControl=cloud rejects local configuration; changing it to local requires a factory reset"
            ),
            Self::VerificationFailed(field) => {
                write!(formatter, "AirGradient did not confirm updated {field}")
            }
            Self::Runtime(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for AirGradientError {}

impl From<LocalHttpError> for AirGradientError {
    fn from(error: LocalHttpError) -> Self {
        Self::LocalHttp(error)
    }
}

impl From<UrlError> for AirGradientError {
    fn from(error: UrlError) -> Self {
        Self::Url(error)
    }
}

impl From<serde_json::Error> for AirGradientError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<RuntimeError> for AirGradientError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AirGradientConfig {
    pub bridge_id: BridgeId,
    pub base_url: String,
    pub display_name: String,
    pub expected_serial: Option<String>,
    pub timeout: Duration,
}

impl AirGradientConfig {
    pub fn new(bridge_id: BridgeId, base_url: impl Into<String>) -> Result<Self, AirGradientError> {
        let base_url = base_url.into().trim_end_matches('/').to_string();
        let parsed = Url::parse(&base_url)?;
        if parsed.scheme != "http"
            || parsed.host.is_none()
            || parsed.userinfo.is_some()
            || parsed.query.is_some()
            || parsed.fragment.is_some()
            || !matches!(parsed.path.as_str(), "" | "/")
        {
            return Err(AirGradientError::Validation(
                "base URL must be a credential-free HTTP origin".to_string(),
            ));
        }
        Ok(Self {
            bridge_id,
            base_url,
            display_name: "AirGradient monitor".to_string(),
            expected_serial: None,
            timeout: Duration::from_secs(5),
        })
    }

    pub fn from_serial(
        bridge_id: BridgeId,
        serial: impl Into<String>,
    ) -> Result<Self, AirGradientError> {
        let serial = validate_serial(serial.into())?;
        let mut config = Self::new(bridge_id, format!("http://airgradient_{serial}.local"))?;
        config.expected_serial = Some(serial);
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

    fn endpoint(&self) -> Result<LocalHttpEndpoint, AirGradientError> {
        let parsed = Url::parse(&self.base_url)?;
        let host = parsed
            .host
            .as_deref()
            .ok_or(AirGradientError::MissingField("base URL host"))?;
        Ok(LocalHttpEndpoint::new(
            IntegrationId::trusted(INTEGRATION_ID),
            self.bridge_id.clone(),
            LocalHttpScheme::Http,
            host.to_string(),
        )?
        .with_port(parsed.port.unwrap_or(DEFAULT_PORT))
        .with_metadata(Metadata::new("http.profile", "airgradient.local-api")))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AirGradientMeasurement {
    pub id: String,
    pub name: String,
    pub value: f64,
    pub unit: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AirGradientDeviceInfo {
    pub serial: String,
    pub model: String,
    pub firmware: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AirGradientSnapshot {
    pub device_info: AirGradientDeviceInfo,
    pub measurements: Vec<AirGradientMeasurement>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AirGradientConfigurationControl {
    Local,
    Both,
    Cloud,
}

impl AirGradientConfigurationControl {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Both => "both",
            Self::Cloud => "cloud",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AirGradientConfiguration {
    pub control: AirGradientConfigurationControl,
    pub led_bar_mode: String,
    pub led_bar_brightness: u8,
    pub display_brightness: u8,
}

pub fn discovery_record(
    config: &AirGradientConfig,
    snapshot: &AirGradientSnapshot,
    discovered_at_ms: u64,
) -> Result<DiscoveryRecord, AirGradientError> {
    let endpoint = config.endpoint()?;
    let host = Url::parse(&config.base_url)?
        .host
        .ok_or(AirGradientError::MissingField("base URL host"))?;
    let source = if host.to_ascii_lowercase().ends_with(".local") {
        DiscoverySource::Mdns
    } else {
        DiscoverySource::Manual
    };
    Ok(DiscoveryRecord::new(
        IntegrationId::trusted(INTEGRATION_ID),
        ProtocolFamily::Vendor(PROTOCOL_ID.to_string()),
        stable_component(&snapshot.device_info.serial),
        source,
        BridgeTransport::LanHttp,
        discovered_at_ms,
    )
    .map_err(|error| AirGradientError::Validation(error.to_string()))?
    .with_display_name(&config.display_name)
    .with_address(endpoint.origin())
    .with_hardware_model(&snapshot.device_info.model)
    .with_confidence(DiscoveryConfidence::Verified)
    .with_pairing_requirement(PairingRequirement::None)
    .with_metadata("airgradient.serial", &snapshot.device_info.serial))
}

pub trait AirGradientTransport {
    fn execute(&mut self, plan: &LocalHttpRequestPlan) -> Result<Vec<u8>, AirGradientError>;
}

#[derive(Debug, Clone)]
pub struct AirGradientLanTransport {
    pub maximum_response_bytes: usize,
}

impl Default for AirGradientLanTransport {
    fn default() -> Self {
        Self {
            maximum_response_bytes: DEFAULT_MAX_RESPONSE_BYTES,
        }
    }
}

impl AirGradientTransport for AirGradientLanTransport {
    fn execute(&mut self, plan: &LocalHttpRequestPlan) -> Result<Vec<u8>, AirGradientError> {
        let url = Url::parse(&plan.url)?;
        if url.scheme != "http" {
            return Err(AirGradientError::Validation(
                "AirGradient transport only permits local HTTP".to_string(),
            ));
        }
        let host = url
            .host
            .as_deref()
            .ok_or(AirGradientError::MissingField("URL host"))?;
        let port = url
            .effective_port()
            .ok_or(AirGradientError::MissingField("URL port"))?;
        let timeout = Duration::from_millis(plan.timeout_ms.max(1));
        let request = encode_http_request(&url, plan)?;
        let mut stream = connect_tcp(host, port, timeout)?;
        stream
            .write_all(&request)
            .map_err(|error| AirGradientError::Io(error.to_string()))?;
        let bytes = read_bounded(&mut stream, self.maximum_response_bytes)?;
        decode_http_response(&bytes, self.maximum_response_bytes)
    }
}

pub struct AirGradientClient<T> {
    config: AirGradientConfig,
    endpoint: LocalHttpEndpoint,
    transport: T,
}

impl<T: AirGradientTransport> AirGradientClient<T> {
    pub fn new(config: AirGradientConfig, transport: T) -> Result<Self, AirGradientError> {
        let endpoint = config.endpoint()?;
        Ok(Self {
            config,
            endpoint,
            transport,
        })
    }

    pub fn config(&self) -> &AirGradientConfig {
        &self.config
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    pub fn transport_mut(&mut self) -> &mut T {
        &mut self.transport
    }

    pub fn inspect(&mut self) -> Result<AirGradientSnapshot, AirGradientError> {
        let data = self.request_json(MEASUREMENT_PATH)?;
        let snapshot = parse_snapshot(&data)?;
        if self
            .config
            .expected_serial
            .as_deref()
            .is_some_and(|expected| !expected.eq_ignore_ascii_case(&snapshot.device_info.serial))
        {
            return Err(AirGradientError::Validation(
                "monitor serial does not match the requested mDNS identity".to_string(),
            ));
        }
        Ok(snapshot)
    }

    pub fn configuration(&mut self) -> Result<AirGradientConfiguration, AirGradientError> {
        parse_configuration(&self.request_json(CONFIGURATION_PATH)?)
    }

    pub fn update_configuration(&mut self, update: &JsonValue) -> Result<(), AirGradientError> {
        let template = LocalHttpRequestTemplate::new(LocalHttpMethod::Put, CONFIGURATION_PATH)?
            .with_accept("application/json")
            .with_content_type("application/json")
            .with_timeout_ms(duration_ms(self.config.timeout));
        let body = serde_json::to_vec(update)?;
        self.transport
            .execute(&template.plan(&self.endpoint, body)?)?;
        Ok(())
    }

    fn request_json(&mut self, path: &str) -> Result<JsonValue, AirGradientError> {
        let template = LocalHttpRequestTemplate::new(LocalHttpMethod::Get, path)?
            .with_accept("application/json")
            .with_timeout_ms(duration_ms(self.config.timeout));
        let bytes = self
            .transport
            .execute(&template.plan(&self.endpoint, Vec::new())?)?;
        Ok(serde_json::from_slice(&bytes)?)
    }
}

impl<T> fmt::Debug for AirGradientClient<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AirGradientClient")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledAirGradientDevice {
    pub bridge_id: BridgeId,
    pub device_id: DeviceId,
    pub entity_ids: Vec<EntityId>,
}

pub struct AirGradientRuntimeIntegration<T> {
    client: AirGradientClient<T>,
    command_targets: BTreeMap<EntityId, AirGradientCommandTarget>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AirGradientCommandTarget {
    Indicator,
    Co2Sensor,
}

impl<T: AirGradientTransport> AirGradientRuntimeIntegration<T> {
    pub fn new(client: AirGradientClient<T>) -> Self {
        Self {
            client,
            command_targets: BTreeMap::new(),
        }
    }

    pub fn transport(&self) -> &T {
        self.client.transport()
    }

    pub fn transport_mut(&mut self) -> &mut T {
        self.client.transport_mut()
    }

    pub fn inspect_and_install_authorized(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        principal_id: AgentId,
        observed_at_ms: u64,
    ) -> Result<InstalledAirGradientDevice, AirGradientError> {
        authorize_read(runtime, principal_id, observed_at_ms)?;
        let snapshot = self.client.inspect()?;
        let configuration = self.client.configuration()?;
        let mut installed = self.install_snapshot(runtime, &snapshot, observed_at_ms)?;
        self.install_control_surface(
            runtime,
            &snapshot,
            &configuration,
            &mut installed,
            observed_at_ms,
        )?;
        Ok(installed)
    }

    pub fn install_snapshot(
        &self,
        runtime: &mut SmartHomeRuntime,
        snapshot: &AirGradientSnapshot,
        observed_at_ms: u64,
    ) -> Result<InstalledAirGradientDevice, AirGradientError> {
        if snapshot.measurements.is_empty() {
            return Err(AirGradientError::NoMeasurements);
        }
        let endpoint = self.client.config.endpoint()?;
        let native_id = stable_component(&snapshot.device_info.serial);
        if native_id.is_empty() {
            return Err(AirGradientError::Validation(
                "device serial does not contain a stable identifier".to_string(),
            ));
        }
        let bridge_id = self.client.config.bridge_id.clone();
        let device_id = DeviceId::trusted(format!("airgradient:{native_id}"));

        let mut bridge = Bridge::new(
            bridge_id.clone(),
            IntegrationId::trusted(INTEGRATION_ID),
            BridgeTransport::LanHttp,
        );
        bridge.address = Some(endpoint.origin());
        bridge.hardware_model = Some(snapshot.device_info.model.clone());
        bridge.health = Health::Online;
        bridge.last_seen_at_ms = Some(observed_at_ms);
        bridge.identifiers = vec![protocol_identifier("http_endpoint", &endpoint.origin())?];
        bridge.metadata = vec![Metadata::new("airgradient.transport", "local_http_polling")];
        runtime.upsert_bridge(bridge)?;

        let entities = snapshot
            .measurements
            .iter()
            .map(|measurement| {
                let entity_id = EntityId::trusted(format!(
                    "airgradient:{native_id}:sensor:{}",
                    stable_component(&measurement.id)
                ));
                Entity {
                    entity_id: entity_id.clone(),
                    device_id: device_id.clone(),
                    kind: EntityKind::Sensor,
                    name: format!("{} {}", self.client.config.display_name, measurement.name),
                    capabilities: vec![Capability::new(
                        CapabilityId::trusted("sensor.measurement"),
                        CapabilityMode::Observe,
                        ValueKind::Object,
                    )],
                    state: Some(confirmed_state(
                        entity_id,
                        measurement_value(measurement),
                        observed_at_ms,
                    )),
                    metadata: vec![
                        Metadata::new("airgradient.measurement", measurement.id.clone()),
                        Metadata::new("airgradient.unit", measurement.unit.clone()),
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
            bridge_id: bridge_id.clone(),
            manufacturer: "AirGradient".to_string(),
            model: snapshot.device_info.model.clone(),
            name: self.client.config.display_name.clone(),
            serial: Some(snapshot.device_info.serial.clone()),
            firmware_version: Some(snapshot.device_info.firmware.clone()),
            room_id: None,
            entity_ids: entity_ids.clone(),
            identifiers: vec![protocol_identifier("serial", &snapshot.device_info.serial)?],
            health: Health::Online,
            metadata: Vec::new(),
        })?;
        for entity in entities {
            runtime.upsert_entity(entity)?;
        }
        Ok(InstalledAirGradientDevice {
            bridge_id,
            device_id,
            entity_ids,
        })
    }

    fn install_control_surface(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        snapshot: &AirGradientSnapshot,
        configuration: &AirGradientConfiguration,
        installed: &mut InstalledAirGradientDevice,
        observed_at_ms: u64,
    ) -> Result<(), AirGradientError> {
        let native_id = stable_component(&snapshot.device_info.serial);
        let co2_entity_id = EntityId::trusted(format!(
            "airgradient:{native_id}:sensor:rco2"
        ));
        let mut co2_entity = runtime
            .registry()
            .entity(&co2_entity_id)
            .cloned()
            .ok_or_else(|| AirGradientError::UnknownEntity(co2_entity_id.clone()))?;
        co2_entity.capabilities.push(Capability::sensor_calibration());
        runtime.upsert_entity(co2_entity)?;

        let indicator_entity_id = EntityId::trusted(format!("airgradient:{native_id}:indicator"));
        runtime.upsert_entity(Entity {
            entity_id: indicator_entity_id.clone(),
            device_id: installed.device_id.clone(),
            kind: EntityKind::Light,
            name: format!("{} indicator and display", self.client.config.display_name),
            capabilities: vec![Capability::device_indicator(), Capability::device_display()],
            state: Some(confirmed_state(
                indicator_entity_id.clone(),
                configuration_value(configuration),
                observed_at_ms,
            )),
            metadata: vec![
                Metadata::new("airgradient.control_surface", "indicator_display"),
                Metadata::new(
                    "airgradient.configuration_control",
                    configuration.control.as_str(),
                ),
            ],
        })?;

        let mut device = runtime
            .registry()
            .device(&installed.device_id)
            .cloned()
            .ok_or_else(|| AirGradientError::Validation("installed device disappeared".to_string()))?;
        device.entity_ids.push(indicator_entity_id.clone());
        device.metadata.push(Metadata::new(
            "airgradient.configuration_control",
            configuration.control.as_str(),
        ));
        runtime.upsert_device(device)?;
        installed.entity_ids.push(indicator_entity_id.clone());
        self.command_targets = BTreeMap::from([
            (indicator_entity_id, AirGradientCommandTarget::Indicator),
            (co2_entity_id, AirGradientCommandTarget::Co2Sensor),
        ]);
        Ok(())
    }

    pub fn dispatch_command_authorized(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        principal_id: AgentId,
        request: RuntimeCommandToolRequest,
        now_ms: u64,
    ) -> Result<CommandResult, AirGradientError> {
        let plan = airgradient_command_plan(&self.command_targets, &request)?;
        let target_entity_id = request.entity_id.clone();
        let command = runtime.authorize_command_tool(principal_id, request, now_ms)?;
        let configuration = self.client.configuration()?;
        if configuration.control == AirGradientConfigurationControl::Cloud {
            return Err(AirGradientError::CloudConfigurationConflict);
        }
        let mut result = runtime.submit_command(command, now_ms)?;
        self.client.update_configuration(&plan.update)?;
        if let Some(expected) = plan.expected {
            let confirmed = self.client.configuration()?;
            expected.verify(&confirmed)?;
            let mut entity = runtime
                .registry()
                .entity(&target_entity_id)
                .cloned()
                .ok_or_else(|| AirGradientError::UnknownEntity(target_entity_id.clone()))?;
            entity.state = Some(confirmed_state(
                target_entity_id,
                configuration_value(&confirmed),
                now_ms,
            ));
            runtime.upsert_entity(entity)?;
        }
        result.message = Some(match configuration.control {
            AirGradientConfigurationControl::Local => {
                "AirGradient confirmed the local control request".to_string()
            }
            AirGradientConfigurationControl::Both => {
                "AirGradient confirmed the local request, but cloud configuration may overwrite it while configurationControl=both".to_string()
            }
            AirGradientConfigurationControl::Cloud => unreachable!(),
        });
        Ok(result)
    }
}

#[derive(Debug, Clone, PartialEq)]
struct AirGradientCommandPlan {
    update: JsonValue,
    expected: Option<ExpectedConfiguration>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ExpectedConfiguration {
    IndicatorMode(String),
    IndicatorBrightness(u8),
    DisplayBrightness(u8),
}

impl ExpectedConfiguration {
    fn verify(&self, configuration: &AirGradientConfiguration) -> Result<(), AirGradientError> {
        let (matches, field) = match self {
            Self::IndicatorMode(expected) => {
                (configuration.led_bar_mode == *expected, "ledBarMode")
            }
            Self::IndicatorBrightness(expected) => (
                configuration.led_bar_brightness == *expected,
                "ledBarBrightness",
            ),
            Self::DisplayBrightness(expected) => (
                configuration.display_brightness == *expected,
                "displayBrightness",
            ),
        };
        if matches {
            Ok(())
        } else {
            Err(AirGradientError::VerificationFailed(field))
        }
    }
}

fn airgradient_command_plan(
    targets: &BTreeMap<EntityId, AirGradientCommandTarget>,
    request: &RuntimeCommandToolRequest,
) -> Result<AirGradientCommandPlan, AirGradientError> {
    let target = targets
        .get(&request.entity_id)
        .copied()
        .ok_or_else(|| AirGradientError::UnknownEntity(request.entity_id.clone()))?;
    match request.command_type {
        CommandType::DeviceControl(DeviceControlCommandType::SetIndicatorMode)
            if target == AirGradientCommandTarget::Indicator =>
        {
            let Value::Text(mode) = &request.arguments else {
                return invalid_command_arguments(request.command_type, "co2, pm, iaqs, or off text");
            };
            let mode = mode.to_ascii_lowercase();
            if !matches!(mode.as_str(), "co2" | "pm" | "iaqs" | "off") {
                return invalid_command_arguments(request.command_type, "co2, pm, iaqs, or off text");
            }
            Ok(AirGradientCommandPlan {
                update: configuration_update("ledBarMode", JsonValue::String(mode.clone())),
                expected: Some(ExpectedConfiguration::IndicatorMode(mode)),
            })
        }
        CommandType::DeviceControl(DeviceControlCommandType::SetIndicatorBrightness)
            if target == AirGradientCommandTarget::Indicator =>
        {
            let Value::Percentage(brightness) = request.arguments else {
                return invalid_command_arguments(request.command_type, "a percentage brightness");
            };
            Ok(AirGradientCommandPlan {
                update: configuration_update(
                    "ledBarBrightness",
                    JsonValue::from(brightness),
                ),
                expected: Some(ExpectedConfiguration::IndicatorBrightness(brightness)),
            })
        }
        CommandType::DeviceControl(DeviceControlCommandType::SetDisplayBrightness)
            if target == AirGradientCommandTarget::Indicator =>
        {
            let Value::Percentage(brightness) = request.arguments else {
                return invalid_command_arguments(request.command_type, "a percentage brightness");
            };
            Ok(AirGradientCommandPlan {
                update: configuration_update(
                    "displayBrightness",
                    JsonValue::from(brightness),
                ),
                expected: Some(ExpectedConfiguration::DisplayBrightness(brightness)),
            })
        }
        CommandType::DeviceControl(DeviceControlCommandType::CalibrateSensor)
            if target == AirGradientCommandTarget::Co2Sensor =>
        {
            if request.arguments != Value::Null {
                return invalid_command_arguments(request.command_type, "null arguments");
            }
            Ok(AirGradientCommandPlan {
                update: configuration_update(
                    "co2CalibrationRequested",
                    JsonValue::Bool(true),
                ),
                expected: None,
            })
        }
        CommandType::DeviceControl(_) => invalid_command_arguments(
            request.command_type,
            "an AirGradient entity that advertises the command capability",
        ),
        command_type => Err(AirGradientError::UnsupportedCommand(command_type)),
    }
}

fn invalid_command_arguments<T>(
    command_type: CommandType,
    expected: &'static str,
) -> Result<T, AirGradientError> {
    Err(AirGradientError::InvalidCommandArguments {
        command_type,
        expected,
    })
}

fn configuration_update(field: &str, value: JsonValue) -> JsonValue {
    let mut update = JsonMap::new();
    update.insert(field.to_string(), value);
    JsonValue::Object(update)
}

fn configuration_value(configuration: &AirGradientConfiguration) -> Value {
    Value::Object(vec![
        (
            "mode".to_string(),
            Value::Text(configuration.led_bar_mode.clone()),
        ),
        (
            "indicator_brightness".to_string(),
            Value::Percentage(configuration.led_bar_brightness),
        ),
        (
            "display_brightness".to_string(),
            Value::Percentage(configuration.display_brightness),
        ),
        (
            "configuration_control".to_string(),
            Value::Text(configuration.control.as_str().to_string()),
        ),
    ])
}

fn authorize_read(
    runtime: &mut SmartHomeRuntime,
    principal_id: AgentId,
    now_ms: u64,
) -> Result<(), AirGradientError> {
    let tool = SmartHomeTool::GetState;
    let decision = runtime.authorize_tool_for_principal(principal_id.clone(), tool, now_ms);
    if decision.missing_capabilities.is_empty() {
        Ok(())
    } else {
        Err(AirGradientError::Runtime(RuntimeError::UnauthorizedTool {
            principal_id,
            tool,
            missing_capabilities: decision.missing_capabilities,
        }))
    }
}

fn parse_configuration(data: &JsonValue) -> Result<AirGradientConfiguration, AirGradientError> {
    let data = data
        .as_object()
        .ok_or(AirGradientError::MissingField("configuration object"))?;
    let control = match required_string(data, "configurationControl")?
        .to_ascii_lowercase()
        .as_str()
    {
        "local" => AirGradientConfigurationControl::Local,
        "both" => AirGradientConfigurationControl::Both,
        "cloud" => AirGradientConfigurationControl::Cloud,
        _ => {
            return Err(AirGradientError::Validation(
                "configurationControl must be local, both, or cloud".to_string(),
            ))
        }
    };
    Ok(AirGradientConfiguration {
        control,
        led_bar_mode: required_string(data, "ledBarMode")?.to_ascii_lowercase(),
        led_bar_brightness: required_percentage(data, "ledBarBrightness")?,
        display_brightness: required_percentage(data, "displayBrightness")?,
    })
}

fn parse_snapshot(data: &JsonValue) -> Result<AirGradientSnapshot, AirGradientError> {
    let data = data
        .as_object()
        .ok_or(AirGradientError::MissingField("measurement object"))?;
    let device_info = AirGradientDeviceInfo {
        serial: validate_serial(required_string(data, "serialno")?)?,
        model: required_string(data, "model")?,
        firmware: required_string(data, "firmware")?,
    };
    let mut measurements = Vec::new();
    for definition in MEASUREMENT_DEFINITIONS {
        if let Some(value) = data
            .get(definition.field)
            .and_then(JsonValue::as_f64)
            .filter(|value| value.is_finite())
        {
            measurements.push(AirGradientMeasurement {
                id: definition.field.to_string(),
                name: definition.name.to_string(),
                value,
                unit: definition.unit.to_string(),
            });
        }
    }
    if measurements.is_empty() {
        return Err(AirGradientError::NoMeasurements);
    }
    Ok(AirGradientSnapshot {
        device_info,
        measurements,
    })
}

#[derive(Debug, Clone, Copy)]
struct MeasurementDefinition {
    field: &'static str,
    name: &'static str,
    unit: &'static str,
}

const MEASUREMENT_DEFINITIONS: &[MeasurementDefinition] = &[
    MeasurementDefinition {
        field: "rco2",
        name: "Carbon dioxide",
        unit: "ppm",
    },
    MeasurementDefinition {
        field: "pm01",
        name: "PM1.0",
        unit: "ug/m3",
    },
    MeasurementDefinition {
        field: "pm02",
        name: "PM2.5",
        unit: "ug/m3",
    },
    MeasurementDefinition {
        field: "pm02Compensated",
        name: "PM2.5 compensated",
        unit: "ug/m3",
    },
    MeasurementDefinition {
        field: "pm10",
        name: "PM10",
        unit: "ug/m3",
    },
    MeasurementDefinition {
        field: "atmp",
        name: "Temperature",
        unit: "C",
    },
    MeasurementDefinition {
        field: "atmpCompensated",
        name: "Temperature compensated",
        unit: "C",
    },
    MeasurementDefinition {
        field: "rhum",
        name: "Relative humidity",
        unit: "%",
    },
    MeasurementDefinition {
        field: "rhumCompensated",
        name: "Relative humidity compensated",
        unit: "%",
    },
    MeasurementDefinition {
        field: "tvocIndex",
        name: "TVOC index",
        unit: "index",
    },
    MeasurementDefinition {
        field: "noxIndex",
        name: "NOx index",
        unit: "index",
    },
    MeasurementDefinition {
        field: "pm003Count",
        name: "Particle count 0.3um",
        unit: "count/dL",
    },
    MeasurementDefinition {
        field: "wifi",
        name: "Wi-Fi signal",
        unit: "dBm",
    },
];

fn required_string(
    value: &JsonMap<String, JsonValue>,
    field: &'static str,
) -> Result<String, AirGradientError> {
    value
        .get(field)
        .and_then(JsonValue::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .ok_or(AirGradientError::MissingField(field))
}

fn required_percentage(
    value: &JsonMap<String, JsonValue>,
    field: &'static str,
) -> Result<u8, AirGradientError> {
    let value = value
        .get(field)
        .and_then(JsonValue::as_u64)
        .ok_or(AirGradientError::MissingField(field))?;
    u8::try_from(value)
        .ok()
        .filter(|value| *value <= 100)
        .ok_or_else(|| {
            AirGradientError::Validation(format!("{field} must be between 0 and 100"))
        })
}

fn measurement_value(measurement: &AirGradientMeasurement) -> Value {
    Value::Object(vec![
        ("value".to_string(), Value::Number(measurement.value)),
        (
            "unit".to_string(),
            Value::Text(measurement.unit.to_string()),
        ),
    ])
}

fn validate_serial(serial: String) -> Result<String, AirGradientError> {
    let serial = serial.trim().to_ascii_lowercase();
    if serial.is_empty()
        || serial.len() > 64
        || !serial.chars().all(|value| value.is_ascii_alphanumeric())
    {
        return Err(AirGradientError::Validation(
            "serial must contain 1 to 64 ASCII letters or digits".to_string(),
        ));
    }
    Ok(serial)
}

fn confirmed_state(entity_id: EntityId, value: Value, observed_at_ms: u64) -> StateSnapshot {
    StateSnapshot {
        entity_id,
        value,
        source: StateSource::Poll,
        observed_at_ms,
        received_at_ms: observed_at_ms,
        expires_at_ms: None,
        confidence: StateConfidence::Confirmed,
    }
}

fn protocol_identifier(kind: &str, value: &str) -> Result<ProtocolIdentifier, AirGradientError> {
    ProtocolIdentifier::new(ProtocolFamily::Vendor(PROTOCOL_ID.to_string()), kind, value)
        .map_err(|error| AirGradientError::Validation(error.to_string()))
}

fn stable_component(value: &str) -> String {
    let mut output = String::new();
    let mut separator = false;
    for character in value.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() {
            output.push(character);
            separator = false;
        } else if !output.is_empty() && !separator {
            output.push('-');
            separator = true;
        }
    }
    output.trim_matches('-').to_string()
}

fn duration_ms(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

fn has_unsafe_http_text(value: &str) -> bool {
    value.contains(['\r', '\n', '\0'])
}

fn encode_http_request(
    url: &Url,
    plan: &LocalHttpRequestPlan,
) -> Result<Vec<u8>, AirGradientError> {
    let host = url
        .host
        .as_deref()
        .ok_or(AirGradientError::MissingField("URL host"))?;
    let port = url
        .effective_port()
        .ok_or(AirGradientError::MissingField("URL port"))?;
    let target = if url.path.is_empty() {
        "/".to_string()
    } else if let Some(query) = &url.query {
        format!("{}?{query}", url.path)
    } else {
        url.path.clone()
    };
    if has_unsafe_http_text(&target) || has_unsafe_http_text(host) {
        return Err(AirGradientError::Validation(
            "request target contains unsafe HTTP text".to_string(),
        ));
    }
    let host_header = if port == DEFAULT_PORT {
        host.to_string()
    } else {
        format!("{host}:{port}")
    };
    let mut request = format!(
        "{} {target} HTTP/1.1\r\nHost: {host_header}\r\nConnection: close\r\n",
        plan.method.as_str()
    )
    .into_bytes();
    let mut seen = BTreeSet::new();
    for header in &plan.headers {
        if has_unsafe_http_text(&header.name) || has_unsafe_http_text(&header.value) {
            return Err(AirGradientError::Validation(
                "request header contains unsafe HTTP text".to_string(),
            ));
        }
        seen.insert(header.name.to_ascii_lowercase());
        request.extend_from_slice(format!("{}: {}\r\n", header.name, header.value).as_bytes());
    }
    if !seen.contains("content-length") {
        request.extend_from_slice(format!("Content-Length: {}\r\n", plan.body.len()).as_bytes());
    }
    request.extend_from_slice(b"\r\n");
    request.extend_from_slice(&plan.body);
    Ok(request)
}

fn connect_tcp(host: &str, port: u16, timeout: Duration) -> Result<TcpStream, AirGradientError> {
    let addresses = (host, port)
        .to_socket_addrs()
        .map_err(|error| AirGradientError::Io(error.to_string()))?
        .collect::<Vec<_>>();
    let mut last_error = None;
    for address in addresses {
        match TcpStream::connect_timeout(&address, timeout) {
            Ok(stream) => {
                stream
                    .set_read_timeout(Some(timeout))
                    .map_err(|error| AirGradientError::Io(error.to_string()))?;
                stream
                    .set_write_timeout(Some(timeout))
                    .map_err(|error| AirGradientError::Io(error.to_string()))?;
                return Ok(stream);
            }
            Err(error) => last_error = Some(error),
        }
    }
    Err(AirGradientError::Io(
        last_error
            .unwrap_or_else(|| {
                io::Error::new(
                    io::ErrorKind::AddrNotAvailable,
                    "host resolved to no addresses",
                )
            })
            .to_string(),
    ))
}

fn read_bounded(reader: &mut dyn Read, maximum: usize) -> Result<Vec<u8>, AirGradientError> {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| AirGradientError::Io(error.to_string()))?;
        if read == 0 {
            break;
        }
        if read > maximum.saturating_sub(bytes.len()) {
            return Err(AirGradientError::ResponseTooLarge { limit: maximum });
        }
        bytes.extend_from_slice(&buffer[..read]);
    }
    Ok(bytes)
}

fn decode_http_response(bytes: &[u8], maximum: usize) -> Result<Vec<u8>, AirGradientError> {
    let parsed = parse_response_head(bytes)
        .map_err(|error: Http1ParseError| AirGradientError::Http(error.to_string()))?;
    if !(200..300).contains(&parsed.head.status) {
        return Err(AirGradientError::HttpStatus(parsed.head.status));
    }
    let input = &bytes[parsed.body_offset..];
    let body = match parsed.body_kind {
        BodyKind::None => Vec::new(),
        BodyKind::ContentLength(expected) => {
            if input.len() < expected {
                return Err(AirGradientError::TruncatedBody {
                    expected,
                    actual: input.len(),
                });
            }
            input[..expected].to_vec()
        }
        BodyKind::UntilEof => input.to_vec(),
        BodyKind::Chunked => decode_chunked(input, maximum)?,
    };
    if body.len() > maximum {
        return Err(AirGradientError::ResponseTooLarge { limit: maximum });
    }
    Ok(body)
}

fn decode_chunked(input: &[u8], maximum: usize) -> Result<Vec<u8>, AirGradientError> {
    let mut cursor = 0usize;
    let mut output = Vec::new();
    loop {
        let offset = input[cursor..]
            .windows(2)
            .position(|window| window == b"\r\n")
            .ok_or_else(|| AirGradientError::Http("missing chunk-size terminator".to_string()))?;
        let end = cursor + offset;
        let size_text = std::str::from_utf8(&input[cursor..end])
            .map_err(|_| AirGradientError::Http("chunk size is not ASCII".to_string()))?
            .split(';')
            .next()
            .unwrap_or_default();
        let size = usize::from_str_radix(size_text.trim(), 16)
            .map_err(|_| AirGradientError::Http("invalid chunk size".to_string()))?;
        cursor = end + 2;
        if size == 0 {
            return Ok(output);
        }
        if size > maximum.saturating_sub(output.len()) {
            return Err(AirGradientError::ResponseTooLarge { limit: maximum });
        }
        let chunk_end = cursor
            .checked_add(size)
            .ok_or_else(|| AirGradientError::Http("chunk size overflow".to_string()))?;
        if input.len() < chunk_end + 2 || &input[chunk_end..chunk_end + 2] != b"\r\n" {
            return Err(AirGradientError::Http("truncated chunk".to_string()));
        }
        output.extend_from_slice(&input[cursor..chunk_end]);
        cursor = chunk_end + 2;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smart_home_core::{CapabilityGrant, CapabilityGrantId, PrivilegeTier};
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::thread;

    const MEASUREMENTS: &str = r#"{"wifi":-46,"serialno":"ecda3b1eaaaf","rco2":447,"pm01":3,"pm02":7,"pm10":8,"pm003Count":442,"atmp":25.87,"atmpCompensated":24.47,"rhum":43,"rhumCompensated":49,"tvocIndex":100,"tvocRaw":33051,"noxIndex":1,"noxRaw":16307,"boot":6,"firmware":"3.1.3","model":"I-9PSL"}"#;
    const CONFIG_BOTH: &str = r#"{"configurationControl":"both","ledBarMode":"co2","ledBarBrightness":80,"displayBrightness":70}"#;
    const CONFIG_CLOUD: &str = r#"{"configurationControl":"cloud","ledBarMode":"co2","ledBarBrightness":80,"displayBrightness":70}"#;
    const CONFIG_MODE_PM: &str = r#"{"configurationControl":"both","ledBarMode":"pm","ledBarBrightness":80,"displayBrightness":70}"#;
    const CONFIG_LED_35: &str = r#"{"configurationControl":"both","ledBarMode":"pm","ledBarBrightness":35,"displayBrightness":70}"#;
    const CONFIG_DISPLAY_25: &str = r#"{"configurationControl":"both","ledBarMode":"pm","ledBarBrightness":35,"displayBrightness":25}"#;

    fn response(body: &str) -> Vec<u8> {
        format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).into_bytes()
    }

    fn start_server(
        payloads: Vec<Vec<u8>>,
    ) -> (u16, Arc<Mutex<Vec<String>>>, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&requests);
        let handle = thread::spawn(move || {
            for payload in payloads {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut bytes = Vec::new();
                let mut buffer = [0u8; 4096];
                loop {
                    let read = stream.read(&mut buffer).unwrap();
                    if read == 0 {
                        break;
                    }
                    bytes.extend_from_slice(&buffer[..read]);
                    if request_complete(&bytes) {
                        break;
                    }
                }
                captured
                    .lock()
                    .unwrap()
                    .push(String::from_utf8(bytes).unwrap());
                stream.write_all(&payload).unwrap();
            }
        });
        (port, requests, handle)
    }

    fn request_complete(bytes: &[u8]) -> bool {
        let Some(header_end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") else {
            return false;
        };
        let headers = String::from_utf8_lossy(&bytes[..header_end]);
        let content_length = headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().ok())?
            })
            .unwrap_or(0);
        bytes.len() >= header_end + 4 + content_length
    }

    fn config(port: u16) -> AirGradientConfig {
        AirGradientConfig::new(
            BridgeId::trusted("airgradient.test"),
            format!("http://127.0.0.1:{port}"),
        )
        .unwrap()
        .with_display_name("Living Room Air")
    }

    fn grant(runtime: &mut SmartHomeRuntime, principal: &AgentId) {
        let _ = runtime.registry_mut().upsert_capability_grant(
            CapabilityGrant::for_all_smart_home(
                CapabilityGrantId::trusted("grant:airgradient-test"),
                principal.clone(),
                PrivilegeTier::HumanApproval,
                "test",
                1_000,
            )
            .with_expiry(20_000),
        );
    }

    #[test]
    fn documented_mdns_identity_becomes_verified_http_discovery() {
        let config = AirGradientConfig::from_serial(
            BridgeId::trusted("airgradient.discovery"),
            "ECDA3B1EAAAF",
        )
        .unwrap();
        assert_eq!(config.base_url, "http://airgradient_ecda3b1eaaaf.local");
        let snapshot = parse_snapshot(&serde_json::from_str(MEASUREMENTS).unwrap()).unwrap();
        let record = discovery_record(&config, &snapshot, 1_000).unwrap();
        assert_eq!(record.native_bridge_id, "ecda3b1eaaaf");
        assert_eq!(
            record.address.as_deref(),
            Some("http://airgradient_ecda3b1eaaaf.local")
        );
        assert_eq!(record.confidence, DiscoveryConfidence::Verified);
        assert_eq!(record.source, DiscoverySource::Mdns);
        assert_eq!(record.hardware_model.as_deref(), Some("I-9PSL"));
    }

    #[test]
    fn serial_validation_rejects_unsafe_mdns_names() {
        assert!(AirGradientConfig::from_serial(
            BridgeId::trusted("airgradient.discovery"),
            "../../monitor"
        )
        .is_err());
    }

    #[test]
    fn real_tcp_inspection_installs_environmental_sensors() {
        let (port, requests, handle) =
            start_server(vec![response(MEASUREMENTS), response(CONFIG_BOTH)]);
        let client =
            AirGradientClient::new(config(port), AirGradientLanTransport::default()).unwrap();
        let mut integration = AirGradientRuntimeIntegration::new(client);
        let principal = AgentId::trusted("agent:airgradient-read");
        let mut runtime = SmartHomeRuntime::new();
        grant(&mut runtime, &principal);
        let installed = integration
            .inspect_and_install_authorized(&mut runtime, principal, 5_000)
            .unwrap();
        handle.join().unwrap();
        assert_eq!(installed.entity_ids.len(), 13);
        let co2 = runtime
            .registry()
            .entity(&EntityId::trusted(format!(
                "{}:sensor:rco2",
                installed.device_id.as_str()
            )))
            .unwrap();
        assert_eq!(co2.kind, EntityKind::Sensor);
        assert_eq!(co2.name, "Living Room Air Carbon dioxide");
        assert_eq!(
            co2.state.as_ref().unwrap().confidence,
            StateConfidence::Confirmed
        );
        let device = runtime.registry().device(&installed.device_id).unwrap();
        assert_eq!(device.manufacturer, "AirGradient");
        assert_eq!(device.model, "I-9PSL");
        assert_eq!(device.serial.as_deref(), Some("ecda3b1eaaaf"));
        assert!(device
            .metadata
            .iter()
            .any(|item| item.key == "airgradient.configuration_control" && item.value == "both"));
        let indicator = runtime
            .registry()
            .entity(&EntityId::trusted("airgradient:ecda3b1eaaaf:indicator"))
            .unwrap();
        assert_eq!(indicator.kind, EntityKind::Light);
        assert!(indicator
            .capabilities
            .iter()
            .any(|capability| capability.capability_id.as_str() == "device.display"));
        assert!(co2
            .capabilities
            .iter()
            .any(|capability| capability.capability_id.as_str() == "sensor.calibration"));
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 2);
        assert!(requests[0].contains(&format!("GET {MEASUREMENT_PATH} HTTP/1.1")));
        assert!(requests[1].contains(&format!("GET {CONFIGURATION_PATH} HTTP/1.1")));
    }

    #[derive(Debug)]
    struct CountingTransport(Arc<AtomicUsize>);

    impl AirGradientTransport for CountingTransport {
        fn execute(&mut self, _plan: &LocalHttpRequestPlan) -> Result<Vec<u8>, AirGradientError> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(Vec::new())
        }
    }

    #[derive(Debug)]
    struct StaticTransport(Vec<u8>);

    impl AirGradientTransport for StaticTransport {
        fn execute(&mut self, _plan: &LocalHttpRequestPlan) -> Result<Vec<u8>, AirGradientError> {
            Ok(self.0.clone())
        }
    }

    #[derive(Debug)]
    struct SequenceTransport {
        responses: Vec<Vec<u8>>,
        calls: usize,
    }

    impl SequenceTransport {
        fn new(responses: &[&str]) -> Self {
            Self {
                responses: responses
                    .iter()
                    .rev()
                    .map(|response| response.as_bytes().to_vec())
                    .collect(),
                calls: 0,
            }
        }
    }

    impl AirGradientTransport for SequenceTransport {
        fn execute(&mut self, _plan: &LocalHttpRequestPlan) -> Result<Vec<u8>, AirGradientError> {
            self.calls += 1;
            self.responses
                .pop()
                .ok_or_else(|| AirGradientError::Io("unexpected transport call".to_string()))
        }
    }

    #[test]
    fn discovered_serial_must_match_the_monitor_response() {
        let config = AirGradientConfig::from_serial(
            BridgeId::trusted("airgradient.mismatch"),
            "aaaaaaaaaaaa",
        )
        .unwrap();
        let mut client =
            AirGradientClient::new(config, StaticTransport(MEASUREMENTS.as_bytes().to_vec()))
                .unwrap();
        assert!(matches!(
            client.inspect(),
            Err(AirGradientError::Validation(message))
                if message.contains("does not match")
        ));
    }

    #[test]
    fn denied_read_reaches_no_transport() {
        let calls = Arc::new(AtomicUsize::new(0));
        let client = AirGradientClient::new(
            AirGradientConfig::new(BridgeId::trusted("airgradient.denied"), "http://127.0.0.1")
                .unwrap(),
            CountingTransport(Arc::clone(&calls)),
        )
        .unwrap();
        let mut integration = AirGradientRuntimeIntegration::new(client);
        assert!(matches!(
            integration.inspect_and_install_authorized(
                &mut SmartHomeRuntime::new(),
                AgentId::trusted("agent:denied"),
                5_000,
            ),
            Err(AirGradientError::Runtime(_))
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn denied_device_control_reaches_no_additional_transport() {
        let client = AirGradientClient::new(
            AirGradientConfig::new(
                BridgeId::trusted("airgradient.command-denied"),
                "http://127.0.0.1",
            )
            .unwrap(),
            SequenceTransport::new(&[MEASUREMENTS, CONFIG_BOTH]),
        )
        .unwrap();
        let mut integration = AirGradientRuntimeIntegration::new(client);
        let installer = AgentId::trusted("agent:airgradient-installer");
        let mut runtime = SmartHomeRuntime::new();
        grant(&mut runtime, &installer);
        integration
            .inspect_and_install_authorized(&mut runtime, installer, 5_000)
            .unwrap();

        let request = RuntimeCommandToolRequest::new(
            EntityId::trusted("airgradient:ecda3b1eaaaf:indicator"),
            CommandType::DeviceControl(DeviceControlCommandType::SetDisplayBrightness),
            Value::Percentage(25),
        );
        assert!(matches!(
            integration.dispatch_command_authorized(
                &mut runtime,
                AgentId::trusted("agent:airgradient-denied"),
                request,
                6_000,
            ),
            Err(AirGradientError::Runtime(_))
        ));
        assert_eq!(integration.transport().calls, 2);
    }

    #[test]
    fn cloud_control_conflict_stops_before_local_put() {
        let client = AirGradientClient::new(
            AirGradientConfig::new(
                BridgeId::trusted("airgradient.cloud-conflict"),
                "http://127.0.0.1",
            )
            .unwrap(),
            SequenceTransport::new(&[MEASUREMENTS, CONFIG_BOTH, CONFIG_CLOUD]),
        )
        .unwrap();
        let mut integration = AirGradientRuntimeIntegration::new(client);
        let principal = AgentId::trusted("agent:airgradient-cloud-conflict");
        let mut runtime = SmartHomeRuntime::new();
        grant(&mut runtime, &principal);
        integration
            .inspect_and_install_authorized(&mut runtime, principal.clone(), 5_000)
            .unwrap();
        let original_state = runtime
            .registry()
            .entity(&EntityId::trusted("airgradient:ecda3b1eaaaf:indicator"))
            .unwrap()
            .state
            .clone();
        let request = RuntimeCommandToolRequest::new(
            EntityId::trusted("airgradient:ecda3b1eaaaf:indicator"),
            CommandType::DeviceControl(DeviceControlCommandType::SetIndicatorMode),
            Value::Text("off".to_string()),
        );
        assert!(matches!(
            integration.dispatch_command_authorized(&mut runtime, principal, request, 6_000),
            Err(AirGradientError::CloudConfigurationConflict)
        ));
        assert_eq!(integration.transport().calls, 3);
        assert_eq!(
            runtime
                .registry()
                .entity(&EntityId::trusted("airgradient:ecda3b1eaaaf:indicator"))
                .unwrap()
                .state,
            original_state
        );
        assert_eq!(runtime.optimistic_state_count(), 0);
    }

    #[test]
    fn loopback_authorized_indicator_display_and_calibration_controls() {
        let payloads = [
            MEASUREMENTS,
            CONFIG_BOTH,
            CONFIG_BOTH,
            "{}",
            CONFIG_MODE_PM,
            CONFIG_MODE_PM,
            "{}",
            CONFIG_LED_35,
            CONFIG_LED_35,
            "{}",
            CONFIG_DISPLAY_25,
            CONFIG_DISPLAY_25,
            "{}",
        ]
        .into_iter()
        .map(response)
        .collect();
        let (port, requests, handle) = start_server(payloads);
        let client =
            AirGradientClient::new(config(port), AirGradientLanTransport::default()).unwrap();
        let mut integration = AirGradientRuntimeIntegration::new(client);
        let principal = AgentId::trusted("agent:airgradient-control");
        let mut runtime = SmartHomeRuntime::new();
        grant(&mut runtime, &principal);
        integration
            .inspect_and_install_authorized(&mut runtime, principal.clone(), 5_000)
            .unwrap();
        let indicator = EntityId::trusted("airgradient:ecda3b1eaaaf:indicator");
        let co2 = EntityId::trusted("airgradient:ecda3b1eaaaf:sensor:rco2");
        let commands = [
            RuntimeCommandToolRequest::new(
                indicator.clone(),
                CommandType::DeviceControl(DeviceControlCommandType::SetIndicatorMode),
                Value::Text("pm".to_string()),
            ),
            RuntimeCommandToolRequest::new(
                indicator.clone(),
                CommandType::DeviceControl(DeviceControlCommandType::SetIndicatorBrightness),
                Value::Percentage(35),
            ),
            RuntimeCommandToolRequest::new(
                indicator,
                CommandType::DeviceControl(DeviceControlCommandType::SetDisplayBrightness),
                Value::Percentage(25),
            ),
            RuntimeCommandToolRequest::new(
                co2,
                CommandType::DeviceControl(DeviceControlCommandType::CalibrateSensor),
                Value::Null,
            ),
        ];
        for (index, request) in commands.into_iter().enumerate() {
            let result = integration
                .dispatch_command_authorized(
                    &mut runtime,
                    principal.clone(),
                    request,
                    6_000 + index as u64,
                )
                .unwrap();
            assert!(result
                .message
                .as_deref()
                .is_some_and(|message| message.contains("cloud configuration may overwrite")));
        }
        let indicator_state = runtime
            .registry()
            .entity(&EntityId::trusted("airgradient:ecda3b1eaaaf:indicator"))
            .unwrap()
            .state
            .as_ref()
            .unwrap();
        assert_eq!(indicator_state.confidence, StateConfidence::Confirmed);
        assert!(matches!(
            &indicator_state.value,
            Value::Object(fields)
                if fields.contains(&("display_brightness".to_string(), Value::Percentage(25)))
        ));
        handle.join().unwrap();
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 13);
        let put_requests = requests
            .iter()
            .filter(|request| request.starts_with(&format!("PUT {CONFIGURATION_PATH}")))
            .collect::<Vec<_>>();
        assert_eq!(put_requests.len(), 4);
        assert!(put_requests[0].contains(r#"{"ledBarMode":"pm"}"#));
        assert!(put_requests[1].contains(r#"{"ledBarBrightness":35}"#));
        assert!(put_requests[2].contains(r#"{"displayBrightness":25}"#));
        assert!(put_requests[3].contains(r#"{"co2CalibrationRequested":true}"#));
    }

    #[test]
    fn parser_requires_identity_and_known_measurements() {
        let missing_identity: JsonValue = serde_json::from_str(r#"{"rco2":447}"#).unwrap();
        assert!(matches!(
            parse_snapshot(&missing_identity),
            Err(AirGradientError::MissingField("serialno"))
        ));
        let empty: JsonValue = serde_json::from_str(
            r#"{"serialno":"ecda3b1eaaaf","model":"I-9PSL","firmware":"3.1.3"}"#,
        )
        .unwrap();
        assert!(matches!(
            parse_snapshot(&empty),
            Err(AirGradientError::NoMeasurements)
        ));
    }

    #[test]
    fn parser_assigns_documented_units_and_ignores_raw_diagnostics() {
        let data: JsonValue = serde_json::from_str(MEASUREMENTS).unwrap();
        let snapshot = parse_snapshot(&data).unwrap();
        assert_eq!(snapshot.device_info.model, "I-9PSL");
        assert!(snapshot
            .measurements
            .iter()
            .any(|measurement| measurement.id == "pm02" && measurement.unit == "ug/m3"));
        assert!(!snapshot
            .measurements
            .iter()
            .any(|measurement| measurement.id == "tvocRaw"));
    }

    #[test]
    fn parser_validates_configuration_control_and_percentages() {
        let configuration =
            parse_configuration(&serde_json::from_str(CONFIG_BOTH).unwrap()).unwrap();
        assert_eq!(configuration.control, AirGradientConfigurationControl::Both);
        assert_eq!(configuration.led_bar_mode, "co2");
        assert_eq!(configuration.led_bar_brightness, 80);
        assert_eq!(configuration.display_brightness, 70);

        let invalid = serde_json::from_str(
            r#"{"configurationControl":"cloudish","ledBarMode":"co2","ledBarBrightness":80,"displayBrightness":101}"#,
        )
        .unwrap();
        assert!(matches!(
            parse_configuration(&invalid),
            Err(AirGradientError::Validation(_))
        ));
    }

    #[test]
    fn response_bounds_are_enforced() {
        assert!(matches!(
            decode_http_response(&response("{}"), 1),
            Err(AirGradientError::ResponseTooLarge { limit: 1 })
        ));
    }
}
