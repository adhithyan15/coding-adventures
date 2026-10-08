//! D18S build step 4, checked from inside a real confined child.
//!
//! Each test spawns `linux-sandbox-probe` under a plan, asks it to do one
//! thing, and checks the outcome:
//!
//! ```text
//!   outcome                  meaning
//!   -----------------------  ------------------------------------------
//!   exit 0, "ok"             allowed
//!   exit 0, "errno=13"       Landlock refused the path (EACCES)
//!   killed by SIGSYS         seccomp killed the process (S-P1)
//! ```
//!
//! Every denial has a control: the same probe, unconfined, survives. A test
//! that only ever saw the probe die could not tell the sandbox from a probe
//! that is simply broken.

#![cfg(target_os = "linux")]

use capability_os_sandbox::{plan_from_json, OsFamily, SandboxPlan};
use chief_of_staff_linux_sandbox::{ConfinementError, LinuxConfinement};
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const PROBE: &str = env!("CARGO_BIN_EXE_linux-sandbox-probe");

/// A plan for a manifest with these `(action, target)` filesystem grants.
fn plan(grants: &[(&str, &str)]) -> SandboxPlan {
    let capabilities: Vec<String> = grants
        .iter()
        .map(|(action, target)| {
            format!(
                r#"{{"category":"fs","action":"{action}","target":"{target}","justification":"test grant"}}"#
            )
        })
        .collect();
    let manifest = format!(
        r#"{{"version":1,"package":"rust/linux-sandbox-probe","capabilities":[{}],"justification":"Probe for the Linux applier."}}"#,
        capabilities.join(",")
    );
    plan_from_json(&manifest, OsFamily::Linux).expect("plan builds")
}

fn confinement(grants: &[(&str, &str)]) -> LinuxConfinement {
    LinuxConfinement::prepare(&plan(grants), Path::new(PROBE)).expect("confinement prepares")
}

/// Run the probe with `args`, confined by `confinement` or not at all.
fn run(confinement: Option<&LinuxConfinement>, args: &[&str]) -> Output {
    let mut command = Command::new(PROBE);
    command.args(args).stdin(Stdio::null());
    if let Some(confinement) = confinement {
        confinement.apply(&mut command);
    }
    command.output().expect("the probe spawns")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// A fresh temporary file holding `contents`.
fn temp_file(label: &str, contents: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "linux-sandbox-{label}-{}-{nanos}",
        std::process::id()
    ));
    std::fs::write(&path, contents).unwrap();
    path
}

#[test]
fn a_confined_agent_runs_and_starts_threads() {
    let output = run(Some(&confinement(&[])), &["hello"]);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(stdout(&output), "hello\nthread=4");
}

#[test]
fn the_landlock_abi_is_recent_enough_for_grants() {
    // CI runs a kernel with Landlock. A kernel without it refuses every
    // launch (S-P3), which the other tests would show as a panic at prepare.
    assert!(confinement(&[]).landlock_abi() >= 3);
}

#[test]
fn each_denied_syscall_class_kills_the_agent() {
    let confined = confinement(&[]);
    for mode in [
        "socket", "unix", "fork", "io_uring", "ptrace", "kill", "tiocsti", "mount", "bpf",
    ] {
        let control = run(None, &[mode]);
        assert_eq!(
            stdout(&control),
            "survived",
            "control for {mode}: {control:?}"
        );
        let output = run(Some(&confined), &[mode]);
        assert_eq!(
            output.status.signal(),
            Some(libc::SIGSYS),
            "{mode} must be killed by seccomp: {output:?}"
        );
        assert_eq!(stdout(&output), "", "{mode} printed before dying");
    }
}

