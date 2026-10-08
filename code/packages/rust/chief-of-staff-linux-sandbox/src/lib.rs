//! # The Linux applier (D18S build step 4)
//!
//! `capability-os-sandbox` says what an agent may do. This crate makes the
//! Linux kernel enforce it, for a compiled agent, from the instant it
//! exists:
//!
//! ```text
//!   parent (supervisor)                    child (after fork, before exec)
//!   ----------------------------------     ---------------------------------
//!   LinuxConfinement::prepare(plan, exe)
//!     opens exe O_RDONLY (fd B)
//!     reads the Landlock ABI                 spawn-isolation:
//!     builds the Landlock ruleset (an fd)      no terminal, setsid,
//!     builds the seccomp program (a Vec)       fds above 2 close-on-exec
//!   apply(&mut command)                      this crate, in order:
//!     builds argv, registers the hooks         1. PR_SET_NO_NEW_PRIVS
//!   command.spawn()  ──── fork ────────►       2. landlock_restrict_self
//!                                              3. seccomp(SET_MODE_FILTER)
//!                                              4. execveat(B, "", AT_EMPTY_PATH)
//!                                            the agent's first instruction:
//!                                            already confined
//! ```
//!
//! Everything that can fail or allocate happens in the parent. The child
//! makes four syscalls and checks each one; a failure refuses the spawn
//! (S-P3), so an agent never runs half-confined.
//!
//! The exec is the hook's own, by descriptor, not `std`'s by path. Filters
//! survive exec, so whatever exec the filter allows stays allowed to the
//! agent (S-I4d). It allows only `execveat` on descriptor B with
//! `AT_EMPTY_PATH`, and B, being close-on-exec, is gone once the agent runs.
//! It also means the file that runs is the one that was parsed and given its
//! Landlock rule, even if its path is replaced in between.
//!
//! The child is single-threaded at that point, which is what S-I4b
//! requires: a seccomp filter or a Landlock domain applies only to the
//! thread that installs it and to what it creates afterwards. A compiled
//! agent therefore gets deny-all at `exec` (S-I4c). An interpreted runtime
//! needs the shim (D18S build step 5); that is not this crate.
//!
//! ## Landlock: which paths exist at all
//!
//! The ruleset *handles* every filesystem right the running ABI knows, so
//! anything without a rule is denied. From ABI 4 it also handles TCP bind and
//! connect, and from ABI 6 abstract-unix and signal scoping, with no rules:
//! denied too. The rules are:
//!
//! ```text
//!   path                                    rights
//!   --------------------------------------  ------------------------------
//!   the agent executable                    read, execute
//!   its ELF interpreter (PT_INTERP)         read, execute
//!   /lib /lib64 /usr/lib /usr/lib64         read files, list (no execute)
//!   /etc/ld.so.cache                        read
//!   /dev/null                               read, write
//!   /dev/urandom                            read
//!   each Direct fs read/write grant         read, or write (+ truncate)
//! ```
//!
//! `/proc`, `/sys`, the rest of `/dev`, the vault and every other path
//! cannot be opened by the agent (S-I1).
//!
//! A grant must name an existing regular file, reached through no symlink
//! (`openat2(RESOLVE_NO_SYMLINKS)`), and not the agent's own executable or
//! interpreter. A directory would grant its whole tree, a symlink whatever
//! it points at, and a writable image would let the agent rewrite its code
//! (S-I6). The rest of S-I6's never-grantable set (the vault, the audit log,
//! the shim, ...) is the supervisor's to check: only it knows those paths.
//!
//! The interpreter is named by the executable itself (`PT_INTERP`), so by
//! the agent's author. It is resolved, must be a loader (`ld-*.so*`) in a
//! library directory, and is then opened as exactly as a grant; anything
//! else refuses the launch. No write grant may land in a library directory.
//!
//! Landlock mediates opening, not looking: `stat` and `access` on any path
//! still answer, so an agent can learn whether a file exists and its size.
//! It cannot read it.
//!
//! ## seccomp: which syscalls exist at all
//!
//! The program checks the architecture first, then allows a fixed list of
//! syscall numbers, and kills the process for everything else
//! (`SECCOMP_RET_KILL_PROCESS`, S-P1). A few allowed syscalls are only
//! allowed with particular arguments:
//!
//! ```text
//!   clone      only with CLONE_THREAD: threads yes, processes no
//!   clone3     ENOSYS, so libc falls back to clone, whose flags are visible
//!   ioctl      never TIOCSTI or TIOCLINUX (S-I2)
//!   prctl      only PR_SET_NAME / PR_GET_NAME (thread names)
//!   prlimit64  only on the calling process (pid 0)
//!   readlink   EACCES: Landlock does not mediate it, and through
//!              /proc/<supervisor>/fd it would name the supervisor's files
//!   execve     never: the agent's own exec is the execveat below
//!   execveat   only on the prepared descriptor, with AT_EMPTY_PATH
//! ```
//!
//! Not on the list, so a kill: `socket`, `socketpair`, `io_uring_*`,
//! `ptrace`, `kill` and its relatives, SysV and POSIX IPC, `bpf`,
//! `perf_event_open`, `userfaultfd`, `mount`, `unshare`, `setns`, the
//! `pidfd` family, `fork`, `vfork`, and every syscall multiplexer: all of
//! S-I1's list.

#![deny(unsafe_op_in_unsafe_fn)]

use capability_cage::{Action, Category};
use capability_os_sandbox::{OsFamily, PlanRejection, SandboxCoverage, SandboxPlan};
use std::fmt;
use std::path::Path;
use std::process::Command;

