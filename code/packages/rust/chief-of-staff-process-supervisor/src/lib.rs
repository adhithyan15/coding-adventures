//! Verified OS-process supervision for D18 Chief hosts.
//!
//! The service registry contains durable intent, not process authority. This
//! adapter re-verifies packages, owns child handles, carries the secure control
//! protocol over bounded pipes, and fails closed on transport or protocol error.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use chief_of_staff_broker_callbacks::InFlight;
use chief_of_staff_broker_launcher::{
    abandon_pending_on_write_channels, launch as launch_broker, start_relay, BrokerKeyFiles,
    BrokerRelay, HostGone, PinnedBindingResolver, RelayConfig, ResponseSink, VerifiedExecutable,
};
use chief_of_staff_channel_crypto::ChannelId;
use chief_of_staff_channel_endpoints::MessageMetadataSource;
use chief_of_staff_host_control_protocol::{
    ChildControl, ChildEvent, CompletionCall, DataPlaneFailure, DataPlaneRequest,
    DataPlaneResponse, LaunchBindings, ModelToolCall, OrchestratorControl, OrchestratorEvent,
    PackageTrust, PackageTrustType, ToolCompletionCall,
};
use chief_of_staff_host_data_plane::HostDataPlaneDispatcher;
use chief_of_staff_host_runtime::{
    verify_agent_package, AgentPackageRuntime, PackageKeyType, PackageKeyring, TrustedPackageKey,
};
use chief_of_staff_pipeline_bindings::{HostPipelineBinding, PipelineBindingStore, PipelineId};
use chief_of_staff_secure_host_channel::{
    BootstrapOffer, ChildBootstrap, ClientHello, HostId, OrchestratorBootstrap, SessionId,
};
use chief_of_staff_service_reconciler::{HostSupervisor, SupervisorObservation};
use chief_of_staff_service_registry::{HostName, HostRegistration};
use chief_of_staff_tool_api::PrivilegeTier;
use coding_adventures_x3dh::IdentityKeyPair;
use core::fmt::{self, Display, Formatter};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, TryRecvError};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use storage_core::StorageBackend;

mod request_budget;
pub use request_budget::RequestBudget;
use request_budget::TokenBucket;

const MAX_RECORD_BYTES: usize = 1024 * 1024;
const MAX_FIXED_ARGUMENTS: usize = 128;
const MAX_PENDING_RECORDS: usize = 64;
const STOP_POLL_INTERVAL: Duration = Duration::from_millis(10);
const PACKAGE_RUNTIME_ARGUMENT: &str = "--package-runtime";

/// Stable, input-independent process-supervision failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcessSupervisorError {
    /// Program, argument, or timeout configuration is invalid.
    InvalidConfiguration,
    /// Signed-package verification failed.
    PackageVerification,
    /// The verified package identity differs from the registered hash.
    PackageMismatch,
    /// A fresh valid UUID-v7 session could not be produced.
    SessionGeneration,
    /// Secure-channel bootstrap construction or authentication failed.
    Bootstrap,
    /// Process creation or required pipe acquisition failed.
    Spawn,
    /// A pipe read, write, flush, or process-management operation failed.
    ProcessIo,
    /// The child did not complete secure bootstrap before the deadline.
    BootstrapTimeout,
    /// A length-prefixed stream record was invalid or incomplete.
    Framing,
    /// Authenticated host-control processing failed closed.
    Control,
    /// The orchestrator requested graceful termination during a data-plane exchange.
    Terminated,
    /// Pipeline launch bindings were absent, invalid, or incompatible with the package runtime.
    LaunchBindings,
    /// A different package is already active under this host name.
    ActivePackageMismatch,
    /// No supervised process exists under the requested host name.
    HostNotFound,
    /// The agent's broker could not be launched, or its keys did not match
    /// its channel definitions (D18S P2.6d).
    BrokerLaunch,
    /// The agent's previous broker has not finished yet; try again later.
    BrokerBusy,
    /// The host's previous incarnation still has a dispatch running; try
    /// again later (D18S P2.6d-4).
    DispatchBusy,
    /// The agent's broker ended or misbehaved; its host was ended with it.
    Broker,
}

impl Display for ProcessSupervisorError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidConfiguration => "process-supervisor: invalid configuration",
            Self::PackageVerification => "process-supervisor: package verification failed",
            Self::PackageMismatch => "process-supervisor: package identity mismatch",
            Self::SessionGeneration => "process-supervisor: session generation failed",
            Self::Bootstrap => "process-supervisor: secure bootstrap failed",
            Self::Spawn => "process-supervisor: child spawn failed",
            Self::ProcessIo => "process-supervisor: process I/O failed",
            Self::BootstrapTimeout => "process-supervisor: bootstrap timed out",
            Self::Framing => "process-supervisor: invalid framed record",
            Self::Control => "process-supervisor: host-control failure",
            Self::Terminated => "process-supervisor: graceful termination requested",
            Self::LaunchBindings => "process-supervisor: launch bindings unavailable",
            Self::ActivePackageMismatch => "process-supervisor: active package identity mismatch",
            Self::HostNotFound => "process-supervisor: host not found",
            Self::BrokerLaunch => "process-supervisor: broker launch failed",
            Self::BrokerBusy => "process-supervisor: previous broker still finishing",
            Self::DispatchBusy => "process-supervisor: previous dispatch still finishing",
            Self::Broker => "process-supervisor: broker ended",
        })
    }
}

impl std::error::Error for ProcessSupervisorError {}

/// Redacted failure returned by a manifest-blind launch-binding authority.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LaunchBindingProviderError;

impl Display for LaunchBindingProviderError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("host launch bindings unavailable")
    }
}

impl std::error::Error for LaunchBindingProviderError {}

/// Injected manifest-blind authority for one registered host's launch bindings.
///
/// Implementations are expected to resolve durable pipeline wiring by immutable
/// host and package identity. The independently verifying child remains
/// responsible for matching returned names to its signed manifest.
pub trait HostLaunchBindingProvider: Send + Sync {
    /// Return the exact authorized bindings for this registered package launch.
    fn launch_bindings(
        &self,
        registration: &HostRegistration,
        runtime: AgentPackageRuntime,
    ) -> Result<LaunchBindings, LaunchBindingProviderError>;

    /// The whole resolved pipeline binding, which a broker serves (D18S
    /// P2.6d). Providers without durable bindings refuse.
    fn pipeline_binding(
        &self,
        _registration: &HostRegistration,
    ) -> Result<HostPipelineBinding, LaunchBindingProviderError> {
        Err(LaunchBindingProviderError)
    }
}

/// Fail-closed launch-binding provider for compositions without pipeline wiring.
#[derive(Default)]
pub struct DenyHostLaunchBindings;

impl HostLaunchBindingProvider for DenyHostLaunchBindings {
    fn launch_bindings(
        &self,
        _registration: &HostRegistration,
        _runtime: AgentPackageRuntime,
    ) -> Result<LaunchBindings, LaunchBindingProviderError> {
        Err(LaunchBindingProviderError)
    }
}

/// Storage-backed manifest-blind launch authority for production composition.
pub struct DurableHostLaunchBindings {
    backend: Arc<dyn StorageBackend>,
}

impl DurableHostLaunchBindings {
    /// Bind launch resolution to the daemon's durable storage backend.
    pub fn new(backend: Arc<dyn StorageBackend>) -> Self {
        Self { backend }
    }
}

impl HostLaunchBindingProvider for DurableHostLaunchBindings {
    fn launch_bindings(
        &self,
        registration: &HostRegistration,
        _runtime: AgentPackageRuntime,
    ) -> Result<LaunchBindings, LaunchBindingProviderError> {
        PipelineBindingStore::new(self.backend.as_ref())
            .resolve_launch(registration)
            .map_err(|_| LaunchBindingProviderError)
    }

    fn pipeline_binding(
        &self,
        registration: &HostRegistration,
    ) -> Result<HostPipelineBinding, LaunchBindingProviderError> {
        PipelineBindingStore::new(self.backend.as_ref())
            .resolve_launch_binding(registration)
            .map_err(|_| LaunchBindingProviderError)
    }
}

/// One shell-free executable plus bounded fixed arguments used for every host.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostProgram {
    executable: PathBuf,
    arguments: Vec<OsString>,
}

impl HostProgram {
    /// Validate a host executable and its fixed arguments.
    pub fn new<I, S>(
        executable: impl Into<PathBuf>,
        arguments: I,
    ) -> Result<Self, ProcessSupervisorError>
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        let executable = executable.into();
        let arguments = arguments.into_iter().map(Into::into).collect::<Vec<_>>();
        if !executable.is_absolute() || arguments.len() > MAX_FIXED_ARGUMENTS {
            return Err(ProcessSupervisorError::InvalidConfiguration);
        }
        Ok(Self {
            executable,
            arguments,
        })
    }

    /// Return the configured executable path.
    pub fn executable(&self) -> &Path {
        &self.executable
    }

    /// Return the shell-free fixed argument list.
    pub fn arguments(&self) -> &[OsString] {
        &self.arguments
    }
}

/// Validated launch and deadline configuration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessSupervisorConfig {
    program: HostProgram,
    bootstrap_timeout: Duration,
    graceful_stop_timeout: Duration,
}

