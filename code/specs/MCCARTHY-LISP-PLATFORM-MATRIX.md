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
| **WASM** | `iir-to-wasm` + `wasm-*` | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ☐ | uniform-anyref |
| **JVM** | `iir-to-jvm-class-file` | ✅ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | uniform-Object |
| **CLR** | `iir-to-cil-bytecode` | ✅ | ☐ | ☐ | ☐ | ☐ | ☐ | ☐ | uniform-object |
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
- ☐ **W2 — WASM `LAMBDA`/`LABEL`/user calls + recursion (F7).** Per-`LAMBDA`/
  `LABEL` wasm function; `call`; bound params as anyref locals; closure value if
  captured (env object). Recursion via the existing `call`. A recursive `LABEL`
  (e.g. a length/append) runs end-to-end. **Completes WASM.**

### Phase B — JVM (replicate the uniform-ref model as `Object`)

- ☐ **W3 — JVM cons (F2).** A `$LispyPair` class (two `Object` fields);
  `cons`/`car`/`cdr` → `new`/`getfield`/… ; integers boxed as `java.lang.Integer`
  (or a small `LispyInt`); `lower_lisp_repr_structural` adapted to JVM boxing.
  `(CAR (CONS 7 9))` → 7, run on a JVM (or the in-repo class-file
  verifier/interpreter, mirroring how wasm used `wasm-runtime`).
- ☐ **W4 — JVM `ATOM`/`EQ`/`COND` (F3–F5).** `instanceof $LispyPair` for `pair?`;
  `Integer.equals`/identity for `EQ`; truthiness for `COND`.
- ☐ **W5 — JVM symbols + lambda (F6–F7).** Interned symbol objects; lambda →
  method or `invokedynamic`/inner class. **Completes JVM.**

### Phase C — CLR (replicate as `object`)

- ☐ **W6 — CLR cons (F2).** `$LispyPair` type, `newobj`/`ldfld`; boxed ints.
- ☐ **W7 — CLR `ATOM`/`EQ`/`COND` (F3–F5).** `isinst`; equality; truthiness.
- ☐ **W8 — CLR symbols + lambda (F6–F7).** **Completes CLR.**

### Phase D — BEAM (Erlang terms)

- ☐ **W9 — BEAM cons (F2).** Cons as a 2-tuple (or proper list cell); `car`/`cdr`
  = element access; integers are native Erlang integers; nil = `[]`/`nil` atom.
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
