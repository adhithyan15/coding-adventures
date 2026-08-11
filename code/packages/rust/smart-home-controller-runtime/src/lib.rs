//! Central, restart-safe ownership of smart-home controller state.
//!
//! The controller owns the normalized runtime, the automation runtime, and
//! their combined durable store. Mutations are serialized, evaluated against
//! clones, persisted as one envelope, and published to shared adapters only
//! after the durable write succeeds.

#![forbid(unsafe_code)]

use smart_home_automation_runtime::{
    AutomationError, AutomationEvaluationReport, AutomationTriggerInput, SmartHomeAutomationRuntime,
};
use smart_home_core::AgentId;
use smart_home_runtime::SmartHomeRuntime;
use smart_home_runtime_store::{RuntimeStoreError, SmartHomeRuntimeStore};
use std::convert::Infallible;
use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard};
use storage_core::{Revision, StorageBackend};

/// Shared handle used by HTTP, discovery, pairing, and integration adapters.
pub type SharedSmartHomeRuntime = Arc<Mutex<SmartHomeRuntime>>;

/// Shared handle used by HTTP and automation worker adapters.
pub type SharedSmartHomeAutomationRuntime = Arc<Mutex<SmartHomeAutomationRuntime>>;

struct DurableCoordinator<B> {
    store: SmartHomeRuntimeStore<B>,
    revision: Option<Revision>,
    last_saved_at_ms: Option<u64>,
}

/// Metadata returned after one controller transaction commits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControllerCommit<T> {
    pub value: T,
    pub revision: Revision,
    pub saved_at_ms: u64,
}

/// Failure produced while restoring both controller runtimes.
#[derive(Debug)]
pub enum ControllerRestoreError {
    RuntimeStore(RuntimeStoreError),
    Automation(AutomationError),
}

impl fmt::Display for ControllerRestoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RuntimeStore(error) => write!(f, "could not restore controller store: {error}"),
            Self::Automation(error) => {
                write!(f, "could not restore controller automations: {error}")
            }
        }
    }
}

impl std::error::Error for ControllerRestoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::RuntimeStore(error) => Some(error),
            Self::Automation(error) => Some(error),
        }
    }
}

/// Failure produced by a persistence callback adapter.
#[derive(Debug)]
pub enum ControllerPersistenceError {
    LockPoisoned(&'static str),
    Automation(AutomationError),
    RuntimeStore(RuntimeStoreError),
}

impl fmt::Display for ControllerPersistenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LockPoisoned(component) => {
                write!(f, "smart-home controller {component} mutex was poisoned")
            }
            Self::Automation(error) => {
                write!(f, "could not snapshot controller automations: {error}")
            }
            Self::RuntimeStore(error) => write!(f, "could not persist controller state: {error}"),
        }
    }
}

impl std::error::Error for ControllerPersistenceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::LockPoisoned(_) => None,
            Self::Automation(error) => Some(error),
            Self::RuntimeStore(error) => Some(error),
        }
    }
}

/// Failure produced by a serialized controller transaction.
#[derive(Debug)]
pub enum ControllerTransactionError<E> {
    LockPoisoned(&'static str),
    RevisionConflict {
        expected: Revision,
        actual: Option<Revision>,
    },
    Mutation(E),
    Persistence(ControllerPersistenceError),
}

impl<E: fmt::Display> fmt::Display for ControllerTransactionError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LockPoisoned(component) => {
                write!(f, "smart-home controller {component} mutex was poisoned")
            }
            Self::RevisionConflict { expected, actual } => match actual {
                Some(actual) => write!(
                    f,
                    "controller revision conflict: expected {expected}, actual {actual}"
                ),
                None => write!(
                    f,
                    "controller revision conflict: expected {expected}, actual none"
                ),
            },
            Self::Mutation(error) => write!(f, "controller mutation failed: {error}"),
            Self::Persistence(error) => write!(f, "{error}"),
        }
    }
}

impl<E> std::error::Error for ControllerTransactionError<E>
where
    E: std::error::Error + 'static,
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::LockPoisoned(_) => None,
            Self::RevisionConflict { .. } => None,
            Self::Mutation(error) => Some(error),
            Self::Persistence(error) => Some(error),
        }
    }
}

