//! Verified OS-process supervision for D18 Chief hosts.
//!
//! The service registry contains durable intent, not process authority. This
//! adapter re-verifies packages, owns child handles, carries the secure control
//! protocol over bounded pipes, and fails closed on transport or protocol error.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use chief_of_staff_channel_crypto::ChannelId;
use chief_of_staff_host_control_protocol::{
    ChildControl, ChildEvent, OrchestratorControl, OrchestratorEvent,
};
use chief_of_staff_host_runtime::{verify_agent_package, PackageKeyring};
use chief_of_staff_secure_host_channel::{
    BootstrapOffer, ChildBootstrap, ClientHello, HostId, OrchestratorBootstrap, SessionId,
};
use chief_of_staff_service_reconciler::{HostSupervisor, SupervisorObservation};
use chief_of_staff_service_registry::{HostName, HostRegistration};
use coding_adventures_x3dh::IdentityKeyPair;
use core::fmt::{self, Display, Formatter};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, TryRecvError};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

const MAX_RECORD_BYTES: usize = 1024 * 1024;
const MAX_FIXED_ARGUMENTS: usize = 128;
const MAX_PENDING_RECORDS: usize = 64;
const STOP_POLL_INTERVAL: Duration = Duration::from_millis(10);

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
    /// A different package is already active under this host name.
    ActivePackageMismatch,
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
            Self::ActivePackageMismatch => "process-supervisor: active package identity mismatch",
        })
    }
}

impl std::error::Error for ProcessSupervisorError {}

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
pub trait SessionIdSource {
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

struct OwnedInstance {
    package_hash: [u8; 32],
    child: Option<Child>,
    stdin: Option<BufWriter<ChildStdin>>,
    reader: Option<JoinHandle<()>>,
    records: Receiver<ReaderEvent>,
    control: Option<OrchestratorControl>,
    phase: InstancePhase,
    process_id: u32,
    started_at_ns: u64,
    last_heartbeat_ns: Option<u64>,
    channel_id: ChannelId,
}

impl OwnedInstance {
    fn is_active(&self) -> bool {
        !matches!(self.phase, InstancePhase::Exited { .. })
    }

    fn finish_exit(&mut self, status: ExitStatus) {
        self.child.take();
        self.stdin.take();
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
        self.control.take();
        self.phase = InstancePhase::Exited {
            exit_code: status.code(),
        };
    }

    fn hard_kill_and_reap(&mut self) -> Result<(), ProcessSupervisorError> {
        self.stdin.take();
        let status = if let Some(child) = self.child.as_mut() {
            match child
                .try_wait()
                .map_err(|_| ProcessSupervisorError::ProcessIo)?
            {
                Some(status) => status,
                None => {
                    child
                        .kill()
                        .map_err(|_| ProcessSupervisorError::ProcessIo)?;
                    child
                        .wait()
                        .map_err(|_| ProcessSupervisorError::ProcessIo)?
                }
            }
        } else {
            return Ok(());
        };
        self.finish_exit(status);
        Ok(())
    }

