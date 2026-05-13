# Changelog — matrix-profile

## [0.2.0] — 2026-05-13

### Added — MX05 Phase 4.6 (`folded_slot` for non-commutative ops)

- New field `pub folded_slot: Option<u8>` on `SpecKey`.  Records
  which input slot the policy folded into a `RangeClass::Constant`,
  so downstream emitters can choose the correct variant for
  non-commutative binary ops (Sub / Div / Pow).
- `DefaultPolicy::should_specialise` now sets `folded_slot =
  Some(tobs.slot as u8)` when it picks a constant input, and
  `None` for non-constant range classes (FloatBits/Integer/Unknown).
- `forced_fire` test helper updated to set `folded_slot: None`.

### Why

Phase 4.5 deferred Sub / Div / Pow because the emitter had no way
to know which side carried the constant — emitting one of the two
non-commutative variants risked wrong output if the policy
happened to fold the opposite slot.  Adding `folded_slot` to the
key gives both the emitter and the dispatcher an unambiguous
signal.

Side effect: two SpecKeys that previously hashed equal
(differing only on which constant slot the policy observed) now
hash distinctly and produce distinct cache entries.  For
commutative ops this is wasted cache space (a future optimisation
can normalise by always reporting slot 1, say); for non-commutative
ops it's correctness-critical.

### Tests

All 57 existing tests continue to pass; the new field is exercised
by downstream tests in `matrix-cpu` (handle hash includes
`folded_slot`), `matrix-metal` (emitter dispatches on it), and
`image-gpu-core` (Phase 4.6 LHS-folded routing test).

## [0.1.0] — 2026-05-05

Initial release.  Implements **MX05 Phase 3 V5** — promotes the
specialisation pipeline (profile sampler, SpecKey + Specialiser
trait, SpecCache, SpecialisationPolicy, SpecRouter) from inline
modules in `matrix-runtime` into its own crate per spec MX05's
layering plan.

### Code moved (no behavioural change)

- `Profiler`, `ProfileObservation`, `TensorObservation` from
  `matrix_runtime::profile` (Phase 1 + 2a).
- `SpecKey`, `ShapeClass`, `RangeClass`, `Specialiser` trait,
  `NoopSpecialiser`, `SpecialisedKernel`, `SpecCache` from
  `matrix_runtime::spec` (Phase 3 V1).
- `SpecialisationPolicy` trait, `DefaultPolicy` from
  `matrix_runtime::policy` (Phase 3 V2).
- `SpecRouter` from `matrix_runtime::router` (Phase 3 V3).

### Back-compat

`matrix-runtime` re-exports every public item from this crate, so
existing code using `use matrix_runtime::Profiler;` (etc.)
continues to work unchanged.

### Tests

All 57 specialisation-pipeline unit tests moved over with their
modules:

- `profile::tests` — 17 tests (Phase 1 + 2a)
- `spec::tests` — 13 tests (Phase 3 V1)
- `policy::tests` — 12 tests (Phase 3 V2)
- `router::tests` — 11 tests (Phase 3 V3)
- 4 from earlier housekeeping

`matrix-runtime` test count drops accordingly (17 unit + 8
integration = 25); all moved tests now run against `matrix-profile`.

### Dependencies

- `matrix-ir` — for `DType` and `Shape` (used by `SpecKey`,
  `TensorObservation`).
- `compute-ir` — for `ComputeGraph`, `ExecutorId`, `PlacedOp` (used
  by `Profiler::record_dispatch`).

No `executor-protocol` or `matrix-runtime` dependency — keeping the
graph acyclic and the specialisation pipeline reusable from
domain libraries (e.g. `image-gpu-core`) without pulling in the
planner.