#[test]
fn ungranted_paths_do_not_exist_for_the_agent() {
    let confined = confinement(&[]);
    let secret = temp_file("secret", "the vault, say");
    let secret = secret.to_str().unwrap();
    for path in [secret, "/etc/hostname", "/proc/self/status", "/etc/passwd"] {
        if !Path::new(path).exists() {
            continue;
        }
        assert_eq!(
            stdout(&run(None, &["read", path])),
            "ok",
            "control for {path}"
        );
        assert_eq!(
            stdout(&run(Some(&confined), &["read", path])),
            "errno=13",
            "{path} must be unreadable"
        );
    }
    assert_eq!(
        stdout(&run(Some(&confined), &["write", secret])),
        "errno=13"
    );
    std::fs::remove_file(secret).unwrap();
}

#[test]
fn the_agent_cannot_read_symlinks_to_learn_the_supervisors_files() {
    // Landlock does not mediate readlink. Unrefused, it names every file
    // the supervisor holds open, through /proc/<ppid>/fd.
    let fd_link = format!("/proc/{}/fd/0", std::process::id());
    assert_eq!(stdout(&run(None, &["readlink", &fd_link])), "ok", "control");
    let confined = confinement(&[]);
    for link in [fd_link.as_str(), "/proc/self/exe"] {
        assert_eq!(
            stdout(&run(Some(&confined), &["readlink", link])),
            "errno=13",
            "readlink {link} must be refused"
        );
    }
}

#[test]
fn the_agent_cannot_exec_anything() {
    // S-I4d: filters survive exec, so the exec that starts the agent must
    // not stay available to it. `execve` is gone, and `execveat` only
    // works on the supervisor's own descriptor, which closed at the exec.
    let confined = confinement(&[]);
    for (mode, program) in [
        ("exec", "/bin/true"),
        ("exec", PROBE),
        ("execveat", "/bin/true"),
        ("execveat", PROBE),
    ] {
        if !Path::new(program).exists() {
            continue;
        }
        let control = run(None, &[mode, program, "hello"]);
        assert!(
            control.status.success(),
            "control {mode} {program}: {control:?}"
        );
        let output = run(Some(&confined), &[mode, program, "hello"]);
        assert_eq!(
            output.status.signal(),
            Some(libc::SIGSYS),
            "{mode} {program} must be killed: {output:?}"
        );
    }
}

#[test]
fn the_agent_gets_the_commands_arguments_and_exactly_its_environment() {
    // S-I4a: the environment is a closed set. The agent gets exactly the
    // variables set on the command, nothing inherited, with or without
    // `env_clear`; and argv is the command's.
    // The first version passed the child's `environ`, which std had not
    // yet replaced: every inherited variable reached the agent, tokens
    // included. This test caught it.
    let confined = confinement(&[]);
    for clear in [true, false] {
        let mut command = Command::new(PROBE);
        command.args(["env", "AGENT_TOKEN", "second arg"]);
        if clear {
            command.env_clear();
        }
        command.env("AGENT_TOKEN", "granted").stdin(Stdio::null());
        confined.apply(&mut command);
        let output = command.output().unwrap();
        assert_eq!(
            stdout(&output),
            "names=AGENT_TOKEN\nargs=AGENT_TOKEN,second arg\nAGENT_TOKEN=granted\nsecond arg=",
            "env_clear={clear}: {output:?}"
        );
    }
}

#[test]
fn what_runs_is_the_file_prepared_not_whatever_the_path_names_later() {
    // The binary is opened at prepare and exec'd by descriptor, so
    // replacing the path in between changes nothing.
    let agent = temp_file("swapped-agent", "");
    std::fs::copy(PROBE, &agent).unwrap();
    let confined = LinuxConfinement::prepare(&plan(&[]), &agent).unwrap();
    std::fs::remove_file(&agent).unwrap();
    std::fs::write(&agent, b"#!/bin/sh\necho swapped\n").unwrap();
    let mut command = Command::new(&agent);
    command.arg("hello").stdin(Stdio::null());
    confined.apply(&mut command);
    let output = command.output().unwrap();
    assert_eq!(stdout(&output), "hello\nthread=4", "{output:?}");
    std::fs::remove_file(agent).unwrap();
}

