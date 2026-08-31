//! Read-only Modbus TCP telemetry integration for D23.

#![forbid(unsafe_code)]

use modbus_protocol::{
    decode_read_device_identification_response, decode_read_response,
    encode_read_device_identification_request, encode_read_request, ModbusError,
    ReadRegistersRequest, RegisterTable, MAX_ADU_BYTES, MAX_BASIC_DEVICE_IDENTIFICATION_PAGES,
};
use smart_home_core::{
    AgentId, Bridge, BridgeId, BridgeTransport, Capability, CapabilityId, CapabilityMode, Device,
    DeviceId, Entity, EntityId, EntityKind, Health, IntegrationId, Metadata, ProtocolFamily,
    ProtocolIdentifier, SmartHomeTool, StateConfidence, StateSnapshot, StateSource, Value,
    ValueKind,
};
use smart_home_runtime::{RuntimeError, SmartHomeRuntime};
use std::fmt;
use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::time::Duration;
use tcp_client::{connect, ConnectOptions, TcpConnection, TcpError};

pub const VERSION: &str = "0.1.0";
pub const INTEGRATION_ID: &str = "modbus_tcp";
pub const PROTOCOL_ID: &str = "modbus_tcp";
pub const DEFAULT_PORT: u16 = 502;
pub const MAX_PROFILE_POINTS: usize = 64;

#[derive(Debug)]
pub enum ModbusIntegrationError {
    Validation(String),
    Protocol(ModbusError),
    Tcp(TcpError),
    InvalidResponseLength(u16),
    IncompleteDeviceIdentification,
    DeviceIdentificationPageLimit,
    DeviceIdentificationConformityChanged { expected: u8, actual: u8 },
    NonFiniteMeasurement { point_id: String },
    Runtime(RuntimeError),
}

