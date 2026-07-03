# McCarthy LISP — Full Platform Implementation Matrix

> ## ✅ COMPLETE — McCarthy 1960 LISP runs F1–F7 on all eight LANG VM backends.
> Every feature (scalar, cons/car/cdr, ATOM, EQ, COND, symbols, LAMBDA/LABEL/
> recursion) runs on VM, native AOT, JIT, WASM, JVM, CLR, BEAM, and LLVM. The W16
> conformance suite proves uniformity: one source × 8 backends × 19 programs → one
> answer. All W1–W16 worklist items are shipped.

**Goal:** a *complete* McCarthy 1960 LISP on **every** backend the LANG VM
supports — VM, AOT (native), JIT, WASM, JVM, CLR, BEAM, LLVM — so the same
`.lisp` source runs identically everywhere.

This document is the **ordered worklist** for the autonomous build loop. Each row
is a shippable, independently-reviewable slice. The loop picks the first
un-shipped (`☐`) item, implements it (specs → tests → impl → changelog → README →
security review → PR), babysits the PR to green, and on merge advances to the
next. Status legend: `✅` done · `🔄` in progress · `☐` not started.

See [`MCCARTHY-LISP-PLAN.md`](MCCARTHY-LISP-PLAN.md) for the per-phase L1–L7
history (parser, VM, native AOT). This matrix is the cross-backend completion
plan that supersedes the "Then replicate to jvm/clr/beam" note there.

---

## The McCarthy core (the feature set every backend must run)

| # | Feature | Example | Notes |
|---|---------|---------|-------|
| F1 | Scalar / integer atoms | `42` | exit/return the integer |
| F2 | `CONS` / `CAR` / `CDR` | `(CAR (CONS 7 9))` → 7 | the heap pair |
| F3 | `ATOM` / `pair?` | `(ATOM 5)` → T | is-a-cons type test |
| F4 | `EQ` | `(EQ 5 5)` → T | atom equality |
| F5 | `COND` | `(COND (0 7) (5 9))` → 7 | lisp-truthiness (only `nil`/`#f` false) |
| F6 | Symbols / `QUOTE` | `(EQ 'A 'A)` → T | interned symbol identity |
| F7 | `LAMBDA` / `LABEL` / user calls + recursion | `((LAMBDA (X) (CONS X X)) 5)`; recursive `LABEL` | closures, bound params, self-call |

A backend is **McCarthy-complete** when F1–F7 all run, verified end-to-end (run
the emitted artifact and assert the result), plus a DoS/termination guard on any
new recursion/loop over untrusted input.

## The two value models

- **Tagged-word model** (native-ish backends: AOT, LLVM, JIT). Every lisp value
  is one machine word with low-bit tags (`lispy-runtime`'s NaN-box: int `n<<3`,
  nil `0b001`, symbol `(id<<32)|0b010`, cons = tagged pointer). Shared C runtime
  `lispy_runtime.c` (LANG77) provides `cons`/`car`/`cdr`/`pair?`/`eq`/symbols.
- **Uniform-reference model** (managed backends: WASM, JVM, CLR, BEAM). Every
  lisp value is the platform's universal reference (`anyref` / `Object` /
  `object` / Erlang term); a small integer is boxed (`i31ref` / `Integer` /
  boxed `int` / Erlang integer), a cons is a 2-field object/tuple, nil is the
  null/empty reference, predicates are type tests. The WASM implementation
  (LANG77 L3b-3a) is the **reference design**; JVM/CLR/BEAM replicate its passes
  (`lower_lisp_repr_structural` + the per-builtin lowerings) with the platform's
  object/boxing ops.

Both share the frontend (`mccarthy-lisp-{lexer,parser,iir-compiler}`) and the
structural heap lowering (`lower_heap_builtins`). The fork is only the value
representation + per-builtin backend lowering.

---

## Backend status matrix