/// Central owner for all durable smart-home controller state.
///
/// Shared consumers must acquire the runtime mutex before the automation mutex
/// when they need both. Controller transactions and callback adapters use that
/// same ordering, followed by the durable coordinator mutex.
pub struct SmartHomeControllerRuntime<B> {
    runtime: SharedSmartHomeRuntime,
    automations: SharedSmartHomeAutomationRuntime,
    durable: Arc<Mutex<DurableCoordinator<B>>>,
    restored_at_ms: Option<u64>,
}

impl<B> Clone for SmartHomeControllerRuntime<B> {
    fn clone(&self) -> Self {
        Self {
            runtime: Arc::clone(&self.runtime),
            automations: Arc::clone(&self.automations),
            durable: Arc::clone(&self.durable),
            restored_at_ms: self.restored_at_ms,
        }
    }
}

impl<B: StorageBackend> SmartHomeControllerRuntime<B> {
    /// Restore the default smart-home runtime record, or start empty.
    pub fn restore(backend: B) -> Result<Self, ControllerRestoreError> {
        Self::restore_store(SmartHomeRuntimeStore::new(backend))
    }

    /// Restore a smart-home runtime record at a caller-selected location.
    pub fn restore_with_location(
        backend: B,
        namespace: impl Into<String>,
        key: impl Into<String>,
    ) -> Result<Self, ControllerRestoreError> {
        Self::restore_store(SmartHomeRuntimeStore::with_location(
            backend, namespace, key,
        ))
    }

    fn restore_store(store: SmartHomeRuntimeStore<B>) -> Result<Self, ControllerRestoreError> {
        let restored = store.load().map_err(ControllerRestoreError::RuntimeStore)?;
        let (runtime, automations, revision, restored_at_ms) = match restored {
            Some(restored) => {
                let automations = SmartHomeAutomationRuntime::restore(
                    &restored.automation_definitions,
                    restored.automation_state.as_ref(),
                )
                .map_err(ControllerRestoreError::Automation)?;
                (
                    restored.runtime,
                    automations,
                    Some(restored.revision),
                    Some(restored.saved_at_ms),
                )
            }
            None => (
                SmartHomeRuntime::new(),
                SmartHomeAutomationRuntime::new(),
                None,
                None,
            ),
        };

        Ok(Self {
            runtime: Arc::new(Mutex::new(runtime)),
            automations: Arc::new(Mutex::new(automations)),
            durable: Arc::new(Mutex::new(DurableCoordinator {
                store,
                revision,
                last_saved_at_ms: restored_at_ms,
            })),
            restored_at_ms,
        })
    }

    /// Clone the normalized runtime adapter handle.
    pub fn runtime_handle(&self) -> SharedSmartHomeRuntime {
        Arc::clone(&self.runtime)
    }

    /// Clone the automation runtime adapter handle.
    pub fn automation_runtime_handle(&self) -> SharedSmartHomeAutomationRuntime {
        Arc::clone(&self.automations)
    }

    /// Timestamp stored on the snapshot loaded at startup, if one existed.
    pub fn restored_at_ms(&self) -> Option<u64> {
        self.restored_at_ms
    }

    /// Revision of the most recently loaded or committed durable envelope.
    pub fn revision(&self) -> Result<Option<Revision>, ControllerPersistenceError> {
        Ok(self.lock_durable()?.revision.clone())
    }

    /// Timestamp supplied for the most recently loaded or committed snapshot.
    pub fn last_saved_at_ms(&self) -> Result<Option<u64>, ControllerPersistenceError> {
        Ok(self.lock_durable()?.last_saved_at_ms)
    }

    /// Mutate cloned state, persist it, then atomically publish both candidates.
    ///
    /// Callback and persistence failures discard the candidates, leaving both
    /// shared runtimes byte-for-byte equivalent to their prior snapshots.
    pub fn transaction<T, E>(
        &self,
        saved_at_ms: u64,
        mutation: impl FnOnce(&mut SmartHomeRuntime, &mut SmartHomeAutomationRuntime) -> Result<T, E>,
    ) -> Result<ControllerCommit<T>, ControllerTransactionError<E>> {
        self.transaction_inner(None, saved_at_ms, mutation)
    }

