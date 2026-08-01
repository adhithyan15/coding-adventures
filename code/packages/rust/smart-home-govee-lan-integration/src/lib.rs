//! Production Govee LAN UDP integration for D23.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
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
use udp_client::{send_to_and_collect, UdpClient, UdpDatagram, UdpError, UdpOptions};

pub const VERSION: &str = "0.1.0";
pub const INTEGRATION_ID: &str = "govee_light_local";
pub const PROTOCOL_ID: &str = "govee_lan";
pub const MULTICAST_ADDRESS: Ipv4Addr = Ipv4Addr::new(239, 255, 255, 250);
pub const SCAN_PORT: u16 = 4001;
pub const RESPONSE_PORT: u16 = 4002;
pub const DEVICE_PORT: u16 = 4003;
pub const DEFAULT_MAX_DATAGRAM_BYTES: usize = 8 * 1024;
pub const DEFAULT_MAX_SCAN_RESPONSES: usize = 64;

#[derive(Debug)]
pub enum GoveeError {
    Validation(String),
    Udp(UdpError),
    Json(serde_json::Error),
    UnexpectedCommand {
        expected: &'static str,
        actual: String,
    },
    SourceMismatch {
        advertised: IpAddr,
        source: IpAddr,
    },
    NoResponse,
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

impl fmt::Display for GoveeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => write!(formatter, "invalid Govee LAN input: {message}"),
            Self::Udp(error) => error.fmt(formatter),
            Self::Json(error) => write!(formatter, "invalid Govee LAN JSON: {error}"),
            Self::UnexpectedCommand { expected, actual } => write!(
                formatter,
                "expected Govee LAN `{expected}` response, received `{actual}`"
            ),
            Self::SourceMismatch { advertised, source } => write!(
                formatter,
                "Govee LAN response advertised {advertised} but arrived from {source}"
            ),
            Self::NoResponse => formatter.write_str("Govee LAN device did not respond"),
            Self::UnknownEntity(entity_id) => write!(formatter, "unknown Govee entity {entity_id}"),
            Self::UnsupportedCommand {
                entity_id,
                command_type,
            } => write!(
                formatter,
                "Govee entity {entity_id} does not support {command_type:?}"
            ),
            Self::InvalidCommandArguments {
                command_type,
                expected,
            } => write!(
                formatter,
                "invalid {command_type:?} arguments; expected {expected}"
            ),
            Self::VerificationFailed { command_type } => {
                write!(formatter, "Govee device did not confirm {command_type:?}")
            }
            Self::Runtime(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for GoveeError {}

impl From<UdpError> for GoveeError {
    fn from(error: UdpError) -> Self {
        Self::Udp(error)
    }
}

impl From<serde_json::Error> for GoveeError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<RuntimeError> for GoveeError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct GoveeColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoveeAdvertisement {
    pub ip: IpAddr,
    pub device_id: String,
    pub sku: String,
    pub ble_hardware_version: String,
    pub ble_software_version: String,
    pub wifi_hardware_version: String,
    pub wifi_software_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoveeStatus {
    pub on: bool,
    pub brightness: u8,
    pub color: GoveeColor,
    pub color_temperature_kelvin: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoveeScanConfig {
    pub destination: SocketAddr,
    pub response_port: u16,
    pub timeout: Duration,
    pub maximum_datagram_bytes: usize,
    pub maximum_responses: usize,
}

impl Default for GoveeScanConfig {
    fn default() -> Self {
        Self {
            destination: SocketAddr::new(IpAddr::V4(MULTICAST_ADDRESS), SCAN_PORT),
            response_port: RESPONSE_PORT,
            timeout: Duration::from_secs(2),
            maximum_datagram_bytes: DEFAULT_MAX_DATAGRAM_BYTES,
            maximum_responses: DEFAULT_MAX_SCAN_RESPONSES,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoveeScanResult {
    pub devices: Vec<GoveeAdvertisement>,
    pub failures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoveeDeviceConfig {
    pub bridge_id: BridgeId,
    pub host: IpAddr,
    pub device_id: String,
    pub sku: String,
    pub control_port: u16,
    pub response_port: u16,
    pub timeout: Duration,
    pub maximum_datagram_bytes: usize,
}

impl GoveeDeviceConfig {
    pub fn new(
        bridge_id: BridgeId,
        host: impl AsRef<str>,
        device_id: impl Into<String>,
        sku: impl Into<String>,
    ) -> Result<Self, GoveeError> {
        let host = host
            .as_ref()
            .parse::<IpAddr>()
            .map_err(|_| GoveeError::Validation("host must be a fixed IP address".to_string()))?;
        let device_id = device_id.into();
        let sku = sku.into();
        if stable_component(&device_id).is_empty() || sku.trim().is_empty() {
            return Err(GoveeError::Validation(
                "device_id and sku must be non-empty".to_string(),
            ));
        }
        Ok(Self {
            bridge_id,
            host,
            device_id,
            sku,
            control_port: DEVICE_PORT,
            response_port: RESPONSE_PORT,
            timeout: Duration::from_secs(2),
            maximum_datagram_bytes: DEFAULT_MAX_DATAGRAM_BYTES,
        })
    }

    pub fn with_control_port(mut self, control_port: u16) -> Self {
        self.control_port = control_port;
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
        SocketAddr::new(self.host, self.control_port)
    }
}

pub trait GoveeTransport {
    fn exchange(
        &mut self,
        destination: SocketAddr,
        response_port: u16,
        payload: &[u8],
        timeout: Duration,
        maximum_datagram_bytes: usize,
        maximum_responses: usize,
    ) -> Result<Vec<UdpDatagram>, GoveeError>;

    fn send(
        &mut self,
        destination: SocketAddr,
        response_port: u16,
        payload: &[u8],
        timeout: Duration,
        maximum_datagram_bytes: usize,
    ) -> Result<(), GoveeError>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct GoveeLanTransport;

impl GoveeTransport for GoveeLanTransport {
    fn exchange(
        &mut self,
        destination: SocketAddr,
        response_port: u16,
        payload: &[u8],
        timeout: Duration,
        maximum_datagram_bytes: usize,
        maximum_responses: usize,
    ) -> Result<Vec<UdpDatagram>, GoveeError> {
        let options = udp_options(response_port, timeout, maximum_datagram_bytes);
        Ok(send_to_and_collect(
            destination,
            payload,
            options,
            maximum_responses,
        )?)
    }

    fn send(
        &mut self,
        destination: SocketAddr,
        response_port: u16,
        payload: &[u8],
        timeout: Duration,
        maximum_datagram_bytes: usize,
    ) -> Result<(), GoveeError> {
        let client = UdpClient::bind(udp_options(response_port, timeout, maximum_datagram_bytes))?;
        client.send_to(payload, destination)?;
        Ok(())
    }
}

pub fn scan_lan<T: GoveeTransport>(
    config: &GoveeScanConfig,
    transport: &mut T,
) -> Result<GoveeScanResult, GoveeError> {
    let request = serde_json::to_vec(&json!({
        "msg": {"cmd": "scan", "data": {"account_topic": "reserve"}}
    }))?;
    let datagrams = transport.exchange(
        config.destination,
        config.response_port,
        &request,
        config.timeout,
        config.maximum_datagram_bytes,
        config.maximum_responses,
    )?;
    let mut devices = Vec::new();
    let mut failures = Vec::new();
    for datagram in datagrams {
        match parse_scan_response(&datagram) {
            Ok(device) => devices.push(device),
            Err(error) => failures.push(error.to_string()),
        }
    }
    devices.sort_by(|left, right| left.device_id.cmp(&right.device_id));
    devices.dedup_by(|left, right| left.device_id == right.device_id);
    Ok(GoveeScanResult { devices, failures })
}

pub fn discovery_record(
    advertisement: &GoveeAdvertisement,
    discovered_at_ms: u64,
) -> Result<DiscoveryRecord, GoveeError> {
    let native_id = stable_component(&advertisement.device_id);
    if native_id.is_empty() {
        return Err(GoveeError::Validation(
            "device id does not contain a stable component".to_string(),
        ));
    }
    Ok(DiscoveryRecord::new(
        IntegrationId::trusted(INTEGRATION_ID),
        ProtocolFamily::Vendor(PROTOCOL_ID.to_string()),
        native_id,
        DiscoverySource::UdpMulticast,
        BridgeTransport::LanUdp,
        discovered_at_ms,
    )
    .map_err(|error| GoveeError::Validation(error.to_string()))?
    .with_display_name(format!("Govee {}", advertisement.sku))
    .with_address(format!("udp://{}:{}", advertisement.ip, DEVICE_PORT))
    .with_hardware_model(&advertisement.sku)
    .with_confidence(DiscoveryConfidence::Verified)
    .with_pairing_requirement(PairingRequirement::None)
    .with_metadata("govee.device_id", &advertisement.device_id)
    .with_metadata("govee.wifi_version", &advertisement.wifi_software_version))
}

#[derive(Debug)]
pub struct GoveeClient<T> {
    config: GoveeDeviceConfig,
    transport: T,
}

impl<T: GoveeTransport> GoveeClient<T> {
    pub fn new(config: GoveeDeviceConfig, transport: T) -> Self {
        Self { config, transport }
    }

    pub fn config(&self) -> &GoveeDeviceConfig {
        &self.config
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    pub fn transport_mut(&mut self) -> &mut T {
        &mut self.transport
    }

    pub fn inspect(&mut self) -> Result<GoveeStatus, GoveeError> {
        let request = serde_json::to_vec(&json!({"msg": {"cmd": "devStatus", "data": {}}}))?;
        let datagrams = self.transport.exchange(
            self.config.endpoint(),
            self.config.response_port,
            &request,
            self.config.timeout,
            self.config.maximum_datagram_bytes,
            1,
        )?;
        let datagram = datagrams.first().ok_or(GoveeError::NoResponse)?;
        if datagram.source.ip() != self.config.host {
            return Err(GoveeError::SourceMismatch {
                advertised: self.config.host,
                source: datagram.source.ip(),
            });
        }
        parse_status_response(&datagram.payload)
    }

    fn send_command(&mut self, payload: &JsonValue) -> Result<(), GoveeError> {
        let bytes = serde_json::to_vec(payload)?;
        self.transport.send(
            self.config.endpoint(),
            self.config.response_port,
            &bytes,
            self.config.timeout,
            self.config.maximum_datagram_bytes,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledGoveeDevice {
    pub bridge_id: BridgeId,
    pub device_id: DeviceId,
    pub entity_id: EntityId,
}

#[derive(Debug)]
pub struct GoveeRuntimeIntegration<T> {
    client: GoveeClient<T>,
    entity_id: Option<EntityId>,
}

impl<T: GoveeTransport> GoveeRuntimeIntegration<T> {
    pub fn new(client: GoveeClient<T>) -> Self {
        Self {
            client,
            entity_id: None,
        }
    }

    pub fn client(&self) -> &GoveeClient<T> {
        &self.client
    }

    pub fn client_mut(&mut self) -> &mut GoveeClient<T> {
        &mut self.client
    }

    pub fn inspect_and_install(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        observed_at_ms: u64,
    ) -> Result<InstalledGoveeDevice, GoveeError> {
        let status = self.client.inspect()?;
        self.install_status(runtime, &status, observed_at_ms)
    }

    pub fn install_status(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        status: &GoveeStatus,
        observed_at_ms: u64,
    ) -> Result<InstalledGoveeDevice, GoveeError> {
        let config = &self.client.config;
        let native_id = stable_component(&config.device_id);
        let device_id = DeviceId::trusted(format!("govee-lan:{native_id}"));
        let entity_id = EntityId::trusted(format!("govee-lan:{native_id}:light"));
        let address = format!("udp://{}:{}", config.host, config.control_port);
        let mut bridge = Bridge::new(
            config.bridge_id.clone(),
            IntegrationId::trusted(INTEGRATION_ID),
            BridgeTransport::LanUdp,
        );
        bridge.address = Some(address.clone());
        bridge.hardware_model = Some(config.sku.clone());
        bridge.health = Health::Online;
        bridge.last_seen_at_ms = Some(observed_at_ms);
        bridge.identifiers = vec![protocol_identifier("udp_endpoint", &address)?];
        bridge.metadata = vec![Metadata::new("govee.transport", "lan_udp_polling")];
        runtime.upsert_bridge(bridge)?;

        runtime.upsert_device(Device {
            device_id: device_id.clone(),
            bridge_id: config.bridge_id.clone(),
            manufacturer: "Govee".to_string(),
            model: config.sku.clone(),
            name: format!("Govee {}", config.sku),
            serial: Some(config.device_id.clone()),
            firmware_version: None,
            room_id: None,
            entity_ids: vec![entity_id.clone()],
            identifiers: vec![protocol_identifier("device_id", &config.device_id)?],
            health: Health::Online,
            metadata: vec![Metadata::new("govee.lan_control", "enabled")],
        })?;
        runtime.upsert_entity(Entity {
            entity_id: entity_id.clone(),
            device_id: device_id.clone(),
            kind: EntityKind::Light,
            name: format!("Govee {} light", config.sku),
            capabilities: vec![
                Capability::light_on_off(),
                Capability::light_brightness(),
                Capability::light_color(),
                Capability::light_color_temperature(),
            ],
            state: Some(StateSnapshot {
                entity_id: entity_id.clone(),
                value: status_value(status),
                source: StateSource::Poll,
                observed_at_ms,
                received_at_ms: observed_at_ms,
                expires_at_ms: None,
                confidence: StateConfidence::Confirmed,
            }),
            metadata: vec![Metadata::new("govee.protocol", PROTOCOL_ID)],
        })?;
        self.entity_id = Some(entity_id.clone());
        Ok(InstalledGoveeDevice {
            bridge_id: config.bridge_id.clone(),
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
    ) -> Result<CommandResult, GoveeError> {
        if self.entity_id.as_ref() != Some(&request.entity_id) {
            return Err(GoveeError::UnknownEntity(request.entity_id.clone()));
        }
        let (payload, expectation) = command_payload(&request)?;
        let command_type = request.command_type;
        let result = runtime.execute_command_tool(principal_id, request, now_ms)?;
        self.client.send_command(&payload)?;
        let confirmed = self.client.inspect()?;
        if !expectation.matches(&confirmed) {
            return Err(GoveeError::VerificationFailed { command_type });
        }
        self.install_status(runtime, &confirmed, now_ms)?;
        Ok(result)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CommandExpectation {
    Power(bool),
    Brightness(u8),
    Color(GoveeColor),
    ColorTemperature(u16),
}

impl CommandExpectation {
    fn matches(&self, status: &GoveeStatus) -> bool {
        match self {
            Self::Power(expected) => status.on == *expected,
            Self::Brightness(expected) => status.brightness == *expected,
            Self::Color(expected) => status.color == *expected,
            Self::ColorTemperature(expected) => status.color_temperature_kelvin == *expected,
        }
    }
}

fn command_payload(
    request: &RuntimeCommandToolRequest,
) -> Result<(JsonValue, CommandExpectation), GoveeError> {
    match request.command_type {
        CommandType::TurnOn => Ok((
            envelope("turn", json!({"value": 1})),
            CommandExpectation::Power(true),
        )),
        CommandType::TurnOff => Ok((
            envelope("turn", json!({"value": 0})),
            CommandExpectation::Power(false),
        )),
        CommandType::SetBrightness => {
            let Value::Percentage(brightness) = request.arguments else {
                return invalid_arguments(request.command_type, "percentage");
            };
            Ok((
                envelope("brightness", json!({"value": brightness})),
                CommandExpectation::Brightness(brightness),
            ))
        }
        CommandType::SetColor => {
            let color = rgb_color(&request.arguments, request.command_type)?;
            Ok((
                envelope("colorwc", json!({"color": color, "colorTemInKelvin": 0})),
                CommandExpectation::Color(color),
            ))
        }
        CommandType::SetColorTemperature => {
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
            let kelvin = u16::try_from((1_000_000 / mirek).clamp(2_000, 9_000)).map_err(|_| {
                GoveeError::InvalidCommandArguments {
                    command_type: request.command_type,
                    expected: "integer mirek from 111 through 500",
                }
            })?;
            Ok((
                envelope(
                    "colorwc",
                    json!({
                        "color": {"r": 0, "g": 0, "b": 0},
                        "colorTemInKelvin": kelvin,
                    }),
                ),
                CommandExpectation::ColorTemperature(kelvin),
            ))
        }
        _ => Err(GoveeError::UnsupportedCommand {
            entity_id: request.entity_id.clone(),
            command_type: request.command_type,
        }),
    }
}

fn envelope(command: &str, data: JsonValue) -> JsonValue {
    json!({"msg": {"cmd": command, "data": data}})
}

fn rgb_color(arguments: &Value, command_type: CommandType) -> Result<GoveeColor, GoveeError> {
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
        .ok_or(GoveeError::InvalidCommandArguments {
            command_type,
            expected: "RGB array with three integer channels from 0 through 255",
        })?;
    Ok(GoveeColor {
        r: channels[0],
        g: channels[1],
        b: channels[2],
    })
}

fn invalid_arguments<T>(
    command_type: CommandType,
    expected: &'static str,
) -> Result<T, GoveeError> {
    Err(GoveeError::InvalidCommandArguments {
        command_type,
        expected,
    })
}

#[derive(Debug, Deserialize)]
struct IncomingEnvelope<T> {
    msg: IncomingMessage<T>,
}

#[derive(Debug, Deserialize)]
struct IncomingMessage<T> {
    cmd: String,
    data: T,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ScanData {
    ip: String,
    device: String,
    sku: String,
    #[serde(default)]
    ble_version_hard: String,
    #[serde(default)]
    ble_version_soft: String,
    #[serde(default)]
    wifi_version_hard: String,
    #[serde(default)]
    wifi_version_soft: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StatusData {
    on_off: u8,
    brightness: u8,
    color: GoveeColor,
    color_tem_in_kelvin: u16,
}

fn parse_scan_response(datagram: &UdpDatagram) -> Result<GoveeAdvertisement, GoveeError> {
    let envelope: IncomingEnvelope<ScanData> = serde_json::from_slice(&datagram.payload)?;
    if envelope.msg.cmd != "scan" {
        return Err(GoveeError::UnexpectedCommand {
            expected: "scan",
            actual: envelope.msg.cmd,
        });
    }
    let data = envelope.msg.data;
    let ip = data
        .ip
        .parse::<IpAddr>()
        .map_err(|_| GoveeError::Validation("scan response ip is invalid".to_string()))?;
    if ip != datagram.source.ip() {
        return Err(GoveeError::SourceMismatch {
            advertised: ip,
            source: datagram.source.ip(),
        });
    }
    if stable_component(&data.device).is_empty() || data.sku.trim().is_empty() {
        return Err(GoveeError::Validation(
            "scan response device and sku must be non-empty".to_string(),
        ));
    }
    Ok(GoveeAdvertisement {
        ip,
        device_id: data.device,
        sku: data.sku,
        ble_hardware_version: data.ble_version_hard,
        ble_software_version: data.ble_version_soft,
        wifi_hardware_version: data.wifi_version_hard,
        wifi_software_version: data.wifi_version_soft,
    })
}

fn parse_status_response(payload: &[u8]) -> Result<GoveeStatus, GoveeError> {
    let envelope: IncomingEnvelope<StatusData> = serde_json::from_slice(payload)?;
    if envelope.msg.cmd != "devStatus" {
        return Err(GoveeError::UnexpectedCommand {
            expected: "devStatus",
            actual: envelope.msg.cmd,
        });
    }
    let data = envelope.msg.data;
    if data.on_off > 1 || data.brightness > 100 {
        return Err(GoveeError::Validation(
            "status power or brightness is outside the documented range".to_string(),
        ));
    }
    if data.color_tem_in_kelvin != 0 && !(2_000..=9_000).contains(&data.color_tem_in_kelvin) {
        return Err(GoveeError::Validation(
            "status color temperature is outside the documented range".to_string(),
        ));
    }
    Ok(GoveeStatus {
        on: data.on_off == 1,
        brightness: data.brightness,
        color: data.color,
        color_temperature_kelvin: data.color_tem_in_kelvin,
    })
}

fn status_value(status: &GoveeStatus) -> Value {
    let mut fields = vec![
        ("on".to_string(), Value::Bool(status.on)),
        (
            "brightness".to_string(),
            Value::Percentage(status.brightness),
        ),
        (
            "color".to_string(),
            Value::Array(vec![
                Value::Integer(i64::from(status.color.r)),
                Value::Integer(i64::from(status.color.g)),
                Value::Integer(i64::from(status.color.b)),
            ]),
        ),
    ];
    if status.color_temperature_kelvin > 0 {
        let mirek = 1_000_000u32 / u32::from(status.color_temperature_kelvin);
        fields.push((
            "color_temperature".to_string(),
            Value::Integer(i64::from(mirek)),
        ));
    }
    Value::Object(fields)
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

fn protocol_identifier(kind: &str, value: &str) -> Result<ProtocolIdentifier, GoveeError> {
    ProtocolIdentifier::new(ProtocolFamily::Vendor(PROTOCOL_ID.to_string()), kind, value)
        .map_err(|error| GoveeError::Validation(error.to_string()))
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

#[cfg(test)]
mod tests {
    use super::*;
    use smart_home_core::{CapabilityGrant, CapabilityGrantId, PrivilegeTier};
    use std::net::UdpSocket;
    use std::sync::{Arc, Mutex};
    use std::thread;

    fn status_payload(on: bool, brightness: u8, color: [u8; 3], kelvin: u16) -> Vec<u8> {
        serde_json::to_vec(&json!({
            "msg": {
                "cmd": "devStatus",
                "data": {
                    "onOff": if on { 1 } else { 0 },
                    "brightness": brightness,
                    "color": {"r": color[0], "g": color[1], "b": color[2]},
                    "colorTemInKelvin": kelvin,
                }
            }
        }))
        .unwrap()
    }

    fn grant(runtime: &mut SmartHomeRuntime, principal: &AgentId) {
        let _ = runtime.registry_mut().upsert_capability_grant(
            CapabilityGrant::for_all_smart_home(
                CapabilityGrantId::trusted("grant:govee-lan-test"),
                principal.clone(),
                PrivilegeTier::LowRisk,
                "test",
                1_000,
            )
            .with_expiry(20_000),
        );
    }

    #[test]
    fn real_udp_scan_produces_verified_discovery_record() {
        let server = UdpSocket::bind("127.0.0.1:0").unwrap();
        let destination = server.local_addr().unwrap();
        let handle = thread::spawn(move || {
            let mut buffer = [0u8; 2048];
            let (size, source) = server.recv_from(&mut buffer).unwrap();
            let request: JsonValue = serde_json::from_slice(&buffer[..size]).unwrap();
            assert_eq!(request["msg"]["cmd"], "scan");
            assert_eq!(request["msg"]["data"]["account_topic"], "reserve");
            let response = serde_json::to_vec(&json!({
                "msg": {
                    "cmd": "scan",
                    "data": {
                        "ip": "127.0.0.1",
                        "device": "1F:80:C5:32:32:36:72:4E",
                        "sku": "H605C",
                        "bleVersionHard": "3.01.01",
                        "bleVersionSoft": "1.03.01",
                        "wifiVersionHard": "1.00.10",
                        "wifiVersionSoft": "1.02.03"
                    }
                }
            }))
            .unwrap();
            server.send_to(&response, source).unwrap();
        });
        let config = GoveeScanConfig {
            destination,
            response_port: 0,
            timeout: Duration::from_millis(500),
            maximum_datagram_bytes: 2048,
            maximum_responses: 4,
        };
        let result = scan_lan(&config, &mut GoveeLanTransport).unwrap();
        handle.join().unwrap();
        assert_eq!(result.devices.len(), 1);
        assert!(result.failures.is_empty());
        let record = discovery_record(&result.devices[0], 5_000).unwrap();
        assert_eq!(record.source, DiscoverySource::UdpMulticast);
        assert_eq!(record.transport, BridgeTransport::LanUdp);
        assert_eq!(record.hardware_model.as_deref(), Some("H605C"));
        assert_eq!(record.pairing_requirement, PairingRequirement::None);
    }

    #[test]
    fn real_udp_install_and_authorized_command_are_verified() {
        let server = UdpSocket::bind("127.0.0.1:0").unwrap();
        let port = server.local_addr().unwrap().port();
        server
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let captured = Arc::new(Mutex::new(Vec::new()));
        let server_captured = Arc::clone(&captured);
        let handle = thread::spawn(move || {
            let responses = [
                Some(status_payload(false, 40, [1, 2, 3], 4_000)),
                None,
                Some(status_payload(true, 40, [1, 2, 3], 4_000)),
            ];
            for response in responses {
                let mut buffer = [0u8; 2048];
                let (size, source) = server.recv_from(&mut buffer).unwrap();
                let request = String::from_utf8(buffer[..size].to_vec()).unwrap();
                server_captured.lock().unwrap().push(request);
                if let Some(response) = response {
                    server.send_to(&response, source).unwrap();
                }
            }
        });
        let config = GoveeDeviceConfig::new(
            BridgeId::trusted("govee.bridge.test"),
            "127.0.0.1",
            "1F:80:C5:32:32:36:72:4E",
            "H605C",
        )
        .unwrap()
        .with_control_port(port)
        .with_response_port(0);
        let client = GoveeClient::new(config, GoveeLanTransport);
        let mut integration = GoveeRuntimeIntegration::new(client);
        let mut runtime = SmartHomeRuntime::new();
        let installed = integration
            .inspect_and_install(&mut runtime, 5_000)
            .unwrap();
        let principal = AgentId::trusted("agent:govee-test");
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
        assert_eq!(runtime.registry().counts().states, 1);
        let state = runtime.registry().state(&installed.entity_id).unwrap();
        assert_eq!(state.observed_at_ms, 6_000);
        assert_eq!(
            state.value,
            status_value(&GoveeStatus {
                on: true,
                brightness: 40,
                color: GoveeColor { r: 1, g: 2, b: 3 },
                color_temperature_kelvin: 4_000,
            })
        );
        let captured = captured.lock().unwrap();
        assert!(captured[0].contains(r#""cmd":"devStatus""#));
        assert!(captured[1].contains(r#""cmd":"turn""#));
        assert!(captured[1].contains(r#""value":1"#));
        assert!(captured[2].contains(r#""cmd":"devStatus""#));
    }

    #[test]
    fn brightness_rgb_and_temperature_match_official_envelopes() {
        let entity = EntityId::trusted("govee-lan:test:light");
        let brightness = RuntimeCommandToolRequest::new(
            entity.clone(),
            CommandType::SetBrightness,
            Value::Percentage(37),
        );
        assert_eq!(
            command_payload(&brightness).unwrap().0,
            envelope("brightness", json!({"value": 37}))
        );

        let rgb = RuntimeCommandToolRequest::new(
            entity.clone(),
            CommandType::SetColor,
            Value::Array(vec![
                Value::Integer(12),
                Value::Integer(34),
                Value::Integer(56),
            ]),
        );
        assert_eq!(
            command_payload(&rgb).unwrap().0,
            envelope(
                "colorwc",
                json!({
                    "color": {"r": 12, "g": 34, "b": 56},
                    "colorTemInKelvin": 0,
                })
            )
        );

        let temperature = RuntimeCommandToolRequest::new(
            entity,
            CommandType::SetColorTemperature,
            Value::Integer(250),
        );
        assert_eq!(
            command_payload(&temperature).unwrap().0,
            envelope(
                "colorwc",
                json!({
                    "color": {"r": 0, "g": 0, "b": 0},
                    "colorTemInKelvin": 4_000,
                })
            )
        );

        let upper_temperature = RuntimeCommandToolRequest::new(
            EntityId::trusted("govee-lan:test:light"),
            CommandType::SetColorTemperature,
            Value::Integer(111),
        );
        assert_eq!(
            command_payload(&upper_temperature).unwrap().0,
            envelope(
                "colorwc",
                json!({
                    "color": {"r": 0, "g": 0, "b": 0},
                    "colorTemInKelvin": 9_000,
                })
            )
        );
    }

    #[test]
    fn malformed_and_unverified_responses_fail_closed() {
        assert!(matches!(
            parse_status_response(&status_payload(true, 101, [1, 2, 3], 4_000)),
            Err(GoveeError::Validation(_))
        ));
        let datagram = UdpDatagram {
            source: "127.0.0.1:4003".parse().unwrap(),
            destination: "127.0.0.1:4002".parse().unwrap(),
            payload: serde_json::to_vec(&json!({
                "msg": {
                    "cmd": "scan",
                    "data": {"ip": "192.0.2.99", "device": "id", "sku": "H605C"}
                }
            }))
            .unwrap(),
        };
        assert!(matches!(
            parse_scan_response(&datagram),
            Err(GoveeError::SourceMismatch { .. })
        ));
    }
}