impl ProcessSupervisorConfig {
    /// Construct configuration with non-zero bounded waits.
    pub fn new(
        program: HostProgram,
        bootstrap_timeout: Duration,
        graceful_stop_timeout: Duration,
    ) -> Result<Self, ProcessSupervisorError> {
        if bootstrap_timeout.is_zero() || graceful_stop_timeout.is_zero() {
            return Err(ProcessSupervisorError::InvalidConfiguration);
        }
        Ok(Self {
            program,
            bootstrap_timeout,
            graceful_stop_timeout,
        })
    }

    /// Return the configured host program.
    pub fn program(&self) -> &HostProgram {
        &self.program
    }

    /// Return the secure-bootstrap deadline.
    pub fn bootstrap_timeout(&self) -> Duration {
        self.bootstrap_timeout
    }

    /// Return the graceful-stop deadline.
    pub fn graceful_stop_timeout(&self) -> Duration {
        self.graceful_stop_timeout
    }
}

/// Trusted monotonic nanosecond source.
pub trait MonotonicClock: Send + Sync {
    /// Sample opaque monotonic nanoseconds.
    fn now_ns(&self) -> u64;
}

/// Everything the supervisor needs to give each agent its own broker (D18S
/// P2.6d-2b). Without it, channel requests go to the in-daemon dispatcher,
/// as before.
pub struct ChannelBrokers {
    program: Arc<VerifiedExecutable>,
    keys: BrokerKeyFiles,
    backend: Arc<dyn StorageBackend>,
    metadata: Arc<dyn MessageMetadataSource>,
    relay: RelayConfig,
    ready_timeout: Duration,
}

impl ChannelBrokers {
    /// Brokers launched from `program`, holding the keys `keys` names, over
    /// `backend`, with message ids and timestamps from `metadata`.
    pub fn new(
        program: Arc<VerifiedExecutable>,
        keys: BrokerKeyFiles,
        backend: Arc<dyn StorageBackend>,
        metadata: Arc<dyn MessageMetadataSource>,
        relay: RelayConfig,
        ready_timeout: Duration,
    ) -> Self {
        Self {
            program,
            keys,
            backend,
            metadata,
            relay,
            ready_timeout,
        }
    }
}

/// Monotonic clock measured from one process-local `Instant` origin.
pub struct SystemMonotonicClock {
    origin: Instant,
}

impl SystemMonotonicClock {
    /// Create a fresh process-local origin.
    pub fn new() -> Self {
        Self {
            origin: Instant::now(),
        }
    }
}

impl Default for SystemMonotonicClock {
    fn default() -> Self {
        Self::new()
    }
}

impl MonotonicClock for SystemMonotonicClock {
    fn now_ns(&self) -> u64 {
        self.origin.elapsed().as_nanos().min(u64::MAX as u128) as u64
    }
}

/// Source of fresh valid UUID-v7 secure-session identities.
pub trait SessionIdSource: Send {
    /// Return the next per-spawn session.
    fn next_session(&mut self) -> Result<SessionId, ProcessSupervisorError>;
}

/// Production UUID-v7 session source.
#[derive(Default)]
pub struct UuidV7SessionIdSource;

impl SessionIdSource for UuidV7SessionIdSource {
    fn next_session(&mut self) -> Result<SessionId, ProcessSupervisorError> {
        let uuid =
            coding_adventures_uuid::v7().map_err(|_| ProcessSupervisorError::SessionGeneration)?;
        SessionId::new(uuid.bytes()).map_err(|_| ProcessSupervisorError::SessionGeneration)
    }
}

enum ReaderEvent {
    Record { bytes: Vec<u8>, received_at_ns: u64 },
    Failure(ProcessSupervisorError),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InstancePhase {
    Starting,
    Running,
    Stopping,
    Exited { exit_code: Option<i32> },
}

/// The host's end of the secure channel: its control state, the writer to
/// its stdin, and the request awaiting an answer.
///
/// Shared, because two threads answer the host: the supervisor's, for the
/// dispatcher, and a broker's relay thread, for channel operations (D18S
/// P2.6d-2b). Encrypting a response and queueing it happen under one lock,
/// so frames reach the host in the order the secure channel numbered them.
/// The writer only queues (`try_send`), so the lock is never held across
/// blocking I/O.
struct HostLink {
    control: Option<OrchestratorControl>,
    writer: Option<RecordWriter>,
    pending: Option<DataPlaneRequest>,
    /// The pending request is being answered elsewhere: by the agent's
    /// broker's relay (P2.6d-2b), or by the host's dispatch worker
    /// (P2.6d-4). It is not offered to `pending_data_plane_request`, and
    /// `respond_data_plane` may not answer it (P2.6d-2b review round 1).
    delegated: bool,
    /// Why the dispatch worker could not answer, if it could not. The next
    /// refresh ends the host with it (P2.6d-4).
    fault: Option<ProcessSupervisorError>,
}

type SharedLink = Arc<Mutex<HostLink>>;

fn lock(link: &SharedLink) -> Result<MutexGuard<'_, HostLink>, ProcessSupervisorError> {
    link.lock().map_err(|_| ProcessSupervisorError::Control)
}

/// Answer the host's pending request on its link.
fn respond_on(
    link: &SharedLink,
    response: DataPlaneResponse,
) -> Result<(), ProcessSupervisorError> {
    let mut link = lock(link)?;
    let frame = link
        .control
        .as_mut()
        .ok_or(ProcessSupervisorError::Control)?
        .respond(response)
        .map_err(|_| ProcessSupervisorError::Control)?;
    link.writer
        .as_ref()
        .ok_or(ProcessSupervisorError::ProcessIo)?
        .send(frame)?;
    link.pending = None;
    link.delegated = false;
    Ok(())
}

/// One host's dispatch worker (D18S P2.6d-4).
///
/// Completions and tool calls can take seconds or minutes. Served on the
/// supervisor's thread, one slow request held every other host's requests
/// and heartbeats behind it. The worker takes the host's admitted
/// non-channel requests one at a time (the control protocol allows only
/// one outstanding request per host, so the queue holds one) and answers
/// through the shared link, as a broker's relay does.
struct DispatchWorker {
    /// `None` once the host has ended: nothing more is handed over.
    requests: Option<mpsc::SyncSender<DataPlaneRequest>>,
    /// Kept so a restart can wait for a dispatch still running (review
    /// round 1, L3): one host never has two workers at once.
    thread: JoinHandle<()>,
}

impl DispatchWorker {
    fn start(
        dispatcher: Arc<dyn HostDataPlaneDispatcher>,
        registration: HostRegistration,
        link: SharedLink,
    ) -> Result<Self, ProcessSupervisorError> {
        let (requests, queue) = mpsc::sync_channel::<DataPlaneRequest>(1);
        let thread = thread::Builder::new()
            .name("host-dispatch".into())
            .spawn(move || {
                let fail = |error| {
                    // The supervisor's thread ends the host on its next
                    // refresh, as the synchronous path did.
                    if let Ok(mut link) = link.lock() {
                        link.fault.get_or_insert(error);
                    }
                };
                // Ends when the host's end drops the sender. A dispatch
                // already running finishes first, bounded by the
                // dispatcher's own timeouts, and then finds the link closed.
                for request in queue {
                    // A request queued just before its host ended is not
                    // run for a host that is gone (review round 1, L4).
                    if link.lock().map_or(true, |link| link.control.is_none()) {
                        break;
                    }
                    // A dispatcher that panics must not leave the host
                    // waiting forever for an answer (review round 1, M1).
                    let response = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        dispatcher.dispatch(&registration, &request)
                    }));
                    let Ok(response) = response else {
                        fail(ProcessSupervisorError::Control);
                        break;
                    };
                    if let Err(error) = respond_on(&link, response) {
                        fail(error);
                        break;
                    }
                }
            })
            .map_err(|_| ProcessSupervisorError::ProcessIo)?;
        Ok(Self {
            requests: Some(requests),
            thread,
        })
    }

    /// Stop handing over requests. A dispatch already running finishes.
    fn close(&mut self) {
        self.requests = None;
    }

    /// Whether a dispatch may still be running.
    fn is_busy(&self) -> bool {
        !self.thread.is_finished()
    }

    /// Hand over one request. Refused only if the worker has stopped: the
    /// queue cannot be full while the protocol allows one request at a time.
    fn dispatch(&self, request: DataPlaneRequest) -> Result<(), ProcessSupervisorError> {
        self.requests
            .as_ref()
            .ok_or(ProcessSupervisorError::ProcessIo)?
            .try_send(request)
            .map_err(|_| ProcessSupervisorError::ProcessIo)
    }
}

/// Where a broker's relay delivers: the host's link.
struct LinkSink(SharedLink);

impl ResponseSink for LinkSink {
    fn deliver(&mut self, response: DataPlaneResponse) -> Result<(), HostGone> {
        respond_on(&self.0, response).map_err(|_| HostGone)
    }
}

/// One agent's broker, while its host lives (D18S P2.6d-2b).
struct OwnedBroker {
    child: Option<Child>,
    relay: BrokerRelay,
    identity: (PipelineId, Vec<u8>),
    /// Latched once an end is seen: the relay's report is consumed when it
    /// is read, and the host must still be ended on a later refresh if
    /// ending it failed this time (P2.6d-2b review round 1).
    ended: bool,
}

