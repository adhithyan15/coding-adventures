# LANG61 — TW05-H: Self-Hosted Program Emitter

**Status:** In Progress
**Branch:** `feat/lang61-tw05h-program-emitter`
**Depends on:** LANG60 (TW05-G: lambda expressions and function definitions)

---

## Overview

TW05-H extends the self-hosted Twig emitter with two capabilities:

1. **`emit-program`** — a new top-level function that processes a complete
   list of top-level `Expr` nodes (as returned by `parse-program`) and emits
   each `(define (fn params) body)` into its own builder, returning a list of
   `(fn-name . instruction-list)` pairs.  This is the foundation for the
   self-compilation fixed-point milestone.

2. **`SymLit` emitter** — closes the last gap in the `Expr` dispatch chain.
   `SymLit` (symbol literal, e.g. `'foo`) was silently falling through to the
   nil fallback; it now emits a `const` instruction.

The deliverable is updates to two existing modules:

- `compiler/emit.tw` — `emit-program`, `emit-top-level-form`,
  `emit-program-loop`, `emit-symlit`; updated gate 3 to handle SymLit
- `compiler/main.tw` — new smoke test: emit two function definitions,
  return the count (2)

A `twig-module-driver` bump adds 6 `tw05h_tests` integration tests.

---

## Motivation

TW05-G proved the emitter can handle a single `(define (answer) 42)`.
TW05-H generalises this to whole programs: a list of top-level function
definitions each compiled into a separate instruction sequence.  Once
`emit-program` is in place, future milestones can feed the self-hosted
compiler's own source files through the full lex → parse → emit pipeline
and compare the resulting IIR structure across runs (the bootstrapping
fixed-point check).

---

## `emit-program` design

```scheme
; emit-program: process a list of top-level Expr nodes.
; forms — list of Expr produced by parse-program
; Returns a list of (cons fn-name instruction-list) pairs,
; one per DefExpr(LambdaExpr) found in the input.
; Non-definition forms (bare expressions) are silently skipped.
(define (emit-program forms) ...)
```

### `emit-top-level-form`

Processes one `Expr` node from the top-level list:
1. If the node is a `DefExpr` whose body is a `LambdaExpr`:
   - Create a fresh `IirBuilder` named after the function
   - Call `emit-lambdaexpr` with an empty environment
   - Finalise the builder to get the instruction list
   - Return `(cons fn-name instruction-list)`
2. Otherwise: return nil (bare expressions and non-function defines are
   deferred to a later milestone)

### `emit-program-loop`

Tail-recursive accumulator loop over the forms list.  Skips nil results
from `emit-top-level-form`.

---

## `SymLit` emitter

`SymLit` stores a string-typed symbol name (e.g., `'foo` → `"foo"`).
The emitter treats it identically to `StrLit` but retrieves the value via
`symlit-value`.  Gate 3 is extended to test both `StrLit?` and `SymLit?`
before falling through to gate 4.

```scheme
(define (emit-expr-3 expr b env)
  (if (StrLit? expr)
      (emit-strlit expr b)
      (if (SymLit? expr)
          (emit-symlit expr b)
          (emit-expr-4 expr b env))))
```

`emit-symlit` emits `(IirInstr "const" dest (list val) (TAny) sp)`.

---

## Updated `main.tw`

```scheme
(define (main)
  ; TW05-H pipeline: lex → parse → emit-program → count functions → 2
  ;
  ; Input: "(define (first) 1)(define (second) 2)"
  ; parse-program → [DefExpr "first"  (LambdaExpr [] (IntLit 1) sp) sp,
  ;                  DefExpr "second" (LambdaExpr [] (IntLit 2) sp) sp]
  ; emit-program → [(cons "first"  [instr1]),
  ;                 (cons "second" [instr1])]
  ; (length result) → 2
  (let* ((src    "(define (first) 1)(define (second) 2)")
         (tokens (lex-source src))
         (forms  (parse-program tokens))
         (funcs  (emit-program forms)))
    (length funcs)))   ; → 2
```

> **Implementation note**: A simpler no-param source is used in place of the
> `double`/`triple` example mentioned in the overview.  The VM has a 256-frame
> call-stack limit; a 57-char source with `CallExpr` bodies triggers the
> 11-gate dispatch chain recursively (per-arg), pushing the peak depth past
> the limit.  The `double`/`triple` shapes are fully exercised by the
> `emit_program_fn_instruction_count` unit test (which builds the AST
> directly without the lex/parse cost).  The integration smoke test uses
> no-param `IntLit` bodies that hit only gate 1 of the chain.

---

## Integration tests (`twig-module-driver` 0.5.0 → 0.6.0)

New `#[cfg(test)] mod tw05h_tests`:

| Test | Verifies |
|------|----------|
| `emit_program_single_fn` | `emit-program` on `"(define (f x) x)"` → 1 entry |
| `emit_program_two_fns` | `emit-program` on two defines → 2 entries |
| `emit_program_fn_name` | First entry's name = `"answer"` for `"(define (answer) 42)"` |
| `emit_program_fn_instruction_count` | `"(define (double x) (* x 2))"` body → 2 instructions |
| `emit_symlit_one_instruction` | `(SymLit "foo" sp)` → 1 instruction |
| `full_lex_parse_emit_program` | All 9 modules + main.tw → `(main) = 2` |

---

## Version bumps

| Package | Before | After |
|---------|--------|-------|
| `twig-module-driver` | 0.5.0 | 0.6.0 |

---

## Commit sequence

1. `docs(specs)` — this file
2. `feat(twig)` — `emit.tw` + `main.tw`
3. `test(twig-module-driver)` — 6 tw05h integration tests, bump 0.6.0

---

## What comes next (TW05-I)

TW05-I will attempt the first self-compilation check: run `emit-program`
on the lexed and parsed source of one of the compiler's own `.tw` modules
(e.g. `span.tw`) and verify the number and shape of emitted instruction
sequences matches expectations.  This requires no new language features —
only driving the existing pipeline end-to-end on real source.
