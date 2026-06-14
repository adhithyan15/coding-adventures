//! Resource limits enforced by the SIR core.
//!
//! These caps exist primarily to make the IR safe to run on
//! attacker-controlled inputs without risking stack-overflow panics
//! that would DoS the host process.  Rust's default 8 MiB thread
//! stack overflows somewhere in the tens of thousands of frames on
//! typical SIR walks — these caps deliberately stay well under that.
//!
//! Hosts that need to push beyond the cap can re-implement the
//! traversal iteratively; nothing about the IR data model requires
//! recursion.  The public traversal helpers (validator, text
//! printer, default `Backend::check_module`) honour these caps and
//! report `Error`-severity diagnostics when exceeded.

/// Maximum recursion depth for IR-tree traversal.
///
/// Chosen to be high enough that no realistic hand- or compiler-
/// written SIR program comes close, but low enough that an
/// attacker cannot exhaust the host thread stack.  Roughly two
/// orders of magnitude below the empirical Rust default-stack
/// overflow point for the validator's per-frame footprint.
pub const MAX_IR_DEPTH: usize = 1024;
