//! Production TP-Link Kasa legacy LAN integration for D23.

#![forbid(unsafe_code)]

use serde_json::{json, Map as JsonMap, Value as JsonValue};
use smart_home_core::{
    AgentId, Bridge, BridgeId, BridgeTransport, Capability, CommandResult, CommandType, Device,
    DeviceId, Entity, EntityId, EntityKind, Health, IntegrationId, Metadata, ProtocolFamily,
    ProtocolIdentifier, StateConfidence, StateSnapshot, StateSource, Value,
};
use smart_home_discovery::{
    DiscoveryConfidence, DiscoveryRecord, DiscoverySource, PairingRequirement,
};
use smart_home_runtime::{RuntimeCommandToolRequest, RuntimeError, SmartHomeRuntime};
use std::fmt;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;
use udp_client::{UdpClient, UdpDatagram, UdpError, UdpOptions};

pub const VERSION: &str = "0.1.0";
pub const INTEGRATION_ID: &str = "tplink";
pub const PROTOCOL_ID: &str = "kasa_legacy_lan";
pub const KASA_PORT: u16 = 9_999;
pub const DEFAULT_MAX_DATAGRAM_BYTES: usize = 16 * 1024;
pub const DEFAULT_MAX_SCAN_RESPONSES: usize = 128;
const XOR_SEED: u8 = 0xab;
const BULB_SERVICE: &str = "smartlife.iot.smartbulb.lightingservice";

#[derive(Debug)]
pub enum KasaError {
    Validation(String),
    Udp(UdpError),
    Json(serde_json::Error),
    NoResponse,
    SourceMismatch {
        expected: IpAddr,
        actual: IpAddr,
    },
    DeviceError {
        operation: String,
        code: i64,
    },
    UnsupportedProtocol,
    UnknownEntity(EntityId),
    UnsupportedCommand {
        entity_id: EntityId,
        command_type: CommandType,
    },
    InvalidCommandArguments {
        command_type: CommandType,
        expected: &'static str,
    },
    VerificationFailed {
        command_type: CommandType,
    },
    Runtime(RuntimeError),
}

