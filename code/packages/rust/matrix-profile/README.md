# matrix-profile

Profile-guided specialisation runtime for the matrix execution layer.
Implements **spec MX05** (tiered specialisation runtime).

## What's in here

- **Profile sampler** — `Profiler`, `ProfileObservation`,
  `TensorObservation`.  Per-(graph, op) invocation counters plus
  optional tensor-byte sampling for range observation.
- **SpecKey** machinery — `SpecKey`, `ShapeClass`, `RangeClass`.  The
  equivalence class identifying which observed pattern a specialised
  kernel targets.  All `Hash + Eq` so they work as HashMap keys.
- **Specialiser trait** — `Specialiser`, `NoopSpecialiser`,
  `SpecialisedKernel`.  The per-backend hook for emitting specialised
  kernels.  Default `NoopSpecialiser` declines every key.
- **SpecCache** — bounded LRU keyed by `SpecKey`.  Default capacity
  64 entries.
- **Specialisation policy** — `SpecialisationPolicy` trait,
  `DefaultPolicy`.  Decides when a `ProfileObservation` is hot
  enough and narrow enough to be worth specialising.  V1 default
  thresholds match the spec (1000 invocations, 95% stability).
- **SpecRouter** — glues the four pieces above into a single
  `route()` decision point.

## Where this crate sits

```
domain library (e.g. image-gpu-core)
    │
    ├── matrix-runtime  (planner + registry + cost model)
    │       │
    │       ├── matrix-profile  ← you are here
    │       ├── matrix-ir
    │       ├── compute-ir
    │       └── executor-protocol
    │
    ├── matrix-cpu      (CPU executor)
    └── matrix-metal    (Metal executor)
```

`matrix-profile` depends only on `matrix-ir` and `compute-ir`.  No
executor-protocol, no matrix-runtime — keeping the dependency
graph acyclic so backends and domain libraries can pull in the
specialisation pipeline alone if they want, without taking on the
planner.

## Quickstart

```rust
use matrix_profile::{Profiler, SpecRouter, SpecCache, NoopSpecialiser, DefaultPolicy};

let profiler = Profiler::new();
let router = SpecRouter::new(
    Box::new(DefaultPolicy::new()),
    SpecCache::default_capacity(),
    Box::new(NoopSpecialiser),
);

// Inside the dispatch loop:
//   profiler.record_dispatch(&placed);
//   for op in placed.ops { ... router.route(...) ... }
```

## History

Phases 1, 2a, 3 V1, V2, and V3 of MX05 all shipped these modules
inline in `matrix-runtime` per the spec's "promote later" plan.
Phase 3 V4 wired them into `image-gpu-core::pipeline` and confirmed
the interface is stable.  Phase 3 V5 (this crate's initial release)
lifted everything into its own dependency surface; `matrix-runtime`
re-exports every public item so existing callers see no change.

## See also

- [`code/specs/MX05-tiered-specialisation-runtime.md`](../../specs/MX05-tiered-specialisation-runtime.md)
- [`matrix-runtime`](../matrix-runtime/) — planner + registry; re-exports every item from here.
- [`image-gpu-core`](../image-gpu-core/) — domain library that uses the SpecRouter end-to-end.