    fn refresh(&mut self) -> Result<(), ProcessSupervisorError> {
        if matches!(self.phase, InstancePhase::Exited { .. }) {
            return Ok(());
        }
        loop {
            match self.records.try_recv() {
                Ok(ReaderEvent::Record {
                    bytes,
                    received_at_ns,
                }) => {
                    let event = self
                        .control
                        .as_mut()
                        .ok_or(ProcessSupervisorError::Control)?
                        .receive_child(&bytes, received_at_ns)
                        .map_err(|_| ProcessSupervisorError::Control);
                    match event {
                        Ok(ChildEvent::Ready { received_at_ns, .. })
                        | Ok(ChildEvent::Heartbeat { received_at_ns }) => {
                            self.phase = InstancePhase::Running;
                            self.last_heartbeat_ns = Some(received_at_ns);
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

        if let Some(child) = self.child.as_mut() {
            if let Some(status) = child
                .try_wait()
                .map_err(|_| ProcessSupervisorError::ProcessIo)?
            {
                self.finish_exit(status);
            }
        }
        Ok(())
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
pub struct ProcessHostSupervisor<'a> {
    config: ProcessSupervisorConfig,
    keyring: &'a PackageKeyring,
    identity: &'a IdentityKeyPair,
    clock: Arc<dyn MonotonicClock>,
    sessions: Box<dyn SessionIdSource>,
    instances: BTreeMap<String, OwnedInstance>,
}

impl<'a> ProcessHostSupervisor<'a> {
    /// Construct a supervisor around injected package trust, identity, time, and sessions.
    pub fn new(
        config: ProcessSupervisorConfig,
        keyring: &'a PackageKeyring,
        identity: &'a IdentityKeyPair,
        clock: Arc<dyn MonotonicClock>,
        sessions: Box<dyn SessionIdSource>,
    ) -> Self {
        Self {
            config,
            keyring,
            identity,
            clock,
            sessions,
            instances: BTreeMap::new(),
        }
    }

    fn spawn_verified(
        &mut self,
        registration: &HostRegistration,
    ) -> Result<OwnedInstance, ProcessSupervisorError> {
        let package_path = Path::new(registration.package_path().as_str());
        let package = verify_agent_package(package_path, self.keyring)
            .map_err(|_| ProcessSupervisorError::PackageVerification)?;
        if package.digest() != *registration.package_hash() {
            return Err(ProcessSupervisorError::PackageMismatch);
        }

        let session = self.sessions.next_session()?;
        let host = HostId::new(registration.host_name().as_str().to_owned())
            .map_err(|_| ProcessSupervisorError::Bootstrap)?;
        let bootstrap = OrchestratorBootstrap::new(self.identity, host, session)
            .map_err(|_| ProcessSupervisorError::Bootstrap)?;
        let offer = bootstrap
            .offer()
            .map_err(|_| ProcessSupervisorError::Bootstrap)?;

        let mut command = Command::new(self.config.program.executable());
        command
            .args(self.config.program.arguments())
            .current_dir(package.path())
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
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
        let mut stdin = BufWriter::new(child_stdin);
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
            write_record(&mut stdin, offer.as_bytes())?;
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
            OrchestratorControl::new(channel, *registration.package_hash())
                .map_err(|_| ProcessSupervisorError::Control)
        })();

        match startup {
            Ok(control) => Ok(OwnedInstance {
                package_hash: *registration.package_hash(),
                child: Some(child),
                stdin: Some(stdin),
                reader: Some(reader),
                records,
                channel_id: ChannelId(control.session_id().as_bytes()),
                control: Some(control),
                phase: InstancePhase::Starting,
                process_id,
                started_at_ns,
                last_heartbeat_ns: None,
            }),
            Err(error) => {
                drop(stdin);
                let _ = child.kill();
                let _ = child.wait();
                let _ = reader.join();
                Err(error)
            }
        }
    }
}

impl HostSupervisor for ProcessHostSupervisor<'_> {
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
        let terminate = instance
            .control
            .as_mut()
            .ok_or(ProcessSupervisorError::Control)?
            .terminate()
            .map_err(|_| ProcessSupervisorError::Control);
        let write_result = terminate.and_then(|frame| {
            let stdin = instance
                .stdin
                .as_mut()
                .ok_or(ProcessSupervisorError::ProcessIo)?;
            write_record(stdin, &frame)
        });
        if let Err(error) = write_result {
            let _ = instance.hard_kill_and_reap();
            return Err(error);
        }

        let deadline = Instant::now() + self.config.graceful_stop_timeout;
        loop {
            if let Some(status) = instance
                .child
                .as_mut()
                .ok_or(ProcessSupervisorError::ProcessIo)?
                .try_wait()
                .map_err(|_| ProcessSupervisorError::ProcessIo)?
            {
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

impl Drop for ProcessHostSupervisor<'_> {
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

    /// Block for and authenticate the orchestrator's graceful termination request.
    pub fn receive_terminate(&mut self) -> Result<(), ProcessSupervisorError> {
        let frame = read_record(&mut self.reader)?;
        match self
            .control
            .receive_orchestrator(&frame)
            .map_err(|_| ProcessSupervisorError::Control)?
        {
            OrchestratorEvent::Terminate => Ok(()),
        }
    }

    /// Return this launch's UUID-v7 secure-session identity.
    pub fn session_id(&self) -> SessionId {
        self.control.session_id()
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
                ProcessSupervisorError::ActivePackageMismatch,
                "active package identity mismatch",
            ),
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
            control.ready(package_hash).unwrap();
            control.heartbeat().unwrap();
            control.receive_terminate().unwrap();
        });

        write_record(&mut parent_writer, offer.as_bytes()).unwrap();
        let hello = ClientHello::from_bytes(&read_record(&mut parent_reader).unwrap()).unwrap();
        let channel = bootstrap.accept(&hello).unwrap();
        let mut control = OrchestratorControl::new(channel, package_hash).unwrap();
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
