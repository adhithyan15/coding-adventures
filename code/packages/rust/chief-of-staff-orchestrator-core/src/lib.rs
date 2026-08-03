//! Transport-independent runnable orchestration for D18 Chief.
//!
//! This application layer preserves the distinction between durable intent and
//! live process authority while remaining keyless, payload-blind, and bounded.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use chief_of_staff_channel_crypto::ChannelId;
use chief_of_staff_channel_endpoints::{
    ChannelDefinition, ChannelDefinitionStore, ChannelEndpointError,
};
use chief_of_staff_host_runtime::PackageKeyring;
use chief_of_staff_process_supervisor::{
    MonotonicClock, ProcessHostSupervisor, ProcessSupervisorConfig, SessionIdSource,
};
use chief_of_staff_service_reconciler::{
    HostSupervisor, ReconcileConfig, ReconcileError, ReconcileReport, ServiceReconciler,
    SupervisorObservation, SupervisorPhase,
};
use chief_of_staff_service_registry::{
    DesiredState, HostEntry, HostName, HostRegistration, LoadedHost, RegistryError, ServiceRegistry,
};
use coding_adventures_x3dh::IdentityKeyPair;
use core::fmt::{self, Display, Formatter};
use std::sync::Arc;
use storage_core::StorageBackend;

/// Exact channel-topology mutation presented to a trust-checking adapter.
#[derive(Clone, Copy, Debug)]
pub enum ChannelWiringRequest<'a> {
    /// Authorize creation of this complete immutable active definition.
    Create(&'a ChannelDefinition),
    /// Authorize irreversible destruction of this current definition.
    Destroy(&'a ChannelDefinition),
}

impl<'a> ChannelWiringRequest<'a> {
    /// Return the channel whose topology would change.
    pub fn channel_id(self) -> ChannelId {
        match self {
            Self::Create(definition) | Self::Destroy(definition) => definition.channel_id(),
        }
    }

    /// Return the complete durable definition presented for authorization.
    pub fn definition(self) -> &'a ChannelDefinition {
        match self {
            Self::Create(definition) | Self::Destroy(definition) => definition,
        }
    }
}

/// Injected privilege and human-approval boundary for channel topology changes.
pub trait ChannelWiringAuthorizer {
    /// Concrete authorization failure retained for programmatic handling.
    type Error;

    /// Approve this exact mutation or fail before any storage change.
    fn authorize(&mut self, request: ChannelWiringRequest<'_>) -> Result<(), Self::Error>;
}

/// Durable intent together with a fresh authoritative supervisor observation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostHealth {
    durable: LoadedHost,
    authoritative: SupervisorObservation,
}

impl HostHealth {
    /// Return the durable registry entry and its CAS revision.
    pub fn durable(&self) -> &LoadedHost {
        &self.durable
    }

    /// Return fresh evidence from the owned process supervisor.
    pub fn authoritative(&self) -> &SupervisorObservation {
        &self.authoritative
    }
}

/// Typed failure from one bounded orchestrator-core operation.
#[derive(Debug)]
pub enum OrchestratorCoreError<SupervisorError, AuthorizationError> {
    /// Durable registry storage, validation, or CAS failure.
    Registry(RegistryError),
    /// One deterministic reconciliation tick failed.
    Reconciliation(ReconcileError<SupervisorError>),
    /// A direct authoritative health inspection failed.
    Supervisor {
        /// Host whose authority could not be inspected.
        host_name: HostName,
        /// Original injected supervisor failure.
        source: SupervisorError,
    },
    /// Durable channel topology validation, storage, or CAS failure.
    Channel(ChannelEndpointError),
    /// The injected trust boundary denied or failed a topology mutation.
    Authorization(AuthorizationError),
    /// Deregistration was requested before durable intent became stopped.
    HostDesiredRunning(HostName),
    /// Deregistration was requested while supervisor authority remained active.
    HostStillActive(HostName),
    /// The injected monotonic clock moved earlier than a successful prior tick.
    ClockRegressed,
}

