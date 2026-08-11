//! Restart-safe persistence for the normalized smart-home runtime.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use smart_home_core::{AgentId, VaultRef};
use smart_home_runtime::{
    RuntimeCompletePairingToolOutput, RuntimeCompletePairingToolRequest, RuntimeDurableSnapshot,
    RuntimeError, RuntimeRetainedIdentityMigration, RuntimeRetainedIdentityMigrationReport,
    SmartHomeRuntime,
};
use std::fmt;
use storage_core::{Revision, StorageBackend, StorageError, StorageMetadata, StoragePutInput};

const SCHEMA_VERSION: u32 = 1;
const DEFAULT_NAMESPACE: &str = "smart-home-runtime";
const DEFAULT_KEY: &str = "runtime-state";
const CONTENT_TYPE: &str = "application/vnd.coding-adventures.smart-home-runtime+json";

/// A durable automation document kept alongside runtime state.
///
/// The rules engine owns the document schema. This store only requires a stable
/// identifier and JSON object so definitions survive restarts before and after
/// an engine implementation changes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DurableAutomationDefinition {
    pub automation_id: String,
    pub enabled: bool,
    pub definition: serde_json::Value,
}

impl DurableAutomationDefinition {
    pub fn new(
        automation_id: impl Into<String>,
        enabled: bool,
        definition: serde_json::Value,
    ) -> Result<Self, RuntimeStoreError> {
        let automation_id = automation_id.into();
        if automation_id.trim().is_empty() {
            return Err(RuntimeStoreError::Validation {
                field: "automation_id",
                message: "must not be empty".to_string(),
            });
        }
        if !definition.is_object() {
            return Err(RuntimeStoreError::Validation {
                field: "definition",
                message: "must be a JSON object".to_string(),
            });
        }
        Ok(Self {
            automation_id,
            enabled,
            definition,
        })
    }
}

/// State returned after loading one durable runtime snapshot.
#[derive(Debug)]
pub struct RestoredSmartHomeRuntime {
    pub runtime: SmartHomeRuntime,
    pub automation_definitions: Vec<DurableAutomationDefinition>,
    pub automation_state: Option<serde_json::Value>,
    pub saved_at_ms: u64,
    pub revision: Revision,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct RuntimeStoreEnvelope {
    schema_version: u32,
    saved_at_ms: u64,
    runtime: RuntimeDurableSnapshot,
    automation_definitions: Vec<DurableAutomationDefinition>,
    #[serde(default)]
    automation_state: Option<serde_json::Value>,
}

/// Errors surfaced by durable runtime persistence.
#[derive(Debug)]
pub enum RuntimeStoreError {
    Storage(StorageError),
    Runtime(RuntimeError),
    Encode(String),
    Decode(String),
    UnsupportedSchema(u32),
    Validation {
        field: &'static str,
        message: String,
    },
}

impl fmt::Display for RuntimeStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Storage(error) => write!(f, "{error}"),
            Self::Runtime(error) => write!(f, "{error}"),
            Self::Encode(message) => write!(f, "could not encode smart-home runtime: {message}"),
            Self::Decode(message) => write!(f, "could not decode smart-home runtime: {message}"),
            Self::UnsupportedSchema(version) => {
                write!(f, "unsupported smart-home runtime schema version {version}")
            }
            Self::Validation { field, message } => {
                write!(f, "invalid smart-home runtime {field}: {message}")
            }
        }
    }
}

impl std::error::Error for RuntimeStoreError {}

impl From<StorageError> for RuntimeStoreError {
    fn from(error: StorageError) -> Self {
        Self::Storage(error)
    }
}

impl From<RuntimeError> for RuntimeStoreError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

/// Versioned durable runtime store over any repository-owned backend.
pub struct SmartHomeRuntimeStore<B> {
    backend: B,
    namespace: String,
    key: String,
}

