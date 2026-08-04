//! Production Philips Hue LAN workers for the D23 smart-home runtime.

#![forbid(unsafe_code)]

use std::any::Any;
use std::fmt;
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::Arc;
use std::time::Duration;

use actor::{ActorError, ActorResult, ActorSystem, Message};
use coding_adventures_vault_sealed_store::{SealedStore, SealedStoreError};
use coding_adventures_zeroize::Zeroizing;
use http1::{parse_response_head, Http1ParseError};
use http_core::{BodyKind, Header};
use hue_client::{
    parse_event_stream, parse_state_updates_from_event_batches, HueClient, HueClientConfig,
    HueClientError, HueEventStreamSummary, HueHttpRequest, HueHttpResponse, HueSnapshot,
    HueSnapshotSummary, HueTransport,
};
use hue_core::{
    hue_button_to_entity, hue_device_to_core, hue_entity_id_for_resource_ref, hue_light_to_entity,
    hue_motion_to_entity, HueApplicationCredentials, HueCommand, HueError, HueResourceRef,
    HueResourceType,
};
use smart_home_core::{
    Bridge, BridgeId, CommandResult, CommandStatus, CommandType, DeviceCommand, DeviceEvent,
    DeviceEventType, DeviceId, EntityId, EventId, Health, IntegrationId, Metadata, StateDelta,
    Value, VaultRef,
};
use smart_home_hue_pairing_service::{vault_record_key, HUE_VAULT_NAMESPACE};
use smart_home_local_http::{LocalHttpEndpoint, LocalHttpError, LocalHttpScheme};
use smart_home_runtime::{
    BridgeHealthReport, CommandAuthorization, RuntimeError, RuntimeEvent, SmartHomeRuntime,
};
use tls_platform::{default_connector, TlsConfig, TlsConnector, TlsError};
use url_parser::{Url, UrlError};

pub const WORKER_REQUEST_CONTENT_TYPE: &str = "application/vnd.smart-home.hue-worker-request+json";
pub const DEFAULT_MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
pub const DEFAULT_EVENT_READ_WINDOW_MS: u64 = 2_000;

#[derive(Debug)]
pub enum HueIntegrationError {
    UnknownBridge(BridgeId),
    WrongIntegration(IntegrationId),
    MissingBridgeAddress(BridgeId),
    MissingVaultRef(BridgeId),
    InvalidVaultRef(VaultRef),
    MissingVaultRecord(VaultRef),
    InvalidWorkerRequest(String),
    MissingEntity(EntityId),
    MissingHueResourceMetadata(EntityId),
    UnsupportedCommand(CommandType),
    LocalHttp(LocalHttpError),
    Hue(HueError),
    Client(HueClientError),
    Transport(HueLanTransportError),
    Vault(SealedStoreError),
    Runtime(RuntimeError),
}

impl fmt::Display for HueIntegrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownBridge(bridge_id) => write!(formatter, "unknown Hue bridge {bridge_id}"),
            Self::WrongIntegration(integration_id) => write!(
                formatter,
                "Hue integration cannot serve integration {integration_id}"
            ),
            Self::MissingBridgeAddress(bridge_id) => {
                write!(formatter, "Hue bridge {bridge_id} has no LAN address")
            }
            Self::MissingVaultRef(bridge_id) => {
                write!(
                    formatter,
                    "Hue bridge {bridge_id} has no credential VaultRef"
                )
            }
            Self::InvalidVaultRef(vault_ref) => {
                write!(formatter, "Hue credential reference {vault_ref} is invalid")
            }
            Self::MissingVaultRecord(vault_ref) => {
                write!(
                    formatter,
                    "Hue credential record {vault_ref} does not exist"
                )
            }
            Self::InvalidWorkerRequest(message) => {
                write!(formatter, "invalid Hue worker request: {message}")
            }
            Self::MissingEntity(entity_id) => {
                write!(formatter, "unknown Hue entity {entity_id}")
            }
            Self::MissingHueResourceMetadata(entity_id) => write!(
                formatter,
                "Hue entity {entity_id} is missing native resource metadata"
            ),
            Self::UnsupportedCommand(command_type) => {
                write!(formatter, "Hue command {command_type:?} is unsupported")
            }
            Self::LocalHttp(error) => write!(formatter, "{error}"),
            Self::Hue(error) => write!(formatter, "{error}"),
            Self::Client(error) => write!(formatter, "{error}"),
            Self::Transport(error) => write!(formatter, "{error}"),
            Self::Vault(error) => write!(formatter, "{error}"),
            Self::Runtime(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for HueIntegrationError {}

impl From<LocalHttpError> for HueIntegrationError {
    fn from(error: LocalHttpError) -> Self {
        Self::LocalHttp(error)
    }
}

impl From<HueError> for HueIntegrationError {
    fn from(error: HueError) -> Self {
        Self::Hue(error)
    }
}

impl From<HueClientError> for HueIntegrationError {
    fn from(error: HueClientError) -> Self {
        Self::Client(error)
    }
}

impl From<HueLanTransportError> for HueIntegrationError {
    fn from(error: HueLanTransportError) -> Self {
        Self::Transport(error)
    }
}

impl From<SealedStoreError> for HueIntegrationError {
    fn from(error: SealedStoreError) -> Self {
        Self::Vault(error)
    }
}

impl From<RuntimeError> for HueIntegrationError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

#[derive(Debug)]
pub enum HueLanTransportError {
    Url(UrlError),
    MissingHost,
    UnsupportedScheme(String),
    UnsafeRequest,
    Io(io::Error),
    Tls(TlsError),
    Http(Http1ParseError),
    HttpStatus(u16),
    ResponseTooLarge { limit: usize },
    TruncatedBody { expected: usize, actual: usize },
    InvalidChunkedBody(String),
}

impl fmt::Display for HueLanTransportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Url(error) => write!(formatter, "invalid Hue URL: {error}"),
            Self::MissingHost => write!(formatter, "Hue URL is missing a host or port"),
            Self::UnsupportedScheme(scheme) => {
                write!(formatter, "unsupported Hue URL scheme `{scheme}`")
            }
            Self::UnsafeRequest => write!(formatter, "Hue request contains unsafe HTTP text"),
            Self::Io(error) => write!(formatter, "Hue LAN I/O failed: {error}"),
            Self::Tls(error) => write!(formatter, "Hue LAN TLS failed: {error}"),
            Self::Http(error) => write!(formatter, "Hue HTTP response is invalid: {error}"),
            Self::HttpStatus(status) => write!(formatter, "Hue bridge returned HTTP {status}"),
            Self::ResponseTooLarge { limit } => {
                write!(formatter, "Hue response exceeds {limit} bytes")
            }
            Self::TruncatedBody { expected, actual } => write!(
                formatter,
                "Hue response body is truncated: expected {expected} bytes, got {actual}"
            ),
            Self::InvalidChunkedBody(message) => {
                write!(formatter, "Hue chunked response is invalid: {message}")
            }
        }
    }
}