impl<S, A> Display for OrchestratorCoreError<S, A> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Registry(_) => formatter.write_str("orchestrator-core: registry failure"),
            Self::Reconciliation(_) => {
                formatter.write_str("orchestrator-core: reconciliation failure")
            }
            Self::Supervisor { host_name, .. } => {
                write!(
                    formatter,
                    "orchestrator-core: supervisor failure for {host_name}"
                )
            }
            Self::Channel(_) => formatter.write_str("orchestrator-core: channel failure"),
            Self::Authorization(_) => {
                formatter.write_str("orchestrator-core: channel authorization failed")
            }
            Self::HostDesiredRunning(host_name) => write!(
                formatter,
                "orchestrator-core: host still desires running: {host_name}"
            ),
            Self::HostStillActive(host_name) => {
                write!(
                    formatter,
                    "orchestrator-core: host is still active: {host_name}"
                )
            }
            Self::ClockRegressed => formatter.write_str("orchestrator-core: clock regressed"),
        }
    }
}

impl<S, A> std::error::Error for OrchestratorCoreError<S, A>
where
    S: std::error::Error + 'static,
    A: std::error::Error + 'static,
{
}

/// Bounded application core parameterized by process and authorization adapters.
pub struct OrchestratorCore<S, A> {
    backend: Arc<dyn StorageBackend>,
    reconcile_config: ReconcileConfig,
    supervisor: S,
    authorizer: A,
    clock: Arc<dyn MonotonicClock>,
    last_reconcile_ns: Option<u64>,
}

impl<S, A> OrchestratorCore<S, A> {
    /// Bind storage, reconciliation, process authority, authorization, and time.
    pub fn new(
        backend: Arc<dyn StorageBackend>,
        supervisor: S,
        authorizer: A,
        clock: Arc<dyn MonotonicClock>,
        reconcile_config: ReconcileConfig,
    ) -> Self {
        Self {
            backend,
            reconcile_config,
            supervisor,
            authorizer,
            clock,
            last_reconcile_ns: None,
        }
    }

    /// Return the time of the last successful reconciliation tick.
    pub fn last_reconcile_ns(&self) -> Option<u64> {
        self.last_reconcile_ns
    }

    /// Consume the core and return its supervisor for orderly outer shutdown.
    pub fn into_supervisor(self) -> S {
        self.supervisor
    }
}