impl<B: StorageBackend> SmartHomeRuntimeStore<B> {
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            namespace: DEFAULT_NAMESPACE.to_string(),
            key: DEFAULT_KEY.to_string(),
        }
    }

    pub fn with_location(backend: B, namespace: impl Into<String>, key: impl Into<String>) -> Self {
        Self {
            backend,
            namespace: namespace.into(),
            key: key.into(),
        }
    }

    pub fn save(
        &self,
        runtime: &SmartHomeRuntime,
        automation_definitions: &[DurableAutomationDefinition],
        saved_at_ms: u64,
    ) -> Result<Revision, RuntimeStoreError> {
        self.save_with_automation_state(runtime, automation_definitions, None, saved_at_ms)
    }

    pub fn save_with_automation_state(
        &self,
        runtime: &SmartHomeRuntime,
        automation_definitions: &[DurableAutomationDefinition],
        automation_state: Option<serde_json::Value>,
        saved_at_ms: u64,
    ) -> Result<Revision, RuntimeStoreError> {
        validate_automation_definitions(automation_definitions)?;
        if automation_state
            .as_ref()
            .is_some_and(|state| !state.is_object())
        {
            return Err(RuntimeStoreError::Validation {
                field: "automation_state",
                message: "must be a JSON object".to_string(),
            });
        }
        self.backend.initialize()?;
        let previous = self.backend.get(&self.namespace, &self.key)?;
        let envelope = RuntimeStoreEnvelope {
            schema_version: SCHEMA_VERSION,
            saved_at_ms,
            runtime: runtime.durable_snapshot(),
            automation_definitions: automation_definitions.to_vec(),
            automation_state,
        };
        let body = serde_json::to_vec(&envelope)
            .map_err(|error| RuntimeStoreError::Encode(error.to_string()))?;
        let input = StoragePutInput::new(
            self.namespace.clone(),
            self.key.clone(),
            CONTENT_TYPE,
            StorageMetadata::Object(Default::default()),
            body,
        )?
        .with_if_revision(previous.map(|record| record.revision));
        Ok(self.backend.put(input)?.revision)
    }

    /// Persist one complete runtime envelope only at the caller-owned revision.
    ///
    /// `None` requires the record to be absent. A concrete revision requires
    /// exact compare-and-swap ownership. Unlike [`Self::save_with_automation_state`],
    /// this method never adopts a newer revision observed during the call, so a
    /// long-lived controller cannot overwrite state committed by another owner.
    pub fn save_with_automation_state_at_revision(
        &self,
        runtime: &SmartHomeRuntime,
        automation_definitions: &[DurableAutomationDefinition],
        automation_state: Option<serde_json::Value>,
        saved_at_ms: u64,
        expected_revision: Option<&Revision>,
    ) -> Result<Revision, RuntimeStoreError> {
        validate_automation_definitions(automation_definitions)?;
        if automation_state
            .as_ref()
            .is_some_and(|state| !state.is_object())
        {
            return Err(RuntimeStoreError::Validation {
                field: "automation_state",
                message: "must be a JSON object".to_string(),
            });
        }
        self.backend.initialize()?;
        let envelope = RuntimeStoreEnvelope {
            schema_version: SCHEMA_VERSION,
            saved_at_ms,
            runtime: runtime.durable_snapshot(),
            automation_definitions: automation_definitions.to_vec(),
            automation_state,
        };
        let body = serde_json::to_vec(&envelope)
            .map_err(|error| RuntimeStoreError::Encode(error.to_string()))?;
        let input = StoragePutInput::new(
            self.namespace.clone(),
            self.key.clone(),
            CONTENT_TYPE,
            StorageMetadata::Object(Default::default()),
            body,
        )?;
        let input = match expected_revision {
            Some(revision) => input.with_if_revision(Some(revision.clone())),
            None => input.with_if_absent(),
        };
        Ok(self.backend.put(input)?.revision)
    }

    /// Migrates a live runtime and its durable snapshot as one revision-guarded
    /// operation.
    ///
    /// The candidate runtime is validated and migrated first, then persisted
    /// only if `expected_revision` still owns the stored record. The caller's
    /// live runtime is replaced only after that durable write succeeds.
    pub fn migrate_retained_identities(
        &self,
        runtime: &mut SmartHomeRuntime,
        migration: &RuntimeRetainedIdentityMigration,
        automation_definitions: &[DurableAutomationDefinition],
        automation_state: Option<serde_json::Value>,
        saved_at_ms: u64,
        expected_revision: Revision,
    ) -> Result<(RuntimeRetainedIdentityMigrationReport, Revision), RuntimeStoreError> {
        validate_automation_definitions(automation_definitions)?;
        if automation_state
            .as_ref()
            .is_some_and(|state| !state.is_object())
        {
            return Err(RuntimeStoreError::Validation {
                field: "automation_state",
                message: "must be a JSON object".to_string(),
            });
        }
        validate_automation_identity_references(
            migration,
            automation_definitions,
            automation_state.as_ref(),
        )?;

        let mut candidate = runtime.clone();
        let report = candidate.migrate_retained_identities(migration)?;
        let envelope = RuntimeStoreEnvelope {
            schema_version: SCHEMA_VERSION,
            saved_at_ms,
            runtime: candidate.durable_snapshot(),
            automation_definitions: automation_definitions.to_vec(),
            automation_state,
        };
        let body = serde_json::to_vec(&envelope)
            .map_err(|error| RuntimeStoreError::Encode(error.to_string()))?;
        self.backend.initialize()?;
        let input = StoragePutInput::new(
            self.namespace.clone(),
            self.key.clone(),
            CONTENT_TYPE,
            StorageMetadata::Object(Default::default()),
            body,
        )?
        .with_if_revision(Some(expected_revision));
        let revision = self.backend.put(input)?.revision;
        *runtime = candidate;
        Ok((report, revision))
    }

    /// Completes one authorized pairing session as a revision-guarded durable
    /// runtime mutation.
    ///
    /// The caller must have already sealed the credential payload and supplies
    /// only its opaque `VaultRef`. The completion runs against a cloned runtime,
    /// including D23 Human Approval authorization, then persists that complete
    /// candidate with compare-and-swap. Live state changes only after the write
    /// succeeds. The prior bridge reference is returned so a credential host
    /// can perform explicit replacement cleanup after the durable commit.
    pub fn complete_pairing(
        &self,
        runtime: &mut SmartHomeRuntime,
        principal_id: AgentId,
        request: RuntimeCompletePairingToolRequest,
        automation_definitions: &[DurableAutomationDefinition],
        automation_state: Option<serde_json::Value>,
        expected_revision: Revision,
    ) -> Result<(RuntimeCompletePairingToolOutput, Option<VaultRef>, Revision), RuntimeStoreError>
    {
        validate_automation_definitions(automation_definitions)?;
        if automation_state
            .as_ref()
            .is_some_and(|state| !state.is_object())
        {
            return Err(RuntimeStoreError::Validation {
                field: "automation_state",
                message: "must be a JSON object".to_string(),
            });
        }

        let mut candidate = runtime.clone();
        let completed_at_ms = request.completion.completed_at_ms;
        let output =
            candidate.execute_complete_pairing_tool(principal_id, request, completed_at_ms)?;
        let previous_vault_ref = runtime
            .registry()
            .bridge(&output.session.bridge_id)
            .and_then(|bridge| bridge.auth_ref.clone());
        let envelope = RuntimeStoreEnvelope {
            schema_version: SCHEMA_VERSION,
            saved_at_ms: completed_at_ms,
            runtime: candidate.durable_snapshot(),
            automation_definitions: automation_definitions.to_vec(),
            automation_state,
        };
        let body = serde_json::to_vec(&envelope)
            .map_err(|error| RuntimeStoreError::Encode(error.to_string()))?;
        self.backend.initialize()?;
        let input = StoragePutInput::new(
            self.namespace.clone(),
            self.key.clone(),
            CONTENT_TYPE,
            StorageMetadata::Object(Default::default()),
            body,
        )?
        .with_if_revision(Some(expected_revision));
        let revision = self.backend.put(input)?.revision;
        *runtime = candidate;
        Ok((output, previous_vault_ref, revision))
    }

    pub fn load(&self) -> Result<Option<RestoredSmartHomeRuntime>, RuntimeStoreError> {
        self.backend.initialize()?;
        let Some(record) = self.backend.get(&self.namespace, &self.key)? else {
            return Ok(None);
        };
        let envelope: RuntimeStoreEnvelope = serde_json::from_slice(&record.body)
            .map_err(|error| RuntimeStoreError::Decode(error.to_string()))?;
        if envelope.schema_version != SCHEMA_VERSION {
            return Err(RuntimeStoreError::UnsupportedSchema(
                envelope.schema_version,
            ));
        }
        validate_automation_definitions(&envelope.automation_definitions)?;
        Ok(Some(RestoredSmartHomeRuntime {
            runtime: SmartHomeRuntime::restore_durable_snapshot(envelope.runtime)?,
            automation_definitions: envelope.automation_definitions,
            automation_state: envelope.automation_state,
            saved_at_ms: envelope.saved_at_ms,
            revision: record.revision,
        }))
    }

    pub fn into_backend(self) -> B {
        self.backend
    }
}

