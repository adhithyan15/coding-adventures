//! Unit tests: manifest -> plan -> policy -> filter, the strace classifier on
//! recorded fixtures, the verdict table, the container rendering, JSON.
//!
//! None of these install a sandbox into the test process; the live behaviour
//! is covered by the `selftest_*` targets and by ct-compare's gate.

use capability_runtime_conformance::container::{
    docker_args, parse_ldd, seccomp_profile_json, ARMING_SYSCALLS, STARTUP_SYSCALLS,
};
use capability_runtime_conformance::failure::FailureId;
use capability_runtime_conformance::harness::{judge, ChildStatus, SIGSYS};
use capability_runtime_conformance::json::Json;
use capability_runtime_conformance::observe::{
    analyze, env_violations, parse_execve_env, parse_line, Ending, Event,
};
use capability_runtime_conformance::policy::{
    family_of, PolicyOptions, Pred, RuntimePolicy, BASELINE, NEVER_ALLOWED,
};

fn manifest(caps: &[(&str, &str, &str)]) -> String {
    let items: Vec<String> = caps
        .iter()
        .map(|(category, action, target)| {
            format!(
                r#"{{"category":"{category}","action":"{action}","target":"{target}","justification":"unit-test capability for the mapping table"}}"#
            )
        })
        .collect();
    format!(
        r#"{{"version":1,"package":"rust/unit-subject","capabilities":[{}],"justification":"unit-test manifest for the runtime gate"}}"#,
        items.join(",")
    )
}

fn policy(caps: &[(&str, &str, &str)]) -> RuntimePolicy {
    RuntimePolicy::from_manifest_json(&manifest(caps), PolicyOptions::default())
        .unwrap_or_else(|r| panic!("{r}"))
}

/// Every capability, split into groups the capability cage accepts together
/// (it rejects manifests that combine untrusted inputs with external
/// actuations, so one giant manifest is not valid).
const EVERY_GROUP: &[&[(&str, &str, &str)]] = &[
    &[
        ("fs", "read", "/etc/a"),
        ("fs", "list", "/var/tmp/e"),
        ("time", "read", "*"),
        ("time", "sleep", "*"),
        ("env", "read", "*"),
        ("stdin", "read", "*"),
        ("stdout", "write", "*"),
        ("rand", "read", "*"),
    ],
    &[
        ("fs", "write", "/var/tmp/b"),
        ("fs", "create", "/var/tmp/c"),
        ("fs", "delete", "/var/tmp/d"),
    ],
    &[
        ("net", "connect", "example.com:443"),
        ("net", "listen", "127.0.0.1:80"),
        ("net", "dns", "example.com"),
    ],
    &[
        ("proc", "exec", "git"),
        ("proc", "fork", "git"),
        ("proc", "signal", "git"),
    ],
];

fn names(policy: &RuntimePolicy) -> Vec<&'static str> {
    policy.allow.keys().copied().collect()
}

fn baseline_names() -> Vec<&'static str> {
    let mut v: Vec<&'static str> = BASELINE.iter().map(|b| b.name).collect();
    v.sort_unstable();
    v
}

// ---------------------------------------------------------------- mapping

#[test]
fn empty_manifest_yields_exactly_the_baseline() {
    let p = policy(&[]);
    assert_eq!(names(&p), baseline_names());
    assert!(p.declared.is_empty());
    assert!(p.landlock_paths.is_empty());
    assert!(!p.needs_network);
    assert!(!p.landlock_required);
    assert_eq!(p.baseline_version, "rcg-baseline-1");
    assert_eq!(p.manifest_sha256.len(), 64);
    assert_eq!(p.plan_sha256.len(), 64);
}

#[test]
fn baseline_is_small_justified_and_has_no_forbidden_calls() {
    assert!(BASELINE.len() <= 8, "the baseline must stay minimal");
    for entry in BASELINE {
        assert!(
            entry.why.len() > 20,
            "{} needs a real justification",
            entry.name
        );
        assert!(
            !NEVER_ALLOWED.contains(&entry.name),
            "{} is never allowed",
            entry.name
        );
    }
    // write is fd-scoped, mmap forbids PROT_EXEC, futex is wake-only.
    let p = policy(&[]);
    assert_eq!(p.allow["write"], Pred::ArgIn(0, &[1, 2]));
    assert_eq!(p.allow["mmap"], Pred::NoExecProt(2));
    assert_eq!(p.allow["futex"], Pred::ArgIn(1, &[1, 129]));
}

fn assert_adds(caps: &[(&str, &str, &str)], expected: &[&str]) {
    let p = policy(caps);
    for name in expected {
        assert!(p.allow.contains_key(name), "{caps:?} should allow {name}");
    }
    // ...and nothing outside baseline + expected.
    let base = baseline_names();
    for name in names(&p) {
        assert!(
            base.contains(&name) || expected.contains(&name),
            "{caps:?} unexpectedly allows {name}"
        );
    }
}

#[test]
fn fs_read_maps_to_read_syscalls_and_a_landlock_read_rule() {
    let caps = [("fs", "read", "/etc/hostname")];
    assert_adds(
        &caps,
        &[
            "openat",
            "open",
            "close",
            "fstat",
            "newfstatat",
            "stat",
            "lstat",
            "statx",
            "lseek",
            "fcntl",
            "read",
            "pread64",
            "readv",
            "preadv",
            "access",
            "faccessat",
            "faccessat2",
            "readlink",
            "readlinkat",
        ],
    );
    let p = policy(&caps);
    assert_eq!(
        p.allow["write"],
        Pred::ArgIn(0, &[1, 2]),
        "reads never widen write"
    );
    assert_eq!(
        p.landlock_paths,
        vec![("/etc/hostname".to_string(), vec!["READ_FILE"])]
    );
    assert!(p.landlock_required);
}

