//! Actor-owned ZoneMinder credential verification and recoverable durable handoff.

#![forbid(unsafe_code)]

use std::any::Any;
use std::collections::BTreeSet;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use actor::{ActorError, ActorResult, ActorSystem, Message};
use chief_of_staff_daemon_secret_file::{read_owner_only_secret, SecretFileError};
use coding_adventures_csprng::random_array;
use coding_adventures_vault_sealed_store::SealedStore;
use coding_adventures_zeroize::Zeroizing;
use smart_home_core::{
    AgentId, Bridge, BridgeId, IntegrationId, Metadata, ProtocolFamily, VaultRef,
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
use smart_home_zoneminder_integration::{
    ZoneMinderClient, ZoneMinderConfig, ZoneMinderCredentials, ZoneMinderError,
    ZoneMinderLanTransport, PROTOCOL_ID,
};
use smart_home_zoneminder_snapshot_host::{
    encode_zoneminder_credentials, ZoneMinderSnapshotHostError, ZONEMINDER_VAULT_NAMESPACE,
    ZONEMINDER_VAULT_REF_PREFIX,
};
use storage_core::{Revision, StorageBackend};

pub const PAIR_REQUEST_CONTENT_TYPE: &str =
    "application/vnd.smart-home.zoneminder-pairing-request+json";
const ZONEMINDER_INTEGRATION_ID: &str = "zoneminder";
const HTTPS_ENDPOINT_KIND: &str = "https_endpoint";
const MONITOR_ID_KIND: &str = "monitor_id";

#[derive(Debug)]
pub enum ZoneMinderPairingServiceError {
    UnknownSession(RuntimePairingSessionId),
    SessionNotPending {
        session_id: RuntimePairingSessionId,
        status: PairingSessionStatus,
    },
    UnknownBridge(BridgeId),
    WrongIntegration(IntegrationId),
    MissingBridgeAddress(BridgeId),
    InvalidHttpsEndpoint(BridgeId),
    InvalidInstalledMonitors(BridgeId),
    MonitorCorrespondence,
    InvalidRequest(String),
    SecretInput(&'static str),
    ZoneMinder(ZoneMinderError),
    CredentialEncoding(ZoneMinderSnapshotHostError),
    Runtime(RuntimeError),
    RuntimeStore(RuntimeStoreError),
    Transaction(PairingTransactionError),
    Entropy(String),
    MissingDurableRuntime,
    ExistingCredentialReference(VaultRef),
    TransactionRolledBack(String),
}

impl fmt::Display for ZoneMinderPairingServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownSession(session_id) => {
                write!(formatter, "unknown ZoneMinder pairing session {session_id}")
            }
            Self::SessionNotPending { session_id, status } => write!(
                formatter,
                "ZoneMinder pairing session {session_id} is not pending user presence ({status:?})"
            ),
            Self::UnknownBridge(bridge_id) => write!(formatter, "unknown ZoneMinder bridge {bridge_id}"),
            Self::WrongIntegration(integration_id) => write!(
                formatter,
                "pairing service only accepts ZoneMinder bridges, got integration {integration_id}"
            ),
            Self::MissingBridgeAddress(bridge_id) => {
                write!(formatter, "ZoneMinder bridge {bridge_id} has no HTTPS address")
            }
            Self::InvalidHttpsEndpoint(bridge_id) => write!(
                formatter,
                "ZoneMinder bridge {bridge_id} must have one exact HTTPS endpoint identifier"
            ),
            Self::InvalidInstalledMonitors(bridge_id) => write!(
                formatter,
                "ZoneMinder bridge {bridge_id} has ambiguous installed monitor identifiers"
            ),
            Self::MonitorCorrespondence => formatter.write_str(
                "ZoneMinder credential verification did not match the installed monitor set",
            ),
            Self::InvalidRequest(message) => {
                write!(formatter, "invalid ZoneMinder pairing request: {message}")
            }
            Self::SecretInput(message) => write!(formatter, "ZoneMinder secret input failed: {message}"),
            Self::ZoneMinder(error) => write!(formatter, "ZoneMinder credential verification failed: {error}"),
            Self::CredentialEncoding(_) => {
                formatter.write_str("ZoneMinder credential envelope encoding failed")
            }
            Self::Runtime(error) => write!(formatter, "ZoneMinder runtime completion failed: {error}"),
            Self::RuntimeStore(error) => write!(formatter, "ZoneMinder runtime store failed: {error}"),
            Self::Transaction(error) => write!(formatter, "ZoneMinder pairing transaction failed: {error}"),
            Self::Entropy(message) => write!(
                formatter,
                "ZoneMinder pairing transaction generation failed: {message}"
            ),
            Self::MissingDurableRuntime => {
                formatter.write_str("ZoneMinder pairing requires a durable runtime snapshot")
            }
            Self::ExistingCredentialReference(vault_ref) => write!(
                formatter,
                "existing ZoneMinder credential reference is outside the ZoneMinder Vault namespace: {vault_ref}"
            ),
            Self::TransactionRolledBack(transaction_id) => write!(
                formatter,
                "ZoneMinder pairing transaction {transaction_id} rolled back before runtime commit"
            ),
        }
    }
}