    /// Mutate and commit only when the controller still owns `expected_revision`.
    ///
    /// The revision check happens while all controller locks are held and before
    /// candidates are cloned or `mutation` is invoked. A conflict therefore has
    /// no callback, in-memory, or persistence side effects.
    pub fn transaction_at_revision<T, E>(
        &self,
        expected_revision: &Revision,
        saved_at_ms: u64,
        mutation: impl FnOnce(&mut SmartHomeRuntime, &mut SmartHomeAutomationRuntime) -> Result<T, E>,
    ) -> Result<ControllerCommit<T>, ControllerTransactionError<E>> {
        self.transaction_inner(Some(expected_revision), saved_at_ms, mutation)
    }

    fn transaction_inner<T, E>(
        &self,
        expected_revision: Option<&Revision>,
        saved_at_ms: u64,
        mutation: impl FnOnce(&mut SmartHomeRuntime, &mut SmartHomeAutomationRuntime) -> Result<T, E>,
    ) -> Result<ControllerCommit<T>, ControllerTransactionError<E>> {
        let mut runtime = self
            .runtime
            .lock()
            .map_err(|_| ControllerTransactionError::LockPoisoned("runtime"))?;
        let mut automations = self
            .automations
            .lock()
            .map_err(|_| ControllerTransactionError::LockPoisoned("automation runtime"))?;
        let mut durable = self
            .durable
            .lock()
            .map_err(|_| ControllerTransactionError::LockPoisoned("durable coordinator"))?;

        if let Some(expected) = expected_revision {
            if durable.revision.as_ref() != Some(expected) {
                return Err(ControllerTransactionError::RevisionConflict {
                    expected: expected.clone(),
                    actual: durable.revision.clone(),
                });
            }
        }

        let mut runtime_candidate = runtime.clone();
        let mut automation_candidate = automations.clone();
        let value = mutation(&mut runtime_candidate, &mut automation_candidate)
            .map_err(ControllerTransactionError::Mutation)?;
        let revision = persist_locked(
            &mut durable,
            &runtime_candidate,
            &automation_candidate,
            saved_at_ms,
        )
        .map_err(ControllerTransactionError::Persistence)?;

        *runtime = runtime_candidate;
        *automations = automation_candidate;
        Ok(ControllerCommit {
            value,
            revision,
            saved_at_ms,
        })
    }

    /// Persist the current combined snapshot without otherwise mutating it.
    pub fn save_snapshot(
        &self,
        saved_at_ms: u64,
    ) -> Result<ControllerCommit<()>, ControllerTransactionError<Infallible>> {
        self.transaction(saved_at_ms, |_, _| Ok(()))
    }

    /// Evaluate one schedule or event trigger through the durable transaction.
    pub fn evaluate_automations(
        &self,
        principal_id: AgentId,
        input: AutomationTriggerInput,
        dry_run: bool,
        now_ms: u64,
    ) -> Result<
        ControllerCommit<AutomationEvaluationReport>,
        ControllerTransactionError<AutomationError>,
    > {
        self.transaction(now_ms, move |runtime, automations| {
            automations.evaluate(runtime, principal_id, input, dry_run, now_ms)
        })
    }

    /// Evaluate the current schedule occurrence and commit its result.
    pub fn tick(
        &self,
        principal_id: AgentId,
        now_ms: u64,
    ) -> Result<
        ControllerCommit<AutomationEvaluationReport>,
        ControllerTransactionError<AutomationError>,
    > {
        self.evaluate_automations(
            principal_id,
            AutomationTriggerInput::Schedule,
            false,
            now_ms,
        )
    }

    /// Persist a runtime already locked and mutated by an adapter.
    ///
    /// This method deliberately does not lock the runtime mutex again. It
    /// snapshots the controller-owned automation runtime while the caller still
    /// owns the runtime lock, then commits the combined envelope.
    pub fn persist_runtime_snapshot(
        &self,
        runtime: &SmartHomeRuntime,
        saved_at_ms: u64,
    ) -> Result<Revision, ControllerPersistenceError> {
        let automations = self
            .automations
            .lock()
            .map_err(|_| ControllerPersistenceError::LockPoisoned("automation runtime"))?;
        let mut durable = self.lock_durable()?;
        persist_locked(&mut durable, runtime, &automations, saved_at_ms)
    }

