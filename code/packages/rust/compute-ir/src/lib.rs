//! # `compute-ir` — The Placed Compute Graph
//!
//! `compute-ir` is the lower IR of the matrix execution layer.  It is
//! what the planner produces from `matrix-ir` and what executors
//! consume.  It carries the same dataflow shape as `matrix-ir` but
//! adds three things `matrix-ir` deliberately lacks:
//!
//! 1. **Placement.**  Every op carries an [`ExecutorId`] saying which
//!    executor runs it.
//! 2. **Residency.**  Every tensor carries a [`Residency`]: which
//!    executor's memory holds it, under what [`BufferId`].
//! 3. **Explicit transfers.**  When an op needs an input that lives on
//!    a different executor than the op runs on, a [`PlacedOp::Transfer`]
//!    moves it.  Transfers are first-class graph nodes, not implicit
//!    machinery.
//!
//! See `code/specs/MX02-compute-ir.md` for the full specification and
//! `code/specs/MX00-matrix-execution-overview.md` for context.
//!
//! ## What lives here
//!
//! - [`ExecutorId`], [`BufferId`], [`KernelId`] — placement primitives.
//! - [`Residency`] — `(executor, buffer)` pair.
//! - [`PlacedTensor`], [`PlacedConstant`] — tensors and constants with
//!   residency assigned.
//! - [`PlacedOp`] — the placed-graph op variant (Compute, Transfer,
//!   Alloc, Free).
//! - [`OpTiming`] — per-op cost annotation, kept for telemetry.
//! - [`ComputeGraph`] — the aggregate placed graph.
//! - [`ComputeIrError`] — failures from validation and wire decoding.
//! - [`Graph::dump`] — human-readable pretty-printer.
//! - [`Graph::to_bytes`] / [`Graph::from_bytes`] — versioned binary
//!   wire format that round-trips without loss.
//!
//! ## Zero dependencies
//!
//! Per the MX00 zero-dependency mandate, this crate uses only `core`,
//! `alloc`, `std`, and the upstream `matrix-ir` (path-only,
//! zero-dep).  No `serde`, no `bincode`, no async runtime.  The wire
//! format is hand-rolled and implementation-agnostic.

#![warn(rust_2018_idioms)]

mod placement;
mod graph;
mod validate;
mod dump;
mod wire;

pub use placement::{
    BufferId, ExecutorId, KernelId, OpTiming, PlacedConstant, PlacedOp, PlacedTensor, Residency,
    CPU_EXECUTOR,
};
pub use graph::ComputeGraph;
pub use validate::ComputeIrError;

/// Wire format version for `ComputeGraph`.  Distinct from `matrix-ir`'s
/// version because the layouts are different.
pub const WIRE_FORMAT_VERSION: u32 = 1;
