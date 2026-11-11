//! Manifest -> sandbox plan -> runtime policy (VER08b sections 3 and 4).
//!
//! The policy answers one question for the whole gate: *which syscalls may a
//! process make after it is armed?* Everything else (the BPF filter, the
//! strace classifier, the Docker profile) is a different rendering of this one
//! answer, which is why the three can be cross-checked against each other.
//!
//! ```text
//!   required_capabilities.json
//!        |  capability_os_sandbox::plan_from_json(.., Linux)
//!        v
//!   SandboxPlan  --launch_preconditions()-->  refuse if unenforceable (RCG006)
//!        |
//!        v
//!   RuntimePolicy = BASELINE  U  allowance(cap) for every declared cap
//! ```
//!
//! The policy is deny-by-default: an empty manifest yields exactly
//! [`BASELINE`], and nothing a capability does not name is ever allowed.

use crate::failure::FailureId;
use capability_cage::{Action, Category};
use capability_os_sandbox::{plan_from_json, OsFamily, PlanRejection, SandboxPlan};
use coding_adventures_sha256::sha256_hex;
use std::collections::BTreeMap;
use std::fmt;

/// Version string recorded in every evidence file. Changing [`BASELINE`]
/// requires bumping it, so evidence from two baselines can never be confused.
pub const BASELINE_VERSION: &str = "rcg-baseline-1";

/// A condition on a syscall's arguments. seccomp-BPF can test the six
/// integer arguments (not what they point at), and strace text exposes the
/// same integers, so one predicate is checked identically by both rings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pred {
    /// Allowed with any arguments.
    Any,
    /// The (low 32 bits of the) argument at this index must be one of these
    /// values. Used for file descriptors (`write` fd 1/2) and for the futex
    /// operation code (wake-only).
    ArgIn(usize, &'static [u32]),
    /// The `prot` argument at this index must not contain `PROT_EXEC`.
    NoExecProt(usize),
}

impl Pred {
    /// Combine two allowances of the same syscall: the broader one wins.
    fn union(self, other: Pred) -> Pred {
        match (self, other) {
            (Pred::Any, _) | (_, Pred::Any) => Pred::Any,
            (Pred::ArgIn(i, a), Pred::ArgIn(j, b)) if i == j => {
                // Static slices cannot be concatenated; the only users are
                // [1,2] (baseline) and [0] (stdin) / [1] (stdout). If one
                // covers the other keep it; otherwise widen to Any.
                if a.iter().all(|v| b.contains(v)) {
                    Pred::ArgIn(i, b)
                } else if b.iter().all(|v| a.contains(v)) {
                    Pred::ArgIn(i, a)
                } else {
                    Pred::Any
                }
            }
            (Pred::ArgIn(..), Pred::ArgIn(..)) => Pred::Any,
            (a, _) => a,
        }
    }
}

/// One baseline entry: syscall, argument predicate, and why it is allowed.
pub struct BaselineEntry {
    /// Kernel syscall name (as strace prints it).
    pub name: &'static str,
    /// Argument constraint.
    pub pred: Pred,
    /// Why every armed process may make this call.
    pub why: &'static str,
}

