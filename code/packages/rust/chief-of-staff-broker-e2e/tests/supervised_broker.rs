//! The supervisor with per-agent brokers (D18S P2.6d-2b), end to end: a real
//! broker binary, verified by digest and holding only its agent's keys, a
//! scripted host, and the daemon's callback server over shared storage.
//!
//! On Linux the broker is confined by the Linux applier (P2.6d-3); on macOS
//! by the Seatbelt shim, pinned by digest like the broker (P2.7b); on
//! Windows by the AppContainer applier, its keys passed as handles
//! (P2.8b-2).
//!
//! Each test has an agent of its own. On Windows a broker's container is
//! named for its agent and runs one instance at a time, so tests running
//! in parallel must not share one.

#![cfg(any(target_os = "linux", target_os = "macos", windows))]

use std::collections::VecDeque;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use chief_of_staff_broker_launcher::{
    BrokerKeyFiles, BrokerProgram, KeyFileDeclaration, RelayConfig, VerifiedExecutable,
};
use chief_of_staff_broker_protocol::KeyKind;
use chief_of_staff_channel_crypto::{
    ChannelId, KeyEpoch, OriginatorSigningKey, ReceiverKeyPair, Sequence,
};
use chief_of_staff_channel_endpoints::{
    AgentId, ChannelDefinition, ChannelDefinitionStore, MessageId, MessageMetadata,
    MessageMetadataError, MessageMetadataSource, OriginatorIdentity, ReceiverIdentity,
};
use chief_of_staff_channel_store::{AppendRequest, ChannelStore};
use chief_of_staff_host_control_protocol::{ChannelBinding, ChannelBindingAccess, LaunchBindings};
use chief_of_staff_host_runtime::{
    sign_agent_package, AgentPackageRuntime, DenoLaunchPlan, PackageKeyType, PackageKeyring,
    TrustedPackageKey,
};
use chief_of_staff_pipeline_bindings::{HostPipelineBinding, PipelineId};
use chief_of_staff_process_supervisor::{
    ChannelBrokers, HostLaunchBindingProvider, HostProgram, LaunchBindingProviderError,
    MonotonicClock, ProcessHostSupervisor, ProcessSupervisorConfig, ProcessSupervisorError,
    SessionIdSource,
};
use chief_of_staff_secure_host_channel::SessionId;
use chief_of_staff_service_reconciler::{HostSupervisor, SupervisorObservation, SupervisorPhase};
use chief_of_staff_service_registry::{HostName, HostRegistration, PackagePath, RestartPolicy};
use chief_of_staff_tool_api::PrivilegeTier;
use storage_core::{InMemoryStorageBackend, StorageBackend};

const HOST: &str = env!("CARGO_BIN_EXE_broker-e2e-host");
const BROKER: &str = env!("CARGO_BIN_EXE_broker-e2e-broker");
#[cfg(target_os = "macos")]
const SHIM: &str = env!("CARGO_BIN_EXE_broker-e2e-seatbelt-shim");
const PACKAGE_SEED: [u8; 32] = [42; 32];
const KEY_ID: &str = "e2e-test";
const SEED: [u8; 32] = [0x31; 32];
const CMK: [u8; 32] = [0xa5; 32];

fn uuid_v7(tag: u8) -> [u8; 16] {
    let mut bytes = [0u8; 16];
    bytes[0] = tag;
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes
}

fn channel() -> ChannelId {
    ChannelId(uuid_v7(12))
}

fn agent(name: &str) -> AgentId {
    AgentId::new(name.as_bytes().to_vec()).unwrap()
}

