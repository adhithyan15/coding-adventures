//! The macOS applier end to end (D18S step 7, P2.7a): the probe agent run
//! through the real shim under a real Seatbelt profile, on `macos-latest`.
//!
//! Each denial is shown against a control: the same probe, unconfined,
//! succeeds. Without the control a test could pass because the operation
//! fails anyway.

#![cfg(target_os = "macos")]

use capability_os_sandbox::{plan_from_json, OsFamily, SandboxPlan};
use chief_of_staff_macos_sandbox::{
    ConfinementError, MacosConfinement, ShimInstructions, INSTRUCTIONS_VARIABLE,
};
use chief_of_staff_spawn_isolation::VerifiedExecutable;
use std::fs;
use std::os::fd::OwnedFd;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

const PROBE: &str = env!("CARGO_BIN_EXE_macos-sandbox-probe");
const SHIM: &str = env!("CARGO_BIN_EXE_chief-of-staff-seatbelt-shim");
const EPERM: &str = "errno=1";

fn digest(path: &Path) -> [u8; 32] {
    coding_adventures_sha256::sha256(&fs::read(path).unwrap())
}

fn verified(path: &Path) -> VerifiedExecutable {
    let path = fs::canonicalize(path).unwrap();
    VerifiedExecutable::open(&path, digest(&path)).unwrap()
}

/// A private directory under the target's scratch space, removed on drop.
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let base = fs::canonicalize(env!("CARGO_TARGET_TMPDIR")).unwrap();
        let path = base.join(format!(
            "macos-sandbox-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        Self(path)
    }

    fn file(&self, name: &str, contents: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, contents).unwrap();
        path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// A plan for a manifest with these `(action, target)` filesystem grants.
fn plan(grants: &[(&str, &Path)]) -> SandboxPlan {
    let capabilities: Vec<String> = grants
        .iter()
        .map(|(action, target)| {
            format!(
                r#"{{"category":"fs","action":"{action}","target":"{}","justification":"test grant"}}"#,
                target.display()
            )
        })
        .collect();
    let manifest = format!(
        r#"{{"version":1,"package":"rust/macos-sandbox-probe","capabilities":[{}],"justification":"Probe for the macOS applier."}}"#,
        capabilities.join(",")
    );
    plan_from_json(&manifest, OsFamily::Macos).expect("plan builds")
}

fn confinement(grants: &[(&str, &Path)]) -> MacosConfinement {
    MacosConfinement::prepare_verified(
        &plan(grants),
        &verified(Path::new(PROBE)),
        &verified(Path::new(SHIM)),
    )
    .expect("confinement prepares")
}

/// Run the probe with `args`, confined. stderr is piped after `apply` (which
/// sends it to /dev/null) so a refusal by the shim explains itself.
fn run(confinement: &MacosConfinement, args: &[&str]) -> Output {
    let mut command = Command::new(PROBE);
    command.args(args).stdin(Stdio::null());
    confinement.apply(&mut command).expect("apply");
    command.stderr(Stdio::piped());
    command.output().expect("the shim spawns")
}

fn unconfined(args: &[&str]) -> Output {
    Command::new(PROBE)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .expect("the probe spawns")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// The probe's answer, or a failure that shows what the shim said.
fn answer(output: &Output) -> String {
    assert!(
        output.status.success(),
        "status {:?}, stderr: {}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr),
        diagnose()
    );
    stdout(output)
}

/// What macOS recorded about a confined launch that failed: Seatbelt's
/// denials from the unified log, and the newest crash report for the probe
/// or the shim. A process killed early (dyld, say) often says nothing on
/// stderr; these say what it was refused. Only ever run on a failure.
fn diagnose() -> String {
    // Give the log a moment to receive the kernel's reports.
    std::thread::sleep(std::time::Duration::from_secs(2));
    let mut report = String::from("--- Seatbelt denials (last 5 minutes) ---\n");
    let denials = Command::new("/usr/bin/log")
        .args([
            "show",
            "--style",
            "compact",
            "--last",
            "5m",
            "--predicate",
            r#"eventMessage CONTAINS[c] "deny" AND (eventMessage CONTAINS "macos-sandbox-probe" OR eventMessage CONTAINS "seatbelt-shim")"#,
        ])
        .output();
    match denials {
        Ok(output) => {
            let text = String::from_utf8_lossy(&output.stdout);
            let lines: Vec<&str> = text.lines().collect();
            let start = lines.len().saturating_sub(40);
            for line in &lines[start..] {
                report.push_str(line);
                report.push('\n');
            }
        }
        Err(error) => report.push_str(&format!("log show failed: {error}\n")),
    }
    report.push_str("--- newest crash report ---\n");
    let reports = std::env::var_os("HOME")
        .map(|home| PathBuf::from(home).join("Library/Logs/DiagnosticReports"));
    let newest = reports
        .and_then(|directory| fs::read_dir(directory).ok())
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            name.contains("macos-sandbox-probe") || name.contains("seatbelt-shim")
        })
        .max_by_key(|entry| entry.metadata().and_then(|m| m.modified()).ok());
    match newest {
        Some(entry) => {
            let text = fs::read_to_string(entry.path()).unwrap_or_default();
            report.push_str(&format!("{}\n", entry.path().display()));
            report.push_str(&text.chars().take(6000).collect::<String>());
        }
        None => report.push_str("none found\n"),
    }
    report
}

