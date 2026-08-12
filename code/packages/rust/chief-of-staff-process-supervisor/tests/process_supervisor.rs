use chief_of_staff_host_control_protocol::{
    ChannelBinding, ChannelBindingAccess, CompletionFinishReason, CompletionProvider,
    CompletionUsage, DataPlaneFailure, DataPlaneRequest, DataPlaneResponse, LaunchBindings,
    LevelOneModelBinding, ModelToolCall, ModelToolResult, RequestId, ToolCompletionOutput,
    ToolCompletionResult,
};
use chief_of_staff_host_data_plane::HostDataPlaneDispatcher;
use chief_of_staff_host_runtime::{
    AgentPackageRuntime, DenoLaunchPlan, PackageKeyType, PackageKeyring, TrustedPackageKey,
};
use chief_of_staff_process_supervisor::{
    DenyHostLaunchBindings, HostLaunchBindingProvider, HostProgram, LaunchBindingProviderError,
    MonotonicClock, ProcessHostSupervisor, ProcessSupervisorConfig, ProcessSupervisorError,
    SessionIdSource,
};
use chief_of_staff_secure_host_channel::SessionId;
use chief_of_staff_service_reconciler::{HostSupervisor, SupervisorObservation, SupervisorPhase};
use chief_of_staff_service_registry::{HostName, HostRegistration, PackagePath, RestartPolicy};
use chief_of_staff_tool_api::PrivilegeTier;
use coding_adventures_ed25519::{generate_keypair, sign};
use coding_adventures_sha256::Sha256Hasher;
use coding_adventures_x3dh::generate_identity_keypair;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const TEST_SEED: [u8; 32] = [42; 32];
const TEST_KEY_ID: &str = "prod-test";
const HASH_DOMAIN: &[u8] = b"chief-agent-package-v1\0";

struct TestPackage {
    path: PathBuf,
    digest: [u8; 32],
}

