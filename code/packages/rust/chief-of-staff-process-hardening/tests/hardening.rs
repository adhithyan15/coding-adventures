//! D18S S-I5: the daemon's core-dump suppression, read back from outside the
//! hardened process.

#![cfg(unix)]

use std::process::Command;

const PROBE: &str = env!("CARGO_BIN_EXE_process-hardening-probe");

fn probe(args: &[&str]) -> String {
    let output = Command::new(PROBE).args(args).output().expect("probe runs");
    assert!(output.status.success(), "{output:?}");
    String::from_utf8(output.stdout).unwrap()
}

fn field<'a>(report: &'a str, name: &str) -> &'a str {
    report
        .lines()
        .find_map(|line| line.strip_prefix(name))
        .unwrap_or_else(|| panic!("{name} missing from {report:?}"))
}

#[test]
fn the_core_limit_is_zero_and_cannot_be_raised() {
    let report = probe(&[]);
    // Soft and hard both zero: the hard limit is what makes it permanent.
    assert_eq!(field(&report, "core_limit="), "0/0");
    assert!(field(&report, "applied=").contains("RLIMIT_CORE"));
}

#[cfg(target_os = "linux")]
#[test]
fn linux_marks_the_process_not_dumpable() {
    // The control: an ordinary process is dumpable.
    assert_eq!(field(&probe(&["untouched"]), "dumpable="), "1");
    let report = probe(&[]);
    assert_eq!(field(&report, "dumpable="), "0");
    assert!(field(&report, "applied=").contains("PR_SET_DUMPABLE"));
    assert_eq!(field(&report, "missing="), "");
}

#[cfg(target_os = "linux")]
#[test]
fn a_hardened_process_can_still_list_its_own_descriptors() {
    // spawn-isolation and the Linux sandbox's shim read /proc/self/fd in
    // the supervisor's forked children, which inherit non-dumpability until
    // their exec. That must keep working.
    assert_eq!(field(&probe(&[]), "own_fds_listable="), "true");
}

#[cfg(target_os = "linux")]
#[test]
fn a_hardened_process_is_hidden_from_its_own_user() {
    // Non-dumpable makes /proc/<pid> belong to root, so a process of the
    // same user (an agent, say) cannot open its mem or ptrace it. Run as
    // root (some CI containers), ownership cannot show the difference.
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::fs::MetadataExt;
    use std::process::Stdio;
    // SAFETY: geteuid cannot fail.
    let me = unsafe { libc::geteuid() };
    let owner = |hardened: bool| {
        let mut child = Command::new(PROBE)
            .args(if hardened {
                vec!["wait"]
            } else {
                vec!["untouched", "wait"]
            })
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        // Wait until the probe has hardened itself and said so.
        let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
        assert!(lines.any(|line| line.unwrap() == "waiting"));
        let uid = std::fs::metadata(format!("/proc/{}/mem", child.id()))
            .unwrap()
            .uid();
        child.stdin.take().unwrap().write_all(b"\n").unwrap();
        child.wait().unwrap();
        uid
    };
    assert_eq!(owner(true), 0);
    if me != 0 {
        assert_eq!(owner(false), me, "control");
    }
}

#[cfg(target_os = "linux")]
#[test]
fn an_exec_d_child_is_dumpable_again_but_keeps_the_zero_limit() {
    // Agents are not affected by the daemon's dumpability: exec resets it.
    // The zero core limit is inherited, which costs an agent nothing.
    let report = probe(&["child"]);
    assert_eq!(field(&report, "child_dumpable="), "1");
    assert_eq!(field(&report, "child_core_limit="), "0/0");
}
