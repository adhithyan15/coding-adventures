# LANG59 — TW05-F: Self-Hosted Twig IIR Emitter

**Status:** In Progress
**Branch:** `feat/lang59-tw05f-self-hosted-iir-emitter`
**Depends on:** LANG58 (TW05-E: self-hosted lexer + parser)

---

## Overview

TW05-F is the third compilation phase of the self-hosted Twig compiler: turning
an Abstract Syntax Tree (AST) into Interpreter IR (IIR) instructions.  The
deliverable is one new Twig module:

- `code/twig/compiler/emit.tw` — `emit-expr : Expr × IirBuilder × Env → (cons IirBuilder reg)`

`emit-expr` walks an `Expr` node (from `compiler/ast`), threads an `IirBuilder`
(from `compiler/iir-builder`), emits `IirInstr` records for each AST node, and
returns the pair `(updated-builder . result-register)`.

A smoke-test update to `main.tw` round-trips `"42"` through the full pipeline —
lex → parse → emit — and extracts the constant value from the emitted instruction
(returning `42` again).

---

## Why self-host the emitter?

TW05-E produced an in-memory AST.  TW05-F converts that AST to IIR — the flat
three-address representation the VM executes.  Writing this phase in Twig:

1. Exercises recursive AST traversal and builder-threading idioms in Twig
2. Validates the `IirBuilder` and `IirInstr` data model from TW05-D
3. Brings us one phase closer to TW05-G (self-compilation): lex → parse → emit
   in Twig produces IIR that can in principle compile *more* Twig

---

## Language features used

| Feature | Provided by |
|---------|-------------|
| `string-append`, `number->string`, `string=?` | LANG47 / LANG58 |
| `cons`, `car`, `cdr`, `list`, `reverse`, `null?`, `length` | LANG52 |
| `let*`, `if`, `and`, `or` | LANG52 |
| Records, unions, predicate functions | LANG48 / LANG57 |
| Multi-file modules, `(import ...)` | LANG56 |

---

## Key design constraints

### 1. No `(match ...)` on imported union variants

`Expr` variants are imported from `compiler/ast`; their tags are not propagated
across module boundaries.  All dispatch uses generated predicates:
`IntLit?`, `BoolLit?`, `VarRef?`, `CallExpr?`, etc.

### 2. No top-level value constants

Top-level `(define NAME value)` forms (non-lambda) are not exposed across module
boundaries by the module driver.  All inline constants are literals.

### 3. No mutable state

The `IirBuilder` is threaded functionally through every `emit-*` helper.  Each
function receives a builder and returns an updated one.

### 4. Gate-chain dispatch

Since we cannot use `(match ...)` on `Expr`, the dispatch uses a chain of small
functions (`emit-expr`, `emit-expr-2`, …, `emit-expr-9`), each testing one
predicate and tailing into the next.  This keeps paren-nesting flat.

---

## Module: `compiler/emit`

### Exported API

```scheme
(define (emit-expr expr b env) ...)
; expr : Expr (from compiler/ast)
; b    : IirBuilder
; env  : association list mapping name-strings to register-symbols
; Returns: (cons updated-IirBuilder result-register)

(define (env-empty) ...)
; Returns an empty environment (nil)

(define (env-lookup env name) ...)
; env  : association list
; name : string
; Returns: register symbol, or nil if not found

(define (env-extend env name reg) ...)
; env  : association list
; name : string
; reg  : register symbol
; Returns: extended environment
```

### Instruction formats emitted

| AST node | IIR instructions emitted |
|----------|-------------------------|
| `IntLit(v)`  | `%rN = const v : TInt` |
| `BoolLit(v)` | `%rN = const v : TBool` |
| `StrLit(v)`  | `%rN = const v : TStr` |
| `NilLit`     | `%rN = call_builtin make_nil : TAny` |
| `VarRef(x)`  | *(no new instruction; return existing register from env)* |
| `CallExpr(fn, args)` | emit each arg → `%rN = call_builtin fn arg0 arg1 … : TAny` |
| `BeginExpr(es)` | emit each ei in sequence; return last register |
| `LetExpr(bindings, body)` | emit each RHS, extend env, emit body |
| `IfExpr(cond, then, else)` | cond + `jmp_if_false` + then + `_move` + `jmp` + label + else + `_move` + label |
| *(other)*   | emit nil fallback |

### IfExpr instruction sequence

For `(if cond then-br else-br)`:

```
%r_cond   = emit cond
            jmp_if_false %r_cond L_else
%r_then   = emit then-br
%r_result = call_builtin _move %r_then
            jmp L_end
            label L_else
%r_else   = emit else-br
%r_result = call_builtin _move %r_else
            label L_end
```

Total: 9 instructions when cond, then, and else are each single literals.

### Environment

An environment is a simple association list of `(cons name reg)` pairs.
`env-lookup` performs linear search using `string=?` on the name.
`env-extend` prepends a new pair (shadowing earlier bindings with the same name).

---

## Updated `main.tw`

```scheme
(define (main)
  ; Full pipeline: lex "42" → parse → emit → extract constant value
  (let* ((tokens  (lex-source "42"))
         (exprs   (parse-program tokens))
         (expr    (car exprs))            ; IntLit 42
         (b0      (new-builder "test"))
         (env     (env-empty))
         (result  (emit-expr expr b0 env))
         (b-final (car result))
         (instrs  (finalise-builder b-final))
         ; emitted: (IirInstr "const" r0 (list 42) (TInt) sp)
         (instr   (car instrs)))
    (car (iirinstr-srcs instr))))   ; → 42
```

---

## Integration tests (`twig-module-driver` 0.3.0 → 0.4.0)

New `#[cfg(test)] mod tw05f_tests` block in `twig-module-driver/src/lib.rs`.

| Test | Verifies |
|------|----------|
| `emit_intlit_one_instruction` | emit `(IntLit 42 sp)` → 1 instruction |
| `emit_call_plus_1_2` | emit `(CallExpr (VarRef "+") [IntLit 1, IntLit 2])` → 3 instructions |
| `emit_if_expr_count` | emit `(IfExpr (BoolLit #t) (IntLit 1) (IntLit 2))` → 9 instructions |
| `emit_let_binding_count` | emit `(LetExpr [("x", IntLit 1)] (VarRef "x"))` → 1 instruction |
| `emit_begin_sequence_count` | emit `(BeginExpr [IntLit 1, IntLit 2, IntLit 3])` → 3 instructions |
| `full_lex_parse_emit_roundtrip` | All 9 modules + main.tw → `(main) = 42` |

---

## Version bumps

| Package | Before | After |
|---------|--------|-------|
| `twig-module-driver` | 0.3.0 | 0.4.0 |

No Rust compiler changes needed — all new functionality is pure Twig.

---

## Commit sequence

1. `docs(specs)` — this file
2. `feat(twig)` — `emit.tw` + updated `main.tw` (new emitter phase)
3. `test(twig-module-driver)` — 6 tw05f integration tests, bump 0.4.0

---

## Divergence from plan

None — implementation matches the LANG59 plan exactly.

---

## What comes next (TW05-G)

TW05-G will attempt the fixed-point self-compilation: stage0 (existing Rust
compiler) compiles a Twig source of the compiler → stage1 binary.  Stage1 then
compiles the same source → stage2 binary.  If stage1 == stage2, bootstrapping
is complete.
