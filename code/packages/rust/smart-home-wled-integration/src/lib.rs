//! Production WLED local JSON API integration for D23.

#![forbid(unsafe_code)]

use http1::{parse_response_head, Http1ParseError};
use http_core::BodyKind;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use smart_home_core::{
    AgentId, Bridge, BridgeId, BridgeTransport, Capability, CommandResult, CommandType, Device,
    DeviceId, Entity, EntityId, EntityKind, Health, IntegrationId, Metadata, ProtocolFamily,
    ProtocolIdentifier, StateConfidence, StateSnapshot, StateSource, Value,
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
pub const INTEGRATION_ID: &str = "wled";
pub const MDNS_SERVICE_TYPE: &str = "_wled._tcp.local";
pub const DEFAULT_PORT: u16 = 80;
pub const DEFAULT_MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
pub const DEFAULT_MAX_DISCOVERY_RESPONSES: usize = 64;

#[derive(Debug)]
pub enum WledError {
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
    NoSegments,
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

impl fmt::Display for WledError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => write!(formatter, "invalid WLED input: {message}"),
            Self::Discovery(message) => write!(formatter, "WLED discovery failed: {message}"),
            Self::LocalHttp(error) => error.fmt(formatter),
            Self::Url(error) => write!(formatter, "invalid WLED URL: {error}"),
            Self::Io(message) => write!(formatter, "WLED LAN I/O failed: {message}"),
            Self::Http(message) => write!(formatter, "invalid WLED HTTP response: {message}"),
            Self::HttpStatus(status) => write!(formatter, "WLED endpoint returned HTTP {status}"),
            Self::ResponseTooLarge { limit } => {
                write!(formatter, "WLED response exceeds {limit} bytes")
            }
            Self::TruncatedBody { expected, actual } => write!(
                formatter,
                "WLED response is truncated: expected {expected} bytes, got {actual}"
            ),
            Self::Json(error) => write!(formatter, "invalid WLED JSON: {error}"),
            Self::MissingField(field) => write!(formatter, "WLED response is missing {field}"),
            Self::NoSegments => formatter.write_str("WLED device has no segments"),
            Self::UnknownEntity(entity_id) => write!(formatter, "unknown WLED entity {entity_id}"),
            Self::UnsupportedCommand {
                entity_id,
                command_type,
            } => write!(
                formatter,
                "WLED entity {entity_id} does not support {command_type:?}"
            ),
            Self::InvalidCommandArguments {
                command_type,
                expected,
            } => write!(
                formatter,
                "invalid {command_type:?} arguments; expected {expected}"
            ),
            Self::Runtime(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for WledError {}

impl From<LocalHttpError> for WledError {
    fn from(error: LocalHttpError) -> Self {
        Self::LocalHttp(error)
    }
}

impl From<UrlError> for WledError {
    fn from(error: UrlError) -> Self {
        Self::Url(error)
    }
}

impl From<serde_json::Error> for WledError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<RuntimeError> for WledError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct WledLedInfo {
    #[serde(default)]
    pub count: u32,
    #[serde(default)]
    pub lc: u8,
    #[serde(default)]
    pub seglc: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct WledInfo {
    #[serde(default, rename = "ver")]
    pub version: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub mac: String,
    #[serde(default)]
    pub arch: String,
    #[serde(default)]
    pub brand: String,
    #[serde(default)]
    pub product: String,
    pub leds: WledLedInfo,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct WledSegment {
    pub id: u8,
    #[serde(default, rename = "n")]
    pub name: String,
    #[serde(default)]
    pub start: u32,
    #[serde(default)]
    pub stop: u32,
    #[serde(default)]
    pub len: u32,
    #[serde(default)]
    pub on: bool,
    #[serde(default, rename = "bri")]
    pub brightness: u8,
    #[serde(default, rename = "col")]
    pub colors: Vec<Vec<u8>>,
    #[serde(default, rename = "fx")]
    pub effect: u16,
    #[serde(default)]
    pub cct: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct WledState {
    #[serde(default)]
    pub on: bool,
    #[serde(default, rename = "bri")]
    pub brightness: u8,
    #[serde(default, rename = "seg")]
    pub segments: Vec<WledSegment>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct WledSnapshot {
    pub state: WledState,
    pub info: WledInfo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WledDeviceConfig {
    pub bridge_id: BridgeId,
    pub host: String,
    pub port: u16,
    pub timeout: Duration,
}

impl WledDeviceConfig {
    pub fn new(bridge_id: BridgeId, host: impl Into<String>) -> Result<Self, WledError> {
        let host = host.into();
        if host.trim().is_empty() || has_unsafe_http_text(&host) {
            return Err(WledError::Validation(
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

    pub fn endpoint(&self) -> Result<LocalHttpEndpoint, WledError> {
        Ok(LocalHttpEndpoint::new(
            IntegrationId::trusted(INTEGRATION_ID),
            self.bridge_id.clone(),
            LocalHttpScheme::Http,
            self.host.clone(),
        )?
        .with_port(self.port)
        .with_metadata(Metadata::new("http.profile", "wled.json.v1")))
    }
}

pub fn scan_mdns_ipv4(
    discovered_at_ms: u64,
    timeout: Duration,
) -> Result<MdnsScanResult, WledError> {
    let options = MdnsScanOptions::new(MDNS_SERVICE_TYPE, discovered_at_ms, timeout)
        .map_err(|error| WledError::Discovery(error.to_string()))?
        .with_max_responses(DEFAULT_MAX_DISCOVERY_RESPONSES);
    run_mdns_ipv4_scan(options).map_err(|error| WledError::Discovery(error.to_string()))
}

pub fn discovery_record(advertisement: &MdnsAdvertisement) -> Result<DiscoveryRecord, WledError> {
    if advertisement.service_type.trim_end_matches('.') != MDNS_SERVICE_TYPE.trim_end_matches('.') {
        return Err(WledError::Validation(format!(
            "unexpected mDNS service type `{}`",
            advertisement.service_type
        )));
    }
    let native_id = advertisement
        .txt_value("mac")
        .map(stable_component)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| stable_component(&advertisement.instance_name));
    Ok(DiscoveryRecord::new(
        IntegrationId::trusted(INTEGRATION_ID),
        ProtocolFamily::Vendor(INTEGRATION_ID.to_string()),
        native_id,
        DiscoverySource::Mdns,
        BridgeTransport::LanHttp,
        advertisement.discovered_at_ms,
    )
    .map_err(|error| WledError::Discovery(error.to_string()))?
    .with_display_name(&advertisement.instance_name)
    .with_address(advertisement.endpoint_with_scheme("http"))
    .with_confidence(DiscoveryConfidence::Verified)
    .with_pairing_requirement(PairingRequirement::None)
    .with_metadata("smart_home.discovery.service_type", MDNS_SERVICE_TYPE))
}

pub trait WledTransport {
    fn execute(&mut self, plan: &LocalHttpRequestPlan) -> Result<Vec<u8>, WledError>;
}

#[derive(Debug, Clone)]
pub struct WledLanTransport {
    pub maximum_response_bytes: usize,
}

impl Default for WledLanTransport {
    fn default() -> Self {
        Self {
            maximum_response_bytes: DEFAULT_MAX_RESPONSE_BYTES,
        }
    }
}

impl WledTransport for WledLanTransport {
    fn execute(&mut self, plan: &LocalHttpRequestPlan) -> Result<Vec<u8>, WledError> {
        let url = Url::parse(&plan.url)?;
        if url.scheme != "http" {
            return Err(WledError::Validation(
                "WLED transport only permits local HTTP".to_string(),
            ));
        }
        let host = url
            .host
            .as_deref()
            .ok_or(WledError::MissingField("URL host"))?;
        let port = url
            .effective_port()
            .ok_or(WledError::MissingField("URL port"))?;
        let timeout = Duration::from_millis(plan.timeout_ms.max(1));
        let request = encode_http_request(&url, plan)?;
        let mut stream = connect_tcp(host, port, timeout)?;
        stream
            .write_all(&request)
            .map_err(|error| WledError::Io(error.to_string()))?;
        let bytes = read_bounded(&mut stream, self.maximum_response_bytes)?;
        decode_http_response(&bytes, self.maximum_response_bytes)
    }
}

#[derive(Debug)]
pub struct WledClient<T> {
    config: WledDeviceConfig,
    endpoint: LocalHttpEndpoint,
    transport: T,
}

impl<T: WledTransport> WledClient<T> {
    pub fn new(config: WledDeviceConfig, transport: T) -> Result<Self, WledError> {
        let endpoint = config.endpoint()?;
        Ok(Self {
            config,
            endpoint,
            transport,
        })
    }

    pub fn config(&self) -> &WledDeviceConfig {
        &self.config
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    pub fn transport_mut(&mut self) -> &mut T {
        &mut self.transport
    }

    pub fn inspect(&mut self) -> Result<WledSnapshot, WledError> {
        let template = LocalHttpRequestTemplate::new(LocalHttpMethod::Get, "/json/si")?
            .with_accept("application/json")
            .with_timeout_ms(duration_ms(self.config.timeout));
        let bytes = self
            .transport
            .execute(&template.plan(&self.endpoint, Vec::new())?)?;
        let snapshot: WledSnapshot = serde_json::from_slice(&bytes)?;
        validate_snapshot(&snapshot)?;
        Ok(snapshot)
    }

    pub fn update_state(&mut self, update: JsonValue) -> Result<WledState, WledError> {
        let body = serde_json::to_vec(&update)?;
        let template = LocalHttpRequestTemplate::new(LocalHttpMethod::Post, "/json/state")?
            .with_accept("application/json")
            .with_content_type("application/json")
            .with_timeout_ms(duration_ms(self.config.timeout));
        let bytes = self
            .transport
            .execute(&template.plan(&self.endpoint, body)?)?;
        Ok(serde_json::from_slice(&bytes)?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledWledDevice {
    pub bridge_id: BridgeId,
    pub device_id: DeviceId,
    pub entity_ids: Vec<EntityId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum EntityBinding {
    Master,
    Segment { id: u8, color_capabilities: u8 },
}

#[derive(Debug)]
pub struct WledRuntimeIntegration<T> {
    client: WledClient<T>,
    bindings: BTreeMap<EntityId, EntityBinding>,
}

impl<T: WledTransport> WledRuntimeIntegration<T> {
    pub fn new(client: WledClient<T>) -> Self {
        Self {
            client,
            bindings: BTreeMap::new(),
        }
    }

    pub fn client(&self) -> &WledClient<T> {
        &self.client
    }

    pub fn client_mut(&mut self) -> &mut WledClient<T> {
        &mut self.client
    }

    pub fn inspect_and_install(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        observed_at_ms: u64,
    ) -> Result<InstalledWledDevice, WledError> {
        let snapshot = self.client.inspect()?;
        self.install_snapshot(runtime, &snapshot, observed_at_ms)
    }

    pub fn install_snapshot(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        snapshot: &WledSnapshot,
        observed_at_ms: u64,
    ) -> Result<InstalledWledDevice, WledError> {
        validate_snapshot(snapshot)?;
        let native_id = stable_component(&snapshot.info.mac);
        let device_id = DeviceId::trusted(format!("wled:{native_id}"));
        let bridge_id = self.client.config.bridge_id.clone();
        let mut bindings = BTreeMap::new();
        let mut entities = Vec::new();

        let master_id = EntityId::trusted(format!("wled:{native_id}:master"));
        entities.push(light_entity(
            master_id.clone(),
            device_id.clone(),
            display_name(&snapshot.info),
            vec![Capability::light_on_off(), Capability::light_brightness()],
            master_state(&snapshot.state),
            observed_at_ms,
            vec![Metadata::new("wled.scope", "master")],
        ));
        bindings.insert(master_id, EntityBinding::Master);

        for segment in &snapshot.state.segments {
            let color_capabilities = snapshot
                .info
                .leds
                .seglc
                .get(usize::from(segment.id))
                .copied()
                .unwrap_or(snapshot.info.leds.lc);
            let entity_id = EntityId::trusted(format!("wled:{native_id}:segment:{}", segment.id));
            let mut capabilities = vec![Capability::light_on_off(), Capability::light_brightness()];
            if color_capabilities & 0b001 != 0 {
                capabilities.push(Capability::light_color());
            }
            if color_capabilities & 0b100 != 0 {
                capabilities.push(Capability::light_color_temperature());
            }
            entities.push(light_entity(
                entity_id.clone(),
                device_id.clone(),
                if segment.name.is_empty() {
                    format!("{} segment {}", display_name(&snapshot.info), segment.id)
                } else {
                    segment.name.clone()
                },
                capabilities,
                segment_state(segment),
                observed_at_ms,
                vec![
                    Metadata::new("wled.scope", "segment"),
                    Metadata::new("wled.segment_id", segment.id.to_string()),
                ],
            ));
            bindings.insert(
                entity_id,
                EntityBinding::Segment {
                    id: segment.id,
                    color_capabilities,
                },
            );
        }

        let endpoint = self.client.config.endpoint()?;
        let model = if snapshot.info.product.is_empty() {
            snapshot.info.arch.clone()
        } else {
            snapshot.info.product.clone()
        };
        let mut bridge = Bridge::new(
            bridge_id.clone(),
            IntegrationId::trusted(INTEGRATION_ID),
            BridgeTransport::LanHttp,
        );
        bridge.address = Some(endpoint.origin());
        bridge.hardware_model = Some(model.clone());
        bridge.firmware_version = Some(snapshot.info.version.clone());
        bridge.health = Health::Online;
        bridge.last_seen_at_ms = Some(observed_at_ms);
        bridge.identifiers = vec![protocol_identifier("http_endpoint", &endpoint.origin())?];
        bridge.metadata = vec![Metadata::new("wled.transport", "http_json_polling")];
        runtime.upsert_bridge(bridge)?;

        let entity_ids = entities
            .iter()
            .map(|entity| entity.entity_id.clone())
            .collect::<Vec<_>>();
        runtime.upsert_device(Device {
            device_id: device_id.clone(),
            bridge_id: bridge_id.clone(),
            manufacturer: if snapshot.info.brand.is_empty() {
                "WLED".to_string()
            } else {
                snapshot.info.brand.clone()
            },
            model,
            name: display_name(&snapshot.info),
            serial: Some(snapshot.info.mac.clone()),
            firmware_version: Some(snapshot.info.version.clone()),
            room_id: None,
            entity_ids: entity_ids.clone(),
            identifiers: vec![protocol_identifier("mac", &snapshot.info.mac)?],
            health: Health::Online,
            metadata: vec![Metadata::new(
                "wled.led_count",
                snapshot.info.leds.count.to_string(),
            )],
        })?;
        for entity in entities {
            runtime.upsert_entity(entity)?;
        }
        self.bindings = bindings;
        Ok(InstalledWledDevice {
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
    ) -> Result<CommandResult, WledError> {
        let binding = self
            .bindings
            .get(&request.entity_id)
            .cloned()
            .ok_or_else(|| WledError::UnknownEntity(request.entity_id.clone()))?;
        let body = command_body(&binding, &request)?;
        let result = runtime.execute_command_tool(principal_id, request, now_ms)?;
        self.client.update_state(body)?;
        Ok(result)
    }
}

fn validate_snapshot(snapshot: &WledSnapshot) -> Result<(), WledError> {
    if snapshot.info.mac.trim().is_empty() {
        return Err(WledError::MissingField("info.mac"));
    }
    if snapshot.state.segments.is_empty() {
        return Err(WledError::NoSegments);
    }
    Ok(())
}

fn display_name(info: &WledInfo) -> String {
    if info.name.trim().is_empty() {
        "WLED".to_string()
    } else {
        info.name.clone()
    }
}

fn light_entity(
    entity_id: EntityId,
    device_id: DeviceId,
    name: String,
    capabilities: Vec<Capability>,
    value: Value,
    observed_at_ms: u64,
    metadata: Vec<Metadata>,
) -> Entity {
    Entity {
        entity_id: entity_id.clone(),
        device_id,
        kind: EntityKind::Light,
        name,
        capabilities,
        state: Some(StateSnapshot {
            entity_id,
            value,
            source: StateSource::Poll,
            observed_at_ms,
            received_at_ms: observed_at_ms,
            expires_at_ms: None,
            confidence: StateConfidence::Confirmed,
        }),
        metadata,
    }
}

fn master_state(state: &WledState) -> Value {
    Value::Object(vec![
        ("on".to_string(), Value::Bool(state.on)),
        (
            "brightness".to_string(),
            Value::Percentage(brightness_percent(state.brightness)),
        ),
    ])
}

fn segment_state(segment: &WledSegment) -> Value {
    let mut fields = vec![
        ("on".to_string(), Value::Bool(segment.on)),
        (
            "brightness".to_string(),
            Value::Percentage(brightness_percent(segment.brightness)),
        ),
        (
            "effect".to_string(),
            Value::Integer(i64::from(segment.effect)),
        ),
    ];
    if let Some(color) = segment.colors.first() {
        fields.push((
            "color".to_string(),
            Value::Array(
                color
                    .iter()
                    .take(3)
                    .map(|channel| Value::Integer(i64::from(*channel)))
                    .collect(),
            ),
        ));
    }
    if segment.cct > 0 {
        fields.push((
            "wled_cct".to_string(),
            Value::Integer(i64::from(segment.cct)),
        ));
    }
    Value::Object(fields)
}

fn command_body(
    binding: &EntityBinding,
    request: &RuntimeCommandToolRequest,
) -> Result<JsonValue, WledError> {
    let command = match request.command_type {
        CommandType::TurnOn => json!({"on": true}),
        CommandType::TurnOff => json!({"on": false}),
        CommandType::SetBrightness => {
            let Value::Percentage(percent) = request.arguments else {
                return invalid_arguments(request.command_type, "percentage");
            };
            json!({"bri": brightness_byte(percent)})
        }
        CommandType::SetColor => {
            let channels = rgb_channels(&request.arguments, request.command_type)?;
            json!({"col": [channels]})
        }
        CommandType::SetColorTemperature => {
            let Value::Integer(mirek) = request.arguments else {
                return invalid_arguments(
                    request.command_type,
                    "integer mirek from 100 through 526",
                );
            };
            if !(100..=526).contains(&mirek) {
                return invalid_arguments(
                    request.command_type,
                    "integer mirek from 100 through 526",
                );
            }
            json!({"cct": 1_000_000 / mirek})
        }
        _ => {
            return Err(WledError::UnsupportedCommand {
                entity_id: request.entity_id.clone(),
                command_type: request.command_type,
            });
        }
    };

    match binding {
        EntityBinding::Master => match request.command_type {
            CommandType::TurnOn | CommandType::TurnOff | CommandType::SetBrightness => {
                let mut body = command;
                body["v"] = JsonValue::Bool(true);
                Ok(body)
            }
            _ => Err(WledError::UnsupportedCommand {
                entity_id: request.entity_id.clone(),
                command_type: request.command_type,
            }),
        },
        EntityBinding::Segment {
            id,
            color_capabilities,
        } => {
            if request.command_type == CommandType::SetColor && color_capabilities & 0b001 == 0 {
                return unsupported(request);
            }
            if request.command_type == CommandType::SetColorTemperature
                && color_capabilities & 0b100 == 0
            {
                return unsupported(request);
            }
            let mut segment = command;
            segment["id"] = json!(*id);
            Ok(json!({"seg": [segment], "v": true}))
        }
    }
}

fn rgb_channels(arguments: &Value, command_type: CommandType) -> Result<[u8; 3], WledError> {
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
        .ok_or(WledError::InvalidCommandArguments {
            command_type,
            expected: "RGB array with three integer channels from 0 through 255",
        })?;
    Ok([channels[0], channels[1], channels[2]])
}

fn invalid_arguments<T>(command_type: CommandType, expected: &'static str) -> Result<T, WledError> {
    Err(WledError::InvalidCommandArguments {
        command_type,
        expected,
    })
}

fn unsupported(request: &RuntimeCommandToolRequest) -> Result<JsonValue, WledError> {
    Err(WledError::UnsupportedCommand {
        entity_id: request.entity_id.clone(),
        command_type: request.command_type,
    })
}

fn brightness_percent(value: u8) -> u8 {
    ((u16::from(value) * 100 + 127) / 255) as u8
}

fn brightness_byte(value: u8) -> u8 {
    ((u16::from(value) * 255 + 50) / 100) as u8
}

fn protocol_identifier(kind: &str, value: &str) -> Result<ProtocolIdentifier, WledError> {
    ProtocolIdentifier::new(
        ProtocolFamily::Vendor(INTEGRATION_ID.to_string()),
        kind,
        value,
    )
    .map_err(|error| WledError::Validation(error.to_string()))
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

fn encode_http_request(url: &Url, plan: &LocalHttpRequestPlan) -> Result<Vec<u8>, WledError> {
    let host = url
        .host
        .as_deref()
        .ok_or(WledError::MissingField("URL host"))?;
    let port = url
        .effective_port()
        .ok_or(WledError::MissingField("URL port"))?;
    let target = if url.path.is_empty() {
        "/".to_string()
    } else if let Some(query) = &url.query {
        format!("{}?{query}", url.path)
    } else {
        url.path.clone()
    };
    if has_unsafe_http_text(&target) || has_unsafe_http_text(host) {
        return Err(WledError::Validation(
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
            return Err(WledError::Validation(
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

fn connect_tcp(host: &str, port: u16, timeout: Duration) -> Result<TcpStream, WledError> {
    let addresses = (host, port)
        .to_socket_addrs()
        .map_err(|error| WledError::Io(error.to_string()))?
        .collect::<Vec<_>>();
    let mut last_error = None;
    for address in addresses {
        match TcpStream::connect_timeout(&address, timeout) {
            Ok(stream) => {
                stream
                    .set_read_timeout(Some(timeout))
                    .map_err(|error| WledError::Io(error.to_string()))?;
                stream
                    .set_write_timeout(Some(timeout))
                    .map_err(|error| WledError::Io(error.to_string()))?;
                return Ok(stream);
            }
            Err(error) => last_error = Some(error),
        }
    }
    Err(WledError::Io(
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

fn read_bounded(reader: &mut dyn Read, maximum: usize) -> Result<Vec<u8>, WledError> {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| WledError::Io(error.to_string()))?;
        if read == 0 {
            break;
        }
        if read > maximum.saturating_sub(bytes.len()) {
            return Err(WledError::ResponseTooLarge { limit: maximum });
        }
        bytes.extend_from_slice(&buffer[..read]);
    }
    Ok(bytes)
}

fn decode_http_response(bytes: &[u8], maximum: usize) -> Result<Vec<u8>, WledError> {
    let parsed = parse_response_head(bytes)
        .map_err(|error: Http1ParseError| WledError::Http(error.to_string()))?;
    if !(200..300).contains(&parsed.head.status) {
        return Err(WledError::HttpStatus(parsed.head.status));
    }
    let input = &bytes[parsed.body_offset..];
    let body = match parsed.body_kind {
        BodyKind::None => Vec::new(),
        BodyKind::ContentLength(expected) => {
            if input.len() < expected {
                return Err(WledError::TruncatedBody {
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
        return Err(WledError::ResponseTooLarge { limit: maximum });
    }
    Ok(body)
}

fn decode_chunked(input: &[u8], maximum: usize) -> Result<Vec<u8>, WledError> {
    let mut cursor = 0usize;
    let mut output = Vec::new();
    loop {
        let offset = input[cursor..]
            .windows(2)
            .position(|window| window == b"\r\n")
            .ok_or_else(|| WledError::Http("missing chunk-size terminator".to_string()))?;
        let end = cursor + offset;
        let size_text = std::str::from_utf8(&input[cursor..end])
            .map_err(|_| WledError::Http("chunk size is not ASCII".to_string()))?
            .split(';')
            .next()
            .unwrap_or_default();
        let size = usize::from_str_radix(size_text.trim(), 16)
            .map_err(|_| WledError::Http("invalid chunk size".to_string()))?;
        cursor = end + 2;
        if size == 0 {
            return Ok(output);
        }
        if size > maximum.saturating_sub(output.len()) {
            return Err(WledError::ResponseTooLarge { limit: maximum });
        }
        let chunk_end = cursor
            .checked_add(size)
            .ok_or_else(|| WledError::Http("chunk size overflow".to_string()))?;
        if input.len() < chunk_end + 2 || &input[chunk_end..chunk_end + 2] != b"\r\n" {
            return Err(WledError::Http("truncated chunk".to_string()));
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

    const SNAPSHOT: &str = r#"{"state":{"on":true,"bri":128,"seg":[{"id":0,"n":"Desk","start":0,"stop":60,"on":true,"bri":64,"col":[[255,32,0]],"fx":42,"cct":3200},{"id":1,"n":"Shelf","start":60,"stop":120,"on":false,"bri":255,"col":[[1,2,3]],"fx":0}]},"info":{"ver":"0.15.0","name":"Studio WLED","mac":"AABBCCDDEEFF","arch":"esp32","brand":"WLED","product":"Controller","leds":{"count":120,"lc":5,"seglc":[5,1]}}}"#;

    fn state_response() -> &'static str {
        r#"{"on":true,"bri":128,"seg":[{"id":0,"n":"Desk","start":0,"stop":60,"on":true,"bri":64,"col":[[12,34,56]],"fx":42,"cct":3200},{"id":1,"n":"Shelf","start":60,"stop":120,"on":false,"bri":255,"col":[[1,2,3]],"fx":0}]}"#
    }

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

    fn grant(runtime: &mut SmartHomeRuntime, principal: &AgentId) {
        let _ = runtime.registry_mut().upsert_capability_grant(
            CapabilityGrant::for_all_smart_home(
                CapabilityGrantId::trusted("grant:wled-test"),
                principal.clone(),
                PrivilegeTier::LowRisk,
                "test",
                1_000,
            )
            .with_expiry(20_000),
        );
    }

    #[test]
    fn wled_mdns_advertisement_becomes_verified_discovery_record() {
        let advertisement = MdnsAdvertisement::new(
            MDNS_SERVICE_TYPE,
            "Studio WLED",
            "wled-aabbcc.local",
            80,
            1_000,
        )
        .unwrap()
        .with_address("192.0.2.30")
        .unwrap()
        .with_txt("mac", "AABBCCDDEEFF")
        .unwrap();
        let record = discovery_record(&advertisement).unwrap();
        assert_eq!(record.native_bridge_id, "aabbccddeeff");
        assert_eq!(record.address.as_deref(), Some("http://192.0.2.30:80"));
        assert_eq!(record.confidence, DiscoveryConfidence::Verified);
        assert_eq!(record.pairing_requirement, PairingRequirement::None);
    }

    #[test]
    fn real_tcp_inspection_installs_capability_aware_runtime_entities() {
        let (port, requests, handle) = start_server(vec![response(SNAPSHOT)]);
        let config = WledDeviceConfig::new(BridgeId::trusted("wled.bridge.test"), "127.0.0.1")
            .unwrap()
            .with_port(port);
        let client = WledClient::new(config, WledLanTransport::default()).unwrap();
        let mut integration = WledRuntimeIntegration::new(client);
        let mut runtime = SmartHomeRuntime::new();
        let installed = integration
            .inspect_and_install(&mut runtime, 5_000)
            .unwrap();
        handle.join().unwrap();
        assert_eq!(installed.entity_ids.len(), 3);
        assert_eq!(runtime.registry().counts().states, 3);
        let segment_zero = runtime
            .registry()
            .entity(&EntityId::trusted("wled:aabbccddeeff:segment:0"))
            .unwrap();
        assert_eq!(segment_zero.capabilities.len(), 4);
        let segment_one = runtime
            .registry()
            .entity(&EntityId::trusted("wled:aabbccddeeff:segment:1"))
            .unwrap();
        assert_eq!(segment_one.capabilities.len(), 3);
        assert!(requests.lock().unwrap()[0].starts_with("GET /json/si HTTP/1.1"));
    }

    #[test]
    fn authorized_rgb_command_crosses_real_tcp_transport() {
        let (port, requests, handle) =
            start_server(vec![response(SNAPSHOT), response(state_response())]);
        let config = WledDeviceConfig::new(BridgeId::trusted("wled.bridge.command"), "127.0.0.1")
            .unwrap()
            .with_port(port);
        let client = WledClient::new(config, WledLanTransport::default()).unwrap();
        let mut integration = WledRuntimeIntegration::new(client);
        let mut runtime = SmartHomeRuntime::new();
        integration
            .inspect_and_install(&mut runtime, 5_000)
            .unwrap();
        let principal = AgentId::trusted("agent:wled-test");
        grant(&mut runtime, &principal);
        let result = integration
            .dispatch_command(
                &mut runtime,
                principal,
                RuntimeCommandToolRequest::new(
                    EntityId::trusted("wled:aabbccddeeff:segment:0"),
                    CommandType::SetColor,
                    Value::Array(vec![
                        Value::Integer(12),
                        Value::Integer(34),
                        Value::Integer(56),
                    ]),
                ),
                6_000,
            )
            .unwrap();
        handle.join().unwrap();
        assert_eq!(result.status, smart_home_core::CommandStatus::Accepted);
        let requests = requests.lock().unwrap();
        assert!(requests[1].starts_with("POST /json/state HTTP/1.1"));
        assert!(requests[1].contains(r#""seg":[{"col":[[12,34,56]],"id":0}]"#));
        assert!(requests[1].contains(r#""v":true"#));
    }

    #[test]
    fn brightness_and_cct_use_documented_wled_ranges() {
        let master = RuntimeCommandToolRequest::new(
            EntityId::trusted("wled:test:master"),
            CommandType::SetBrightness,
            Value::Percentage(100),
        );
        assert_eq!(
            command_body(&EntityBinding::Master, &master).unwrap(),
            json!({"bri": 255, "v": true})
        );
        let segment = EntityBinding::Segment {
            id: 3,
            color_capabilities: 5,
        };
        let cct = RuntimeCommandToolRequest::new(
            EntityId::trusted("wled:test:segment:3"),
            CommandType::SetColorTemperature,
            Value::Integer(250),
        );
        assert_eq!(
            command_body(&segment, &cct).unwrap(),
            json!({"seg": [{"id": 3, "cct": 4000}], "v": true})
        );
    }

    #[test]
    fn malformed_capabilities_and_response_limits_fail_closed() {
        let no_cct = EntityBinding::Segment {
            id: 1,
            color_capabilities: 1,
        };
        let request = RuntimeCommandToolRequest::new(
            EntityId::trusted("wled:test:segment:1"),
            CommandType::SetColorTemperature,
            Value::Integer(250),
        );
        assert!(matches!(
            command_body(&no_cct, &request),
            Err(WledError::UnsupportedCommand { .. })
        ));

        let payload = response("{\"large\":true}");
        assert!(matches!(
            decode_http_response(&payload, 2),
            Err(WledError::ResponseTooLarge { limit: 2 })
        ));
    }
}
