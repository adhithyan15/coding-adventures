//! The Windows applier on a real Windows (D18S step 8, P2.8a): the probe
//! agent, run in its AppContainer through `WindowsConfinement`, and the same
//! probe run unconfined as a control. Each denial is checked against the
//! control, so none passes only because the action fails everywhere.

#![cfg(windows)]

use capability_os_sandbox::{plan_from_json, OsFamily, SandboxPlan};
use chief_of_staff_spawn_isolation::VerifiedExecutable;
use chief_of_staff_windows_sandbox::{
    container_name, ConfinementError, SpawnRequest, WindowsConfinement,
};
use std::fs;
use std::io::Read;
use std::os::windows::io::{AsRawHandle, OwnedHandle};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

const PROBE: &str = env!("CARGO_BIN_EXE_windows-sandbox-probe");

/// The probe, copied once into a directory only this user and the system
/// may write (the user's temp directory; the build directory may inherit
/// write access for others, which `VerifiedExecutable` refuses), at its
/// canonical path: exactly its final path, which the applier requires.
fn probe_path() -> PathBuf {
    static COPY: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    COPY.get_or_init(|| {
        let directory =
            std::env::temp_dir().join(format!("windows-sandbox-probe-{}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        let copy = directory.join("windows-sandbox-probe.exe");
        fs::copy(PROBE, &copy).unwrap();
        let canonical = fs::canonicalize(&copy).unwrap();
        let text = canonical.to_str().unwrap().to_string();
        PathBuf::from(text.strip_prefix(r"\\?\").unwrap_or(&text))
    })
    .clone()
}

fn plan() -> SandboxPlan {
    plan_from_json(
        r#"{"version":1,"package":"rust/windows-sandbox-probe","capabilities":[],"justification":"A probe with no capabilities."}"#,
        OsFamily::Windows,
    )
    .unwrap()
}

/// A distinct agent per test.
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
    // The profile the creation needed was removed before the agent ran,
    // so it has no storage of its own to keep state in across runs.
    let local = std::env::var("LOCALAPPDATA").unwrap();
    let storage = PathBuf::from(local)
        .join("Packages")
        .join(container_name(&agent.0));
    assert!(!storage.exists(), "{}", storage.display());
    // Nor is the profile registered: creating it again is a fresh creation,
    // not ERROR_ALREADY_EXISTS. (Removed again at once.)
    use windows_sys::Win32::Security::FreeSid;
    use windows_sys::Win32::Security::Isolation::{
        CreateAppContainerProfile, DeleteAppContainerProfile,
    };
    let name: Vec<u16> = container_name(&agent.0)
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let mut sid = std::ptr::null_mut();
    // SAFETY: NUL-terminated strings, no capabilities, a SID out that is
    // freed once; the profile is deleted by name below.
    let created = unsafe {
        let created = CreateAppContainerProfile(
            name.as_ptr(),
            name.as_ptr(),
            name.as_ptr(),
            std::ptr::null(),
            0,
            &mut sid,
        );
        if !sid.is_null() {
            FreeSid(sid);
        }
        DeleteAppContainerProfile(name.as_ptr());
        created
    };
    assert_eq!(created, 0, "{created:#x}");
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

/// A denial: the probe ran and reported an error. A probe that never ran
/// prints nothing, which is not a denial and fails this.
fn denied(output: &str) -> bool {
    output.starts_with("error=")
}

#[test]
fn the_network_processes_and_the_registry_are_out_of_reach() {
    let agent = Agent::new();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap().to_string();
    assert_eq!(control(&["connect", &address]), "ok");
    let output = agent.run(&["connect", &address]).0;
    assert!(denied(&output), "{output}");

    let shell = std::env::var("ComSpec").unwrap_or_else(|_| r"C:\Windows\System32\cmd.exe".into());
    assert_eq!(control(&["spawn", &shell]), "ok");
    let output = agent.run(&["spawn", &shell]).0;
    assert!(denied(&output), "{output}");

    let me = std::process::id().to_string();
    assert_eq!(control(&["openprocess", &me]), "ok");
    let output = agent.run(&["openprocess", &me]).0;
    assert!(denied(&output), "{output}");

    assert_eq!(control(&["registry"]), "ok");
    let output = agent.run(&["registry"]).0;
    assert!(denied(&output), "{output}");
}

#[test]
fn the_mitigation_policies_are_in_force() {
    let agent = Agent::new();
    // Arbitrary code guard: no new executable memory.
    assert_eq!(control(&["dynamic-code"]), "ok");
    let output = agent.run(&["dynamic-code"]).0;
    assert!(denied(&output), "{output}");
    // No win32k, read back from inside the process.
    assert_eq!(control(&["win32k"]), "enabled");
    assert_eq!(agent.run(&["win32k"]).0, "disabled");
}

#[test]
fn only_listed_handles_are_inherited() {
    use windows_sys::Win32::Foundation::{SetHandleInformation, HANDLE_FLAG_INHERIT};
    let agent = Agent::new();
    // An inheritable pipe end this process holds, not on any list.
    let (mut reader, writer) = std::io::pipe().unwrap();
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
    // The confined agent writes to the handle it was given, which proves it
    // ran, and then to that number. Whatever the number names in the agent
    // (nothing, or one of its own handles), it is not this pipe.
    let (mut listed_reader, listed_writer) = std::io::pipe().unwrap();
    agent.run_with(SpawnRequest {
        arguments: vec!["write-handle".to_string(), value.clone()],
        inherited: vec![listed_writer.into()],
        ..SpawnRequest::default()
    });
    let mut listed = String::new();
    listed_reader.read_to_string(&mut listed).unwrap();
    assert_eq!(listed, "inherited\n", "the agent ran");
    // std's spawn inherits every inheritable handle: the control writes
    // into this pipe.
    assert_eq!(control(&["write-handle", &value]), "ok");
    drop(stray);
    let mut received = String::new();
    reader.read_to_string(&mut received).unwrap();
    assert_eq!(received, "stray\n", "only the control's write arrives");
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
    use windows_sys::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0};
    use windows_sys::Win32::System::Threading::{
        OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE,
    };
    let agent = Agent::new();
    let sleeper = || {
        agent
            .confinement()
            .spawn(SpawnRequest {
                arguments: vec!["sleep".to_string()],
                ..SpawnRequest::default()
            })
            .unwrap()
    };

    let mut child = sleeper();
    assert_eq!(child.try_wait().unwrap(), None, "it sleeps for a minute");
    child.kill().unwrap();
    // TerminateJobObject's exit code, long before the minute is up.
    assert_eq!(child.wait().unwrap().code(), Some(1));
    // Until a child is dropped, its agent may not start again: the two
    // would share a SID (and the profile a spawn registers).
    drop(child);

    let mut child = sleeper();
    assert_eq!(child.try_wait().unwrap(), None, "it is running");
    assert!(matches!(
        agent.confinement().spawn(SpawnRequest::default()),
        Err(ConfinementError::Container(_))
    ));
    let id = child.id();
    // SAFETY: an id and a wait-only right; the handle is closed below.
    let handle = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, id) };
    assert_ne!(handle, 0);
    drop(child);
    // Closing the job killed it.
    // SAFETY: the handle opened above, waited on and closed once.
    unsafe {
        assert_eq!(WaitForSingleObject(handle, 10_000), WAIT_OBJECT_0);
        CloseHandle(handle);
    }
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
