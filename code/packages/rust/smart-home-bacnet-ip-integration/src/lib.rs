//! Authorized bounded BACnet/IP discovery and Device-object telemetry for D23.

#![forbid(unsafe_code)]

use bacnet_protocol::{
    decode_i_am, decode_read_property_ack, encode_read_property, encode_who_is, BacnetError,
    DeviceProperty, IAmResponse, ReadPropertyRequest, ReadPropertyValue, WhoIsRequest,
    BACNET_IP_DEFAULT_PORT, MAX_BACNET_IP_DATAGRAM_BYTES,
};
use smart_home_core::{
    AgentId, Bridge, BridgeId, BridgeTransport, Capability, CapabilityId, CapabilityMode, Device,
    DeviceId, Entity, EntityId, EntityKind, Health, IntegrationId, Metadata, ProtocolFamily,
    ProtocolIdentifier, SmartHomeTool, StateConfidence, StateSnapshot, StateSource, Value,
    ValueKind,
};
use smart_home_discovery::{
    DiscoveryConfidence, DiscoveryError, DiscoveryRecord, DiscoverySource, DiscoveryUpsert,
    PairingRequirement,
};
use smart_home_runtime::{RuntimeError, SmartHomeRuntime};
use std::collections::BTreeMap;
use std::fmt;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::time::Duration;
use udp_client::{UdpClient, UdpDatagram, UdpError, UdpOptions};

pub const VERSION: &str = "0.1.0";
pub const INTEGRATION_ID: &str = "bacnet_ip";
pub const PROTOCOL_ID: &str = "bacnet_ip";
pub const MAX_RESPONSES: usize = 64;
pub const MAX_DEVICE_READ_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug)]
pub enum BacnetIntegrationError {
    Validation(String),
    Protocol(BacnetError),
    Udp(UdpError),
    Discovery(DiscoveryError),
    Runtime(RuntimeError),
    ResponseCount {
        expected: usize,
        actual: usize,
    },
    PropertyType {
        property: DeviceProperty,
        expected: &'static str,
    },
    EndpointMismatch {
        expected: SocketAddrV4,
        actual: SocketAddr,
    },
}

impl fmt::Display for BacnetIntegrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => write!(formatter, "invalid BACnet/IP input: {message}"),
            Self::Protocol(error) => error.fmt(formatter),
            Self::Udp(error) => error.fmt(formatter),
            Self::Discovery(error) => error.fmt(formatter),
            Self::Runtime(error) => error.fmt(formatter),
            Self::ResponseCount { expected, actual } => write!(
                formatter,
                "BACnet Device inspection expected {expected} property responses, got {actual}"
            ),
            Self::PropertyType { property, expected } => write!(
                formatter,
                "BACnet Device property `{}` did not return {expected}",
                property.as_str()
            ),
            Self::EndpointMismatch { expected, actual } => write!(
                formatter,
                "BACnet property reply came from {actual}, expected {expected}"
            ),
        }
    }
}

impl std::error::Error for BacnetIntegrationError {}

impl From<BacnetError> for BacnetIntegrationError {
    fn from(error: BacnetError) -> Self {
        Self::Protocol(error)
    }
}

impl From<UdpError> for BacnetIntegrationError {
    fn from(error: UdpError) -> Self {
        Self::Udp(error)
    }
}

impl From<DiscoveryError> for BacnetIntegrationError {
    fn from(error: DiscoveryError) -> Self {
        Self::Discovery(error)
    }
}

impl From<RuntimeError> for BacnetIntegrationError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BacnetIpDiscoveryConfig {
    pub destination: SocketAddrV4,
    pub bind_addr: SocketAddrV4,
    pub timeout: Duration,
    pub maximum_responses: usize,
    pub record_ttl: Duration,
    pub request: WhoIsRequest,
}

impl BacnetIpDiscoveryConfig {
    pub fn new(destination: Ipv4Addr) -> Self {
        Self {
            destination: SocketAddrV4::new(destination, BACNET_IP_DEFAULT_PORT),
            bind_addr: SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0),
            timeout: Duration::from_millis(750),
            maximum_responses: 32,
            record_ttl: Duration::from_secs(300),
            request: WhoIsRequest::All,
        }
    }

    pub fn validate(&self) -> Result<(), BacnetIntegrationError> {
        if self.destination.ip().is_unspecified() {
            return Err(BacnetIntegrationError::Validation(
                "destination must be an explicit IPv4 address".to_string(),
            ));
        }
        if self.destination.port() == 0 {
            return Err(BacnetIntegrationError::Validation(
                "destination port must be non-zero".to_string(),
            ));
        }
        if self.timeout.is_zero() {
            return Err(BacnetIntegrationError::Validation(
                "timeout must be non-zero".to_string(),
            ));
        }
        if !(1..=MAX_RESPONSES).contains(&self.maximum_responses) {
            return Err(BacnetIntegrationError::Validation(format!(
                "maximum responses must be between 1 and {MAX_RESPONSES}"
            )));
        }
        if self.record_ttl.is_zero() {
            return Err(BacnetIntegrationError::Validation(
                "record TTL must be non-zero".to_string(),
            ));
        }
        let _ = encode_who_is(self.request)?;
        Ok(())
    }
}

