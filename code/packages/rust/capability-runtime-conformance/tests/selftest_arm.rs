//! Live self-test 4: `arm()` itself, without strace, in forked children.
//!
//! `harness = false` and single-threaded, so `fork` is safe (a forked child
//! of a multithreaded libtest process could inherit a held allocator lock).
//! Each child arms a policy, makes raw syscalls, and the parent checks how it
//! ended. This is the unit-level proof that the installed filter allows what
//! the policy allows and kills everything else, and that arming cannot be
//! repeated or loosened.

#[cfg(target_os = "linux")]
mod live {
    use capability_runtime_conformance::arm::{arm, landlock_abi};
    use capability_runtime_conformance::sabotage::{attempt, CONTROLS};
    use capability_runtime_conformance::{PolicyOptions, RuntimePolicy};

    fn policy(caps: &str) -> RuntimePolicy {
        let json = format!(
            r#"{{"version":1,"package":"rust/arm-selftest","capabilities":[{caps}],"justification":"arm self-test manifest"}}"#
        );
        RuntimePolicy::from_manifest_json(&json, PolicyOptions::default()).unwrap()
    }

    #[derive(Debug, PartialEq, Eq)]
    enum End {
        Exit(i32),
        Signal(i32),
    }

    /// Fork; run `body` in the child (it must end via `_exit` or be killed).
    fn in_child(body: impl FnOnce()) -> End {
        // SAFETY: single-threaded process; the child only uses raw syscalls
        // and the allocator after fork, and never returns into libtest.
        let pid = unsafe { libc::fork() };
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            body();
            // SAFETY: leave without running destructors/atexit.
            unsafe { libc::_exit(99) };
        }
        let mut status = 0;
        // SAFETY: waiting for our own child.
        unsafe { libc::waitpid(pid, &mut status, 0) };
        if libc::WIFSIGNALED(status) {
            End::Signal(libc::WTERMSIG(status))
        } else {
            End::Exit(libc::WEXITSTATUS(status))
        }
    }

    fn exit(code: i32) -> ! {
        // SAFETY: raw exit_group, allowed by the baseline.
        unsafe { libc::syscall(libc::SYS_exit_group, code) };
        unreachable!()
    }

    pub fn run() {
        assert!(landlock_abi() >= 0);
        assert!(!attempt("no-such-control", "n"));
        let mut names: Vec<&str> = CONTROLS.iter().map(|c| c.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), CONTROLS.len(), "control names are unique");
        assert_eq!(CONTROLS.len(), 18);

        // 1. Allowed baseline calls work; the exit code proves we got there.
        let pure = policy("");
        assert_eq!(
            in_child(|| {
                arm(&pure).expect("arm");
                // SAFETY: write to fd 1 is in the baseline.
                unsafe { libc::syscall(libc::SYS_write, 1, b"".as_ptr(), 0) };
                exit(7);
            }),
            End::Exit(7)
        );

        // 2. Anything else kills the process with SIGSYS.
        for name in ["fs-open", "net-socket", "bad-fd-write", "exec-map", "bpf"] {
            assert_eq!(
                in_child(|| {
                    arm(&pure).expect("arm");
                    attempt(name, "arm-selftest");
                    exit(0);
                }),
                End::Signal(libc::SIGSYS),
                "{name} must be killed"
            );
        }

        // 3. Arming cannot be repeated or loosened: the second arm's first
        //    call (prctl) is outside the filter.
        assert_eq!(
            in_child(|| {
                arm(&pure).expect("arm");
                let _ = arm(&pure);
                exit(0);
            }),
            End::Signal(libc::SIGSYS)
        );

        // 4. Landlock scoping: seccomp allows openat for fs:read, Landlock
        //    allows only the declared file.
        if landlock_abi() > 0 {
            let fs = policy(
                r#"{"category":"fs","action":"read","target":"/etc/passwd","justification":"arm self-test file"}"#,
            );
            assert_eq!(
                in_child(|| {
                    let report = arm(&fs).expect("arm");
                    assert!(report.landlock_enforced && report.seccomp_enforced);
                    let open = |path: &str| {
                        let c = std::ffi::CString::new(path).unwrap();
                        // SAFETY: openat on a valid C string.
                        unsafe {
                            libc::syscall(
                                libc::SYS_openat,
                                libc::AT_FDCWD,
                                c.as_ptr(),
                                libc::O_RDONLY,
                            )
                        }
                    };
                    let allowed = open("/etc/passwd") >= 0;
                    let denied = open("/etc/group") < 0
                        && std::io::Error::last_os_error().raw_os_error() == Some(libc::EACCES);
                    exit(if allowed && denied { 0 } else { 3 });
                }),
                End::Exit(0)
            );
        } else {
            println!("note: Landlock unavailable, fs scoping case skipped");
        }
        println!("arm self-test: ok");
    }
}

fn main() {
    #[cfg(target_os = "linux")]
    live::run();
    #[cfg(not(target_os = "linux"))]
    println!("not-live: runtime capability gate is Linux-only (RCG010)");
}
