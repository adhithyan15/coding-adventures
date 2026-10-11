//! # Launching and relaying to one agent's broker (D18S S-K1, S-K7; P2.6d-2a)
//!
//! The supervisor-side half of a broker's life, holding no key itself:
//!
//! ```text
//!   BrokerKeyFiles::slots_for(binding)   which key file goes on which descriptor
//!   abandon_pending_on_write_channels    give back what a dead broker reserved
//!   launch(program, binding, keys, ..)   open the key files, exec the verified
//!                                        binary with them on 3..3+n, Bootstrap,
//!                                        and check Ready's public keys against
//!                                        the channel definitions
//!   start_relay(broker, ..)              one thread: relay each channel request,
//!                                        answer each callback with the daemon's
//!                                        CallbackServer, deliver the response
//! ```
//!
//! Nothing here decides *when* a broker runs; the supervisor does (2b).
//! This crate makes each step correct on its own, and testable.
//!
//! ## The deadline counts only the broker's time
//!
//! A broker gets [`RelayConfig::deadline`] to answer a request. The clock
//! runs only while the relay waits on the broker. Time spent serving the
//! broker's callbacks, which is storage work in the daemon, is not charged
//! to it, so a slow disk never ends an honest broker.
//!
//! ## The binding is pinned
//!
//! Callbacks re-resolve the binding every time (S-K2). The resolver here
//! also requires the result to name the same pipeline and agent the broker
//! was launched for. A host rewired to another identity must not have its
//! old broker's callbacks authorized as the new agent.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::{Child, ChildStdin};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TrySendError};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use chief_of_staff_broker_callbacks::{BindingResolver, CallbackServer, InFlight, Violation};
use chief_of_staff_broker_protocol::{
    encode_to_broker, write_frame, FromBroker, KeyKind, KeySlot, ProtocolError, PublicKey, ToBroker,
};
use chief_of_staff_channel_crypto::ChannelId;
use chief_of_staff_channel_endpoints::{
    AgentId, ChannelDefinitionStore, ChannelLifecycle, MessageMetadataSource,
};
use chief_of_staff_channel_store::{ChannelStore, ChannelStoreError};
use chief_of_staff_host_control_protocol::{
    validate_data_plane_response, ChannelBindingAccess, DataPlaneRequest, DataPlaneResponse,
};
use chief_of_staff_pipeline_bindings::{HostPipelineBinding, PipelineId};
use storage_core::StorageBackend;

/// A broker binary verified against its pinned digest (Linux).
#[cfg(target_os = "linux")]
pub use chief_of_staff_spawn_isolation::{VerifiedExecutable, VerifyError};

/// Off Linux there is no verified launch yet (S-P3), so no value of this
/// type can exist, and [`launch`] cannot be reached.
#[cfg(not(target_os = "linux"))]
#[derive(Debug)]
pub enum VerifiedExecutable {}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Why a broker did not launch. Payload-blind: no path or key appears.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LaunchError {
    /// Two declarations for one key, or a channel declared in both
    /// directions.
    DuplicateKey,
    /// A bound channel has no key file declared for it.
    MissingKey,
    /// A key file could not be opened under the owner-only policy.
    KeyFile,
    /// The binary failed re-verification, or could not be spawned.
    Spawn,
    /// The broker did not send `Ready` in time, or exited first.
    NotReady,
    /// `Ready`'s public keys do not match the slots or the definitions.
    WrongKeys,
    /// Storage failed while checking or abandoning.
    Storage,
    /// No verified launch on this platform yet (S-P3).
    Unsupported,
    /// A directory holding secrets is open to someone else, or is not a
    /// directory reached without links (P2.6d-3).
    SecretDirectory,
    /// The broker's confinement could not be prepared or applied (P2.6d-3).
    Confinement,
}

