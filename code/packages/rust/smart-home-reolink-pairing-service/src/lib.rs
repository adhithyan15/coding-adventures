//! Actor-owned Reolink credential verification and recoverable durable handoff.

#![forbid(unsafe_code)]

use std::any::Any;
use std::collections::BTreeSet;
use std::fmt;
use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use actor::{ActorError, ActorResult, ActorSystem, Message};
use chief_of_staff_daemon_secret_file::{read_owner_only_secret, SecretFileError};
use coding_adventures_csprng::random_array;
use coding_adventures_vault_sealed_store::SealedStore;
use coding_adventures_zeroize::Zeroizing;
use smart_home_core::{
    AgentId, Bridge, BridgeId, CapabilityId, EntityKind, IntegrationId, Metadata, ProtocolFamily,
    VaultRef,
};
use smart_home_pairing_transaction::{
    PairingCredentialLocation, PairingTransactionCoordinator, PairingTransactionError,
    PairingTransactionOutcome, PairingTransactionRequest,
};
use smart_home_reolink_integration::{
    supports_documented_jpeg_snapshot, ReolinkClient, ReolinkConfig, ReolinkCredentials,
    ReolinkError, ReolinkLanTransport, INTEGRATION_ID, PROTOCOL_ID,
};
use smart_home_reolink_snapshot_host::{
    encode_reolink_credentials, ReolinkSnapshotHostError, REOLINK_VAULT_NAMESPACE,
    REOLINK_VAULT_REF_PREFIX,
};
use smart_home_runtime::{
    PairingSessionStatus, RuntimeCompletePairingToolRequest, RuntimeError, RuntimePairingSessionId,
    SmartHomeRuntime,
};
use smart_home_runtime_store::{
    DurableAutomationDefinition, RestoredSmartHomeRuntime, RuntimeStoreError, SmartHomeRuntimeStore,
};
use storage_core::{Revision, StorageBackend};
use url_parser::Url;

pub const PAIR_REQUEST_CONTENT_TYPE: &str =
    "application/vnd.smart-home.reolink-pairing-request+json";
const SERIAL_KIND: &str = "serial";
const CHANNEL_KIND: &str = "channel";

#[derive(Debug)]
pub enum ReolinkPairingServiceError {
    UnknownSession(RuntimePairingSessionId),
    SessionNotPending {
        session_id: RuntimePairingSessionId,
        status: PairingSessionStatus,
    },
    UnknownBridge(BridgeId),
    WrongIntegration(IntegrationId),
    MissingBridgeAddress(BridgeId),
    InvalidHttpsEndpoint(BridgeId),
    InvalidInstalledCamera(BridgeId),
    CameraCorrespondence,
    MissingSnapshotCapability,
    InvalidRequest(String),
    SecretInput(&'static str),
    Reolink(ReolinkError),
    CredentialEncoding(ReolinkSnapshotHostError),
    Runtime(RuntimeError),
    RuntimeStore(RuntimeStoreError),
    Transaction(PairingTransactionError),
    Entropy(String),
    MissingDurableRuntime,
    ExistingCredentialReference(VaultRef),
    TransactionRolledBack(String),
}

impl fmt::Display for ReolinkPairingServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownSession(session_id) => {
                write!(formatter, "unknown Reolink pairing session {session_id}")
            }
            Self::SessionNotPending { session_id, status } => write!(
                formatter,
                "Reolink pairing session {session_id} is not pending user presence ({status:?})"
            ),
            Self::UnknownBridge(bridge_id) => write!(formatter, "unknown Reolink bridge {bridge_id}"),
            Self::WrongIntegration(integration_id) => write!(
                formatter,
                "pairing service only accepts Reolink bridges, got integration {integration_id}"
            ),
            Self::MissingBridgeAddress(bridge_id) => {
                write!(formatter, "Reolink bridge {bridge_id} has no HTTPS address")
            }
            Self::InvalidHttpsEndpoint(bridge_id) => write!(
                formatter,
                "Reolink bridge {bridge_id} does not match the reviewed HTTPS connection target"
            ),
            Self::InvalidInstalledCamera(bridge_id) => write!(
                formatter,
                "Reolink bridge {bridge_id} has an ambiguous installed camera identity"
            ),
            Self::CameraCorrespondence => formatter.write_str(
                "Reolink credential verification did not match the installed camera identity",
            ),
            Self::MissingSnapshotCapability => formatter.write_str(
                "Reolink credential verification did not prove an awake online RLC snapshot channel",
            ),
            Self::InvalidRequest(message) => {
                write!(formatter, "invalid Reolink pairing request: {message}")
            }
            Self::SecretInput(message) => write!(formatter, "Reolink secret input failed: {message}"),
            Self::Reolink(error) => write!(formatter, "Reolink credential verification failed: {error}"),
            Self::CredentialEncoding(_) => {
                formatter.write_str("Reolink credential envelope encoding failed")
            }
            Self::Runtime(error) => write!(formatter, "Reolink runtime completion failed: {error}"),
            Self::RuntimeStore(error) => write!(formatter, "Reolink runtime store failed: {error}"),
            Self::Transaction(error) => write!(formatter, "Reolink pairing transaction failed: {error}"),
            Self::Entropy(message) => write!(
                formatter,
                "Reolink pairing transaction generation failed: {message}"
            ),
            Self::MissingDurableRuntime => {
                formatter.write_str("Reolink pairing requires a durable runtime snapshot")
            }
            Self::ExistingCredentialReference(vault_ref) => write!(
                formatter,
                "existing Reolink credential reference is outside the Reolink Vault namespace: {vault_ref}"
            ),
            Self::TransactionRolledBack(transaction_id) => write!(
                formatter,
                "Reolink pairing transaction {transaction_id} rolled back before runtime commit"
            ),
        }
    }
}

impl std::error::Error for ReolinkPairingServiceError {}

impl From<ReolinkError> for ReolinkPairingServiceError {
    fn from(error: ReolinkError) -> Self {
        Self::Reolink(error)
    }
}

impl From<ReolinkSnapshotHostError> for ReolinkPairingServiceError {
    fn from(error: ReolinkSnapshotHostError) -> Self {
        Self::CredentialEncoding(error)
    }
}

impl From<RuntimeError> for ReolinkPairingServiceError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

impl From<RuntimeStoreError> for ReolinkPairingServiceError {
    fn from(error: RuntimeStoreError) -> Self {
        Self::RuntimeStore(error)
    }
}