impl OwnedBroker {
    /// Kill and reap the broker, then stop its relay. Killing first is what
    /// lets the relay's joins finish at once.
    fn end(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = chief_of_staff_spawn_isolation::kill_session(&child);
            let _ = child.kill();
            let _ = child.wait();
        }
        let _ = self.relay.stop();
    }

    /// Whether anything of it may still act: the process, or a relay thread
    /// that has not finished (it commits on the broker's behalf).
    fn is_live(&self) -> bool {
        self.child.is_some() || !self.relay.is_finished()
    }

    /// Whether it has ended on its own: its relay reported an end, or its
    /// process exited. The exit is seen without reaping, so the session
    /// kill that follows still reaches the right group.
    fn has_ended(&mut self) -> bool {
        if !self.ended {
            self.ended = self.relay.ended().is_some()
                || self.child.as_ref().is_some_and(|child| {
                    matches!(chief_of_staff_spawn_isolation::has_exited(child), Ok(true))
                });
        }
        self.ended
    }
}

struct OwnedInstance {
    package_hash: [u8; 32],
    child: Option<Child>,
    reader: Option<JoinHandle<()>>,
    records: Receiver<ReaderEvent>,
    link: SharedLink,
    broker: Option<OwnedBroker>,
    /// Serves the host's non-channel requests, when a dispatcher is set.
    worker: Option<DispatchWorker>,
    phase: InstancePhase,
    process_id: u32,
    started_at_ns: u64,
    last_heartbeat_ns: Option<u64>,
    channel_id: ChannelId,
    /// This host's request budget (D18S S-K5, P2.6b).
    requests: TokenBucket,
    /// When a host still `Starting` is ended: it has had the bootstrap
    /// timeout to say it is ready (review round 9).
    ready_deadline: Instant,
    /// How many of this host's requests the budget refused.
    rate_limited: u64,
}

impl OwnedInstance {
    fn is_active(&self) -> bool {
        !matches!(self.phase, InstancePhase::Exited { .. })
    }

    /// Reap the host if it has exited, killing its session first.
    ///
    /// Whatever the host left behind dies with it (review round 8): a
    /// descendant holding its stdout would keep the reader from end-of-file,
    /// and one holding its stdin would keep the writer blocked. The exit is
    /// seen with `has_exited`, which does not reap, so the session kill runs
    /// while the pid, and so the group id, is still the host's (round 9).
    fn try_reap(&mut self) -> Result<Option<ExitStatus>, ProcessSupervisorError> {
        let Some(child) = self.child.as_mut() else {
            return Ok(None);
        };
        // On Unix, reap only an exit `has_exited` saw, so no reap can slip
        // in between the check and the kill (review round 10). An error
        // means the child was already reaped (ECHILD), and `try_wait`
        // returns its cached status. On Windows `has_exited` is always
        // false, and `try_wait` decides alone.
        #[cfg(unix)]
        match chief_of_staff_spawn_isolation::has_exited(child) {
            Ok(false) => return Ok(None),
            Ok(true) => {
                let _ = chief_of_staff_spawn_isolation::kill_session(child);
            }
            Err(_) => {}
        }
        child
            .try_wait()
            .map_err(|_| ProcessSupervisorError::ProcessIo)
    }

    fn finish_exit(&mut self, status: ExitStatus) {
        self.child.take();
        if let Ok(mut link) = self.link.lock() {
            link.writer.take();
        }
        if let Some(reader) = self.reader.take() {
            join_bounded(reader);
        }
        if let Ok(mut link) = self.link.lock() {
            link.control.take();
            link.pending = None;
            link.delegated = false;
        }
        // The host's end ends its broker, and closes its dispatch worker's
        // queue. A dispatch already running is left to finish: it then
        // finds the link closed.
        if let Some(broker) = self.broker.as_mut() {
            broker.end();
        }
        if let Some(worker) = self.worker.as_mut() {
            worker.close();
        }
        self.phase = InstancePhase::Exited {
            exit_code: status.code(),
        };
    }

    fn hard_kill_and_reap(&mut self) -> Result<(), ProcessSupervisorError> {
        if let Ok(mut link) = self.link.lock() {
            link.writer.take();
        }
        let status = match self.try_reap()? {
            Some(status) => status,
            None => {
                let Some(child) = self.child.as_mut() else {
                    return Ok(());
                };
                // The whole session, then the child itself (a no-op if the
                // group kill already reached it), both before the reap.
                let _ = chief_of_staff_spawn_isolation::kill_session(child);
                child
                    .kill()
                    .map_err(|_| ProcessSupervisorError::ProcessIo)?;
                child
                    .wait()
                    .map_err(|_| ProcessSupervisorError::ProcessIo)?
            }
        };
        self.finish_exit(status);
        Ok(())
    }

    fn refresh(&mut self) -> Result<(), ProcessSupervisorError> {
        if matches!(self.phase, InstancePhase::Exited { .. }) {
            return Ok(());
        }
        // Capped (review round 7): a host writing faster than the
        // supervisor decrypts would otherwise keep this loop running, and
        // with it the one thread that drives every host. The rest waits for
        // the next refresh.
        self.end_on_fault()?;
        self.drain_records(Some(MAX_RECORDS_PER_DRAIN))?;
        self.end_on_fault()?;

        // The broker's end, or misbehaviour, ends its host.
        // The broker is ended here, whatever happens to the host: if
        // ending the host fails, the latched end tries again next refresh.
        if self.broker.as_mut().is_some_and(OwnedBroker::has_ended) {
            if let Some(broker) = self.broker.as_mut() {
                broker.end();
            }
            let _ = self.hard_kill_and_reap();
            return Err(ProcessSupervisorError::Broker);
        }

        if let Some(status) = self.try_reap()? {
            // `try_reap` has already killed the host's session, so the
            // reader sees end-of-file.
            //
            // The child can exit between the drain above and this
            // `try_wait`, while the reader thread is still on its way to
            // delivering the child's last records or the end-of-stream
            // failure.  Once the phase is `Exited`, `refresh` never looks
            // at the channel again, so settling now would silently turn
            // "exited before ready" into a clean exit:
            //
            //   supervisor                 reader thread
            //   ----------                 -------------
            //   drain: channel empty
            //                              read_record -> EOF
            //   try_wait: exited
            //   finish_exit -> Exited      send(Failure)   <- never read
            //
            // Joining the reader first means every event it will ever
            // send is already queued, and the second drain surfaces it.
            // This is the same join `finish_exit` always performed, just
            // moved ahead of the drain, so it waits no longer than before.  A failure found here reaps through
            // `hard_kill_and_reap`, which sees the exit status and
            // finishes the exit itself.
            if let Some(reader) = self.reader.take() {
                join_bounded(reader);
            }
            // Uncapped: the reader is joined, so the queue is final, and
            // bounded by the reader channel's capacity.
            self.drain_records(None)?;
            self.finish_exit(status);
            return Ok(());
        }
        // A host that never becomes ready is ended (review round 9). It has
        // had the bootstrap timeout to send Ready since its spawn.
        if self.phase == InstancePhase::Starting && Instant::now() >= self.ready_deadline {
            let _ = self.hard_kill_and_reap();
            return Err(ProcessSupervisorError::BootstrapTimeout);
        }
        Ok(())
    }