/// The minimal armed-phase baseline (VER08b section 4). Each line is
/// justified; nothing is here "just in case".
pub const BASELINE: &[BaselineEntry] = &[
    BaselineEntry {
        name: "write",
        pred: Pred::ArgIn(0, &[1, 2]),
        why: "TAP result lines on fd 1; ARMED marker and panic text on fd 2; no other fd",
    },
    BaselineEntry {
        name: "exit_group",
        pred: Pred::Any,
        why: "process termination (the harness leaves through _exit, which is exit_group); without it a process cannot end",
    },
    BaselineEntry {
        name: "brk",
        pred: Pred::Any,
        why: "glibc malloc main-arena growth/trim: ct_select_bytes allocates a Vec on every call",
    },
    BaselineEntry {
        name: "mmap",
        pred: Pred::NoExecProt(2),
        why: "allocations above the mmap threshold (the 1 MiB select vector exercises this); PROT_EXEC forbidden (W^X: executable maps are code loading)",
    },
    BaselineEntry {
        name: "futex",
        pred: Pred::ArgIn(1, &[1, 129]),
        why: "wake-only (FUTEX_WAKE[_PRIVATE]): glibc's dynamic-loader lock release during the first panic unwind (ct_select_bytes panics by contract); a wake can never block or wait on anything in a single-threaded process",
    },
    BaselineEntry {
        name: "munmap",
        pred: Pred::Any,
        why: "returning mmap-backed allocations (observed with the 1 MiB select vector)",
    },
];

/// Syscalls that no capability ever allows (VER08b section 3.2). They are
/// listed so the classifier can label a violation, and so a test can prove
/// that no capability's allowance contains one.
pub const NEVER_ALLOWED: &[&str] = &[
    "ptrace",
    "bpf",
    "io_uring_setup",
    "io_uring_enter",
    "io_uring_register",
    "mount",
    "umount2",
    "pivot_root",
    "chroot",
    "setns",
    "unshare",
    "perf_event_open",
    "keyctl",
    "add_key",
    "request_key",
    "init_module",
    "finit_module",
    "kexec_load",
    "reboot",
    "seccomp",
    "prctl",
    "landlock_create_ruleset",
    "landlock_add_rule",
    "landlock_restrict_self",
    "mprotect_exec",
];

const FS_OPEN: &[&str] = &[
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
];
const FS_READ: &[&str] = &[
    "read",
    "pread64",
    "readv",
    "preadv",
    "access",
    "faccessat",
    "faccessat2",
    "readlink",
    "readlinkat",
];
const FS_WRITE: &[&str] = &[
    "pwrite64",
    "writev",
    "pwritev",
    "ftruncate",
    "fsync",
    "fdatasync",
];
const FS_CREATE: &[&str] = &["creat", "mkdir", "mkdirat"];
const FS_DELETE: &[&str] = &["unlink", "unlinkat", "rmdir"];
const FS_LIST: &[&str] = &["getdents", "getdents64"];
const NET_COMMON: &[&str] = &[
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
const NET_CONNECT: &[&str] = &["connect", "sendto", "recvfrom", "sendmsg", "recvmsg"];
const NET_LISTEN: &[&str] = &["bind", "listen", "accept", "accept4"];
const DNS_FILES: &[&str] = &["/etc/resolv.conf", "/etc/hosts", "/etc/nsswitch.conf"];
const PROC_EXEC: &[&str] = &[
    "execve", "execveat", "wait4", "waitid", "pipe", "pipe2", "dup", "dup2", "dup3", "clone",
    "vfork",
];
const PROC_FORK: &[&str] = &["fork", "vfork", "clone", "clone3", "wait4", "waitid"];
const PROC_SIGNAL: &[&str] = &["kill", "tgkill", "tkill", "rt_sigqueueinfo"];
const TIME_READ: &[&str] = &["clock_gettime", "clock_getres", "gettimeofday", "time"];
const TIME_SLEEP: &[&str] = &["nanosleep", "clock_nanosleep"];
const THREAD: &[&str] = &[
    "clone",
    "clone3",
    "futex",
    "set_robust_list",
    "rseq",
    "sched_yield",
    "sigaltstack",
    "rt_sigprocmask",
    "sched_getaffinity",
];

/// Coarse family label of a syscall, for human-readable violation reports.
/// Unknown names are labelled `unclassified` (and are violations anyway).
pub fn family_of(name: &str) -> &'static str {
    let groups: [(&str, &[&[&str]]); 8] = [
        (
            "fs",
            &[FS_OPEN, FS_READ, FS_WRITE, FS_CREATE, FS_DELETE, FS_LIST],
        ),
        ("net", &[NET_COMMON, NET_CONNECT, NET_LISTEN]),
        ("proc", &[PROC_EXEC, PROC_FORK, PROC_SIGNAL]),
        ("time", &[TIME_READ, TIME_SLEEP]),
        ("thread", &[THREAD]),
        ("rand", &[&["getrandom"]]),
        ("never-allowed", &[NEVER_ALLOWED]),
        (
            "baseline",
            &[&["write", "exit_group", "exit", "brk", "mmap", "munmap"]],
        ),
    ];
    for (family, sets) in groups {
        if sets.iter().any(|set| set.contains(&name)) {
            return family;
        }
    }
    "unclassified"
}