impl std::error::Error for ZoneMinderPairingServiceError {}

impl From<ZoneMinderError> for ZoneMinderPairingServiceError {
    fn from(error: ZoneMinderError) -> Self {
        Self::ZoneMinder(error)
    }
}

impl From<ZoneMinderSnapshotHostError> for ZoneMinderPairingServiceError {
    fn from(error: ZoneMinderSnapshotHostError) -> Self {
        Self::CredentialEncoding(error)
    }
}

impl From<RuntimeError> for ZoneMinderPairingServiceError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

impl From<RuntimeStoreError> for ZoneMinderPairingServiceError {
    fn from(error: RuntimeStoreError) -> Self {
        Self::RuntimeStore(error)
    }
}

impl From<PairingTransactionError> for ZoneMinderPairingServiceError {
    fn from(error: PairingTransactionError) -> Self {
        Self::Transaction(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZoneMinderPairingRequest {
    pub session_id: RuntimePairingSessionId,
    pub principal_id: AgentId,
    pub expected_runtime_revision: Revision,
    pub completed_at_ms: u64,
}

impl ZoneMinderPairingRequest {
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

    pub fn into_message(self, sender_id: &str) -> Result<Message, ZoneMinderPairingServiceError> {
        let payload = serde_json::to_vec(&serde_json::json!({
            "schema_version": 1,
            "session_id": self.session_id.as_str(),
            "principal_id": self.principal_id.as_str(),
            "expected_runtime_revision": self.expected_runtime_revision.as_str(),
            "completed_at_ms": self.completed_at_ms,
        }))
        .map_err(|error| ZoneMinderPairingServiceError::InvalidRequest(error.to_string()))?;
        Ok(Message::new(
            sender_id,
            PAIR_REQUEST_CONTENT_TYPE,
            payload,
            None,
        ))
    }

    fn from_message(message: &Message) -> Result<Self, ZoneMinderPairingServiceError> {
        if message.content_type != PAIR_REQUEST_CONTENT_TYPE {
            return Err(ZoneMinderPairingServiceError::InvalidRequest(format!(
                "message content type must be `{PAIR_REQUEST_CONTENT_TYPE}`"
            )));
        }
        let value: serde_json::Value = serde_json::from_slice(&message.payload)
            .map_err(|error| ZoneMinderPairingServiceError::InvalidRequest(error.to_string()))?;
        let object = value.as_object().ok_or_else(|| {
            ZoneMinderPairingServiceError::InvalidRequest(
                "message body must be an object".to_string(),
            )
        })?;
        if object
            .get("schema_version")
            .and_then(serde_json::Value::as_u64)
            != Some(1)
        {
            return Err(ZoneMinderPairingServiceError::InvalidRequest(
                "unsupported schema_version".to_string(),
            ));
        }
        Ok(Self::new(
            RuntimePairingSessionId::trusted(required_json_string(object, "session_id")?),
            AgentId::new(required_json_string(object, "principal_id")?).map_err(|error| {
                ZoneMinderPairingServiceError::InvalidRequest(error.to_string())
            })?,
            Revision::new(required_json_string(object, "expected_runtime_revision")?).map_err(
                |error| ZoneMinderPairingServiceError::InvalidRequest(error.to_string()),
            )?,
            object
                .get("completed_at_ms")
                .and_then(serde_json::Value::as_u64)
                .ok_or_else(|| {
                    ZoneMinderPairingServiceError::InvalidRequest(
                        "completed_at_ms must be a non-negative integer".to_string(),
                    )
                })?,
        ))
    }
}

pub struct ZoneMinderCredentialSecret {
    username: Zeroizing<String>,
    password: Zeroizing<String>,
}

impl ZoneMinderCredentialSecret {
    pub fn new(
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Result<Self, ZoneMinderPairingServiceError> {
        let username = Zeroizing::new(username.into());
        let password = Zeroizing::new(password.into());
        ZoneMinderCredentials::new(username.as_str(), password.as_str())?;
        Ok(Self { username, password })
    }

    fn username(&self) -> &str {
        self.username.as_str()
    }

    fn password(&self) -> &str {
        self.password.as_str()
    }
}

impl fmt::Debug for ZoneMinderCredentialSecret {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ZoneMinderCredentialSecret([REDACTED])")
    }
}

pub trait ZoneMinderCredentialInput {
    fn take_for_bridge(
        &mut self,
        bridge: &Bridge,
    ) -> Result<ZoneMinderCredentialSecret, ZoneMinderPairingServiceError>;
}

pub struct OwnerOnlyZoneMinderCredentialInput {
    bridge_id: BridgeId,
    username_path: PathBuf,
    username_length: usize,
    password_path: PathBuf,
    password_length: usize,
    consumed: bool,
}

impl OwnerOnlyZoneMinderCredentialInput {
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
    ) -> Result<Zeroizing<String>, ZoneMinderPairingServiceError> {
        let bytes = read_owner_only_secret(path, length).map_err(map_secret_file_error)?;
        let value = std::str::from_utf8(bytes.as_slice())
            .map_err(|_| ZoneMinderPairingServiceError::SecretInput("secret is not UTF-8"))?;
        Ok(Zeroizing::new(value.to_string()))
    }
}

impl fmt::Debug for OwnerOnlyZoneMinderCredentialInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OwnerOnlyZoneMinderCredentialInput")
            .field("bridge_id", &self.bridge_id)
            .field("paths", &"[REDACTED]")
            .field("consumed", &self.consumed)
            .finish()
    }
}