    /// Apply every event the reader thread has queued so far, failing closed
    /// (hard kill + reap) on the first framing, control, or dispatch error.
    fn drain_records(&mut self, limit: Option<usize>) -> Result<(), ProcessSupervisorError> {
        let mut handled = 0usize;
        loop {
            if limit.is_some_and(|limit| handled >= limit) {
                break;
            }
            handled += 1;
            match self.records.try_recv() {
                Ok(ReaderEvent::Record {
                    bytes,
                    received_at_ns,
                }) => {
                    let event = lock(&self.link).and_then(|mut link| {
                        link.control
                            .as_mut()
                            .ok_or(ProcessSupervisorError::Control)?
                            .receive_child(&bytes, received_at_ns)
                            .map_err(|_| ProcessSupervisorError::Control)
                    });
                    match event {
                        Ok(ChildEvent::Ready { received_at_ns, .. })
                        | Ok(ChildEvent::Heartbeat { received_at_ns }) => {
                            self.phase = InstancePhase::Running;
                            self.last_heartbeat_ns = Some(received_at_ns);
                        }
                        Ok(ChildEvent::Request(request)) => {
                            // Over budget: answered at once, never queued or
                            // dispatched. Hosts treat Unavailable as "idle,
                            // retry later", so a polite host never notices.
                            if !self.requests.take(received_at_ns) {
                                self.rate_limited = self.rate_limited.saturating_add(1);
                                let refusal = DataPlaneResponse::Failed {
                                    id: request.id(),
                                    failure: DataPlaneFailure::Unavailable,
                                };
                                if let Err(error) = self.send_data_plane_response(refusal) {
                                    let _ = self.hard_kill_and_reap();
                                    return Err(error);
                                }
                                continue;
                            }
                            // A channel operation goes to the agent's broker,
                            // never waited on here: its relay answers the
                            // host itself (D18S P2.6d-2b).
                            let relayed =
                                self.broker.is_some() && InFlight::for_request(&request).is_some();
                            // Everything else goes to the dispatch worker,
                            // also never waited on here (D18S P2.6d-4).
                            let delegated = relayed || self.worker.is_some();
                            let stored = lock(&self.link).map(|mut link| {
                                link.pending = Some(request.clone());
                                link.delegated = delegated;
                            });
                            if let Err(error) = stored {
                                let _ = self.hard_kill_and_reap();
                                return Err(error);
                            }
                            if let Some(broker) = self.broker.as_ref().filter(|_| relayed) {
                                if broker.relay.relay(request).is_err() {
                                    let _ = self.hard_kill_and_reap();
                                    return Err(ProcessSupervisorError::Broker);
                                }
                                continue;
                            }
                            if let Some(worker) = self.worker.as_ref() {
                                if let Err(error) = worker.dispatch(request) {
                                    let _ = self.hard_kill_and_reap();
                                    return Err(error);
                                }
                            }
                        }
                        Err(error) => {
                            let _ = self.hard_kill_and_reap();
                            return Err(error);
                        }
                    }
                }
                Ok(ReaderEvent::Failure(error)) => {
                    let _ = self.hard_kill_and_reap();
                    return Err(error);
                }
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }

        Ok(())
    }

    /// A response the dispatch worker could not deliver ends the host, as
    /// it did when dispatch ran here (D18S P2.6d-4). The fault stays latched
    /// on the link, so if ending the host fails now, the next refresh tries
    /// again (review round 1, L2); once the host has exited, refresh no
    /// longer looks.
    fn end_on_fault(&mut self) -> Result<(), ProcessSupervisorError> {
        let fault = lock(&self.link)?.fault;
        match fault {
            Some(fault) => {
                let _ = self.hard_kill_and_reap();
                Err(fault)
            }
            None => Ok(()),
        }
    }

    fn send_data_plane_response(
        &mut self,
        response: DataPlaneResponse,
    ) -> Result<(), ProcessSupervisorError> {
        respond_on(&self.link, response)
    }

    fn observation(&self) -> Result<SupervisorObservation, ProcessSupervisorError> {
        let result = match self.phase {
            InstancePhase::Starting => SupervisorObservation::starting(
                self.package_hash,
                self.process_id,
                self.started_at_ns,
                None,
                None,
            ),
            InstancePhase::Running => SupervisorObservation::running(
                self.package_hash,
                self.process_id,
                self.started_at_ns,
                self.last_heartbeat_ns
                    .ok_or(ProcessSupervisorError::Control)?,
                self.channel_id,
            ),
            InstancePhase::Stopping => SupervisorObservation::stopping(
                self.package_hash,
                self.process_id,
                self.started_at_ns,
                self.last_heartbeat_ns,
                Some(self.channel_id),
            ),
            InstancePhase::Exited { exit_code } => SupervisorObservation::exited(
                self.package_hash,
                exit_code,
                Some(self.started_at_ns),
                self.last_heartbeat_ns,
            ),
        };
        result.map_err(|_| ProcessSupervisorError::Control)
    }
}

/// Concrete verified process authority for the D18 service reconciler.
pub struct ProcessHostSupervisor {
    config: ProcessSupervisorConfig,
    keyring: Arc<PackageKeyring>,
    launch_bindings: Arc<dyn HostLaunchBindingProvider>,
    identity: Arc<IdentityKeyPair>,
    clock: Arc<dyn MonotonicClock>,
    sessions: Box<dyn SessionIdSource>,
    data_plane_dispatcher: Option<Arc<dyn HostDataPlaneDispatcher>>,
    channel_brokers: Option<ChannelBrokers>,
    request_budget: RequestBudget,
    instances: BTreeMap<String, OwnedInstance>,
}

impl ProcessHostSupervisor {
    /// Construct a supervisor around injected package trust, identity, time, and sessions.
    pub fn new(
        config: ProcessSupervisorConfig,
        keyring: Arc<PackageKeyring>,
        launch_bindings: Arc<dyn HostLaunchBindingProvider>,
        identity: Arc<IdentityKeyPair>,
        clock: Arc<dyn MonotonicClock>,
        sessions: Box<dyn SessionIdSource>,
    ) -> Self {
        Self {
            config,
            keyring,
            launch_bindings,
            identity,
            clock,
            sessions,
            data_plane_dispatcher: None,
            channel_brokers: None,
            request_budget: RequestBudget::DEFAULT,
            instances: BTreeMap::new(),
        }
    }

    /// Set the per-host request budget (D18S S-K5). Applies to hosts
    /// started afterwards; the default is [`RequestBudget::DEFAULT`].
    /// A zero `burst` refuses every request, and a zero `per_second` refuses
    /// every request after the first burst: both are fail-closed
    /// misconfigurations, not unlimited.
    pub fn with_request_budget(mut self, budget: RequestBudget) -> Self {
        self.request_budget = budget;
        self
    }

    /// How many of a host's data-plane requests its budget has refused,
    /// for the audit record.
    pub fn rate_limited_requests(
        &self,
        host_name: &HostName,
    ) -> Result<u64, ProcessSupervisorError> {
        self.instances
            .get(host_name.as_str())
            .map(|instance| instance.rate_limited)
            .ok_or(ProcessSupervisorError::HostNotFound)
    }

    /// Give every agent with bound channels its own broker (D18S P2.6d-2b).
    ///
    /// Each such agent's broker is launched before its host, holding only
    /// that agent's channel keys. The host's Receive, Publish and
    /// Acknowledge requests go to it; everything else still goes to the
    /// dispatcher. The host's end ends the broker, and the broker's end, or
    /// any misbehaviour, ends the host.
    pub fn with_channel_brokers(mut self, brokers: ChannelBrokers) -> Self {
        self.channel_brokers = Some(brokers);
        self
    }

    /// The process id of a host's broker, while it runs: for the audit
    /// record and for tests.
    pub fn broker_process_id(&self, host_name: &HostName) -> Option<u32> {
        self.instances
            .get(host_name.as_str())?
            .broker
            .as_ref()?
            .child
            .as_ref()
            .map(Child::id)
    }

    /// Automatically answer authenticated child requests through one injected dispatcher.
    pub fn with_data_plane_dispatcher(
        mut self,
        dispatcher: Arc<dyn HostDataPlaneDispatcher>,
    ) -> Self {
        self.data_plane_dispatcher = Some(dispatcher);
        self
    }

    fn spawn_verified(
        &mut self,
        registration: &HostRegistration,
    ) -> Result<OwnedInstance, ProcessSupervisorError> {
        let package_path = Path::new(registration.package_path().as_str());
        let package = verify_agent_package(package_path, self.keyring.as_ref())
            .map_err(|_| ProcessSupervisorError::PackageVerification)?;
        if package.digest() != *registration.package_hash() {
            return Err(ProcessSupervisorError::PackageMismatch);
        }
        let package_trust = self
            .keyring
            .trusted_key(package.key_id())
            .ok_or(ProcessSupervisorError::PackageVerification)
            .and_then(package_trust_record)?;
        let launch_bindings = self
            .launch_bindings
            .launch_bindings(registration, package.runtime())
            .map_err(|_| ProcessSupervisorError::LaunchBindings)?;
        validate_runtime_bindings(package.runtime(), &launch_bindings)?;

        let broker = self.launch_broker_for(registration, &launch_bindings)?;
        let mut instance = match self.spawn_host(
            registration,
            package.runtime(),
            package.path().to_path_buf(),
            package_trust,
            launch_bindings,
        ) {
            Ok(instance) => instance,
            Err(error) => {
                if let Some((_, _, launched)) = broker {
                    launched.discard();
                }
                return Err(error);
            }
        };
        if let Some((binding, identity, launched)) = broker {
            let Some(brokers) = self.channel_brokers.as_ref() else {
                launched.discard();
                let _ = instance.hard_kill_and_reap();
                return Err(ProcessSupervisorError::BrokerLaunch);
            };
            let provider = Arc::clone(&self.launch_bindings);
            let resolve_as = registration.clone();
            let resolver = PinnedBindingResolver::new(
                move || provider.pipeline_binding(&resolve_as).ok(),
                &binding,
            );
            match start_relay(
                launched,
                Arc::clone(&brokers.backend),
                Arc::clone(&brokers.metadata),
                Box::new(resolver),
                Box::new(LinkSink(Arc::clone(&instance.link))),
                brokers.relay,
            ) {
                Ok((child, relay)) => {
                    instance.broker = Some(OwnedBroker {
                        child: Some(child),
                        relay,
                        identity,
                        ended: false,
                    });
                }
                Err(_) => {
                    let _ = instance.hard_kill_and_reap();
                    return Err(ProcessSupervisorError::BrokerLaunch);
                }
            }
        }
        Ok(instance)
    }

    /// Launch the agent's broker, if brokers are configured and the agent
    /// has bound channels (D18S P2.6d-2b).
    ///
    /// At most one live broker per agent: while a previous one, or its
    /// relay, is still finishing, the launch is refused and the reconciler
    /// retries. Only then are the agent's pending reservations abandoned, so
    /// no late commit can race the abandon.
    #[allow(clippy::type_complexity)]
    fn launch_broker_for(
        &self,
        registration: &HostRegistration,
        launch_bindings: &LaunchBindings,
    ) -> Result<
        Option<(
            HostPipelineBinding,
            (PipelineId, Vec<u8>),
            chief_of_staff_broker_launcher::LaunchedBroker,
        )>,
        ProcessSupervisorError,
    > {
        let Some(brokers) = self.channel_brokers.as_ref() else {
            return Ok(None);
        };
        let binding = self
            .launch_bindings
            .pipeline_binding(registration)
            .map_err(|_| ProcessSupervisorError::LaunchBindings)?;
        // Host and broker must see one resolution, not two.
        if binding.launch_bindings() != launch_bindings {
            return Err(ProcessSupervisorError::LaunchBindings);
        }
        if binding.launch_bindings().channels().is_empty() {
            return Ok(None);
        }
        let identity = (
            binding.pipeline_id(),
            binding.agent_id().as_bytes().to_vec(),
        );
        let busy = self.instances.values().any(|instance| {
            instance
                .broker
                .as_ref()
                .is_some_and(|broker| broker.identity == identity && broker.is_live())
        });
        if busy {
            return Err(ProcessSupervisorError::BrokerBusy);
        }
        abandon_pending_on_write_channels(brokers.backend.as_ref(), &binding)
            .map_err(|_| ProcessSupervisorError::BrokerLaunch)?;
        let launched = launch_broker(
            &brokers.program,
            &binding,
            &brokers.keys,
            brokers.backend.as_ref(),
            brokers.ready_timeout,
        )
        .map_err(|_| ProcessSupervisorError::BrokerLaunch)?;
        Ok(Some((binding, identity, launched)))
    }