/// Why a manifest cannot be certified by the runtime gate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    /// The failure id to report.
    pub id: FailureId,
    /// Human-readable reasons, one per offending rule or error.
    pub reasons: Vec<String>,
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.id, self.reasons.join("; "))
    }
}

/// Options for [`RuntimePolicy::from_manifest_json`].
#[derive(Debug, Clone, Copy, Default)]
pub struct PolicyOptions {
    /// Accept `thread:*` capabilities with the coarse mapping of section 3.1.
    /// The plan marks them `Unsupported`, so the default is to refuse.
    pub allow_coarse_thread: bool,
}

/// The armed-phase policy derived from one manifest.
#[derive(Debug, Clone)]
pub struct RuntimePolicy {
    /// `rust/ct-compare`-style package label from the manifest.
    pub package: String,
    /// SHA-256 of the manifest text exactly as supplied.
    pub manifest_sha256: String,
    /// SHA-256 of the canonical plan rendering.
    pub plan_sha256: String,
    /// [`BASELINE_VERSION`].
    pub baseline_version: &'static str,
    /// `category:action:target` for every declared capability.
    pub declared: Vec<String>,
    /// Allowed syscalls and their argument predicates.
    pub allow: BTreeMap<&'static str, Pred>,
    /// Which capability (or `baseline`) allowed each syscall.
    pub reasons: BTreeMap<&'static str, Vec<String>>,
    /// Landlock rules: (absolute path, rights mask names).
    pub landlock_paths: Vec<(String, Vec<&'static str>)>,
    /// Environment variable names the manifest permits (`*` = any).
    pub env_allowed: Vec<String>,
    /// True if any `net` capability is declared (container needs a network).
    pub needs_network: bool,
    /// True if fs scoping needs Landlock (any `fs` capability).
    pub landlock_required: bool,
}

impl RuntimePolicy {
    /// Build the policy for a manifest: lower to a Linux plan, refuse what
    /// cannot be enforced, then map each capability to its allowance.
    pub fn from_manifest_json(
        manifest_json: &str,
        options: PolicyOptions,
    ) -> Result<RuntimePolicy, Refusal> {
        let plan = plan_from_json(manifest_json, OsFamily::Linux).map_err(|error| Refusal {
            id: FailureId::Rcg006,
            reasons: vec![format!(
                "manifest rejected by capability-os-sandbox: {error}"
            )],
        })?;
        Self::from_plan(&plan, &sha256_hex(manifest_json.as_bytes()), options)
    }