pub trait BacnetIpTransport {
    fn discover(
        &mut self,
        config: &BacnetIpDiscoveryConfig,
        probe: &[u8],
    ) -> Result<Vec<UdpDatagram>, BacnetIntegrationError>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct UdpBacnetIpTransport;

impl BacnetIpTransport for UdpBacnetIpTransport {
    fn discover(
        &mut self,
        config: &BacnetIpDiscoveryConfig,
        probe: &[u8],
    ) -> Result<Vec<UdpDatagram>, BacnetIntegrationError> {
        let client = UdpClient::bind(UdpOptions {
            bind_addr: Some(SocketAddr::V4(config.bind_addr)),
            max_datagram_size: MAX_BACNET_IP_DATAGRAM_BYTES,
            read_timeout: Some(config.timeout),
            write_timeout: Some(config.timeout),
        })?;
        client.set_broadcast(true)?;
        client.send_to(probe, SocketAddr::V4(config.destination))?;
        let mut datagrams = Vec::new();
        while datagrams.len() < config.maximum_responses {
            match client.recv_from() {
                Ok(datagram) => datagrams.push(datagram),
                Err(UdpError::Timeout) => break,
                Err(error) => return Err(error.into()),
            }
        }
        Ok(datagrams)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BacnetDeviceReadConfig {
    pub endpoint: SocketAddrV4,
    pub bind_addr: SocketAddrV4,
    pub timeout: Duration,
    pub device_instance: u32,
}

impl BacnetDeviceReadConfig {
    pub fn new(
        endpoint: SocketAddrV4,
        device_instance: u32,
    ) -> Result<Self, BacnetIntegrationError> {
        let config = Self {
            endpoint,
            bind_addr: SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0),
            timeout: Duration::from_millis(750),
            device_instance,
        };
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), BacnetIntegrationError> {
        if !is_local_ipv4(*self.endpoint.ip()) {
            return Err(BacnetIntegrationError::Validation(
                "property endpoint must use a private, link-local, or loopback IPv4 address"
                    .to_string(),
            ));
        }
        if self.endpoint.port() == 0 {
            return Err(BacnetIntegrationError::Validation(
                "property endpoint port must be non-zero".to_string(),
            ));
        }
        if self.timeout.is_zero() {
            return Err(BacnetIntegrationError::Validation(
                "property timeout must be non-zero".to_string(),
            ));
        }
        if self.timeout > MAX_DEVICE_READ_TIMEOUT {
            return Err(BacnetIntegrationError::Validation(format!(
                "property timeout must not exceed {} seconds",
                MAX_DEVICE_READ_TIMEOUT.as_secs()
            )));
        }
        let _ = ReadPropertyRequest::new(0, self.device_instance, DeviceProperty::ObjectName)?;
        Ok(())
    }
}

pub trait BacnetPropertyTransport {
    fn read_properties(
        &mut self,
        config: &BacnetDeviceReadConfig,
        requests: &[ReadPropertyRequest],
    ) -> Result<Vec<ReadPropertyValue>, BacnetIntegrationError>;
}

impl BacnetPropertyTransport for UdpBacnetIpTransport {
    fn read_properties(
        &mut self,
        config: &BacnetDeviceReadConfig,
        requests: &[ReadPropertyRequest],
    ) -> Result<Vec<ReadPropertyValue>, BacnetIntegrationError> {
        config.validate()?;
        validate_property_requests(config, requests)?;
        let mut client = UdpClient::bind(UdpOptions {
            bind_addr: Some(SocketAddr::V4(config.bind_addr)),
            max_datagram_size: MAX_BACNET_IP_DATAGRAM_BYTES,
            read_timeout: Some(config.timeout),
            write_timeout: Some(config.timeout),
        })?;
        client.connect(SocketAddr::V4(config.endpoint))?;

        let mut values = Vec::with_capacity(requests.len());
        for request in requests {
            let payload = encode_read_property(*request)?;
            client.send(&payload)?;
            let reply = client.recv_from()?;
            if reply.source != SocketAddr::V4(config.endpoint) {
                return Err(BacnetIntegrationError::EndpointMismatch {
                    expected: config.endpoint,
                    actual: reply.source,
                });
            }
            values.push(decode_read_property_ack(&reply.payload, *request)?);
        }
        Ok(values)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BacnetSystemStatus {
    Operational,
    OperationalReadOnly,
    DownloadRequired,
    DownloadInProgress,
    NonOperational,
    BackupInProgress,
    Other(u32),
}

impl BacnetSystemStatus {
    pub const fn from_value(value: u32) -> Self {
        match value {
            0 => Self::Operational,
            1 => Self::OperationalReadOnly,
            2 => Self::DownloadRequired,
            3 => Self::DownloadInProgress,
            4 => Self::NonOperational,
            5 => Self::BackupInProgress,
            other => Self::Other(other),
        }
    }

    pub fn as_str(self) -> String {
        match self {
            Self::Operational => "operational".to_string(),
            Self::OperationalReadOnly => "operational_read_only".to_string(),
            Self::DownloadRequired => "download_required".to_string(),
            Self::DownloadInProgress => "download_in_progress".to_string(),
            Self::NonOperational => "non_operational".to_string(),
            Self::BackupInProgress => "backup_in_progress".to_string(),
            Self::Other(value) => format!("other_{value}"),
        }
    }

    pub const fn health(self) -> Health {
        match self {
            Self::Operational | Self::OperationalReadOnly => Health::Online,
            Self::DownloadRequired | Self::DownloadInProgress | Self::BackupInProgress => {
                Health::Degraded
            }
            Self::NonOperational => Health::Offline,
            Self::Other(_) => Health::Unknown,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BacnetDeviceSnapshot {
    pub device_instance: u32,
    pub object_name: String,
    pub system_status: BacnetSystemStatus,
    pub vendor_name: String,
    pub vendor_identifier: u16,
    pub model_name: String,
    pub firmware_revision: String,
    pub application_software_version: String,
    pub protocol_version: u32,
}

pub struct BacnetDeviceClient<T> {
    config: BacnetDeviceReadConfig,
    transport: T,
    next_invoke_id: u8,
}

impl<T: BacnetPropertyTransport> BacnetDeviceClient<T> {
    pub fn new(config: BacnetDeviceReadConfig, transport: T) -> Self {
        Self {
            config,
            transport,
            next_invoke_id: 1,
        }
    }

    pub fn config(&self) -> &BacnetDeviceReadConfig {
        &self.config
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    pub fn inspect(&mut self) -> Result<BacnetDeviceSnapshot, BacnetIntegrationError> {
        self.config.validate()?;
        let requests = DeviceProperty::ALL
            .iter()
            .copied()
            .map(|property| {
                let invoke_id = self.next_invoke_id;
                self.next_invoke_id = self.next_invoke_id.wrapping_add(1);
                ReadPropertyRequest::new(invoke_id, self.config.device_instance, property)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let responses = self.transport.read_properties(&self.config, &requests)?;
        if responses.len() != requests.len() {
            return Err(BacnetIntegrationError::ResponseCount {
                expected: requests.len(),
                actual: responses.len(),
            });
        }
        let mut values = requests
            .iter()
            .map(|request| request.property)
            .zip(responses)
            .collect::<BTreeMap<_, _>>();
        let vendor_identifier = take_unsigned(&mut values, DeviceProperty::VendorIdentifier)?;
        let vendor_identifier = u16::try_from(vendor_identifier).map_err(|_| {
            BacnetIntegrationError::Validation(
                "BACnet vendor identifier exceeds the 16-bit standard range".to_string(),
            )
        })?;
        Ok(BacnetDeviceSnapshot {
            device_instance: self.config.device_instance,
            object_name: take_string(&mut values, DeviceProperty::ObjectName)?,
            system_status: BacnetSystemStatus::from_value(take_enumerated(
                &mut values,
                DeviceProperty::SystemStatus,
            )?),
            vendor_name: take_string(&mut values, DeviceProperty::VendorName)?,
            vendor_identifier,
            model_name: take_string(&mut values, DeviceProperty::ModelName)?,
            firmware_revision: take_string(&mut values, DeviceProperty::FirmwareRevision)?,
            application_software_version: take_string(
                &mut values,
                DeviceProperty::ApplicationSoftwareVersion,
            )?,
            protocol_version: take_unsigned(&mut values, DeviceProperty::ProtocolVersion)?,
        })
    }
}

impl<T> fmt::Debug for BacnetDeviceClient<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BacnetDeviceClient")
            .field("config", &self.config)
            .field("next_invoke_id", &self.next_invoke_id)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledBacnetDevice {
    pub bridge_id: BridgeId,
    pub device_id: DeviceId,
    pub entity_id: EntityId,
}

pub struct BacnetDeviceRuntimeIntegration<T> {
    client: BacnetDeviceClient<T>,
}

impl<T: BacnetPropertyTransport> BacnetDeviceRuntimeIntegration<T> {
    pub fn new(client: BacnetDeviceClient<T>) -> Self {
        Self { client }
    }

    pub fn inspect_and_install_authorized(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        principal_id: AgentId,
        observed_at_ms: u64,
    ) -> Result<InstalledBacnetDevice, BacnetIntegrationError> {
        authorize_read(runtime, principal_id, observed_at_ms)?;
        let snapshot = self.client.inspect()?;
        self.install_snapshot(runtime, &snapshot, observed_at_ms)
    }

    pub fn install_snapshot(
        &self,
        runtime: &mut SmartHomeRuntime,
        snapshot: &BacnetDeviceSnapshot,
        observed_at_ms: u64,
    ) -> Result<InstalledBacnetDevice, BacnetIntegrationError> {
        if snapshot.device_instance != self.client.config.device_instance {
            return Err(BacnetIntegrationError::Validation(
                "BACnet snapshot device instance does not match configuration".to_string(),
            ));
        }
        validate_snapshot(snapshot)?;
        let native_id = format!("device-{}", snapshot.device_instance);
        let bridge_id = BridgeId::trusted(format!("{INTEGRATION_ID}.bridge.{native_id}"));
        let device_id = DeviceId::trusted(format!("bacnet:{native_id}"));
        let entity_id = EntityId::trusted(format!("{}:diagnostic:device", device_id.as_str()));
        let endpoint = self.client.config.endpoint.to_string();
        let health = snapshot.system_status.health();

        let mut bridge = Bridge::new(
            bridge_id.clone(),
            IntegrationId::trusted(INTEGRATION_ID),
            BridgeTransport::LanUdp,
        );
        bridge.address = Some(endpoint.clone());
        bridge.hardware_model = Some(snapshot.model_name.clone());
        bridge.health = health;
        bridge.last_seen_at_ms = Some(observed_at_ms);
        bridge.identifiers = vec![protocol_identifier("udp_endpoint", &endpoint)?];
        bridge.metadata = vec![
            Metadata::new("bacnet.access", "read_only"),
            Metadata::new(
                "bacnet.protocol_version",
                snapshot.protocol_version.to_string(),
            ),
            Metadata::new(
                "bacnet.vendor_identifier",
                snapshot.vendor_identifier.to_string(),
            ),
        ];
        runtime.upsert_bridge(bridge)?;

        runtime.upsert_device(Device {
            device_id: device_id.clone(),
            bridge_id: bridge_id.clone(),
            manufacturer: snapshot.vendor_name.clone(),
            model: snapshot.model_name.clone(),
            name: snapshot.object_name.clone(),
            serial: None,
            firmware_version: Some(snapshot.firmware_revision.clone()),
            room_id: None,
            entity_ids: vec![entity_id.clone()],
            identifiers: vec![protocol_identifier(
                "device_instance",
                &snapshot.device_instance.to_string(),
            )?],
            health,
            metadata: vec![
                Metadata::new("bacnet.endpoint", endpoint),
                Metadata::new(
                    "bacnet.application_software_version",
                    snapshot.application_software_version.clone(),
                ),
            ],
        })?;
        runtime.upsert_entity(Entity {
            entity_id: entity_id.clone(),
            device_id: device_id.clone(),
            kind: EntityKind::NetworkDiagnostic,
            name: format!("{} Device Status", snapshot.object_name),
            capabilities: vec![Capability::new(
                CapabilityId::trusted("bacnet.device_information"),
                CapabilityMode::Observe,
                ValueKind::Object,
            )],
            state: Some(StateSnapshot {
                entity_id: entity_id.clone(),
                value: snapshot_value(snapshot),
                source: StateSource::Poll,
                observed_at_ms,
                received_at_ms: observed_at_ms,
                expires_at_ms: None,
                confidence: StateConfidence::Confirmed,
            }),
            metadata: vec![Metadata::new("bacnet.object_type", "device")],
        })?;
        Ok(InstalledBacnetDevice {
            bridge_id,
            device_id,
            entity_id,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BacnetDiscoveryReport {
    pub records: Vec<DiscoveryRecord>,
    pub failures: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BacnetRuntimeCommitSummary {
    pub inserted: usize,
    pub replaced: usize,
    pub ignored: usize,
    pub failures: usize,
}

pub fn discover<T: BacnetIpTransport>(
    config: &BacnetIpDiscoveryConfig,
    transport: &mut T,
    discovered_at_ms: u64,
) -> Result<BacnetDiscoveryReport, BacnetIntegrationError> {
    config.validate()?;
    let probe = encode_who_is(config.request)?;
    let datagrams = transport.discover(config, &probe)?;
    let mut devices = BTreeMap::<u32, (SocketAddrV4, IAmResponse)>::new();
    let mut failures = Vec::new();

    for datagram in datagrams {
        let source = match datagram.source {
            SocketAddr::V4(source) => source,
            SocketAddr::V6(source) => {
                failures.push(format!("ignored IPv6 BACnet/IP reply from {source}"));
                continue;
            }
        };
        match decode_i_am(&datagram.payload) {
            Ok(response) => {
                let effective_source = response.forwarded_from.unwrap_or(source);
                match devices.get(&response.device_instance) {
                    Some((existing, _)) if *existing != effective_source => failures.push(format!(
                        "BACnet device {} replied from both {existing} and {effective_source}",
                        response.device_instance
                    )),
                    Some(_) => {}
                    None => {
                        devices.insert(response.device_instance, (effective_source, response));
                    }
                }
            }
            Err(error) => failures.push(format!("invalid BACnet/IP reply from {source}: {error}")),
        }
    }

    let ttl_ms = u64::try_from(config.record_ttl.as_millis()).unwrap_or(u64::MAX);
    let expires_at_ms = discovered_at_ms.saturating_add(ttl_ms);
    let records = devices
        .into_values()
        .map(|(source, response)| {
            discovery_record(config, source, &response, discovered_at_ms, expires_at_ms)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(BacnetDiscoveryReport { records, failures })
}

pub fn discover_into_runtime<T: BacnetIpTransport>(
    runtime: &mut SmartHomeRuntime,
    principal_id: AgentId,
    config: &BacnetIpDiscoveryConfig,
    transport: &mut T,
    now_ms: u64,
) -> Result<BacnetRuntimeCommitSummary, BacnetIntegrationError> {
    let tool = SmartHomeTool::Discover;
    let decision = runtime.authorize_tool_for_principal(principal_id.clone(), tool, now_ms);
    if !decision.missing_capabilities.is_empty() {
        return Err(BacnetIntegrationError::Runtime(
            RuntimeError::UnauthorizedTool {
                principal_id,
                tool,
                missing_capabilities: decision.missing_capabilities,
            },
        ));
    }

    let report = discover(config, transport, now_ms)?;
    let mut summary = BacnetRuntimeCommitSummary {
        failures: report.failures.len(),
        ..BacnetRuntimeCommitSummary::default()
    };
    for record in report.records {
        match runtime.record_discovery(record)? {
            DiscoveryUpsert::Inserted => summary.inserted += 1,
            DiscoveryUpsert::Replaced(_) => summary.replaced += 1,
            DiscoveryUpsert::Ignored(_) => summary.ignored += 1,
        }
    }
    Ok(summary)
}

fn discovery_record(
    config: &BacnetIpDiscoveryConfig,
    source: SocketAddrV4,
    response: &IAmResponse,
    discovered_at_ms: u64,
    expires_at_ms: u64,
) -> Result<DiscoveryRecord, DiscoveryError> {
    Ok(DiscoveryRecord::new(
        IntegrationId::trusted(INTEGRATION_ID),
        ProtocolFamily::Vendor(PROTOCOL_ID.to_string()),
        format!("device-{}", response.device_instance),
        DiscoverySource::UdpBroadcast,
        BridgeTransport::LanUdp,
        discovered_at_ms,
    )?
    .with_display_name(format!("BACnet Device {}", response.device_instance))
    .with_address(source.to_string())
    .with_confidence(DiscoveryConfidence::Verified)
    .with_pairing_requirement(PairingRequirement::None)
    .with_expires_at_ms(expires_at_ms)
    .with_metadata(
        "bacnet.device_instance",
        response.device_instance.to_string(),
    )
    .with_metadata(
        "bacnet.max_apdu_length_accepted",
        response.max_apdu_length_accepted.to_string(),
    )
    .with_metadata(
        "bacnet.segmentation_supported",
        response.segmentation_supported.as_str(),
    )
    .with_metadata("bacnet.vendor_id", response.vendor_id.to_string())
    .with_metadata("bacnet.bvlc_function", response.bvlc_function.as_str())
    .with_metadata(
        "bacnet.discovery_destination",
        config.destination.to_string(),
    ))
}

fn authorize_read(
    runtime: &mut SmartHomeRuntime,
    principal_id: AgentId,
    now_ms: u64,
) -> Result<(), BacnetIntegrationError> {
    let tool = SmartHomeTool::GetState;
    let decision = runtime.authorize_tool_for_principal(principal_id.clone(), tool, now_ms);
    if decision.missing_capabilities.is_empty() {
        Ok(())
    } else {
        Err(BacnetIntegrationError::Runtime(
            RuntimeError::UnauthorizedTool {
                principal_id,
                tool,
                missing_capabilities: decision.missing_capabilities,
            },
        ))
    }
}

fn validate_property_requests(
    config: &BacnetDeviceReadConfig,
    requests: &[ReadPropertyRequest],
) -> Result<(), BacnetIntegrationError> {
    if requests.len() != DeviceProperty::ALL.len()
        || requests
            .iter()
            .zip(DeviceProperty::ALL)
            .any(|(request, property)| {
                request.device_instance != config.device_instance || request.property != property
            })
    {
        return Err(BacnetIntegrationError::Validation(
            "property plan must contain the fixed Device property allowlist in canonical order"
                .to_string(),
        ));
    }
    Ok(())
}

fn validate_snapshot(snapshot: &BacnetDeviceSnapshot) -> Result<(), BacnetIntegrationError> {
    for (field, value) in [
        ("object name", snapshot.object_name.as_str()),
        ("vendor name", snapshot.vendor_name.as_str()),
        ("model name", snapshot.model_name.as_str()),
        ("firmware revision", snapshot.firmware_revision.as_str()),
        (
            "application software version",
            snapshot.application_software_version.as_str(),
        ),
    ] {
        if value.is_empty()
            || value.len() > 128
            || !value.bytes().all(|byte| (0x20..=0x7e).contains(&byte))
        {
            return Err(BacnetIntegrationError::Validation(format!(
                "BACnet {field} must contain 1..=128 printable ANSI X3.4 bytes"
            )));
        }
    }
    if snapshot.protocol_version != 1 {
        return Err(BacnetIntegrationError::Validation(format!(
            "BACnet protocol version must be 1, got {}",
            snapshot.protocol_version
        )));
    }
    Ok(())
}

fn take_string(
    values: &mut BTreeMap<DeviceProperty, ReadPropertyValue>,
    property: DeviceProperty,
) -> Result<String, BacnetIntegrationError> {
    match values.remove(&property) {
        Some(ReadPropertyValue::CharacterString(value)) => Ok(value),
        _ => Err(BacnetIntegrationError::PropertyType {
            property,
            expected: "a bounded ANSI X3.4 character string",
        }),
    }
}

fn take_unsigned(
    values: &mut BTreeMap<DeviceProperty, ReadPropertyValue>,
    property: DeviceProperty,
) -> Result<u32, BacnetIntegrationError> {
    match values.remove(&property) {
        Some(ReadPropertyValue::Unsigned(value)) => Ok(value),
        _ => Err(BacnetIntegrationError::PropertyType {
            property,
            expected: "an unsigned integer",
        }),
    }
}

fn take_enumerated(
    values: &mut BTreeMap<DeviceProperty, ReadPropertyValue>,
    property: DeviceProperty,
) -> Result<u32, BacnetIntegrationError> {
    match values.remove(&property) {
        Some(ReadPropertyValue::Enumerated(value)) => Ok(value),
        _ => Err(BacnetIntegrationError::PropertyType {
            property,
            expected: "an enumerated value",
        }),
    }
}

fn snapshot_value(snapshot: &BacnetDeviceSnapshot) -> Value {
    Value::Object(vec![
        (
            "application_software_version".to_string(),
            Value::Text(snapshot.application_software_version.clone()),
        ),
        (
            "firmware_revision".to_string(),
            Value::Text(snapshot.firmware_revision.clone()),
        ),
        (
            "model_name".to_string(),
            Value::Text(snapshot.model_name.clone()),
        ),
        (
            "object_name".to_string(),
            Value::Text(snapshot.object_name.clone()),
        ),
        (
            "protocol_version".to_string(),
            Value::Integer(i64::from(snapshot.protocol_version)),
        ),
        (
            "system_status".to_string(),
            Value::Text(snapshot.system_status.as_str()),
        ),
        (
            "vendor_identifier".to_string(),
            Value::Integer(i64::from(snapshot.vendor_identifier)),
        ),
        (
            "vendor_name".to_string(),
            Value::Text(snapshot.vendor_name.clone()),
        ),
    ])
}

fn protocol_identifier(
    kind: &str,
    value: &str,
) -> Result<ProtocolIdentifier, BacnetIntegrationError> {
    ProtocolIdentifier::new(ProtocolFamily::Vendor(PROTOCOL_ID.to_string()), kind, value)
        .map_err(|error| BacnetIntegrationError::Validation(error.to_string()))
}

fn is_local_ipv4(address: Ipv4Addr) -> bool {
    address.is_private() || address.is_link_local() || address.is_loopback()
}

#[cfg(test)]
mod tests {
    use super::*;
    use smart_home_core::{CapabilityGrant, CapabilityGrantId, PrivilegeTier};
    use std::net::UdpSocket;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::thread;

    fn i_am(instance: u32) -> Vec<u8> {
        let object_id = (8u32 << 22) | instance;
        let mut bytes = vec![
            0x81, 0x0a, 0x00, 0x14, 0x01, 0x00, 0x10, 0x00, 0xc4, 0, 0, 0, 0, 0x22, 0x05, 0xc4,
            0x91, 0x03, 0x21, 0x63,
        ];
        bytes[9..13].copy_from_slice(&object_id.to_be_bytes());
        bytes
    }

    #[derive(Debug)]
    struct FakeTransport {
        calls: Arc<AtomicUsize>,
        replies: Vec<UdpDatagram>,
    }

    impl BacnetIpTransport for FakeTransport {
        fn discover(
            &mut self,
            _config: &BacnetIpDiscoveryConfig,
            probe: &[u8],
        ) -> Result<Vec<UdpDatagram>, BacnetIntegrationError> {
            assert_eq!(probe, [0x81, 0x0b, 0, 8, 1, 0, 0x10, 0x08]);
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.replies.clone())
        }
    }

    #[derive(Debug)]
    struct FakePropertyTransport {
        calls: Arc<AtomicUsize>,
        requests: Arc<Mutex<Vec<ReadPropertyRequest>>>,
        responses: Vec<ReadPropertyValue>,
    }

    impl BacnetPropertyTransport for FakePropertyTransport {
        fn read_properties(
            &mut self,
            _config: &BacnetDeviceReadConfig,
            requests: &[ReadPropertyRequest],
        ) -> Result<Vec<ReadPropertyValue>, BacnetIntegrationError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.requests.lock().unwrap().extend_from_slice(requests);
            Ok(self.responses.clone())
        }
    }

    fn property_values(status: u32) -> Vec<ReadPropertyValue> {
        vec![
            ReadPropertyValue::CharacterString("AHU-1".to_string()),
            ReadPropertyValue::Enumerated(status),
            ReadPropertyValue::CharacterString("Acme Controls".to_string()),
            ReadPropertyValue::Unsigned(42),
            ReadPropertyValue::CharacterString("VAV-200".to_string()),
            ReadPropertyValue::CharacterString("3.4.5".to_string()),
            ReadPropertyValue::CharacterString("Control 9".to_string()),
            ReadPropertyValue::Unsigned(1),
        ]
    }

    fn read_property_ack(request: ReadPropertyRequest, value: &ReadPropertyValue) -> Vec<u8> {
        let object_id = (8u32 << 22) | request.device_instance;
        let mut bytes = vec![0x81, 0x0a, 0, 0, 1, 0, 0x30, request.invoke_id, 12, 0x0c];
        bytes.extend_from_slice(&object_id.to_be_bytes());
        bytes.extend_from_slice(&[0x19, request.property.id() as u8, 0x3e]);
        match value {
            ReadPropertyValue::CharacterString(value) => {
                bytes.extend_from_slice(&[0x75, (value.len() + 1) as u8, 0]);
                bytes.extend_from_slice(value.as_bytes());
            }
            ReadPropertyValue::Enumerated(value) => {
                bytes.extend_from_slice(&[0x91, *value as u8]);
            }
            ReadPropertyValue::Unsigned(value) => {
                bytes.extend_from_slice(&[0x21, *value as u8]);
            }
        }
        bytes.push(0x3f);
        let length = bytes.len() as u16;
        bytes[2..4].copy_from_slice(&length.to_be_bytes());
        bytes
    }

    fn datagram(source: SocketAddrV4, payload: Vec<u8>) -> UdpDatagram {
        UdpDatagram {
            source: SocketAddr::V4(source),
            destination: "127.0.0.1:47808".parse().unwrap(),
            payload,
        }
    }

    fn grant(runtime: &mut SmartHomeRuntime, principal: &AgentId) {
        let _ =
            runtime
                .registry_mut()
                .upsert_capability_grant(CapabilityGrant::for_all_smart_home(
                    CapabilityGrantId::trusted("grant:bacnet-test"),
                    principal.clone(),
                    PrivilegeTier::LowRisk,
                    "test",
                    0,
                ));
    }

    #[test]
    fn validates_configuration_bounds() {
        let mut config = BacnetIpDiscoveryConfig::new(Ipv4Addr::UNSPECIFIED);
        assert!(config.validate().is_err());
        config.destination = "127.0.0.1:47808".parse().unwrap();
        config.maximum_responses = 0;
        assert!(config.validate().is_err());
        config.maximum_responses = MAX_RESPONSES + 1;
        assert!(config.validate().is_err());

        assert!(BacnetDeviceReadConfig::new("192.0.2.1:47808".parse().unwrap(), 1).is_err());
        assert!(BacnetDeviceReadConfig::new("8.8.8.8:47808".parse().unwrap(), 1).is_err());
        assert!(BacnetDeviceReadConfig::new("127.0.0.1:0".parse().unwrap(), 1).is_err());

        let mut read_config =
            BacnetDeviceReadConfig::new("127.0.0.1:47808".parse().unwrap(), 1).unwrap();
        read_config.timeout = MAX_DEVICE_READ_TIMEOUT + Duration::from_millis(1);
        assert!(read_config.validate().is_err());
        let incomplete = [ReadPropertyRequest::new(1, 1, DeviceProperty::ObjectName).unwrap()];
        assert!(validate_property_requests(&read_config, &incomplete).is_err());

        let invalid_snapshot = BacnetDeviceSnapshot {
            device_instance: 1,
            object_name: String::new(),
            system_status: BacnetSystemStatus::Operational,
            vendor_name: "Acme".to_string(),
            vendor_identifier: 1,
            model_name: "Model".to_string(),
            firmware_revision: "1".to_string(),
            application_software_version: "1".to_string(),
            protocol_version: 1,
        };
        assert!(validate_snapshot(&invalid_snapshot).is_err());
    }

    #[test]
    fn normalizes_valid_replies_and_preserves_partial_failures() {
        let source: SocketAddrV4 = "192.0.2.10:47808".parse().unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let mut transport = FakeTransport {
            calls: calls.clone(),
            replies: vec![datagram(source, i_am(123)), datagram(source, vec![0; 8])],
        };
        let config = BacnetIpDiscoveryConfig::new(Ipv4Addr::BROADCAST);
        let report = discover(&config, &mut transport, 1_000).unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(report.records.len(), 1);
        assert_eq!(report.failures.len(), 1);
        let record = &report.records[0];
        assert_eq!(record.native_bridge_id, "device-123");
        assert_eq!(record.address.as_deref(), Some("192.0.2.10:47808"));
        assert_eq!(record.expires_at_ms, Some(301_000));
    }

    #[test]
    fn reports_conflicting_duplicate_device_instances() {
        let mut transport = FakeTransport {
            calls: Arc::new(AtomicUsize::new(0)),
            replies: vec![
                datagram("192.0.2.10:47808".parse().unwrap(), i_am(7)),
                datagram("192.0.2.11:47808".parse().unwrap(), i_am(7)),
            ],
        };
        let report = discover(
            &BacnetIpDiscoveryConfig::new(Ipv4Addr::BROADCAST),
            &mut transport,
            0,
        )
        .unwrap();
        assert_eq!(report.records.len(), 1);
        assert_eq!(report.failures.len(), 1);
    }

    #[test]
    fn denies_before_transport_io() {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut transport = FakeTransport {
            calls: calls.clone(),
            replies: Vec::new(),
        };
        let mut runtime = SmartHomeRuntime::new();
        let result = discover_into_runtime(
            &mut runtime,
            AgentId::trusted("agent:denied"),
            &BacnetIpDiscoveryConfig::new(Ipv4Addr::LOCALHOST),
            &mut transport,
            0,
        );
        assert!(matches!(
            result,
            Err(BacnetIntegrationError::Runtime(
                RuntimeError::UnauthorizedTool { .. }
            ))
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn device_read_denies_before_transport_io() {
        let calls = Arc::new(AtomicUsize::new(0));
        let client = BacnetDeviceClient::new(
            BacnetDeviceReadConfig::new("127.0.0.1:47808".parse().unwrap(), 321).unwrap(),
            FakePropertyTransport {
                calls: calls.clone(),
                requests: Arc::new(Mutex::new(Vec::new())),
                responses: property_values(0),
            },
        );
        let mut integration = BacnetDeviceRuntimeIntegration::new(client);
        let mut runtime = SmartHomeRuntime::new();
        let result = integration.inspect_and_install_authorized(
            &mut runtime,
            AgentId::trusted("agent:denied"),
            1_000,
        );
        assert!(matches!(
            result,
            Err(BacnetIntegrationError::Runtime(
                RuntimeError::UnauthorizedTool { .. }
            ))
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn reads_fixed_device_properties_and_normalizes_status() {
        let calls = Arc::new(AtomicUsize::new(0));
        let requests = Arc::new(Mutex::new(Vec::new()));
        let client = BacnetDeviceClient::new(
            BacnetDeviceReadConfig::new("127.0.0.1:47808".parse().unwrap(), 321).unwrap(),
            FakePropertyTransport {
                calls: calls.clone(),
                requests: requests.clone(),
                responses: property_values(2),
            },
        );
        let mut integration = BacnetDeviceRuntimeIntegration::new(client);
        let principal = AgentId::trusted("agent:bacnet-read");
        let mut runtime = SmartHomeRuntime::new();
        grant(&mut runtime, &principal);
        let installed = integration
            .inspect_and_install_authorized(&mut runtime, principal, 2_000)
            .unwrap();

        assert_eq!(calls.load(Ordering::SeqCst), 1);
        let observed = requests.lock().unwrap();
        assert_eq!(observed.len(), DeviceProperty::ALL.len());
        for (index, request) in observed.iter().enumerate() {
            assert_eq!(request.invoke_id, index as u8 + 1);
            assert_eq!(request.device_instance, 321);
            assert_eq!(request.property, DeviceProperty::ALL[index]);
        }
        let device = runtime.registry().device(&installed.device_id).unwrap();
        assert_eq!(device.manufacturer, "Acme Controls");
        assert_eq!(device.model, "VAV-200");
        assert_eq!(device.health, Health::Degraded);
        let entity = runtime.registry().entity(&installed.entity_id).unwrap();
        assert_eq!(entity.kind, EntityKind::NetworkDiagnostic);
        assert_eq!(
            entity.state.as_ref().unwrap().confidence,
            StateConfidence::Confirmed
        );
    }

    #[test]
    fn discovers_over_live_loopback_udp_and_records_runtime_bridge() {
        let server = UdpSocket::bind("127.0.0.1:0").unwrap();
        server
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let destination = match server.local_addr().unwrap() {
            SocketAddr::V4(address) => address,
            SocketAddr::V6(_) => unreachable!(),
        };
        let responder = thread::spawn(move || {
            let mut probe = [0u8; 64];
            let (length, source) = server.recv_from(&mut probe).unwrap();
            assert_eq!(&probe[..length], [0x81, 0x0b, 0, 8, 1, 0, 0x10, 0x08]);
            server.send_to(&i_am(321), source).unwrap();
        });

        let mut config = BacnetIpDiscoveryConfig::new(*destination.ip());
        config.destination = destination;
        config.timeout = Duration::from_millis(100);
        let principal = AgentId::trusted("agent:bacnet-discovery");
        let mut runtime = SmartHomeRuntime::new();
        grant(&mut runtime, &principal);
        let summary = discover_into_runtime(
            &mut runtime,
            principal,
            &config,
            &mut UdpBacnetIpTransport,
            2_000,
        )
        .unwrap();
        responder.join().unwrap();
        assert_eq!(summary.inserted, 1);
        assert!(runtime
            .registry()
            .bridge(&smart_home_core::BridgeId::trusted(
                "bacnet_ip.bridge.device-321"
            ))
            .is_some());
    }

    #[test]
    fn reads_device_properties_over_live_connected_udp_in_wire_order() {
        let server = UdpSocket::bind("127.0.0.1:0").unwrap();
        server
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let endpoint = match server.local_addr().unwrap() {
            SocketAddr::V4(address) => address,
            SocketAddr::V6(_) => unreachable!(),
        };
        let responses = property_values(0);
        let responder = thread::spawn(move || {
            let mut peer = None;
            for (index, property) in DeviceProperty::ALL.iter().copied().enumerate() {
                let request = ReadPropertyRequest::new(index as u8 + 1, 321, property).unwrap();
                let mut payload = [0u8; 256];
                let (length, source) = server.recv_from(&mut payload).unwrap();
                assert_eq!(&payload[..length], encode_read_property(request).unwrap());
                if let Some(expected) = peer {
                    assert_eq!(source, expected);
                } else {
                    peer = Some(source);
                }
                server
                    .send_to(&read_property_ack(request, &responses[index]), source)
                    .unwrap();
            }
        });

        let mut config = BacnetDeviceReadConfig::new(endpoint, 321).unwrap();
        config.timeout = Duration::from_secs(1);
        let client = BacnetDeviceClient::new(config, UdpBacnetIpTransport);
        let mut integration = BacnetDeviceRuntimeIntegration::new(client);
        let principal = AgentId::trusted("agent:bacnet-loopback-read");
        let mut runtime = SmartHomeRuntime::new();
        grant(&mut runtime, &principal);
        let installed = integration
            .inspect_and_install_authorized(&mut runtime, principal, 3_000)
            .unwrap();
        responder.join().unwrap();
        assert_eq!(installed.device_id.as_str(), "bacnet:device-321");
        assert_eq!(
            runtime
                .registry()
                .bridge(&installed.bridge_id)
                .unwrap()
                .health,
            Health::Online
        );
    }
}
