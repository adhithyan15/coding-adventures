//! Arming: installing the strict, deny-by-default sandbox into the *current*
//! process (VER08b section 5). Linux only.
//!
//! The order matters and every step is checked:
//!
//! ```text
//!   1. prctl(PR_SET_NO_NEW_PRIVS, 1)   a filter can then never be sidestepped
//!                                      by exec'ing a set-uid binary, and an
//!                                      unprivileged process may install one
//!   2. Landlock                        every filesystem right handled, no rule
//!                                      except the manifest's -> no fs at all
//!   3. seccomp(SET_MODE_FILTER)        the BPF program from `bpf::build_program`
//! ```
//!
//! Landlock goes before seccomp because `landlock_restrict_self` is itself a
//! syscall the filter will not allow. The filter is installed with
//! `SECCOMP_FILTER_FLAG_TSYNC` so that, should a harness have started threads,
//! they are covered too (or the call fails loudly).
//!
//! ## Unsafe
//!
//! Every `unsafe` block here is a raw `libc::syscall`/`prctl`/`open` whose
//! arguments are plain integers or pointers to stack/heap values that outlive
//! the call. There is no safe std API for seccomp or Landlock. The same
//! approach is used by `chief-of-staff-linux-sandbox`; the manifest declares
//! the `ffi` use.

use crate::bpf::{build_program, Insn};
use crate::policy::RuntimePolicy;
use std::ffi::CString;
use std::io;

const SYS_LANDLOCK_CREATE_RULESET: libc::c_long = 444;
const SYS_LANDLOCK_ADD_RULE: libc::c_long = 445;
const SYS_LANDLOCK_RESTRICT_SELF: libc::c_long = 446;
const LANDLOCK_CREATE_RULESET_VERSION: u32 = 1;
const LANDLOCK_RULE_PATH_BENEATH: u32 = 1;

const SECCOMP_SET_MODE_FILTER: libc::c_long = 1;
const SECCOMP_FILTER_FLAG_TSYNC: libc::c_long = 1;

/// Filesystem rights by name, with the Landlock ABI that introduced them.
const FS_RIGHTS: &[(&str, u64, i64)] = &[
    ("EXECUTE", 1 << 0, 1),
    ("WRITE_FILE", 1 << 1, 1),
    ("READ_FILE", 1 << 2, 1),
    ("READ_DIR", 1 << 3, 1),
    ("REMOVE_DIR", 1 << 4, 1),
    ("REMOVE_FILE", 1 << 5, 1),
    ("MAKE_CHAR", 1 << 6, 1),
    ("MAKE_DIR", 1 << 7, 1),
    ("MAKE_REG", 1 << 8, 1),
    ("MAKE_SOCK", 1 << 9, 1),
    ("MAKE_FIFO", 1 << 10, 1),
    ("MAKE_BLOCK", 1 << 11, 1),
    ("MAKE_SYM", 1 << 12, 1),
    ("REFER", 1 << 13, 2),
    ("TRUNCATE", 1 << 14, 3),
    ("IOCTL_DEV", 1 << 15, 5),
];
const NET_BIND_TCP: u64 = 1 << 0;
const NET_CONNECT_TCP: u64 = 1 << 1;

/// What arming achieved; written into the ARMED marker and the evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArmReport {
    /// Landlock ABI version (0 = unavailable).
    pub landlock_abi: i64,
    /// True if a Landlock domain was installed.
    pub landlock_enforced: bool,
    /// True if the seccomp filter was installed.
    pub seccomp_enforced: bool,
}

/// Why arming failed.
#[derive(Debug)]
pub enum ArmError {
    /// A syscall failed (name, OS error).
    Syscall(&'static str, io::Error),
    /// Landlock is needed to scope fs capabilities but is not available.
    LandlockRequired,
}

impl std::fmt::Display for ArmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Syscall(name, error) => write!(f, "{name} failed: {error}"),
            Self::LandlockRequired => f.write_str("Landlock required but unavailable"),
        }
    }
}

impl std::error::Error for ArmError {}

#[repr(C)]
struct SockFilter {
    code: u16,
    jt: u8,
    jf: u8,
    k: u32,
}

#[repr(C)]
struct SockFprog {
    len: u16,
    filter: *const SockFilter,
}

#[repr(C)]
struct RulesetAttr {
    handled_access_fs: u64,
    handled_access_net: u64,
}

#[repr(C, packed)]
struct PathBeneath {
    allowed_access: u64,
    parent_fd: i32,
}

/// Probe the Landlock ABI version; 0 means unavailable.
pub fn landlock_abi() -> i64 {
    // SAFETY: a null attr with size 0 and the VERSION flag is the documented
    // probe; the kernel reads no memory.
    let result = unsafe {
        libc::syscall(
            SYS_LANDLOCK_CREATE_RULESET,
            std::ptr::null::<RulesetAttr>(),
            0usize,
            LANDLOCK_CREATE_RULESET_VERSION,
        )
    };
    if result < 0 {
        0
    } else {
        result
    }
}

fn handled_fs_mask(abi: i64) -> u64 {
    FS_RIGHTS
        .iter()
        .filter(|(_, _, since)| *since <= abi)
        .fold(0, |mask, (_, bit, _)| mask | bit)
}