#[test]
fn a_read_grant_reads_that_file_and_nothing_else() {
    let granted = temp_file("granted", "allowed");
    let other = temp_file("other", "not allowed");
    let confined = confinement(&[("read", granted.to_str().unwrap())]);
    let read = |path: &Path| stdout(&run(Some(&confined), &["read", path.to_str().unwrap()]));
    assert_eq!(read(&granted), "ok");
    assert_eq!(read(&other), "errno=13");
    // Read, not write.
    assert_eq!(
        stdout(&run(Some(&confined), &["write", granted.to_str().unwrap()])),
        "errno=13"
    );
    assert_eq!(std::fs::read_to_string(&granted).unwrap(), "allowed");
    std::fs::remove_file(granted).unwrap();
    std::fs::remove_file(other).unwrap();
}

#[test]
fn a_write_grant_writes_that_file_and_nothing_else() {
    let granted = temp_file("writable", "before");
    let other = temp_file("untouched", "before");
    let confined = confinement(&[("write", granted.to_str().unwrap())]);
    let write = |path: &Path| stdout(&run(Some(&confined), &["write", path.to_str().unwrap()]));
    assert_eq!(write(&granted), "ok");
    assert_eq!(std::fs::read_to_string(&granted).unwrap(), "written");
    assert_eq!(write(&other), "errno=13");
    assert_eq!(std::fs::read_to_string(&other).unwrap(), "before");
    // Write, not read.
    assert_eq!(
        stdout(&run(Some(&confined), &["read", granted.to_str().unwrap()])),
        "errno=13"
    );
    std::fs::remove_file(granted).unwrap();
    std::fs::remove_file(other).unwrap();
}

#[test]
fn grants_landlock_cannot_express_exactly_refuse_the_launch() {
    let file = temp_file("create", "x");
    let file = file.to_str().unwrap();
    // Create and delete would need rights over the whole parent directory.
    for action in ["create", "delete"] {
        let error = LinuxConfinement::prepare(&plan(&[(action, file)]), Path::new(PROBE))
            .err()
            .unwrap_or_else(|| panic!("fs:{action} must refuse the launch"));
        assert!(
            matches!(error, ConfinementError::InexpressibleGrant(_)),
            "{action}: {error:?}"
        );
    }
    std::fs::remove_file(file).unwrap();
}

