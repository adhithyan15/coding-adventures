# Changelog

## 0.5.0 — 2026-04-29 — CLR02 Phase 2c lowering (structural)

### Added — `MAKE_CLOSURE` and `APPLY_CLOSURE` lowering

- New handlers in `_emit_instruction`:
  - **`MAKE_CLOSURE`** → `ldloc capt0; ...; ldloc captN-1;
    newobj Closure_<fn>::.ctor(int32, ...); stloc dst`.
  - **`APPLY_CLOSURE`** → `ldloc closure; ldloc arg0;
    callvirt int32 IClosure::Apply(int32); stloc dst`.
- `CILBackendConfig.closure_free_var_counts` declares which IR
  regions are lifted-lambda bodies; the backend lowers those
  regions as `Apply` methods on auto-generated
  `Closure_<name>` TypeDefs (captures-first prologue copies
  fields and the explicit arg into IR register slots).
- Closure regions are no longer included in
  `CILProgramArtifact.methods` (the main user TypeDef's method
  list); instead they land on their own TypeDefs via
  `extra_types`.
- `_discover_callable_regions` now finds `MAKE_CLOSURE`'s
  `fn_label` operand as a callable region (the lambda body is
  invoked indirectly via `callvirt` rather than directly via
  `CALL`).
- `SequentialCILTokenProvider` gains five closure-aware
  methods (`iclosure_apply_token`, `closure_ctor_token`,
  `closure_apply_token`, `closure_field_token`,
  `system_object_ctor_token`), all returning deterministic
  tokens that match the row layout the writer emits.

### Auto-generated TypeArtifacts

For any program with at least one closure region, the lowerer
emits:

1. `CodingAdventures.IClosure` — abstract interface with one
   abstract instance method `Apply(int32) → int32`.
2. One `CodingAdventures.Closure_<name>` per lifted lambda —
   concrete class extending `System.Object`, implementing
   `IClosure`, with one `int32` field per capture, a `.ctor`
   that chains into `System.Object::.ctor()` and stores
   captures into fields, and an `Apply` instance method whose
   body is the lambda's lowered IR with a captures-from-fields
   prologue.

### v1 limitations

