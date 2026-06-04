# McCarthy Lisp on the LANG VM chain — plan

**Status:** Active.  **L1 complete and grammar-driven as of 2026-06-03** (the lexer/parser were initially merged hand-written in #4967 and then rewritten to wrap the shared `GrammarLexer`/`GrammarParser` — see the "L1 divergence" note below).  **L2 in progress** — decomposed into L2a/L2b/L2c (see the L2 note below).  **L2a ✓** (literals + `QUOTE` + `CONS`/`CAR`/`CDR`/`ATOM`/`EQ`, run on the new `mccarthy-lisp-vm`), **L2b ✓** (`COND`), **L2c-1 ✓** (direct `LAMBDA` application + the VM `call` opcode), and **L2c-2 ✓** (`LABEL` named/recursive functions — compiler-only; recursion reuses the existing `call` opcode) are merged; **L2c-3** (closures) is next.  Confirmed decisions: **Lisp 1.0** (1960 paper), **IBM 704** as historical arch target, **no-CONS at runtime** on the historical-arch backends in v0.1.0.  Crate naming follows the existing `*-lexer` / `*-parser` / `*-iir-compiler` pattern.
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
>   - **L2c-3** — closures: lambda as a first-class value + free-variable capture (needed for the L7 metacircular evaluator).

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

Default to **5 bytes per word** (matches the GE-225 precedent — 20-bit words as 3 bytes).  Downstream `ibm704-simulator` (a future crate) reads 5 bytes and masks off the high 4 bits.

### Halt sentinel

The IBM 704 had `HTR` (Halt and Transfer) at opcode `0o420` (octal 420 = `0b100_010_000`).  Emit `HTR 0` (jump-to-self halt) as the canonical halt word, same idiom GE-225 / Intel 4004 used.

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

Nothing in that fan-out is McCarthy-specific — it is the same IIR→CIR→Backend-trait layer the historical-arch migration (predecessor spec) already shipped.  L3 is therefore mostly *wiring*: adding a `Language::McCarthyLisp` variant to `lang-aot` and routing the existing `--emit` flags through `mccarthy_lisp_iir_compiler::compile`.

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
| **L2c-3** | Closures — lambda as a first-class value + free-variable capture. |
| **L3** | Wire `mccarthy-lisp` into `lang-aot` as a new `Language` variant.  All 10 existing backends light up automatically (CARSON the migration architecture). |
| **L4** | `ibm704-encoder` + `ibm704-backend` v0.1.0 (minimal viable: `const_*` + `ret_*`, just like the Phase 5/6 minimal-viable backends). |
| **L5** | Wire `lang-aot --emit=ibm704` through `ibm704-backend`. |
| **L6** | **The historical round-trip demo**: compile a McCarthy Lisp program (e.g. `(CDR '(A B C))` returning `(B C)`) to IBM 704 byte code via lang-aot, alongside the same program compiled to wasm/jvm/clr/beam/native/ge225/4004/8008/armv7/riscv.  E2e smoke tests pin byte sequences for each. |
| **L7** | **The metacircular evaluator** — port McCarthy's 1960 `eval` from the paper into McCarthy Lisp source, compile it to every backend.  Each backend now runs Lisp-in-Lisp. |

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
