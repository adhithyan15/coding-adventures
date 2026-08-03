//! Deterministic D18 service-registry reconciliation.
//!
//! Cached registry observations are durable recovery hints, not process
//! authority. This crate accepts fresh evidence from an injected supervisor,
//! converges one bounded tick, and performs no I/O of its own.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use chief_of_staff_channel_crypto::ChannelId;
use chief_of_staff_service_registry::{
    DesiredState, HostEntry, HostName, HostObservation, HostRegistration, HostStatus, LoadedHost,
    RegistryError, RestartPolicy, ServiceRegistry,
};
use core::fmt::{self, Display, Formatter};

const RESTART_COUNTER_EXHAUSTED: &str = "restart counter exhausted";

/// Validated lifecycle phase reported by the process authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SupervisorPhase {
    /// The process exists but has not completed its secure bootstrap.
    Starting,
    /// The process has a live authenticated control channel and heartbeat.
    Running,
    /// The supervisor is draining or terminating the process.
    Stopping,
    /// The process exited and no longer has a live PID or channel.
    Exited {
        /// Process exit code, or `None` for a signal or unavailable status.
        exit_code: Option<i32>,
    },
}

/// A current, structurally validated observation from process authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SupervisorObservation {
    /// No owned process instance exists for the registered host name.
    Absent,
    /// An owned instance exists and is bound to the supplied package hash.
    Instance(SupervisorInstance),
}

impl SupervisorObservation {
    /// Construct an absent observation.
    pub fn absent() -> Self {
        Self::Absent
    }

    /// Construct a starting instance.
    pub fn starting(
        package_hash: [u8; 32],
        process_id: u32,
        started_at_ns: u64,
        last_heartbeat_ns: Option<u64>,
        control_channel_id: Option<ChannelId>,
    ) -> Result<Self, ObservationError> {
        SupervisorInstance::new(
            package_hash,
            SupervisorPhase::Starting,
            Some(process_id),
            Some(started_at_ns),
            last_heartbeat_ns,
            control_channel_id,
        )
        .map(Self::Instance)
    }

    /// Construct a running instance.
    pub fn running(
        package_hash: [u8; 32],
        process_id: u32,
        started_at_ns: u64,
        last_heartbeat_ns: u64,
        control_channel_id: ChannelId,
    ) -> Result<Self, ObservationError> {
        SupervisorInstance::new(
            package_hash,
            SupervisorPhase::Running,
            Some(process_id),
            Some(started_at_ns),
            Some(last_heartbeat_ns),
            Some(control_channel_id),
        )
        .map(Self::Instance)
    }

    /// Construct a stopping instance.
    pub fn stopping(
        package_hash: [u8; 32],
        process_id: u32,
        started_at_ns: u64,
        last_heartbeat_ns: Option<u64>,
        control_channel_id: Option<ChannelId>,
    ) -> Result<Self, ObservationError> {
        SupervisorInstance::new(
            package_hash,
            SupervisorPhase::Stopping,
            Some(process_id),
            Some(started_at_ns),
            last_heartbeat_ns,
            control_channel_id,
        )
        .map(Self::Instance)
    }

    /// Construct an exited instance without retaining live authority fields.
    pub fn exited(
        package_hash: [u8; 32],
        exit_code: Option<i32>,
        started_at_ns: Option<u64>,
        last_heartbeat_ns: Option<u64>,
    ) -> Result<Self, ObservationError> {
        SupervisorInstance::new(
            package_hash,
            SupervisorPhase::Exited { exit_code },
            None,
            started_at_ns,
            last_heartbeat_ns,
            None,
        )
        .map(Self::Instance)
    }
}

/// One owned process instance and its package identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SupervisorInstance {
    package_hash: [u8; 32],
    phase: SupervisorPhase,
    process_id: Option<u32>,
    started_at_ns: Option<u64>,
    last_heartbeat_ns: Option<u64>,
    control_channel_id: Option<ChannelId>,
}

impl SupervisorInstance {
    fn new(
        package_hash: [u8; 32],
        phase: SupervisorPhase,
        process_id: Option<u32>,
        started_at_ns: Option<u64>,
        last_heartbeat_ns: Option<u64>,
        control_channel_id: Option<ChannelId>,
    ) -> Result<Self, ObservationError> {
        let status = match phase {
            SupervisorPhase::Starting => HostStatus::Starting,
            SupervisorPhase::Running => HostStatus::Running,
            SupervisorPhase::Stopping => HostStatus::Stopping,
            SupervisorPhase::Exited { exit_code } => HostStatus::Crashed { exit_code },
        };
        HostObservation::new(
            status,
            process_id,
            started_at_ns,
            last_heartbeat_ns,
            control_channel_id,
            0,
            None,
        )
        .map_err(ObservationError::RegistryValidation)?;
        Ok(Self {
            package_hash,
            phase,
            process_id,
            started_at_ns,
            last_heartbeat_ns,
            control_channel_id,
        })
    }

    /// Return the exact package hash used to launch this process.
    pub fn package_hash(&self) -> &[u8; 32] {
        &self.package_hash
    }

    /// Return the current authoritative lifecycle phase.
    pub fn phase(&self) -> SupervisorPhase {
        self.phase
    }

    /// Return the live process ID, when active.
    pub fn process_id(&self) -> Option<u32> {
        self.process_id
    }

    /// Return the supervisor-observed monotonic start time.
    pub fn started_at_ns(&self) -> Option<u64> {
        self.started_at_ns
    }

    /// Return the last authenticated heartbeat time.
    pub fn last_heartbeat_ns(&self) -> Option<u64> {
        self.last_heartbeat_ns
    }

    /// Return the current authenticated control-channel identifier.
    pub fn control_channel_id(&self) -> Option<ChannelId> {
        self.control_channel_id
    }
}

/// Structural failure while constructing supervisor evidence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ObservationError {
    /// The shape violates the durable registry observation contract.
    RegistryValidation(RegistryError),
}

impl Display for ObservationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::RegistryValidation(error) => write!(formatter, "invalid observation: {error}"),
        }
    }
}

impl std::error::Error for ObservationError {}

/// Process authority injected by the runnable orchestrator.
pub trait HostSupervisor {
    /// Concrete supervisor failure type.
    type Error;

    /// Inspect current authority for this exact registration.
    fn inspect(
        &mut self,
        registration: &HostRegistration,
    ) -> Result<SupervisorObservation, Self::Error>;