impl fmt::Display for KasaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => write!(formatter, "invalid Kasa LAN input: {message}"),
            Self::Udp(error) => error.fmt(formatter),
            Self::Json(error) => write!(formatter, "invalid Kasa LAN JSON: {error}"),
            Self::NoResponse => formatter.write_str("Kasa LAN device did not respond"),
            Self::SourceMismatch { expected, actual } => {
                write!(
                    formatter,
                    "Kasa reply expected from {expected}, arrived from {actual}"
                )
            }
            Self::DeviceError { operation, code } => {
                write!(
                    formatter,
                    "Kasa operation {operation} failed with error {code}"
                )
            }
            Self::UnsupportedProtocol => formatter
                .write_str("device did not expose the credential-free Kasa legacy LAN protocol"),
            Self::UnknownEntity(entity_id) => write!(formatter, "unknown Kasa entity {entity_id}"),
            Self::UnsupportedCommand {
                entity_id,
                command_type,
            } => {
                write!(
                    formatter,
                    "Kasa entity {entity_id} does not support {command_type:?}"
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
            Self::VerificationFailed { command_type } => {
                write!(formatter, "Kasa device did not confirm {command_type:?}")
            }
            Self::Runtime(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for KasaError {}

impl From<UdpError> for KasaError {
    fn from(error: UdpError) -> Self {
        Self::Udp(error)
    }
}

impl From<serde_json::Error> for KasaError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<RuntimeError> for KasaError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KasaDeviceKind {
    Switch,
    Light,
}

impl KasaDeviceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Switch => "switch",
            Self::Light => "light",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KasaStatus {
    pub device_id: String,
    pub alias: String,
    pub model: String,
    pub mac: Option<String>,
    pub firmware_version: Option<String>,
    pub kind: KasaDeviceKind,
    pub on: bool,
    pub supports_brightness: bool,
    pub supports_color: bool,
    pub supports_color_temperature: bool,
    pub brightness: Option<u8>,
    pub hue: Option<u16>,
    pub saturation: Option<u8>,
    pub color_temperature_kelvin: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KasaAdvertisement {
    pub host: IpAddr,
    pub port: u16,
    pub status: KasaStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KasaScanConfig {
    pub destination: SocketAddr,
    pub response_port: u16,
    pub timeout: Duration,
    pub maximum_datagram_bytes: usize,
    pub maximum_responses: usize,
    pub broadcast: bool,
}

impl Default for KasaScanConfig {
    fn default() -> Self {
        Self {
            destination: SocketAddr::new(IpAddr::V4(Ipv4Addr::BROADCAST), KASA_PORT),
            response_port: 0,
            timeout: Duration::from_secs(2),
            maximum_datagram_bytes: DEFAULT_MAX_DATAGRAM_BYTES,
            maximum_responses: DEFAULT_MAX_SCAN_RESPONSES,
            broadcast: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KasaScanResult {
    pub devices: Vec<KasaAdvertisement>,
    pub failures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KasaDeviceConfig {
    pub bridge_id: BridgeId,
    pub host: IpAddr,
    pub port: u16,
    pub response_port: u16,
    pub timeout: Duration,
    pub maximum_datagram_bytes: usize,
}

impl KasaDeviceConfig {
    pub fn new(bridge_id: BridgeId, host: impl AsRef<str>) -> Result<Self, KasaError> {
        let host = host
            .as_ref()
            .parse::<IpAddr>()
            .map_err(|_| KasaError::Validation("host must be a fixed IP address".to_string()))?;
        Ok(Self {
            bridge_id,
            host,
            port: KASA_PORT,
            response_port: 0,
            timeout: Duration::from_secs(2),
            maximum_datagram_bytes: DEFAULT_MAX_DATAGRAM_BYTES,
        })
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub fn with_response_port(mut self, response_port: u16) -> Self {
        self.response_port = response_port;
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn endpoint(&self) -> SocketAddr {
        SocketAddr::new(self.host, self.port)
    }
}

pub trait KasaTransport {
    fn exchange(
        &mut self,
        destination: SocketAddr,
        payload: &[u8],
        options: &KasaExchangeOptions,
    ) -> Result<Vec<UdpDatagram>, KasaError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KasaExchangeOptions {
    pub response_port: u16,
    pub timeout: Duration,
    pub maximum_datagram_bytes: usize,
    pub maximum_responses: usize,
    pub broadcast: bool,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct KasaLanTransport;

impl KasaTransport for KasaLanTransport {
    fn exchange(
        &mut self,
        destination: SocketAddr,
        payload: &[u8],
        options: &KasaExchangeOptions,
    ) -> Result<Vec<UdpDatagram>, KasaError> {
        if options.maximum_responses == 0 {
            return Err(KasaError::Validation(
                "maximum responses must be positive".to_string(),
            ));
        }
        let client = UdpClient::bind(udp_options(
            options.response_port,
            options.timeout,
            options.maximum_datagram_bytes,
        ))?;
        client.set_broadcast(options.broadcast)?;
        client.send_to(payload, destination)?;
        let mut datagrams = Vec::new();
        while datagrams.len() < options.maximum_responses {
            match client.recv_from() {
                Ok(datagram) => datagrams.push(datagram),
                Err(UdpError::Timeout) => break,
                Err(error) => return Err(error.into()),
            }
        }
        Ok(datagrams)
    }
}

pub fn scan_lan<T: KasaTransport>(
    config: &KasaScanConfig,
    transport: &mut T,
) -> Result<KasaScanResult, KasaError> {
    let request = encode_json(&sysinfo_request())?;
    let datagrams = transport.exchange(
        config.destination,
        &request,
        &KasaExchangeOptions {
            response_port: config.response_port,
            timeout: config.timeout,
            maximum_datagram_bytes: config.maximum_datagram_bytes,
            maximum_responses: config.maximum_responses,
            broadcast: config.broadcast,
        },
    )?;
    let mut devices = Vec::new();
    let mut failures = Vec::new();
    for datagram in datagrams {
        match parse_status_datagram(&datagram) {
            Ok(status) => devices.push(KasaAdvertisement {
                host: datagram.source.ip(),
                port: datagram.source.port(),
                status,
            }),
            Err(error) => failures.push(error.to_string()),
        }
    }
    devices.sort_by(|left, right| left.status.device_id.cmp(&right.status.device_id));
    devices.dedup_by(|left, right| left.status.device_id == right.status.device_id);
    Ok(KasaScanResult { devices, failures })
}

pub fn discovery_record(
    advertisement: &KasaAdvertisement,
    discovered_at_ms: u64,
) -> Result<DiscoveryRecord, KasaError> {
    let native_id = stable_component(&advertisement.status.device_id);
    if native_id.is_empty() {
        return Err(KasaError::Validation(
            "device id does not contain a stable component".to_string(),
        ));
    }
    Ok(DiscoveryRecord::new(
        IntegrationId::trusted(INTEGRATION_ID),
        ProtocolFamily::Vendor(PROTOCOL_ID.to_string()),
        native_id,
        DiscoverySource::UdpBroadcast,
        BridgeTransport::LanUdp,
        discovered_at_ms,
    )
    .map_err(|error| KasaError::Validation(error.to_string()))?
    .with_display_name(advertisement.status.alias.clone())
    .with_address(format!(
        "udp://{}:{}",
        advertisement.host, advertisement.port
    ))
    .with_hardware_model(&advertisement.status.model)
    .with_confidence(DiscoveryConfidence::Verified)
    .with_pairing_requirement(PairingRequirement::None)
    .with_metadata("kasa.protocol", PROTOCOL_ID)
    .with_metadata("kasa.device_kind", advertisement.status.kind.as_str()))
}

#[derive(Debug)]
pub struct KasaClient<T> {
    config: KasaDeviceConfig,
    transport: T,
}

impl<T: KasaTransport> KasaClient<T> {
    pub fn new(config: KasaDeviceConfig, transport: T) -> Self {
        Self { config, transport }
    }

    pub fn config(&self) -> &KasaDeviceConfig {
        &self.config
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    pub fn inspect(&mut self) -> Result<KasaStatus, KasaError> {
        let response = self.query(&sysinfo_request())?;
        parse_status_response(&response)
    }

    fn command(&mut self, payload: &JsonValue, operation: &str) -> Result<(), KasaError> {
        let response = self.query(payload)?;
        let code = response
            .pointer(operation)
            .and_then(|value| value.get("err_code"))
            .and_then(JsonValue::as_i64)
            .ok_or_else(|| {
                KasaError::Validation("command response is missing err_code".to_string())
            })?;
        if code != 0 {
            return Err(KasaError::DeviceError {
                operation: operation.to_string(),
                code,
            });
        }
        Ok(())
    }

    fn query(&mut self, payload: &JsonValue) -> Result<JsonValue, KasaError> {
        let encoded = encode_json(payload)?;
        let datagrams = self.transport.exchange(
            self.config.endpoint(),
            &encoded,
            &KasaExchangeOptions {
                response_port: self.config.response_port,
                timeout: self.config.timeout,
                maximum_datagram_bytes: self.config.maximum_datagram_bytes,
                maximum_responses: 1,
                broadcast: false,
            },
        )?;
        let datagram = datagrams.first().ok_or(KasaError::NoResponse)?;
        if datagram.source.ip() != self.config.host {
            return Err(KasaError::SourceMismatch {
                expected: self.config.host,
                actual: datagram.source.ip(),
            });
        }
        decode_json(&datagram.payload)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledKasaDevice {
    pub bridge_id: BridgeId,
    pub device_id: DeviceId,
    pub entity_id: EntityId,
}

#[derive(Debug)]
pub struct KasaRuntimeIntegration<T> {
    client: KasaClient<T>,
    entity_id: Option<EntityId>,
    status: Option<KasaStatus>,
}

impl<T: KasaTransport> KasaRuntimeIntegration<T> {
    pub fn new(client: KasaClient<T>) -> Self {
        Self {
            client,
            entity_id: None,
            status: None,
        }
    }

    pub fn client(&self) -> &KasaClient<T> {
        &self.client
    }

    pub fn inspect_and_install(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        observed_at_ms: u64,
    ) -> Result<InstalledKasaDevice, KasaError> {
        let status = self.client.inspect()?;
        self.install_status(runtime, &status, observed_at_ms)
    }

    pub fn install_status(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        status: &KasaStatus,
        observed_at_ms: u64,
    ) -> Result<InstalledKasaDevice, KasaError> {
        let native_id = stable_component(&status.device_id);
        if native_id.is_empty() {
            return Err(KasaError::Validation("device id is empty".to_string()));
        }
        let device_id = DeviceId::trusted(format!("kasa:{native_id}"));
        let suffix = status.kind.as_str();
        let entity_id = EntityId::trusted(format!("kasa:{native_id}:{suffix}"));
        let address = format!(
            "udp://{}:{}",
            self.client.config.host, self.client.config.port
        );
        let mut bridge = Bridge::new(
            self.client.config.bridge_id.clone(),
            IntegrationId::trusted(INTEGRATION_ID),
            BridgeTransport::LanUdp,
        );
        bridge.address = Some(address.clone());
        bridge.hardware_model = Some(status.model.clone());
        bridge.firmware_version = status.firmware_version.clone();
        bridge.health = Health::Online;
        bridge.last_seen_at_ms = Some(observed_at_ms);
        bridge.identifiers = vec![protocol_identifier("udp_endpoint", &address)?];
        bridge.metadata = vec![Metadata::new("kasa.transport", "legacy_xor_udp_polling")];
        runtime.upsert_bridge(bridge)?;

        let mut identifiers = vec![protocol_identifier("device_id", &status.device_id)?];
        if let Some(mac) = &status.mac {
            identifiers.push(protocol_identifier("mac", mac)?);
        }
        runtime.upsert_device(Device {
            device_id: device_id.clone(),
            bridge_id: self.client.config.bridge_id.clone(),
            manufacturer: "TP-Link".to_string(),
            model: status.model.clone(),
            name: status.alias.clone(),
            serial: Some(status.device_id.clone()),
            firmware_version: status.firmware_version.clone(),
            room_id: None,
            entity_ids: vec![entity_id.clone()],
            identifiers,
            health: Health::Online,
            metadata: vec![
                Metadata::new("kasa.protocol", PROTOCOL_ID),
                Metadata::new("kasa.cloud_required", "false"),
            ],
        })?;
        runtime.upsert_entity(Entity {
            entity_id: entity_id.clone(),
            device_id: device_id.clone(),
            kind: match status.kind {
                KasaDeviceKind::Switch => EntityKind::Switch,
                KasaDeviceKind::Light => EntityKind::Light,
            },
            name: status.alias.clone(),
            capabilities: capabilities(status),
            state: Some(StateSnapshot {
                entity_id: entity_id.clone(),
                value: status_value(status),
                source: StateSource::Poll,
                observed_at_ms,
                received_at_ms: observed_at_ms,
                expires_at_ms: None,
                confidence: StateConfidence::Confirmed,
            }),
            metadata: vec![Metadata::new("kasa.device_kind", status.kind.as_str())],
        })?;
        self.entity_id = Some(entity_id.clone());
        self.status = Some(status.clone());
        Ok(InstalledKasaDevice {
            bridge_id: self.client.config.bridge_id.clone(),
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
    ) -> Result<CommandResult, KasaError> {
        if self.entity_id.as_ref() != Some(&request.entity_id) {
            return Err(KasaError::UnknownEntity(request.entity_id.clone()));
        }
        let status = self.status.as_ref().ok_or(KasaError::UnsupportedProtocol)?;
        let (payload, operation, expectation) = command_payload(status, &request)?;
        let command_type = request.command_type;
        let result = runtime.execute_command_tool(principal_id, request, now_ms)?;
        self.client.command(&payload, operation)?;
        let confirmed = self.client.inspect()?;
        if !expectation.matches(&confirmed) {
            return Err(KasaError::VerificationFailed { command_type });
        }
        self.install_status(runtime, &confirmed, now_ms)?;
        Ok(result)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CommandExpectation {
    Power(bool),
    Brightness(u8),
    Color { hue: u16, saturation: u8 },
    ColorTemperature(u16),
}

impl CommandExpectation {
    fn matches(&self, status: &KasaStatus) -> bool {
        match self {
            Self::Power(expected) => status.on == *expected,
            Self::Brightness(expected) => status.brightness == Some(*expected),
            Self::Color { hue, saturation } => {
                status.hue == Some(*hue) && status.saturation == Some(*saturation)
            }
            Self::ColorTemperature(expected) => status.color_temperature_kelvin == Some(*expected),
        }
    }
}

fn command_payload(
    status: &KasaStatus,
    request: &RuntimeCommandToolRequest,
) -> Result<(JsonValue, &'static str, CommandExpectation), KasaError> {
    match (status.kind, request.command_type) {
        (KasaDeviceKind::Switch, CommandType::TurnOn | CommandType::TurnOff) => {
            let on = request.command_type == CommandType::TurnOn;
            Ok((
                json!({"system": {"set_relay_state": {"state": i32::from(on)}}}),
                "/system/set_relay_state",
                CommandExpectation::Power(on),
            ))
        }
        (KasaDeviceKind::Light, CommandType::TurnOn | CommandType::TurnOff) => {
            let on = request.command_type == CommandType::TurnOn;
            bulb_command(
                json!({"on_off": i32::from(on)}),
                CommandExpectation::Power(on),
            )
        }
        (KasaDeviceKind::Light, CommandType::SetBrightness) if status.supports_brightness => {
            let Value::Percentage(brightness) = request.arguments else {
                return invalid_arguments(request.command_type, "percentage");
            };
            bulb_command(
                json!({"brightness": brightness}),
                CommandExpectation::Brightness(brightness),
            )
        }
        (KasaDeviceKind::Light, CommandType::SetColor) if status.supports_color => {
            let rgb = rgb_arguments(&request.arguments, request.command_type)?;
            let (hue, saturation) = rgb_to_hs(rgb);
            bulb_command(
                json!({"hue": hue, "saturation": saturation, "color_temp": 0}),
                CommandExpectation::Color { hue, saturation },
            )
        }
        (KasaDeviceKind::Light, CommandType::SetColorTemperature)
            if status.supports_color_temperature =>
        {
            let Value::Integer(mirek) = request.arguments else {
                return invalid_arguments(
                    request.command_type,
                    "integer mirek from 111 through 500",
                );
            };
            if !(111..=500).contains(&mirek) {
                return invalid_arguments(
                    request.command_type,
                    "integer mirek from 111 through 500",
                );
            }
            let kelvin = u16::try_from(1_000_000 / mirek).map_err(|_| {
                KasaError::InvalidCommandArguments {
                    command_type: request.command_type,
                    expected: "integer mirek from 111 through 500",
                }
            })?;
            bulb_command(
                json!({"color_temp": kelvin}),
                CommandExpectation::ColorTemperature(kelvin),
            )
        }
        _ => Err(KasaError::UnsupportedCommand {
            entity_id: request.entity_id.clone(),
            command_type: request.command_type,
        }),
    }
}

fn bulb_command(
    mut fields: JsonValue,
    expectation: CommandExpectation,
) -> Result<(JsonValue, &'static str, CommandExpectation), KasaError> {
    let object = fields
        .as_object_mut()
        .ok_or_else(|| KasaError::Validation("bulb command must be an object".to_string()))?;
    object.insert("transition_period".to_string(), json!(0));
    Ok((
        json!({BULB_SERVICE: {"transition_light_state": fields}}),
        "/smartlife.iot.smartbulb.lightingservice/transition_light_state",
        expectation,
    ))
}

fn sysinfo_request() -> JsonValue {
    json!({"system": {"get_sysinfo": {}}})
}

fn parse_status_datagram(datagram: &UdpDatagram) -> Result<KasaStatus, KasaError> {
    parse_status_response(&decode_json(&datagram.payload)?)
}

fn parse_status_response(response: &JsonValue) -> Result<KasaStatus, KasaError> {
    let sysinfo = response
        .pointer("/system/get_sysinfo")
        .and_then(JsonValue::as_object)
        .ok_or(KasaError::UnsupportedProtocol)?;
    let code = integer(sysinfo, "err_code").unwrap_or(0);
    if code != 0 {
        return Err(KasaError::DeviceError {
            operation: "get_sysinfo".to_string(),
            code,
        });
    }
    let device_id = text_any(sysinfo, &["deviceId", "device_id"])
        .ok_or_else(|| KasaError::Validation("get_sysinfo is missing device id".to_string()))?;
    let alias = text_any(sysinfo, &["alias"]).unwrap_or_else(|| format!("Kasa {device_id}"));
    let model = text_any(sysinfo, &["model"]).unwrap_or_else(|| "Kasa device".to_string());
    let light_state = sysinfo.get("light_state").and_then(JsonValue::as_object);
    let is_light = light_state.is_some()
        || integer(sysinfo, "is_dimmable").unwrap_or(0) != 0
        || text_any(sysinfo, &["mic_type"])
            .is_some_and(|value| value.to_ascii_lowercase().contains("bulb"));
    let state = light_state.unwrap_or(sysinfo);
    let detail_state = state
        .get("dft_on_state")
        .and_then(JsonValue::as_object)
        .unwrap_or(state);
    let kind = if is_light {
        KasaDeviceKind::Light
    } else {
        KasaDeviceKind::Switch
    };
    let on = integer_any(state, &["on_off", "relay_state"]).unwrap_or(0) != 0;
    let supports_brightness = is_light
        && integer(sysinfo, "is_dimmable")
            .unwrap_or_else(|| i64::from(detail_state.contains_key("brightness")))
            != 0;
    let supports_color = integer(sysinfo, "is_color").unwrap_or_else(|| {
        i64::from(detail_state.contains_key("hue") || detail_state.contains_key("saturation"))
    }) != 0;
    let supports_color_temperature = integer(sysinfo, "is_variable_color_temp")
        .unwrap_or_else(|| i64::from(detail_state.contains_key("color_temp")))
        != 0;
    Ok(KasaStatus {
        device_id,
        alias,
        model,
        mac: text_any(sysinfo, &["mac"]),
        firmware_version: text_any(sysinfo, &["sw_ver"]),
        kind,
        on,
        supports_brightness,
        supports_color,
        supports_color_temperature,
        brightness: supports_brightness
            .then(|| integer(detail_state, "brightness"))
            .flatten()
            .map(|value| clamp_u8(value, 0, 100)),
        hue: supports_color
            .then(|| integer(detail_state, "hue"))
            .flatten()
            .map(|value| clamp_u16(value, 0, 360)),
        saturation: supports_color
            .then(|| integer(detail_state, "saturation"))
            .flatten()
            .map(|value| clamp_u8(value, 0, 100)),
        color_temperature_kelvin: supports_color_temperature
            .then(|| integer(detail_state, "color_temp"))
            .flatten()
            .filter(|value| *value > 0)
            .map(|value| clamp_u16(value, 1_000, 20_000)),
    })
}

fn capabilities(status: &KasaStatus) -> Vec<Capability> {
    let mut values = vec![Capability::light_on_off()];
    if status.supports_brightness {
        values.push(Capability::light_brightness());
    }
    if status.supports_color {
        values.push(Capability::light_color());
    }
    if status.supports_color_temperature {
        values.push(Capability::light_color_temperature());
    }
    values
}

fn status_value(status: &KasaStatus) -> Value {
    let mut values = vec![("on".to_string(), Value::Bool(status.on))];
    if let Some(brightness) = status.brightness {
        values.push(("brightness".to_string(), Value::Percentage(brightness)));
    }
    if let (Some(hue), Some(saturation)) = (status.hue, status.saturation) {
        let [red, green, blue] = hs_to_rgb(hue, saturation);
        values.push((
            "color".to_string(),
            Value::Array(vec![
                Value::Integer(i64::from(red)),
                Value::Integer(i64::from(green)),
                Value::Integer(i64::from(blue)),
            ]),
        ));
    }
    if let Some(kelvin) = status.color_temperature_kelvin {
        values.push((
            "color_temperature".to_string(),
            Value::Integer(i64::from(1_000_000u32 / u32::from(kelvin))),
        ));
    }
    Value::Object(values)
}

pub fn encrypt_legacy_payload(plaintext: &[u8]) -> Vec<u8> {
    let mut key = XOR_SEED;
    plaintext
        .iter()
        .map(|byte| {
            let encrypted = *byte ^ key;
            key = encrypted;
            encrypted
        })
        .collect()
}

pub fn decrypt_legacy_payload(ciphertext: &[u8]) -> Vec<u8> {
    let mut key = XOR_SEED;
    ciphertext
        .iter()
        .map(|byte| {
            let plaintext = *byte ^ key;
            key = *byte;
            plaintext
        })
        .collect()
}

fn encode_json(value: &JsonValue) -> Result<Vec<u8>, KasaError> {
    Ok(encrypt_legacy_payload(&serde_json::to_vec(value)?))
}

fn decode_json(payload: &[u8]) -> Result<JsonValue, KasaError> {
    Ok(serde_json::from_slice(&decrypt_legacy_payload(payload))?)
}

fn text_any(object: &JsonMap<String, JsonValue>, fields: &[&str]) -> Option<String> {
    fields
        .iter()
        .find_map(|field| object.get(*field).and_then(JsonValue::as_str))
        .map(str::to_string)
}

fn integer(object: &JsonMap<String, JsonValue>, field: &str) -> Option<i64> {
    object.get(field).and_then(JsonValue::as_i64)
}

fn integer_any(object: &JsonMap<String, JsonValue>, fields: &[&str]) -> Option<i64> {
    fields.iter().find_map(|field| integer(object, field))
}

fn clamp_u8(value: i64, minimum: i64, maximum: i64) -> u8 {
    u8::try_from(value.clamp(minimum, maximum)).unwrap_or_default()
}

fn clamp_u16(value: i64, minimum: i64, maximum: i64) -> u16 {
    u16::try_from(value.clamp(minimum, maximum)).unwrap_or_default()
}

fn rgb_arguments(value: &Value, command_type: CommandType) -> Result<[u8; 3], KasaError> {
    let Value::Array(values) = value else {
        return invalid_arguments(command_type, "RGB array with three integer channels");
    };
    if values.len() != 3 {
        return invalid_arguments(command_type, "RGB array with three integer channels");
    }
    let mut rgb = [0u8; 3];
    for (index, value) in values.iter().enumerate() {
        let Value::Integer(channel) = value else {
            return invalid_arguments(command_type, "RGB array with three integer channels");
        };
        rgb[index] = u8::try_from(*channel).map_err(|_| KasaError::InvalidCommandArguments {
            command_type,
            expected: "RGB array with three integer channels",
        })?;
    }
    Ok(rgb)
}

fn rgb_to_hs([red, green, blue]: [u8; 3]) -> (u16, u8) {
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
    (hue.round() as u16, (saturation * 100.0).round() as u8)
}

fn hs_to_rgb(hue: u16, saturation: u8) -> [u8; 3] {
    let hue = f64::from(hue % 360);
    let saturation = f64::from(saturation) / 100.0;
    let chroma = saturation;
    let x = chroma * (1.0 - ((hue / 60.0).rem_euclid(2.0) - 1.0).abs());
    let (red, green, blue) = match hue as u16 {
        0..=59 => (chroma, x, 0.0),
        60..=119 => (x, chroma, 0.0),
        120..=179 => (0.0, chroma, x),
        180..=239 => (0.0, x, chroma),
        240..=299 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };
    [
        (red * 255.0).round() as u8,
        (green * 255.0).round() as u8,
        (blue * 255.0).round() as u8,
    ]
}

fn invalid_arguments<T>(command_type: CommandType, expected: &'static str) -> Result<T, KasaError> {
    Err(KasaError::InvalidCommandArguments {
        command_type,
        expected,
    })
}

fn stable_component(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn udp_options(response_port: u16, timeout: Duration, maximum_datagram_bytes: usize) -> UdpOptions {
    UdpOptions {
        bind_addr: Some(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            response_port,
        )),
        max_datagram_size: maximum_datagram_bytes,
        read_timeout: Some(timeout),
        write_timeout: Some(timeout),
    }
}

fn protocol_identifier(kind: &str, value: &str) -> Result<ProtocolIdentifier, KasaError> {
    ProtocolIdentifier::new(ProtocolFamily::Vendor(PROTOCOL_ID.to_string()), kind, value)
        .map_err(|error| KasaError::Validation(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use smart_home_core::{CapabilityGrant, CapabilityGrantId, PrivilegeTier};
    use std::net::UdpSocket;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::thread;

    fn switch_response(on: bool) -> JsonValue {
        json!({
            "system": {"get_sysinfo": {
                "err_code": 0,
                "deviceId": "8006C27F1234567890",
                "alias": "Desk plug",
                "model": "HS103(US)",
                "mac": "AA:BB:CC:DD:EE:FF",
                "sw_ver": "1.0.0",
                "relay_state": i32::from(on)
            }}
        })
    }

    fn bulb_response(on: bool, brightness: u8) -> JsonValue {
        json!({
            "system": {"get_sysinfo": {
                "err_code": 0,
                "deviceId": "8012BULB1234567890",
                "alias": "Desk bulb",
                "model": "KL130(US)",
                "is_dimmable": 1,
                "is_color": 1,
                "is_variable_color_temp": 1,
                "light_state": {
                    "on_off": i32::from(on),
                    "brightness": brightness,
                    "hue": 120,
                    "saturation": 80,
                    "color_temp": 4000
                }
            }}
        })
    }

    fn grant(runtime: &mut SmartHomeRuntime, principal: &AgentId) {
        let _ = runtime.registry_mut().upsert_capability_grant(
            CapabilityGrant::for_all_smart_home(
                CapabilityGrantId::trusted("grant:kasa-lan-test"),
                principal.clone(),
                PrivilegeTier::LowRisk,
                "test",
                1_000,
            )
            .with_expiry(20_000),
        );
    }

    #[test]
    fn legacy_cipher_round_trips_and_status_parser_distinguishes_devices() {
        let plaintext = br#"{"system":{"get_sysinfo":{}}}"#;
        let encrypted = encrypt_legacy_payload(plaintext);
        assert_ne!(encrypted, plaintext);
        assert_eq!(decrypt_legacy_payload(&encrypted), plaintext);

        let switch = parse_status_response(&switch_response(false)).unwrap();
        assert_eq!(switch.kind, KasaDeviceKind::Switch);
        assert!(!switch.on);
        assert_eq!(switch.brightness, None);

        let bulb = parse_status_response(&bulb_response(true, 73)).unwrap();
        assert_eq!(bulb.kind, KasaDeviceKind::Light);
        assert_eq!(bulb.brightness, Some(73));
        assert_eq!(bulb.hue, Some(120));
        assert_eq!(capabilities(&bulb).len(), 4);

        let off_bulb = json!({
            "system": {"get_sysinfo": {
                "err_code": 0,
                "deviceId": "8012BULB1234567890",
                "alias": "Off bulb",
                "model": "KL130(US)",
                "is_dimmable": 1,
                "is_color": 1,
                "is_variable_color_temp": 1,
                "light_state": {
                    "on_off": 0,
                    "dft_on_state": {
                        "brightness": 42,
                        "hue": 240,
                        "saturation": 50,
                        "color_temp": 0
                    }
                }
            }}
        });
        let off_bulb = parse_status_response(&off_bulb).unwrap();
        assert_eq!(off_bulb.brightness, Some(42));
        assert_eq!(off_bulb.hue, Some(240));
        assert_eq!(off_bulb.color_temperature_kelvin, None);
        assert!(off_bulb.supports_color_temperature);
        assert_eq!(capabilities(&off_bulb).len(), 4);
    }

    #[test]
    fn real_udp_scan_produces_verified_broadcast_record() {
        let server = UdpSocket::bind("127.0.0.1:0").unwrap();
        let destination = server.local_addr().unwrap();
        let port = destination.port();
        let handle = thread::spawn(move || {
            let mut buffer = [0u8; 16 * 1024];
            let (size, source) = server.recv_from(&mut buffer).unwrap();
            assert_eq!(decode_json(&buffer[..size]).unwrap(), sysinfo_request());
            server
                .send_to(&encode_json(&switch_response(false)).unwrap(), source)
                .unwrap();
        });
        let config = KasaScanConfig {
            destination,
            response_port: 0,
            timeout: Duration::from_millis(100),
            maximum_datagram_bytes: DEFAULT_MAX_DATAGRAM_BYTES,
            maximum_responses: 4,
            broadcast: false,
        };
        let result = scan_lan(&config, &mut KasaLanTransport).unwrap();
        handle.join().unwrap();
        assert_eq!(result.devices.len(), 1);
        assert!(result.failures.is_empty());
        let record = discovery_record(&result.devices[0], 5_000).unwrap();
        assert_eq!(record.source, DiscoverySource::UdpBroadcast);
        assert_eq!(record.transport, BridgeTransport::LanUdp);
        assert_eq!(record.native_bridge_id, "8006c27f1234567890");
        assert_eq!(record.hardware_model.as_deref(), Some("HS103(US)"));
        assert_eq!(result.devices[0].port, port);
    }

    #[test]
    fn real_udp_install_and_authorized_switch_command_are_verified() {
        let server = UdpSocket::bind("127.0.0.1:0").unwrap();
        let port = server.local_addr().unwrap().port();
        server
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let captured = Arc::new(Mutex::new(Vec::new()));
        let server_captured = Arc::clone(&captured);
        let handle = thread::spawn(move || {
            for response in [
                switch_response(false),
                json!({"system": {"set_relay_state": {"err_code": 0}}}),
                switch_response(true),
            ] {
                let mut buffer = [0u8; 16 * 1024];
                let (size, source) = server.recv_from(&mut buffer).unwrap();
                server_captured
                    .lock()
                    .unwrap()
                    .push(decode_json(&buffer[..size]).unwrap());
                server
                    .send_to(&encode_json(&response).unwrap(), source)
                    .unwrap();
            }
        });
        let config = KasaDeviceConfig::new(BridgeId::trusted("kasa.bridge.test"), "127.0.0.1")
            .unwrap()
            .with_port(port)
            .with_timeout(Duration::from_secs(1));
        let client = KasaClient::new(config, KasaLanTransport);
        let mut integration = KasaRuntimeIntegration::new(client);
        let mut runtime = SmartHomeRuntime::new();
        let installed = integration
            .inspect_and_install(&mut runtime, 5_000)
            .unwrap();
        let principal = AgentId::trusted("agent:kasa-test");
        grant(&mut runtime, &principal);
        let result = integration
            .dispatch_command(
                &mut runtime,
                principal,
                RuntimeCommandToolRequest::new(
                    installed.entity_id.clone(),
                    CommandType::TurnOn,
                    Value::Null,
                ),
                6_000,
            )
            .unwrap();
        handle.join().unwrap();
        assert_eq!(result.status, smart_home_core::CommandStatus::Accepted);
        let requests = captured.lock().unwrap();
        assert_eq!(requests.len(), 3);
        assert_eq!(requests[0], sysinfo_request());
        assert_eq!(
            requests[1],
            json!({"system": {"set_relay_state": {"state": 1}}})
        );
        assert_eq!(requests[2], sysinfo_request());
        assert_eq!(
            runtime
                .registry()
                .state(&installed.entity_id)
                .unwrap()
                .observed_at_ms,
            6_000
        );
    }

    #[test]
    fn bulb_commands_use_native_transition_shape() {
        let status = parse_status_response(&bulb_response(true, 50)).unwrap();
        let request = RuntimeCommandToolRequest::new(
            EntityId::trusted("kasa:bulb:light"),
            CommandType::SetColor,
            Value::Array(vec![
                Value::Integer(0),
                Value::Integer(255),
                Value::Integer(0),
            ]),
        );
        let (payload, operation, expectation) = command_payload(&status, &request).unwrap();
        assert_eq!(
            operation,
            "/smartlife.iot.smartbulb.lightingservice/transition_light_state"
        );
        assert_eq!(
            payload.pointer("/smartlife.iot.smartbulb.lightingservice/transition_light_state/hue"),
            Some(&json!(120))
        );
        assert_eq!(
            payload.pointer(
                "/smartlife.iot.smartbulb.lightingservice/transition_light_state/saturation"
            ),
            Some(&json!(100))
        );
        assert_eq!(
            expectation,
            CommandExpectation::Color {
                hue: 120,
                saturation: 100
            }
        );
    }

    #[derive(Debug)]
    struct CountingTransport {
        calls: Arc<AtomicUsize>,
    }

    impl KasaTransport for CountingTransport {
        fn exchange(
            &mut self,
            _destination: SocketAddr,
            _payload: &[u8],
            _options: &KasaExchangeOptions,
        ) -> Result<Vec<UdpDatagram>, KasaError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(Vec::new())
        }
    }

    #[test]
    fn denied_command_reaches_no_transport() {
        let calls = Arc::new(AtomicUsize::new(0));
        let config = KasaDeviceConfig::new(BridgeId::trusted("kasa.denied"), "127.0.0.1").unwrap();
        let client = KasaClient::new(
            config,
            CountingTransport {
                calls: Arc::clone(&calls),
            },
        );
        let mut integration = KasaRuntimeIntegration::new(client);
        let mut runtime = SmartHomeRuntime::new();
        let status = parse_status_response(&switch_response(false)).unwrap();
        let installed = integration
            .install_status(&mut runtime, &status, 5_000)
            .unwrap();
        let error = integration
            .dispatch_command(
                &mut runtime,
                AgentId::trusted("agent:denied"),
                RuntimeCommandToolRequest::new(
                    installed.entity_id,
                    CommandType::TurnOn,
                    Value::Null,
                ),
                6_000,
            )
            .unwrap_err();
        assert!(matches!(error, KasaError::Runtime(_)));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }
}
