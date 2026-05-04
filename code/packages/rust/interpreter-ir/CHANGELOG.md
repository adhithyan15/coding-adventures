# Changelog — interpreter-ir

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