    /// Spawn the host process and complete its secure bootstrap.
    fn spawn_host(
        &mut self,
        registration: &HostRegistration,
        runtime: AgentPackageRuntime,
        package_dir: PathBuf,
        package_trust: PackageTrust,
        launch_bindings: LaunchBindings,
    ) -> Result<OwnedInstance, ProcessSupervisorError> {
        let request_budget = self.request_budget;
        let ready_timeout = self.config.bootstrap_timeout;
        let dispatcher = self.data_plane_dispatcher.clone();
        let session = self.sessions.next_session()?;
        let host = HostId::new(registration.host_name().as_str().to_owned())
            .map_err(|_| ProcessSupervisorError::Bootstrap)?;
        let bootstrap = OrchestratorBootstrap::new(self.identity.as_ref(), host, session)
            .map_err(|_| ProcessSupervisorError::Bootstrap)?;
        let offer = bootstrap
            .offer()
            .map_err(|_| ProcessSupervisorError::Bootstrap)?;

        let mut command = Command::new(self.config.program.executable());
        command
            .args(self.config.program.arguments())
            .arg(PACKAGE_RUNTIME_ARGUMENT)
            .arg(package_runtime_label(runtime))
            .current_dir(&package_dir)
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped());
        // D18S S-I2, S-I3. fd 2 goes to /dev/null rather than the daemon's own
        // stderr, which may be a terminal or the journal: an agent-to-
        // supervisor byte channel the broker never sees. Nothing above fd 2
        // is inherited, and a terminal on fd 0-2 refuses the spawn.
        chief_of_staff_spawn_isolation::isolate(&mut command);
        let mut child = command.spawn().map_err(|_| ProcessSupervisorError::Spawn)?;
        let process_id = child.id();
        let started_at_ns = self.clock.now_ns();
        let child_stdin = match child.stdin.take() {
            Some(stdin) => stdin,
            None => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(ProcessSupervisorError::Spawn);
            }
        };
        let child_stdout = match child.stdout.take() {
            Some(stdout) => stdout,
            None => {
                drop(child_stdin);
                let _ = child.kill();
                let _ = child.wait();
                return Err(ProcessSupervisorError::Spawn);
            }
        };
        // From the first byte, the host's stdin is written by its own thread
        // (review round 8, L2): a host that sends its hello and then stops
        // reading cannot block the supervisor in startup either. A frame it
        // never reads leaves the bootstrap timeout (no hello) or the
        // readiness deadline in `refresh` (no Ready) to end it.
        let stdin = match RecordWriter::start(BufWriter::new(child_stdin)) {
            Ok(stdin) => stdin,
            Err(error) => {
                let _ = chief_of_staff_spawn_isolation::kill_session(&child);
                let _ = child.kill();
                let _ = child.wait();
                return Err(error);
            }
        };
        let (sender, records) = mpsc::sync_channel(MAX_PENDING_RECORDS);
        let clock = Arc::clone(&self.clock);
        let reader = thread::spawn(move || {
            let mut stdout = BufReader::new(child_stdout);
            loop {
                match read_record(&mut stdout) {
                    Ok(bytes) => {
                        let received_at_ns = clock.now_ns();
                        if sender
                            .send(ReaderEvent::Record {
                                bytes,
                                received_at_ns,
                            })
                            .is_err()
                        {
                            break;
                        }
                    }
                    Err(error) => {
                        let _ = sender.send(ReaderEvent::Failure(error));
                        break;
                    }
                }
            }
        });

        let startup = (|| {
            stdin.send(offer.as_bytes().to_vec())?;
            let hello = match records.recv_timeout(self.config.bootstrap_timeout) {
                Ok(ReaderEvent::Record { bytes, .. }) => ClientHello::from_bytes(&bytes)
                    .map_err(|_| ProcessSupervisorError::Bootstrap)?,
                Ok(ReaderEvent::Failure(error)) => return Err(error),
                Err(RecvTimeoutError::Timeout) => {
                    return Err(ProcessSupervisorError::BootstrapTimeout)
                }
                Err(RecvTimeoutError::Disconnected) => {
                    return Err(ProcessSupervisorError::ProcessIo)
                }
            };
            let channel = bootstrap
                .accept(&hello)
                .map_err(|_| ProcessSupervisorError::Bootstrap)?;
            let mut control = OrchestratorControl::new(channel, *registration.package_hash())
                .map_err(|_| ProcessSupervisorError::Control)?;
            let trust = control
                .provide_package_trust(package_trust)
                .map_err(|_| ProcessSupervisorError::Control)?;
            stdin.send(trust)?;
            let bindings = control
                .provide_launch_bindings(launch_bindings)
                .map_err(|_| ProcessSupervisorError::Control)?;
            stdin.send(bindings)?;
            Ok(control)
        })();

        match startup {
            Ok(control) => {
                let channel_id = ChannelId(control.session_id().as_bytes());
                let link = Arc::new(Mutex::new(HostLink {
                    control: Some(control),
                    writer: Some(stdin),
                    pending: None,
                    delegated: false,
                    fault: None,
                }));
                // D18S P2.6d-4: the host's non-channel requests are served
                // off the supervisor's thread.
                let worker = match dispatcher
                    .map(|dispatcher| {
                        DispatchWorker::start(dispatcher, registration.clone(), Arc::clone(&link))
                    })
                    .transpose()
                {
                    Ok(worker) => worker,
                    Err(error) => {
                        if let Ok(mut link) = link.lock() {
                            link.writer.take();
                        }
                        let _ = chief_of_staff_spawn_isolation::kill_session(&child);
                        let _ = child.kill();
                        let _ = child.wait();
                        join_bounded(reader);
                        return Err(error);
                    }
                };
                Ok(OwnedInstance {
                    package_hash: *registration.package_hash(),
                    child: Some(child),
                    reader: Some(reader),
                    records,
                    channel_id,
                    link,
                    broker: None,
                    worker,
                    phase: InstancePhase::Starting,
                    process_id,
                    started_at_ns,
                    last_heartbeat_ns: None,
                    requests: TokenBucket::new(request_budget),
                    ready_deadline: Instant::now() + ready_timeout,
                    rate_limited: 0,
                })
            }
            Err(error) => {
                drop(stdin);
                let _ = chief_of_staff_spawn_isolation::kill_session(&child);
                let _ = child.kill();
                let _ = child.wait();
                join_bounded(reader);
                Err(error)
            }
        }
    }
}

fn package_runtime_label(runtime: AgentPackageRuntime) -> &'static str {
    match runtime {
        AgentPackageRuntime::Deno => "deno",
        AgentPackageRuntime::Skill => "skill",
    }
}

impl ProcessHostSupervisor {
    /// Return the one authenticated request awaiting service for a host.
    ///
    /// The request remains pending until [`Self::respond_data_plane`] succeeds,
    /// so an adapter can retry its own service lookup without losing correlation.
    pub fn pending_data_plane_request(
        &mut self,
        host_name: &HostName,
    ) -> Result<Option<DataPlaneRequest>, ProcessSupervisorError> {
        let instance = self
            .instances
            .get_mut(host_name.as_str())
            .ok_or(ProcessSupervisorError::HostNotFound)?;
        instance.refresh()?;
        let link = lock(&instance.link)?;
        Ok(link.pending.clone().filter(|_| !link.delegated))
    }

    /// Whether a host has a request awaiting its answer, wherever it is being
    /// answered: for the audit record and for tests.
    pub fn data_plane_request_in_flight(
        &mut self,
        host_name: &HostName,
    ) -> Result<bool, ProcessSupervisorError> {
        let instance = self
            .instances
            .get_mut(host_name.as_str())
            .ok_or(ProcessSupervisorError::HostNotFound)?;
        instance.refresh()?;
        let in_flight = lock(&instance.link)?.pending.is_some();
        Ok(in_flight)
    }

    /// Send the exact correlated response for a host's pending request.
    pub fn respond_data_plane(
        &mut self,
        host_name: &HostName,
        response: DataPlaneResponse,
    ) -> Result<(), ProcessSupervisorError> {
        let instance = self
            .instances
            .get_mut(host_name.as_str())
            .ok_or(ProcessSupervisorError::HostNotFound)?;
        instance.refresh()?;
        // A request with the broker is the broker's to answer. Refused
        // without ending the host: the host did nothing wrong.
        if lock(&instance.link)?.delegated {
            return Err(ProcessSupervisorError::Control);
        }
        if let Err(error) = instance.send_data_plane_response(response) {
            let _ = instance.hard_kill_and_reap();
            return Err(error);
        }
        Ok(())
    }
}

