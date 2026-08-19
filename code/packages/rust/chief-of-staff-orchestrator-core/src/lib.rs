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
use chief_of_staff_host_control_protocol::ChannelBindingAccess;
use chief_of_staff_host_data_plane::HostDataPlaneDispatcher;
use chief_of_staff_host_runtime::PackageKeyring;
use chief_of_staff_pipeline_bindings::{
    HostPipelineBinding, LoadedHostPipelineBinding, PipelineBindingError, PipelineBindingStore,
};
use chief_of_staff_process_supervisor::{
    HostLaunchBindingProvider, MonotonicClock, ProcessHostSupervisor, ProcessSupervisorConfig,
    SessionIdSource,
};
use chief_of_staff_service_reconciler::{
    HostSupervisor, ReconcileConfig, ReconcileError, ReconcileReport, ServiceReconciler,
    SupervisorObservation, SupervisorPhase,
};
use chief_of_staff_service_registry::{
    DesiredState, HostEntry, HostName, HostRegistration, LoadedHost, RegistryError, ServiceRegistry,
};
use chief_of_staff_tool_api::PrivilegeTier;
use chief_of_staff_trust_checker::{
    ApprovalProvider, TrustChecker, TrustCheckerError, TrustRequestContext, TrustRequestError,
    TrustResource,
};
use coding_adventures_sha256::sha256_hex;
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

/// Stable operation represented by a channel wiring request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChannelWiringOperation {
    /// Create an immutable active channel definition.
    Create,
    /// Irreversibly destroy the current active definition.
    Destroy,
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

    /// Return the exact topology operation.
    pub fn operation(self) -> ChannelWiringOperation {
        match self {
            Self::Create(_) => ChannelWiringOperation::Create,
            Self::Destroy(_) => ChannelWiringOperation::Destroy,
        }
    }
}

/// Injected privilege and human-approval boundary for channel topology changes.
pub trait ChannelWiringAuthorizer {
    /// Concrete authorization failure retained for programmatic handling.
    type Error;

    /// Approve this exact mutation or fail before any storage change.
    fn authorize(
        &mut self,
        context: &TrustRequestContext,
        request: ChannelWiringRequest<'_>,
    ) -> Result<(), Self::Error>;
}

/// Exact durable host-binding mutation presented to a trust-checking adapter.
#[derive(Clone, Copy, Debug)]
pub enum PipelineWiringRequest<'a> {
    /// Authorize creation of this complete immutable host launch authority.
    Wire(&'a HostPipelineBinding),
    /// Authorize removal of this current durable host launch authority.
    Unwire(&'a HostPipelineBinding),
}

/// Stable operation represented by a pipeline wiring request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PipelineWiringOperation {
    /// Create one durable host launch binding.
    Wire,
    /// Remove one durable host launch binding.
    Unwire,
}

impl<'a> PipelineWiringRequest<'a> {
    /// Return the complete durable binding presented for authorization.
    pub fn binding(self) -> &'a HostPipelineBinding {
        match self {
            Self::Wire(binding) | Self::Unwire(binding) => binding,
        }
    }

    /// Return the exact binding operation.
    pub fn operation(self) -> PipelineWiringOperation {
        match self {
            Self::Wire(_) => PipelineWiringOperation::Wire,
            Self::Unwire(_) => PipelineWiringOperation::Unwire,
        }
    }
}

/// Injected privilege and human-approval boundary for pipeline launch authority.
pub trait PipelineWiringAuthorizer {
    /// Concrete authorization failure retained for programmatic handling.
    type Error;

    /// Approve this exact mutation or fail before any binding-store change.
    fn authorize_pipeline(
        &mut self,
        context: &TrustRequestContext,
        request: PipelineWiringRequest<'_>,
    ) -> Result<(), Self::Error>;
}

/// Authoritative privilege lookup for one exact pipeline-binding mutation.
pub trait PipelinePrivilegeResolver {
    /// Concrete lookup failure retained for programmatic recovery.
    type Error;

    /// Resolve the maximum current tier of this exact pipeline binding.
    ///
    /// Implementations must include every referenced channel and selected model
    /// or package policy in this result; the exact binding is separately bound
    /// into the approval resource fingerprint.
    fn pipeline_tier(
        &mut self,
        request: PipelineWiringRequest<'_>,
    ) -> Result<PrivilegeTier, Self::Error>;

    /// Resolve the current tier assigned to the bound agent identity.
    fn pipeline_agent_tier(
        &mut self,
        agent_id: &chief_of_staff_channel_endpoints::AgentId,
    ) -> Result<PrivilegeTier, Self::Error>;
}

/// Authoritative privilege lookup for one exact channel mutation.
pub trait ChannelPrivilegeResolver {
    /// Concrete lookup failure retained for programmatic recovery.
    type Error;

    /// Resolve the tier assigned to this exact channel and operation.
    fn channel_tier(
        &mut self,
        request: ChannelWiringRequest<'_>,
    ) -> Result<PrivilegeTier, Self::Error>;

    /// Resolve the current tier assigned to one channel member.
    fn agent_tier(
        &mut self,
        agent_id: &chief_of_staff_channel_endpoints::AgentId,
    ) -> Result<PrivilegeTier, Self::Error>;
}