#[test]
fn fs_write_widens_write_and_adds_landlock_write_rights() {
    let caps = [("fs", "write", "/var/tmp/out.txt")];
    assert_adds(
        &caps,
        &[
            "openat",
            "open",
            "close",
            "fstat",
            "newfstatat",
            "stat",
            "lstat",
            "statx",
            "lseek",
            "fcntl",
            "write",
            "pwrite64",
            "writev",
            "pwritev",
            "ftruncate",
            "fsync",
            "fdatasync",
        ],
    );
    let p = policy(&caps);
    assert_eq!(p.allow["write"], Pred::Any);
    assert_eq!(
        p.landlock_paths,
        vec![(
            "/var/tmp/out.txt".to_string(),
            vec!["WRITE_FILE", "TRUNCATE"]
        )]
    );
}

#[test]
fn fs_create_delete_list_map_to_their_own_calls_and_rights() {
    assert_adds(
        &[("fs", "create", "/var/tmp/new.txt")],
        &[
            "openat",
            "open",
            "close",
            "fstat",
            "newfstatat",
            "stat",
            "lstat",
            "statx",
            "lseek",
            "fcntl",
            "creat",
            "mkdir",
            "mkdirat",
        ],
    );
    assert_adds(
        &[("fs", "delete", "/var/tmp/old.txt")],
        &["unlink", "unlinkat", "rmdir"],
    );
    assert_adds(
        &[("fs", "list", "/var/tmp/dir")],
        &["getdents", "getdents64"],
    );
    let p = policy(&[("fs", "delete", "/var/tmp/old.txt")]);
    assert_eq!(p.landlock_paths[0].1, vec!["REMOVE_FILE", "REMOVE_DIR"]);
    let p = policy(&[("fs", "list", "/var/tmp/dir")]);
    assert_eq!(p.landlock_paths[0].1, vec!["READ_DIR"]);
    let p = policy(&[("fs", "create", "/var/tmp/new.txt")]);
    assert_eq!(p.landlock_paths[0].1, vec!["MAKE_REG", "MAKE_DIR"]);
}

#[test]
fn fs_star_target_grants_landlock_on_root_and_relative_targets_are_refused() {
    let p = policy(&[("fs", "read", "*")]);
    assert_eq!(p.landlock_paths[0].0, "/");
    for bad in ["relative/file", "/glob/*.txt", "../up"] {
        let refusal = RuntimePolicy::from_manifest_json(
            &manifest(&[("fs", "read", bad)]),
            PolicyOptions::default(),
        )
        .expect_err(bad);
        assert_eq!(refusal.id, FailureId::Rcg006, "{bad}");
    }
}

#[test]
fn net_capabilities_map_to_socket_calls_and_need_a_network() {
    let common = [
        "socket",
        "getsockopt",
        "setsockopt",
        "getsockname",
        "getpeername",
        "poll",
        "ppoll",
        "epoll_create1",
        "epoll_ctl",
        "epoll_wait",
        "epoll_pwait",
        "shutdown",
        "close",
        "read",
    ];
    let mut connect: Vec<&str> = common.to_vec();
    connect.extend(["connect", "sendto", "recvfrom", "sendmsg", "recvmsg"]);
    assert_adds(&[("net", "connect", "example.com:443")], &connect);
    let mut listen: Vec<&str> = common.to_vec();
    listen.extend(["bind", "listen", "accept", "accept4"]);
    assert_adds(&[("net", "listen", "127.0.0.1:8080")], &listen);
    assert!(policy(&[("net", "connect", "example.com:443")]).needs_network);

    let dns = policy(&[("net", "dns", "example.com")]);
    assert!(dns.allow.contains_key("connect") && dns.allow.contains_key("openat"));
    let paths: Vec<&str> = dns.landlock_paths.iter().map(|(p, _)| p.as_str()).collect();
    assert_eq!(
        paths,
        ["/etc/resolv.conf", "/etc/hosts", "/etc/nsswitch.conf"]
    );
}

#[test]
fn proc_capabilities_map_to_spawn_fork_signal_calls() {
    assert_adds(
        &[("proc", "exec", "git")],
        &[
            "execve", "execveat", "wait4", "waitid", "pipe", "pipe2", "dup", "dup2", "dup3",
            "clone", "vfork",
        ],
    );
    assert_adds(
        &[("proc", "fork", "worker")],
        &["fork", "vfork", "clone", "clone3", "wait4", "waitid"],
    );
    assert_adds(
        &[("proc", "signal", "worker")],
        &["kill", "tgkill", "tkill", "rt_sigqueueinfo"],
    );
}

#[test]
fn env_adds_no_syscalls_only_an_allowlist() {
    let p = policy(&[("env", "read", "HOME"), ("env", "write", "TMPDIR")]);
    assert_eq!(names(&p), baseline_names());
    assert!(p.env_name_allowed("HOME") && p.env_name_allowed("TMPDIR"));
    assert!(!p.env_name_allowed("SECRET"));
    assert!(policy(&[("env", "read", "*")]).env_name_allowed("ANYTHING"));
    assert!(!policy(&[]).env_name_allowed("HOME"));
}

#[test]
fn time_stdin_stdout_rand_map_as_documented() {
    assert_adds(
        &[("time", "read", "*")],
        &["clock_gettime", "clock_getres", "gettimeofday", "time"],
    );
    assert_adds(
        &[("time", "sleep", "*")],
        &[
            "clock_gettime",
            "clock_getres",
            "gettimeofday",
            "time",
            "nanosleep",
            "clock_nanosleep",
        ],
    );
    let stdin = policy(&[("stdin", "read", "*")]);
    assert_eq!(stdin.allow["read"], Pred::ArgIn(0, &[0]));
    let stdout = policy(&[("stdout", "write", "*")]);
    assert_eq!(
        stdout.allow["write"],
        Pred::ArgIn(0, &[1, 2]),
        "stdout is inside the baseline"
    );
    assert_adds(&[("rand", "read", "*")], &["getrandom"]);
}

