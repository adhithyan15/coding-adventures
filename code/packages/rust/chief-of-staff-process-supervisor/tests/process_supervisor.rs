use chief_of_staff_host_runtime::{
    DenoLaunchPlan, PackageKeyType, PackageKeyring, TrustedPackageKey,
};
use chief_of_staff_process_supervisor::{
    HostProgram, MonotonicClock, ProcessHostSupervisor, ProcessSupervisorConfig,
    ProcessSupervisorError, SessionIdSource,
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
use std::sync::Arc;
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
    let mut files = vec![
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
    ];
    for marker in [
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

fn new_supervisor<'a>(
    keyring: &'a PackageKeyring,
    identity: &'a coding_adventures_x3dh::IdentityKeyPair,
    bootstrap_timeout: Duration,
    graceful_timeout: Duration,
) -> ProcessHostSupervisor<'a> {
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
        identity,
        Arc::new(TestClock::default()),
        Box::new(TestSessions(0)),
    )
}

fn await_phase(
    supervisor: &mut ProcessHostSupervisor<'_>,
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
    let keyring = keyring();
    let identity = generate_identity_keypair();
    let mut supervisor = new_supervisor(
        &keyring,
        &identity,
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
fn exact_hash_is_checked_before_spawn_and_active_hash_cannot_change() {
    let package = TestPackage::new("identity", None);
    let mut registration = package.registration("identity-host");
    let keyring = keyring();
    let identity = generate_identity_keypair();
    let mut supervisor = new_supervisor(
        &keyring,
        &identity,
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
        let keyring = keyring();
        let identity = generate_identity_keypair();
        let mut supervisor =
            new_supervisor(&keyring, &identity, timeout, Duration::from_millis(100));
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
        let keyring = keyring();
        let identity = generate_identity_keypair();
        let mut supervisor = new_supervisor(
            &keyring,
            &identity,
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
    let keyring = keyring();
    let identity = generate_identity_keypair();
    let mut supervisor = new_supervisor(
        &keyring,
        &identity,
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
        &keyring,
        &identity,
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
    let keyring = keyring();
    let identity = generate_identity_keypair();
    let mut supervisor = new_supervisor(
        &keyring,
        &identity,
        Duration::from_secs(1),
        Duration::from_secs(1),
    );
    assert_eq!(
        supervisor.start(&registration),
        Err(ProcessSupervisorError::PackageVerification)
    );
}
