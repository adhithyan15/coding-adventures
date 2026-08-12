//! Actor-owned Synology credential verification and recoverable durable handoff.

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
use smart_home_controller_runtime::{ControllerPersistenceError, SmartHomeControllerRuntime};
use smart_home_core::{
    AgentId, Bridge, BridgeId, CapabilityId, EntityId, EntityKind, IntegrationId, Metadata,
    ProtocolFamily, VaultRef,
};
use smart_home_pairing_transaction::{
    PairingCredentialLocation, PairingTransactionCoordinator, PairingTransactionError,
    PairingTransactionOutcome, PairingTransactionRequest,
};
use smart_home_runtime::{
    PairingSessionStatus, RuntimeCompletePairingToolRequest, RuntimeError, RuntimePairingSessionId,
    SmartHomeRuntime,
};
use smart_home_synology_snapshot_host::{
    encode_synology_credentials, SynologySnapshotHostError, SYNOLOGY_VAULT_NAMESPACE,
    SYNOLOGY_VAULT_REF_PREFIX,
};
use smart_home_synology_surveillance_integration::{
    SynologyClient, SynologyConfig, SynologyCredentials, SynologyError, SynologyLanTransport,
    INTEGRATION_ID, PROTOCOL_ID,
};
use storage_core::{Revision, StorageBackend};
use url_parser::Url;

pub const PAIR_REQUEST_CONTENT_TYPE: &str =
    "application/vnd.smart-home.synology-pairing-request+json";
const HTTPS_ENDPOINT_KIND: &str = "https_endpoint";
const CAMERA_ID_KIND: &str = "camera_id";

#[derive(Debug)]
pub enum SynologyPairingServiceError {
    UnknownSession(RuntimePairingSessionId),
    SessionNotPending {
        session_id: RuntimePairingSessionId,
        status: PairingSessionStatus,
    },
    UnknownBridge(BridgeId),
    WrongIntegration(IntegrationId),
    MissingBridgeAddress(BridgeId),
    InvalidHttpsEndpoint(BridgeId),
    InvalidInstalledCameras(BridgeId),
    CameraCorrespondence,
    InvalidRequest(String),
    SecretInput(&'static str),
    Synology(SynologyError),
    CredentialEncoding(SynologySnapshotHostError),
    Runtime(RuntimeError),
    Controller(ControllerPersistenceError),
    Transaction(PairingTransactionError),
    Entropy(String),
    MissingDurableRuntime,
    ExistingCredentialReference(VaultRef),
    TransactionRolledBack(String),
}

impl fmt::Display for SynologyPairingServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownSession(session_id) => {
                write!(formatter, "unknown Synology pairing session {session_id}")
            }
            Self::SessionNotPending { session_id, status } => write!(
                formatter,
                "Synology pairing session {session_id} is not pending user presence ({status:?})"
            ),
            Self::UnknownBridge(bridge_id) => write!(formatter, "unknown Synology bridge {bridge_id}"),
            Self::WrongIntegration(integration_id) => write!(
                formatter,
                "pairing service only accepts Synology bridges, got integration {integration_id}"
            ),
            Self::MissingBridgeAddress(bridge_id) => {
                write!(formatter, "Synology bridge {bridge_id} has no HTTPS address")
            }
            Self::InvalidHttpsEndpoint(bridge_id) => write!(
                formatter,
                "Synology bridge {bridge_id} does not match the reviewed HTTPS connection target"
            ),
            Self::InvalidInstalledCameras(bridge_id) => write!(
                formatter,
                "Synology bridge {bridge_id} has ambiguous installed camera identifiers"
            ),
            Self::CameraCorrespondence => formatter.write_str(
                "Synology credential verification did not match the installed camera set",
            ),
            Self::InvalidRequest(message) => {
                write!(formatter, "invalid Synology pairing request: {message}")
            }
            Self::SecretInput(message) => write!(formatter, "Synology secret input failed: {message}"),
            Self::Synology(error) => write!(formatter, "Synology credential verification failed: {error}"),
            Self::CredentialEncoding(_) => {
                formatter.write_str("Synology credential envelope encoding failed")
            }
            Self::Runtime(error) => write!(formatter, "Synology runtime completion failed: {error}"),
            Self::Controller(error) => write!(formatter, "Synology controller failed: {error}"),
            Self::Transaction(error) => write!(formatter, "Synology pairing transaction failed: {error}"),
            Self::Entropy(message) => write!(
                formatter,
                "Synology pairing transaction generation failed: {message}"
            ),
            Self::MissingDurableRuntime => {
                formatter.write_str("Synology pairing requires a durable runtime snapshot")
            }
            Self::ExistingCredentialReference(vault_ref) => write!(
                formatter,
                "existing Synology credential reference is outside the Synology Vault namespace: {vault_ref}"
            ),
            Self::TransactionRolledBack(transaction_id) => write!(
                formatter,
                "Synology pairing transaction {transaction_id} rolled back before runtime commit"
            ),
        }
    }
}

impl std::error::Error for SynologyPairingServiceError {}

impl From<SynologyError> for SynologyPairingServiceError {
    fn from(error: SynologyError) -> Self {
        Self::Synology(error)
    }
}

impl From<SynologySnapshotHostError> for SynologyPairingServiceError {
    fn from(error: SynologySnapshotHostError) -> Self {
        Self::CredentialEncoding(error)
    }
}

impl From<RuntimeError> for SynologyPairingServiceError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

impl From<ControllerPersistenceError> for SynologyPairingServiceError {
    fn from(error: ControllerPersistenceError) -> Self {
        Self::Controller(error)
    }
}