#[test]
fn thread_is_refused_unless_coarse_mapping_is_requested() {
    let json = manifest(&[("thread", "spawn", "decode-pool")]);
    let refusal =
        RuntimePolicy::from_manifest_json(&json, PolicyOptions::default()).expect_err("refused");
    assert_eq!(refusal.id, FailureId::Rcg006);
    assert!(refusal.to_string().contains("RCG006"));

    let p = RuntimePolicy::from_manifest_json(
        &json,
        PolicyOptions {
            allow_coarse_thread: true,
        },
    )
    .expect("coarse");
    for name in [
        "clone",
        "clone3",
        "futex",
        "set_robust_list",
        "rseq",
        "sched_yield",
    ] {
        assert!(p.allow.contains_key(name), "{name}");
    }
    assert_eq!(p.allow["mprotect"], Pred::NoExecProt(2));
    // The coarse option must not let ffi through.
    let both = manifest(&[("thread", "spawn", "pool"), ("ffi", "call", "libssl")]);
    let refusal = RuntimePolicy::from_manifest_json(
        &both,
        PolicyOptions {
            allow_coarse_thread: true,
        },
    )
    .expect_err("ffi still refused");
    assert_eq!(refusal.id, FailureId::Rcg006);
}

#[test]
fn ffi_and_ui_are_always_refused() {
    for caps in [
        [("ffi", "call", "libssl")],
        [("ffi", "load", "libssl")],
        [("ui", "read", "#editor")],
    ] {
        let refusal = RuntimePolicy::from_manifest_json(&manifest(&caps), PolicyOptions::default())
            .expect_err("refused");
        assert_eq!(refusal.id, FailureId::Rcg006, "{caps:?}");
    }
}

#[test]
fn invalid_manifest_is_refused_not_panicked() {
    for bad in ["", "not json", r#"{"version":1}"#] {
        let refusal =
            RuntimePolicy::from_manifest_json(bad, PolicyOptions::default()).expect_err(bad);
        assert_eq!(refusal.id, FailureId::Rcg006);
    }
}

#[test]
fn no_capability_ever_allows_a_never_allowed_syscall() {
    let mut declared = 0;
    for group in EVERY_GROUP {
        let p = policy(group);
        for name in NEVER_ALLOWED {
            assert!(
                !p.allow.contains_key(name),
                "{name} must never be allowed ({group:?})"
            );
        }
        declared += p.declared.len();
    }
    assert_eq!(declared, 17, "every capability category/action is covered");
}

