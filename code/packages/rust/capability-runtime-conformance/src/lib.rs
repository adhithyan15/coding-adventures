//! # Runtime capability conformance (VER08b)
//!
//! `required_capabilities.json` says what a package may do. The static
//! analyzer (VER08a) checks the *source* against that claim; this crate checks
//! the *running program* against it, with two independent mechanisms that must
//! agree:
//!
//! * **enforcement** - the test binary installs a deny-by-default seccomp +
//!   Landlock sandbox into itself, derived mechanically from the manifest,
//!   immediately before the first library call (module `arm`, Linux only);
//! * **observation** - a parent runs it under `strace -f` and classifies every
//!   syscall after the ARMED marker ([`observe`]).
//!
//! Negative controls (module `sabotage`, Linux only) prove each mechanism catches each
//! forbidden category on its own, and a Docker ring ([`container`]) renders
//! the same policy at the container boundary. Results are written as
//! `capability_runtime_evidence.json` (schema in the spec).
//!
//! Typical use is a `harness = false` test target:
//!
//! ```ignore
//! fn main() {
//!     std::process::exit(run_package_gate(&GateConfig { /* ... */ }));
//! }
//! ```
//!
//! Only Linux is live. Elsewhere [`run_package_gate`] reports `NOT-LIVE`
//! (failure id `RCG010`) and says so in the evidence; it never reports a pass.

#[cfg(target_os = "linux")]
pub mod arm;
#[cfg(target_os = "linux")]
pub mod bpf;
pub mod container;
pub mod failure;
pub mod harness;
pub mod json;
pub mod observe;
pub mod policy;
#[cfg(target_os = "linux")]
pub mod sabotage;

pub use failure::FailureId;
pub use harness::{run_package_gate, Check, Exercise, GateConfig, Rings};
pub use policy::{PolicyOptions, Refusal, RuntimePolicy};