impl HostSupervisor for ProcessHostSupervisor {
    type Error = ProcessSupervisorError;

    fn inspect(
        &mut self,
        registration: &HostRegistration,
    ) -> Result<SupervisorObservation, Self::Error> {
        let Some(instance) = self.instances.get_mut(registration.host_name().as_str()) else {
            return Ok(SupervisorObservation::absent());
        };
        instance.refresh()?;
        instance.observation()
    }

    fn start(&mut self, registration: &HostRegistration) -> Result<(), Self::Error> {
        if let Some(instance) = self.instances.get_mut(registration.host_name().as_str()) {
            instance.refresh()?;
            if instance.is_active() {
                return if instance.package_hash == *registration.package_hash() {
                    Ok(())
                } else {
                    Err(ProcessSupervisorError::ActivePackageMismatch)
                };
            }
            // One worker per host at a time: a restart waits until the
            // previous incarnation's dispatch has finished (review round 1,
            // L3). The reconciler retries.
            if instance
                .worker
                .as_ref()
                .is_some_and(DispatchWorker::is_busy)
            {
                return Err(ProcessSupervisorError::DispatchBusy);
            }
        }
        let instance = self.spawn_verified(registration)?;
        self.instances
            .insert(registration.host_name().as_str().to_owned(), instance);
        Ok(())
    }

    fn stop(&mut self, host_name: &HostName) -> Result<(), Self::Error> {
        let Some(instance) = self.instances.get_mut(host_name.as_str()) else {
            return Ok(());
        };
        instance.refresh()?;
        if matches!(
            instance.phase,
            InstancePhase::Stopping | InstancePhase::Exited { .. }
        ) {
            return Ok(());
        }

        instance.phase = InstancePhase::Stopping;
        let write_result = lock(&instance.link).and_then(|mut link| {
            let frame = link
                .control
                .as_mut()
                .ok_or(ProcessSupervisorError::Control)?
                .terminate()
                .map_err(|_| ProcessSupervisorError::Control)?;
            link.writer
                .as_ref()
                .ok_or(ProcessSupervisorError::ProcessIo)?
                .send(frame)
        });
        if let Err(error) = write_result {
            let _ = instance.hard_kill_and_reap();
            return Err(error);
        }

        let deadline = Instant::now() + self.config.graceful_stop_timeout;
        loop {
            if instance.child.is_none() {
                return Err(ProcessSupervisorError::ProcessIo);
            }
            if let Some(status) = instance.try_reap()? {
                instance.finish_exit(status);
                return Ok(());
            }
            let now = Instant::now();
            if now >= deadline {
                return instance.hard_kill_and_reap();
            }
            thread::sleep(STOP_POLL_INTERVAL.min(deadline - now));
        }
    }
}

impl Drop for ProcessHostSupervisor {
    fn drop(&mut self) {
        for instance in self.instances.values_mut() {
            if instance.is_active() {
                let _ = instance.hard_kill_and_reap();
            }
        }
    }
}

/// Child-side secure bootstrap and lifecycle protocol over caller-owned streams.
pub struct ChildProcessControl<R: Read, W: Write> {
    reader: R,
    writer: W,
    control: ChildControl,
}

impl<R: Read, W: Write> ChildProcessControl<R, W> {
    /// Read one offer, authenticate it, write one hello, and retain the live channel.
    pub fn bootstrap(mut reader: R, mut writer: W) -> Result<Self, ProcessSupervisorError> {
        let offer = read_record(&mut reader)?;
        let offer =
            BootstrapOffer::from_bytes(&offer).map_err(|_| ProcessSupervisorError::Bootstrap)?;
        let (channel, hello) =
            ChildBootstrap::open(&offer).map_err(|_| ProcessSupervisorError::Bootstrap)?;
        write_record(&mut writer, hello.as_bytes())?;
        let control = ChildControl::new(channel).map_err(|_| ProcessSupervisorError::Control)?;
        Ok(Self {
            reader,
            writer,
            control,
        })
    }

    /// Receive the exact authenticated public trust required for package verification.
    pub fn receive_package_trust(&mut self) -> Result<TrustedPackageKey, ProcessSupervisorError> {
        let frame = read_record(&mut self.reader)?;
        match self
            .control
            .receive_orchestrator(&frame)
            .map_err(|_| ProcessSupervisorError::Control)?
        {
            OrchestratorEvent::PackageTrust(trust) => trusted_package_key(trust),
            OrchestratorEvent::LaunchBindings(_)
            | OrchestratorEvent::Terminate
            | OrchestratorEvent::Response(_) => Err(ProcessSupervisorError::Control),
        }
    }

    /// Receive pipeline-authorized channel UUIDs and optional Level 1 model settings.
    pub fn receive_launch_bindings(&mut self) -> Result<LaunchBindings, ProcessSupervisorError> {
        let frame = read_record(&mut self.reader)?;
        match self
            .control
            .receive_orchestrator(&frame)
            .map_err(|_| ProcessSupervisorError::Control)?
        {
            OrchestratorEvent::LaunchBindings(bindings) => Ok(bindings),
            OrchestratorEvent::PackageTrust(_)
            | OrchestratorEvent::Terminate
            | OrchestratorEvent::Response(_) => Err(ProcessSupervisorError::Control),
        }
    }

    /// Send one authenticated readiness record with the independently verified hash.
    pub fn ready(&mut self, package_hash: [u8; 32]) -> Result<(), ProcessSupervisorError> {
        let frame = self
            .control
            .ready(package_hash)
            .map_err(|_| ProcessSupervisorError::Control)?;
        write_record(&mut self.writer, &frame)
    }

    /// Send one authenticated heartbeat after readiness.
    pub fn heartbeat(&mut self) -> Result<(), ProcessSupervisorError> {
        let frame = self
            .control
            .heartbeat()
            .map_err(|_| ProcessSupervisorError::Control)?;
        write_record(&mut self.writer, &frame)
    }

    /// Request one bounded page from an authorized channel.
    pub fn request_receive(
        &mut self,
        channel_id: [u8; 16],
        limit: u16,
    ) -> Result<DataPlaneResponse, ProcessSupervisorError> {
        let (_, frame) = self
            .control
            .request_receive(channel_id, limit)
            .map_err(|_| ProcessSupervisorError::Control)?;
        self.exchange_data_plane(frame)
    }

    /// Request publication of one bounded plaintext channel payload.
    pub fn request_publish(
        &mut self,
        channel_id: [u8; 16],
        content_type: String,
        payload: Vec<u8>,
    ) -> Result<DataPlaneResponse, ProcessSupervisorError> {
        let (_, frame) = self
            .control
            .request_publish(channel_id, content_type, payload)
            .map_err(|_| ProcessSupervisorError::Control)?;
        self.exchange_data_plane(frame)
    }

    /// Request acknowledgement of one previously delivered message.
    pub fn request_acknowledge(
        &mut self,
        channel_id: [u8; 16],
        message_id: [u8; 16],
    ) -> Result<DataPlaneResponse, ProcessSupervisorError> {
        let (_, frame) = self
            .control
            .request_acknowledge(channel_id, message_id)
            .map_err(|_| ProcessSupervisorError::Control)?;
        self.exchange_data_plane(frame)
    }

    /// Request one provider-neutral completion.
    pub fn request_completion(
        &mut self,
        call: CompletionCall,
    ) -> Result<DataPlaneResponse, ProcessSupervisorError> {
        let (_, frame) = self
            .control
            .request_completion(call)
            .map_err(|_| ProcessSupervisorError::Control)?;
        self.exchange_data_plane(frame)
    }

    /// Request the exact model tool catalog installed for this host binding.
    pub fn request_model_tools(&mut self) -> Result<DataPlaneResponse, ProcessSupervisorError> {
        let (_, frame) = self
            .control
            .request_model_tools()
            .map_err(|_| ProcessSupervisorError::Control)?;
        self.exchange_data_plane(frame)
    }

    /// Request one provider-neutral tool-aware completion turn.
    pub fn request_tool_completion(
        &mut self,
        call: ToolCompletionCall,
    ) -> Result<DataPlaneResponse, ProcessSupervisorError> {
        let (_, frame) = self
            .control
            .request_tool_completion(call)
            .map_err(|_| ProcessSupervisorError::Control)?;
        self.exchange_data_plane(frame)
    }

    /// Dispatch one model-returned call through the parent-owned D18D runtime.
    pub fn request_tool_execution(
        &mut self,
        call: ModelToolCall,
    ) -> Result<DataPlaneResponse, ProcessSupervisorError> {
        let (_, frame) = self
            .control
            .request_tool_execution(call)
            .map_err(|_| ProcessSupervisorError::Control)?;
        self.exchange_data_plane(frame)
    }

    /// Block for and authenticate the orchestrator's graceful termination request.
    pub fn receive_terminate(&mut self) -> Result<(), ProcessSupervisorError> {
        let frame = read_record(&mut self.reader)?;
        match self
            .control
            .receive_orchestrator(&frame)
            .map_err(|_| ProcessSupervisorError::Control)?
        {
            OrchestratorEvent::Terminate => Ok(()),
            OrchestratorEvent::PackageTrust(_)
            | OrchestratorEvent::LaunchBindings(_)
            | OrchestratorEvent::Response(_) => Err(ProcessSupervisorError::Control),
        }
    }

    /// Return this launch's UUID-v7 secure-session identity.
    pub fn session_id(&self) -> SessionId {
        self.control.session_id()
    }