#[test]
fn a_confined_agent_runs_as_itself_with_its_arguments() {
    let confined = confinement(&[]);
    assert!(confined
        .profile()
        .starts_with("(version 1)\n(deny default)\n"));
    assert_eq!(answer(&run(&confined, &["hello"])), "ok");
    let argv = answer(&run(&confined, &["argv", "one", "two words"]));
    let probe = fs::canonicalize(PROBE).unwrap();
    assert_eq!(
        argv.lines().collect::<Vec<_>>(),
        [probe.to_str().unwrap(), "argv", "one", "two words"]
    );
}

#[test]
fn ungranted_files_cannot_be_read_or_written() {
    let scratch = Scratch::new();
    let secret = scratch.file("secret", "s");
    let secret = secret.to_str().unwrap();
    assert_eq!(stdout(&unconfined(&["read", secret])), "ok");
    assert_eq!(stdout(&unconfined(&["write", secret])), "ok");
    let confined = confinement(&[]);
    assert_eq!(answer(&run(&confined, &["read", secret])), EPERM);
    assert_eq!(answer(&run(&confined, &["write", secret])), EPERM);
    // Nor anything of the user's or the system's beyond the libraries.
    assert_eq!(answer(&run(&confined, &["read", "/etc/hosts"])), EPERM);
}

#[test]
fn a_direct_grant_opens_exactly_its_file_for_exactly_its_action() {
    let scratch = Scratch::new();
    let granted = scratch.file("granted", "g");
    let neighbour = scratch.file("neighbour", "n");
    let (granted_str, neighbour_str) = (granted.to_str().unwrap(), neighbour.to_str().unwrap());

    let reader = confinement(&[("read", &granted)]);
    assert_eq!(answer(&run(&reader, &["read", granted_str])), "ok");
    assert_eq!(answer(&run(&reader, &["read", neighbour_str])), EPERM);
    assert_eq!(answer(&run(&reader, &["write", granted_str])), EPERM);

    let writer = confinement(&[("write", &granted)]);
    assert_eq!(answer(&run(&writer, &["write", granted_str])), "ok");
    assert_eq!(answer(&run(&writer, &["read", granted_str])), EPERM);
    assert_eq!(answer(&run(&writer, &["write", neighbour_str])), EPERM);
}

