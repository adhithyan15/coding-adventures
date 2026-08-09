//! Actor-owned Hue LAN registration and durable credential handoff.

#![forbid(unsafe_code)]

use std::any::Any;
use std::fmt;
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::Arc;
use std::time::Duration;

use actor::{ActorError, ActorResult, ActorSystem, Message};
use coding_adventures_csprng::random_array;
use coding_adventures_vault_sealed_store::SealedStore;
use coding_adventures_zeroize::Zeroizing;
use http1::{parse_response_head, Http1ParseError};
use http_core::{BodyKind, Header};
use hue_core::{
    hue_application_credentials_from_registration_response, hue_pairing_registration_request_plan,
    hue_registration_request, HueBridgePairingPlan, HueError, CLIP_V2_EVENT_STREAM_PATH,
    HUE_APPLICATION_KEY_HEADER,
};
use smart_home_core::{AgentId, Bridge, BridgeId, IntegrationId, VaultRef};
use smart_home_local_http::{
    LocalHttpEndpoint, LocalHttpError, LocalHttpRequestPlan, LocalHttpScheme,
};
use smart_home_pairing_transaction::{
    PairingCredentialLocation, PairingTransactionCoordinator, PairingTransactionError,
    PairingTransactionOutcome, PairingTransactionRequest,
};
use smart_home_runtime::{
    PairingSessionStatus, RuntimeCompletePairingToolRequest, RuntimeError, RuntimePairingSessionId,
    SmartHomeRuntime,
};
use smart_home_runtime_store::{
    DurableAutomationDefinition, RestoredSmartHomeRuntime, RuntimeStoreError, SmartHomeRuntimeStore,
};
use storage_core::{Revision, StorageBackend};
use tls_platform::{default_connector, TlsConfig, TlsConnector, TlsError};
use url_parser::{Url, UrlError};

pub const PAIR_REQUEST_CONTENT_TYPE: &str = "application/vnd.smart-home.hue-pairing-request+json";
pub const HUE_VAULT_NAMESPACE: &str = "smart_home.hue.credentials";
pub const HUE_VAULT_REF_PREFIX: &str = "vault://smart-home/hue/";
pub const DEFAULT_MAX_RESPONSE_BYTES: usize = 64 * 1024;

#[derive(Debug)]
pub enum HuePairingServiceError {
    UnknownSession(RuntimePairingSessionId),
    SessionNotPending {
        session_id: RuntimePairingSessionId,
        status: PairingSessionStatus,
    },
    UnknownBridge(BridgeId),
    WrongIntegration(IntegrationId),
    MissingBridgeAddress(BridgeId),
    InvalidRequest(String),
    LocalHttp(LocalHttpError),
    Hue(HueError),
    Transport(HuePairingTransportError),
    Runtime(RuntimeError),
    RuntimeStore(RuntimeStoreError),
    Transaction(PairingTransactionError),
    Entropy(String),
    MissingDurableRuntime,
    ExistingCredentialReference(VaultRef),
    TransactionRolledBack(String),
}

impl fmt::Display for HuePairingServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownSession(session_id) => {
                write!(formatter, "unknown Hue pairing session {session_id}")
            }
            Self::SessionNotPending { session_id, status } => write!(
                formatter,
                "Hue pairing session {session_id} is not pending user presence ({status:?})"
            ),
            Self::UnknownBridge(bridge_id) => {
                write!(formatter, "unknown Hue bridge {bridge_id}")
            }
            Self::WrongIntegration(integration_id) => write!(
                formatter,
                "pairing service only accepts Hue bridges, got integration {integration_id}"
            ),
            Self::MissingBridgeAddress(bridge_id) => {
                write!(formatter, "Hue bridge {bridge_id} has no LAN address")
            }
            Self::InvalidRequest(message) => {
                write!(formatter, "invalid Hue pairing request: {message}")
            }
            Self::LocalHttp(error) => write!(formatter, "Hue request planning failed: {error}"),
            Self::Hue(error) => write!(formatter, "Hue registration failed: {error}"),
            Self::Transport(error) => write!(formatter, "Hue LAN transport failed: {error}"),
            Self::Runtime(error) => write!(formatter, "Hue runtime completion failed: {error}"),
            Self::RuntimeStore(error) => write!(formatter, "Hue runtime store failed: {error}"),
            Self::Transaction(error) => {
                write!(formatter, "Hue pairing transaction failed: {error}")
            }
            Self::Entropy(message) => write!(
                formatter,
                "Hue pairing transaction generation failed: {message}"
            ),
            Self::MissingDurableRuntime => {
                formatter.write_str("Hue pairing requires a durable runtime snapshot")
            }
            Self::ExistingCredentialReference(vault_ref) => write!(
                formatter,
                "existing Hue credential reference is outside the Hue Vault namespace: {vault_ref}"
            ),
            Self::TransactionRolledBack(transaction_id) => write!(
                formatter,
                "Hue pairing transaction {transaction_id} rolled back before runtime commit"
            ),
        }
    }
}

impl std::error::Error for HuePairingServiceError {}

impl From<LocalHttpError> for HuePairingServiceError {
    fn from(error: LocalHttpError) -> Self {
        Self::LocalHttp(error)
    }
}

impl From<HueError> for HuePairingServiceError {
    fn from(error: HueError) -> Self {
        Self::Hue(error)
    }
}

impl From<HuePairingTransportError> for HuePairingServiceError {
    fn from(error: HuePairingTransportError) -> Self {
        Self::Transport(error)
    }
}

impl From<RuntimeError> for HuePairingServiceError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

impl From<RuntimeStoreError> for HuePairingServiceError {
    fn from(error: RuntimeStoreError) -> Self {
        Self::RuntimeStore(error)
    }
}