    /// Idempotently begin one verified host launch.
    fn start(&mut self, registration: &HostRegistration) -> Result<(), Self::Error>;

    /// Idempotently begin stopping the named host.
    fn stop(&mut self, host_name: &HostName) -> Result<(), Self::Error>;
}

/// Validated health configuration for one reconciliation tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReconcileConfig {
    max_heartbeat_age_ns: u64,
}

impl ReconcileConfig {
    /// Require a non-zero maximum heartbeat age.
    pub fn new(max_heartbeat_age_ns: u64) -> Result<Self, ConfigError> {
        if max_heartbeat_age_ns == 0 {
            return Err(ConfigError::ZeroHeartbeatAge);
        }
        Ok(Self {
            max_heartbeat_age_ns,
        })
    }

    /// Return the maximum accepted heartbeat age.
    pub fn max_heartbeat_age_ns(self) -> u64 {
        self.max_heartbeat_age_ns
    }
}

/// Invalid reconciliation configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfigError {
    /// A zero heartbeat age would make every non-future heartbeat ambiguous.
    ZeroHeartbeatAge,
}

impl Display for ConfigError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("maximum heartbeat age must be non-zero")
    }
}

impl std::error::Error for ConfigError {}

/// Stable action performed for one registry entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReconcileAction {
    /// Only the durable observation was refreshed or already matched.
    Observed,
    /// One start mutation was claimed and issued.
    Started,
    /// One stop mutation was claimed and issued.
    Stopped,
    /// Restart policy or quarantine intentionally suppressed a mutation.
    Deferred,
}

/// Outcome for one host in a full reconciliation tick.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostReconcileOutcome {
    host_name: HostName,
    action: ReconcileAction,
    status: HostStatus,
}

impl HostReconcileOutcome {
    fn new(host_name: HostName, action: ReconcileAction, status: HostStatus) -> Self {
        Self {
            host_name,
            action,
            status,
        }
    }

    /// Return the reconciled host name.
    pub fn host_name(&self) -> &HostName {
        &self.host_name
    }

    /// Return the bounded action performed during this tick.
    pub fn action(&self) -> ReconcileAction {
        self.action
    }

    /// Return the durable status after the tick.
    pub fn status(&self) -> &HostStatus {
        &self.status
    }
}

/// Stable host-name-ordered outcomes for one full registry pass.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReconcileReport {
    outcomes: Vec<HostReconcileOutcome>,
}

impl ReconcileReport {
    /// Borrow every per-host outcome in registry order.
    pub fn outcomes(&self) -> &[HostReconcileOutcome] {
        &self.outcomes
    }
}

/// Operation name attached to a supervisor failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SupervisorOperation {
    /// Inspect live process authority.
    Inspect,
    /// Begin a host launch.
    Start,
    /// Begin host termination.
    Stop,
}

impl Display for SupervisorOperation {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Inspect => formatter.write_str("inspect"),
            Self::Start => formatter.write_str("start"),
            Self::Stop => formatter.write_str("stop"),
        }
    }
}

/// Explicit reconciliation failure.
#[derive(Debug)]
pub enum ReconcileError<E> {
    /// Durable registry read or CAS failure.
    Registry(RegistryError),
    /// Current supervisor evidence contains a future timestamp.
    FutureObservation {
        /// Host whose observation is later than the caller-provided clock.
        host_name: HostName,
    },
    /// An injected supervisor operation failed.
    Supervisor {
        /// Host whose operation failed.
        host_name: HostName,
        /// Failed operation.
        operation: SupervisorOperation,
        /// Original supervisor error.
        source: E,
    },
}

impl<E: Display> Display for ReconcileError<E> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Registry(error) => write!(formatter, "registry reconciliation failed: {error}"),
            Self::FutureObservation { host_name } => {
                write!(
                    formatter,
                    "supervisor observation is in the future: {host_name}"
                )
            }
            Self::Supervisor {
                host_name,
                operation,
                source,
            } => write!(
                formatter,
                "supervisor {operation} failed for {host_name}: {source}"
            ),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for ReconcileError<E> {}

impl<E> From<RegistryError> for ReconcileError<E> {
    fn from(error: RegistryError) -> Self {
        Self::Registry(error)
    }
}

/// Stateless registry-to-supervisor reconciliation kernel.
pub struct ServiceReconciler<'a> {
    registry: ServiceRegistry<'a>,
    config: ReconcileConfig,
}

impl<'a> ServiceReconciler<'a> {
    /// Bind a registry handle and validated health configuration.
    pub fn new(registry: ServiceRegistry<'a>, config: ReconcileConfig) -> Self {
        Self { registry, config }
    }

    /// Reconcile every registered host in stable host-name order.
    pub fn reconcile_all<S: HostSupervisor>(
        &self,
        supervisor: &mut S,
        now_ns: u64,
    ) -> Result<ReconcileReport, ReconcileError<S::Error>> {
        let loaded = self.registry.list()?;
        let mut outcomes = Vec::with_capacity(loaded.len());
        for host in loaded {
            outcomes.push(self.reconcile_one(supervisor, host, now_ns)?);
        }
        Ok(ReconcileReport { outcomes })
    }

    fn reconcile_one<S: HostSupervisor>(
        &self,
        supervisor: &mut S,
        loaded: LoadedHost,
        now_ns: u64,
    ) -> Result<HostReconcileOutcome, ReconcileError<S::Error>> {
        let registration = loaded.entry().registration();
        let host_name = registration.host_name().clone();
        let observed =
            supervisor
                .inspect(registration)
                .map_err(|source| ReconcileError::Supervisor {
                    host_name: host_name.clone(),
                    operation: SupervisorOperation::Inspect,
                    source,
                })?;
        validate_time(&host_name, &observed, now_ns)?;
        match loaded.entry().desired_state() {
            DesiredState::Stopped => self.reconcile_stopped(supervisor, loaded, observed, now_ns),
            DesiredState::Running => self.reconcile_running(supervisor, loaded, observed, now_ns),
        }
    }