impl std::error::Error for HueLanTransportError {}

pub struct HueLanTransport {
    endpoint: LocalHttpEndpoint,
    connector: Box<dyn TlsConnector>,
    tls_config: TlsConfig,
    max_response_bytes: usize,
    event_read_window: Duration,
}

impl HueLanTransport {
    pub fn for_bridge(bridge: &Bridge) -> Result<Self, HueIntegrationError> {
        Ok(Self::new(
            endpoint_for_bridge(bridge)?,
            default_connector(),
            TlsConfig::https_default(),
        ))
    }

    pub fn new(
        endpoint: LocalHttpEndpoint,
        connector: Box<dyn TlsConnector>,
        tls_config: TlsConfig,
    ) -> Self {
        Self {
            endpoint,
            connector,
            tls_config,
            max_response_bytes: DEFAULT_MAX_RESPONSE_BYTES,
            event_read_window: Duration::from_millis(DEFAULT_EVENT_READ_WINDOW_MS),
        }
    }

    pub fn with_max_response_bytes(mut self, max_response_bytes: usize) -> Self {
        self.max_response_bytes = max_response_bytes.max(1);
        self
    }

    pub fn with_event_read_window(mut self, event_read_window: Duration) -> Self {
        self.event_read_window = event_read_window.max(Duration::from_millis(1));
        self
    }
}

impl HueTransport for HueLanTransport {
    fn send(&mut self, request: HueHttpRequest) -> Result<HueHttpResponse, HueClientError> {
        self.execute(request)
            .map_err(|error| HueClientError::transport(error.to_string()))
    }
}