    /// Build the policy from an already-lowered plan.
    pub fn from_plan(
        plan: &SandboxPlan,
        manifest_sha256: &str,
        options: PolicyOptions,
    ) -> Result<RuntimePolicy, Refusal> {
        if let Err(rejections) = plan.launch_preconditions() {
            let blocking: Vec<String> = rejections
                .iter()
                .filter(|r| {
                    !(options.allow_coarse_thread
                        && matches!(r, PlanRejection::UnsupportedRule(k) if k.starts_with("thread:")))
                })
                .map(ToString::to_string)
                .collect();
            if !blocking.is_empty() {
                return Err(Refusal {
                    id: FailureId::Rcg006,
                    reasons: blocking,
                });
            }
        }

        let mut allow: BTreeMap<&'static str, Pred> = BTreeMap::new();
        let mut reasons: BTreeMap<&'static str, Vec<String>> = BTreeMap::new();
        let mut note = |allow: &mut BTreeMap<&'static str, Pred>,
                        name: &'static str,
                        pred: Pred,
                        who: &str| {
            let merged = match allow.get(name) {
                Some(existing) => existing.union(pred),
                None => pred,
            };
            allow.insert(name, merged);
            reasons.entry(name).or_default().push(who.to_string());
        };
        for entry in BASELINE {
            note(&mut allow, entry.name, entry.pred, "baseline");
        }

        let mut declared = Vec::new();
        let mut landlock_paths: Vec<(String, Vec<&'static str>)> = Vec::new();
        let mut env_allowed = Vec::new();
        let mut needs_network = false;
        let mut landlock_required = false;

        for rule in &plan.rules {
            let cap = &rule.capability;
            let key = rule.capability_key();
            declared.push(key.clone());
            let mut add =
                |allow: &mut BTreeMap<&'static str, Pred>, names: &[&'static str], pred: Pred| {
                    for name in names {
                        note(allow, name, pred, &key);
                    }
                };
            match (cap.category, cap.action) {
                (Category::Fs, action) => {
                    landlock_required = true;
                    let (names, rights): (Vec<&'static str>, Vec<&'static str>) = match action {
                        Action::Read => ([FS_OPEN, FS_READ].concat(), vec!["READ_FILE"]),
                        Action::Write => (
                            [FS_OPEN, FS_WRITE, &["write"]].concat(),
                            vec!["WRITE_FILE", "TRUNCATE"],
                        ),
                        Action::Create => {
                            ([FS_OPEN, FS_CREATE].concat(), vec!["MAKE_REG", "MAKE_DIR"])
                        }
                        Action::Delete => (FS_DELETE.to_vec(), vec!["REMOVE_FILE", "REMOVE_DIR"]),
                        _ => (FS_LIST.to_vec(), vec!["READ_DIR"]),
                    };
                    add(&mut allow, &names, Pred::Any);
                    let path = fs_landlock_path(&cap.target)?;
                    landlock_paths.push((path, rights));
                }
                (Category::Net, action) => {
                    needs_network = true;
                    add(&mut allow, NET_COMMON, Pred::Any);
                    match action {
                        Action::Listen => add(&mut allow, NET_LISTEN, Pred::Any),
                        Action::Dns => {
                            add(&mut allow, NET_CONNECT, Pred::Any);
                            add(&mut allow, &[FS_OPEN, FS_READ].concat(), Pred::Any);
                            landlock_required = true;
                            for file in DNS_FILES {
                                landlock_paths.push((file.to_string(), vec!["READ_FILE"]));
                            }
                        }
                        _ => add(&mut allow, NET_CONNECT, Pred::Any),
                    }
                }
                (Category::Proc, Action::Exec) => add(&mut allow, PROC_EXEC, Pred::Any),
                (Category::Proc, Action::Fork) => add(&mut allow, PROC_FORK, Pred::Any),
                (Category::Proc, _) => add(&mut allow, PROC_SIGNAL, Pred::Any),
                (Category::Env, _) => env_allowed.push(cap.target.clone()),
                (Category::Time, Action::Sleep) => {
                    add(&mut allow, TIME_READ, Pred::Any);
                    add(&mut allow, TIME_SLEEP, Pred::Any);
                }
                (Category::Time, _) => add(&mut allow, TIME_READ, Pred::Any),
                (Category::Stdin, _) => add(&mut allow, &["read"], Pred::ArgIn(0, &[0])),
                (Category::Stdout, _) => add(&mut allow, &["write"], Pred::ArgIn(0, &[1])),
                (Category::Rand, _) => add(&mut allow, &["getrandom"], Pred::Any),
                (Category::Thread, _) => {
                    add(&mut allow, THREAD, Pred::Any);
                    add(&mut allow, &["mprotect"], Pred::NoExecProt(2));
                }
                // The plan refuses these before we get here (RCG006); keep the
                // arm explicit so a new Category forces a decision.
                (Category::Ffi, _) | (Category::Ui, _) => {
                    return Err(Refusal {
                        id: FailureId::Rcg006,
                        reasons: vec![format!("{key} cannot be enforced at runtime")],
                    })
                }
            }
        }

        Ok(RuntimePolicy {
            package: plan.package.clone(),
            manifest_sha256: manifest_sha256.to_string(),
            plan_sha256: plan_digest(plan),
            baseline_version: BASELINE_VERSION,
            declared,
            allow,
            reasons,
            landlock_paths,
            env_allowed,
            needs_network,
            landlock_required,
        })
    }