/// Fail-closed error from Trust Checker channel authorization.
#[derive(Debug)]
pub enum TrustChannelWiringError<ResolverError, ProviderError> {
    /// Authoritative resource-tier resolution failed.
    Resolver(ResolverError),
    /// The resolved exact request violated a Trust Checker bound.
    Request(TrustRequestError),
    /// Approval was denied, timed out, too weak, or unavailable.
    Approval(TrustCheckerError<ProviderError>),
}

/// Fail-closed error from Trust Checker pipeline-binding authorization.
#[derive(Debug)]
pub enum TrustPipelineWiringError<ResolverError, ProviderError> {
    /// Authoritative resource-tier resolution failed.
    Resolver(ResolverError),
    /// The resolved exact request violated a Trust Checker bound.
    Request(TrustRequestError),
    /// Approval was denied, timed out, too weak, or unavailable.
    Approval(TrustCheckerError<ProviderError>),
}

impl<R, P> Display for TrustPipelineWiringError<R, P> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Resolver(_) => "orchestrator-core: pipeline privilege resolution failed",
            Self::Request(_) => "orchestrator-core: pipeline trust request invalid",
            Self::Approval(_) => "orchestrator-core: pipeline approval failed",
        })
    }
}

impl<R, P> std::error::Error for TrustPipelineWiringError<R, P>
where
    R: std::error::Error + 'static,
    P: std::error::Error + 'static,
{
}

impl<R, P> Display for TrustChannelWiringError<R, P> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Resolver(_) => "orchestrator-core: channel privilege resolution failed",
            Self::Request(_) => "orchestrator-core: channel trust request invalid",
            Self::Approval(_) => "orchestrator-core: channel approval failed",
        })
    }
}

impl<R, P> std::error::Error for TrustChannelWiringError<R, P>
where
    R: std::error::Error + 'static,
    P: std::error::Error + 'static,
{
}

/// Exact channel authorizer backed by authoritative tiers and the Trust Checker.
pub struct TrustCheckingChannelWiring<P, R> {
    checker: TrustChecker<P>,
    resolver: R,
}

impl<P, R> TrustCheckingChannelWiring<P, R> {
    /// Compose one trusted approval provider with one authoritative tier resolver.
    pub fn new(provider: P, resolver: R) -> Self {
        Self {
            checker: TrustChecker::new(provider),
            resolver,
        }
    }

    /// Consume the adapter and recover its provider and resolver.
    pub fn into_parts(self) -> (P, R) {
        (self.checker.into_provider(), self.resolver)
    }
}

impl<P, R> ChannelWiringAuthorizer for TrustCheckingChannelWiring<P, R>
where
    P: ApprovalProvider,
    R: ChannelPrivilegeResolver,
{
    type Error = TrustChannelWiringError<R::Error, P::Error>;

    fn authorize(
        &mut self,
        context: &TrustRequestContext,
        request: ChannelWiringRequest<'_>,
    ) -> Result<(), Self::Error> {
        let definition = request.definition();
        let mut resources = Vec::with_capacity(definition.receivers().len() + 2);
        resources.push(
            TrustResource::new(
                mutation_resource_id(request),
                self.resolver
                    .channel_tier(request)
                    .map_err(TrustChannelWiringError::Resolver)?,
            )
            .map_err(TrustChannelWiringError::Request)?,
        );
        resources.push(
            TrustResource::new(
                agent_resource_id(definition.originator().agent_id.as_bytes()),
                self.resolver
                    .agent_tier(&definition.originator().agent_id)
                    .map_err(TrustChannelWiringError::Resolver)?,
            )
            .map_err(TrustChannelWiringError::Request)?,
        );
        for receiver in definition.receivers() {
            resources.push(
                TrustResource::new(
                    agent_resource_id(receiver.agent_id.as_bytes()),
                    self.resolver
                        .agent_tier(&receiver.agent_id)
                        .map_err(TrustChannelWiringError::Resolver)?,
                )
                .map_err(TrustChannelWiringError::Request)?,
            );
        }
        let trust_request = context
            .with_resources(resources)
            .map_err(TrustChannelWiringError::Request)?;
        self.checker
            .authorize(&trust_request)
            .map(|_| ())
            .map_err(TrustChannelWiringError::Approval)
    }
}

impl<P, R> PipelineWiringAuthorizer for TrustCheckingChannelWiring<P, R>
where
    P: ApprovalProvider,
    R: PipelinePrivilegeResolver,
{
    type Error = TrustPipelineWiringError<R::Error, P::Error>;

    fn authorize_pipeline(
        &mut self,
        context: &TrustRequestContext,
        request: PipelineWiringRequest<'_>,
    ) -> Result<(), Self::Error> {
        let binding = request.binding();
        let resources = vec![
            TrustResource::new(
                pipeline_mutation_resource_id(request),
                self.resolver
                    .pipeline_tier(request)
                    .map_err(TrustPipelineWiringError::Resolver)?,
            )
            .map_err(TrustPipelineWiringError::Request)?,
            TrustResource::new(
                agent_resource_id(binding.agent_id().as_bytes()),
                self.resolver
                    .pipeline_agent_tier(binding.agent_id())
                    .map_err(TrustPipelineWiringError::Resolver)?,
            )
            .map_err(TrustPipelineWiringError::Request)?,
        ];
        let trust_request = context
            .with_resources(resources)
            .map_err(TrustPipelineWiringError::Request)?;
        self.checker
            .authorize(&trust_request)
            .map(|_| ())
            .map_err(TrustPipelineWiringError::Approval)
    }
}

