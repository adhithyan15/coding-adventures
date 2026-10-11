//! # Process hardening for the Chief daemon (D18S S-I5, step 6 P2.6a)
//!
//! The daemon is the supervisor, and today also the broker: it holds every
//! agent's channel keys, the vault's unsealed key material while a lease is
//! served, and the audit log. A core dump would write all of that to disk,
//! for whoever can read the crash directory. A same-user debugger could read
//! it live.
//!
//! [`suppress_core_dumps`] closes both, as the first thing `main` does, before
//! any key exists:
//!
//! ```text
//!   platform   what it sets                              what it stops
//!   ---------  ----------------------------------------  ------------------------------
//!   every Unix RLIMIT_CORE = 0 (soft and hard)           a core file on crash; the hard
//!                                                        limit means it cannot be raised
//!   Linux      prctl(PR_SET_DUMPABLE, 0)                 core dumps even with a limit;
//!                                                        ptrace and /proc/<pid>/mem by
//!                                                        any process of the same user
//!   macOS      ptrace(PT_DENY_ATTACH)                    a debugger attaching later
//!   Windows    nothing yet (a restrictive process DACL   reported, not hidden: see
//!              is D18S step 8)                           `CoreDumpProtection`
//! ```
//!
//! Every setting that can be read back is, and a mismatch is an error: S-P3
//! asks for loud failure, and the caller refuses to start. `PT_DENY_ATTACH`
//! has no getter, so its success is the call's own return value. On macOS,
//! `task_for_pid` memory reads are stopped by the Hardened Runtime without
//! `get-task-allow` (a build-signing setting), not by `PT_DENY_ATTACH`; until
//! the build signs that way, the report lists it as missing.
//!
//! Children: `exec` resets dumpability for the new image, so an agent is
//! dumpable again. The zero core limit is inherited by every child, with a
//! hard limit it cannot raise, which costs a child nothing.

#![deny(unsafe_op_in_unsafe_fn)]

use std::fmt;

/// What [`suppress_core_dumps`] applied, for the daemon's startup audit
/// record (S-P4: report enforcement, never assume it).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreDumpProtection {
    /// The measures in force, each verified after it was set.
    pub applied: Vec<&'static str>,
    /// Measures this platform should have and this crate does not yet set.
    pub missing: Vec<&'static str>,
}

/// A measure could not be set, or did not read back as set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HardeningError {
    pub measure: &'static str,
    pub detail: String,
}

impl fmt::Display for HardeningError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "cannot apply {}: {}", self.measure, self.detail)
    }
}

impl std::error::Error for HardeningError {}

/// Suppress this process's core dumps and later debugger attach. Call it
/// first in `main`, and refuse to start on `Err`.
pub fn suppress_core_dumps() -> Result<CoreDumpProtection, HardeningError> {
    let mut protection = CoreDumpProtection {
        applied: Vec::new(),
        missing: Vec::new(),
    };
    platform::apply(&mut protection)?;
    Ok(protection)
}

#[cfg(unix)]
mod platform {
    use super::*;
    use std::io;

    fn failed(measure: &'static str) -> HardeningError {
        HardeningError {
            measure,
            detail: io::Error::last_os_error().to_string(),
        }
    }

    pub(super) fn apply(protection: &mut CoreDumpProtection) -> Result<(), HardeningError> {
        zero_core_limit()?;
        protection.applied.push("RLIMIT_CORE = 0 (soft and hard)");
        not_dumpable(protection)?;
        deny_attach(protection)?;
        Ok(())
    }

    /// RLIMIT_CORE to zero. The hard limit too: a process may lower its
    /// hard limit but never raise it again without privilege, so nothing
    /// later in the daemon can undo this.
    fn zero_core_limit() -> Result<(), HardeningError> {
        const MEASURE: &str = "RLIMIT_CORE = 0";
        let zero = libc::rlimit {
            rlim_cur: 0,
            rlim_max: 0,
        };
        // SAFETY: a valid rlimit for a valid resource.
        if unsafe { libc::setrlimit(libc::RLIMIT_CORE, &zero) } != 0 {
            return Err(failed(MEASURE));
        }
        let mut now = libc::rlimit {
            rlim_cur: 1,
            rlim_max: 1,
        };
        // SAFETY: getrlimit writes one rlimit.
        if unsafe { libc::getrlimit(libc::RLIMIT_CORE, &mut now) } != 0 {
            return Err(failed(MEASURE));
        }
        if now.rlim_cur != 0 || now.rlim_max != 0 {
            return Err(HardeningError {
                measure: MEASURE,
                detail: format!("reads back as {}/{}", now.rlim_cur, now.rlim_max),
            });
        }
        Ok(())
    }

    /// Linux: not dumpable. That also makes `/proc/<pid>` root-owned and
    /// refuses ptrace from any process of the same user, an agent included.
    #[cfg(any(target_os = "linux", target_os = "android"))]
    fn not_dumpable(protection: &mut CoreDumpProtection) -> Result<(), HardeningError> {
        const MEASURE: &str = "PR_SET_DUMPABLE = 0";
        // SAFETY: integer arguments only.
        if unsafe { libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0) } != 0 {
            return Err(failed(MEASURE));
        }
        // SAFETY: integer arguments only; returns the current value.
        let now = unsafe { libc::prctl(libc::PR_GET_DUMPABLE, 0, 0, 0, 0) };
        if now != 0 {
            return Err(HardeningError {
                measure: MEASURE,
                detail: format!("reads back as {now}"),
            });
        }
        protection.applied.push(MEASURE);
        Ok(())
    }

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    fn not_dumpable(_protection: &mut CoreDumpProtection) -> Result<(), HardeningError> {
        Ok(())
    }

    /// macOS: refuse any later debugger attach. A process already being
    /// traced is killed by this call, which for the daemon is the right
    /// answer.
    #[cfg(target_os = "macos")]
    fn deny_attach(protection: &mut CoreDumpProtection) -> Result<(), HardeningError> {
        const MEASURE: &str = "ptrace(PT_DENY_ATTACH)";
        // SAFETY: PT_DENY_ATTACH takes no address or data.
        if unsafe { libc::ptrace(libc::PT_DENY_ATTACH, 0, std::ptr::null_mut(), 0) } != 0 {
            return Err(failed(MEASURE));
        }
        protection
            .applied
            .push("ptrace(PT_DENY_ATTACH) (no getter: not read back)");
        protection
            .missing
            .push("Hardened Runtime without get-task-allow (task_for_pid memory reads)");
        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    fn deny_attach(protection: &mut CoreDumpProtection) -> Result<(), HardeningError> {
        if !cfg!(any(target_os = "linux", target_os = "android")) {
            // The BSDs: procctl(PROC_TRACE_CTL) and its OpenBSD equivalent
            // are not set yet; the limit above still stops the core file.
            protection
                .missing
                .push("debugger-attach denial (procctl PROC_TRACE_CTL or equivalent)");
        }
        Ok(())
    }
}

#[cfg(not(unix))]
mod platform {
    use super::*;

    pub(super) fn apply(protection: &mut CoreDumpProtection) -> Result<(), HardeningError> {
        protection
            .missing
            .push("restrictive process DACL and WER exclusion (D18S step 8)");
        Ok(())
    }
}