    /// Persist runtime and automation values already locked by an adapter.
    ///
    /// Neither shared state mutex is acquired here, avoiding recursive locking
    /// from `SmartHomePlatformHttpRuntime` automation callbacks.
    pub fn persist_combined_snapshot(
        &self,
        runtime: &SmartHomeRuntime,
        automations: &SmartHomeAutomationRuntime,
        saved_at_ms: u64,
    ) -> Result<Revision, ControllerPersistenceError> {
        let mut durable = self.lock_durable()?;
        persist_locked(&mut durable, runtime, automations, saved_at_ms)
    }

    /// Build the callback expected by HTTP runtime-only mutation routes.
    pub fn runtime_persistence_adapter(
        &self,
    ) -> impl Fn(&SmartHomeRuntime, u64) -> Result<(), String> + Send + Sync + 'static
    where
        B: 'static,
    {
        let controller = self.clone();
        move |runtime, saved_at_ms| {
            controller
                .persist_runtime_snapshot(runtime, saved_at_ms)
                .map(|_| ())
                .map_err(|error| error.to_string())
        }
    }

    /// Build the callback expected by HTTP automation mutation routes.
    pub fn automation_persistence_adapter(
        &self,
    ) -> impl Fn(&SmartHomeRuntime, &SmartHomeAutomationRuntime, u64) -> Result<(), String>
           + Send
           + Sync
           + 'static
    where
        B: 'static,
    {
        let controller = self.clone();
        move |runtime, automations, saved_at_ms| {
            controller
                .persist_combined_snapshot(runtime, automations, saved_at_ms)
                .map(|_| ())
                .map_err(|error| error.to_string())
        }
    }

    fn lock_durable(
        &self,
    ) -> Result<MutexGuard<'_, DurableCoordinator<B>>, ControllerPersistenceError> {
        self.durable
            .lock()
            .map_err(|_| ControllerPersistenceError::LockPoisoned("durable coordinator"))
    }
}

fn persist_locked<B: StorageBackend>(
    durable: &mut DurableCoordinator<B>,
    runtime: &SmartHomeRuntime,
    automations: &SmartHomeAutomationRuntime,
    saved_at_ms: u64,
) -> Result<Revision, ControllerPersistenceError> {
    let definitions = automations
        .durable_definitions()
        .map_err(ControllerPersistenceError::Automation)?;
    let automation_state = automations
        .snapshot_json()
        .map_err(ControllerPersistenceError::Automation)?;
    let revision = durable
        .store
        .save_with_automation_state_at_revision(
            runtime,
            &definitions,
            Some(automation_state),
            saved_at_ms,
            durable.revision.as_ref(),
        )
        .map_err(ControllerPersistenceError::RuntimeStore)?;
    durable.revision = Some(revision.clone());
    durable.last_saved_at_ms = Some(saved_at_ms);
    Ok(revision)
}