impl HueLanTransport {
    fn execute(
        &mut self,
        request: HueHttpRequest,
    ) -> Result<HueHttpResponse, HueLanTransportError> {
        let url = Url::parse(
            &self
                .endpoint
                .url_for_path(&request.path)
                .map_err(|error| HueLanTransportError::Io(io::Error::other(error.to_string())))?,
        )
        .map_err(HueLanTransportError::Url)?;
        let host = url
            .host
            .as_deref()
            .ok_or(HueLanTransportError::MissingHost)?;
        let port = url
            .effective_port()
            .ok_or(HueLanTransportError::MissingHost)?;
        let event_stream = request.path == hue_core::CLIP_V2_EVENT_STREAM_PATH;
        let request_bytes = Zeroizing::new(encode_http_request(&url, &request)?);
        let timeout = if event_stream {
            self.event_read_window
        } else {
            Duration::from_millis(5_000)
        };

        let response_bytes = match url.scheme.as_str() {
            "http" => {
                let mut stream = connect_tcp(host, port, timeout)?;
                stream
                    .write_all(&request_bytes)
                    .map_err(HueLanTransportError::Io)?;
                stream.flush().map_err(HueLanTransportError::Io)?;
                read_bounded(&mut stream, self.max_response_bytes, event_stream)?
            }
            "https" => {
                let mut config = self.tls_config.clone();
                config.connect_timeout = timeout;
                config.read_timeout = Some(timeout);
                config.write_timeout = Some(timeout);
                if config.server_name.is_none() {
                    config.server_name = self.endpoint.tls_name.clone();
                }
                let mut stream = self
                    .connector
                    .connect(host, port, &config)
                    .map_err(HueLanTransportError::Tls)?;
                stream
                    .write_all(&request_bytes)
                    .map_err(HueLanTransportError::Io)?;
                stream.flush().map_err(HueLanTransportError::Io)?;
                let bytes = read_bounded(&mut stream, self.max_response_bytes, event_stream)?;
                stream.close_notify().map_err(HueLanTransportError::Tls)?;
                bytes
            }
            scheme => return Err(HueLanTransportError::UnsupportedScheme(scheme.to_string())),
        };
        decode_http_response(&response_bytes, self.max_response_bytes, event_stream)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HueIntegrationSnapshot {
    pub refresh_count: u64,
    pub command_count: u64,
    pub event_poll_count: u64,
    pub failed_count: u64,
    pub event_sequence: u64,
    pub last_success_at_ms: Option<u64>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HueRefreshReport {
    pub bridge_id: BridgeId,
    pub refreshed_at_ms: u64,
    pub snapshot: HueSnapshotSummary,
    pub devices_upserted: usize,
    pub entities_upserted: usize,
    pub scenes_upserted: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HueEventPollReport {
    pub bridge_id: BridgeId,
    pub polled_at_ms: u64,
    pub stream: HueEventStreamSummary,
    pub updates_received: usize,
    pub events_applied: usize,
    pub updates_without_entity: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HueCommandDispatchReport {
    pub accepted: CommandResult,
    pub completed: CommandResult,
}

pub struct HueIntegrationActorState<T> {
    bridge_id: BridgeId,
    runtime: SmartHomeRuntime,
    vault: Arc<SealedStore>,
    transport: T,
    snapshot: HueIntegrationSnapshot,
    last_refresh: Option<HueRefreshReport>,
    last_event_poll: Option<HueEventPollReport>,
    last_command: Option<HueCommandDispatchReport>,
}

impl<T: HueTransport> HueIntegrationActorState<T> {
    pub fn new(
        bridge_id: BridgeId,
        runtime: SmartHomeRuntime,
        vault: Arc<SealedStore>,
        transport: T,
    ) -> Self {
        Self {
            bridge_id,
            runtime,
            vault,
            transport,
            snapshot: HueIntegrationSnapshot::default(),
            last_refresh: None,
            last_event_poll: None,
            last_command: None,
        }
    }

    pub fn runtime(&self) -> &SmartHomeRuntime {
        &self.runtime
    }

    pub fn runtime_mut(&mut self) -> &mut SmartHomeRuntime {
        &mut self.runtime
    }

    pub fn snapshot(&self) -> &HueIntegrationSnapshot {
        &self.snapshot
    }

    pub fn last_refresh(&self) -> Option<&HueRefreshReport> {
        self.last_refresh.as_ref()
    }

    pub fn last_event_poll(&self) -> Option<&HueEventPollReport> {
        self.last_event_poll.as_ref()
    }

    pub fn last_command(&self) -> Option<&HueCommandDispatchReport> {
        self.last_command.as_ref()
    }

    pub fn refresh(&mut self, now_ms: u64) -> Result<&HueRefreshReport, HueIntegrationError> {
        self.snapshot.refresh_count = self.snapshot.refresh_count.saturating_add(1);
        match self.execute_refresh(now_ms) {
            Ok(report) => {
                self.record_success(now_ms);
                self.last_refresh = Some(report);
                Ok(self
                    .last_refresh
                    .as_ref()
                    .expect("refresh report was assigned before returning"))
            }
            Err(error) => {
                self.record_error(&error);
                Err(error)
            }
        }
    }

    pub fn poll_events(&mut self, now_ms: u64) -> Result<&HueEventPollReport, HueIntegrationError> {
        self.snapshot.event_poll_count = self.snapshot.event_poll_count.saturating_add(1);
        match self.execute_event_poll(now_ms) {
            Ok(report) => {
                self.record_success(now_ms);
                self.last_event_poll = Some(report);
                Ok(self
                    .last_event_poll
                    .as_ref()
                    .expect("event poll report was assigned before returning"))
            }
            Err(error) => {
                self.record_error(&error);
                Err(error)
            }
        }
    }

    pub fn dispatch_authorized_command(
        &mut self,
        authorization: &CommandAuthorization,
        command: DeviceCommand,
        now_ms: u64,
    ) -> Result<&HueCommandDispatchReport, HueIntegrationError> {
        self.snapshot.command_count = self.snapshot.command_count.saturating_add(1);
        match self.execute_command(authorization, command, now_ms) {
            Ok(report) => {
                if report.completed.status == CommandStatus::Failed {
                    self.snapshot.failed_count = self.snapshot.failed_count.saturating_add(1);
                    self.snapshot.last_error = report.completed.message.clone();
                } else {
                    self.record_success(now_ms);
                }
                self.last_command = Some(report);
                Ok(self
                    .last_command
                    .as_ref()
                    .expect("command report was assigned before returning"))
            }
            Err(error) => {
                self.record_error(&error);
                Err(error)
            }
        }
    }

    fn record_success(&mut self, now_ms: u64) {
        self.snapshot.last_success_at_ms = Some(now_ms);
        self.snapshot.last_error = None;
    }

    fn record_error(&mut self, error: &HueIntegrationError) {
        self.snapshot.failed_count = self.snapshot.failed_count.saturating_add(1);
        self.snapshot.last_error = Some(error.to_string());
    }

    fn execute_refresh(&mut self, now_ms: u64) -> Result<HueRefreshReport, HueIntegrationError> {
        let credentials = self.load_credentials()?;
        let snapshot = {
            let mut client = HueClient::new(
                HueClientConfig::paired(credentials.application_key.clone()),
                BorrowedTransport(&mut self.transport),
            );
            client.get_snapshot()?
        };
        let summary = snapshot.summary();
        let (devices_upserted, entities_upserted, scenes_upserted) =
            self.apply_snapshot(snapshot, now_ms)?;
        self.runtime.apply_bridge_health(BridgeHealthReport {
            event_id: EventId::trusted(format!("hue.refresh.health:{}:{now_ms}", self.bridge_id)),
            bridge_id: self.bridge_id.clone(),
            health: Health::Online,
            observed_at_ms: now_ms,
            received_at_ms: now_ms,
            metadata: vec![Metadata::new("hue.worker", "snapshot")],
        })?;
        Ok(HueRefreshReport {
            bridge_id: self.bridge_id.clone(),
            refreshed_at_ms: now_ms,
            snapshot: summary,
            devices_upserted,
            entities_upserted,
            scenes_upserted,
        })
    }

    fn apply_snapshot(
        &mut self,
        snapshot: HueSnapshot,
        now_ms: u64,
    ) -> Result<(usize, usize, usize), HueIntegrationError> {
        let mut devices_upserted = 0usize;
        let mut entities_upserted = 0usize;
        let mut scenes_upserted = 0usize;
        for device in snapshot.devices {
            self.runtime
                .upsert_device(device.to_core(&self.bridge_id))?;
            devices_upserted += 1;
        }
        for light in snapshot.lights {
            let device_id =
                self.ensure_owner_device(&light.owner_device_id, &light.name, "Hue light")?;
            self.runtime.upsert_entity(hue_light_to_entity(
                &self.bridge_id,
                device_id,
                light,
                now_ms,
            ))?;
            entities_upserted += 1;
        }
        for motion in snapshot.motions {
            let device_id = self.ensure_owner_device(
                &motion.owner_device_id,
                &motion.name,
                "Hue motion sensor",
            )?;
            self.runtime.upsert_entity(hue_motion_to_entity(
                &self.bridge_id,
                device_id,
                motion,
                now_ms,
            ))?;
            entities_upserted += 1;
        }
        for button in snapshot.buttons {
            let device_id =
                self.ensure_owner_device(&button.owner_device_id, &button.name, "Hue button")?;
            self.runtime.upsert_entity(hue_button_to_entity(
                &self.bridge_id,
                device_id,
                button,
                now_ms,
            ))?;
            entities_upserted += 1;
        }
        for scene in snapshot.scenes {
            self.runtime.upsert_scene(scene.to_core(&self.bridge_id))?;
            scenes_upserted += 1;
        }
        Ok((devices_upserted, entities_upserted, scenes_upserted))
    }

    fn ensure_owner_device(
        &mut self,
        owner_id: &hue_core::HueResourceId,
        name: &str,
        model: &str,
    ) -> Result<DeviceId, HueIntegrationError> {
        let device = hue_device_to_core(
            &self.bridge_id,
            owner_id.clone(),
            "Philips Hue",
            model,
            name,
        );
        let device_id = device.device_id.clone();
        if self.runtime.registry().device(&device_id).is_none() {
            self.runtime.upsert_device(device)?;
        }
        Ok(device_id)
    }

    fn execute_event_poll(
        &mut self,
        now_ms: u64,
    ) -> Result<HueEventPollReport, HueIntegrationError> {
        let credentials = self.load_credentials()?;
        let request = hue_client::event_stream_request(&credentials.application_key)?;
        let response = self.transport.send(request)?;
        if !(200..300).contains(&response.status) {
            return Err(HueLanTransportError::HttpStatus(response.status).into());
        }
        let batches = parse_event_stream(&response.body)?;
        let summary = HueEventStreamSummary::from_batches(&batches);
        let updates = parse_state_updates_from_event_batches(&batches)?;
        let updates_received = updates.len();
        let mut events_applied = 0usize;
        let mut updates_without_entity = 0usize;
        for update in updates {
            let resource = update.summary().resource;
            let entity_id = hue_entity_id_for_resource_ref(&self.bridge_id, &resource);
            let Some(entity) = self.runtime.registry().entity(&entity_id).cloned() else {
                updates_without_entity += 1;
                continue;
            };
            for delta in update.state_deltas() {
                self.snapshot.event_sequence = self.snapshot.event_sequence.saturating_add(1);
                self.runtime.apply_device_event(DeviceEvent {
                    event_id: EventId::trusted(format!(
                        "hue.event:{}:{now_ms}:{}",
                        self.bridge_id, self.snapshot.event_sequence
                    )),
                    bridge_id: self.bridge_id.clone(),
                    device_id: Some(entity.device_id.clone()),
                    entity_id: Some(entity_id.clone()),
                    observed_at_ms: now_ms,
                    received_at_ms: now_ms,
                    event_type: DeviceEventType::Updated,
                    state_delta: Some(delta),
                    raw_ref: Some(resource.id.as_str().to_string()),
                    correlation_id: None,
                    metadata: vec![
                        Metadata::new("hue.worker", "event_stream"),
                        Metadata::new("hue.resource_type", resource.resource_type.as_hue_type()),
                    ],
                })?;
                events_applied += 1;
            }
        }
        Ok(HueEventPollReport {
            bridge_id: self.bridge_id.clone(),
            polled_at_ms: now_ms,
            stream: summary,
            updates_received,
            events_applied,
            updates_without_entity,
        })
    }

    fn execute_command(
        &mut self,
        authorization: &CommandAuthorization,
        command: DeviceCommand,
        now_ms: u64,
    ) -> Result<HueCommandDispatchReport, HueIntegrationError> {
        let accepted =
            self.runtime
                .submit_authorized_command(authorization, command.clone(), now_ms)?;
        let result = (|| {
            let hue_command = self.hue_command_for(&command)?;
            let credentials = self.load_credentials()?;
            let mut client = HueClient::new(
                HueClientConfig::paired(credentials.application_key.clone()),
                BorrowedTransport(&mut self.transport),
            );
            client.send_command(hue_command)?;
            Ok::<(), HueIntegrationError>(())
        })();
        let completed = match result {
            Ok(_) => CommandResult {
                command_id: command.command_id,
                status: CommandStatus::Accepted,
                bridge_id: self.bridge_id.clone(),
                correlation_id: command.correlation_id,
                message: Some("Hue bridge applied command".to_string()),
            },
            Err(error) => CommandResult {
                command_id: command.command_id,
                status: CommandStatus::Failed,
                bridge_id: self.bridge_id.clone(),
                correlation_id: command.correlation_id,
                message: Some(error.to_string()),
            },
        };
        self.runtime
            .event_bus_mut()
            .publish(RuntimeEvent::CommandResult(completed.clone()));
        Ok(HueCommandDispatchReport {
            accepted,
            completed,
        })
    }

    fn hue_command_for(&self, command: &DeviceCommand) -> Result<HueCommand, HueIntegrationError> {
        let entity = self
            .runtime
            .registry()
            .entity(&command.entity_id)
            .ok_or_else(|| HueIntegrationError::MissingEntity(command.entity_id.clone()))?;
        let resource_type = metadata_value(&entity.metadata, "hue.resource_type")
            .map(HueResourceType::from_hue_type);
        let resource_id = metadata_value(&entity.metadata, "hue.resource_id")
            .map(hue_core::HueResourceId::trusted);
        let (Some(resource_type), Some(resource_id)) = (resource_type, resource_id) else {
            return Err(HueIntegrationError::MissingHueResourceMetadata(
                command.entity_id.clone(),
            ));
        };
        let delta = StateDelta {
            capability_id: command.command_type.canonical_capability_id().ok_or(
                HueIntegrationError::UnsupportedCommand(command.command_type),
            )?,
            value: command_value(command)?,
        };
        HueCommand::from_state_delta(&HueResourceRef::new(resource_type, resource_id), &delta)?
            .ok_or(HueIntegrationError::UnsupportedCommand(
                command.command_type,
            ))
    }

    fn load_credentials(&self) -> Result<HueApplicationCredentials, HueIntegrationError> {
        let bridge = self
            .runtime
            .registry()
            .bridge(&self.bridge_id)
            .ok_or_else(|| HueIntegrationError::UnknownBridge(self.bridge_id.clone()))?;
        if bridge.integration_id.as_str() != "hue" {
            return Err(HueIntegrationError::WrongIntegration(
                bridge.integration_id.clone(),
            ));
        }
        let vault_ref = bridge
            .auth_ref
            .as_ref()
            .ok_or_else(|| HueIntegrationError::MissingVaultRef(self.bridge_id.clone()))?;
        let key = vault_record_key(vault_ref)
            .ok_or_else(|| HueIntegrationError::InvalidVaultRef(vault_ref.clone()))?;
        let record = self
            .vault
            .get(HUE_VAULT_NAMESPACE, key)?
            .ok_or_else(|| HueIntegrationError::MissingVaultRecord(vault_ref.clone()))?;
        Ok(HueApplicationCredentials::from_vault_secret_json(
            &record.plaintext,
        )?)
    }
}

struct BorrowedTransport<'a, T>(&'a mut T);

impl<T: HueTransport> HueTransport for BorrowedTransport<'_, T> {
    fn send(&mut self, request: HueHttpRequest) -> Result<HueHttpResponse, HueClientError> {
        self.0.send(request)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HueWorkerAction {
    Refresh,
    PollEvents,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HueWorkerRequest {
    pub action: HueWorkerAction,
    pub now_ms: u64,
}

impl HueWorkerRequest {
    pub fn from_message(message: &Message) -> Result<Self, HueIntegrationError> {
        if message.content_type != WORKER_REQUEST_CONTENT_TYPE {
            return Err(HueIntegrationError::InvalidWorkerRequest(format!(
                "expected content type `{WORKER_REQUEST_CONTENT_TYPE}`"
            )));
        }
        let value: serde_json::Value = serde_json::from_slice(&message.payload)
            .map_err(|error| HueIntegrationError::InvalidWorkerRequest(error.to_string()))?;
        let action = match value.get("action").and_then(serde_json::Value::as_str) {
            Some("refresh") => HueWorkerAction::Refresh,
            Some("poll_events") => HueWorkerAction::PollEvents,
            _ => {
                return Err(HueIntegrationError::InvalidWorkerRequest(
                    "`action` must be `refresh` or `poll_events`".to_string(),
                ))
            }
        };
        let now_ms = value
            .get("now_ms")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| {
                HueIntegrationError::InvalidWorkerRequest(
                    "`now_ms` must be a non-negative integer".to_string(),
                )
            })?;
        Ok(Self { action, now_ms })
    }
}

pub fn install_hue_integration_actor<T>(
    system: &mut ActorSystem,
    actor_id: &str,
    state: HueIntegrationActorState<T>,
) -> Result<String, ActorError>
where
    T: HueTransport + 'static,
{
    system.create_actor(
        actor_id,
        Box::new(state),
        Box::new(|state: Box<dyn Any>, message| {
            let mut state = *state
                .downcast::<HueIntegrationActorState<T>>()
                .expect("Hue integration actor received the wrong state type");
            match HueWorkerRequest::from_message(message) {
                Ok(request) => {
                    let _ = match request.action {
                        HueWorkerAction::Refresh => state.refresh(request.now_ms).map(|_| ()),
                        HueWorkerAction::PollEvents => {
                            state.poll_events(request.now_ms).map(|_| ())
                        }
                    };
                }
                Err(error) => {
                    state.snapshot.failed_count = state.snapshot.failed_count.saturating_add(1);
                    state.snapshot.last_error = Some(error.to_string());
                }
            }
            ActorResult {
                new_state: Box::new(state),
                messages_to_send: Vec::new(),
                actors_to_create: Vec::new(),
                stop: false,
            }
        }),
    )
}

pub fn endpoint_for_bridge(bridge: &Bridge) -> Result<LocalHttpEndpoint, HueIntegrationError> {
    let address = bridge
        .address
        .as_deref()
        .ok_or_else(|| HueIntegrationError::MissingBridgeAddress(bridge.bridge_id.clone()))?;
    let url = Url::parse(address).map_err(|error| {
        HueIntegrationError::InvalidWorkerRequest(format!("bridge address is invalid: {error}"))
    })?;
    let scheme = match url.scheme.as_str() {
        "http" => LocalHttpScheme::Http,
        "https" => LocalHttpScheme::Https,
        other => {
            return Err(HueIntegrationError::InvalidWorkerRequest(format!(
                "bridge address scheme `{other}` is unsupported"
            )))
        }
    };
    let host = url
        .host
        .ok_or_else(|| HueIntegrationError::MissingBridgeAddress(bridge.bridge_id.clone()))?;
    let mut endpoint = LocalHttpEndpoint::new(
        bridge.integration_id.clone(),
        bridge.bridge_id.clone(),
        scheme,
        host,
    )?;
    if let Some(port) = url.port {
        endpoint = endpoint.with_port(port);
    }
    if !url.path.is_empty() && url.path != "/" {
        endpoint = endpoint.with_base_path(url.path);
    }
    Ok(endpoint)
}

fn command_value(command: &DeviceCommand) -> Result<Value, HueIntegrationError> {
    match command.command_type {
        CommandType::TurnOn => Ok(Value::Bool(true)),
        CommandType::TurnOff => Ok(Value::Bool(false)),
        CommandType::SetBrightness | CommandType::SetColorTemperature => {
            Ok(command.arguments.clone())
        }
        other => Err(HueIntegrationError::UnsupportedCommand(other)),
    }
}

fn metadata_value<'a>(metadata: &'a [Metadata], key: &str) -> Option<&'a str> {
    metadata
        .iter()
        .find(|entry| entry.key == key)
        .map(|entry| entry.value.as_str())
}

fn encode_http_request(
    url: &Url,
    request: &HueHttpRequest,
) -> Result<Vec<u8>, HueLanTransportError> {
    let host = url
        .host
        .as_deref()
        .ok_or(HueLanTransportError::MissingHost)?;
    let port = url
        .effective_port()
        .ok_or(HueLanTransportError::MissingHost)?;
    let mut target = if url.path.is_empty() {
        "/".to_string()
    } else {
        url.path.clone()
    };
    if let Some(query) = &url.query {
        target.push('?');
        target.push_str(query);
    }
    if has_unsafe_http_text(&target) || request.headers.iter().any(unsafe_header) {
        return Err(HueLanTransportError::UnsafeRequest);
    }
    let default_port = match url.scheme.as_str() {
        "http" => 80,
        "https" => 443,
        scheme => return Err(HueLanTransportError::UnsupportedScheme(scheme.to_string())),
    };
    let host_header = if url.port.is_some() && port != default_port {
        format!("{host}:{port}")
    } else {
        host.to_string()
    };
    let mut bytes = format!(
        "{} {target} HTTP/1.1\r\nHost: {host_header}\r\nConnection: close\r\nContent-Length: {}\r\n",
        request.method_name(),
        request.body.len()
    )
    .into_bytes();
    for header in &request.headers {
        bytes.extend_from_slice(header.name.as_bytes());
        bytes.extend_from_slice(b": ");
        bytes.extend_from_slice(header.value.as_bytes());
        bytes.extend_from_slice(b"\r\n");
    }
    bytes.extend_from_slice(b"\r\n");
    bytes.extend_from_slice(&request.body);
    Ok(bytes)
}

fn connect_tcp(
    host: &str,
    port: u16,
    timeout: Duration,
) -> Result<TcpStream, HueLanTransportError> {
    let addresses = (host, port)
        .to_socket_addrs()
        .map_err(HueLanTransportError::Io)?
        .collect::<Vec<SocketAddr>>();
    let mut last_error = None;
    for address in addresses {
        match TcpStream::connect_timeout(&address, timeout) {
            Ok(stream) => {
                stream
                    .set_read_timeout(Some(timeout))
                    .map_err(HueLanTransportError::Io)?;
                stream
                    .set_write_timeout(Some(timeout))
                    .map_err(HueLanTransportError::Io)?;
                return Ok(stream);
            }
            Err(error) => last_error = Some(error),
        }
    }
    Err(HueLanTransportError::Io(last_error.unwrap_or_else(|| {
        io::Error::new(
            io::ErrorKind::AddrNotAvailable,
            "host resolved to no addresses",
        )
    })))
}

fn read_bounded(
    reader: &mut dyn Read,
    max_bytes: usize,
    allow_timeout_after_data: bool,
) -> Result<Vec<u8>, HueLanTransportError> {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => {
                if read > max_bytes.saturating_sub(bytes.len()) {
                    return Err(HueLanTransportError::ResponseTooLarge { limit: max_bytes });
                }
                bytes.extend_from_slice(&buffer[..read]);
            }
            Err(error)
                if allow_timeout_after_data
                    && !bytes.is_empty()
                    && matches!(
                        error.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                    ) =>
            {
                break;
            }
            Err(error) => return Err(HueLanTransportError::Io(error)),
        }
    }
    Ok(bytes)
}

fn decode_http_response(
    bytes: &[u8],
    max_body_bytes: usize,
    allow_incomplete_chunked: bool,
) -> Result<HueHttpResponse, HueLanTransportError> {
    let parsed = parse_response_head(bytes).map_err(HueLanTransportError::Http)?;
    let body_bytes = &bytes[parsed.body_offset..];
    let body = match parsed.body_kind {
        BodyKind::None => Vec::new(),
        BodyKind::ContentLength(expected) => {
            if body_bytes.len() < expected {
                return Err(HueLanTransportError::TruncatedBody {
                    expected,
                    actual: body_bytes.len(),
                });
            }
            body_bytes[..expected].to_vec()
        }
        BodyKind::UntilEof => body_bytes.to_vec(),
        BodyKind::Chunked => {
            decode_chunked_body(body_bytes, max_body_bytes, allow_incomplete_chunked)?
        }
    };
    if body.len() > max_body_bytes {
        return Err(HueLanTransportError::ResponseTooLarge {
            limit: max_body_bytes,
        });
    }
    Ok(HueHttpResponse {
        status: parsed.head.status,
        headers: parsed.head.headers,
        body,
    })
}

fn decode_chunked_body(
    input: &[u8],
    max_body_bytes: usize,
    allow_incomplete: bool,
) -> Result<Vec<u8>, HueLanTransportError> {
    let mut cursor = 0usize;
    let mut output = Vec::new();
    loop {
        let Some(line_offset) = input[cursor..]
            .windows(2)
            .position(|window| window == b"\r\n")
        else {
            if allow_incomplete {
                return Ok(output);
            }
            return Err(HueLanTransportError::InvalidChunkedBody(
                "missing chunk-size terminator".to_string(),
            ));
        };
        let line_end = cursor + line_offset;
        let size_text = std::str::from_utf8(&input[cursor..line_end])
            .map_err(|_| {
                HueLanTransportError::InvalidChunkedBody("chunk size is not ASCII".to_string())
            })?
            .split(';')
            .next()
            .unwrap_or_default();
        let size = usize::from_str_radix(size_text.trim(), 16).map_err(|_| {
            HueLanTransportError::InvalidChunkedBody("invalid chunk size".to_string())
        })?;
        cursor = line_end + 2;
        if size == 0 {
            return Ok(output);
        }
        if size > max_body_bytes.saturating_sub(output.len()) {
            return Err(HueLanTransportError::ResponseTooLarge {
                limit: max_body_bytes,
            });
        }
        let end = cursor.checked_add(size).ok_or_else(|| {
            HueLanTransportError::InvalidChunkedBody("chunk size overflow".to_string())
        })?;
        if end + 2 > input.len() {
            if allow_incomplete {
                return Ok(output);
            }
            return Err(HueLanTransportError::InvalidChunkedBody(
                "truncated chunk payload".to_string(),
            ));
        }
        if &input[end..end + 2] != b"\r\n" {
            return Err(HueLanTransportError::InvalidChunkedBody(
                "chunk payload has no terminator".to_string(),
            ));
        }
        output.extend_from_slice(&input[cursor..end]);
        cursor = end + 2;
    }
}

fn has_unsafe_http_text(value: &str) -> bool {
    value
        .bytes()
        .any(|byte| byte == b'\r' || byte == b'\n' || byte == 0)
}

fn unsafe_header(header: &Header) -> bool {
    has_unsafe_http_text(&header.name)
        || has_unsafe_http_text(&header.value)
        || header.name.contains(':')
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::io::{BufRead, BufReader};
    use std::net::TcpListener;
    use std::thread;

    use coding_adventures_vault_sealed_store::InitOptions;
    use hue_core::HUE_APPLICATION_KEY_HEADER;
    use smart_home_core::{
        AgentId, BridgeTransport, CapabilityGrant, CapabilityGrantId, CommandId, CorrelationId,
        PrivilegeTier,
    };
    use storage_core::{InMemoryStorageBackend, StorageBackend};

    use super::*;

    const APPLICATION_KEY: &str = "fixture-hue-application-key";
    const SNAPSHOT_BODY: &[u8] = br#"{"data":[
        {"id":"bridge-resource-1","type":"bridge","bridge_id":"bridge-1","owner":{"rid":"device-bridge","rtype":"device"}},
        {"id":"device-1","type":"device","metadata":{"name":"Kitchen lamp"},"product_data":{"manufacturer_name":"Signify Netherlands B.V.","model_id":"LCA001","product_name":"Hue color lamp","software_version":"1.116.3"},"services":[{"rid":"light-1","rtype":"light"}]},
        {"id":"light-1","type":"light","metadata":{"name":"Kitchen lamp"},"owner":{"rid":"device-1","rtype":"device"},"on":{"on":false},"dimming":{"brightness":10}}
    ],"errors":[]}"#;
    const COMMAND_BODY: &[u8] = br#"{"data":[{"rid":"light-1","rtype":"light"}],"errors":[]}"#;
    const EVENT_BODY: &[u8] = b"id: stream-1\n\
event: update\n\
data: [{\"creationtime\":\"2026-07-30T02:00:00Z\",\"data\":[{\"id\":\"light-1\",\"type\":\"light\",\"on\":{\"on\":true}}],\"id\":\"event-1\",\"type\":\"update\"}]\n\n";

    struct QueuedTransport {
        responses: VecDeque<HueHttpResponse>,
    }

    impl HueTransport for QueuedTransport {
        fn send(&mut self, _request: HueHttpRequest) -> Result<HueHttpResponse, HueClientError> {
            self.responses
                .pop_front()
                .ok_or_else(|| HueClientError::transport("no queued response"))
        }
    }

    fn open_vault() -> Arc<SealedStore> {
        let backend: Arc<dyn StorageBackend> = Arc::new(InMemoryStorageBackend::new());
        backend.initialize().unwrap();
        let vault = Arc::new(SealedStore::new(backend));
        vault
            .init(
                b"test-password",
                &InitOptions {
                    argon2id_time_cost: 1,
                    argon2id_memory_kib: 32,
                    argon2id_parallelism: 1,
                    salt_override: Some(vec![9; 16]),
                },
            )
            .unwrap();
        vault
    }

    fn runtime_and_vault(address: String) -> (SmartHomeRuntime, Arc<SealedStore>) {
        let vault = open_vault();
        let vault_ref = VaultRef::trusted("vault://smart-home/hue/bridge-1/fixture");
        let credentials =
            HueApplicationCredentials::new(APPLICATION_KEY, Some("client-key".to_string()))
                .unwrap();
        vault
            .put(
                HUE_VAULT_NAMESPACE,
                vault_record_key(&vault_ref).unwrap(),
                &credentials.vault_secret_json(),
                None,
            )
            .unwrap();

        let mut bridge = Bridge::new(
            BridgeId::trusted("bridge-1"),
            IntegrationId::trusted("hue"),
            BridgeTransport::LanHttp,
        );
        bridge.address = Some(address);
        bridge.auth_ref = Some(vault_ref);
        let mut runtime = SmartHomeRuntime::new();
        runtime.upsert_bridge(bridge).unwrap();
        (runtime, vault)
    }

    fn spawn_bridge(responses: Vec<&'static [u8]>) -> (String, thread::JoinHandle<Vec<Vec<u8>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let handle = thread::spawn(move || {
            let mut requests = Vec::new();
            for response_body in responses {
                let (stream, _) = listener.accept().unwrap();
                let mut reader = BufReader::new(stream);
                let mut request = Vec::new();
                loop {
                    let mut line = Vec::new();
                    reader.read_until(b'\n', &mut line).unwrap();
                    let done = line == b"\r\n";
                    request.extend_from_slice(&line);
                    if done {
                        break;
                    }
                }
                let content_length = request
                    .split(|byte| *byte == b'\n')
                    .filter_map(|line| std::str::from_utf8(line).ok())
                    .find_map(|line| {
                        let (name, value) = line.trim().split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().ok())
                            .flatten()
                    })
                    .unwrap_or(0);
                let mut body = vec![0; content_length];
                reader.read_exact(&mut body).unwrap();
                request.extend_from_slice(&body);
                requests.push(request);

                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    response_body.len()
                );
                let stream = reader.get_mut();
                stream.write_all(response.as_bytes()).unwrap();
                stream.write_all(response_body).unwrap();
                stream.flush().unwrap();
            }
            requests
        });
        (format!("http://{address}"), handle)
    }

    #[test]
    fn real_lan_worker_refreshes_commands_and_applies_event_stream() {
        let (address, server) = spawn_bridge(vec![SNAPSHOT_BODY, COMMAND_BODY, EVENT_BODY]);
        let (runtime, vault) = runtime_and_vault(address);
        let bridge = runtime
            .registry()
            .bridge(&BridgeId::trusted("bridge-1"))
            .unwrap()
            .clone();
        let transport = HueLanTransport::for_bridge(&bridge).unwrap();
        let mut worker =
            HueIntegrationActorState::new(bridge.bridge_id.clone(), runtime, vault, transport);

        let refresh = worker.refresh(1_000).unwrap();
        assert_eq!(refresh.snapshot.total_resources, 3);
        assert_eq!(refresh.devices_upserted, 1);
        assert_eq!(refresh.entities_upserted, 1);

        let principal = AgentId::trusted("agent:lighting");
        let authorization = CommandAuthorization::new(
            principal.clone(),
            vec![CapabilityGrant::for_all_smart_home(
                CapabilityGrantId::trusted("grant-lighting"),
                principal,
                PrivilegeTier::LowRisk,
                "test",
                900,
            )],
        );
        let entity_id = EntityId::trusted("hue.light.bridge-1.light-1");
        let command = DeviceCommand::new(
            CommandId::trusted("command-1"),
            entity_id.clone(),
            CommandType::TurnOn,
            Value::Null,
            "agent:lighting",
            CorrelationId::trusted("correlation-1"),
        )
        .unwrap();
        let command_report = worker
            .dispatch_authorized_command(&authorization, command, 1_100)
            .unwrap();
        assert_eq!(command_report.accepted.status, CommandStatus::Accepted);
        assert_eq!(command_report.completed.status, CommandStatus::Accepted);

        let event_report = worker.poll_events(1_200).unwrap();
        assert_eq!(event_report.updates_received, 1);
        assert_eq!(event_report.events_applied, 1);
        assert_eq!(
            worker
                .runtime()
                .registry()
                .entity(&entity_id)
                .and_then(|entity| entity.state.as_ref())
                .map(|state| &state.value),
            Some(&Value::Object(vec![(
                "light.on_off".to_string(),
                Value::Bool(true),
            )]))
        );

        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 3);
        let request_text = requests
            .iter()
            .map(|request| String::from_utf8_lossy(request))
            .collect::<Vec<_>>();
        assert!(request_text[0].starts_with("GET /clip/v2/resource HTTP/1.1\r\n"));
        assert!(request_text[1].starts_with("PUT /clip/v2/resource/light/light-1 HTTP/1.1\r\n"));
        assert!(request_text[2].starts_with("GET /eventstream/clip/v2 HTTP/1.1\r\n"));
        assert!(request_text
            .iter()
            .all(|request| request
                .contains(&format!("{HUE_APPLICATION_KEY_HEADER}: {APPLICATION_KEY}"))));
        assert!(request_text[1].contains(r#"{"on":{"on":true}}"#));
        assert_eq!(worker.snapshot().failed_count, 0);
        assert_eq!(worker.snapshot().last_success_at_ms, Some(1_200));
        assert!(!format!("{:?}", worker.snapshot()).contains(APPLICATION_KEY));
    }

    #[test]
    fn authorization_rejection_happens_before_vault_or_network_access() {
        let (address, server) = spawn_bridge(Vec::new());
        let (runtime, vault) = runtime_and_vault(address);
        let bridge = runtime
            .registry()
            .bridge(&BridgeId::trusted("bridge-1"))
            .unwrap()
            .clone();
        let transport = HueLanTransport::for_bridge(&bridge).unwrap();
        let mut worker =
            HueIntegrationActorState::new(bridge.bridge_id.clone(), runtime, vault, transport);
        let command = DeviceCommand::new(
            CommandId::trusted("command-denied"),
            EntityId::trusted("missing-entity"),
            CommandType::TurnOn,
            Value::Null,
            "agent:denied",
            CorrelationId::trusted("correlation-denied"),
        )
        .unwrap();
        let authorization = CommandAuthorization::new(AgentId::trusted("agent:denied"), Vec::new());

        let error = worker
            .dispatch_authorized_command(&authorization, command, 1_000)
            .unwrap_err();

        assert!(matches!(error, HueIntegrationError::Runtime(_)));
        assert_eq!(worker.snapshot().failed_count, 1);
        assert_eq!(server.join().unwrap(), Vec::<Vec<u8>>::new());
    }

    #[test]
    fn failed_bridge_command_is_published_and_counted_as_worker_failure() {
        let (runtime, vault) = runtime_and_vault("http://127.0.0.1:9".to_string());
        let transport = QueuedTransport {
            responses: VecDeque::from([
                HueHttpResponse::json(200, SNAPSHOT_BODY.to_vec()),
                HueHttpResponse::json(
                    500,
                    br#"{"data":[],"errors":[{"description":"bridge busy"}]}"#.to_vec(),
                ),
            ]),
        };
        let mut worker =
            HueIntegrationActorState::new(BridgeId::trusted("bridge-1"), runtime, vault, transport);
        worker.refresh(1_000).unwrap();
        let principal = AgentId::trusted("agent:lighting");
        let authorization = CommandAuthorization::new(
            principal.clone(),
            vec![CapabilityGrant::for_all_smart_home(
                CapabilityGrantId::trusted("grant-lighting"),
                principal,
                PrivilegeTier::LowRisk,
                "test",
                900,
            )],
        );
        let command = DeviceCommand::new(
            CommandId::trusted("command-failed"),
            EntityId::trusted("hue.light.bridge-1.light-1"),
            CommandType::TurnOn,
            Value::Null,
            "agent:lighting",
            CorrelationId::trusted("correlation-failed"),
        )
        .unwrap();

        let report = worker
            .dispatch_authorized_command(&authorization, command, 1_100)
            .unwrap();

        assert_eq!(report.accepted.status, CommandStatus::Accepted);
        assert_eq!(report.completed.status, CommandStatus::Failed);
        assert_eq!(worker.snapshot().failed_count, 1);
        assert!(worker
            .snapshot()
            .last_error
            .as_deref()
            .is_some_and(|message| message.contains("HTTP 500")));
        assert_eq!(
            worker
                .last_command()
                .map(|command| command.completed.status),
            Some(CommandStatus::Failed)
        );
    }

    #[test]
    fn actor_worker_messages_carry_schedule_data_not_credentials() {
        let message = Message::new(
            "scheduler",
            WORKER_REQUEST_CONTENT_TYPE,
            br#"{"action":"poll_events","now_ms":1234}"#.to_vec(),
            None,
        );

        let request = HueWorkerRequest::from_message(&message).unwrap();

        assert_eq!(request.action, HueWorkerAction::PollEvents);
        assert_eq!(request.now_ms, 1_234);
        assert!(!String::from_utf8_lossy(&message.payload).contains(APPLICATION_KEY));
    }
}