- **Arity-1 closures only** for `APPLY_CLOSURE`.  Multi-arity
  needs `int[]` parameter typing on `IClosure::Apply` plus
  `newarr int32` machinery (which needs a `System.Int32`
  TypeRef the writer doesn't yet emit).
- **Runtime end-to-end is not yet wired.**  Closure references
  are managed pointers but the existing CLR backend uses an
  int32-uniform local/parameter convention, so a closure ref
  stored into an `int32` local truncates the pointer.  The
  full `((make-adder 7) 35) → 42` pipeline lives as
  an `xfail(strict=True)` real-`dotnet` test in
  `cli-assembly-writer` so the structural shape stays right —
  it'll flip to passing when the typed-register pool work
  lands (planned next phase).

### Added — opcode renumbering (compiler-ir 0.5.0)

`MAKE_CLOSURE` moved from opcode 25 → 47 and `APPLY_CLOSURE`
from 26 → 48.  The original numbering collided with `MUL` /
`DIV` (added in compiler-ir 0.2.0); the IR enum's old values
silently shadowed them, causing the lowering dispatcher to
treat closure ops as arithmetic.  This change matches what
ir-to-beam already adopted in its TW03 Phase 2 PR.

## 0.4.0 — 2026-04-29 — CLR02 Phase 2b dataclasses

### Added — closure-shape input to the writer

Foundation for CLR02 Phase 2 closures (the lowering itself —
MAKE_CLOSURE / APPLY_CLOSURE → newobj/callvirt — comes in
Phase 2c).

- `CILTypeArtifact` — describes one extra `TypeDef` row to emit
  alongside the user's main type.  Fields: `name`, `namespace`,
  `is_interface`, `extends`, `implements`, `fields`, `methods`.
- `CILFieldArtifact` — instance field declaration on an extra
  type.  Currently only `int32` and other supported `int*` /
  `bool` / reference-typed fields are accepted.
- `CILMethodArtifact` gains `is_instance` (sets `HASTHIS` in the
  emitted MethodSig blob), `is_special_name` (used by `.ctor`),
  and `is_abstract` (interface methods — RVA=0, no body).
- `CILProgramArtifact.extra_types: tuple[CILTypeArtifact, ...]`
  defaults to `()` so existing callers see no behavior change.

These are pure data-shape additions; the lowering pass in
`backend.py` does not yet emit closure types.

## 0.3.0 — 2026-04-27

### Added — LANG20: `CILCodeGenerator` — `CodeGenerator[IrProgram, CILProgramArtifact]` adapter

**New module: `ir_to_cil_bytecode.generator`**

- `CILCodeGenerator` — thin adapter satisfying the
  `CodeGenerator[IrProgram, CILProgramArtifact]` structural protocol (LANG20).

  ```
  [Optimizer] → [CILCodeGenerator] → CILProgramArtifact
                                       ├─→ PE packager → .exe/.dll  (AOT)
                                       └─→ CLR simulator            (sim)
  ```

  - `name = "cil"` — unique backend identifier.
  - `validate(ir) -> list[str]` — delegates to `validate_for_clr()`.  Never
    raises; returns `[]` for valid programs.  Three rules: opcode support,
    int32 constant range, valid SYSCALL numbers (1/2/10 only).
  - `generate(ir) -> CILProgramArtifact` — delegates to
    `lower_ir_to_cil_bytecode(ir, config)`.  Raises `CILBackendError` on
    invalid IR.
  - Optional `config: CILBackendConfig` — forwarded to the underlying compiler.

- `CILCodeGenerator` exported from `ir_to_cil_bytecode.__init__`.

**New tests: `tests/test_codegen_generator.py`** — 14 tests covering: `name`,
`isinstance(gen, CodeGenerator)` structural check, `validate()` on valid /
bad-SYSCALL / overflow-constant IR, `generate()` returns `CILProgramArtifact`,
artifact has correct `entry_label`, artifact has `>= 1` method, each
`method.body` is `bytes`, `generate()` raises `CILBackendError` on invalid IR,
custom `CILBackendConfig` accepted, round-trip, export check.

### Added — pre-flight validator (released with this version)

- **`validate_for_clr(program)` pre-flight validator**: inspects an `IrProgram`
  for CLR backend incompatibilities *before* any bytecode is generated.  Returns
  a list of human-readable error strings (empty list = valid).  Three rules are
  checked:
  1. **Opcode support** — every opcode must appear in `_CLR_SUPPORTED_OPCODES`.
  2. **Constant range** — `LOAD_IMM` and `ADD_IMM` immediates must fit in a CIL
     `int32` (−2 147 483 648 to 2 147 483 647).
  3. **SYSCALL number** — only SYSCALL 1 (write byte), 2 (read byte), and 10
     (process exit) are wired in the V1 CLR host.  Oct's 8008-specific SYSCALL
     numbers (20+PORT for input, 40+PORT for output) are caught here instead of
     failing silently at runtime.
- `validate_for_clr` exported from `ir_to_cil_bytecode.__init__`.
- `TestValidateForClr` test class (12 new tests in `tests/test_backend.py`)
  covering: passing arithmetic programs, all three wired SYSCALLs (1/2/10),
  int32 boundary values, Oct SYSCALL 57 rejection, Oct SYSCALL 23 rejection,
  SYSCALL 0 rejection, LOAD_IMM above/below int32 range, integration with
  `lower_ir_to_cil_bytecode()`, and multi-error accumulation.

### Changed

- `lower_ir_to_cil_bytecode()` now calls `validate_for_clr()` as a pre-flight
  check before `CILLoweringPipeline` runs.  Any violation raises
  `CILBackendError` with message prefix
  `"IR program failed CLR pre-flight validation"`.  Previously, unsupported
  SYSCALL numbers would pass through to the PE binary and only be caught at
  runtime by the CLR VM.

---

## 0.2.0

- Add `IrOp.OR`, `IrOp.OR_IMM` lowering: emits CIL `or` (0x60).
- Add `IrOp.XOR`, `IrOp.XOR_IMM` lowering: emits CIL `xor` (0x61).
- Add `IrOp.NOT` lowering: emits `ldc.i4.m1` (0x15) + `xor` (0x61), the
  canonical CIL bitwise-complement idiom (`NOT x = x XOR -1`).
- Switch `IrOp.AND`/`IrOp.AND_IMM` emission to use the new `emit_and()`
  builder helper for consistency.
- Add seven new tests covering OR, OR_IMM, XOR, XOR_IMM, NOT, double NOT
  round-trip, and a mixed bitwise-ops method body.

## 0.1.0

- Add the initial composable IR-to-CIL bytecode lowering package.
- Support compiler IR arithmetic, comparisons, branches, calls, static data
  offsets, memory helper calls, and syscall helper calls.
- Expose an injectable token provider so CLI metadata assembly can be composed
  above bytecode lowering.
