//! The container ring (VER08b section 7): the same manifest-derived policy,
//! rendered as a Docker invocation and a seccomp profile.
//!
//! ```text
//!   ring 0  container   starts with the filter: needs STARTUP_SYSCALLS too
//!   ring 1  in-process  armed after startup: baseline only (much smaller)
//!   ring 2  observer    strace on the host
//! ```
//!
//! The container filter applies from the container init's first instruction,
//! so it must include everything `runc` and the dynamic loader do before
//! `main`. That is why it is looser than ring 1; it proves "the container
//! holds exactly these permissions" at the container boundary, while ring 1
//! proves what the library did inside it.

use crate::policy::RuntimePolicy;
use std::collections::BTreeSet;

/// Syscalls needed between container creation and `main` of a glibc-linked
/// Rust program (runc init, `execve`, the loader, libc and Rust runtime
/// start-up). Measured with `strace -f` on the pilot and on `runc init`; kept
/// as one explicit list so it can be reviewed.
pub const STARTUP_SYSCALLS: &[&str] = &[
    // runc init -> exec (openat2: runc's safe /proc access; found by bisecting
    // a KILL_PROCESS profile against Docker 29 / runc, see the spec)
    "openat2",
    "capget",
    "capset",
    "chdir",
    "fchdir",
    "fchown",
    "fchmod",
    "setuid",
    "setgid",
    "setgroups",
    "prctl",
    "close",
    "close_range",
    "dup3",
    "fcntl",
    "getpid",
    "gettid",
    "getppid",
    "getuid",
    "geteuid",
    "getgid",
    "getegid",
    "epoll_create1",
    "epoll_ctl",
    "epoll_pwait",
    "epoll_wait",
    "eventfd2",
    "pipe2",
    "futex",
    "sched_yield",
    "sched_getaffinity",
    "tgkill",
    "clone",
    "clone3",
    "wait4",
    "waitid",
    "nanosleep",
    "clock_nanosleep",
    "clock_gettime",
    "getcwd",
    "readlinkat",
    "readlink",
    "ioctl",
    "umask",
    "uname",
    "socket",
    "connect",
    "recvmsg",
    "sendmsg",
    "sendto",
    "recvfrom",
    "getsockopt",
    "setsockopt",
    "getsockname",
    "poll",
    "ppoll",
    "select",
    "pselect6",
    "unlinkat",
    "mkdirat",
    "symlinkat",
    "linkat",
    "renameat",
    "renameat2",
    "statfs",
    "fstatfs",
    "getdents64",
    "flock",
    "lseek",
    "pread64",
    "pwrite64",
    "readv",
    "writev",
    "dup",
    "dup2",
    "pipe",
    "kill",
    "tkill",
    "sigaltstack",
    "rt_sigaction",
    "rt_sigprocmask",
    "rt_sigsuspend",
    "rt_sigtimedwait",
    "restart_syscall",
    "setrlimit",
    "prlimit64",
    "getrlimit",
    "sysinfo",
    "madvise",
    "mlock",
    "msync",
    "capget",
    "keyctl",
    "setsid",
    "setpgid",
    "getpgid",
    // execve and the loader
    "execve",
    "execveat",
    "arch_prctl",
    "set_tid_address",
    "set_robust_list",
    "rseq",
    "openat",
    "open",
    "read",
    "write",
    "fstat",
    "newfstatat",
    "stat",
    "lstat",
    "statx",
    "access",
    "faccessat",
    "faccessat2",
    "mmap",
    "mprotect",
    "munmap",
    "mremap",
    "brk",
    "getrandom",
    "exit",
    "exit_group",
    "rt_sigreturn",
];

/// The calls the test binary itself makes in phase A to install ring 1.
/// Ring 0 must permit them (ring 1 is installed from inside the container),
/// and ring 1 then forbids them again for everything after arming.
pub const ARMING_SYSCALLS: &[&str] = &[
    "prctl",
    "seccomp",
    "landlock_create_ruleset",
    "landlock_add_rule",
    "landlock_restrict_self",
];

/// Docker's architecture name for the host.
fn docker_arch() -> &'static str {
    if cfg!(target_arch = "aarch64") {
        "SCMP_ARCH_AARCH64"
    } else {
        "SCMP_ARCH_X86_64"
    }
}

/// The Docker seccomp profile JSON: KILL by default, allow startup set ∪
/// baseline ∪ declared. Argument predicates are left to ring 1.
pub fn seccomp_profile_json(policy: &RuntimePolicy) -> String {
    let mut names: BTreeSet<&str> = STARTUP_SYSCALLS.iter().copied().collect();
    names.extend(ARMING_SYSCALLS.iter().copied());
    names.extend(policy.allow.keys().copied());
    let list = names
        .iter()
        .map(|n| format!("\"{n}\""))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "{{\n  \"defaultAction\": \"SCMP_ACT_KILL_PROCESS\",\n  \"architectures\": [\"{}\"],\n  \"syscalls\": [\n    {{ \"names\": [{}], \"action\": \"SCMP_ACT_ALLOW\" }}\n  ]\n}}\n",
        docker_arch(),
        list
    )
}

/// The `docker run` argument vector for `policy` (without the leading
/// `docker`). `profile_path` is the generated profile, `image` the imported
/// image, `command` the program and its arguments inside the container.
///
/// Notably absent: `-v`, `--mount`, `-e`, `--env-file`, `--privileged`,
/// `--cap-add`, `--device`, `--pid host`, `--network host`.
pub fn docker_args(
    policy: &RuntimePolicy,
    profile_path: &str,
    image: &str,
    name: &str,
    command: &[String],
) -> Vec<String> {
    let mut args: Vec<String> = ["run", "--rm", "--name", name]
        .iter()
        .map(ToString::to_string)
        .collect();
    if !policy.needs_network {
        args.push("--network".into());
        args.push("none".into());
    }
    for flag in [
        "--read-only",
        "--cap-drop",
        "ALL",
        "--security-opt",
        "no-new-privileges",
        "--security-opt",
    ] {
        args.push(flag.to_string());
    }
    args.push(format!("seccomp={profile_path}"));
    for flag in [
        "--pids-limit",
        "16",
        "--memory",
        "256m",
        "--user",
        "65534:65534",
    ] {
        args.push(flag.to_string());
    }
    args.push(image.to_string());
    args.extend(command.iter().cloned());
    args
}

/// Parse `ldd` output into the absolute paths of the shared objects a binary
/// needs (the vDSO has no path and is skipped).
pub fn parse_ldd(output: &str) -> Vec<String> {
    let mut paths = Vec::new();
    for line in output.lines() {
        let line = line.trim();
        let candidate = if let Some((_, rhs)) = line.split_once("=>") {
            rhs.split_whitespace().next()
        } else {
            line.split_whitespace().next()
        };
        if let Some(path) = candidate {
            if path.starts_with('/') {
                paths.push(path.to_string());
            }
        }
    }
    paths.sort();
    paths.dedup();
    paths
}
