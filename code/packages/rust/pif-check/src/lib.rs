//! `pif-check`: the release gate for Proof Interchange Format claims
//! (VER32 section 5 and backlog item PIF-003; trusted-base id `tcb.pif.gate-binding`).
//!
//! # Where it sits
//!
//! ```text
//!   untrusted producers ──► PIF files ──► [ resolver ──► reader ──► kernel ] ──► [ gate ]
//!                                          digest check   lean-kernel           goal digest,
//!                                          (this crate)   (trusted)             footprint,
//!                                                                               report
//! ```
//!
//! The kernel decides whether a declaration is well typed. It does **not** decide *which
//! files it was shown* or *which theorem is accepted for which claim*. That is this crate:
//!
//! * [`resolver`]: maps a SHA-256 to the bytes of a file in an explicit local store and
//!   verifies the hash before anything reads them (VER32 E5);
//! * [`control`] and [`chain`]: the ordered environment chain the sign-off fixes, each file
//!   read with the right [`lean_kernel::pif::FileClass`] and the right gate inputs (E1 to E6);
//! * [`cli`] and [`report`]: the command line and its machine-readable report.
//!
//! Because the gate is part of the trusted base it is written like the kernel: literate
//! comments, `#![forbid(unsafe_code)]`, no third-party dependencies, failure closed (every
//! doubt is an error, and an error means no environment is returned), and no panics on any
//! input.

#![forbid(unsafe_code)]

pub mod chain;
pub mod cli;
pub mod control;
pub mod digest;
pub mod error;
pub mod json;
pub mod report;
pub mod resolver;

pub use digest::Digest;
pub use error::CheckError;
