# Changelog

## 0.8.0 — 2026-04-30 — per-region parameter typing + obj-aware CALL marshalling

Closes the heap-program runtime gap from Phase 3c.  Twig source
that passes a cons cell (or any obj-typed value) across a function
call now runs correctly on real `dotnet`:

```
(define (length xs)
  (if (null? xs) 0 (+ 1 (length (cdr xs)))))
(length (cons 1 (cons 2 (cons 3 nil)))) → 3
```

Was a NullReferenceException pre-fix because the CLR backend
declared all method parameters as `int32` and ldloc'd args from
the int slot at every CALL site — so any heap reference flowing
through a parameter slot was lost.

### Added — per-region parameter typing

`CILLoweringPlan.function_parameter_types` maps each region name
to a tuple of `"int32"` / `"object"` per parameter slot, computed
by `_classify_function_parameter_types` during analysis.  The
classifier scans each region's body for which slots are obj-typed
and declares them `"object"`.

### Added — obj-source-read inference

`_collect_object_typed_registers` now also pulls in registers
that are *read* as object operands (CAR/CDR/IS_NULL/IS_PAIR/
IS_SYMBOL src; MAKE_CONS tail; APPLY_CLOSURE closure_reg).  This
is critical for parameter slots that the body only ever reads —
without it, the writes-only inference would miss them and the
lowerer would emit `ldloc` from the int32 slot.

### Added — back-prop fixed point through ADD_IMM-0