#[test]
fn the_network_new_processes_and_other_processes_are_out_of_reach() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port().to_string();
    let me = std::process::id().to_string();
    let confined = confinement(&[]);

    assert_eq!(stdout(&unconfined(&["connect", &port])), "ok");
    assert_eq!(answer(&run(&confined, &["connect", &port])), EPERM);

    assert_eq!(stdout(&unconfined(&["fork"])), "ok");
    assert_eq!(answer(&run(&confined, &["fork"])), EPERM);

    // Exec: only its own image is allowed, so another program is refused.
    let exec = run(&confined, &["exec", "/bin/echo"]);
    assert_eq!(answer(&exec), EPERM);

    assert_eq!(stdout(&unconfined(&["signal", &me])), "ok");
    assert_eq!(answer(&run(&confined, &["signal", &me])), EPERM);

    assert_eq!(stdout(&unconfined(&["pidpath", &me])), "ok");
    assert_ne!(answer(&run(&confined, &["pidpath", &me])), "ok");
}

#[test]
fn only_the_standard_descriptors_and_granted_variables_reach_the_agent() {
    let confined = confinement(&[]);
    assert_eq!(answer(&run(&confined, &["descriptors"])), "");
    assert_eq!(answer(&run(&confined, &["core"])), "0 0");

    let mut command = Command::new(PROBE);
    command
        .args(["env"])
        .stdin(Stdio::null())
        .env("TZ", "UTC")
        .env("LANG", "C");
    confined.apply(&mut command).unwrap();
    let mut names: Vec<String> = answer(&command.output().unwrap())
        .lines()
        .map(str::to_string)
        .collect();
    names.sort();
    // Not the instructions, and nothing of the test's own environment.
    assert_eq!(names, ["LANG", "TZ"]);
}

#[test]
fn inherited_descriptors_arrive_at_3_and_up_and_nothing_else_does() {
    let scratch = Scratch::new();
    let first = scratch.file("first", "first key");
    let second = scratch.file("second", "second key");
    let confined = confinement(&[]);
    let open = || -> Vec<OwnedFd> {
        vec![
            fs::File::open(&first).unwrap().into(),
            fs::File::open(&second).unwrap().into(),
        ]
    };
    let spawn = |args: &[&str]| {
        let mut command = Command::new(PROBE);
        command.args(args).stdin(Stdio::null());
        confined.apply_inheriting(&mut command, open()).unwrap();
        command.stderr(Stdio::piped());
        command.output().unwrap()
    };
    assert_eq!(answer(&spawn(&["descriptors"])), "3\n4");
    assert_eq!(answer(&spawn(&["inherited", "3"])), "first key");
    assert_eq!(answer(&spawn(&["inherited", "4"])), "second key");
    // Granted by descriptor, not by path: the file itself stays closed.
    assert_eq!(answer(&spawn(&["read", first.to_str().unwrap()])), EPERM);
}

#[test]
fn what_cannot_be_confined_is_refused_before_anything_runs() {
    let scratch = Scratch::new();
    let file = scratch.file("file", "f");
    let link = scratch.0.join("link");
    std::os::unix::fs::symlink(&file, &link).unwrap();
    let probe = verified(Path::new(PROBE));
    let shim = verified(Path::new(SHIM));
    let prepare = |plan: &SandboxPlan| MacosConfinement::prepare_verified(plan, &probe, &shim);

    assert!(matches!(
        prepare(&plan(&[("read", &link)])),
        Err(ConfinementError::Grant(_))
    ));
    assert!(matches!(
        prepare(&plan(&[("read", &scratch.0)])),
        Err(ConfinementError::Grant(_))
    ));
    let probe_path = fs::canonicalize(PROBE).unwrap();
    assert!(matches!(
        prepare(&plan(&[("write", &probe_path)])),
        Err(ConfinementError::Grant(_))
    ));
    let linux = plan_from_json(
        r#"{"version":1,"package":"rust/p","capabilities":[],"justification":"t"}"#,
        OsFamily::Linux,
    )
    .unwrap();
    assert!(matches!(
        prepare(&linux),
        Err(ConfinementError::NotMacosPlan)
    ));

    // A refused command is poisoned: spawning it anyway fails, and nothing
    // runs, confined or not.
    let confined = confinement(&[]);
    let mut injected = Command::new(PROBE);
    injected
        .arg("hello")
        .env("DYLD_INSERT_LIBRARIES", "/tmp/x.dylib");
    assert!(matches!(
        confined.apply(&mut injected),
        Err(ConfinementError::Environment(_))
    ));
    assert!(injected.output().is_err());
    let many: Vec<OwnedFd> = (0..65)
        .map(|_| fs::File::open(&file).unwrap().into())
        .collect();
    let mut command = Command::new(PROBE);
    command.arg("hello");
    assert!(matches!(
        confined.apply_inheriting(&mut command, many),
        Err(ConfinementError::Inherited(_))
    ));
    assert!(command.output().is_err());
}

