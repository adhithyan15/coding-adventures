# Changelog — coding-adventures-interpreter-ir

## [Unreleased]

### Added — VMCOND00 Phase 2: ExceptionTableEntry and throw opcode

Implements VMCOND00 Layer 2 — unwind exceptions — in the interpreter IR world.
The new `ExceptionTableEntry` type gives each `IIRFunction` a static exception
table that the VM walks during `throw` propagation, and the `"throw"` mnemonic
is added to the opcode registry so analyzers and validators see it as a side-
effecting instruction.

**New module: `interpreter_ir.exception_table`**

- **`CATCH_ALL: str = "*"`** — sentinel `type_id` value that matches every
  thrown condition regardless of Python type.  Use `"*"` in `type_id` to
  write a catch-all handler.

- **`ExceptionTableEntry`** — frozen dataclass describing one protected region
  and its handler:
  - `from_ip: int` — first IIR instruction index in the guarded range
    (inclusive).
  - `to_ip: int` — first IIR instruction index *outside* the guarded range
    (exclusive).  Semantics: `from_ip <= throw_ip < to_ip` triggers the handler.
    This matches JVM and CPython half-open convention.
  - `handler_ip: int` — IIR instruction index of the first handler instruction.
  - `type_id: str` — `"*"` (catch-all) or a Python type name such as `"ValueError"`.
    Phase 2 matching is exact name equality (`type(condition).__name__`); subtype
    hierarchy is deferred to Phase 3.
  - `val_reg: str` — name of the register that receives the caught condition
    object when the handler is entered.

**Changes to `interpreter_ir.function`**

- `IIRFunction.exception_table: list[ExceptionTableEntry]` — new per-function
  field, `default_factory=list`.  The field is marked `repr=False, compare=False`
  (consistent with `feedback_slots` and `source_map`) so existing equality checks
  and textual representations are unaffected; the exception table is pure runtime
  metadata assembled by the front-end and consumed by the VM.

**Changes to `interpreter_ir.opcodes`**

- **`THROW_OPS: frozenset[str] = frozenset({"throw"})`** — new opcode-category
  frozenset for the single `"throw"` mnemonic.
- `THROW_OPS` is folded into `SIDE_EFFECT_OPS` (a throw has observable side
  effects — it may unwind the call stack) and into `ALL_OPS`.  It is intentionally
  **not** in `BRANCH_OPS` or `CONTROL_OPS`: the VM handles the IP jump internally
  inside `handle_throw`; the static analyzer does not need to model throw as a
  branch.

**Exports**

`ExceptionTableEntry`, `CATCH_ALL`, and `THROW_OPS` are now exported from the
package root (`interpreter_ir.__init__`).

**Test additions (`tests/test_interpreter_ir.py`):**

- 9 new tests in `TestOpcodeSets` covering `THROW_OPS` membership, set-algebra
  relationships with `SIDE_EFFECT_OPS` / `ALL_OPS`, and the exclusions from
  `BRANCH_OPS` / `CONTROL_OPS`.
- New `TestExceptionTableEntry` class (7 tests): construction, immutability
  (`frozen=True`), equality, hash stability, the `CATCH_ALL` constant, and the
  `compare=False` contract on `IIRFunction.exception_table`.

**Spec reference:** VMCOND00 §3 Layer 2 — unwind exceptions.

---

### Added — VMCOND00 Phase 1: syscall_checked and branch_err opcodes

Two new IIR string mnemonics implementing the VMCOND00 Layer 1 result-value
error protocol in the interpreter IR world.

- **`"syscall_checked"`** — Invoke a SYSCALL00-numbered host syscall without
  trapping on errors.  IIR operand layout:
  `srcs = [n (immediate int), arg_reg, val_dst, err_dst]`
  Placed in **`SYSCALL_CHECKED_OPS`** (new frozenset) and **`SIDE_EFFECT_OPS`**
  (it performs I/O).  Not in `VALUE_OPS` (two output slots in `srcs`, not a
  single `dest`).

- **`"branch_err"`** — Branch to a label when an error register is non-zero.
  IIR operand layout: `srcs = [err_reg, label_str]`.
  Added to **`BRANCH_OPS`** so live-variable analysis and control-flow passes
  treat it as a conditional branch.  Falls through on `err_reg == 0`.

**New export:** `SYSCALL_CHECKED_OPS: frozenset[str]` — exported from
`interpreter_ir.__init__` alongside the other opcode-category frozensets.

Programs that don't use these opcodes are fully unaffected — the new mnemonics
never appear in their IIR.

**Spec reference:** VMCOND00 §3 Layer 1; SYSCALL00 §2 canonical table.

---

### Added — LANG16 PR1: heap / GC opcodes and ref<T> type encoding

Schema-only foundation for the GC plug-in framework — vm-core, jit-core,
and the GC packages will fill in the runtime semantics in follow-up
PRs.  Programs that don't allocate are unaffected: the new opcodes
never appear in their IIR, and ``IIRInstr.may_alloc`` defaults to
``False``.

