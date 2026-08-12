//! Recoverable coordination for sealed-Vault pairing credentials and runtime state.

#![forbid(unsafe_code)]

use coding_adventures_vault_sealed_store::{SealedStore, SealedStoreError};
use serde::{Deserialize, Serialize};
use smart_home_controller_runtime::SmartHomeControllerRuntime;
use smart_home_core::{AgentId, BridgeId, Metadata, VaultRef};
use smart_home_runtime::{
    PairingSessionStatus, RuntimeCompletePairingToolOutput, RuntimeCompletePairingToolRequest,
    RuntimeError, RuntimePairingSessionId,
};
use smart_home_runtime_store::{
    RestoredSmartHomeRuntime, RuntimeStoreError, SmartHomeRuntimeStore,
};
use std::fmt;
use storage_core::{
    Revision, StorageBackend, StorageError, StorageListOptions, StorageMetadata, StoragePutInput,
};

const SCHEMA_VERSION: u32 = 1;
const DEFAULT_JOURNAL_NAMESPACE: &str = "smart-home-pairing-transactions";
const JOURNAL_CONTENT_TYPE: &str =
    "application/vnd.coding-adventures.smart-home-pairing-transaction+json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairingCredentialLocation {
    pub vault_ref: VaultRef,
    pub namespace: String,
    pub key: String,
}

