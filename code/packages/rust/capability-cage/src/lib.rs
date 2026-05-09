//! capability-cage — Rust port of the Go capability-cage runtime.
//!
//! See `code/specs/capability-cage-rust.md` for the full design.
//!
//! V1 scope: foundational types, manifest loading and validation, glob
//! matcher, the [`Backend`] trait with [`OpenBackend`] / [`TestBackend`]
//! / [`DenyAllBackend`] implementations, and the [`secure_file`] family
//! of secure-wrapper functions. Other secure-wrapper categories
//! (`net`, `proc`, `env`, `time`, `stdio`) and the audit envelope land
//! in subsequent PRs.

mod backend;
mod capability;
mod category;
mod errors;
mod glob;
mod manifest;
pub mod secure_file;

pub use backend::{with_backend, Backend, BackendGuard, DenyAllBackend, OpenBackend, TestBackend};
pub use capability::Capability;
pub use category::{Action, Category};
pub use errors::{CapabilityViolationError, InvalidCombination, ManifestError};
pub use glob::match_target;
pub use manifest::Manifest;
pub use read_write_separation::{CapabilityFlavor, CapabilityTrust};
