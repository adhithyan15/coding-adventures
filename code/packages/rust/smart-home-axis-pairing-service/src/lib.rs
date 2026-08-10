//! Actor-owned Axis credential verification and recoverable durable handoff.

#![forbid(unsafe_code)]

use std::any::Any;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use actor::{ActorError, ActorResult, ActorSystem, Message};
use chief_of_staff_daemon_secret_file::{read_owner_only_secret, SecretFileError};
use coding_adventures_csprng::random_array;
use coding_adventures_vault_sealed_store::SealedStore;
use coding_adventures_zeroize::Zeroizing;
use smart_home_axis_snapshot_host::{
    encode_axis_credentials, AxisSnapshotHostError, AXIS_VAULT_NAMESPACE, AXIS_VAULT_REF_PREFIX,
};
use smart_home_axis_vapix_integration::{
    AxisClient, AxisConfig, AxisCredentials, AxisError, AxisLanTransport, INTEGRATION_ID,
    PROTOCOL_ID,
};
use smart_home_core::{
    AgentId, Bridge, BridgeId, CapabilityId, EntityKind, IntegrationId, Metadata, ProtocolFamily,
    VaultRef,
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

pub const PAIR_REQUEST_CONTENT_TYPE: &str = "application/vnd.smart-home.axis-pairing-request+json";
const HTTPS_ENDPOINT_KIND: &str = "https_endpoint";
const SERIAL_KIND: &str = "serial";
const CAMERA_CHANNEL: &str = "1";

#[derive(Debug)]
pub enum AxisPairingServiceError {
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
    MissingCameraCapability,
    InvalidRequest(String),
    SecretInput(&'static str),
    Axis(AxisError),
    CredentialEncoding(AxisSnapshotHostError),
    Runtime(RuntimeError),
    RuntimeStore(RuntimeStoreError),
    Transaction(PairingTransactionError),
    Entropy(String),
    MissingDurableRuntime,
    ExistingCredentialReference(VaultRef),
    TransactionRolledBack(String),
}

impl fmt::Display for AxisPairingServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownSession(session_id) => {
                write!(formatter, "unknown Axis pairing session {session_id}")
            }
            Self::SessionNotPending { session_id, status } => write!(
                formatter,
                "Axis pairing session {session_id} is not pending user presence ({status:?})"
            ),
            Self::UnknownBridge(bridge_id) => write!(formatter, "unknown Axis bridge {bridge_id}"),
            Self::WrongIntegration(integration_id) => write!(
                formatter,
                "pairing service only accepts Axis bridges, got integration {integration_id}"
            ),
            Self::MissingBridgeAddress(bridge_id) => {
                write!(formatter, "Axis bridge {bridge_id} has no HTTPS address")
            }
            Self::InvalidHttpsEndpoint(bridge_id) => write!(
                formatter,
                "Axis bridge {bridge_id} must have one exact HTTPS endpoint identifier"
            ),
            Self::InvalidInstalledCamera(bridge_id) => write!(
                formatter,
                "Axis bridge {bridge_id} has an ambiguous installed camera identity"
            ),
            Self::CameraCorrespondence => formatter.write_str(
                "Axis credential verification did not match the installed camera identity",
            ),
            Self::MissingCameraCapability => formatter.write_str(
                "Axis credential verification did not prove enabled camera-1 JPEG snapshots",
            ),
            Self::InvalidRequest(message) => {
                write!(formatter, "invalid Axis pairing request: {message}")
            }
            Self::SecretInput(message) => write!(formatter, "Axis secret input failed: {message}"),
            Self::Axis(error) => write!(formatter, "Axis credential verification failed: {error}"),
            Self::CredentialEncoding(_) => {
                formatter.write_str("Axis credential envelope encoding failed")
            }
            Self::Runtime(error) => write!(formatter, "Axis runtime completion failed: {error}"),
            Self::RuntimeStore(error) => write!(formatter, "Axis runtime store failed: {error}"),
            Self::Transaction(error) => write!(formatter, "Axis pairing transaction failed: {error}"),
            Self::Entropy(message) => write!(
                formatter,
                "Axis pairing transaction generation failed: {message}"
            ),
            Self::MissingDurableRuntime => {
                formatter.write_str("Axis pairing requires a durable runtime snapshot")
            }
            Self::ExistingCredentialReference(vault_ref) => write!(
                formatter,
                "existing Axis credential reference is outside the Axis Vault namespace: {vault_ref}"
            ),
            Self::TransactionRolledBack(transaction_id) => write!(
                formatter,
                "Axis pairing transaction {transaction_id} rolled back before runtime commit"
            ),
        }
    }
}

impl std::error::Error for AxisPairingServiceError {}

impl From<AxisError> for AxisPairingServiceError {
    fn from(error: AxisError) -> Self {
        Self::Axis(error)
    }
}