fn mutation_resource_id(request: ChannelWiringRequest<'_>) -> String {
    let operation = match request.operation() {
        ChannelWiringOperation::Create => "create",
        ChannelWiringOperation::Destroy => "destroy",
    };
    format!(
        "channel:{operation}:sha256:{}",
        sha256_hex(&canonical_mutation_bytes(request))
    )
}

fn pipeline_mutation_resource_id(request: PipelineWiringRequest<'_>) -> String {
    let operation = match request.operation() {
        PipelineWiringOperation::Wire => "wire",
        PipelineWiringOperation::Unwire => "unwire",
    };
    format!(
        "pipeline:{operation}:sha256:{}",
        sha256_hex(&canonical_pipeline_mutation_bytes(request))
    )
}

fn agent_resource_id(agent_id: &[u8]) -> String {
    format!("agent:{}", encode_hex(agent_id))
}

fn canonical_mutation_bytes(request: ChannelWiringRequest<'_>) -> Vec<u8> {
    let definition = request.definition();
    let mut bytes = b"chief-channel-wiring-v1\0".to_vec();
    bytes.push(match request.operation() {
        ChannelWiringOperation::Create => 0,
        ChannelWiringOperation::Destroy => 1,
    });
    bytes.extend_from_slice(&definition.channel_id().0);
    put_bounded_bytes(&mut bytes, definition.originator().agent_id.as_bytes());
    bytes.extend_from_slice(&definition.originator().public_key);
    bytes.extend_from_slice(&(definition.receivers().len() as u32).to_be_bytes());
    for receiver in definition.receivers() {
        put_bounded_bytes(&mut bytes, receiver.agent_id.as_bytes());
        bytes.extend_from_slice(&receiver.public_key);
    }
    bytes.extend_from_slice(&definition.created_at_ns().to_be_bytes());
    bytes.extend_from_slice(&definition.key_epoch().0.to_be_bytes());
    bytes.push(match definition.lifecycle() {
        chief_of_staff_channel_endpoints::ChannelLifecycle::Active => 0,
        chief_of_staff_channel_endpoints::ChannelLifecycle::Destroyed => 1,
    });
    bytes
}

fn canonical_pipeline_mutation_bytes(request: PipelineWiringRequest<'_>) -> Vec<u8> {
    let binding = request.binding();
    let registration = binding.registration();
    let mut bytes = b"chief-pipeline-wiring-v1\0".to_vec();
    bytes.push(match request.operation() {
        PipelineWiringOperation::Wire => 0,
        PipelineWiringOperation::Unwire => 1,
    });
    bytes.extend_from_slice(binding.pipeline_id().as_bytes());
    put_bounded_bytes(&mut bytes, registration.host_name().as_str().as_bytes());
    put_bounded_bytes(&mut bytes, registration.package_path().as_str().as_bytes());
    bytes.extend_from_slice(registration.package_hash());
    bytes.push(match registration.restart_policy() {
        chief_of_staff_service_registry::RestartPolicy::Always => 0,
        chief_of_staff_service_registry::RestartPolicy::OnFailure => 1,
        chief_of_staff_service_registry::RestartPolicy::Never => 2,
    });
    put_bounded_bytes(&mut bytes, binding.agent_id().as_bytes());
    let launch = binding.launch_bindings();
    bytes.extend_from_slice(&(launch.channels().len() as u32).to_be_bytes());
    for channel in launch.channels() {
        put_bounded_bytes(&mut bytes, channel.name().as_bytes());
        bytes.push(match channel.access() {
            ChannelBindingAccess::Read => 0,
            ChannelBindingAccess::Write => 1,
        });
        bytes.extend_from_slice(&channel.channel_id());
    }
    match launch.level_one_model() {
        None => bytes.push(0),
        Some(model) => {
            bytes.push(1);
            put_bounded_bytes(&mut bytes, model.model().as_bytes());
            bytes.extend_from_slice(&model.temperature().to_bits().to_be_bytes());
            bytes.extend_from_slice(&model.max_tokens().to_be_bytes());
        }
    }
    bytes
}