impl std::fmt::Display for LaunchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::DuplicateKey => "broker key declared twice",
            Self::MissingKey => "a bound channel has no broker key",
            Self::KeyFile => "a broker key file was refused",
            Self::Spawn => "the broker could not be started",
            Self::NotReady => "the broker did not become ready",
            Self::WrongKeys => "the broker's keys do not match the channel definitions",
            Self::Storage => "storage failed during broker launch",
            Self::Unsupported => "verified broker launch is not supported on this platform",
            Self::SecretDirectory => "a directory holding secrets is not owner-only",
            Self::Confinement => "the broker's sandbox could not be set up",
        })
    }
}

impl std::error::Error for LaunchError {}

// ---------------------------------------------------------------------------
// Key files
// ---------------------------------------------------------------------------

/// One configured key file: whose, for which channel, which key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyFileDeclaration {
    pub pipeline_id: PipelineId,
    pub agent_id: AgentId,
    pub channel_id: ChannelId,
    pub kind: KeyKind,
    pub path: PathBuf,
}

type AgentKey = (PipelineId, Vec<u8>);

/// Every configured key file, indexed by agent. It holds paths only.
#[derive(Clone, Debug, Default)]
pub struct BrokerKeyFiles {
    by_agent: BTreeMap<AgentKey, BTreeMap<(ChannelId, KeyKind), PathBuf>>,
    /// Directories holding other secrets (the vault's), checked owner-only
    /// with the key files' own directories before every launch (P2.6d-3).
    secret_directories: Vec<PathBuf>,
}

impl BrokerKeyFiles {
    /// Index `declarations`, refusing a key declared twice, or a channel
    /// with keys for both directions.
    pub fn new(declarations: Vec<KeyFileDeclaration>) -> Result<Self, LaunchError> {
        let mut by_agent: BTreeMap<AgentKey, BTreeMap<(ChannelId, KeyKind), PathBuf>> =
            BTreeMap::new();
        for declaration in declarations {
            let keys = by_agent
                .entry((
                    declaration.pipeline_id,
                    declaration.agent_id.as_bytes().to_vec(),
                ))
                .or_default();
            let reading = declaration.kind == KeyKind::ReceiverPrivateKey;
            let crosses = keys.keys().any(|(channel, kind)| {
                *channel == declaration.channel_id
                    && (*kind == KeyKind::ReceiverPrivateKey) != reading
            });
            if crosses
                || keys
                    .insert((declaration.channel_id, declaration.kind), declaration.path)
                    .is_some()
            {
                return Err(LaunchError::DuplicateKey);
            }
        }
        Ok(Self {
            by_agent,
            secret_directories: Vec::new(),
        })
    }

    /// Also check `directories` owner-only before every launch: the other
    /// places secrets live, such as the vault's storage (P2.6d-3).
    pub fn with_secret_directories(mut self, directories: Vec<PathBuf>) -> Self {
        self.secret_directories = directories;
        self
    }