impl From<AxisSnapshotHostError> for AxisPairingServiceError {
    fn from(error: AxisSnapshotHostError) -> Self {
        Self::CredentialEncoding(error)
    }
}

impl From<RuntimeError> for AxisPairingServiceError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

impl From<RuntimeStoreError> for AxisPairingServiceError {
    fn from(error: RuntimeStoreError) -> Self {
        Self::RuntimeStore(error)
    }
}

impl From<PairingTransactionError> for AxisPairingServiceError {
    fn from(error: PairingTransactionError) -> Self {
        Self::Transaction(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AxisPairingRequest {
    pub session_id: RuntimePairingSessionId,
    pub principal_id: AgentId,
    pub expected_runtime_revision: Revision,
    pub completed_at_ms: u64,
}

impl AxisPairingRequest {
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

    pub fn into_message(self, sender_id: &str) -> Result<Message, AxisPairingServiceError> {
        let payload = serde_json::to_vec(&serde_json::json!({
            "schema_version": 1,
            "session_id": self.session_id.as_str(),
            "principal_id": self.principal_id.as_str(),
            "expected_runtime_revision": self.expected_runtime_revision.as_str(),
            "completed_at_ms": self.completed_at_ms,
        }))
        .map_err(|error| AxisPairingServiceError::InvalidRequest(error.to_string()))?;
        Ok(Message::new(
            sender_id,
            PAIR_REQUEST_CONTENT_TYPE,
            payload,
            None,
        ))
    }

    fn from_message(message: &Message) -> Result<Self, AxisPairingServiceError> {
        if message.content_type != PAIR_REQUEST_CONTENT_TYPE {
            return Err(AxisPairingServiceError::InvalidRequest(format!(
                "message content type must be `{PAIR_REQUEST_CONTENT_TYPE}`"
            )));
        }
        let value: serde_json::Value = serde_json::from_slice(&message.payload)
            .map_err(|error| AxisPairingServiceError::InvalidRequest(error.to_string()))?;
        let object = value.as_object().ok_or_else(|| {
            AxisPairingServiceError::InvalidRequest("message body must be an object".to_string())
        })?;
        if object
            .get("schema_version")
            .and_then(serde_json::Value::as_u64)
            != Some(1)
        {
            return Err(AxisPairingServiceError::InvalidRequest(
                "unsupported schema_version".to_string(),
            ));
        }
        Ok(Self::new(
            RuntimePairingSessionId::trusted(required_json_string(object, "session_id")?),
            AgentId::new(required_json_string(object, "principal_id")?)
                .map_err(|error| AxisPairingServiceError::InvalidRequest(error.to_string()))?,
            Revision::new(required_json_string(object, "expected_runtime_revision")?)
                .map_err(|error| AxisPairingServiceError::InvalidRequest(error.to_string()))?,
            object
                .get("completed_at_ms")
                .and_then(serde_json::Value::as_u64)
                .ok_or_else(|| {
                    AxisPairingServiceError::InvalidRequest(
                        "completed_at_ms must be a non-negative integer".to_string(),
                    )
                })?,
        ))
    }
}

pub struct AxisCredentialSecret {
    username: Zeroizing<String>,
    password: Zeroizing<String>,
}

impl AxisCredentialSecret {
    pub fn new(
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Result<Self, AxisPairingServiceError> {
        let username = Zeroizing::new(username.into());
        let password = Zeroizing::new(password.into());
        AxisCredentials::new(username.as_str(), password.as_str())?;
        Ok(Self { username, password })
    }

    fn username(&self) -> &str {
        self.username.as_str()
    }

    fn password(&self) -> &str {
        self.password.as_str()
    }
}

impl fmt::Debug for AxisCredentialSecret {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("AxisCredentialSecret([REDACTED])")
    }
}

pub trait AxisCredentialInput {
    fn take_for_bridge(
        &mut self,
        bridge: &Bridge,
    ) -> Result<AxisCredentialSecret, AxisPairingServiceError>;
}

pub struct OwnerOnlyAxisCredentialInput {
    bridge_id: BridgeId,
    username_path: PathBuf,
    username_length: usize,
    password_path: PathBuf,
    password_length: usize,
    consumed: bool,
}

impl OwnerOnlyAxisCredentialInput {
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

    fn read_utf8(path: &Path, length: usize) -> Result<Zeroizing<String>, AxisPairingServiceError> {
        let bytes = read_owner_only_secret(path, length).map_err(map_secret_file_error)?;
        let value = std::str::from_utf8(bytes.as_slice())
            .map_err(|_| AxisPairingServiceError::SecretInput("secret is not UTF-8"))?;
        Ok(Zeroizing::new(value.to_string()))
    }
}

impl fmt::Debug for OwnerOnlyAxisCredentialInput {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OwnerOnlyAxisCredentialInput")
            .field("bridge_id", &self.bridge_id)
            .field("paths", &"[REDACTED]")
            .field("consumed", &self.consumed)
            .finish()
    }
}

impl AxisCredentialInput for OwnerOnlyAxisCredentialInput {
    fn take_for_bridge(
        &mut self,
        bridge: &Bridge,
    ) -> Result<AxisCredentialSecret, AxisPairingServiceError> {
        if bridge.bridge_id != self.bridge_id {
            return Err(AxisPairingServiceError::SecretInput(
                "credential input is bound to another bridge",
            ));
        }
        if self.consumed {
            return Err(AxisPairingServiceError::SecretInput(
                "credential input was already consumed",
            ));
        }
        let username = Self::read_utf8(&self.username_path, self.username_length)?;
        let password = Self::read_utf8(&self.password_path, self.password_length)?;
        let secret = AxisCredentialSecret::new(username.as_str(), password.as_str())?;
        self.consumed = true;
        Ok(secret)
    }
}

fn map_secret_file_error(error: SecretFileError) -> AxisPairingServiceError {
    let message = match error {
        SecretFileError::InvalidPath => "secret path is invalid",
        SecretFileError::ParentUnavailable => "secret parent is unavailable",
        SecretFileError::AccessFailed => "secret file is unavailable",
        SecretFileError::UnsafeFileType => "secret file type is unsafe",
        SecretFileError::InsecureOwner => "secret file owner is unsafe",
        SecretFileError::InsecurePermissions => "secret file permissions are unsafe",
        SecretFileError::InvalidLength => "secret length is invalid",
    };
    AxisPairingServiceError::SecretInput(message)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedAxisCamera {
    pub serial_number: String,
}

pub trait AxisPairingVerifier {
    fn verify(
        &mut self,
        bridge: &Bridge,
        credentials: &AxisCredentialSecret,
        expected_serial_number: Option<&str>,
    ) -> Result<VerifiedAxisCamera, AxisPairingServiceError>;
}

#[derive(Default)]
pub struct NativeAxisPairingVerifier;

impl AxisPairingVerifier for NativeAxisPairingVerifier {
    fn verify(
        &mut self,
        bridge: &Bridge,
        credentials: &AxisCredentialSecret,
        expected_serial_number: Option<&str>,
    ) -> Result<VerifiedAxisCamera, AxisPairingServiceError> {
        let address = bridge.address.as_deref().ok_or_else(|| {
            AxisPairingServiceError::MissingBridgeAddress(bridge.bridge_id.clone())
        })?;
        let config = AxisConfig::new(
            bridge.bridge_id.clone(),
            address,
            VaultRef::trusted("vault://smart-home/axis/verification-only"),
        )?;
        let credentials = AxisCredentials::new(credentials.username(), credentials.password())?;
        let mut client = AxisClient::new(config, credentials, AxisLanTransport::default())?;
        let snapshot = client.inspect()?;
        let serial_number = snapshot.device.serial_number.trim();
        validate_camera_correspondence(expected_serial_number, serial_number)?;
        if !snapshot
            .video
            .as_ref()
            .is_some_and(|video| video.supports_jpeg_snapshot())
        {
            return Err(AxisPairingServiceError::MissingCameraCapability);
        }
        Ok(VerifiedAxisCamera {
            serial_number: serial_number.to_string(),
        })
    }
}

fn validate_camera_correspondence(
    expected_serial_number: Option<&str>,
    observed_serial_number: &str,
) -> Result<(), AxisPairingServiceError> {
    if observed_serial_number.is_empty()
        || expected_serial_number.is_some_and(|expected| expected != observed_serial_number)
    {
        Err(AxisPairingServiceError::CameraCorrespondence)
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AxisPairingServiceSnapshot {
    pub request_count: u64,
    pub completed_count: u64,
    pub failed_count: u64,
    pub recovered_transaction_count: u64,
    pub last_completed_at_ms: Option<u64>,
    pub last_bridge_id: Option<BridgeId>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AxisPairingReport {
    pub session_id: RuntimePairingSessionId,
    pub bridge_id: BridgeId,
    pub vault_ref: VaultRef,
    pub completed_at_ms: u64,
    pub serial_number: String,
}

pub struct AxisPairingServiceActorState<I, V, J, R> {
    runtime: SmartHomeRuntime,
    automation_definitions: Vec<DurableAutomationDefinition>,
    automation_state: Option<serde_json::Value>,
    runtime_revision: Revision,
    journal_backend: J,
    runtime_store: SmartHomeRuntimeStore<R>,
    vault: Arc<SealedStore>,
    credential_input: I,
    verifier: V,
    snapshot: AxisPairingServiceSnapshot,
    last_report: Option<AxisPairingReport>,
}

impl<I, V, J, R> AxisPairingServiceActorState<I, V, J, R>
where
    I: AxisCredentialInput,
    V: AxisPairingVerifier,
    J: StorageBackend,
    R: StorageBackend,
{
    pub fn restore(
        journal_backend: J,
        vault: Arc<SealedStore>,
        runtime_store: SmartHomeRuntimeStore<R>,
        credential_input: I,
        verifier: V,
    ) -> Result<Self, AxisPairingServiceError> {
        let mut restored = runtime_store
            .load()?
            .ok_or(AxisPairingServiceError::MissingDurableRuntime)?;
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
                            .ok_or(AxisPairingServiceError::MissingDurableRuntime)?;
                    }
                }
            }
            if !coordinator.pending_transaction_ids()?.is_empty() {
                return Err(AxisPairingServiceError::InvalidRequest(
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
            snapshot: AxisPairingServiceSnapshot {
                recovered_transaction_count,
                ..AxisPairingServiceSnapshot::default()
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

    pub fn snapshot(&self) -> &AxisPairingServiceSnapshot {
        &self.snapshot
    }

    pub fn last_report(&self) -> Option<&AxisPairingReport> {
        self.last_report.as_ref()
    }

    pub fn pair(
        &mut self,
        request: AxisPairingRequest,
    ) -> Result<&AxisPairingReport, AxisPairingServiceError> {
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
        request: AxisPairingRequest,
    ) -> Result<AxisPairingReport, AxisPairingServiceError> {
        let session = self
            .runtime
            .pairing_session(&request.session_id)
            .cloned()
            .ok_or_else(|| AxisPairingServiceError::UnknownSession(request.session_id.clone()))?;
        if session.status != PairingSessionStatus::PendingUserPresence {
            return Err(AxisPairingServiceError::SessionNotPending {
                session_id: session.session_id,
                status: session.status,
            });
        }
        let bridge = self
            .runtime
            .registry()
            .bridge(&session.bridge_id)
            .cloned()
            .ok_or_else(|| AxisPairingServiceError::UnknownBridge(session.bridge_id.clone()))?;
        if bridge.integration_id.as_str() != INTEGRATION_ID {
            return Err(AxisPairingServiceError::WrongIntegration(
                bridge.integration_id,
            ));
        }
        let https_endpoint = exact_https_endpoint(&bridge)?;
        let expected_serial_number = installed_camera_serial(&self.runtime, &bridge)?;
        AxisConfig::new(
            bridge.bridge_id.clone(),
            https_endpoint.as_str(),
            VaultRef::trusted("vault://smart-home/axis/validation-only"),
        )?;
        if request.expected_runtime_revision != self.runtime_revision {
            return Err(AxisPairingServiceError::InvalidRequest(
                "expected runtime revision is stale".to_string(),
            ));
        }

        let authorization_probe = RuntimeCompletePairingToolRequest::new(
            request.session_id.clone(),
            VaultRef::trusted("vault://smart-home/axis/authorization-preflight"),
            request.completed_at_ms,
        );
        self.runtime.clone().execute_complete_pairing_tool(
            request.principal_id.clone(),
            authorization_probe,
            request.completed_at_ms,
        )?;

        let credentials = self.credential_input.take_for_bridge(&bridge)?;
        let verified =
            self.verifier
                .verify(&bridge, &credentials, expected_serial_number.as_deref())?;
        let payload = encode_axis_credentials(credentials.username(), credentials.password())?;
        let transaction_id = new_transaction_id()?;
        let vault_key = format!("{}/{}", bridge.bridge_id.as_str(), transaction_id);
        let vault_ref = VaultRef::trusted(format!("{AXIS_VAULT_REF_PREFIX}{vault_key}"));
        let new_credential =
            PairingCredentialLocation::new(vault_ref.clone(), AXIS_VAULT_NAMESPACE, vault_key)?;
        let previous_credential = bridge
            .auth_ref
            .as_ref()
            .map(axis_credential_location)
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
            Metadata::new("axis.pairing.verified", "true"),
            Metadata::new("axis.pairing.https_endpoint", https_endpoint),
            Metadata::new("axis.pairing.serial_number", verified.serial_number.clone()),
            Metadata::new("axis.pairing.camera", CAMERA_CHANNEL),
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
            return Err(AxisPairingServiceError::TransactionRolledBack(
                transaction_id,
            ));
        };
        self.install_restored_runtime(*restored);
        Ok(AxisPairingReport {
            session_id: request.session_id,
            bridge_id: bridge.bridge_id,
            vault_ref,
            completed_at_ms: request.completed_at_ms,
            serial_number: verified.serial_number,
        })
    }

    fn install_restored_runtime(&mut self, restored: RestoredSmartHomeRuntime) {
        self.runtime = restored.runtime;
        self.automation_definitions = restored.automation_definitions;
        self.automation_state = restored.automation_state;
        self.runtime_revision = restored.revision;
    }
}

pub fn install_axis_pairing_service_actor<I, V, J, R>(
    system: &mut ActorSystem,
    actor_id: &str,
    state: AxisPairingServiceActorState<I, V, J, R>,
) -> Result<String, ActorError>
where
    I: AxisCredentialInput + 'static,
    V: AxisPairingVerifier + 'static,
    J: StorageBackend + 'static,
    R: StorageBackend + 'static,
{
    system.create_actor(
        actor_id,
        Box::new(state),
        Box::new(|state: Box<dyn Any>, message| {
            let mut state = *state
                .downcast::<AxisPairingServiceActorState<I, V, J, R>>()
                .expect("Axis pairing actor received the wrong state type");
            match AxisPairingRequest::from_message(message) {
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
    vault_ref.as_str().strip_prefix(AXIS_VAULT_REF_PREFIX)
}

fn exact_https_endpoint(bridge: &Bridge) -> Result<String, AxisPairingServiceError> {
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
        return Err(AxisPairingServiceError::InvalidHttpsEndpoint(
            bridge.bridge_id.clone(),
        ));
    }
    Ok(matches[0].value.clone())
}

fn installed_camera_serial(
    runtime: &SmartHomeRuntime,
    bridge: &Bridge,
) -> Result<Option<String>, AxisPairingServiceError> {
    let expected_family = ProtocolFamily::Vendor(PROTOCOL_ID.to_string());
    let devices = runtime
        .registry()
        .devices_for_bridge(&bridge.bridge_id)
        .collect::<Vec<_>>();
    if devices.is_empty() {
        return Ok(None);
    }
    if devices.len() != 1 {
        return Err(AxisPairingServiceError::InvalidInstalledCamera(
            bridge.bridge_id.clone(),
        ));
    }
    let device = devices[0];
    let serials = device
        .identifiers
        .iter()
        .filter(|identifier| identifier.family == expected_family && identifier.kind == SERIAL_KIND)
        .collect::<Vec<_>>();
    if serials.len() != 1 || serials[0].value.trim().is_empty() || device.entity_ids.len() != 1 {
        return Err(AxisPairingServiceError::InvalidInstalledCamera(
            bridge.bridge_id.clone(),
        ));
    }
    let entity = runtime
        .registry()
        .entity(&device.entity_ids[0])
        .ok_or_else(|| AxisPairingServiceError::InvalidInstalledCamera(bridge.bridge_id.clone()))?;
    let camera_one = entity.kind == EntityKind::Camera
        && entity.metadata.iter().any(|metadata| {
            metadata.key == "axis.video_channel" && metadata.value == CAMERA_CHANNEL
        })
        && entity
            .capabilities
            .iter()
            .any(|capability| capability.capability_id == CapabilityId::trusted("camera.snapshot"));
    if !camera_one {
        return Err(AxisPairingServiceError::InvalidInstalledCamera(
            bridge.bridge_id.clone(),
        ));
    }
    Ok(Some(serials[0].value.clone()))
}

fn axis_credential_location(
    vault_ref: &VaultRef,
) -> Result<PairingCredentialLocation, AxisPairingServiceError> {
    let key = vault_record_key(vault_ref)
        .ok_or_else(|| AxisPairingServiceError::ExistingCredentialReference(vault_ref.clone()))?;
    Ok(PairingCredentialLocation::new(
        vault_ref.clone(),
        AXIS_VAULT_NAMESPACE,
        key,
    )?)
}

fn new_transaction_id() -> Result<String, AxisPairingServiceError> {
    let random: [u8; 24] =
        random_array().map_err(|error| AxisPairingServiceError::Entropy(error.to_string()))?;
    let mut suffix = String::with_capacity(random.len() * 2);
    for byte in random {
        use std::fmt::Write as _;
        write!(&mut suffix, "{byte:02x}").expect("writing to a String cannot fail");
    }
    Ok(format!("axis-{suffix}"))
}

fn required_json_string(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &str,
) -> Result<String, AxisPairingServiceError> {
    object
        .get(field)
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| {
            AxisPairingServiceError::InvalidRequest(format!("`{field}` must be a string"))
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
    const DEVICE_INFO: &str = r#"{"apiVersion":"1.0","context":"smart-home","data":{"propertyList":{"Brand":"AXIS","ProdFullName":"AXIS Q1785-LE Network Camera","ProdNbr":"Q1785-LE","ProdType":"Network Camera","SerialNumber":"ACCC8EAF8C30","Version":"12.1.0"}}}"#;
    const API_LIST: &str = r#"{"apiVersion":"1.0","context":"smart-home","data":{"apiList":[{"id":"basic-device-info","version":"1.2","status":"released"}]}}"#;
    const VIDEO_CAPABILITY: &str = "root.Properties.API.HTTP.Version=3\nroot.Properties.Image.Format=jpeg,mjpeg,h264\nroot.Image.NbrOfConfigs=1\nroot.Image.I0.Enabled=yes\n";

    fn test_directory(label: &str) -> PathBuf {
        let suffix = TEST_DIRECTORY_COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!(
            "smart-home-axis-pairing-service-{}-{label}-{suffix}",
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
            BridgeId::trusted("axis-camera-front"),
            IntegrationId::trusted(INTEGRATION_ID),
            BridgeTransport::LanHttp,
        );
        bridge.address = Some("https://axis.local".to_string());
        bridge.health = Health::Unpaired;
        bridge.auth_ref = previous;
        bridge.identifiers.push(
            ProtocolIdentifier::new(
                ProtocolFamily::Vendor(PROTOCOL_ID.to_string()),
                HTTPS_ENDPOINT_KIND,
                "https://axis.local",
            )
            .unwrap(),
        );
        runtime.upsert_bridge(bridge.clone()).unwrap();
        let principal = AgentId::trusted("operator");
        runtime
            .start_pairing_session(RuntimePairingSession::pending(
                RuntimePairingSessionId::trusted("axis-pairing-1"),
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
                    CapabilityGrantId::trusted("grant-axis-pairing"),
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

    impl AxisCredentialInput for FixedInput {
        fn take_for_bridge(
            &mut self,
            bridge: &Bridge,
        ) -> Result<AxisCredentialSecret, AxisPairingServiceError> {
            assert_eq!(bridge.bridge_id.as_str(), "axis-camera-front");
            self.calls.fetch_add(1, Ordering::SeqCst);
            AxisCredentialSecret::new(self.username, self.password)
        }
    }

    struct ExactVerifier {
        calls: Arc<AtomicUsize>,
    }

    impl AxisPairingVerifier for ExactVerifier {
        fn verify(
            &mut self,
            bridge: &Bridge,
            credentials: &AxisCredentialSecret,
            expected_serial_number: Option<&str>,
        ) -> Result<VerifiedAxisCamera, AxisPairingServiceError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            assert_eq!(bridge.address.as_deref(), Some("https://axis.local"));
            assert_eq!(exact_https_endpoint(bridge).unwrap(), "https://axis.local");
            assert_eq!(expected_serial_number, None);
            assert_eq!(credentials.username(), "camera-user");
            assert_eq!(credentials.password(), "camera-password");
            assert_eq!(
                format!("{credentials:?}"),
                "AxisCredentialSecret([REDACTED])"
            );
            Ok(VerifiedAxisCamera {
                serial_number: "ACCC8EAF8C30".to_string(),
            })
        }
    }

    type LocalService = AxisPairingServiceActorState<
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
    ) -> Result<LocalService, AxisPairingServiceError> {
        AxisPairingServiceActorState::restore(
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

    fn request(service: &LocalService) -> AxisPairingRequest {
        AxisPairingRequest::new(
            RuntimePairingSessionId::trusted("axis-pairing-1"),
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
        assert_ne!(service.runtime_revision(), &initial_revision);
        assert_eq!(
            service
                .runtime()
                .registry()
                .bridge(&BridgeId::trusted("axis-camera-front"))
                .unwrap()
                .auth_ref,
            Some(report.vault_ref.clone())
        );
        let record = vault
            .get(
                AXIS_VAULT_NAMESPACE,
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
            AxisPairingServiceError::Runtime(_)
        ));
        assert_eq!(input_calls.load(Ordering::SeqCst), 0);
        assert_eq!(verifier_calls.load(Ordering::SeqCst), 0);

        let mut ambiguous = service
            .runtime()
            .registry()
            .bridge(&BridgeId::trusted("axis-camera-front"))
            .unwrap()
            .clone();
        ambiguous.identifiers.push(
            ProtocolIdentifier::new(
                ProtocolFamily::Vendor(PROTOCOL_ID.to_string()),
                HTTPS_ENDPOINT_KIND,
                "https://other-axis.local",
            )
            .unwrap(),
        );
        service.runtime.upsert_bridge(ambiguous).unwrap();
        let current = request(&service);
        assert!(matches!(
            service.pair(current).unwrap_err(),
            AxisPairingServiceError::InvalidHttpsEndpoint(_)
        ));
        assert_eq!(input_calls.load(Ordering::SeqCst), 0);
        assert_eq!(verifier_calls.load(Ordering::SeqCst), 0);
        assert!(vault
            .list(AXIS_VAULT_NAMESPACE, Default::default())
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
        let stale = AxisPairingRequest::new(
            RuntimePairingSessionId::trusted("axis-pairing-1"),
            AgentId::trusted("operator"),
            Revision::new("stale-runtime").unwrap(),
            2_000,
        );
        assert!(matches!(
            service.pair(stale).unwrap_err(),
            AxisPairingServiceError::InvalidRequest(_)
        ));
        assert_eq!(input_calls.load(Ordering::SeqCst), 0);
        assert_eq!(verifier_calls.load(Ordering::SeqCst), 0);

        let mut embedded_credentials = service
            .runtime()
            .registry()
            .bridge(&BridgeId::trusted("axis-camera-front"))
            .unwrap()
            .clone();
        embedded_credentials.address = Some("https://operator:password@axis.local".to_string());
        embedded_credentials.identifiers[0].value =
            "https://operator:password@axis.local".to_string();
        service.runtime.upsert_bridge(embedded_credentials).unwrap();
        let current = request(&service);
        assert!(matches!(
            service.pair(current).unwrap_err(),
            AxisPairingServiceError::Axis(_)
        ));
        assert_eq!(input_calls.load(Ordering::SeqCst), 0);
        assert_eq!(verifier_calls.load(Ordering::SeqCst), 0);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn installed_camera_identity_is_exact_and_capability_bound() {
        let mut runtime = runtime_for_bridge(true, None);
        let bridge = runtime
            .registry()
            .bridge(&BridgeId::trusted("axis-camera-front"))
            .unwrap()
            .clone();
        let camera_entity_id = smart_home_core::EntityId::trusted("axis:ACCC8EAF8C30:camera");
        runtime
            .upsert_device(Device {
                device_id: DeviceId::trusted("axis:ACCC8EAF8C30"),
                bridge_id: bridge.bridge_id.clone(),
                manufacturer: "Axis".to_string(),
                model: "Managed camera".to_string(),
                name: "Front".to_string(),
                serial: Some("ACCC8EAF8C30".to_string()),
                firmware_version: None,
                room_id: None,
                entity_ids: vec![camera_entity_id.clone()],
                identifiers: vec![ProtocolIdentifier::new(
                    ProtocolFamily::Vendor(PROTOCOL_ID.to_string()),
                    SERIAL_KIND,
                    "ACCC8EAF8C30",
                )
                .unwrap()],
                health: Health::Online,
                metadata: Vec::new(),
            })
            .unwrap();
        runtime
            .upsert_entity(Entity {
                entity_id: camera_entity_id,
                device_id: DeviceId::trusted("axis:ACCC8EAF8C30"),
                kind: EntityKind::Camera,
                name: "Front".to_string(),
                capabilities: vec![Capability::new(
                    CapabilityId::trusted("camera.snapshot"),
                    CapabilityMode::Command,
                    ValueKind::Text,
                )],
                state: None,
                metadata: vec![Metadata::new("axis.video_channel", CAMERA_CHANNEL)],
            })
            .unwrap();
        assert_eq!(
            installed_camera_serial(&runtime, &bridge).unwrap(),
            Some("ACCC8EAF8C30".to_string())
        );

        let mut invalid = runtime
            .registry()
            .device(&DeviceId::trusted("axis:ACCC8EAF8C30"))
            .unwrap()
            .clone();
        invalid.identifiers[0].value.clear();
        runtime.upsert_device(invalid).unwrap();
        assert!(matches!(
            installed_camera_serial(&runtime, &bridge).unwrap_err(),
            AxisPairingServiceError::InvalidInstalledCamera(_)
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
    fn native_verifier_uses_authenticated_vapix_and_exact_camera_one_inspection() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let server_requests = Arc::clone(&requests);
        let responses = vec![
            "HTTP/1.1 401 Unauthorized\r\nWWW-Authenticate: Basic realm=\"AXIS\"\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_string(),
            format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", DEVICE_INFO.len(), DEVICE_INFO),
            format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", API_LIST.len(), API_LIST),
            format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", VIDEO_CAPABILITY.len(), VIDEO_CAPABILITY),
        ];
        let handle = thread::spawn(move || {
            for response in responses {
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
                reader.get_mut().write_all(response.as_bytes()).unwrap();
            }
        });

        let mut bridge = Bridge::new(
            BridgeId::trusted("axis-camera-front"),
            IntegrationId::trusted(INTEGRATION_ID),
            BridgeTransport::LanHttp,
        );
        bridge.address = Some(format!("http://{address}"));
        let credentials = AxisCredentialSecret::new("operator name", "secret&password").unwrap();
        let verified = NativeAxisPairingVerifier
            .verify(&bridge, &credentials, Some("ACCC8EAF8C30"))
            .unwrap();
        handle.join().unwrap();

        assert_eq!(verified.serial_number, "ACCC8EAF8C30");
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 4);
        assert!(requests[0]
            .0
            .starts_with("POST /axis-cgi/basicdeviceinfo.cgi HTTP/1.1"));
        assert!(!requests[0].0.contains("Authorization:"));
        assert!(requests[1].0.contains("Authorization: Basic "));
        assert!(requests[2]
            .0
            .starts_with("POST /axis-cgi/apidiscovery.cgi HTTP/1.1"));
        assert!(requests[3]
            .0
            .starts_with("GET /axis-cgi/param.cgi?action=list"));
        assert!(!format!("{verified:?}").contains("secret&password"));
    }

    #[test]
    fn actor_message_contains_authority_and_revision_but_no_secret_input() {
        let request = AxisPairingRequest::new(
            RuntimePairingSessionId::trusted("axis-pairing-1"),
            AgentId::trusted("operator"),
            Revision::new("runtime-r1").unwrap(),
            2_000,
        );
        let message = request.clone().into_message("scheduler").unwrap();
        assert_eq!(AxisPairingRequest::from_message(&message).unwrap(), request);
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
            .bridge(&BridgeId::trusted("axis-camera-front"))
            .unwrap()
            .clone();
        let mut input = OwnerOnlyAxisCredentialInput::new(
            bridge.bridge_id.clone(),
            &username_path,
            11,
            &password_path,
            15,
        );
        let mut other_bridge = bridge.clone();
        other_bridge.bridge_id = BridgeId::trusted("axis-camera-other");
        assert!(matches!(
            input.take_for_bridge(&other_bridge).unwrap_err(),
            AxisPairingServiceError::SecretInput("credential input is bound to another bridge")
        ));
        let secret = input.take_for_bridge(&bridge).unwrap();
        assert_eq!(secret.username(), "camera-user");
        assert_eq!(secret.password(), "camera-password");
        assert!(matches!(
            input.take_for_bridge(&bridge).unwrap_err(),
            AxisPairingServiceError::SecretInput("credential input was already consumed")
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
        let new_key = format!("axis-camera-front/{transaction_id}");
        let new_ref = VaultRef::trusted(format!("{AXIS_VAULT_REF_PREFIX}{new_key}"));
        let request = PairingTransactionRequest::new(
            transaction_id,
            AgentId::trusted("operator"),
            BridgeId::trusted("axis-camera-front"),
            RuntimePairingSessionId::trusted("axis-pairing-1"),
            PairingCredentialLocation::new(new_ref, AXIS_VAULT_NAMESPACE, new_key).unwrap(),
            2_000,
            runtime_revision,
        )
        .unwrap()
        .with_metadata(vec![Metadata::new("axis.pairing.verified", "true")]);
        match previous {
            Some(previous) => {
                request.with_previous_credential(axis_credential_location(&previous).unwrap())
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
                    transaction_request("axis-restart", revision, None),
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
                .pairing_session(&RuntimePairingSessionId::trusted("axis-pairing-1"))
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
        let old_ref =
            VaultRef::trusted(format!("{AXIS_VAULT_REF_PREFIX}axis-camera-front/previous"));
        let old_key = vault_record_key(&old_ref).unwrap().to_string();
        vault
            .put(AXIS_VAULT_NAMESPACE, &old_key, b"previous-secret", None)
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
        assert!(vault.get(AXIS_VAULT_NAMESPACE, &old_key).unwrap().is_none());
        assert!(vault
            .get(
                AXIS_VAULT_NAMESPACE,
                vault_record_key(&report.vault_ref).unwrap(),
            )
            .unwrap()
            .is_some());
        assert_eq!(
            service
                .runtime()
                .registry()
                .bridge(&BridgeId::trusted("axis-camera-front"))
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
        let old_ref =
            VaultRef::trusted(format!("{AXIS_VAULT_REF_PREFIX}axis-camera-front/previous"));
        let old_key = vault_record_key(&old_ref).unwrap().to_string();
        let old_revision = vault
            .put(AXIS_VAULT_NAMESPACE, &old_key, b"previous-secret", None)
            .unwrap();
        let revision = persist_runtime(&root, &runtime_for_bridge(true, Some(old_ref.clone())));
        let runtime_store =
            SmartHomeRuntimeStore::new(LocalFolderStorageBackend::new(runtime_root(&root)));
        let failing_journal = FailOnPutBackend::new(journal_root(&root), 3);
        assert!(
            PairingTransactionCoordinator::new(&failing_journal, &vault, &runtime_store)
                .execute(
                    transaction_request("axis-cleanup", revision, Some(old_ref)),
                    br#"{"schema_version":1,"username":"u","password":"p"}"#,
                )
                .is_err()
        );
        vault
            .put(
                AXIS_VAULT_NAMESPACE,
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
            Err(AxisPairingServiceError::Transaction(_))
        ));
        assert_eq!(
            vault
                .get(AXIS_VAULT_NAMESPACE, &old_key)
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