impl TestPackage {
    fn new(label: &str, marker: Option<&str>) -> Self {
        let path = std::env::temp_dir().join(format!(
            "chief-process-supervisor-{label}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(path.join("code")).unwrap();
        fs::write(path.join("manifest.json"), b"{\"runtime\":\"typescript\"}").unwrap();
        DenoLaunchPlan::write_launch_script(&path).unwrap();
        fs::write(
            path.join("code/agent_runtime.ts"),
            b"console.log('fixture');\n",
        )
        .unwrap();
        if let Some(marker) = marker {
            fs::write(path.join(marker), b"1").unwrap();
        }
        fs::write(path.join("PUBKEY_ID"), TEST_KEY_ID).unwrap();
        let digest = package_digest(&path);
        let (_, secret_key) = generate_keypair(&TEST_SEED);
        fs::write(path.join("SIGNATURE"), sign(&digest, &secret_key)).unwrap();
        Self { path, digest }
    }

    fn skill(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "chief-process-supervisor-{label}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join("manifest.json"), b"{\"runtime\":\"skill\"}").unwrap();
        fs::write(
            path.join("SKILL.md"),
            b"# Weather\n\nDescribe the weather.\n",
        )
        .unwrap();
        fs::write(path.join("PUBKEY_ID"), TEST_KEY_ID).unwrap();
        let digest = package_digest(&path);
        let (_, secret_key) = generate_keypair(&TEST_SEED);
        fs::write(path.join("SIGNATURE"), sign(&digest, &secret_key)).unwrap();
        Self { path, digest }
    }

    fn registration(&self, host: &str) -> HostRegistration {
        HostRegistration::new(
            HostName::new(host).unwrap(),
            PackagePath::new(self.path.to_str().unwrap()).unwrap(),
            self.digest,
            RestartPolicy::Always,
        )
    }
}

impl Drop for TestPackage {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn package_digest(path: &Path) -> [u8; 32] {
    let mut files = if path.join("SKILL.md").is_file() {
        vec![
            (
                "SKILL.md".to_owned(),
                fs::read(path.join("SKILL.md")).unwrap(),
            ),
            (
                "manifest.json".to_owned(),
                fs::read(path.join("manifest.json")).unwrap(),
            ),
        ]
    } else {
        vec![
            (
                "code/agent_runtime.ts".to_owned(),
                fs::read(path.join("code/agent_runtime.ts")).unwrap(),
            ),
            (
                "launch.sh".to_owned(),
                fs::read(path.join("launch.sh")).unwrap(),
            ),
            (
                "manifest.json".to_owned(),
                fs::read(path.join("manifest.json")).unwrap(),
            ),
        ]
    };
    for marker in [
        "DATA_PLANE",
        "EXIT_BEFORE_READY",
        "IGNORE_TERMINATE",
        "NO_HEARTBEAT",
        "OVERSIZED_BOOTSTRAP",
        "SILENT_BOOTSTRAP",
        "WRONG_READY",
    ] {
        let marker_path = path.join(marker);
        if marker_path.is_file() {
            files.push((marker.to_owned(), fs::read(marker_path).unwrap()));
        }
    }
    files.sort_by(|left, right| left.0.cmp(&right.0));
    let mut hasher = Sha256Hasher::new();
    hasher.update(HASH_DOMAIN);
    for (name, bytes) in files {
        hasher.update(&(name.len() as u64).to_be_bytes());
        hasher.update(name.as_bytes());
        hasher.update(&(bytes.len() as u64).to_be_bytes());
        hasher.update(&bytes);
    }
    hasher.digest()
}

fn uuid_v7(last: u8) -> [u8; 16] {
    let mut bytes = [0u8; 16];
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes[15] = last;
    bytes
}

fn tool_completion_result() -> ToolCompletionResult {
    ToolCompletionResult {
        output: ToolCompletionOutput::ToolCall(ModelToolCall {
            call_id: "call-1".to_string(),
            name: "smart_home.list_entities".to_string(),
            arguments: serde_json::json!({}),
        }),
        model: "test-model".to_string(),
        provider: CompletionProvider {
            vendor: "fixture".to_string(),
            model_family: "weather".to_string(),
            model_version: "v1".to_string(),
            endpoint: None,
        },
        usage: CompletionUsage {
            input_tokens: 1,
            output_tokens: 1,
            cached_tokens: 0,
        },
        finish_reason: CompletionFinishReason::Stop,
        latency_ms: 1,
        polyfill_used: false,
    }
}

fn keyring() -> PackageKeyring {
    let (public_key, _) = generate_keypair(&TEST_SEED);
    let mut keyring = PackageKeyring::new();
    keyring
        .trust(
            TrustedPackageKey::new(
                TEST_KEY_ID,
                PackageKeyType::Production,
                public_key,
                PrivilegeTier::Tier3,
            )
            .unwrap(),
        )
        .unwrap();
    keyring
}

#[derive(Default)]
struct TestClock(AtomicU64);

impl MonotonicClock for TestClock {
    fn now_ns(&self) -> u64 {
        self.0.fetch_add(1, Ordering::SeqCst) + 1
    }
}

struct TestSessions(u8);

impl SessionIdSource for TestSessions {
    fn next_session(&mut self) -> Result<SessionId, ProcessSupervisorError> {
        self.0 = self.0.wrapping_add(1);
        let mut bytes = [0u8; 16];
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        bytes[15] = self.0;
        SessionId::new(bytes).map_err(|_| ProcessSupervisorError::SessionGeneration)
    }
}

struct TestLaunchBindings;

impl HostLaunchBindingProvider for TestLaunchBindings {
    fn launch_bindings(
        &self,
        _registration: &HostRegistration,
        runtime: AgentPackageRuntime,
    ) -> Result<LaunchBindings, LaunchBindingProviderError> {
        let channels = vec![
            ChannelBinding::new("weather-requests", ChannelBindingAccess::Read, uuid_v7(1))
                .unwrap(),
            ChannelBinding::new("weather-reports", ChannelBindingAccess::Write, uuid_v7(2))
                .unwrap(),
        ];
        let model = match runtime {
            AgentPackageRuntime::Skill => {
                Some(LevelOneModelBinding::new("test-model", 0.0, 128).unwrap())
            }
            AgentPackageRuntime::Deno => None,
        };
        LaunchBindings::new(channels, model).map_err(|_| LaunchBindingProviderError)
    }
}

fn new_supervisor(
    keyring: Arc<PackageKeyring>,
    identity: Arc<coding_adventures_x3dh::IdentityKeyPair>,
    bootstrap_timeout: Duration,
    graceful_timeout: Duration,
) -> ProcessHostSupervisor {
    new_supervisor_with_bindings(
        keyring,
        Arc::new(TestLaunchBindings),
        identity,
        bootstrap_timeout,
        graceful_timeout,
    )
}

fn new_supervisor_with_bindings(
    keyring: Arc<PackageKeyring>,
    launch_bindings: Arc<dyn HostLaunchBindingProvider>,
    identity: Arc<coding_adventures_x3dh::IdentityKeyPair>,
    bootstrap_timeout: Duration,
    graceful_timeout: Duration,
) -> ProcessHostSupervisor {
    let program = HostProgram::new(
        env!("CARGO_BIN_EXE_process-supervisor-test-child"),
        std::iter::empty::<&str>(),
    )
    .unwrap();
    let config =
        ProcessSupervisorConfig::new(program, bootstrap_timeout, graceful_timeout).unwrap();
    ProcessHostSupervisor::new(
        config,
        keyring,
        launch_bindings,
        identity,
        Arc::new(TestClock::default()),
        Box::new(TestSessions(0)),
    )
}

fn await_phase(
    supervisor: &mut ProcessHostSupervisor,
    registration: &HostRegistration,
    expected: SupervisorPhase,
) -> chief_of_staff_service_reconciler::SupervisorInstance {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match supervisor.inspect(registration) {
            Ok(SupervisorObservation::Instance(instance)) if instance.phase() == expected => {
                return instance
            }
            Ok(_) => {}
            Err(error) => panic!("unexpected supervisor error: {error}"),
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for {expected:?}"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn real_child_reaches_running_and_stops_gracefully() {
    let package = TestPackage::new("graceful", None);
    let registration = package.registration("fixture-host");
    let keyring = Arc::new(keyring());
    let identity = Arc::new(generate_identity_keypair());
    let mut supervisor = new_supervisor(
        Arc::clone(&keyring),
        Arc::clone(&identity),
        Duration::from_secs(3),
        Duration::from_secs(1),
    );

    supervisor.start(&registration).unwrap();
    let starting = supervisor.inspect(&registration).unwrap();
    let first_pid = match starting {
        SupervisorObservation::Instance(instance) => instance.process_id().unwrap(),
        SupervisorObservation::Absent => panic!("spawned child was absent"),
    };
    supervisor.start(&registration).unwrap();
    let running = await_phase(&mut supervisor, &registration, SupervisorPhase::Running);
    assert_eq!(running.process_id(), Some(first_pid));
    assert_eq!(running.package_hash(), &package.digest);
    assert!(running.last_heartbeat_ns().is_some());
    assert!(running.control_channel_id().is_some());

    supervisor.stop(registration.host_name()).unwrap();
    let exited = await_phase(
        &mut supervisor,
        &registration,
        SupervisorPhase::Exited { exit_code: Some(0) },
    );
    assert_eq!(exited.process_id(), None);
    supervisor.stop(registration.host_name()).unwrap();
}

#[test]
fn real_child_exchanges_all_authenticated_data_plane_operations() {
    let package = TestPackage::new("data-plane", Some("DATA_PLANE"));
    let registration = package.registration("data-plane-host");
    let host_name = HostName::new("data-plane-host").unwrap();
    let mut supervisor = new_supervisor(
        Arc::new(keyring()),
        Arc::new(generate_identity_keypair()),
        Duration::from_secs(2),
        Duration::from_secs(2),
    );
    assert_eq!(
        supervisor.pending_data_plane_request(&host_name),
        Err(ProcessSupervisorError::HostNotFound)
    );
    assert_eq!(
        supervisor.respond_data_plane(
            &host_name,
            DataPlaneResponse::Failed {
                id: RequestId::new(1).unwrap(),
                failure: DataPlaneFailure::Unavailable,
            },
        ),
        Err(ProcessSupervisorError::HostNotFound)
    );
    supervisor.start(&registration).unwrap();

    let deadline = Instant::now() + Duration::from_secs(5);
    let mut operations = Vec::new();
    while operations.len() < 6 {
        if let Some(request) = supervisor.pending_data_plane_request(&host_name).unwrap() {
            let response = match request {
                DataPlaneRequest::Receive { id, .. } => {
                    operations.push("receive");
                    DataPlaneResponse::Received {
                        id,
                        messages: Vec::new(),
                    }
                }
                DataPlaneRequest::Publish { id, .. } => {
                    operations.push("publish");
                    DataPlaneResponse::Published {
                        id,
                        message_id: uuid_v7(4),
                        sequence: 1,
                        timestamp_ns: 10,
                    }
                }
                DataPlaneRequest::Acknowledge { id, .. } => {
                    operations.push("acknowledge");
                    DataPlaneResponse::Acknowledged { id, sequence: 2 }
                }
                DataPlaneRequest::Complete { id, .. } => {
                    operations.push("complete");
                    DataPlaneResponse::Failed {
                        id,
                        failure: DataPlaneFailure::Unavailable,
                    }
                }
                DataPlaneRequest::CompleteWithTools { id, .. } => {
                    operations.push("complete_with_tools");
                    DataPlaneResponse::ToolCompleted {
                        id,
                        result: Box::new(tool_completion_result()),
                    }
                }
                DataPlaneRequest::ExecuteTool { id, call } => {
                    operations.push("execute_tool");
                    DataPlaneResponse::ToolExecuted {
                        id,
                        result: Box::new(ModelToolResult {
                            call: *call,
                            output: serde_json::json!({"entities": []}),
                            is_error: false,
                        }),
                    }
                }
            };
            supervisor.respond_data_plane(&host_name, response).unwrap();
        } else {
            assert!(Instant::now() < deadline, "timed out waiting for request");
            thread::sleep(Duration::from_millis(10));
        }
    }
    assert_eq!(
        operations,
        [
            "receive",
            "publish",
            "acknowledge",
            "complete",
            "complete_with_tools",
            "execute_tool"
        ]
    );
    supervisor.stop(&host_name).unwrap();
    assert_eq!(supervisor.pending_data_plane_request(&host_name), Ok(None));
}

#[derive(Default)]
struct TestDataPlaneDispatcher {
    operations: Mutex<Vec<&'static str>>,
}

impl HostDataPlaneDispatcher for TestDataPlaneDispatcher {
    fn dispatch(
        &self,
        _registration: &HostRegistration,
        request: &DataPlaneRequest,
    ) -> DataPlaneResponse {
        match request {
            DataPlaneRequest::Receive { id, .. } => {
                self.operations.lock().unwrap().push("receive");
                DataPlaneResponse::Received {
                    id: *id,
                    messages: Vec::new(),
                }
            }
            DataPlaneRequest::Publish { id, .. } => {
                self.operations.lock().unwrap().push("publish");
                DataPlaneResponse::Published {
                    id: *id,
                    message_id: uuid_v7(4),
                    sequence: 1,
                    timestamp_ns: 10,
                }
            }
            DataPlaneRequest::Acknowledge { id, .. } => {
                self.operations.lock().unwrap().push("acknowledge");
                DataPlaneResponse::Acknowledged {
                    id: *id,
                    sequence: 2,
                }
            }
            DataPlaneRequest::Complete { id, .. } => {
                self.operations.lock().unwrap().push("complete");
                DataPlaneResponse::Failed {
                    id: *id,
                    failure: DataPlaneFailure::Unavailable,
                }
            }
            DataPlaneRequest::CompleteWithTools { id, .. } => {
                self.operations.lock().unwrap().push("complete_with_tools");
                DataPlaneResponse::ToolCompleted {
                    id: *id,
                    result: Box::new(tool_completion_result()),
                }
            }
            DataPlaneRequest::ExecuteTool { id, call } => {
                self.operations.lock().unwrap().push("execute_tool");
                DataPlaneResponse::ToolExecuted {
                    id: *id,
                    result: Box::new(ModelToolResult {
                        call: (**call).clone(),
                        output: serde_json::json!({"entities": []}),
                        is_error: false,
                    }),
                }
            }
        }
    }
}

#[test]
fn injected_dispatcher_answers_authenticated_requests_automatically() {
    let package = TestPackage::new("automatic-data-plane", Some("DATA_PLANE"));
    let registration = package.registration("automatic-data-plane-host");
    let dispatcher = Arc::new(TestDataPlaneDispatcher::default());
    let mut supervisor = new_supervisor(
        Arc::new(keyring()),
        Arc::new(generate_identity_keypair()),
        Duration::from_secs(2),
        Duration::from_secs(2),
    )
    .with_data_plane_dispatcher(dispatcher.clone());
    supervisor.start(&registration).unwrap();

    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        supervisor.inspect(&registration).unwrap();
        if dispatcher.operations.lock().unwrap().len() == 6 {
            break;
        }
        assert!(Instant::now() < deadline, "timed out waiting for dispatch");
        thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(
        *dispatcher.operations.lock().unwrap(),
        [
            "receive",
            "publish",
            "acknowledge",
            "complete",
            "complete_with_tools",
            "execute_tool"
        ]
    );
    assert_eq!(
        supervisor
            .pending_data_plane_request(registration.host_name())
            .unwrap(),
        None
    );
    supervisor.stop(registration.host_name()).unwrap();
    let exited = await_phase(
        &mut supervisor,
        &registration,
        SupervisorPhase::Exited { exit_code: Some(0) },
    );
    assert_eq!(exited.process_id(), None);
}

#[test]
fn signed_skill_package_receives_authenticated_runtime_dispatch() {
    let package = TestPackage::skill("skill-runtime");
    let registration = package.registration("skill-host");
    let keyring = Arc::new(keyring());
    let identity = Arc::new(generate_identity_keypair());
    let mut supervisor = new_supervisor(
        Arc::clone(&keyring),
        Arc::clone(&identity),
        Duration::from_secs(3),
        Duration::from_secs(1),
    );

    supervisor.start(&registration).unwrap();
    let running = await_phase(&mut supervisor, &registration, SupervisorPhase::Running);
    assert_eq!(running.package_hash(), &package.digest);
    supervisor.stop(registration.host_name()).unwrap();
    let exited = await_phase(
        &mut supervisor,
        &registration,
        SupervisorPhase::Exited { exit_code: Some(0) },
    );
    assert_eq!(exited.process_id(), None);
}

#[test]
fn exact_hash_is_checked_before_spawn_and_active_hash_cannot_change() {
    let package = TestPackage::new("identity", None);
    let mut registration = package.registration("identity-host");
    let keyring = Arc::new(keyring());
    let identity = Arc::new(generate_identity_keypair());
    let mut supervisor = new_supervisor(
        Arc::clone(&keyring),
        Arc::clone(&identity),
        Duration::from_secs(3),
        Duration::from_millis(200),
    );

    let mut wrong_hash = package.digest;
    wrong_hash[0] ^= 0xff;
    registration = HostRegistration::new(
        registration.host_name().clone(),
        registration.package_path().clone(),
        wrong_hash,
        RestartPolicy::Always,
    );
    assert_eq!(
        supervisor.start(&registration),
        Err(ProcessSupervisorError::PackageMismatch)
    );

    let registration = package.registration("identity-host");
    supervisor.start(&registration).unwrap();
    let different = HostRegistration::new(
        registration.host_name().clone(),
        registration.package_path().clone(),
        wrong_hash,
        RestartPolicy::Always,
    );
    assert_eq!(
        supervisor.start(&different),
        Err(ProcessSupervisorError::ActivePackageMismatch)
    );
    supervisor.stop(registration.host_name()).unwrap();
}

#[test]
fn bootstrap_timeout_and_oversized_record_are_cleaned_up() {
    for (label, marker, timeout, expected) in [
        (
            "timeout",
            "SILENT_BOOTSTRAP",
            Duration::from_millis(250),
            ProcessSupervisorError::BootstrapTimeout,
        ),
        (
            "oversized",
            "OVERSIZED_BOOTSTRAP",
            Duration::from_secs(3),
            ProcessSupervisorError::Framing,
        ),
    ] {
        let package = TestPackage::new(label, Some(marker));
        let registration = package.registration(&format!("{label}-host"));
        let keyring = Arc::new(keyring());
        let identity = Arc::new(generate_identity_keypair());
        let mut supervisor = new_supervisor(keyring, identity, timeout, Duration::from_millis(100));
        assert_eq!(supervisor.start(&registration), Err(expected));
        assert_eq!(
            supervisor.inspect(&registration).unwrap(),
            SupervisorObservation::Absent
        );
    }
}

#[test]
fn wrong_ready_and_exit_before_ready_fail_closed() {
    for (label, marker, expected) in [
        (
            "wrong-ready",
            "WRONG_READY",
            ProcessSupervisorError::Control,
        ),
        (
            "early-exit",
            "EXIT_BEFORE_READY",
            ProcessSupervisorError::Framing,
        ),
    ] {
        let package = TestPackage::new(label, Some(marker));
        let registration = package.registration(&format!("{label}-host"));
        let keyring = Arc::new(keyring());
        let identity = Arc::new(generate_identity_keypair());
        let mut supervisor = new_supervisor(
            keyring,
            identity,
            Duration::from_secs(3),
            Duration::from_millis(100),
        );
        supervisor.start(&registration).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Err(error) = supervisor.inspect(&registration) {
                assert_eq!(error, expected);
                break;
            }
            assert!(Instant::now() < deadline, "failure was not observed");
            thread::sleep(Duration::from_millis(10));
        }
        let exited = supervisor.inspect(&registration).unwrap();
        assert!(matches!(
            exited,
            SupervisorObservation::Instance(ref instance)
                if matches!(instance.phase(), SupervisorPhase::Exited { .. })
        ));
    }
}

#[test]
fn graceful_timeout_hard_kills_and_drop_reaps() {
    let package = TestPackage::new("hard-kill", Some("IGNORE_TERMINATE"));
    let registration = package.registration("hard-kill-host");
    let keyring = Arc::new(keyring());
    let identity = Arc::new(generate_identity_keypair());
    let mut supervisor = new_supervisor(
        Arc::clone(&keyring),
        Arc::clone(&identity),
        Duration::from_secs(3),
        Duration::from_millis(100),
    );
    supervisor.start(&registration).unwrap();
    await_phase(&mut supervisor, &registration, SupervisorPhase::Running);
    let started = Instant::now();
    supervisor.stop(registration.host_name()).unwrap();
    assert!(started.elapsed() >= Duration::from_millis(90));
    assert!(matches!(
        supervisor.inspect(&registration).unwrap(),
        SupervisorObservation::Instance(ref instance)
            if matches!(instance.phase(), SupervisorPhase::Exited { .. })
    ));

    let second = TestPackage::new("drop", None);
    let second_registration = second.registration("drop-host");
    let mut dropped = new_supervisor(
        keyring,
        identity,
        Duration::from_secs(3),
        Duration::from_millis(100),
    );
    dropped.start(&second_registration).unwrap();
    await_phase(&mut dropped, &second_registration, SupervisorPhase::Running);
    drop(dropped);
}

#[test]
fn invalid_package_fails_before_process_creation() {
    let path = std::env::temp_dir().join(format!("chief-process-invalid-{}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    let registration = HostRegistration::new(
        HostName::new("invalid-host").unwrap(),
        PackagePath::new(path.to_str().unwrap()).unwrap(),
        [0; 32],
        RestartPolicy::Never,
    );
    let keyring = Arc::new(keyring());
    let identity = Arc::new(generate_identity_keypair());
    let mut supervisor = new_supervisor(
        keyring,
        identity,
        Duration::from_secs(1),
        Duration::from_secs(1),
    );
    assert_eq!(
        supervisor.start(&registration),
        Err(ProcessSupervisorError::PackageVerification)
    );
}

#[test]
fn unavailable_launch_bindings_fail_before_process_creation() {
    let package = TestPackage::new("missing-bindings", None);
    let registration = package.registration("unbound-host");
    let mut supervisor = new_supervisor_with_bindings(
        Arc::new(keyring()),
        Arc::new(DenyHostLaunchBindings),
        Arc::new(generate_identity_keypair()),
        Duration::from_secs(1),
        Duration::from_secs(1),
    );
    assert_eq!(
        supervisor.start(&registration),
        Err(ProcessSupervisorError::LaunchBindings)
    );
    assert_eq!(
        supervisor.inspect(&registration),
        Ok(SupervisorObservation::Absent)
    );
}
