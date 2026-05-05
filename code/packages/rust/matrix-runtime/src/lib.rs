//! # `matrix-runtime` — Planner, Registry, Cost Model
//!
//! The brain of the matrix execution layer.  Implements **spec MX04**.
//!
//! See:
//! - [`code/specs/MX04-compute-runtime.md`] — this crate's contract
//! - [`code/specs/MX00-matrix-execution-overview.md`] — architecture
//! - [`code/specs/MX01-matrix-ir.md`] — the upper IR
//! - [`code/specs/MX02-compute-ir.md`] — the lower IR
//! - [`code/specs/MX03-executor-protocol.md`] — wire protocol
//!
//! ## What this crate does
//!
//! 1. **Owns the registry** of available executors and their capabilities.
//! 2. **Lowers `MatrixIR`** to `ComputeIR` via the planner: a
//!    four-pass algorithm (capability filter, greedy cost
//!    minimisation, transfer insertion, lifetime annotation).
//! 3. **Exposes a `Runtime` API** that domain libraries call.
//!
//! V1 ships the **planner** as the load-bearing piece.  Execution
//! (driving ComputeGraphs through transports to executors and
//! collecting outputs) is left as a seam: the runtime exposes
//! `plan()` for inspection and `executors()` for registry access;
//! the actual `run()` end-to-end loop lands when the first executor
//! crate (`matrix-cpu`) lands.
//!
//! ## Naming
//!
//! Originally proposed as `compute-runtime` in spec MX04; renamed to
//! `matrix-runtime` to avoid colliding with the existing G05 GPU
//! runtime simulator already in the workspace under that name.
//!
//! ## Zero dependencies
//!
//! Per the MX00 zero-dependency mandate, this crate uses only
//! `core`, `alloc`, `std`, and the upstream matrix-execution-layer
//! crates (`matrix-ir`, `compute-ir`, `executor-protocol`).  No
//! external crates.

#![warn(rust_2018_idioms)]

mod cost;
mod planner;
mod policy;
mod profile;
mod registry;
mod router;
mod runtime;
mod spec;

pub use cost::{compute_cost, estimate_flops, transfer_cost_ns};
pub use planner::{plan, PlanError};
pub use policy::{DefaultPolicy, SpecialisationPolicy};
pub use profile::{ProfileObservation, Profiler, TensorObservation};
pub use registry::{RegisteredExecutor, Registry};
pub use router::SpecRouter;
pub use runtime::{Runtime, RuntimeError};
pub use spec::{
    NoopSpecialiser, RangeClass, ShapeClass, SpecCache, SpecKey, Specialiser, SpecialisedKernel,
};

// Re-export BackendProfile from executor-protocol so callers don't
// need a separate import.
pub use executor_protocol::BackendProfile;

/// The CPU executor's id — always-available fallback.  Same as
/// [`compute_ir::CPU_EXECUTOR`] but re-exported here for callers
/// that only depend on `matrix-runtime`.
pub use compute_ir::CPU_EXECUTOR;