impl<S, A> OrchestratorCore<S, A>
where
    S: HostSupervisor,
    A: ChannelWiringAuthorizer,
{
    /// Idempotently register immutable package identity and initial desired state.
    pub fn register_host(
        &self,
        registration: HostRegistration,
        desired_state: DesiredState,
    ) -> Result<LoadedHost, OrchestratorCoreError<S::Error, A::Error>> {
        ServiceRegistry::new(self.backend.as_ref())
            .register(&HostEntry::registered(registration, desired_state))
            .map_err(OrchestratorCoreError::Registry)
    }

    /// Load one durable host without treating its observation as live authority.
    pub fn load_host(
        &self,
        host_name: &HostName,
    ) -> Result<Option<LoadedHost>, OrchestratorCoreError<S::Error, A::Error>> {
        ServiceRegistry::new(self.backend.as_ref())
            .load(host_name)
            .map_err(OrchestratorCoreError::Registry)
    }

    /// List durable hosts in stable host-name order.
    pub fn list_hosts(&self) -> Result<Vec<LoadedHost>, OrchestratorCoreError<S::Error, A::Error>> {
        ServiceRegistry::new(self.backend.as_ref())
            .list()
            .map_err(OrchestratorCoreError::Registry)
    }

    /// CAS-update only the desired lifecycle state of an existing registration.
    pub fn set_desired_state(
        &self,
        host_name: &HostName,
        desired_state: DesiredState,
    ) -> Result<LoadedHost, OrchestratorCoreError<S::Error, A::Error>> {
        let registry = ServiceRegistry::new(self.backend.as_ref());
        let loaded = registry
            .load(host_name)
            .map_err(OrchestratorCoreError::Registry)?
            .ok_or_else(|| {
                OrchestratorCoreError::Registry(RegistryError::HostNotFound(host_name.clone()))
            })?;
        if loaded.entry().desired_state() == desired_state {
            return Ok(loaded);
        }
        let replacement = loaded.entry().clone().with_desired_state(desired_state);
        registry
            .update(&loaded, &replacement)
            .map_err(OrchestratorCoreError::Registry)
    }

    /// Sample time once and perform one stable-order bounded reconciliation tick.
    pub fn reconcile_once(
        &mut self,
    ) -> Result<ReconcileReport, OrchestratorCoreError<S::Error, A::Error>> {
        let now_ns = self.clock.now_ns();
        if self
            .last_reconcile_ns
            .is_some_and(|previous| now_ns < previous)
        {
            return Err(OrchestratorCoreError::ClockRegressed);
        }
        let report = ServiceReconciler::new(
            ServiceRegistry::new(self.backend.as_ref()),
            self.reconcile_config,
        )
        .reconcile_all(&mut self.supervisor, now_ns)
        .map_err(OrchestratorCoreError::Reconciliation)?;
        self.last_reconcile_ns = Some(now_ns);
        Ok(report)
    }

    /// Return durable intent and a separate fresh supervisor observation.
    pub fn health_check(
        &mut self,
        host_name: &HostName,
    ) -> Result<HostHealth, OrchestratorCoreError<S::Error, A::Error>> {
        let durable = ServiceRegistry::new(self.backend.as_ref())
            .load(host_name)
            .map_err(OrchestratorCoreError::Registry)?
            .ok_or_else(|| {
                OrchestratorCoreError::Registry(RegistryError::HostNotFound(host_name.clone()))
            })?;
        let authoritative = self
            .supervisor
            .inspect(durable.entry().registration())
            .map_err(|source| OrchestratorCoreError::Supervisor {
                host_name: host_name.clone(),
                source,
            })?;
        Ok(HostHealth {
            durable,
            authoritative,
        })
    }

    /// Delete stopped intent only after process authority is absent or reaped.
    pub fn deregister_host(
        &mut self,
        host_name: &HostName,
    ) -> Result<(), OrchestratorCoreError<S::Error, A::Error>> {
        let registry = ServiceRegistry::new(self.backend.as_ref());
        let loaded = registry
            .load(host_name)
            .map_err(OrchestratorCoreError::Registry)?
            .ok_or_else(|| {
                OrchestratorCoreError::Registry(RegistryError::HostNotFound(host_name.clone()))
            })?;
        if loaded.entry().desired_state() != DesiredState::Stopped {
            return Err(OrchestratorCoreError::HostDesiredRunning(host_name.clone()));
        }
        let authority = self
            .supervisor
            .inspect(loaded.entry().registration())
            .map_err(|source| OrchestratorCoreError::Supervisor {
                host_name: host_name.clone(),
                source,
            })?;
        if matches!(
            authority,
            SupervisorObservation::Instance(ref instance)
                if !matches!(instance.phase(), SupervisorPhase::Exited { .. })
        ) {
            return Err(OrchestratorCoreError::HostStillActive(host_name.clone()));
        }
        registry
            .deregister(&loaded)
            .map_err(OrchestratorCoreError::Registry)
    }

    /// Authorize then idempotently create one durable active channel definition.
    pub fn create_channel(
        &mut self,
        definition: &ChannelDefinition,
    ) -> Result<ChannelDefinition, OrchestratorCoreError<S::Error, A::Error>> {
        self.authorizer
            .authorize(ChannelWiringRequest::Create(definition))
            .map_err(OrchestratorCoreError::Authorization)?;
        ChannelDefinitionStore::new(self.backend.as_ref())
            .create(definition)
            .map_err(OrchestratorCoreError::Channel)
    }

    /// Load durable topology without opening a key-bearing endpoint.
    pub fn load_channel(
        &self,
        channel_id: ChannelId,
    ) -> Result<Option<ChannelDefinition>, OrchestratorCoreError<S::Error, A::Error>> {
        ChannelDefinitionStore::new(self.backend.as_ref())
            .load(channel_id)
            .map_err(OrchestratorCoreError::Channel)
    }

    /// Authorize then irreversibly destroy the current durable definition.
    pub fn destroy_channel(
        &mut self,
        channel_id: ChannelId,
    ) -> Result<ChannelDefinition, OrchestratorCoreError<S::Error, A::Error>> {
        let store = ChannelDefinitionStore::new(self.backend.as_ref());
        let definition = store
            .load(channel_id)
            .map_err(OrchestratorCoreError::Channel)?
            .ok_or(OrchestratorCoreError::Channel(
                ChannelEndpointError::DefinitionNotFound,
            ))?;
        self.authorizer
            .authorize(ChannelWiringRequest::Destroy(&definition))
            .map_err(OrchestratorCoreError::Authorization)?;
        store
            .destroy(channel_id)
            .map_err(OrchestratorCoreError::Channel)
    }
}

/// Production process-supervised specialization of the transport-independent core.
pub type ProcessOrchestratorCore<A> = OrchestratorCore<ProcessHostSupervisor, A>;