impl From<PairingTransactionError> for SynologyPairingServiceError {
    fn from(error: PairingTransactionError) -> Self {
        Self::Transaction(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SynologyPairingRequest {
    pub session_id: RuntimePairingSessionId,
    pub principal_id: AgentId,
    pub expected_runtime_revision: Revision,
    pub completed_at_ms: u64,
}

impl SynologyPairingRequest {
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

    pub fn into_message(self, sender_id: &str) -> Result<Message, SynologyPairingServiceError> {
        let payload = serde_json::to_vec(&serde_json::json!({
            "schema_version": 1,
            "session_id": self.session_id.as_str(),
            "principal_id": self.principal_id.as_str(),
            "expected_runtime_revision": self.expected_runtime_revision.as_str(),
            "completed_at_ms": self.completed_at_ms,
        }))
        .map_err(|error| SynologyPairingServiceError::InvalidRequest(error.to_string()))?;
        Ok(Message::new(
            sender_id,
            PAIR_REQUEST_CONTENT_TYPE,
            payload,
            None,
        ))
    }

    fn from_message(message: &Message) -> Result<Self, SynologyPairingServiceError> {
        if message.content_type != PAIR_REQUEST_CONTENT_TYPE {
            return Err(SynologyPairingServiceError::InvalidRequest(format!(
                "message content type must be `{PAIR_REQUEST_CONTENT_TYPE}`"
            )));
        }
        let value: serde_json::Value = serde_json::from_slice(&message.payload)
            .map_err(|error| SynologyPairingServiceError::InvalidRequest(error.to_string()))?;
        let object = value.as_object().ok_or_else(|| {
            SynologyPairingServiceError::InvalidRequest(
                "message body must be an object".to_string(),
            )
        })?;
        if object
            .get("schema_version")
            .and_then(serde_json::Value::as_u64)
            != Some(1)
        {
            return Err(SynologyPairingServiceError::InvalidRequest(
                "unsupported schema_version".to_string(),
            ));
        }
        Ok(Self::new(
            RuntimePairingSessionId::trusted(required_json_string(object, "session_id")?),
            AgentId::new(required_json_string(object, "principal_id")?)
                .map_err(|error| SynologyPairingServiceError::InvalidRequest(error.to_string()))?,
            Revision::new(required_json_string(object, "expected_runtime_revision")?)
                .map_err(|error| SynologyPairingServiceError::InvalidRequest(error.to_string()))?,
            object
                .get("completed_at_ms")
                .and_then(serde_json::Value::as_u64)
                .ok_or_else(|| {
                    SynologyPairingServiceError::InvalidRequest(
                        "completed_at_ms must be a non-negative integer".to_string(),
                    )
                })?,
        ))
    }
}

pub struct SynologyCredentialSecret {
    username: Zeroizing<String>,
    password: Zeroizing<String>,
}

impl SynologyCredentialSecret {
    pub fn new(
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Result<Self, SynologyPairingServiceError> {
        let username = Zeroizing::new(username.into());
        let password = Zeroizing::new(password.into());
        SynologyCredentials::new(username.as_str(), password.as_str())?;
        Ok(Self { username, password })
    }

    fn username(&self) -> &str {
        self.username.as_str()
    }

    fn password(&self) -> &str {
        self.password.as_str()
    }
}

impl fmt::Debug for SynologyCredentialSecret {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SynologyCredentialSecret([REDACTED])")
    }
}

pub trait SynologyCredentialInput {
    fn take_for_bridge(
        &mut self,
        bridge: &Bridge,
    ) -> Result<SynologyCredentialSecret, SynologyPairingServiceError>;
}

pub struct OwnerOnlySynologyCredentialInput {
    bridge_id: BridgeId,
    username_path: PathBuf,
    username_length: usize,
    password_path: PathBuf,
    password_length: usize,
    consumed: bool,
}

impl OwnerOnlySynologyCredentialInput {
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
    ) -> Result<Zeroizing<String>, SynologyPairingServiceError> {
        let bytes = read_owner_only_secret(path, length).map_err(map_secret_file_error)?;
        let value = std::str::from_utf8(bytes.as_slice())
            .map_err(|_| SynologyPairingServiceError::SecretInput("secret is not UTF-8"))?;
        Ok(Zeroizing::new(value.to_string()))
    }
}

impl fmt::Debug for OwnerOnlySynologyCredentialInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OwnerOnlySynologyCredentialInput")
            .field("bridge_id", &self.bridge_id)
            .field("paths", &"[REDACTED]")
            .field("consumed", &self.consumed)
            .finish()
    }
}

impl SynologyCredentialInput for OwnerOnlySynologyCredentialInput {
    fn take_for_bridge(
        &mut self,
        bridge: &Bridge,
    ) -> Result<SynologyCredentialSecret, SynologyPairingServiceError> {
        if bridge.bridge_id != self.bridge_id {
            return Err(SynologyPairingServiceError::SecretInput(
                "credential input is bound to another bridge",
            ));
        }
        if self.consumed {
            return Err(SynologyPairingServiceError::SecretInput(
                "credential input was already consumed",
            ));
        }
        let username = Self::read_utf8(&self.username_path, self.username_length)?;
        let password = Self::read_utf8(&self.password_path, self.password_length)?;
        let secret = SynologyCredentialSecret::new(username.as_str(), password.as_str())?;
        self.consumed = true;
        Ok(secret)
    }
}