    fn reconcile_stopped<S: HostSupervisor>(
        &self,
        supervisor: &mut S,
        loaded: LoadedHost,
        observed: SupervisorObservation,
        _now_ns: u64,
    ) -> Result<HostReconcileOutcome, ReconcileError<S::Error>> {
        match observed {
            SupervisorObservation::Absent => self.persist_status(
                &loaded,
                inactive_observation(loaded.entry().observation(), HostStatus::Stopped, None)?,
                ReconcileAction::Observed,
            ),
            SupervisorObservation::Instance(instance)
                if matches!(instance.phase(), SupervisorPhase::Exited { .. }) =>
            {
                self.persist_status(
                    &loaded,
                    inactive_observation(
                        loaded.entry().observation(),
                        HostStatus::Stopped,
                        Some(&instance),
                    )?,
                    ReconcileAction::Observed,
                )
            }
            SupervisorObservation::Instance(instance) => {
                self.stop_instance(supervisor, loaded, &instance, HostStatus::Stopping)
            }
        }
    }

    fn reconcile_running<S: HostSupervisor>(
        &self,
        supervisor: &mut S,
        loaded: LoadedHost,
        observed: SupervisorObservation,
        now_ns: u64,
    ) -> Result<HostReconcileOutcome, ReconcileError<S::Error>> {
        if let HostStatus::Quarantined { until_ns, reason } = loaded.entry().observation().status()
        {
            if now_ns < *until_ns {
                let status = HostStatus::Quarantined {
                    until_ns: *until_ns,
                    reason: reason.clone(),
                };
                return match observed {
                    SupervisorObservation::Instance(instance)
                        if !matches!(instance.phase(), SupervisorPhase::Exited { .. }) =>
                    {
                        self.stop_instance(supervisor, loaded, &instance, status)
                    }
                    _ => self.persist_status(
                        &loaded,
                        inactive_observation(loaded.entry().observation(), status, None)?,
                        ReconcileAction::Deferred,
                    ),
                };
            }
        }
        match observed {
            SupervisorObservation::Absent => {
                if should_start_absent(loaded.entry(), now_ns) {
                    self.start_instance(supervisor, loaded, now_ns, false)
                } else {
                    let status = deferred_status(loaded.entry().observation(), now_ns);
                    self.persist_status(
                        &loaded,
                        inactive_observation(loaded.entry().observation(), status, None)?,
                        ReconcileAction::Deferred,
                    )
                }
            }
            SupervisorObservation::Instance(instance)
                if instance.package_hash() != loaded.entry().registration().package_hash() =>
            {
                if matches!(instance.phase(), SupervisorPhase::Exited { .. }) {
                    self.start_instance(supervisor, loaded, now_ns, true)
                } else {
                    self.stop_instance(supervisor, loaded, &instance, HostStatus::Restarting)
                }
            }
            SupervisorObservation::Instance(instance) => match instance.phase() {
                SupervisorPhase::Starting => self.persist_instance(
                    &loaded,
                    &instance,
                    HostStatus::Starting,
                    ReconcileAction::Observed,
                ),
                SupervisorPhase::Stopping => self.persist_instance(
                    &loaded,
                    &instance,
                    HostStatus::Stopping,
                    ReconcileAction::Deferred,
                ),
                SupervisorPhase::Running => {
                    let heartbeat = instance
                        .last_heartbeat_ns()
                        .expect("running observation validation requires a heartbeat");
                    if now_ns - heartbeat > self.config.max_heartbeat_age_ns {
                        if restart_after_failure(loaded.entry().registration().restart_policy()) {
                            self.stop_instance(
                                supervisor,
                                loaded,
                                &instance,
                                HostStatus::Restarting,
                            )
                        } else {
                            self.stop_instance(supervisor, loaded, &instance, HostStatus::Stopping)
                        }
                    } else {
                        self.persist_instance(
                            &loaded,
                            &instance,
                            HostStatus::Running,
                            ReconcileAction::Observed,
                        )
                    }
                }
                SupervisorPhase::Exited { exit_code } => {
                    if restart_after_exit(loaded.entry().registration().restart_policy(), exit_code)
                    {
                        self.start_instance(supervisor, loaded, now_ns, true)
                    } else {
                        let status = if exit_code == Some(0) {
                            HostStatus::Stopped
                        } else {
                            HostStatus::Crashed { exit_code }
                        };
                        self.persist_status(
                            &loaded,
                            inactive_observation(
                                loaded.entry().observation(),
                                status,
                                Some(&instance),
                            )?,
                            ReconcileAction::Deferred,
                        )
                    }
                }
            },
        }
    }

    fn start_instance<S: HostSupervisor>(
        &self,
        supervisor: &mut S,
        loaded: LoadedHost,
        now_ns: u64,
        observed_previous_attempt: bool,
    ) -> Result<HostReconcileOutcome, ReconcileError<S::Error>> {
        let previous = loaded.entry().observation();
        let retrying_claim = !observed_previous_attempt
            && (matches!(previous.status(), HostStatus::Starting)
                || matches!(previous.status(), HostStatus::Restarting)
                    && previous.process_id().is_none());
        let restarting = if retrying_claim {
            matches!(previous.status(), HostStatus::Restarting)
        } else {
            observed_previous_attempt || has_previous_attempt(previous)
        };
        let (restart_count, last_restart_ns) = if retrying_claim {
            (previous.restart_count(), previous.last_restart_ns())
        } else if restarting {
            match previous.restart_count().checked_add(1) {
                Some(count) => (count, Some(now_ns)),
                None => {
                    let quarantined = HostObservation::new(
                        HostStatus::Quarantined {
                            until_ns: u64::MAX,
                            reason: RESTART_COUNTER_EXHAUSTED.to_string(),
                        },
                        None,
                        previous.started_at_ns(),
                        previous.last_heartbeat_ns(),
                        None,
                        previous.restart_count(),
                        previous.last_restart_ns(),
                    )?;
                    return self.persist_status(&loaded, quarantined, ReconcileAction::Deferred);
                }
            }
        } else {
            (previous.restart_count(), previous.last_restart_ns())
        };
        let transition = HostObservation::new(
            if restarting {
                HostStatus::Restarting
            } else {
                HostStatus::Starting
            },
            None,
            None,
            None,
            None,
            restart_count,
            last_restart_ns,
        )?;
        let claimed = self.replace_observation(&loaded, transition)?;
        let registration = claimed.entry().registration();
        let host_name = registration.host_name().clone();
        if let Err(source) = supervisor.start(registration) {
            let failed = HostObservation::new(
                HostStatus::Crashed { exit_code: None },
                None,
                None,
                None,
                None,
                restart_count,
                last_restart_ns,
            )
            .expect("reconciler constructs a valid inactive failure");
            let _ = self.replace_observation::<S::Error>(&claimed, failed);
            return Err(ReconcileError::Supervisor {
                host_name,
                operation: SupervisorOperation::Start,
                source,
            });
        }
        Ok(HostReconcileOutcome::new(
            host_name,
            ReconcileAction::Started,
            claimed.entry().observation().status().clone(),
        ))
    }

