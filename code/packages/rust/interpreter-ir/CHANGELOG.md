# Changelog — interpreter-ir

## [0.3.0] — 2026-05-11

### Added (LANG28A — cooperative-multitasking opcode taxonomy)

This release adds 27 new opcode mnemonics, 6 new predicate functions, 8 new
type-string helpers, and `unwrap_option_type` (symmetry gap) to the `opcodes`
module.  No VM or backend implementation is included; this is the naming and
classification layer that future `vm-concurrency` (LANG28B) and updated
backends will build on.

#### New opcode predicates

- `is_task(op)` — 8 task opcodes:
  `task_spawn`, `task_current`, `task_yield`, `task_sleep`,
  `task_join`, `task_cancel`, `task_check_cancel`, `task_detach`.
- `is_task_group(op)` — 5 group opcodes:
  `group_new`, `group_spawn`, `group_join`, `group_cancel`, `group_close`.
- `is_channel(op)` — 6 channel opcodes:
  `chan_new`, `chan_send`, `chan_recv`, `chan_try_send`, `chan_try_recv`, `chan_close`.
- `is_select(op)` — 8 select opcodes:
  `select_new`, `select_recv`, `select_send`, `select_join`,
  `select_timer`, `select_cancel`, `select_wait`, `select_default`.
- `is_concurrency(op)` — union of all four families (27 total).
- `is_parking(op)` — 7 ops that may suspend the current task:
  `task_yield`, `task_sleep`, `task_join`, `chan_send`, `chan_recv`,
  `group_join`, `select_wait`.

#### Updated predicates

- `is_known_op(op)` — extended with `|| is_concurrency(op)`; all 27 new
  mnemonics are now accepted by the module validator.
- `is_value_producing(op)` — extended with 18 concurrency ops that produce a
  non-`None` dest (task handles, channel handles, arm IDs, received values, …).
- `has_side_effects(op)` — extended with 9 concurrency ops that mutate
  observable state without producing a value (yield, sleep, cancel, send, close, …).
- `is_allocating(op)` — extended with 5 ops that create heap-resident objects:
  `task_spawn`, `group_new`, `group_spawn`, `chan_new`, `select_new`.

#### New type-string helpers

- `is_task_type(s)`, `is_channel_type(s)`, `is_option_type(s)` — recognise
  `"task<T>"`, `"channel<T>"`, `"option<T>"`.
- `is_concurrency_type(s)` — covers all seven concurrency type strings:
  `task<T>`, `channel<T>`, `option<T>`, `task_group`, `select_set`,
  `cancel_token`, `deadline`.
- `make_task_type(T)` / `unwrap_task_type(s)` — construct / decompose `"task<T>"`.
- `make_channel_type(T)` / `unwrap_channel_type(s)` — construct / decompose `"channel<T>"`.
- `make_option_type(T)` / `unwrap_option_type(s)` — construct / decompose `"option<T>"`.
  (`unwrap_option_type` fills a symmetry gap from 0.2.0 where `make_option_type`
  existed but no inverse was provided.)

#### Tests

- 14 new unit tests covering all new predicates, parking subset, type round-trips,
  and updated predicate extensions.
- 6 new doc-tests covering the new type helpers.
- Total test count: 59 unit tests + 21 doc-tests.

### Design note

All 27 new opcodes are **classification-only** in this release.  Backends that
encounter them should return an `UnsupportedOp` validation error (consistent
with how they handle heap opcodes today).  The cooperative-multitasking runtime
(`vm-concurrency`) and updated backends are the scope of LANG28B and later.

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
