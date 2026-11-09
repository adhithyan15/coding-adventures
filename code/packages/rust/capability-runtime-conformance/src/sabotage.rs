//! Negative controls (VER08b section 8): deliberately forbidden operations.
//!
//! A gate that has never been seen to fail is not evidence. After arming, the
//! test binary can be told (`--rcg-sabotage <name>`) to attempt exactly one
//! operation the manifest does not allow. The parent then checks that the
//! enforcer (SIGSYS) and the observer (strace) each catch it, on their own.
//!
//! Syscalls are issued through `libc::syscall` with explicit numbers rather
//! than through libc wrappers, so what reaches the kernel is exactly what the
//! control name says (a wrapper may pick `openat` for `open`, or use the
//! vDSO). The operations are harmless if they do get through: reads of
//! `/etc/hostname`, a socket that is never connected, `kill(0, 0)` (a probe
//! that signals nobody), `unlink` of a path that does not exist.

use std::ffi::CString;

/// One control: its name and the syscall names an observer may report first.
pub struct Control {
    /// Name accepted by `--rcg-sabotage`.
    pub name: &'static str,
    /// Observer must report a violation among these names.
    pub expected: &'static [&'static str],
}

/// Every control, in the order the spec lists them.
pub const CONTROLS: &[Control] = &[
    Control {
        name: "fs-open",
        expected: &["openat"],
    },
    Control {
        name: "fs-create",
        expected: &["openat"],
    },
    Control {
        name: "fs-delete",
        expected: &["unlink", "unlinkat"],
    },
    Control {
        name: "fs-list",
        expected: &["getdents64"],
    },
    Control {
        name: "net-socket",
        expected: &["socket"],
    },
    Control {
        name: "proc-spawn",
        expected: &[
            "clone",
            "clone3",
            "vfork",
            "execve",
            "rt_sigprocmask",
            "pipe2",
            "poll",
        ],
    },
    Control {
        name: "proc-exec",
        expected: &["execve"],
    },
    Control {
        name: "proc-signal",
        expected: &["kill"],
    },
    Control {
        name: "time-syscall",
        expected: &["clock_gettime"],
    },
    Control {
        name: "time-sleep",
        expected: &["nanosleep"],
    },
    Control {
        name: "rand-syscall",
        expected: &["getrandom"],
    },
    Control {
        name: "stdin-read",
        expected: &["read"],
    },
    Control {
        name: "bad-fd-write",
        expected: &["write"],
    },
    Control {
        name: "thread-spawn",
        expected: &[
            "mprotect",
            "clone3",
            "clone",
            "rt_sigprocmask",
            "sigaltstack",
            "rseq",
            "set_robust_list",
            "rt_sigaction",
        ],
    },
    Control {
        name: "exec-map",
        expected: &["mmap"],
    },
    Control {
        name: "mprotect-exec",
        expected: &["mprotect"],
    },
    Control {
        name: "io-uring",
        expected: &["io_uring_setup"],
    },
    Control {
        name: "bpf",
        expected: &["bpf"],
    },
];

/// Controls that are not syscalls and are handled by the parent/launcher.
pub const ENV_LEAK: &str = "env-leak";
/// A known gap, recorded rather than asserted (VER08b section 8).
pub const TIME_VDSO: &str = "time-vdso";