| Backend | Crate(s) | F1 | F2 | F3 | F4 | F5 | F6 | F7 | Value model |
|---------|----------|----|----|----|----|----|----|----|-------------|
| **VM** | `mccarthy-lisp-vm` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | tagged (interpreter over `lispy-runtime`) |
| **AOT native** | `twig-aot` + `aarch64`/`x86_64-backend` + `lispy_runtime.c` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | tagged-word |
| **WASM** | `iir-to-wasm` + `wasm-*` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | uniform-anyref |
| **JVM** | `iir-to-jvm-class-file` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | uniform-Object |
| **CLR** | `iir-to-cil-bytecode` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | uniform-object |
| **BEAM** | `iir-to-beam` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | Erlang terms |
| **LLVM** | `iir-to-llvm` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | tagged-word |
| **JIT** | `jit-core` + `lispy-runtime` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | tagged-word |

(F-cell `✅` = verified end-to-end; `☐` = not yet. The VM and native AOT are the
furthest along; the managed backends are the frontier.)

---

## Ordered worklist (the loop advances through this)

Each `W#` is one PR-sized slice. Sub-items split when an item is large; the loop
slices further as needed (as it did for L3b-3a). After each item, tick its
F-cell(s) in the matrix above and add a row to the relevant CHANGELOG(s).

### Phase A — finish WASM (the reference uniform-ref backend)

- ✅ **W1 — WASM symbols (F6).** `iir-builtin-lowering::intern_symbols_structural`
  (the managed twin of native `intern_symbols`) interns each symbol literal to a
  distinct integer in a reserved range (`SYMBOL_ID_BASE = 2²⁹` + module-wide id),
  retyped to `i32` so it boxes as an `i31ref` and `EQ` compares with `i32.eq` —
  no new value type, no polymorphic `EQ`. `lang-aot::compile_source_to_wasm` runs
  it before the repr pass. `(EQ 'A 'A)` → T, `(EQ 'A 'B)` → nil, `(EQ 'A 5)` → nil
  (disjoint range), symbols through cons + `COND`, end-to-end on `wasm-runtime`.
- ✅ **W2 — WASM `LAMBDA`/`LABEL`/user calls + recursion (F7).** The frontend
  already lifts each `LAMBDA`/`LABEL` to its own function + `call`; the structural
  pass now makes the **call boundary uniform-anyref** — a new `lisp_functions`
  module analysis (heap/predicate/lisp-param users, closed under *calling*)
  replaces the per-function gate; lisp **params** retype to `ref<any>`, **call
  args** box to `i31ref`, **call results** are references, and a **non-entry**
  function returns `ref<any>` (boxing a scalar/bool/atom). Recursion is just a
  self-`call`, so it needs nothing extra. `((LAMBDA (X) X) 5)` → 5,
  `(CDR ((LAMBDA (X Y) (CONS X Y)) 3 4))` → 4, `((LAMBDA (X) (EQ X X)) 5)` → T,
  and a recursive `LABEL` walking a list to its atom — all end-to-end on
  `wasm-runtime`. **WASM is now McCarthy-complete (F1–F7).**

### Phase B — JVM (replicate the uniform-ref model as `Object`)

- ✅ **W3a — JVM run-foundation (F1, scalar).** `lang-aot::compile_source_to_jvm` /
  `compile_file_to_jvm` + `concretize_scalar_any_for_jvm` (scalar `any`/`i64` →
  JVM `i32`) → `iir-to-jvm-class-file` → a serialized `.class`. **Verified by
  RUNNING** — parse the emitted bytes (`jvm-class-file::parse_class_file`) and run
  the entry method on the in-repo **`jvm-simulator`** (zero external `java`,
  mirroring `wasm-runtime`): `42`→42, `0`→0, `7`→7, Twig `42`→42. Establishes the
  JVM pipeline + run-verify harness that W3b+ build on.