fn rights_mask(names: &[&str], abi: i64) -> u64 {
    FS_RIGHTS
        .iter()
        .filter(|(name, _, since)| names.contains(name) && *since <= abi)
        .fold(0, |mask, (_, bit, _)| mask | bit)
}

fn install_landlock(policy: &RuntimePolicy, abi: i64) -> Result<(), ArmError> {
    let attr = RulesetAttr {
        handled_access_fs: handled_fs_mask(abi),
        // ABI 4+: also handle TCP bind/connect, with no rules (denied) unless
        // the manifest declares net, in which case seccomp + the container
        // network govern it and Landlock leaves TCP unhandled.
        handled_access_net: if abi >= 4 && !policy.needs_network {
            NET_BIND_TCP | NET_CONNECT_TCP
        } else {
            0
        },
    };
    let attr_size = if abi >= 4 {
        std::mem::size_of::<RulesetAttr>()
    } else {
        8
    };
    // SAFETY: `attr` is a live repr(C) struct and `attr_size` never exceeds
    // its size; the kernel copies from it during the call.
    let fd = unsafe {
        libc::syscall(
            SYS_LANDLOCK_CREATE_RULESET,
            &attr as *const RulesetAttr,
            attr_size,
            0u32,
        )
    };
    if fd < 0 {
        return Err(ArmError::Syscall(
            "landlock_create_ruleset",
            io::Error::last_os_error(),
        ));
    }
    let ruleset_fd = fd as i32;

    for (path, rights) in &policy.landlock_paths {
        let mask = rights_mask(rights, abi) & attr.handled_access_fs;
        let c_path = CString::new(path.as_str()).map_err(|_| {
            ArmError::Syscall(
                "landlock path",
                io::Error::new(io::ErrorKind::InvalidInput, "NUL in path"),
            )
        })?;
        // SAFETY: `c_path` is a valid NUL-terminated string for the call.
        let parent_fd = unsafe { libc::open(c_path.as_ptr(), libc::O_PATH | libc::O_CLOEXEC) };
        if parent_fd < 0 {
            // SAFETY: closing the ruleset fd we own.
            unsafe { libc::close(ruleset_fd) };
            return Err(ArmError::Syscall(
                "open(O_PATH)",
                io::Error::last_os_error(),
            ));
        }
        let rule = PathBeneath {
            allowed_access: mask,
            parent_fd,
        };
        // SAFETY: `rule` is a live packed struct matching the kernel ABI.
        let added = unsafe {
            libc::syscall(
                SYS_LANDLOCK_ADD_RULE,
                ruleset_fd,
                LANDLOCK_RULE_PATH_BENEATH,
                &rule as *const PathBeneath,
                0u32,
            )
        };
        let add_error = io::Error::last_os_error();
        // SAFETY: closing the path fd we opened above.
        unsafe { libc::close(parent_fd) };
        if added < 0 {
            // SAFETY: closing the ruleset fd we own.
            unsafe { libc::close(ruleset_fd) };
            return Err(ArmError::Syscall("landlock_add_rule", add_error));
        }
    }

    // SAFETY: restrict_self takes only the ruleset fd and flags.
    let restricted = unsafe { libc::syscall(SYS_LANDLOCK_RESTRICT_SELF, ruleset_fd, 0u32) };
    let restrict_error = io::Error::last_os_error();
    // SAFETY: closing the ruleset fd we own; the domain outlives it.
    unsafe { libc::close(ruleset_fd) };
    if restricted < 0 {
        return Err(ArmError::Syscall("landlock_restrict_self", restrict_error));
    }
    Ok(())
}

fn install_seccomp(program: &[Insn]) -> Result<(), ArmError> {
    let filters: Vec<SockFilter> = program
        .iter()
        .map(|i| SockFilter {
            code: i.code,
            jt: i.jt,
            jf: i.jf,
            k: i.k,
        })
        .collect();
    let fprog = SockFprog {
        len: filters.len() as u16,
        filter: filters.as_ptr(),
    };
    // SAFETY: `fprog` points at `filters`, alive until after the call; the
    // kernel copies the program.
    let result = unsafe {
        libc::syscall(
            libc::SYS_seccomp,
            SECCOMP_SET_MODE_FILTER,
            SECCOMP_FILTER_FLAG_TSYNC,
            &fprog as *const SockFprog,
        )
    };
    if result != 0 {
        return Err(ArmError::Syscall("seccomp", io::Error::last_os_error()));
    }
    Ok(())
}

/// Install no_new_privs, Landlock and seccomp for `policy` into this process.
///
/// After `Ok`, the process can make only the syscalls in the policy; any other
/// kills it with SIGSYS. There is no way back.
pub fn arm(policy: &RuntimePolicy) -> Result<ArmReport, ArmError> {
    // SAFETY: PR_SET_NO_NEW_PRIVS takes integer arguments only.
    let rc = unsafe { libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) };
    if rc != 0 {
        return Err(ArmError::Syscall(
            "prctl(NO_NEW_PRIVS)",
            io::Error::last_os_error(),
        ));
    }
    let abi = landlock_abi();
    let mut landlock_enforced = false;
    if abi > 0 {
        install_landlock(policy, abi)?;
        landlock_enforced = true;
    } else if policy.landlock_required {
        return Err(ArmError::LandlockRequired);
    }
    install_seccomp(&build_program(policy))?;
    Ok(ArmReport {
        landlock_abi: abi,
        landlock_enforced,
        seccomp_enforced: true,
    })
}
