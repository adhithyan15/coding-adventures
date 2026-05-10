//! # `matrix-profile` — profile-guided specialisation for the matrix execution layer
//!
//! Implements **spec MX05** (tiered specialisation runtime).  Owns:
//!
//! - **Profile sampler** ([`Profiler`], [`ProfileObservation`],
//!   [`TensorObservation`]) — Phase 1 invocation counters and Phase
//!   2a tensor-byte sampling.
//! - **SpecKey** and friends ([`SpecKey`], [`ShapeClass`],
//!   [`RangeClass`]) — the equivalence class identifying which
//!   observed pattern a specialised kernel targets.  Phase 3 V1.
//! - **Specialiser trait** ([`Specialiser`], [`NoopSpecialiser`],
//!   [`SpecialisedKernel`]) — the per-backend hook for emitting
//!   specialised kernels.  Phase 3 V1.
//! - **SpecCache** — bounded LRU keyed by `SpecKey`.  Phase 3 V1.
//! - **SpecialisationPolicy** trait + [`DefaultPolicy`] — turns a
//!   `ProfileObservation` into an `Option<SpecKey>`.  Phase 3 V2.
//! - **SpecRouter** — glues the four pieces above into a single
//!   `route()` decision point.  Phase 3 V3.
//!
//! ## Promotion from `matrix-runtime`
//!
//! Phases 1, 2a, 3 V1, V2, and V3 all shipped these modules inline in
//! `matrix-runtime` (per the spec's "promote later" plan).  Phase 3
//! V4 wired the router into `image-gpu-core::pipeline` end-to-end
//! and confirmed the interface is stable; **Phase 3 V5** (this
//! crate) lifts everything into its own dependency surface so:
//!
//! - Backends (`matrix-cpu`, `matrix-metal`) can install a custom
//!   `Specialiser` without taking on `matrix-runtime`'s planner /
//!   registry / cost-model machinery.
//! - Domain libraries (`image-gpu-core`) and standalone tests can
//!   pull in the specialisation pipeline alone.
//! - `matrix-runtime` continues to re-export every public item from
//!   here for back-compat — existing callers see no change.
//!
//! ## Zero dependencies
//!
//! Per the MX00 zero-dependency mandate, this crate uses only
//! `core` + `alloc` + `std` plus the upstream layer crates
//! (`matrix-ir`, `compute-ir`).  Notably no executor-protocol or
//! matrix-runtime — keeping the dependency graph acyclic.

#![warn(rust_2018_idioms)]

mod policy;
mod profile;
mod router;
mod spec;

pub use policy::{DefaultPolicy, SpecialisationPolicy};
pub use profile::{ProfileObservation, Profiler, TensorObservation};
pub use router::SpecRouter;
pub use spec::{
    NoopSpecialiser, RangeClass, ShapeClass, SpecCache, SpecKey, Specialiser, SpecialisedKernel,
};