#[test]
fn policy_allows_applies_argument_predicates() {
    let p = policy(&[]);
    assert!(p.allows("write", r#"1, "ok\n", 3) = 3"#));
    assert!(p.allows("write", r#"2, "x", 1"#));
    assert!(!p.allows("write", r#"7, "x", 1"#));
    assert!(!p.allows("write", ""));
    assert!(p.allows("mmap", "NULL, 4096, PROT_READ|PROT_WRITE, MAP_PRIVATE"));
    assert!(!p.allows("mmap", "NULL, 4096, PROT_READ|PROT_EXEC, MAP_PRIVATE"));
    assert!(p.allows("futex", "0x7f00, FUTEX_WAKE_PRIVATE, 2147483647) = 202"));
    assert!(p.allows("futex", "0x7f00, FUTEX_WAKE, 1"));
    assert!(!p.allows("futex", "0x7f00, FUTEX_WAIT_PRIVATE, 0"));
    assert!(!p.allows("openat", "AT_FDCWD"));
    assert!(p.allows("brk", "NULL"));
}

#[test]
fn syscall_families_label_violations() {
    assert_eq!(family_of("openat"), "fs");
    assert_eq!(family_of("socket"), "net");
    assert_eq!(family_of("execve"), "proc");
    assert_eq!(family_of("clock_gettime"), "time");
    assert_eq!(family_of("clone3"), "proc");
    assert_eq!(family_of("getrandom"), "rand");
    assert_eq!(family_of("bpf"), "never-allowed");
    assert_eq!(family_of("write"), "baseline");
    assert_eq!(family_of("frobnicate"), "unclassified");
}

// ------------------------------------------------------------------- BPF

#[cfg(target_os = "linux")]
mod bpf_tests {
    use super::*;
    use capability_runtime_conformance::bpf::{
        build_program, evaluate, syscall_number, SeccompData, AUDIT_ARCH, RET_ALLOW,
        RET_KILL_PROCESS,
    };

    fn run(p: &RuntimePolicy, name: &str, args: [u64; 6]) -> u32 {
        let program = build_program(p);
        let nr = syscall_number(name).unwrap_or_else(|| panic!("no number for {name}"));
        evaluate(
            &program,
            &SeccompData {
                nr,
                arch: AUDIT_ARCH,
                args,
            },
        )
    }

    #[test]
    fn program_starts_with_the_arch_check_and_ends_in_kill() {
        let program = build_program(&policy(&[]));
        assert_eq!(program[0].k, 4, "first instruction loads seccomp_data.arch");
        assert_eq!(program[1].k, AUDIT_ARCH);
        assert_eq!(program[2].k, RET_KILL_PROCESS);
        assert_eq!(program.last().unwrap().k, RET_KILL_PROCESS);
        assert!(
            program.len() < 64,
            "a baseline filter is tiny: {}",
            program.len()
        );
    }

    #[test]
    fn wrong_architecture_is_killed() {
        let program = build_program(&policy(&[]));
        let nr = syscall_number("exit_group").unwrap();
        let verdict = evaluate(
            &program,
            &SeccompData {
                nr,
                arch: AUDIT_ARCH ^ 1,
                args: [0; 6],
            },
        );
        assert_eq!(verdict, RET_KILL_PROCESS);
    }

    #[test]
    fn baseline_calls_are_allowed_and_everything_else_is_killed() {
        let p = policy(&[]);
        let z = [0u64; 6];
        for allowed in ["exit_group", "brk", "munmap"] {
            assert_eq!(run(&p, allowed, z), RET_ALLOW, "{allowed}");
        }
        for denied in [
            "openat",
            "read",
            "socket",
            "execve",
            "clone3",
            "getrandom",
            "clock_gettime",
            "ptrace",
            "bpf",
            "io_uring_setup",
            "mprotect",
            "rt_sigprocmask",
            "kill",
            "unlinkat",
            "seccomp",
            "prctl",
            "mremap",
            "madvise",
        ] {
            assert_eq!(run(&p, denied, z), RET_KILL_PROCESS, "{denied}");
        }
        assert_eq!(
            evaluate(
                &build_program(&p),
                &SeccompData {
                    nr: 0x4000_0000 | 1,
                    arch: AUDIT_ARCH,
                    args: [0; 6]
                }
            ),
            RET_KILL_PROCESS,
            "x32 ABI numbers fall to the default"
        );
    }

    #[test]
    fn write_is_limited_to_fd_1_and_2() {
        let p = policy(&[]);
        for fd in [1u64, 2] {
            assert_eq!(run(&p, "write", [fd, 0, 0, 0, 0, 0]), RET_ALLOW);
        }
        for fd in [0u64, 3, 7, 255] {
            assert_eq!(
                run(&p, "write", [fd, 0, 0, 0, 0, 0]),
                RET_KILL_PROCESS,
                "fd {fd}"
            );
        }
        // High garbage in the upper half of the register does not matter:
        // the kernel only uses the low 32 bits as the fd.
        assert_eq!(
            run(&p, "write", [(5u64 << 32) | 1, 0, 0, 0, 0, 0]),
            RET_ALLOW
        );
    }

    #[test]
    fn mmap_forbids_prot_exec_and_futex_allows_only_wake() {
        let p = policy(&[]);
        assert_eq!(run(&p, "mmap", [0, 4096, 3, 0, 0, 0]), RET_ALLOW); // R|W
        assert_eq!(run(&p, "mmap", [0, 4096, 5, 0, 0, 0]), RET_KILL_PROCESS); // R|X
        assert_eq!(run(&p, "mmap", [0, 4096, 7, 0, 0, 0]), RET_KILL_PROCESS); // RWX
        assert_eq!(run(&p, "futex", [0, 1, 0, 0, 0, 0]), RET_ALLOW); // WAKE
        assert_eq!(run(&p, "futex", [0, 129, 0, 0, 0, 0]), RET_ALLOW); // WAKE_PRIVATE
        assert_eq!(run(&p, "futex", [0, 0, 0, 0, 0, 0]), RET_KILL_PROCESS); // WAIT
        assert_eq!(run(&p, "futex", [0, 128, 0, 0, 0, 0]), RET_KILL_PROCESS); // WAIT_PRIVATE
    }

    #[test]
    fn declared_capabilities_widen_exactly_their_calls() {
        let z = [0u64; 6];
        let time = policy(&[("time", "read", "*")]);
        assert_eq!(run(&time, "clock_gettime", z), RET_ALLOW);
        assert_eq!(run(&time, "nanosleep", z), RET_KILL_PROCESS);
        assert_eq!(run(&time, "openat", z), RET_KILL_PROCESS);

        let fs = policy(&[("fs", "read", "/etc/hostname")]);
        assert_eq!(run(&fs, "openat", z), RET_ALLOW);
        assert_eq!(run(&fs, "read", z), RET_ALLOW);
        assert_eq!(run(&fs, "write", [1, 0, 0, 0, 0, 0]), RET_ALLOW);
        assert_eq!(run(&fs, "write", [3, 0, 0, 0, 0, 0]), RET_KILL_PROCESS);
        assert_eq!(run(&fs, "unlinkat", z), RET_KILL_PROCESS);

        let stdin = policy(&[("stdin", "read", "*")]);
        assert_eq!(run(&stdin, "read", [0, 0, 0, 0, 0, 0]), RET_ALLOW);
        assert_eq!(run(&stdin, "read", [4, 0, 0, 0, 0, 0]), RET_KILL_PROCESS);

        let fsw = policy(&[("fs", "write", "/var/tmp/x")]);
        assert_eq!(run(&fsw, "write", [9, 0, 0, 0, 0, 0]), RET_ALLOW);

        let thread = RuntimePolicy::from_manifest_json(
            &manifest(&[("thread", "spawn", "pool")]),
            PolicyOptions {
                allow_coarse_thread: true,
            },
        )
        .unwrap();
        assert_eq!(run(&thread, "clone3", z), RET_ALLOW);
        assert_eq!(run(&thread, "mprotect", [0, 0, 1, 0, 0, 0]), RET_ALLOW);
        assert_eq!(
            run(&thread, "mprotect", [0, 0, 4, 0, 0, 0]),
            RET_KILL_PROCESS
        );
    }

    #[test]
    fn every_baseline_and_allowed_name_has_a_number_on_this_arch() {
        for group in EVERY_GROUP {
            let p = policy(group);
            for name in p.allow.keys() {
                if cfg!(target_arch = "x86_64") {
                    assert!(syscall_number(name).is_some(), "{name} has no number");
                }
            }
        }
        assert!(syscall_number("frobnicate").is_none());
        for name in NEVER_ALLOWED.iter().filter(|n| **n != "mprotect_exec") {
            assert!(syscall_number(name).is_some(), "{name}");
        }
    }
}

// ------------------------------------------------------------- observer

const CLEAN_TRACE: &str = r#"4242  execve("/x/test", ["/x/test", "--rcg-child", "n1", "enforce"], []) = 0
4242  brk(NULL)                         = 0x55d0
4242  openat(AT_FDCWD, "/lib/libc.so.6", O_RDONLY|O_CLOEXEC) = 3
4242  seccomp(SECCOMP_SET_MODE_FILTER, SECCOMP_FILTER_FLAG_TSYNC, {len=20, filter=0x7ff}) = 0
4242  write(2, "RCG-ARMED n1 landlock=1 abi=7 seccomp=1\n", 41) = 41
4242  mmap(NULL, 1052672, PROT_READ|PROT_WRITE, MAP_PRIVATE|MAP_ANONYMOUS, -1, 0) = 0x7f00
4242  futex(0x7f00, FUTEX_WAKE_PRIVATE, 2147483647) = 0
4242  munmap(0x7f00, 1052672)           = 0
4242  write(1, "ok 1 - a\n", 9)         = 9
4242  exit_group(0)                     = ?
4242  +++ exited with 0 +++
"#;

#[test]
fn clean_trace_has_marker_and_no_violations() {
    let p = policy(&[]);
    let a = analyze(CLEAN_TRACE, "n1", &p);
    assert!(a.armed);
    assert_eq!(a.marker_detail, "landlock=1 abi=7 seccomp=1");
    assert_eq!(
        a.pre_arm_calls, 5,
        "execve, brk, openat, seccomp, marker write"
    );
    assert!(a.violations.is_empty(), "{:?}", a.violations);
    assert_eq!(a.post_arm.get("write"), Some(&2));
    assert_eq!(a.post_arm.get("mmap"), Some(&1));
    assert!(
        !a.post_arm.contains_key("openat"),
        "pre-arm calls are not evidence"
    );
    assert_eq!(a.ending, Ending::Exited(0));
    assert_eq!(a.exec_env, Some(vec![]));
    assert!(!a.saw_sigsys);
}

#[test]
fn marker_nonce_must_match() {
    let a = analyze(CLEAN_TRACE, "someone-elses-nonce", &policy(&[]));
    assert!(!a.armed);
    assert!(a.post_arm.is_empty(), "nothing counts without the marker");
}

#[test]
fn violation_trace_lists_every_offending_call() {
    let trace = CLEAN_TRACE.replace(
        "4242  munmap(0x7f00, 1052672)           = 0\n",
        "4242  openat(AT_FDCWD, \"/etc/passwd\", O_RDONLY) = 3\n4242  socket(AF_INET, SOCK_STREAM, IPPROTO_IP) = 4\n4242  write(7, \"x\", 1) = 1\n",
    );
    let a = analyze(&trace, "n1", &policy(&[]));
    let found: Vec<(&str, &str)> = a
        .violations
        .iter()
        .map(|v| (v.name.as_str(), v.family))
        .collect();
    assert_eq!(
        found,
        vec![("openat", "fs"), ("socket", "net"), ("write", "baseline")]
    );
    // The same trace is clean for a package that declares those capabilities.
    // Each violation disappears once its own capability is declared (and
    // only then): openat <- fs:read, socket <- net:connect, write(7) <- fs:write.
    let with = |caps: &[(&str, &str, &str)]| {
        analyze(&trace, "n1", &policy(caps))
            .violations
            .iter()
            .map(|v| v.name.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(
        with(&[("fs", "read", "/etc/hostname")]),
        ["socket", "write"]
    );
    assert_eq!(
        with(&[("net", "connect", "example.com:443")]),
        ["openat", "write"]
    );
    assert_eq!(with(&[("fs", "write", "/var/tmp/w")]), ["socket"]);
}

#[test]
fn killed_trace_still_shows_the_attempt_and_the_signal() {
    let trace = r#"7  execve("/x", ["/x"], []) = 0
7  write(2, "RCG-ARMED k landlock=1 abi=7 seccomp=1\n", 40) = 40
7  openat(AT_FDCWD, "/etc/hostname", O_RDONLY <unfinished ...>
7  --- SIGSYS {si_signo=SIGSYS, si_code=SYS_SECCOMP, si_syscall=__NR_openat} ---
7  +++ killed by SIGSYS +++
"#;
    let p = policy(&[]);
    let a = analyze(trace, "k", &p);
    assert_eq!(a.violations.len(), 1);
    assert_eq!(a.violations[0].name, "openat");
    assert_eq!(a.ending, Ending::Killed("SIGSYS".into()));
    assert!(a.saw_sigsys);
}

#[test]
fn multi_thread_interleaving_unfinished_and_resumed_lines() {
    let trace = r#"10  execve("/x", ["/x"], []) = 0
10  write(2, "RCG-ARMED m landlock=0 abi=0 seccomp=1\n", 40) = 40
10  clone3({flags=CLONE_VM|CLONE_THREAD}, 88 <unfinished ...>
11  rseq(0x7f, 0x20, 0, 0x53053053 <unfinished ...>
10  <... clone3 resumed>)             = 11
11  <... rseq resumed>)               = 0
11  futex(0x7f, FUTEX_WAIT_PRIVATE, 0, NULL <unfinished ...>
[pid  10] exit_group(0)                = ?
11  +++ exited with 0 +++
10  +++ exited with 0 +++
"#;
    let a = analyze(trace, "m", &policy(&[]));
    let names: Vec<&str> = a.violations.iter().map(|v| v.name.as_str()).collect();
    assert_eq!(
        names,
        vec!["clone3", "rseq", "futex"],
        "resumed lines are not counted twice"
    );
    assert_eq!(
        a.ending,
        Ending::Exited(0),
        "only the main pid's epilogue counts"
    );
    let declared = RuntimePolicy::from_manifest_json(
        &manifest(&[("thread", "spawn", "pool")]),
        PolicyOptions {
            allow_coarse_thread: true,
        },
    )
    .unwrap();
    // WAIT futex still violates: thread:spawn allows futex only via the
    // coarse set, which is unconditional.
    assert!(analyze(trace, "m", &declared).violations.is_empty());
}

#[test]
fn missing_marker_means_never_armed() {
    let trace =
        "1  execve(\"/x\", [\"/x\"], []) = 0\n1  exit_group(0) = ?\n1  +++ exited with 0 +++\n";
    let a = analyze(trace, "n", &policy(&[]));
    assert!(!a.armed);
    assert_eq!(a.pre_arm_calls, 2);
}

#[test]
fn unknown_syscall_names_are_violations() {
    let trace = "1  execve(\"/x\", [\"/x\"], []) = 0\n1  write(2, \"RCG-ARMED n x\\n\", 14) = 14\n1  syscall_0x1234(0, 0) = -1\n";
    let a = analyze(trace, "n", &policy(&[]));
    assert_eq!(a.violations[0].family, "unclassified");
}

#[test]
fn line_parser_handles_every_event_kind() {
    assert_eq!(parse_line(""), Event::Other);
    assert_eq!(parse_line("garbage without parens"), Event::Other);
    assert_eq!(parse_line("+++ weird +++"), Event::Other);
    assert!(matches!(
        parse_line("5 +++ exited with 3 +++"),
        Event::Exited { pid: 5, code: 3 }
    ));
    assert!(matches!(
        parse_line("5 +++ killed by SIGSYS (core dumped) +++"),
        Event::Killed { pid: 5, ref signal } if signal == "SIGSYS"
    ));
    assert!(
        matches!(parse_line("5 --- SIGCHLD {si_signo=SIGCHLD} ---"), Event::Signal { ref name } if name == "SIGCHLD")
    );
    assert_eq!(parse_line("5 <... read resumed>) = 1"), Event::Resumed);
    assert!(
        matches!(parse_line("[pid  77] brk(NULL) = 1"), Event::Syscall { pid: 77, ref name, .. } if name == "brk")
    );
    assert!(
        matches!(parse_line("brk(NULL) = 1"), Event::Syscall { pid: 0, ref name, .. } if name == "brk")
    );
    assert_eq!(parse_line("9 (nothing)"), Event::Other);
}

#[test]
fn execve_environment_is_extracted_and_checked() {
    let args =
        r#""/x/test", ["/x/test", "--rcg-child", "n"], ["A=1", "RCG_CANARY=le\"aked", "B=2"]) = 0"#;
    assert_eq!(
        parse_execve_env(args),
        vec!["A=1", "RCG_CANARY=le\"aked", "B=2"]
    );
    assert_eq!(
        parse_execve_env("\"/x\", [\"/x\"], []) = 0"),
        Vec::<String>::new()
    );
    assert_eq!(parse_execve_env("no brackets at all"), Vec::<String>::new());
    let p = policy(&[("env", "read", "A")]);
    let bad = env_violations(&["A=1".into(), "B=2".into(), "RCG_CANARY=x".into()], &p);
    assert_eq!(bad, vec!["B", "RCG_CANARY"]);
    assert!(env_violations(&["A=1".into()], &p).is_empty());
}

#[test]
fn leaked_environment_in_the_trace_is_reported_by_judge() {
    let trace = CLEAN_TRACE.replace("\"enforce\"], [])", "\"enforce\"], [\"RCG_CANARY=1\"])");
    let p = policy(&[]);
    let a = analyze(&trace, "n1", &p);
    let j = judge(&a, &ChildStatus::Exited(0), true, &p);
    assert_eq!(j.env_violations, vec!["RCG_CANARY"]);
    assert!(j.ids.contains(&FailureId::Rcg005));
    assert!(!j.observation_pass);
}

// --------------------------------------------------------------- judge

fn judge_trace(
    trace: &str,
    status: ChildStatus,
    enforced: bool,
) -> capability_runtime_conformance::harness::Judgement {
    let p = policy(&[]);
    judge(&analyze(trace, "n1", &p), &status, enforced, &p)
}

#[test]
fn agreement_table_exit0_clean_is_a_pass() {
    let j = judge_trace(CLEAN_TRACE, ChildStatus::Exited(0), true);
    assert!(j.pass() && j.agreement && j.enforcement_pass && j.observation_pass);
    assert_eq!(j.exit_code, Some(0));
}

#[test]
fn agreement_table_exit0_with_violation_means_the_filter_had_a_hole() {
    let trace = CLEAN_TRACE.replace(
        "munmap(0x7f00, 1052672)           = 0",
        "openat(AT_FDCWD, \"/x\", O_RDONLY) = 3",
    );
    let j = judge_trace(&trace, ChildStatus::Exited(0), true);
    assert_eq!(j.ids, vec![FailureId::Rcg002, FailureId::Rcg003]);
    assert!(!j.agreement);
    // With the filter deliberately off (observe-only control) there is no
    // enforcement to disagree with.
    let j = judge_trace(&trace, ChildStatus::Exited(0), false);
    assert_eq!(j.ids, vec![FailureId::Rcg002]);
    assert!(j.agreement);
}

#[test]
fn agreement_table_sigsys_with_violation_is_enforced_rcg001() {
    let trace = "1  execve(\"/x\", [\"/x\"], []) = 0\n1  write(2, \"RCG-ARMED n1 x\\n\", 15) = 15\n1  socket(AF_INET, SOCK_STREAM, 0 <unfinished ...>\n1  +++ killed by SIGSYS +++\n";
    let j = judge_trace(trace, ChildStatus::Signaled(SIGSYS), true);
    assert_eq!(j.ids, vec![FailureId::Rcg001]);
    assert!(j.killed_by_sigsys && j.agreement && !j.enforcement_pass);
}

#[test]
fn agreement_table_sigsys_without_observed_violation_is_rcg003() {
    let trace =
        "1  execve(\"/x\", [\"/x\"], []) = 0\n1  write(2, \"RCG-ARMED n1 x\\n\", 15) = 15\n";
    let j = judge_trace(trace, ChildStatus::Signaled(SIGSYS), true);
    assert_eq!(j.ids, vec![FailureId::Rcg003]);
    assert!(!j.agreement);
}

#[test]
fn agreement_table_library_failure_and_pre_arm_failures() {
    let failing = CLEAN_TRACE.replace("exited with 0", "exited with 71");
    let j = judge_trace(&failing, ChildStatus::Exited(71), true);
    assert_eq!(j.ids, vec![FailureId::Rcg007]);
    let none =
        "1  execve(\"/x\", [\"/x\"], []) = 0\n1  exit_group(0) = ?\n1  +++ exited with 0 +++\n";
    assert_eq!(
        judge_trace(none, ChildStatus::Exited(0), true).ids,
        vec![FailureId::Rcg004]
    );
    let cases = [
        (70, FailureId::Rcg008),
        (75, FailureId::Rcg005),
        (76, FailureId::Rcg011),
        (77, FailureId::Rcg012),
    ];
    for (code, id) in cases {
        let t = format!("1  execve(\"/x\", [\"/x\"], []) = 0\n1  exit_group({code}) = ?\n1  +++ exited with {code} +++\n");
        assert_eq!(
            judge_trace(&t, ChildStatus::Exited(code), true).ids,
            vec![id],
            "exit {code}"
        );
    }
}

#[test]
fn sigsys_status_without_a_trace_epilogue_still_counts_as_killed() {
    let trace = "1  execve(\"/x\", [\"/x\"], []) = 0\n1  write(2, \"RCG-ARMED n1 x\\n\", 15) = 15\n1  bpf(BPF_MAP_CREATE, NULL, 0 <unfinished ...>\n";
    let j = judge_trace(trace, ChildStatus::Signaled(SIGSYS), true);
    assert!(j.killed_by_sigsys);
    assert_eq!(j.exit_code, None);
    assert_eq!(j.ids, vec![FailureId::Rcg001]);
}

// ------------------------------------------------------------- container

#[test]
fn docker_invocation_has_exactly_the_documented_flags_and_no_ambient_authority() {
    let p = policy(&[]);
    let args = docker_args(
        &p,
        "/tmp/p.json",
        "img:1",
        "name",
        &["/rcg/test".into(), "--rcg-child".into()],
    );
    let joined = args.join(" ");
    for flag in [
        "--network none",
        "--read-only",
        "--cap-drop ALL",
        "--security-opt no-new-privileges",
        "--security-opt seccomp=/tmp/p.json",
        "--pids-limit 16",
        "--memory 256m",
        "--user 65534:65534",
    ] {
        assert!(joined.contains(flag), "missing {flag}: {joined}");
    }
    for forbidden in [
        "-v",
        "--volume",
        "--mount",
        "-e",
        "--env",
        "--env-file",
        "--privileged",
        "--cap-add",
        "--device",
        "host",
    ] {
        assert!(
            !args.iter().any(|a| a == forbidden),
            "{forbidden} must not appear"
        );
    }
    assert_eq!(args[args.len() - 2], "/rcg/test");
    assert!(args.iter().any(|a| a == "img:1"));
}

#[test]
fn docker_network_is_enabled_only_for_net_capabilities() {
    let with_net = policy(&[("net", "connect", "example.com:443")]);
    let args = docker_args(&with_net, "/p", "i", "n", &[]);
    assert!(!args.iter().any(|a| a == "--network"));
    assert!(docker_args(&policy(&[]), "/p", "i", "n", &[])
        .iter()
        .any(|a| a == "none"));
}

#[test]
fn seccomp_profile_is_kill_by_default_with_startup_arming_and_policy_names() {
    let p = policy(&[("time", "read", "*")]);
    let profile = seccomp_profile_json(&p);
    assert!(profile.contains("\"defaultAction\": \"SCMP_ACT_KILL_PROCESS\""));
    assert!(profile.contains("SCMP_ACT_ALLOW"));
    for name in [
        "clock_gettime",
        "execve",
        "openat2",
        "landlock_restrict_self",
        "exit_group",
    ] {
        assert!(profile.contains(&format!("\"{name}\"")), "{name}");
    }
    for name in [
        "ptrace",
        "bpf",
        "io_uring_setup",
        "mount",
        "unshare",
        "setns",
    ] {
        assert!(
            !profile.contains(&format!("\"{name}\"")),
            "{name} must stay denied"
        );
    }
    assert!(ARMING_SYSCALLS.contains(&"seccomp"));
    assert!(STARTUP_SYSCALLS.contains(&"execve"));
    let aligned: Vec<&str> = profile
        .lines()
        .filter(|l| l.contains("architectures"))
        .collect();
    assert_eq!(aligned.len(), 1);
}

#[test]
fn ldd_output_is_parsed_into_absolute_paths() {
    let out = "\tlinux-vdso.so.1 (0x00007ffc)\n\tlibc.so.6 => /lib/x86_64-linux-gnu/libc.so.6 (0x00007f)\n\tlibgcc_s.so.1 => /lib/x86_64-linux-gnu/libgcc_s.so.1 (0x1)\n\t/lib64/ld-linux-x86-64.so.2 (0x00007f)\n\tlibmissing.so => not found\n";
    assert_eq!(
        parse_ldd(out),
        vec![
            "/lib/x86_64-linux-gnu/libc.so.6",
            "/lib/x86_64-linux-gnu/libgcc_s.so.1",
            "/lib64/ld-linux-x86-64.so.2"
        ]
    );
}

// ------------------------------------------------------------ json, ids

#[test]
fn json_writer_escapes_and_indents() {
    let value = Json::obj([
        ("s", Json::str("a\"b\\c\nd\te\u{1}")),
        ("n", Json::Num(7)),
        ("b", Json::Bool(true)),
        ("z", Json::Null),
        ("a", Json::Arr(vec![Json::Num(1), Json::str("x")])),
        ("e", Json::Arr(vec![])),
        ("o", Json::Obj(vec![])),
    ]);
    let text = value.pretty();
    assert!(text.contains(r#""s": "a\"b\\c\nd\te\u0001""#));
    assert!(text.contains("\"a\": [\n    1,\n    \"x\"\n  ]"));
    assert!(text.contains("\"e\": []") && text.contains("\"o\": {}"));
    assert!(text.ends_with("}\n"));
    assert_eq!(
        Json::strings(["p", "q"]),
        Json::Arr(vec![Json::str("p"), Json::str("q")])
    );
    assert!(Json::str("\r").pretty().contains("\\r"));
}

#[test]
fn failure_registry_is_complete_unique_and_documented_in_the_spec() {
    let all = FailureId::all();
    let mut seen = std::collections::BTreeSet::new();
    let spec = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../specs/VER08b-runtime-capability-conformance.md"
    ))
    .expect("spec must exist");
    for id in all {
        assert!(seen.insert(id.as_str()), "duplicate {id}");
        assert!(!id.meaning().is_empty());
        assert_eq!(id.to_string(), id.as_str());
        assert!(spec.contains(id.as_str()), "{id} is missing from VER08b");
    }
    assert_eq!(seen.len(), 14);
}

#[test]
fn spec_baseline_table_matches_the_code() {
    let spec = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../specs/VER08b-runtime-capability-conformance.md"
    ))
    .expect("spec must exist");
    for entry in BASELINE {
        assert!(
            spec.contains(&format!("| `{}` |", entry.name)),
            "baseline entry {} is missing from the spec table",
            entry.name
        );
    }
    assert!(spec.contains("rcg-baseline-1"));
}

// ------------------------------------------- recorded real strace fixtures
//
// These were recorded from the real ct-compare gate binary on Linux 6.18
// (`strace -f -v -q -s 4096`, empty environment) and are checked in so the
// classifier is tested against real strace output, not only hand-written text.

fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap_or_else(|e| panic!("{name}: {e}"))
}

#[test]
fn recorded_clean_ct_compare_trace_is_exactly_the_baseline_subset() {
    let p = policy(&[]);
    let a = analyze(&fixture("ct_compare_clean.strace"), "fixture-clean", &p);
    assert!(a.armed, "{a:?}");
    assert_eq!(a.marker_detail, "landlock=1 abi=7 seccomp=1");
    assert!(a.violations.is_empty(), "{:?}", a.violations);
    assert!(a.pre_arm_calls > 50, "setup is traced but is not evidence");
    let observed: Vec<&str> = a.post_arm.keys().map(String::as_str).collect();
    assert_eq!(observed, ["exit_group", "futex", "mmap", "munmap", "write"]);
    assert_eq!(a.exec_env, Some(vec![]), "empty launch environment");
    assert_eq!(a.ending, Ending::Exited(0));
    // Observed set is inside the baseline: adding an undeclared syscall to
    // the library would make this fail before the gate does.
    for name in a.post_arm.keys() {
        assert!(
            BASELINE.iter().any(|b| b.name == name),
            "{name} not in baseline"
        );
    }
    let j = judge(&a, &ChildStatus::Exited(0), true, &p);
    assert!(j.pass() && j.agreement);
}

#[test]
fn recorded_enforced_sabotage_trace_shows_the_attempt_then_sigsys() {
    let p = policy(&[]);
    let a = analyze(
        &fixture("sabotage_fs_open_enforced.strace"),
        "fixture-open",
        &p,
    );
    assert!(a.armed);
    assert_eq!(a.violations.len(), 1);
    assert_eq!(a.violations[0].name, "openat");
    assert_eq!(a.violations[0].family, "fs");
    assert!(a.violations[0].args.contains("/etc/hostname"));
    assert_eq!(a.ending, Ending::Killed("SIGSYS".into()));
    let j = judge(&a, &ChildStatus::Signaled(SIGSYS), true, &p);
    assert_eq!(j.ids, vec![FailureId::Rcg001]);
    assert!(j.agreement);
}

#[test]
fn recorded_observe_only_spawn_trace_is_flagged_without_any_kill() {
    let p = policy(&[]);
    let trace = fixture("sabotage_proc_spawn_observe_only.strace");
    let a = analyze(&trace, "fixture-spawn", &p);
    assert!(a.armed);
    assert_eq!(
        a.marker_detail, "landlock=0 abi=7 seccomp=0",
        "filter was off"
    );
    let names: Vec<&str> = a.violations.iter().map(|v| v.name.as_str()).collect();
    assert!(
        names.contains(&"clone3") && names.contains(&"execve"),
        "{names:?}"
    );
    assert!(a.violations.iter().any(|v| v.family == "proc"));
    assert_eq!(a.ending, Ending::Exited(0));
    let j = judge(&a, &ChildStatus::Exited(0), false, &p);
    assert_eq!(j.ids, vec![FailureId::Rcg002]);
    assert!(!j.killed_by_sigsys);
    // Declaring proc:exec explains execve but not clone3: the mapping is
    // deliberately exact, so an undeclared call is still flagged.
    let exec = policy(&[("proc", "exec", "true")]);
    let remaining = analyze(&trace, "fixture-spawn", &exec);
    assert!(remaining.violations.iter().any(|v| v.name == "clone3"));
    assert!(!remaining.violations.iter().any(|v| v.name == "execve"));
}
