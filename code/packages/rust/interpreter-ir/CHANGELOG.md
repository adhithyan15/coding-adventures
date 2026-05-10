# Changelog — interpreter-ir

## [0.2.0] — 2026-05-04

### Added (LANG23 PR 23-E — refinement annotation fields, additive/opt-in)

- `param_refinements: Vec<Option<RefinedType>>` field on `IIRFunction`.  In
  lockstep with `params` — `param_refinements[i]` is `Some(rt)` when param `i`
  carries a LANG23 annotation, `None` otherwise.  Empty `Vec` (not a `Vec` of
  `None`s) from callers that never set annotations, distinguishing "no LANG23"
  from "every param annotated as Any".
- `return_refinement: Option<RefinedType>` field on `IIRFunction`.  `Some(rt)`
  when the function carries a `-> TypeAnnotation` return type, `None` when
  unannotated.
- `IIRFunction::new()` updated to include both new fields (default empty/`None`).
- `Default` impl for `IIRFunction` — enables `IIRFunction { name: "f".into(), ..Default::default() }`
  struct-update syntax in tests and incremental builders; eliminates the need
  to list every field at every construction site when adding new optional fields.
- `lang-refined-types` added as a dependency.
- Doc-test in `function.rs` updated to include both new fields.
- Serialisation (`serialise.rs`) struct literal updated.

### Design note

These fields are **additive and opt-in**: all existing callers leave them at
their default empty/`None` values and see no behaviour change.  The refinement
checker (`lang-refinement-checker`) reads these fields to discharge proof
obligations without any changes to the instruction stream or the existing
`type_hint` string mechanism (which continues to carry unrefined kind
information for the JIT/profiler).

## [0.1.0] — 2026-04-27

Initial Rust port of the Python `interpreter-ir` package (LANG01).

### Added

- `IIRModule` — top-level container for an InterpreterIR program.  Holds all
  `IIRFunction` objects plus `entry_point` and `language` metadata.  `validate()`
  checks for duplicate names, missing entry point, and undefined branch labels.

- `IIRFunction` — a named, parameterised sequence of `IIRInstr`.  Auto-infers
  `FunctionTypeStatus` from param types and instruction `type_hint`s.  Stores
  `call_count` (incremented by `vm-core`), `feedback_slots`, and `source_map`.

- `FunctionTypeStatus` — `FullyTyped / PartiallyTyped / Untyped` compilation
  tiers that drive the JIT threshold (0 / 10 / 100 calls).

- `IIRInstr` — one instruction.  Static fields: `op`, `dest`, `srcs`,
  `type_hint`, `may_alloc`.  Runtime fields: `observed_slot` (`SlotState`),
  `observed_type`, `observation_count`, `deopt_anchor`.
  `record_observation()` advances the slot state machine.

- `Operand` — `Var(String) | Int(i64) | Float(f64) | Bool(bool)`.

- `SlotState` — V8 Ignition–style per-instruction type-feedback.
  States: `Uninitialized → Monomorphic → Polymorphic → Megamorphic`.
  Caps at `MAX_POLYMORPHIC_OBSERVATIONS = 4` distinct types.

- `opcodes` module — opcode category predicates (`is_arithmetic`, `is_branch`,
  `is_call`, …), concrete type set, and ref-type helpers.

- Binary serialisation (`serialise` module) — `b"IIR\0"` magic, version 1.0,
  little-endian, all operand variants.  Profiling fields not serialised
  (runtime-only).

- 38 unit tests + 11 doctests.