**New opcodes** (in ``HEAP_OPS``):

- ``alloc(size, kind)`` — allocate ``size`` bytes tagged ``kind``;
  produces ``ref<any>``.  Sets a safepoint.
- ``box(value)`` — allocate a single-slot heap cell holding ``value``;
  produces ``ref<typeof(value)>``.  Sets a safepoint.
- ``unbox(ref)`` — deref a single-slot box; traps on null.
- ``field_load(ref, offset)`` — read one field from a heap object.
- ``field_store(ref, offset, value)`` — write one field; emits a
  write barrier when the active GC requires one.
- ``is_null(ref)`` — produces ``bool``.
- ``safepoint()`` — yield to the GC if a collection is pending.

**New opcode-category sets**:

- ``HEAP_OPS`` — the seven opcodes above.
- ``ALLOCATING_OPS`` — the subset that may trigger a collection
  (``alloc``, ``box``, ``safepoint``).

``alloc``, ``box``, ``unbox``, ``field_load``, ``is_null`` are added
to ``VALUE_OPS``.  ``field_store`` and ``safepoint`` are added to
``SIDE_EFFECT_OPS``.

**``ref<T>`` type encoding**:

Heap pointers use the string form ``"ref<T>"`` where ``T`` is the
pointee type.  Examples: ``"ref<u8>"``, ``"ref<any>"``,
``"ref<ref<any>>"``.  Three helpers are exported:

- ``is_ref_type(s) -> bool``
- ``unwrap_ref_type(s) -> str | None`` (``"ref<u8>"`` → ``"u8"``)
- ``make_ref_type(t) -> str`` (``"u8"`` → ``"ref<u8>"``)

``IIRInstr.is_typed()`` was updated to recognise ``ref<T>`` as a
concrete type so the profiler skips them (heap references are not
profiled the way primitives are).

**``IIRInstr.may_alloc: bool`` field**:

A new optional flag (default ``False``) frontends set on any
instruction that may trigger a heap allocation — directly via the
allocating opcodes, or transitively via a ``call`` whose callee
allocates.  vm-core uses this in PR3 to decide where to insert GC
safepoints; for now it is metadata.  ``compare=False`` so it does
not affect ``==``.

**Test coverage**: 140 tests, 100% coverage (was 114, 100%).  Pre-existing
lint warnings on ``feedback_slots`` and ``source_map`` field declarations
fixed in passing.

### Added — LANG17 PR4: optional frontend side tables on IIRFunction

- `IIRFunction.feedback_slots: dict[int, int]` — optional
  ``slot_index → iir_instr_index`` mapping.  Frontends that allocate
  named feedback slots at compile time (Tetrad, SpiderMonkey, V8)
  populate this so a slot index can be resolved back to the IIR
  instruction that owns it.  ``vm-core`` does not interpret this field
  — it is the frontend's contract with its own metric APIs.
- `IIRFunction.source_map: list[tuple[int, int, int]]` — optional
  ``(iir_index, source_a, source_b)`` triples.  Conventional uses:
  ``(iir_index, source_line, source_column)`` for debuggers, or
  ``(iir_index, original_byte_code_ip, 0)`` for legacy-API
  re-projection (used by tetrad-runtime to map Tetrad bytecode IPs
  to IIR IPs).

Both fields default to empty so existing callers see no behaviour
change.  Neither is serialised by `serialise.py` — they are runtime
metadata only, repopulated each time a frontend translates source.

### Added — LANG17 feedback-slot state machine

- `slot_state.py` — `SlotKind` enum (UNINITIALIZED / MONOMORPHIC /
  POLYMORPHIC / MEGAMORPHIC) and `SlotState` dataclass implementing the
  V8 Ignition-style inline-cache state machine.  `SlotState.record()`
  advances the machine; `is_specialisable()`, `is_megamorphic()`, and
  `dominant_type()` are JIT-oriented read helpers.
  `MAX_POLYMORPHIC_OBSERVATIONS = 4` is exposed as a module-level
  constant.
- `IIRInstr.observed_slot: SlotState | None` — new field holding the
  live state machine.  Populated on first call to `record_observation`.
- `IIRInstr.record_observation()` now advances `observed_slot` *and*
  keeps the legacy `observed_type` / `observation_count` fields in
  sync.  Callers that read those legacy fields keep working unchanged;
  callers that want the full four-state view read `observed_slot`.
- `SlotKind`, `SlotState`, and `MAX_POLYMORPHIC_OBSERVATIONS` re-exported
  from the package root.

### Notes

- `SlotState` lives in `interpreter-ir` (not `vm-core` as LANG17's
  first draft suggested) because `IIRInstr.observed_slot` references
  it directly.  Putting the type in `vm-core` would require an import
  cycle (`vm-core` already depends on `interpreter-ir`).  Grouping the
  runtime-observation type with the instruction it annotates also
  matches how `observed_type` and `observation_count` have always
  lived on `IIRInstr`.

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