impl ZoneMinderCredentialInput for OwnerOnlyZoneMinderCredentialInput {
    fn take_for_bridge(
        &mut self,
        bridge: &Bridge,
    ) -> Result<ZoneMinderCredentialSecret, ZoneMinderPairingServiceError> {
        if bridge.bridge_id != self.bridge_id {
            return Err(ZoneMinderPairingServiceError::SecretInput(
                "credential input is bound to another bridge",
            ));
        }
        if self.consumed {
            return Err(ZoneMinderPairingServiceError::SecretInput(
                "credential input was already consumed",
            ));
        }
        let username = Self::read_utf8(&self.username_path, self.username_length)?;
        let password = Self::read_utf8(&self.password_path, self.password_length)?;
        let secret = ZoneMinderCredentialSecret::new(username.as_str(), password.as_str())?;
        self.consumed = true;
        Ok(secret)
    }
}

fn map_secret_file_error(error: SecretFileError) -> ZoneMinderPairingServiceError {
    let message = match error {
        SecretFileError::InvalidPath => "secret path is invalid",
        SecretFileError::ParentUnavailable => "secret parent is unavailable",
        SecretFileError::AccessFailed => "secret file is unavailable",
        SecretFileError::UnsafeFileType => "secret file type is unsafe",
        SecretFileError::InsecureOwner => "secret file owner is unsafe",
        SecretFileError::InsecurePermissions => "secret file permissions are unsafe",
        SecretFileError::InvalidLength => "secret length is invalid",
    };
    ZoneMinderPairingServiceError::SecretInput(message)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedZoneMinderNvr {
    pub monitor_count: usize,
}

pub trait ZoneMinderPairingVerifier {
    fn verify(
        &mut self,
        bridge: &Bridge,
        credentials: &ZoneMinderCredentialSecret,
        expected_monitor_ids: &BTreeSet<u64>,
    ) -> Result<VerifiedZoneMinderNvr, ZoneMinderPairingServiceError>;
}

#[derive(Default)]
pub struct NativeZoneMinderPairingVerifier;

impl ZoneMinderPairingVerifier for NativeZoneMinderPairingVerifier {
    fn verify(
        &mut self,
        bridge: &Bridge,
        credentials: &ZoneMinderCredentialSecret,
        expected_monitor_ids: &BTreeSet<u64>,
    ) -> Result<VerifiedZoneMinderNvr, ZoneMinderPairingServiceError> {
        let address = bridge.address.as_deref().ok_or_else(|| {
            ZoneMinderPairingServiceError::MissingBridgeAddress(bridge.bridge_id.clone())
        })?;
        let config = ZoneMinderConfig::new(
            bridge.bridge_id.clone(),
            address,
            VaultRef::trusted("vault://smart-home/zoneminder/verification-only"),
        )?;
        let credentials =
            ZoneMinderCredentials::new(credentials.username(), credentials.password())?;
        let mut client =
            ZoneMinderClient::new(config, credentials, ZoneMinderLanTransport::default())?;
        let snapshot = client.inspect()?;
        let observed_monitor_ids = snapshot
            .monitors
            .iter()
            .map(|monitor| monitor.id)
            .collect::<BTreeSet<_>>();
        validate_monitor_correspondence(expected_monitor_ids, &observed_monitor_ids)?;
        Ok(VerifiedZoneMinderNvr {
            monitor_count: observed_monitor_ids.len(),
        })
    }
}

fn validate_monitor_correspondence(
    expected_monitor_ids: &BTreeSet<u64>,
    observed_monitor_ids: &BTreeSet<u64>,
) -> Result<(), ZoneMinderPairingServiceError> {
    if observed_monitor_ids.is_empty()
        || (!expected_monitor_ids.is_empty() && observed_monitor_ids != expected_monitor_ids)
    {
        Err(ZoneMinderPairingServiceError::MonitorCorrespondence)
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ZoneMinderPairingServiceSnapshot {
    pub request_count: u64,
    pub completed_count: u64,
    pub failed_count: u64,
    pub recovered_transaction_count: u64,
    pub last_completed_at_ms: Option<u64>,
    pub last_bridge_id: Option<BridgeId>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZoneMinderPairingReport {
    pub session_id: RuntimePairingSessionId,
    pub bridge_id: BridgeId,
    pub vault_ref: VaultRef,
    pub completed_at_ms: u64,
    pub monitor_count: usize,
}

pub struct ZoneMinderPairingServiceActorState<I, V, J, R> {
    runtime: SmartHomeRuntime,
    automation_definitions: Vec<DurableAutomationDefinition>,
    automation_state: Option<serde_json::Value>,
    runtime_revision: Revision,
    journal_backend: J,
    runtime_store: SmartHomeRuntimeStore<R>,
    vault: Arc<SealedStore>,
    credential_input: I,
    verifier: V,
    snapshot: ZoneMinderPairingServiceSnapshot,
    last_report: Option<ZoneMinderPairingReport>,
}

impl<I, V, J, R> ZoneMinderPairingServiceActorState<I, V, J, R>
where
    I: ZoneMinderCredentialInput,
    V: ZoneMinderPairingVerifier,
    J: StorageBackend,
    R: StorageBackend,
{
    pub fn restore(
        journal_backend: J,
        vault: Arc<SealedStore>,
        runtime_store: SmartHomeRuntimeStore<R>,
        credential_input: I,
        verifier: V,
    ) -> Result<Self, ZoneMinderPairingServiceError> {
        let mut restored = runtime_store
            .load()?
            .ok_or(ZoneMinderPairingServiceError::MissingDurableRuntime)?;
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
                            .ok_or(ZoneMinderPairingServiceError::MissingDurableRuntime)?;
                    }
                }
            }
            if !coordinator.pending_transaction_ids()?.is_empty() {
                return Err(ZoneMinderPairingServiceError::InvalidRequest(
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
            snapshot: ZoneMinderPairingServiceSnapshot {
                recovered_transaction_count,
                ..ZoneMinderPairingServiceSnapshot::default()
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

    pub fn snapshot(&self) -> &ZoneMinderPairingServiceSnapshot {
        &self.snapshot
    }

    pub fn last_report(&self) -> Option<&ZoneMinderPairingReport> {
        self.last_report.as_ref()
    }

    pub fn pair(
        &mut self,
        request: ZoneMinderPairingRequest,
    ) -> Result<&ZoneMinderPairingReport, ZoneMinderPairingServiceError> {
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
        request: ZoneMinderPairingRequest,
    ) -> Result<ZoneMinderPairingReport, ZoneMinderPairingServiceError> {
        let session = self
            .runtime
            .pairing_session(&request.session_id)
            .cloned()
            .ok_or_else(|| {
                ZoneMinderPairingServiceError::UnknownSession(request.session_id.clone())
            })?;
        if session.status != PairingSessionStatus::PendingUserPresence {
            return Err(ZoneMinderPairingServiceError::SessionNotPending {
                session_id: session.session_id,
                status: session.status,
            });
        }
        let bridge = self
            .runtime
            .registry()
            .bridge(&session.bridge_id)
            .cloned()
            .ok_or_else(|| {
                ZoneMinderPairingServiceError::UnknownBridge(session.bridge_id.clone())
            })?;
        if bridge.integration_id.as_str() != ZONEMINDER_INTEGRATION_ID {
            return Err(ZoneMinderPairingServiceError::WrongIntegration(
                bridge.integration_id,
            ));
        }
        let https_endpoint = exact_https_endpoint(&bridge)?;
        let expected_monitor_ids = installed_monitor_ids(&self.runtime, &bridge)?;
        ZoneMinderConfig::new(
            bridge.bridge_id.clone(),
            https_endpoint.as_str(),
            VaultRef::trusted("vault://smart-home/zoneminder/validation-only"),
        )?;
        if request.expected_runtime_revision != self.runtime_revision {
            return Err(ZoneMinderPairingServiceError::InvalidRequest(
                "expected runtime revision is stale".to_string(),
            ));
        }

        let authorization_probe = RuntimeCompletePairingToolRequest::new(
            request.session_id.clone(),
            VaultRef::trusted("vault://smart-home/zoneminder/authorization-preflight"),
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
            .verify(&bridge, &credentials, &expected_monitor_ids)?;
        let payload =
            encode_zoneminder_credentials(credentials.username(), credentials.password())?;
        let transaction_id = new_transaction_id()?;
        let vault_key = format!("{}/{}", bridge.bridge_id.as_str(), transaction_id);
        let vault_ref = VaultRef::trusted(format!("{ZONEMINDER_VAULT_REF_PREFIX}{vault_key}"));
        let new_credential = PairingCredentialLocation::new(
            vault_ref.clone(),
            ZONEMINDER_VAULT_NAMESPACE,
            vault_key,
        )?;
        let previous_credential = bridge
            .auth_ref
            .as_ref()
            .map(zoneminder_credential_location)
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
            Metadata::new("zoneminder.pairing.verified", "true"),
            Metadata::new("zoneminder.pairing.https_endpoint", https_endpoint),
            Metadata::new(
                "zoneminder.pairing.monitor_count",
                verified.monitor_count.to_string(),
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
            return Err(ZoneMinderPairingServiceError::TransactionRolledBack(
                transaction_id,
            ));
        };
        self.install_restored_runtime(*restored);
        Ok(ZoneMinderPairingReport {
            session_id: request.session_id,
            bridge_id: bridge.bridge_id,
            vault_ref,
            completed_at_ms: request.completed_at_ms,
            monitor_count: verified.monitor_count,
        })
    }

    fn install_restored_runtime(&mut self, restored: RestoredSmartHomeRuntime) {
        self.runtime = restored.runtime;
        self.automation_definitions = restored.automation_definitions;
        self.automation_state = restored.automation_state;
        self.runtime_revision = restored.revision;
    }
}

pub fn install_zoneminder_pairing_service_actor<I, V, J, R>(
    system: &mut ActorSystem,
    actor_id: &str,
    state: ZoneMinderPairingServiceActorState<I, V, J, R>,
) -> Result<String, ActorError>
where
    I: ZoneMinderCredentialInput + 'static,
    V: ZoneMinderPairingVerifier + 'static,
    J: StorageBackend + 'static,
    R: StorageBackend + 'static,
{
    system.create_actor(
        actor_id,
        Box::new(state),
        Box::new(|state: Box<dyn Any>, message| {
            let mut state = *state
                .downcast::<ZoneMinderPairingServiceActorState<I, V, J, R>>()
                .expect("ZoneMinder pairing actor received the wrong state type");
            match ZoneMinderPairingRequest::from_message(message) {
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
    vault_ref.as_str().strip_prefix(ZONEMINDER_VAULT_REF_PREFIX)
}

fn exact_https_endpoint(bridge: &Bridge) -> Result<String, ZoneMinderPairingServiceError> {
    let matches = bridge
        .identifiers
        .iter()
        .filter(|identifier| {
            identifier.family == ProtocolFamily::Vendor(PROTOCOL_ID.to_string())
                && identifier.kind == HTTPS_ENDPOINT_KIND
        })
        .collect::<Vec<_>>();
    if matches.len() != 1
        || bridge.address.as_deref() != Some(matches[0].value.as_str())
        || !matches[0].value.starts_with("https://")
    {
        return Err(ZoneMinderPairingServiceError::InvalidHttpsEndpoint(
            bridge.bridge_id.clone(),
        ));
    }
    Ok(matches[0].value.clone())
}

fn installed_monitor_ids(
    runtime: &SmartHomeRuntime,
    bridge: &Bridge,
) -> Result<BTreeSet<u64>, ZoneMinderPairingServiceError> {
    let expected_family = ProtocolFamily::Vendor(PROTOCOL_ID.to_string());
    let mut monitor_ids = BTreeSet::new();
    for device in runtime.registry().devices_for_bridge(&bridge.bridge_id) {
        let matches = device
            .identifiers
            .iter()
            .filter(|identifier| {
                identifier.family == expected_family && identifier.kind == MONITOR_ID_KIND
            })
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            return Err(ZoneMinderPairingServiceError::InvalidInstalledMonitors(
                bridge.bridge_id.clone(),
            ));
        }
        let monitor_id = matches[0]
            .value
            .parse::<u64>()
            .ok()
            .filter(|value| *value > 0)
            .ok_or_else(|| {
                ZoneMinderPairingServiceError::InvalidInstalledMonitors(bridge.bridge_id.clone())
            })?;
        if !monitor_ids.insert(monitor_id) {
            return Err(ZoneMinderPairingServiceError::InvalidInstalledMonitors(
                bridge.bridge_id.clone(),
            ));
        }
    }
    Ok(monitor_ids)
}

fn zoneminder_credential_location(
    vault_ref: &VaultRef,
) -> Result<PairingCredentialLocation, ZoneMinderPairingServiceError> {
    let key = vault_record_key(vault_ref).ok_or_else(|| {
        ZoneMinderPairingServiceError::ExistingCredentialReference(vault_ref.clone())
    })?;
    Ok(PairingCredentialLocation::new(
        vault_ref.clone(),
        ZONEMINDER_VAULT_NAMESPACE,
        key,
    )?)
}

fn new_transaction_id() -> Result<String, ZoneMinderPairingServiceError> {
    let random: [u8; 24] = random_array()
        .map_err(|error| ZoneMinderPairingServiceError::Entropy(error.to_string()))?;
    let mut suffix = String::with_capacity(random.len() * 2);
    for byte in random {
        use std::fmt::Write as _;
        write!(&mut suffix, "{byte:02x}").expect("writing to a String cannot fail");
    }
    Ok(format!("zoneminder-{suffix}"))
}

fn required_json_string(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &str,
) -> Result<String, ZoneMinderPairingServiceError> {
    object
        .get(field)
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| {
            ZoneMinderPairingServiceError::InvalidRequest(format!("`{field}` must be a string"))
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
        BridgeTransport, CapabilityGrant, CapabilityGrantId, CapabilityId, Device, DeviceId,
        Health, PrivilegeTier, ProtocolIdentifier,
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
            "smart-home-zoneminder-pairing-service-{}-{label}-{suffix}",
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
            BridgeId::trusted("zoneminder-camera-front"),
            IntegrationId::trusted(ZONEMINDER_INTEGRATION_ID),
            BridgeTransport::LanHttp,
        );
        bridge.address = Some("https://zoneminder.local/zm".to_string());
        bridge.health = Health::Unpaired;
        bridge.auth_ref = previous;
        bridge.identifiers.push(
            ProtocolIdentifier::new(
                ProtocolFamily::Vendor(PROTOCOL_ID.to_string()),
                HTTPS_ENDPOINT_KIND,
                "https://zoneminder.local/zm",
            )
            .unwrap(),
        );
        runtime.upsert_bridge(bridge.clone()).unwrap();
        let principal = AgentId::trusted("operator");
        runtime
            .start_pairing_session(RuntimePairingSession::pending(
                RuntimePairingSessionId::trusted("zoneminder-pairing-1"),
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
                    CapabilityGrantId::trusted("grant-zoneminder-pairing"),
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

    impl ZoneMinderCredentialInput for FixedInput {
        fn take_for_bridge(
            &mut self,
            bridge: &Bridge,
        ) -> Result<ZoneMinderCredentialSecret, ZoneMinderPairingServiceError> {
            assert_eq!(bridge.bridge_id.as_str(), "zoneminder-camera-front");
            self.calls.fetch_add(1, Ordering::SeqCst);
            ZoneMinderCredentialSecret::new(self.username, self.password)
        }
    }

    struct ExactVerifier {
        calls: Arc<AtomicUsize>,
    }

    impl ZoneMinderPairingVerifier for ExactVerifier {
        fn verify(
            &mut self,
            bridge: &Bridge,
            credentials: &ZoneMinderCredentialSecret,
            expected_monitor_ids: &BTreeSet<u64>,
        ) -> Result<VerifiedZoneMinderNvr, ZoneMinderPairingServiceError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            assert_eq!(
                bridge.address.as_deref(),
                Some("https://zoneminder.local/zm")
            );
            assert_eq!(
                exact_https_endpoint(bridge).unwrap(),
                "https://zoneminder.local/zm"
            );
            assert!(expected_monitor_ids.is_empty());
            assert_eq!(credentials.username(), "camera-user");
            assert_eq!(credentials.password(), "camera-password");
            assert_eq!(
                format!("{credentials:?}"),
                "ZoneMinderCredentialSecret([REDACTED])"
            );
            Ok(VerifiedZoneMinderNvr { monitor_count: 2 })
        }
    }

    type LocalService = ZoneMinderPairingServiceActorState<
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
    ) -> Result<LocalService, ZoneMinderPairingServiceError> {
        ZoneMinderPairingServiceActorState::restore(
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

    fn request(service: &LocalService) -> ZoneMinderPairingRequest {
        ZoneMinderPairingRequest::new(
            RuntimePairingSessionId::trusted("zoneminder-pairing-1"),
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
        assert_eq!(report.monitor_count, 2);
        assert_ne!(service.runtime_revision(), &initial_revision);
        assert_eq!(
            service
                .runtime()
                .registry()
                .bridge(&BridgeId::trusted("zoneminder-camera-front"))
                .unwrap()
                .auth_ref,
            Some(report.vault_ref.clone())
        );
        let record = vault
            .get(
                ZONEMINDER_VAULT_NAMESPACE,
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
            ZoneMinderPairingServiceError::Runtime(_)
        ));
        assert_eq!(input_calls.load(Ordering::SeqCst), 0);
        assert_eq!(verifier_calls.load(Ordering::SeqCst), 0);

        let mut ambiguous = service
            .runtime()
            .registry()
            .bridge(&BridgeId::trusted("zoneminder-camera-front"))
            .unwrap()
            .clone();
        ambiguous.identifiers.push(
            ProtocolIdentifier::new(
                ProtocolFamily::Vendor(PROTOCOL_ID.to_string()),
                HTTPS_ENDPOINT_KIND,
                "https://other-zoneminder.local/zm",
            )
            .unwrap(),
        );
        service.runtime.upsert_bridge(ambiguous).unwrap();
        let current = request(&service);
        assert!(matches!(
            service.pair(current).unwrap_err(),
            ZoneMinderPairingServiceError::InvalidHttpsEndpoint(_)
        ));
        assert_eq!(input_calls.load(Ordering::SeqCst), 0);
        assert_eq!(verifier_calls.load(Ordering::SeqCst), 0);
        assert!(vault
            .list(ZONEMINDER_VAULT_NAMESPACE, Default::default())
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
        let stale = ZoneMinderPairingRequest::new(
            RuntimePairingSessionId::trusted("zoneminder-pairing-1"),
            AgentId::trusted("operator"),
            Revision::new("stale-runtime").unwrap(),
            2_000,
        );
        assert!(matches!(
            service.pair(stale).unwrap_err(),
            ZoneMinderPairingServiceError::InvalidRequest(_)
        ));
        assert_eq!(input_calls.load(Ordering::SeqCst), 0);
        assert_eq!(verifier_calls.load(Ordering::SeqCst), 0);

        let mut embedded_credentials = service
            .runtime()
            .registry()
            .bridge(&BridgeId::trusted("zoneminder-camera-front"))
            .unwrap()
            .clone();
        embedded_credentials.address =
            Some("https://operator:password@zoneminder.local/zm".to_string());
        embedded_credentials.identifiers[0].value =
            "https://operator:password@zoneminder.local/zm".to_string();
        service.runtime.upsert_bridge(embedded_credentials).unwrap();
        let current = request(&service);
        assert!(matches!(
            service.pair(current).unwrap_err(),
            ZoneMinderPairingServiceError::ZoneMinder(_)
        ));
        assert_eq!(input_calls.load(Ordering::SeqCst), 0);
        assert_eq!(verifier_calls.load(Ordering::SeqCst), 0);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn installed_monitor_identity_is_exact_positive_and_unique() {
        let mut runtime = runtime_for_bridge(true, None);
        let bridge = runtime
            .registry()
            .bridge(&BridgeId::trusted("zoneminder-camera-front"))
            .unwrap()
            .clone();
        runtime
            .upsert_device(Device {
                device_id: DeviceId::trusted("zoneminder:monitor:7"),
                bridge_id: bridge.bridge_id.clone(),
                manufacturer: "ZoneMinder".to_string(),
                model: "Managed camera".to_string(),
                name: "Front".to_string(),
                serial: None,
                firmware_version: None,
                room_id: None,
                entity_ids: Vec::new(),
                identifiers: vec![ProtocolIdentifier::new(
                    ProtocolFamily::Vendor(PROTOCOL_ID.to_string()),
                    MONITOR_ID_KIND,
                    "7",
                )
                .unwrap()],
                health: Health::Online,
                metadata: Vec::new(),
            })
            .unwrap();
        assert_eq!(
            installed_monitor_ids(&runtime, &bridge).unwrap(),
            BTreeSet::from([7])
        );

        let mut invalid = runtime
            .registry()
            .device(&DeviceId::trusted("zoneminder:monitor:7"))
            .unwrap()
            .clone();
        invalid.identifiers[0].value = "0".to_string();
        runtime.upsert_device(invalid).unwrap();
        assert!(matches!(
            installed_monitor_ids(&runtime, &bridge).unwrap_err(),
            ZoneMinderPairingServiceError::InvalidInstalledMonitors(_)
        ));
    }

    #[test]
    fn monitor_correspondence_requires_a_nonempty_exact_installed_set() {
        assert!(validate_monitor_correspondence(&BTreeSet::new(), &BTreeSet::new()).is_err());
        assert!(validate_monitor_correspondence(&BTreeSet::new(), &BTreeSet::from([2])).is_ok());
        assert!(
            validate_monitor_correspondence(&BTreeSet::from([2]), &BTreeSet::from([2])).is_ok()
        );
        assert!(
            validate_monitor_correspondence(&BTreeSet::from([2]), &BTreeSet::from([2, 3])).is_err()
        );
    }

    #[test]
    fn native_verifier_uses_api_v2_login_and_exact_monitor_inspection() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let server_requests = Arc::clone(&requests);
        let responses = vec![
            r#"{"access_token":"secret.jwt.value","access_token_expires":3600,"refresh_token":"refresh.secret","refresh_token_expires":86400,"version":"1.36.33","apiversion":"2.0"}"#,
            r#"{"version":"1.36.33","apiversion":"2.0"}"#,
            r#"{"monitors":[{"Monitor":{"Id":"2","Name":"Front","Enabled":"1","Capturing":"Always","Analysing":"Always","Recording":"OnMotion"},"Monitor_Status":{"MonitorId":"2","Status":"Connected","CaptureFPS":"5.00","AnalysisFPS":"1.67","CaptureBandwidth":"52095"}}]}"#,
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
            BridgeId::trusted("zoneminder-camera-front"),
            IntegrationId::trusted(ZONEMINDER_INTEGRATION_ID),
            BridgeTransport::LanHttp,
        );
        bridge.address = Some(format!("http://{address}/zm"));
        let credentials =
            ZoneMinderCredentialSecret::new("operator name", "secret&password").unwrap();
        let verified = NativeZoneMinderPairingVerifier
            .verify(&bridge, &credentials, &BTreeSet::from([2]))
            .unwrap();
        handle.join().unwrap();

        assert_eq!(verified.monitor_count, 1);
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 3);
        assert_eq!(requests[0].1, "user=operator%20name&pass=secret%26password");
        assert!(requests[1]
            .0
            .starts_with("GET /zm/api/host/getVersion.json?token=secret.jwt.value HTTP/1.1"));
        assert!(requests[2]
            .0
            .starts_with("GET /zm/api/monitors.json?token=secret.jwt.value HTTP/1.1"));
        assert!(!format!("{verified:?}").contains("secret.jwt.value"));
    }

    #[test]
    fn actor_message_contains_authority_and_revision_but_no_secret_input() {
        let request = ZoneMinderPairingRequest::new(
            RuntimePairingSessionId::trusted("zoneminder-pairing-1"),
            AgentId::trusted("operator"),
            Revision::new("runtime-r1").unwrap(),
            2_000,
        );
        let message = request.clone().into_message("scheduler").unwrap();
        assert_eq!(
            ZoneMinderPairingRequest::from_message(&message).unwrap(),
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
            .bridge(&BridgeId::trusted("zoneminder-camera-front"))
            .unwrap()
            .clone();
        let mut input = OwnerOnlyZoneMinderCredentialInput::new(
            bridge.bridge_id.clone(),
            &username_path,
            11,
            &password_path,
            15,
        );
        let mut other_bridge = bridge.clone();
        other_bridge.bridge_id = BridgeId::trusted("zoneminder-camera-other");
        assert!(matches!(
            input.take_for_bridge(&other_bridge).unwrap_err(),
            ZoneMinderPairingServiceError::SecretInput(
                "credential input is bound to another bridge"
            )
        ));
        let secret = input.take_for_bridge(&bridge).unwrap();
        assert_eq!(secret.username(), "camera-user");
        assert_eq!(secret.password(), "camera-password");
        assert!(matches!(
            input.take_for_bridge(&bridge).unwrap_err(),
            ZoneMinderPairingServiceError::SecretInput("credential input was already consumed")
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
        let new_key = format!("zoneminder-camera-front/{transaction_id}");
        let new_ref = VaultRef::trusted(format!("{ZONEMINDER_VAULT_REF_PREFIX}{new_key}"));
        let request = PairingTransactionRequest::new(
            transaction_id,
            AgentId::trusted("operator"),
            BridgeId::trusted("zoneminder-camera-front"),
            RuntimePairingSessionId::trusted("zoneminder-pairing-1"),
            PairingCredentialLocation::new(new_ref, ZONEMINDER_VAULT_NAMESPACE, new_key).unwrap(),
            2_000,
            runtime_revision,
        )
        .unwrap()
        .with_metadata(vec![Metadata::new("zoneminder.pairing.verified", "true")]);
        match previous {
            Some(previous) => {
                request.with_previous_credential(zoneminder_credential_location(&previous).unwrap())
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
                    transaction_request("zoneminder-restart", revision, None),
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
                .pairing_session(&RuntimePairingSessionId::trusted("zoneminder-pairing-1"))
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
            "{ZONEMINDER_VAULT_REF_PREFIX}zoneminder-camera-front/previous"
        ));
        let old_key = vault_record_key(&old_ref).unwrap().to_string();
        vault
            .put(
                ZONEMINDER_VAULT_NAMESPACE,
                &old_key,
                b"previous-secret",
                None,
            )
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
            .get(ZONEMINDER_VAULT_NAMESPACE, &old_key)
            .unwrap()
            .is_none());
        assert!(vault
            .get(
                ZONEMINDER_VAULT_NAMESPACE,
                vault_record_key(&report.vault_ref).unwrap(),
            )
            .unwrap()
            .is_some());
        assert_eq!(
            service
                .runtime()
                .registry()
                .bridge(&BridgeId::trusted("zoneminder-camera-front"))
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
            "{ZONEMINDER_VAULT_REF_PREFIX}zoneminder-camera-front/previous"
        ));
        let old_key = vault_record_key(&old_ref).unwrap().to_string();
        let old_revision = vault
            .put(
                ZONEMINDER_VAULT_NAMESPACE,
                &old_key,
                b"previous-secret",
                None,
            )
            .unwrap();
        let revision = persist_runtime(&root, &runtime_for_bridge(true, Some(old_ref.clone())));
        let runtime_store =
            SmartHomeRuntimeStore::new(LocalFolderStorageBackend::new(runtime_root(&root)));
        let failing_journal = FailOnPutBackend::new(journal_root(&root), 3);
        assert!(
            PairingTransactionCoordinator::new(&failing_journal, &vault, &runtime_store)
                .execute(
                    transaction_request("zoneminder-cleanup", revision, Some(old_ref)),
                    br#"{"schema_version":1,"username":"u","password":"p"}"#,
                )
                .is_err()
        );
        vault
            .put(
                ZONEMINDER_VAULT_NAMESPACE,
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
            Err(ZoneMinderPairingServiceError::Transaction(_))
        ));
        assert_eq!(
            vault
                .get(ZONEMINDER_VAULT_NAMESPACE, &old_key)
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