    fn exchange_data_plane(
        &mut self,
        frame: Vec<u8>,
    ) -> Result<DataPlaneResponse, ProcessSupervisorError> {
        write_record(&mut self.writer, &frame)?;
        let response = read_record(&mut self.reader)?;
        match self
            .control
            .receive_orchestrator(&response)
            .map_err(|_| ProcessSupervisorError::Control)?
        {
            OrchestratorEvent::Response(response) => Ok(response),
            OrchestratorEvent::Terminate => Err(ProcessSupervisorError::Terminated),
            OrchestratorEvent::PackageTrust(_) | OrchestratorEvent::LaunchBindings(_) => {
                Err(ProcessSupervisorError::Control)
            }
        }
    }
}

fn package_trust_record(key: &TrustedPackageKey) -> Result<PackageTrust, ProcessSupervisorError> {
    let key_type = match key.key_type {
        PackageKeyType::Production => PackageTrustType::Production,
        PackageKeyType::Developer => PackageTrustType::Developer,
        PackageKeyType::ThirdParty => PackageTrustType::ThirdParty,
    };
    PackageTrust::new(
        &key.key_id,
        key_type,
        key.public_key,
        privilege_tier_number(key.maximum_tier),
    )
    .map_err(|_| ProcessSupervisorError::PackageVerification)
}

fn trusted_package_key(trust: PackageTrust) -> Result<TrustedPackageKey, ProcessSupervisorError> {
    let key_type = match trust.key_type() {
        PackageTrustType::Production => PackageKeyType::Production,
        PackageTrustType::Developer => PackageKeyType::Developer,
        PackageTrustType::ThirdParty => PackageKeyType::ThirdParty,
    };
    let maximum_tier = match trust.maximum_tier() {
        0 => PrivilegeTier::Tier0,
        1 => PrivilegeTier::Tier1,
        2 => PrivilegeTier::Tier2,
        3 => PrivilegeTier::Tier3,
        _ => return Err(ProcessSupervisorError::Control),
    };
    TrustedPackageKey::new(trust.key_id(), key_type, trust.public_key(), maximum_tier)
        .map_err(|_| ProcessSupervisorError::Control)
}

fn privilege_tier_number(tier: PrivilegeTier) -> u8 {
    match tier {
        PrivilegeTier::Tier0 => 0,
        PrivilegeTier::Tier1 => 1,
        PrivilegeTier::Tier2 => 2,
        PrivilegeTier::Tier3 => 3,
    }
}

fn validate_runtime_bindings(
    runtime: AgentPackageRuntime,
    bindings: &LaunchBindings,
) -> Result<(), ProcessSupervisorError> {
    match (runtime, bindings.level_one_model()) {
        (AgentPackageRuntime::Skill, Some(_)) | (AgentPackageRuntime::Deno, None) => Ok(()),
        _ => Err(ProcessSupervisorError::LaunchBindings),
    }
}

/// How long the supervisor waits for a host's reader thread to finish once
/// the host is gone. The session kill normally ends it at once; if
/// something still holds the pipe, the thread is left to finish on its own
/// rather than hold the supervisor (review round 8, L1).
const READER_JOIN_DEADLINE: Duration = Duration::from_secs(2);

fn join_bounded(reader: JoinHandle<()>) {
    let deadline = Instant::now() + READER_JOIN_DEADLINE;
    while !reader.is_finished() {
        if Instant::now() >= deadline {
            return;
        }
        thread::sleep(Duration::from_millis(5));
    }
    let _ = reader.join();
}

/// Most records one `refresh` handles for a host; the rest wait for the
/// next one (D18S S-K5, review round 7).
const MAX_RECORDS_PER_DRAIN: usize = MAX_PENDING_RECORDS;

/// Most response frames queued for a host that is not reading them.
const MAX_QUEUED_FRAMES: usize = 8;

/// A host's stdin, written by a thread of its own (D18S S-K5, review round
/// 7). A blocking write on the supervisor's thread would let a host that
/// stops reading its stdin stall every host. Here the supervisor only
/// queues: when a host lets `MAX_QUEUED_FRAMES` pile up, the queue is full,
/// `send` fails, and the caller ends the host. Ending it breaks the pipe,
/// which ends a write the thread is blocked in.
struct RecordWriter {
    queue: mpsc::SyncSender<Vec<u8>>,
}

impl RecordWriter {
    fn start(stdin: BufWriter<ChildStdin>) -> Result<Self, ProcessSupervisorError> {
        let (queue, frames) = mpsc::sync_channel::<Vec<u8>>(MAX_QUEUED_FRAMES);
        thread::Builder::new()
            .name("host-stdin".into())
            .spawn(move || {
                let mut stdin = stdin;
                for frame in frames {
                    if write_record(&mut stdin, &frame).is_err() {
                        break;
                    }
                }
            })
            .map_err(|_| ProcessSupervisorError::ProcessIo)?;
        Ok(Self { queue })
    }

    /// Queue one frame. The length is checked here, so a bad frame is the
    /// caller's error at once; a full or closed queue is `ProcessIo`.
    fn send(&self, frame: Vec<u8>) -> Result<(), ProcessSupervisorError> {
        if frame.is_empty() || frame.len() > MAX_RECORD_BYTES {
            return Err(ProcessSupervisorError::Framing);
        }
        self.queue
            .try_send(frame)
            .map_err(|_| ProcessSupervisorError::ProcessIo)
    }
}

fn write_record(writer: &mut impl Write, payload: &[u8]) -> Result<(), ProcessSupervisorError> {
    if payload.is_empty() || payload.len() > MAX_RECORD_BYTES {
        return Err(ProcessSupervisorError::Framing);
    }
    let length = u32::try_from(payload.len()).map_err(|_| ProcessSupervisorError::Framing)?;
    writer
        .write_all(&length.to_be_bytes())
        .and_then(|()| writer.write_all(payload))
        .and_then(|()| writer.flush())
        .map_err(|_| ProcessSupervisorError::ProcessIo)
}

fn read_record(reader: &mut impl Read) -> Result<Vec<u8>, ProcessSupervisorError> {
    let mut length = [0u8; 4];
    reader.read_exact(&mut length).map_err(map_read_error)?;
    let length = u32::from_be_bytes(length) as usize;
    if length == 0 || length > MAX_RECORD_BYTES {
        return Err(ProcessSupervisorError::Framing);
    }
    let mut payload = vec![0u8; length];
    reader.read_exact(&mut payload).map_err(map_read_error)?;
    Ok(payload)
}