    /// Check every directory in [`Self::secret_directories`] is owner-only,
    /// as each launch does (P2.6d-3). One that does not exist yet holds no
    /// secret yet, and is skipped; it is checked at the next launch after
    /// it appears (review round 1, L2). A daemon calls this at startup too,
    /// so a bad layout stops it there, not at every launch.
    pub fn check_secret_directories(&self) -> Result<(), LaunchError> {
        for directory in self.secret_directories() {
            match std::fs::symlink_metadata(&directory) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                _ => {}
            }
            chief_of_staff_daemon_secret_file::check_owner_only_directory(&directory)
                .map_err(|_| LaunchError::SecretDirectory)?;
        }
        Ok(())
    }

    /// Every directory that must be owner-only before a broker launches:
    /// those given to [`Self::with_secret_directories`], and the directory
    /// of every configured key file, each once.
    pub fn secret_directories(&self) -> Vec<PathBuf> {
        let mut directories: std::collections::BTreeSet<PathBuf> =
            self.secret_directories.iter().cloned().collect();
        for keys in self.by_agent.values() {
            for path in keys.values() {
                if let Some(parent) = path.parent() {
                    directories.insert(parent.to_path_buf());
                }
            }
        }
        directories.into_iter().collect()
    }

    /// The slots for `binding`'s channels, in channel order, each with its
    /// key file. A bound channel without its keys is refused here, before
    /// anything is spawned. Keys for channels the binding does not bind are
    /// not passed.
    pub fn slots_for(
        &self,
        binding: &HostPipelineBinding,
    ) -> Result<Vec<(KeySlot, PathBuf)>, LaunchError> {
        let keys = self.by_agent.get(&(
            binding.pipeline_id(),
            binding.agent_id().as_bytes().to_vec(),
        ));
        let mut channels: Vec<_> = binding
            .launch_bindings()
            .channels()
            .iter()
            .map(|channel| (ChannelId(channel.channel_id()), channel.access()))
            .collect();
        channels.sort();
        let mut slots = Vec::new();
        for (channel, access) in channels {
            let kinds: &[KeyKind] = match access {
                ChannelBindingAccess::Read => &[KeyKind::ReceiverPrivateKey],
                ChannelBindingAccess::Write => {
                    &[KeyKind::OriginatorSigningSeed, KeyKind::ChannelMasterKey]
                }
            };
            for kind in kinds {
                let path = keys
                    .and_then(|keys| keys.get(&(channel, *kind)))
                    .ok_or(LaunchError::MissingKey)?;
                slots.push((
                    KeySlot {
                        channel_id: channel,
                        kind: *kind,
                    },
                    path.clone(),
                ));
            }
        }
        Ok(slots)
    }
}

// ---------------------------------------------------------------------------
// Abandoning what a previous broker reserved
// ---------------------------------------------------------------------------

/// Abandon any pending append on `binding`'s write channels.
///
/// Call this only once the previous broker for the agent is killed and
/// reaped, and its relay thread joined. Commits run on that thread, so after
/// the join nothing can commit late against a reservation abandoned here.
/// Returns the channels where something was abandoned.
pub fn abandon_pending_on_write_channels(
    backend: &dyn StorageBackend,
    binding: &HostPipelineBinding,
) -> Result<Vec<ChannelId>, LaunchError> {
    let mut abandoned = Vec::new();
    for channel in binding.launch_bindings().channels() {
        if channel.access() != ChannelBindingAccess::Write {
            continue;
        }
        let id = ChannelId(channel.channel_id());
        match ChannelStore::new(backend, id).abandon_pending() {
            Ok(Some(_)) => abandoned.push(id),
            Ok(None) | Err(ChannelStoreError::NotInitialized) => {}
            Err(_) => return Err(LaunchError::Storage),
        }
    }
    Ok(abandoned)
}

// ---------------------------------------------------------------------------
// The Ready check
// ---------------------------------------------------------------------------

