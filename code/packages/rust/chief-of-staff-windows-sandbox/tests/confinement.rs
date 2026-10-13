//! The Windows applier on a real Windows (D18S step 8, P2.8a): the probe
//! agent, run in its AppContainer through `WindowsConfinement`, and the same
//! probe run unconfined as a control. Each denial is checked against the
//! control, so none passes only because the action fails everywhere.

#![cfg(windows)]

use capability_os_sandbox::{plan_from_json, OsFamily, SandboxPlan};
use chief_of_staff_spawn_isolation::VerifiedExecutable;
use chief_of_staff_windows_sandbox::{
    remove_profile, ConfinementError, SpawnRequest, WindowsConfinement,
};
use std::fs;
use std::io::Read;
use std::os::windows::io::{AsRawHandle, OwnedHandle};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

const PROBE: &str = env!("CARGO_BIN_EXE_windows-sandbox-probe");

/// The probe's canonical path: exactly its final path, which the applier
/// requires.
fn probe_path() -> PathBuf {
    let canonical = fs::canonicalize(PROBE).unwrap();
    let text = canonical.to_str().unwrap();
    PathBuf::from(text.strip_prefix(r"\\?\").unwrap_or(text))
}

fn plan() -> SandboxPlan {
    plan_from_json(
        r#"{"version":1,"package":"rust/windows-sandbox-probe","capabilities":[],"justification":"A probe with no capabilities."}"#,
        OsFamily::Windows,
    )
    .unwrap()
}

/// A distinct agent per test, whose profile is removed at the end.
struct Agent(String);

impl Agent {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        Self(format!(
            "windows-sandbox-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn confinement(&self) -> WindowsConfinement {
        let path = probe_path();
        let digest = coding_adventures_sha256::sha256(&fs::read(&path).unwrap());
        let probe = VerifiedExecutable::open(&path, digest).unwrap();
        WindowsConfinement::prepare_verified(&plan(), &probe, &self.0)
            .unwrap_or_else(|error| panic!("prepare: {error}"))
    }

    /// Run the probe confined; its stdout, trimmed, and exit code.
    fn run(&self, arguments: &[&str]) -> (String, u32) {
        self.run_with(SpawnRequest {
            arguments: arguments.iter().map(|s| s.to_string()).collect(),
            ..SpawnRequest::default()
        })
    }

    fn run_with(&self, request: SpawnRequest) -> (String, u32) {
        let mut child = self
            .confinement()
            .spawn(request)
            .unwrap_or_else(|error| panic!("spawn: {error}"));
        let mut output = String::new();
        child
            .stdout
            .take()
            .unwrap()
            .read_to_string(&mut output)
            .unwrap();
        let status = child.wait().unwrap();
        (
            output.trim().to_string(),
            status.code().unwrap_or(-1) as u32,
        )
    }
}

impl Drop for Agent {
    fn drop(&mut self) {
        let _ = remove_profile(&self.0);
    }
}

/// The probe unconfined: the control.
fn control(arguments: &[&str]) -> String {
    let output = std::process::Command::new(PROBE)
        .args(arguments)
        .output()
        .unwrap();
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "windows-sandbox-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn the_probe_runs_in_its_own_less_privileged_appcontainer() {
    let agent = Agent::new();
    assert_eq!(agent.run(&["hello"]), ("hello".to_string(), 0));
    assert_eq!(
        agent.run(&["token"]).0,
        "appcontainer=1 lpac=1 integrity=0x1000"
    );
    assert!(control(&["token"]).starts_with("appcontainer=0"));
}

#[test]
fn arguments_arrive_exactly() {
    let agent = Agent::new();
    let arguments = [
        "argv",
        "plain",
        "two words",
        "",
        r#"say "hi""#,
        r"C:\my dir\",
    ];
    assert_eq!(agent.run(&arguments).0, arguments[1..].join("\n"));
}

#[test]
fn the_environment_is_only_what_was_given() {
    let agent = Agent::new();
    let (output, _) = agent.run_with(SpawnRequest {
        arguments: vec!["env".to_string()],
        environment: vec![("TZ".to_string(), "UTC".to_string())],
        ..SpawnRequest::default()
    });
    let names: Vec<&str> = output
        .lines()
        .map(|line| line.split('=').next().unwrap())
        .collect();
    assert_eq!(names, ["SystemRoot", "TZ"], "{output}");

    let refused = agent.confinement().spawn(SpawnRequest {
        arguments: vec!["env".to_string()],
        environment: vec![("PATH".to_string(), "C:\\Windows".to_string())],
        ..SpawnRequest::default()
    });
    assert!(matches!(refused, Err(ConfinementError::Environment(_))));
}

#[test]
fn files_outside_the_container_are_out_of_reach() {
    let agent = Agent::new();
    let scratch = Scratch::new();
    let secret = scratch.0.join("secret.txt");
    fs::write(&secret, b"the supervisor's").unwrap();
    let secret = secret.to_str().unwrap();
    let created = scratch.0.join("created.txt");
    let created = created.to_str().unwrap();

    assert_eq!(control(&["read", secret]), "ok");
    assert_eq!(agent.run(&["read", secret]).0, "error=5");
    assert_eq!(agent.run(&["write", created]).0, "error=5");
    assert!(!scratch.0.join("created.txt").exists());
    assert_eq!(control(&["write", created]), "ok");
}

#[test]
fn the_network_processes_and_the_registry_are_out_of_reach() {
    let agent = Agent::new();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap().to_string();
    assert_eq!(control(&["connect", &address]), "ok");
    assert_ne!(agent.run(&["connect", &address]).0, "ok");

    let shell = std::env::var("ComSpec").unwrap_or_else(|_| r"C:\Windows\System32\cmd.exe".into());
    assert_eq!(control(&["spawn", &shell]), "ok");
    assert_ne!(agent.run(&["spawn", &shell]).0, "ok");

    let me = std::process::id().to_string();
    assert_eq!(control(&["openprocess", &me]), "ok");
    assert_ne!(agent.run(&["openprocess", &me]).0, "ok");

    assert_eq!(control(&["registry"]), "ok");
    assert_ne!(agent.run(&["registry"]).0, "ok");
}

#[test]
fn the_mitigation_policies_are_in_force() {
    let agent = Agent::new();
    // Arbitrary code guard: no new executable memory.
    assert_eq!(control(&["dynamic-code"]), "ok");
    assert_ne!(agent.run(&["dynamic-code"]).0, "ok");
    // No win32k: the window manager's library cannot even load.
    assert_eq!(control(&["user32"]), "ok");
    assert_ne!(agent.run(&["user32"]).0, "ok");
}

#[test]
fn only_listed_handles_are_inherited() {
    use windows_sys::Win32::Foundation::{SetHandleInformation, HANDLE_FLAG_INHERIT};
    let agent = Agent::new();
    // An inheritable handle this process holds, not on the list.
    let (reader, writer) = std::io::pipe().unwrap();
    let stray: OwnedHandle = writer.into();
    // SAFETY: an open handle this test owns.
    assert_ne!(
        unsafe {
            SetHandleInformation(
                stray.as_raw_handle() as isize,
                HANDLE_FLAG_INHERIT,
                HANDLE_FLAG_INHERIT,
            )
        },
        0
    );
    let value = (stray.as_raw_handle() as usize).to_string();
    // std's spawn inherits every inheritable handle: the control sees it.
    assert_eq!(control(&["handle", &value]), "ok");
    // The applier's child has only its list: the value names nothing, and
    // strict handle checks make using it fatal.
    assert_ne!(agent.run(&["handle", &value]).0, "ok");
    drop((reader, stray));
}

#[test]
fn passed_handles_reach_the_agent() {
    let agent = Agent::new();
    let (mut reader, writer) = std::io::pipe().unwrap();
    let (output, status) = agent.run_with(SpawnRequest {
        arguments: vec!["inherited".to_string()],
        inherited: vec![writer.into()],
        ..SpawnRequest::default()
    });
    assert_eq!((output.as_str(), status), ("wrote 1", 0));
    let mut written = String::new();
    reader.read_to_string(&mut written).unwrap();
    assert_eq!(written, "inherited\n");
}

#[test]
fn a_killed_or_dropped_agent_ends() {
    let agent = Agent::new();
    // `argv` with stdin left open exits at once; use the env mode's quick
    // exit too: what matters is that kill and drop are honoured.
    let mut child = agent
        .confinement()
        .spawn(SpawnRequest {
            arguments: vec!["hello".to_string()],
            ..SpawnRequest::default()
        })
        .unwrap();
    child.kill().unwrap();
    assert!(child.wait().is_ok());
    let child = agent.confinement().spawn(SpawnRequest::default()).unwrap();
    drop(child);
}

#[test]
fn a_plan_this_applier_cannot_enforce_is_refused() {
    let path = probe_path();
    let digest = coding_adventures_sha256::sha256(&fs::read(&path).unwrap());
    let probe = VerifiedExecutable::open(&path, digest).unwrap();
    let linux = plan_from_json(
        r#"{"version":1,"package":"rust/probe","capabilities":[],"justification":"none"}"#,
        OsFamily::Linux,
    )
    .unwrap();
    assert_eq!(
        WindowsConfinement::prepare_verified(&linux, &probe, "refused").err(),
        Some(ConfinementError::NotWindowsPlan)
    );
    let with_grant = plan_from_json(
        r#"{"version":1,"package":"rust/probe","capabilities":[{"category":"fs","action":"read","target":"C:\\data\\in.txt","justification":"input"}],"justification":"reads"}"#,
        OsFamily::Windows,
    )
    .unwrap();
    assert!(matches!(
        WindowsConfinement::prepare_verified(&with_grant, &probe, "refused"),
        Err(ConfinementError::Unsupported(_))
    ));
}

#[test]
fn a_path_that_is_not_canonical_is_refused() {
    let path = probe_path();
    let digest = coding_adventures_sha256::sha256(&fs::read(&path).unwrap());
    // The same file, named with a `.` component.
    let parent = path.parent().unwrap();
    let dotted = parent.join(".").join(path.file_name().unwrap());
    let probe = VerifiedExecutable::open(&dotted, digest).unwrap();
    assert!(matches!(
        WindowsConfinement::prepare_verified(&plan(), &probe, "dotted"),
        Err(ConfinementError::Executable(_))
    ));
}