fn put_bounded_bytes(output: &mut Vec<u8>, value: &[u8]) {
    output.extend_from_slice(&(value.len() as u32).to_be_bytes());
    output.extend_from_slice(value);
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
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
    /// Durable pipeline launch binding validation, storage, or CAS failure.
    Pipeline(PipelineBindingError),
    /// The injected trust boundary denied or failed a topology mutation.
    Authorization(AuthorizationError),
    /// The injected trust boundary denied or failed a pipeline binding mutation.
    PipelineAuthorization(AuthorizationError),
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
            Self::Pipeline(_) => formatter.write_str("orchestrator-core: pipeline binding failure"),
            Self::Authorization(_) => {
                formatter.write_str("orchestrator-core: channel authorization failed")
            }
            Self::PipelineAuthorization(_) => {
                formatter.write_str("orchestrator-core: pipeline authorization failed")
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

    /// Atomically activate new package identity for a stopped, inactive host.
    ///
    /// The stable host name is retained, cached observation is reset, and the
    /// requested post-reload desired state is written in the same revision-CAS
    /// transaction. A running replacement is started by the next reconciliation
    /// tick; this operation never overlaps old and new host processes.
    pub fn reload_host(
        &mut self,
        registration: HostRegistration,
        desired_state: DesiredState,
    ) -> Result<LoadedHost, OrchestratorCoreError<S::Error, A::Error>> {
        let host_name = registration.host_name().clone();
        let registry = ServiceRegistry::new(self.backend.as_ref());
        let loaded = registry
            .load(&host_name)
            .map_err(OrchestratorCoreError::Registry)?
            .ok_or_else(|| {
                OrchestratorCoreError::Registry(RegistryError::HostNotFound(host_name.clone()))
            })?;
        if loaded.entry().registration() == &registration
            && loaded.entry().desired_state() == desired_state
        {
            return Ok(loaded);
        }
        if loaded.entry().desired_state() != DesiredState::Stopped {
            return Err(OrchestratorCoreError::HostDesiredRunning(host_name));
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
            return Err(OrchestratorCoreError::HostStillActive(host_name));
        }
        registry
            .replace_stopped_registration(&loaded, registration, desired_state)
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
        context: &TrustRequestContext,
        definition: &ChannelDefinition,
    ) -> Result<ChannelDefinition, OrchestratorCoreError<S::Error, A::Error>> {
        self.authorizer
            .authorize(context, ChannelWiringRequest::Create(definition))
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
        context: &TrustRequestContext,
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
            .authorize(context, ChannelWiringRequest::Destroy(&definition))
            .map_err(OrchestratorCoreError::Authorization)?;
        store
            .destroy(channel_id)
            .map_err(OrchestratorCoreError::Channel)
    }
}

impl<S, A> OrchestratorCore<S, A>
where
    S: HostSupervisor,
    A: PipelineWiringAuthorizer,
{
    /// Authorize then idempotently create one exact durable host pipeline binding.
    pub fn wire_host_pipeline(
        &mut self,
        context: &TrustRequestContext,
        binding: &HostPipelineBinding,
    ) -> Result<LoadedHostPipelineBinding, OrchestratorCoreError<S::Error, A::Error>> {
        self.authorizer
            .authorize_pipeline(context, PipelineWiringRequest::Wire(binding))
            .map_err(OrchestratorCoreError::PipelineAuthorization)?;
        PipelineBindingStore::new(self.backend.as_ref())
            .wire(binding)
            .map_err(OrchestratorCoreError::Pipeline)
    }

    /// Load durable launch authority without treating it as current process authority.
    pub fn load_host_pipeline(
        &self,
        host_name: &HostName,
    ) -> Result<Option<LoadedHostPipelineBinding>, OrchestratorCoreError<S::Error, A::Error>> {
        PipelineBindingStore::new(self.backend.as_ref())
            .load(host_name)
            .map_err(OrchestratorCoreError::Pipeline)
    }

    /// Authorize then revision-CAS remove the current host pipeline binding.
    ///
    /// Absence is an idempotent no-op and therefore requires no approval.
    pub fn unwire_host_pipeline(
        &mut self,
        context: &TrustRequestContext,
        host_name: &HostName,
    ) -> Result<Option<HostPipelineBinding>, OrchestratorCoreError<S::Error, A::Error>> {
        let store = PipelineBindingStore::new(self.backend.as_ref());
        let Some(loaded) = store
            .load(host_name)
            .map_err(OrchestratorCoreError::Pipeline)?
        else {
            return Ok(None);
        };
        self.authorizer
            .authorize_pipeline(context, PipelineWiringRequest::Unwire(loaded.binding()))
            .map_err(OrchestratorCoreError::PipelineAuthorization)?;
        store
            .unwire(&loaded)
            .map_err(OrchestratorCoreError::Pipeline)?;
        Ok(Some(loaded.binding().clone()))
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
        launch_bindings: Arc<dyn HostLaunchBindingProvider>,
        data_plane_dispatcher: Arc<dyn HostDataPlaneDispatcher>,
        identity: Arc<IdentityKeyPair>,
        clock: Arc<dyn MonotonicClock>,
        sessions: Box<dyn SessionIdSource>,
        reconcile_config: ReconcileConfig,
        authorizer: A,
    ) -> Self {
        let supervisor = ProcessHostSupervisor::new(
            process_config,
            keyring,
            launch_bindings,
            identity,
            Arc::clone(&clock),
            sessions,
        )
        .with_data_plane_dispatcher(data_plane_dispatcher);
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
    use chief_of_staff_host_control_protocol::{
        ChannelBinding, LaunchBindings, LevelOneModelBinding,
    };
    use chief_of_staff_host_data_plane::UnavailableHostDataPlaneDispatcher;
    use chief_of_staff_pipeline_bindings::PipelineId;
    use chief_of_staff_process_supervisor::{
        DenyHostLaunchBindings, HostProgram, UuidV7SessionIdSource,
    };
    use chief_of_staff_service_reconciler::{ReconcileAction, SupervisorOperation};
    use chief_of_staff_service_registry::{PackagePath, RestartPolicy};
    use chief_of_staff_tool_api::ApprovalAssurance;
    use chief_of_staff_trust_checker::{ApprovalOutcome, ApprovalPrompt, TrustRequest};
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
        Wire(PipelineId),
        Unwire(PipelineId),
    }

    struct FakeAuthorizer {
        events: Arc<Mutex<Vec<WiringEvent>>>,
        allow: bool,
    }

    struct RecordingApprovalProvider {
        outcome: Result<ApprovalOutcome, FakeError>,
        requests: Vec<(
            TrustRequest,
            chief_of_staff_trust_checker::ApprovalRequirement,
        )>,
    }

    impl RecordingApprovalProvider {
        fn approving(assurance: ApprovalAssurance) -> Self {
            Self {
                outcome: Ok(ApprovalOutcome::Approved(assurance)),
                requests: Vec::new(),
            }
        }
    }

    impl ApprovalProvider for RecordingApprovalProvider {
        type Error = FakeError;

        fn request_approval(
            &mut self,
            prompt: ApprovalPrompt<'_>,
        ) -> Result<ApprovalOutcome, Self::Error> {
            self.requests
                .push((prompt.request().clone(), prompt.requirement()));
            self.outcome.clone()
        }
    }

    struct FixedTierResolver {
        channel: PrivilegeTier,
        pipeline: PrivilegeTier,
        agents: BTreeMap<Vec<u8>, PrivilegeTier>,
        fail: bool,
    }

    impl FixedTierResolver {
        fn tier0() -> Self {
            Self {
                channel: PrivilegeTier::Tier0,
                pipeline: PrivilegeTier::Tier0,
                agents: BTreeMap::new(),
                fail: false,
            }
        }
    }

    impl ChannelPrivilegeResolver for FixedTierResolver {
        type Error = FakeError;

        fn channel_tier(
            &mut self,
            _request: ChannelWiringRequest<'_>,
        ) -> Result<PrivilegeTier, Self::Error> {
            if self.fail {
                Err(FakeError("private tier resolution detail"))
            } else {
                Ok(self.channel)
            }
        }

        fn agent_tier(&mut self, agent_id: &AgentId) -> Result<PrivilegeTier, Self::Error> {
            if self.fail {
                return Err(FakeError("private tier resolution detail"));
            }
            Ok(self
                .agents
                .get(agent_id.as_bytes())
                .copied()
                .unwrap_or(PrivilegeTier::Tier0))
        }
    }

    impl PipelinePrivilegeResolver for FixedTierResolver {
        type Error = FakeError;

        fn pipeline_tier(
            &mut self,
            _request: PipelineWiringRequest<'_>,
        ) -> Result<PrivilegeTier, Self::Error> {
            if self.fail {
                Err(FakeError("private pipeline tier resolution detail"))
            } else {
                Ok(self.pipeline)
            }
        }

        fn pipeline_agent_tier(
            &mut self,
            agent_id: &AgentId,
        ) -> Result<PrivilegeTier, Self::Error> {
            if self.fail {
                return Err(FakeError("private pipeline tier resolution detail"));
            }
            Ok(self
                .agents
                .get(agent_id.as_bytes())
                .copied()
                .unwrap_or(PrivilegeTier::Tier0))
        }
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

        fn authorize(
            &mut self,
            context: &TrustRequestContext,
            request: ChannelWiringRequest<'_>,
        ) -> Result<(), Self::Error> {
            assert_eq!(context.request_id(), "wire-request");
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

    impl PipelineWiringAuthorizer for FakeAuthorizer {
        type Error = FakeError;

        fn authorize_pipeline(
            &mut self,
            context: &TrustRequestContext,
            request: PipelineWiringRequest<'_>,
        ) -> Result<(), Self::Error> {
            assert_eq!(context.request_id(), "wire-request");
            let event = match request {
                PipelineWiringRequest::Wire(binding) => WiringEvent::Wire(binding.pipeline_id()),
                PipelineWiringRequest::Unwire(binding) => {
                    WiringEvent::Unwire(binding.pipeline_id())
                }
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

    fn pipeline_id(byte: u8) -> PipelineId {
        PipelineId::new(channel_id(byte).0).expect("valid pipeline id")
    }

    fn wiring_context() -> TrustRequestContext {
        TrustRequestContext::new("wire-request", "operator:local").unwrap()
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

    fn pipeline_channels(
        backend: &dyn StorageBackend,
        agent_id: &AgentId,
    ) -> (ChannelId, ChannelId) {
        let read = channel_id(31);
        let write = channel_id(32);
        let store = ChannelDefinitionStore::new(backend);
        store
            .create(
                &ChannelDefinition::new(
                    read,
                    OriginatorIdentity {
                        agent_id: AgentId::new(b"request-source".to_vec()).unwrap(),
                        public_key: [3; 32],
                    },
                    vec![ReceiverIdentity {
                        agent_id: agent_id.clone(),
                        public_key: [4; 32],
                    }],
                    200,
                    KeyEpoch(1),
                )
                .unwrap(),
            )
            .unwrap();
        store
            .create(
                &ChannelDefinition::new(
                    write,
                    OriginatorIdentity {
                        agent_id: agent_id.clone(),
                        public_key: [5; 32],
                    },
                    vec![ReceiverIdentity {
                        agent_id: AgentId::new(b"report-sink".to_vec()).unwrap(),
                        public_key: [6; 32],
                    }],
                    201,
                    KeyEpoch(1),
                )
                .unwrap(),
            )
            .unwrap();
        (read, write)
    }

    fn pipeline_binding(
        pipeline_id: PipelineId,
        registration: HostRegistration,
        agent_id: AgentId,
        channels: (ChannelId, ChannelId),
        model: &str,
    ) -> HostPipelineBinding {
        HostPipelineBinding::new(
            pipeline_id,
            registration,
            agent_id,
            LaunchBindings::new(
                vec![
                    ChannelBinding::new("requests", ChannelBindingAccess::Read, channels.0 .0)
                        .unwrap(),
                    ChannelBinding::new("reports", ChannelBindingAccess::Write, channels.1 .0)
                        .unwrap(),
                ],
                Some(LevelOneModelBinding::new(model, 0.25, 256).unwrap()),
            )
            .unwrap(),
        )
    }

    fn core<A>(
        backend: Arc<InMemoryStorageBackend>,
        supervisor: FakeSupervisor,
        authorizer: A,
        clock: Arc<TestClock>,
    ) -> OrchestratorCore<FakeSupervisor, A> {
        OrchestratorCore::new(
            backend,
            supervisor,
            authorizer,
            clock,
            ReconcileConfig::new(1, 100).expect("valid reconcile config"),
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
    fn reload_atomically_replaces_inactive_package_and_resumes_on_next_tick() {
        let backend = Arc::new(InMemoryStorageBackend::new());
        let supervisor = FakeSupervisor::default();
        let state = Arc::clone(&supervisor.0);
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut core = core(
            Arc::clone(&backend),
            supervisor,
            FakeAuthorizer::allowing(events),
            Arc::new(TestClock::new([100])),
        );
        core.register_host(registration("agent-host", 1), DesiredState::Stopped)
            .expect("register stopped host");
        let replacement = registration("agent-host", 2);
        let reloaded = core
            .reload_host(replacement.clone(), DesiredState::Running)
            .expect("reload inactive host");
        assert_eq!(reloaded.entry().registration(), &replacement);
        assert_eq!(reloaded.entry().desired_state(), DesiredState::Running);
        assert_eq!(
            reloaded.entry().observation().status(),
            &chief_of_staff_service_registry::HostStatus::Stopped
        );
        state
            .lock()
            .expect("supervisor mutex poisoned")
            .fail_inspect = true;
        assert_eq!(
            core.reload_host(replacement, DesiredState::Running)
                .expect("idempotent replay"),
            reloaded
        );
        state
            .lock()
            .expect("supervisor mutex poisoned")
            .fail_inspect = false;
        assert_eq!(
            core.reconcile_once().expect("start replacement").outcomes()[0].action(),
            ReconcileAction::Started
        );
        assert_eq!(
            state.lock().expect("supervisor mutex poisoned").starts,
            ["agent-host"]
        );
    }

    #[test]
    fn reload_refuses_running_intent_or_live_supervisor_authority() {
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
        let old = registration("agent-host", 1);
        core.register_host(old.clone(), DesiredState::Running)
            .expect("register running host");
        assert!(matches!(
            core.reload_host(registration("agent-host", 2), DesiredState::Running),
            Err(OrchestratorCoreError::HostDesiredRunning(_))
        ));
        core.set_desired_state(old.host_name(), DesiredState::Stopped)
            .expect("request stop");
        state
            .lock()
            .expect("supervisor mutex poisoned")
            .observations
            .insert(
                "agent-host".to_string(),
                SupervisorObservation::running([1; 32], 17, 100, 101, channel_id(4))
                    .expect("valid running observation"),
            );
        assert!(matches!(
            core.reload_host(registration("agent-host", 2), DesiredState::Running),
            Err(OrchestratorCoreError::HostStillActive(_))
        ));
        state
            .lock()
            .expect("supervisor mutex poisoned")
            .observations
            .insert(
                "agent-host".to_string(),
                SupervisorObservation::exited([1; 32], Some(0), Some(100), Some(101))
                    .expect("valid exited observation"),
            );
        assert_eq!(
            core.reload_host(registration("agent-host", 2), DesiredState::Stopped)
                .expect("reload exited host")
                .entry()
                .registration()
                .package_hash(),
            &[2; 32]
        );
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
            core.create_channel(&wiring_context(), &definition)
                .expect("create channel"),
            definition
        );
        assert_eq!(
            core.create_channel(&wiring_context(), &definition)
                .expect("idempotent channel creation"),
            definition
        );
        assert_eq!(
            core.load_channel(definition.channel_id())
                .expect("load channel"),
            Some(definition.clone())
        );
        let destroyed = core
            .destroy_channel(&wiring_context(), definition.channel_id())
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
            core.destroy_channel(&wiring_context(), definition.channel_id())
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
            core.create_channel(&wiring_context(), &definition),
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
            core.destroy_channel(&wiring_context(), definition.channel_id()),
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
    fn trust_checker_adapter_binds_the_complete_mutation_and_authoritative_tiers() {
        let definition = definition(7);
        let mut agents = BTreeMap::new();
        agents.insert(
            definition.originator().agent_id.as_bytes().to_vec(),
            PrivilegeTier::Tier1,
        );
        agents.insert(
            definition.receivers()[0].agent_id.as_bytes().to_vec(),
            PrivilegeTier::Tier2,
        );
        let resolver = FixedTierResolver {
            channel: PrivilegeTier::Tier0,
            pipeline: PrivilegeTier::Tier0,
            agents,
            fail: false,
        };
        let mut authorizer = TrustCheckingChannelWiring::new(
            RecordingApprovalProvider::approving(ApprovalAssurance::Biometric),
            resolver,
        );
        authorizer
            .authorize(&wiring_context(), ChannelWiringRequest::Create(&definition))
            .unwrap();
        let (provider, _) = authorizer.into_parts();
        let (request, requirement) = &provider.requests[0];
        assert_eq!(request.request_id(), "wire-request");
        assert_eq!(request.requested_by(), "operator:local");
        assert_eq!(request.effective_tier(), PrivilegeTier::Tier2);
        assert_eq!(request.resources().len(), 3);
        assert_eq!(
            *requirement,
            chief_of_staff_trust_checker::ApprovalRequirement::Biometric {
                timeout: std::time::Duration::from_secs(30)
            }
        );
        assert_eq!(
            request.resources()[0].resource_id(),
            mutation_resource_id(ChannelWiringRequest::Create(&definition))
        );
        assert_eq!(
            request.resources()[1].resource_id(),
            agent_resource_id(definition.originator().agent_id.as_bytes())
        );

        let replacement = ChannelDefinition::new(
            definition.channel_id(),
            OriginatorIdentity {
                agent_id: definition.originator().agent_id.clone(),
                public_key: [99; 32],
            },
            definition.receivers().to_vec(),
            definition.created_at_ns(),
            definition.key_epoch(),
        )
        .unwrap();
        assert_ne!(
            mutation_resource_id(ChannelWiringRequest::Create(&definition)),
            mutation_resource_id(ChannelWiringRequest::Create(&replacement))
        );
        assert_ne!(
            mutation_resource_id(ChannelWiringRequest::Create(&definition)),
            mutation_resource_id(ChannelWiringRequest::Destroy(&definition))
        );
    }

    #[test]
    fn tier_resolution_and_approval_fail_before_channel_storage_mutates() {
        let backend = Arc::new(InMemoryStorageBackend::new());
        let resolver = FixedTierResolver {
            channel: PrivilegeTier::Tier0,
            pipeline: PrivilegeTier::Tier0,
            agents: BTreeMap::new(),
            fail: true,
        };
        let authorizer = TrustCheckingChannelWiring::new(
            RecordingApprovalProvider::approving(ApprovalAssurance::HardwareKey),
            resolver,
        );
        let mut core = core(
            Arc::clone(&backend),
            FakeSupervisor::default(),
            authorizer,
            Arc::new(TestClock::new([])),
        );
        let definition = definition(8);
        let error = core
            .create_channel(&wiring_context(), &definition)
            .unwrap_err();
        assert!(matches!(
            error,
            OrchestratorCoreError::Authorization(TrustChannelWiringError::Resolver(FakeError(_)))
        ));
        assert_eq!(
            error.to_string(),
            "orchestrator-core: channel authorization failed"
        );
        assert_eq!(core.load_channel(definition.channel_id()).unwrap(), None);
    }

    #[test]
    fn maximum_channel_membership_fits_the_bounded_trust_request() {
        let receivers = (0..1024u16)
            .map(|index| ReceiverIdentity {
                agent_id: AgentId::new(index.to_be_bytes().to_vec()).unwrap(),
                public_key: [u8::try_from(index % 251).unwrap(); 32],
            })
            .collect();
        let definition = ChannelDefinition::new(
            channel_id(9),
            OriginatorIdentity {
                agent_id: AgentId::new(b"originator".to_vec()).unwrap(),
                public_key: [1; 32],
            },
            receivers,
            1,
            KeyEpoch(1),
        )
        .unwrap();
        let mut authorizer = TrustCheckingChannelWiring::new(
            RecordingApprovalProvider::approving(ApprovalAssurance::ExplicitConsent),
            FixedTierResolver::tier0(),
        );
        authorizer
            .authorize(&wiring_context(), ChannelWiringRequest::Create(&definition))
            .unwrap();
        assert!(authorizer.into_parts().0.requests.is_empty());
    }

    #[test]
    fn pipeline_bindings_are_authorized_before_wire_and_unwire() {
        let backend = Arc::new(InMemoryStorageBackend::new());
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut core = core(
            Arc::clone(&backend),
            FakeSupervisor::default(),
            FakeAuthorizer::allowing(Arc::clone(&events)),
            Arc::new(TestClock::new([])),
        );
        let registration = registration("pipeline-host", 41);
        core.register_host(registration.clone(), DesiredState::Stopped)
            .unwrap();
        let agent_id = AgentId::new(b"pipeline-agent".to_vec()).unwrap();
        let channels = pipeline_channels(backend.as_ref(), &agent_id);
        let binding = pipeline_binding(
            pipeline_id(41),
            registration.clone(),
            agent_id,
            channels,
            "local/model",
        );

        let first = core
            .wire_host_pipeline(&wiring_context(), &binding)
            .unwrap();
        let repeated = core
            .wire_host_pipeline(&wiring_context(), &binding)
            .unwrap();
        assert_eq!(first, repeated);
        assert_eq!(
            core.load_host_pipeline(registration.host_name())
                .unwrap()
                .unwrap()
                .binding(),
            &binding
        );
        assert_eq!(
            core.unwire_host_pipeline(&wiring_context(), registration.host_name())
                .unwrap(),
            Some(binding.clone())
        );
        assert_eq!(
            core.unwire_host_pipeline(&wiring_context(), registration.host_name())
                .unwrap(),
            None
        );
        assert_eq!(
            events
                .lock()
                .expect("authorization mutex poisoned")
                .as_slice(),
            [
                WiringEvent::Wire(pipeline_id(41)),
                WiringEvent::Wire(pipeline_id(41)),
                WiringEvent::Unwire(pipeline_id(41)),
            ]
        );
    }

    #[test]
    fn denied_pipeline_mutations_leave_bindings_and_claims_unchanged() {
        let backend = Arc::new(InMemoryStorageBackend::new());
        let registration = registration("denied-pipeline-host", 42);
        let agent_id = AgentId::new(b"denied-pipeline-agent".to_vec()).unwrap();
        ServiceRegistry::new(backend.as_ref())
            .register(&HostEntry::registered(
                registration.clone(),
                DesiredState::Stopped,
            ))
            .unwrap();
        let channels = pipeline_channels(backend.as_ref(), &agent_id);
        let denied_binding = pipeline_binding(
            pipeline_id(42),
            registration.clone(),
            agent_id.clone(),
            channels,
            "local/model",
        );
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut denied = core(
            Arc::clone(&backend),
            FakeSupervisor::default(),
            FakeAuthorizer::denying(Arc::clone(&events)),
            Arc::new(TestClock::new([])),
        );
        assert!(matches!(
            denied.wire_host_pipeline(&wiring_context(), &denied_binding),
            Err(OrchestratorCoreError::PipelineAuthorization(FakeError(
                "denied"
            )))
        ));
        assert_eq!(
            denied.load_host_pipeline(registration.host_name()).unwrap(),
            None
        );

        let allowed_binding = pipeline_binding(
            pipeline_id(43),
            registration.clone(),
            agent_id,
            channels,
            "local/model",
        );
        let mut allowed = core(
            Arc::clone(&backend),
            FakeSupervisor::default(),
            FakeAuthorizer::allowing(Arc::new(Mutex::new(Vec::new()))),
            Arc::new(TestClock::new([])),
        );
        allowed
            .wire_host_pipeline(&wiring_context(), &allowed_binding)
            .expect("denial must not leave an immutable channel claim");
        assert!(matches!(
            denied.unwire_host_pipeline(&wiring_context(), registration.host_name()),
            Err(OrchestratorCoreError::PipelineAuthorization(FakeError(
                "denied"
            )))
        ));
        assert_eq!(
            denied
                .load_host_pipeline(registration.host_name())
                .unwrap()
                .unwrap()
                .binding(),
            &allowed_binding
        );
    }

    #[test]
    fn trust_checker_binds_exact_pipeline_authority_and_tiers() {
        let registration = registration("trusted-pipeline-host", 44);
        let agent_id = AgentId::new(b"trusted-pipeline-agent".to_vec()).unwrap();
        let channels = (channel_id(44), channel_id(45));
        let binding = pipeline_binding(
            pipeline_id(44),
            registration.clone(),
            agent_id.clone(),
            channels,
            "local/model-a",
        );
        let mut agents = BTreeMap::new();
        agents.insert(agent_id.as_bytes().to_vec(), PrivilegeTier::Tier1);
        let resolver = FixedTierResolver {
            channel: PrivilegeTier::Tier0,
            pipeline: PrivilegeTier::Tier2,
            agents,
            fail: false,
        };
        let mut authorizer = TrustCheckingChannelWiring::new(
            RecordingApprovalProvider::approving(ApprovalAssurance::Biometric),
            resolver,
        );
        authorizer
            .authorize_pipeline(&wiring_context(), PipelineWiringRequest::Wire(&binding))
            .unwrap();
        let (provider, _) = authorizer.into_parts();
        let (request, requirement) = &provider.requests[0];
        assert_eq!(request.effective_tier(), PrivilegeTier::Tier2);
        assert_eq!(request.resources().len(), 2);
        assert_eq!(
            request.resources()[0].resource_id(),
            pipeline_mutation_resource_id(PipelineWiringRequest::Wire(&binding))
        );
        assert_eq!(
            request.resources()[1].resource_id(),
            agent_resource_id(agent_id.as_bytes())
        );
        assert_eq!(
            *requirement,
            chief_of_staff_trust_checker::ApprovalRequirement::Biometric {
                timeout: Duration::from_secs(30)
            }
        );

        let changed_model = pipeline_binding(
            binding.pipeline_id(),
            registration,
            agent_id,
            channels,
            "local/model-b",
        );
        assert_ne!(
            pipeline_mutation_resource_id(PipelineWiringRequest::Wire(&binding)),
            pipeline_mutation_resource_id(PipelineWiringRequest::Wire(&changed_model))
        );
        assert_ne!(
            pipeline_mutation_resource_id(PipelineWiringRequest::Wire(&binding)),
            pipeline_mutation_resource_id(PipelineWiringRequest::Unwire(&binding))
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
            OrchestratorCoreError::Pipeline(PipelineBindingError::ChannelUnavailable),
            OrchestratorCoreError::Authorization(FakeError("secret authorization payload")),
            OrchestratorCoreError::PipelineAuthorization(FakeError(
                "secret pipeline authorization payload",
            )),
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
            Arc::new(DenyHostLaunchBindings),
            Arc::new(UnavailableHostDataPlaneDispatcher),
            identity,
            clock,
            Box::<UuidV7SessionIdSource>::default(),
            ReconcileConfig::new(1, 100).expect("reconcile config"),
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
