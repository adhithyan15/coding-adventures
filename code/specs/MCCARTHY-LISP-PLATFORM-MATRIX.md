# McCarthy LISP — Full Platform Implementation Matrix

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
| **AOT native** | `twig-aot` + `aarch64`/`x86_64-backend` + `lispy_runtime.c` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ☐ | tagged-word |
| **WASM** | `iir-to-wasm` + `wasm-*` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | uniform-anyref |
| **JVM** | `iir-to-jvm-class-file` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | uniform-Object |
| **CLR** | `iir-to-cil-bytecode` | ✅ | ✅ | ☐ | ☐ | ☐ | ☐ | ☐ | uniform-object |
| **BEAM** | `iir-to-beam` | ✅ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | Erlang terms |
| **LLVM** | `iir-to-llvm` | ✅ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | tagged-word |
| **JIT** | (lang JIT path) | ✅ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | tagged-word |

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
- ☐ **W7 — CLR `ATOM`/`EQ`/`COND` (F3–F5).** `isinst`; equality; truthiness.
- ☐ **W8 — CLR symbols + lambda (F6–F7).** **Completes CLR.**

### Phase D — BEAM (Erlang terms)

- ◑ **W9 — BEAM cons (F2).** *In progress.* Cons as a list cell `[a|b]`;
  `car`/`cdr` = `hd`/`tl`; integers are native Erlang integers; nil = `[]`.
  - ✅ **W9a — BEAM run-foundation (F1, scalar).** `lang-aot::compile_source_to_beam`
    + `concretize_scalar_any_for_beam` (scalar `any` → `i64`) → `iir-to-beam` →
    `encode_beam`. **Verified by RUNNING** the emitted `.beam` on a real `erl`
    (OTP 28): `42`→42, `0`→0, `7`→7, Twig `42`→42. Started as a parallel stream
    (independent of the held CLR W6b PR); reuses the backend's established
    real-`erl` round-trip harness. Establishes the BEAM pipeline.
  - ☐ **W9b — BEAM cons.** Lower `cons`/`car`/`cdr` to BEAM list ops
    (`put_list`/`get_hd`/`get_tl`) — the native Erlang-terms model (NOT the
    structural uniform-ref pass). `(CAR (CONS 7 9))`→7 on a real `erl`.
- ☐ **W10 — BEAM `ATOM`/`EQ`/`COND` (F3–F5).** `is_tuple`/guards; `=:=`; truthiness.
- ☐ **W11 — BEAM symbols + lambda (F6–F7).** Symbols = Erlang atoms; lambda = fun.
  **Completes BEAM.** (Mind the OTP-27 AtU8 atom-format constraint from
  `ir-to-beam`.)

### Phase E — LLVM (tagged-word, like native)

- ☐ **W12 — LLVM cons + predicates (F2–F5).** Reuse the tagged-word C runtime
  (`lispy_runtime.c`) the native AOT path links; lower cons/car/cdr/pair?/eq via
  `call` to `__twig_lispy_*`; COND truthiness via `lispy_truthy`.
- ☐ **W13 — LLVM symbols + lambda (F6–F7).** **Completes LLVM.**

### Phase F — native AOT + JIT completion

- ☐ **W14 — native AOT `LAMBDA`/`LABEL` (F7).** Finish closures/user-calls on the
  tagged-word native backend (cons/ATOM/EQ/COND/symbols already done in L3b-2).
  Note the macOS runtime-link gap.
- ☐ **W15 — JIT McCarthy core (F1–F7).** Drive McCarthy through the JIT path.

### Phase G — conformance

- ☐ **W16 — cross-backend conformance suite.** One table of McCarthy programs ×
  every backend, each asserting the identical result — the proof that McCarthy
  is complete and uniform across the whole LANG VM. Wire into CI.

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
