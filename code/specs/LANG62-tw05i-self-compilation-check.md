# LANG62 — TW05-I: First Self-Compilation Check

**Status:** In Progress
**Branch:** `feat/lang62-tw05i-self-compilation-check`
**Depends on:** LANG61 (TW05-H: program emitter)

---

## Overview

TW05-I makes two changes:

1. **Bump `MAX_DISPATCH_DEPTH`** in `twig-vm` from 256 to 4096.
   The lex-loop in `compiler/lexer.tw` recurses once per character of input.
   `span.tw` is 2426 bytes; without a deeper limit the full-pipeline test
   returns `Run(DepthExceeded)` before it even finishes lexing.  4096 gives
   headroom for any realistic `.tw` source file (< 3500 chars after stripping
   comments).

2. **First real self-compilation smoke test**: `main.tw` now feeds a
   comment-stripped version of `span.tw` through the full lex → parse →
   `emit-program` pipeline and verifies that exactly 2 function definitions
   are emitted (`make-span` and `dummy-span`).

Six new integration tests in `twig-module-driver` (`tw05i_tests`) validate
the self-compilation at varying granularities, including one test that uses
the actual `span.tw` source file content (via `include_str!`).

---

## Motivation

TW05-H proved that `emit-program` works on hand-crafted short programs.
TW05-I is the first time the pipeline runs on **real compiler source** — a
module the compiler itself uses.  This exercises:

- The lexer's comment-skipping and colon-token handling on real code.
- The parser's fallback `parse-call` path for `module`, `record`, and
  field-annotation forms that have no dedicated parser gate.
- The emitter's ability to skip non-`DefExpr(LambdaExpr)` top-level forms.
- The IfExpr emitter path (`make-span` body uses `if`).
- The CallExpr emitter path for multi-argument builtins (`and`, `>=`, `<=`,
  `Span`).

---

## `MAX_DISPATCH_DEPTH` bump

**File:** `twig-vm/src/dispatch.rs`

```rust
// Before
pub const MAX_DISPATCH_DEPTH: usize = 256;

// After
pub const MAX_DISPATCH_DEPTH: usize = 4096;
```

The existing `deep_recursion_surfaces_depth_exceeded` test is unaffected:
it tests that *infinite* recursion eventually terminates with either
`DepthExceeded` or `InstructionLimitExceeded`; the exact limit is not
asserted.

---

## Stripped `span.tw` source

The source fed to the pipeline is `span.tw` with all comments stripped and
collapsed to a single line (≈ 365 chars):

```
(module compiler/span (typed lenient) (export Span span? span-source-id span-start span-end make-span dummy-span)) (record Span (source-id : int) (start : int) (end : int)) (define (make-span source-id start end) (if (and (>= start 0) (<= start end)) (Span source-id start end) nil)) (define (dummy-span) (Span 0 0 0))
```

Why stripped?  Because the Twig string-literal syntax requires escaping real
newlines as `\n` and double-quotes as `\"`.  A single-line form avoids the
escaping boilerplate in `main.tw` while still exercising all the
non-trivial forms (module declaration, record definition, function definition).

### What `parse-program` produces for this source

| Form | Parsed as | Emitted? |
|------|-----------|----------|
| `(module compiler/span ...)` | `CallExpr(VarRef "module", ...)` | ✗ (not DefExpr) |
| `(record Span ...)` | `CallExpr(VarRef "record", ...)` | ✗ |
| `(define (make-span ...) ...)` | `DefExpr "make-span" LambdaExpr` | ✓ |
| `(define (dummy-span) ...)` | `DefExpr "dummy-span" LambdaExpr` | ✓ |

Result: `(length (emit-program ...))` = **2**.

### Notes on colon tokens (`:`

Field annotations like `(source-id : int)` produce a `TkColon` token.  The
parser has no gate for `TkColon`; it falls through gate 7's fallback and
becomes `NilLit` (consuming the token).  The surrounding call expression is
still parsed successfully as `CallExpr(VarRef "source-id", [NilLit, VarRef "int"])`.

---

## Updated `main.tw`

```scheme
(define (main)
  ; TW05-I pipeline: lex + parse + emit-program on stripped span.tw → 2 fns
  (let* ((src    "(module compiler/span ...)  (record Span ...)  (define (make-span ...) ...)  (define (dummy-span) ...)")
         (tokens (lex-source src))
         (forms  (parse-program tokens))
         (funcs  (emit-program forms)))
    (length funcs)))   ; → 2
```

The lex-loop makes ≈ 365 recursive calls for this source.  With
`MAX_DISPATCH_DEPTH = 4096`, the peak call depth is ≈ 370 — well within the
limit.

---

## Instruction counts for span.tw functions

### `dummy-span` body: `(Span 0 0 0)`

```
const r0 0          ; IntLit 0
const r1 0          ; IntLit 0
const r2 0          ; IntLit 0
call_builtin r3 Span r0 r1 r2
```

**4 instructions.**

### `make-span` body: `(if (and (>= start 0) (<= start end)) (Span source-id start end) nil)`

```
const r3 0                        ; IntLit 0 for >= check
call_builtin r4 >= r1 r3          ; (>= start 0)
call_builtin r5 <= r1 r2          ; (<= start end)
call_builtin r6 and r4 r5         ; (and ...)
jmp_if_false r6 L0                ; branch to else
call_builtin r8 Span r0 r1 r2     ; then: (Span source-id start end)
call_builtin r7 _move r8          ; move then-result to result reg
jmp L1                            ; skip else
label L0                          ; else branch
call_builtin r9 make_nil          ; nil
call_builtin r7 _move r9          ; move else-result to result reg
label L1                          ; end
```

**12 instructions.**

(Note: param registers r0=source-id, r1=start, r2=end are pre-allocated by
`emit-lambda-params` without emitting instructions.)

---

## Integration tests (`twig-module-driver` 0.6.0 → 0.7.0)

New `#[cfg(test)] mod tw05i_tests`:

| Test | Verifies |
|------|----------|
| `self_compile_stripped_span_fn_count` | Stripped span source → 2 functions from emit-program |
| `self_compile_stripped_span_fn_names` | `make-span` is first, `dummy-span` is second |
| `self_compile_dummy_span_instr_count` | `dummy-span` → 4 instructions |
| `self_compile_make_span_instr_count` | `make-span` → 12 instructions |
| `self_compile_real_span_tw` | Actual `span.tw` content (via `include_str!`) → 2 functions |
| `full_lex_parse_emit_self_compile` | All 9 modules + main.tw → `(main) = 2` |

---

## Version bumps

| Package | Before | After |
|---------|--------|-------|
| `twig-vm` | 0.15.0 | 0.16.0 |
| `twig-module-driver` | 0.6.0 | 0.7.0 |

---

## Commit sequence

1. `docs(specs)` — this file
2. `fix(twig-vm)` — `MAX_DISPATCH_DEPTH` 256 → 4096, bump 0.16.0
3. `feat(twig,twig-module-driver)` — `main.tw` + 6 `tw05i_tests`, bump 0.7.0

---

## What comes next (TW05-J)

TW05-J will run the self-hosted compiler on `diagnostic.tw` and `token.tw`,
expanding the self-compilation check to modules with richer union/record
structures.  No new infrastructure is needed — TW05-I's depth increase and
pipeline are sufficient.
