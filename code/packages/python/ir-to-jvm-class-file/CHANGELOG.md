# ir-to-jvm-class-file

## 0.15.0 — 2026-04-29 — multi-arity closures (per-region explicit_arity)

The lifted-lambda emission and per-closure subclass `apply([I)I`
forwarder now honour each closure's explicit arity (was
hard-coded arity 1 with a `_CLR_CLOSURE_EXPLICIT_ARITY` constant).

### What changed

* `JvmBackendConfig.closure_explicit_arities: dict[str, int]` — new
  field, parallel to `closure_free_var_counts`.  Maps lifted lambda
  region name → number of source-level explicit args (NOT counting
  captures).  Defaults to 1 per region for back-compat.
* `_build_lifted_lambda_method` reads `explicit_arity` from config
  and emits the static method with `num_free + explicit_arity` int
  params.
* `build_closure_subclass_artifact(..., explicit_arity=1)` — new
  kwarg.  The `apply([I)I` body's prologue forwards `args[0..n-1]`
  for `n = explicit_arity` and `invokestatic`s the lifted lambda's
  static method with the matching descriptor.
* `lower_ir_to_jvm_classes` plumbs each closure's arity through to
  `build_closure_subclass_artifact` from
  `config.closure_explicit_arities`.

### Frontend wiring

`twig-jvm-compiler` now records each lifted lambda's source-level
param count in `closure_explicit_arities` so multi-arg lambdas like
`(lambda (x y) (+ x y))` no longer silently drop the second arg.

## 0.14.0 — 2026-04-30 — heterogeneous cons cells (Cons.head widened to Object)

`Cons.head` is now typed `Object` (was `int`) so cons cells can
hold any Twig value — boxed Int32, Symbol, another cons, closure
ref, nil.  Unblocks AST-shaped data, list-of-symbols, and
nested-cons patterns that any real Lisp program (including a
self-hosted compiler) needs.

### Class-layout change

```java
public final class Cons {
    public final Object head;   // was: int head
    public final Object tail;
    public Cons(Object head, Object tail) { ... }
}
```

### MAKE_CONS lowering

When the head register is obj-typed in the current region, ldload
from the obj slot directly.  When int-typed, read the int and box
via `Integer.valueOf(int) Integer`.  Tail is always Object.

### CAR lowering

Read `Cons.head` as Object.  If dst is obj-typed in the current
region (e.g. `(symbol? (car ...))`), stloc directly into the obj
slot.  If int-typed (the common list-of-ints case), `checkcast
Integer; invokevirtual Integer.intValue()` to unwrap to int.

### Tests

All 113 ir-to-jvm-class-file tests pass; coverage 92%.  Existing
list-of-ints tests like `(length [1,2,3,4,5])` and `(sum [10 20 30])`
continue to pass — they exercise the boxing/unboxing path
transparently.

## 0.13.0 — 2026-04-30 — APPLY_CLOSURE obj-result propagation (3-deep curry)

Closure-returning closures (e.g. `(((mk2 a) b) c)`) now run
end-to-end on real `java`.

### Bug

APPLY_CLOSURE only stored its int result into `__ca_regs[dst]`.
When the callee actually returned a closure ref, the lifted
lambda's body had propagated it into `__ca_objregs[1]` via the
obj-typed RET, but APPLY_CLOSURE didn't carry it onward to
`__ca_objregs[dst]` — so the next
`APPLY_CLOSURE closure_reg=v13` read null.

### Fix

After APPLY_CLOSURE, when the dst register is obj-typed in the
current region (per `_collect_region_obj_regs`), also copy
`__ca_objregs[1] → __ca_objregs[dst]`.  Mirrors the obj-pool
caller-restore's "skip index 1" convention.

## 0.12.0 — 2026-04-30 — gate ADD_IMM-0 obj propagation on per-region typing

Fixes a memory-safety class of bug in the previous obj-pool work:
the unconditional ADD_IMM-0 obj-slot copy was clobbering caller's
`__ca_objregs` slots when an int-typed source got moved.

### Bug

JVM uses a SHARED static `__ca_objregs` array.  v0.11.0's
ADD_IMM-0 obj propagation copied `__ca_objregs[src] →
__ca_objregs[dst]` whenever `_needs_objregs` was True and the
immediate was 0 — regardless of whether the source was actually
obj-typed in that region.