    fn stop_instance<S: HostSupervisor>(
        &self,
        supervisor: &mut S,
        loaded: LoadedHost,
        instance: &SupervisorInstance,
        transition_status: HostStatus,
    ) -> Result<HostReconcileOutcome, ReconcileError<S::Error>> {
        let preserve_transition_on_failure =
            matches!(transition_status, HostStatus::Quarantined { .. });
        let transition = if preserve_transition_on_failure {
            inactive_observation(loaded.entry().observation(), transition_status, None)?
        } else {
            instance_observation(loaded.entry().observation(), instance, transition_status)?
        };
        let claimed = self.replace_observation(&loaded, transition)?;
        let host_name = claimed.entry().registration().host_name().clone();
        if let Err(source) = supervisor.stop(&host_name) {
            if !preserve_transition_on_failure {
                let restored = phase_observation(loaded.entry().observation(), instance)
                    .expect("validated supervisor evidence maps to a registry observation");
                let _ = self.replace_observation::<S::Error>(&claimed, restored);
            }
            return Err(ReconcileError::Supervisor {
                host_name,
                operation: SupervisorOperation::Stop,
                source,
            });
        }
        Ok(HostReconcileOutcome::new(
            host_name,
            ReconcileAction::Stopped,
            claimed.entry().observation().status().clone(),
        ))
    }

    fn persist_instance<E>(
        &self,
        loaded: &LoadedHost,
        instance: &SupervisorInstance,
        status: HostStatus,
        action: ReconcileAction,
    ) -> Result<HostReconcileOutcome, ReconcileError<E>> {
        self.persist_status(
            loaded,
            instance_observation(loaded.entry().observation(), instance, status)?,
            action,
        )
    }

    fn persist_status<E>(
        &self,
        loaded: &LoadedHost,
        observation: HostObservation,
        action: ReconcileAction,
    ) -> Result<HostReconcileOutcome, ReconcileError<E>> {
        let host_name = loaded.entry().registration().host_name().clone();
        let status = observation.status().clone();
        if loaded.entry().observation() != &observation {
            self.replace_observation(loaded, observation)?;
        }
        Ok(HostReconcileOutcome::new(host_name, action, status))
    }

    fn replace_observation<E>(
        &self,
        loaded: &LoadedHost,
        observation: HostObservation,
    ) -> Result<LoadedHost, ReconcileError<E>> {
        let replacement = loaded.entry().clone().with_observation(observation);
        self.registry
            .update(loaded, &replacement)
            .map_err(ReconcileError::Registry)
    }
}

fn validate_time<E>(
    host_name: &HostName,
    observed: &SupervisorObservation,
    now_ns: u64,
) -> Result<(), ReconcileError<E>> {
    let SupervisorObservation::Instance(instance) = observed else {
        return Ok(());
    };
    if instance.started_at_ns().is_some_and(|time| time > now_ns)
        || instance
            .last_heartbeat_ns()
            .is_some_and(|time| time > now_ns)
    {
        return Err(ReconcileError::FutureObservation {
            host_name: host_name.clone(),
        });
    }
    Ok(())
}

fn should_start_absent(entry: &HostEntry, now_ns: u64) -> bool {
    let observation = entry.observation();
    match observation.status() {
        HostStatus::Starting | HostStatus::Restarting => true,
        HostStatus::Running | HostStatus::Crashed { .. } => {
            restart_after_failure(entry.registration().restart_policy())
        }
        HostStatus::Stopping => entry.registration().restart_policy() != RestartPolicy::Never,
        HostStatus::Stopped => {
            observation.started_at_ns().is_none()
                || entry.registration().restart_policy() == RestartPolicy::Always
        }
        HostStatus::Quarantined { until_ns, .. } => {
            now_ns >= *until_ns && entry.registration().restart_policy() != RestartPolicy::Never
        }
    }
}

fn deferred_status(observation: &HostObservation, now_ns: u64) -> HostStatus {
    match observation.status() {
        HostStatus::Quarantined { until_ns, reason } if now_ns < *until_ns => {
            HostStatus::Quarantined {
                until_ns: *until_ns,
                reason: reason.clone(),
            }
        }
        HostStatus::Crashed { exit_code } => HostStatus::Crashed {
            exit_code: *exit_code,
        },
        HostStatus::Running => HostStatus::Crashed { exit_code: None },
        _ => HostStatus::Stopped,
    }
}

fn restart_after_exit(policy: RestartPolicy, exit_code: Option<i32>) -> bool {
    match policy {
        RestartPolicy::Always => true,
        RestartPolicy::OnFailure => exit_code != Some(0),
        RestartPolicy::Never => false,
    }
}

fn restart_after_failure(policy: RestartPolicy) -> bool {
    matches!(policy, RestartPolicy::Always | RestartPolicy::OnFailure)
}

fn has_previous_attempt(observation: &HostObservation) -> bool {
    observation.started_at_ns().is_some()
        || matches!(
            observation.status(),
            HostStatus::Running
                | HostStatus::Restarting
                | HostStatus::Stopping
                | HostStatus::Crashed { .. }
                | HostStatus::Quarantined { .. }
        )
}

fn phase_observation(
    previous: &HostObservation,
    instance: &SupervisorInstance,
) -> Result<HostObservation, RegistryError> {
    let status = match instance.phase() {
        SupervisorPhase::Starting => HostStatus::Starting,
        SupervisorPhase::Running => HostStatus::Running,
        SupervisorPhase::Stopping => HostStatus::Stopping,
        SupervisorPhase::Exited { exit_code } => {
            if exit_code == Some(0) {
                HostStatus::Stopped
            } else {
                HostStatus::Crashed { exit_code }
            }
        }
    };
    if matches!(instance.phase(), SupervisorPhase::Exited { .. }) {
        inactive_observation(previous, status, Some(instance))
    } else {
        instance_observation(previous, instance, status)
    }
}