#[cfg(test)]
mod tests {
    use super::*;
    use smart_home_automation_runtime::{
        AutomationAction, AutomationDefinition, AutomationTrigger,
    };
    use smart_home_core::{
        Bridge, BridgeId, BridgeTransport, CommandType, EntityId, IntegrationId, Value,
    };
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Barrier, Mutex};
    use std::thread;
    use std::time::{SystemTime, UNIX_EPOCH};
    use storage_core::{
        InMemoryStorageBackend, StorageError, StorageLease, StorageListOptions, StoragePage,
        StoragePutInput, StorageRecord, StorageRecordSummary, StorageStat, StorageSummaryPage,
    };
    use storage_local_folder::LocalFolderStorageBackend;

    fn temp_root(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("test clock should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "smart-home-controller-runtime-{}-{name}-{nanos}",
            std::process::id()
        ))
    }

    fn bridge(id: &str) -> Bridge {
        Bridge::new(
            BridgeId::trusted(id),
            IntegrationId::trusted(format!("integration-{id}")),
            BridgeTransport::LanHttp,
        )
    }

    fn automation(id: &str) -> AutomationDefinition {
        AutomationDefinition {
            automation_id: id.to_string(),
            enabled: true,
            trigger: AutomationTrigger::Schedule {
                every_ms: 1_000,
                offset_ms: 0,
            },
            conditions: Vec::new(),
            actions: vec![AutomationAction::Command {
                entity_id: EntityId::trusted("entity-light"),
                command_type: CommandType::TurnOn,
                arguments: Value::Null,
                timeout_ms: None,
            }],
        }
    }

    #[test]
    fn local_folder_restart_restores_runtime_and_automations_together() {
        let root = temp_root("restart");
        let first = SmartHomeControllerRuntime::restore(LocalFolderStorageBackend::new(&root))
            .expect("empty controller should start");
        let commit = first
            .transaction(42, |runtime, automations| {
                runtime.upsert_bridge(bridge("bridge-one")).unwrap();
                automations
                    .upsert_definition(automation("morning"))
                    .unwrap();
                Ok::<_, Infallible>(())
            })
            .expect("combined transaction should commit");
        drop(first);

        let restored = SmartHomeControllerRuntime::restore(LocalFolderStorageBackend::new(&root))
            .expect("controller should restore");
        assert_eq!(restored.restored_at_ms(), Some(42));
        assert_eq!(restored.revision().unwrap(), Some(commit.revision));
        assert!(restored
            .runtime_handle()
            .lock()
            .unwrap()
            .registry()
            .bridge(&BridgeId::trusted("bridge-one"))
            .is_some());
        assert_eq!(
            restored
                .automation_runtime_handle()
                .lock()
                .unwrap()
                .definitions()
                .map(|definition| definition.automation_id.as_str())
                .collect::<Vec<_>>(),
            vec!["morning"]
        );

        fs::remove_dir_all(root).expect("temporary controller folder should be removable");
    }

    #[test]
    fn callback_and_cas_failures_leave_both_states_exact() {
        let fail_put = Arc::new(AtomicBool::new(false));
        let backend = ConflictBackend::new(Arc::clone(&fail_put));
        let controller = SmartHomeControllerRuntime::restore(backend).unwrap();
        controller
            .transaction(1, |runtime, automations| {
                runtime.upsert_bridge(bridge("baseline")).unwrap();
                automations
                    .upsert_definition(automation("baseline-rule"))
                    .unwrap();
                Ok::<_, Infallible>(())
            })
            .unwrap();
        let runtime_before = controller
            .runtime_handle()
            .lock()
            .unwrap()
            .durable_snapshot();
        let automation_before = controller
            .automation_runtime_handle()
            .lock()
            .unwrap()
            .snapshot();
        let revision_before = controller.revision().unwrap();

        let callback_result = controller.transaction(2, |runtime, automations| {
            runtime.upsert_bridge(bridge("discard-callback")).unwrap();
            automations
                .upsert_definition(automation("discard-callback-rule"))
                .unwrap();
            Err::<(), _>("callback rejected candidate")
        });
        assert!(matches!(
            callback_result,
            Err(ControllerTransactionError::Mutation(
                "callback rejected candidate"
            ))
        ));

        fail_put.store(true, Ordering::SeqCst);
        let persistence_result = controller.transaction(3, |runtime, automations| {
            runtime.upsert_bridge(bridge("discard-cas")).unwrap();
            automations
                .upsert_definition(automation("discard-cas-rule"))
                .unwrap();
            Ok::<_, Infallible>(())
        });
        assert!(matches!(
            persistence_result,
            Err(ControllerTransactionError::Persistence(
                ControllerPersistenceError::RuntimeStore(RuntimeStoreError::Storage(
                    StorageError::Conflict { .. }
                ))
            ))
        ));

        assert_eq!(
            controller
                .runtime_handle()
                .lock()
                .unwrap()
                .durable_snapshot(),
            runtime_before
        );
        assert_eq!(
            controller
                .automation_runtime_handle()
                .lock()
                .unwrap()
                .snapshot(),
            automation_before
        );
        assert_eq!(controller.revision().unwrap(), revision_before);
        assert_eq!(controller.last_saved_at_ms().unwrap(), Some(1));
    }

    #[test]
    fn real_external_revision_drift_is_rejected_without_publishing_candidate() {
        let root = temp_root("external-drift");
        let controller = SmartHomeControllerRuntime::restore(LocalFolderStorageBackend::new(&root))
            .expect("empty controller should start");
        let baseline = controller
            .transaction(1, |runtime, _| {
                runtime.upsert_bridge(bridge("baseline")).unwrap();
                Ok::<_, Infallible>(())
            })
            .expect("baseline should commit");

        let mut external_runtime = controller.runtime_handle().lock().unwrap().clone();
        external_runtime
            .upsert_bridge(bridge("external-owner"))
            .unwrap();
        SmartHomeRuntimeStore::new(LocalFolderStorageBackend::new(&root))
            .save_with_automation_state(&external_runtime, &[], None, 2)
            .expect("external owner should advance the durable revision");

        let result = controller.transaction(3, |runtime, _| {
            runtime.upsert_bridge(bridge("stale-candidate")).unwrap();
            Ok::<_, Infallible>(())
        });
        assert!(matches!(
            result,
            Err(ControllerTransactionError::Persistence(
                ControllerPersistenceError::RuntimeStore(RuntimeStoreError::Storage(
                    StorageError::Conflict { .. }
                ))
            ))
        ));
        let runtime = controller.runtime_handle();
        let runtime = runtime.lock().unwrap();
        assert!(runtime
            .registry()
            .bridge(&BridgeId::trusted("baseline"))
            .is_some());
        assert!(runtime
            .registry()
            .bridge(&BridgeId::trusted("external-owner"))
            .is_none());
        assert!(runtime
            .registry()
            .bridge(&BridgeId::trusted("stale-candidate"))
            .is_none());
        assert_eq!(controller.revision().unwrap(), Some(baseline.revision));

        fs::remove_dir_all(root).expect("temporary controller folder should be removable");
    }

    #[test]
    fn concurrent_transactions_are_serialized_without_lost_updates() {
        const THREADS: usize = 24;
        let controller =
            Arc::new(SmartHomeControllerRuntime::restore(InMemoryStorageBackend::new()).unwrap());
        let barrier = Arc::new(Barrier::new(THREADS));
        let mut workers = Vec::new();
        for index in 0..THREADS {
            let controller = Arc::clone(&controller);
            let barrier = Arc::clone(&barrier);
            workers.push(thread::spawn(move || {
                barrier.wait();
                controller
                    .transaction(index as u64 + 1, |runtime, _| {
                        runtime
                            .upsert_bridge(bridge(&format!("bridge-{index}")))
                            .unwrap();
                        Ok::<_, Infallible>(())
                    })
                    .unwrap();
            }));
        }
        for worker in workers {
            worker.join().unwrap();
        }

        assert_eq!(
            controller
                .runtime_handle()
                .lock()
                .unwrap()
                .registry()
                .bridges()
                .count(),
            THREADS
        );
        assert!(controller.revision().unwrap().is_some());
    }

    #[test]
    fn guarded_transaction_commits_only_at_the_expected_revision() {
        let controller =
            SmartHomeControllerRuntime::restore(InMemoryStorageBackend::new()).unwrap();
        let baseline = controller.save_snapshot(1).unwrap();

        let commit = controller
            .transaction_at_revision(&baseline.revision, 2, |runtime, _| {
                runtime.upsert_bridge(bridge("guarded")).unwrap();
                Ok::<_, Infallible>("committed")
            })
            .expect("matching revision should commit");

        assert_eq!(commit.value, "committed");
        assert_ne!(commit.revision, baseline.revision);
        assert_eq!(controller.revision().unwrap(), Some(commit.revision));
        assert_eq!(controller.last_saved_at_ms().unwrap(), Some(2));
        assert!(controller
            .runtime_handle()
            .lock()
            .unwrap()
            .registry()
            .bridge(&BridgeId::trusted("guarded"))
            .is_some());
    }

    #[test]
    fn stale_guard_rejects_before_invoking_or_publishing_mutation() {
        let controller =
            SmartHomeControllerRuntime::restore(InMemoryStorageBackend::new()).unwrap();
        let stale = controller.save_snapshot(1).unwrap();
        let current = controller
            .transaction(2, |runtime, automations| {
                runtime.upsert_bridge(bridge("current")).unwrap();
                automations
                    .upsert_definition(automation("current-rule"))
                    .unwrap();
                Ok::<_, Infallible>(())
            })
            .unwrap();
        let runtime_before = controller
            .runtime_handle()
            .lock()
            .unwrap()
            .durable_snapshot();
        let automation_before = controller
            .automation_runtime_handle()
            .lock()
            .unwrap()
            .snapshot();
        let invoked = AtomicBool::new(false);

        let result = controller.transaction_at_revision(&stale.revision, 3, |runtime, _| {
            invoked.store(true, Ordering::SeqCst);
            runtime.upsert_bridge(bridge("stale")).unwrap();
            Ok::<_, Infallible>(())
        });

        assert!(matches!(
            result,
            Err(ControllerTransactionError::RevisionConflict { expected, actual })
                if expected == stale.revision && actual == Some(current.revision.clone())
        ));
        assert!(!invoked.load(Ordering::SeqCst));
        assert_eq!(
            controller
                .runtime_handle()
                .lock()
                .unwrap()
                .durable_snapshot(),
            runtime_before
        );
        assert_eq!(
            controller
                .automation_runtime_handle()
                .lock()
                .unwrap()
                .snapshot(),
            automation_before
        );
        assert_eq!(controller.revision().unwrap(), Some(current.revision));
        assert_eq!(controller.last_saved_at_ms().unwrap(), Some(2));
    }

    #[test]
    fn persistence_adapters_do_not_relock_supplied_runtime_guards() {
        let controller =
            SmartHomeControllerRuntime::restore(InMemoryStorageBackend::new()).unwrap();
        let runtime_handle = controller.runtime_handle();
        let automations_handle = controller.automation_runtime_handle();

        let runtime_adapter = controller.runtime_persistence_adapter();
        let mut runtime = runtime_handle.lock().unwrap();
        runtime.upsert_bridge(bridge("adapter-bridge")).unwrap();
        runtime_adapter(&runtime, 11).unwrap();
        drop(runtime);
        assert_eq!(controller.last_saved_at_ms().unwrap(), Some(11));

        let automation_adapter = controller.automation_persistence_adapter();
        let runtime = runtime_handle.lock().unwrap();
        let mut automations = automations_handle.lock().unwrap();
        automations
            .upsert_definition(automation("adapter-rule"))
            .unwrap();
        automation_adapter(&runtime, &automations, 12).unwrap();
        drop(automations);
        drop(runtime);
        assert_eq!(controller.last_saved_at_ms().unwrap(), Some(12));
    }

    struct ConflictBackend {
        inner: InMemoryStorageBackend,
        fail_put: Arc<AtomicBool>,
        calls: Mutex<u64>,
    }

    impl ConflictBackend {
        fn new(fail_put: Arc<AtomicBool>) -> Self {
            Self {
                inner: InMemoryStorageBackend::new(),
                fail_put,
                calls: Mutex::new(0),
            }
        }
    }

    impl StorageBackend for ConflictBackend {
        fn initialize(&self) -> Result<(), StorageError> {
            self.inner.initialize()
        }

        fn get(&self, namespace: &str, key: &str) -> Result<Option<StorageRecord>, StorageError> {
            self.inner.get(namespace, key)
        }

        fn put(&self, input: StoragePutInput) -> Result<StorageRecord, StorageError> {
            *self.calls.lock().unwrap() += 1;
            if self.fail_put.load(Ordering::SeqCst) {
                return Err(StorageError::Conflict {
                    namespace: input.namespace,
                    key: input.key,
                    expected_revision: input.if_revision.map(|revision| revision.to_string()),
                    actual_revision: Some("externally-advanced".to_string()),
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

        fn get_summary(
            &self,
            namespace: &str,
            key: &str,
        ) -> Result<Option<StorageRecordSummary>, StorageError> {
            self.inner.get_summary(namespace, key)
        }

        fn list_summaries(
            &self,
            namespace: &str,
            options: StorageListOptions,
        ) -> Result<StorageSummaryPage, StorageError> {
            self.inner.list_summaries(namespace, options)
        }

        fn acquire_lease(
            &self,
            name: &str,
            ttl_ms: u64,
        ) -> Result<Option<StorageLease>, StorageError> {
            self.inner.acquire_lease(name, ttl_ms)
        }
    }
}