- ✅ **W3b — JVM cons (F2).** cons cells are `Object[]` (the JVM backend already
  lowered `alloc`/`field_*` for `ref<LispyPair>` → `anewarray`/`aastore`/`aaload`);
  this slice added the missing **atom boxing** — `box` → `Integer.valueOf(I)`,
  `unbox` → `checkcast Integer` + `intValue()` — plus the `ref<any>`→`Object` type,
  and wired `lang-aot::compile_source_to_jvm` to run the *same* structural passes
  as wasm. The pass output is **backend-agnostic** (`box`/`unbox`/`alloc`/`field_*`);
  wasm lowers to `i31ref`/`$LispyPair`, JVM to `Integer`/`Object[]` — the reusable
  primitive. **Verified on the real `java`** (Temurin 21; cons cells are `Object[]`
  the `jvm-simulator` can't run): `(CAR (CONS 7 9))`→7, `(CDR (CONS 7 9))`→9,
  nested cons→2, via an injected `main` launcher (`tests/jvm_cons.rs`).
- ✅ **W4 — JVM `ATOM`/`EQ`/`COND` (F3–F5).** The JVM backend lowers the *shared*
  structural-pass predicates: `pair?` → `instanceof [Ljava/lang/Object;` (a cons
  is an `Object[]`), `not` → `ixor 1`, `equal?` → unbox both `Integer`s + a
  `if_icmpeq`-synthesised 0/1. `jmp_if_false`/`is_null` were already lowered, so
  `COND` works too. Verified on the real `java` (`tests/jvm_predicates.rs`, a
  **descriptor-aware** launcher: predicate→`()I`, COND-over-int→`()J`):
  `(ATOM 5)`→1, `(ATOM (CONS 1 2))`→0, `(EQ 5 5)`→1, `(EQ 5 6)`→0,
  `(COND ((EQ 1 1) 7) (5 9))`→7, fall-through→9.
- ✅ **W5a — JVM symbols (F6) + the large-`int` `ldc` fix.** The JVM `const`
  lowering emitted `ldc 0` (the *reserved* CP slot) for any `int` beyond ±32767 —
  which crashed real JVMs (`constantTag.cpp ShouldNotReachHere`). Added
  `emit_iconst_cp` (a `CONSTANT_Integer` entry + `ldc`/`ldc_w`) and routed every
  user-constant site through it. A symbol id (`SYMBOL_ID_BASE = 2²⁹`) is exactly
  that large const, so symbols now run: `(EQ 'X 'X)`→1, `(EQ 'X 'Y)`→0,
  `(QUOTE X)`→its id, `(ATOM 'X)`→1, on a real `java` (`tests/jvm_symbols.rs`).
  The shared `intern_symbols_structural` pass (same as wasm W1) needed no change.
- ✅ **W5b — JVM lambda/`LABEL`/recursion (F7).** The JVM backend already lowered
  the uniform-anyref boundary (`Object`-param/return methods + `invokestatic`); the
  gap was in the *shared* structural pass, where the loose wasm model had hidden
  two strict-backend bugs: a lisp `call` result was hinted `i64` (the JVM stored an
  `Object` into a `long` slot), and a `COND` **funnel** mixing atom and reference
  clauses was boxed wholesale at `ret`. Fixed both in `lower_lisp_repr_structural`
  (call results → `ref<any>`; reference funnels box each atom clause *into* the
  funnel via a `mov`-chain fixpoint). `((LAMBDA (X) X) 5)`→5, multi-arg,
  `(CAR ((LAMBDA (X) (CONS X X)) 7))`→7, a recursive `LABEL`→99, and a mixed
  atom/cons `COND`→7, on a real `java` (`tests/jvm_lambda.rs`). wasm unaffected.
  **JVM is now McCarthy-complete (F1–F7).**

### Phase C — CLR (replicate as `object`)

- ✅ **W6a — CLR run-foundation (F1, scalar).** `lang-aot::compile_source_to_cil_artifact`
  + `concretize_scalar_any_for_cil` (scalar `any`/`i64` → CLR `i32`) →
  `iir-to-cil-bytecode`. **Verified by RUNNING** — the entry method's CIL on the
  in-repo **`clr-simulator`** (zero external `dotnet`, mirroring `jvm-simulator`):
  `42`→42, `0`→0, `7`→7, Twig `42`→42. Establishes the CLR pipeline + run-verify
  harness that W6b+ build on.
- ✅ **W6b — CLR cons (F2).** Completed in two sub-slices.
  - ✅ **W6b-1 — `clr-simulator` object/reference value model.** Real `dotnet`
    turned out non-viable as an in-repo runner (no PE/assembly emitter, no
    `ilasm`), so — per the backend's design intent ("artifact ready for the CLR
    simulator") — extended `clr-simulator` (0.2.0) to execute **reference types**:
    a `Value { Int | Ref }` stack model + an object heap + `newarr`/`stelem.ref`/
    `ldelem.ref`/`dup` and identity `box`/`unbox.any` (the loose model, like the
    wasm `i31`). Scalar behaviour unchanged; all CLR-backend consumers
    (`nib-clr`, `brainfuck-clr`, `iir-to-cil`) green. This is the in-repo
    run-verify substrate for CLR objects (the analog of `wasm-execution`'s
    `GcStruct`).
  - ✅ **W6b-2 — McCarthy cons on the simulator.** Added the `iir-to-cil` atom
    boxing (`box [int32]` / `unbox.any` — new `Box`/`UnboxAny` opcodes +
    `emit_box`/`emit_unbox_any` + `INT32_TYPE_TOKEN` in `ir-to-cil-bytecode`),
    removed `box`/`unbox` from `UNSUPPORTED_OPS` (the validator already accepted
    `ref<any>`), and ran the shared structural passes (incl. the JVM
    strict-backend fixes) in `lang-aot::compile_source_to_cil_artifact`.
    `(CAR (CONS 7 9))`→7, `(CDR (CONS 7 9))`→9, nested cons→2, on the
    object-capable `clr-simulator` (`tests/cil_cons.rs`).
- ✅ **W7 — CLR `ATOM`/`EQ`/`COND` (F3–F5).** `pair?`→`isinst object[]`, `not`→`x^1`,
  `equal?`→`unbox.any; unbox.any; ceq`; `COND`→`jmp_if_true`/`jmp_if_false`.
  `clr-simulator` 0.3.0 gained `isinst`/`xor` + ref-aware `ceq` (and an `ldnull`
  opcode fix). `(ATOM 7)`→1, `(ATOM (CONS 1 2))`→0, `(EQ 7 7)`→1, `(COND …)`
  verified on the simulator (`lang-aot/tests/cil_predicates.rs`).
- ✅ **W8 — CLR symbols + lambda (F6–F7). 🎉 COMPLETES CLR (F1–F7).**
  - ✅ **W8a — CLR symbols (F6).** **Zero new backend code** — the shared
    `intern_symbols_structural` pass interns symbols to `i32` ids (`SYMBOL_ID_BASE
    = 1<<29`) and W6b boxing + W7 `equal?`/`pair?`/`jmp_if` execute them.
    `(QUOTE A)`→536870912, `(EQ (QUOTE A) (QUOTE A))`→1, `(EQ (QUOTE A) (QUOTE B))`
    →0, `(ATOM (QUOTE A))`→1, on the simulator (`lang-aot/tests/cil_symbols.rs`).
  - ✅ **W8b — CLR lambda (F7).** Validator accepts `call`/`ref<any>` (the `call`
    arm already computes the MethodDef token + boxes args via the structural pass);
    `clr-simulator` 0.4.0 gained an inter-method **call-frame** model (method table +
    frame stack + `ldarg`, DoS-capped at `MAX_CALL_DEPTH`). `((LAMBDA (X) (CAR X))
    (CONS 7 9))`→7, `((LAMBDA (X Y) (EQ X Y)) 3 3)`→1, on the simulator
    (`lang-aot/tests/cil_lambda.rs`). **CLR is the third backend to reach F1–F7.**

### Phase D — BEAM (Erlang terms)

- ✅ **W9 — BEAM cons (F2).** Cons as a list cell `[a|b]`;
  `car`/`cdr` = `hd`/`tl`; integers are native Erlang integers; nil = `[]`.
  - ✅ **W9a — BEAM run-foundation (F1, scalar).** `lang-aot::compile_source_to_beam`
    + `concretize_scalar_any_for_beam` (scalar `any` → `i64`) → `iir-to-beam` →
    `encode_beam`. **Verified by RUNNING** the emitted `.beam` on a real `erl`
    (OTP 28): `42`→42, `0`→0, `7`→7, Twig `42`→42. Started as a parallel stream
    (independent of the held CLR W6b PR); reuses the backend's established
    real-`erl` round-trip harness. Establishes the BEAM pipeline.
  - ✅ **W9b — BEAM cons.** `cons`/`car`/`cdr` → BEAM list ops
    (`put_list`/`get_hd`/`get_tl`) — the native Erlang-terms model (NOT the
    structural uniform-ref pass). `compile_source_to_beam` now runs
    `lower_heap_builtins` (cons → `alloc`/`field_*`, which `iir-to-beam` already
    lowers) and concretizes `any`→`i64` per-instruction (leaving `ref<LispyPair>`
    cells). **Verified by RUNNING** on a real `erl`: `(CAR (CONS 7 9))`→7,
    `(CDR (CONS 7 9))`→9, nested→2, `(CONS 7 9)`→`[7|9]` (a native list cell)
    (`lang-aot/tests/beam_cons.rs`).
- ✅ **W10 — BEAM `ATOM`/`EQ`/`COND` (F3–F5).** Native Erlang guards via the same
  0/1 synthesis the `cmp_*` ops use: `pair?`→`is_nonempty_list` (a cons IS `[H|T]`),
  `equal?`→`is_eq_exact` (`=:=`), `not`→`is_eq_exact x 0` (`x==0`); `COND`→`jmp_if`.
  Removed `call_builtin` from the BEAM `UNSUPPORTED_OPS` (the lowering arm now
  dispatches the predicate set + rejects others). **Verified by RUNNING** on a real
  `erl`: `(ATOM 7)`→1, `(ATOM (CONS 1 2))`→0, `(EQ 7 7)`→1, `(COND …)`→100/200
  (`lang-aot/tests/beam_predicates.rs`).
- ✅ **W11 — BEAM symbols + lambda (F6–F7). 🎉 COMPLETES BEAM (F1–F7).**
  **Symbols (F6):** one-line pipeline addition — `compile_source_to_beam` runs the
  shared `intern_symbols_structural`, interning each symbol to a stable `i32` id
  (`SYMBOL_ID_BASE = 1<<29`, the SAME id as wasm/JVM/CLR), carried as a native
  Erlang integer; `EQ` → `is_eq_exact`. **Lambda (F7) needed NOTHING extra** — a
  `(LAMBDA …)` application is a method `call`, already lowered natively (a BEAM
  fun). **Verified by RUNNING** on a real `erl`: `(QUOTE A)`→536870912,
  `(EQ (QUOTE A) (QUOTE A))`→1, `((LAMBDA (X) (CAR X)) (CONS 7 9))`→7,
  `((LAMBDA (X) (EQ X (QUOTE A))) (QUOTE A))`→1
  (`lang-aot/tests/beam_symbols_lambda.rs`). **BEAM is the FIFTH backend at F1–F7.**

### Phase E — LLVM (tagged-word, like native)

- ✅ **W12 — LLVM cons + predicates (F2–F5). DONE** (via sub-slices W12a/W12b-1/2/3).
  Reused the tagged-word C runtime (`lispy_runtime.c`) the native AOT path links;
  lowered cons/car/cdr/pair?/eq via `call` to `__twig_lispy_*`; `COND` truthiness via
  `lispy_truthy`. **LLVM core F1–F5 complete; only symbols+lambda (W13) remain.**
  - ✅ **W12a — LLVM scalar run-foundation (F1, verify-by-running).** Established the
    LLVM execution substrate: `compile_source_to_llvm[_with_target]` (concretize
    `any`→`i64`, lower to LLVM IR). `lang-aot/tests/llvm_scalar.rs` emits **host**-
    triple IR (`clang -dumpmachine`), compiles it with `clang -x ir`, and **runs** the
    native executable — exit code = result: `42`→42, `7`→7, `0`→0, `100`→100, Twig
    `42`→42. Uses the `clang` already on the box (no `lli`/`qemu` needed; self-skips if
    absent). The LLVM analogue of `wasm-runtime`/`clr-simulator`/real `erl`.
  - ✅ **W12b-1 — LLVM cons (F2).** Lower cons/car/cdr → `call @__twig_lispy_*`
    (the `LISPY_BUILTINS` table maps `lispy_*`→runtime symbols; `ref<LispyPair>`/`any`
    carried as a tagged `i64`). `compile_source_to_llvm` runs the native lisp pipeline
    (`lower_heap_builtins_runtime`→`intern_symbols`→`lower_lisp_repr`). Verified by
    RUNNING on a clang-built executable **linked against `lispy_runtime.c`**:
    `(CAR (CONS 7 9))`→7, `(CDR (CONS 7 9))`→9, `(CAR (CDR (CONS 1 (CONS 2 3))))`→2,
    scalar `42`→42 (`lang-aot/tests/llvm_cons.rs`).
  - ✅ **W12b-2 — LLVM predicates ATOM/EQ (F3–F4).** Closed the deferred
    boolean-result gap in the shared `lower_lisp_repr`: a predicate result is a
    tagged boolean (`LISPY_TRUE=5`/`FALSE=3`), so the program-exit coercion is now
    type-directed — a **bool** result uses `lispy_truthy` (→ 0/1), an **int** result
    uses `lispy_unbox_int` (`>>3`). (Unboxing true gave `5>>3=0` — the bug.) Verified
    by RUNNING (clang + `lispy_runtime.c`): `(ATOM 7)`→1, `(ATOM (CONS 1 2))`→0,
    `(EQ 7 7)`→1, `(EQ 7 8)`→0 (`lang-aot/tests/llvm_predicates.rs`). Reusable for all
    tagged-word backends; `iir-to-llvm` unchanged.
  - ✅ **W12b-3 — LLVM `COND` (F5).** Solved the cross-block SSA merge the
    naive-frontend way: a variable assigned in 2+ instructions (a `COND` result
    written per clause) is promoted to a stack **slot** — an entry `alloca`, a
    `store` per assignment, a `load` per read (what `opt -mem2reg` would collapse);
    single-assignment vars keep the `const`/`mov` side-map. Two supporting fixes:
    `jmp_if` on the `i64` `lispy_truthy` result compares `!= 0` (not `trunc void`),
    and an all-`const`/`mov` (empty) clause block still gets an explicit fallthrough
    `br`. Verified by RUNNING (clang + `lispy_runtime.c`):
    `(COND ((ATOM 7) 11) …)`→11, second-clause→22, **nested `COND`**→44
    (`lang-aot/tests/llvm_cond.rs`). **LLVM core F1–F5 complete.**
- ✅ **W13 — LLVM symbols + lambda (F6–F7). DONE — LLVM COMPLETE (F1–F7).** Sixth
  backend to finish all seven features (after VM/WASM/JVM/CLR/BEAM).
  - ✅ **W13a — LLVM symbols (F6).** `intern_symbols` already runs in the LLVM
    pipeline; two fixes finish it: `llvm_type_for("symbol") = i64` (a tagged
    immediate), and the shared `lower_lisp_repr` returns a **symbol** result verbatim
    (its tagged word) instead of `unbox_int`'ing it (`>> 3` would corrupt id+tag) —
    the same type-directed exit coercion as bools (W12b-2). Verified by RUNNING
    (clang + `lispy_runtime.c`): `(EQ (QUOTE A) (QUOTE A))`→1, `(EQ (QUOTE A) (QUOTE B))`→0,
    `(ATOM (QUOTE A))`→1, symbol-in-`COND`→11, `(QUOTE A)`→its tagged word
    (`lang-aot/tests/llvm_symbols.rs`).
  - ✅ **W13b — LLVM lambda (F7). COMPLETES LLVM.** The lambda *mechanism* was
    already free from the shared pipeline; W13b closed the two value-model gaps:
    (1) **argument boxing** — a lambda's params are lisp values, so an integer atom
    argument is boxed (`lisp_arg_regs` now includes user-`call` args; without it a
    raw `5` reads as tag `0b101` = `#t`, `7` as `0b111` = a pair); (2) **polymorphic
    result coercion** — the entry sees a `call` typed `any` (runtime tag unknown), so
    a new shared runtime helper `__twig_lispy_to_exit_code` dispatches on the tag at
    RUN time (int → `>> 3`, `#t`/`#f`/nil → `1`/`0`/`0`, symbol/pair → verbatim). Both
    gated on the source language so the pass stays a faithful no-op for Twig (which
    also types untyped params `any`). The helper lives in `lispy_runtime.c`, so the
    native AOT (W14) and JIT (W15) tagged-word backends inherit it. Verified by
    RUNNING (clang + `lispy_runtime.c`): `((LAMBDA (X) X) 5)`→5,
    `((LAMBDA (X) (CAR X)) (CONS 7 9))`→7, `((LAMBDA (X Y) (EQ X Y)) 3 3)`→1,
    `((LAMBDA (X) (ATOM X)) 7)`→1, lambda-with-`COND`-body→100/200
    (`lang-aot/tests/llvm_lambda.rs`).

### Phase F — native AOT + JIT completion

- ✅ **W14 — native AOT `LAMBDA`/`LABEL` (F7). DONE — NATIVE AOT COMPLETE (F1–F7).**
  Seventh backend to finish all seven features (after VM/WASM/JVM/CLR/BEAM/LLVM).
  - ✅ **W14a — close the macOS Mach-O runtime-link gap.** The native object
    referenced the runtime helpers by their raw C name (`__twig_lispy_car`), but the
    `cc`-built archive — Mach-O C ABI — exports them decorated (`___twig_lispy_car`),
    so `ld` reported "Undefined symbols for architecture arm64" for **every**
    `__twig_*` call on macOS (lisp **and** `io_out`). `code-packager` now applies the
    leading-`_` decoration to external symbols (the ELF emitter deliberately does
    not — that's why the gap was macOS-only and native F2–F6 "passed" on Linux CI).
    Verified by RUNNING natively on macOS arm64: `(CAR (CONS 7 9))`→7, `(ATOM 7)`→1,
    `(EQ 7 7)`→1, `(COND …)`→11, `(EQ (QUOTE A) (QUOTE A))`→1
    (`lang-aot/tests/macos_native_lisp.rs`). **F2–F6 now run natively on macOS too.**
  - ✅ **W14b — native backend `LAMBDA` (F7).** The reusable-primitives thesis at
    its sharpest: native lambda was **one builtin-table row** away. All the machinery
    already existed — cross-function `call` (from Twig `fib`), `any`/`ref<Lispy…>`
    tagged-word values (from cons), arg boxing + result coercion (shared, W13b). The
    only gap was `lispy_to_exit_code` missing from the `aarch64`/`x86_64` backends'
    `V1_BUILTINS` table, so the `call_builtin` to it was refused as an unsupported op.
    Adding the row to both backends makes native lambda run. Verified by RUNNING on
    macOS arm64: `((LAMBDA (X) X) 5)`→5, `((LAMBDA (X) (CAR X)) (CONS 7 9))`→7,
    `((LAMBDA (X Y) (EQ X Y)) 3 3)`→1, `((LAMBDA (X) (ATOM X)) 7)`→1,
    lambda-with-`COND`-body→100/200 (`lang-aot/tests/macos_native_lisp.rs`).
- ✅ **W15 — JIT McCarthy core (F1–F7). DONE — JIT COMPLETE.** The **eighth and final
  backend** finishes all seven features. **McCarthy 1960 LISP now runs on every LANG
  VM backend (F1–F7): VM, native AOT, JIT, WASM, JVM, CLR, BEAM, LLVM.**
  - ✅ **W15a — JIT F1–F6 (scalar / cons / ATOM / EQ / COND / symbols).** The JIT
    dispatches `call_builtin "lispy_*"` to **Rust callbacks** (not native `__twig_lispy_*`
    calls), so the lisp ops are registered against the shared **`lispy-runtime`** crate
    (the C runtime's Rust twin — identical `u64` tagged-word model). A `LispyValue`
    rides inside `Value::Int` as its bit pattern; the JIT moves it opaquely. The
    `unbox_int`/`truthy` exit coercions are derived from `LispyValue::as_int`/`is_truthy`
    (existing primitives — not duplicated). New reusable entry `lang_aot::run_mccarthy_on_jit`.
    Verified by RUNNING (`lang-aot/tests/jit_mccarthy.rs`): `(CAR (CONS 7 9))`→7,
    `(ATOM 7)`→1, `(EQ 7 7)`→1, nested `COND`→44, `(EQ (QUOTE A) (QUOTE A))`→1.
  - ✅ **W15b — JIT `LAMBDA`/`LABEL` (F7). COMPLETES THE JIT — and McCarthy across all
    eight backends.** Two small fixes: (1) `vm-core::VMFrame::for_function` now sizes
    the register file to `max(register_count, params.len())` — a hoisted `LAMBDA` body
    reports `register_count = 0`, so the dispatcher's direct `registers[i] = arg` write
    indexed past the end and panicked; (2) `jit_lisp` registers `lispy_to_exit_code`
    (the polymorphic-result coercion — a tag dispatch derived from `LispyValue`'s
    predicates, the only builtin lambda needs beyond W15a's set). Verified by RUNNING
    (`lang-aot/tests/jit_mccarthy.rs`): `((LAMBDA (X) X) 5)`→5,
    `((LAMBDA (X) (CAR X)) (CONS 7 9))`→7, `((LAMBDA (X Y) (EQ X Y)) 3 3)`→1,
    lambda-with-`COND`-body→100/200, and a recursive `LABEL` (`FF` descending the
    car-spine to the leftmost atom)→7. Recursion depth is bounded by the JIT's fuel
    step-cap.

### Phase G — conformance

- ✅ **W16 — cross-backend conformance suite. DONE — THE PLATFORM MATRIX IS COMPLETE.**
  `lang-aot/tests/conformance.rs` runs one shared table of **19** McCarthy programs
  (F1–F7: scalar, cons/car/cdr, ATOM, EQ, COND, symbols, `LAMBDA`/`LABEL`/recursion)
  through **all eight backends** and asserts the *identical* integer result from
  each: VM (`mccarthy_lisp_vm`), JIT (`run_mccarthy_on_jit`), WASM (`wasm-runtime`),
  CLR (`clr-simulator`), JVM (real `java`), BEAM (real `erl`), LLVM (`clang` +
  `lispy_runtime.c`), native AOT (system `ld`). The four pure-in-process backends
  (VM/JIT/WASM/CLR) are the conformance floor — they must run every program; the
  external-tool backends skip gracefully when their tool is absent (so CI proves
  uniformity across whatever is installed). On a fully-equipped host all eight agree
  on all 19 programs. **This is the proof: one source, eight independent code
  generators, three value models (tagged-word / uniform-anyref / object-boxing /
  Erlang-terms), one answer.**

---

## Working rules (the loop honors these)

- **Slice small.** One PR per `W#` (sub-slice further when large, as L3b-3a did).
- **Verify by running**, not just emitting — execute the artifact (in-repo
  runtime / verifier / emulator) and assert the value. Reuse the wasm pattern.
- **Reusable primitives, no language-specific hacks** — shared passes
  (`lower_heap_builtins`, the structural repr pass) and the shared C runtime; a
  future lisp-family language should inherit all of this for free.
- **Specs → tests → impl → changelog → README → security review → PR.** Update
  this matrix's status cells in the same PR.
- **Never merge without explicit sign-off**; babysit each PR to green and let the
  user merge. On merge, advance.
- DoS/termination guard on any recursion/loop over untrusted input (the
  interpreters/JIT especially).
