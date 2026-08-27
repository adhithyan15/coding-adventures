# McCarthy Lisp on the LANG VM chain — plan

**Status:** Active.  **L1 complete and grammar-driven as of 2026-06-03** (the lexer/parser were initially merged hand-written in #4967 and then rewritten to wrap the shared `GrammarLexer`/`GrammarParser` — see the "L1 divergence" note below).  **L2 complete** — decomposed into L2a/L2b/L2c (see the L2 note below).  **L2a ✓** (literals + `QUOTE` + `CONS`/`CAR`/`CDR`/`ATOM`/`EQ`, run on the new `mccarthy-lisp-vm`), **L2b ✓** (`COND`), **L2c-1 ✓** (direct `LAMBDA` application + the VM `call` opcode), **L2c-2 ✓** (`LABEL` named/recursive functions — compiler-only; recursion reuses the existing `call` opcode), **L2c-3a ✓** (lambda-as-value + the dynamic `apply` opcode; closures with empty env, no capture yet), **L2c-3b ✓** (free-variable capture for `LAMBDA` — direct application + value — via lambda lifting / precise capture), and **L2c-3c ✓** (capture for `LABEL` + `LABEL`-as-value, i.e. recursive closures) are merged — **the full closure story is done**.  **L3a ✓** wires `mccarthy-lisp` into `lang-aot` (the `Language::McCarthyLisp` frontend) — scalar McCarthy programs now compile to a native executable end-to-end (`42` → exits 42).  **L3b-1 ✓** lowers **heap cons cells** into the native backends: `prepare_module_for_aot` now runs `lower_heap_builtins` (`cons`/`car`/`cdr` → `alloc`/`field_store`/`field_load`), and aarch64/x86_64 lower those word-granular ops over a `__twig_alloc_bytes` cell — so a **cons-of-integers** program compiles to a native executable (`(CAR (CONS 7 9))` → exits 7 on Linux/Windows; macOS native can't link the runtime helper yet — a pre-existing gap).  **L3b-2a ✓** lands the **reusable lisp-native runtime** ([`LANG77`](LANG77-lisp-native-runtime.md)): a language-agnostic C implementation of `lispy-runtime`'s NaN-box value model (`twig-aot/runtime/lispy_runtime.c`) added to the AOT runtime archive, with a golden divergence-guard test pinning it to the Rust `pub const`s — so any lisp frontend (Twig too) can compile tagged cons/symbols natively.  This supersedes L3b-1's tag-less raw-word cons.  Remaining for **L3b**: wire the lowering + backends to call the runtime (**L3b-2b** cons via tagged values; **L3b-2c** `make_symbol`/`ATOM`/`EQ` — the gap blocking the literal `(CAR '(A B C))` → `A`), and the other `--emit` backends (wasm/jvm/clr/beam already lower cons; llvm/historical are scalar-only).  Confirmed decisions: **Lisp 1.0** (1960 paper), **IBM 704** as historical arch target, **no-CONS at runtime** on the historical-arch backends in v0.1.0.  Crate naming follows the existing `*-lexer` / `*-parser` / `*-iir-compiler` pattern.
**Predecessors:** [`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](HISTORICAL-ARCH-BACKEND-MIGRATION.md), [`MULTILANG-ARCHITECTURE-BACKENDS.md`](MULTILANG-ARCHITECTURE-BACKENDS.md).

## Goal

Implement **McCarthy's original Lisp** (1958–1960) as a first-class LANG VM frontend that targets *every* backend in the chain — including the **IBM 704**, the vacuum-tube mainframe Lisp was originally written on.

Same shape as the existing Twig / Nib / Brainfuck / Dartmouth BASIC / Oct frontends, but with two distinguishing features:

1. **The historical round-trip** — like Dartmouth BASIC → GE-225, Lisp source compiles back to the IBM 704 byte code McCarthy's original implementation produced (modulo modern packing conventions).
2. **The metacircular evaluator** — McCarthy's `eval` is itself implementable in 30 lines of Lisp.  Once the frontend is in place, we get a Lisp-in-Lisp interpreter on every backend for free.

## What's already wired (free wins from the historical-arch migration)

The Phase 1–7 migration we just finished established the clean **IIR → CIR → Backend trait** layer.  That means McCarthy Lisp gets these targets *for the cost of writing one frontend crate*:

| Target | Crate | Status |
|--------|-------|--------|
| **AOT native** (x86_64, AArch64) | `aarch64-backend` / `x86_64-backend` via `lang-aot` | ✓ ready |
| **VM (interpreter)** | `vm-core` (executes IIR directly) | ✓ ready |
| **JIT** | `jit-core::GenericCirJit` over CIR | ✓ ready |
| **WASM** | `iir-to-wasm` | ✓ ready |
| **JVM** | `iir-to-jvm-class-file` | ✓ ready |
| **CLR** | `iir-to-cil-bytecode` | ✓ ready |
| **BEAM** | `iir-to-beam` | ✓ ready |
| **LLVM IR** | `iir-to-llvm` | ✓ ready |
| **GE-225, Intel 4004, Intel 8008, ARMv7, RV32I** | the new `*-backend` crates from the migration | ✓ ready |
| **IBM 704** | **needs new `ibm704-encoder` + `ibm704-backend`** | 🔨 build |

So the work splits into three buckets: **frontend**, **runtime support**, **IBM 704 backend**.

---

## Bucket 1 — Frontend crates

Three crates, mirroring the structure of the existing language frontends — and, critically, **grammar-driven**: the lexer and parser are thin wrappers over the shared `GrammarLexer`/`GrammarParser`, exactly like `twig-lexer`/`twig-parser` and `nib`/`oct`.  Per the repo rule (`feedback_no_handwritten_lexers_parsers`), new language frontends MUST NOT hand-write lexers/parsers.

### Grammar files (the single source of truth)

Two files in `code/grammars/` encode the entire Lisp 1.0 surface syntax:

- **`mccarthy_lisp.tokens`** — six token kinds: `LPAREN` `RPAREN` `QUOTE` `DOT`, `SYMBOL = /[A-Z][A-Z0-9-]*/` (all-uppercase), `INTEGER = /-?[0-9]+/`; skips whitespace and `;` comments.  The dialect restrictions are enforced *here*: no lowercase, no operator symbols (a bare `-` matches nothing), no strings.
- **`mccarthy_lisp.grammar`** — six rules: `program / sexpr / atom / list / list_body / quoted`.  All structural rules — balanced parens, at-most-one dotted tail, "a dot must follow an element" — live in the grammar, so there is no hand-written validation to drift.

Both files are deliberately distinct from the existing `lisp.tokens` / `lisp.grammar`, which target a modern Scheme-ish dialect (lowercase symbols, strings, operator symbols).

### `mccarthy-lisp-lexer`
Thin wrapper over `GrammarLexer`.  `build.rs` compiles `mccarthy_lisp.tokens` to Rust at build time (the `twig-lexer` pattern: no runtime file I/O, Miri-safe, `OnceLock`-cached).  Public API: `tokenize_mccarthy(src) -> Result<Vec<lexer::token::Token>, LexerError>`, `create_mccarthy_lexer`, `mccarthy_token_grammar_spec`.

### `mccarthy-lisp-parser`
Thin wrapper over `GrammarParser` plus a CST → typed-AST extractor.  `build.rs` compiles `mccarthy_lisp.grammar` at build time.  The extractor lowers the generic `GrammarASTNode` CST into the typed AST:

```rust
enum LispExpr {
    Nil,              // ()
    Symbol(String),   // FOO
    Int(i64),         // 42
    Cons(Box<LispExpr>, Box<LispExpr>),
}
```

Sugar expansions happen in the extractor: `'X` → `(QUOTE X)`, `(A B C)` → `(A . (B . (C . NIL)))`, and `(a . b)` → `Cons(a, b)`.  Public API: `parse(src) -> Result<Vec<LispExpr>, ParseError>`, plus `parse_to_cst` / `extract_program` for tooling.  DoS hardening: `MAX_PAREN_DEPTH = 64` (pre-parse) + `MAX_AST_DEPTH = 64` (extractor).

> **L1 divergence note.** #4967 first merged this frontend as a *hand-written* lexer (bespoke `Token`/`Loc`/`LexError` enums, a byte-at-a-time tokenizer) and a *hand-written* recursive-descent parser (with bespoke `ParseError` variants `StrayDot`, `MultipleDotsInList`, …).  That violated `feedback_no_handwritten_lexers_parsers`.  The follow-up rewrite (this spec revision) deleted both hand-written implementations in favour of the grammar-driven wrappers above and authored the two `code/grammars/mccarthy_lisp.*` files.  The `LispExpr` AST shape is unchanged, so L2 is unaffected.  `MAX_PAREN_DEPTH` dropped from 256 → 64 because the shared `GrammarParser` uses much more stack per paren than the old hand-written descent did.

### `mccarthy-lisp-iir-compiler`
Source → `IIRModule`.  Lowering shape:

| Lisp form | IIR ops |
|-----------|---------|
| `(QUOTE X)` | `const dest, sym/int/list` (where list literals are materialized as a series of `alloc` + `field_store` ops) |
| `(CAR X)` | `field_load dest, X, 0` (where cons cells are 2-field records: `car`, `cdr`) |
| `(CDR X)` | `field_load dest, X, 1` |
| `(CONS A B)` | `alloc dest, "LispPair"` + `field_store dest, 0, A` + `field_store dest, 1, B` |
| `(ATOM X)` | type-test branch → `const dest, bool` |
| `(EQ A B)` | `cmp_eq dest, A, B` |
| `(COND (p1 e1) (p2 e2) ...)` | chained `jmp_if_false` + `label`s |
| `(LAMBDA (x y) body)` | new IIR function `gensym_42` with params `[x, y]`; the LAMBDA expression itself becomes a `const dest, fn_ref` |
| `(LABEL name (LAMBDA ...))` | named-function definition |
| `(f a b)` | `call dest, f, [a, b]` |
| Number literal `42` | `const dest, Int(42)` |
| Symbol `X` | `mov dest, X` |
| `nil` / `()` | `const dest, nil` |

The lowering reuses the existing Twig pattern for `LispPair` cells — see `code/packages/rust/twig-ir-compiler/src/lib.rs` for the `make_nil` / `cons` / `car` / `cdr` shape that already targets every backend.  **This is a huge head-start** — Twig already implements McCarthy's CONS/CAR/CDR primitives.

## Bucket 2 — Runtime support

Each backend needs to know how to allocate cons cells.  Existing infrastructure:
- `gc-core`, `garbage-collector` — already in the workspace
- `iir-builtin-lowering` — already turns the abstract `alloc` IIR op into backend-specific calls
- `lispy-runtime` — already exists with a `LispyPair` type and is used by Twig

McCarthy Lisp will reuse `lispy-runtime` directly (Twig is essentially a typed Lisp; McCarthy Lisp is its untyped cousin).  This means **zero new runtime code** for the AOT / VM / JIT / WASM / JVM / CLR / BEAM paths.

> **Execution-VM correction (L2).** The original plan said L2 runs examples "via the existing `vm-core` interpreter."  That is wrong: `vm-core`'s `Value` is **scalar-only** (`Int`/`Float`/`Bool`/`Str`/`Null`) — it has no representation for a cons cell or a symbol, so it cannot run a program whose result is `(CAR '(A B C))` → `A` or `(CDR '(A B C))` → `(B C)`.  The genuine requirement is a **Lisp value model + heap**, which is exactly `lispy-runtime` (tagged-`i64` `int`/`nil`/`symbol`/`#t`/`#f`/`heap-cons`, an interner, and the `cons`/`car`/`cdr`/`pair?`/`not`/`equal?` builtins).  The "VM" is just the loop that drives an `IIRModule` against that model.  `twig-vm` is one such loop, but it is the VM for the **Twig** language — coupling McCarthy Lisp to it would be an architectural mistake.  So McCarthy Lisp gets its **own** small VM on the shared foundation: **`mccarthy-lisp-vm`**, whose `run(&IIRModule) -> LispyValue` executes the module against `lispy-runtime`.  L2 lowers to the shared `lispy-runtime` conventions (`const` for ints, `const Var(name)` interns a symbol, `const 0 : ref<LispyPair>` is nil, `call_builtin "cons"/"car"/"cdr"/"pair?"/"not"/"equal?"`) and runs end-to-end on `mccarthy-lisp-vm`.  Because the IIR is the *same* artifact every backend consumes, L3's backends still light up unchanged — `mccarthy-lisp-vm` is just the L2 reference interpreter.  (`EQ` lowers to `equal?`, not the numeric `=`, since `=` rejects symbols.)  No `lispy-runtime` / `lang-runtime-core` source is modified, so the per-PR Miri obligation does not apply.

> **L2 decomposition.** L2 is split into mergeable increments (per the smaller-PRs working style):
> - **L2a** — `mccarthy-lisp-iir-compiler` v0.1.0 **+ `mccarthy-lisp-vm` v0.1.0**: lower a single top-level form sequence over integer/nil literals, `QUOTE` (symbols + nested lists → cons), and the data primitives `CONS`/`CAR`/`CDR` plus the predicates `ATOM` (= `not pair?`) and `EQ` (= `equal?`); run it end-to-end on McCarthy Lisp's own `lispy-runtime`-backed VM.  *No control flow or user functions yet.*
> - **L2b** — `COND` (chained `jmp_if_false` + labels).
> - **L2c** — `LAMBDA` / `LABEL` / user-defined function application.  Itself split:
>   - **L2c-1** — direct lambda application `((LAMBDA (p…) body) a…)`: each lambda becomes a top-level `IIRFunction`, the application emits a `call`, the VM gains a `call` opcode (fresh frame, params bound to args, call-depth guard).  *No closures: a lambda body sees only its own params; lambda-as-value is rejected.*
>   - **L2c-2 ✓** — `LABEL` (named / recursive functions).  `(LABEL F (LAMBDA (p…) body))` lowers like a lambda, but the compiler first binds `F` (in a *function scope*) to the new function, so a call `(F …)` inside `body` lowers to a `call` back into that function — i.e. `F` recurses.  **No new VM opcode**: a self-call is an ordinary `call`, already bounded by `MAX_CALL_DEPTH` + the instruction budget, so a non-terminating recursion errors cleanly instead of overflowing the native stack.  A labelled name used in *value* position (not called) is rejected — that is a first-class function value, which needs closures (L2c-3).
>   - **L2c-3** — closures: lambda as a first-class value + free-variable capture (needed for the L7 metacircular evaluator).  Split into two PRs:
>     - **L2c-3a** — *lambda-as-value + dynamic apply, no capture yet.*  A `LAMBDA` (or any expression) used in **value** position becomes a **closure value** — a tagged cons `(*CLOSURE* fn-name . env)` in the existing `lispy-runtime` model (env is nil in 3a).  The tag symbol `*CLOSURE*` is **un-forgeable from source**: McCarthy symbols are `[A-Z][A-Z0-9-]*`, so `*CLOSURE*` cannot be lexed and a user cannot construct one via `QUOTE`.  Applying a value — a call whose head is a parameter or a nested application — lowers to a new VM **`apply`** opcode that destructures the closure, looks the function up by name, and runs it in a fresh frame (bounded by `MAX_CALL_DEPTH` + the instruction budget; the Ω combinator `((LAMBDA (X) (X X)) (LAMBDA (X) (X X)))` errors cleanly).  **No `lispy-runtime` change** (closure conversion, encode-in-cons).  *Free variables in a lambda body are still unbound — capture is 3b.*  Each lambda lifts to a top-level `IIRFunction` exactly as in L2c-1; the only new IIR is the closure-value `cons`es + the `apply` op.
>     - **L2c-3b** — *free-variable capture for `LAMBDA` (direct application + value).*  A lambda lifts with its captured free variables as **extra leading parameters** (closure conversion / lambda lifting).  Capture is **precise**: the lambda captures exactly the free variables its body references (the body's free symbols — respecting own params, nested `LAMBDA`/`LABEL` binders, and `QUOTE` — intersected with the enclosing scope, sorted for determinism).  Precision matters for more than tidiness: capturing the *whole* enclosing frame would make a flat fan-out of `k` lambdas over `m` enclosing variables emit `O(m·k)` IIR (a compile-time DoS), whereas precise capture keeps the emitted IIR **linear in the source** (bounded by the parser's nesting cap).  A **direct application** forwards the captured registers as leading `call` args; a **closure value** stores the captured *values* in its `env` list `(*CLOSURE* fn-name v1 … vk)`, which the VM's `apply` flattens and prepends to the call args on entry.  So `(((LAMBDA (X) (LAMBDA (Y) (CONS X Y))) 'A) 'B)` ⇒ `(A . B)`.  **No `lispy-runtime` change.**  *Scoped to `LAMBDA`; `LABEL` capture + `LABEL`-as-value are L2c-3c (this does not regress `LABEL`, which keeps its own-params-only behaviour).*
>     - **L2c-3c ✓** — *capture for `LABEL` + `LABEL`-as-value (recursive closures).*  A labelled body now captures enclosing free variables too — `lift_label` mirrors `lift_lambda` (precise capture, captured-as-leading-params), excluding the label name itself (it denotes the function, resolved statically), and its `functions_in_scope` entry records the captured names so a recursive `(F …)` **forwards the captured registers** as leading args.  A `LABEL` in value position — or a labelled name `F` used as a value inside its own body — becomes a **recursive closure value** `(*CLOSURE* label-fn . env)`, applied through the same `apply` opcode.  **No VM change** (a self-call is an ordinary `call`; a captured env is leading `apply` args).  This completes closures.

For the historical-arch and IBM 704 backends, allocation is awkward (no heap on a 4004!).  Plan: **disallow CONS** at the IBM 704 backend for v0.1.0 — only programs that operate on pre-existing symbol literals are emittable.  This still covers a surprising amount of McCarthy Lisp's example programs (which were heavily symbol-shuffling).  Real CONS support on IBM 704 needs a static heap area which is a future increment.

## Bucket 3 — IBM 704 backend (the historical round-trip)

The IBM 704 is the natural fit — it's where Lisp was *born*.  John McCarthy and his students Steve Russell, Tim Hart, and Mike Levin first ran Lisp on this machine at MIT around 1959.

### What the silicon looked like

- **36-bit words**.  One cons cell = one word.
- **15-bit addresses** (32 K word memory, ≈144 KB).
- Word layout for instructions and pointers:

```text
35..21  prefix (3 bits) + decrement (15 bits)   → CDR field on a cons cell
20..6   tag (3 bits) + address (15 bits)        → CAR field on a cons cell
 5..0   opcode-specific
```

That's where `CAR` and `CDR` got their names — **C**ontents of **A**ddress / **D**ecrement part of **R**egister.  These were *literally* IBM 704 instruction mnemonics.

### New crates

| Crate | Lines (est) | Mirror of |
|-------|-------------|-----------|
| `ibm704-encoder` | ~150 | `intel8008-encoder` |
| `ibm704-backend` | ~250 (minimal viable: `const_*` + `ret_*`, like Phase 5/6) | `intel8008-backend` |

### Word packing in `Vec<u8>`

Two choices for 36-bit words → bytes:
- **5 bytes per word** (40 bits, 4 wasted) — simple, easy round-trip, ~11 % overhead
- **9 bytes per 2 words** (72 bits exact) — denser but requires bit-packing

Use **5 big-endian bytes per word**. Downstream `ibm704-simulator` reads the
low nibble of byte 0 plus bytes 1–4 and rejects a non-zero reserved high nibble.

### Halt sentinel

The IBM 704 had `HTR` (Halt and Transfer) at signed operation code `+0000`.
`+0420` is the distinct `HPR` (Halt and Proceed). Emit the all-zero `HTR 0`
word as the canonical halt sentinel.

### lang-aot wiring

Add `--emit=ibm704` (aliases `704`, `ibm-704`) routing through the standard `aot_core::infer` + `aot_core::specialise` + `ibm704_backend::compile` pipeline.

---

## The backend pipeline (L3 detail) — every target from one IIR module

This is the heart of the user-facing ask: *get Lisp all the way through AOT, VM, JIT, CLR, JVM, WASM, BEAM, and the historical backends.*  The key insight is that **all of this is downstream of a single artifact** — the `IIRModule` that L2's `mccarthy-lisp-iir-compiler` produces.  Once that module exists, every backend is a fan-out, not new frontend work.

### Flow

```text
McCarthy source
   │  mccarthy-lisp-lexer  (GrammarLexer)        ← L1 ✓ grammar-driven
   ▼  mccarthy-lisp-parser (GrammarParser → LispExpr)
LispExpr AST
   │  mccarthy-lisp-iir-compiler                  ← L2
   ▼
IIRModule  ──────────────────────────────────────────────────┐
   │                                                          │
   │  aot_core::infer → aot_core::specialise → CIR            │  (typed/optimised)
   ▼                                                          ▼
 ┌─ vm-core ............... interpret IIR directly  → VM result
 ├─ jit-core::GenericCirJit  CIR → host machine code → JIT result
 ├─ x86_64-backend / aarch64-backend (via lang-aot) → native object
 ├─ iir-to-wasm ........... → .wasm module
 ├─ iir-to-jvm-class-file . → .class file
 ├─ iir-to-cil-bytecode ... → CLR assembly
 ├─ iir-to-beam ........... → BEAM .beam (classic AtU8 atoms — see lessons)
 ├─ iir-to-llvm ........... → LLVM IR text
 ├─ ge225 / intel4004 / intel8008 / armv7 / rv32i backends → historical byte code
 └─ ibm704-backend ........ → IBM 704 byte code (L4/L5, the birthplace round-trip)
```

Nothing in that fan-out is McCarthy-specific — it is the same IIR→CIR→Backend-trait layer the historical-arch migration (predecessor spec) already shipped.  The *wiring* part (**L3a ✓**) was indeed a single-arm change: a `Language::McCarthyLisp` variant in `lang-aot` routing source through `mccarthy_lisp_iir_compiler::compile_source`; the emit dispatch is language-agnostic once an `IIRModule` exists.  But the **value-model** part is not free: L3a's empirical finding is that the native backend compiles a *scalar* McCarthy program end-to-end (`42` → exe exits 42) yet `BackendRefused`s a *symbol/cons* one (`(CAR '(A B C))`, `(CONS 'A 'B)`) — the backends don't yet lower `lispy-runtime` symbols/heap-cons.  Teaching them to (so the worked example below emits everywhere) is **L3b**.

### `lang-aot --emit` matrix + per-target acceptance

L3 is "done" when each of these emits a non-trivial artifact for the worked example `(CAR '(A B C))` (expected value `A`) and the listed acceptance test passes.  CONS-using programs are in scope for every modern target; the historical/small-machine targets are restricted to no-CONS programs in v0.1.0 (decision 3).

| `--emit` | Backend crate | Artifact | Acceptance test | CONS in v0.1.0 |
|----------|---------------|----------|-----------------|----------------|
| `vm` (default) | `vm-core` | in-process value | interpreter returns `A` | ✓ |
| `jit` | `jit-core::GenericCirJit` | host code | JIT-run returns `A` | ✓ |
| `native` / `x86_64` / `aarch64` | `*-backend` via `lang-aot` | object/exe | run exits `0`, prints `A` | ✓ |
| `wasm` | `iir-to-wasm` | `.wasm` | `wasmtime`/in-repo runner returns `A` | ✓ |
| `jvm` | `iir-to-jvm-class-file` | `.class` | class verifies; `main` returns `A` | ✓ |
| `clr` | `iir-to-cil-bytecode` | CLR asm | bytecode validates; returns `A` | ✓ |
| `beam` | `iir-to-beam` | `.beam` | OTP 27 loads module (classic AtU8); returns `A` | ✓ |
| `llvm` | `iir-to-llvm` | `.ll` text | `llc`/FileCheck pins IR shape | ✓ |
| `ge225` / `intel4004` / `intel8008` / `armv7` / `rv32i` | `*-backend` | byte code | e2e smoke test pins byte sequence | ✗ (no-CONS only) |
| `ibm704` (aliases `704`, `ibm-704`) | `ibm704-backend` (L4) | byte code | e2e smoke test pins byte sequence | ✗ (no-CONS only) |

> **BEAM caveat (carried from lessons):** `iir-to-beam` must emit the classic `AtU8` atom-table format; the nibble-packed form breaks OTP 27 in CI.  See `project_beam_atom_format_otp27`.

### Worked end-to-end example (the L6 demo)

One source file, eleven+ artifacts:

```lisp
(CAR '(A B C))   ; → A   (no CONS needed → emittable on every target incl. IBM 704)
(CDR '(A B C))   ; → (B C)  (needs CONS → modern targets only in v0.1.0)
```

L6 pins, per backend, either the runtime result (`A`) or the exact emitted byte sequence, in a single table-driven e2e test.  That test is the proof that "Lisp runs everywhere in the chain."

---

## Phases

Same single-PR-per-phase cadence the historical-arch migration used.

| Phase | Scope |
|-------|-------|
| **L1** ✓ | `mccarthy-lisp-lexer` + `mccarthy-lisp-parser` — **grammar-driven** (wrap `GrammarLexer`/`GrammarParser`; grammar in `code/grammars/mccarthy_lisp.tokens`+`.grammar`).  Tests pin S-expression round-trips + dialect errors.  *Done; rewritten from the hand-written #4967 merge — see L1 divergence note.* |
| **L2a** ✓ | `mccarthy-lisp-iir-compiler` + **`mccarthy-lisp-vm`** — literals + `QUOTE` + `CONS`/`CAR`/`CDR`/`ATOM`/`EQ`.  Compiles `(CAR '(A B C))` → `A`, `(CDR '(A B C))` → `(B C)`, `(CONS 'A 'B)` → `(A . B)`, etc., end-to-end on McCarthy Lisp's **own `lispy-runtime`-backed VM** (`vm-core` is scalar-only; `twig-vm` is Twig-specific — see the execution-VM correction). |
| **L2b** ✓ | `COND` — compiler lowers to chained `jmp_if_false` + `label`s (+ `mov` to funnel clause values); VM gains `jmp`/`jmp_if_false`/`mov`.  `(COND ((ATOM 'X) 'A) ('T 'B))` → `A`; no-match → `nil`. |
| **L2c-1** ✓ | Direct `LAMBDA` application `((LAMBDA (p…) body) a…)` — each lambda → a top-level `IIRFunction`; the application emits a `call`; the VM gains a `call` opcode (fresh frame, params bound to args, `MAX_CALL_DEPTH` guard).  `((LAMBDA (X) (CAR X)) '(A B))` → `A`.  *No closures yet.* |
| **L2c-2** ✓ | `LABEL` (named / recursive functions).  `((LABEL F (LAMBDA (p…) body)) a…)` — the body may call `F` (recursion); compiled to a function whose body `call`s itself.  `((LABEL FF (LAMBDA (X) (COND ((ATOM X) X) ('T (FF (CAR X)))))) '((A B) C))` → `A`.  *Compiler-only — no new VM opcode; recursion reuses `call`, bounded by `MAX_CALL_DEPTH`.*  *Labelled-name-as-value still rejected (→ closures).* |
| **L2c-3a** ✓ | Closures, part 1 — lambda **as a value** + dynamic **`apply`**.  A `LAMBDA` (or any expression) in value position lowers to a closure value `(*CLOSURE* fn-name . env)` (env nil; tag un-forgeable — not a lexable symbol); a call whose head is a parameter or a nested application lowers to the new VM `apply` opcode (destructure → look up by name → run in a fresh frame, `MAX_CALL_DEPTH`-bounded).  `((LAMBDA (F) (F 'A)) (LAMBDA (X) X))` → `A`; Ω errors cleanly.  *No `lispy-runtime` change; free-variable capture is 3b.* |
| **L2c-3b** ✓ | Closures, part 2 — free-variable capture for `LAMBDA` (direct application + value).  A lambda lifts with its captured frees as **extra leading params** (precise: the body's free symbols ∩ enclosing scope, sorted — keeps IIR linear in source); a direct application forwards the captured registers as leading `call` args, a closure value stores the captured *values* in `env` `(*CLOSURE* fn-name v1 … vk)` and `apply` prepends them on entry.  `(((LAMBDA (X) (LAMBDA (Y) (CONS X Y))) 'A) 'B)` → `(A . B)`.  *Scoped to `LAMBDA`; `LABEL` capture is 3c.* |
| **L2c-3c** ✓ | Closures, part 3 — capture for `LABEL` + `LABEL`-as-value (recursive closures).  `lift_label` mirrors `lift_lambda` (precise capture) but binds `F` for recursion and forwards captured registers on each self-call; a `LABEL` in value position is a recursive closure `(*CLOSURE* label-fn . env)`.  `((LAMBDA (G) (G '(A B C))) (LABEL LAST (LAMBDA (L) (COND ((ATOM (CDR L)) (CAR L)) ('T (LAST (CDR L)))))))` → `C`.  *No VM change.  Completes L2c / closures.* |
| **L3a** ✓ | Wire `mccarthy-lisp` into `lang-aot` as a new `Language::McCarthyLisp` variant (`--lang mccarthy-lisp`/`mcl`/`lisp`, `.mcl`/`.lisp` detection, `compile_source_to_iir` arm).  The emit/back-end dispatch is language-agnostic, so McCarthy reaches every `--emit` target; **scalar** programs run end-to-end on the native pipeline (`42` → exe exits 42).  *Symbol/cons programs are accepted by the frontend but `BackendRefused` by the native backend until L3b.* |
| **L3b-1** ✓ | Lower **heap cons cells** into the native backends.  `twig-aot::prepare_module_for_aot` runs `lower_heap_builtins` (`cons`→`alloc`+`field_store`, `car`/`cdr`→`field_load`, `()`→`const 0:ref<LispyPair>`); aarch64-backend / x86_64-backend gain `alloc`/`field_store`/`field_load`/`is_null` emitters (a 2-word `__twig_alloc_bytes` cell, raw-word values).  `(CAR (CONS 7 9))` → native exe exits 7 (Linux/Windows; macOS native runtime-helper linking is a pre-existing gap).  No Twig regression — the pass is a no-op without cons builtins. |
| **L3b-2** | **Tagged lisp values on native via a *reusable, language-agnostic* runtime** — see [`LANG77-lisp-native-runtime.md`](LANG77-lisp-native-runtime.md).  Rather than a McCarthy-specific tagging hack, implement `lispy-runtime`'s NaN-box value model once in C (`twig-aot/runtime/lispy_runtime.c`) and link it into AOT executables, so *any* lisp-family frontend (Twig too) gets native cons/symbols/`pair?`/`equal?` for free.  This supersedes L3b-1's raw-word cons (no type tag → no `pair?`/`ATOM`/`EQ`/symbols). **Sliced:** **L3b-2a ✓** — the C runtime + its golden divergence-guard test (pins the C tag constants/encodings to lispy-runtime's `pub const`s/constructors) + `build.rs` wiring; **no** lowering/backend changes, so zero regression and fully host-verifiable.  **L3b-2b ✓** — target-aware native lowering: `iir-builtin-lowering::lower_heap_builtins_runtime` renames `cons`/`car`/`cdr` → `call_builtin "lispy_cons"/"lispy_car"/"lispy_cdr"` (the runtime-call counterpart of the structural `alloc`/`field_*` form the managed wasm/jvm/clr/beam backends keep); `twig-aot::prepare_module_for_aot` calls it; aarch64/x86_64 gain `lispy_cons`/`lispy_car`/`lispy_cdr` `V1_BUILTINS` rows (the generic `call_builtin` path emits `BL/CALL __twig_lispy_*` — no new opcodes).  `(CAR (CONS 7 9))` → 7 through the linked C runtime (tagged cons pointer; integer payloads stay raw — boxing is deferred to 2c where the tag is first inspected).  **L3b-2c-1 ✓** — type-directed value **representation** (`iir-builtin-lowering::lower_lisp_repr`, run by `twig-aot` after the rename): box integer atoms feeding `lispy_*` calls (`n<<3`), tag the nil sentinel, unbox the program result (`lispy_unbox_int`) at the exit boundary.  Gate-free / use-site-directed (no language check — McCarthy boxes all atoms; Twig/Nib arithmetic untouched).  `(CAR (CONS 7 9))` → 7 now through fully tagged values.  **L3b-2c-2 ✓** — `pair?`/`not`/`equal?` (`ATOM`/`EQ`) renamed to `lispy_pair_p`/`lispy_not`/`lispy_equal`; a new C `__twig_lispy_truthy` (tagged → raw 0/1) normalises tagged `COND` predicates for `jmp_if_false`; `lower_lisp_repr` gains predicate-arg boxing, **bidirectional `mov`** boxing (so a `COND` clause literal funnelled alongside the tagged nil fallthrough is boxed uniformly), and the truthiness wrap.  `(COND ((ATOM 5) 7) (5 9))` → 7; `(COND ((ATOM (CONS 1 2)) 7) (5 9))` → 9 (Linux/Windows e2e).  **L3b-2c-3 ✓** — **compile-time symbol interning** (`iir-builtin-lowering::intern_symbols`, run by `twig-aot` before `lower_lisp_repr`): `const Var(name):symbol` → the tagged immediate `(id<<32)\|TAG_SYMBOL`, module-wide ids, so `EQ` on symbols is word equality.  Diverges from the planned runtime-`make_symbol`+string-literal route, which is needed only to *print* symbol names / create symbols dynamically (no native string-constant support yet) — deferred.  `lisp_repr` treats symbol immediates as tagged-but-never-boxed; no backend change.  `(CAR '(A B C))` → `A`, observed via `(COND ((EQ (CAR '(A B C)) 'A) 7) ('T 9))` → 7 (Linux/Windows e2e).  **This completes L3b-2c — McCarthy's full value model (cons/symbols/ATOM/EQ) now compiles to native.** |
| **L3b-3** *(in progress — wasm-first)* | The other `--emit` backends (wasm/jvm/clr/beam).  **Key finding:** these are *typed* backends — they reject McCarthy's polymorphic `"any"` lisp values (even scalar `42`→wasm fails), so each needs a real value-model design, not just wiring.  **Decision: wasm-first** — solve wasm via a *uniform-anyref* model (every lisp value = WasmGC `anyref`; integers boxed as `i31ref`; cons = `$LispyPair`; unbox to `i32` only at the return boundary — mirroring the native box/unbox), then replicate the pattern to jvm/clr/beam.  **L3b-3a-1 ✓** — enable the boxing primitive in `iir-to-wasm`: `box`→`ref.i31` (`I31New`), `unbox`→`i31.get_s` (`I31GetS`), opcode-byte verified.  **L3b-3a-2 ✓** — `lang-aot::compile_source_to_wasm`/`compile_file_to_wasm` + a `concretize_scalar_any_for_wasm` retype (`"any"`→`i64` for heap-free functions); **scalar McCarthy `42` emits a `.wasm` that RUNS** (verified end-to-end on the in-repo `wasm-runtime` → `i64 42`; Twig `42` too; cons is a clean `WasmBackendError`).  **Verification corrected:** the repo's own `wasm-runtime` *does* load+run emitted modules (i64 today), so wasm is verified *end-to-end, zero-external-dep* — the user chose to **extend the repo's own wasm tooling**.  **L3b-3a-3a ✓** — extend the `wasm-execution` engine to *run* the first WasmGC opcodes: `decode_function_body` now handles the two-byte `0xFB` prefix, and the engine executes `i31.new` (`0xFB 0x1C`) / `i31.get_s` (`0xFB 0x1D`) — an `i31ref` is its `i32` payload on the stack, so both are stack-identity no-ops; `i32.const 42→i31.new→i31.get_s`→`42` (run on the engine).  Unimplemented GC sub-opcodes (`struct.*`/`ref.*`) are a clean error.  **L3b-3a-3b ✓** — the `wasm-execution` engine now *runs* the WasmGC **object** opcodes: a new `WasmValue::Ref(Option<handle>)` (`None` = null = `nil`; tagged `anyref` on the typed stack), an append-only GC object heap of `GcStruct { type_idx, fields }` on the context (bounded by the instruction budget — no reclamation), and handlers for `struct.new` (`0xFB 0x00`) / `struct.get` (`0xFB 0x02`) / `struct.set` (`0xFB 0x04`) / `ref.null` (`0xD0 0x0F`) / `ref.is_null` (`0xD1`).  The decoder reads the struct ops' type/field index immediates (a `Gc{sub,type_idx,field_idx}` operand spilled to a per-function side-table like `br_table`) and consumes `ref.null`'s heap-type byte.  Struct arity comes from `WasmExecutionEngine::set_struct_field_counts` (the parser doesn't yet surface struct types — wired from the parsed module in 3a-3c).  **`(CAR (CONS 7 9))` → 7 run on the engine** (10 new tests incl. cdr ordering, `struct.set` mutation, `ref.is_null`, and clean traps for null-deref / out-of-range field / missing arity).  **L3b-3a-3c** *(in progress — the cons end-to-end through `wasm-runtime`, sliced)*: **L3b-3a-3c-1 ✓** — `wasm-module-parser` now parses **WasmGC struct types** from the type section (`0x50 <supers> 0x5F <fields>`), so an emitted `$LispyPair` cons module round-trips into `WasmModule.struct_types` instead of being rejected at the `0x50` tag (it previously accepted only `0x60` func types — a hard blocker for the e2e). Adds `anyref`/`i31ref`/concrete `structref` field decoding, preserves func/struct type-index alignment (the encoder emits funcs first), and caps untrusted vector pre-allocation against a crafted-count DoS. 8 new tests. **L3b-3a-3c-2 ✓** — `WasmRuntime::call` now derives each struct type's field count from the parsed `module.struct_types` and registers it with the engine (`set_struct_field_counts`), indexed by wasm type index (funcs first, then structs — matching the encoder), so a struct module runs with **no manual setup**. A hand-assembled `$LispyPair` cons module computing `(CAR (CONS 7 9))` now **parses, instantiates, and runs to `7`** on the in-repo `wasm-runtime` (via both the explicit `load`→`instantiate`→`call` path and `load_and_run`); previously it trapped with "no field count registered". (Assumes structs follow all func types — true for current import-free cons outputs; the ref-return placeholder is unchanged, unexercised since the cons return boundary unboxes to `i32`.) 2 new tests. **L3b-3a-3c-3 ✓** — the capstone: a new **structural lisp-value representation pass** (`iir-builtin-lowering::lower_lisp_repr_structural`, the managed-backend twin of the native `lower_lisp_repr`) boxes integer atoms stored into cons fields as `i31ref` (`box`, narrowing the atom const to `i32`) and unboxes the entry function's reference result (`unbox` → `i32`), so the uniform-anyref model is concretely typed; it partitions the module with `concretize_scalar_any_for_wasm` (heap fns vs pure-scalar). `lang-aot::compile_source_to_wasm` runs it between the heap lowering and the scalar concretizer. Also fixed `iir-to-wasm`'s `alloc`, which emitted a bare `ref.null` (so `field_store` trapped on null) — it now pushes a typed null per field and `struct.new`s a real `$LispyPair` (reusing the existing struct ops, no engine change). **`(CAR (CONS 7 9))` now compiles from McCarthy source and runs to `7`** on the in-repo `wasm-runtime` (`CDR`→9 and nested cons too; scalar McCarthy/Twig unaffected). **This completes L3b-3a-3c — the McCarthy cons value model runs end-to-end on wasm.** **L3b-3a-4** *(in progress — predicates + `COND` on wasm)*: **L3b-3a-4a ✓** — the `wasm-execution` engine + `wasm-module-encoder` gain **`ref.test`** (`0xFB 0x14 <typeidx>`) and its nullable `ref.test null` (`0xFB 0x15`) — the WasmGC type-test op McCarthy `pair?` lowers to ("is this lisp value a cons cell?"). The decoder reads the heap-type immediate; the engine pops a reference and pushes `i32 1` if it is a (non-null) `$LispyPair` struct ref, else `0` (`pair?(cons)`→1, `pair?(atom)`→0, `pair?(nil)`→0); the encoder gains `GcInstruction::RefTest`/`RefTestNull`. 4 new tests (3 engine + 1 encoder), hand-built-bytecode verified. **L3b-3a-4b ✓** — lower `pair?`/`not` (hence `ATOM`) from McCarthy source: `iir-to-wasm` whitelists + lowers `pair?`→`ref.test $LispyPair` and the lisp `not`→`i32.eqz` (and `module_uses_lispy_pair` now triggers on `pair?` so the struct type is emitted even with no `cons`); the `iir-builtin-lowering` structural pass now also owns predicate-using functions — it boxes the atom feeding `pair?`/`equal?` as an `i31ref` and concretises the boolean result to `i32` (not widened to `i64`, not unboxed). **`(ATOM 5)` → 1, `(ATOM (CONS 1 2))` → 0** compiled+run end-to-end (cons/scalar/Twig regression-tested); 2 new tests (1 pass unit + 1 e2e). **L3b-3a-4c ✓** — `EQ`/`equal?` (McCarthy atom equality): the structural pass already boxes the predicate's atoms as `i31ref` (3a-4b), so `iir-to-wasm` lowers `call_builtin "equal?"` to **unbox-both + `i32.eq`** (`i31.get_s` each arg, then compare) and whitelists it. **`(EQ 5 5)` → 1, `(EQ 5 6)` → 0**, compared values may be computed (`(EQ (CAR (CONS 3 4)) 3)` → 1). Atom equality only (McCarthy `eq`); deep structural `equal` over cons cells is a later builtin. 1 new e2e test; regression-tested. **L3b-3a-4d ✓** — `COND` with lisp-truthiness: the structural pass wraps a **lisp-value** clause guard (an atom/`nil`/cons/variable — anything whose producer isn't a `"bool"` predicate result) with `t = not(is_null(cond))` (boxing an atom guard to `i31ref` first), so a `jmp_if_false` branches on McCarthy truthiness — an integer atom is true **even `0`**, only `nil` is false; predicate guards (`pair?`/`EQ`, hint `bool`) test directly. The control flow (`jmp_if_false`/`label`/`jmp`/`mov`) already lowered, and the result funnel is left in the loose model (returns the clause value, or `nil` as `0`; uniform funnel boxing deferred — it would make a `nil` return unbox-trap). **`(COND ((ATOM 5) 7) (5 9))` → 7, `(COND ((ATOM (CONS 1 2)) 7) (5 9))` → 9, `(COND (0 7) (5 9))` → 7 (`0` truthy!), `(COND ((ATOM (CONS 1 2)) 7))` → nil/0**; 2 new tests (unit + e2e), regression-tested. **This completes L3b-3a-4 and the McCarthy core — cons, `ATOM`/`pair?`, `EQ`, `COND` — running end-to-end on the wasm backend.** Next: **3a-5** symbols (`QUOTE`/`'A`).  Then replicate the uniform-anyref pattern to jvm/clr/beam. |
| **L4** ✓ | `ibm704-encoder` + `ibm704-backend` v0.2.0. RCPU-P001 replaced the idealized layout with historical Type A/Type B fields and canonical five-byte big-endian transport. HTR is `+0000` (`+0420` is HPR). Constants live in an addressable literal pool, so Twig `42` is `[CLA 2, HTR 0, +42]` = 15 bytes and is executable by the forthcoming RCPU-003 simulator. |
| **L5** ✓ | Wire `lang-aot --emit=ibm704` (aliases `ibm-704`, `704`) through `ibm704-backend`. `lang-aot::compile_file_to_ibm704_bin` routes source through `aot_core::infer` + `aot_core::specialise` + `ibm704_backend::compile`. Two e2e smoke tests pin **Twig `42` → 15 bytes** and **McCarthy `42` → identical 15 bytes** (the IIR convergence proof — same source, same backend, same machine code, two frontends). |
| **L6** ✓ | **The historical round-trip demo**: one table-driven test in `tests/historical_round_trip.rs` asserts that **McCarthy `7` and Twig `7` emit byte-for-byte identical machine code on every historical-arch backend** — GE-225 / Intel 4004 / Intel 8008 / ARMv7 / RV32I / IBM 704.  The IIR convergence proof on the historical lanes: one IR layer, two surface languages, six machines spanning 71 years (1954 IBM 704 → 2025 ARMv7), six bit-identical outputs.  Chose `7` over `42` because Intel 4004 is 4-bit (max immediate `15`); `7` is the canonical small non-trivial integer that fits every historical-arch backend's narrowest immediate window.  CONS/symbol programs remain out of scope for v0.1.0 on these backends per the migration's no-runtime-CONS decision.  Combined with the W16 conformance suite (8 modern backends) and the per-arch byte-pinned tests in `end_to_end_smoke.rs`, McCarthy Lisp now runs uniformly on **14 backends** total. |
| **L7** ✓ | **The metacircular evaluator** — McCarthy's 1960 `EVAL` function written in McCarthy Lisp source, compiled to every modern backend.  Covers QUOTE / ATOM / EQ / CAR / CDR / CONS / COND applied recursively.  9 test programs (incl. `(CAR (CDR (CONS 1 (CONS 2 3))))` → 2) verified end-to-end on every available backend.  **VM / JIT / WASM are the conformance floor** (always run); CLR / JVM / BEAM / LLVM / native are tool-gated.  CLR runs the simpler conformance.rs programs but the deeply-nested CIL the metacircular evaluator emits exceeds what `clr-simulator` currently implements (panics on opcode `0x38` long-form `br`) — caught and treated as opt-in here; a future `clr-simulator` increment removes that caveat.  **Lisp-in-Lisp now runs on every available LANG VM backend.** |

7 phases — same depth as the historical-arch migration that produced 7 PRs.

---

## Confirmed decisions (2026-06-03)

1. **Dialect: Lisp 1.0** (1960 paper).  Pure 7-primitive Lisp (`CAR`, `CDR`, `CONS`, `ATOM`, `EQ`, `QUOTE`, `COND`) + `LAMBDA` + `LABEL` + `EVAL`.  All-uppercase, integers only, no strings.  Lisp 1.5 extensions can come in later phases.

2. **Historical-arch target: IBM 704.**  Matches the Dartmouth BASIC → GE-225 framing; CAR / CDR are literally IBM 704 instruction mnemonics.

3. **GC / allocation: no-CONS-required programs only on the historical-arch + IBM 704 backends in v0.1.0.**  The modern paths (AOT / VM / JIT / WASM / JVM / CLR / BEAM) get CONS support for free via `lispy-runtime`.  CONS on the small machines lands when each backend grows a static-heap region in a later phase.

4. **Crate naming: `mccarthy-lisp-lexer`, `mccarthy-lisp-parser`, `mccarthy-lisp-iir-compiler`, `ibm704-encoder`, `ibm704-backend`** — follows the existing `twig-ir-compiler`, `nib-iir-compiler`, `intel8008-encoder` pattern.

---

## Why this is worth doing

Three reasons beyond the obvious "it's cool":

1. **Validates the migration architecture.**  The historical-arch backend migration just landed a clean IIR → CIR → Backend-trait pipeline.  McCarthy Lisp is the perfect first new frontend to consume it — small surface area, exercises every backend.  If anything's wrong with the migration's design, this finds it fast.

2. **The metacircular demo is a load-bearing showcase.**  Compiling McCarthy's `eval` to every backend means *every backend can host every other backend's Lisp programs* by recursive interpretation.  That's a 11×11 = 121-cell compatibility matrix demonstrated by one source file.

3. **Two historical round-trips.**  Dartmouth BASIC → GE-225 (already done) and Lisp → IBM 704 (this) cover two of the most important "language born on machine X" pairings in computing history.  The combination is hard to find in any single project.