Concrete failure: `(let ((add5 (mk-adder 5))) (+ (add5 10) (add5 27)))`.
Inside the lifted lambda body: `ADD_IMM v11, v3, 0` (v3 is the
explicit int arg).  Pre-fix this wrote `__ca_objregs[3] (= null)`
into `__ca_objregs[11]`, clobbering the closure ref the CALLER
had stored there.  Second `(add5 27)` then NPE'd reading
`__ca_objregs[11]`.

### Fix — `_collect_region_obj_regs`

New per-region analysis (mirrors the CLR backend's typed-pool
approach, just adapted to JVM's static layout).  Computes the
set of registers each region uses obj-style by:

* **Writes** producing object refs (MAKE_CLOSURE, MAKE_CONS,
  MAKE_SYMBOL, LOAD_NIL, CDR).
* **Reads** consuming object operands (CAR/CDR/IS_*/MAKE_CONS-tail/
  APPLY_CLOSURE-closure_reg).
* **Back-prop** through ADD_IMM-0 to a fixed point.

ADD_IMM-0 obj propagation now fires only when SRC is in the
region's obj_regs set — int-typed sources stop polluting the
shared obj pool.

### Tests

All 113 ir-to-jvm-class-file tests pass; coverage 92%.  No new
test cases — the fix's correctness is validated by the
twig-jvm-compiler tests for closure / heap / mutual-recursion
patterns that exercise the cross-region obj flow.

## 0.11.0 — 2026-04-30 — obj-pool caller-saves + ADD_IMM-0 obj propagation

Closes the obj-pool gap left by JVM Phase 3b.  Recursive heap
programs (e.g. `(length (cons 1 (cons 2 (cons 3 nil))))`) now run
correctly on real `java`.

### Fix — obj-pool caller-saves around CALL

The JVM01 caller-saves convention snapshots `__ca_regs` (int pool)
into JVM locals before each CALL and restores it after, so
recursion through int registers works.  But the obj pool
(`__ca_objregs` — cons cells, symbols, nil sentinels, closure
refs) was uncovered.  Recursion through any obj-typed register
clobbered the caller's reference when the recursive call's body
wrote the same slot.

This change extends the existing pattern to the obj pool:

- `_emit_caller_save_objregs` snapshots `__ca_objregs[0..N-1]`
  into JVM ref locals `N..2N-1` (using `aload`/`astore`).
- `_emit_caller_restore_objregs` writes them back, skipping
  index 1 (same convention as the int-pool restore — closure-
  returning functions put the result in `__ca_objregs[1]`).
- Triggered only when `_needs_objregs` is True.  Pure-int
  programs see zero extra emission.
- `max_locals` doubles to `2 * reg_count` when triggered.

### Fix — ADD_IMM-0 (move idiom) propagates obj slot

Twig compilers use `ADD_IMM dst, src, 0` as the canonical
register-move idiom.  Pre-fix it only copied the int half of the
register, so any object reference in the source's obj slot was
lost.  This worked *by accident* — `make_adder` happened to write
its closure to a holding reg whose index matched the one `_start`
then read.  Once obj-pool caller-saves landed, that accident
broke.

Fix: when `_needs_objregs` is True and the immediate is 0, also
emit `__ca_objregs[dst] = __ca_objregs[src]` after the int copy.
Net: `ADD_IMM dst, src, 0` is now a true register-to-register
copy that propagates BOTH int and obj slots.

### Tests

All 113 ir-to-jvm-class-file tests still pass; coverage 93%.  The
previously xfail-strict
`twig_jvm_compiler.test_real_jvm.test_heap_list_of_ints_length`
now passes (XPASS unblocks; xfail marker removed in
twig-jvm-compiler v0.4.0).

## 0.10.0 — 2026-04-30 — TW03 Phase 3b (heap primitives on real java)

Implements the JVM-side lowering for the eight TW03 Phase 3a heap
opcodes (`MAKE_CONS`, `CAR`, `CDR`, `IS_NULL`, `IS_PAIR`,
`MAKE_SYMBOL`, `IS_SYMBOL`, `LOAD_NIL`).  A program that uses any
heap opcode now compiles to a multi-class JAR that runs on stock
`java` and produces correct list-walking output.

### Added — three runtime classes auto-included by the multi-class lowering

* `coding_adventures.twig.runtime.Cons` — `int head; Object tail`
  pair.  Phase 3b v1 targets list-of-ints (the spec acceptance
  criterion); a follow-up phase widens `head` to `Object` once
  typed-register inference covers the head slot.  Built by
  `build_cons_class_artifact()`.
* `coding_adventures.twig.runtime.Symbol` — interned identifier;
  `String name` plus a `static HashMap<String,Symbol>`-backed
  `intern(String) Symbol` method.  Single-threaded — Twig has no
  concurrency surface today.  Built by
  `build_symbol_class_artifact()`.
* `coding_adventures.twig.runtime.Nil` — singleton sentinel via
  `public static final Nil INSTANCE` and a private no-arg ctor.
  Built by `build_nil_class_artifact()`.

### Added — eight opcode lowerings

| Opcode | Lowering shape |
|---|---|
| `MAKE_CONS dst, head, tail` | `new Cons; dup; iload head; aload tail; invokespecial Cons.<init>(I,LObject;)V; aastore __ca_objregs[dst]` |
| `CAR dst, src` | `aaload __ca_objregs[src]; checkcast Cons; getfield head:I → __ca_regs[dst]` |
| `CDR dst, src` | `aaload __ca_objregs[src]; checkcast Cons; getfield tail:Object → __ca_objregs[dst]` |
| `IS_NULL dst, src` | `aaload __ca_objregs[src]; getstatic Nil.INSTANCE; if_acmpne; iconst_1 / iconst_0 → __ca_regs[dst]` |
| `IS_PAIR dst, src` | `aaload __ca_objregs[src]; instanceof Cons → __ca_regs[dst]` |
| `IS_SYMBOL dst, src` | `aaload __ca_objregs[src]; instanceof Symbol → __ca_regs[dst]` |
| `MAKE_SYMBOL dst, name_label` | `ldc "name"; invokestatic Symbol.intern(String) Symbol; aastore __ca_objregs[dst]` |
| `LOAD_NIL dst` | `getstatic Nil.INSTANCE; aastore __ca_objregs[dst]` |

### Added — multi-class auto-include

`lower_ir_to_jvm_classes` now scans the program for any heap
opcode and appends `Cons`, `Symbol`, `Nil` artifacts to the
returned `JVMMultiClassArtifact` when any are found.  Closure-free
heap-free programs see zero extra emission — the existing
single-class flow is unchanged.

### Added — `__ca_objregs` triggered by heap ops

The parallel `Object[] __ca_objregs` static field (introduced for
closures in Phase 2c.5) is now also triggered by any heap opcode.
Cons / Symbol / Nil values share the slot pool with closures.

### Test coverage

* 9 new unit tests covering each runtime class layout and each
  opcode's lowering byte fingerprints.
* New real-`java` end-to-end test:
  `(length (cons 1 (cons 2 (cons 3 nil)))) → 3` runs on real
  `java` and exits with stdout = `\x03`.
* All 98 tests pass; coverage 93%.

### Limitations (intentional, scoped to follow-up)

* `Cons.head` is typed `int` (matches list-of-ints).
  Heterogeneous cells (`(cons 'foo nil)`, nested cons) need a
  follow-up phase that widens `head` to `Object` and threads
  Integer boxing through the type-aware register pool.
* `Symbol.intern` uses a plain `HashMap` (single-threaded only).

## 0.9.0 — 2026-04-29 — JVM02 Phase 2c.5 (typed register pool, end-to-end)

Closes the loop on JVM02 Phase 2 closures.  The headline
`((make-adder 7) 35) → 42` test now **actually runs** end-to-end
on real `java -jar` — previously it shipped as
`xfail(strict=True)` because the existing JVM backend used a
static `int[] __ca_regs` shared across method invocations and
storing a closure ref there truncated the pointer.

### How it works

* **Parallel `Object[] __ca_objregs` static field** on the main
  class, initialized in `<clinit>` (only when at least one
  closure is declared).  MAKE_CLOSURE writes the new ref into
  `__ca_objregs[dst]`; APPLY_CLOSURE reads it back via
  `aaload + checkcast Closure`.
* **Lifted lambdas live on the main class** as PUBLIC static
  methods with widened arity (`num_free + explicit_arity`,
  matching the BEAM/CLR captures-first convention).  A
  one-time prologue copies their JVM args into
  `__ca_regs[REG_PARAM_BASE+i]` so the existing IR-body
  emitter runs unchanged.
* **Closure subclass `apply` body forwards** to the lifted
  lambda via `invokestatic Main._lambda_N(I,I,…)I` — pushes
  captures from `this.captI` fields, pushes explicit args from
  the `int[]` parameter, calls, ireturns the int.  Cross-class
  resolution works because the main-class lambda is public.
* **MAKE_CLOSURE** emits `getstatic __ca_objregs; ldc dst;
  new Closure_<fn>; dup; iload caps; invokespecial <init>;
  aastore`.
* **APPLY_CLOSURE** emits `getstatic __ca_objregs; ldc
  closure_reg; aaload; checkcast Closure; build int[] args;
  invokeinterface Closure.apply([I)I; ldc dst; swap;
  invokestatic __ca_regSet`.

### Backwards compatibility

Pure-int programs (no `closure_free_var_counts` entries) get
zero extra emission: no `__ca_objregs` field, no `<clinit>`
initialization for it, no extra methods.  All existing JVM
tests (75 pre-existing) continue to pass.

### Tests (8 new, 83 total + 2 pre-existing brainfuck/nib failures at 87% coverage)

* `test_objregs_field_present_when_closures_declared` — the
  parallel `Object[]` field appears in the constant pool.
* `test_objregs_field_absent_in_pure_int_program` — no extra
  emission for non-closure programs.
* `test_lifted_lambda_emitted_as_public_method_on_main` —
  `_lambda_0` lives on the main class with descriptor `(II)I`
  and `ACC_PUBLIC` set.
* `test_main_class_callable_labels_includes_lambda` — JAR
  builders see the lifted lambda in `callable_labels`.
* `test_subclass_apply_forwards_via_invokestatic` — subclass
  `apply` is a real forwarder, not the placeholder.
* `test_make_closure_uses_aastore_into_objregs` — MAKE_CLOSURE
  emits `aastore` (0x53).
* `test_apply_closure_uses_aaload_and_checkcast` —
  APPLY_CLOSURE emits `aaload` (0x32) + `checkcast` (0xC0).
* `test_phase2c_make_adder_closure_returns_42_on_real_java`
  is now a regular passing test (was `xfail(strict=True)`).

### Limitations

* **Caller-saves on the obj pool**: the existing JVM01
  caller-saves (snapshot/restore around CALL) only covers
  `__ca_regs`, not `__ca_objregs`.  For straight-line
  closure-flow code (matches what twig-jvm-compiler will
  produce for v1) this is fine — closure refs flow through
  unique slots.  Multi-closure-in-flight scenarios would need
  parallel obj-pool caller-saves; document this for the
  follow-up.
* **Arity-1 closures only** — APPLY_CLOSURE supports a single
  explicit arg.  Multi-arity awaits Phase 2c.6.

## 0.8.0 — 2026-04-29 — JVM02 Phase 2c (closure lowering, structural)

### Added — `MAKE_CLOSURE` and `APPLY_CLOSURE` lowering

Builds on Phase 2b multi-class scaffolding.  The lowerer now:

* **Auto-generates per-lambda `Closure_<name>.class`
  artifacts** via `build_closure_subclass_artifact()` —
  fields per capture (`capt0`, `capt1`, …),
  `.ctor(int, …)V` chaining into `Object::.ctor()` and
  storing each capture into its field, and a placeholder
  `apply([I)I` body (`iconst_0; ireturn`).
* **Lowers MAKE_CLOSURE** to `new Closure_<fn>; dup; iload
  caps from __ca_regs; invokespecial ctor`.  The resulting
  reference is **popped** (not stored) — the existing
  static `int[] __ca_regs` register convention can't hold
  object refs.  Phase 2c.5 will add a parallel `Object[]`
  pool to retain the reference.
* **Lowers APPLY_CLOSURE** to `aconst_null` (placeholder
  receiver) `; build int[] args; invokeinterface
  Closure.apply([I)I; pop`.  The `invokeinterface` operand
  format (count = 2: receiver + int[]) is correct; the
  bytecode verifies cleanly even though it would NPE at
  runtime today.
* **Routes closure regions away from the main user class** —
  the lifted lambda body lives on the per-lambda
  `Closure_<name>` subclass via the multi-class artifact,
  not as a method on the main class.
* `JvmBackendConfig.closure_free_var_counts` declares which
  IR regions are lifted lambdas; the lowerer uses this to
  detect closure regions, dispatch them to subclass emission,
  and validate `MAKE_CLOSURE` operand counts.
* `lower_ir_to_jvm_classes()` automatically appends the
  `Closure` interface + per-lambda subclasses when
  `closure_free_var_counts` is non-empty (no need to set
  `include_closure_interface=True`).

### V1 limitations (documented in the JVM02 spec)

* **Runtime end-to-end is not yet wired.**  The headline
  `((make-adder 7) 35) → 42` test ships as
  `xfail(strict=True)` so the structural shape stays right
  and it'll auto-flip to passing once Phase 2c.5 lands the
  typed register pool + cross-class register access.
* **Placeholder `apply` body**: every Closure subclass's
  `apply` returns 0 today.  Phase 2c.5 wires the real body
  through.

### Tests (8 new, 75 total + 1 xfail at 86% coverage)

* 8 structural tests cover multi-class artifact contents
  (interface + subclass), `callable_labels` filtering of
  lambda regions, subclass parsing through `jvm-class-file`,
  zero-captures edge case, two validation paths
  (unknown-lambda, capture-count mismatch), and bytecode
  presence (`new` + `invokeinterface` opcodes appear in the
  expected places).
* 1 `xfail(strict=True)` real-`java` JAR test packs main +
  interface + subclass and runs them through `java -jar`.
* All Phase 2b tests continue to pass — the multi-class API
  surface is purely additive.

### Validator

`validate_for_jvm` now accepts `MAKE_CLOSURE` and
`APPLY_CLOSURE` opcodes (per-instruction validation runs in
the lowerer instead).

## 0.7.0 — 2026-04-29 — JVM02 Phase 2b (multi-class scaffolding)

### Added — `JVMMultiClassArtifact` and the `Closure` interface

Foundation for [JVM02 Phase 2 closures](../../../specs/JVM02-phase2-multi-class-closure-lowering.md).
The single-class `JVMClassArtifact` is enough for IR programs
that don't use closures; closure-enabled programs need to ship
a shared `Closure` interface plus per-lambda `Closure_<name>`
subclasses alongside the user's main class.

This phase ships the data shape + the interface; per-lambda
subclass emission lands in Phase 2c (with the actual
`MAKE_CLOSURE` / `APPLY_CLOSURE` IR-op lowering).

* `JVMMultiClassArtifact` dataclass — wraps
  `tuple[JVMClassArtifact, ...]` with a stable invariant that
  `classes[0]` is always the main user class.  Exposes
  `.main` and `.class_filenames` for JAR builders.
* `build_closure_interface_artifact()` returns a
  `JVMClassArtifact` for the `Closure` interface, byte-rolled
  per JVMS §4.1 — `ACC_PUBLIC | ACC_INTERFACE | ACC_ABSTRACT`,
  one abstract method `int apply(int[] args)`.  The class
  binary name is fixed at
  `coding_adventures/twig/runtime/Closure` so future
  closure-aware artifacts can reference it via a stable symbol.
* `lower_ir_to_jvm_classes(program, config, *,
  include_closure_interface=False)` — multi-class API that
  always returns the main class and, when opted in, appends
  the `Closure` interface.

Three new public constants exported from `__init__`:
`CLOSURE_INTERFACE_BINARY_NAME`,
`CLOSURE_INTERFACE_METHOD_NAME` (`"apply"`), and
`CLOSURE_INTERFACE_METHOD_DESCRIPTOR` (`"([I)I"`).

### Tests

* 6 structural tests cover the multi-class artifact shape, the
  `main`-first invariant, the `class_filenames` JAR-path
  format, and round-tripping the interface bytecode through
  `jvm-class-file`'s decoder.
* 1 real-`java` JAR conformance test packs the main user
  class + the `Closure` interface into a JAR via
  `jvm-jar-writer` and proves real `java -jar` loads both
  without `ClassFormatError` / `VerifyError`.

### Backwards compatibility

`lower_ir_to_jvm_class_file` is unchanged; existing tests
stay green.  The new multi-class API is purely additive.

## 0.6.1 — 2026-04-28

### Fixed — JVM01: caller-saves around `IrOp.CALL` so recursion works on real `java`

The "register" model uses a class-level static `int[]` array
shared across every `invokestatic`, so a recursive call would
clobber the caller's register values.  `IrOp.CALL` emission now
snapshots every register slot into JVM locals immediately before
the `invokestatic` and restores them immediately after (skipping
`r1`, the return-value slot).  Each callable's `max_locals` is
bumped to `reg_count` to cover the snapshot stash.

This is the minimal-diff path described in
`code/specs/JVM01-jvm-per-method-locals.md` — the bigger
descriptor rewrite stays as a future cleanup option.

Regression test:
`tests/test_oct_8bit_e2e.py::test_call_preserves_caller_registers`.

## 0.6.0 — 2026-04-27

### Added — LANG20: `JVMCodeGenerator` — `CodeGenerator[IrProgram, JVMClassArtifact]` adapter

**New module: `ir_to_jvm_class_file.generator`**

- `JVMCodeGenerator` — thin adapter satisfying the
  `CodeGenerator[IrProgram, JVMClassArtifact]` structural protocol (LANG20).

  - `name = "jvm"` — unique backend identifier.
  - `validate(ir) -> list[str]` — delegates to `validate_for_jvm()`.  Never
    raises; returns `[]` for valid programs.
  - `generate(ir) -> JVMClassArtifact` — delegates to
    `lower_ir_to_jvm_class_file(ir, config)`.  Raises on invalid IR.
  - Optional `config: JvmBackendConfig` — forwarded to the underlying compiler.

- `JVMCodeGenerator` exported from `ir_to_jvm_class_file.__init__`.

**New tests: `tests/test_codegen_generator.py`** — 14 tests covering: `name`,
`isinstance(gen, CodeGenerator)` structural check, `validate()` on valid / bad-
SYSCALL / overflow-constant IR, `generate()` returns `JVMClassArtifact`,
`class_bytes` starts with JVM magic `0xCAFEBABE`, `class_bytes` non-empty,
`generate()` raises on invalid IR, custom config accepted, round-trip, export
check.

---

## [Unreleased]

### Added

- **Oct 8-bit arithmetic e2e tests** (`tests/test_oct_8bit_e2e.py`):
  7 end-to-end tests confirming the JVM backend correctly compiles and
  executes 8-bit integer arithmetic IR — the same IR that the Oct compiler
  generates.  Tests cover: LOAD_IMM, ADD, SUB, AND (inc. 0xFF masking),
  multi-output programs, and validation of Oct's unsupported SYSCALL numbers.
  Execution uses the system ``java`` binary; tests are skipped if ``java``
  is not on PATH.  Key findings:
  - Pure 8-bit arithmetic compiles to standard JVM .class files and runs
    correctly through the full IR → JVM → java subprocess pipeline.
  - Oct's I/O intrinsics (SYSCALL 40+PORT / 20+PORT) are correctly rejected
    by the JVM validator.  The JVM backend only supports SYSCALL 1 and 4.

## 0.5.0 — 2026-04-20

### Added

- **`IrOp.OR` support**: emits `ior` (`0x80`) for register-register bitwise OR.
- **`IrOp.OR_IMM` support**: emits `ior` (`0x80`) for register-immediate bitwise OR.
- **`IrOp.XOR` support**: emits `ixor` (`0x82`) for register-register bitwise XOR.
  New opcode constant `_OP_IXOR = 0x82` added alongside the existing `_OP_IOR`.
- **`IrOp.XOR_IMM` support**: emits `ixor` (`0x82`) for register-immediate bitwise XOR.
- **`IrOp.NOT` support**: emits the source register value, then `iconst_m1` (`0x02`)
  to push -1 (all 32 bits set), then `ixor` (`0x82`) to flip every bit.  This
  correctly implements two's-complement bitwise NOT: `NOT(x) = x XOR 0xFFFFFFFF`.
- All five new opcodes added to `_JVM_SUPPORTED_OPCODES` so the pre-flight
  validator accepts them without error.
- 15 new tests in `TestValidateForJvm` covering:
  - Validator acceptance of OR, XOR, NOT, OR_IMM, XOR_IMM.
  - Successful lowering to structurally-valid class files for all five ops.
  - Bytecode-presence checks confirming `ior` (0x80) and `ixor` (0x82) appear
    in generated output, and that NOT specifically emits `iconst_m1` + `ixor`.
- Removed the `_BITWISE_V1_UNSUPPORTED` frozenset and the
  `test_bitwise_opcodes_are_intentionally_unsupported` test now that all five
  ops are implemented.  `test_all_supported_opcodes_pass_opcode_check` now
  iterates every `IrOp` without exclusions.

### Motivation

These opcodes were blocking end-to-end Oct → JVM compilation: Oct programs use
the `|`, `^`, and `~` operators which lower to `OR`, `XOR`, and `NOT` in
`compiler_ir`.  All three map directly to single JVM instructions.

## 0.4.0 — 2026-04-20

### Added

- **`validate_for_jvm(program)` pre-flight validator**: inspects an
  `IrProgram` for JVM backend incompatibilities *before* any bytecode is
  generated.  Returns a list of human-readable error strings (empty list =
  valid).  Three rules are checked:
  1. **Opcode support** — every opcode must appear in the V1 supported set.
     Currently all `IrOp` values are handled; the check is future-proofing
     against new IR opcodes added before the JVM backend implements them.
  2. **Constant range** — `LOAD_IMM` and `ADD_IMM` immediates must fit in a
     JVM 32-bit signed integer (−2 147 483 648 to 2 147 483 647).
  3. **SYSCALL number** — only SYSCALL 1 (write byte) and SYSCALL 4 (read
     byte) are wired up in the V1 JVM backend.
- `validate_for_jvm` exported from `ir_to_jvm_class_file.__init__`.
- `TestValidateForJvm` test class (14 tests) covering all three rules,
  boundary-value constants, multi-error accumulation, and integration with
  `lower_ir_to_jvm_class_file`.

### Changed

- `lower_ir_to_jvm_class_file()` now calls `validate_for_jvm()` as a
  pre-flight check before `_JvmClassLowerer` runs.  Any violation raises
  `JvmBackendError` with message prefix
  `"IR program failed JVM pre-flight validation"`.

## 0.3.0 — 2026-04-20

### Changed

- **`JvmBackendConfig.syscall_arg_reg` field removed.**  The SYSCALL IR
  instruction now carries the arg register as `operands[1]` (an `IrRegister`),
  so the backend reads the register index directly from the instruction rather
  than from a config parameter.  Callers no longer need to pass
  `syscall_arg_reg=0` for BASIC or `syscall_arg_reg=4` for Brainfuck.

- **`__ca_syscall` helper descriptor changed from `(I)V` to `(II)V`.**
  The helper method now accepts two `int` parameters: syscall number and
  arg-register index.  The WRITE path loads `__ca_regs[arg_reg]` at runtime
  using the passed-in register index instead of a compile-time constant.
  The READ path stores the byte received from stdin in local 2 (shifted up from
  local 1 to make room for the new arg-register parameter in local 1).
  Max locals increased from 2 to 3 accordingly.

## 0.2.0 — 2026-04-19

### Added

- `IrOp.MUL` support: emits `imul` (`0x68`) so Dartmouth BASIC multiplication
  expressions (`LET A = B * C`, `PRINT 3 * I`) lower correctly to JVM bytecode.
- `IrOp.DIV` support: emits `idiv` (`0x6C`) so Dartmouth BASIC integer division
  expressions lower correctly.  Integer division truncates toward zero, matching
  Dartmouth BASIC semantics.

## 0.1.0 - 2026-04-17

- Add the initial Python prototype for lowering `compiler_ir.IrProgram` to JVM
  class-file bytes.
- Emit a single generated class with static register and memory fields.
- Add helper-method-based lowering for byte memory, word memory, syscalls,
  branching, arithmetic, and comparisons.
- Add frontend integration tests for Brainfuck and Nib IR producers.
- Add GraalVM runtime smoke tests that execute generated Brainfuck and Nib
  programs on a locally installed GraalVM JDK and compile them with
  `native-image`.
- Harden class-name validation and class-file writes so malformed names cannot
  escape the requested output directory.
- Flush stdout in the generated write syscall helper so captured JVM/native
  output is observable in end-to-end tests.
- Fix the package `BUILD` file to install Brainfuck's transitive
  `virtual-machine` dependency during local test setup.
- Declare the package's test-only sibling dependencies in `pyproject.toml`
  so the build validator accepts the BUILD graph during CI, and remove the
  now-redundant standalone `grammar-tools` editable install from `BUILD`.
- Rewrite `write_class_file()` to anchor output writes on directory file
  descriptors and reject symlinked path components, closing a symlink-race
  overwrite hole in the original output-path validation.
- Bound total static data size and switch non-zero data initialization to
  compact `java.util.Arrays.fill()` range calls so hostile IR cannot explode
  the generated class initializer into a denial-of-service sized method body.
