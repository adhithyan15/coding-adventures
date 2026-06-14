# LANG60 — TW05-G: Lambda Expressions and Function Definitions

**Status:** In Progress
**Branch:** `feat/lang60-tw05g-lambda-and-defexpr`
**Depends on:** LANG59 (TW05-F: self-hosted IIR emitter)

---

## Overview

TW05-G extends the self-hosted Twig compiler with support for **lambda
expressions** and the **function-definition shorthand**, completing the ability
to parse and emit complete Twig programs.

The deliverable is updates to three existing modules:

- `compiler/ast.tw` — new `LambdaExpr` variant (tag 11)
- `compiler/parser.tw` — parse `(lambda (args) body)` and `(define (name args) body)`
- `compiler/emit.tw` — emit `LambdaExpr` and `DefExpr` with lambda bodies

A smoke-test update to `main.tw` compiles `"(define (answer) 42)"` through the
full lex → parse → emit pipeline and extracts `42` from the emitted constant
instruction.

---

## Motivation

Every Twig source file uses `(define (fn args) body)` — the function-definition
shorthand.  Without it, the self-hosted compiler cannot parse any real Twig
module, including itself.  TW05-G adds this capability, enabling future
milestones (TW05-H: self-compilation) to process actual `.tw` source files.

---

## AST change: `LambdaExpr`

A new variant is added to the `Expr` union in `compiler/ast.tw`:

```scheme
(LambdaExpr (params : any) (body : any) (span : any))
```

- **Tag**: 11 (auto-assigned after `BeginExpr` at tag 10)
- **`params`**: a list of parameter-name strings (e.g., `(list "x" "y")`)
- **`body`**: a single `Expr` representing the function body
- **Generated exports**: `LambdaExpr`, `LambdaExpr?`,
  `lambdaexpr-params`, `lambdaexpr-body`, `lambdaexpr-span`

---

## Parser changes

### `(lambda (params) body)` → `LambdaExpr`

New helper chain in `parse-list`:

```scheme
; Gate 6: `lambda`
(define (parse-list-6 tokens head-tok head-kind head-lex open-sp)
  (if (and (TkIdentifier? head-kind) (string=? head-lex "lambda"))
      (parse-lambda (cdr tokens) open-sp)
      (parse-call tokens open-sp)))
```

`parse-lambda`:
1. Expect `(` — consume it to get the parameter list
2. `parse-param-list` — read identifier tokens until `)`, collect as list of strings
3. `parse-expr` — parse the body
4. Consume the closing `)` of the lambda form
5. Return `(cons rest (LambdaExpr params body sp))`

### `(define (name args) body)` shorthand

`parse-define` now dispatches on the first token:

- If `TkLParen`: call `parse-define-fn` (function shorthand)
- Otherwise: call `parse-define-simple` (original `(define name expr)` behaviour)

`parse-define-fn`:
1. Read the function name token (first after `(`)
2. `parse-param-list` — read params until `)`
3. `parse-expr` — parse the body
4. Consume closing `)`
5. Return `(cons rest (DefExpr name (LambdaExpr params body sp) sp))`

`parse-define-simple` is the original `parse-define` body (unchanged logic).

---

## Emitter changes

### New dispatch gates

Two new gates are inserted before `CallExpr` in the chain:

| Gate | Predicate | Action |
|------|-----------|--------|
| `emit-expr-9` | `DefExpr?` | → `emit-defexpr` |
| `emit-expr-10` | `LambdaExpr?` | → `emit-lambdaexpr` |
| `emit-expr-11` | `CallExpr?` / fallback | (was gate 9) |

### `emit-lambdaexpr`

1. Allocate one register per parameter (via `alloc-slot`), bind each to its name in `env`
2. Emit the body expression with the extended environment
3. Return `(cons updated-builder body-result-register)`

No "load param" instructions are emitted — the VM's calling convention
pre-populates parameter registers when calling a function.

### `emit-defexpr`

- If `(defexpr-expr def)` is a `LambdaExpr`: delegate to `emit-lambdaexpr`
- Otherwise: delegate to `emit-expr` on the body (simple value binding)

---

## Updated `main.tw`

```scheme
(define (main)
  ; TW05-G pipeline: lex → parse → emit function definition → 42
  ;
  ; (define (answer) 42) parses to:
  ;   DefExpr "answer" (LambdaExpr [] (IntLit 42 sp) sp)
  ;
  ; emit-expr on this DefExpr → emit-lambdaexpr → emit IntLit 42:
  ;   (IirInstr "const" r0 (list 42) (TInt) sp)
  ;
  ; Extracting (car (iirinstr-srcs (car instrs))) = 42.
  (let* ((tokens  (lex-source "(define (answer) 42)"))
         (exprs   (parse-program tokens))
         (expr    (car exprs))
         (b0      (new-builder "answer"))
         (env     (env-empty))
         (result  (emit-expr expr b0 env))
         (b-final (car result))
         (instrs  (finalise-builder b-final))
         (instr   (car instrs)))
    (car (iirinstr-srcs instr))))   ; → 42
```

---

## Integration tests (`twig-module-driver` 0.4.0 → 0.5.0)

New `#[cfg(test)] mod tw05g_tests`:

| Test | Verifies |
|------|----------|
| `parser_lambda_expr` | `(parse-program "(lambda (x) x)")` → `(LambdaExpr? result) = 1` |
| `parser_define_fn_form` | `(parse-program "(define (f x) x)")` → `(LambdaExpr? (defexpr-expr def)) = 1` |
| `emit_lambda_no_params` | emit `(lambda () 99)` → 1 instruction |
| `emit_lambda_with_param` | emit `(lambda (x) (+ x 1))` → 2 instructions |
| `emit_defexpr_answer_42` | emit `(define (answer) 42)` → 1 instruction |
| `full_lex_parse_emit_defexpr` | all 9 modules + main.tw → `(main) = 42` |

---

## Version bumps

| Package | Before | After |
|---------|--------|-------|
| `twig-module-driver` | 0.4.0 | 0.5.0 |

---

## Commit sequence

1. `docs(specs)` — this file
2. `feat(twig)` — `ast.tw` + `parser.tw` + `emit.tw` + updated `main.tw`
3. `test(twig-module-driver)` — 6 tw05g integration tests, bump 0.5.0

---

## What comes next (TW05-H)

TW05-H will attempt the self-compilation fixed-point: compiling the Twig
compiler source files through the self-hosted pipeline (lex → parse → emit)
and verifying that the emitted IIR structure is consistent across runs.  This
requires the emitter to handle all remaining `Expr` variants and produce
executable IIR.