    /// True if `name` may be called with the argument text `args` (strace
    /// style, e.g. `1, "ok\n", 3`). Used by the observer; the BPF filter
    /// implements the same predicates on raw argument registers.
    pub fn allows(&self, name: &str, args: &str) -> bool {
        match self.allow.get(name) {
            None => false,
            Some(Pred::Any) => true,
            Some(Pred::ArgIn(index, values)) => args
                .split(',')
                .nth(*index)
                .and_then(token_value)
                .is_some_and(|v| values.contains(&v)),
            Some(Pred::NoExecProt(index)) => {
                let prot = args.split(',').nth(*index).unwrap_or("");
                !prot.contains("PROT_EXEC")
            }
        }
    }

    /// Environment variable names permitted by the manifest.
    pub fn env_name_allowed(&self, name: &str) -> bool {
        self.env_allowed.iter().any(|a| a == "*" || a == name)
    }
}

/// Value of one strace argument token: a decimal integer, or one of the few
/// symbolic names the predicates compare against (strace prints futex ops
/// symbolically).
fn token_value(token: &str) -> Option<u32> {
    let token = token.trim();
    let digits: String = token.chars().take_while(char::is_ascii_digit).collect();
    if !digits.is_empty() && digits.len() == token.len() {
        return digits.parse().ok();
    }
    match token {
        "FUTEX_WAIT" => Some(0),
        "FUTEX_WAKE" => Some(1),
        "FUTEX_WAIT_PRIVATE" => Some(128),
        "FUTEX_WAKE_PRIVATE" => Some(129),
        _ => None,
    }
}

/// Landlock needs a concrete path. Exact absolute files are used as-is, `*`
/// means "anywhere" (rule on `/`), and anything else (relative, globbed)
/// cannot be scoped by Landlock, so the gate refuses rather than guess.
fn fs_landlock_path(target: &str) -> Result<String, Refusal> {
    if target == "*" {
        return Ok("/".to_string());
    }
    let exact = target.starts_with('/') && !target.contains(['*', '?', '[', '~']);
    if exact {
        Ok(target.to_string())
    } else {
        Err(Refusal {
            id: FailureId::Rcg006,
            reasons: vec![format!(
                "fs target `{target}` is not an absolute exact path or `*`; Landlock cannot scope it"
            )],
        })
    }
}

/// Canonical text of a plan, hashed for the evidence file. Built by hand
/// (not `{:?}`) so the digest does not change when a `Debug` derive does.
pub fn plan_digest(plan: &SandboxPlan) -> String {
    let mut text = format!("package={}\nos={}\n", plan.package, plan.os);
    for primitive in &plan.base.primitives {
        text.push_str(&format!("base={primitive}\n"));
    }
    for rule in &plan.rules {
        text.push_str(&format!(
            "rule={}|{}|{}|{}\n",
            rule.capability_key(),
            rule.primitive,
            rule.coverage,
            rule.os
        ));
    }
    sha256_hex(text.as_bytes())
}