#[test]
fn a_grant_must_name_an_existing_regular_file_through_no_symlink() {
    let target = temp_file("link-target", "secret");
    let link = target.with_extension("link");
    std::os::unix::fs::symlink(&target, &link).unwrap();
    let directory = std::env::temp_dir();
    let refused = |path: &Path| {
        LinuxConfinement::prepare(&plan(&[("read", path.to_str().unwrap())]), Path::new(PROBE))
            .err()
    };
    // A directory would grant its whole tree.
    assert!(matches!(
        refused(&directory),
        Some(ConfinementError::InexpressibleGrant(_))
    ));
    // A symlink would grant what it points at.
    assert!(refused(&link).is_some(), "a symlink grant must refuse");
    // A symlink earlier in the path, too.
    let linked_directory = target.with_extension("dirlink");
    std::os::unix::fs::symlink(&directory, &linked_directory).unwrap();
    let through = linked_directory.join(target.file_name().unwrap());
    assert!(through.exists());
    assert!(
        refused(&through).is_some(),
        "a path through a symlink must refuse"
    );
    // A missing file cannot be checked at all.
    assert!(matches!(
        refused(&target.with_extension("missing")),
        Some(ConfinementError::Path(_))
    ));
    // And the plain path is fine: the control.
    assert!(refused(&target).is_none());
    for path in [&link, &linked_directory, &target] {
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn the_agents_own_executable_is_never_writable() {
    // S-I6: the runtime image is never grantable.
    let error = LinuxConfinement::prepare(&plan(&[("write", PROBE)]), Path::new(PROBE))
        .err()
        .expect("a write grant on the agent itself must refuse");
    assert!(
        matches!(error, ConfinementError::InexpressibleGrant(_)),
        "{error:?}"
    );
}

#[test]
fn a_shared_library_is_never_writable() {
    // S-I6: the libraries are the runtime image, the host's included.
    let library = [
        "/usr/lib/x86_64-linux-gnu/libc.so.6",
        "/usr/lib/aarch64-linux-gnu/libc.so.6",
        "/usr/lib64/libc.so.6",
        "/usr/lib/libc.so.6",
    ]
    .into_iter()
    .find(|path| Path::new(path).exists())
    .expect("a libc to test with");
    let error = LinuxConfinement::prepare(&plan(&[("write", library)]), Path::new(PROBE))
        .err()
        .expect("a write grant on libc must refuse");
    assert!(
        matches!(error, ConfinementError::InexpressibleGrant(_)),
        "{error:?}"
    );
    // A read grant there adds nothing: the libraries are readable anyway.
    assert!(LinuxConfinement::prepare(&plan(&[("read", library)]), Path::new(PROBE)).is_ok());
}

#[test]
fn a_plan_for_another_os_is_refused() {
    let manifest = r#"{"version":1,"package":"rust/p","capabilities":[],"justification":"x"}"#;
    let macos = plan_from_json(manifest, OsFamily::Macos).unwrap();
    assert!(matches!(
        LinuxConfinement::prepare(&macos, Path::new(PROBE)),
        Err(ConfinementError::NotLinuxPlan)
    ));
}

#[test]
fn a_plan_that_may_not_launch_is_refused() {
    let mut tampered = plan(&[]);
    tampered.base.primitives.clear();
    assert!(matches!(
        LinuxConfinement::prepare(&tampered, Path::new(PROBE)),
        Err(ConfinementError::Plan(_))
    ));
}

#[test]
fn descriptor_isolation_still_applies() {
    // `apply` runs spawn-isolation first: stderr is /dev/null even though
    // the test harness's stderr is a pipe.
    let output = run(Some(&confinement(&[])), &["hello"]);
    assert!(output.stderr.is_empty());
}

/// A copy of the probe whose `PT_INTERP` names `interpreter` instead of the
/// real loader: what a hostile agent author can ship.
fn probe_with_interpreter(interpreter: &str) -> PathBuf {
    let mut bytes = std::fs::read(PROBE).unwrap();
    let at = bytes
        .windows(8)
        .position(|window| window == b"ld-linux")
        .expect("the probe is dynamically linked");
    let start = bytes[..at].iter().rposition(|b| *b == 0).unwrap() + 1;
    let end = at + bytes[at..].iter().position(|b| *b == 0).unwrap();
    assert!(
        interpreter.len() <= end - start,
        "{interpreter} does not fit"
    );
    bytes[start..end].fill(0);
    bytes[start..start + interpreter.len()].copy_from_slice(interpreter.as_bytes());
    let path = temp_file("hostile-agent", "");
    std::fs::write(&path, bytes).unwrap();
    path
}

#[test]
fn an_interpreter_outside_the_library_directories_is_refused() {
    // Review M2: PT_INTERP is the agent author's to write. Trusted, it put
    // a directory (a whole-tree read) or any file into the ruleset.
    for interpreter in [
        "/tmp",
        "/etc/passwd",
        "/lib",
        "/lib/../etc/passwd",
        "/no/such",
        // Inside a library directory, but not a loader.
        "/usr/lib/os-release",
    ] {
        let hostile = probe_with_interpreter(interpreter);
        let error = LinuxConfinement::prepare(&plan(&[]), &hostile)
            .err()
            .unwrap_or_else(|| panic!("PT_INTERP {interpreter} must refuse the launch"));
        assert!(
            matches!(error, ConfinementError::Executable(_)),
            "{interpreter}: {error:?}"
        );
        std::fs::remove_file(hostile).unwrap();
    }
}
