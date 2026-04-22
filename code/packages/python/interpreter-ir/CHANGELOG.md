# Changelog — coding-adventures-interpreter-ir

## [0.1.0] — 2026-04-21

### Added

- `instr.py` — `IIRInstr` dataclass with `op`, `dest`, `srcs`, `type_hint`;
  runtime feedback slots `observed_type`, `observation_count`, `deopt_anchor`;
  `record_observation()`, `is_typed()`, `has_observation()`, `is_polymorphic()`,
  `effective_type()` helpers.
- `function.py` — `IIRFunction` dataclass with `name`, `params`, `return_type`,
  `instructions`, `register_count`, `type_status`, `call_count`;
  `FunctionTypeStatus` enum (FULLY_TYPED / PARTIALLY_TYPED / UNTYPED);
  `infer_type_status()`, `label_index()`, `param_names()`, `param_types()`.
- `module.py` — `IIRModule` dataclass with `name`, `functions`, `entry_point`,
  `language`; `get_function()`, `function_names()`, `add_or_replace()`,
  `validate()`.
- `opcodes.py` — frozenset opcode category constants: `ARITHMETIC_OPS`,
  `BITWISE_OPS`, `CMP_OPS`, `BRANCH_OPS`, `CONTROL_OPS`, `MEMORY_OPS`,
  `CALL_OPS`, `IO_OPS`, `COERCION_OPS`, `VALUE_OPS`, `SIDE_EFFECT_OPS`,
  `ALL_OPS`; type constants `CONCRETE_TYPES`, `DYNAMIC_TYPE`,
  `POLYMORPHIC_TYPE`.
- `serialise.py` — `serialise(IIRModule) → bytes` and
  `deserialise(bytes) → IIRModule`; little-endian binary format with
  `IIR\x00` magic, version byte, length-prefixed UTF-8 strings, and
  per-operand kind tags; observation fields are intentionally not serialised
  (they are runtime state).
- `__init__.py` — re-exports all public types and opcode sets.

### Design decisions

- **Zero dependencies.** `interpreter-ir` is a leaf package.  No language
  runtime, VM, or compiler dependency is introduced.  Any language compiler
  can depend on this package without pulling in the full pipeline.
- **Observation fields excluded from equality and serialisation.**  The
  `observed_type`, `observation_count`, and `deopt_anchor` fields are runtime
  state written by `vm-core`.  They are excluded from `__eq__` (via
  `compare=False`) so that two logically identical programs compare equal
  regardless of profiling history, and from serialisation so that `.iir`
  snapshots always produce fresh, un-profiled state on load.
- **`Operand = str | int | float | bool`** rather than `Any`.  Restricting
  operand types to four concrete Python types makes serialisation deterministic
  and allows the assembler to check literal ranges without runtime surprises.
- **`add_or_replace()` for REPL sessions.**  The REPL integration (LANG08)
  calls this method on each new user input.  By placing it on `IIRModule`
  directly, the REPL plugin stays thin — it does not need to know about
  module internals.
- **`validate()` returns a list of error strings** rather than raising on the
  first error.  This allows tooling (LSP, build system) to collect all errors
  in one pass and report them together.