#[test]
fn an_agent_changed_after_verification_is_never_run() {
    let scratch = Scratch::new();
    let agent = scratch.0.join("agent");
    fs::copy(PROBE, &agent).unwrap();
    let program = verified(&agent);

    // Rewritten in place: the digest no longer matches.
    let shim = verified(Path::new(SHIM));
    fs::write(&agent, b"not the probe").unwrap();
    assert!(matches!(
        MacosConfinement::prepare_verified(&plan(&[]), &program, &shim),
        Err(ConfinementError::Executable(_))
    ));

    // Replaced after the confinement was prepared: another file at the
    // path. The shim sees the inode changed and refuses to exec it.
    fs::remove_file(&agent).unwrap();
    fs::copy(PROBE, &agent).unwrap();
    let program = verified(&agent);
    let confined = MacosConfinement::prepare_verified(&plan(&[]), &program, &shim).unwrap();
    fs::rename(&agent, scratch.0.join("old")).unwrap();
    fs::copy(PROBE, &agent).unwrap();
    let output = run(&confined, &["hello"]);
    assert_eq!(output.status.code(), Some(127));
    assert_eq!(stdout(&output), "");
    assert!(String::from_utf8_lossy(&output.stderr).contains("no longer names"));
}

/// Run the shim directly with these instructions (or none).
fn shim_with(instructions: Option<String>) -> Output {
    let mut command = Command::new(fs::canonicalize(SHIM).unwrap());
    command.arg("hello").stdin(Stdio::null()).env_clear();
    if let Some(instructions) = instructions {
        command.env(INSTRUCTIONS_VARIABLE, instructions);
    }
    command.output().unwrap()
}

#[test]
fn the_shim_refuses_without_instructions_or_with_a_profile_that_does_not_confine() {
    assert_eq!(shim_with(None).status.code(), Some(127));
    assert_eq!(shim_with(Some("garbage".into())).status.code(), Some(127));

    // A profile that allows everything installs fine, but the launch probes
    // (S-P4) see that it does not confine, and the agent never runs.
    let probe = fs::canonicalize(PROBE).unwrap();
    let metadata = fs::metadata(&probe).unwrap();
    let permissive = ShimInstructions {
        profile: "(version 1)\n(allow default)\n".into(),
        agent: probe.to_str().unwrap().into(),
        agent_dev: metadata.dev(),
        agent_ino: metadata.ino(),
        shim: fs::canonicalize(SHIM).unwrap().to_str().unwrap().into(),
        inherited: Vec::new(),
        grants: Vec::new(),
    };
    let output = shim_with(Some(permissive.encode()));
    assert_eq!(output.status.code(), Some(127));
    assert_eq!(stdout(&output), "");
    assert!(String::from_utf8_lossy(&output.stderr).contains("probe"));

    // A profile Seatbelt cannot compile is refused too.
    let broken = ShimInstructions {
        profile: "(version 1)\n(this is not sbpl".into(),
        ..permissive
    };
    let output = shim_with(Some(broken.encode()));
    assert_eq!(output.status.code(), Some(127));
    assert!(String::from_utf8_lossy(&output.stderr).contains("sandbox_init"));
}

