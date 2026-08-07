//! Native Tasmota local HTTP API integration for D23.

#![forbid(unsafe_code)]

use coding_adventures_zeroize::Zeroizing;
use http1::{parse_response_head, Http1ParseError};
use http_core::BodyKind;
use serde_json::{Map as JsonMap, Value as JsonValue};
use smart_home_core::{
    AgentId, Bridge, BridgeId, BridgeTransport, Capability, CapabilityId, CapabilityMode,
    CommandResult, CommandType, Device, DeviceId, Entity, EntityId, EntityKind, Health,
    IntegrationId, Metadata, ProtocolFamily, ProtocolIdentifier, SmartHomeTool, StateConfidence,
    StateSnapshot, StateSource, Value, ValueKind, VaultRef,
};
use smart_home_discovery::{
    run_mdns_ipv4_scan, DiscoveryConfidence, DiscoveryRecord, DiscoverySource, MdnsAdvertisement,
    MdnsScanOptions, MdnsScanResult, PairingRequirement,
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

pub const VERSION: &str = "0.1.0";
pub const INTEGRATION_ID: &str = "tasmota";
pub const PROTOCOL_ID: &str = "tasmota_http";
pub const MDNS_SERVICE_TYPE: &str = "_http._tcp.local";
pub const DEFAULT_PORT: u16 = 80;
pub const DEFAULT_MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
pub const DEFAULT_MAX_DISCOVERY_RESPONSES: usize = 128;

#[derive(Debug)]
pub enum TasmotaError {
    Validation(String),
    Discovery(String),
    LocalHttp(LocalHttpError),
    Url(UrlError),
    Io(String),
    Http(String),
    HttpStatus(u16),
    ResponseTooLarge {
        limit: usize,
    },
    TruncatedBody {
        expected: usize,
        actual: usize,
    },
    Json(serde_json::Error),
    MissingField(&'static str),
    NoOutputs,
    UnknownEntity(EntityId),
    UnsupportedCommand {
        entity_id: EntityId,
        command_type: CommandType,
    },
    InvalidCommandArguments {
        command_type: CommandType,
        expected: &'static str,
    },
    VerificationFailed(String),
    Runtime(RuntimeError),
}

impl fmt::Display for TasmotaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => write!(formatter, "invalid Tasmota input: {message}"),
            Self::Discovery(message) => write!(formatter, "Tasmota discovery failed: {message}"),
            Self::LocalHttp(error) => error.fmt(formatter),
            Self::Url(error) => write!(formatter, "invalid Tasmota URL: {error}"),
            Self::Io(message) => write!(formatter, "Tasmota LAN I/O failed: {message}"),
            Self::Http(message) => write!(formatter, "invalid Tasmota HTTP response: {message}"),
            Self::HttpStatus(status) => {
                write!(formatter, "Tasmota endpoint returned HTTP {status}")
            }
            Self::ResponseTooLarge { limit } => {
                write!(formatter, "Tasmota response exceeds {limit} bytes")
            }
            Self::TruncatedBody { expected, actual } => write!(
                formatter,
                "Tasmota response is truncated: expected {expected} bytes, got {actual}"
            ),
            Self::Json(error) => write!(formatter, "invalid Tasmota JSON: {error}"),
            Self::MissingField(field) => write!(formatter, "Tasmota response is missing {field}"),
            Self::NoOutputs => formatter.write_str("Tasmota status contains no power outputs"),
            Self::UnknownEntity(entity_id) => {
                write!(formatter, "unknown Tasmota entity {entity_id}")
            }
            Self::UnsupportedCommand {
                entity_id,
                command_type,
            } => write!(
                formatter,
                "Tasmota entity {entity_id} does not support {command_type:?}"
            ),
            Self::InvalidCommandArguments {
                command_type,
                expected,
            } => write!(
                formatter,
                "invalid {command_type:?} arguments; expected {expected}"
            ),
            Self::VerificationFailed(message) => {
                write!(formatter, "Tasmota command verification failed: {message}")
            }
            Self::Runtime(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for TasmotaError {}

impl From<LocalHttpError> for TasmotaError {
    fn from(error: LocalHttpError) -> Self {
        Self::LocalHttp(error)
    }
}

impl From<UrlError> for TasmotaError {
    fn from(error: UrlError) -> Self {
        Self::Url(error)
    }
}

impl From<serde_json::Error> for TasmotaError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<RuntimeError> for TasmotaError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

pub struct TasmotaCredentials {
    username: Zeroizing<String>,
    password: Zeroizing<String>,
}

impl TasmotaCredentials {
    pub fn new(
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Result<Self, TasmotaError> {
        let username = username.into();
        let password = password.into();
        if username.trim().is_empty() || password.is_empty() {
            return Err(TasmotaError::Validation(
                "username and password must not be empty".to_string(),
            ));
        }
        Ok(Self {
            username: Zeroizing::new(username),
            password: Zeroizing::new(password),
        })
    }
}

impl fmt::Debug for TasmotaCredentials {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("TasmotaCredentials([REDACTED])")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TasmotaConfig {
    pub bridge_id: BridgeId,
    pub base_url: String,
    pub credential_ref: Option<VaultRef>,
    pub timeout: Duration,
}

impl TasmotaConfig {
    pub fn new(bridge_id: BridgeId, base_url: impl Into<String>) -> Result<Self, TasmotaError> {
        let base_url = base_url.into().trim_end_matches('/').to_string();
        let parsed = Url::parse(&base_url)?;
        if parsed.scheme != "http"
            || parsed.host.is_none()
            || parsed.userinfo.is_some()
            || parsed.query.is_some()
            || parsed.fragment.is_some()
            || !matches!(parsed.path.as_str(), "" | "/")
        {
            return Err(TasmotaError::Validation(
                "base URL must be a credential-free HTTP origin".to_string(),
            ));
        }
        Ok(Self {
            bridge_id,
            base_url,
            credential_ref: None,
            timeout: Duration::from_secs(5),
        })
    }

    pub fn with_credentials(mut self, credential_ref: VaultRef) -> Self {
        self.credential_ref = Some(credential_ref);
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout.max(Duration::from_millis(1));
        self
    }

    fn endpoint(&self) -> Result<LocalHttpEndpoint, TasmotaError> {
        let parsed = Url::parse(&self.base_url)?;
        let host = parsed
            .host
            .as_deref()
            .ok_or(TasmotaError::MissingField("base URL host"))?;
        Ok(LocalHttpEndpoint::new(
            IntegrationId::trusted(INTEGRATION_ID),
            self.bridge_id.clone(),
            LocalHttpScheme::Http,
            host.to_string(),
        )?
        .with_port(parsed.port.unwrap_or(DEFAULT_PORT))
        .with_metadata(Metadata::new("http.profile", "tasmota.command.v1")))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TasmotaOutput {
    pub index: u8,
    pub on: bool,
    pub is_light: bool,
    pub brightness: Option<u8>,
    pub hue: Option<i64>,
    pub saturation: Option<i64>,
    pub color_temperature_mirek: Option<i64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TasmotaSensor {
    pub name: String,
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TasmotaSnapshot {
    pub device_name: String,
    pub friendly_names: Vec<String>,
    pub module: String,
    pub topic: String,
    pub firmware_version: String,
    pub hostname: String,
    pub ip_address: String,
    pub mac_address: String,
    pub outputs: Vec<TasmotaOutput>,
    pub sensors: Vec<TasmotaSensor>,
}

pub fn scan_mdns_ipv4(
    discovered_at_ms: u64,
    timeout: Duration,
) -> Result<MdnsScanResult, TasmotaError> {
    let options = MdnsScanOptions::new(MDNS_SERVICE_TYPE, discovered_at_ms, timeout)
        .map_err(|error| TasmotaError::Discovery(error.to_string()))?
        .with_max_responses(DEFAULT_MAX_DISCOVERY_RESPONSES);
    run_mdns_ipv4_scan(options).map_err(|error| TasmotaError::Discovery(error.to_string()))
}

pub fn discovery_record(
    advertisement: &MdnsAdvertisement,
) -> Result<DiscoveryRecord, TasmotaError> {
    if advertisement.service_type.trim_end_matches('.') != MDNS_SERVICE_TYPE.trim_end_matches('.') {
        return Err(TasmotaError::Validation(format!(
            "unexpected mDNS service type `{}`",
            advertisement.service_type
        )));
    }
    let model = advertisement.txt_value("md").unwrap_or_default();
    if !model.eq_ignore_ascii_case("tasmota")
        && !advertisement
            .instance_name
            .to_ascii_lowercase()
            .starts_with("tasmota")
    {
        return Err(TasmotaError::Validation(
            "HTTP advertisement is not identified as Tasmota".to_string(),
        ));
    }
    let native_id = advertisement
        .txt_value("id")
        .map(stable_component)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| stable_component(&advertisement.instance_name));
    Ok(DiscoveryRecord::new(
        IntegrationId::trusted(INTEGRATION_ID),
        ProtocolFamily::Vendor(PROTOCOL_ID.to_string()),
        native_id,
        DiscoverySource::Mdns,
        BridgeTransport::LanHttp,
        advertisement.discovered_at_ms,
    )
    .map_err(|error| TasmotaError::Discovery(error.to_string()))?
    .with_display_name(&advertisement.instance_name)
    .with_address(advertisement.endpoint_with_scheme("http"))
    .with_hardware_model(model)
    .with_confidence(DiscoveryConfidence::Verified)
    .with_pairing_requirement(PairingRequirement::Unknown)
    .with_metadata("smart_home.discovery.service_type", MDNS_SERVICE_TYPE))
}

pub trait TasmotaTransport {
    fn execute(&mut self, plan: &LocalHttpRequestPlan) -> Result<Vec<u8>, TasmotaError>;
}

#[derive(Debug, Clone)]
pub struct TasmotaLanTransport {
    pub maximum_response_bytes: usize,
}

impl Default for TasmotaLanTransport {
    fn default() -> Self {
        Self {
            maximum_response_bytes: DEFAULT_MAX_RESPONSE_BYTES,
        }
    }
}

impl TasmotaTransport for TasmotaLanTransport {
    fn execute(&mut self, plan: &LocalHttpRequestPlan) -> Result<Vec<u8>, TasmotaError> {
        let url = Url::parse(&plan.url)?;
        if url.scheme != "http" {
            return Err(TasmotaError::Validation(
                "Tasmota transport only permits local HTTP".to_string(),
            ));
        }
        let host = url
            .host
            .as_deref()
            .ok_or(TasmotaError::MissingField("URL host"))?;
        let port = url
            .effective_port()
            .ok_or(TasmotaError::MissingField("URL port"))?;
        let timeout = Duration::from_millis(plan.timeout_ms.max(1));
        let request = encode_http_request(&url, plan)?;
        let mut stream = connect_tcp(host, port, timeout)?;
        stream
            .write_all(&request)
            .map_err(|error| TasmotaError::Io(error.to_string()))?;
        let bytes = read_bounded(&mut stream, self.maximum_response_bytes)?;
        decode_http_response(&bytes, self.maximum_response_bytes)
    }
}

pub struct TasmotaClient<T> {
    config: TasmotaConfig,
    credentials: Option<TasmotaCredentials>,
    endpoint: LocalHttpEndpoint,
    transport: T,
}

impl<T: TasmotaTransport> TasmotaClient<T> {
    pub fn new(
        config: TasmotaConfig,
        credentials: Option<TasmotaCredentials>,
        transport: T,
    ) -> Result<Self, TasmotaError> {
        if config.credential_ref.is_some() != credentials.is_some() {
            return Err(TasmotaError::Validation(
                "credential reference and credential material must be configured together"
                    .to_string(),
            ));
        }
        let endpoint = config.endpoint()?;
        Ok(Self {
            config,
            credentials,
            endpoint,
            transport,
        })
    }

    pub fn config(&self) -> &TasmotaConfig {
        &self.config
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    pub fn inspect(&mut self) -> Result<TasmotaSnapshot, TasmotaError> {
        let value = self.command("Status 0")?;
        parse_snapshot(&value)
    }

    pub fn command(&mut self, command: &str) -> Result<JsonValue, TasmotaError> {
        let path = command_path(command, self.credentials.as_ref());
        let template = LocalHttpRequestTemplate::new(LocalHttpMethod::Get, path.as_str())?
            .with_accept("application/json")
            .with_timeout_ms(duration_ms(self.config.timeout));
        let bytes = self
            .transport
            .execute(&template.plan(&self.endpoint, Vec::new())?)?;
        Ok(serde_json::from_slice(&bytes)?)
    }
}

impl<T> fmt::Debug for TasmotaClient<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TasmotaClient")
            .field("config", &self.config)
            .field("credentials", &self.credentials)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledTasmotaDevice {
    pub bridge_id: BridgeId,
    pub device_id: DeviceId,
    pub entity_ids: Vec<EntityId>,
}

#[derive(Debug, Clone)]
struct OutputBinding {
    entity_id: EntityId,
    index: u8,
    is_light: bool,
    brightness: Option<u8>,
}

pub struct TasmotaRuntimeIntegration<T> {
    client: TasmotaClient<T>,
    bindings: BTreeMap<EntityId, OutputBinding>,
}

impl<T: TasmotaTransport> TasmotaRuntimeIntegration<T> {
    pub fn new(client: TasmotaClient<T>) -> Self {
        Self {
            client,
            bindings: BTreeMap::new(),
        }
    }

    pub fn inspect_and_install_authorized(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        principal_id: AgentId,
        observed_at_ms: u64,
    ) -> Result<InstalledTasmotaDevice, TasmotaError> {
        authorize(
            runtime,
            principal_id,
            SmartHomeTool::GetState,
            observed_at_ms,
        )?;
        let snapshot = self.client.inspect()?;
        self.install_snapshot(runtime, &snapshot, observed_at_ms)
    }

    pub fn install_snapshot(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        snapshot: &TasmotaSnapshot,
        observed_at_ms: u64,
    ) -> Result<InstalledTasmotaDevice, TasmotaError> {
        if snapshot.outputs.is_empty() {
            return Err(TasmotaError::NoOutputs);
        }
        let native_id = stable_component(&snapshot.mac_address);
        if native_id.is_empty() {
            return Err(TasmotaError::MissingField("StatusNET.Mac"));
        }
        let bridge_id = self.client.config.bridge_id.clone();
        let device_id = DeviceId::trusted(format!("tasmota:{native_id}"));
        let endpoint = self.client.config.endpoint()?;

        let mut bridge = Bridge::new(
            bridge_id.clone(),
            IntegrationId::trusted(INTEGRATION_ID),
            BridgeTransport::LanHttp,
        );
        bridge.address = Some(endpoint.origin());
        bridge.hardware_model = Some(snapshot.module.clone());
        bridge.firmware_version = Some(snapshot.firmware_version.clone());
        bridge.auth_ref = self.client.config.credential_ref.clone();
        bridge.health = Health::Online;
        bridge.last_seen_at_ms = Some(observed_at_ms);
        bridge.identifiers = vec![protocol_identifier("http_endpoint", &endpoint.origin())?];
        bridge.metadata = vec![Metadata::new("tasmota.transport", "local_http_polling")];
        runtime.upsert_bridge(bridge)?;

        let mut entities = Vec::new();
        let mut bindings = BTreeMap::new();
        for output in &snapshot.outputs {
            let entity_id =
                EntityId::trusted(format!("tasmota:{native_id}:output:{}", output.index));
            let name = snapshot
                .friendly_names
                .get(usize::from(output.index.saturating_sub(1)))
                .filter(|name| !name.trim().is_empty())
                .cloned()
                .unwrap_or_else(|| format!("{} output {}", display_name(snapshot), output.index));
            let mut capabilities = vec![Capability::light_on_off()];
            if output.is_light {
                if output.brightness.is_some() {
                    capabilities.push(Capability::light_brightness());
                }
                if output.hue.is_some() && output.saturation.is_some() {
                    capabilities.push(Capability::light_color());
                }
                if output.color_temperature_mirek.is_some() {
                    capabilities.push(Capability::light_color_temperature());
                }
            }
            entities.push(Entity {
                entity_id: entity_id.clone(),
                device_id: device_id.clone(),
                kind: if output.is_light {
                    EntityKind::Light
                } else {
                    EntityKind::Switch
                },
                name,
                capabilities,
                state: Some(confirmed_state(
                    entity_id.clone(),
                    output_state(output),
                    observed_at_ms,
                )),
                metadata: vec![Metadata::new("tasmota.output", output.index.to_string())],
            });
            bindings.insert(
                entity_id.clone(),
                OutputBinding {
                    entity_id,
                    index: output.index,
                    is_light: output.is_light,
                    brightness: output.brightness,
                },
            );
        }
        for sensor in &snapshot.sensors {
            let entity_id = EntityId::trusted(format!(
                "tasmota:{native_id}:sensor:{}",
                stable_component(&sensor.name)
            ));
            entities.push(Entity {
                entity_id: entity_id.clone(),
                device_id: device_id.clone(),
                kind: EntityKind::Sensor,
                name: format!("{} {}", display_name(snapshot), sensor.name),
                capabilities: vec![Capability::new(
                    CapabilityId::trusted("sensor.measurement"),
                    CapabilityMode::Observe,
                    ValueKind::Object,
                )],
                state: Some(confirmed_state(
                    entity_id,
                    sensor.value.clone(),
                    observed_at_ms,
                )),
                metadata: vec![Metadata::new("tasmota.sensor", sensor.name.clone())],
            });
        }

        let entity_ids = entities
            .iter()
            .map(|entity| entity.entity_id.clone())
            .collect::<Vec<_>>();
        runtime.upsert_device(Device {
            device_id: device_id.clone(),
            bridge_id: bridge_id.clone(),
            manufacturer: "Tasmota".to_string(),
            model: snapshot.module.clone(),
            name: display_name(snapshot),
            serial: Some(snapshot.mac_address.clone()),
            firmware_version: Some(snapshot.firmware_version.clone()),
            room_id: None,
            entity_ids: entity_ids.clone(),
            identifiers: vec![protocol_identifier("mac", &snapshot.mac_address)?],
            health: Health::Online,
            metadata: vec![
                Metadata::new("tasmota.topic", snapshot.topic.clone()),
                Metadata::new("tasmota.hostname", snapshot.hostname.clone()),
                Metadata::new("tasmota.ip_address", snapshot.ip_address.clone()),
            ],
        })?;
        for entity in entities {
            runtime.upsert_entity(entity)?;
        }
        self.bindings = bindings;
        Ok(InstalledTasmotaDevice {
            bridge_id,
            device_id,
            entity_ids,
        })
    }

    pub fn dispatch_command(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        principal_id: AgentId,
        request: RuntimeCommandToolRequest,
        now_ms: u64,
    ) -> Result<CommandResult, TasmotaError> {
        let binding = self
            .bindings
            .get(&request.entity_id)
            .cloned()
            .ok_or_else(|| TasmotaError::UnknownEntity(request.entity_id.clone()))?;
        let command = native_command(&binding, &request)?;
        let result = runtime.execute_command_tool(principal_id, request.clone(), now_ms)?;
        self.client.command(&command)?;
        let verified = self.client.inspect()?;
        verify_command(&binding, &request, &verified)?;
        self.install_snapshot(runtime, &verified, now_ms)?;
        Ok(result)
    }
}

fn authorize(
    runtime: &mut SmartHomeRuntime,
    principal_id: AgentId,
    tool: SmartHomeTool,
    now_ms: u64,
) -> Result<(), TasmotaError> {
    let decision = runtime.authorize_tool_for_principal(principal_id.clone(), tool, now_ms);
    if decision.missing_capabilities.is_empty() {
        Ok(())
    } else {
        Err(TasmotaError::Runtime(RuntimeError::UnauthorizedTool {
            principal_id,
            tool,
            missing_capabilities: decision.missing_capabilities,
        }))
    }
}

fn parse_snapshot(value: &JsonValue) -> Result<TasmotaSnapshot, TasmotaError> {
    let status = object_field(value, "Status")?;
    let firmware = object_field(value, "StatusFWR")?;
    let network = object_field(value, "StatusNET")?;
    let state = object_field(value, "StatusSTS")?;
    let device_name = string_field(status, "DeviceName")
        .unwrap_or("Tasmota")
        .to_string();
    let friendly_names = status
        .get("FriendlyName")
        .and_then(JsonValue::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(JsonValue::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    let mut outputs = BTreeMap::new();
    for (key, value) in state {
        if let Some(index) = power_index(key) {
            if let Some(on) = parse_on_off(value) {
                outputs.insert(
                    index,
                    TasmotaOutput {
                        index,
                        on,
                        is_light: false,
                        brightness: None,
                        hue: None,
                        saturation: None,
                        color_temperature_mirek: None,
                    },
                );
            }
        }
    }
    if let Some(primary) = outputs.get_mut(&1) {
        primary.brightness = state
            .get("Dimmer")
            .and_then(JsonValue::as_u64)
            .and_then(|value| u8::try_from(value).ok())
            .filter(|value| *value <= 100);
        if let Some((hue, saturation, brightness)) = state
            .get("HSBColor")
            .and_then(JsonValue::as_str)
            .and_then(parse_hsb)
        {
            primary.hue = Some(hue);
            primary.saturation = Some(saturation);
            primary.brightness = primary.brightness.or(Some(brightness));
        }
        primary.color_temperature_mirek = state.get("CT").and_then(JsonValue::as_i64);
        primary.is_light = primary.brightness.is_some()
            || primary.hue.is_some()
            || primary.color_temperature_mirek.is_some();
    }
    if outputs.is_empty() {
        return Err(TasmotaError::NoOutputs);
    }

    let sensors = value
        .get("StatusSNS")
        .and_then(JsonValue::as_object)
        .map(|values| {
            values
                .iter()
                .filter(|(name, value)| name.as_str() != "Time" && value.is_object())
                .filter_map(|(name, value)| {
                    json_to_value(value).map(|value| TasmotaSensor {
                        name: name.clone(),
                        value,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(TasmotaSnapshot {
        device_name,
        friendly_names,
        module: status
            .get("Module")
            .map(json_scalar_text)
            .unwrap_or_else(|| "Unknown".to_string()),
        topic: string_field(status, "Topic")
            .unwrap_or_default()
            .to_string(),
        firmware_version: string_field(firmware, "Version")
            .unwrap_or_default()
            .to_string(),
        hostname: string_field(network, "Hostname")
            .unwrap_or_default()
            .to_string(),
        ip_address: string_field(network, "IPAddress")
            .unwrap_or_default()
            .to_string(),
        mac_address: string_field(network, "Mac")
            .ok_or(TasmotaError::MissingField("StatusNET.Mac"))?
            .to_string(),
        outputs: outputs.into_values().collect(),
        sensors,
    })
}

fn native_command(
    binding: &OutputBinding,
    request: &RuntimeCommandToolRequest,
) -> Result<String, TasmotaError> {
    let power = if binding.index == 1 {
        "Power".to_string()
    } else {
        format!("Power{}", binding.index)
    };
    match request.command_type {
        CommandType::TurnOn => Ok(format!("{power} ON")),
        CommandType::TurnOff => Ok(format!("{power} OFF")),
        CommandType::SetBrightness if binding.is_light => {
            let Value::Percentage(value) = request.arguments else {
                return invalid_arguments(request.command_type, "percentage from 0 through 100");
            };
            Ok(format!("Dimmer {value}"))
        }
        CommandType::SetColor if binding.is_light => {
            let channels = rgb_channels(&request.arguments, request.command_type)?;
            let (hue, saturation) = rgb_to_hsv(channels);
            Ok(format!(
                "HSBColor {hue},{saturation},{}",
                binding.brightness.unwrap_or(100)
            ))
        }
        CommandType::SetColorTemperature if binding.is_light => {
            let Value::Integer(mirek) = request.arguments else {
                return invalid_arguments(
                    request.command_type,
                    "integer mirek from 153 through 500",
                );
            };
            if !(153..=500).contains(&mirek) {
                return invalid_arguments(
                    request.command_type,
                    "integer mirek from 153 through 500",
                );
            }
            Ok(format!("CT {mirek}"))
        }
        _ => Err(TasmotaError::UnsupportedCommand {
            entity_id: binding.entity_id.clone(),
            command_type: request.command_type,
        }),
    }
}

fn verify_command(
    binding: &OutputBinding,
    request: &RuntimeCommandToolRequest,
    snapshot: &TasmotaSnapshot,
) -> Result<(), TasmotaError> {
    let output = snapshot
        .outputs
        .iter()
        .find(|output| output.index == binding.index)
        .ok_or_else(|| TasmotaError::VerificationFailed("output disappeared".to_string()))?;
    let matches = match request.command_type {
        CommandType::TurnOn => output.on,
        CommandType::TurnOff => !output.on,
        CommandType::SetBrightness => {
            matches!(request.arguments, Value::Percentage(value) if output.brightness == Some(value))
        }
        CommandType::SetColor => {
            let (hue, saturation) =
                rgb_to_hsv(rgb_channels(&request.arguments, request.command_type)?);
            output.hue.is_some_and(|value| (value - hue).abs() <= 1)
                && output
                    .saturation
                    .is_some_and(|value| (value - saturation).abs() <= 1)
        }
        CommandType::SetColorTemperature => {
            matches!(request.arguments, Value::Integer(value) if output.color_temperature_mirek == Some(value))
        }
        _ => false,
    };
    if matches {
        Ok(())
    } else {
        Err(TasmotaError::VerificationFailed(format!(
            "output {} did not confirm {:?}",
            binding.index, request.command_type
        )))
    }
}

fn output_state(output: &TasmotaOutput) -> Value {
    let mut fields = vec![("on".to_string(), Value::Bool(output.on))];
    if let Some(value) = output.brightness {
        fields.push(("brightness".to_string(), Value::Percentage(value)));
    }
    if let (Some(hue), Some(saturation)) = (output.hue, output.saturation) {
        fields.push(("hue".to_string(), Value::Integer(hue)));
        fields.push(("saturation".to_string(), Value::Integer(saturation)));
        let rgb = hsv_to_rgb(hue, saturation, 100);
        fields.push((
            "color".to_string(),
            Value::Array(
                rgb.into_iter()
                    .map(|channel| Value::Integer(i64::from(channel)))
                    .collect(),
            ),
        ));
    }
    if let Some(value) = output.color_temperature_mirek {
        fields.push(("color_temperature_mirek".to_string(), Value::Integer(value)));
    }
    Value::Object(fields)
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

fn display_name(snapshot: &TasmotaSnapshot) -> String {
    if snapshot.device_name.trim().is_empty() {
        "Tasmota".to_string()
    } else {
        snapshot.device_name.clone()
    }
}

fn object_field<'a>(
    value: &'a JsonValue,
    field: &'static str,
) -> Result<&'a JsonMap<String, JsonValue>, TasmotaError> {
    value
        .get(field)
        .and_then(JsonValue::as_object)
        .ok_or(TasmotaError::MissingField(field))
}

fn string_field<'a>(object: &'a JsonMap<String, JsonValue>, field: &str) -> Option<&'a str> {
    object.get(field).and_then(JsonValue::as_str)
}

fn json_scalar_text(value: &JsonValue) -> String {
    value
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| value.to_string())
}

fn power_index(key: &str) -> Option<u8> {
    if key == "POWER" {
        Some(1)
    } else {
        key.strip_prefix("POWER")?
            .parse::<u8>()
            .ok()
            .filter(|value| *value > 0)
    }
}

fn parse_on_off(value: &JsonValue) -> Option<bool> {
    value.as_bool().or_else(|| {
        value.as_str().and_then(|value| match value {
            "ON" | "1" => Some(true),
            "OFF" | "0" => Some(false),
            _ => None,
        })
    })
}

fn parse_hsb(value: &str) -> Option<(i64, i64, u8)> {
    let mut parts = value.split(',');
    let hue = parts.next()?.parse::<i64>().ok()?;
    let saturation = parts.next()?.parse::<i64>().ok()?;
    let brightness = parts.next()?.parse::<u8>().ok()?;
    if parts.next().is_some()
        || !(0..=360).contains(&hue)
        || !(0..=100).contains(&saturation)
        || brightness > 100
    {
        return None;
    }
    Some((hue, saturation, brightness))
}

fn json_to_value(value: &JsonValue) -> Option<Value> {
    match value {
        JsonValue::Null => Some(Value::Null),
        JsonValue::Bool(value) => Some(Value::Bool(*value)),
        JsonValue::Number(value) => value
            .as_i64()
            .map(Value::Integer)
            .or_else(|| value.as_f64().map(Value::Number)),
        JsonValue::String(value) => Some(Value::Text(value.clone())),
        JsonValue::Array(values) => Some(Value::Array(
            values.iter().filter_map(json_to_value).collect(),
        )),
        JsonValue::Object(values) => Some(Value::Object(
            values
                .iter()
                .filter_map(|(key, value)| json_to_value(value).map(|value| (key.clone(), value)))
                .collect(),
        )),
    }
}

fn command_path(command: &str, credentials: Option<&TasmotaCredentials>) -> Zeroizing<String> {
    let mut parameters = Vec::new();
    if let Some(credentials) = credentials {
        parameters.push(format!("user={}", percent_encode(&credentials.username)));
        parameters.push(format!(
            "password={}",
            percent_encode(&credentials.password)
        ));
    }
    parameters.push(format!("cmnd={}", percent_encode(command)));
    Zeroizing::new(format!("/cm?{}", parameters.join("&")))
}

fn percent_encode(value: &str) -> String {
    let mut output = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            output.push(char::from(byte));
        } else {
            output.push_str(&format!("%{byte:02X}"));
        }
    }
    output
}

fn rgb_channels(arguments: &Value, command_type: CommandType) -> Result<[u8; 3], TasmotaError> {
    let Value::Array(values) = arguments else {
        return invalid_arguments(
            command_type,
            "RGB array with three integer channels from 0 through 255",
        );
    };
    if values.len() != 3 {
        return invalid_arguments(
            command_type,
            "RGB array with three integer channels from 0 through 255",
        );
    }
    let channels = values
        .iter()
        .map(|value| match value {
            Value::Integer(channel) => u8::try_from(*channel).ok(),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()
        .ok_or(TasmotaError::InvalidCommandArguments {
            command_type,
            expected: "RGB array with three integer channels from 0 through 255",
        })?;
    Ok([channels[0], channels[1], channels[2]])
}

fn rgb_to_hsv([red, green, blue]: [u8; 3]) -> (i64, i64) {
    let red = f64::from(red) / 255.0;
    let green = f64::from(green) / 255.0;
    let blue = f64::from(blue) / 255.0;
    let maximum = red.max(green).max(blue);
    let minimum = red.min(green).min(blue);
    let delta = maximum - minimum;
    let hue = if delta == 0.0 {
        0.0
    } else if maximum == red {
        60.0 * ((green - blue) / delta).rem_euclid(6.0)
    } else if maximum == green {
        60.0 * (((blue - red) / delta) + 2.0)
    } else {
        60.0 * (((red - green) / delta) + 4.0)
    };
    let saturation = if maximum == 0.0 { 0.0 } else { delta / maximum };
    (hue.round() as i64, (saturation * 100.0).round() as i64)
}

fn hsv_to_rgb(hue: i64, saturation: i64, value: i64) -> [u8; 3] {
    let hue = hue.rem_euclid(360) as f64;
    let saturation = saturation.clamp(0, 100) as f64 / 100.0;
    let value = value.clamp(0, 100) as f64 / 100.0;
    let chroma = value * saturation;
    let x = chroma * (1.0 - ((hue / 60.0).rem_euclid(2.0) - 1.0).abs());
    let m = value - chroma;
    let (red, green, blue) = match hue {
        value if value < 60.0 => (chroma, x, 0.0),
        value if value < 120.0 => (x, chroma, 0.0),
        value if value < 180.0 => (0.0, chroma, x),
        value if value < 240.0 => (0.0, x, chroma),
        value if value < 300.0 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };
    [
        ((red + m) * 255.0).round() as u8,
        ((green + m) * 255.0).round() as u8,
        ((blue + m) * 255.0).round() as u8,
    ]
}

fn invalid_arguments<T>(
    command_type: CommandType,
    expected: &'static str,
) -> Result<T, TasmotaError> {
    Err(TasmotaError::InvalidCommandArguments {
        command_type,
        expected,
    })
}

fn protocol_identifier(kind: &str, value: &str) -> Result<ProtocolIdentifier, TasmotaError> {
    ProtocolIdentifier::new(ProtocolFamily::Vendor(PROTOCOL_ID.to_string()), kind, value)
        .map_err(|error| TasmotaError::Validation(error.to_string()))
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

fn encode_http_request(url: &Url, plan: &LocalHttpRequestPlan) -> Result<Vec<u8>, TasmotaError> {
    let host = url
        .host
        .as_deref()
        .ok_or(TasmotaError::MissingField("URL host"))?;
    let port = url
        .effective_port()
        .ok_or(TasmotaError::MissingField("URL port"))?;
    let target = if url.path.is_empty() {
        "/".to_string()
    } else if let Some(query) = &url.query {
        format!("{}?{query}", url.path)
    } else {
        url.path.clone()
    };
    if has_unsafe_http_text(&target) || has_unsafe_http_text(host) {
        return Err(TasmotaError::Validation(
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
            return Err(TasmotaError::Validation(
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

fn connect_tcp(host: &str, port: u16, timeout: Duration) -> Result<TcpStream, TasmotaError> {
    let addresses = (host, port)
        .to_socket_addrs()
        .map_err(|error| TasmotaError::Io(error.to_string()))?
        .collect::<Vec<_>>();
    let mut last_error = None;
    for address in addresses {
        match TcpStream::connect_timeout(&address, timeout) {
            Ok(stream) => {
                stream
                    .set_read_timeout(Some(timeout))
                    .map_err(|error| TasmotaError::Io(error.to_string()))?;
                stream
                    .set_write_timeout(Some(timeout))
                    .map_err(|error| TasmotaError::Io(error.to_string()))?;
                return Ok(stream);
            }
            Err(error) => last_error = Some(error),
        }
    }
    Err(TasmotaError::Io(
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

fn read_bounded(reader: &mut dyn Read, maximum: usize) -> Result<Vec<u8>, TasmotaError> {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| TasmotaError::Io(error.to_string()))?;
        if read == 0 {
            break;
        }
        if read > maximum.saturating_sub(bytes.len()) {
            return Err(TasmotaError::ResponseTooLarge { limit: maximum });
        }
        bytes.extend_from_slice(&buffer[..read]);
    }
    Ok(bytes)
}

fn decode_http_response(bytes: &[u8], maximum: usize) -> Result<Vec<u8>, TasmotaError> {
    let parsed = parse_response_head(bytes)
        .map_err(|error: Http1ParseError| TasmotaError::Http(error.to_string()))?;
    if !(200..300).contains(&parsed.head.status) {
        return Err(TasmotaError::HttpStatus(parsed.head.status));
    }
    let input = &bytes[parsed.body_offset..];
    let body = match parsed.body_kind {
        BodyKind::None => Vec::new(),
        BodyKind::ContentLength(expected) => {
            if input.len() < expected {
                return Err(TasmotaError::TruncatedBody {
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
        return Err(TasmotaError::ResponseTooLarge { limit: maximum });
    }
    Ok(body)
}

fn decode_chunked(input: &[u8], maximum: usize) -> Result<Vec<u8>, TasmotaError> {
    let mut cursor = 0usize;
    let mut output = Vec::new();
    loop {
        let offset = input[cursor..]
            .windows(2)
            .position(|window| window == b"\r\n")
            .ok_or_else(|| TasmotaError::Http("missing chunk-size terminator".to_string()))?;
        let end = cursor + offset;
        let size_text = std::str::from_utf8(&input[cursor..end])
            .map_err(|_| TasmotaError::Http("chunk size is not ASCII".to_string()))?
            .split(';')
            .next()
            .unwrap_or_default();
        let size = usize::from_str_radix(size_text.trim(), 16)
            .map_err(|_| TasmotaError::Http("invalid chunk size".to_string()))?;
        cursor = end + 2;
        if size == 0 {
            return Ok(output);
        }
        if size > maximum.saturating_sub(output.len()) {
            return Err(TasmotaError::ResponseTooLarge { limit: maximum });
        }
        let chunk_end = cursor
            .checked_add(size)
            .ok_or_else(|| TasmotaError::Http("chunk size overflow".to_string()))?;
        if input.len() < chunk_end + 2 || &input[chunk_end..chunk_end + 2] != b"\r\n" {
            return Err(TasmotaError::Http("truncated chunk".to_string()));
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

    const SNAPSHOT: &str = r#"{"Status":{"Module":18,"DeviceName":"Workshop Lamp","FriendlyName":["Bench Lamp"],"Topic":"tasmota_workshop"},"StatusFWR":{"Version":"14.6.0"},"StatusNET":{"Hostname":"tasmota-workshop","IPAddress":"192.0.2.40","Mac":"AA:BB:CC:DD:EE:FF"},"StatusSTS":{"POWER":"ON","Dimmer":40,"HSBColor":"30,50,40","CT":250},"StatusSNS":{"Time":"2026-08-02T12:00:00","ENERGY":{"Power":12.5,"Voltage":121,"Current":0.1}}}"#;
    const VERIFIED: &str = r#"{"Status":{"Module":18,"DeviceName":"Workshop Lamp","FriendlyName":["Bench Lamp"],"Topic":"tasmota_workshop"},"StatusFWR":{"Version":"14.6.0"},"StatusNET":{"Hostname":"tasmota-workshop","IPAddress":"192.0.2.40","Mac":"AA:BB:CC:DD:EE:FF"},"StatusSTS":{"POWER":"ON","Dimmer":40,"HSBColor":"210,67,40","CT":250},"StatusSNS":{"Time":"2026-08-02T12:00:01","ENERGY":{"Power":12.5,"Voltage":121,"Current":0.1}}}"#;

    fn response(body: &str) -> Vec<u8> {
        format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).into_bytes()
    }

    fn start_server(
        responses: Vec<Vec<u8>>,
    ) -> (u16, Arc<Mutex<Vec<String>>>, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&requests);
        let handle = thread::spawn(move || {
            for payload in responses {
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
                    if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
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

    fn config(port: u16, authenticated: bool) -> TasmotaConfig {
        let config = TasmotaConfig::new(
            BridgeId::trusted("tasmota.test"),
            format!("http://127.0.0.1:{port}"),
        )
        .unwrap();
        if authenticated {
            config.with_credentials(VaultRef::trusted("vault:tasmota/workshop"))
        } else {
            config
        }
    }

    fn grant(runtime: &mut SmartHomeRuntime, principal: &AgentId) {
        let _ = runtime.registry_mut().upsert_capability_grant(
            CapabilityGrant::for_all_smart_home(
                CapabilityGrantId::trusted("grant:tasmota-test"),
                principal.clone(),
                PrivilegeTier::LowRisk,
                "test",
                1_000,
            )
            .with_expiry(20_000),
        );
    }

    #[test]
    fn tasmota_mdns_advertisement_becomes_verified_http_discovery() {
        let advertisement = MdnsAdvertisement::new(
            MDNS_SERVICE_TYPE,
            "tasmota-workshop",
            "tasmota-workshop.local",
            80,
            1_000,
        )
        .unwrap()
        .with_address("192.0.2.40")
        .unwrap()
        .with_txt("md", "tasmota")
        .unwrap()
        .with_txt("id", "AABBCCDDEEFF")
        .unwrap();
        let record = discovery_record(&advertisement).unwrap();
        assert_eq!(record.native_bridge_id, "aabbccddeeff");
        assert_eq!(record.address.as_deref(), Some("http://192.0.2.40:80"));
        assert_eq!(record.confidence, DiscoveryConfidence::Verified);

        let other =
            MdnsAdvertisement::new(MDNS_SERVICE_TYPE, "printer", "printer.local", 80, 1_000)
                .unwrap();
        assert!(discovery_record(&other).is_err());
    }

    #[test]
    fn real_tcp_authenticated_inspection_installs_light_and_sensor() {
        let (port, requests, handle) = start_server(vec![response(SNAPSHOT)]);
        let client = TasmotaClient::new(
            config(port, true),
            Some(TasmotaCredentials::new("admin", "s e c r e t").unwrap()),
            TasmotaLanTransport::default(),
        )
        .unwrap();
        let mut integration = TasmotaRuntimeIntegration::new(client);
        let principal = AgentId::trusted("agent:tasmota-read");
        let mut runtime = SmartHomeRuntime::new();
        grant(&mut runtime, &principal);
        let installed = integration
            .inspect_and_install_authorized(&mut runtime, principal, 5_000)
            .unwrap();
        handle.join().unwrap();
        assert_eq!(installed.entity_ids.len(), 2);
        let light = runtime
            .registry()
            .entity(&EntityId::trusted("tasmota:aa-bb-cc-dd-ee-ff:output:1"))
            .unwrap();
        assert_eq!(light.kind, EntityKind::Light);
        assert_eq!(light.capabilities.len(), 4);
        let bridge = runtime.registry().bridge(&installed.bridge_id).unwrap();
        assert_eq!(
            bridge.auth_ref.as_ref().map(VaultRef::as_str),
            Some("vault:tasmota/workshop")
        );
        assert!(!format!("{runtime:?}").contains("s e c r e t"));
        let request = &requests.lock().unwrap()[0];
        assert!(request.contains("user=admin"));
        assert!(request.contains("password=s%20e%20c%20r%20e%20t"));
        assert!(request.contains("cmnd=Status%200"));
    }

    #[test]
    fn authorized_color_command_crosses_http_and_confirms_state() {
        let (port, requests, handle) = start_server(vec![
            response(SNAPSHOT),
            response(r#"{"HSBColor":"210,67,40"}"#),
            response(VERIFIED),
        ]);
        let client =
            TasmotaClient::new(config(port, false), None, TasmotaLanTransport::default()).unwrap();
        let mut integration = TasmotaRuntimeIntegration::new(client);
        let principal = AgentId::trusted("agent:tasmota-command");
        let mut runtime = SmartHomeRuntime::new();
        grant(&mut runtime, &principal);
        let installed = integration
            .inspect_and_install_authorized(&mut runtime, principal.clone(), 5_000)
            .unwrap();
        let entity_id = installed
            .entity_ids
            .iter()
            .find(|id| id.as_str().ends_with("output:1"))
            .unwrap()
            .clone();
        let result = integration
            .dispatch_command(
                &mut runtime,
                principal,
                RuntimeCommandToolRequest::new(
                    entity_id.clone(),
                    CommandType::SetColor,
                    Value::Array(vec![
                        Value::Integer(84),
                        Value::Integer(170),
                        Value::Integer(255),
                    ]),
                ),
                6_000,
            )
            .unwrap();
        handle.join().unwrap();
        assert_eq!(result.status, smart_home_core::CommandStatus::Accepted);
        let requests = requests.lock().unwrap();
        assert!(requests[1].contains("cmnd=HSBColor%20210%2C67%2C40"));
        assert!(requests[2].contains("cmnd=Status%200"));
        assert_eq!(
            runtime.registry().state(&entity_id).unwrap().confidence,
            StateConfidence::Confirmed
        );
    }

    #[derive(Debug)]
    struct CountingTransport(Arc<AtomicUsize>);

    impl TasmotaTransport for CountingTransport {
        fn execute(&mut self, _plan: &LocalHttpRequestPlan) -> Result<Vec<u8>, TasmotaError> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(Vec::new())
        }
    }

    #[test]
    fn denied_read_reaches_no_transport_or_credentials() {
        let calls = Arc::new(AtomicUsize::new(0));
        let client = TasmotaClient::new(
            TasmotaConfig::new(BridgeId::trusted("tasmota.denied"), "http://127.0.0.1")
                .unwrap()
                .with_credentials(VaultRef::trusted("vault:tasmota/denied")),
            Some(TasmotaCredentials::new("admin", "secret").unwrap()),
            CountingTransport(Arc::clone(&calls)),
        )
        .unwrap();
        let mut integration = TasmotaRuntimeIntegration::new(client);
        assert!(matches!(
            integration.inspect_and_install_authorized(
                &mut SmartHomeRuntime::new(),
                AgentId::trusted("agent:denied"),
                5_000,
            ),
            Err(TasmotaError::Runtime(_))
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn parser_supports_multiple_relays_and_command_ranges() {
        let value: JsonValue = serde_json::from_str(
            r#"{"Status":{"DeviceName":"Strip","FriendlyName":["One","Two"]},"StatusFWR":{"Version":"1"},"StatusNET":{"Mac":"001122334455"},"StatusSTS":{"POWER1":"ON","POWER2":"OFF"}}"#,
        )
        .unwrap();
        let snapshot = parse_snapshot(&value).unwrap();
        assert_eq!(snapshot.outputs.len(), 2);
        assert!(snapshot.outputs[0].on);
        assert!(!snapshot.outputs[1].on);
        let binding = OutputBinding {
            entity_id: EntityId::trusted("tasmota:test:output:1"),
            index: 1,
            is_light: true,
            brightness: Some(40),
        };
        let request = RuntimeCommandToolRequest::new(
            binding.entity_id.clone(),
            CommandType::SetColorTemperature,
            Value::Integer(250),
        );
        assert_eq!(native_command(&binding, &request).unwrap(), "CT 250");
        assert!(matches!(
            decode_http_response(&response("{}"), 1),
            Err(TasmotaError::ResponseTooLarge { limit: 1 })
        ));
    }
}