/// `Ready`'s public keys must be exactly the slots' public halves, in slot
/// order, and each must be the key the channel definition names for this
/// agent: a receiver's for a read channel, the originator's for a write
/// channel.
pub fn check_ready(
    backend: &dyn StorageBackend,
    binding: &HostPipelineBinding,
    slots: &[KeySlot],
    public_keys: &[PublicKey],
) -> Result<(), LaunchError> {
    let expected: Vec<(ChannelId, KeyKind)> = slots
        .iter()
        .filter(|slot| slot.kind != KeyKind::ChannelMasterKey)
        .map(|slot| (slot.channel_id, slot.kind))
        .collect();
    let reported: Vec<(ChannelId, KeyKind)> = public_keys
        .iter()
        .map(|key| (key.channel_id, key.kind))
        .collect();
    if expected != reported {
        return Err(LaunchError::WrongKeys);
    }
    let definitions = ChannelDefinitionStore::new(backend);
    let agent = binding.agent_id();
    for key in public_keys {
        let definition = definitions
            .load(key.channel_id)
            .map_err(|_| LaunchError::Storage)?
            .ok_or(LaunchError::WrongKeys)?;
        if definition.lifecycle() != ChannelLifecycle::Active {
            return Err(LaunchError::WrongKeys);
        }
        let matches = match key.kind {
            KeyKind::ReceiverPrivateKey => definition
                .receiver(agent)
                .is_some_and(|receiver| receiver.public_key == key.public_key),
            KeyKind::OriginatorSigningSeed => {
                definition.originator().agent_id == *agent
                    && definition.originator().public_key == key.public_key
            }
            KeyKind::ChannelMasterKey => false,
        };
        if !matches {
            return Err(LaunchError::WrongKeys);
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Launch
// ---------------------------------------------------------------------------

/// A broker that has started and passed the `Ready` check.
pub struct LaunchedBroker {
    child: Child,
    stdin: ChildStdin,
    frames: Frames,
    reader: Option<JoinHandle<()>>,
}

impl LaunchedBroker {
    /// The broker process, for the supervisor to watch, kill and reap.
    pub fn child(&mut self) -> &mut Child {
        &mut self.child
    }

    /// Kill and reap a broker that will not be used after all, and close
    /// its pipes.
    pub fn discard(self) {
        let (child, io) = self.into_parts();
        discard(child);
        io.close();
    }

    /// Split into the process and the relay's I/O.
    fn into_parts(self) -> (Child, BrokerIo) {
        (
            self.child,
            BrokerIo {
                stdin: Box::new(self.stdin),
                frames: self.frames,
                reader: self.reader,
            },
        )
    }
}

/// The broker's decoded output frames, as its reader delivers them. The
/// channel closes when the broker's output does.
pub type Frames = Receiver<Result<FromBroker, ProtocolError>>;

struct BrokerIo {
    stdin: Box<dyn std::io::Write + Send>,
    frames: Frames,
    reader: Option<JoinHandle<()>>,
}

impl BrokerIo {
    /// Close the broker's stdin and wait, bounded, for its reader. Call
    /// after the broker is killed: its stdout then closes, and the reader
    /// ends.
    fn close(mut self) {
        drop(self.stdin);
        drop(self.frames);
        if let Some(reader) = self.reader.take() {
            let _ = join_bounded(reader);
        }
    }
}

/// Frames to the broker, written by a thread of their own.
///
/// The relay never writes to the broker directly: a broker that stops
/// reading its stdin would otherwise block the relay in that write, where
/// no deadline runs. Through this writer, the relay only queues. A broker
/// that stops reading fills the queue and stops answering, which the
/// deadline catches. A failed write closes the queue, which the relay sees.
struct FrameWriter {
    frames: Option<SyncSender<Vec<u8>>>,
    thread: Option<JoinHandle<()>>,
}

impl FrameWriter {
    fn start(mut sink: Box<dyn std::io::Write + Send>) -> std::io::Result<Self> {
        // Two: an honest broker reads each frame before it answers, so at
        // most one is ever waiting.
        let (frames, queued) = mpsc::sync_channel::<Vec<u8>>(2);
        let thread = std::thread::Builder::new()
            .name("broker-stdin".into())
            .spawn(move || {
                while let Ok(body) = queued.recv() {
                    if write_frame(&mut sink, &body).is_err() {
                        return;
                    }
                }
            })?;
        Ok(Self {
            frames: Some(frames),
            thread: Some(thread),
        })
    }

    /// Queue `body`, without waiting. `false` if the queue is full or the
    /// writer has stopped.
    fn send(&self, body: Vec<u8>) -> bool {
        self.frames
            .as_ref()
            .is_some_and(|frames| frames.try_send(body).is_ok())
    }

    /// Stop the writer. A write still blocked ends when the broker is
    /// killed and its stdin breaks; the join is bounded.
    fn close(mut self) {
        self.frames = None;
        if let Some(thread) = self.thread.take() {
            let _ = join_bounded(thread);
        }
    }
}

/// Join `thread`, giving up after two seconds. A thread still blocked then
/// is left to finish on its own; it holds nothing that outlives it.
/// Returns whether it joined.
fn join_bounded(thread: JoinHandle<()>) -> bool {
    let deadline = Instant::now() + Duration::from_secs(2);
    while !thread.is_finished() {
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let _ = thread.join();
    true
}

#[cfg(target_os = "linux")]
/// Read the broker's frames on a thread of their own, so the relay can wait
/// on them with a deadline. The channel closes when the broker's stdout
/// does.
fn spawn_reader(
    mut stdout: std::process::ChildStdout,
) -> std::io::Result<(Frames, JoinHandle<()>)> {
    use chief_of_staff_broker_protocol::{decode_from_broker, read_frame};
    let (frames, receiver) = mpsc::sync_channel(4);
    let reader = std::thread::Builder::new()
        .name("broker-out".into())
        .spawn(move || loop {
            let frame = read_frame(&mut stdout).and_then(|body| decode_from_broker(&body));
            let failed = frame.is_err();
            if frames.send(frame).is_err() || failed {
                return;
            }
        })?;
    Ok((receiver, reader))
}

/// Kill and reap a broker that will not be used.
fn discard(mut child: Child) {
    let _ = chief_of_staff_spawn_isolation::kill_session(&child);
    let _ = child.kill();
    let _ = child.wait();
}

/// Check the secret directories, open `binding`'s key files, start
/// `program` confined and holding them at 3..3+n, send Bootstrap, and wait
/// up to `ready_timeout` for a `Ready` that passes [`check_ready`]. On any
/// failure the broker is killed and reaped.
///
/// The broker runs under [`broker_plan`], a plan with no capabilities,
/// through `chief-of-staff-linux-sandbox` (P2.6d-3): it can open no file by
/// path, has no network, and can never exec. What it holds is its keys, on
/// those descriptors, and its pipes.
#[cfg(target_os = "linux")]
pub fn launch(
    program: &VerifiedExecutable,
    binding: &HostPipelineBinding,
    keys: &BrokerKeyFiles,
    backend: &dyn StorageBackend,
    ready_timeout: Duration,
) -> Result<LaunchedBroker, LaunchError> {
    use std::os::fd::OwnedFd;
    use std::process::{Command, Stdio};

    // Before anything is opened: every directory holding secrets is
    // owner-only, as P2.6c's hard-link check assumes.
    keys.check_secret_directories()?;
    let slotted = keys.slots_for(binding)?;
    // Prepared from the verified descriptor, which it re-verifies.
    let confinement =
        chief_of_staff_linux_sandbox::LinuxConfinement::prepare_verified(&broker_plan()?, program)
            .map_err(|_| LaunchError::Confinement)?;
    let descriptors = slotted
        .iter()
        .map(|(_, path)| {
            chief_of_staff_daemon_secret_file::open_owner_only_secret(path)
                .map(OwnedFd::from)
                .map_err(|_| LaunchError::KeyFile)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let slots: Vec<KeySlot> = slotted.into_iter().map(|(slot, _)| slot).collect();

    let mut command = Command::new("chief-of-staff-agent-broker");
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .env_clear();
    confinement
        .apply_inheriting(&mut command, descriptors)
        .map_err(|_| LaunchError::Confinement)?;
    let spawned = command.spawn();
    // The command holds the parked key descriptors; dropping it closes
    // this process's last copies.
    drop(command);
    let mut child = spawned.map_err(|_| LaunchError::Spawn)?;
    let (Some(stdin), Some(stdout)) = (child.stdin.take(), child.stdout.take()) else {
        discard(child);
        return Err(LaunchError::Spawn);
    };
    let Ok((frames, reader)) = spawn_reader(stdout) else {
        discard(child);
        return Err(LaunchError::Spawn);
    };
    let broker = LaunchedBroker {
        child,
        stdin,
        frames,
        reader: Some(reader),
    };
    let bootstrap = encode_to_broker(&ToBroker::Bootstrap {
        binding: binding.clone(),
        slots: slots.clone(),
    })
    .map_err(|_| LaunchError::Spawn);
    let mut broker = broker;
    let ready = bootstrap
        .and_then(|body| write_frame(&mut broker.stdin, &body).map_err(|_| LaunchError::NotReady))
        .and_then(|()| match broker.frames.recv_timeout(ready_timeout) {
            Ok(Ok(FromBroker::Ready { public_keys })) => {
                check_ready(backend, binding, &slots, &public_keys)
            }
            _ => Err(LaunchError::NotReady),
        });
    if let Err(error) = ready {
        let (child, io) = broker.into_parts();
        discard(child);
        io.close();
        return Err(error);
    }
    Ok(broker)
}

/// The broker's sandbox plan: a manifest with no capabilities, for Linux
/// (P2.6d-3). Everything the broker needs it already holds when it starts.
#[cfg(target_os = "linux")]
pub fn broker_plan() -> Result<capability_os_sandbox::SandboxPlan, LaunchError> {
    capability_os_sandbox::plan_from_json(
        r#"{"version":1,"package":"rust/chief-of-staff-agent-broker","capabilities":[],"justification":"A per-agent channel broker holds its keys on inherited descriptors and talks to its supervisor over its pipes. It needs nothing else."}"#,
        capability_os_sandbox::OsFamily::Linux,
    )
    .map_err(|_| LaunchError::Confinement)
}

/// Verified launch needs `execveat`; elsewhere there is none yet, and no
/// [`VerifiedExecutable`] can exist to call this with.
#[cfg(not(target_os = "linux"))]
pub fn launch(
    program: &VerifiedExecutable,
    _binding: &HostPipelineBinding,
    _keys: &BrokerKeyFiles,
    _backend: &dyn StorageBackend,
    _ready_timeout: Duration,
) -> Result<LaunchedBroker, LaunchError> {
    match *program {}
}

// ---------------------------------------------------------------------------
// The relay
// ---------------------------------------------------------------------------

/// The host could not be given a response: it is gone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HostGone;

/// Where a broker's response goes: the host's secure channel, in the
/// supervisor.
pub trait ResponseSink: Send {
    /// Deliver `response` to the host.
    fn deliver(&mut self, response: DataPlaneResponse) -> Result<(), HostGone>;
}

/// A [`BindingResolver`] that answers only with the binding the broker was
/// launched for: the same pipeline and the same agent, freshly resolved.
pub struct PinnedBindingResolver<F> {
    resolve: F,
    pipeline_id: PipelineId,
    agent_id: AgentId,
}

impl<F: Fn() -> Option<HostPipelineBinding>> PinnedBindingResolver<F> {
    /// Pin `resolve` to `launched`'s identity.
    pub fn new(resolve: F, launched: &HostPipelineBinding) -> Self {
        Self {
            resolve,
            pipeline_id: launched.pipeline_id(),
            agent_id: launched.agent_id().clone(),
        }
    }
}

impl<F: Fn() -> Option<HostPipelineBinding>> BindingResolver for PinnedBindingResolver<F> {
    fn current_binding(&self) -> Option<HostPipelineBinding> {
        (self.resolve)().filter(|binding| {
            binding.pipeline_id() == self.pipeline_id && binding.agent_id() == &self.agent_id
        })
    }
}

/// Why a relay ended. Every one ends the broker, and with it the agent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelayEnd {
    /// The broker made a callback an honest broker never makes.
    Violation(Violation),
    /// It took longer than the deadline, counting only its own time.
    Deadline,
    /// Its response did not answer the request, or failed validation.
    BadResponse,
    /// It sent a frame out of order, or one that did not decode.
    Protocol,
    /// Its output closed: it exited.
    Exited,
    /// Writing to it failed.
    Write,
    /// The host could not be given the response.
    HostGone,
}

/// The relay's settings.
#[derive(Clone, Copy, Debug)]
pub struct RelayConfig {
    /// How long a broker may take over one request, not counting time spent
    /// serving its callbacks.
    pub deadline: Duration,
}

impl Default for RelayConfig {
    fn default() -> Self {
        Self {
            deadline: Duration::from_secs(10),
        }
    }
}

/// Why a request could not be handed to the relay.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelayRefused {
    /// A request is already in flight. A host has one at a time, so an
    /// honest host never causes this.
    Busy,
    /// The relay has ended.
    Gone,
}

/// The running relay for one broker.
pub struct BrokerRelay {
    commands: Option<SyncSender<DataPlaneRequest>>,
    ended: Receiver<RelayEnd>,
    thread: Option<JoinHandle<()>>,
}

impl BrokerRelay {
    /// Hand a channel request to the relay, without waiting.
    pub fn relay(&self, request: DataPlaneRequest) -> Result<(), RelayRefused> {
        let commands = self.commands.as_ref().ok_or(RelayRefused::Gone)?;
        commands.try_send(request).map_err(|error| match error {
            TrySendError::Full(_) => RelayRefused::Busy,
            TrySendError::Disconnected(_) => RelayRefused::Gone,
        })
    }

    /// Why the relay ended, once it has.
    pub fn ended(&self) -> Option<RelayEnd> {
        self.ended.try_recv().ok()
    }

    /// Whether the relay thread has finished.
    pub fn is_finished(&self) -> bool {
        self.thread.as_ref().is_none_or(JoinHandle::is_finished)
    }

    /// Stop the relay and join it, bounded. Kill the broker first: a relay
    /// waiting on a live broker's output only ends when that output closes.
    /// Returns whether the thread joined.
    ///
    /// The relay commits on the broker's behalf, so until it has joined, a
    /// commit may still be in progress. Do not abandon the agent's pending
    /// reservations, or launch its next broker, unless this returned
    /// `true`. (If that is missed, nothing is corrupted: the store's
    /// compare-and-swap lets exactly one of the commit and the abandon win.
    /// But the rule keeps "at most one live broker per agent" exact.)
    pub fn stop(&mut self) -> bool {
        self.commands = None;
        let deadline = Instant::now() + Duration::from_secs(2);
        while !self.is_finished() {
            if Instant::now() >= deadline {
                // Still running: keep the handle, so `is_finished` can be
                // asked again later.
                return false;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        true
    }
}

/// Start relaying for `broker`. Returns its process, which the supervisor
/// keeps to watch, kill and reap, and the relay.
pub fn start_relay(
    broker: LaunchedBroker,
    backend: Arc<dyn StorageBackend>,
    metadata: Arc<dyn MessageMetadataSource>,
    resolver: Box<dyn BindingResolver + Send>,
    sink: Box<dyn ResponseSink>,
    config: RelayConfig,
) -> std::io::Result<(Child, BrokerRelay)> {
    let (child, io) = broker.into_parts();
    // The relay thread owns the I/O; if it cannot start, the broker is
    // killed and reaped here rather than left running unrelayed.
    match spawn_relay(io, backend, metadata, resolver, sink, config) {
        Ok(relay) => Ok((child, relay)),
        Err(error) => {
            discard(child);
            Err(error)
        }
    }
}

/// Relay over any byte sink to the broker and any source of its frames:
/// what [`start_relay`] does for a launched broker, without the process.
/// It exists so the relay's handling of a misbehaving broker can be tested
/// in process.
pub fn start_relay_over(
    to_broker: Box<dyn std::io::Write + Send>,
    from_broker: Frames,
    backend: Arc<dyn StorageBackend>,
    metadata: Arc<dyn MessageMetadataSource>,
    resolver: Box<dyn BindingResolver + Send>,
    sink: Box<dyn ResponseSink>,
    config: RelayConfig,
) -> std::io::Result<BrokerRelay> {
    let io = BrokerIo {
        stdin: to_broker,
        frames: from_broker,
        reader: None,
    };
    spawn_relay(io, backend, metadata, resolver, sink, config)
}

fn spawn_relay(
    io: BrokerIo,
    backend: Arc<dyn StorageBackend>,
    metadata: Arc<dyn MessageMetadataSource>,
    resolver: Box<dyn BindingResolver + Send>,
    sink: Box<dyn ResponseSink>,
    config: RelayConfig,
) -> std::io::Result<BrokerRelay> {
    let (commands, requests) = mpsc::sync_channel(1);
    let (report, ended) = mpsc::sync_channel(1);
    let thread = std::thread::Builder::new()
        .name("broker-relay".into())
        .spawn(move || {
            let server = CallbackServer::new(&*backend, &*metadata, &*resolver);
            let BrokerIo {
                stdin,
                frames,
                reader,
            } = io;
            let (end, writer) = match FrameWriter::start(stdin) {
                Ok(writer) => {
                    let mut sink = sink;
                    let end =
                        relay_loop(&server, &writer, &frames, &requests, sink.as_mut(), config);
                    (end, Some(writer))
                }
                Err(_) => (Some(RelayEnd::Write), None),
            };
            // Refuse further requests before reporting, so that once the
            // end is visible, relay() says Gone. Then report, before any
            // slow cleanup: the supervisor kills the broker on the report,
            // and that is what unblocks a writer stuck on a broker that
            // stopped reading.
            drop(requests);
            if let Some(end) = end {
                let _ = report.send(end);
            }
            if let Some(writer) = writer {
                writer.close();
            }
            drop(frames);
            if let Some(reader) = reader {
                let _ = join_bounded(reader);
            }
        })?;
    Ok(BrokerRelay {
        commands: Some(commands),
        ended,
        thread: Some(thread),
    })
}

/// Serve requests until told to stop (`None`) or until something ends the
/// broker (`Some`).
fn relay_loop(
    server: &CallbackServer<'_>,
    writer: &FrameWriter,
    frames: &Frames,
    requests: &Receiver<DataPlaneRequest>,
    sink: &mut dyn ResponseSink,
    config: RelayConfig,
) -> Option<RelayEnd> {
    while let Ok(request) = requests.recv() {
        let Some(mut in_flight) = InFlight::for_request(&request) else {
            // Not a channel operation: the supervisor never sends one here.
            return Some(RelayEnd::Protocol);
        };
        let Ok(body) = encode_to_broker(&ToBroker::Request(request.clone())) else {
            return Some(RelayEnd::Protocol);
        };
        if !writer.send(body) {
            return Some(RelayEnd::Write);
        }
        let mut remaining = config.deadline;
        let response = loop {
            let waited = Instant::now();
            let frame = match frames.recv_timeout(remaining) {
                Ok(Ok(frame)) => frame,
                Ok(Err(_)) => return Some(RelayEnd::Protocol),
                Err(RecvTimeoutError::Timeout) => return Some(RelayEnd::Deadline),
                Err(RecvTimeoutError::Disconnected) => return Some(RelayEnd::Exited),
            };
            // Charged: only the time spent waiting on the broker.
            remaining = remaining.saturating_sub(waited.elapsed());
            match frame {
                FromBroker::Callback {
                    callback_id,
                    request_id,
                    call,
                } => {
                    let outcome = match server.serve(&mut in_flight, request_id, call) {
                        Ok(outcome) => outcome,
                        Err(violation) => return Some(RelayEnd::Violation(violation)),
                    };
                    let Ok(body) = encode_to_broker(&ToBroker::CallbackResult {
                        callback_id,
                        outcome,
                    }) else {
                        return Some(RelayEnd::Protocol);
                    };
                    if !writer.send(body) {
                        return Some(RelayEnd::Write);
                    }
                }
                FromBroker::Response(response) => break response,
                FromBroker::Ready { .. } => return Some(RelayEnd::Protocol),
            }
        };
        let answers = response.id() == request.id()
            && response
                .operation()
                .is_none_or(|operation| operation == request.operation())
            && validate_data_plane_response(&response).is_ok();
        if !answers {
            return Some(RelayEnd::BadResponse);
        }
        if sink.deliver(response).is_err() {
            return Some(RelayEnd::HostGone);
        }
    }
    None
}
