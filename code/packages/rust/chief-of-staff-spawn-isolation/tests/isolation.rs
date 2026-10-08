//! The spawn-site guarantees of D18S S-I2 and S-I3, checked from inside a
//! real child process.

#![cfg(unix)]

use chief_of_staff_spawn_isolation::isolate;
use std::os::fd::{FromRawFd, OwnedFd};
use std::process::{Command, Stdio};

const PROBE: &str = env!("CARGO_BIN_EXE_spawn-isolation-probe");

struct Report {
    open_above_2: Vec<u32>,
    stderr_is_dev_null: bool,
    session_leader: bool,
}

/// Run the probe and return its report.
fn probe(isolated: bool) -> Report {
    let mut command = Command::new(PROBE);
    command.stdin(Stdio::null());
    if isolated {
        isolate(&mut command);
    }
    let output = command.output().expect("probe runs");
    let report = String::from_utf8(output.stdout).unwrap();
    let field = |name: &str| {
        report
            .lines()
            .find_map(|line| line.strip_prefix(name))
            .unwrap_or_else(|| panic!("{name} missing from {report:?}"))
            .to_string()
    };
    let open = field("open_above_2=")
        .split(',')
        .filter(|fd| !fd.is_empty())
        .map(|fd| fd.parse().unwrap())
        .collect();
    Report {
        open_above_2: open,
        stderr_is_dev_null: field("stderr_is_dev_null=") == "true",
        session_leader: field("session_leader=") == "true",
    }
}

/// A descriptor without FD_CLOEXEC: what a raw `open` that forgot the flag
/// would leave behind.
fn leaked_descriptor() -> OwnedFd {
    // SAFETY: `dup` returns a new descriptor (never FD_CLOEXEC) or -1.
    let fd = unsafe { libc::dup(1) };
    assert!(fd > 2, "dup failed");
    // SAFETY: `fd` was just returned by `dup`, and nothing else owns it.
    unsafe { OwnedFd::from_raw_fd(fd) }
}

#[test]
fn an_inherited_descriptor_does_not_reach_the_child() {
    use std::os::fd::AsRawFd;
    let leak = leaked_descriptor();
    let fd = leak.as_raw_fd() as u32;
    // The control: without isolation the child holds it.
    let open = probe(false).open_above_2;
    assert!(
        open.contains(&fd),
        "control: {fd} should leak into {open:?}"
    );
    // With isolation the child holds nothing above fd 2.
    let open = probe(true).open_above_2;
    assert!(open.is_empty(), "isolated child still holds {open:?}");
}

#[test]
fn stderr_is_dev_null() {
    assert!(probe(true).stderr_is_dev_null);
}

#[test]
fn the_child_leaves_the_supervisors_session() {
    // Review M1: in the supervisor's session, a child can open /dev/tty and
    // push keystrokes into the terminal the supervisor runs in. A session
    // of its own has no controlling terminal.
    assert!(!probe(false).session_leader, "control");
    assert!(probe(true).session_leader);
}

#[test]
fn a_failed_exec_is_still_a_failed_spawn() {
    // Marking close-on-exec, rather than closing, keeps std's exec-error
    // pipe working until the exec. (Closing instead makes this spawn
    // "succeed" with a child that exits; mutating the flag to 0 shows it.)
    let error = isolate(&mut Command::new("/nonexistent/agent-runtime"))
        .spawn()
        .expect_err("a missing program must not spawn");
    assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn a_terminal_on_a_standard_descriptor_refuses_the_spawn() {
    let mut master = -1;
    let mut slave = -1;
    // SAFETY: `openpty` writes two descriptors through the pointers, or
    // fails; the null name, termios and winsize arguments are allowed.
    let opened = unsafe {
        libc::openpty(
            &mut master,
            &mut slave,
            std::ptr::null_mut(),
            std::ptr::null(),
            std::ptr::null(),
        )
    };
    assert_eq!(opened, 0, "openpty failed");
    // SAFETY: both descriptors were just opened and are owned here.
    let (_master, slave) = unsafe { (OwnedFd::from_raw_fd(master), OwnedFd::from_raw_fd(slave)) };
    let mut command = Command::new(PROBE);
    command.stdin(Stdio::from(slave)).stdout(Stdio::null());
    let error = isolate(&mut command)
        .spawn()
        .expect_err("a terminal on stdin must refuse the spawn");
    assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
}