impl<A> OrchestratorCore<ProcessHostSupervisor, A> {
    /// Compose package trust, X3DH identity, process authority, storage, and time.
    #[allow(clippy::too_many_arguments)]
    pub fn with_process_supervisor(
        backend: Arc<dyn StorageBackend>,
        process_config: ProcessSupervisorConfig,
        keyring: Arc<PackageKeyring>,
        identity: Arc<IdentityKeyPair>,
        clock: Arc<dyn MonotonicClock>,
        sessions: Box<dyn SessionIdSource>,
        reconcile_config: ReconcileConfig,
        authorizer: A,
    ) -> Self {
        let supervisor = ProcessHostSupervisor::new(
            process_config,
            keyring,
            identity,
            Arc::clone(&clock),
            sessions,
        );
        Self::new(backend, supervisor, authorizer, clock, reconcile_config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chief_of_staff_channel_crypto::KeyEpoch;
    use chief_of_staff_channel_endpoints::{
        AgentId, ChannelLifecycle, OriginatorIdentity, ReceiverIdentity,
    };
    use chief_of_staff_process_supervisor::{HostProgram, UuidV7SessionIdSource};
    use chief_of_staff_service_reconciler::{ReconcileAction, SupervisorOperation};
    use chief_of_staff_service_registry::{PackagePath, RestartPolicy};
    use coding_adventures_x3dh::generate_identity_keypair;
    use std::collections::{BTreeMap, VecDeque};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;
    use std::time::Duration;
    use storage_core::InMemoryStorageBackend;

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct FakeError(&'static str);

    impl Display for FakeError {
        fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
            formatter.write_str(self.0)
        }
    }

    impl std::error::Error for FakeError {}

    #[derive(Default)]
    struct SupervisorState {
        observations: BTreeMap<String, SupervisorObservation>,
        starts: Vec<String>,
        stops: Vec<String>,
        fail_inspect: bool,
        fail_start: bool,
    }

    #[derive(Clone, Default)]
    struct FakeSupervisor(Arc<Mutex<SupervisorState>>);

    impl HostSupervisor for FakeSupervisor {
        type Error = FakeError;

        fn inspect(
            &mut self,
            registration: &HostRegistration,
        ) -> Result<SupervisorObservation, Self::Error> {
            let state = self.0.lock().expect("supervisor mutex poisoned");
            if state.fail_inspect {
                return Err(FakeError("inspect failed"));
            }
            Ok(state
                .observations
                .get(registration.host_name().as_str())
                .cloned()
                .unwrap_or_else(SupervisorObservation::absent))
        }

        fn start(&mut self, registration: &HostRegistration) -> Result<(), Self::Error> {
            let mut state = self.0.lock().expect("supervisor mutex poisoned");
            if state.fail_start {
                return Err(FakeError("start failed"));
            }
            state
                .starts
                .push(registration.host_name().as_str().to_string());
            Ok(())
        }

        fn stop(&mut self, host_name: &HostName) -> Result<(), Self::Error> {
            self.0
                .lock()
                .expect("supervisor mutex poisoned")
                .stops
                .push(host_name.as_str().to_string());
            Ok(())
        }
    }

    struct TestClock {
        values: Mutex<VecDeque<u64>>,
        calls: AtomicUsize,
    }

    impl TestClock {
        fn new(values: impl IntoIterator<Item = u64>) -> Self {
            Self {
                values: Mutex::new(values.into_iter().collect()),
                calls: AtomicUsize::new(0),
            }
        }
    }

    impl MonotonicClock for TestClock {
        fn now_ns(&self) -> u64 {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.values
                .lock()
                .expect("clock mutex poisoned")
                .pop_front()
                .expect("test clock exhausted")
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    enum WiringEvent {
        Create(ChannelId),
        Destroy(ChannelId),
    }

    struct FakeAuthorizer {
        events: Arc<Mutex<Vec<WiringEvent>>>,
        allow: bool,
    }

    impl FakeAuthorizer {
        fn allowing(events: Arc<Mutex<Vec<WiringEvent>>>) -> Self {
            Self {
                events,
                allow: true,
            }
        }

        fn denying(events: Arc<Mutex<Vec<WiringEvent>>>) -> Self {
            Self {
                events,
                allow: false,
            }
        }
    }

    impl ChannelWiringAuthorizer for FakeAuthorizer {
        type Error = FakeError;

        fn authorize(&mut self, request: ChannelWiringRequest<'_>) -> Result<(), Self::Error> {
            assert_eq!(request.definition().channel_id(), request.channel_id());
            let event = match request {
                ChannelWiringRequest::Create(_) => WiringEvent::Create(request.channel_id()),
                ChannelWiringRequest::Destroy(_) => WiringEvent::Destroy(request.channel_id()),
            };
            self.events
                .lock()
                .expect("authorization mutex poisoned")
                .push(event);
            if self.allow {
                Ok(())
            } else {
                Err(FakeError("denied"))
            }
        }
    }

    fn name(value: &str) -> HostName {
        HostName::new(value).expect("valid host name")
    }

    fn registration(host_name: &str, hash_byte: u8) -> HostRegistration {
        HostRegistration::new(
            name(host_name),
            PackagePath::new(format!("/packages/{host_name}"))
                .expect("valid portable package path"),
            [hash_byte; 32],
            RestartPolicy::Always,
        )
    }

    fn channel_id(byte: u8) -> ChannelId {
        let mut bytes = [byte; 16];
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        ChannelId(bytes)
    }

    fn definition(byte: u8) -> ChannelDefinition {
        ChannelDefinition::new(
            channel_id(byte),
            OriginatorIdentity {
                agent_id: AgentId::new(b"originator".to_vec()).expect("valid agent"),
                public_key: [1; 32],
            },
            vec![ReceiverIdentity {
                agent_id: AgentId::new(b"receiver".to_vec()).expect("valid agent"),
                public_key: [2; 32],
            }],
            100,
            KeyEpoch(1),
        )
        .expect("valid channel definition")
    }

    fn core(
        backend: Arc<InMemoryStorageBackend>,
        supervisor: FakeSupervisor,
        authorizer: FakeAuthorizer,
        clock: Arc<TestClock>,
    ) -> OrchestratorCore<FakeSupervisor, FakeAuthorizer> {
        OrchestratorCore::new(
            backend,
            supervisor,
            authorizer,
            clock,
            ReconcileConfig::new(100).expect("valid reconcile config"),
        )
    }

    #[test]
    fn host_registration_listing_and_desired_state_are_idempotent() {
        let backend = Arc::new(InMemoryStorageBackend::new());
        let supervisor = FakeSupervisor::default();
        let events = Arc::new(Mutex::new(Vec::new()));
        let core = core(
            Arc::clone(&backend),
            supervisor,
            FakeAuthorizer::allowing(events),
            Arc::new(TestClock::new([])),
        );
        let bravo = registration("bravo-host", 2);
        let alpha = registration("alpha-host", 1);

        let first = core
            .register_host(bravo.clone(), DesiredState::Running)
            .expect("register bravo");
        let repeated = core
            .register_host(bravo, DesiredState::Running)
            .expect("repeat identical registration");
        assert_eq!(first, repeated);
        core.register_host(alpha, DesiredState::Stopped)
            .expect("register alpha");
        assert_eq!(
            core.list_hosts()
                .expect("list hosts")
                .iter()
                .map(|loaded| loaded.entry().registration().host_name().as_str())
                .collect::<Vec<_>>(),
            ["alpha-host", "bravo-host"]
        );

        let unchanged = core
            .set_desired_state(&name("alpha-host"), DesiredState::Stopped)
            .expect("idempotent desired state");
        let changed = core
            .set_desired_state(&name("alpha-host"), DesiredState::Running)
            .expect("update desired state");
        assert_eq!(changed.entry().desired_state(), DesiredState::Running);
        assert_ne!(unchanged.revision(), changed.revision());
        assert_eq!(
            core.load_host(&name("missing-host")).expect("load missing"),
            None
        );
    }

    #[test]
    fn conflicting_registration_and_unknown_host_updates_are_typed() {
        let backend = Arc::new(InMemoryStorageBackend::new());
        let events = Arc::new(Mutex::new(Vec::new()));
        let core = core(
            Arc::clone(&backend),
            FakeSupervisor::default(),
            FakeAuthorizer::allowing(events),
            Arc::new(TestClock::new([])),
        );
        core.register_host(registration("agent-host", 1), DesiredState::Running)
            .expect("register host");
        let conflict = core
            .register_host(registration("agent-host", 2), DesiredState::Running)
            .expect_err("different package must conflict");
        assert!(matches!(
            conflict,
            OrchestratorCoreError::Registry(RegistryError::ConflictingRegistration(_))
        ));
        let missing = core
            .set_desired_state(&name("missing-host"), DesiredState::Stopped)
            .expect_err("unknown host must fail");
        assert!(matches!(
            missing,
            OrchestratorCoreError::Registry(RegistryError::HostNotFound(_))
        ));
    }

    #[test]
    fn reconciliation_samples_time_once_and_tracks_only_success() {
        let backend = Arc::new(InMemoryStorageBackend::new());
        let supervisor = FakeSupervisor::default();
        let supervisor_state = Arc::clone(&supervisor.0);
        let clock = Arc::new(TestClock::new([1_000, 1_001, 1_002]));
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut core = core(
            Arc::clone(&backend),
            supervisor,
            FakeAuthorizer::allowing(events),
            Arc::clone(&clock),
        );
        core.register_host(registration("agent-host", 1), DesiredState::Running)
            .expect("register host");

        let report = core.reconcile_once().expect("start tick");
        assert_eq!(report.outcomes()[0].action(), ReconcileAction::Started);
        assert_eq!(core.last_reconcile_ns(), Some(1_000));
        assert_eq!(clock.calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            supervisor_state
                .lock()
                .expect("supervisor mutex poisoned")
                .starts,
            ["agent-host"]
        );

        supervisor_state
            .lock()
            .expect("supervisor mutex poisoned")
            .fail_inspect = true;
        let error = core.reconcile_once().expect_err("inspection must fail");
        assert!(matches!(
            error,
            OrchestratorCoreError::Reconciliation(ReconcileError::Supervisor {
                operation: SupervisorOperation::Inspect,
                ..
            })
        ));
        assert_eq!(core.last_reconcile_ns(), Some(1_000));

        supervisor_state
            .lock()
            .expect("supervisor mutex poisoned")
            .fail_inspect = false;
        core.reconcile_once().expect("later successful tick");
        assert_eq!(core.last_reconcile_ns(), Some(1_002));
        assert_eq!(clock.calls.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn reconciliation_drives_start_observe_stop_and_restart() {
        let backend = Arc::new(InMemoryStorageBackend::new());
        let supervisor = FakeSupervisor::default();
        let state = Arc::clone(&supervisor.0);
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut core = core(
            Arc::clone(&backend),
            supervisor,
            FakeAuthorizer::allowing(events),
            Arc::new(TestClock::new([100, 110, 120, 130])),
        );
        let registration = registration("agent-host", 8);
        core.register_host(registration.clone(), DesiredState::Running)
            .expect("register host");

        assert_eq!(
            core.reconcile_once().expect("start tick").outcomes()[0].action(),
            ReconcileAction::Started
        );
        state
            .lock()
            .expect("supervisor mutex poisoned")
            .observations
            .insert(
                "agent-host".to_string(),
                SupervisorObservation::running([8; 32], 17, 101, 109, channel_id(8))
                    .expect("valid running observation"),
            );
        assert_eq!(
            core.reconcile_once().expect("observation tick").outcomes()[0].action(),
            ReconcileAction::Observed
        );

        core.set_desired_state(registration.host_name(), DesiredState::Stopped)
            .expect("request stop");
        assert_eq!(
            core.reconcile_once().expect("stop tick").outcomes()[0].action(),
            ReconcileAction::Stopped
        );
        assert_eq!(
            state.lock().expect("supervisor mutex poisoned").stops,
            ["agent-host"]
        );

        state
            .lock()
            .expect("supervisor mutex poisoned")
            .observations
            .insert(
                "agent-host".to_string(),
                SupervisorObservation::exited([8; 32], Some(0), Some(101), Some(109))
                    .expect("valid exited observation"),
            );
        core.set_desired_state(registration.host_name(), DesiredState::Running)
            .expect("request restart");
        assert_eq!(
            core.reconcile_once().expect("restart tick").outcomes()[0].action(),
            ReconcileAction::Started
        );
        assert_eq!(
            state.lock().expect("supervisor mutex poisoned").starts,
            ["agent-host", "agent-host"]
        );
    }

    #[test]
    fn reconciliation_rejects_clock_regression_before_supervisor_io() {
        let backend = Arc::new(InMemoryStorageBackend::new());
        let supervisor = FakeSupervisor::default();
        let state = Arc::clone(&supervisor.0);
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut core = core(
            Arc::clone(&backend),
            supervisor,
            FakeAuthorizer::allowing(events),
            Arc::new(TestClock::new([20, 19])),
        );
        core.reconcile_once().expect("empty first tick");
        assert!(matches!(
            core.reconcile_once(),
            Err(OrchestratorCoreError::ClockRegressed)
        ));
        assert!(state
            .lock()
            .expect("supervisor mutex poisoned")
            .starts
            .is_empty());
        assert_eq!(core.last_reconcile_ns(), Some(20));
    }

    #[test]
    fn health_keeps_cached_state_separate_from_fresh_authority() {
        let backend = Arc::new(InMemoryStorageBackend::new());
        let supervisor = FakeSupervisor::default();
        let state = Arc::clone(&supervisor.0);
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut core = core(
            Arc::clone(&backend),
            supervisor,
            FakeAuthorizer::allowing(events),
            Arc::new(TestClock::new([])),
        );
        let registration = registration("agent-host", 7);
        core.register_host(registration.clone(), DesiredState::Running)
            .expect("register host");
        let running = SupervisorObservation::running([7; 32], 41, 10, 11, channel_id(9))
            .expect("valid running observation");
        state
            .lock()
            .expect("supervisor mutex poisoned")
            .observations
            .insert("agent-host".to_string(), running.clone());

        let health = core.health_check(registration.host_name()).expect("health");
        assert_eq!(health.authoritative(), &running);
        assert_eq!(
            health.durable().entry().observation().status(),
            &chief_of_staff_service_registry::HostStatus::Stopped
        );

        state
            .lock()
            .expect("supervisor mutex poisoned")
            .fail_inspect = true;
        assert!(matches!(
            core.health_check(registration.host_name()),
            Err(OrchestratorCoreError::Supervisor { .. })
        ));
        assert!(matches!(
            core.health_check(&name("missing-host")),
            Err(OrchestratorCoreError::Registry(
                RegistryError::HostNotFound(_)
            ))
        ));
    }

    #[test]
    fn deregistration_requires_stopped_intent_and_absent_or_exited_authority() {
        let backend = Arc::new(InMemoryStorageBackend::new());
        let supervisor = FakeSupervisor::default();
        let state = Arc::clone(&supervisor.0);
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut core = core(
            Arc::clone(&backend),
            supervisor,
            FakeAuthorizer::allowing(events),
            Arc::new(TestClock::new([])),
        );
        let registration = registration("agent-host", 3);
        core.register_host(registration.clone(), DesiredState::Running)
            .expect("register host");
        assert!(matches!(
            core.deregister_host(registration.host_name()),
            Err(OrchestratorCoreError::HostDesiredRunning(_))
        ));

        core.set_desired_state(registration.host_name(), DesiredState::Stopped)
            .expect("stop desired state");
        state
            .lock()
            .expect("supervisor mutex poisoned")
            .observations
            .insert(
                "agent-host".to_string(),
                SupervisorObservation::starting([3; 32], 7, 1, None, None)
                    .expect("valid starting observation"),
            );
        assert!(matches!(
            core.deregister_host(registration.host_name()),
            Err(OrchestratorCoreError::HostStillActive(_))
        ));

        state
            .lock()
            .expect("supervisor mutex poisoned")
            .observations
            .insert(
                "agent-host".to_string(),
                SupervisorObservation::exited([3; 32], Some(0), Some(1), Some(2))
                    .expect("valid exited observation"),
            );
        core.deregister_host(registration.host_name())
            .expect("exited host may be removed");
        assert_eq!(
            core.load_host(registration.host_name()).expect("load"),
            None
        );
        assert!(matches!(
            core.deregister_host(registration.host_name()),
            Err(OrchestratorCoreError::Registry(
                RegistryError::HostNotFound(_)
            ))
        ));
    }

    #[test]
    fn channel_mutations_are_authorized_before_storage_changes() {
        let backend = Arc::new(InMemoryStorageBackend::new());
        let supervisor = FakeSupervisor::default();
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut core = core(
            Arc::clone(&backend),
            supervisor,
            FakeAuthorizer::allowing(Arc::clone(&events)),
            Arc::new(TestClock::new([])),
        );
        let definition = definition(4);

        assert_eq!(
            core.create_channel(&definition).expect("create channel"),
            definition
        );
        assert_eq!(
            core.create_channel(&definition)
                .expect("idempotent channel creation"),
            definition
        );
        assert_eq!(
            core.load_channel(definition.channel_id())
                .expect("load channel"),
            Some(definition.clone())
        );
        let destroyed = core
            .destroy_channel(definition.channel_id())
            .expect("destroy channel");
        assert_eq!(destroyed.lifecycle(), ChannelLifecycle::Destroyed);
        assert_eq!(
            events
                .lock()
                .expect("authorization mutex poisoned")
                .as_slice(),
            [
                WiringEvent::Create(definition.channel_id()),
                WiringEvent::Create(definition.channel_id()),
                WiringEvent::Destroy(definition.channel_id())
            ]
        );
        assert_eq!(
            core.destroy_channel(definition.channel_id())
                .expect("destroying an already destroyed channel is idempotent"),
            destroyed
        );
        assert_eq!(
            events.lock().expect("authorization mutex poisoned").last(),
            Some(&WiringEvent::Destroy(definition.channel_id()))
        );
    }

    #[test]
    fn denied_channel_mutations_leave_storage_unchanged() {
        let backend = Arc::new(InMemoryStorageBackend::new());
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut core = core(
            Arc::clone(&backend),
            FakeSupervisor::default(),
            FakeAuthorizer::denying(Arc::clone(&events)),
            Arc::new(TestClock::new([])),
        );
        let definition = definition(5);
        assert!(matches!(
            core.create_channel(&definition),
            Err(OrchestratorCoreError::Authorization(FakeError("denied")))
        ));
        assert_eq!(
            core.load_channel(definition.channel_id())
                .expect("load denied channel"),
            None
        );

        ChannelDefinitionStore::new(backend.as_ref())
            .create(&definition)
            .expect("seed definition");
        assert!(matches!(
            core.destroy_channel(definition.channel_id()),
            Err(OrchestratorCoreError::Authorization(FakeError("denied")))
        ));
        assert_eq!(
            core.load_channel(definition.channel_id())
                .expect("load preserved channel"),
            Some(definition.clone())
        );
        assert_eq!(
            events
                .lock()
                .expect("authorization mutex poisoned")
                .as_slice(),
            [
                WiringEvent::Create(definition.channel_id()),
                WiringEvent::Destroy(definition.channel_id())
            ]
        );
    }

    #[test]
    fn stable_errors_do_not_render_nested_payloads() {
        let host = name("agent-host");
        let errors: Vec<OrchestratorCoreError<FakeError, FakeError>> = vec![
            OrchestratorCoreError::Registry(RegistryError::HostNotFound(host.clone())),
            OrchestratorCoreError::Supervisor {
                host_name: host.clone(),
                source: FakeError("secret supervisor payload"),
            },
            OrchestratorCoreError::Channel(ChannelEndpointError::DefinitionNotFound),
            OrchestratorCoreError::Authorization(FakeError("secret authorization payload")),
            OrchestratorCoreError::HostDesiredRunning(host.clone()),
            OrchestratorCoreError::HostStillActive(host),
            OrchestratorCoreError::ClockRegressed,
        ];
        let rendered = errors.iter().map(ToString::to_string).collect::<Vec<_>>();
        assert!(rendered.iter().all(|message| !message.contains("secret")));
        assert!(rendered
            .iter()
            .all(|message| message.starts_with("orchestrator-core:")));
    }

    #[test]
    fn production_constructor_composes_an_empty_process_supervised_core() {
        let backend = Arc::new(InMemoryStorageBackend::new());
        let keyring = Arc::new(PackageKeyring::new());
        let identity = Arc::new(generate_identity_keypair());
        let executable = std::env::current_exe().expect("current executable");
        let config = ProcessSupervisorConfig::new(
            HostProgram::new(executable, std::iter::empty::<&str>()).expect("host program"),
            Duration::from_secs(1),
            Duration::from_secs(1),
        )
        .expect("process configuration");
        let clock = Arc::new(TestClock::new([1]));
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut core = ProcessOrchestratorCore::with_process_supervisor(
            backend,
            config,
            keyring,
            identity,
            clock,
            Box::<UuidV7SessionIdSource>::default(),
            ReconcileConfig::new(100).expect("reconcile config"),
            FakeAuthorizer::allowing(events),
        );
        fn assert_send_static<T: Send + 'static>(_: &T) {}
        assert_send_static(&core);
        assert!(core
            .reconcile_once()
            .expect("empty production tick")
            .outcomes()
            .is_empty());
        drop(core.into_supervisor());
    }
}