fn validate_automation_definitions(
    definitions: &[DurableAutomationDefinition],
) -> Result<(), RuntimeStoreError> {
    let mut ids = std::collections::BTreeSet::new();
    for definition in definitions {
        DurableAutomationDefinition::new(
            definition.automation_id.clone(),
            definition.enabled,
            definition.definition.clone(),
        )?;
        if !ids.insert(definition.automation_id.as_str()) {
            return Err(RuntimeStoreError::Validation {
                field: "automation_id",
                message: format!("duplicate id {}", definition.automation_id),
            });
        }
    }
    Ok(())
}

fn validate_automation_identity_references(
    migration: &RuntimeRetainedIdentityMigration,
    definitions: &[DurableAutomationDefinition],
    state: Option<&serde_json::Value>,
) -> Result<(), RuntimeStoreError> {
    let source_ids = migration
        .source_device_ids()
        .map(|device_id| device_id.as_str())
        .chain(
            migration
                .source_entity_ids()
                .map(|entity_id| entity_id.as_str()),
        )
        .collect::<std::collections::BTreeSet<_>>();
    if definitions
        .iter()
        .any(|definition| json_references_any(&definition.definition, &source_ids))
    {
        return Err(RuntimeStoreError::Validation {
            field: "automation_definitions",
            message: "must not retain a source device or entity identity".to_string(),
        });
    }
    if state.is_some_and(|state| json_references_any(state, &source_ids)) {
        return Err(RuntimeStoreError::Validation {
            field: "automation_state",
            message: "must not retain a source device or entity identity".to_string(),
        });
    }
    Ok(())
}