fn instance_observation(
    previous: &HostObservation,
    instance: &SupervisorInstance,
    status: HostStatus,
) -> Result<HostObservation, RegistryError> {
    HostObservation::new(
        status,
        instance.process_id(),
        instance.started_at_ns(),
        instance.last_heartbeat_ns(),
        instance.control_channel_id(),
        previous.restart_count(),
        previous.last_restart_ns(),
    )
}

fn inactive_observation(
    previous: &HostObservation,
    status: HostStatus,
    instance: Option<&SupervisorInstance>,
) -> Result<HostObservation, RegistryError> {
    HostObservation::new(
        status,
        None,
        instance
            .and_then(SupervisorInstance::started_at_ns)
            .or(previous.started_at_ns()),
        instance
            .and_then(SupervisorInstance::last_heartbeat_ns)
            .or(previous.last_heartbeat_ns()),
        None,
        previous.restart_count(),
        previous.last_restart_ns(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chief_of_staff_service_registry::{PackagePath, ServiceRegistry};
    use std::collections::BTreeMap;
    use storage_core::InMemoryStorageBackend;

    const NOW: u64 = 1_000;

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct FakeError(&'static str);

    impl Display for FakeError {
        fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
            formatter.write_str(self.0)
        }
    }

    impl std::error::Error for FakeError {}

    #[derive(Default)]
    struct FakeSupervisor {
        observations: BTreeMap<String, SupervisorObservation>,
        starts: Vec<String>,
        stops: Vec<String>,
        fail_inspect: bool,
        fail_start: bool,
        fail_stop: bool,
    }

    impl FakeSupervisor {
        fn with(host: &str, observation: SupervisorObservation) -> Self {
            let mut observations = BTreeMap::new();
            observations.insert(host.to_string(), observation);
            Self {
                observations,
                ..Self::default()
            }
        }
    }

    impl HostSupervisor for FakeSupervisor {
        type Error = FakeError;

        fn inspect(
            &mut self,
            registration: &HostRegistration,
        ) -> Result<SupervisorObservation, Self::Error> {
            if self.fail_inspect {
                return Err(FakeError("inspect failed"));
            }
            Ok(self
                .observations
                .get(registration.host_name().as_str())
                .cloned()
                .unwrap_or(SupervisorObservation::Absent))
        }

        fn start(&mut self, registration: &HostRegistration) -> Result<(), Self::Error> {
            if self.fail_start {
                return Err(FakeError("start failed"));
            }
            self.starts
                .push(registration.host_name().as_str().to_string());
            Ok(())
        }

        fn stop(&mut self, host_name: &HostName) -> Result<(), Self::Error> {
            if self.fail_stop {
                return Err(FakeError("stop failed"));
            }
            self.stops.push(host_name.as_str().to_string());
            Ok(())
        }
    }

    fn channel(byte: u8) -> ChannelId {
        let mut value = [byte; 16];
        value[6] = 0x70;
        value[8] = 0x80;
        ChannelId(value)
    }

    fn name(value: &str) -> HostName {
        HostName::new(value).unwrap()
    }

    fn registration(host: &str, policy: RestartPolicy, hash: u8) -> HostRegistration {
        HostRegistration::new(
            name(host),
            PackagePath::new(format!("agents/{host}.agent")).unwrap(),
            [hash; 32],
            policy,
        )
    }

    fn register(
        backend: &InMemoryStorageBackend,
        host: &str,
        policy: RestartPolicy,
        desired: DesiredState,
        observation: HostObservation,
    ) {
        ServiceRegistry::new(backend)
            .register(&HostEntry::new(
                registration(host, policy, 7),
                desired,
                observation,
            ))
            .unwrap();
    }

    fn reconciler(backend: &InMemoryStorageBackend) -> ServiceReconciler<'_> {
        ServiceReconciler::new(
            ServiceRegistry::new(backend),
            ReconcileConfig::new(100).unwrap(),
        )
    }

    fn load(backend: &InMemoryStorageBackend, host: &str) -> HostEntry {
        ServiceRegistry::new(backend)
            .load(&name(host))
            .unwrap()
            .unwrap()
            .entry()
            .clone()
    }

    fn running_observation(restarts: u32) -> HostObservation {
        HostObservation::new(
            HostStatus::Running,
            Some(42),
            Some(800),
            Some(950),
            Some(channel(1)),
            restarts,
            (restarts > 0).then_some(700),
        )
        .unwrap()
    }

    #[test]
    fn configuration_and_observation_constructors_validate_and_expose_fields() {
        assert_eq!(ReconcileConfig::new(0), Err(ConfigError::ZeroHeartbeatAge));
        assert_eq!(
            ConfigError::ZeroHeartbeatAge.to_string(),
            "maximum heartbeat age must be non-zero"
        );
        let config = ReconcileConfig::new(12).unwrap();
        assert_eq!(config.max_heartbeat_age_ns(), 12);

        let absent = SupervisorObservation::absent();
        assert_eq!(absent, SupervisorObservation::Absent);
        let starting = SupervisorObservation::starting([7; 32], 41, 10, None, None).unwrap();
        let SupervisorObservation::Instance(instance) = starting else {
            panic!("expected instance");
        };
        assert_eq!(instance.package_hash(), &[7; 32]);
        assert_eq!(instance.phase(), SupervisorPhase::Starting);
        assert_eq!(instance.process_id(), Some(41));
        assert_eq!(instance.started_at_ns(), Some(10));
        assert_eq!(instance.last_heartbeat_ns(), None);
        assert_eq!(instance.control_channel_id(), None);

        assert!(SupervisorObservation::starting([7; 32], 0, 10, None, None).is_err());
        let invalid =
            SupervisorObservation::running([7; 32], 1, 10, 11, ChannelId([0; 16])).unwrap_err();
        assert!(invalid.to_string().starts_with("invalid observation:"));
        assert!(SupervisorObservation::exited([7; 32], Some(0), None, Some(1)).is_err());
    }

    #[test]
    fn first_launch_runs_once_for_every_policy_in_stable_order() {
        let backend = InMemoryStorageBackend::new();
        for (host, policy) in [
            ("zeta-host", RestartPolicy::Never),
            ("alpha-host", RestartPolicy::Always),
            ("middle-host", RestartPolicy::OnFailure),
        ] {
            register(
                &backend,
                host,
                policy,
                DesiredState::Running,
                HostObservation::stopped(),
            );
        }
        let mut supervisor = FakeSupervisor::default();
        let report = reconciler(&backend)
            .reconcile_all(&mut supervisor, NOW)
            .unwrap();
        assert_eq!(
            supervisor.starts,
            vec!["alpha-host", "middle-host", "zeta-host"]
        );
        assert_eq!(report.outcomes().len(), 3);
        assert_eq!(report.outcomes()[0].host_name().as_str(), "alpha-host");
        assert_eq!(report.outcomes()[0].action(), ReconcileAction::Started);
        assert_eq!(report.outcomes()[0].status(), &HostStatus::Starting);
    }

    #[test]
    fn clean_exit_restarts_only_always_and_failed_exit_obeys_policy() {
        for (policy, exit_code, should_restart) in [
            (RestartPolicy::Always, Some(0), true),
            (RestartPolicy::OnFailure, Some(0), false),
            (RestartPolicy::Never, Some(0), false),
            (RestartPolicy::Always, Some(9), true),
            (RestartPolicy::OnFailure, Some(9), true),
            (RestartPolicy::Never, Some(9), false),
            (RestartPolicy::OnFailure, None, true),
        ] {
            let backend = InMemoryStorageBackend::new();
            register(
                &backend,
                "mail-host",
                policy,
                DesiredState::Running,
                HostObservation::stopped(),
            );
            let exited =
                SupervisorObservation::exited([7; 32], exit_code, Some(800), Some(900)).unwrap();
            let mut supervisor = FakeSupervisor::with("mail-host", exited);
            let outcome = reconciler(&backend)
                .reconcile_all(&mut supervisor, NOW)
                .unwrap()
                .outcomes()[0]
                .clone();
            assert_eq!(outcome.action() == ReconcileAction::Started, should_restart);
            assert_eq!(!supervisor.starts.is_empty(), should_restart);
            let persisted = load(&backend, "mail-host");
            if should_restart {
                assert_eq!(persisted.observation().status(), &HostStatus::Restarting);
                assert_eq!(persisted.observation().restart_count(), 1);
                assert_eq!(persisted.observation().last_restart_ns(), Some(NOW));
            } else if exit_code == Some(0) {
                assert_eq!(persisted.observation().status(), &HostStatus::Stopped);
            } else {
                assert_eq!(
                    persisted.observation().status(),
                    &HostStatus::Crashed { exit_code }
                );
            }
        }
    }

    #[test]
    fn current_running_evidence_replaces_stale_cache_and_honors_age_boundary() {
        for heartbeat in [900, 901] {
            let backend = InMemoryStorageBackend::new();
            register(
                &backend,
                "mail-host",
                RestartPolicy::OnFailure,
                DesiredState::Running,
                HostObservation::new(
                    HostStatus::Running,
                    Some(999),
                    Some(1),
                    Some(2),
                    Some(channel(2)),
                    2,
                    Some(1),
                )
                .unwrap(),
            );
            let observed =
                SupervisorObservation::running([7; 32], 42, 800, heartbeat, channel(3)).unwrap();
            let mut supervisor = FakeSupervisor::with("mail-host", observed);
            let report = reconciler(&backend)
                .reconcile_all(&mut supervisor, NOW)
                .unwrap();
            assert_eq!(report.outcomes()[0].action(), ReconcileAction::Observed);
            let persisted = load(&backend, "mail-host");
            assert_eq!(persisted.observation().process_id(), Some(42));
            assert_eq!(persisted.observation().last_heartbeat_ns(), Some(heartbeat));
            assert_eq!(persisted.observation().restart_count(), 2);
            assert!(supervisor.stops.is_empty());
        }
    }

    #[test]
    fn stale_heartbeat_is_drained_and_policy_controls_relaunch_marker() {
        for (policy, status) in [
            (RestartPolicy::Always, HostStatus::Restarting),
            (RestartPolicy::OnFailure, HostStatus::Restarting),
            (RestartPolicy::Never, HostStatus::Stopping),
        ] {
            let backend = InMemoryStorageBackend::new();
            register(
                &backend,
                "mail-host",
                policy,
                DesiredState::Running,
                HostObservation::stopped(),
            );
            let observed =
                SupervisorObservation::running([7; 32], 42, 700, 899, channel(1)).unwrap();
            let mut supervisor = FakeSupervisor::with("mail-host", observed);
            let report = reconciler(&backend)
                .reconcile_all(&mut supervisor, NOW)
                .unwrap();
            assert_eq!(report.outcomes()[0].action(), ReconcileAction::Stopped);
            assert_eq!(supervisor.stops, vec!["mail-host"]);
            assert_eq!(load(&backend, "mail-host").observation().status(), &status);
        }
    }

    #[test]
    fn future_observation_and_inspect_failure_are_explicit() {
        let backend = InMemoryStorageBackend::new();
        register(
            &backend,
            "mail-host",
            RestartPolicy::Always,
            DesiredState::Running,
            HostObservation::stopped(),
        );
        let observed =
            SupervisorObservation::running([7; 32], 42, NOW, NOW + 1, channel(1)).unwrap();
        let mut supervisor = FakeSupervisor::with("mail-host", observed);
        let error = reconciler(&backend)
            .reconcile_all(&mut supervisor, NOW)
            .unwrap_err();
        assert!(matches!(error, ReconcileError::FutureObservation { .. }));
        assert!(error.to_string().contains("mail-host"));

        let mut supervisor = FakeSupervisor {
            fail_inspect: true,
            ..FakeSupervisor::default()
        };
        let error = reconciler(&backend)
            .reconcile_all(&mut supervisor, NOW)
            .unwrap_err();
        assert!(matches!(
            error,
            ReconcileError::Supervisor {
                operation: SupervisorOperation::Inspect,
                ..
            }
        ));
        assert!(error.to_string().contains("inspect failed"));
    }

    #[test]
    fn mismatched_package_is_stopped_before_registered_package_starts() {
        let backend = InMemoryStorageBackend::new();
        register(
            &backend,
            "mail-host",
            RestartPolicy::Never,
            DesiredState::Running,
            HostObservation::stopped(),
        );
        let observed = SupervisorObservation::running([8; 32], 42, 800, 950, channel(1)).unwrap();
        let mut supervisor = FakeSupervisor::with("mail-host", observed);
        let first = reconciler(&backend)
            .reconcile_all(&mut supervisor, NOW)
            .unwrap();
        assert_eq!(first.outcomes()[0].action(), ReconcileAction::Stopped);
        assert_eq!(supervisor.stops, vec!["mail-host"]);
        assert!(supervisor.starts.is_empty());
        assert_eq!(
            load(&backend, "mail-host").observation().status(),
            &HostStatus::Restarting
        );

        supervisor.observations.clear();
        let second = reconciler(&backend)
            .reconcile_all(&mut supervisor, NOW + 1)
            .unwrap();
        assert_eq!(second.outcomes()[0].action(), ReconcileAction::Started);
        assert_eq!(supervisor.starts, vec!["mail-host"]);
        assert_eq!(load(&backend, "mail-host").observation().restart_count(), 1);
    }

    #[test]
    fn stopped_intent_drains_active_and_normalizes_inactive_state() {
        let backend = InMemoryStorageBackend::new();
        register(
            &backend,
            "mail-host",
            RestartPolicy::Always,
            DesiredState::Stopped,
            running_observation(1),
        );
        let observed = SupervisorObservation::starting([8; 32], 42, 800, None, None).unwrap();
        let mut supervisor = FakeSupervisor::with("mail-host", observed);
        let report = reconciler(&backend)
            .reconcile_all(&mut supervisor, NOW)
            .unwrap();
        assert_eq!(report.outcomes()[0].action(), ReconcileAction::Stopped);
        assert_eq!(supervisor.stops, vec!["mail-host"]);
        assert_eq!(
            load(&backend, "mail-host").observation().status(),
            &HostStatus::Stopping
        );

        supervisor.observations.clear();
        let report = reconciler(&backend)
            .reconcile_all(&mut supervisor, NOW + 1)
            .unwrap();
        assert_eq!(report.outcomes()[0].action(), ReconcileAction::Observed);
        let persisted = load(&backend, "mail-host");
        assert_eq!(persisted.observation().status(), &HostStatus::Stopped);
        assert_eq!(persisted.observation().process_id(), None);
        assert_eq!(persisted.observation().restart_count(), 1);
    }

    #[test]
    fn exited_or_absent_stopped_intent_preserves_lifecycle_history() {
        for (observation, expected_start) in [
            (SupervisorObservation::Absent, 800),
            (
                SupervisorObservation::exited([7; 32], Some(9), Some(810), Some(820)).unwrap(),
                810,
            ),
        ] {
            let backend = InMemoryStorageBackend::new();
            register(
                &backend,
                "mail-host",
                RestartPolicy::OnFailure,
                DesiredState::Stopped,
                running_observation(2),
            );
            let mut supervisor = FakeSupervisor::with("mail-host", observation);
            reconciler(&backend)
                .reconcile_all(&mut supervisor, NOW)
                .unwrap();
            let persisted = load(&backend, "mail-host");
            assert_eq!(persisted.observation().status(), &HostStatus::Stopped);
            assert_eq!(persisted.observation().restart_count(), 2);
            assert_eq!(
                persisted.observation().started_at_ns(),
                Some(expected_start)
            );
        }
    }

    #[test]
    fn quarantine_defers_then_expires_and_overflow_fails_closed() {
        let backend = InMemoryStorageBackend::new();
        let quarantined = HostObservation::new(
            HostStatus::Quarantined {
                until_ns: NOW + 1,
                reason: "cooldown".to_string(),
            },
            None,
            Some(700),
            Some(800),
            None,
            2,
            Some(600),
        )
        .unwrap();
        register(
            &backend,
            "mail-host",
            RestartPolicy::OnFailure,
            DesiredState::Running,
            quarantined,
        );
        let mut supervisor = FakeSupervisor::default();
        let deferred = reconciler(&backend)
            .reconcile_all(&mut supervisor, NOW)
            .unwrap();
        assert_eq!(deferred.outcomes()[0].action(), ReconcileAction::Deferred);
        assert!(supervisor.starts.is_empty());
        supervisor.observations.insert(
            "mail-host".to_string(),
            SupervisorObservation::running([7; 32], 42, 700, 950, channel(1)).unwrap(),
        );
        supervisor.fail_stop = true;
        let error = reconciler(&backend)
            .reconcile_all(&mut supervisor, NOW)
            .unwrap_err();
        assert!(matches!(
            error,
            ReconcileError::Supervisor {
                operation: SupervisorOperation::Stop,
                ..
            }
        ));
        assert!(matches!(
            load(&backend, "mail-host").observation().status(),
            HostStatus::Quarantined { .. }
        ));
        supervisor.fail_stop = false;
        let drained = reconciler(&backend)
            .reconcile_all(&mut supervisor, NOW)
            .unwrap();
        assert_eq!(drained.outcomes()[0].action(), ReconcileAction::Stopped);
        assert_eq!(supervisor.stops, vec!["mail-host"]);
        assert!(matches!(
            load(&backend, "mail-host").observation().status(),
            HostStatus::Quarantined { .. }
        ));
        supervisor.observations.clear();
        let started = reconciler(&backend)
            .reconcile_all(&mut supervisor, NOW + 1)
            .unwrap();
        assert_eq!(started.outcomes()[0].action(), ReconcileAction::Started);
        assert_eq!(load(&backend, "mail-host").observation().restart_count(), 3);

        let backend = InMemoryStorageBackend::new();
        let exhausted = HostObservation::new(
            HostStatus::Crashed { exit_code: None },
            None,
            Some(700),
            Some(800),
            None,
            u32::MAX,
            Some(600),
        )
        .unwrap();
        register(
            &backend,
            "mail-host",
            RestartPolicy::Always,
            DesiredState::Running,
            exhausted,
        );
        let mut supervisor = FakeSupervisor::default();
        let report = reconciler(&backend)
            .reconcile_all(&mut supervisor, NOW)
            .unwrap();
        assert_eq!(report.outcomes()[0].action(), ReconcileAction::Deferred);
        assert!(supervisor.starts.is_empty());
        assert_eq!(
            load(&backend, "mail-host").observation().status(),
            &HostStatus::Quarantined {
                until_ns: u64::MAX,
                reason: RESTART_COUNTER_EXHAUSTED.to_string()
            }
        );
    }

    #[test]
    fn absent_cached_states_apply_recovery_policy() {
        let cases = [
            (HostStatus::Starting, RestartPolicy::Never, true, 1),
            (HostStatus::Restarting, RestartPolicy::Never, true, 1),
            (HostStatus::Running, RestartPolicy::OnFailure, true, 2),
            (HostStatus::Running, RestartPolicy::Never, false, 1),
            (HostStatus::Stopping, RestartPolicy::OnFailure, true, 2),
            (HostStatus::Stopping, RestartPolicy::Never, false, 1),
            (HostStatus::Stopped, RestartPolicy::Always, true, 2),
            (HostStatus::Stopped, RestartPolicy::OnFailure, false, 1),
        ];
        for (status, policy, starts, expected_restarts) in cases {
            let backend = InMemoryStorageBackend::new();
            let observation = if status == HostStatus::Running {
                running_observation(1)
            } else {
                HostObservation::new(status, None, Some(700), Some(800), None, 1, Some(600))
                    .unwrap()
            };
            register(
                &backend,
                "mail-host",
                policy,
                DesiredState::Running,
                observation,
            );
            let mut supervisor = FakeSupervisor::default();
            let report = reconciler(&backend)
                .reconcile_all(&mut supervisor, NOW)
                .unwrap();
            assert_eq!(
                report.outcomes()[0].action() == ReconcileAction::Started,
                starts
            );
            assert_eq!(
                load(&backend, "mail-host").observation().restart_count(),
                expected_restarts
            );
        }
    }

    #[test]
    fn starting_stopping_and_exited_supervisor_phases_are_persisted() {
        let observations = [
            (
                SupervisorObservation::starting([7; 32], 42, 800, None, None).unwrap(),
                HostStatus::Starting,
                ReconcileAction::Observed,
            ),
            (
                SupervisorObservation::stopping([7; 32], 42, 800, Some(900), Some(channel(1)))
                    .unwrap(),
                HostStatus::Stopping,
                ReconcileAction::Deferred,
            ),
        ];
        for (observed, status, action) in observations {
            let backend = InMemoryStorageBackend::new();
            register(
                &backend,
                "mail-host",
                RestartPolicy::OnFailure,
                DesiredState::Running,
                HostObservation::stopped(),
            );
            let mut supervisor = FakeSupervisor::with("mail-host", observed);
            let report = reconciler(&backend)
                .reconcile_all(&mut supervisor, NOW)
                .unwrap();
            assert_eq!(report.outcomes()[0].action(), action);
            assert_eq!(load(&backend, "mail-host").observation().status(), &status);
        }
    }

    #[test]
    fn start_and_stop_failures_leave_recoverable_observations() {
        let backend = InMemoryStorageBackend::new();
        register(
            &backend,
            "mail-host",
            RestartPolicy::OnFailure,
            DesiredState::Running,
            HostObservation::stopped(),
        );
        let mut supervisor = FakeSupervisor {
            fail_start: true,
            ..FakeSupervisor::default()
        };
        let error = reconciler(&backend)
            .reconcile_all(&mut supervisor, NOW)
            .unwrap_err();
        assert!(matches!(
            error,
            ReconcileError::Supervisor {
                operation: SupervisorOperation::Start,
                ..
            }
        ));
        assert_eq!(
            load(&backend, "mail-host").observation().status(),
            &HostStatus::Crashed { exit_code: None }
        );

        let backend = InMemoryStorageBackend::new();
        register(
            &backend,
            "mail-host",
            RestartPolicy::Always,
            DesiredState::Stopped,
            HostObservation::stopped(),
        );
        let observed = SupervisorObservation::running([7; 32], 42, 800, 950, channel(1)).unwrap();
        let mut supervisor = FakeSupervisor {
            fail_stop: true,
            ..FakeSupervisor::with("mail-host", observed)
        };
        let error = reconciler(&backend)
            .reconcile_all(&mut supervisor, NOW)
            .unwrap_err();
        assert!(matches!(
            error,
            ReconcileError::Supervisor {
                operation: SupervisorOperation::Stop,
                ..
            }
        ));
        assert_eq!(
            load(&backend, "mail-host").observation().status(),
            &HostStatus::Running
        );
    }

    struct ConflictingSupervisor<'a> {
        backend: &'a InMemoryStorageBackend,
        starts: usize,
    }

    impl HostSupervisor for ConflictingSupervisor<'_> {
        type Error = FakeError;

        fn inspect(
            &mut self,
            registration: &HostRegistration,
        ) -> Result<SupervisorObservation, Self::Error> {
            let registry = ServiceRegistry::new(self.backend);
            let loaded = registry.load(registration.host_name()).unwrap().unwrap();
            let changed = loaded
                .entry()
                .clone()
                .with_desired_state(DesiredState::Stopped);
            registry.update(&loaded, &changed).unwrap();
            Ok(SupervisorObservation::Absent)
        }

        fn start(&mut self, _registration: &HostRegistration) -> Result<(), Self::Error> {
            self.starts += 1;
            Ok(())
        }

        fn stop(&mut self, _host_name: &HostName) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    #[test]
    fn claim_cas_conflict_prevents_external_mutation() {
        let backend = InMemoryStorageBackend::new();
        register(
            &backend,
            "mail-host",
            RestartPolicy::Always,
            DesiredState::Running,
            HostObservation::stopped(),
        );
        let mut supervisor = ConflictingSupervisor {
            backend: &backend,
            starts: 0,
        };
        let error = reconciler(&backend)
            .reconcile_all(&mut supervisor, NOW)
            .unwrap_err();
        assert!(matches!(
            error,
            ReconcileError::Registry(RegistryError::ConcurrentUpdate(_))
        ));
        assert_eq!(supervisor.starts, 0);
    }

    #[test]
    fn public_error_and_operation_diagnostics_are_bounded() {
        assert_eq!(SupervisorOperation::Inspect.to_string(), "inspect");
        assert_eq!(SupervisorOperation::Start.to_string(), "start");
        assert_eq!(SupervisorOperation::Stop.to_string(), "stop");
        let error: ReconcileError<FakeError> =
            ReconcileError::Registry(RegistryError::TooManyHosts);
        assert!(error.to_string().contains("4096-host"));
        let error = ReconcileError::Supervisor {
            host_name: name("mail-host"),
            operation: SupervisorOperation::Start,
            source: FakeError("offline"),
        };
        assert_eq!(
            error.to_string(),
            "supervisor start failed for mail-host: offline"
        );
    }
}