impl fmt::Display for ModbusIntegrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => write!(formatter, "invalid Modbus input: {message}"),
            Self::Protocol(error) => error.fmt(formatter),
            Self::Tcp(error) => error.fmt(formatter),
            Self::InvalidResponseLength(length) => write!(
                formatter,
                "Modbus TCP response declares invalid MBAP length {length}"
            ),
            Self::IncompleteDeviceIdentification => formatter.write_str(
                "Modbus basic device identification must contain vendor, product code, and revision",
            ),
            Self::DeviceIdentificationPageLimit => write!(
                formatter,
                "Modbus basic device identification exceeded the {MAX_BASIC_DEVICE_IDENTIFICATION_PAGES}-page limit"
            ),
            Self::DeviceIdentificationConformityChanged { expected, actual } => write!(
                formatter,
                "Modbus device identification conformity changed from 0x{expected:02x} to 0x{actual:02x}"
            ),
            Self::NonFiniteMeasurement { point_id } => {
                write!(
                    formatter,
                    "Modbus point `{point_id}` produced a non-finite value"
                )
            }
            Self::Runtime(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for ModbusIntegrationError {}

impl From<ModbusError> for ModbusIntegrationError {
    fn from(error: ModbusError) -> Self {
        Self::Protocol(error)
    }
}

impl From<TcpError> for ModbusIntegrationError {
    fn from(error: TcpError) -> Self {
        Self::Tcp(error)
    }
}

impl From<RuntimeError> for ModbusIntegrationError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterEncoding {
    Unsigned16,
    Signed16,
    Unsigned32,
    Signed32,
    Float32,
}

impl RegisterEncoding {
    pub const fn register_count(self) -> u16 {
        match self {
            Self::Unsigned16 | Self::Signed16 => 1,
            Self::Unsigned32 | Self::Signed32 | Self::Float32 => 2,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unsigned16 => "u16",
            Self::Signed16 => "i16",
            Self::Unsigned32 => "u32",
            Self::Signed32 => "i32",
            Self::Float32 => "f32",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WordOrder {
    HighWordFirst,
    LowWordFirst,
}

impl WordOrder {
    const fn as_str(self) -> &'static str {
        match self {
            Self::HighWordFirst => "high_word_first",
            Self::LowWordFirst => "low_word_first",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModbusPoint {
    pub id: String,
    pub name: String,
    pub table: RegisterTable,
    pub address: u16,
    pub encoding: RegisterEncoding,
    pub word_order: WordOrder,
    pub scale: f64,
    pub offset: f64,
    pub unit: String,
}

impl ModbusPoint {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        table: RegisterTable,
        address: u16,
        encoding: RegisterEncoding,
        unit: impl Into<String>,
    ) -> Result<Self, ModbusIntegrationError> {
        let id = id.into();
        let name = name.into();
        let unit = unit.into();
        if stable_component(&id).is_empty() {
            return Err(ModbusIntegrationError::Validation(
                "point id must contain an ASCII letter or digit".to_string(),
            ));
        }
        if name.trim().is_empty() {
            return Err(ModbusIntegrationError::Validation(
                "point name must not be empty".to_string(),
            ));
        }
        ReadRegistersRequest::new(table, address, encoding.register_count())?;
        Ok(Self {
            id,
            name,
            table,
            address,
            encoding,
            word_order: WordOrder::HighWordFirst,
            scale: 1.0,
            offset: 0.0,
            unit,
        })
    }

    pub fn with_transform(
        mut self,
        scale: f64,
        offset: f64,
    ) -> Result<Self, ModbusIntegrationError> {
        if !scale.is_finite() || !offset.is_finite() {
            return Err(ModbusIntegrationError::Validation(
                "point scale and offset must be finite".to_string(),
            ));
        }
        self.scale = scale;
        self.offset = offset;
        Ok(self)
    }

    pub fn with_word_order(mut self, word_order: WordOrder) -> Self {
        self.word_order = word_order;
        self
    }

    fn request(&self) -> Result<ReadRegistersRequest, ModbusIntegrationError> {
        Ok(ReadRegistersRequest::new(
            self.table,
            self.address,
            self.encoding.register_count(),
        )?)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModbusTcpConfig {
    pub bridge_id: BridgeId,
    pub host: String,
    pub port: u16,
    pub unit_id: u8,
    pub display_name: String,
    pub manufacturer: String,
    pub model: String,
    pub timeout: Duration,
    pub points: Vec<ModbusPoint>,
}

impl ModbusTcpConfig {
    pub fn new(
        bridge_id: BridgeId,
        host: impl Into<String>,
        unit_id: u8,
        points: Vec<ModbusPoint>,
    ) -> Result<Self, ModbusIntegrationError> {
        let host = host.into();
        validate_host(&host)?;
        if points.is_empty() || points.len() > MAX_PROFILE_POINTS {
            return Err(ModbusIntegrationError::Validation(format!(
                "profile must contain between 1 and {MAX_PROFILE_POINTS} points"
            )));
        }
        let mut ids = points
            .iter()
            .map(|point| stable_component(&point.id))
            .collect::<Vec<_>>();
        ids.sort();
        if ids.windows(2).any(|window| window[0] == window[1]) {
            return Err(ModbusIntegrationError::Validation(
                "point ids must be unique after normalization".to_string(),
            ));
        }
        Ok(Self {
            bridge_id,
            host,
            port: DEFAULT_PORT,
            unit_id,
            display_name: format!("Modbus Unit {unit_id}"),
            manufacturer: "Modbus".to_string(),
            model: "TCP device".to_string(),
            timeout: Duration::from_secs(5),
            points,
        })
    }

    pub fn with_port(mut self, port: u16) -> Result<Self, ModbusIntegrationError> {
        if port == 0 {
            return Err(ModbusIntegrationError::Validation(
                "port must be non-zero".to_string(),
            ));
        }
        self.port = port;
        Ok(self)
    }

    pub fn with_display_name(mut self, display_name: impl Into<String>) -> Self {
        let display_name = display_name.into();
        if !display_name.trim().is_empty() {
            self.display_name = display_name;
        }
        self
    }

    pub fn with_device_identity(
        mut self,
        manufacturer: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        let manufacturer = manufacturer.into();
        let model = model.into();
        if !manufacturer.trim().is_empty() {
            self.manufacturer = manufacturer;
        }
        if !model.trim().is_empty() {
            self.model = model;
        }
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout.max(Duration::from_millis(1));
        self
    }

    fn endpoint(&self) -> String {
        SocketAddr::new(
            self.host
                .parse::<IpAddr>()
                .expect("validated Modbus host must remain an IP literal"),
            self.port,
        )
        .to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModbusReadPlan<'a> {
    pub host: &'a str,
    pub port: u16,
    pub timeout: Duration,
    pub transaction_id: u16,
    pub unit_id: u8,
    pub request: ReadRegistersRequest,
}

pub trait ModbusTransport {
    fn read_registers(
        &mut self,
        plan: ModbusReadPlan<'_>,
    ) -> Result<Vec<u16>, ModbusIntegrationError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModbusDeviceIdentityPlan<'a> {
    pub host: &'a str,
    pub port: u16,
    pub timeout: Duration,
    pub first_transaction_id: u16,
    pub unit_id: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModbusDeviceIdentity {
    pub vendor_name: String,
    pub product_code: String,
    pub revision: String,
    pub conformity_level: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModbusDeviceIdentityRead {
    pub identity: ModbusDeviceIdentity,
    pub transaction_count: u16,
}

pub trait ModbusDeviceIdentityTransport {
    fn read_device_identity(
        &mut self,
        plan: ModbusDeviceIdentityPlan<'_>,
    ) -> Result<ModbusDeviceIdentityRead, ModbusIntegrationError>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ModbusTcpTransport;

impl ModbusTransport for ModbusTcpTransport {
    fn read_registers(
        &mut self,
        plan: ModbusReadPlan<'_>,
    ) -> Result<Vec<u16>, ModbusIntegrationError> {
        let options = ConnectOptions {
            connect_timeout: plan.timeout,
            read_timeout: Some(plan.timeout),
            write_timeout: Some(plan.timeout),
            ..ConnectOptions::default()
        };
        let mut connection = connect(plan.host, plan.port, options)?;
        connection.write_all(&encode_read_request(
            plan.transaction_id,
            plan.unit_id,
            plan.request,
        ))?;
        connection.flush()?;

        let response = read_adu(&mut connection)?;
        Ok(decode_read_response(
            &response,
            plan.transaction_id,
            plan.unit_id,
            plan.request,
        )?)
    }
}

impl ModbusDeviceIdentityTransport for ModbusTcpTransport {
    fn read_device_identity(
        &mut self,
        plan: ModbusDeviceIdentityPlan<'_>,
    ) -> Result<ModbusDeviceIdentityRead, ModbusIntegrationError> {
        let options = ConnectOptions {
            connect_timeout: plan.timeout,
            read_timeout: Some(plan.timeout),
            write_timeout: Some(plan.timeout),
            ..ConnectOptions::default()
        };
        let mut connection = connect(plan.host, plan.port, options)?;
        let mut starting_object_id = 0;
        let mut objects = Vec::with_capacity(3);
        let mut conformity_level = None;

        for page_index in 0..MAX_BASIC_DEVICE_IDENTIFICATION_PAGES {
            let transaction_id = plan.first_transaction_id.wrapping_add(page_index as u16);
            connection.write_all(&encode_read_device_identification_request(
                transaction_id,
                plan.unit_id,
                starting_object_id,
            )?)?;
            connection.flush()?;
            let response = read_adu(&mut connection)?;
            let page = decode_read_device_identification_response(
                &response,
                transaction_id,
                plan.unit_id,
                starting_object_id,
            )?;
            if let Some(expected) = conformity_level {
                if page.conformity_level != expected {
                    return Err(
                        ModbusIntegrationError::DeviceIdentificationConformityChanged {
                            expected,
                            actual: page.conformity_level,
                        },
                    );
                }
            } else {
                conformity_level = Some(page.conformity_level);
            }
            objects.extend(page.objects);
            if !page.more_follows {
                if objects.len() != 3 {
                    return Err(ModbusIntegrationError::IncompleteDeviceIdentification);
                }
                return Ok(ModbusDeviceIdentityRead {
                    identity: ModbusDeviceIdentity {
                        vendor_name: objects[0].value.clone(),
                        product_code: objects[1].value.clone(),
                        revision: objects[2].value.clone(),
                        conformity_level: conformity_level
                            .expect("a completed identity has at least one page"),
                    },
                    transaction_count: page_index as u16 + 1,
                });
            }
            starting_object_id = page.next_object_id;
        }
        Err(ModbusIntegrationError::DeviceIdentificationPageLimit)
    }
}

fn read_adu(connection: &mut TcpConnection) -> Result<Vec<u8>, ModbusIntegrationError> {
    let mut response = connection.read_exact(7)?;
    let declared = u16::from_be_bytes([response[4], response[5]]);
    let total = 6usize + usize::from(declared);
    if declared < 3 || total > MAX_ADU_BYTES {
        return Err(ModbusIntegrationError::InvalidResponseLength(declared));
    }
    response.extend_from_slice(&connection.read_exact(usize::from(declared) - 1)?);
    Ok(response)
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModbusMeasurement {
    pub point_id: String,
    pub name: String,
    pub value: f64,
    pub unit: String,
    pub table: RegisterTable,
    pub address: u16,
    pub encoding: RegisterEncoding,
    pub word_order: WordOrder,
    pub scale: f64,
    pub offset: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModbusSnapshot {
    pub unit_id: u8,
    pub device_identity: Option<ModbusDeviceIdentity>,
    pub measurements: Vec<ModbusMeasurement>,
}

pub struct ModbusClient<T> {
    config: ModbusTcpConfig,
    transport: T,
    next_transaction_id: u16,
}

impl<T: ModbusTransport> ModbusClient<T> {
    pub fn new(config: ModbusTcpConfig, transport: T) -> Self {
        Self {
            config,
            transport,
            next_transaction_id: 1,
        }
    }

    pub fn config(&self) -> &ModbusTcpConfig {
        &self.config
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    pub fn inspect(&mut self) -> Result<ModbusSnapshot, ModbusIntegrationError> {
        let mut measurements = Vec::with_capacity(self.config.points.len());
        for point in &self.config.points {
            let transaction_id = self.next_transaction_id;
            self.next_transaction_id = self.next_transaction_id.wrapping_add(1);
            let registers = self.transport.read_registers(ModbusReadPlan {
                host: &self.config.host,
                port: self.config.port,
                timeout: self.config.timeout,
                transaction_id,
                unit_id: self.config.unit_id,
                request: point.request()?,
            })?;
            let value = decode_measurement(point, &registers)?;
            measurements.push(ModbusMeasurement {
                point_id: point.id.clone(),
                name: point.name.clone(),
                value,
                unit: point.unit.clone(),
                table: point.table,
                address: point.address,
                encoding: point.encoding,
                word_order: point.word_order,
                scale: point.scale,
                offset: point.offset,
            });
        }
        Ok(ModbusSnapshot {
            unit_id: self.config.unit_id,
            device_identity: None,
            measurements,
        })
    }
}

impl<T: ModbusTransport + ModbusDeviceIdentityTransport> ModbusClient<T> {
    pub fn inspect_with_device_identity(
        &mut self,
    ) -> Result<ModbusSnapshot, ModbusIntegrationError> {
        let identity_read = self
            .transport
            .read_device_identity(ModbusDeviceIdentityPlan {
                host: &self.config.host,
                port: self.config.port,
                timeout: self.config.timeout,
                first_transaction_id: self.next_transaction_id,
                unit_id: self.config.unit_id,
            })?;
        if identity_read.transaction_count == 0
            || usize::from(identity_read.transaction_count) > MAX_BASIC_DEVICE_IDENTIFICATION_PAGES
        {
            return Err(ModbusIntegrationError::Validation(
                "device identification transaction count must be between 1 and 3".to_string(),
            ));
        }
        validate_device_identity(&identity_read.identity)?;
        self.next_transaction_id = self
            .next_transaction_id
            .wrapping_add(identity_read.transaction_count);
        let mut snapshot = self.inspect()?;
        snapshot.device_identity = Some(identity_read.identity);
        Ok(snapshot)
    }
}

impl<T> fmt::Debug for ModbusClient<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ModbusClient")
            .field("config", &self.config)
            .field("next_transaction_id", &self.next_transaction_id)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledModbusDevice {
    pub bridge_id: BridgeId,
    pub device_id: DeviceId,
    pub entity_ids: Vec<EntityId>,
}

pub struct ModbusRuntimeIntegration<T> {
    client: ModbusClient<T>,
}

impl<T: ModbusTransport> ModbusRuntimeIntegration<T> {
    pub fn new(client: ModbusClient<T>) -> Self {
        Self { client }
    }

    pub fn inspect_and_install_authorized(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        principal_id: AgentId,
        observed_at_ms: u64,
    ) -> Result<InstalledModbusDevice, ModbusIntegrationError> {
        authorize_read(runtime, principal_id, observed_at_ms)?;
        let snapshot = self.client.inspect()?;
        self.install_snapshot(runtime, &snapshot, observed_at_ms)
    }

    pub fn install_snapshot(
        &self,
        runtime: &mut SmartHomeRuntime,
        snapshot: &ModbusSnapshot,
        observed_at_ms: u64,
    ) -> Result<InstalledModbusDevice, ModbusIntegrationError> {
        if let Some(identity) = &snapshot.device_identity {
            validate_device_identity(identity)?;
        }
        let profile_matches = snapshot
            .measurements
            .iter()
            .zip(&self.client.config.points)
            .all(|(measurement, point)| measurement_matches_point(measurement, point));
        if snapshot.unit_id != self.client.config.unit_id
            || snapshot.measurements.len() != self.client.config.points.len()
            || !profile_matches
        {
            return Err(ModbusIntegrationError::Validation(
                "snapshot does not match the configured unit and point profile".to_string(),
            ));
        }
        let endpoint = self.client.config.endpoint();
        let native_id = format!(
            "{}-port-{}-unit-{}",
            stable_component(&self.client.config.host),
            self.client.config.port,
            snapshot.unit_id
        );
        let bridge_id = self.client.config.bridge_id.clone();
        let device_id = DeviceId::trusted(format!("modbus:{native_id}"));

        let mut bridge = Bridge::new(
            bridge_id.clone(),
            IntegrationId::trusted(INTEGRATION_ID),
            BridgeTransport::LanTcp,
        );
        bridge.address = Some(endpoint.clone());
        bridge.hardware_model = Some(snapshot.device_identity.as_ref().map_or_else(
            || "Modbus TCP endpoint".to_string(),
            |identity| identity.product_code.clone(),
        ));
        bridge.health = Health::Online;
        bridge.last_seen_at_ms = Some(observed_at_ms);
        bridge.identifiers = vec![protocol_identifier("tcp_endpoint", &endpoint)?];
        bridge.metadata = vec![
            Metadata::new("modbus.transport", "tcp"),
            Metadata::new("modbus.access", "read_only"),
        ];
        runtime.upsert_bridge(bridge)?;

        let entities = snapshot
            .measurements
            .iter()
            .map(|measurement| {
                let entity_id = EntityId::trusted(format!(
                    "{}:sensor:{}",
                    device_id.as_str(),
                    stable_component(&measurement.point_id)
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
                        Metadata::new("modbus.point_id", measurement.point_id.clone()),
                        Metadata::new("modbus.table", measurement.table.as_str()),
                        Metadata::new("modbus.address", measurement.address.to_string()),
                        Metadata::new("modbus.encoding", measurement.encoding.as_str()),
                        Metadata::new("modbus.word_order", measurement.word_order.as_str()),
                        Metadata::new("modbus.scale", measurement.scale.to_string()),
                        Metadata::new("modbus.offset", measurement.offset.to_string()),
                        Metadata::new("modbus.unit", measurement.unit.clone()),
                    ],
                }
            })
            .collect::<Vec<_>>();
        let entity_ids = entities
            .iter()
            .map(|entity| entity.entity_id.clone())
            .collect::<Vec<_>>();
        let identity = snapshot.device_identity.as_ref();
        let mut device_metadata = vec![
            Metadata::new("modbus.endpoint", endpoint.clone()),
            Metadata::new("modbus.profile_points", entity_ids.len().to_string()),
        ];
        if let Some(identity) = identity {
            device_metadata.push(Metadata::new("modbus.device_identification", "basic"));
            device_metadata.push(Metadata::new(
                "modbus.device_identification_conformity",
                format!("0x{:02x}", identity.conformity_level),
            ));
        }
        runtime.upsert_device(Device {
            device_id: device_id.clone(),
            bridge_id: bridge_id.clone(),
            manufacturer: identity.map_or_else(
                || self.client.config.manufacturer.clone(),
                |identity| identity.vendor_name.clone(),
            ),
            model: identity.map_or_else(
                || self.client.config.model.clone(),
                |identity| identity.product_code.clone(),
            ),
            name: self.client.config.display_name.clone(),
            serial: None,
            firmware_version: identity.map(|identity| identity.revision.clone()),
            room_id: None,
            entity_ids: entity_ids.clone(),
            identifiers: vec![protocol_identifier(
                "tcp_endpoint_unit",
                &format!("{endpoint}/{}", snapshot.unit_id),
            )?],
            health: Health::Online,
            metadata: device_metadata,
        })?;
        for entity in entities {
            runtime.upsert_entity(entity)?;
        }
        Ok(InstalledModbusDevice {
            bridge_id,
            device_id,
            entity_ids,
        })
    }
}

impl<T: ModbusTransport + ModbusDeviceIdentityTransport> ModbusRuntimeIntegration<T> {
    pub fn inspect_with_device_identity_and_install_authorized(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        principal_id: AgentId,
        observed_at_ms: u64,
    ) -> Result<InstalledModbusDevice, ModbusIntegrationError> {
        authorize_read(runtime, principal_id, observed_at_ms)?;
        let snapshot = self.client.inspect_with_device_identity()?;
        self.install_snapshot(runtime, &snapshot, observed_at_ms)
    }
}

fn measurement_matches_point(measurement: &ModbusMeasurement, point: &ModbusPoint) -> bool {
    measurement.point_id == point.id
        && measurement.name == point.name
        && measurement.table == point.table
        && measurement.address == point.address
        && measurement.encoding == point.encoding
        && measurement.word_order == point.word_order
        && measurement.scale == point.scale
        && measurement.offset == point.offset
        && measurement.unit == point.unit
        && measurement.value.is_finite()
}

fn validate_device_identity(identity: &ModbusDeviceIdentity) -> Result<(), ModbusIntegrationError> {
    for (label, value) in [
        ("vendor name", &identity.vendor_name),
        ("product code", &identity.product_code),
        ("revision", &identity.revision),
    ] {
        if value.is_empty()
            || value.len() > modbus_protocol::MAX_DEVICE_IDENTIFICATION_VALUE_BYTES
            || !value.bytes().all(|byte| matches!(byte, 0x20..=0x7e))
        {
            return Err(ModbusIntegrationError::Validation(format!(
                "device identification {label} must contain 1 to 128 printable ASCII bytes"
            )));
        }
    }
    if !matches!(identity.conformity_level, 0x01..=0x03 | 0x81..=0x83) {
        return Err(ModbusIntegrationError::Validation(
            "device identification conformity level is invalid".to_string(),
        ));
    }
    Ok(())
}

fn authorize_read(
    runtime: &mut SmartHomeRuntime,
    principal_id: AgentId,
    now_ms: u64,
) -> Result<(), ModbusIntegrationError> {
    let tool = SmartHomeTool::GetState;
    let decision = runtime.authorize_tool_for_principal(principal_id.clone(), tool, now_ms);
    if decision.missing_capabilities.is_empty() {
        Ok(())
    } else {
        Err(ModbusIntegrationError::Runtime(
            RuntimeError::UnauthorizedTool {
                principal_id,
                tool,
                missing_capabilities: decision.missing_capabilities,
            },
        ))
    }
}

fn decode_measurement(
    point: &ModbusPoint,
    registers: &[u16],
) -> Result<f64, ModbusIntegrationError> {
    if registers.len() != usize::from(point.encoding.register_count()) {
        return Err(ModbusIntegrationError::Validation(format!(
            "point `{}` expected {} registers, got {}",
            point.id,
            point.encoding.register_count(),
            registers.len()
        )));
    }
    let raw = match point.encoding {
        RegisterEncoding::Unsigned16 => f64::from(registers[0]),
        RegisterEncoding::Signed16 => f64::from(registers[0] as i16),
        RegisterEncoding::Unsigned32 => f64::from(join_words(registers, point.word_order)),
        RegisterEncoding::Signed32 => f64::from(join_words(registers, point.word_order) as i32),
        RegisterEncoding::Float32 => {
            f64::from(f32::from_bits(join_words(registers, point.word_order)))
        }
    };
    let value = raw.mul_add(point.scale, point.offset);
    if value.is_finite() {
        Ok(value)
    } else {
        Err(ModbusIntegrationError::NonFiniteMeasurement {
            point_id: point.id.clone(),
        })
    }
}

fn join_words(registers: &[u16], word_order: WordOrder) -> u32 {
    let (high, low) = match word_order {
        WordOrder::HighWordFirst => (registers[0], registers[1]),
        WordOrder::LowWordFirst => (registers[1], registers[0]),
    };
    (u32::from(high) << 16) | u32::from(low)
}

fn measurement_value(measurement: &ModbusMeasurement) -> Value {
    Value::Object(vec![
        ("value".to_string(), Value::Number(measurement.value)),
        ("unit".to_string(), Value::Text(measurement.unit.clone())),
    ])
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

fn protocol_identifier(
    kind: &str,
    value: &str,
) -> Result<ProtocolIdentifier, ModbusIntegrationError> {
    ProtocolIdentifier::new(ProtocolFamily::Vendor(PROTOCOL_ID.to_string()), kind, value)
        .map_err(|error| ModbusIntegrationError::Validation(error.to_string()))
}

fn validate_host(host: &str) -> Result<(), ModbusIntegrationError> {
    let address = host.parse::<IpAddr>().map_err(|_| {
        ModbusIntegrationError::Validation(
            "host must be an explicit private, link-local, or loopback IP literal".to_string(),
        )
    })?;
    if is_local_ip(address) {
        Ok(())
    } else {
        Err(ModbusIntegrationError::Validation(
            "host must be an explicit private, link-local, or loopback IP literal".to_string(),
        ))
    }
}

fn is_local_ip(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => {
            address.is_private() || address.is_link_local() || address.is_loopback()
        }
        IpAddr::V6(address) => {
            address.is_loopback() || address.is_unique_local() || is_ipv6_link_local(address)
        }
    }
}

fn is_ipv6_link_local(address: Ipv6Addr) -> bool {
    (address.segments()[0] & 0xffc0) == 0xfe80
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

#[cfg(test)]
mod tests {
    use super::*;
    use smart_home_core::{CapabilityGrant, CapabilityGrantId, PrivilegeTier};
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::thread;

    fn point(
        id: &str,
        table: RegisterTable,
        address: u16,
        encoding: RegisterEncoding,
        unit: &str,
    ) -> ModbusPoint {
        ModbusPoint::new(id, id.replace('-', " "), table, address, encoding, unit).unwrap()
    }

    fn config(port: u16) -> ModbusTcpConfig {
        ModbusTcpConfig::new(
            BridgeId::trusted("modbus.test"),
            "127.0.0.1",
            7,
            vec![
                point(
                    "line-voltage",
                    RegisterTable::Input,
                    10,
                    RegisterEncoding::Unsigned16,
                    "V",
                )
                .with_transform(0.1, 0.0)
                .unwrap(),
                point(
                    "active-power",
                    RegisterTable::Holding,
                    20,
                    RegisterEncoding::Float32,
                    "W",
                ),
            ],
        )
        .unwrap()
        .with_port(port)
        .unwrap()
        .with_display_name("Plant Meter")
        .with_device_identity("Acme", "PM-1")
    }

    fn grant(runtime: &mut SmartHomeRuntime, principal: &AgentId) {
        let _ = runtime.registry_mut().upsert_capability_grant(
            CapabilityGrant::for_all_smart_home(
                CapabilityGrantId::trusted("grant:modbus-test"),
                principal.clone(),
                PrivilegeTier::LowRisk,
                "test",
                1_000,
            )
            .with_expiry(20_000),
        );
    }

    struct TestServer {
        port: u16,
        requests: Arc<Mutex<Vec<Vec<u8>>>>,
        handle: thread::JoinHandle<()>,
    }

    fn start_server() -> TestServer {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&requests);
        let handle = thread::spawn(move || {
            for register_bytes in [vec![0x08, 0xfc], 123.5f32.to_bits().to_be_bytes().to_vec()] {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = [0u8; 12];
                stream.read_exact(&mut request).unwrap();
                captured.lock().unwrap().push(request.to_vec());
                let mut response = request[..7].to_vec();
                let length = 3 + register_bytes.len();
                response[4..6].copy_from_slice(&(length as u16).to_be_bytes());
                response.push(request[7]);
                response.push(register_bytes.len() as u8);
                response.extend_from_slice(&register_bytes);
                stream.write_all(&response).unwrap();
            }
        });
        TestServer {
            port,
            requests,
            handle,
        }
    }

    fn start_identity_server() -> TestServer {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&requests);
        let handle = thread::spawn(move || {
            let (mut identity_stream, _) = listener.accept().unwrap();
            for response_pdu in [
                vec![
                    0x2b, 0x0e, 0x01, 0x81, 0xff, 2, 2, 0, 4, b'A', b'c', b'm', b'e', 1, 4, b'P',
                    b'M', b'-', b'1',
                ],
                vec![
                    0x2b, 0x0e, 0x01, 0x81, 0, 0, 1, 2, 5, b'1', b'.', b'2', b'.', b'3',
                ],
            ] {
                let mut request = [0u8; 11];
                identity_stream.read_exact(&mut request).unwrap();
                captured.lock().unwrap().push(request.to_vec());
                let mut response = request[..7].to_vec();
                response[4..6].copy_from_slice(
                    &(u16::try_from(response_pdu.len()).unwrap() + 1).to_be_bytes(),
                );
                response.extend_from_slice(&response_pdu);
                identity_stream.write_all(&response).unwrap();
            }
            for register_bytes in [vec![0x08, 0xfc], 123.5f32.to_bits().to_be_bytes().to_vec()] {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = [0u8; 12];
                stream.read_exact(&mut request).unwrap();
                captured.lock().unwrap().push(request.to_vec());
                let mut response = request[..7].to_vec();
                let length = 3 + register_bytes.len();
                response[4..6].copy_from_slice(&(length as u16).to_be_bytes());
                response.push(request[7]);
                response.push(register_bytes.len() as u8);
                response.extend_from_slice(&register_bytes);
                stream.write_all(&response).unwrap();
            }
        });
        TestServer {
            port,
            requests,
            handle,
        }
    }

    #[test]
    fn real_tcp_poll_installs_normalized_sensors() {
        let server = start_server();
        let client = ModbusClient::new(config(server.port), ModbusTcpTransport);
        let mut integration = ModbusRuntimeIntegration::new(client);
        let principal = AgentId::trusted("agent:modbus-read");
        let mut runtime = SmartHomeRuntime::new();
        grant(&mut runtime, &principal);
        let installed = integration
            .inspect_and_install_authorized(&mut runtime, principal, 5_000)
            .unwrap();
        server.handle.join().unwrap();

        assert_eq!(installed.entity_ids.len(), 2);
        let voltage = runtime
            .registry()
            .entity(&EntityId::trusted(format!(
                "{}:sensor:line-voltage",
                installed.device_id.as_str()
            )))
            .unwrap();
        assert_eq!(voltage.kind, EntityKind::Sensor);
        assert_eq!(voltage.name, "Plant Meter line voltage");
        assert_eq!(
            voltage.state.as_ref().unwrap().value,
            Value::Object(vec![
                ("value".to_string(), Value::Number(230.0)),
                ("unit".to_string(), Value::Text("V".to_string())),
            ])
        );
        let device = runtime.registry().device(&installed.device_id).unwrap();
        assert_eq!(device.manufacturer, "Acme");
        assert_eq!(device.model, "PM-1");
        let requests = server.requests.lock().unwrap();
        assert_eq!(requests[0][7], RegisterTable::Input.function_code());
        assert_eq!(requests[1][7], RegisterTable::Holding.function_code());
        assert_eq!(&requests[0][8..12], &[0, 10, 0, 1]);
        assert_eq!(&requests[1][8..12], &[0, 20, 0, 2]);
    }

    #[test]
    fn real_tcp_identity_poll_installs_native_device_identity() {
        let server = start_identity_server();
        let client = ModbusClient::new(config(server.port), ModbusTcpTransport);
        let mut integration = ModbusRuntimeIntegration::new(client);
        let principal = AgentId::trusted("agent:modbus-identity-read");
        let mut runtime = SmartHomeRuntime::new();
        grant(&mut runtime, &principal);
        let installed = integration
            .inspect_with_device_identity_and_install_authorized(&mut runtime, principal, 5_000)
            .unwrap();
        server.handle.join().unwrap();

        let device = runtime.registry().device(&installed.device_id).unwrap();
        assert_eq!(device.manufacturer, "Acme");
        assert_eq!(device.model, "PM-1");
        assert_eq!(device.firmware_version.as_deref(), Some("1.2.3"));
        let requests = server.requests.lock().unwrap();
        assert_eq!(requests.len(), 4);
        assert_eq!(&requests[0], &[0, 1, 0, 0, 0, 5, 7, 0x2b, 0x0e, 1, 0]);
        assert_eq!(&requests[1], &[0, 2, 0, 0, 0, 5, 7, 0x2b, 0x0e, 1, 2]);
        assert_eq!(requests[2][7], RegisterTable::Input.function_code());
        assert_eq!(requests[3][7], RegisterTable::Holding.function_code());
        assert_eq!(&requests[2][..2], &[0, 3]);
        assert_eq!(&requests[3][..2], &[0, 4]);
    }

    #[derive(Debug)]
    struct CountingTransport(Arc<AtomicUsize>);

    impl ModbusTransport for CountingTransport {
        fn read_registers(
            &mut self,
            _plan: ModbusReadPlan<'_>,
        ) -> Result<Vec<u16>, ModbusIntegrationError> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(vec![0])
        }
    }

    impl ModbusDeviceIdentityTransport for CountingTransport {
        fn read_device_identity(
            &mut self,
            _plan: ModbusDeviceIdentityPlan<'_>,
        ) -> Result<ModbusDeviceIdentityRead, ModbusIntegrationError> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(ModbusDeviceIdentityRead {
                identity: ModbusDeviceIdentity {
                    vendor_name: "Acme".to_string(),
                    product_code: "PM-1".to_string(),
                    revision: "1.2.3".to_string(),
                    conformity_level: 1,
                },
                transaction_count: 1,
            })
        }
    }

    #[test]
    fn denied_read_reaches_no_transport() {
        let calls = Arc::new(AtomicUsize::new(0));
        let client = ModbusClient::new(config(DEFAULT_PORT), CountingTransport(Arc::clone(&calls)));
        let mut integration = ModbusRuntimeIntegration::new(client);
        assert!(matches!(
            integration.inspect_and_install_authorized(
                &mut SmartHomeRuntime::new(),
                AgentId::trusted("agent:denied"),
                5_000,
            ),
            Err(ModbusIntegrationError::Runtime(_))
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn denied_identity_read_reaches_no_transport() {
        let calls = Arc::new(AtomicUsize::new(0));
        let client = ModbusClient::new(config(DEFAULT_PORT), CountingTransport(Arc::clone(&calls)));
        let mut integration = ModbusRuntimeIntegration::new(client);
        assert!(matches!(
            integration.inspect_with_device_identity_and_install_authorized(
                &mut SmartHomeRuntime::new(),
                AgentId::trusted("agent:identity-denied"),
                5_000,
            ),
            Err(ModbusIntegrationError::Runtime(_))
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn mismatched_snapshot_is_rejected_before_runtime_mutation() {
        let client = ModbusClient::new(
            config(DEFAULT_PORT),
            CountingTransport(Arc::new(AtomicUsize::new(0))),
        );
        let integration = ModbusRuntimeIntegration::new(client);
        let snapshot = ModbusSnapshot {
            unit_id: 7,
            device_identity: None,
            measurements: vec![
                ModbusMeasurement {
                    point_id: "different-point".to_string(),
                    name: "line voltage".to_string(),
                    value: 230.0,
                    unit: "V".to_string(),
                    table: RegisterTable::Input,
                    address: 10,
                    encoding: RegisterEncoding::Unsigned16,
                    word_order: WordOrder::HighWordFirst,
                    scale: 0.1,
                    offset: 0.0,
                },
                ModbusMeasurement {
                    point_id: "active-power".to_string(),
                    name: "active power".to_string(),
                    value: 123.5,
                    unit: "W".to_string(),
                    table: RegisterTable::Holding,
                    address: 20,
                    encoding: RegisterEncoding::Float32,
                    word_order: WordOrder::HighWordFirst,
                    scale: 1.0,
                    offset: 0.0,
                },
            ],
        };
        let mut runtime = SmartHomeRuntime::new();
        assert!(matches!(
            integration.install_snapshot(&mut runtime, &snapshot, 5_000),
            Err(ModbusIntegrationError::Validation(_))
        ));
        assert!(runtime
            .registry()
            .bridge(&BridgeId::trusted("modbus.test"))
            .is_none());
    }

    #[test]
    fn validates_profile_bounds_and_duplicate_ids_before_io() {
        assert!(ModbusTcpConfig::new(
            BridgeId::trusted("modbus.empty"),
            "127.0.0.1",
            1,
            Vec::new(),
        )
        .is_err());
        let duplicate = point(
            "line voltage",
            RegisterTable::Input,
            11,
            RegisterEncoding::Unsigned16,
            "V",
        );
        assert!(ModbusTcpConfig::new(
            BridgeId::trusted("modbus.duplicate"),
            "127.0.0.1",
            1,
            vec![
                point(
                    "line-voltage",
                    RegisterTable::Input,
                    10,
                    RegisterEncoding::Unsigned16,
                    "V",
                ),
                duplicate,
            ],
        )
        .is_err());
        assert!(ModbusTcpConfig::new(
            BridgeId::trusted("modbus.dns"),
            "equipment.example.com",
            1,
            vec![point(
                "value",
                RegisterTable::Input,
                0,
                RegisterEncoding::Unsigned16,
                "raw",
            )],
        )
        .is_err());
        assert!(ModbusTcpConfig::new(
            BridgeId::trusted("modbus.public"),
            "8.8.8.8",
            1,
            vec![point(
                "value",
                RegisterTable::Input,
                0,
                RegisterEncoding::Unsigned16,
                "raw",
            )],
        )
        .is_err());
        assert!(ModbusTcpConfig::new(
            BridgeId::trusted("modbus.ipv6-loopback"),
            "::1",
            1,
            vec![point(
                "value",
                RegisterTable::Input,
                0,
                RegisterEncoding::Unsigned16,
                "raw",
            )],
        )
        .is_ok());
        assert!(ModbusTcpConfig::new(
            BridgeId::trusted("modbus.zero-port"),
            "127.0.0.1",
            1,
            vec![point(
                "value",
                RegisterTable::Input,
                0,
                RegisterEncoding::Unsigned16,
                "raw",
            )],
        )
        .unwrap()
        .with_port(0)
        .is_err());
    }

    #[test]
    fn decodes_signed_and_word_swapped_values() {
        let signed = point(
            "temperature",
            RegisterTable::Input,
            0,
            RegisterEncoding::Signed16,
            "C",
        )
        .with_transform(0.1, 0.0)
        .unwrap();
        assert_eq!(decode_measurement(&signed, &[0xff9c]).unwrap(), -10.0);

        let swapped = point(
            "counter",
            RegisterTable::Holding,
            0,
            RegisterEncoding::Unsigned32,
            "count",
        )
        .with_word_order(WordOrder::LowWordFirst);
        assert_eq!(decode_measurement(&swapped, &[2, 1]).unwrap(), 65_538.0);
    }

    #[test]
    fn rejects_non_finite_float_measurements() {
        let float = point(
            "bad-float",
            RegisterTable::Input,
            0,
            RegisterEncoding::Float32,
            "value",
        );
        assert!(matches!(
            decode_measurement(&float, &[0x7fc0, 0]),
            Err(ModbusIntegrationError::NonFiniteMeasurement { .. })
        ));
    }
}