/// A writing agent of this test's own (see the module documentation).
fn fresh_agent() -> AgentId {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    agent(&format!(
        "weather-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}

fn pipeline() -> PipelineId {
    PipelineId::new(uuid_v7(1)).unwrap()
}

/// A binary, pinned to the digest of what is there now.
fn pinned(path: &str) -> VerifiedExecutable {
    let digest = coding_adventures_sha256::sha256(&fs::read(path).unwrap());
    VerifiedExecutable::open(Path::new(path), digest).unwrap()
}

#[cfg(target_os = "linux")]
fn broker_program(_scratch: &Scratch) -> BrokerProgram {
    BrokerProgram::new(pinned(BROKER))
}

#[cfg(target_os = "macos")]
fn broker_program(_scratch: &Scratch) -> BrokerProgram {
    BrokerProgram::new(pinned(BROKER), pinned(SHIM)).unwrap()
}

/// On Windows, a copy in the scratch directory: the applier runs only a
/// binary at its final path, whose file no one else may change, and the
/// build directory may be writable by others.
#[cfg(windows)]
fn broker_program(scratch: &Scratch) -> BrokerProgram {
    let copy = scratch.0.join("broker-e2e-broker.exe");
    fs::copy(BROKER, &copy).unwrap();
    BrokerProgram::new(pinned(copy.to_str().unwrap()))
}

// ---- a scratch area: the package, the key files, results -----------------

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let name = format!(
            "broker-e2e-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        // Under Cargo's per-target scratch directory, not a shared /tmp,
        // and created 0700 by this call alone (review round 1): the key
        // files live here.
        #[cfg(unix)]
        {
            let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
            let _ = fs::remove_dir_all(&path);
            fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
            Self(fs::canonicalize(path).unwrap())
        }
        // On Windows, under the user's own temporary directory, whose
        // list names only the user, SYSTEM and Administrators, as the
        // key files' directory must; the build directory's may name
        // others. The path is the canonical one, without the `\\?\`
        // prefix, which is the spelling the applier compares.
        #[cfg(windows)]
        {
            let path = std::env::temp_dir().join(name);
            let _ = fs::remove_dir_all(&path);
            fs::create_dir(&path).unwrap();
            let canonical = fs::canonicalize(path).unwrap();
            let text = canonical.to_str().unwrap();
            Self(PathBuf::from(text.strip_prefix(r"\\?\").unwrap_or(text)))
        }
    }

    /// A signed Deno package whose markers script the host.
    fn package(&self, markers: &[(&str, &str)]) -> HostRegistration {
        let path = self.0.join("package");
        fs::create_dir_all(path.join("code")).unwrap();
        fs::write(path.join("manifest.json"), b"{\"runtime\":\"typescript\"}").unwrap();
        DenoLaunchPlan::write_launch_script(&path).unwrap();
        fs::write(path.join("code/agent_runtime.ts"), b"console.log('e2e');\n").unwrap();
        for (name, content) in markers {
            fs::write(path.join(name), content).unwrap();
        }
        let (_, secret_key) = coding_adventures_ed25519::generate_keypair(&PACKAGE_SEED);
        let digest = sign_agent_package(&path, KEY_ID, &secret_key).unwrap();
        HostRegistration::new(
            HostName::new("weather").unwrap(),
            PackagePath::new(path.to_str().unwrap()).unwrap(),
            digest,
            RestartPolicy::Always,
        )
    }

    /// A key file the owner-only policy accepts: mode 0600 on Unix; on
    /// Windows owned by the user, with a protected list that names only
    /// the user.
    fn key(&self, name: &str, bytes: &[u8; 32]) -> PathBuf {
        let path = self.0.join(name);
        #[cfg(unix)]
        {
            fs::write(&path, bytes).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        }
        #[cfg(windows)]
        chief_of_staff_daemon_secret_file::write_test_secret(&path, bytes);
        path
    }

    fn result(&self) -> PathBuf {
        self.0.join("RESULT")
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

// ---- the supervisor's collaborators ---------------------------------------

fn launch_bindings() -> LaunchBindings {
    LaunchBindings::new(
        vec![ChannelBinding::new("reports", ChannelBindingAccess::Write, uuid_v7(12)).unwrap()],
        None,
    )
    .unwrap()
}

/// Resolves the one binding, for the world's agent, until unwired.
struct Bindings {
    wired: Mutex<bool>,
    agent: AgentId,
}

impl HostLaunchBindingProvider for Bindings {
    fn launch_bindings(
        &self,
        _registration: &HostRegistration,
        _runtime: AgentPackageRuntime,
    ) -> Result<LaunchBindings, LaunchBindingProviderError> {
        Ok(launch_bindings())
    }

    fn pipeline_binding(
        &self,
        registration: &HostRegistration,
    ) -> Result<HostPipelineBinding, LaunchBindingProviderError> {
        if !*self.wired.lock().unwrap() {
            return Err(LaunchBindingProviderError);
        }
        Ok(HostPipelineBinding::new(
            pipeline(),
            registration.clone(),
            self.agent.clone(),
            launch_bindings(),
        ))
    }
}

#[derive(Default)]
struct Clock(AtomicU64);

impl MonotonicClock for Clock {
    fn now_ns(&self) -> u64 {
        self.0.fetch_add(1, Ordering::SeqCst) + 1
    }
}

struct Sessions(u8);

impl SessionIdSource for Sessions {
    fn next_session(&mut self) -> Result<SessionId, ProcessSupervisorError> {
        self.0 = self.0.wrapping_add(1);
        let mut bytes = uuid_v7(200);
        bytes[15] = self.0;
        SessionId::new(bytes).map_err(|_| ProcessSupervisorError::SessionGeneration)
    }
}

struct Metadata(Mutex<VecDeque<MessageMetadata>>);

impl MessageMetadataSource for Metadata {
    fn next_metadata(&self) -> Result<MessageMetadata, MessageMetadataError> {
        self.0
            .lock()
            .unwrap()
            .pop_front()
            .ok_or_else(|| MessageMetadataError::new("exhausted"))
    }
}

fn keyring() -> PackageKeyring {
    let (public_key, _) = coding_adventures_ed25519::generate_keypair(&PACKAGE_SEED);
    let mut keyring = PackageKeyring::new();
    keyring
        .trust(
            TrustedPackageKey::new(
                KEY_ID,
                PackageKeyType::Production,
                public_key,
                PrivilegeTier::Tier3,
            )
            .unwrap(),
        )
        .unwrap();
    keyring
}

struct World {
    scratch: Scratch,
    backend: Arc<dyn StorageBackend>,
    bindings: Arc<Bindings>,
    agent: AgentId,
}

impl World {
    fn new() -> Self {
        let originator = fresh_agent();
        let backend: Arc<dyn StorageBackend> = Arc::new(InMemoryStorageBackend::new());
        ChannelDefinitionStore::new(&*backend)
            .create(
                &ChannelDefinition::new(
                    channel(),
                    OriginatorIdentity {
                        agent_id: originator.clone(),
                        public_key: OriginatorSigningKey::from_seed(SEED).public_key(),
                    },
                    vec![ReceiverIdentity {
                        agent_id: agent("sink"),
                        public_key: ReceiverKeyPair::from_private_key([0x42; 32])
                            .unwrap()
                            .public_key(),
                    }],
                    1,
                    KeyEpoch(0),
                )
                .unwrap(),
            )
            .unwrap();
        Self {
            scratch: Scratch::new(),
            backend,
            bindings: Arc::new(Bindings {
                wired: Mutex::new(true),
                agent: originator.clone(),
            }),
            agent: originator,
        }
    }

    fn supervisor(&self, seed: &[u8; 32]) -> ProcessHostSupervisor {
        let declare = |kind, path| KeyFileDeclaration {
            pipeline_id: pipeline(),
            agent_id: self.agent.clone(),
            channel_id: channel(),
            kind,
            path,
        };
        let keys = BrokerKeyFiles::new(vec![
            declare(
                KeyKind::OriginatorSigningSeed,
                self.scratch.key("seed", seed),
            ),
            declare(KeyKind::ChannelMasterKey, self.scratch.key("cmk", &CMK)),
        ])
        .unwrap();
        let program = Arc::new(broker_program(&self.scratch));
        let metadata = Arc::new(Metadata(Mutex::new(
            (1..=10)
                .map(|n| MessageMetadata {
                    message_id: MessageId::from_uuid_v7(uuid_v7(100 + n)).unwrap(),
                    timestamp_ns: u64::from(n),
                })
                .collect(),
        )));
        let config = ProcessSupervisorConfig::new(
            HostProgram::new(HOST, std::iter::empty::<&str>()).unwrap(),
            Duration::from_secs(10),
            Duration::from_secs(2),
        )
        .unwrap();
        let bindings: Arc<dyn HostLaunchBindingProvider> = self.bindings.clone();
        ProcessHostSupervisor::new(
            config,
            Arc::new(keyring()),
            bindings,
            Arc::new(coding_adventures_x3dh::generate_identity_keypair()),
            Arc::new(Clock::default()),
            Box::new(Sessions(0)),
        )
        .with_channel_brokers(ChannelBrokers::new(
            program,
            keys,
            Arc::clone(&self.backend),
            metadata,
            RelayConfig::default(),
            Duration::from_secs(10),
        ))
    }
}

fn await_phase(
    supervisor: &mut ProcessHostSupervisor,
    registration: &HostRegistration,
    expected: impl Fn(SupervisorPhase) -> bool,
) {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if let Ok(SupervisorObservation::Instance(instance)) = supervisor.inspect(registration) {
            if expected(instance.phase()) {
                return;
            }
        }
        assert!(Instant::now() < deadline, "timed out waiting for the phase");
        thread::sleep(Duration::from_millis(10));
    }
}

fn running(phase: SupervisorPhase) -> bool {
    phase == SupervisorPhase::Running
}

fn exited(phase: SupervisorPhase) -> bool {
    matches!(phase, SupervisorPhase::Exited { .. })
}

/// Keep the supervisor refreshing until `done`, as the reconciler would.
fn drive_until(
    supervisor: &mut ProcessHostSupervisor,
    registration: &HostRegistration,
    mut done: impl FnMut() -> bool,
) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while !done() {
        let _ = supervisor.inspect(registration);
        assert!(Instant::now() < deadline, "timed out");
        thread::sleep(Duration::from_millis(10));
    }
}

#[cfg(unix)]
fn alive(pid: u32) -> bool {
    // SAFETY: signal 0 only checks that the process exists.
    unsafe { libc::kill(pid as libc::pid_t, 0) == 0 }
}

/// Killed from outside, as a crash would end it.
#[cfg(unix)]
fn kill(pid: u32) {
    // SAFETY: killing the broker this test started.
    unsafe { libc::kill(pid as libc::pid_t, libc::SIGKILL) };
}

/// Whether `pid` names a process that has not exited. A process that
/// cannot be opened for some reason other than not existing counts as
/// alive, so a refusal never passes for an end.
#[cfg(windows)]
fn alive(pid: u32) -> bool {
    use windows_sys::Win32::Foundation::{
        CloseHandle, GetLastError, ERROR_INVALID_PARAMETER, STILL_ACTIVE,
    };
    use windows_sys::Win32::System::Threading::{
        GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    // SAFETY: the handle is checked, used for one query and closed; the
    // exit code is written to a u32.
    unsafe {
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if process == 0 {
            return GetLastError() != ERROR_INVALID_PARAMETER;
        }
        let mut code = 0u32;
        let queried = GetExitCodeProcess(process, &mut code) != 0;
        CloseHandle(process);
        !queried || code == STILL_ACTIVE as u32
    }
}

#[cfg(windows)]
fn kill(pid: u32) {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};
    // SAFETY: the handle is checked, used once and closed.
    unsafe {
        let process = OpenProcess(PROCESS_TERMINATE, 0, pid);
        assert_ne!(process, 0, "the broker could not be opened to end it");
        TerminateProcess(process, 1);
        CloseHandle(process);
    }
}

// ---- the tests ------------------------------------------------------------

#[test]
fn a_host_s_publish_goes_through_its_own_broker() {
    let world = World::new();
    let result = world.scratch.result();
    let registration = world
        .scratch
        .package(&[("PUBLISH", result.to_str().unwrap())]);
    let mut supervisor = world.supervisor(&SEED);
    supervisor.start(&registration).unwrap();
    let broker = supervisor
        .broker_process_id(registration.host_name())
        .expect("a broker for the host");
    assert!(alive(broker));

    drive_until(&mut supervisor, &registration, || result.is_file());
    let answered = fs::read_to_string(&result).unwrap();
    assert!(answered.starts_with("Published"), "{answered}");
    // In storage: ciphertext the broker sealed, at sequence 0.
    let page = ChannelStore::new(&*world.backend, channel())
        .read_messages(Sequence(0), 10)
        .unwrap();
    assert_eq!(page.messages.len(), 1);

    // Stopping the host ends its broker.
    supervisor.stop(registration.host_name()).unwrap();
    assert_eq!(supervisor.broker_process_id(registration.host_name()), None);
    assert!(!alive(broker), "the broker outlived its host");
}

#[test]
fn a_dead_broker_ends_its_host() {
    let world = World::new();
    let registration = world.scratch.package(&[]);
    let mut supervisor = world.supervisor(&SEED);
    supervisor.start(&registration).unwrap();
    await_phase(&mut supervisor, &registration, running);
    let broker = supervisor
        .broker_process_id(registration.host_name())
        .unwrap();
    kill(broker);
    let deadline = Instant::now() + Duration::from_secs(30);
    let ended = loop {
        match supervisor.inspect(&registration) {
            Err(ProcessSupervisorError::Broker) => break true,
            Ok(SupervisorObservation::Instance(instance)) if exited(instance.phase()) => {
                break true
            }
            _ => {}
        }
        if Instant::now() > deadline {
            break false;
        }
        thread::sleep(Duration::from_millis(10));
    };
    assert!(ended, "the host outlived its broker");
    await_phase(&mut supervisor, &registration, exited);
}

#[test]
fn a_host_that_exits_takes_its_broker_with_it() {
    let world = World::new();
    let registration = world.scratch.package(&[("EXIT_AT_ONCE", "1")]);
    let mut supervisor = world.supervisor(&SEED);
    supervisor.start(&registration).unwrap();
    let broker = supervisor
        .broker_process_id(registration.host_name())
        .unwrap();
    await_phase(&mut supervisor, &registration, exited);
    assert!(!alive(broker), "the broker outlived its host");
}

#[test]
fn a_wrong_key_file_stops_the_launch_before_the_host_starts() {
    let world = World::new();
    let registration = world.scratch.package(&[]);
    let mut supervisor = world.supervisor(&[0x32; 32]);
    assert_eq!(
        supervisor.start(&registration),
        Err(ProcessSupervisorError::BrokerLaunch)
    );
    assert_eq!(
        supervisor.inspect(&registration),
        Ok(SupervisorObservation::absent())
    );
}

#[test]
fn a_reservation_left_by_a_dead_broker_is_abandoned_at_launch() {
    let world = World::new();
    // What a broker killed mid-publish leaves behind.
    let store = ChannelStore::new(&*world.backend, channel());
    store.initialize().unwrap();
    store
        .reserve_append(
            AppendRequest {
                message_id: uuid_v7(99),
                timestamp_ns: 1,
                originator_id: b"weather".to_vec(),
                key_epoch: KeyEpoch(0),
                content_type: "text/plain".to_owned(),
            },
            b"lost",
        )
        .unwrap();
    let result = world.scratch.result();
    let registration = world
        .scratch
        .package(&[("PUBLISH", result.to_str().unwrap())]);
    let mut supervisor = world.supervisor(&SEED);
    supervisor.start(&registration).unwrap();
    drive_until(&mut supervisor, &registration, || result.is_file());
    let answered = fs::read_to_string(&result).unwrap();
    // Not stuck behind the old reservation: published, at the next sequence.
    assert!(answered.contains("sequence: 1"), "{answered}");
}

#[test]
fn a_host_with_no_binding_resolution_is_refused() {
    let world = World::new();
    *world.bindings.wired.lock().unwrap() = false;
    let registration = world.scratch.package(&[]);
    let mut supervisor = world.supervisor(&SEED);
    assert_eq!(
        supervisor.start(&registration),
        Err(ProcessSupervisorError::LaunchBindings)
    );
}