impl PairingCredentialLocation {
    pub fn new(
        vault_ref: VaultRef,
        namespace: impl Into<String>,
        key: impl Into<String>,
    ) -> Result<Self, PairingTransactionError> {
        let namespace = namespace.into();
        let key = key.into();
        if namespace.trim().is_empty() {
            return Err(PairingTransactionError::Validation(
                "credential namespace must not be empty".to_string(),
            ));
        }
        if key.trim().is_empty() {
            return Err(PairingTransactionError::Validation(
                "credential key must not be empty".to_string(),
            ));
        }
        Ok(Self {
            vault_ref,
            namespace,
            key,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairingTransactionRequest {
    pub transaction_id: String,
    pub principal_id: AgentId,
    pub bridge_id: BridgeId,
    pub session_id: RuntimePairingSessionId,
    pub new_credential: PairingCredentialLocation,
    pub previous_credential: Option<PairingCredentialLocation>,
    pub completed_at_ms: u64,
    pub metadata: Vec<Metadata>,
    pub expected_runtime_revision: Revision,
}

impl PairingTransactionRequest {
    pub fn new(
        transaction_id: impl Into<String>,
        principal_id: AgentId,
        bridge_id: BridgeId,
        session_id: RuntimePairingSessionId,
        new_credential: PairingCredentialLocation,
        completed_at_ms: u64,
        expected_runtime_revision: Revision,
    ) -> Result<Self, PairingTransactionError> {
        let transaction_id = transaction_id.into();
        if transaction_id.trim().is_empty() {
            return Err(PairingTransactionError::Validation(
                "transaction id must not be empty".to_string(),
            ));
        }
        if !new_credential.key.contains(&transaction_id)
            || !new_credential.vault_ref.as_str().contains(&transaction_id)
        {
            return Err(PairingTransactionError::Validation(
                "new credential key and opaque reference must contain the transaction id"
                    .to_string(),
            ));
        }
        Ok(Self {
            transaction_id,
            principal_id,
            bridge_id,
            session_id,
            new_credential,
            previous_credential: None,
            completed_at_ms,
            metadata: Vec::new(),
            expected_runtime_revision,
        })
    }

    pub fn with_previous_credential(mut self, location: PairingCredentialLocation) -> Self {
        self.previous_credential = Some(location);
        self
    }

    pub fn with_metadata(mut self, metadata: Vec<Metadata>) -> Self {
        self.metadata = metadata;
        self
    }
}

#[derive(Debug)]
pub enum PairingTransactionOutcome {
    Committed {
        restored: Box<RestoredSmartHomeRuntime>,
        previous_vault_ref: Option<VaultRef>,
    },
    RolledBack {
        transaction_id: String,
    },
}

#[derive(Debug)]
pub enum PairingTransactionError {
    Validation(String),
    UnknownTransaction(String),
    Journal(StorageError),
    Vault(SealedStoreError),
    Runtime(RuntimeError),
    RuntimeStore(RuntimeStoreError),
    RuntimeAuthority(String),
    Invariant(String),
}

impl fmt::Display for PairingTransactionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => {
                write!(formatter, "invalid pairing transaction: {message}")
            }
            Self::UnknownTransaction(id) => write!(formatter, "unknown pairing transaction {id}"),
            Self::Journal(error) => write!(formatter, "pairing journal failure: {error}"),
            Self::Vault(error) => write!(formatter, "pairing Vault failure: {error}"),
            Self::Runtime(error) => write!(formatter, "pairing authorization failure: {error}"),
            Self::RuntimeStore(error) => {
                write!(formatter, "pairing runtime-store failure: {error}")
            }
            Self::RuntimeAuthority(message) => {
                write!(formatter, "pairing runtime-authority failure: {message}")
            }
            Self::Invariant(message) => {
                write!(formatter, "pairing transaction invariant failed: {message}")
            }
        }
    }
}

impl std::error::Error for PairingTransactionError {}

impl From<StorageError> for PairingTransactionError {
    fn from(error: StorageError) -> Self {
        Self::Journal(error)
    }
}

impl From<SealedStoreError> for PairingTransactionError {
    fn from(error: SealedStoreError) -> Self {
        Self::Vault(error)
    }
}

impl From<RuntimeError> for PairingTransactionError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

impl From<RuntimeStoreError> for PairingTransactionError {
    fn from(error: RuntimeStoreError) -> Self {
        Self::RuntimeStore(error)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum PairingTransactionStage {
    Prepared,
    VaultWritten,
    RuntimeCommitted,
    CleanupComplete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct VersionedCredentialLocation {
    location: PairingCredentialLocation,
    revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct PairingTransactionJournal {
    schema_version: u32,
    transaction_id: String,
    stage: PairingTransactionStage,
    principal_id: AgentId,
    bridge_id: BridgeId,
    session_id: RuntimePairingSessionId,
    new_credential: PairingCredentialLocation,
    new_credential_revision: Option<String>,
    previous_credential: Option<VersionedCredentialLocation>,
    completed_at_ms: u64,
    metadata: Vec<Metadata>,
    expected_runtime_revision: String,
    committed_runtime_revision: Option<String>,
}

struct LoadedJournal {
    journal: PairingTransactionJournal,
    revision: Revision,
}

pub trait PairingRuntimeAuthority {
    fn load_pairing_runtime(
        &self,
    ) -> Result<Option<RestoredSmartHomeRuntime>, PairingTransactionError>;

    fn complete_pairing(
        &self,
        principal_id: AgentId,
        request: RuntimeCompletePairingToolRequest,
        expected_revision: Revision,
    ) -> Result<
        (RuntimeCompletePairingToolOutput, Option<VaultRef>, Revision),
        PairingTransactionError,
    >;
}

impl<R: StorageBackend> PairingRuntimeAuthority for SmartHomeRuntimeStore<R> {
    fn load_pairing_runtime(
        &self,
    ) -> Result<Option<RestoredSmartHomeRuntime>, PairingTransactionError> {
        self.load().map_err(PairingTransactionError::RuntimeStore)
    }

    fn complete_pairing(
        &self,
        principal_id: AgentId,
        request: RuntimeCompletePairingToolRequest,
        expected_revision: Revision,
    ) -> Result<
        (RuntimeCompletePairingToolOutput, Option<VaultRef>, Revision),
        PairingTransactionError,
    > {
        let mut restored = self.load()?.ok_or_else(|| {
            PairingTransactionError::Invariant("runtime store is empty".to_string())
        })?;
        let definitions = restored.automation_definitions;
        let automation_state = restored.automation_state;
        SmartHomeRuntimeStore::complete_pairing(
            self,
            &mut restored.runtime,
            principal_id,
            request,
            &definitions,
            automation_state,
            expected_revision,
        )
        .map_err(PairingTransactionError::RuntimeStore)
    }
}

impl<B: StorageBackend> PairingRuntimeAuthority for SmartHomeControllerRuntime<B> {
    fn load_pairing_runtime(
        &self,
    ) -> Result<Option<RestoredSmartHomeRuntime>, PairingTransactionError> {
        self.durable_snapshot()
            .map_err(|error| PairingTransactionError::RuntimeAuthority(error.to_string()))
    }

    fn complete_pairing(
        &self,
        principal_id: AgentId,
        request: RuntimeCompletePairingToolRequest,
        expected_revision: Revision,
    ) -> Result<
        (RuntimeCompletePairingToolOutput, Option<VaultRef>, Revision),
        PairingTransactionError,
    > {
        let commit = SmartHomeControllerRuntime::complete_pairing(
            self,
            principal_id,
            request,
            expected_revision,
        )
        .map_err(|error| PairingTransactionError::RuntimeAuthority(error.to_string()))?;
        Ok((commit.value.0, commit.value.1, commit.revision))
    }
}

pub struct PairingTransactionCoordinator<'a, J: StorageBackend, A: PairingRuntimeAuthority> {
    journal_backend: &'a J,
    journal_namespace: String,
    vault: &'a SealedStore,
    runtime_authority: &'a A,
}

impl<'a, J: StorageBackend, A: PairingRuntimeAuthority> PairingTransactionCoordinator<'a, J, A> {
    pub fn new(journal_backend: &'a J, vault: &'a SealedStore, runtime_authority: &'a A) -> Self {
        Self {
            journal_backend,
            journal_namespace: DEFAULT_JOURNAL_NAMESPACE.to_string(),
            vault,
            runtime_authority,
        }
    }

    pub fn with_journal_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.journal_namespace = namespace.into();
        self
    }

    /// Lists every pending transaction in stable key order so a restart host
    /// can call `recover` for each entry.
    pub fn pending_transaction_ids(&self) -> Result<Vec<String>, PairingTransactionError> {
        self.journal_backend.initialize()?;
        let mut transaction_ids = Vec::new();
        let mut cursor = None;
        loop {
            let page = self.journal_backend.list(
                &self.journal_namespace,
                StorageListOptions {
                    prefix: None,
                    recursive: true,
                    page_size: Some(100),
                    cursor,
                },
            )?;
            transaction_ids.extend(page.records.into_iter().map(|record| record.key));
            let Some(next_cursor) = page.next_cursor else {
                break;
            };
            cursor = Some(next_cursor);
        }
        Ok(transaction_ids)
    }

    /// Executes a fresh transaction. Authorization is proven against a clone
    /// before the journal or Vault is written.
    pub fn execute(
        &self,
        request: PairingTransactionRequest,
        credential: &[u8],
    ) -> Result<PairingTransactionOutcome, PairingTransactionError> {
        let loaded = self.preflight(&request)?;
        let journal_revision = self.prepare(&request, &loaded)?;
        match self.vault.put_if_absent(
            &request.new_credential.namespace,
            &request.new_credential.key,
            credential,
        ) {
            Ok(vault_revision) => {
                self.record_vault_write(&request.transaction_id, journal_revision, vault_revision)?;
            }
            Err(error) => {
                self.journal_backend.delete(
                    &self.journal_namespace,
                    &request.transaction_id,
                    Some(&journal_revision),
                )?;
                return Err(error.into());
            }
        }
        self.recover(&request.transaction_id)
    }

    /// Resumes one journal after a process restart. The method is idempotent;
    /// it either reaches durable runtime completion and exact old-record
    /// cleanup, or rolls the uncommitted new record back at its exact revision.
    pub fn recover(
        &self,
        transaction_id: &str,
    ) -> Result<PairingTransactionOutcome, PairingTransactionError> {
        for _ in 0..8 {
            let loaded = self.load_journal(transaction_id)?.ok_or_else(|| {
                PairingTransactionError::UnknownTransaction(transaction_id.into())
            })?;
            match loaded.journal.stage {
                PairingTransactionStage::Prepared => {
                    let Some(summary) = self.vault.summarize(
                        &loaded.journal.new_credential.namespace,
                        &loaded.journal.new_credential.key,
                    )?
                    else {
                        self.delete_journal(transaction_id, &loaded.revision)?;
                        return Ok(PairingTransactionOutcome::RolledBack {
                            transaction_id: transaction_id.to_string(),
                        });
                    };
                    self.record_vault_write(transaction_id, loaded.revision, summary.revision)?;
                }
                PairingTransactionStage::VaultWritten => {
                    if self.advance_runtime_or_rollback(loaded)? {
                        continue;
                    }
                    return Ok(PairingTransactionOutcome::RolledBack {
                        transaction_id: transaction_id.to_string(),
                    });
                }
                PairingTransactionStage::RuntimeCommitted => {
                    if let Some(previous) = &loaded.journal.previous_credential {
                        self.vault.delete(
                            &previous.location.namespace,
                            &previous.location.key,
                            Some(parse_revision(&previous.revision)?),
                        )?;
                    }
                    let mut journal = loaded.journal;
                    journal.stage = PairingTransactionStage::CleanupComplete;
                    self.update_journal(journal, loaded.revision)?;
                }
                PairingTransactionStage::CleanupComplete => {
                    let previous_vault_ref = loaded
                        .journal
                        .previous_credential
                        .as_ref()
                        .map(|previous| previous.location.vault_ref.clone());
                    self.delete_journal(transaction_id, &loaded.revision)?;
                    let restored =
                        self.runtime_authority
                            .load_pairing_runtime()?
                            .ok_or_else(|| {
                                PairingTransactionError::Invariant(
                                    "runtime disappeared after pairing commit".to_string(),
                                )
                            })?;
                    return Ok(PairingTransactionOutcome::Committed {
                        restored: Box::new(restored),
                        previous_vault_ref,
                    });
                }
            }
        }
        Err(PairingTransactionError::Invariant(
            "recovery exceeded the bounded state-transition count".to_string(),
        ))
    }

    fn preflight(
        &self,
        request: &PairingTransactionRequest,
    ) -> Result<RestoredSmartHomeRuntime, PairingTransactionError> {
        self.journal_backend.initialize()?;
        if self.load_journal(&request.transaction_id)?.is_some() {
            return Err(PairingTransactionError::Validation(
                "transaction id is already in use".to_string(),
            ));
        }
        let loaded = self
            .runtime_authority
            .load_pairing_runtime()?
            .ok_or_else(|| {
                PairingTransactionError::Invariant("runtime store is empty".to_string())
            })?;
        if loaded.revision != request.expected_runtime_revision {
            return Err(PairingTransactionError::Validation(
                "expected runtime revision is stale".to_string(),
            ));
        }
        let session = loaded
            .runtime
            .pairing_session(&request.session_id)
            .ok_or_else(|| {
                PairingTransactionError::Validation("pairing session is absent".to_string())
            })?;
        if session.bridge_id != request.bridge_id {
            return Err(PairingTransactionError::Validation(
                "pairing session does not target the requested bridge".to_string(),
            ));
        }
        let previous_ref = loaded
            .runtime
            .registry()
            .bridge(&request.bridge_id)
            .and_then(|bridge| bridge.auth_ref.as_ref());
        if previous_ref
            != request
                .previous_credential
                .as_ref()
                .map(|location| &location.vault_ref)
        {
            return Err(PairingTransactionError::Validation(
                "previous credential location does not match runtime state".to_string(),
            ));
        }

        let mut candidate = loaded.runtime.clone();
        candidate.execute_complete_pairing_tool(
            request.principal_id.clone(),
            completion_request(request),
            request.completed_at_ms,
        )?;
        if self
            .vault
            .summarize(
                &request.new_credential.namespace,
                &request.new_credential.key,
            )?
            .is_some()
        {
            return Err(PairingTransactionError::Validation(
                "new credential location already exists".to_string(),
            ));
        }
        Ok(loaded)
    }

    fn prepare(
        &self,
        request: &PairingTransactionRequest,
        loaded: &RestoredSmartHomeRuntime,
    ) -> Result<Revision, PairingTransactionError> {
        let previous_credential = match &request.previous_credential {
            Some(location) => {
                let summary = self
                    .vault
                    .summarize(&location.namespace, &location.key)?
                    .ok_or_else(|| {
                        PairingTransactionError::Validation(
                            "previous credential record is absent".to_string(),
                        )
                    })?;
                Some(VersionedCredentialLocation {
                    location: location.clone(),
                    revision: summary.revision.as_str().to_string(),
                })
            }
            None => None,
        };
        if loaded.revision != request.expected_runtime_revision {
            return Err(PairingTransactionError::Validation(
                "runtime revision changed during preparation".to_string(),
            ));
        }
        let journal = PairingTransactionJournal {
            schema_version: SCHEMA_VERSION,
            transaction_id: request.transaction_id.clone(),
            stage: PairingTransactionStage::Prepared,
            principal_id: request.principal_id.clone(),
            bridge_id: request.bridge_id.clone(),
            session_id: request.session_id.clone(),
            new_credential: request.new_credential.clone(),
            new_credential_revision: None,
            previous_credential,
            completed_at_ms: request.completed_at_ms,
            metadata: request.metadata.clone(),
            expected_runtime_revision: request.expected_runtime_revision.as_str().to_string(),
            committed_runtime_revision: None,
        };
        self.put_journal(journal, None, true)
    }

    fn record_vault_write(
        &self,
        transaction_id: &str,
        journal_revision: Revision,
        vault_revision: Revision,
    ) -> Result<Revision, PairingTransactionError> {
        let mut loaded = self
            .load_journal(transaction_id)?
            .ok_or_else(|| PairingTransactionError::UnknownTransaction(transaction_id.into()))?;
        if loaded.revision != journal_revision {
            return Err(PairingTransactionError::Validation(
                "journal revision changed before Vault acknowledgement".to_string(),
            ));
        }
        loaded.journal.stage = PairingTransactionStage::VaultWritten;
        loaded.journal.new_credential_revision = Some(vault_revision.as_str().to_string());
        self.update_journal(loaded.journal, loaded.revision)
    }

    fn advance_runtime_or_rollback(
        &self,
        loaded: LoadedJournal,
    ) -> Result<bool, PairingTransactionError> {
        let restored = self
            .runtime_authority
            .load_pairing_runtime()?
            .ok_or_else(|| {
                PairingTransactionError::Invariant("runtime store is empty".to_string())
            })?;
        match runtime_credential_state(&restored, &loaded.journal) {
            RuntimeCredentialState::Committed => {
                let mut journal = loaded.journal;
                journal.stage = PairingTransactionStage::RuntimeCommitted;
                journal.committed_runtime_revision = Some(restored.revision.as_str().to_string());
                self.update_journal(journal, loaded.revision)?;
                return Ok(true);
            }
            RuntimeCredentialState::Diverged => {
                return Err(PairingTransactionError::Invariant(
                    "runtime only partially references the new credential".to_string(),
                ));
            }
            RuntimeCredentialState::Absent => {}
        }
        if restored.revision.as_str() != loaded.journal.expected_runtime_revision {
            self.rollback_new_credential(&loaded)?;
            return Ok(false);
        }

        let expected_revision = restored.revision;
        let completion = RuntimeCompletePairingToolRequest::new(
            loaded.journal.session_id.clone(),
            loaded.journal.new_credential.vault_ref.clone(),
            loaded.journal.completed_at_ms,
        )
        .with_metadata(loaded.journal.metadata.clone());
        let (_, previous_vault_ref, committed_revision) = self.runtime_authority.complete_pairing(
            loaded.journal.principal_id.clone(),
            completion,
            expected_revision,
        )?;
        let expected_previous = loaded
            .journal
            .previous_credential
            .as_ref()
            .map(|previous| previous.location.vault_ref.clone());
        if previous_vault_ref != expected_previous {
            return Err(PairingTransactionError::Invariant(
                "runtime committed a different previous credential reference".to_string(),
            ));
        }
        let mut journal = loaded.journal;
        journal.stage = PairingTransactionStage::RuntimeCommitted;
        journal.committed_runtime_revision = Some(committed_revision.as_str().to_string());
        self.update_journal(journal, loaded.revision)?;
        Ok(true)
    }

    fn rollback_new_credential(
        &self,
        loaded: &LoadedJournal,
    ) -> Result<(), PairingTransactionError> {
        let revision = loaded
            .journal
            .new_credential_revision
            .as_deref()
            .ok_or_else(|| {
                PairingTransactionError::Invariant(
                    "Vault-written journal has no credential revision".to_string(),
                )
            })?;
        self.vault.delete(
            &loaded.journal.new_credential.namespace,
            &loaded.journal.new_credential.key,
            Some(parse_revision(revision)?),
        )?;
        self.delete_journal(&loaded.journal.transaction_id, &loaded.revision)
    }

    fn load_journal(
        &self,
        transaction_id: &str,
    ) -> Result<Option<LoadedJournal>, PairingTransactionError> {
        self.journal_backend.initialize()?;
        let Some(record) = self
            .journal_backend
            .get(&self.journal_namespace, transaction_id)?
        else {
            return Ok(None);
        };
        let journal: PairingTransactionJournal = serde_json::from_slice(&record.body)
            .map_err(|error| PairingTransactionError::Validation(error.to_string()))?;
        if journal.schema_version != SCHEMA_VERSION || journal.transaction_id != transaction_id {
            return Err(PairingTransactionError::Validation(
                "journal schema or identity mismatch".to_string(),
            ));
        }
        Ok(Some(LoadedJournal {
            journal,
            revision: record.revision,
        }))
    }

    fn put_journal(
        &self,
        journal: PairingTransactionJournal,
        expected_revision: Option<Revision>,
        if_absent: bool,
    ) -> Result<Revision, PairingTransactionError> {
        let body = serde_json::to_vec(&journal)
            .map_err(|error| PairingTransactionError::Validation(error.to_string()))?;
        let mut input = StoragePutInput::new(
            self.journal_namespace.clone(),
            journal.transaction_id,
            JOURNAL_CONTENT_TYPE,
            StorageMetadata::Object(Default::default()),
            body,
        )?;
        input = if if_absent {
            input.with_if_absent()
        } else {
            input.with_if_revision(expected_revision)
        };
        Ok(self.journal_backend.put(input)?.revision)
    }

    fn update_journal(
        &self,
        journal: PairingTransactionJournal,
        expected_revision: Revision,
    ) -> Result<Revision, PairingTransactionError> {
        self.put_journal(journal, Some(expected_revision), false)
    }

    fn delete_journal(
        &self,
        transaction_id: &str,
        revision: &Revision,
    ) -> Result<(), PairingTransactionError> {
        self.journal_backend
            .delete(&self.journal_namespace, transaction_id, Some(revision))?;
        Ok(())
    }
}

fn completion_request(request: &PairingTransactionRequest) -> RuntimeCompletePairingToolRequest {
    RuntimeCompletePairingToolRequest::new(
        request.session_id.clone(),
        request.new_credential.vault_ref.clone(),
        request.completed_at_ms,
    )
    .with_metadata(request.metadata.clone())
}

fn parse_revision(value: &str) -> Result<Revision, PairingTransactionError> {
    Revision::new(value.to_string()).map_err(PairingTransactionError::Journal)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuntimeCredentialState {
    Absent,
    Committed,
    Diverged,
}

fn runtime_credential_state(
    restored: &RestoredSmartHomeRuntime,
    journal: &PairingTransactionJournal,
) -> RuntimeCredentialState {
    let session = restored
        .runtime
        .pairing_session(&journal.session_id)
        .filter(|session| session.vault_ref.as_ref() == Some(&journal.new_credential.vault_ref));
    let bridge_matches = restored
        .runtime
        .registry()
        .bridge(&journal.bridge_id)
        .is_some_and(|bridge| bridge.auth_ref.as_ref() == Some(&journal.new_credential.vault_ref));
    match (session, bridge_matches) {
        (Some(session), true) if session.status == PairingSessionStatus::Completed => {
            RuntimeCredentialState::Committed
        }
        (None, false) => RuntimeCredentialState::Absent,
        _ => RuntimeCredentialState::Diverged,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_vault_sealed_store::InitOptions;
    use smart_home_core::{
        Bridge, BridgeTransport, CapabilityGrant, CapabilityGrantId, CapabilityId, IntegrationId,
        PrivilegeTier,
    };
    use smart_home_runtime::{RuntimePairingSession, SmartHomeRuntime};
    use std::sync::Arc;
    use storage_core::InMemoryStorageBackend;

    const VAULT_NAMESPACE: &str = "smart_home.test.credentials";

    struct Fixture {
        journal_backend: InMemoryStorageBackend,
        vault: SealedStore,
        runtime_store: SmartHomeRuntimeStore<InMemoryStorageBackend>,
        runtime_revision: Revision,
        principal_id: AgentId,
    }

    fn fixture(with_previous: bool, authorized: bool) -> Fixture {
        let vault_backend: Arc<dyn StorageBackend> = Arc::new(InMemoryStorageBackend::new());
        let vault = SealedStore::new(vault_backend);
        vault
            .init(
                b"test-password",
                &InitOptions {
                    argon2id_time_cost: 1,
                    argon2id_memory_kib: 32,
                    argon2id_parallelism: 4,
                    salt_override: Some(vec![0x42; 16]),
                },
            )
            .unwrap();
        let principal_id = AgentId::trusted("agent:installer");
        let mut runtime = SmartHomeRuntime::new();
        let mut bridge = Bridge::new(
            BridgeId::trusted("bridge-1"),
            IntegrationId::trusted("hue"),
            BridgeTransport::LanHttp,
        );
        if with_previous {
            let old_ref = VaultRef::trusted("vault://smart-home/test/bridge-1/old");
            vault
                .put(VAULT_NAMESPACE, "bridge-1/old", b"old-secret", None)
                .unwrap();
            bridge.auth_ref = Some(old_ref);
        }
        runtime.upsert_bridge(bridge.clone()).unwrap();
        runtime
            .start_pairing_session(RuntimePairingSession::pending(
                RuntimePairingSessionId::trusted("pairing-1"),
                &bridge,
                principal_id.clone(),
                100,
                1_000,
                Vec::new(),
            ))
            .unwrap();
        if authorized {
            runtime.registry_mut().upsert_capability_grant(
                CapabilityGrant::for_capability(
                    CapabilityGrantId::trusted("grant-pair"),
                    principal_id.clone(),
                    CapabilityId::trusted("smart_home.pair"),
                    PrivilegeTier::HumanApproval,
                    "test",
                    150,
                )
                .with_expiry(900),
            );
        }
        let runtime_store = SmartHomeRuntimeStore::new(InMemoryStorageBackend::new());
        let runtime_revision = runtime_store.save(&runtime, &[], 200).unwrap();
        Fixture {
            journal_backend: InMemoryStorageBackend::new(),
            vault,
            runtime_store,
            runtime_revision,
            principal_id,
        }
    }

    fn request(fixture: &Fixture, with_previous: bool) -> PairingTransactionRequest {
        let new_location = PairingCredentialLocation::new(
            VaultRef::trusted("vault://smart-home/test/bridge-1/transaction-1"),
            VAULT_NAMESPACE,
            "bridge-1/transaction-1",
        )
        .unwrap();
        let request = PairingTransactionRequest::new(
            "transaction-1",
            fixture.principal_id.clone(),
            BridgeId::trusted("bridge-1"),
            RuntimePairingSessionId::trusted("pairing-1"),
            new_location,
            300,
            fixture.runtime_revision.clone(),
        )
        .unwrap()
        .with_metadata(vec![Metadata::new("integration", "hue")]);
        if with_previous {
            request.with_previous_credential(
                PairingCredentialLocation::new(
                    VaultRef::trusted("vault://smart-home/test/bridge-1/old"),
                    VAULT_NAMESPACE,
                    "bridge-1/old",
                )
                .unwrap(),
            )
        } else {
            request
        }
    }

    #[test]
    fn execute_commits_runtime_cleans_old_credential_and_removes_journal() {
        let fixture = fixture(true, true);
        let coordinator = PairingTransactionCoordinator::new(
            &fixture.journal_backend,
            &fixture.vault,
            &fixture.runtime_store,
        );

        let outcome = coordinator
            .execute(request(&fixture, true), b"new-secret")
            .unwrap();

        let PairingTransactionOutcome::Committed {
            restored,
            previous_vault_ref,
        } = outcome
        else {
            panic!("transaction should commit");
        };
        assert_eq!(
            previous_vault_ref,
            Some(VaultRef::trusted("vault://smart-home/test/bridge-1/old"))
        );
        assert_eq!(
            restored
                .runtime
                .registry()
                .bridge(&BridgeId::trusted("bridge-1"))
                .unwrap()
                .auth_ref,
            Some(VaultRef::trusted(
                "vault://smart-home/test/bridge-1/transaction-1"
            ))
        );
        assert!(fixture
            .vault
            .get(VAULT_NAMESPACE, "bridge-1/old")
            .unwrap()
            .is_none());
        assert_eq!(
            &*fixture
                .vault
                .get(VAULT_NAMESPACE, "bridge-1/transaction-1")
                .unwrap()
                .unwrap()
                .plaintext,
            b"new-secret"
        );
        assert!(fixture
            .journal_backend
            .get(DEFAULT_JOURNAL_NAMESPACE, "transaction-1")
            .unwrap()
            .is_none());
    }

    #[test]
    fn prepared_journal_is_secret_free_and_recovers_an_unacknowledged_vault_write() {
        let fixture = fixture(false, true);
        let coordinator = PairingTransactionCoordinator::new(
            &fixture.journal_backend,
            &fixture.vault,
            &fixture.runtime_store,
        );
        let request = request(&fixture, false);
        let loaded = coordinator.preflight(&request).unwrap();
        coordinator.prepare(&request, &loaded).unwrap();
        let journal = fixture
            .journal_backend
            .get(DEFAULT_JOURNAL_NAMESPACE, "transaction-1")
            .unwrap()
            .unwrap();
        assert!(!journal
            .body
            .windows(b"credential-must-not-appear".len())
            .any(|window| window == b"credential-must-not-appear"));
        fixture
            .vault
            .put_if_absent(
                VAULT_NAMESPACE,
                "bridge-1/transaction-1",
                b"credential-must-not-appear",
            )
            .unwrap();

        let outcome = coordinator.recover("transaction-1").unwrap();

        assert!(matches!(
            outcome,
            PairingTransactionOutcome::Committed { .. }
        ));
        assert!(fixture
            .journal_backend
            .get(DEFAULT_JOURNAL_NAMESPACE, "transaction-1")
            .unwrap()
            .is_none());
    }

    #[test]
    fn pending_transactions_are_discoverable_after_restart() {
        let fixture = fixture(false, true);
        let coordinator = PairingTransactionCoordinator::new(
            &fixture.journal_backend,
            &fixture.vault,
            &fixture.runtime_store,
        );
        let request = request(&fixture, false);
        let loaded = coordinator.preflight(&request).unwrap();
        coordinator.prepare(&request, &loaded).unwrap();

        assert_eq!(
            coordinator.pending_transaction_ids().unwrap(),
            vec!["transaction-1".to_string()]
        );
    }

    #[test]
    fn stale_runtime_revision_rolls_back_new_credential_at_exact_revision() {
        let fixture = fixture(false, true);
        let coordinator = PairingTransactionCoordinator::new(
            &fixture.journal_backend,
            &fixture.vault,
            &fixture.runtime_store,
        );
        let request = request(&fixture, false);
        let loaded = coordinator.preflight(&request).unwrap();
        let journal_revision = coordinator.prepare(&request, &loaded).unwrap();
        let vault_revision = fixture
            .vault
            .put_if_absent(VAULT_NAMESPACE, "bridge-1/transaction-1", b"new-secret")
            .unwrap();
        coordinator
            .record_vault_write("transaction-1", journal_revision, vault_revision)
            .unwrap();
        fixture
            .runtime_store
            .save(&loaded.runtime, &[], 250)
            .unwrap();

        let outcome = coordinator.recover("transaction-1").unwrap();

        assert!(matches!(
            outcome,
            PairingTransactionOutcome::RolledBack { .. }
        ));
        assert!(fixture
            .vault
            .get(VAULT_NAMESPACE, "bridge-1/transaction-1")
            .unwrap()
            .is_none());
    }

    #[test]
    fn recovery_detects_runtime_commit_before_journal_acknowledgement() {
        let fixture = fixture(true, true);
        let coordinator = PairingTransactionCoordinator::new(
            &fixture.journal_backend,
            &fixture.vault,
            &fixture.runtime_store,
        );
        let request = request(&fixture, true);
        let loaded = coordinator.preflight(&request).unwrap();
        let journal_revision = coordinator.prepare(&request, &loaded).unwrap();
        let vault_revision = fixture
            .vault
            .put_if_absent(VAULT_NAMESPACE, "bridge-1/transaction-1", b"new-secret")
            .unwrap();
        coordinator
            .record_vault_write("transaction-1", journal_revision, vault_revision)
            .unwrap();
        let mut runtime = loaded.runtime;
        fixture
            .runtime_store
            .complete_pairing(
                &mut runtime,
                request.principal_id.clone(),
                completion_request(&request),
                &loaded.automation_definitions,
                loaded.automation_state,
                loaded.revision,
            )
            .unwrap();

        let outcome = coordinator.recover("transaction-1").unwrap();

        assert!(matches!(
            outcome,
            PairingTransactionOutcome::Committed { .. }
        ));
        assert!(fixture
            .vault
            .get(VAULT_NAMESPACE, "bridge-1/old")
            .unwrap()
            .is_none());
    }

    #[test]
    fn cleanup_never_deletes_a_replaced_old_record_at_the_wrong_revision() {
        let fixture = fixture(true, true);
        let coordinator = PairingTransactionCoordinator::new(
            &fixture.journal_backend,
            &fixture.vault,
            &fixture.runtime_store,
        );
        let request = request(&fixture, true);
        let loaded = coordinator.preflight(&request).unwrap();
        let journal_revision = coordinator.prepare(&request, &loaded).unwrap();
        let vault_revision = fixture
            .vault
            .put_if_absent(VAULT_NAMESPACE, "bridge-1/transaction-1", b"new-secret")
            .unwrap();
        coordinator
            .record_vault_write("transaction-1", journal_revision, vault_revision)
            .unwrap();
        let mut runtime = loaded.runtime;
        fixture
            .runtime_store
            .complete_pairing(
                &mut runtime,
                request.principal_id.clone(),
                completion_request(&request),
                &loaded.automation_definitions,
                loaded.automation_state,
                loaded.revision,
            )
            .unwrap();
        fixture
            .vault
            .put(VAULT_NAMESPACE, "bridge-1/old", b"replacement-old", None)
            .unwrap();

        let error = coordinator.recover("transaction-1").unwrap_err();

        assert!(matches!(
            error,
            PairingTransactionError::Vault(SealedStoreError::Storage(
                StorageError::Conflict { .. }
            ))
        ));
        assert_eq!(
            &*fixture
                .vault
                .get(VAULT_NAMESPACE, "bridge-1/old")
                .unwrap()
                .unwrap()
                .plaintext,
            b"replacement-old"
        );
        assert_eq!(
            coordinator.pending_transaction_ids().unwrap(),
            vec!["transaction-1".to_string()]
        );
    }

    #[test]
    fn partial_runtime_reference_retains_the_new_credential_and_journal() {
        let fixture = fixture(false, true);
        let coordinator = PairingTransactionCoordinator::new(
            &fixture.journal_backend,
            &fixture.vault,
            &fixture.runtime_store,
        );
        let request = request(&fixture, false);
        let mut loaded = coordinator.preflight(&request).unwrap();
        let journal_revision = coordinator.prepare(&request, &loaded).unwrap();
        let vault_revision = fixture
            .vault
            .put_if_absent(VAULT_NAMESPACE, "bridge-1/transaction-1", b"new-secret")
            .unwrap();
        coordinator
            .record_vault_write("transaction-1", journal_revision, vault_revision)
            .unwrap();
        let mut bridge = loaded
            .runtime
            .registry()
            .bridge(&BridgeId::trusted("bridge-1"))
            .unwrap()
            .clone();
        bridge.auth_ref = Some(request.new_credential.vault_ref.clone());
        loaded.runtime.upsert_bridge(bridge).unwrap();
        fixture
            .runtime_store
            .save(&loaded.runtime, &[], 250)
            .unwrap();

        let error = coordinator.recover("transaction-1").unwrap_err();

        assert!(matches!(error, PairingTransactionError::Invariant(_)));
        assert!(fixture
            .vault
            .get(VAULT_NAMESPACE, "bridge-1/transaction-1")
            .unwrap()
            .is_some());
        assert_eq!(
            coordinator.pending_transaction_ids().unwrap(),
            vec!["transaction-1".to_string()]
        );
    }

    #[test]
    fn authorization_denial_happens_before_journal_or_vault_write() {
        let fixture = fixture(false, false);
        let coordinator = PairingTransactionCoordinator::new(
            &fixture.journal_backend,
            &fixture.vault,
            &fixture.runtime_store,
        );

        let error = coordinator
            .execute(request(&fixture, false), b"must-not-be-written")
            .unwrap_err();

        assert!(matches!(error, PairingTransactionError::Runtime(_)));
        assert!(fixture
            .vault
            .get(VAULT_NAMESPACE, "bridge-1/transaction-1")
            .unwrap()
            .is_none());
        assert!(fixture
            .journal_backend
            .get(DEFAULT_JOURNAL_NAMESPACE, "transaction-1")
            .unwrap()
            .is_none());
    }
}