impl From<PairingTransactionError> for HuePairingServiceError {
    fn from(error: PairingTransactionError) -> Self {
        Self::Transaction(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HuePairingRequest {
    pub session_id: RuntimePairingSessionId,
    pub principal_id: AgentId,
    pub expected_runtime_revision: Revision,
    pub app_name: String,
    pub instance_name: String,
    pub completed_at_ms: u64,
}

impl HuePairingRequest {
    pub fn new(
        session_id: RuntimePairingSessionId,
        principal_id: AgentId,
        expected_runtime_revision: Revision,
        app_name: impl Into<String>,
        instance_name: impl Into<String>,
        completed_at_ms: u64,
    ) -> Result<Self, HuePairingServiceError> {
        let app_name = app_name.into();
        let instance_name = instance_name.into();
        if app_name.trim().is_empty() {
            return Err(HuePairingServiceError::InvalidRequest(
                "app_name must not be empty".to_string(),
            ));
        }
        if instance_name.trim().is_empty() {
            return Err(HuePairingServiceError::InvalidRequest(
                "instance_name must not be empty".to_string(),
            ));
        }
        Ok(Self {
            session_id,
            principal_id,
            expected_runtime_revision,
            app_name,
            instance_name,
            completed_at_ms,
        })
    }

    pub fn into_message(self, sender_id: &str) -> Result<Message, HuePairingServiceError> {
        let payload = serde_json::to_vec(&serde_json::json!({
            "schema_version": 2,
            "session_id": self.session_id.as_str(),
            "principal_id": self.principal_id.as_str(),
            "expected_runtime_revision": self.expected_runtime_revision.as_str(),
            "app_name": self.app_name,
            "instance_name": self.instance_name,
            "completed_at_ms": self.completed_at_ms,
        }))
        .map_err(|error| HuePairingServiceError::InvalidRequest(error.to_string()))?;
        Ok(Message::new(
            sender_id,
            PAIR_REQUEST_CONTENT_TYPE,
            payload,
            None,
        ))
    }

    fn from_message(message: &Message) -> Result<Self, HuePairingServiceError> {
        if message.content_type != PAIR_REQUEST_CONTENT_TYPE {
            return Err(HuePairingServiceError::InvalidRequest(format!(
                "message content type must be `{PAIR_REQUEST_CONTENT_TYPE}`"
            )));
        }
        let value: serde_json::Value = serde_json::from_slice(&message.payload)
            .map_err(|error| HuePairingServiceError::InvalidRequest(error.to_string()))?;
        let object = value.as_object().ok_or_else(|| {
            HuePairingServiceError::InvalidRequest("message body must be an object".to_string())
        })?;
        if object
            .get("schema_version")
            .and_then(serde_json::Value::as_u64)
            != Some(2)
        {
            return Err(HuePairingServiceError::InvalidRequest(
                "unsupported schema_version".to_string(),
            ));
        }
        Self::new(
            RuntimePairingSessionId::trusted(required_json_string(object, "session_id")?),
            AgentId::new(required_json_string(object, "principal_id")?)
                .map_err(|error| HuePairingServiceError::InvalidRequest(error.to_string()))?,
            Revision::new(required_json_string(object, "expected_runtime_revision")?)
                .map_err(|error| HuePairingServiceError::InvalidRequest(error.to_string()))?,
            required_json_string(object, "app_name")?,
            required_json_string(object, "instance_name")?,
            object
                .get("completed_at_ms")
                .and_then(serde_json::Value::as_u64)
                .ok_or_else(|| {
                    HuePairingServiceError::InvalidRequest(
                        "completed_at_ms must be a non-negative integer".to_string(),
                    )
                })?,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HueRegistrationResponse {
    pub status: u16,
    pub headers: Vec<Header>,
    pub body: Vec<u8>,
}

pub trait HueRegistrationTransport {
    fn execute(
        &mut self,
        endpoint: &LocalHttpEndpoint,
        request: &LocalHttpRequestPlan,
    ) -> Result<HueRegistrationResponse, HuePairingTransportError>;
}

#[derive(Debug)]
pub enum HuePairingTransportError {
    Url(UrlError),
    Tls(TlsError),
    Io(io::Error),
    Http(Http1ParseError),
    UnsupportedScheme(String),
    MissingHost,
    UnsafeRequest,
    HttpStatus(u16),
    ResponseTooLarge { limit: usize },
    TruncatedBody { expected: usize, actual: usize },
    InvalidChunkedBody(String),
}

impl fmt::Display for HuePairingTransportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Url(error) => write!(formatter, "invalid URL: {error}"),
            Self::Tls(error) => write!(formatter, "TLS failed: {error}"),
            Self::Io(error) => write!(formatter, "network I/O failed: {error}"),
            Self::Http(error) => write!(formatter, "HTTP response failed: {error}"),
            Self::UnsupportedScheme(scheme) => {
                write!(formatter, "unsupported LAN URL scheme `{scheme}`")
            }
            Self::MissingHost => formatter.write_str("LAN URL is missing a host"),
            Self::UnsafeRequest => formatter.write_str("HTTP request contains unsafe characters"),
            Self::HttpStatus(status) => {
                write!(formatter, "bridge returned HTTP {status}")
            }
            Self::ResponseTooLarge { limit } => {
                write!(formatter, "HTTP response exceeds {limit} bytes")
            }
            Self::TruncatedBody { expected, actual } => write!(
                formatter,
                "HTTP response body was truncated: expected {expected} bytes, got {actual}"
            ),
            Self::InvalidChunkedBody(message) => {
                write!(formatter, "invalid chunked HTTP response: {message}")
            }
        }
    }
}

impl std::error::Error for HuePairingTransportError {}

pub struct HueLanRegistrationTransport {
    connector: Box<dyn TlsConnector>,
    tls_config: TlsConfig,
    max_response_bytes: usize,
}

impl Default for HueLanRegistrationTransport {
    fn default() -> Self {
        Self::new(default_connector(), TlsConfig::https_default())
    }
}

impl HueLanRegistrationTransport {
    pub fn new(connector: Box<dyn TlsConnector>, tls_config: TlsConfig) -> Self {
        Self {
            connector,
            tls_config,
            max_response_bytes: DEFAULT_MAX_RESPONSE_BYTES,
        }
    }

    pub fn with_max_response_bytes(mut self, max_response_bytes: usize) -> Self {
        self.max_response_bytes = max_response_bytes.max(1);
        self
    }
}

impl HueRegistrationTransport for HueLanRegistrationTransport {
    fn execute(
        &mut self,
        endpoint: &LocalHttpEndpoint,
        request: &LocalHttpRequestPlan,
    ) -> Result<HueRegistrationResponse, HuePairingTransportError> {
        let url = Url::parse(&request.url).map_err(HuePairingTransportError::Url)?;
        let host = url
            .host
            .as_deref()
            .ok_or(HuePairingTransportError::MissingHost)?;
        let port = url
            .effective_port()
            .ok_or(HuePairingTransportError::MissingHost)?;
        let request_bytes = encode_http_request(&url, request)?;
        let timeout = Duration::from_millis(request.timeout_ms.max(1));

        let response_bytes = match url.scheme.as_str() {
            "http" => {
                let mut stream = connect_tcp(host, port, timeout)?;
                stream
                    .write_all(&request_bytes)
                    .map_err(HuePairingTransportError::Io)?;
                stream.flush().map_err(HuePairingTransportError::Io)?;
                read_bounded(&mut stream, self.max_response_bytes)?
            }
            "https" => {
                let mut config = self.tls_config.clone();
                config.connect_timeout = timeout;
                config.read_timeout = Some(timeout);
                config.write_timeout = Some(timeout);
                if config.server_name.is_none() {
                    config.server_name = endpoint.tls_name.clone();
                }
                let mut stream = self
                    .connector
                    .connect(host, port, &config)
                    .map_err(HuePairingTransportError::Tls)?;
                stream
                    .write_all(&request_bytes)
                    .map_err(HuePairingTransportError::Io)?;
                stream.flush().map_err(HuePairingTransportError::Io)?;
                let bytes = read_bounded(&mut stream, self.max_response_bytes)?;
                stream
                    .close_notify()
                    .map_err(HuePairingTransportError::Tls)?;
                bytes
            }
            scheme => {
                return Err(HuePairingTransportError::UnsupportedScheme(
                    scheme.to_string(),
                ))
            }
        };
        decode_http_response(&response_bytes, self.max_response_bytes)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HuePairingServiceSnapshot {
    pub request_count: u64,
    pub completed_count: u64,
    pub failed_count: u64,
    pub recovered_transaction_count: u64,
    pub last_completed_at_ms: Option<u64>,
    pub last_bridge_id: Option<BridgeId>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HuePairingReport {
    pub session_id: RuntimePairingSessionId,
    pub bridge_id: BridgeId,
    pub vault_ref: VaultRef,
    pub completed_at_ms: u64,
    pub client_key_present: bool,
}

pub struct HuePairingServiceActorState<T, J, R> {
    runtime: SmartHomeRuntime,
    automation_definitions: Vec<DurableAutomationDefinition>,
    automation_state: Option<serde_json::Value>,
    runtime_revision: Revision,
    journal_backend: J,
    runtime_store: SmartHomeRuntimeStore<R>,
    vault: Arc<SealedStore>,
    transport: T,
    snapshot: HuePairingServiceSnapshot,
    last_report: Option<HuePairingReport>,
}

impl<T, J, R> HuePairingServiceActorState<T, J, R>
where
    T: HueRegistrationTransport,
    J: StorageBackend,
    R: StorageBackend,
{
    pub fn restore(
        journal_backend: J,
        vault: Arc<SealedStore>,
        runtime_store: SmartHomeRuntimeStore<R>,
        transport: T,
    ) -> Result<Self, HuePairingServiceError> {
        let mut restored = runtime_store
            .load()?
            .ok_or(HuePairingServiceError::MissingDurableRuntime)?;
        let recovered_transaction_count = {
            let coordinator =
                PairingTransactionCoordinator::new(&journal_backend, &vault, &runtime_store);
            let pending = coordinator.pending_transaction_ids()?;
            let recovered_count = pending.len() as u64;
            for transaction_id in pending {
                match coordinator.recover(&transaction_id)? {
                    PairingTransactionOutcome::Committed {
                        restored: committed,
                        ..
                    } => restored = *committed,
                    PairingTransactionOutcome::RolledBack { .. } => {
                        restored = runtime_store
                            .load()?
                            .ok_or(HuePairingServiceError::MissingDurableRuntime)?;
                    }
                }
            }
            if !coordinator.pending_transaction_ids()?.is_empty() {
                return Err(HuePairingServiceError::InvalidRequest(
                    "pairing transaction recovery left unresolved journals".to_string(),
                ));
            }
            recovered_count
        };
        Ok(Self::from_restored(
            restored,
            journal_backend,
            vault,
            runtime_store,
            transport,
            recovered_transaction_count,
        ))
    }

    fn from_restored(
        restored: RestoredSmartHomeRuntime,
        journal_backend: J,
        vault: Arc<SealedStore>,
        runtime_store: SmartHomeRuntimeStore<R>,
        transport: T,
        recovered_transaction_count: u64,
    ) -> Self {
        Self {
            runtime: restored.runtime,
            automation_definitions: restored.automation_definitions,
            automation_state: restored.automation_state,
            runtime_revision: restored.revision,
            journal_backend,
            runtime_store,
            vault,
            transport,
            snapshot: HuePairingServiceSnapshot {
                recovered_transaction_count,
                ..HuePairingServiceSnapshot::default()
            },
            last_report: None,
        }
    }

    pub fn runtime(&self) -> &SmartHomeRuntime {
        &self.runtime
    }

    pub fn runtime_revision(&self) -> &Revision {
        &self.runtime_revision
    }

    pub fn snapshot(&self) -> &HuePairingServiceSnapshot {
        &self.snapshot
    }

    pub fn last_report(&self) -> Option<&HuePairingReport> {
        self.last_report.as_ref()
    }

    pub fn pair(
        &mut self,
        request: HuePairingRequest,
    ) -> Result<&HuePairingReport, HuePairingServiceError> {
        self.snapshot.request_count = self.snapshot.request_count.saturating_add(1);
        let result = self.execute_pairing(request);
        match result {
            Ok(report) => {
                self.snapshot.completed_count = self.snapshot.completed_count.saturating_add(1);
                self.snapshot.last_completed_at_ms = Some(report.completed_at_ms);
                self.snapshot.last_bridge_id = Some(report.bridge_id.clone());
                self.snapshot.last_error = None;
                self.last_report = Some(report);
                Ok(self
                    .last_report
                    .as_ref()
                    .expect("pairing report was assigned before returning"))
            }
            Err(error) => {
                self.snapshot.failed_count = self.snapshot.failed_count.saturating_add(1);
                self.snapshot.last_error = Some(error.to_string());
                Err(error)
            }
        }
    }

    fn execute_pairing(
        &mut self,
        request: HuePairingRequest,
    ) -> Result<HuePairingReport, HuePairingServiceError> {
        let session = self
            .runtime
            .pairing_session(&request.session_id)
            .cloned()
            .ok_or_else(|| HuePairingServiceError::UnknownSession(request.session_id.clone()))?;
        if session.status != PairingSessionStatus::PendingUserPresence {
            return Err(HuePairingServiceError::SessionNotPending {
                session_id: session.session_id,
                status: session.status,
            });
        }
        let bridge = self
            .runtime
            .registry()
            .bridge(&session.bridge_id)
            .cloned()
            .ok_or_else(|| HuePairingServiceError::UnknownBridge(session.bridge_id.clone()))?;
        if bridge.integration_id.as_str() != "hue" {
            return Err(HuePairingServiceError::WrongIntegration(
                bridge.integration_id,
            ));
        }

        if request.expected_runtime_revision != self.runtime_revision {
            return Err(HuePairingServiceError::InvalidRequest(
                "expected runtime revision is stale".to_string(),
            ));
        }

        let authorization_probe = RuntimeCompletePairingToolRequest::new(
            request.session_id.clone(),
            VaultRef::trusted("vault://smart-home/hue/authorization-preflight"),
            request.completed_at_ms,
        );
        self.runtime.clone().execute_complete_pairing_tool(
            request.principal_id.clone(),
            authorization_probe,
            request.completed_at_ms,
        )?;

        let endpoint = endpoint_for_bridge(&bridge)?;
        let plan = HueBridgePairingPlan {
            bridge,
            registration_request: hue_registration_request(request.app_name, request.instance_name),
            application_key_header: HUE_APPLICATION_KEY_HEADER.to_string(),
            event_stream_path: CLIP_V2_EVENT_STREAM_PATH.to_string(),
            requires_user_presence: true,
        };
        let request_plan = hue_pairing_registration_request_plan(&plan, &endpoint)?;
        let response = self.transport.execute(&endpoint, &request_plan)?;
        if !(200..300).contains(&response.status) {
            return Err(HuePairingTransportError::HttpStatus(response.status).into());
        }

        let credentials = hue_application_credentials_from_registration_response(&response.body)?;
        let client_key_present = credentials.has_client_key();
        let transaction_id = new_transaction_id()?;
        let vault_key = new_vault_key(plan.bridge_id(), &transaction_id);
        let vault_ref = VaultRef::trusted(format!("{HUE_VAULT_REF_PREFIX}{vault_key}"));
        let secret = Zeroizing::new(credentials.vault_secret_json());
        let handoff = credentials.vault_handoff(&plan, vault_ref.clone(), request.completed_at_ms);
        let new_credential =
            PairingCredentialLocation::new(vault_ref.clone(), HUE_VAULT_NAMESPACE, vault_key)?;
        let previous_credential = plan
            .bridge
            .auth_ref
            .as_ref()
            .map(hue_credential_location)
            .transpose()?;
        let transaction = PairingTransactionRequest::new(
            transaction_id.clone(),
            request.principal_id,
            plan.bridge_id().clone(),
            request.session_id.clone(),
            new_credential,
            request.completed_at_ms,
            request.expected_runtime_revision,
        )?
        .with_metadata(handoff.metadata);
        let transaction = match previous_credential {
            Some(previous) => transaction.with_previous_credential(previous),
            None => transaction,
        };
        let outcome = PairingTransactionCoordinator::new(
            &self.journal_backend,
            &self.vault,
            &self.runtime_store,
        )
        .execute(transaction, &secret)?;
        let PairingTransactionOutcome::Committed { restored, .. } = outcome else {
            return Err(HuePairingServiceError::TransactionRolledBack(
                transaction_id,
            ));
        };
        self.install_restored_runtime(*restored);

        Ok(HuePairingReport {
            session_id: request.session_id,
            bridge_id: plan.bridge_id().clone(),
            vault_ref,
            completed_at_ms: request.completed_at_ms,
            client_key_present,
        })
    }

    fn install_restored_runtime(&mut self, restored: RestoredSmartHomeRuntime) {
        self.runtime = restored.runtime;
        self.automation_definitions = restored.automation_definitions;
        self.automation_state = restored.automation_state;
        self.runtime_revision = restored.revision;
    }
}

pub fn install_hue_pairing_service_actor<T, J, R>(
    system: &mut ActorSystem,
    actor_id: &str,
    state: HuePairingServiceActorState<T, J, R>,
) -> Result<String, ActorError>
where
    T: HueRegistrationTransport + 'static,
    J: StorageBackend + 'static,
    R: StorageBackend + 'static,
{
    system.create_actor(
        actor_id,
        Box::new(state),
        Box::new(|state: Box<dyn Any>, message| {
            let mut state = *state
                .downcast::<HuePairingServiceActorState<T, J, R>>()
                .expect("Hue pairing actor received the wrong state type");
            match HuePairingRequest::from_message(message) {
                Ok(request) => {
                    let _ = state.pair(request);
                }
                Err(error) => {
                    state.snapshot.request_count = state.snapshot.request_count.saturating_add(1);
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

pub fn vault_record_key(vault_ref: &VaultRef) -> Option<&str> {
    vault_ref.as_str().strip_prefix(HUE_VAULT_REF_PREFIX)
}

fn endpoint_for_bridge(bridge: &Bridge) -> Result<LocalHttpEndpoint, HuePairingServiceError> {
    let address = bridge
        .address
        .as_deref()
        .ok_or_else(|| HuePairingServiceError::MissingBridgeAddress(bridge.bridge_id.clone()))?;
    let url = Url::parse(address).map_err(|error| {
        HuePairingServiceError::InvalidRequest(format!("bridge address is invalid: {error}"))
    })?;
    let scheme = match url.scheme.as_str() {
        "http" => LocalHttpScheme::Http,
        "https" => LocalHttpScheme::Https,
        other => {
            return Err(HuePairingServiceError::InvalidRequest(format!(
                "bridge address scheme `{other}` is unsupported"
            )))
        }
    };
    let host = url
        .host
        .ok_or_else(|| HuePairingServiceError::MissingBridgeAddress(bridge.bridge_id.clone()))?;
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

fn new_transaction_id() -> Result<String, HuePairingServiceError> {
    let random: [u8; 24] =
        random_array().map_err(|error| HuePairingServiceError::Entropy(error.to_string()))?;
    let mut suffix = String::with_capacity(random.len() * 2);
    for byte in random {
        use std::fmt::Write as _;
        write!(&mut suffix, "{byte:02x}").expect("writing to a String cannot fail");
    }
    Ok(format!("hue-{suffix}"))
}

fn new_vault_key(bridge_id: &BridgeId, transaction_id: &str) -> String {
    format!("{}/{transaction_id}", bridge_id.as_str())
}

fn hue_credential_location(
    vault_ref: &VaultRef,
) -> Result<PairingCredentialLocation, HuePairingServiceError> {
    let key = vault_record_key(vault_ref)
        .ok_or_else(|| HuePairingServiceError::ExistingCredentialReference(vault_ref.clone()))?;
    Ok(PairingCredentialLocation::new(
        vault_ref.clone(),
        HUE_VAULT_NAMESPACE,
        key,
    )?)
}

fn encode_http_request(
    url: &Url,
    request: &LocalHttpRequestPlan,
) -> Result<Vec<u8>, HuePairingTransportError> {
    let host = url
        .host
        .as_deref()
        .ok_or(HuePairingTransportError::MissingHost)?;
    let port = url
        .effective_port()
        .ok_or(HuePairingTransportError::MissingHost)?;
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
        return Err(HuePairingTransportError::UnsafeRequest);
    }
    let default_port = match url.scheme.as_str() {
        "http" => 80,
        "https" => 443,
        scheme => {
            return Err(HuePairingTransportError::UnsupportedScheme(
                scheme.to_string(),
            ))
        }
    };
    let host_header = if url.port.is_some() && port != default_port {
        format!("{host}:{port}")
    } else {
        host.to_string()
    };
    let mut bytes = format!(
        "{} {target} HTTP/1.0\r\nHost: {host_header}\r\nConnection: close\r\nContent-Length: {}\r\n",
        request.method.as_str(),
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
) -> Result<TcpStream, HuePairingTransportError> {
    let addresses = (host, port)
        .to_socket_addrs()
        .map_err(HuePairingTransportError::Io)?
        .collect::<Vec<SocketAddr>>();
    let mut last_error = None;
    for address in addresses {
        match TcpStream::connect_timeout(&address, timeout) {
            Ok(stream) => {
                stream
                    .set_read_timeout(Some(timeout))
                    .map_err(HuePairingTransportError::Io)?;
                stream
                    .set_write_timeout(Some(timeout))
                    .map_err(HuePairingTransportError::Io)?;
                return Ok(stream);
            }
            Err(error) => last_error = Some(error),
        }
    }
    Err(HuePairingTransportError::Io(last_error.unwrap_or_else(
        || {
            io::Error::new(
                io::ErrorKind::AddrNotAvailable,
                "host resolved to no addresses",
            )
        },
    )))
}

fn read_bounded(
    reader: &mut dyn Read,
    max_bytes: usize,
) -> Result<Vec<u8>, HuePairingTransportError> {
    let mut bytes = Vec::new();
    reader
        .take(max_bytes as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(HuePairingTransportError::Io)?;
    if bytes.len() > max_bytes {
        return Err(HuePairingTransportError::ResponseTooLarge { limit: max_bytes });
    }
    Ok(bytes)
}

fn decode_http_response(
    bytes: &[u8],
    max_body_bytes: usize,
) -> Result<HueRegistrationResponse, HuePairingTransportError> {
    let parsed = parse_response_head(bytes).map_err(HuePairingTransportError::Http)?;
    let body_bytes = &bytes[parsed.body_offset..];
    let body = match parsed.body_kind {
        BodyKind::None => Vec::new(),
        BodyKind::ContentLength(expected) => {
            if body_bytes.len() < expected {
                return Err(HuePairingTransportError::TruncatedBody {
                    expected,
                    actual: body_bytes.len(),
                });
            }
            body_bytes[..expected].to_vec()
        }
        BodyKind::UntilEof => body_bytes.to_vec(),
        BodyKind::Chunked => decode_chunked_body(body_bytes, max_body_bytes)?,
    };
    if body.len() > max_body_bytes {
        return Err(HuePairingTransportError::ResponseTooLarge {
            limit: max_body_bytes,
        });
    }
    Ok(HueRegistrationResponse {
        status: parsed.head.status,
        headers: parsed.head.headers,
        body,
    })
}

fn decode_chunked_body(
    input: &[u8],
    max_body_bytes: usize,
) -> Result<Vec<u8>, HuePairingTransportError> {
    let mut cursor = 0usize;
    let mut output = Vec::new();
    loop {
        let line_end = input[cursor..]
            .windows(2)
            .position(|window| window == b"\r\n")
            .map(|offset| cursor + offset)
            .ok_or_else(|| {
                HuePairingTransportError::InvalidChunkedBody(
                    "missing chunk-size terminator".to_string(),
                )
            })?;
        let size_text = std::str::from_utf8(&input[cursor..line_end])
            .map_err(|_| {
                HuePairingTransportError::InvalidChunkedBody("chunk size is not ASCII".to_string())
            })?
            .split(';')
            .next()
            .unwrap_or_default();
        let size = usize::from_str_radix(size_text.trim(), 16).map_err(|_| {
            HuePairingTransportError::InvalidChunkedBody("invalid chunk size".to_string())
        })?;
        cursor = line_end + 2;
        if size == 0 {
            return Ok(output);
        }
        if size > max_body_bytes.saturating_sub(output.len()) {
            return Err(HuePairingTransportError::ResponseTooLarge {
                limit: max_body_bytes,
            });
        }
        let end = cursor.checked_add(size).ok_or_else(|| {
            HuePairingTransportError::InvalidChunkedBody("chunk size overflow".to_string())
        })?;
        if end + 2 > input.len() || &input[end..end + 2] != b"\r\n" {
            return Err(HuePairingTransportError::InvalidChunkedBody(
                "truncated chunk payload".to_string(),
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

fn required_json_string(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &str,
) -> Result<String, HuePairingServiceError> {
    object
        .get(field)
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| {
            HuePairingServiceError::InvalidRequest(format!("`{field}` must be a string"))
        })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::{BufRead, BufReader};
    use std::net::TcpListener;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
    use std::thread;

    use coding_adventures_vault_sealed_store::InitOptions;
    use smart_home_core::{
        AgentId, BridgeTransport, CapabilityGrant, CapabilityGrantId, CapabilityId, Health,
        IntegrationId, Metadata, PrivilegeTier, VaultRef,
    };
    use smart_home_runtime::RuntimePairingSession;
    use storage_core::{
        StorageBackend, StorageError, StorageLease, StorageListOptions, StoragePage,
        StoragePutInput, StorageRecord, StorageStat,
    };
    use storage_local_folder::LocalFolderStorageBackend;

    use super::*;

    static TEST_DIRECTORY_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn test_directory(label: &str) -> PathBuf {
        let suffix = TEST_DIRECTORY_COUNTER.fetch_add(1, Ordering::SeqCst);
        std::env::temp_dir().join(format!(
            "smart-home-hue-pairing-service-{}-{label}-{suffix}",
            std::process::id()
        ))
    }

    fn open_vault(root: &Path, password: &[u8]) -> Arc<SealedStore> {
        let backend: Arc<dyn StorageBackend> = Arc::new(LocalFolderStorageBackend::new(root));
        backend.initialize().unwrap();
        let vault = Arc::new(SealedStore::new(backend));
        if vault.status().unwrap().initialized {
            vault.unseal(password).unwrap();
        } else {
            vault
                .init(
                    password,
                    &InitOptions {
                        argon2id_time_cost: 1,
                        argon2id_memory_kib: 32,
                        argon2id_parallelism: 1,
                        salt_override: Some(vec![7; 16]),
                    },
                )
                .unwrap();
        }
        vault
    }

    fn runtime_for_bridge(
        address: String,
        authorized: bool,
        previous_vault_ref: Option<VaultRef>,
    ) -> SmartHomeRuntime {
        let mut runtime = SmartHomeRuntime::new();
        let mut bridge = Bridge::new(
            BridgeId::trusted("001788fffeabcdef"),
            IntegrationId::trusted("hue"),
            BridgeTransport::LanHttp,
        );
        bridge.address = Some(address);
        bridge.health = Health::Unpaired;
        bridge.auth_ref = previous_vault_ref;
        runtime.upsert_bridge(bridge.clone()).unwrap();
        let principal_id = AgentId::trusted("operator");
        runtime
            .start_pairing_session(RuntimePairingSession::pending(
                RuntimePairingSessionId::trusted("pairing-1"),
                &bridge,
                principal_id.clone(),
                1_000,
                30_000,
                vec![Metadata::new("pairing.mode", "physical_presence")],
            ))
            .unwrap();
        if authorized {
            runtime
                .registry_mut()
                .upsert_capability_grant(CapabilityGrant::for_capability(
                    CapabilityGrantId::trusted("grant-hue-pairing"),
                    principal_id,
                    CapabilityId::trusted("smart_home.pair"),
                    PrivilegeTier::HumanApproval,
                    "test",
                    1_000,
                ));
        }
        runtime
    }

    fn runtime_root(root: &Path) -> PathBuf {
        root.join("runtime")
    }

    fn journal_root(root: &Path) -> PathBuf {
        root.join("journal")
    }

    fn persist_runtime(root: &Path, runtime: &SmartHomeRuntime) -> Revision {
        SmartHomeRuntimeStore::new(LocalFolderStorageBackend::new(runtime_root(root)))
            .save(runtime, &[], 1_500)
            .unwrap()
    }

    type LocalService<T> =
        HuePairingServiceActorState<T, LocalFolderStorageBackend, LocalFolderStorageBackend>;

    fn restore_service<T: HueRegistrationTransport>(
        root: &Path,
        vault: Arc<SealedStore>,
        transport: T,
    ) -> Result<LocalService<T>, HuePairingServiceError> {
        HuePairingServiceActorState::restore(
            LocalFolderStorageBackend::new(journal_root(root)),
            vault,
            SmartHomeRuntimeStore::new(LocalFolderStorageBackend::new(runtime_root(root))),
            transport,
        )
    }

    fn request<T: HueRegistrationTransport>(
        service: &LocalService<T>,
        completed_at_ms: u64,
    ) -> HuePairingRequest {
        HuePairingRequest::new(
            RuntimePairingSessionId::trusted("pairing-1"),
            AgentId::trusted("operator"),
            service.runtime_revision().clone(),
            "coding-adventures",
            "test-host",
            completed_at_ms,
        )
        .unwrap()
    }

    fn spawn_registration_server(
        response_body: &'static [u8],
    ) -> (String, thread::JoinHandle<Vec<u8>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let handle = thread::spawn(move || {
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
                    line.trim()
                        .strip_prefix("Content-Length: ")
                        .and_then(|value| value.parse::<usize>().ok())
                })
                .unwrap();
            let mut body = vec![0; content_length];
            reader.read_exact(&mut body).unwrap();
            request.extend_from_slice(&body);
            let stream = reader.get_mut();
            write!(
                stream,
                "HTTP/1.0 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
                response_body.len()
            )
            .unwrap();
            stream.write_all(response_body).unwrap();
            stream.flush().unwrap();
            request
        });
        (format!("http://{address}"), handle)
    }

    #[test]
    fn real_lan_registration_seals_credentials_and_completes_runtime_with_only_vault_ref() {
        let root = test_directory("success");
        let password = b"test-only-vault-password";
        let response =
            br#"[{"success":{"username":"raw-application-key","clientkey":"raw-client-key"}}]"#;
        let (address, server) = spawn_registration_server(response);
        let vault = open_vault(&root.join("vault"), password);
        let runtime = runtime_for_bridge(address, true, None);
        let initial_revision = persist_runtime(&root, &runtime);
        let mut service =
            restore_service(&root, vault.clone(), HueLanRegistrationTransport::default()).unwrap();
        assert_eq!(service.runtime_revision(), &initial_revision);

        let pairing_request = request(&service, 2_000);
        let report = service.pair(pairing_request).unwrap().clone();

        let request = String::from_utf8(server.join().unwrap()).unwrap();
        assert!(request.starts_with("POST /api HTTP/1.0\r\n"));
        assert!(request.contains("\"devicetype\":\"coding-adventures#test-host\""));
        assert!(!report.vault_ref.as_str().contains("raw-application-key"));
        assert!(report.client_key_present);

        let bridge = service
            .runtime()
            .registry()
            .bridge(&BridgeId::trusted("001788fffeabcdef"))
            .unwrap();
        assert_eq!(bridge.auth_ref.as_ref(), Some(&report.vault_ref));
        assert_eq!(bridge.health, Health::Online);
        assert_ne!(service.runtime_revision(), &initial_revision);
        let audit_text = service
            .runtime()
            .registry()
            .events()
            .flat_map(|event| event.metadata.iter())
            .map(|metadata| format!("{}={}", metadata.key, metadata.value))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(!audit_text.contains("raw-application-key"));
        assert!(!audit_text.contains("raw-client-key"));

        vault.seal();
        drop(service);
        drop(vault);
        let reopened = open_vault(&root.join("vault"), password);
        let key = vault_record_key(&report.vault_ref).unwrap();
        let stored = reopened.get(HUE_VAULT_NAMESPACE, key).unwrap().unwrap();
        let stored_json: serde_json::Value = serde_json::from_slice(&stored.plaintext).unwrap();
        assert_eq!(stored_json["application_key"], "raw-application-key");
        assert_eq!(stored_json["client_key"], "raw-client-key");

        fs::remove_dir_all(root).unwrap();
    }

    #[derive(Default)]
    struct FailingTransport;

    impl HueRegistrationTransport for FailingTransport {
        fn execute(
            &mut self,
            _endpoint: &LocalHttpEndpoint,
            _request: &LocalHttpRequestPlan,
        ) -> Result<HueRegistrationResponse, HuePairingTransportError> {
            Err(HuePairingTransportError::Io(io::Error::new(
                io::ErrorKind::ConnectionRefused,
                "fixture refused",
            )))
        }
    }

    struct SuccessfulTransport;

    impl HueRegistrationTransport for SuccessfulTransport {
        fn execute(
            &mut self,
            _endpoint: &LocalHttpEndpoint,
            _request: &LocalHttpRequestPlan,
        ) -> Result<HueRegistrationResponse, HuePairingTransportError> {
            Ok(HueRegistrationResponse {
                status: 200,
                headers: Vec::new(),
                body: br#"[{"success":{"username":"rollback-application-key"}}]"#.to_vec(),
            })
        }
    }

    #[test]
    fn transport_failure_keeps_session_pending_and_writes_no_vault_record() {
        let root = test_directory("failure");
        let password = b"test-only-vault-password";
        let vault = open_vault(&root.join("vault"), password);
        let runtime = runtime_for_bridge("http://127.0.0.1:1".to_string(), true, None);
        persist_runtime(&root, &runtime);
        let mut service = restore_service(&root, vault.clone(), FailingTransport).unwrap();

        let pairing_request = request(&service, 2_000);
        let error = service.pair(pairing_request).unwrap_err();
        assert!(matches!(error, HuePairingServiceError::Transport(_)));
        assert_eq!(service.snapshot().failed_count, 1);
        assert_eq!(
            service
                .runtime()
                .pairing_session(&RuntimePairingSessionId::trusted("pairing-1"))
                .unwrap()
                .status,
            PairingSessionStatus::PendingUserPresence
        );
        assert!(vault
            .list(HUE_VAULT_NAMESPACE, Default::default())
            .unwrap()
            .is_empty());
        assert!(LocalFolderStorageBackend::new(journal_root(&root))
            .list("smart-home-pairing-transactions", Default::default())
            .unwrap()
            .records
            .is_empty());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn authorization_denial_precedes_transport_vault_and_journal_activity() {
        let root = test_directory("denial");
        let password = b"test-only-vault-password";
        let vault = open_vault(&root.join("vault"), password);
        let runtime = runtime_for_bridge("http://127.0.0.1:80".to_string(), false, None);
        persist_runtime(&root, &runtime);
        let calls = Arc::new(AtomicUsize::new(0));
        let mut service = restore_service(
            &root,
            vault.clone(),
            CountingSuccessfulTransport(calls.clone()),
        )
        .unwrap();

        let pairing_request = request(&service, 2_000);
        assert!(matches!(
            service.pair(pairing_request).unwrap_err(),
            HuePairingServiceError::Runtime(_)
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert!(vault
            .list(HUE_VAULT_NAMESPACE, Default::default())
            .unwrap()
            .is_empty());
        assert!(LocalFolderStorageBackend::new(journal_root(&root))
            .list("smart-home-pairing-transactions", Default::default())
            .unwrap()
            .records
            .is_empty());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn actor_request_round_trips_without_credential_fields() {
        let request = HuePairingRequest::new(
            RuntimePairingSessionId::trusted("pairing-1"),
            AgentId::trusted("operator"),
            Revision::new("runtime-r1").unwrap(),
            "coding-adventures",
            "living-room-host",
            2_000,
        )
        .unwrap();
        let message = request.clone().into_message("scheduler").unwrap();
        assert_eq!(HuePairingRequest::from_message(&message).unwrap(), request);
        let payload = String::from_utf8(message.payload).unwrap();
        assert!(!payload.contains("application_key"));
        assert!(!payload.contains("client_key"));
        assert!(!payload.contains("vault_ref"));
        assert!(payload.contains("\"principal_id\":\"operator\""));
        assert!(payload.contains("\"expected_runtime_revision\":\"runtime-r1\""));
    }

    struct CountingSuccessfulTransport(Arc<AtomicUsize>);

    impl HueRegistrationTransport for CountingSuccessfulTransport {
        fn execute(
            &mut self,
            _endpoint: &LocalHttpEndpoint,
            _request: &LocalHttpRequestPlan,
        ) -> Result<HueRegistrationResponse, HuePairingTransportError> {
            self.0.fetch_add(1, Ordering::SeqCst);
            SuccessfulTransport.execute(_endpoint, _request)
        }
    }

    struct FailOnPutBackend {
        inner: LocalFolderStorageBackend,
        fail_on_put: usize,
        put_count: AtomicUsize,
    }

    impl FailOnPutBackend {
        fn new(root: PathBuf, fail_on_put: usize) -> Self {
            Self {
                inner: LocalFolderStorageBackend::new(root),
                fail_on_put,
                put_count: AtomicUsize::new(0),
            }
        }
    }

    impl StorageBackend for FailOnPutBackend {
        fn initialize(&self) -> Result<(), StorageError> {
            self.inner.initialize()
        }

        fn get(&self, namespace: &str, key: &str) -> Result<Option<StorageRecord>, StorageError> {
            self.inner.get(namespace, key)
        }

        fn put(&self, input: StoragePutInput) -> Result<StorageRecord, StorageError> {
            let call = self.put_count.fetch_add(1, Ordering::SeqCst) + 1;
            if call == self.fail_on_put {
                return Err(StorageError::Unavailable {
                    message: "injected journal interruption".to_string(),
                });
            }
            self.inner.put(input)
        }

        fn delete(
            &self,
            namespace: &str,
            key: &str,
            if_revision: Option<&Revision>,
        ) -> Result<(), StorageError> {
            self.inner.delete(namespace, key, if_revision)
        }

        fn list(
            &self,
            namespace: &str,
            options: StorageListOptions,
        ) -> Result<StoragePage, StorageError> {
            self.inner.list(namespace, options)
        }

        fn stat(&self, namespace: &str, key: &str) -> Result<Option<StorageStat>, StorageError> {
            self.inner.stat(namespace, key)
        }

        fn acquire_lease(
            &self,
            name: &str,
            ttl_ms: u64,
        ) -> Result<Option<StorageLease>, StorageError> {
            self.inner.acquire_lease(name, ttl_ms)
        }
    }

    fn transaction_request(
        transaction_id: &str,
        runtime_revision: Revision,
        previous: Option<VaultRef>,
    ) -> PairingTransactionRequest {
        let new_key = format!("001788fffeabcdef/{transaction_id}");
        let new_ref = VaultRef::trusted(format!("{HUE_VAULT_REF_PREFIX}{new_key}"));
        let request = PairingTransactionRequest::new(
            transaction_id,
            AgentId::trusted("operator"),
            BridgeId::trusted("001788fffeabcdef"),
            RuntimePairingSessionId::trusted("pairing-1"),
            PairingCredentialLocation::new(new_ref, HUE_VAULT_NAMESPACE, new_key).unwrap(),
            2_000,
            runtime_revision,
        )
        .unwrap()
        .with_metadata(vec![Metadata::new(
            "hue.pairing.phase",
            "credential_stored",
        )]);
        match previous {
            Some(previous_ref) => {
                request.with_previous_credential(hue_credential_location(&previous_ref).unwrap())
            }
            None => request,
        }
    }

    #[test]
    fn startup_recovers_runtime_commit_interrupted_before_journal_ack() {
        let root = test_directory("restart-recovery");
        let vault = open_vault(&root.join("vault"), b"test-only-vault-password");
        let runtime = runtime_for_bridge("http://127.0.0.1:80".to_string(), true, None);
        let revision = persist_runtime(&root, &runtime);
        let runtime_store =
            SmartHomeRuntimeStore::new(LocalFolderStorageBackend::new(runtime_root(&root)));
        let failing_journal = FailOnPutBackend::new(journal_root(&root), 3);
        let coordinator =
            PairingTransactionCoordinator::new(&failing_journal, &vault, &runtime_store);
        assert!(coordinator
            .execute(
                transaction_request("hue-restart", revision, None),
                b"restart-secret",
            )
            .is_err());

        let service = restore_service(&root, vault.clone(), SuccessfulTransport).unwrap();
        assert_eq!(service.snapshot().recovered_transaction_count, 1);
        assert_eq!(
            service
                .runtime()
                .pairing_session(&RuntimePairingSessionId::trusted("pairing-1"))
                .unwrap()
                .status,
            PairingSessionStatus::Completed
        );
        assert!(LocalFolderStorageBackend::new(journal_root(&root))
            .list("smart-home-pairing-transactions", Default::default())
            .unwrap()
            .records
            .is_empty());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn startup_rolls_back_interrupted_vault_write_after_runtime_revision_changes() {
        let root = test_directory("stale-revision");
        let vault = open_vault(&root.join("vault"), b"test-only-vault-password");
        let runtime = runtime_for_bridge("http://127.0.0.1:80".to_string(), true, None);
        let revision = persist_runtime(&root, &runtime);
        let runtime_store =
            SmartHomeRuntimeStore::new(LocalFolderStorageBackend::new(runtime_root(&root)));
        let failing_journal = FailOnPutBackend::new(journal_root(&root), 2);
        let coordinator =
            PairingTransactionCoordinator::new(&failing_journal, &vault, &runtime_store);
        assert!(coordinator
            .execute(
                transaction_request("hue-stale", revision, None),
                b"stale-secret",
            )
            .is_err());
        let restored = runtime_store.load().unwrap().unwrap();
        runtime_store
            .save_with_automation_state(
                &restored.runtime,
                &restored.automation_definitions,
                restored.automation_state,
                1_900,
            )
            .unwrap();

        let service = restore_service(&root, vault.clone(), SuccessfulTransport).unwrap();
        assert_eq!(service.snapshot().recovered_transaction_count, 1);
        assert_eq!(
            service
                .runtime()
                .pairing_session(&RuntimePairingSessionId::trusted("pairing-1"))
                .unwrap()
                .status,
            PairingSessionStatus::PendingUserPresence
        );
        assert!(vault
            .list(HUE_VAULT_NAMESPACE, Default::default())
            .unwrap()
            .is_empty());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn replacement_commit_removes_only_the_captured_previous_credential() {
        let root = test_directory("replacement");
        let vault = open_vault(&root.join("vault"), b"test-only-vault-password");
        let old_ref =
            VaultRef::trusted("vault://smart-home/hue/001788fffeabcdef/previous-credential");
        vault
            .put(
                HUE_VAULT_NAMESPACE,
                vault_record_key(&old_ref).unwrap(),
                b"previous-secret",
                None,
            )
            .unwrap();
        let runtime = runtime_for_bridge(
            "http://127.0.0.1:80".to_string(),
            true,
            Some(old_ref.clone()),
        );
        persist_runtime(&root, &runtime);
        let mut service = restore_service(&root, vault.clone(), SuccessfulTransport).unwrap();
        let pairing_request = request(&service, 2_000);
        let report = service.pair(pairing_request).unwrap().clone();

        assert_ne!(report.vault_ref, old_ref);
        assert!(vault
            .get(HUE_VAULT_NAMESPACE, vault_record_key(&old_ref).unwrap(),)
            .unwrap()
            .is_none());
        assert_eq!(
            service
                .runtime()
                .registry()
                .bridge(&BridgeId::trusted("001788fffeabcdef"))
                .unwrap()
                .auth_ref,
            Some(report.vault_ref)
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn startup_refuses_cleanup_when_previous_credential_revision_drifted() {
        let root = test_directory("cleanup-drift");
        let vault = open_vault(&root.join("vault"), b"test-only-vault-password");
        let old_ref =
            VaultRef::trusted("vault://smart-home/hue/001788fffeabcdef/previous-credential");
        let old_key = vault_record_key(&old_ref).unwrap().to_string();
        let old_revision = vault
            .put(HUE_VAULT_NAMESPACE, &old_key, b"previous-secret", None)
            .unwrap();
        let runtime = runtime_for_bridge(
            "http://127.0.0.1:80".to_string(),
            true,
            Some(old_ref.clone()),
        );
        let revision = persist_runtime(&root, &runtime);
        let runtime_store =
            SmartHomeRuntimeStore::new(LocalFolderStorageBackend::new(runtime_root(&root)));
        let failing_journal = FailOnPutBackend::new(journal_root(&root), 3);
        let coordinator =
            PairingTransactionCoordinator::new(&failing_journal, &vault, &runtime_store);
        assert!(coordinator
            .execute(
                transaction_request("hue-cleanup", revision, Some(old_ref)),
                b"new-secret",
            )
            .is_err());
        vault
            .put(
                HUE_VAULT_NAMESPACE,
                &old_key,
                b"replacement-owned-secret",
                Some(old_revision),
            )
            .unwrap();

        let recovery_error = match restore_service(&root, vault.clone(), SuccessfulTransport) {
            Ok(_) => panic!("cleanup drift must block actor startup"),
            Err(error) => error,
        };
        assert!(matches!(
            recovery_error,
            HuePairingServiceError::Transaction(_)
        ));
        assert_eq!(
            LocalFolderStorageBackend::new(journal_root(&root))
                .list("smart-home-pairing-transactions", Default::default())
                .unwrap()
                .records
                .len(),
            1
        );
        assert_eq!(
            vault
                .get(HUE_VAULT_NAMESPACE, &old_key)
                .unwrap()
                .unwrap()
                .plaintext
                .as_slice(),
            b"replacement-owned-secret"
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn chunked_registration_response_is_decoded_with_bounds() {
        let response = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n8\r\n[{\"succe\r\n8\r\nss\":{}}]\r\n0\r\n\r\n";
        let decoded = decode_http_response(response, 1024).unwrap();
        assert_eq!(decoded.status, 200);
        assert_eq!(decoded.body, b"[{\"success\":{}}]");
    }

    #[test]
    fn vault_reference_parser_rejects_non_hue_handles() {
        assert_eq!(
            vault_record_key(&VaultRef::trusted("vault://smart-home/hue/bridge/random")),
            Some("bridge/random")
        );
        assert_eq!(
            vault_record_key(&VaultRef::trusted("vault://other/secret")),
            None
        );
    }
}
