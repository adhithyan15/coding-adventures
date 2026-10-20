//! The Windows applier on a real Windows (D18S step 8, P2.8a): the probe
//! agent, run in its AppContainer through `WindowsConfinement`, and the same
//! probe run unconfined as a control. Each denial is checked against the
//! control, so none passes only because the action fails everywhere.
//!
//! P2.8c adds `Direct` read grants: a granted file is readable while its
//! neighbour is not, the grant is gone once the agent has exited, and each
//! grant the applier must not make is refused before anything changes.

#![cfg(windows)]

use capability_os_sandbox::{plan_from_json, OsFamily, SandboxPlan};
use chief_of_staff_spawn_isolation::VerifiedExecutable;
use chief_of_staff_windows_sandbox::{
    broker_container_name, container_name, ConfinementError, SpawnRequest, WindowsConfinement,
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
        self.confinement_under(&plan())
    }

    fn confinement_under(&self, plan: &SandboxPlan) -> WindowsConfinement {
        self.prepare(plan)
            .unwrap_or_else(|error| panic!("prepare: {error}"))
    }

    fn prepare(&self, plan: &SandboxPlan) -> Result<WindowsConfinement, ConfinementError> {
        WindowsConfinement::prepare_verified(plan, &verified_probe(), &self.0)
    }

    /// Run the probe confined; its stdout, trimmed, and exit code.
    fn run(&self, arguments: &[&str]) -> (String, u32) {
        self.run_with(SpawnRequest {
            arguments: arguments.iter().map(|s| s.to_string()).collect(),
            ..SpawnRequest::default()
        })
    }

    fn run_with(&self, request: SpawnRequest) -> (String, u32) {
        self.run_under(&plan(), request)
    }

    /// Run the probe confined under `plan`.
    fn run_under(&self, plan: &SandboxPlan, request: SpawnRequest) -> (String, u32) {
        let mut child = self
            .confinement_under(plan)
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

/// The probe, verified against what is there now.
fn verified_probe() -> VerifiedExecutable {
    let path = probe_path();
    let digest = coding_adventures_sha256::sha256(&fs::read(&path).unwrap());
    VerifiedExecutable::open(&path, digest).unwrap()
}

/// A plan granting `action` on each of `targets` (P2.8c).
fn plan_granting(action: &str, targets: &[&str]) -> SandboxPlan {
    let capabilities: Vec<String> = targets
        .iter()
        .map(|target| {
            format!(
                r#"{{"category":"fs","action":"{action}","target":"{}","justification":"input"}}"#,
                target.replace('\\', "\\\\")
            )
        })
        .collect();
    plan_from_json(
        &format!(
            r#"{{"version":1,"package":"rust/windows-sandbox-probe","capabilities":[{}],"justification":"The probe reads what it is granted."}}"#,
            capabilities.join(",")
        ),
        OsFamily::Windows,
    )
    .unwrap()
}

/// `path` as the applier compares it: its final path, without `\\?\`.
fn canonical(path: &std::path::Path) -> String {
    let canonical = fs::canonicalize(path).unwrap();
    let text = canonical.to_str().unwrap().to_string();
    text.strip_prefix(r"\\?\").unwrap_or(&text).to_string()
}

/// The probe unconfined: the control.
fn control(arguments: &[&str]) -> String {
    // Through the spawn lock, as the supervisor's std spawns are, so a
    // control never inherits another test's agent handles mid-creation.
    let child = chief_of_staff_spawn_isolation::spawn(
        std::process::Command::new(PROBE)
            .args(arguments)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null()),
    )
    .unwrap();
    let output = child.wait_with_output().unwrap();
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
    // What was given, SystemRoot, and LOCALAPPDATA, which CreateProcessW
    // requires of an AppContainer environment. Windows may also point TEMP
    // and TMP into the container's folder. Each of those three names a
    // place inside the agent's own (removed) storage, never the user's.
    let own = PathBuf::from(std::env::var("LOCALAPPDATA").unwrap())
        .join("Packages")
        .join(container_name(&agent.0))
        .to_string_lossy()
        .to_ascii_lowercase();
    let mut names = Vec::new();
    for line in output.lines() {
        let (name, value) = line.split_once('=').unwrap();
        match name {
            "TZ" => assert_eq!(value, "UTC"),
            "SystemRoot" => {}
            "LOCALAPPDATA" | "TEMP" | "TMP" => assert!(
                format!("{}\\", value.to_ascii_lowercase()).starts_with(&format!("{own}\\")),
                "{name}={value} is outside {own}"
            ),
            _ => panic!("{name} was not given: {output}"),
        }
        names.push(name);
    }
    for required in ["LOCALAPPDATA", "SystemRoot", "TZ"] {
        assert!(names.contains(&required), "{required}: {output}");
    }

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
    let (output, code) = agent.run(&["connect", &address]);
    assert!(denied(&output), "{output:?} (exit {code:#x})");

    let shell = std::env::var("ComSpec").unwrap_or_else(|_| r"C:\Windows\System32\cmd.exe".into());
    assert_eq!(control(&["spawn", &shell]), "ok");
    let (output, code) = agent.run(&["spawn", &shell]);
    assert!(denied(&output), "{output:?} (exit {code:#x})");

    let me = std::process::id().to_string();
    assert_eq!(control(&["openprocess", &me]), "ok");
    let (output, code) = agent.run(&["openprocess", &me]);
    assert!(denied(&output), "{output:?} (exit {code:#x})");

    assert_eq!(control(&["registry"]), "ok");
    let (output, code) = agent.run(&["registry"]);
    assert!(denied(&output), "{output:?} (exit {code:#x})");
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
    // A Direct write cannot be expressed (no-write-up, P2.8c).
    assert!(matches!(
        WindowsConfinement::prepare_verified(
            &plan_granting("write", &[r"C:\data\in.txt"]),
            &probe,
            "refused"
        ),
        Err(ConfinementError::Unsupported(_))
    ));
}

#[test]
fn a_granted_file_is_readable_and_its_neighbour_is_not() {
    let agent = Agent::new();
    let scratch = Scratch::new();
    fs::write(scratch.0.join("granted.txt"), b"for the agent").unwrap();
    fs::write(scratch.0.join("neighbour.txt"), b"not for the agent").unwrap();
    let granted = canonical(&scratch.0.join("granted.txt"));
    let neighbour = canonical(&scratch.0.join("neighbour.txt"));
    assert_eq!(control(&["read", &granted]), "ok");
    assert_eq!(control(&["read", &neighbour]), "ok");

    let with_grant = plan_granting("read", &[&granted]);
    let read = |plan: &SandboxPlan, path: &str| {
        agent
            .run_under(
                plan,
                SpawnRequest {
                    arguments: vec!["read".to_string(), path.to_string()],
                    ..SpawnRequest::default()
                },
            )
            .0
    };
    assert_eq!(read(&with_grant, &granted), "ok");
    assert_eq!(read(&with_grant, &neighbour), "error=5");
    // The agent has exited, so its entry is gone: the same agent, the same
    // SID, without the grant, cannot read the file.
    assert_eq!(read(&plan(), &granted), "error=5");
    // And the file is still the user's to read.
    assert_eq!(control(&["read", &granted]), "ok");
}

/// Whether `path`'s access list is protected: inherits nothing from its
/// directory.
fn protected(path: &str) -> bool {
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Authorization::{GetSecurityInfo, SE_FILE_OBJECT};
    use windows_sys::Win32::Security::{
        GetSecurityDescriptorControl, DACL_SECURITY_INFORMATION, SE_DACL_PROTECTED,
    };
    let file = fs::File::open(path).unwrap();
    let mut descriptor: *mut core::ffi::c_void = std::ptr::null_mut();
    let mut control: u16 = 0;
    let mut revision: u32 = 0;
    // SAFETY: an open handle with read access (READ_CONTROL); the
    // descriptor is freed once, below.
    unsafe {
        assert_eq!(
            GetSecurityInfo(
                file.as_raw_handle() as _,
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut descriptor,
            ),
            0
        );
        assert_ne!(
            GetSecurityDescriptorControl(descriptor, &mut control, &mut revision),
            0
        );
        LocalFree(descriptor as _);
    }
    control & SE_DACL_PROTECTED != 0
}

#[test]
fn a_protected_list_stays_protected_through_a_grant() {
    let agent = Agent::new();
    let scratch = Scratch::new();
    fs::write(scratch.0.join("owner-only.txt"), b"for the agent").unwrap();
    let granted = canonical(&scratch.0.join("owner-only.txt"));
    // Protect it: inherited entries become explicit, nothing new inherits.
    let protected_now = chief_of_staff_spawn_isolation::spawn(
        std::process::Command::new("icacls")
            .args([granted.as_str(), "/inheritance:d"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null()),
    )
    .unwrap()
    .wait()
    .unwrap();
    assert!(protected_now.success(), "icacls /inheritance:d");
    assert!(protected(&granted));
    let (read, _) = agent.run_under(
        &plan_granting("read", &[&granted]),
        SpawnRequest {
            arguments: vec!["read".to_string(), granted.clone()],
            ..SpawnRequest::default()
        },
    );
    assert_eq!(read, "ok");
    // Granted and taken back, the list is still protected.
    assert!(protected(&granted), "the grant unprotected the list");
}

#[test]
fn a_grant_the_applier_must_not_make_is_refused() {
    let agent = Agent::new();
    let scratch = Scratch::new();
    let file = scratch.0.join("data.txt");
    fs::write(&file, b"data").unwrap();
    let file = canonical(&file);
    fs::create_dir(scratch.0.join("folder")).unwrap();
    let folder = canonical(&scratch.0.join("folder"));
    let refused = |target: &str, why: &str| {
        assert!(
            matches!(
                agent.prepare(&plan_granting("read", &[target])),
                Err(ConfinementError::Grant(_))
            ),
            "{why}: {target}"
        );
    };

    refused(&format!("{folder}\\absent.txt"), "a missing file");
    refused(&folder, "a directory");
    // A junction: a reparse point, opened as itself and refused.
    let junction = format!("{}\\junction", canonical(&scratch.0));
    let made = chief_of_staff_spawn_isolation::spawn(
        std::process::Command::new("cmd")
            .args(["/c", "mklink", "/J", &junction, &folder])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null()),
    )
    .unwrap()
    .wait()
    .unwrap();
    assert!(made.success(), "mklink /J");
    refused(&junction, "a junction");
    // The same file, spelled in another case: not its final path.
    refused(&file.to_uppercase(), "a non-canonical spelling");
    // The agent's own image, and a hard link to it.
    refused(probe_path().to_str().unwrap(), "the agent's image");
    let link = scratch.0.join("link.exe");
    fs::hard_link(probe_path(), &link).unwrap();
    refused(&canonical(&link), "a hard link to the agent's image");
    // A file Everyone may write.
    let open = scratch.0.join("open.txt");
    fs::write(&open, b"anyone's").unwrap();
    let open = canonical(&open);
    let granted = chief_of_staff_spawn_isolation::spawn(
        std::process::Command::new("icacls")
            .args([open.as_str(), "/grant", "*S-1-1-0:(W)"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null()),
    )
    .unwrap()
    .wait()
    .unwrap();
    assert!(granted.success(), "icacls");
    refused(&open, "a file Everyone may write");
    // The plain file itself passes, so the refusals above are each for
    // their own reason.
    assert!(agent.prepare(&plan_granting("read", &[&file])).is_ok());
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

#[test]
fn a_broker_and_its_agent_are_different_principals() {
    let agent = Agent::new();
    let path = probe_path();
    let digest = coding_adventures_sha256::sha256(&fs::read(&path).unwrap());
    let probe = VerifiedExecutable::open(&path, digest).unwrap();
    // The agent runs ...
    let mut sleeping = agent
        .confinement()
        .spawn(SpawnRequest {
            arguments: vec!["sleep".to_string()],
            ..SpawnRequest::default()
        })
        .unwrap();
    assert_eq!(sleeping.try_wait().unwrap(), None, "the agent is running");
    // ... and its broker starts beside it: another container, so the
    // one-instance rule does not see the agent, and so another SID.
    let broker = WindowsConfinement::prepare_verified_broker(&plan(), &probe, agent.0.as_bytes())
        .unwrap_or_else(|error| panic!("prepare the broker: {error}"));
    let mut child = broker
        .spawn(SpawnRequest {
            arguments: vec!["env".to_string()],
            ..SpawnRequest::default()
        })
        .unwrap_or_else(|error| panic!("spawn the broker: {error}"));
    let mut output = String::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut output)
        .unwrap();
    child.wait().unwrap();
    let local = output
        .lines()
        .find_map(|line| line.strip_prefix("LOCALAPPDATA="))
        .unwrap_or_else(|| panic!("no LOCALAPPDATA: {output}"))
        .to_ascii_lowercase();
    assert!(local.contains(&broker_container_name(&agent.0)), "{local}");
    assert!(
        !local.contains(&format!("\\{}\\", container_name(&agent.0))),
        "{local}"
    );
    sleeping.kill().unwrap();
}
