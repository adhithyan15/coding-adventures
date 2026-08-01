//! Production Shelly Gen2 and Gen3 local integration for D23.

#![forbid(unsafe_code)]

use http1::{parse_response_head, Http1ParseError};
use http_core::BodyKind;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map as JsonMap, Value as JsonValue};
use smart_home_core::{
    AgentId, Bridge, BridgeId, BridgeTransport, Capability, CapabilityId, CapabilityMode,
    CommandResult, CommandType, Device, DeviceId, Entity, EntityId, EntityKind, Health,
    IntegrationId, Metadata, ProtocolFamily, ProtocolIdentifier, StateConfidence, StateSnapshot,
    StateSource, Value, ValueKind,
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
pub const INTEGRATION_ID: &str = "shelly";
pub const MDNS_SERVICE_TYPE: &str = "_shelly._tcp.local";
pub const DEFAULT_PORT: u16 = 80;
pub const DEFAULT_MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
pub const DEFAULT_MAX_DISCOVERY_RESPONSES: usize = 64;

#[derive(Debug)]
pub enum ShellyError {
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
    AuthenticationRequired {
        device_id: String,
    },
    Rpc {
        code: i64,
        message: String,
    },
    MissingField(&'static str),
    UnsupportedGeneration(u8),
    NoSupportedComponents,
    UnknownEntity(EntityId),
    UnsupportedCommand {
        entity_id: EntityId,
        command_type: CommandType,
    },
    InvalidCommandArguments {
        command_type: CommandType,
        expected: &'static str,
    },
    Runtime(RuntimeError),
}

impl fmt::Display for ShellyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => write!(formatter, "invalid Shelly input: {message}"),
            Self::Discovery(message) => write!(formatter, "Shelly discovery failed: {message}"),
            Self::LocalHttp(error) => error.fmt(formatter),
            Self::Url(error) => write!(formatter, "invalid Shelly URL: {error}"),
            Self::Io(message) => write!(formatter, "Shelly LAN I/O failed: {message}"),
            Self::Http(message) => write!(formatter, "invalid Shelly HTTP response: {message}"),
            Self::HttpStatus(status) => write!(formatter, "Shelly endpoint returned HTTP {status}"),
            Self::ResponseTooLarge { limit } => {
                write!(formatter, "Shelly response exceeds {limit} bytes")
            }
            Self::TruncatedBody { expected, actual } => write!(
                formatter,
                "Shelly response is truncated: expected {expected} bytes, got {actual}"
            ),
            Self::Json(error) => write!(formatter, "invalid Shelly JSON: {error}"),
            Self::AuthenticationRequired { device_id } => {
                write!(
                    formatter,
                    "Shelly device {device_id} requires authentication"
                )
            }
            Self::Rpc { code, message } => write!(formatter, "Shelly RPC {code}: {message}"),
            Self::MissingField(field) => write!(formatter, "Shelly response is missing {field}"),
            Self::UnsupportedGeneration(generation) => {
                write!(formatter, "Shelly generation {generation} is not supported")
            }
            Self::NoSupportedComponents => {
                formatter.write_str("Shelly device has no supported components")
            }
            Self::UnknownEntity(entity_id) => {
                write!(formatter, "unknown Shelly entity {entity_id}")
            }
            Self::UnsupportedCommand {
                entity_id,
                command_type,
            } => {
                write!(
                    formatter,
                    "Shelly entity {entity_id} does not support {command_type:?}"
                )
            }
            Self::InvalidCommandArguments {
                command_type,
                expected,
            } => {
                write!(
                    formatter,
                    "invalid {command_type:?} arguments; expected {expected}"
                )
            }
            Self::Runtime(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for ShellyError {}

impl From<LocalHttpError> for ShellyError {
    fn from(error: LocalHttpError) -> Self {
        Self::LocalHttp(error)
    }
}

impl From<UrlError> for ShellyError {
    fn from(error: UrlError) -> Self {
        Self::Url(error)
    }
}

impl From<serde_json::Error> for ShellyError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<RuntimeError> for ShellyError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ShellyDeviceInfo {
    pub id: String,
    pub mac: String,
    pub model: String,
    pub gen: u8,
    #[serde(default)]
    pub fw_id: String,
    #[serde(default)]
    pub ver: String,
    #[serde(default)]
    pub app: String,
    #[serde(default)]
    pub profile: Option<String>,
    #[serde(default)]
    pub auth_en: bool,
    #[serde(default)]
    pub auth_domain: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShellySnapshot {
    pub device_info: ShellyDeviceInfo,
    pub components: BTreeMap<String, JsonValue>,
}

impl ShellySnapshot {
    pub fn component_count(&self) -> usize {
        self.components.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellyDeviceConfig {
    pub bridge_id: BridgeId,
    pub host: String,
    pub port: u16,
    pub timeout: Duration,
}

impl ShellyDeviceConfig {
    pub fn new(bridge_id: BridgeId, host: impl Into<String>) -> Result<Self, ShellyError> {
        let host = host.into();
        if host.trim().is_empty() || has_unsafe_http_text(&host) {
            return Err(ShellyError::Validation(
                "host must be non-empty safe HTTP text".to_string(),
            ));
        }
        Ok(Self {
            bridge_id,
            host,
            port: DEFAULT_PORT,
            timeout: Duration::from_secs(5),
        })
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
    pub fn endpoint(&self) -> Result<LocalHttpEndpoint, ShellyError> {
        Ok(LocalHttpEndpoint::new(
            IntegrationId::trusted(INTEGRATION_ID),
            self.bridge_id.clone(),
            LocalHttpScheme::Http,
            self.host.clone(),
        )?
        .with_port(self.port)
        .with_metadata(Metadata::new("http.profile", "shelly.rpc.gen2plus")))
    }
}

pub fn scan_mdns_ipv4(
    discovered_at_ms: u64,
    timeout: Duration,
) -> Result<MdnsScanResult, ShellyError> {
    let options = MdnsScanOptions::new(MDNS_SERVICE_TYPE, discovered_at_ms, timeout)
        .map_err(|error| ShellyError::Discovery(error.to_string()))?
        .with_max_responses(DEFAULT_MAX_DISCOVERY_RESPONSES);
    run_mdns_ipv4_scan(options).map_err(|error| ShellyError::Discovery(error.to_string()))
}

pub fn discovery_record(advertisement: &MdnsAdvertisement) -> Result<DiscoveryRecord, ShellyError> {
    if advertisement.service_type.trim_end_matches('.') != MDNS_SERVICE_TYPE.trim_end_matches('.') {
        return Err(ShellyError::Validation(format!(
            "unexpected mDNS service type `{}`",
            advertisement.service_type
        )));
    }
    let generation = advertisement
        .txt_value("gen")
        .ok_or(ShellyError::MissingField("mDNS TXT gen"))?
        .parse::<u8>()
        .map_err(|_| ShellyError::Validation("mDNS TXT gen must be an integer".to_string()))?;
    if !matches!(generation, 2 | 3) {
        return Err(ShellyError::UnsupportedGeneration(generation));
    }
    let native_id = stable_component(&advertisement.instance_name);
    let mut record = DiscoveryRecord::new(
        IntegrationId::trusted(INTEGRATION_ID),
        ProtocolFamily::Vendor(INTEGRATION_ID.to_string()),
        native_id,
        DiscoverySource::Mdns,
        BridgeTransport::LanHttp,
        advertisement.discovered_at_ms,
    )
    .map_err(|error| ShellyError::Discovery(error.to_string()))?
    .with_display_name(&advertisement.instance_name)
    .with_address(advertisement.endpoint_with_scheme("http"))
    .with_confidence(DiscoveryConfidence::Verified)
    .with_pairing_requirement(PairingRequirement::Unknown)
    .with_metadata("shelly.generation", generation.to_string())
    .with_metadata("smart_home.discovery.service_type", MDNS_SERVICE_TYPE);
    if let Some(model) = advertisement.txt_value("model") {
        record = record.with_hardware_model(model);
    }
    Ok(record)
}

pub trait ShellyTransport {
    fn execute(&mut self, plan: &LocalHttpRequestPlan) -> Result<Vec<u8>, ShellyError>;
}

#[derive(Debug, Clone)]
pub struct ShellyLanTransport {
    pub maximum_response_bytes: usize,
}

impl Default for ShellyLanTransport {
    fn default() -> Self {
        Self {
            maximum_response_bytes: DEFAULT_MAX_RESPONSE_BYTES,
        }
    }
}

impl ShellyTransport for ShellyLanTransport {
    fn execute(&mut self, plan: &LocalHttpRequestPlan) -> Result<Vec<u8>, ShellyError> {
        let url = Url::parse(&plan.url)?;
        if url.scheme != "http" {
            return Err(ShellyError::Validation(
                "Shelly transport only permits local HTTP".to_string(),
            ));
        }
        let host = url
            .host
            .as_deref()
            .ok_or(ShellyError::MissingField("URL host"))?;
        let port = url
            .effective_port()
            .ok_or(ShellyError::MissingField("URL port"))?;
        let timeout = Duration::from_millis(plan.timeout_ms.max(1));
        let request = encode_http_request(&url, plan)?;
        let mut stream = connect_tcp(host, port, timeout)?;
        stream
            .write_all(&request)
            .map_err(|error| ShellyError::Io(error.to_string()))?;
        let bytes = read_bounded(&mut stream, self.maximum_response_bytes)?;
        decode_http_response(&bytes, self.maximum_response_bytes)
    }
}

#[derive(Debug)]
pub struct ShellyClient<T> {
    config: ShellyDeviceConfig,
    endpoint: LocalHttpEndpoint,
    transport: T,
}

impl<T: ShellyTransport> ShellyClient<T> {
    pub fn new(config: ShellyDeviceConfig, transport: T) -> Result<Self, ShellyError> {
        let endpoint = config.endpoint()?;
        Ok(Self {
            config,
            endpoint,
            transport,
        })
    }

    pub fn config(&self) -> &ShellyDeviceConfig {
        &self.config
    }
    pub fn transport(&self) -> &T {
        &self.transport
    }
    pub fn transport_mut(&mut self) -> &mut T {
        &mut self.transport
    }

    pub fn inspect(&mut self) -> Result<ShellySnapshot, ShellyError> {
        let info: ShellyDeviceInfo = serde_json::from_slice(&self.get("/shelly")?)?;
        if !matches!(info.gen, 2 | 3) {
            return Err(ShellyError::UnsupportedGeneration(info.gen));
        }
        if info.auth_en {
            return Err(ShellyError::AuthenticationRequired { device_id: info.id });
        }
        let status = serde_json::from_slice::<JsonValue>(&self.get("/rpc/Shelly.GetStatus")?)?;
        let components = status
            .as_object()
            .ok_or(ShellyError::MissingField("Shelly.GetStatus result object"))?
            .iter()
            .filter(|(name, _)| name.contains(':'))
            .map(|(name, status)| (name.clone(), status.clone()))
            .collect();
        Ok(ShellySnapshot {
            device_info: info,
            components,
        })
    }

    pub fn call(&mut self, method: &str, params: JsonValue) -> Result<JsonValue, ShellyError> {
        if method.trim().is_empty() || has_unsafe_http_text(method) {
            return Err(ShellyError::Validation(
                "RPC method must be safe non-empty text".to_string(),
            ));
        }
        let body = serde_json::to_vec(&json!({"id": 1, "method": method, "params": params}))?;
        let template = LocalHttpRequestTemplate::new(LocalHttpMethod::Post, "/rpc")?
            .with_accept("application/json")
            .with_content_type("application/json")
            .with_timeout_ms(duration_ms(self.config.timeout));
        let response = self
            .transport
            .execute(&template.plan(&self.endpoint, body)?)?;
        let envelope: RpcEnvelope = serde_json::from_slice(&response)?;
        if let Some(error) = envelope.error {
            return Err(ShellyError::Rpc {
                code: error.code,
                message: error.message,
            });
        }
        Ok(envelope.result.unwrap_or(JsonValue::Null))
    }

    fn get(&mut self, path: &str) -> Result<Vec<u8>, ShellyError> {
        let template = LocalHttpRequestTemplate::new(LocalHttpMethod::Get, path)?
            .with_accept("application/json")
            .with_timeout_ms(duration_ms(self.config.timeout));
        self.transport
            .execute(&template.plan(&self.endpoint, Vec::new())?)
    }
}

#[derive(Debug, Deserialize)]
struct RpcEnvelope {
    #[serde(default)]
    result: Option<JsonValue>,
    #[serde(default)]
    error: Option<RpcError>,
}

#[derive(Debug, Deserialize)]
struct RpcError {
    code: i64,
    message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledShellyDevice {
    pub bridge_id: BridgeId,
    pub device_id: DeviceId,
    pub entity_ids: Vec<EntityId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ComponentKind {
    Switch,
    Light,
    Input,
    Temperature,
    Humidity,
    Energy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ComponentBinding {
    entity_id: EntityId,
    component: String,
    kind: ComponentKind,
}

#[derive(Debug)]
pub struct ShellyRuntimeIntegration<T> {
    client: ShellyClient<T>,
    bindings: BTreeMap<EntityId, ComponentBinding>,
}

impl<T: ShellyTransport> ShellyRuntimeIntegration<T> {
    pub fn new(client: ShellyClient<T>) -> Self {
        Self {
            client,
            bindings: BTreeMap::new(),
        }
    }

    pub fn client(&self) -> &ShellyClient<T> {
        &self.client
    }
    pub fn client_mut(&mut self) -> &mut ShellyClient<T> {
        &mut self.client
    }

    pub fn inspect_and_install(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        observed_at_ms: u64,
    ) -> Result<InstalledShellyDevice, ShellyError> {
        let snapshot = self.client.inspect()?;
        self.install_snapshot(runtime, &snapshot, observed_at_ms)
    }

    pub fn install_snapshot(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        snapshot: &ShellySnapshot,
        observed_at_ms: u64,
    ) -> Result<InstalledShellyDevice, ShellyError> {
        let info = &snapshot.device_info;
        if !matches!(info.gen, 2 | 3) {
            return Err(ShellyError::UnsupportedGeneration(info.gen));
        }
        let native_id = stable_component(&info.id);
        let device_id = DeviceId::trusted(format!("shelly:{native_id}"));
        let bridge_id = self.client.config.bridge_id.clone();
        let mut entities = Vec::new();
        let mut bindings = BTreeMap::new();
        for (component, status) in &snapshot.components {
            if let Some((entity, binding)) =
                project_component(&device_id, &native_id, component, status, observed_at_ms)
            {
                bindings.insert(entity.entity_id.clone(), binding);
                entities.push(entity);
            }
        }
        if entities.is_empty() {
            return Err(ShellyError::NoSupportedComponents);
        }

        let mut bridge = Bridge::new(
            bridge_id.clone(),
            IntegrationId::trusted(INTEGRATION_ID),
            BridgeTransport::LanHttp,
        );
        bridge.address = Some(self.client.config.endpoint()?.origin());
        bridge.hardware_model = Some(info.model.clone());
        bridge.firmware_version = Some(if info.ver.is_empty() {
            info.fw_id.clone()
        } else {
            info.ver.clone()
        });
        bridge.health = Health::Online;
        bridge.last_seen_at_ms = Some(observed_at_ms);
        bridge.identifiers = vec![protocol_identifier(
            "rpc_endpoint",
            &self.client.config.endpoint()?.origin(),
        )?];
        bridge.metadata = vec![
            Metadata::new("shelly.generation", info.gen.to_string()),
            Metadata::new("shelly.app", &info.app),
            Metadata::new("shelly.transport", "http_rpc"),
        ];
        runtime.upsert_bridge(bridge)?;

        let entity_ids = entities
            .iter()
            .map(|entity| entity.entity_id.clone())
            .collect::<Vec<_>>();
        runtime.upsert_device(Device {
            device_id: device_id.clone(),
            bridge_id: bridge_id.clone(),
            manufacturer: "Shelly".to_string(),
            model: info.model.clone(),
            name: if info.app.is_empty() {
                info.id.clone()
            } else {
                info.app.clone()
            },
            serial: Some(info.mac.clone()),
            firmware_version: Some(if info.ver.is_empty() {
                info.fw_id.clone()
            } else {
                info.ver.clone()
            }),
            room_id: None,
            entity_ids: entity_ids.clone(),
            identifiers: vec![
                protocol_identifier("device_id", &info.id)?,
                protocol_identifier("mac", &info.mac)?,
            ],
            health: Health::Online,
            metadata: vec![Metadata::new(
                "shelly.profile",
                info.profile.as_deref().unwrap_or("default"),
            )],
        })?;
        for entity in entities {
            runtime.upsert_entity(entity)?;
        }
        self.bindings = bindings;
        Ok(InstalledShellyDevice {
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
    ) -> Result<CommandResult, ShellyError> {
        let binding = self
            .bindings
            .get(&request.entity_id)
            .cloned()
            .ok_or_else(|| ShellyError::UnknownEntity(request.entity_id.clone()))?;
        let (method, params) = command_rpc(&binding, &request)?;
        let result = runtime.execute_command_tool(principal_id, request, now_ms)?;
        self.client.call(method, params)?;
        Ok(result)
    }
}

fn project_component(
    device_id: &DeviceId,
    native_id: &str,
    component: &str,
    status: &JsonValue,
    observed_at_ms: u64,
) -> Option<(Entity, ComponentBinding)> {
    let (namespace, instance) = component.split_once(':')?;
    let object = status.as_object()?;
    let entity_id = EntityId::trusted(format!(
        "shelly:{native_id}:{namespace}:{}",
        stable_component(instance)
    ));
    let (kind, entity_kind, capabilities, value) = match namespace {
        "switch" => (
            ComponentKind::Switch,
            EntityKind::Switch,
            vec![
                Capability::light_on_off(),
                numeric_capability("sensor.active_power", "W"),
            ],
            component_state(object, &["output", "apower", "voltage", "current"]),
        ),
        "light" => (
            ComponentKind::Light,
            EntityKind::Light,
            vec![Capability::light_on_off(), Capability::light_brightness()],
            component_state(object, &["output", "brightness", "apower"]),
        ),
        "input" => (
            ComponentKind::Input,
            EntityKind::Input,
            vec![Capability::input_button()],
            json_to_state_value(object.get("state").or_else(|| object.get("percent"))?)?,
        ),
        "temperature" => (
            ComponentKind::Temperature,
            EntityKind::Sensor,
            vec![Capability::sensor_temperature()],
            Value::Number(object.get("tC")?.as_f64()?),
        ),
        "humidity" => (
            ComponentKind::Humidity,
            EntityKind::Sensor,
            vec![Capability::sensor_humidity()],
            percentage_value(object.get("rh")?)?,
        ),
        "pm1" | "em1" | "em" => (
            ComponentKind::Energy,
            EntityKind::Sensor,
            vec![
                numeric_capability("sensor.active_power", "W"),
                numeric_capability("sensor.voltage", "V"),
            ],
            component_state(
                object,
                &["apower", "act_power", "voltage", "current", "freq"],
            ),
        ),
        _ => return None,
    };
    let entity = Entity {
        entity_id: entity_id.clone(),
        device_id: device_id.clone(),
        kind: entity_kind,
        name: format!("Shelly {namespace} {instance}"),
        capabilities,
        state: Some(StateSnapshot {
            entity_id: entity_id.clone(),
            value,
            source: StateSource::Poll,
            observed_at_ms,
            received_at_ms: observed_at_ms,
            expires_at_ms: None,
            confidence: StateConfidence::Confirmed,
        }),
        metadata: vec![Metadata::new("shelly.component", component)],
    };
    Some((
        entity,
        ComponentBinding {
            entity_id,
            component: component.to_string(),
            kind,
        },
    ))
}

fn command_rpc(
    binding: &ComponentBinding,
    request: &RuntimeCommandToolRequest,
) -> Result<(&'static str, JsonValue), ShellyError> {
    let id = binding
        .component
        .split_once(':')
        .and_then(|(_, id)| id.parse::<u32>().ok())
        .ok_or_else(|| ShellyError::Validation("component instance must be numeric".to_string()))?;
    match (&binding.kind, request.command_type) {
        (ComponentKind::Switch, CommandType::TurnOn) => {
            Ok(("Switch.Set", json!({"id": id, "on": true})))
        }
        (ComponentKind::Switch, CommandType::TurnOff) => {
            Ok(("Switch.Set", json!({"id": id, "on": false})))
        }
        (ComponentKind::Light, CommandType::TurnOn) => {
            Ok(("Light.Set", json!({"id": id, "on": true})))
        }
        (ComponentKind::Light, CommandType::TurnOff) => {
            Ok(("Light.Set", json!({"id": id, "on": false})))
        }
        (ComponentKind::Light, CommandType::SetBrightness) => {
            let Value::Percentage(brightness) = request.arguments else {
                return Err(ShellyError::InvalidCommandArguments {
                    command_type: request.command_type,
                    expected: "percentage",
                });
            };
            Ok(("Light.Set", json!({"id": id, "brightness": brightness})))
        }
        _ => Err(ShellyError::UnsupportedCommand {
            entity_id: binding.entity_id.clone(),
            command_type: request.command_type,
        }),
    }
}

fn numeric_capability(id: &str, unit: &str) -> Capability {
    Capability::new(
        CapabilityId::trusted(id),
        CapabilityMode::Observe,
        ValueKind::Number,
    )
    .with_unit(unit)
}

fn component_state(object: &JsonMap<String, JsonValue>, fields: &[&str]) -> Value {
    Value::Object(
        fields
            .iter()
            .filter_map(|field| {
                object
                    .get(*field)
                    .and_then(json_to_state_value)
                    .map(|value| ((*field).to_string(), value))
            })
            .collect(),
    )
}

fn json_to_state_value(value: &JsonValue) -> Option<Value> {
    match value {
        JsonValue::Null => Some(Value::Null),
        JsonValue::Bool(value) => Some(Value::Bool(*value)),
        JsonValue::Number(value) => value
            .as_i64()
            .map(Value::Integer)
            .or_else(|| value.as_f64().map(Value::Number)),
        JsonValue::String(value) => Some(Value::Text(value.clone())),
        _ => None,
    }
}

fn percentage_value(value: &JsonValue) -> Option<Value> {
    let value = value.as_f64()?.round().clamp(0.0, 100.0) as u8;
    Some(Value::Percentage(value))
}

fn protocol_identifier(kind: &str, value: &str) -> Result<ProtocolIdentifier, ShellyError> {
    ProtocolIdentifier::new(
        ProtocolFamily::Vendor(INTEGRATION_ID.to_string()),
        kind,
        value,
    )
    .map_err(|error| ShellyError::Validation(error.to_string()))
}

fn stable_component(value: &str) -> String {
    let mut stable = value
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    while stable.contains("--") {
        stable = stable.replace("--", "-");
    }
    stable.trim_matches('-').to_string()
}

fn duration_ms(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis())
        .unwrap_or(u64::MAX)
        .max(1)
}

fn has_unsafe_http_text(value: &str) -> bool {
    value.contains(['\r', '\n', '\0'])
}

fn encode_http_request(url: &Url, plan: &LocalHttpRequestPlan) -> Result<Vec<u8>, ShellyError> {
    let host = url
        .host
        .as_deref()
        .ok_or(ShellyError::MissingField("URL host"))?;
    let port = url
        .effective_port()
        .ok_or(ShellyError::MissingField("URL port"))?;
    let target = if url.path.is_empty() {
        "/".to_string()
    } else if let Some(query) = &url.query {
        format!("{}?{query}", url.path)
    } else {
        url.path.clone()
    };
    if has_unsafe_http_text(&target) || has_unsafe_http_text(host) {
        return Err(ShellyError::Validation(
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
            return Err(ShellyError::Validation(
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

fn connect_tcp(host: &str, port: u16, timeout: Duration) -> Result<TcpStream, ShellyError> {
    let addresses = (host, port)
        .to_socket_addrs()
        .map_err(|error| ShellyError::Io(error.to_string()))?
        .collect::<Vec<_>>();
    let mut last_error = None;
    for address in addresses {
        match TcpStream::connect_timeout(&address, timeout) {
            Ok(stream) => {
                stream
                    .set_read_timeout(Some(timeout))
                    .map_err(|error| ShellyError::Io(error.to_string()))?;
                stream
                    .set_write_timeout(Some(timeout))
                    .map_err(|error| ShellyError::Io(error.to_string()))?;
                return Ok(stream);
            }
            Err(error) => last_error = Some(error),
        }
    }
    Err(ShellyError::Io(
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

fn read_bounded(reader: &mut dyn Read, maximum: usize) -> Result<Vec<u8>, ShellyError> {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| ShellyError::Io(error.to_string()))?;
        if read == 0 {
            break;
        }
        if read > maximum.saturating_sub(bytes.len()) {
            return Err(ShellyError::ResponseTooLarge { limit: maximum });
        }
        bytes.extend_from_slice(&buffer[..read]);
    }
    Ok(bytes)
}

fn decode_http_response(bytes: &[u8], maximum: usize) -> Result<Vec<u8>, ShellyError> {
    let parsed = parse_response_head(bytes)
        .map_err(|error: Http1ParseError| ShellyError::Http(error.to_string()))?;
    if !(200..300).contains(&parsed.head.status) {
        return Err(ShellyError::HttpStatus(parsed.head.status));
    }
    let input = &bytes[parsed.body_offset..];
    let body = match parsed.body_kind {
        BodyKind::None => Vec::new(),
        BodyKind::ContentLength(expected) => {
            if input.len() < expected {
                return Err(ShellyError::TruncatedBody {
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
        return Err(ShellyError::ResponseTooLarge { limit: maximum });
    }
    Ok(body)
}

fn decode_chunked(input: &[u8], maximum: usize) -> Result<Vec<u8>, ShellyError> {
    let mut cursor = 0usize;
    let mut output = Vec::new();
    loop {
        let offset = input[cursor..]
            .windows(2)
            .position(|window| window == b"\r\n")
            .ok_or_else(|| ShellyError::Http("missing chunk-size terminator".to_string()))?;
        let end = cursor + offset;
        let size_text = std::str::from_utf8(&input[cursor..end])
            .map_err(|_| ShellyError::Http("chunk size is not ASCII".to_string()))?
            .split(';')
            .next()
            .unwrap_or_default();
        let size = usize::from_str_radix(size_text.trim(), 16)
            .map_err(|_| ShellyError::Http("invalid chunk size".to_string()))?;
        cursor = end + 2;
        if size == 0 {
            return Ok(output);
        }
        if size > maximum.saturating_sub(output.len()) {
            return Err(ShellyError::ResponseTooLarge { limit: maximum });
        }
        let chunk_end = cursor
            .checked_add(size)
            .ok_or_else(|| ShellyError::Http("chunk size overflow".to_string()))?;
        if input.len() < chunk_end + 2 || &input[chunk_end..chunk_end + 2] != b"\r\n" {
            return Err(ShellyError::Http("truncated chunk".to_string()));
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
    use std::sync::{Arc, Mutex};
    use std::thread;

    const INFO: &str = r#"{"id":"shellypro4pm-f008d1d8b8b8","mac":"F008D1D8B8B8","model":"SPSW-004PE16EU","gen":2,"fw_id":"20260730/test","ver":"1.6.2","app":"Pro4PM","profile":"switch","auth_en":false}"#;
    const STATUS: &str = r#"{"switch:0":{"id":0,"output":true,"apower":23.5,"voltage":120.1,"current":0.2},"light:1":{"id":1,"output":true,"brightness":42},"input:0":{"id":0,"state":false},"temperature:0":{"id":0,"tC":21.5},"humidity:0":{"id":0,"rh":44.2},"pm1:0":{"id":0,"apower":23.5,"voltage":120.1}}"#;

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
                    let Some(head_end) = bytes.windows(4).position(|window| window == b"\r\n\r\n")
                    else {
                        continue;
                    };
                    let head = String::from_utf8_lossy(&bytes[..head_end + 4]);
                    let length = head
                        .lines()
                        .find_map(|line| line.strip_prefix("Content-Length: "))
                        .and_then(|value| value.parse::<usize>().ok())
                        .unwrap_or(0);
                    if bytes.len() >= head_end + 4 + length {
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

    #[test]
    fn shelly_mdns_advertisement_becomes_verified_discovery_record() {
        let advertisement = MdnsAdvertisement::new(
            MDNS_SERVICE_TYPE,
            "shellypro4pm-f008d1d8b8b8",
            "shelly.local",
            80,
            1_000,
        )
        .unwrap()
        .with_address("192.0.2.20")
        .unwrap()
        .with_txt("gen", "3")
        .unwrap()
        .with_txt("model", "SPSW-004PE16EU")
        .unwrap();
        let record = discovery_record(&advertisement).unwrap();
        assert_eq!(
            record.integration_id,
            IntegrationId::trusted(INTEGRATION_ID)
        );
        assert_eq!(record.address.as_deref(), Some("http://192.0.2.20:80"));
        assert_eq!(record.confidence, DiscoveryConfidence::Verified);
        assert_eq!(
            record.protocol_family,
            ProtocolFamily::Vendor("shelly".to_string())
        );
    }

    #[test]
    fn real_tcp_inspection_installs_normalized_runtime_entities() {
        let (port, requests, handle) = start_server(vec![response(INFO), response(STATUS)]);
        let config = ShellyDeviceConfig::new(BridgeId::trusted("shelly.bridge.test"), "127.0.0.1")
            .unwrap()
            .with_port(port);
        let client = ShellyClient::new(config, ShellyLanTransport::default()).unwrap();
        let mut integration = ShellyRuntimeIntegration::new(client);
        let mut runtime = SmartHomeRuntime::new();
        let installed = integration
            .inspect_and_install(&mut runtime, 5_000)
            .unwrap();
        handle.join().unwrap();
        assert_eq!(installed.entity_ids.len(), 6);
        assert!(runtime.registry().bridge(&installed.bridge_id).is_some());
        assert_eq!(runtime.registry().counts().states, 6);
        let requests = requests.lock().unwrap();
        assert!(requests[0].starts_with("GET /shelly HTTP/1.1"));
        assert!(requests[1].starts_with("GET /rpc/Shelly.GetStatus HTTP/1.1"));
    }

    #[test]
    fn authorized_switch_command_crosses_real_tcp_rpc_transport() {
        let (port, requests, handle) = start_server(vec![
            response(INFO),
            response(STATUS),
            response(r#"{"id":1,"src":"shellypro4pm-f008d1d8b8b8","result":null}"#),
        ]);
        let config =
            ShellyDeviceConfig::new(BridgeId::trusted("shelly.bridge.command"), "127.0.0.1")
                .unwrap()
                .with_port(port);
        let client = ShellyClient::new(config, ShellyLanTransport::default()).unwrap();
        let mut integration = ShellyRuntimeIntegration::new(client);
        let mut runtime = SmartHomeRuntime::new();
        let installed = integration
            .inspect_and_install(&mut runtime, 5_000)
            .unwrap();
        let switch = installed
            .entity_ids
            .iter()
            .find(|id| id.as_str().contains(":switch:"))
            .unwrap()
            .clone();
        let principal = AgentId::trusted("agent:shelly-test");
        let _ = runtime.registry_mut().upsert_capability_grant(
            CapabilityGrant::for_all_smart_home(
                CapabilityGrantId::trusted("grant:shelly-test"),
                principal.clone(),
                PrivilegeTier::LowRisk,
                "test",
                1_000,
            )
            .with_expiry(20_000),
        );
        let result = integration
            .dispatch_command(
                &mut runtime,
                principal,
                RuntimeCommandToolRequest::new(switch, CommandType::TurnOff, Value::Null),
                6_000,
            )
            .unwrap();
        handle.join().unwrap();
        assert_eq!(result.status, smart_home_core::CommandStatus::Accepted);
        let requests = requests.lock().unwrap();
        assert!(requests[2].starts_with("POST /rpc HTTP/1.1"));
        assert!(requests[2].contains(r#""method":"Switch.Set""#));
        assert!(requests[2].contains(r#""on":false"#));
    }

    #[test]
    fn authentication_and_response_limits_fail_closed() {
        let auth_info = INFO.replace("\"auth_en\":false", "\"auth_en\":true");
        let (port, _, handle) = start_server(vec![response(&auth_info)]);
        let config = ShellyDeviceConfig::new(BridgeId::trusted("shelly.bridge.auth"), "127.0.0.1")
            .unwrap()
            .with_port(port);
        let mut client = ShellyClient::new(config, ShellyLanTransport::default()).unwrap();
        assert!(matches!(
            client.inspect(),
            Err(ShellyError::AuthenticationRequired { .. })
        ));
        handle.join().unwrap();

        let payload = response("{\"large\":true}");
        assert!(matches!(
            decode_http_response(&payload, 2),
            Err(ShellyError::ResponseTooLarge { limit: 2 })
        ));
    }

    #[test]
    fn light_brightness_maps_to_the_official_rpc_shape() {
        let entity_id = EntityId::trusted("shelly:test:light:1");
        let binding = ComponentBinding {
            entity_id: entity_id.clone(),
            component: "light:1".to_string(),
            kind: ComponentKind::Light,
        };
        let request = RuntimeCommandToolRequest::new(
            entity_id,
            CommandType::SetBrightness,
            Value::Percentage(37),
        );

        let (method, params) = command_rpc(&binding, &request).unwrap();
        assert_eq!(method, "Light.Set");
        assert_eq!(params, json!({"id": 1, "brightness": 37}));
    }
}
