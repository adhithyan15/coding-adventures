//! Authenticated Nanoleaf local API integration for D23.

#![forbid(unsafe_code)]

use coding_adventures_zeroize::Zeroizing;
use http1::{parse_response_head, Http1ParseError};
use http_core::BodyKind;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use smart_home_core::{
    AgentId, Bridge, BridgeId, BridgeTransport, Capability, CommandResult, CommandType, Device,
    DeviceId, Entity, EntityId, EntityKind, Health, IntegrationId, Metadata, ProtocolFamily,
    ProtocolIdentifier, SmartHomeTool, StateConfidence, StateSnapshot, StateSource, Value,
    VaultRef,
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
use std::collections::BTreeSet;
use std::fmt;
use std::io::{self, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;
use url_parser::{Url, UrlError};

pub const VERSION: &str = "0.1.0";
pub const INTEGRATION_ID: &str = "nanoleaf";
pub const PROTOCOL_ID: &str = "nanoleaf_local";
pub const MDNS_SERVICE_TYPE: &str = "_nanoleafapi._tcp.local";
pub const DEFAULT_PORT: u16 = 16021;
pub const DEFAULT_MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
pub const DEFAULT_MAX_DISCOVERY_RESPONSES: usize = 64;

#[derive(Debug)]
pub enum NanoleafError {
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

impl fmt::Display for NanoleafError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => write!(formatter, "invalid Nanoleaf input: {message}"),
            Self::Discovery(message) => write!(formatter, "Nanoleaf discovery failed: {message}"),
            Self::LocalHttp(error) => error.fmt(formatter),
            Self::Url(error) => write!(formatter, "invalid Nanoleaf URL: {error}"),
            Self::Io(message) => write!(formatter, "Nanoleaf LAN I/O failed: {message}"),
            Self::Http(message) => write!(formatter, "invalid Nanoleaf HTTP response: {message}"),
            Self::HttpStatus(status) => {
                write!(formatter, "Nanoleaf endpoint returned HTTP {status}")
            }
            Self::ResponseTooLarge { limit } => {
                write!(formatter, "Nanoleaf response exceeds {limit} bytes")
            }
            Self::TruncatedBody { expected, actual } => write!(
                formatter,
                "Nanoleaf response is truncated: expected {expected} bytes, got {actual}"
            ),
            Self::Json(error) => write!(formatter, "invalid Nanoleaf JSON: {error}"),
            Self::MissingField(field) => write!(formatter, "Nanoleaf response is missing {field}"),
            Self::UnknownEntity(entity_id) => {
                write!(formatter, "unknown Nanoleaf entity {entity_id}")
            }
            Self::UnsupportedCommand {
                entity_id,
                command_type,
            } => write!(
                formatter,
                "Nanoleaf entity {entity_id} does not support {command_type:?}"
            ),
            Self::InvalidCommandArguments {
                command_type,
                expected,
            } => write!(
                formatter,
                "invalid {command_type:?} arguments; expected {expected}"
            ),
            Self::VerificationFailed(message) => {
                write!(formatter, "Nanoleaf command verification failed: {message}")
            }
            Self::Runtime(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for NanoleafError {}

impl From<LocalHttpError> for NanoleafError {
    fn from(error: LocalHttpError) -> Self {
        Self::LocalHttp(error)
    }
}

impl From<UrlError> for NanoleafError {
    fn from(error: UrlError) -> Self {
        Self::Url(error)
    }
}

impl From<serde_json::Error> for NanoleafError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<RuntimeError> for NanoleafError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

pub struct NanoleafCredentials {
    token: Zeroizing<String>,
}

impl NanoleafCredentials {
    pub fn new(token: impl Into<String>) -> Result<Self, NanoleafError> {
        let token = token.into();
        if token.is_empty()
            || !token
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        {
            return Err(NanoleafError::Validation(
                "auth token must be a non-empty URL-safe token".to_string(),
            ));
        }
        Ok(Self {
            token: Zeroizing::new(token),
        })
    }

    fn token(&self) -> &str {
        self.token.as_str()
    }

    /// Returns the token only for an explicit handoff into secret storage.
    pub fn expose_for_storage(&self) -> &str {
        self.token.as_str()
    }
}

impl fmt::Debug for NanoleafCredentials {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("NanoleafCredentials([REDACTED])")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NanoleafConfig {
    pub bridge_id: BridgeId,
    pub base_url: String,
    pub credential_ref: VaultRef,
    pub timeout: Duration,
}

impl NanoleafConfig {
    pub fn new(
        bridge_id: BridgeId,
        base_url: impl Into<String>,
        credential_ref: VaultRef,
    ) -> Result<Self, NanoleafError> {
        let base_url = base_url.into().trim_end_matches('/').to_string();
        let parsed = Url::parse(&base_url)?;
        if parsed.scheme != "http"
            || parsed.host.is_none()
            || parsed.userinfo.is_some()
            || parsed.query.is_some()
            || parsed.fragment.is_some()
            || !matches!(parsed.path.as_str(), "" | "/")
        {
            return Err(NanoleafError::Validation(
                "base URL must be a credential-free HTTP origin".to_string(),
            ));
        }
        Ok(Self {
            bridge_id,
            base_url,
            credential_ref,
            timeout: Duration::from_secs(5),
        })
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout.max(Duration::from_millis(1));
        self
    }

    fn endpoint(&self) -> Result<LocalHttpEndpoint, NanoleafError> {
        let parsed = Url::parse(&self.base_url)?;
        let host = parsed
            .host
            .as_deref()
            .ok_or(NanoleafError::MissingField("base URL host"))?;
        let port = parsed.port.unwrap_or(DEFAULT_PORT);
        Ok(LocalHttpEndpoint::new(
            IntegrationId::trusted(INTEGRATION_ID),
            self.bridge_id.clone(),
            LocalHttpScheme::Http,
            host.to_string(),
        )?
        .with_port(port)
        .with_metadata(Metadata::new("http.profile", "nanoleaf.local.v1")))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct NanoleafBooleanState {
    pub value: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct NanoleafNumericState {
    pub value: i64,
    #[serde(default)]
    pub min: Option<i64>,
    #[serde(default)]
    pub max: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct NanoleafState {
    pub on: NanoleafBooleanState,
    pub brightness: NanoleafNumericState,
    pub hue: NanoleafNumericState,
    pub sat: NanoleafNumericState,
    pub ct: NanoleafNumericState,
    #[serde(default, rename = "colorMode")]
    pub color_mode: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct NanoleafSnapshot {
    pub name: String,
    #[serde(rename = "serialNo")]
    pub serial_number: String,
    pub manufacturer: String,
    #[serde(rename = "firmwareVersion")]
    pub firmware_version: String,
    pub model: String,
    pub state: NanoleafState,
}

#[derive(Debug, Deserialize)]
struct NanoleafPairingResponse {
    auth_token: String,
}

pub fn scan_mdns_ipv4(
    discovered_at_ms: u64,
    timeout: Duration,
) -> Result<MdnsScanResult, NanoleafError> {
    let options = MdnsScanOptions::new(MDNS_SERVICE_TYPE, discovered_at_ms, timeout)
        .map_err(|error| NanoleafError::Discovery(error.to_string()))?
        .with_max_responses(DEFAULT_MAX_DISCOVERY_RESPONSES);
    run_mdns_ipv4_scan(options).map_err(|error| NanoleafError::Discovery(error.to_string()))
}

pub fn discovery_record(
    advertisement: &MdnsAdvertisement,
) -> Result<DiscoveryRecord, NanoleafError> {
    if advertisement.service_type.trim_end_matches('.') != MDNS_SERVICE_TYPE.trim_end_matches('.') {
        return Err(NanoleafError::Validation(format!(
            "unexpected mDNS service type `{}`",
            advertisement.service_type
        )));
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
    .map_err(|error| NanoleafError::Discovery(error.to_string()))?
    .with_display_name(&advertisement.instance_name)
    .with_address(advertisement.endpoint_with_scheme("http"))
    .with_confidence(DiscoveryConfidence::Verified)
    .with_pairing_requirement(PairingRequirement::PhysicalPresence)
    .with_metadata("smart_home.discovery.service_type", MDNS_SERVICE_TYPE))
}

pub trait NanoleafTransport {
    fn execute(&mut self, plan: &LocalHttpRequestPlan) -> Result<Vec<u8>, NanoleafError>;
}

#[derive(Debug, Clone)]
pub struct NanoleafLanTransport {
    pub maximum_response_bytes: usize,
}

impl Default for NanoleafLanTransport {
    fn default() -> Self {
        Self {
            maximum_response_bytes: DEFAULT_MAX_RESPONSE_BYTES,
        }
    }
}

impl NanoleafTransport for NanoleafLanTransport {
    fn execute(&mut self, plan: &LocalHttpRequestPlan) -> Result<Vec<u8>, NanoleafError> {
        let url = Url::parse(&plan.url)?;
        if url.scheme != "http" {
            return Err(NanoleafError::Validation(
                "Nanoleaf transport only permits local HTTP".to_string(),
            ));
        }
        let host = url
            .host
            .as_deref()
            .ok_or(NanoleafError::MissingField("URL host"))?;
        let port = url
            .effective_port()
            .ok_or(NanoleafError::MissingField("URL port"))?;
        let timeout = Duration::from_millis(plan.timeout_ms.max(1));
        let request = encode_http_request(&url, plan)?;
        let mut stream = connect_tcp(host, port, timeout)?;
        stream
            .write_all(&request)
            .map_err(|error| NanoleafError::Io(error.to_string()))?;
        let bytes = read_bounded(&mut stream, self.maximum_response_bytes)?;
        decode_http_response(&bytes, self.maximum_response_bytes)
    }
}

pub struct NanoleafPairingClient<T> {
    config: NanoleafConfig,
    endpoint: LocalHttpEndpoint,
    transport: T,
}

impl<T: NanoleafTransport> NanoleafPairingClient<T> {
    pub fn new(config: NanoleafConfig, transport: T) -> Result<Self, NanoleafError> {
        let endpoint = config.endpoint()?;
        Ok(Self {
            config,
            endpoint,
            transport,
        })
    }

    pub fn pair_authorized(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        principal_id: AgentId,
        now_ms: u64,
    ) -> Result<NanoleafCredentials, NanoleafError> {
        authorize(runtime, principal_id, SmartHomeTool::PairBridge, now_ms)?;
        self.pair()
    }

    pub fn pair(&mut self) -> Result<NanoleafCredentials, NanoleafError> {
        let template = LocalHttpRequestTemplate::new(LocalHttpMethod::Post, "/api/v1/new")?
            .with_accept("application/json")
            .with_content_type("application/json")
            .with_timeout_ms(duration_ms(self.config.timeout));
        let bytes = self
            .transport
            .execute(&template.plan(&self.endpoint, b"{}".to_vec())?)?;
        let response: NanoleafPairingResponse = serde_json::from_slice(&bytes)?;
        NanoleafCredentials::new(response.auth_token)
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }
}

pub struct NanoleafClient<T> {
    config: NanoleafConfig,
    credentials: NanoleafCredentials,
    endpoint: LocalHttpEndpoint,
    transport: T,
}

impl<T: NanoleafTransport> NanoleafClient<T> {
    pub fn new(
        config: NanoleafConfig,
        credentials: NanoleafCredentials,
        transport: T,
    ) -> Result<Self, NanoleafError> {
        let endpoint = config.endpoint()?;
        Ok(Self {
            config,
            credentials,
            endpoint,
            transport,
        })
    }

    pub fn config(&self) -> &NanoleafConfig {
        &self.config
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    pub fn inspect(&mut self) -> Result<NanoleafSnapshot, NanoleafError> {
        let path = format!("/api/v1/{}", self.credentials.token());
        let template = LocalHttpRequestTemplate::new(LocalHttpMethod::Get, path)?
            .with_accept("application/json")
            .with_timeout_ms(duration_ms(self.config.timeout));
        let bytes = self
            .transport
            .execute(&template.plan(&self.endpoint, Vec::new())?)?;
        let snapshot: NanoleafSnapshot = serde_json::from_slice(&bytes)?;
        validate_snapshot(&snapshot)?;
        Ok(snapshot)
    }

    pub fn update_state(&mut self, update: &JsonValue) -> Result<(), NanoleafError> {
        let path = format!("/api/v1/{}/state", self.credentials.token());
        let template = LocalHttpRequestTemplate::new(LocalHttpMethod::Put, path)?
            .with_accept("application/json")
            .with_content_type("application/json")
            .with_timeout_ms(duration_ms(self.config.timeout));
        let body = Zeroizing::new(serde_json::to_vec(update)?);
        self.transport
            .execute(&template.plan(&self.endpoint, body.to_vec())?)?;
        Ok(())
    }
}

impl<T> fmt::Debug for NanoleafClient<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NanoleafClient")
            .field("config", &self.config)
            .field("credentials", &self.credentials)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledNanoleafDevice {
    pub bridge_id: BridgeId,
    pub device_id: DeviceId,
    pub entity_id: EntityId,
}

pub struct NanoleafRuntimeIntegration<T> {
    client: NanoleafClient<T>,
    entity_id: Option<EntityId>,
    color_temperature_range: Option<(i64, i64)>,
}

impl<T: NanoleafTransport> NanoleafRuntimeIntegration<T> {
    pub fn new(client: NanoleafClient<T>) -> Self {
        Self {
            client,
            entity_id: None,
            color_temperature_range: None,
        }
    }

    pub fn client(&self) -> &NanoleafClient<T> {
        &self.client
    }

    pub fn inspect_and_install_authorized(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        principal_id: AgentId,
        observed_at_ms: u64,
    ) -> Result<InstalledNanoleafDevice, NanoleafError> {
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
        snapshot: &NanoleafSnapshot,
        observed_at_ms: u64,
    ) -> Result<InstalledNanoleafDevice, NanoleafError> {
        validate_snapshot(snapshot)?;
        let native_id = stable_component(&snapshot.serial_number);
        let bridge_id = self.client.config.bridge_id.clone();
        let device_id = DeviceId::trusted(format!("nanoleaf:{native_id}"));
        let entity_id = EntityId::trusted(format!("nanoleaf:{native_id}:light"));

        let endpoint = self.client.config.endpoint()?;
        let mut bridge = Bridge::new(
            bridge_id.clone(),
            IntegrationId::trusted(INTEGRATION_ID),
            BridgeTransport::LanHttp,
        );
        bridge.address = Some(endpoint.origin());
        bridge.hardware_model = Some(snapshot.model.clone());
        bridge.firmware_version = Some(snapshot.firmware_version.clone());
        bridge.auth_ref = Some(self.client.config.credential_ref.clone());
        bridge.health = Health::Online;
        bridge.last_seen_at_ms = Some(observed_at_ms);
        bridge.identifiers = vec![protocol_identifier("http_endpoint", &endpoint.origin())?];
        bridge.metadata = vec![Metadata::new(
            "nanoleaf.transport",
            "authenticated_local_http_polling",
        )];
        runtime.upsert_bridge(bridge)?;

        runtime.upsert_device(Device {
            device_id: device_id.clone(),
            bridge_id: bridge_id.clone(),
            manufacturer: snapshot.manufacturer.clone(),
            model: snapshot.model.clone(),
            name: display_name(snapshot),
            serial: Some(snapshot.serial_number.clone()),
            firmware_version: Some(snapshot.firmware_version.clone()),
            room_id: None,
            entity_ids: vec![entity_id.clone()],
            identifiers: vec![protocol_identifier("serial", &snapshot.serial_number)?],
            health: Health::Online,
            metadata: vec![Metadata::new(
                "nanoleaf.color_mode",
                snapshot.state.color_mode.clone(),
            )],
        })?;

        runtime.upsert_entity(Entity {
            entity_id: entity_id.clone(),
            device_id: device_id.clone(),
            kind: EntityKind::Light,
            name: display_name(snapshot),
            capabilities: vec![
                Capability::light_on_off(),
                Capability::light_brightness(),
                Capability::light_color(),
                Capability::light_color_temperature(),
            ],
            state: Some(StateSnapshot {
                entity_id: entity_id.clone(),
                value: normalized_state(&snapshot.state),
                source: StateSource::Poll,
                observed_at_ms,
                received_at_ms: observed_at_ms,
                expires_at_ms: None,
                confidence: StateConfidence::Confirmed,
            }),
            metadata: vec![Metadata::new("nanoleaf.scope", "device")],
        })?;

        let min = snapshot.state.ct.min.unwrap_or(1_200);
        let max = snapshot.state.ct.max.unwrap_or(6_500);
        self.entity_id = Some(entity_id.clone());
        self.color_temperature_range = Some((min, max));
        Ok(InstalledNanoleafDevice {
            bridge_id,
            device_id,
            entity_id,
        })
    }

    pub fn dispatch_command(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        principal_id: AgentId,
        request: RuntimeCommandToolRequest,
        now_ms: u64,
    ) -> Result<CommandResult, NanoleafError> {
        if self.entity_id.as_ref() != Some(&request.entity_id) {
            return Err(NanoleafError::UnknownEntity(request.entity_id.clone()));
        }
        let body = command_body(&request, self.color_temperature_range)?;
        let result = runtime.execute_command_tool(principal_id, request.clone(), now_ms)?;
        self.client.update_state(&body)?;
        let verified = self.client.inspect()?;
        verify_command(&request, &verified.state)?;
        self.install_snapshot(runtime, &verified, now_ms)?;
        Ok(result)
    }
}

fn authorize(
    runtime: &mut SmartHomeRuntime,
    principal_id: AgentId,
    tool: SmartHomeTool,
    now_ms: u64,
) -> Result<(), NanoleafError> {
    let decision = runtime.authorize_tool_for_principal(principal_id.clone(), tool, now_ms);
    if decision.missing_capabilities.is_empty() {
        Ok(())
    } else {
        Err(NanoleafError::Runtime(RuntimeError::UnauthorizedTool {
            principal_id,
            tool,
            missing_capabilities: decision.missing_capabilities,
        }))
    }
}

fn validate_snapshot(snapshot: &NanoleafSnapshot) -> Result<(), NanoleafError> {
    if snapshot.serial_number.trim().is_empty() {
        return Err(NanoleafError::MissingField("serialNo"));
    }
    if snapshot.model.trim().is_empty() {
        return Err(NanoleafError::MissingField("model"));
    }
    if !(0..=100).contains(&snapshot.state.brightness.value)
        || !(0..=360).contains(&snapshot.state.hue.value)
        || !(0..=100).contains(&snapshot.state.sat.value)
        || snapshot.state.ct.value <= 0
    {
        return Err(NanoleafError::Validation(
            "state values are outside Nanoleaf API ranges".to_string(),
        ));
    }
    Ok(())
}

fn display_name(snapshot: &NanoleafSnapshot) -> String {
    if snapshot.name.trim().is_empty() {
        "Nanoleaf".to_string()
    } else {
        snapshot.name.clone()
    }
}

fn normalized_state(state: &NanoleafState) -> Value {
    let color = hsv_to_rgb(state.hue.value, state.sat.value, 100);
    Value::Object(vec![
        ("on".to_string(), Value::Bool(state.on.value)),
        (
            "brightness".to_string(),
            Value::Percentage(u8::try_from(state.brightness.value).unwrap_or(0)),
        ),
        (
            "color".to_string(),
            Value::Array(
                color
                    .into_iter()
                    .map(|channel| Value::Integer(i64::from(channel)))
                    .collect(),
            ),
        ),
        ("hue".to_string(), Value::Integer(state.hue.value)),
        ("saturation".to_string(), Value::Integer(state.sat.value)),
        (
            "color_temperature_mirek".to_string(),
            Value::Integer(1_000_000 / state.ct.value),
        ),
        (
            "color_mode".to_string(),
            Value::Text(state.color_mode.clone()),
        ),
    ])
}

fn command_body(
    request: &RuntimeCommandToolRequest,
    color_temperature_range: Option<(i64, i64)>,
) -> Result<JsonValue, NanoleafError> {
    match request.command_type {
        CommandType::TurnOn => Ok(json!({"on": {"value": true}})),
        CommandType::TurnOff => Ok(json!({"on": {"value": false}})),
        CommandType::SetBrightness => {
            let Value::Percentage(percent) = request.arguments else {
                return invalid_arguments(request.command_type, "percentage from 0 through 100");
            };
            Ok(json!({"brightness": {"value": percent}}))
        }
        CommandType::SetColor => {
            let channels = rgb_channels(&request.arguments, request.command_type)?;
            let (hue, saturation) = rgb_to_hsv(channels);
            Ok(json!({
                "hue": {"value": hue},
                "sat": {"value": saturation},
            }))
        }
        CommandType::SetColorTemperature => {
            let Value::Integer(mirek) = request.arguments else {
                return invalid_arguments(request.command_type, "positive integer mirek");
            };
            if mirek <= 0 {
                return invalid_arguments(request.command_type, "positive integer mirek");
            }
            let kelvin = 1_000_000 / mirek;
            if let Some((min, max)) = color_temperature_range {
                if !(min..=max).contains(&kelvin) {
                    return invalid_arguments(
                        request.command_type,
                        "mirek value within the device color-temperature range",
                    );
                }
            }
            Ok(json!({"ct": {"value": kelvin}}))
        }
        _ => Err(NanoleafError::UnsupportedCommand {
            entity_id: request.entity_id.clone(),
            command_type: request.command_type,
        }),
    }
}

fn verify_command(
    request: &RuntimeCommandToolRequest,
    state: &NanoleafState,
) -> Result<(), NanoleafError> {
    let matches = match request.command_type {
        CommandType::TurnOn => state.on.value,
        CommandType::TurnOff => !state.on.value,
        CommandType::SetBrightness => {
            matches!(request.arguments, Value::Percentage(value) if i64::from(value) == state.brightness.value)
        }
        CommandType::SetColor => {
            let channels = rgb_channels(&request.arguments, request.command_type)?;
            let (hue, saturation) = rgb_to_hsv(channels);
            (state.hue.value - hue).abs() <= 1 && (state.sat.value - saturation).abs() <= 1
        }
        CommandType::SetColorTemperature => {
            matches!(request.arguments, Value::Integer(mirek) if mirek > 0 && 1_000_000 / mirek == state.ct.value)
        }
        _ => false,
    };
    if matches {
        Ok(())
    } else {
        Err(NanoleafError::VerificationFailed(format!(
            "device state did not confirm {:?}",
            request.command_type
        )))
    }
}

fn rgb_channels(arguments: &Value, command_type: CommandType) -> Result<[u8; 3], NanoleafError> {
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
        .ok_or(NanoleafError::InvalidCommandArguments {
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
        h if h < 60.0 => (chroma, x, 0.0),
        h if h < 120.0 => (x, chroma, 0.0),
        h if h < 180.0 => (0.0, chroma, x),
        h if h < 240.0 => (0.0, x, chroma),
        h if h < 300.0 => (x, 0.0, chroma),
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
) -> Result<T, NanoleafError> {
    Err(NanoleafError::InvalidCommandArguments {
        command_type,
        expected,
    })
}

fn protocol_identifier(kind: &str, value: &str) -> Result<ProtocolIdentifier, NanoleafError> {
    ProtocolIdentifier::new(ProtocolFamily::Vendor(PROTOCOL_ID.to_string()), kind, value)
        .map_err(|error| NanoleafError::Validation(error.to_string()))
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

fn encode_http_request(url: &Url, plan: &LocalHttpRequestPlan) -> Result<Vec<u8>, NanoleafError> {
    let host = url
        .host
        .as_deref()
        .ok_or(NanoleafError::MissingField("URL host"))?;
    let port = url
        .effective_port()
        .ok_or(NanoleafError::MissingField("URL port"))?;
    let target = if url.path.is_empty() {
        "/".to_string()
    } else if let Some(query) = &url.query {
        format!("{}?{query}", url.path)
    } else {
        url.path.clone()
    };
    if has_unsafe_http_text(&target) || has_unsafe_http_text(host) {
        return Err(NanoleafError::Validation(
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
            return Err(NanoleafError::Validation(
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

fn connect_tcp(host: &str, port: u16, timeout: Duration) -> Result<TcpStream, NanoleafError> {
    let addresses = (host, port)
        .to_socket_addrs()
        .map_err(|error| NanoleafError::Io(error.to_string()))?
        .collect::<Vec<_>>();
    let mut last_error = None;
    for address in addresses {
        match TcpStream::connect_timeout(&address, timeout) {
            Ok(stream) => {
                stream
                    .set_read_timeout(Some(timeout))
                    .map_err(|error| NanoleafError::Io(error.to_string()))?;
                stream
                    .set_write_timeout(Some(timeout))
                    .map_err(|error| NanoleafError::Io(error.to_string()))?;
                return Ok(stream);
            }
            Err(error) => last_error = Some(error),
        }
    }
    Err(NanoleafError::Io(
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

fn read_bounded(reader: &mut dyn Read, maximum: usize) -> Result<Vec<u8>, NanoleafError> {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| NanoleafError::Io(error.to_string()))?;
        if read == 0 {
            break;
        }
        if read > maximum.saturating_sub(bytes.len()) {
            return Err(NanoleafError::ResponseTooLarge { limit: maximum });
        }
        bytes.extend_from_slice(&buffer[..read]);
    }
    Ok(bytes)
}

fn decode_http_response(bytes: &[u8], maximum: usize) -> Result<Vec<u8>, NanoleafError> {
    let parsed = parse_response_head(bytes)
        .map_err(|error: Http1ParseError| NanoleafError::Http(error.to_string()))?;
    if !(200..300).contains(&parsed.head.status) {
        return Err(NanoleafError::HttpStatus(parsed.head.status));
    }
    let input = &bytes[parsed.body_offset..];
    let body = match parsed.body_kind {
        BodyKind::None => Vec::new(),
        BodyKind::ContentLength(expected) => {
            if input.len() < expected {
                return Err(NanoleafError::TruncatedBody {
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
        return Err(NanoleafError::ResponseTooLarge { limit: maximum });
    }
    Ok(body)
}

fn decode_chunked(input: &[u8], maximum: usize) -> Result<Vec<u8>, NanoleafError> {
    let mut cursor = 0usize;
    let mut output = Vec::new();
    loop {
        let offset = input[cursor..]
            .windows(2)
            .position(|window| window == b"\r\n")
            .ok_or_else(|| NanoleafError::Http("missing chunk-size terminator".to_string()))?;
        let end = cursor + offset;
        let size_text = std::str::from_utf8(&input[cursor..end])
            .map_err(|_| NanoleafError::Http("chunk size is not ASCII".to_string()))?
            .split(';')
            .next()
            .unwrap_or_default();
        let size = usize::from_str_radix(size_text.trim(), 16)
            .map_err(|_| NanoleafError::Http("invalid chunk size".to_string()))?;
        cursor = end + 2;
        if size == 0 {
            return Ok(output);
        }
        if size > maximum.saturating_sub(output.len()) {
            return Err(NanoleafError::ResponseTooLarge { limit: maximum });
        }
        let chunk_end = cursor
            .checked_add(size)
            .ok_or_else(|| NanoleafError::Http("chunk size overflow".to_string()))?;
        if input.len() < chunk_end + 2 || &input[chunk_end..chunk_end + 2] != b"\r\n" {
            return Err(NanoleafError::Http("truncated chunk".to_string()));
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

    const TOKEN: &str = "0123456789abcdef0123456789abcdef";
    const SNAPSHOT: &str = r#"{"name":"Studio Panels","serialNo":"NL42ABC","manufacturer":"Nanoleaf","firmwareVersion":"9.4.1","model":"NL29","state":{"on":{"value":true},"brightness":{"value":40,"min":0,"max":100},"hue":{"value":30,"min":0,"max":360},"sat":{"value":50,"min":0,"max":100},"ct":{"value":4000,"min":1200,"max":6500},"colorMode":"hs"}}"#;
    const VERIFIED: &str = r#"{"name":"Studio Panels","serialNo":"NL42ABC","manufacturer":"Nanoleaf","firmwareVersion":"9.4.1","model":"NL29","state":{"on":{"value":true},"brightness":{"value":40,"min":0,"max":100},"hue":{"value":210,"min":0,"max":360},"sat":{"value":67,"min":0,"max":100},"ct":{"value":4000,"min":1200,"max":6500},"colorMode":"hs"}}"#;

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

    fn config(port: u16) -> NanoleafConfig {
        NanoleafConfig::new(
            BridgeId::trusted("nanoleaf.test"),
            format!("http://127.0.0.1:{port}"),
            VaultRef::trusted("vault:nanoleaf/studio"),
        )
        .unwrap()
    }

    fn grant(runtime: &mut SmartHomeRuntime, principal: &AgentId) {
        let _ = runtime.registry_mut().upsert_capability_grant(
            CapabilityGrant::for_all_smart_home(
                CapabilityGrantId::trusted("grant:nanoleaf-test"),
                principal.clone(),
                PrivilegeTier::HighRisk,
                "test",
                1_000,
            )
            .with_expiry(20_000),
        );
    }

    #[test]
    fn mdns_advertisement_requires_physical_presence_pairing() {
        let advertisement = MdnsAdvertisement::new(
            MDNS_SERVICE_TYPE,
            "Studio Panels",
            "nanoleaf.local",
            DEFAULT_PORT,
            1_000,
        )
        .unwrap()
        .with_address("192.0.2.30")
        .unwrap()
        .with_txt("id", "NL42ABC")
        .unwrap();
        let record = discovery_record(&advertisement).unwrap();
        assert_eq!(record.native_bridge_id, "nl42abc");
        assert_eq!(record.address.as_deref(), Some("http://192.0.2.30:16021"));
        assert_eq!(record.confidence, DiscoveryConfidence::Verified);
        assert_eq!(
            record.pairing_requirement,
            PairingRequirement::PhysicalPresence
        );
    }

    #[test]
    fn real_tcp_pairing_returns_redacted_credentials() {
        let (port, requests, handle) =
            start_server(vec![response(&format!(r#"{{"auth_token":"{TOKEN}"}}"#))]);
        let principal = AgentId::trusted("agent:nanoleaf-pair");
        let mut runtime = SmartHomeRuntime::new();
        grant(&mut runtime, &principal);
        let mut pairing =
            NanoleafPairingClient::new(config(port), NanoleafLanTransport::default()).unwrap();
        let credentials = pairing
            .pair_authorized(&mut runtime, principal, 5_000)
            .unwrap();
        handle.join().unwrap();
        assert_eq!(credentials.token(), TOKEN);
        assert_eq!(
            format!("{credentials:?}"),
            "NanoleafCredentials([REDACTED])"
        );
        assert!(requests.lock().unwrap()[0].starts_with("POST /api/v1/new HTTP/1.1"));
    }

    #[test]
    fn real_tcp_inspection_installs_authenticated_light() {
        let (port, requests, handle) = start_server(vec![response(SNAPSHOT)]);
        let client = NanoleafClient::new(
            config(port),
            NanoleafCredentials::new(TOKEN).unwrap(),
            NanoleafLanTransport::default(),
        )
        .unwrap();
        let mut integration = NanoleafRuntimeIntegration::new(client);
        let principal = AgentId::trusted("agent:nanoleaf-read");
        let mut runtime = SmartHomeRuntime::new();
        grant(&mut runtime, &principal);
        let installed = integration
            .inspect_and_install_authorized(&mut runtime, principal, 5_000)
            .unwrap();
        handle.join().unwrap();
        let entity = runtime.registry().entity(&installed.entity_id).unwrap();
        assert_eq!(entity.capabilities.len(), 4);
        let bridge = runtime.registry().bridge(&installed.bridge_id).unwrap();
        assert_eq!(
            bridge.auth_ref.as_ref().map(VaultRef::as_str),
            Some("vault:nanoleaf/studio")
        );
        let runtime_debug = format!("{runtime:?}");
        assert!(!runtime_debug.contains(TOKEN));
        assert!(requests.lock().unwrap()[0].starts_with(&format!("GET /api/v1/{TOKEN} HTTP/1.1")));
    }

    #[test]
    fn authorized_color_command_is_verified_over_real_tcp() {
        let (port, requests, handle) =
            start_server(vec![response(SNAPSHOT), response("{}"), response(VERIFIED)]);
        let client = NanoleafClient::new(
            config(port),
            NanoleafCredentials::new(TOKEN).unwrap(),
            NanoleafLanTransport::default(),
        )
        .unwrap();
        let mut integration = NanoleafRuntimeIntegration::new(client);
        let principal = AgentId::trusted("agent:nanoleaf-command");
        let mut runtime = SmartHomeRuntime::new();
        grant(&mut runtime, &principal);
        let installed = integration
            .inspect_and_install_authorized(&mut runtime, principal.clone(), 5_000)
            .unwrap();
        let result = integration
            .dispatch_command(
                &mut runtime,
                principal,
                RuntimeCommandToolRequest::new(
                    installed.entity_id.clone(),
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
        assert!(requests[1].starts_with(&format!("PUT /api/v1/{TOKEN}/state HTTP/1.1")));
        assert!(requests[1].contains(r#""hue":{"value":210}"#));
        assert!(requests[1].contains(r#""sat":{"value":67}"#));
        assert!(requests[2].starts_with(&format!("GET /api/v1/{TOKEN} HTTP/1.1")));
        assert_eq!(
            runtime
                .registry()
                .state(&installed.entity_id)
                .unwrap()
                .confidence,
            StateConfidence::Confirmed
        );
    }

    #[derive(Debug)]
    struct CountingTransport(Arc<AtomicUsize>);

    impl NanoleafTransport for CountingTransport {
        fn execute(&mut self, _plan: &LocalHttpRequestPlan) -> Result<Vec<u8>, NanoleafError> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(Vec::new())
        }
    }

    #[test]
    fn denied_read_and_pair_reach_no_transport() {
        let calls = Arc::new(AtomicUsize::new(0));
        let denied_config = NanoleafConfig::new(
            BridgeId::trusted("nanoleaf.denied"),
            "http://127.0.0.1:16021",
            VaultRef::trusted("vault:nanoleaf/denied"),
        )
        .unwrap();
        let client = NanoleafClient::new(
            denied_config.clone(),
            NanoleafCredentials::new(TOKEN).unwrap(),
            CountingTransport(Arc::clone(&calls)),
        )
        .unwrap();
        let mut integration = NanoleafRuntimeIntegration::new(client);
        assert!(matches!(
            integration.inspect_and_install_authorized(
                &mut SmartHomeRuntime::new(),
                AgentId::trusted("agent:denied"),
                5_000,
            ),
            Err(NanoleafError::Runtime(_))
        ));
        let mut pairing =
            NanoleafPairingClient::new(denied_config, CountingTransport(Arc::clone(&calls)))
                .unwrap();
        assert!(matches!(
            pairing.pair_authorized(
                &mut SmartHomeRuntime::new(),
                AgentId::trusted("agent:denied"),
                5_000,
            ),
            Err(NanoleafError::Runtime(_))
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn command_conversion_and_limits_match_nanoleaf_api() {
        let default_port = NanoleafConfig::new(
            BridgeId::trusted("nanoleaf.default-port"),
            "http://192.0.2.50",
            VaultRef::trusted("vault:nanoleaf/default-port"),
        )
        .unwrap();
        assert_eq!(
            default_port.endpoint().unwrap().origin(),
            "http://192.0.2.50:16021"
        );

        let request = RuntimeCommandToolRequest::new(
            EntityId::trusted("nanoleaf:test:light"),
            CommandType::SetColorTemperature,
            Value::Integer(250),
        );
        assert_eq!(
            command_body(&request, Some((1_200, 6_500))).unwrap(),
            json!({"ct": {"value": 4000}})
        );
        assert_eq!(rgb_to_hsv([255, 0, 0]), (0, 100));
        assert_eq!(hsv_to_rgb(120, 100, 100), [0, 255, 0]);
        assert!(matches!(
            decode_http_response(&response("{}"), 1),
            Err(NanoleafError::ResponseTooLarge { limit: 1 })
        ));
    }
}