#[cfg(target_os = "linux")]
mod linux;

/// Why an agent cannot be confined. Every one of these refuses the launch
/// (S-P3): there is no "confine what we can" mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfinementError {
    /// Not Linux, or an architecture the seccomp program is not built for.
    Unsupported(&'static str),
    /// The plan itself may not launch.
    Plan(Vec<PlanRejection>),
    /// The plan is for another OS.
    NotLinuxPlan,
    /// The kernel has no usable Landlock.
    LandlockUnavailable,
    /// A `Direct` filesystem grant needs a newer Landlock ABI (S-P1: no
    /// direct filesystem grant below ABI 3, where truncation is mediated).
    LandlockTooOld { abi: u32, needed: u32 },
    /// A `Direct` grant this applier cannot express exactly, such as create
    /// or delete, which Landlock can only grant over a whole directory.
    InexpressibleGrant(String),
    /// A path the ruleset needs could not be opened.
    Path(String),
    /// The executable is not an ELF file this applier can read.
    Executable(String),
    /// A Landlock call failed while building the ruleset.
    Landlock(String),
}

impl fmt::Display for ConfinementError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported(what) => write!(f, "linux confinement unsupported: {what}"),
            Self::Plan(rejections) => {
                write!(
                    f,
                    "sandbox plan cannot launch: {} reason(s)",
                    rejections.len()
                )
            }
            Self::NotLinuxPlan => f.write_str("sandbox plan is not for Linux"),
            Self::LandlockUnavailable => f.write_str("Landlock is not available"),
            Self::LandlockTooOld { abi, needed } => {
                write!(
                    f,
                    "Landlock ABI {abi} is below the {needed} this plan needs"
                )
            }
            Self::InexpressibleGrant(key) => {
                write!(f, "Landlock cannot express this grant exactly: {key}")
            }
            Self::Path(path) => write!(f, "cannot open a path the ruleset needs: {path}"),
            Self::Executable(why) => write!(f, "agent executable unreadable: {why}"),
            Self::Landlock(why) => write!(f, "Landlock ruleset failed: {why}"),
        }
    }
}

impl std::error::Error for ConfinementError {}

/// A filesystem grant the ruleset will carry, taken from a `Direct` plan
/// rule: one exact path, read or write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileGrant {
    pub path: String,
    pub write: bool,
}

/// The plan's `Direct` filesystem grants, each one an exact path read or
/// written. Anything else `Direct` is refused: today only filesystem rules
/// can be `Direct` (capability-os-sandbox), and create, delete and list
/// would need rights over a whole directory.
pub fn direct_file_grants(plan: &SandboxPlan) -> Result<Vec<FileGrant>, ConfinementError> {
    let mut grants = Vec::new();
    for rule in &plan.rules {
        if rule.coverage != SandboxCoverage::Direct {
            continue;
        }
        let capability = &rule.capability;
        let write = match (capability.category, capability.action) {
            (Category::Fs, Action::Read) => false,
            (Category::Fs, Action::Write) => true,
            _ => return Err(ConfinementError::InexpressibleGrant(rule.capability_key())),
        };
        grants.push(FileGrant {
            path: capability.target.clone(),
            write,
        });
    }
    Ok(grants)
}

/// A prepared confinement: the Landlock ruleset and the seccomp program,
/// built in the parent, ready to be installed in each spawned child.
pub struct LinuxConfinement {
    #[cfg(target_os = "linux")]
    prepared: std::sync::Arc<linux::Prepared>,
    landlock_abi: u32,
}

impl LinuxConfinement {
    /// Check `plan`, then build the ruleset and the program for running
    /// `executable` under it.
    pub fn prepare(plan: &SandboxPlan, executable: &Path) -> Result<Self, ConfinementError> {
        if plan.os != OsFamily::Linux {
            return Err(ConfinementError::NotLinuxPlan);
        }
        plan.launch_preconditions()
            .map_err(ConfinementError::Plan)?;
        let grants = direct_file_grants(plan)?;
        Self::prepare_platform(executable, &grants)
    }

    #[cfg(target_os = "linux")]
    fn prepare_platform(executable: &Path, grants: &[FileGrant]) -> Result<Self, ConfinementError> {
        let prepared = linux::Prepared::build(executable, grants)?;
        Ok(Self {
            landlock_abi: prepared.landlock_abi,
            prepared: std::sync::Arc::new(prepared),
        })
    }

    #[cfg(not(target_os = "linux"))]
    fn prepare_platform(
        _executable: &Path,
        _grants: &[FileGrant],
    ) -> Result<Self, ConfinementError> {
        Err(ConfinementError::Unsupported("not Linux"))
    }

    /// The Landlock ABI the ruleset was built for.
    pub fn landlock_abi(&self) -> u32 {
        self.landlock_abi
    }

    /// Isolate `command`'s descriptors (`chief-of-staff-spawn-isolation`),
    /// then confine it. Call it last, after stdin, stdout, the arguments and
    /// the environment are set: argv is taken from the command here.
    ///
    /// What runs is always the executable `prepare` opened; the command's
    /// program is only argv\[0\]. argv and the environment are taken when
    /// `apply` runs: arguments or variables added afterwards are not
    /// passed, and `CommandExt::arg0` is ignored.
    pub fn apply<'a>(&self, command: &'a mut Command) -> &'a mut Command {
        chief_of_staff_spawn_isolation::isolate(command);
        #[cfg(target_os = "linux")]
        linux::install(std::sync::Arc::clone(&self.prepared), command);
        command
    }
}