#[test]
fn other_processes_and_the_bootstrap_namespace_are_out_of_reach() {
    let confined = confinement(&[]);
    let me = std::process::id().to_string();

    // kern.procargs2 would hand over this test's argv and environment, and
    // kern.proc.all a list of every process.
    assert_eq!(stdout(&unconfined(&["procargs", &me])), "ok");
    assert_eq!(answer(&run(&confined, &["procargs", &me])), EPERM);
    assert_eq!(stdout(&unconfined(&["procs"])), "ok");
    assert_eq!(answer(&run(&confined, &["procs"])), EPERM);
    // What libc and the runtime read is still there.
    assert_eq!(answer(&run(&confined, &["sysctl", "hw.ncpu"])), "ok");
    assert_eq!(stdout(&unconfined(&["sysctl", "kern.hostname"])), "ok");
    assert_eq!(answer(&run(&confined, &["sysctl", "kern.hostname"])), EPERM);

    // No Mach service: the control finds one that every process can.
    let service = "com.apple.system.notification_center";
    assert_eq!(stdout(&unconfined(&["mach", service])), "ok");
    assert_ne!(answer(&run(&confined, &["mach", service])), "ok");
}

#[test]
fn a_grant_replaced_after_it_was_checked_is_never_granted() {
    let scratch = Scratch::new();
    let granted = scratch.file("granted", "g");
    let confined = confinement(&[("read", &granted)]);
    assert_eq!(
        answer(&run(&confined, &["read", granted.to_str().unwrap()])),
        "ok"
    );

    fs::rename(&granted, scratch.0.join("old")).unwrap();
    fs::write(&granted, "someone else's").unwrap();
    let output = run(&confined, &["read", granted.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(127));
    assert_eq!(stdout(&output), "");
    assert!(String::from_utf8_lossy(&output.stderr).contains("no longer names"));
}

#[test]
fn an_access_control_entry_that_lets_others_write_is_refused() {
    let scratch = Scratch::new();
    let shared = scratch.0.join("shared");
    fs::create_dir(&shared).unwrap();
    let granted = shared.join("granted");
    fs::write(&granted, "g").unwrap();
    assert!(confinement_for(&[("read", &granted)]).is_ok());

    // 0700 by its mode, but an ACL lets everyone add and delete entries.
    let status = Command::new("/bin/chmod")
        .args(["+a", "everyone allow add_file,delete_child"])
        .arg(&shared)
        .status()
        .unwrap();
    assert!(status.success());
    assert!(matches!(
        confinement_for(&[("read", &granted)]),
        Err(ConfinementError::Grant(why)) if why.contains("ACL")
    ));

    // An inheritable entry, as shared folders are set up, carries flags
    // after "allow"; it is refused just the same.
    let inheriting = scratch.0.join("inheriting");
    fs::create_dir(&inheriting).unwrap();
    let inside = inheriting.join("granted");
    fs::write(&inside, "g").unwrap();
    let status = Command::new("/bin/chmod")
        .args([
            "+a",
            "everyone allow add_file,delete_child,file_inherit,directory_inherit",
        ])
        .arg(&inheriting)
        .status()
        .unwrap();
    assert!(status.success());
    assert!(matches!(
        confinement_for(&[("read", &inside)]),
        Err(ConfinementError::Grant(why)) if why.contains("ACL")
    ));
}

fn confinement_for(grants: &[(&str, &Path)]) -> Result<MacosConfinement, ConfinementError> {
    MacosConfinement::prepare_verified(
        &plan(grants),
        &verified(Path::new(PROBE)),
        &verified(Path::new(SHIM)),
    )
}