impl From<PairingTransactionError> for ReolinkPairingServiceError {
    fn from(error: PairingTransactionError) -> Self {
        Self::Transaction(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReolinkPairingRequest {
    pub session_id: RuntimePairingSessionId,
    pub principal_id: AgentId,
    pub expected_runtime_revision: Revision,
    pub completed_at_ms: u64,
}

impl ReolinkPairingRequest {
    pub fn new(
        session_id: RuntimePairingSessionId,
        principal_id: AgentId,
        expected_runtime_revision: Revision,
        completed_at_ms: u64,
    ) -> Self {
        Self {
            session_id,
            principal_id,
            expected_runtime_revision,
            completed_at_ms,
        }
    }

    pub fn into_message(self, sender_id: &str) -> Result<Message, ReolinkPairingServiceError> {
        let payload = serde_json::to_vec(&serde_json::json!({
            "schema_version": 1,
            "session_id": self.session_id.as_str(),
            "principal_id": self.principal_id.as_str(),
            "expected_runtime_revision": self.expected_runtime_revision.as_str(),
            "completed_at_ms": self.completed_at_ms,
        }))
        .map_err(|error| ReolinkPairingServiceError::InvalidRequest(error.to_string()))?;
        Ok(Message::new(
            sender_id,
            PAIR_REQUEST_CONTENT_TYPE,
            payload,
            None,
        ))
    }

    fn from_message(message: &Message) -> Result<Self, ReolinkPairingServiceError> {
        if message.content_type != PAIR_REQUEST_CONTENT_TYPE {
            return Err(ReolinkPairingServiceError::InvalidRequest(format!(
                "message content type must be `{PAIR_REQUEST_CONTENT_TYPE}`"
            )));
        }
        let value: serde_json::Value = serde_json::from_slice(&message.payload)
            .map_err(|error| ReolinkPairingServiceError::InvalidRequest(error.to_string()))?;
        let object = value.as_object().ok_or_else(|| {
            ReolinkPairingServiceError::InvalidRequest("message body must be an object".to_string())
        })?;
        if object
            .get("schema_version")
            .and_then(serde_json::Value::as_u64)
            != Some(1)
        {
            return Err(ReolinkPairingServiceError::InvalidRequest(
                "unsupported schema_version".to_string(),
            ));
        }
        Ok(Self::new(
            RuntimePairingSessionId::trusted(required_json_string(object, "session_id")?),
            AgentId::new(required_json_string(object, "principal_id")?)
                .map_err(|error| ReolinkPairingServiceError::InvalidRequest(error.to_string()))?,
            Revision::new(required_json_string(object, "expected_runtime_revision")?)
                .map_err(|error| ReolinkPairingServiceError::InvalidRequest(error.to_string()))?,
            object
                .get("completed_at_ms")
                .and_then(serde_json::Value::as_u64)
                .ok_or_else(|| {
                    ReolinkPairingServiceError::InvalidRequest(
                        "completed_at_ms must be a non-negative integer".to_string(),
                    )
                })?,
        ))
    }
}

pub struct ReolinkCredentialSecret {
    username: Zeroizing<String>,
    password: Zeroizing<String>,
}

impl ReolinkCredentialSecret {
    pub fn new(
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Result<Self, ReolinkPairingServiceError> {
        let username = Zeroizing::new(username.into());
        let password = Zeroizing::new(password.into());
        ReolinkCredentials::new(username.as_str(), password.as_str())?;
        Ok(Self { username, password })
    }

    fn username(&self) -> &str {
        self.username.as_str()
    }

    fn password(&self) -> &str {
        self.password.as_str()
    }
}

impl fmt::Debug for ReolinkCredentialSecret {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ReolinkCredentialSecret([REDACTED])")
    }
}

pub trait ReolinkCredentialInput {
    fn take_for_bridge(
        &mut self,
        bridge: &Bridge,
    ) -> Result<ReolinkCredentialSecret, ReolinkPairingServiceError>;
}

pub struct OwnerOnlyReolinkCredentialInput {
    bridge_id: BridgeId,
    username_path: PathBuf,
    username_length: usize,
    password_path: PathBuf,
    password_length: usize,
    consumed: bool,
}

impl OwnerOnlyReolinkCredentialInput {
    pub fn new(
        bridge_id: BridgeId,
        username_path: impl Into<PathBuf>,
        username_length: usize,
        password_path: impl Into<PathBuf>,
        password_length: usize,
    ) -> Self {
        Self {
            bridge_id,
            username_path: username_path.into(),
            username_length,
            password_path: password_path.into(),
            password_length,
            consumed: false,
        }
    }

    fn read_utf8(
        path: &Path,
        length: usize,
    ) -> Result<Zeroizing<String>, ReolinkPairingServiceError> {
        let bytes = read_owner_only_secret(path, length).map_err(map_secret_file_error)?;
        let value = std::str::from_utf8(bytes.as_slice())
            .map_err(|_| ReolinkPairingServiceError::SecretInput("secret is not UTF-8"))?;
        Ok(Zeroizing::new(value.to_string()))
    }
}

impl fmt::Debug for OwnerOnlyReolinkCredentialInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OwnerOnlyReolinkCredentialInput")
            .field("bridge_id", &self.bridge_id)
            .field("paths", &"[REDACTED]")
            .field("consumed", &self.consumed)
            .finish()
    }
}

impl ReolinkCredentialInput for OwnerOnlyReolinkCredentialInput {
    fn take_for_bridge(
        &mut self,
        bridge: &Bridge,
    ) -> Result<ReolinkCredentialSecret, ReolinkPairingServiceError> {
        if bridge.bridge_id != self.bridge_id {
            return Err(ReolinkPairingServiceError::SecretInput(
                "credential input is bound to another bridge",
            ));
        }
        if self.consumed {
            return Err(ReolinkPairingServiceError::SecretInput(
                "credential input was already consumed",
            ));
        }
        let username = Self::read_utf8(&self.username_path, self.username_length)?;
        let password = Self::read_utf8(&self.password_path, self.password_length)?;
        let secret = ReolinkCredentialSecret::new(username.as_str(), password.as_str())?;
        self.consumed = true;
        Ok(secret)
    }
}

fn map_secret_file_error(error: SecretFileError) -> ReolinkPairingServiceError {
    let message = match error {
        SecretFileError::InvalidPath => "secret path is invalid",
        SecretFileError::ParentUnavailable => "secret parent is unavailable",
        SecretFileError::AccessFailed => "secret file is unavailable",
        SecretFileError::UnsafeFileType => "secret file type is unsafe",
        SecretFileError::InsecureOwner => "secret file owner is unsafe",
        SecretFileError::InsecurePermissions => "secret file permissions are unsafe",
        SecretFileError::InvalidLength => "secret length is invalid",
    };
    ReolinkPairingServiceError::SecretInput(message)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedReolinkCamera {
    pub serial_number: String,
    pub channel_count: usize,
    pub snapshot_channel_count: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InstalledReolinkIdentity {
    pub serial_number: Option<String>,
    pub channels: BTreeSet<u32>,
    pub snapshot_channels: BTreeSet<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReolinkPairingConnectionTarget {
    pub bridge_id: BridgeId,
    pub canonical_host: String,
    pub pinned_address: SocketAddr,
}

impl ReolinkPairingConnectionTarget {
    pub fn new(
        bridge_id: BridgeId,
        canonical_host: impl Into<String>,
        pinned_address: SocketAddr,
    ) -> Result<Self, ReolinkPairingServiceError> {
        let canonical_host = canonical_host.into();
        if canonical_host.trim().is_empty()
            || canonical_host.chars().any(char::is_control)
            || canonical_host.contains(['/', '@', '?', '#'])
        {
            return Err(ReolinkPairingServiceError::InvalidHttpsEndpoint(bridge_id));
        }
        Ok(Self {
            bridge_id,
            canonical_host,
            pinned_address,
        })
    }

    fn validate(&self, bridge: &Bridge) -> Result<String, ReolinkPairingServiceError> {
        if bridge.bridge_id != self.bridge_id {
            return Err(ReolinkPairingServiceError::InvalidHttpsEndpoint(
                bridge.bridge_id.clone(),
            ));
        }
        let address = bridge.address.as_deref().ok_or_else(|| {
            ReolinkPairingServiceError::MissingBridgeAddress(bridge.bridge_id.clone())
        })?;
        let parsed = Url::parse(address).map_err(|_| {
            ReolinkPairingServiceError::InvalidHttpsEndpoint(bridge.bridge_id.clone())
        })?;
        let host = parsed.host.as_deref().ok_or_else(|| {
            ReolinkPairingServiceError::InvalidHttpsEndpoint(bridge.bridge_id.clone())
        })?;
        let loopback_test = parsed.scheme == "http"
            && is_loopback_host(host)
            && self.pinned_address.ip().is_loopback();
        if (parsed.scheme != "https" && !loopback_test)
            || !host.eq_ignore_ascii_case(&self.canonical_host)
            || parsed.effective_port() != Some(self.pinned_address.port())
            || !matches!(parsed.path.as_str(), "" | "/")
            || parsed.userinfo.is_some()
            || parsed.query.is_some()
            || parsed.fragment.is_some()
        {
            return Err(ReolinkPairingServiceError::InvalidHttpsEndpoint(
                bridge.bridge_id.clone(),
            ));
        }
        Ok(address.trim_end_matches('/').to_string())
    }
}

pub trait ReolinkPairingVerifier {
    fn preflight(&self, bridge: &Bridge) -> Result<String, ReolinkPairingServiceError>;

    fn verify(
        &mut self,
        bridge: &Bridge,
        credentials: &ReolinkCredentialSecret,
        expected: &InstalledReolinkIdentity,
    ) -> Result<VerifiedReolinkCamera, ReolinkPairingServiceError>;
}

pub struct NativeReolinkPairingVerifier {
    target: ReolinkPairingConnectionTarget,
}

impl NativeReolinkPairingVerifier {
    pub fn new(target: ReolinkPairingConnectionTarget) -> Self {
        Self { target }
    }
}

impl ReolinkPairingVerifier for NativeReolinkPairingVerifier {
    fn preflight(&self, bridge: &Bridge) -> Result<String, ReolinkPairingServiceError> {
        self.target.validate(bridge)
    }

    fn verify(
        &mut self,
        bridge: &Bridge,
        credentials: &ReolinkCredentialSecret,
        expected: &InstalledReolinkIdentity,
    ) -> Result<VerifiedReolinkCamera, ReolinkPairingServiceError> {
        let address = self.target.validate(bridge)?;
        let config = ReolinkConfig::new(
            bridge.bridge_id.clone(),
            address,
            VaultRef::trusted("vault://smart-home/reolink/verification-only"),
        )?;
        let credentials = ReolinkCredentials::new(credentials.username(), credentials.password())?;
        let mut client = ReolinkClient::new(
            config,
            credentials,
            ReolinkLanTransport::default().with_pinned_address(self.target.pinned_address),
        );
        let snapshot = client.inspect()?;
        let serial_number = snapshot.device.serial.trim();
        validate_camera_correspondence(expected.serial_number.as_deref(), serial_number)?;
        let channels = snapshot
            .channels
            .iter()
            .map(|channel| channel.channel)
            .collect::<BTreeSet<_>>();
        if channels.len() != snapshot.channels.len()
            || (!expected.channels.is_empty() && channels != expected.channels)
        {
            return Err(ReolinkPairingServiceError::CameraCorrespondence);
        }
        let snapshot_channels = snapshot
            .channels
            .iter()
            .filter(|channel| supports_documented_jpeg_snapshot(&snapshot.device.model, channel))
            .map(|channel| channel.channel)
            .collect::<BTreeSet<_>>();
        if snapshot_channels.is_empty() {
            return Err(ReolinkPairingServiceError::MissingSnapshotCapability);
        }
        if !expected.snapshot_channels.is_empty() && snapshot_channels != expected.snapshot_channels
        {
            return Err(ReolinkPairingServiceError::CameraCorrespondence);
        }
        Ok(VerifiedReolinkCamera {
            serial_number: serial_number.to_string(),
            channel_count: channels.len(),
            snapshot_channel_count: snapshot_channels.len(),
        })
    }
}

fn validate_camera_correspondence(
    expected_serial_number: Option<&str>,
    observed_serial_number: &str,
) -> Result<(), ReolinkPairingServiceError> {
    if observed_serial_number.is_empty()
        || expected_serial_number.is_some_and(|expected| expected != observed_serial_number)
    {
        Err(ReolinkPairingServiceError::CameraCorrespondence)
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReolinkPairingServiceSnapshot {
    pub request_count: u64,
    pub completed_count: u64,
    pub failed_count: u64,
    pub recovered_transaction_count: u64,
    pub last_completed_at_ms: Option<u64>,
    pub last_bridge_id: Option<BridgeId>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReolinkPairingReport {
    pub session_id: RuntimePairingSessionId,
    pub bridge_id: BridgeId,
    pub vault_ref: VaultRef,
    pub completed_at_ms: u64,
    pub serial_number: String,
    pub channel_count: usize,
    pub snapshot_channel_count: usize,
}

pub struct ReolinkPairingServiceActorState<I, V, J, R> {
    runtime: SmartHomeRuntime,
    automation_definitions: Vec<DurableAutomationDefinition>,
    automation_state: Option<serde_json::Value>,
    runtime_revision: Revision,
    journal_backend: J,
    runtime_store: SmartHomeRuntimeStore<R>,
    vault: Arc<SealedStore>,
    credential_input: I,
    verifier: V,
    snapshot: ReolinkPairingServiceSnapshot,
    last_report: Option<ReolinkPairingReport>,
}

impl<I, V, J, R> ReolinkPairingServiceActorState<I, V, J, R>
where
    I: ReolinkCredentialInput,
    V: ReolinkPairingVerifier,
    J: StorageBackend,
    R: StorageBackend,
{
    pub fn restore(
        journal_backend: J,
        vault: Arc<SealedStore>,
        runtime_store: SmartHomeRuntimeStore<R>,
        credential_input: I,
        verifier: V,
    ) -> Result<Self, ReolinkPairingServiceError> {
        let mut restored = runtime_store
            .load()?
            .ok_or(ReolinkPairingServiceError::MissingDurableRuntime)?;
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
                            .ok_or(ReolinkPairingServiceError::MissingDurableRuntime)?;
                    }
                }
            }
            if !coordinator.pending_transaction_ids()?.is_empty() {
                return Err(ReolinkPairingServiceError::InvalidRequest(
                    "pairing transaction recovery left unresolved journals".to_string(),
                ));
            }
            recovered_count
        };
        Ok(Self {
            runtime: restored.runtime,
            automation_definitions: restored.automation_definitions,
            automation_state: restored.automation_state,
            runtime_revision: restored.revision,
            journal_backend,
            runtime_store,
            vault,
            credential_input,
            verifier,
            snapshot: ReolinkPairingServiceSnapshot {
                recovered_transaction_count,
                ..ReolinkPairingServiceSnapshot::default()
            },
            last_report: None,
        })
    }

    pub fn runtime(&self) -> &SmartHomeRuntime {
        &self.runtime
    }

    pub fn runtime_revision(&self) -> &Revision {
        &self.runtime_revision
    }

    pub fn snapshot(&self) -> &ReolinkPairingServiceSnapshot {
        &self.snapshot
    }

    pub fn last_report(&self) -> Option<&ReolinkPairingReport> {
        self.last_report.as_ref()
    }

    pub fn pair(
        &mut self,
        request: ReolinkPairingRequest,
    ) -> Result<&ReolinkPairingReport, ReolinkPairingServiceError> {
        self.snapshot.request_count = self.snapshot.request_count.saturating_add(1);
        match self.execute_pairing(request) {
            Ok(report) => {
                self.snapshot.completed_count = self.snapshot.completed_count.saturating_add(1);
                self.snapshot.last_completed_at_ms = Some(report.completed_at_ms);
                self.snapshot.last_bridge_id = Some(report.bridge_id.clone());
                self.snapshot.last_error = None;
                self.last_report = Some(report);
                Ok(self
                    .last_report
                    .as_ref()
                    .expect("pairing report was assigned"))
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
        request: ReolinkPairingRequest,
    ) -> Result<ReolinkPairingReport, ReolinkPairingServiceError> {
        let session = self
            .runtime
            .pairing_session(&request.session_id)
            .cloned()
            .ok_or_else(|| {
                ReolinkPairingServiceError::UnknownSession(request.session_id.clone())
            })?;
        if session.status != PairingSessionStatus::PendingUserPresence {
            return Err(ReolinkPairingServiceError::SessionNotPending {
                session_id: session.session_id,
                status: session.status,
            });
        }
        let bridge = self
            .runtime
            .registry()
            .bridge(&session.bridge_id)
            .cloned()
            .ok_or_else(|| ReolinkPairingServiceError::UnknownBridge(session.bridge_id.clone()))?;
        if bridge.integration_id.as_str() != INTEGRATION_ID {
            return Err(ReolinkPairingServiceError::WrongIntegration(
                bridge.integration_id,
            ));
        }
        let https_endpoint = self.verifier.preflight(&bridge)?;
        let expected_identity = installed_camera_identity(&self.runtime, &bridge)?;
        ReolinkConfig::new(
            bridge.bridge_id.clone(),
            https_endpoint.as_str(),
            VaultRef::trusted("vault://smart-home/reolink/validation-only"),
        )?;
        if request.expected_runtime_revision != self.runtime_revision {
            return Err(ReolinkPairingServiceError::InvalidRequest(
                "expected runtime revision is stale".to_string(),
            ));
        }

        let authorization_probe = RuntimeCompletePairingToolRequest::new(
            request.session_id.clone(),
            VaultRef::trusted("vault://smart-home/reolink/authorization-preflight"),
            request.completed_at_ms,
        );
        self.runtime.clone().execute_complete_pairing_tool(
            request.principal_id.clone(),
            authorization_probe,
            request.completed_at_ms,
        )?;

        let credentials = self.credential_input.take_for_bridge(&bridge)?;
        let verified = self
            .verifier
            .verify(&bridge, &credentials, &expected_identity)?;
        let payload = encode_reolink_credentials(credentials.username(), credentials.password())?;
        let transaction_id = new_transaction_id()?;
        let vault_key = format!("{}/{}", bridge.bridge_id.as_str(), transaction_id);
        let vault_ref = VaultRef::trusted(format!("{REOLINK_VAULT_REF_PREFIX}{vault_key}"));
        let new_credential =
            PairingCredentialLocation::new(vault_ref.clone(), REOLINK_VAULT_NAMESPACE, vault_key)?;
        let previous_credential = bridge
            .auth_ref
            .as_ref()
            .map(reolink_credential_location)
            .transpose()?;
        let transaction = PairingTransactionRequest::new(
            transaction_id.clone(),
            request.principal_id,
            bridge.bridge_id.clone(),
            request.session_id.clone(),
            new_credential,
            request.completed_at_ms,
            request.expected_runtime_revision,
        )?
        .with_metadata(vec![
            Metadata::new("reolink.pairing.verified", "true"),
            Metadata::new("reolink.pairing.https_endpoint", https_endpoint),
            Metadata::new(
                "reolink.pairing.serial_number",
                verified.serial_number.clone(),
            ),
            Metadata::new(
                "reolink.pairing.channel_count",
                verified.channel_count.to_string(),
            ),
            Metadata::new(
                "reolink.pairing.snapshot_channel_count",
                verified.snapshot_channel_count.to_string(),
            ),
        ]);
        let transaction = match previous_credential {
            Some(previous) => transaction.with_previous_credential(previous),
            None => transaction,
        };
        let outcome = PairingTransactionCoordinator::new(
            &self.journal_backend,
            &self.vault,
            &self.runtime_store,
        )
        .execute(transaction, payload.as_bytes())?;
        let PairingTransactionOutcome::Committed { restored, .. } = outcome else {
            return Err(ReolinkPairingServiceError::TransactionRolledBack(
                transaction_id,
            ));
        };
        self.install_restored_runtime(*restored);
        Ok(ReolinkPairingReport {
            session_id: request.session_id,
            bridge_id: bridge.bridge_id,
            vault_ref,
            completed_at_ms: request.completed_at_ms,
            serial_number: verified.serial_number,
            channel_count: verified.channel_count,
            snapshot_channel_count: verified.snapshot_channel_count,
        })
    }

    fn install_restored_runtime(&mut self, restored: RestoredSmartHomeRuntime) {
        self.runtime = restored.runtime;
        self.automation_definitions = restored.automation_definitions;
        self.automation_state = restored.automation_state;
        self.runtime_revision = restored.revision;
    }
}

pub fn install_reolink_pairing_service_actor<I, V, J, R>(
    system: &mut ActorSystem,
    actor_id: &str,
    state: ReolinkPairingServiceActorState<I, V, J, R>,
) -> Result<String, ActorError>
where
    I: ReolinkCredentialInput + 'static,
    V: ReolinkPairingVerifier + 'static,
    J: StorageBackend + 'static,
    R: StorageBackend + 'static,
{
    system.create_actor(
        actor_id,
        Box::new(state),
        Box::new(|state: Box<dyn Any>, message| {
            let mut state = *state
                .downcast::<ReolinkPairingServiceActorState<I, V, J, R>>()
                .expect("Reolink pairing actor received the wrong state type");
            match ReolinkPairingRequest::from_message(message) {
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
    vault_ref.as_str().strip_prefix(REOLINK_VAULT_REF_PREFIX)
}

fn installed_camera_identity(
    runtime: &SmartHomeRuntime,
    bridge: &Bridge,
) -> Result<InstalledReolinkIdentity, ReolinkPairingServiceError> {
    let expected_family = ProtocolFamily::Vendor(PROTOCOL_ID.to_string());
    let bridge_serials = bridge
        .identifiers
        .iter()
        .filter(|identifier| identifier.family == expected_family && identifier.kind == SERIAL_KIND)
        .collect::<Vec<_>>();
    if bridge_serials.len() > 1
        || bridge_serials
            .first()
            .is_some_and(|identifier| identifier.value.trim().is_empty())
    {
        return Err(ReolinkPairingServiceError::InvalidInstalledCamera(
            bridge.bridge_id.clone(),
        ));
    }
    let serial_number = bridge_serials
        .first()
        .map(|identifier| identifier.value.clone());
    let devices = runtime
        .registry()
        .devices_for_bridge(&bridge.bridge_id)
        .collect::<Vec<_>>();
    if devices.is_empty() {
        return Ok(InstalledReolinkIdentity {
            serial_number,
            ..InstalledReolinkIdentity::default()
        });
    }
    let mut channels = BTreeSet::new();
    let mut snapshot_channels = BTreeSet::new();
    for device in devices {
        let channel_ids = device
            .identifiers
            .iter()
            .filter(|identifier| {
                identifier.family == expected_family && identifier.kind == CHANNEL_KIND
            })
            .collect::<Vec<_>>();
        let channel = channel_ids
            .first()
            .filter(|_| channel_ids.len() == 1)
            .and_then(|identifier| identifier.value.parse::<u32>().ok())
            .filter(|channel| channels.insert(*channel))
            .ok_or_else(|| {
                ReolinkPairingServiceError::InvalidInstalledCamera(bridge.bridge_id.clone())
            })?;
        let cameras = device
            .entity_ids
            .iter()
            .filter_map(|entity_id| runtime.registry().entity(entity_id))
            .filter(|entity| entity.kind == EntityKind::Camera)
            .collect::<Vec<_>>();
        if cameras.len() != 1
            || !cameras[0].metadata.iter().any(|metadata| {
                metadata.key == "reolink.channel" && metadata.value == channel.to_string()
            })
        {
            return Err(ReolinkPairingServiceError::InvalidInstalledCamera(
                bridge.bridge_id.clone(),
            ));
        }
        if cameras[0]
            .capabilities
            .iter()
            .any(|capability| capability.capability_id == CapabilityId::trusted("camera.snapshot"))
        {
            snapshot_channels.insert(channel);
        }
    }
    if snapshot_channels.is_empty() {
        return Err(ReolinkPairingServiceError::InvalidInstalledCamera(
            bridge.bridge_id.clone(),
        ));
    }
    Ok(InstalledReolinkIdentity {
        serial_number,
        channels,
        snapshot_channels,
    })
}

fn is_loopback_host(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<IpAddr>()
            .is_ok_and(|address| address.is_loopback())
}

fn reolink_credential_location(
    vault_ref: &VaultRef,
) -> Result<PairingCredentialLocation, ReolinkPairingServiceError> {
    let key = vault_record_key(vault_ref).ok_or_else(|| {
        ReolinkPairingServiceError::ExistingCredentialReference(vault_ref.clone())
    })?;
    Ok(PairingCredentialLocation::new(
        vault_ref.clone(),
        REOLINK_VAULT_NAMESPACE,
        key,
    )?)
}

fn new_transaction_id() -> Result<String, ReolinkPairingServiceError> {
    let random: [u8; 24] =
        random_array().map_err(|error| ReolinkPairingServiceError::Entropy(error.to_string()))?;
    let mut suffix = String::with_capacity(random.len() * 2);
    for byte in random {
        use std::fmt::Write as _;
        write!(&mut suffix, "{byte:02x}").expect("writing to a String cannot fail");
    }
    Ok(format!("reolink-{suffix}"))
}

fn required_json_string(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &str,
) -> Result<String, ReolinkPairingServiceError> {
    object
        .get(field)
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| {
            ReolinkPairingServiceError::InvalidRequest(format!("`{field}` must be a string"))
        })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
    use std::sync::Mutex;
    use std::thread;

    use coding_adventures_vault_sealed_store::InitOptions;
    use smart_home_core::{
        BridgeTransport, Capability, CapabilityGrant, CapabilityGrantId, CapabilityId,
        CapabilityMode, Device, DeviceId, Entity, Health, PrivilegeTier, ProtocolIdentifier,
        ValueKind,
    };
    use smart_home_runtime::RuntimePairingSession;
    use storage_core::{
        StorageError, StorageLease, StorageListOptions, StoragePage, StoragePutInput,
        StorageRecord, StorageStat,
    };
    use storage_local_folder::LocalFolderStorageBackend;

    use super::*;

    static TEST_DIRECTORY_COUNTER: AtomicU64 = AtomicU64::new(0);
    fn test_directory(label: &str) -> PathBuf {
        let suffix = TEST_DIRECTORY_COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!(
            "smart-home-reolink-pairing-service-{}-{label}-{suffix}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        fs::canonicalize(path).unwrap()
    }

    fn open_vault(root: &Path) -> Arc<SealedStore> {
        let backend: Arc<dyn StorageBackend> = Arc::new(LocalFolderStorageBackend::new(root));
        backend.initialize().unwrap();
        let vault = Arc::new(SealedStore::new(backend));
        vault
            .init(
                b"test-only-vault-password",
                &InitOptions {
                    argon2id_time_cost: 1,
                    argon2id_memory_kib: 32,
                    argon2id_parallelism: 1,
                    salt_override: Some(vec![0x31; 16]),
                },
            )
            .unwrap();
        vault
    }

    fn runtime_for_bridge(authorized: bool, previous: Option<VaultRef>) -> SmartHomeRuntime {
        let mut runtime = SmartHomeRuntime::new();
        let mut bridge = Bridge::new(
            BridgeId::trusted("reolink-camera-front"),
            IntegrationId::trusted(INTEGRATION_ID),
            BridgeTransport::LanHttp,
        );
        bridge.address = Some("https://reolink.local".to_string());
        bridge.health = Health::Unpaired;
        bridge.auth_ref = previous;
        runtime.upsert_bridge(bridge.clone()).unwrap();
        let principal = AgentId::trusted("operator");
        runtime
            .start_pairing_session(RuntimePairingSession::pending(
                RuntimePairingSessionId::trusted("reolink-pairing-1"),
                &bridge,
                principal.clone(),
                1_000,
                30_000,
                vec![Metadata::new("pairing.mode", "explicit_credentials")],
            ))
            .unwrap();
        if authorized {
            runtime
                .registry_mut()
                .upsert_capability_grant(CapabilityGrant::for_capability(
                    CapabilityGrantId::trusted("grant-reolink-pairing"),
                    principal,
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

    struct FixedInput {
        calls: Arc<AtomicUsize>,
        username: &'static str,
        password: &'static str,
    }

    impl ReolinkCredentialInput for FixedInput {
        fn take_for_bridge(
            &mut self,
            bridge: &Bridge,
        ) -> Result<ReolinkCredentialSecret, ReolinkPairingServiceError> {
            assert_eq!(bridge.bridge_id.as_str(), "reolink-camera-front");
            self.calls.fetch_add(1, Ordering::SeqCst);
            ReolinkCredentialSecret::new(self.username, self.password)
        }
    }

    struct ExactVerifier {
        calls: Arc<AtomicUsize>,
    }

    impl ReolinkPairingVerifier for ExactVerifier {
        fn preflight(&self, bridge: &Bridge) -> Result<String, ReolinkPairingServiceError> {
            match bridge.address.as_deref() {
                Some("https://reolink.local") => Ok("https://reolink.local".to_string()),
                _ => Err(ReolinkPairingServiceError::InvalidHttpsEndpoint(
                    bridge.bridge_id.clone(),
                )),
            }
        }

        fn verify(
            &mut self,
            bridge: &Bridge,
            credentials: &ReolinkCredentialSecret,
            expected: &InstalledReolinkIdentity,
        ) -> Result<VerifiedReolinkCamera, ReolinkPairingServiceError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            assert_eq!(bridge.address.as_deref(), Some("https://reolink.local"));
            assert_eq!(expected, &InstalledReolinkIdentity::default());
            assert_eq!(credentials.username(), "camera-user");
            assert_eq!(credentials.password(), "camera-password");
            assert_eq!(
                format!("{credentials:?}"),
                "ReolinkCredentialSecret([REDACTED])"
            );
            Ok(VerifiedReolinkCamera {
                serial_number: "ACCC8EAF8C30".to_string(),
                channel_count: 1,
                snapshot_channel_count: 1,
            })
        }
    }

    type LocalService = ReolinkPairingServiceActorState<
        FixedInput,
        ExactVerifier,
        LocalFolderStorageBackend,
        LocalFolderStorageBackend,
    >;

    fn restore_service(
        root: &Path,
        vault: Arc<SealedStore>,
        input_calls: Arc<AtomicUsize>,
        verifier_calls: Arc<AtomicUsize>,
    ) -> Result<LocalService, ReolinkPairingServiceError> {
        ReolinkPairingServiceActorState::restore(
            LocalFolderStorageBackend::new(journal_root(root)),
            vault,
            SmartHomeRuntimeStore::new(LocalFolderStorageBackend::new(runtime_root(root))),
            FixedInput {
                calls: input_calls,
                username: "camera-user",
                password: "camera-password",
            },
            ExactVerifier {
                calls: verifier_calls,
            },
        )
    }

    fn request(service: &LocalService) -> ReolinkPairingRequest {
        ReolinkPairingRequest::new(
            RuntimePairingSessionId::trusted("reolink-pairing-1"),
            AgentId::trusted("operator"),
            service.runtime_revision().clone(),
            2_000,
        )
    }

    #[test]
    fn authorized_pairing_verifies_exact_bridge_and_seals_snapshot_envelope() {
        let root = test_directory("success");
        let vault = open_vault(&root.join("vault"));
        let initial_revision = persist_runtime(&root, &runtime_for_bridge(true, None));
        let input_calls = Arc::new(AtomicUsize::new(0));
        let verifier_calls = Arc::new(AtomicUsize::new(0));
        let mut service = restore_service(
            &root,
            vault.clone(),
            input_calls.clone(),
            verifier_calls.clone(),
        )
        .unwrap();

        let pairing_request = request(&service);
        let report = service.pair(pairing_request).unwrap().clone();

        assert_eq!(input_calls.load(Ordering::SeqCst), 1);
        assert_eq!(verifier_calls.load(Ordering::SeqCst), 1);
        assert_eq!(report.serial_number, "ACCC8EAF8C30");
        assert_eq!(report.channel_count, 1);
        assert_eq!(report.snapshot_channel_count, 1);
        assert_ne!(service.runtime_revision(), &initial_revision);
        assert_eq!(
            service
                .runtime()
                .registry()
                .bridge(&BridgeId::trusted("reolink-camera-front"))
                .unwrap()
                .auth_ref,
            Some(report.vault_ref.clone())
        );
        let record = vault
            .get(
                REOLINK_VAULT_NAMESPACE,
                vault_record_key(&report.vault_ref).unwrap(),
            )
            .unwrap()
            .unwrap();
        let envelope: serde_json::Value = serde_json::from_slice(&record.plaintext).unwrap();
        assert_eq!(envelope["schema_version"], 1);
        assert_eq!(envelope["username"], "camera-user");
        assert_eq!(envelope["password"], "camera-password");
        let durable_text = service
            .runtime()
            .registry()
            .events()
            .flat_map(|event| event.metadata.iter())
            .map(|metadata| format!("{}={}", metadata.key, metadata.value))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(!durable_text.contains("camera-user"));
        assert!(!durable_text.contains("camera-password"));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn denial_precedes_secret_input_verification_vault_and_journal_writes() {
        let root = test_directory("denial");
        let vault = open_vault(&root.join("vault"));
        persist_runtime(&root, &runtime_for_bridge(false, None));
        let input_calls = Arc::new(AtomicUsize::new(0));
        let verifier_calls = Arc::new(AtomicUsize::new(0));
        let mut service = restore_service(
            &root,
            vault.clone(),
            input_calls.clone(),
            verifier_calls.clone(),
        )
        .unwrap();

        let pairing_request = request(&service);
        assert!(matches!(
            service.pair(pairing_request).unwrap_err(),
            ReolinkPairingServiceError::Runtime(_)
        ));
        assert_eq!(input_calls.load(Ordering::SeqCst), 0);
        assert_eq!(verifier_calls.load(Ordering::SeqCst), 0);

        let mut different_target = service
            .runtime()
            .registry()
            .bridge(&BridgeId::trusted("reolink-camera-front"))
            .unwrap()
            .clone();
        different_target.address = Some("https://other-reolink.local".to_string());
        service.runtime.upsert_bridge(different_target).unwrap();
        let current = request(&service);
        assert!(matches!(
            service.pair(current).unwrap_err(),
            ReolinkPairingServiceError::InvalidHttpsEndpoint(_)
        ));
        assert_eq!(input_calls.load(Ordering::SeqCst), 0);
        assert_eq!(verifier_calls.load(Ordering::SeqCst), 0);
        assert!(vault
            .list(REOLINK_VAULT_NAMESPACE, Default::default())
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
    fn stale_revision_and_ambiguous_identity_fail_before_secret_input() {
        let root = test_directory("preflight");
        let vault = open_vault(&root.join("vault"));
        persist_runtime(&root, &runtime_for_bridge(true, None));
        let input_calls = Arc::new(AtomicUsize::new(0));
        let verifier_calls = Arc::new(AtomicUsize::new(0));
        let mut service =
            restore_service(&root, vault, input_calls.clone(), verifier_calls.clone()).unwrap();
        let stale = ReolinkPairingRequest::new(
            RuntimePairingSessionId::trusted("reolink-pairing-1"),
            AgentId::trusted("operator"),
            Revision::new("stale-runtime").unwrap(),
            2_000,
        );
        assert!(matches!(
            service.pair(stale).unwrap_err(),
            ReolinkPairingServiceError::InvalidRequest(_)
        ));
        assert_eq!(input_calls.load(Ordering::SeqCst), 0);
        assert_eq!(verifier_calls.load(Ordering::SeqCst), 0);

        let mut embedded_credentials = service
            .runtime()
            .registry()
            .bridge(&BridgeId::trusted("reolink-camera-front"))
            .unwrap()
            .clone();
        embedded_credentials.address = Some("https://operator:password@reolink.local".to_string());
        service.runtime.upsert_bridge(embedded_credentials).unwrap();
        let current = request(&service);
        assert!(matches!(
            service.pair(current).unwrap_err(),
            ReolinkPairingServiceError::InvalidHttpsEndpoint(_)
        ));
        assert_eq!(input_calls.load(Ordering::SeqCst), 0);
        assert_eq!(verifier_calls.load(Ordering::SeqCst), 0);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn installed_camera_identity_is_exact_and_capability_bound() {
        let mut runtime = runtime_for_bridge(true, None);
        let mut bridge = runtime
            .registry()
            .bridge(&BridgeId::trusted("reolink-camera-front"))
            .unwrap()
            .clone();
        bridge.identifiers.push(
            ProtocolIdentifier::new(
                ProtocolFamily::Vendor(PROTOCOL_ID.to_string()),
                SERIAL_KIND,
                "ACCC8EAF8C30",
            )
            .unwrap(),
        );
        runtime.upsert_bridge(bridge.clone()).unwrap();
        let camera_entity_id =
            smart_home_core::EntityId::trusted("reolink:ACCC8EAF8C30:ch0:camera");
        runtime
            .upsert_device(Device {
                device_id: DeviceId::trusted("reolink:ACCC8EAF8C30:ch0"),
                bridge_id: bridge.bridge_id.clone(),
                manufacturer: "Reolink".to_string(),
                model: "RLC-520A".to_string(),
                name: "Front".to_string(),
                serial: Some("ACCC8EAF8C30".to_string()),
                firmware_version: None,
                room_id: None,
                entity_ids: vec![camera_entity_id.clone()],
                identifiers: vec![ProtocolIdentifier::new(
                    ProtocolFamily::Vendor(PROTOCOL_ID.to_string()),
                    CHANNEL_KIND,
                    "0",
                )
                .unwrap()],
                health: Health::Online,
                metadata: Vec::new(),
            })
            .unwrap();
        runtime
            .upsert_entity(Entity {
                entity_id: camera_entity_id,
                device_id: DeviceId::trusted("reolink:ACCC8EAF8C30:ch0"),
                kind: EntityKind::Camera,
                name: "Front".to_string(),
                capabilities: vec![Capability::new(
                    CapabilityId::trusted("camera.snapshot"),
                    CapabilityMode::Command,
                    ValueKind::Text,
                )],
                state: None,
                metadata: vec![Metadata::new("reolink.channel", "0")],
            })
            .unwrap();
        let expected = installed_camera_identity(&runtime, &bridge).unwrap();
        assert_eq!(expected.serial_number.as_deref(), Some("ACCC8EAF8C30"));
        assert_eq!(expected.channels, BTreeSet::from([0]));
        assert_eq!(expected.snapshot_channels, BTreeSet::from([0]));

        let mut invalid = runtime
            .registry()
            .device(&DeviceId::trusted("reolink:ACCC8EAF8C30:ch0"))
            .unwrap()
            .clone();
        invalid.identifiers[0].value.clear();
        runtime.upsert_device(invalid).unwrap();
        assert!(matches!(
            installed_camera_identity(&runtime, &bridge).unwrap_err(),
            ReolinkPairingServiceError::InvalidInstalledCamera(_)
        ));
    }

    #[test]
    fn camera_correspondence_requires_a_nonempty_exact_serial() {
        assert!(validate_camera_correspondence(None, "").is_err());
        assert!(validate_camera_correspondence(None, "ACCC8EAF8C30").is_ok());
        assert!(validate_camera_correspondence(Some("ACCC8EAF8C30"), "ACCC8EAF8C30").is_ok());
        assert!(validate_camera_correspondence(Some("different"), "ACCC8EAF8C30").is_err());
    }

    #[test]
    fn native_verifier_uses_authenticated_cgi_and_exact_camera_one_inspection() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let server_requests = Arc::clone(&requests);
        let response_bodies = [
            r#"[{"cmd":"Login","code":0,"value":{"Token":{"name":"token123","leaseTime":3600}}}]"#,
            r#"[{"cmd":"GetDevInfo","code":0,"value":{"DevInfo":{"name":"Front Camera","model":"RLC-520A","serial":"ACCC8EAF8C30","firmVer":"v3.5.0","hardVer":"IPC"}}}]"#,
            r#"[{"cmd":"GetChannelstatus","code":0,"value":{"status":[{"channel":0,"name":"Porch","online":1,"sleep":0}]}}]"#,
            r#"[{"cmd":"GetMdState","code":1,"error":{"rspCode":-1,"detail":"unsupported"}}]"#,
            r#"[{"cmd":"GetRecV20","code":1,"error":{"rspCode":-1,"detail":"unsupported"}}]"#,
            r#"[{"cmd":"GetPtzPreset","code":1,"error":{"rspCode":-1,"detail":"unsupported"}}]"#,
            r#"[{"cmd":"Logout","code":0,"value":{}}]"#,
        ];
        let handle = thread::spawn(move || {
            for body in response_bodies {
                let (stream, _) = listener.accept().unwrap();
                let mut reader = BufReader::new(stream);
                let mut head = String::new();
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    if line == "\r\n" {
                        break;
                    }
                    head.push_str(&line);
                }
                let length = head
                    .lines()
                    .find_map(|line| line.strip_prefix("Content-Length: "))
                    .unwrap_or("0")
                    .parse::<usize>()
                    .unwrap();
                let mut request_body = vec![0u8; length];
                reader.read_exact(&mut request_body).unwrap();
                server_requests
                    .lock()
                    .unwrap()
                    .push((head, String::from_utf8(request_body).unwrap()));
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                reader.get_mut().write_all(response.as_bytes()).unwrap();
            }
        });

        let mut bridge = Bridge::new(
            BridgeId::trusted("reolink-camera-front"),
            IntegrationId::trusted(INTEGRATION_ID),
            BridgeTransport::LanHttp,
        );
        bridge.address = Some(format!("http://{address}"));
        let credentials = ReolinkCredentialSecret::new("operator name", "secret&password").unwrap();
        let target =
            ReolinkPairingConnectionTarget::new(bridge.bridge_id.clone(), "127.0.0.1", address)
                .unwrap();
        let mut verifier = NativeReolinkPairingVerifier::new(target);
        assert_eq!(
            verifier.preflight(&bridge).unwrap(),
            format!("http://{address}")
        );
        let verified = verifier
            .verify(&bridge, &credentials, &InstalledReolinkIdentity::default())
            .unwrap();
        handle.join().unwrap();

        assert_eq!(verified.serial_number, "ACCC8EAF8C30");
        assert_eq!(verified.channel_count, 1);
        assert_eq!(verified.snapshot_channel_count, 1);
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 7);
        assert!(requests[0]
            .0
            .starts_with("POST /api.cgi?cmd=Login HTTP/1.1"));
        assert!(requests[0].1.contains("\"userName\":\"operator name\""));
        assert!(requests[0].1.contains("\"password\":\"secret&password\""));
        assert!(requests[2]
            .0
            .starts_with("POST /api.cgi?cmd=GetChannelstatus&token=token123 HTTP/1.1"));
        assert!(requests[6]
            .0
            .starts_with("POST /api.cgi?cmd=Logout&token=token123 HTTP/1.1"));
        assert!(!format!("{verified:?}").contains("secret&password"));
    }

    #[test]
    fn native_verifier_logs_out_after_authenticated_inspection_failure() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let server_requests = Arc::clone(&requests);
        let response_bodies = [
            r#"[{"cmd":"Login","code":0,"value":{"Token":{"name":"token123","leaseTime":3600}}}]"#,
            r#"[{"cmd":"GetDevInfo","code":1,"error":{"rspCode":-1,"detail":"inspection failed"}}]"#,
            r#"[{"cmd":"Logout","code":0,"value":{}}]"#,
        ];
        let handle = thread::spawn(move || {
            for body in response_bodies {
                let (stream, _) = listener.accept().unwrap();
                let mut reader = BufReader::new(stream);
                let mut head = String::new();
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    if line == "\r\n" {
                        break;
                    }
                    head.push_str(&line);
                }
                let length = head
                    .lines()
                    .find_map(|line| line.strip_prefix("Content-Length: "))
                    .unwrap_or("0")
                    .parse::<usize>()
                    .unwrap();
                let mut request_body = vec![0u8; length];
                reader.read_exact(&mut request_body).unwrap();
                server_requests.lock().unwrap().push(head);
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                reader.get_mut().write_all(response.as_bytes()).unwrap();
            }
        });

        let mut bridge = Bridge::new(
            BridgeId::trusted("reolink-camera-front"),
            IntegrationId::trusted(INTEGRATION_ID),
            BridgeTransport::LanHttp,
        );
        bridge.address = Some(format!("http://{address}"));
        let target =
            ReolinkPairingConnectionTarget::new(bridge.bridge_id.clone(), "127.0.0.1", address)
                .unwrap();
        let mut verifier = NativeReolinkPairingVerifier::new(target);
        let credentials = ReolinkCredentialSecret::new("operator", "password").unwrap();
        assert!(matches!(
            verifier.verify(&bridge, &credentials, &InstalledReolinkIdentity::default()),
            Err(ReolinkPairingServiceError::Reolink(
                ReolinkError::Api { .. }
            ))
        ));
        handle.join().unwrap();

        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 3);
        assert!(requests[2].starts_with("POST /api.cgi?cmd=Logout&token=token123 HTTP/1.1"));
    }

    #[test]
    fn actor_message_contains_authority_and_revision_but_no_secret_input() {
        let request = ReolinkPairingRequest::new(
            RuntimePairingSessionId::trusted("reolink-pairing-1"),
            AgentId::trusted("operator"),
            Revision::new("runtime-r1").unwrap(),
            2_000,
        );
        let message = request.clone().into_message("scheduler").unwrap();
        assert_eq!(
            ReolinkPairingRequest::from_message(&message).unwrap(),
            request
        );
        let payload = String::from_utf8(message.payload).unwrap();
        assert!(payload.contains("\"principal_id\":\"operator\""));
        assert!(payload.contains("\"expected_runtime_revision\":\"runtime-r1\""));
        assert!(!payload.contains("username"));
        assert!(!payload.contains("password"));
        assert!(!payload.contains("path"));
        assert!(!payload.contains("vault_ref"));
    }

    #[cfg(unix)]
    #[test]
    fn owner_only_input_is_exact_length_bridge_bound_and_one_shot() {
        use std::os::unix::fs::PermissionsExt;

        let root = test_directory("owner-input");
        let username_path = root.join("username");
        let password_path = root.join("password");
        fs::write(&username_path, b"camera-user").unwrap();
        fs::write(&password_path, b"camera-password").unwrap();
        fs::set_permissions(&username_path, fs::Permissions::from_mode(0o600)).unwrap();
        fs::set_permissions(&password_path, fs::Permissions::from_mode(0o600)).unwrap();
        let bridge = runtime_for_bridge(true, None)
            .registry()
            .bridge(&BridgeId::trusted("reolink-camera-front"))
            .unwrap()
            .clone();
        let mut input = OwnerOnlyReolinkCredentialInput::new(
            bridge.bridge_id.clone(),
            &username_path,
            11,
            &password_path,
            15,
        );
        let mut other_bridge = bridge.clone();
        other_bridge.bridge_id = BridgeId::trusted("reolink-camera-other");
        assert!(matches!(
            input.take_for_bridge(&other_bridge).unwrap_err(),
            ReolinkPairingServiceError::SecretInput("credential input is bound to another bridge")
        ));
        let secret = input.take_for_bridge(&bridge).unwrap();
        assert_eq!(secret.username(), "camera-user");
        assert_eq!(secret.password(), "camera-password");
        assert!(matches!(
            input.take_for_bridge(&bridge).unwrap_err(),
            ReolinkPairingServiceError::SecretInput("credential input was already consumed")
        ));
        assert!(!format!("{input:?}").contains(username_path.to_str().unwrap()));

        fs::remove_dir_all(root).unwrap();
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
        let new_key = format!("reolink-camera-front/{transaction_id}");
        let new_ref = VaultRef::trusted(format!("{REOLINK_VAULT_REF_PREFIX}{new_key}"));
        let request = PairingTransactionRequest::new(
            transaction_id,
            AgentId::trusted("operator"),
            BridgeId::trusted("reolink-camera-front"),
            RuntimePairingSessionId::trusted("reolink-pairing-1"),
            PairingCredentialLocation::new(new_ref, REOLINK_VAULT_NAMESPACE, new_key).unwrap(),
            2_000,
            runtime_revision,
        )
        .unwrap()
        .with_metadata(vec![Metadata::new("reolink.pairing.verified", "true")]);
        match previous {
            Some(previous) => {
                request.with_previous_credential(reolink_credential_location(&previous).unwrap())
            }
            None => request,
        }
    }

    #[test]
    fn startup_recovers_runtime_commit_interrupted_before_journal_ack() {
        let root = test_directory("restart-recovery");
        let vault = open_vault(&root.join("vault"));
        let revision = persist_runtime(&root, &runtime_for_bridge(true, None));
        let runtime_store =
            SmartHomeRuntimeStore::new(LocalFolderStorageBackend::new(runtime_root(&root)));
        let failing_journal = FailOnPutBackend::new(journal_root(&root), 3);
        assert!(
            PairingTransactionCoordinator::new(&failing_journal, &vault, &runtime_store)
                .execute(
                    transaction_request("reolink-restart", revision, None),
                    br#"{"schema_version":1,"username":"u","password":"p"}"#,
                )
                .is_err()
        );

        let service = restore_service(
            &root,
            vault,
            Arc::new(AtomicUsize::new(0)),
            Arc::new(AtomicUsize::new(0)),
        )
        .unwrap();
        assert_eq!(service.snapshot().recovered_transaction_count, 1);
        assert_eq!(
            service
                .runtime()
                .pairing_session(&RuntimePairingSessionId::trusted("reolink-pairing-1"))
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
    fn replacement_commit_removes_the_captured_previous_credential() {
        let root = test_directory("replacement");
        let vault = open_vault(&root.join("vault"));
        let old_ref = VaultRef::trusted(format!(
            "{REOLINK_VAULT_REF_PREFIX}reolink-camera-front/previous"
        ));
        let old_key = vault_record_key(&old_ref).unwrap().to_string();
        vault
            .put(REOLINK_VAULT_NAMESPACE, &old_key, b"previous-secret", None)
            .unwrap();
        persist_runtime(&root, &runtime_for_bridge(true, Some(old_ref.clone())));
        let mut service = restore_service(
            &root,
            vault.clone(),
            Arc::new(AtomicUsize::new(0)),
            Arc::new(AtomicUsize::new(0)),
        )
        .unwrap();

        let pairing_request = request(&service);
        let report = service.pair(pairing_request).unwrap().clone();

        assert_ne!(report.vault_ref, old_ref);
        assert!(vault
            .get(REOLINK_VAULT_NAMESPACE, &old_key)
            .unwrap()
            .is_none());
        assert!(vault
            .get(
                REOLINK_VAULT_NAMESPACE,
                vault_record_key(&report.vault_ref).unwrap(),
            )
            .unwrap()
            .is_some());
        assert_eq!(
            service
                .runtime()
                .registry()
                .bridge(&BridgeId::trusted("reolink-camera-front"))
                .unwrap()
                .auth_ref,
            Some(report.vault_ref)
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn replacement_cleanup_is_exact_and_cleanup_drift_blocks_restart() {
        let root = test_directory("cleanup-drift");
        let vault = open_vault(&root.join("vault"));
        let old_ref = VaultRef::trusted(format!(
            "{REOLINK_VAULT_REF_PREFIX}reolink-camera-front/previous"
        ));
        let old_key = vault_record_key(&old_ref).unwrap().to_string();
        let old_revision = vault
            .put(REOLINK_VAULT_NAMESPACE, &old_key, b"previous-secret", None)
            .unwrap();
        let revision = persist_runtime(&root, &runtime_for_bridge(true, Some(old_ref.clone())));
        let runtime_store =
            SmartHomeRuntimeStore::new(LocalFolderStorageBackend::new(runtime_root(&root)));
        let failing_journal = FailOnPutBackend::new(journal_root(&root), 3);
        assert!(
            PairingTransactionCoordinator::new(&failing_journal, &vault, &runtime_store)
                .execute(
                    transaction_request("reolink-cleanup", revision, Some(old_ref)),
                    br#"{"schema_version":1,"username":"u","password":"p"}"#,
                )
                .is_err()
        );
        vault
            .put(
                REOLINK_VAULT_NAMESPACE,
                &old_key,
                b"replacement-owned-secret",
                Some(old_revision),
            )
            .unwrap();

        let result = restore_service(
            &root,
            vault.clone(),
            Arc::new(AtomicUsize::new(0)),
            Arc::new(AtomicUsize::new(0)),
        );
        assert!(matches!(
            result,
            Err(ReolinkPairingServiceError::Transaction(_))
        ));
        assert_eq!(
            vault
                .get(REOLINK_VAULT_NAMESPACE, &old_key)
                .unwrap()
                .unwrap()
                .plaintext
                .as_slice(),
            b"replacement-owned-secret"
        );
        assert_eq!(
            LocalFolderStorageBackend::new(journal_root(&root))
                .list("smart-home-pairing-transactions", Default::default())
                .unwrap()
                .records
                .len(),
            1
        );

        fs::remove_dir_all(root).unwrap();
    }
}
