//! The registry of failure ids (VER08b section 10).
//!
//! Every way the gate can say "no" has a stable, greppable id. A CI log, an
//! evidence file and a spec table all use the same string, so a reader can go
//! from `RCG002` in a log to its meaning without reading code.

use std::fmt;

/// One registered failure id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FailureId {
    /// Enforcement: killed by SIGSYS after arming.
    Rcg001,
    /// Observation: syscall outside baseline + declared seen after arming.
    Rcg002,
    /// Enforcement and observation disagree.
    Rcg003,
    /// The ARMED marker was never seen.
    Rcg004,
    /// A variable no capability names is present in the environment.
    Rcg005,
    /// A declared capability cannot be enforced at runtime.
    Rcg006,
    /// The library/test checks failed (not a sandbox event).
    Rcg007,
    /// Installing the sandbox failed.
    Rcg008,
    /// A negative control was not detected: the gate itself is defective.
    Rcg009,
    /// The platform cannot run the gate (reported, never silent).
    Rcg010,
    /// An unexpected file descriptor was inherited.
    Rcg011,
    /// Landlock is required for fs scoping but unavailable.
    Rcg012,
    /// The observer is unavailable, or its trace is absent, empty or unparsable.
    Rcg013,
    /// The container ring failed.
    Rcg014,
}

impl FailureId {
    /// The stable string form, e.g. `"RCG001"`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Rcg001 => "RCG001",
            Self::Rcg002 => "RCG002",
            Self::Rcg003 => "RCG003",
            Self::Rcg004 => "RCG004",
            Self::Rcg005 => "RCG005",
            Self::Rcg006 => "RCG006",
            Self::Rcg007 => "RCG007",
            Self::Rcg008 => "RCG008",
            Self::Rcg009 => "RCG009",
            Self::Rcg010 => "RCG010",
            Self::Rcg011 => "RCG011",
            Self::Rcg012 => "RCG012",
            Self::Rcg013 => "RCG013",
            Self::Rcg014 => "RCG014",
        }
    }

    /// One-line meaning, as in the spec table.
    pub fn meaning(self) -> &'static str {
        match self {
            Self::Rcg001 => "killed by SIGSYS after arming (attempted forbidden operation)",
            Self::Rcg002 => "syscall outside baseline + declared observed after arming",
            Self::Rcg003 => "enforcement and observation disagree",
            Self::Rcg004 => "ARMED marker never seen",
            Self::Rcg005 => "environment variable not named by any capability",
            Self::Rcg006 => "declared capability cannot be enforced at runtime",
            Self::Rcg007 => "library/test check failed",
            Self::Rcg008 => "sandbox installation failed",
            Self::Rcg009 => "negative control not detected (gate defect)",
            Self::Rcg010 => "platform not live",
            Self::Rcg011 => "unexpected inherited file descriptor",
            Self::Rcg012 => "Landlock required for fs scoping but unavailable",
            Self::Rcg013 => "observer unavailable, or trace absent/empty/unparsable",
            Self::Rcg014 => "container ring failed",
        }
    }

    /// Every registered id, in order.
    pub fn all() -> [FailureId; 14] {
        [
            Self::Rcg001,
            Self::Rcg002,
            Self::Rcg003,
            Self::Rcg004,
            Self::Rcg005,
            Self::Rcg006,
            Self::Rcg007,
            Self::Rcg008,
            Self::Rcg009,
            Self::Rcg010,
            Self::Rcg011,
            Self::Rcg012,
            Self::Rcg013,
            Self::Rcg014,
        ]
    }
}

impl fmt::Display for FailureId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Process exit codes the child uses to report pre-arm failures to the parent.
/// They sit in the 70s (BSD `sysexits` territory) so they cannot be confused
/// with a test count or a signal.
pub mod exit_code {
    /// Sandbox installation failed (`RCG008`).
    pub const INSTALL_FAILED: i32 = 70;
    /// A library check failed (`RCG007`).
    pub const CHECK_FAILED: i32 = 71;
    /// Environment audit failed (`RCG005`).
    pub const ENV_EXPOSED: i32 = 75;
    /// File-descriptor audit failed (`RCG011`).
    pub const FD_LEAK: i32 = 76;
    /// Landlock required but unavailable (`RCG012`).
    pub const LANDLOCK_MISSING: i32 = 77;
}