If `ADD_IMM dst, src, 0` (Twig's canonical move idiom) has an
obj-typed dst (because some downstream op reads dst as obj),
back-propagate to mark src as obj too.  Iterates to a fixed
point.  Catches the "param → holding-reg" copy at the top of
the body — `length` opens with `ADD_IMM v10, v2, 0` where v10
is later read by CDR; v2 (the xs param) needs to be classified
obj so its slot gets declared `"object"` and ldarg → stloc-obj
at function entry.

### Added — seeded type inference from parameter types

`_collect_object_typed_registers` accepts a `seed_types`
keyword.  `_lower_region` seeds it with the region's classified
parameter types so the body's first instruction sees the param
type from the type pool.  Without the seed, `ADD_IMM v10, v2, 0`
at the top of `length` would default v2 to int32 and the move
would copy garbage from the int slot.

### Added — obj-aware CALL marshalling

`_emit_instruction`'s CALL branch now consults the callee's
`function_parameter_types` and ldloc's each arg from the
caller's obj slot when the callee declares that param `"object"`.
Falls back to `ldnull` when the caller doesn't have the slot
in its obj pool (well-formed Twig output always emits an
obj-propagating move first, so the fallback is a safety net).

### Added — obj-aware function entry shuffle

`_lower_region`'s entry shuffle now stores ldarg for obj-typed
params into the obj_local_for slot rather than the int slot,
matching the per-param typing.

### Tests

All 97 ir-to-cil-bytecode tests still pass; coverage 92%.

## 0.7.0 — 2026-04-30 — TW03 Phase 3c (CLR heap primitives, structural)

Adds CLR-side lowering for the eight TW03 Phase 3a heap opcodes
(`MAKE_CONS`, `CAR`, `CDR`, `IS_NULL`, `IS_PAIR`, `MAKE_SYMBOL`,
`IS_SYMBOL`, `LOAD_NIL`).  Ships **structural-only** — bytecode
shape is correct (right opcode bytes, right token-provider
interaction), three new extra TypeArtifacts (Cons / Symbol / Nil)
get auto-included in the multi-TypeDef assembly artifact, but a
follow-up (Phase 3c.5) wires in the cli-assembly-writer-side
intern table for symbol names and the singleton Nil instance.

Mirrors the staging that worked for closures: Phase 2c shipped
structural, Phase 2c.5 wired runtime correctness.

### Added — three runtime TypeDefs auto-included on heap-using programs

- `CodingAdventures.Cons` — `int32 head; object tail` + ctor that
  chains into `System.Object::.ctor()`.  Phase 3c v1 targets
  list-of-ints (the spec acceptance criterion); a follow-up
  widens `head` once typed-register inference covers cons head
  slots.
- `CodingAdventures.Symbol` — `string name` field + ctor.
  Phase 3c v1 emits a placeholder `ldnull` for the name argument
  so two `MAKE_SYMBOL` calls with the same name yield DIFFERENT
  Symbol instances (semantically wrong, bytecode-shape correct).
  Phase 3c.5 wires in the proper `ldstr` UserString token via the
  writer-side intern table.
- `CodingAdventures.Nil` — empty class with no-arg ctor; used as
  the `isinst` target for `IS_NULL`.  Phase 3c v1 emits a fresh
  `newobj Nil` for every `LOAD_NIL` (instead of a singleton);
  `IS_NULL` still works correctly because every `Nil` instance
  qualifies as null under the `isinst Nil` test.

### Added — eight opcode lowerings

| Opcode | CIL emission |
|---|---|
| `MAKE_CONS dst, head, tail` | `ldloc head; ldloc tail (obj slot); newobj Cons.ctor(int32, object); stloc dst (obj slot)` |
| `CAR dst, src` | `ldloc src (obj); castclass Cons; ldfld Cons::head; stloc dst (int)` |
| `CDR dst, src` | `ldloc src (obj); castclass Cons; ldfld Cons::tail; stloc dst (obj)` |
| `IS_NULL dst, src` | `ldloc src (obj); isinst Nil; ldnull; cgt.un; stloc dst (int)` |
| `IS_PAIR dst, src` | `ldloc src (obj); isinst Cons; ldnull; cgt.un; stloc dst (int)` |
| `IS_SYMBOL dst, src` | `ldloc src (obj); isinst Symbol; ldnull; cgt.un; stloc dst (int)` |
| `MAKE_SYMBOL dst, name_label` | `ldnull; newobj Symbol.ctor(string); stloc dst (obj)` (placeholder) |
| `LOAD_NIL dst` | `newobj Nil.ctor(); stloc dst (obj)` |

### Added — token provider extension

`SequentialCILTokenProvider` accepts `include_heap_types=True`
which lays out heap method, field, and TypeDef tokens deterministically
AFTER any closure rows.  See the docstring for the exact layout.

The Protocol gains 9 new methods: `heap_cons_ctor_token`,
`heap_cons_head_token`, `heap_cons_tail_token`,
`heap_symbol_ctor_token`, `heap_symbol_name_token`,
`heap_nil_ctor_token`, `heap_cons_typedef_token`,
`heap_symbol_typedef_token`, `heap_nil_typedef_token`.

### Added — type-tracking rules

`_instr_register_type_writes` now classifies:
- `MAKE_CONS` / `CDR` / `MAKE_SYMBOL` / `LOAD_NIL` dst → object
- `CAR` / `IS_NULL` / `IS_PAIR` / `IS_SYMBOL` dst → int32

Existing typed-register inference + per-instruction slot picking
reuses without changes.

### Tests

- 4 new TestHeapExtraTypes tests verifying auto-include + field
  layouts.
- 11 new TestHeapOpLowering tests verifying each opcode emits the
  identifying CIL bytes.
- 3 new TestHeapTokenLayout tests locking down the deterministic
  token layout.
- All 97 tests pass; coverage 92%.

### Limitations (intentional, scoped to follow-ups)

- Symbol names are placeholder-`ldnull` until 3c.5 wires the
  writer-side UserString intern table.
- `Nil` is a fresh `newobj` per `LOAD_NIL` until 3c.5 wires a
  singleton `INSTANCE` static field.
- `Cons.head` is typed `int32` (matches list-of-ints, the spec
  acceptance criterion).  Heterogeneous cells need a follow-up.
- End-to-end on real `dotnet` is deferred until 3c.5 lands the
  writer-side support.

## 0.6.0 — 2026-04-29 — CLR02 Phase 2c.5 (typed register pool)

### Added — runtime-correct closure semantics

Closes the loop on CLR02 Phase 2 closures.  The headline
`((make-adder 7) 35) → 42` test now **actually runs**
end-to-end on real `dotnet` — previously it shipped as
`xfail(strict=True)` because the existing CLR backend used
int32-uniform locals/parameters and storing a closure ref into
an int32 local truncated the pointer.

### How it works

A new per-region register-typing pass (in
`backend._classify_function_return_types` +
`_collect_object_typed_registers`) walks the IR and computes:

* For each function: does its `r1` register hold an object
  ref at any RET point?  If yes, the function's
  `return_type` is widened to `"object"` so callers can
  `stloc` the result into an object-typed local.  Computed by
  fixed-point iteration so cross-call dependencies converge.
* For each region: which IR registers ever hold an object
  ref?  Those registers get a parallel `object` local
  appended after the existing int32 locals; the lowerer
  picks the slot based on the IR register's type AT THAT
  PROGRAM POINT.

The MOV idiom `ADD_IMM dst, src, 0` propagates the obj-slot
when src is currently object-typed, so closure refs flow
through register copies cleanly.

CALL emission stores the result into r1's obj slot when the
callee returns object; RET reads from r1's obj slot when the
function returns object.  MAKE_CLOSURE stores into the obj
slot; APPLY_CLOSURE reads the closure ref from the obj slot
and stores the int return into the int slot.

### Backwards compatibility

Pure-int functions (no MAKE_CLOSURE / closure-returning
CALLs / object-typed MOVs) get zero object locals appended —
their generated CIL is byte-identical to the pre-2c.5 path.
All 50 twig-clr-compiler tests stay green; the 64
ir-to-cil-bytecode tests pass at 93% coverage.

### Tests

* 6 new structural unit tests cover function return-type
  inference, object-local allocation per region, and the
  pure-int backward-compat path.
* The previously-`xfail` real-`dotnet` test
  `test_make_adder_closure_returns_42_on_real_dotnet` is now
  a regular `@_skip_if_no_dotnet` test that passes.

### Limitations

* Per-region typing analysis is a single linear pass — it
  doesn't yet handle branch-induced type ambiguity (e.g. an
  `if` where one branch writes object and the other writes
  int to the same register).  Closure-using programs
  produced by twig-clr-compiler are straight-line in the
  closure-flowing portion, so this is fine for v1; a proper
  data-flow analysis lands when the frontend exercises it.

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
