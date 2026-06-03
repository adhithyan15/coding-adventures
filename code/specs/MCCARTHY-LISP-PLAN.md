# McCarthy Lisp on the LANG VM chain — plan

**Status:** Active.  L1 in progress as of 2026-06-03.  Confirmed decisions: **Lisp 1.0** (1960 paper), **IBM 704** as historical arch target, **no-CONS at runtime** on the historical-arch backends in v0.1.0.  Crate naming follows the existing `*-lexer` / `*-parser` / `*-iir-compiler` pattern.
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

Three new crates, mirroring the structure of the existing language frontends:

### `mccarthy-lisp-lexer`
S-expression tokenizer.  Recognises:
- `(`, `)`, `'` (quote sugar), `.` (dotted pair separator)
- Atoms: symbols `[A-Z][A-Z0-9-]*` (original Lisp was all-uppercase) and integers
- Whitespace, comments (`;` to end of line — added in Lisp 1.5; original 704 Lisp had no comments)

### `mccarthy-lisp-parser`
S-expression AST.  Tree shape:

```rust
enum LispExpr {
    Nil,              // ()
    Symbol(String),   // FOO
    Int(i64),         // 42
    Cons(Box<LispExpr>, Box<LispExpr>),
}
```

The parser also expands `'X` → `(QUOTE X)` and `(a . b)` → `Cons(a, b)`.

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

## Phases

Same single-PR-per-phase cadence the historical-arch migration used.

| Phase | Scope |
|-------|-------|
| **L1** | `mccarthy-lisp-lexer` + `mccarthy-lisp-parser` — tokenize + AST.  Tests pin S-expression round-trips. |
| **L2** | `mccarthy-lisp-iir-compiler` v0.1.0 — handles the 7 primitives + `LAMBDA` + `LABEL` + literal symbols/ints.  Compiles small examples (`(CAR '(A B C))` → `A`, etc.) end-to-end via the existing `vm-core` interpreter. |
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