/// Perform the named forbidden operation. Returns false for an unknown name.
///
/// If the sandbox does not stop the call this function returns normally and
/// the caller reports `SABOTAGE-SURVIVED`.
pub fn attempt(name: &str, nonce: &str) -> bool {
    // SAFETY (whole function): each `libc::syscall` below passes integers or
    // pointers to locals that outlive the call; these are controlled probes
    // whose purpose is to be refused.
    unsafe {
        match name {
            "fs-open" => {
                let path = CString::new("/etc/hostname").unwrap();
                libc::syscall(
                    libc::SYS_openat,
                    libc::AT_FDCWD,
                    path.as_ptr(),
                    libc::O_RDONLY,
                );
            }
            "fs-create" => {
                let path = CString::new(format!("/tmp/rcg-sabotage-{nonce}")).unwrap();
                libc::syscall(
                    libc::SYS_openat,
                    libc::AT_FDCWD,
                    path.as_ptr(),
                    libc::O_WRONLY | libc::O_CREAT,
                    0o600,
                );
            }
            "fs-delete" => {
                let path = CString::new("/tmp/rcg-sabotage-does-not-exist").unwrap();
                libc::syscall(libc::SYS_unlinkat, libc::AT_FDCWD, path.as_ptr(), 0);
            }
            "fs-list" => {
                let mut buf = [0u8; 64];
                libc::syscall(libc::SYS_getdents64, 0, buf.as_mut_ptr(), buf.len());
            }
            "net-socket" => {
                libc::syscall(libc::SYS_socket, libc::AF_INET, libc::SOCK_STREAM, 0);
            }
            "proc-spawn" => {
                let _ = std::process::Command::new("/bin/true").spawn();
            }
            "proc-exec" => {
                let path = CString::new("/bin/true").unwrap();
                let argv = [path.as_ptr(), std::ptr::null()];
                let envp = [std::ptr::null::<libc::c_char>()];
                libc::syscall(
                    libc::SYS_execve,
                    path.as_ptr(),
                    argv.as_ptr(),
                    envp.as_ptr(),
                );
            }
            "proc-signal" => {
                // pid 0 = our process group, signal 0 = existence probe only.
                libc::syscall(libc::SYS_kill, 0, 0);
            }
            "time-syscall" => {
                let mut ts = libc::timespec {
                    tv_sec: 0,
                    tv_nsec: 0,
                };
                libc::syscall(libc::SYS_clock_gettime, libc::CLOCK_MONOTONIC, &mut ts);
            }
            "time-sleep" => {
                let ts = libc::timespec {
                    tv_sec: 0,
                    tv_nsec: 1,
                };
                libc::syscall(libc::SYS_nanosleep, &ts, std::ptr::null::<libc::timespec>());
            }
            "rand-syscall" => {
                let mut buf = [0u8; 8];
                libc::syscall(libc::SYS_getrandom, buf.as_mut_ptr(), buf.len(), 0);
            }
            "stdin-read" => {
                let mut buf = [0u8; 1];
                libc::syscall(libc::SYS_read, 0, buf.as_mut_ptr(), 1);
            }
            "bad-fd-write" => {
                libc::syscall(libc::SYS_write, 7, b"x".as_ptr(), 1);
            }
            "thread-spawn" => {
                let _ = std::thread::spawn(|| {}).join();
            }
            "exec-map" => {
                libc::mmap(
                    std::ptr::null_mut(),
                    4096,
                    libc::PROT_READ | libc::PROT_EXEC,
                    libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                    -1,
                    0,
                );
            }
            "mprotect-exec" => {
                let page = libc::mmap(
                    std::ptr::null_mut(),
                    4096,
                    libc::PROT_READ | libc::PROT_WRITE,
                    libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                    -1,
                    0,
                );
                libc::mprotect(page, 4096, libc::PROT_READ | libc::PROT_EXEC);
            }
            "io-uring" => {
                libc::syscall(libc::SYS_io_uring_setup, 1, std::ptr::null::<u8>());
            }
            "bpf" => {
                libc::syscall(libc::SYS_bpf, 0, std::ptr::null::<u8>(), 0);
            }
            "time-vdso" => {
                // Served by the vDSO: no syscall, invisible to seccomp and strace.
                let _ = std::time::Instant::now();
            }
            _ => return false,
        }
    }
    true
}

// In-process tests. Run unarmed, the harmless controls simply succeed, which
// proves each match arm issues a well-formed call and returns `true`. The
// controls that could damage or hang the test process are omitted here and are
// exercised for real, inside the armed child, by the gate's negative controls:
// `proc-exec` (would replace the test process), `stdin-read` (could block),
// `bad-fd-write` (writes to fd 7, which may be open).
#[cfg(test)]
mod tests {
    use super::*;

    const NOT_RUN_UNARMED: &[&str] = &["proc-exec", "stdin-read", "bad-fd-write"];

    #[test]
    fn harmless_controls_are_known_and_return_true() {
        for control in CONTROLS {
            if NOT_RUN_UNARMED.contains(&control.name) {
                continue;
            }
            assert!(attempt(control.name, "unit"), "{}", control.name);
        }
        assert!(attempt(TIME_VDSO, "unit"));
        let _ = std::fs::remove_file("/tmp/rcg-sabotage-unit");
    }

    #[test]
    fn unknown_control_is_reported_unknown() {
        assert!(!attempt("no-such-control", "unit"));
        assert!(!attempt(ENV_LEAK, "unit"), "env-leak is parent-driven");
    }

    #[test]
    fn control_names_are_unique_and_expect_something() {
        let mut names: Vec<_> = CONTROLS.iter().map(|c| c.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), CONTROLS.len());
        assert!(CONTROLS.iter().all(|c| !c.expected.is_empty()));
    }
}
