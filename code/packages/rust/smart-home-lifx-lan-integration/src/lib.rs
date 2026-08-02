//! Production LIFX LAN UDP integration for D23.

#![forbid(unsafe_code)]

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
pub const INTEGRATION_ID: &str = "lifx";
pub const PROTOCOL_ID: &str = "lifx_lan";
pub const LIFX_PORT: u16 = 56_700;
pub const DEFAULT_MAX_DATAGRAM_BYTES: usize = 512;
pub const DEFAULT_MAX_SCAN_RESPONSES: usize = 128;
pub const HEADER_BYTES: usize = 36;
const PROTOCOL: u16 = 1_024;
const SOURCE_ID: u32 = 0x434f_4445;
const SERVICE_UDP: u8 = 1;
const GET_SERVICE: u16 = 2;
const STATE_SERVICE: u16 = 3;
const GET_COLOR: u16 = 101;
const SET_COLOR: u16 = 102;
const LIGHT_STATE: u16 = 107;
const SET_LIGHT_POWER: u16 = 117;

#[derive(Debug)]
pub enum LifxError {
    Validation(String),
    Udp(UdpError),
    NoResponse,
    UnexpectedPacket {
        expected: u16,
        actual: u16,
    },
    SourceMismatch {
        expected: IpAddr,
        actual: IpAddr,
    },
    CorrelationMismatch,
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

impl fmt::Display for LifxError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => write!(formatter, "invalid LIFX LAN input: {message}"),
            Self::Udp(error) => error.fmt(formatter),
            Self::NoResponse => formatter.write_str("LIFX LAN device did not respond"),
            Self::UnexpectedPacket { expected, actual } => {
                write!(
                    formatter,
                    "expected LIFX packet {expected}, received {actual}"
                )
            }
            Self::SourceMismatch { expected, actual } => {
                write!(
                    formatter,
                    "LIFX reply expected from {expected}, arrived from {actual}"
                )
            }
            Self::CorrelationMismatch => {
                formatter.write_str("LIFX reply did not match source, sequence, or target")
            }
            Self::UnknownEntity(entity_id) => write!(formatter, "unknown LIFX entity {entity_id}"),
            Self::UnsupportedCommand {
                entity_id,
                command_type,
            } => {
                write!(
                    formatter,
                    "LIFX entity {entity_id} does not support {command_type:?}"
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
                write!(formatter, "LIFX device did not confirm {command_type:?}")
            }
            Self::Runtime(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for LifxError {}

impl From<UdpError> for LifxError {
    fn from(error: UdpError) -> Self {
        Self::Udp(error)
    }
}

impl From<RuntimeError> for LifxError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LifxHsbk {
    pub hue: u16,
    pub saturation: u16,
    pub brightness: u16,
    pub kelvin: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifxStatus {
    pub on: bool,
    pub color: LifxHsbk,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifxAdvertisement {
    pub host: IpAddr,
    pub port: u16,
    pub serial: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifxScanConfig {
    pub destination: SocketAddr,
    pub response_port: u16,
    pub timeout: Duration,
    pub maximum_datagram_bytes: usize,
    pub maximum_responses: usize,
}

impl Default for LifxScanConfig {
    fn default() -> Self {
        Self {
            destination: SocketAddr::new(IpAddr::V4(Ipv4Addr::BROADCAST), LIFX_PORT),
            response_port: 0,
            timeout: Duration::from_secs(2),
            maximum_datagram_bytes: DEFAULT_MAX_DATAGRAM_BYTES,
            maximum_responses: DEFAULT_MAX_SCAN_RESPONSES,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifxScanResult {
    pub devices: Vec<LifxAdvertisement>,
    pub failures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifxDeviceConfig {
    pub bridge_id: BridgeId,
    pub host: IpAddr,
    pub port: u16,
    pub serial: [u8; 6],
    pub timeout: Duration,
    pub maximum_datagram_bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifxExchangeOptions {
    pub response_port: u16,
    pub timeout: Duration,
    pub maximum_datagram_bytes: usize,
    pub maximum_responses: usize,
    pub broadcast: bool,
}

impl LifxDeviceConfig {
    pub fn new(
        bridge_id: BridgeId,
        host: impl AsRef<str>,
        serial: impl AsRef<str>,
    ) -> Result<Self, LifxError> {
        let host = host
            .as_ref()
            .parse::<IpAddr>()
            .map_err(|_| LifxError::Validation("host must be a fixed IP address".to_string()))?;
        Ok(Self {
            bridge_id,
            host,
            port: LIFX_PORT,
            serial: parse_serial(serial.as_ref())?,
            timeout: Duration::from_secs(2),
            maximum_datagram_bytes: DEFAULT_MAX_DATAGRAM_BYTES,
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

    pub fn endpoint(&self) -> SocketAddr {
        SocketAddr::new(self.host, self.port)
    }

    pub fn serial_string(&self) -> String {
        serial_string(self.serial)
    }
}

pub trait LifxTransport {
    fn exchange(
        &mut self,
        destination: SocketAddr,
        payload: &[u8],
        options: &LifxExchangeOptions,
    ) -> Result<Vec<UdpDatagram>, LifxError>;

    fn send(
        &mut self,
        destination: SocketAddr,
        payload: &[u8],
        timeout: Duration,
        maximum_datagram_bytes: usize,
    ) -> Result<(), LifxError>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct LifxLanTransport;

impl LifxTransport for LifxLanTransport {
    fn exchange(
        &mut self,
        destination: SocketAddr,
        payload: &[u8],
        options: &LifxExchangeOptions,
    ) -> Result<Vec<UdpDatagram>, LifxError> {
        if options.maximum_responses == 0 {
            return Err(LifxError::Validation(
                "maximum_responses must be greater than zero".to_string(),
            ));
        }
        let client = UdpClient::bind(udp_options(
            options.response_port,
            options.timeout,
            options.maximum_datagram_bytes,
        ))?;
        if options.broadcast {
            client.set_broadcast(true)?;
        }
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

    fn send(
        &mut self,
        destination: SocketAddr,
        payload: &[u8],
        timeout: Duration,
        maximum_datagram_bytes: usize,
    ) -> Result<(), LifxError> {
        let client = UdpClient::bind(udp_options(0, timeout, maximum_datagram_bytes))?;
        client.send_to(payload, destination)?;
        Ok(())
    }
}

pub fn scan_lan<T: LifxTransport>(
    config: &LifxScanConfig,
    transport: &mut T,
) -> Result<LifxScanResult, LifxError> {
    let request = encode_packet(None, 0, GET_SERVICE, &[], false);
    let datagrams = transport.exchange(
        config.destination,
        &request,
        &LifxExchangeOptions {
            response_port: config.response_port,
            timeout: config.timeout,
            maximum_datagram_bytes: config.maximum_datagram_bytes,
            maximum_responses: config.maximum_responses,
            broadcast: true,
        },
    )?;
    let mut devices = Vec::new();
    let mut failures = Vec::new();
    for datagram in datagrams {
        match parse_service_response(&datagram) {
            Ok(device) => devices.push(device),
            Err(error) => failures.push(error.to_string()),
        }
    }
    devices.sort_by(|left, right| left.serial.cmp(&right.serial));
    devices.dedup_by(|left, right| left.serial == right.serial);
    Ok(LifxScanResult { devices, failures })
}

pub fn discovery_record(
    advertisement: &LifxAdvertisement,
    discovered_at_ms: u64,
) -> Result<DiscoveryRecord, LifxError> {
    Ok(DiscoveryRecord::new(
        IntegrationId::trusted(INTEGRATION_ID),
        ProtocolFamily::Vendor(PROTOCOL_ID.to_string()),
        advertisement.serial.clone(),
        DiscoverySource::UdpBroadcast,
        BridgeTransport::LanUdp,
        discovered_at_ms,
    )
    .map_err(|error| LifxError::Validation(error.to_string()))?
    .with_display_name(format!("LIFX {}", advertisement.serial))
    .with_address(format!(
        "udp://{}:{}",
        advertisement.host, advertisement.port
    ))
    .with_confidence(DiscoveryConfidence::Verified)
    .with_pairing_requirement(PairingRequirement::None)
    .with_metadata("lifx.serial", &advertisement.serial))
}

#[derive(Debug)]
pub struct LifxClient<T> {
    config: LifxDeviceConfig,
    transport: T,
    sequence: u8,
}

impl<T: LifxTransport> LifxClient<T> {
    pub fn new(config: LifxDeviceConfig, transport: T) -> Self {
        Self {
            config,
            transport,
            sequence: 0,
        }
    }

    pub fn config(&self) -> &LifxDeviceConfig {
        &self.config
    }

    pub fn inspect(&mut self) -> Result<LifxStatus, LifxError> {
        let sequence = self.next_sequence();
        let request = encode_packet(Some(self.config.serial), sequence, GET_COLOR, &[], false);
        let datagrams = self.transport.exchange(
            self.config.endpoint(),
            &request,
            &LifxExchangeOptions {
                response_port: 0,
                timeout: self.config.timeout,
                maximum_datagram_bytes: self.config.maximum_datagram_bytes,
                maximum_responses: 1,
                broadcast: false,
            },
        )?;
        let datagram = datagrams.first().ok_or(LifxError::NoResponse)?;
        if datagram.source.ip() != self.config.host {
            return Err(LifxError::SourceMismatch {
                expected: self.config.host,
                actual: datagram.source.ip(),
            });
        }
        parse_light_state(&datagram.payload, SOURCE_ID, sequence, self.config.serial)
    }

    fn send_command(&mut self, packet_type: u16, payload: &[u8]) -> Result<(), LifxError> {
        let sequence = self.next_sequence();
        let packet = encode_packet(
            Some(self.config.serial),
            sequence,
            packet_type,
            payload,
            false,
        );
        self.transport.send(
            self.config.endpoint(),
            &packet,
            self.config.timeout,
            self.config.maximum_datagram_bytes,
        )
    }

    fn next_sequence(&mut self) -> u8 {
        let current = self.sequence;
        self.sequence = self.sequence.wrapping_add(1);
        current
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledLifxDevice {
    pub bridge_id: BridgeId,
    pub device_id: DeviceId,
    pub entity_id: EntityId,
}

#[derive(Debug)]
pub struct LifxRuntimeIntegration<T> {
    client: LifxClient<T>,
    entity_id: Option<EntityId>,
}

impl<T: LifxTransport> LifxRuntimeIntegration<T> {
    pub fn new(client: LifxClient<T>) -> Self {
        Self {
            client,
            entity_id: None,
        }
    }

    pub fn client(&self) -> &LifxClient<T> {
        &self.client
    }

    pub fn inspect_and_install(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        observed_at_ms: u64,
    ) -> Result<InstalledLifxDevice, LifxError> {
        let status = self.client.inspect()?;
        self.install_status(runtime, &status, observed_at_ms)
    }

    pub fn install_status(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        status: &LifxStatus,
        observed_at_ms: u64,
    ) -> Result<InstalledLifxDevice, LifxError> {
        let config = &self.client.config;
        let serial = config.serial_string();
        let device_id = DeviceId::trusted(format!("lifx:{serial}"));
        let entity_id = EntityId::trusted(format!("lifx:{serial}:light"));
        let address = format!("udp://{}:{}", config.host, config.port);
        let mut bridge = Bridge::new(
            config.bridge_id.clone(),
            IntegrationId::trusted(INTEGRATION_ID),
            BridgeTransport::LanUdp,
        );
        bridge.address = Some(address.clone());
        bridge.health = Health::Online;
        bridge.last_seen_at_ms = Some(observed_at_ms);
        bridge.identifiers = vec![protocol_identifier("udp_endpoint", &address)?];
        bridge.metadata = vec![Metadata::new("lifx.transport", "lan_udp_polling")];
        runtime.upsert_bridge(bridge)?;

        let name = if status.label.is_empty() {
            format!("LIFX {serial}")
        } else {
            status.label.clone()
        };
        runtime.upsert_device(Device {
            device_id: device_id.clone(),
            bridge_id: config.bridge_id.clone(),
            manufacturer: "LIFX".to_string(),
            model: "LIFX LAN light".to_string(),
            name: name.clone(),
            serial: Some(serial.clone()),
            firmware_version: None,
            room_id: None,
            entity_ids: vec![entity_id.clone()],
            identifiers: vec![protocol_identifier("serial", &serial)?],
            health: Health::Online,
            metadata: vec![Metadata::new("lifx.cloud_required", "false")],
        })?;
        runtime.upsert_entity(Entity {
            entity_id: entity_id.clone(),
            device_id: device_id.clone(),
            kind: EntityKind::Light,
            name,
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
            metadata: vec![Metadata::new("lifx.protocol", PROTOCOL_ID)],
        })?;
        self.entity_id = Some(entity_id.clone());
        Ok(InstalledLifxDevice {
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
    ) -> Result<CommandResult, LifxError> {
        if self.entity_id.as_ref() != Some(&request.entity_id) {
            return Err(LifxError::UnknownEntity(request.entity_id.clone()));
        }
        let command_type = request.command_type;
        let result = runtime.execute_command_tool(principal_id, request.clone(), now_ms)?;
        let current = self.client.inspect()?;
        let command = command_packet(&request, &current)?;
        self.client
            .send_command(command.packet_type, &command.payload)?;
        let confirmed = self.client.inspect()?;
        if !command.expectation.matches(&confirmed) {
            return Err(LifxError::VerificationFailed { command_type });
        }
        self.install_status(runtime, &confirmed, now_ms)?;
        Ok(result)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingCommand {
    packet_type: u16,
    payload: Vec<u8>,
    expectation: CommandExpectation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CommandExpectation {
    Power(bool),
    Color(LifxHsbk),
}

impl CommandExpectation {
    fn matches(&self, status: &LifxStatus) -> bool {
        match self {
            Self::Power(expected) => status.on == *expected,
            Self::Color(expected) => status.color == *expected,
        }
    }
}

fn command_packet(
    request: &RuntimeCommandToolRequest,
    current: &LifxStatus,
) -> Result<PendingCommand, LifxError> {
    match request.command_type {
        CommandType::TurnOn | CommandType::TurnOff => {
            let on = request.command_type == CommandType::TurnOn;
            let mut payload = Vec::with_capacity(6);
            payload.extend_from_slice(&(if on { u16::MAX } else { 0 }).to_le_bytes());
            payload.extend_from_slice(&0u32.to_le_bytes());
            Ok(PendingCommand {
                packet_type: SET_LIGHT_POWER,
                payload,
                expectation: CommandExpectation::Power(on),
            })
        }
        CommandType::SetBrightness => {
            let Value::Percentage(brightness) = request.arguments else {
                return invalid_arguments(request.command_type, "percentage");
            };
            let mut color = current.color;
            color.brightness = percentage_to_u16(brightness);
            color_command(color)
        }
        CommandType::SetColor => {
            let rgb = rgb_arguments(&request.arguments, request.command_type)?;
            let (hue, saturation, brightness) = rgb_to_hsb(rgb);
            color_command(LifxHsbk {
                hue,
                saturation,
                brightness,
                kelvin: current.color.kelvin,
            })
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
            let mut color = current.color;
            color.saturation = 0;
            color.kelvin = u16::try_from(1_000_000 / mirek).map_err(|_| {
                LifxError::InvalidCommandArguments {
                    command_type: request.command_type,
                    expected: "integer mirek from 111 through 500",
                }
            })?;
            color_command(color)
        }
        _ => Err(LifxError::UnsupportedCommand {
            entity_id: request.entity_id.clone(),
            command_type: request.command_type,
        }),
    }
}

fn color_command(color: LifxHsbk) -> Result<PendingCommand, LifxError> {
    let mut payload = Vec::with_capacity(13);
    payload.push(0);
    payload.extend_from_slice(&color.hue.to_le_bytes());
    payload.extend_from_slice(&color.saturation.to_le_bytes());
    payload.extend_from_slice(&color.brightness.to_le_bytes());
    payload.extend_from_slice(&color.kelvin.to_le_bytes());
    payload.extend_from_slice(&0u32.to_le_bytes());
    Ok(PendingCommand {
        packet_type: SET_COLOR,
        payload,
        expectation: CommandExpectation::Color(color),
    })
}

fn invalid_arguments<T>(command_type: CommandType, expected: &'static str) -> Result<T, LifxError> {
    Err(LifxError::InvalidCommandArguments {
        command_type,
        expected,
    })
}

fn parse_service_response(datagram: &UdpDatagram) -> Result<LifxAdvertisement, LifxError> {
    let packet = parse_packet(&datagram.payload)?;
    if packet.packet_type != STATE_SERVICE {
        return Err(LifxError::UnexpectedPacket {
            expected: STATE_SERVICE,
            actual: packet.packet_type,
        });
    }
    if packet.source != SOURCE_ID || packet.sequence != 0 {
        return Err(LifxError::CorrelationMismatch);
    }
    if packet.payload.len() != 5 || packet.payload[0] != SERVICE_UDP {
        return Err(LifxError::Validation(
            "StateService must advertise one UDP service and port".to_string(),
        ));
    }
    let port_u32 = u32::from_le_bytes(
        packet.payload[1..5]
            .try_into()
            .map_err(|_| LifxError::Validation("StateService port is truncated".to_string()))?,
    );
    let port = u16::try_from(port_u32)
        .map_err(|_| LifxError::Validation("StateService port is invalid".to_string()))?;
    Ok(LifxAdvertisement {
        host: datagram.source.ip(),
        port,
        serial: serial_string(packet.target),
    })
}

fn parse_light_state(
    bytes: &[u8],
    source: u32,
    sequence: u8,
    target: [u8; 6],
) -> Result<LifxStatus, LifxError> {
    let packet = parse_packet(bytes)?;
    if packet.packet_type != LIGHT_STATE {
        return Err(LifxError::UnexpectedPacket {
            expected: LIGHT_STATE,
            actual: packet.packet_type,
        });
    }
    if packet.source != source || packet.sequence != sequence || packet.target != target {
        return Err(LifxError::CorrelationMismatch);
    }
    if packet.payload.len() != 52 {
        return Err(LifxError::Validation(
            "LightState payload must be 52 bytes".to_string(),
        ));
    }
    let color = LifxHsbk {
        hue: read_u16(packet.payload, 0)?,
        saturation: read_u16(packet.payload, 2)?,
        brightness: read_u16(packet.payload, 4)?,
        kelvin: read_u16(packet.payload, 6)?,
    };
    if !(1_500..=9_000).contains(&color.kelvin) {
        return Err(LifxError::Validation(
            "LightState kelvin is outside the LIFX range".to_string(),
        ));
    }
    let power = read_u16(packet.payload, 10)?;
    let label_bytes = &packet.payload[12..44];
    let label_end = label_bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(label_bytes.len());
    let label = std::str::from_utf8(&label_bytes[..label_end])
        .map_err(|_| LifxError::Validation("LightState label is not UTF-8".to_string()))?
        .to_string();
    Ok(LifxStatus {
        on: power != 0,
        color,
        label,
    })
}

#[derive(Debug)]
struct ParsedPacket<'a> {
    source: u32,
    target: [u8; 6],
    sequence: u8,
    packet_type: u16,
    payload: &'a [u8],
}

fn parse_packet(bytes: &[u8]) -> Result<ParsedPacket<'_>, LifxError> {
    if bytes.len() < HEADER_BYTES {
        return Err(LifxError::Validation(
            "LIFX packet is shorter than its header".to_string(),
        ));
    }
    let declared = usize::from(read_u16(bytes, 0)?);
    if declared != bytes.len() {
        return Err(LifxError::Validation(
            "LIFX packet size does not match datagram".to_string(),
        ));
    }
    let flags = read_u16(bytes, 2)?;
    if flags & 0x0fff != PROTOCOL || flags & 0x1000 == 0 {
        return Err(LifxError::Validation(
            "LIFX protocol or addressable flag is invalid".to_string(),
        ));
    }
    let mut target = [0u8; 6];
    target.copy_from_slice(&bytes[8..14]);
    Ok(ParsedPacket {
        source: read_u32(bytes, 4)?,
        target,
        sequence: bytes[23],
        packet_type: read_u16(bytes, 32)?,
        payload: &bytes[HEADER_BYTES..],
    })
}

fn encode_packet(
    target: Option<[u8; 6]>,
    sequence: u8,
    packet_type: u16,
    payload: &[u8],
    acknowledgement_required: bool,
) -> Vec<u8> {
    let mut packet = vec![0u8; HEADER_BYTES + payload.len()];
    let packet_size = u16::try_from(packet.len()).unwrap_or(u16::MAX);
    packet[0..2].copy_from_slice(&packet_size.to_le_bytes());
    let tagged = target.is_none();
    let flags = PROTOCOL | 0x1000 | if tagged { 0x2000 } else { 0 };
    packet[2..4].copy_from_slice(&flags.to_le_bytes());
    packet[4..8].copy_from_slice(&SOURCE_ID.to_le_bytes());
    if let Some(target) = target {
        packet[8..14].copy_from_slice(&target);
    }
    packet[22] = if acknowledgement_required { 0b10 } else { 0 };
    packet[23] = sequence;
    packet[32..34].copy_from_slice(&packet_type.to_le_bytes());
    packet[HEADER_BYTES..].copy_from_slice(payload);
    packet
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, LifxError> {
    let value = bytes
        .get(offset..offset + 2)
        .ok_or_else(|| LifxError::Validation("LIFX packet field is truncated".to_string()))?;
    Ok(u16::from_le_bytes(value.try_into().map_err(|_| {
        LifxError::Validation("LIFX packet field is truncated".to_string())
    })?))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, LifxError> {
    let value = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| LifxError::Validation("LIFX packet field is truncated".to_string()))?;
    Ok(u32::from_le_bytes(value.try_into().map_err(|_| {
        LifxError::Validation("LIFX packet field is truncated".to_string())
    })?))
}

fn parse_serial(value: &str) -> Result<[u8; 6], LifxError> {
    let compact = value.replace([':', '-'], "").to_ascii_lowercase();
    if compact.len() != 12
        || !compact
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        return Err(LifxError::Validation(
            "serial must contain exactly 12 hexadecimal digits".to_string(),
        ));
    }
    let mut serial = [0u8; 6];
    for (index, byte) in serial.iter_mut().enumerate() {
        let start = index * 2;
        *byte = u8::from_str_radix(&compact[start..start + 2], 16)
            .map_err(|_| LifxError::Validation("serial contains invalid hex".to_string()))?;
    }
    Ok(serial)
}

fn serial_string(serial: [u8; 6]) -> String {
    serial.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn status_value(status: &LifxStatus) -> Value {
    let [red, green, blue] = hsb_to_rgb(status.color);
    let brightness = u16_to_percentage(status.color.brightness);
    Value::Object(vec![
        ("on".to_string(), Value::Bool(status.on)),
        ("brightness".to_string(), Value::Percentage(brightness)),
        (
            "color".to_string(),
            Value::Array(vec![
                Value::Integer(i64::from(red)),
                Value::Integer(i64::from(green)),
                Value::Integer(i64::from(blue)),
            ]),
        ),
        (
            "color_temperature".to_string(),
            Value::Integer(i64::from(1_000_000u32 / u32::from(status.color.kelvin))),
        ),
    ])
}

fn percentage_to_u16(value: u8) -> u16 {
    u16::try_from((u32::from(value) * u32::from(u16::MAX) + 50) / 100).unwrap_or(u16::MAX)
}

fn u16_to_percentage(value: u16) -> u8 {
    u8::try_from((u32::from(value) * 100 + 32_767) / u32::from(u16::MAX)).unwrap_or(100)
}

fn rgb_arguments(value: &Value, command_type: CommandType) -> Result<[u8; 3], LifxError> {
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
        rgb[index] = u8::try_from(*channel).map_err(|_| LifxError::InvalidCommandArguments {
            command_type,
            expected: "RGB array with three integer channels",
        })?;
    }
    Ok(rgb)
}

fn rgb_to_hsb([red, green, blue]: [u8; 3]) -> (u16, u16, u16) {
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
        60.0 * ((blue - red) / delta + 2.0)
    } else {
        60.0 * ((red - green) / delta + 4.0)
    };
    let saturation = if maximum == 0.0 { 0.0 } else { delta / maximum };
    (
        ((hue / 360.0) * 65_536.0).round() as u16,
        (saturation * 65_535.0).round() as u16,
        (maximum * 65_535.0).round() as u16,
    )
}

fn hsb_to_rgb(color: LifxHsbk) -> [u8; 3] {
    let hue = f64::from(color.hue) * 360.0 / 65_536.0;
    let saturation = f64::from(color.saturation) / 65_535.0;
    let value = f64::from(color.brightness) / 65_535.0;
    let chroma = value * saturation;
    let x = chroma * (1.0 - ((hue / 60.0).rem_euclid(2.0) - 1.0).abs());
    let (red, green, blue) = match (hue / 60.0).floor() as u8 {
        0 => (chroma, x, 0.0),
        1 => (x, chroma, 0.0),
        2 => (0.0, chroma, x),
        3 => (0.0, x, chroma),
        4 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };
    let match_value = value - chroma;
    [
        ((red + match_value) * 255.0).round() as u8,
        ((green + match_value) * 255.0).round() as u8,
        ((blue + match_value) * 255.0).round() as u8,
    ]
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

fn protocol_identifier(kind: &str, value: &str) -> Result<ProtocolIdentifier, LifxError> {
    ProtocolIdentifier::new(ProtocolFamily::Vendor(PROTOCOL_ID.to_string()), kind, value)
        .map_err(|error| LifxError::Validation(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use smart_home_core::{CapabilityGrant, CapabilityGrantId, PrivilegeTier};
    use std::net::UdpSocket;
    use std::sync::{Arc, Mutex};
    use std::thread;

    const SERIAL: [u8; 6] = [0xd0, 0x73, 0xd5, 0, 0x13, 0x37];

    fn state_packet(sequence: u8, on: bool, color: LifxHsbk, label: &str) -> Vec<u8> {
        let mut payload = vec![0u8; 52];
        payload[0..2].copy_from_slice(&color.hue.to_le_bytes());
        payload[2..4].copy_from_slice(&color.saturation.to_le_bytes());
        payload[4..6].copy_from_slice(&color.brightness.to_le_bytes());
        payload[6..8].copy_from_slice(&color.kelvin.to_le_bytes());
        payload[10..12].copy_from_slice(&(if on { u16::MAX } else { 0 }).to_le_bytes());
        let label = label.as_bytes();
        payload[12..12 + label.len()].copy_from_slice(label);
        encode_packet(Some(SERIAL), sequence, LIGHT_STATE, &payload, false)
    }

    fn grant(runtime: &mut SmartHomeRuntime, principal: &AgentId) {
        let _ = runtime.registry_mut().upsert_capability_grant(
            CapabilityGrant::for_all_smart_home(
                CapabilityGrantId::trusted("grant:lifx-lan-test"),
                principal.clone(),
                PrivilegeTier::LowRisk,
                "test",
                1_000,
            )
            .with_expiry(20_000),
        );
    }

    #[test]
    fn packet_codec_matches_documented_header_and_rejects_size_mismatch() {
        let packet = encode_packet(None, 7, GET_SERVICE, &[], false);
        assert_eq!(packet.len(), HEADER_BYTES);
        assert_eq!(read_u16(&packet, 2).unwrap(), 0x3400);
        assert_eq!(read_u16(&packet, 32).unwrap(), GET_SERVICE);
        let mut malformed = packet;
        malformed[0] = 99;
        assert!(matches!(
            parse_packet(&malformed),
            Err(LifxError::Validation(_))
        ));
    }

    #[test]
    fn real_udp_scan_produces_verified_broadcast_record() {
        let server = UdpSocket::bind("127.0.0.1:0").unwrap();
        let destination = server.local_addr().unwrap();
        let port = destination.port();
        let handle = thread::spawn(move || {
            let mut buffer = [0u8; 512];
            let (size, source) = server.recv_from(&mut buffer).unwrap();
            assert_eq!(
                parse_packet(&buffer[..size]).unwrap().packet_type,
                GET_SERVICE
            );
            let mut payload = vec![SERVICE_UDP];
            payload.extend_from_slice(&u32::from(port).to_le_bytes());
            let response = encode_packet(Some(SERIAL), 0, STATE_SERVICE, &payload, false);
            server.send_to(&response, source).unwrap();
        });
        let config = LifxScanConfig {
            destination,
            response_port: 0,
            timeout: Duration::from_millis(100),
            maximum_datagram_bytes: 512,
            maximum_responses: 4,
        };
        let result = scan_lan(&config, &mut LifxLanTransport).unwrap();
        handle.join().unwrap();
        assert_eq!(result.devices.len(), 1);
        assert!(result.failures.is_empty());
        let record = discovery_record(&result.devices[0], 5_000).unwrap();
        assert_eq!(record.source, DiscoverySource::UdpBroadcast);
        assert_eq!(record.transport, BridgeTransport::LanUdp);
        assert_eq!(record.native_bridge_id, "d073d5001337");
    }

    #[test]
    fn real_udp_install_and_authorized_power_command_are_verified() {
        let server = UdpSocket::bind("127.0.0.1:0").unwrap();
        let port = server.local_addr().unwrap().port();
        server
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let captured = Arc::new(Mutex::new(Vec::new()));
        let server_captured = Arc::clone(&captured);
        let color = LifxHsbk {
            hue: 10,
            saturation: 20,
            brightness: 30_000,
            kelvin: 4_000,
        };
        let handle = thread::spawn(move || {
            for (index, response) in [Some(false), Some(false), None, Some(true)]
                .into_iter()
                .enumerate()
            {
                let mut buffer = [0u8; 512];
                let (size, source) = server.recv_from(&mut buffer).unwrap();
                let packet = parse_packet(&buffer[..size]).unwrap();
                server_captured.lock().unwrap().push(packet.packet_type);
                if let Some(on) = response {
                    let reply = state_packet(u8::try_from(index).unwrap(), on, color, "Desk");
                    server.send_to(&reply, source).unwrap();
                }
            }
        });
        let config = LifxDeviceConfig::new(
            BridgeId::trusted("lifx.bridge.test"),
            "127.0.0.1",
            "d073d5001337",
        )
        .unwrap()
        .with_port(port);
        let client = LifxClient::new(config, LifxLanTransport);
        let mut integration = LifxRuntimeIntegration::new(client);
        let mut runtime = SmartHomeRuntime::new();
        let installed = integration
            .inspect_and_install(&mut runtime, 5_000)
            .unwrap();
        let principal = AgentId::trusted("agent:lifx-test");
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
        assert_eq!(
            *captured.lock().unwrap(),
            [GET_COLOR, GET_COLOR, SET_LIGHT_POWER, GET_COLOR]
        );
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
    fn color_commands_use_documented_hsbk_payloads() {
        let current = LifxStatus {
            on: true,
            color: LifxHsbk {
                hue: 1,
                saturation: 2,
                brightness: 3,
                kelvin: 4_000,
            },
            label: String::new(),
        };
        let request = RuntimeCommandToolRequest::new(
            EntityId::trusted("lifx:test:light"),
            CommandType::SetColor,
            Value::Array(vec![
                Value::Integer(0),
                Value::Integer(255),
                Value::Integer(0),
            ]),
        );
        let command = command_packet(&request, &current).unwrap();
        assert_eq!(command.packet_type, SET_COLOR);
        assert_eq!(command.payload.len(), 13);
        assert_eq!(read_u16(&command.payload, 1).unwrap(), 21_845);
        assert_eq!(read_u16(&command.payload, 3).unwrap(), u16::MAX);
        assert_eq!(read_u16(&command.payload, 5).unwrap(), u16::MAX);
        assert_eq!(read_u16(&command.payload, 7).unwrap(), 4_000);
    }
}