fn map_secret_file_error(error: SecretFileError) -> SynologyPairingServiceError {
    let message = match error {
        SecretFileError::InvalidPath => "secret path is invalid",
        SecretFileError::ParentUnavailable => "secret parent is unavailable",
        SecretFileError::AccessFailed => "secret file is unavailable",
        SecretFileError::UnsafeFileType => "secret file type is unsafe",
        SecretFileError::InsecureOwner => "secret file owner is unsafe",
        SecretFileError::InsecurePermissions => "secret file permissions are unsafe",
        SecretFileError::InvalidLength => "secret length is invalid",
    };
    SynologyPairingServiceError::SecretInput(message)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedSynologyNvr {
    pub camera_count: usize,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SynologyPairingConnectionTarget {
    pub bridge_id: BridgeId,
    pub canonical_host: String,
    pub pinned_address: SocketAddr,
}

impl SynologyPairingConnectionTarget {
    pub fn new(
        bridge_id: BridgeId,
        canonical_host: impl Into<String>,
        pinned_address: SocketAddr,
    ) -> Result<Self, SynologyPairingServiceError> {
        let canonical_host = canonical_host.into();
        if canonical_host.trim().is_empty()
            || canonical_host.chars().any(char::is_control)
            || canonical_host.contains(['/', '@', '?', '#'])
        {
            return Err(SynologyPairingServiceError::InvalidHttpsEndpoint(bridge_id));
        }
        Ok(Self {
            bridge_id,
            canonical_host,
            pinned_address,
        })
    }

    fn validate(&self, bridge: &Bridge) -> Result<String, SynologyPairingServiceError> {
        if bridge.bridge_id != self.bridge_id {
            return Err(SynologyPairingServiceError::InvalidHttpsEndpoint(
                bridge.bridge_id.clone(),
            ));
        }
        let address = bridge.address.as_deref().ok_or_else(|| {
            SynologyPairingServiceError::MissingBridgeAddress(bridge.bridge_id.clone())
        })?;
        let parsed = Url::parse(address).map_err(|_| {
            SynologyPairingServiceError::InvalidHttpsEndpoint(bridge.bridge_id.clone())
        })?;
        let host = parsed.host.as_deref().ok_or_else(|| {
            SynologyPairingServiceError::InvalidHttpsEndpoint(bridge.bridge_id.clone())
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
            return Err(SynologyPairingServiceError::InvalidHttpsEndpoint(
                bridge.bridge_id.clone(),
            ));
        }
        let endpoints = bridge
            .identifiers
            .iter()
            .filter(|identifier| {
                identifier.family == ProtocolFamily::Vendor(PROTOCOL_ID.to_string())
                    && identifier.kind == HTTPS_ENDPOINT_KIND
            })
            .collect::<Vec<_>>();
        if endpoints.len() > 1
            || endpoints
                .first()
                .is_some_and(|identifier| identifier.value != address)
        {
            return Err(SynologyPairingServiceError::InvalidHttpsEndpoint(
                bridge.bridge_id.clone(),
            ));
        }
        Ok(address.trim_end_matches('/').to_string())
    }
}

pub trait SynologyPairingVerifier {
    fn preflight(&self, bridge: &Bridge) -> Result<String, SynologyPairingServiceError>;

    fn verify(
        &mut self,
        bridge: &Bridge,
        credentials: &SynologyCredentialSecret,
        expected_camera_ids: &BTreeSet<u64>,
    ) -> Result<VerifiedSynologyNvr, SynologyPairingServiceError>;
}

pub struct NativeSynologyPairingVerifier {
    target: SynologyPairingConnectionTarget,
}

impl NativeSynologyPairingVerifier {
    pub fn new(target: SynologyPairingConnectionTarget) -> Self {
        Self { target }
    }
}

impl SynologyPairingVerifier for NativeSynologyPairingVerifier {
    fn preflight(&self, bridge: &Bridge) -> Result<String, SynologyPairingServiceError> {
        self.target.validate(bridge)
    }

    fn verify(
        &mut self,
        bridge: &Bridge,
        credentials: &SynologyCredentialSecret,
        expected_camera_ids: &BTreeSet<u64>,
    ) -> Result<VerifiedSynologyNvr, SynologyPairingServiceError> {
        let address = self.target.validate(bridge)?;
        let config = SynologyConfig::new(
            bridge.bridge_id.clone(),
            address,
            VaultRef::trusted("vault://smart-home/synology/verification-only"),
        )?;
        let credentials = SynologyCredentials::new(credentials.username(), credentials.password())?;
        let mut client = SynologyClient::new(
            config,
            credentials,
            SynologyLanTransport::default().with_pinned_address(self.target.pinned_address),
        )?;
        let snapshot = client.inspect()?;
        if snapshot.info.allow_snapshot != Some(true) {
            return Err(SynologyPairingServiceError::CameraCorrespondence);
        }
        let observed_camera_ids = snapshot
            .cameras
            .iter()
            .map(|camera| camera.id)
            .collect::<BTreeSet<_>>();
        validate_camera_correspondence(expected_camera_ids, &observed_camera_ids)?;
        Ok(VerifiedSynologyNvr {
            camera_count: observed_camera_ids.len(),
            version: snapshot.info.version,
        })
    }
}

fn validate_camera_correspondence(
    expected_camera_ids: &BTreeSet<u64>,
    observed_camera_ids: &BTreeSet<u64>,
) -> Result<(), SynologyPairingServiceError> {
    if observed_camera_ids.is_empty()
        || (!expected_camera_ids.is_empty() && observed_camera_ids != expected_camera_ids)
    {
        Err(SynologyPairingServiceError::CameraCorrespondence)
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SynologyPairingServiceSnapshot {
    pub request_count: u64,
    pub completed_count: u64,
    pub failed_count: u64,
    pub recovered_transaction_count: u64,
    pub last_completed_at_ms: Option<u64>,
    pub last_bridge_id: Option<BridgeId>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SynologyPairingReport {
    pub session_id: RuntimePairingSessionId,
    pub bridge_id: BridgeId,
    pub vault_ref: VaultRef,
    pub completed_at_ms: u64,
    pub camera_count: usize,
    pub version: String,
}

pub struct SynologyPairingServiceActorState<I, V, J, R> {
    controller: SmartHomeControllerRuntime<R>,
    journal_backend: J,
    vault: Arc<SealedStore>,
    credential_input: I,
    verifier: V,
    snapshot: SynologyPairingServiceSnapshot,
    last_report: Option<SynologyPairingReport>,
}

impl<I, V, J, R> SynologyPairingServiceActorState<I, V, J, R>
where
    I: SynologyCredentialInput,
    V: SynologyPairingVerifier,
    J: StorageBackend,
    R: StorageBackend,
{
    pub fn restore(
        journal_backend: J,
        vault: Arc<SealedStore>,
        controller: SmartHomeControllerRuntime<R>,
        credential_input: I,
        verifier: V,
    ) -> Result<Self, SynologyPairingServiceError> {
        controller
            .durable_snapshot()?
            .ok_or(SynologyPairingServiceError::MissingDurableRuntime)?;
        let recovered_transaction_count = {
            let coordinator =
                PairingTransactionCoordinator::new(&journal_backend, &vault, &controller);
            let pending = coordinator.pending_transaction_ids()?;
            let recovered_count = pending.len() as u64;
            for transaction_id in pending {
                let _ = coordinator.recover(&transaction_id)?;
            }
            if !coordinator.pending_transaction_ids()?.is_empty() {
                return Err(SynologyPairingServiceError::InvalidRequest(
                    "pairing transaction recovery left unresolved journals".to_string(),
                ));
            }
            recovered_count
        };
        Ok(Self {
            controller,
            journal_backend,
            vault,
            credential_input,
            verifier,
            snapshot: SynologyPairingServiceSnapshot {
                recovered_transaction_count,
                ..SynologyPairingServiceSnapshot::default()
            },
            last_report: None,
        })
    }

    pub fn runtime(&self) -> Result<SmartHomeRuntime, SynologyPairingServiceError> {
        Ok(self
            .controller
            .durable_snapshot()?
            .ok_or(SynologyPairingServiceError::MissingDurableRuntime)?
            .runtime)
    }

    pub fn runtime_revision(&self) -> Result<Revision, SynologyPairingServiceError> {
        self.controller
            .revision()?
            .ok_or(SynologyPairingServiceError::MissingDurableRuntime)
    }

    pub fn snapshot(&self) -> &SynologyPairingServiceSnapshot {
        &self.snapshot
    }

    pub fn last_report(&self) -> Option<&SynologyPairingReport> {
        self.last_report.as_ref()
    }

    pub fn pair(
        &mut self,
        request: SynologyPairingRequest,
    ) -> Result<&SynologyPairingReport, SynologyPairingServiceError> {
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
        request: SynologyPairingRequest,
    ) -> Result<SynologyPairingReport, SynologyPairingServiceError> {
        let restored = self
            .controller
            .durable_snapshot()?
            .ok_or(SynologyPairingServiceError::MissingDurableRuntime)?;
        let session = restored
            .runtime
            .pairing_session(&request.session_id)
            .cloned()
            .ok_or_else(|| {
                SynologyPairingServiceError::UnknownSession(request.session_id.clone())
            })?;
        if session.status != PairingSessionStatus::PendingUserPresence {
            return Err(SynologyPairingServiceError::SessionNotPending {
                session_id: session.session_id,
                status: session.status,
            });
        }
        let bridge = restored
            .runtime
            .registry()
            .bridge(&session.bridge_id)
            .cloned()
            .ok_or_else(|| SynologyPairingServiceError::UnknownBridge(session.bridge_id.clone()))?;
        if bridge.integration_id.as_str() != INTEGRATION_ID {
            return Err(SynologyPairingServiceError::WrongIntegration(
                bridge.integration_id,
            ));
        }
        let https_endpoint = self.verifier.preflight(&bridge)?;
        let expected_camera_ids = installed_camera_ids(&restored.runtime, &bridge)?;
        SynologyConfig::new(
            bridge.bridge_id.clone(),
            https_endpoint.as_str(),
            VaultRef::trusted("vault://smart-home/synology/validation-only"),
        )?;
        if request.expected_runtime_revision != restored.revision {
            return Err(SynologyPairingServiceError::InvalidRequest(
                "expected runtime revision is stale".to_string(),
            ));
        }

        let authorization_probe = RuntimeCompletePairingToolRequest::new(
            request.session_id.clone(),
            VaultRef::trusted("vault://smart-home/synology/authorization-preflight"),
            request.completed_at_ms,
        );
        restored.runtime.clone().execute_complete_pairing_tool(
            request.principal_id.clone(),
            authorization_probe,
            request.completed_at_ms,
        )?;

        let credentials = self.credential_input.take_for_bridge(&bridge)?;
        let verified = self
            .verifier
            .verify(&bridge, &credentials, &expected_camera_ids)?;
        let payload = encode_synology_credentials(credentials.username(), credentials.password())?;
        let transaction_id = new_transaction_id()?;
        let vault_key = format!("{}/{}", bridge.bridge_id.as_str(), transaction_id);
        let vault_ref = VaultRef::trusted(format!("{SYNOLOGY_VAULT_REF_PREFIX}{vault_key}"));
        let new_credential =
            PairingCredentialLocation::new(vault_ref.clone(), SYNOLOGY_VAULT_NAMESPACE, vault_key)?;
        let previous_credential = bridge
            .auth_ref
            .as_ref()
            .map(synology_credential_location)
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
            Metadata::new("synology.pairing.verified", "true"),
            Metadata::new("synology.pairing.https_endpoint", https_endpoint),
            Metadata::new(
                "synology.pairing.camera_count",
                verified.camera_count.to_string(),
            ),
            Metadata::new("synology.pairing.version", verified.version.clone()),
        ]);
        let transaction = match previous_credential {
            Some(previous) => transaction.with_previous_credential(previous),
            None => transaction,
        };
        let outcome = PairingTransactionCoordinator::new(
            &self.journal_backend,
            &self.vault,
            &self.controller,
        )
        .execute(transaction, payload.as_bytes())?;
        let PairingTransactionOutcome::Committed { .. } = outcome else {
            return Err(SynologyPairingServiceError::TransactionRolledBack(
                transaction_id,
            ));
        };
        Ok(SynologyPairingReport {
            session_id: request.session_id,
            bridge_id: bridge.bridge_id,
            vault_ref,
            completed_at_ms: request.completed_at_ms,
            camera_count: verified.camera_count,
            version: verified.version,
        })
    }
}

pub fn install_synology_pairing_service_actor<I, V, J, R>(
    system: &mut ActorSystem,
    actor_id: &str,
    state: SynologyPairingServiceActorState<I, V, J, R>,
) -> Result<String, ActorError>
where
    I: SynologyCredentialInput + 'static,
    V: SynologyPairingVerifier + 'static,
    J: StorageBackend + 'static,
    R: StorageBackend + 'static,
{
    system.create_actor(
        actor_id,
        Box::new(state),
        Box::new(|state: Box<dyn Any>, message| {
            let mut state = *state
                .downcast::<SynologyPairingServiceActorState<I, V, J, R>>()
                .expect("Synology pairing actor received the wrong state type");
            match SynologyPairingRequest::from_message(message) {
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
    vault_ref.as_str().strip_prefix(SYNOLOGY_VAULT_REF_PREFIX)
}

fn installed_camera_ids(
    runtime: &SmartHomeRuntime,
    bridge: &Bridge,
) -> Result<BTreeSet<u64>, SynologyPairingServiceError> {
    let expected_family = ProtocolFamily::Vendor(PROTOCOL_ID.to_string());
    let mut camera_ids = BTreeSet::new();
    for device in runtime.registry().devices_for_bridge(&bridge.bridge_id) {
        let matches = device
            .identifiers
            .iter()
            .filter(|identifier| {
                identifier.family == expected_family && identifier.kind == CAMERA_ID_KIND
            })
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            return Err(SynologyPairingServiceError::InvalidInstalledCameras(
                bridge.bridge_id.clone(),
            ));
        }
        let camera_id = matches[0]
            .value
            .parse::<u64>()
            .ok()
            .filter(|value| *value > 0)
            .ok_or_else(|| {
                SynologyPairingServiceError::InvalidInstalledCameras(bridge.bridge_id.clone())
            })?;
        if !camera_ids.insert(camera_id) {
            return Err(SynologyPairingServiceError::InvalidInstalledCameras(
                bridge.bridge_id.clone(),
            ));
        }
        let expected_entity_id =
            EntityId::trusted(format!("synology-surveillance:{camera_id}:camera"));
        if device.entity_ids.len() != 1 || device.entity_ids[0] != expected_entity_id {
            return Err(SynologyPairingServiceError::InvalidInstalledCameras(
                bridge.bridge_id.clone(),
            ));
        }
        let entity = runtime
            .registry()
            .entity(&expected_entity_id)
            .ok_or_else(|| {
                SynologyPairingServiceError::InvalidInstalledCameras(bridge.bridge_id.clone())
            })?;
        if entity.device_id != device.device_id
            || entity.kind != EntityKind::Camera
            || !entity.capabilities.iter().any(|capability| {
                capability.capability_id == CapabilityId::trusted("camera.snapshot")
            })
        {
            return Err(SynologyPairingServiceError::InvalidInstalledCameras(
                bridge.bridge_id.clone(),
            ));
        }
    }
    Ok(camera_ids)
}

fn is_loopback_host(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<IpAddr>()
            .is_ok_and(|address| address.is_loopback())
}

fn synology_credential_location(
    vault_ref: &VaultRef,
) -> Result<PairingCredentialLocation, SynologyPairingServiceError> {
    let key = vault_record_key(vault_ref).ok_or_else(|| {
        SynologyPairingServiceError::ExistingCredentialReference(vault_ref.clone())
    })?;
    Ok(PairingCredentialLocation::new(
        vault_ref.clone(),
        SYNOLOGY_VAULT_NAMESPACE,
        key,
    )?)
}

fn new_transaction_id() -> Result<String, SynologyPairingServiceError> {
    let random: [u8; 24] =
        random_array().map_err(|error| SynologyPairingServiceError::Entropy(error.to_string()))?;
    let mut suffix = String::with_capacity(random.len() * 2);
    for byte in random {
        use std::fmt::Write as _;
        write!(&mut suffix, "{byte:02x}").expect("writing to a String cannot fail");
    }
    Ok(format!("synology-{suffix}"))
}

fn required_json_string(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &str,
) -> Result<String, SynologyPairingServiceError> {
    object
        .get(field)
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| {
            SynologyPairingServiceError::InvalidRequest(format!("`{field}` must be a string"))
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
        BridgeTransport, Capability, CapabilityGrant, CapabilityGrantId, CapabilityMode, Device,
        DeviceId, Entity, Health, PrivilegeTier, ProtocolIdentifier, ValueKind,
    };
    use smart_home_runtime::RuntimePairingSession;
    use smart_home_runtime_store::SmartHomeRuntimeStore;
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
            "smart-home-synology-pairing-service-{}-{label}-{suffix}",
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
            BridgeId::trusted("synology-camera-front"),
            IntegrationId::trusted(INTEGRATION_ID),
            BridgeTransport::LanHttp,
        );
        bridge.address = Some("https://synology.local".to_string());
        bridge.health = Health::Unpaired;
        bridge.auth_ref = previous;
        bridge.identifiers.push(
            ProtocolIdentifier::new(
                ProtocolFamily::Vendor(PROTOCOL_ID.to_string()),
                HTTPS_ENDPOINT_KIND,
                "https://synology.local",
            )
            .unwrap(),
        );
        runtime.upsert_bridge(bridge.clone()).unwrap();
        let principal = AgentId::trusted("operator");
        runtime
            .start_pairing_session(RuntimePairingSession::pending(
                RuntimePairingSessionId::trusted("synology-pairing-1"),
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
                    CapabilityGrantId::trusted("grant-synology-pairing"),
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

    impl SynologyCredentialInput for FixedInput {
        fn take_for_bridge(
            &mut self,
            bridge: &Bridge,
        ) -> Result<SynologyCredentialSecret, SynologyPairingServiceError> {
            assert_eq!(bridge.bridge_id.as_str(), "synology-camera-front");
            self.calls.fetch_add(1, Ordering::SeqCst);
            SynologyCredentialSecret::new(self.username, self.password)
        }
    }

    struct ExactVerifier {
        calls: Arc<AtomicUsize>,
    }

    impl SynologyPairingVerifier for ExactVerifier {
        fn preflight(&self, bridge: &Bridge) -> Result<String, SynologyPairingServiceError> {
            SynologyPairingConnectionTarget::new(
                BridgeId::trusted("synology-camera-front"),
                "synology.local",
                "192.0.2.44:443".parse().unwrap(),
            )?
            .validate(bridge)
        }

        fn verify(
            &mut self,
            bridge: &Bridge,
            credentials: &SynologyCredentialSecret,
            expected_camera_ids: &BTreeSet<u64>,
        ) -> Result<VerifiedSynologyNvr, SynologyPairingServiceError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            assert_eq!(bridge.address.as_deref(), Some("https://synology.local"));
            assert!(expected_camera_ids.is_empty());
            assert_eq!(credentials.username(), "camera-user");
            assert_eq!(credentials.password(), "camera-password");
            assert_eq!(
                format!("{credentials:?}"),
                "SynologyCredentialSecret([REDACTED])"
            );
            Ok(VerifiedSynologyNvr {
                camera_count: 2,
                version: "9.2-11289".to_string(),
            })
        }
    }

    type LocalService = SynologyPairingServiceActorState<
        FixedInput,
        ExactVerifier,
        LocalFolderStorageBackend,
        LocalFolderStorageBackend,
    >;
    type LocalController = SmartHomeControllerRuntime<LocalFolderStorageBackend>;

    fn restore_service_with_controller(
        root: &Path,
        vault: Arc<SealedStore>,
        input_calls: Arc<AtomicUsize>,
        verifier_calls: Arc<AtomicUsize>,
    ) -> Result<(LocalService, LocalController), SynologyPairingServiceError> {
        let controller =
            SmartHomeControllerRuntime::restore(LocalFolderStorageBackend::new(runtime_root(root)))
                .expect("test controller must restore");
        let service = SynologyPairingServiceActorState::restore(
            LocalFolderStorageBackend::new(journal_root(root)),
            vault,
            controller.clone(),
            FixedInput {
                calls: input_calls,
                username: "camera-user",
                password: "camera-password",
            },
            ExactVerifier {
                calls: verifier_calls,
            },
        )?;
        Ok((service, controller))
    }

    fn restore_service(
        root: &Path,
        vault: Arc<SealedStore>,
        input_calls: Arc<AtomicUsize>,
        verifier_calls: Arc<AtomicUsize>,
    ) -> Result<LocalService, SynologyPairingServiceError> {
        restore_service_with_controller(root, vault, input_calls, verifier_calls)
            .map(|(service, _)| service)
    }

    fn request(service: &LocalService) -> SynologyPairingRequest {
        SynologyPairingRequest::new(
            RuntimePairingSessionId::trusted("synology-pairing-1"),
            AgentId::trusted("operator"),
            service.runtime_revision().unwrap(),
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
        let (mut service, controller) = restore_service_with_controller(
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
        assert_eq!(report.camera_count, 2);
        assert_ne!(service.runtime_revision().unwrap(), initial_revision);
        let committed_runtime = service.runtime().unwrap();
        assert_eq!(
            committed_runtime
                .registry()
                .bridge(&BridgeId::trusted("synology-camera-front"))
                .unwrap()
                .auth_ref,
            Some(report.vault_ref.clone())
        );
        let central = controller.durable_snapshot().unwrap().unwrap();
        assert_eq!(central.revision, service.runtime_revision().unwrap());
        assert_eq!(
            central
                .runtime
                .registry()
                .bridge(&BridgeId::trusted("synology-camera-front"))
                .unwrap()
                .auth_ref,
            Some(report.vault_ref.clone())
        );
        let record = vault
            .get(
                SYNOLOGY_VAULT_NAMESPACE,
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
            .unwrap()
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
        let (mut service, controller) = restore_service_with_controller(
            &root,
            vault.clone(),
            input_calls.clone(),
            verifier_calls.clone(),
        )
        .unwrap();

        let pairing_request = request(&service);
        assert!(matches!(
            service.pair(pairing_request).unwrap_err(),
            SynologyPairingServiceError::Runtime(_)
        ));
        assert_eq!(input_calls.load(Ordering::SeqCst), 0);
        assert_eq!(verifier_calls.load(Ordering::SeqCst), 0);

        let mut ambiguous = service
            .runtime()
            .unwrap()
            .registry()
            .bridge(&BridgeId::trusted("synology-camera-front"))
            .unwrap()
            .clone();
        ambiguous.identifiers.push(
            ProtocolIdentifier::new(
                ProtocolFamily::Vendor(PROTOCOL_ID.to_string()),
                HTTPS_ENDPOINT_KIND,
                "https://other-synology.local",
            )
            .unwrap(),
        );
        controller
            .transaction(1_900, |runtime, _| runtime.upsert_bridge(ambiguous))
            .unwrap();
        let current = request(&service);
        assert!(matches!(
            service.pair(current).unwrap_err(),
            SynologyPairingServiceError::InvalidHttpsEndpoint(_)
        ));
        assert_eq!(input_calls.load(Ordering::SeqCst), 0);
        assert_eq!(verifier_calls.load(Ordering::SeqCst), 0);
        assert!(vault
            .list(SYNOLOGY_VAULT_NAMESPACE, Default::default())
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
    fn intervening_central_commit_rejects_stale_request_before_external_io() {
        let root = test_directory("central-revision-drift");
        let vault = open_vault(&root.join("vault"));
        persist_runtime(&root, &runtime_for_bridge(true, None));
        let input_calls = Arc::new(AtomicUsize::new(0));
        let verifier_calls = Arc::new(AtomicUsize::new(0));
        let (mut service, controller) = restore_service_with_controller(
            &root,
            vault.clone(),
            input_calls.clone(),
            verifier_calls.clone(),
        )
        .unwrap();
        let stale = request(&service);

        controller.save_snapshot(1_900).unwrap();

        assert!(matches!(
            service.pair(stale).unwrap_err(),
            SynologyPairingServiceError::InvalidRequest(message)
                if message == "expected runtime revision is stale"
        ));
        assert_eq!(input_calls.load(Ordering::SeqCst), 0);
        assert_eq!(verifier_calls.load(Ordering::SeqCst), 0);
        assert!(vault
            .list(SYNOLOGY_VAULT_NAMESPACE, Default::default())
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
    fn credential_bearing_endpoint_fails_before_secret_input() {
        let root = test_directory("credential-bearing-endpoint");
        let vault = open_vault(&root.join("vault"));
        persist_runtime(&root, &runtime_for_bridge(true, None));
        let input_calls = Arc::new(AtomicUsize::new(0));
        let verifier_calls = Arc::new(AtomicUsize::new(0));
        let (mut service, controller) = restore_service_with_controller(
            &root,
            vault,
            input_calls.clone(),
            verifier_calls.clone(),
        )
        .unwrap();

        let mut embedded_credentials = service
            .runtime()
            .unwrap()
            .registry()
            .bridge(&BridgeId::trusted("synology-camera-front"))
            .unwrap()
            .clone();
        embedded_credentials.address = Some("https://operator:password@synology.local".to_string());
        embedded_credentials.identifiers[0].value =
            "https://operator:password@synology.local".to_string();
        controller
            .transaction(1_900, |runtime, _| {
                runtime.upsert_bridge(embedded_credentials)
            })
            .unwrap();
        let current = request(&service);
        assert!(matches!(
            service.pair(current).unwrap_err(),
            SynologyPairingServiceError::InvalidHttpsEndpoint(_)
        ));
        assert_eq!(input_calls.load(Ordering::SeqCst), 0);
        assert_eq!(verifier_calls.load(Ordering::SeqCst), 0);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn installed_camera_identity_is_exact_positive_and_unique() {
        let mut runtime = runtime_for_bridge(true, None);
        let bridge = runtime
            .registry()
            .bridge(&BridgeId::trusted("synology-camera-front"))
            .unwrap()
            .clone();
        let device_id = DeviceId::trusted("synology-surveillance:7");
        let entity_id = EntityId::trusted("synology-surveillance:7:camera");
        runtime
            .upsert_device(Device {
                device_id: device_id.clone(),
                bridge_id: bridge.bridge_id.clone(),
                manufacturer: "Synology".to_string(),
                model: "Managed camera".to_string(),
                name: "Front".to_string(),
                serial: None,
                firmware_version: None,
                room_id: None,
                entity_ids: vec![entity_id.clone()],
                identifiers: vec![ProtocolIdentifier::new(
                    ProtocolFamily::Vendor(PROTOCOL_ID.to_string()),
                    CAMERA_ID_KIND,
                    "7",
                )
                .unwrap()],
                health: Health::Online,
                metadata: Vec::new(),
            })
            .unwrap();
        runtime
            .upsert_entity(Entity {
                entity_id: entity_id.clone(),
                device_id: device_id.clone(),
                kind: EntityKind::Camera,
                name: "Front".to_string(),
                capabilities: vec![Capability::new(
                    CapabilityId::trusted("camera.snapshot"),
                    CapabilityMode::Command,
                    ValueKind::Text,
                )],
                state: None,
                metadata: Vec::new(),
            })
            .unwrap();
        assert_eq!(
            installed_camera_ids(&runtime, &bridge).unwrap(),
            BTreeSet::from([7])
        );

        let mut invalid_entity = runtime.registry().entity(&entity_id).unwrap().clone();
        invalid_entity.capabilities.clear();
        runtime.upsert_entity(invalid_entity).unwrap();
        assert!(matches!(
            installed_camera_ids(&runtime, &bridge).unwrap_err(),
            SynologyPairingServiceError::InvalidInstalledCameras(_)
        ));

        let mut invalid = runtime.registry().device(&device_id).unwrap().clone();
        invalid.identifiers[0].value = "0".to_string();
        runtime.upsert_device(invalid).unwrap();
        assert!(matches!(
            installed_camera_ids(&runtime, &bridge).unwrap_err(),
            SynologyPairingServiceError::InvalidInstalledCameras(_)
        ));
    }

    #[test]
    fn camera_correspondence_requires_a_nonempty_exact_installed_set() {
        assert!(validate_camera_correspondence(&BTreeSet::new(), &BTreeSet::new()).is_err());
        assert!(validate_camera_correspondence(&BTreeSet::new(), &BTreeSet::from([2])).is_ok());
        assert!(validate_camera_correspondence(&BTreeSet::from([2]), &BTreeSet::from([2])).is_ok());
        assert!(
            validate_camera_correspondence(&BTreeSet::from([2]), &BTreeSet::from([2, 3])).is_err()
        );
    }

    #[test]
    fn native_verifier_uses_isolated_synology_session_and_exact_camera_inspection() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let server_requests = Arc::clone(&requests);
        let responses = vec![
            r#"{"success":true,"data":{"SYNO.API.Auth":{"path":"auth.cgi","minVersion":1,"maxVersion":6},"SYNO.SurveillanceStation.Info":{"path":"entry.cgi","minVersion":1,"maxVersion":5},"SYNO.SurveillanceStation.Camera":{"path":"entry.cgi","minVersion":1,"maxVersion":9}}}"#,
            r#"{"success":true,"data":{"sid":"secret.sid.value","synotoken":"secret-token"}}"#,
            r#"{"success":true,"data":{"version":{"major":9,"minor":2,"build":11289},"cameraNumber":1,"maxCameraSupport":40,"userPriv":4,"allowSnapshot":true,"allowManualRec":false}}"#,
            r#"{"success":true,"data":{"total":1,"cameras":[{"id":2,"name":"Front","vendor":"Synology","model":"BC500","channel":"1","status":1}]}}"#,
            r#"{"success":true}"#,
        ];
        let handle = thread::spawn(move || {
            for body in responses {
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
                let reply = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                reader.get_mut().write_all(reply.as_bytes()).unwrap();
            }
        });

        let mut bridge = Bridge::new(
            BridgeId::trusted("synology-camera-front"),
            IntegrationId::trusted(INTEGRATION_ID),
            BridgeTransport::LanHttp,
        );
        bridge.address = Some(format!("http://localhost:{}", address.port()));
        let credentials =
            SynologyCredentialSecret::new("operator name", "secret&password").unwrap();
        let target =
            SynologyPairingConnectionTarget::new(bridge.bridge_id.clone(), "localhost", address)
                .unwrap();
        let verified = NativeSynologyPairingVerifier::new(target)
            .verify(&bridge, &credentials, &BTreeSet::from([2]))
            .unwrap();
        handle.join().unwrap();

        assert_eq!(verified.camera_count, 1);
        assert_eq!(verified.version, "9.2-11289");
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 5);
        assert!(requests[0].0.starts_with("GET /webapi/query.cgi?"));
        assert!(requests[1].0.starts_with("POST /webapi/auth.cgi HTTP/1.1"));
        assert_eq!(
            requests[1].1,
            "api=SYNO.API.Auth&method=login&version=6&account=operator%20name&passwd=secret%26password&session=SurveillanceStation&format=sid&enable_syno_token=yes"
        );
        assert!(requests[2].0.contains("method=GetInfo"));
        assert!(requests[3].0.contains("method=List"));
        assert!(requests[3].0.contains("blPrivilege=true"));
        assert!(requests[4].0.contains("method=logout"));
        assert!(requests[2..]
            .iter()
            .all(|(head, _)| head.contains("_sid=secret.sid.value")
                && head.contains("SynoToken=secret-token")));
        assert!(!format!("{verified:?}").contains("secret.sid.value"));
    }

    #[test]
    fn actor_message_contains_authority_and_revision_but_no_secret_input() {
        let request = SynologyPairingRequest::new(
            RuntimePairingSessionId::trusted("synology-pairing-1"),
            AgentId::trusted("operator"),
            Revision::new("runtime-r1").unwrap(),
            2_000,
        );
        let message = request.clone().into_message("scheduler").unwrap();
        assert_eq!(
            SynologyPairingRequest::from_message(&message).unwrap(),
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
            .bridge(&BridgeId::trusted("synology-camera-front"))
            .unwrap()
            .clone();
        let mut input = OwnerOnlySynologyCredentialInput::new(
            bridge.bridge_id.clone(),
            &username_path,
            11,
            &password_path,
            15,
        );
        let mut other_bridge = bridge.clone();
        other_bridge.bridge_id = BridgeId::trusted("synology-camera-other");
        assert!(matches!(
            input.take_for_bridge(&other_bridge).unwrap_err(),
            SynologyPairingServiceError::SecretInput("credential input is bound to another bridge")
        ));
        let secret = input.take_for_bridge(&bridge).unwrap();
        assert_eq!(secret.username(), "camera-user");
        assert_eq!(secret.password(), "camera-password");
        assert!(matches!(
            input.take_for_bridge(&bridge).unwrap_err(),
            SynologyPairingServiceError::SecretInput("credential input was already consumed")
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
        let new_key = format!("synology-camera-front/{transaction_id}");
        let new_ref = VaultRef::trusted(format!("{SYNOLOGY_VAULT_REF_PREFIX}{new_key}"));
        let request = PairingTransactionRequest::new(
            transaction_id,
            AgentId::trusted("operator"),
            BridgeId::trusted("synology-camera-front"),
            RuntimePairingSessionId::trusted("synology-pairing-1"),
            PairingCredentialLocation::new(new_ref, SYNOLOGY_VAULT_NAMESPACE, new_key).unwrap(),
            2_000,
            runtime_revision,
        )
        .unwrap()
        .with_metadata(vec![Metadata::new("synology.pairing.verified", "true")]);
        match previous {
            Some(previous) => {
                request.with_previous_credential(synology_credential_location(&previous).unwrap())
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
                    transaction_request("synology-restart", revision, None),
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
                .unwrap()
                .pairing_session(&RuntimePairingSessionId::trusted("synology-pairing-1"))
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
            "{SYNOLOGY_VAULT_REF_PREFIX}synology-camera-front/previous"
        ));
        let old_key = vault_record_key(&old_ref).unwrap().to_string();
        vault
            .put(SYNOLOGY_VAULT_NAMESPACE, &old_key, b"previous-secret", None)
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
            .get(SYNOLOGY_VAULT_NAMESPACE, &old_key)
            .unwrap()
            .is_none());
        assert!(vault
            .get(
                SYNOLOGY_VAULT_NAMESPACE,
                vault_record_key(&report.vault_ref).unwrap(),
            )
            .unwrap()
            .is_some());
        assert_eq!(
            service
                .runtime()
                .unwrap()
                .registry()
                .bridge(&BridgeId::trusted("synology-camera-front"))
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
            "{SYNOLOGY_VAULT_REF_PREFIX}synology-camera-front/previous"
        ));
        let old_key = vault_record_key(&old_ref).unwrap().to_string();
        let old_revision = vault
            .put(SYNOLOGY_VAULT_NAMESPACE, &old_key, b"previous-secret", None)
            .unwrap();
        let revision = persist_runtime(&root, &runtime_for_bridge(true, Some(old_ref.clone())));
        let runtime_store =
            SmartHomeRuntimeStore::new(LocalFolderStorageBackend::new(runtime_root(&root)));
        let failing_journal = FailOnPutBackend::new(journal_root(&root), 3);
        assert!(
            PairingTransactionCoordinator::new(&failing_journal, &vault, &runtime_store)
                .execute(
                    transaction_request("synology-cleanup", revision, Some(old_ref)),
                    br#"{"schema_version":1,"username":"u","password":"p"}"#,
                )
                .is_err()
        );
        vault
            .put(
                SYNOLOGY_VAULT_NAMESPACE,
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
            Err(SynologyPairingServiceError::Transaction(_))
        ));
        assert_eq!(
            vault
                .get(SYNOLOGY_VAULT_NAMESPACE, &old_key)
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