fn json_references_any(
    value: &serde_json::Value,
    source_ids: &std::collections::BTreeSet<&str>,
) -> bool {
    match value {
        serde_json::Value::String(value) => source_ids.contains(value.as_str()),
        serde_json::Value::Array(values) => values
            .iter()
            .any(|value| json_references_any(value, source_ids)),
        serde_json::Value::Object(fields) => fields.iter().any(|(key, value)| {
            source_ids.contains(key.as_str()) || json_references_any(value, source_ids)
        }),
        serde_json::Value::Null | serde_json::Value::Bool(_) | serde_json::Value::Number(_) => {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smart_home_core::{
        AgentId, AuthorizationOutcome, Bridge, BridgeId, BridgeTransport, Capability,
        CapabilityGrant, CapabilityGrantId, CapabilityId, CommandId, CommandResult, CommandStatus,
        CorrelationId, Device, DeviceEvent, DeviceEventType, DeviceId, Entity, EntityId,
        EntityKind, EventId, Health, IntegrationId, PrivilegeTier, StateDelta, Value, VaultRef,
    };
    use smart_home_runtime::{
        DesiredEntityState, DesiredStateQuery, PairingSessionStatus,
        RetainedDeviceIdentityReplacement, RetainedEntityIdentityReplacement,
        RuntimeCommandResultQuery, RuntimeCompletePairingToolRequest, RuntimeEvent,
        RuntimePairingSession, RuntimePairingSessionId, RuntimePairingSessionQuery,
        RuntimeRetainedIdentityMigration,
    };
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};
    use storage_local_folder::LocalFolderStorageBackend;

    fn temp_root(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("test clock should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "smart-home-runtime-store-{}-{name}-{nanos}",
            std::process::id()
        ))
    }

    fn runtime_fixture() -> SmartHomeRuntime {
        let mut runtime = SmartHomeRuntime::new();
        runtime
            .upsert_bridge(Bridge::new(
                BridgeId::trusted("bridge-1"),
                IntegrationId::trusted("hue"),
                BridgeTransport::LanHttp,
            ))
            .unwrap();
        runtime
            .upsert_device(Device {
                device_id: DeviceId::trusted("device-1"),
                bridge_id: BridgeId::trusted("bridge-1"),
                manufacturer: "Signify".to_string(),
                model: "Hue bulb".to_string(),
                name: "Kitchen".to_string(),
                serial: None,
                firmware_version: None,
                room_id: Some("kitchen".to_string()),
                entity_ids: Vec::new(),
                identifiers: Vec::new(),
                health: Health::Online,
                metadata: Vec::new(),
            })
            .unwrap();
        runtime
            .upsert_entity(Entity {
                entity_id: EntityId::trusted("entity-1"),
                device_id: DeviceId::trusted("device-1"),
                kind: EntityKind::Light,
                name: "Kitchen Light".to_string(),
                capabilities: vec![Capability::light_on_off()],
                state: None,
                metadata: Vec::new(),
            })
            .unwrap();
        runtime
            .apply_device_event(DeviceEvent {
                event_id: EventId::trusted("event-1"),
                bridge_id: BridgeId::trusted("bridge-1"),
                device_id: Some(DeviceId::trusted("device-1")),
                entity_id: Some(EntityId::trusted("entity-1")),
                observed_at_ms: 100,
                received_at_ms: 101,
                event_type: DeviceEventType::Updated,
                state_delta: Some(StateDelta {
                    capability_id: CapabilityId::trusted("light.on_off"),
                    value: Value::Bool(true),
                }),
                raw_ref: None,
                correlation_id: None,
                metadata: Vec::new(),
            })
            .unwrap();
        runtime
            .event_bus_mut()
            .publish(RuntimeEvent::CommandResult(CommandResult {
                command_id: CommandId::trusted("command-1"),
                status: CommandStatus::Accepted,
                bridge_id: BridgeId::trusted("bridge-1"),
                correlation_id: CorrelationId::trusted("correlation-1"),
                message: None,
            }));
        let bridge = runtime
            .registry()
            .bridge(&BridgeId::trusted("bridge-1"))
            .unwrap()
            .clone();
        runtime
            .start_pairing_session(RuntimePairingSession::pending(
                RuntimePairingSessionId::trusted("pairing-1"),
                &bridge,
                AgentId::trusted("agent:test"),
                200,
                1_200,
                Vec::new(),
            ))
            .unwrap();
        runtime
            .upsert_desired_state(DesiredEntityState::new(
                EntityId::trusted("entity-1"),
                vec![StateDelta {
                    capability_id: CapabilityId::trusted("light.on_off"),
                    value: Value::Bool(false),
                }],
            ))
            .unwrap();
        runtime
    }

    fn identity_migration(runtime: &SmartHomeRuntime) -> RuntimeRetainedIdentityMigration {
        let mut device = runtime
            .registry()
            .device(&DeviceId::trusted("device-1"))
            .unwrap()
            .clone();
        device.device_id = DeviceId::trusted("device-rotated");
        device.entity_ids = vec![EntityId::trusted("entity-rotated")];
        let mut entity = runtime
            .registry()
            .entity(&EntityId::trusted("entity-1"))
            .unwrap()
            .clone();
        entity.entity_id = EntityId::trusted("entity-rotated");
        entity.device_id = DeviceId::trusted("device-rotated");
        entity.state = None;

        RuntimeRetainedIdentityMigration::new(vec![RetainedDeviceIdentityReplacement::new(
            DeviceId::trusted("device-1"),
            device,
            vec![RetainedEntityIdentityReplacement::new(
                EntityId::trusted("entity-1"),
                entity,
            )],
        )])
    }

    fn grant_pairing(runtime: &mut SmartHomeRuntime, principal: &AgentId) {
        runtime.registry_mut().upsert_capability_grant(
            CapabilityGrant::for_capability(
                CapabilityGrantId::trusted("grant-pair"),
                principal.clone(),
                CapabilityId::trusted("smart_home.pair"),
                PrivilegeTier::HumanApproval,
                "test",
                400,
            )
            .with_expiry(1_000),
        );
    }

    #[test]
    fn local_folder_restart_restores_runtime_and_automation_definitions() {
        let root = temp_root("restart");
        let automation = DurableAutomationDefinition::new(
            "automation-kitchen-off",
            true,
            serde_json::json!({
                "trigger": {"kind": "schedule", "at": "23:00"},
                "actions": [{"entity_id": "entity-1", "on": false}]
            }),
        )
        .unwrap();
        let runtime = runtime_fixture();
        let store = SmartHomeRuntimeStore::new(LocalFolderStorageBackend::new(root.clone()));
        let automation_state = serde_json::json!({
            "schema_version": 1,
            "completed_trigger_keys": ["automation-kitchen-off:schedule:1"],
            "audit_records": [{"outcome": "executed"}]
        });
        let saved_revision = store
            .save_with_automation_state(
                &runtime,
                std::slice::from_ref(&automation),
                Some(automation_state.clone()),
                500,
            )
            .unwrap();
        drop(store);

        let reopened = SmartHomeRuntimeStore::new(LocalFolderStorageBackend::new(root.clone()));
        let restored = reopened
            .load()
            .unwrap()
            .expect("saved runtime should exist");

        assert_eq!(restored.saved_at_ms, 500);
        assert_eq!(restored.revision, saved_revision);
        assert_eq!(restored.automation_definitions, vec![automation]);
        assert_eq!(restored.automation_state, Some(automation_state));
        assert_eq!(restored.runtime.registry().counts().bridges, 1);
        assert_eq!(restored.runtime.registry().counts().devices, 1);
        assert_eq!(restored.runtime.registry().counts().entities, 1);
        assert_eq!(restored.runtime.registry().counts().states, 1);
        assert_eq!(restored.runtime.registry().counts().events, 1);
        assert_eq!(
            restored
                .runtime
                .query_command_results(&RuntimeCommandResultQuery::new())
                .len(),
            1
        );
        assert_eq!(
            restored
                .runtime
                .query_pairing_sessions(&RuntimePairingSessionQuery::new())
                .len(),
            1
        );
        assert_eq!(
            restored
                .runtime
                .query_desired_states(&DesiredStateQuery::new())
                .len(),
            1
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn pairing_completion_persists_before_swapping_live_runtime() {
        let root = temp_root("pairing-completion");
        let principal = AgentId::trusted("agent:test");
        let old_vault_ref = VaultRef::trusted("vault://smart-home/hue/bridge-1/old");
        let new_vault_ref = VaultRef::trusted("vault://smart-home/hue/bridge-1/new");
        let mut runtime = runtime_fixture();
        let mut bridge = runtime
            .registry()
            .bridge(&BridgeId::trusted("bridge-1"))
            .unwrap()
            .clone();
        bridge.auth_ref = Some(old_vault_ref.clone());
        runtime.upsert_bridge(bridge).unwrap();
        grant_pairing(&mut runtime, &principal);
        let store = SmartHomeRuntimeStore::new(LocalFolderStorageBackend::new(root.clone()));
        let initial_revision = store.save(&runtime, &[], 500).unwrap();

        let (output, previous_vault_ref, completed_revision) = store
            .complete_pairing(
                &mut runtime,
                principal.clone(),
                RuntimeCompletePairingToolRequest::new(
                    RuntimePairingSessionId::trusted("pairing-1"),
                    new_vault_ref.clone(),
                    600,
                ),
                &[],
                None,
                initial_revision,
            )
            .unwrap();

        assert_eq!(output.session.status, PairingSessionStatus::Completed);
        assert_eq!(previous_vault_ref, Some(old_vault_ref));
        assert_eq!(
            runtime
                .registry()
                .bridge(&BridgeId::trusted("bridge-1"))
                .unwrap()
                .auth_ref,
            Some(new_vault_ref.clone())
        );
        assert!(runtime
            .registry()
            .authorization_decisions_for_principal(&principal)
            .iter()
            .any(|decision| decision.outcome == AuthorizationOutcome::Allowed));

        let restored = store.load().unwrap().unwrap();
        assert_eq!(restored.revision, completed_revision);
        assert_eq!(restored.saved_at_ms, 600);
        assert_eq!(
            restored
                .runtime
                .pairing_session(&RuntimePairingSessionId::trusted("pairing-1"))
                .unwrap()
                .status,
            PairingSessionStatus::Completed
        );
        assert_eq!(
            restored
                .runtime
                .registry()
                .bridge(&BridgeId::trusted("bridge-1"))
                .unwrap()
                .auth_ref,
            Some(new_vault_ref)
        );
        assert!(restored
            .runtime
            .registry()
            .authorization_decisions_for_principal(&principal)
            .iter()
            .any(|decision| decision.outcome == AuthorizationOutcome::Allowed));
        assert!(restored.runtime.registry().events().any(|event| {
            event.metadata.iter().any(|metadata| {
                metadata.key == "smart_home.pairing_session" && metadata.value == "pairing-1"
            })
        }));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn pairing_completion_storage_conflict_keeps_live_session_pending() {
        let root = temp_root("pairing-conflict");
        let principal = AgentId::trusted("agent:test");
        let mut runtime = runtime_fixture();
        grant_pairing(&mut runtime, &principal);
        let store = SmartHomeRuntimeStore::new(LocalFolderStorageBackend::new(root.clone()));
        let stale_revision = store.save(&runtime, &[], 500).unwrap();
        let current_revision = store.save(&runtime, &[], 550).unwrap();
        let before = runtime.durable_snapshot();

        let error = store
            .complete_pairing(
                &mut runtime,
                principal,
                RuntimeCompletePairingToolRequest::new(
                    RuntimePairingSessionId::trusted("pairing-1"),
                    VaultRef::trusted("vault://smart-home/hue/bridge-1/new"),
                    600,
                ),
                &[],
                None,
                stale_revision,
            )
            .unwrap_err();

        assert!(matches!(error, RuntimeStoreError::Storage(_)));
        assert_eq!(runtime.durable_snapshot(), before);
        let restored = store.load().unwrap().unwrap();
        assert_eq!(restored.revision, current_revision);
        assert_eq!(
            restored
                .runtime
                .pairing_session(&RuntimePairingSessionId::trusted("pairing-1"))
                .unwrap()
                .status,
            PairingSessionStatus::PendingUserPresence
        );
        assert_eq!(
            restored
                .runtime
                .registry()
                .bridge(&BridgeId::trusted("bridge-1"))
                .unwrap()
                .auth_ref,
            None
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn pairing_completion_denial_writes_no_candidate() {
        let root = temp_root("pairing-denial");
        let mut runtime = runtime_fixture();
        let before = runtime.durable_snapshot();
        let store = SmartHomeRuntimeStore::new(LocalFolderStorageBackend::new(root.clone()));
        let initial_revision = store.save(&runtime, &[], 500).unwrap();

        let error = store
            .complete_pairing(
                &mut runtime,
                AgentId::trusted("agent:unauthorized"),
                RuntimeCompletePairingToolRequest::new(
                    RuntimePairingSessionId::trusted("pairing-1"),
                    VaultRef::trusted("vault://smart-home/hue/bridge-1/new"),
                    600,
                ),
                &[],
                None,
                initial_revision.clone(),
            )
            .unwrap_err();

        assert!(matches!(
            error,
            RuntimeStoreError::Runtime(RuntimeError::UnauthorizedTool { .. })
        ));
        assert_eq!(runtime.durable_snapshot(), before);
        assert_eq!(store.load().unwrap().unwrap().revision, initial_revision);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn retained_identity_migration_persists_before_swapping_the_live_runtime() {
        let root = temp_root("identity-migration");
        let mut runtime = runtime_fixture();
        let migration = identity_migration(&runtime);
        let store = SmartHomeRuntimeStore::new(LocalFolderStorageBackend::new(root.clone()));
        let initial_revision = store.save(&runtime, &[], 500).unwrap();

        let (report, migrated_revision) = store
            .migrate_retained_identities(&mut runtime, &migration, &[], None, 600, initial_revision)
            .unwrap();

        assert_eq!(report.migrated_devices, 1);
        assert_ne!(migrated_revision.as_str(), "");
        assert!(runtime
            .registry()
            .device(&DeviceId::trusted("device-1"))
            .is_none());
        assert!(runtime
            .registry()
            .state(&EntityId::trusted("entity-rotated"))
            .is_some());
        let restored = store.load().unwrap().unwrap();
        assert_eq!(restored.saved_at_ms, 600);
        assert_eq!(restored.revision, migrated_revision);
        assert!(restored
            .runtime
            .registry()
            .device(&DeviceId::trusted("device-1"))
            .is_none());
        assert!(restored
            .runtime
            .registry()
            .state(&EntityId::trusted("entity-rotated"))
            .is_some());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn retained_identity_migration_storage_conflict_keeps_live_runtime_unchanged() {
        let root = temp_root("identity-conflict");
        let mut runtime = runtime_fixture();
        let migration = identity_migration(&runtime);
        let store = SmartHomeRuntimeStore::new(LocalFolderStorageBackend::new(root.clone()));
        let stale_revision = store.save(&runtime, &[], 500).unwrap();
        let current_revision = store.save(&runtime, &[], 550).unwrap();
        let before = runtime.durable_snapshot();

        let error = store
            .migrate_retained_identities(&mut runtime, &migration, &[], None, 600, stale_revision)
            .unwrap_err();

        assert!(matches!(error, RuntimeStoreError::Storage(_)));
        assert_eq!(runtime.durable_snapshot(), before);
        let restored = store.load().unwrap().unwrap();
        assert_eq!(restored.revision, current_revision);
        assert!(restored
            .runtime
            .registry()
            .device(&DeviceId::trusted("device-1"))
            .is_some());
        assert!(restored
            .runtime
            .registry()
            .device(&DeviceId::trusted("device-rotated"))
            .is_none());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn retained_identity_migration_rejects_stale_automation_identity_references() {
        let root = temp_root("identity-automation-reference");
        let mut runtime = runtime_fixture();
        let migration = identity_migration(&runtime);
        let automation = DurableAutomationDefinition::new(
            "automation-1",
            true,
            serde_json::json!({"action": {"entity_id": "entity-1"}}),
        )
        .unwrap();
        let store = SmartHomeRuntimeStore::new(LocalFolderStorageBackend::new(root.clone()));
        let revision = store
            .save(&runtime, std::slice::from_ref(&automation), 500)
            .unwrap();
        let before = runtime.durable_snapshot();

        let error = store
            .migrate_retained_identities(
                &mut runtime,
                &migration,
                std::slice::from_ref(&automation),
                None,
                600,
                revision.clone(),
            )
            .unwrap_err();

        assert!(matches!(
            error,
            RuntimeStoreError::Validation {
                field: "automation_definitions",
                ..
            }
        ));
        assert_eq!(runtime.durable_snapshot(), before);
        assert_eq!(store.load().unwrap().unwrap().revision, revision);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_duplicate_automation_ids_before_writing() {
        let root = temp_root("duplicate-automation");
        let automation =
            DurableAutomationDefinition::new("duplicate", true, serde_json::json!({})).unwrap();
        let store = SmartHomeRuntimeStore::new(LocalFolderStorageBackend::new(root.clone()));

        let error = store
            .save(
                &SmartHomeRuntime::new(),
                &[automation.clone(), automation],
                1,
            )
            .unwrap_err();

        assert!(matches!(
            error,
            RuntimeStoreError::Validation {
                field: "automation_id",
                ..
            }
        ));
        assert!(!root.exists());
    }

    #[test]
    fn rejects_non_object_automation_state_before_writing() {
        let root = temp_root("invalid-automation-state");
        let store = SmartHomeRuntimeStore::new(LocalFolderStorageBackend::new(root.clone()));

        let error = store
            .save_with_automation_state(
                &SmartHomeRuntime::new(),
                &[],
                Some(serde_json::json!(["not", "an", "object"])),
                1,
            )
            .unwrap_err();

        assert!(matches!(
            error,
            RuntimeStoreError::Validation {
                field: "automation_state",
                ..
            }
        ));
        assert!(!root.exists());
    }
}