fn map_read_error(error: io::Error) -> ProcessSupervisorError {
    match error.kind() {
        io::ErrorKind::UnexpectedEof => ProcessSupervisorError::Framing,
        _ => ProcessSupervisorError::ProcessIo,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chief_of_staff_host_control_protocol::ControlState;

    /// Review round 7: a host that never reads its stdin must not block the
    /// supervisor. Queueing fails once the writer is stuck and the queue is
    /// full, within a bounded number of sends, and quickly.
    #[cfg(unix)]
    #[test]
    fn a_host_that_stops_reading_fills_its_queue_instead_of_blocking() {
        let mut child = Command::new("sleep")
            .arg("30")
            .stdin(Stdio::piped())
            .spawn()
            .expect("sleep runs");
        let writer = RecordWriter::start(BufWriter::new(child.stdin.take().unwrap())).unwrap();
        let started = Instant::now();
        let frame = vec![7u8; MAX_RECORD_BYTES];
        // The pipe holds well under one frame, so the thread blocks on the
        // first it takes, for good, and the queue then holds
        // MAX_QUEUED_FRAMES more: exactly MAX_QUEUED_FRAMES + 1 are ever
        // accepted.
        //
        // When the thread takes that first frame is up to the scheduler.
        // Asserting a refusal right after the queue first fills raced it
        // (macOS CI: the thread took its frame in between, freeing a slot).
        // So keep refilling until the count is reached, which can only
        // happen once the thread holds its one frame and is stuck on it.
        let mut accepted = 0;
        while accepted < MAX_QUEUED_FRAMES + 1 {
            assert!(started.elapsed() < Duration::from_secs(5), "never filled");
            accepted += (0..MAX_QUEUED_FRAMES + 8)
                .take_while(|_| writer.send(frame.clone()).is_ok())
                .count();
            thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(accepted, MAX_QUEUED_FRAMES + 1);
        assert_eq!(
            writer.send(frame.clone()),
            Err(ProcessSupervisorError::ProcessIo)
        );
        assert!(started.elapsed() < Duration::from_secs(5), "send blocked");
        // Ending the host breaks the pipe, which ends the blocked write.
        child.kill().unwrap();
        child.wait().unwrap();
        drop(writer);
    }

    #[cfg(unix)]
    #[test]
    fn a_frame_out_of_bounds_is_refused_before_it_is_queued() {
        let mut child = Command::new("sleep")
            .arg("5")
            .stdin(Stdio::piped())
            .spawn()
            .expect("sleep runs");
        let writer = RecordWriter::start(BufWriter::new(child.stdin.take().unwrap())).unwrap();
        assert_eq!(
            writer.send(Vec::new()),
            Err(ProcessSupervisorError::Framing)
        );
        assert_eq!(
            writer.send(vec![0; MAX_RECORD_BYTES + 1]),
            Err(ProcessSupervisorError::Framing)
        );
        child.kill().unwrap();
        child.wait().unwrap();
    }
    use coding_adventures_x3dh::generate_identity_keypair;
    use std::sync::mpsc::{channel, Receiver, Sender};

    struct MemoryReader {
        receiver: Receiver<Vec<u8>>,
        pending: Vec<u8>,
        offset: usize,
    }

    impl MemoryReader {
        fn new(receiver: Receiver<Vec<u8>>) -> Self {
            Self {
                receiver,
                pending: Vec::new(),
                offset: 0,
            }
        }
    }

    impl Read for MemoryReader {
        fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
            if self.offset == self.pending.len() {
                self.pending = self
                    .receiver
                    .recv()
                    .map_err(|_| io::Error::new(io::ErrorKind::UnexpectedEof, "closed"))?;
                self.offset = 0;
            }
            let count = output.len().min(self.pending.len() - self.offset);
            output[..count].copy_from_slice(&self.pending[self.offset..self.offset + count]);
            self.offset += count;
            Ok(count)
        }
    }

    struct MemoryWriter(Sender<Vec<u8>>);

    impl Write for MemoryWriter {
        fn write(&mut self, input: &[u8]) -> io::Result<usize> {
            self.0
                .send(input.to_vec())
                .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "closed"))?;
            Ok(input.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    struct FailingIo;

    impl Read for FailingIo {
        fn read(&mut self, _output: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("secret read failure"))
        }
    }

    impl Write for FailingIo {
        fn write(&mut self, _input: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("secret write failure"))
        }

        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::other("secret flush failure"))
        }
    }

    #[test]
    fn framing_accepts_exact_bounded_record() {
        let mut wire = Vec::new();
        write_record(&mut wire, b"hello").unwrap();
        assert_eq!(read_record(&mut wire.as_slice()).unwrap(), b"hello");
    }

    #[test]
    fn framing_rejects_zero_oversized_and_truncated_records() {
        assert_eq!(
            read_record(&mut [0u8; 4].as_slice()),
            Err(ProcessSupervisorError::Framing)
        );
        assert_eq!(
            read_record(&mut ((MAX_RECORD_BYTES as u32 + 1).to_be_bytes()).as_slice()),
            Err(ProcessSupervisorError::Framing)
        );
        assert_eq!(
            read_record(&mut [0, 0, 0, 2, 1].as_slice()),
            Err(ProcessSupervisorError::Framing)
        );
        assert_eq!(
            write_record(&mut Vec::new(), &[]),
            Err(ProcessSupervisorError::Framing)
        );
        assert_eq!(
            write_record(&mut Vec::new(), &vec![0; MAX_RECORD_BYTES + 1]),
            Err(ProcessSupervisorError::Framing)
        );
        assert_eq!(
            write_record(&mut FailingIo, b"record"),
            Err(ProcessSupervisorError::ProcessIo)
        );
        assert_eq!(
            read_record(&mut FailingIo),
            Err(ProcessSupervisorError::ProcessIo)
        );
    }

    #[test]
    fn configuration_is_bounded_and_diagnostics_are_redacted() {
        assert_eq!(
            HostProgram::new("", std::iter::empty::<&str>()),
            Err(ProcessSupervisorError::InvalidConfiguration)
        );
        assert_eq!(
            HostProgram::new("relative-host", std::iter::empty::<&str>()),
            Err(ProcessSupervisorError::InvalidConfiguration)
        );
        let too_many = (0..=MAX_FIXED_ARGUMENTS).map(|_| "x");
        assert_eq!(
            HostProgram::new("host", too_many),
            Err(ProcessSupervisorError::InvalidConfiguration)
        );
        let executable = std::env::current_exe().unwrap();
        let program = HostProgram::new(&executable, ["secret-argument"]).unwrap();
        assert_eq!(
            ProcessSupervisorConfig::new(program.clone(), Duration::ZERO, Duration::from_secs(1)),
            Err(ProcessSupervisorError::InvalidConfiguration)
        );
        let config = ProcessSupervisorConfig::new(
            program.clone(),
            Duration::from_secs(2),
            Duration::from_secs(3),
        )
        .unwrap();
        assert_eq!(config.program(), &program);
        assert_eq!(config.bootstrap_timeout(), Duration::from_secs(2));
        assert_eq!(config.graceful_stop_timeout(), Duration::from_secs(3));
        assert_eq!(program.executable(), executable);
        assert_eq!(
            program.arguments(),
            &[std::ffi::OsStr::new("secret-argument")]
        );
        assert!(!ProcessSupervisorError::Spawn.to_string().contains("secret"));
    }

    #[test]
    fn production_sources_create_valid_values() {
        let clock = SystemMonotonicClock::new();
        let first = clock.now_ns();
        let second = clock.now_ns();
        assert!(second >= first);
        let session = UuidV7SessionIdSource.next_session().unwrap();
        assert_eq!(session.as_bytes()[6] >> 4, 7);
    }

    #[test]
    fn error_type_is_standard_error() {
        let cases = [
            (
                ProcessSupervisorError::InvalidConfiguration,
                "invalid configuration",
            ),
            (
                ProcessSupervisorError::PackageVerification,
                "package verification failed",
            ),
            (
                ProcessSupervisorError::PackageMismatch,
                "package identity mismatch",
            ),
            (
                ProcessSupervisorError::SessionGeneration,
                "session generation failed",
            ),
            (ProcessSupervisorError::Bootstrap, "secure bootstrap failed"),
            (ProcessSupervisorError::Spawn, "child spawn failed"),
            (ProcessSupervisorError::ProcessIo, "process I/O failed"),
            (
                ProcessSupervisorError::BootstrapTimeout,
                "bootstrap timed out",
            ),
            (ProcessSupervisorError::Framing, "invalid framed record"),
            (ProcessSupervisorError::Control, "host-control failure"),
            (
                ProcessSupervisorError::Terminated,
                "graceful termination requested",
            ),
            (
                ProcessSupervisorError::ActivePackageMismatch,
                "active package identity mismatch",
            ),
            (ProcessSupervisorError::HostNotFound, "host not found"),
            (ProcessSupervisorError::BrokerLaunch, "broker launch failed"),
            (
                ProcessSupervisorError::BrokerBusy,
                "previous broker still finishing",
            ),
            (
                ProcessSupervisorError::DispatchBusy,
                "previous dispatch still finishing",
            ),
            (ProcessSupervisorError::Broker, "broker ended"),
        ];
        for (error, suffix) in cases {
            let standard: &dyn std::error::Error = &error;
            assert_eq!(
                standard.to_string(),
                format!("process-supervisor: {suffix}")
            );
        }
    }

    #[test]
    fn child_stream_helper_completes_authenticated_lifecycle() {
        let (to_child, child_input) = channel();
        let (to_parent, parent_input) = channel();
        let mut parent_writer = MemoryWriter(to_child);
        let mut parent_reader = MemoryReader::new(parent_input);
        let child_reader = MemoryReader::new(child_input);
        let child_writer = MemoryWriter(to_parent);
        let mut session_bytes = [0u8; 16];
        session_bytes[6] = 0x70;
        session_bytes[8] = 0x80;
        session_bytes[15] = 9;
        let session = SessionId::new(session_bytes).unwrap();
        let identity = generate_identity_keypair();
        let bootstrap =
            OrchestratorBootstrap::new(&identity, HostId::new("memory-host").unwrap(), session)
                .unwrap();
        let offer = bootstrap.offer().unwrap();
        let package_hash = [23; 32];

        let child = thread::spawn(move || {
            let mut control = ChildProcessControl::bootstrap(child_reader, child_writer).unwrap();
            assert_eq!(control.session_id(), session);
            let trust = control.receive_package_trust().unwrap();
            assert_eq!(trust.key_id, "prod-memory");
            assert_eq!(trust.public_key, [19; 32]);
            assert_eq!(
                control.receive_launch_bindings().unwrap(),
                LaunchBindings::new(Vec::new(), None).unwrap()
            );
            control.ready(package_hash).unwrap();
            control.heartbeat().unwrap();
            control.receive_terminate().unwrap();
        });

        write_record(&mut parent_writer, offer.as_bytes()).unwrap();
        let hello = ClientHello::from_bytes(&read_record(&mut parent_reader).unwrap()).unwrap();
        let channel = bootstrap.accept(&hello).unwrap();
        let mut control = OrchestratorControl::new(channel, package_hash).unwrap();
        let trust = control
            .provide_package_trust(
                PackageTrust::new("prod-memory", PackageTrustType::Production, [19; 32], 3)
                    .unwrap(),
            )
            .unwrap();
        write_record(&mut parent_writer, &trust).unwrap();
        let bindings = control
            .provide_launch_bindings(LaunchBindings::new(Vec::new(), None).unwrap())
            .unwrap();
        write_record(&mut parent_writer, &bindings).unwrap();
        let ready = read_record(&mut parent_reader).unwrap();
        assert!(matches!(
            control.receive_child(&ready, 10).unwrap(),
            ChildEvent::Ready {
                received_at_ns: 10,
                ..
            }
        ));
        let heartbeat = read_record(&mut parent_reader).unwrap();
        assert_eq!(
            control.receive_child(&heartbeat, 11).unwrap(),
            ChildEvent::Heartbeat { received_at_ns: 11 }
        );
        let terminate = control.terminate().unwrap();
        write_record(&mut parent_writer, &terminate).unwrap();
        child.join().unwrap();
        assert_eq!(control.state(), ControlState::Terminating);
    }
}
