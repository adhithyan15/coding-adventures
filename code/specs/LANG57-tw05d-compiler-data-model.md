# LANG57 — TW05-D: Compiler Data Model in Typed Twig

## Overview

LANG57 delivers the first typed Twig source files that live permanently in the
repository: the **compiler's own data model** under `code/twig/compiler/`.

It also fixes a latent bug in `compile_match` that caused every `(match …)`
expression to crash at runtime with `UnsupportedOpcode("jmpif")`.

---

## Motivation

LANG56 (multi-file module driver) makes multi-file Twig programs possible.
TW05-D is the first milestone to use that capability: writing the compiler's data
structures — `Token`, `Span`, `Diagnostic`, `AST`, `IirInstr`, `IirBuilder` — as
typed Twig modules.  This establishes the foundation for TW05-E (self-hosted lexer
+ parser) and TW05-F (self-hosted IIR emitter).

### Why fix `compile_match` first?

The data model relies heavily on `(union …)` for `TokenKind`, `Severity`,
`TypeHint`, and `Expr`.  Pattern matching via `(match …)` is the idiomatic way to
dispatch on union variants.  Without the bug fix, every such dispatch would crash.

---

## Part 1 — Bug fix: `compile_match` `jmpif` → `jmp_if_false`

### Root cause

`compile_match` in `twig-ir-compiler/src/compiler.rs` emits a three-operand
opcode `"jmpif"` for each variant arm.  This opcode is not in the VM's dispatch
table (`twig-vm/src/dispatch.rs`); only `jmp`, `jmp_if_false`, and `jmp_if_true`
are recognised (see `interpreter-ir/src/opcodes.rs`).

Generated broken code:
```
jmpif cond_reg arm_label skip_label   ← not a valid IIR opcode
label arm_label:
  <arm body>
jmp end_label
label skip_label:
```

### Fix

Use the existing `jmp_if_false` pattern (identical to `compile_if`): branch to
`skip_label` when the tag comparison is false, and let the arm body fall through:

```
jmp_if_false cond_reg skip_label
  <arm body>                         ← executes when cond is true (fall-through)
jmp end_label
label skip_label:
```

The redundant `label arm_label` instruction is eliminated.

---

## Part 2 — Runtime smoke tests (twig-vm)

Eight new tests in `twig-vm` (module `tw05d_smoke`) confirm that records, unions,
and match expressions work end-to-end.

| Test | Source |
|------|--------|
| `record_construction_and_accessor` | `(record Point (x:int)(y:int)) (point-x (Point 3 4))` → 3 |
| `record_second_field` | `(point-y (Point 3 4))` → 4 |
| `union_match_first_variant` | `(union Shape (Circle(r:int))(Rect(w:int)(h:int))) (match (Circle 5) ((Circle r) r) (_ 0))` → 5 |
| `union_match_second_variant` | `(match (Rect 3 4) ((Circle r) 0) ((Rect w h) w))` → 3 |
| `union_match_wildcard` | `(union Color (Red)(Green)) (match (Green) (_ 99))` → 99 |
| `union_predicate_true` | `(union T (A)(B)) (A? (A))` → #t |
| `union_predicate_false` | `(A? (B))` → #f |
| `match_binding_arm` | `(match 42 (n n))` → 42 |

---

## Part 3 — Compiler data model (`code/twig/compiler/`)

### Directory layout

```
code/twig/compiler/
  span.tw          Span record + validated constructor
  token.tw         TokenKind union + Token record
  diagnostic.tw    Severity union + Diagnostic record
  ast.tw           Expr union (mirrors the Rust AST)
  iir-types.tw     TypeHint union + IirInstr record
  iir-builder.tw   IirBuilder record + functional builder API
  main.tw          Root module + smoke-test entry point (main → 1)
```

### Language feature inventory (all present)

| Feature | Since |
|---------|-------|
| `(record …)` / `(union …)` / `(match …)` | LANG48 (match runtime fixed here) |
| Multi-file modules with `(import …)` | LANG56 |
| `let*`, `and`, `or`, `not`, `symbol-append`, `number->string` | LANG52 |
| `cons`/`car`/`cdr`/`null?`/`list`/`append`/`map`/`fold-left` | LANG52/55 |
| String literals | LANG51 |

**Workarounds for missing features:**

- `(values a b)` (Scheme multiple-return) — use `(cons b2 (cons slot nil))`; caller
  extracts with `(car r)` / `(car (cdr r))`.
- Record setters — implement `<name>-with-<field>` copy functions that construct a
  new record with one field changed.

### Modules

#### `span.tw`

```scheme
(module compiler/span
  (typed lenient)
  (export Span span? span-source-id span-start span-end make-span))

(record Span (source-id : int) (start : int) (end : int))

;;; make-span enforces the basic invariant 0 <= start <= end.
;;; Returns nil when the invariant is violated.
(define (make-span sid s e)
  (if (and (>= s 0) (<= s e))
      (Span sid s e)
      nil))
```

**Invariant:** `make-span` is a guarded constructor — a refinement-type proof
obligation that `start ≤ end` is checked at construction time and reported as a
runtime failure (nil return) rather than silently stored.

#### `token.tw`

```scheme
(module compiler/token
  (typed lenient)
  (export TokenKind TkLParen TkRParen TkInteger TkIdentifier TkBoolean
          TkString TkDot TkQuote TkEOF
          TkLParen? TkRParen? TkInteger? TkIdentifier? TkBoolean?
          TkString? TkDot? TkQuote? TkEOF?
          Token token? token-kind token-lexeme token-span)
  (import compiler/span))

(union TokenKind
  (TkLParen)
  (TkRParen)
  (TkInteger)
  (TkIdentifier)
  (TkBoolean)
  (TkString)
  (TkDot)
  (TkQuote)
  (TkEOF))

;;; Token: the fundamental unit produced by the lexer.
(record Token
  (kind   : any)    ; TokenKind
  (lexeme : any)    ; String — original source text
  (span   : any))   ; Span
```

#### `diagnostic.tw`

```scheme
(module compiler/diagnostic
  (typed lenient)
  (export Severity SevError SevWarning SevInfo
          SevError? SevWarning? SevInfo?
          Diagnostic diag? diag-severity diag-message diag-span)
  (import compiler/span))

(union Severity (SevError) (SevWarning) (SevInfo))

(record Diagnostic
  (severity : any)   ; Severity
  (message  : any)   ; String
  (span     : any))  ; Span
```

#### `ast.tw`

```scheme
(module compiler/ast
  (typed lenient)
  (export Expr
          IntLit BoolLit StrLit SymLit NilLit VarRef
          IfExpr LetExpr DefExpr CallExpr BeginExpr
          IntLit? BoolLit? StrLit? SymLit? NilLit? VarRef?
          IfExpr? LetExpr? DefExpr? CallExpr? BeginExpr?
          ...)
  (import compiler/span))

(union Expr
  (IntLit    (value : int)  (span : any))
  (BoolLit   (value : any)  (span : any))
  (StrLit    (value : any)  (span : any))
  (SymLit    (value : any)  (span : any))
  (NilLit    (span  : any))
  (VarRef    (name  : any)  (span : any))
  (IfExpr    (cond  : any)  (then-br : any) (else-br : any) (span : any))
  (LetExpr   (bindings : any) (body : any)  (span : any))
  (DefExpr   (name  : any)  (expr   : any)  (span : any))
  (CallExpr  (fn-expr : any) (args  : any)  (span : any))
  (BeginExpr (exprs : any)  (span  : any)))
```

#### `iir-types.tw`

```scheme
(module compiler/iir-types
  (typed lenient)
  (export TypeHint TInt TBool TAny TRef
          TInt? TBool? TAny? TRef?
          IirInstr iir-instr? iir-instr-op iir-instr-dest
          iir-instr-srcs iir-instr-type-hint iir-instr-span)
  (import compiler/span))

(union TypeHint
  (TInt)
  (TBool)
  (TAny)
  (TRef (name : any)))   ; named type, e.g. "ref<Token>"

(record IirInstr
  (op        : any)   ; Symbol opcode, e.g. 'const, 'call_builtin
  (dest      : any)   ; Symbol or nil
  (srcs      : any)   ; list of Symbol/Int
  (type-hint : any)   ; TypeHint
  (span      : any))  ; Span
```

#### `iir-builder.tw`

```scheme
(module compiler/iir-builder
  (typed lenient)
  (export IirBuilder iirbuilder?
          iirbuilder-name iirbuilder-instrs
          iirbuilder-reg-count iirbuilder-label-count
          new-builder alloc-slot alloc-label append-instr)
  (import compiler/iir-types compiler/span))

(record IirBuilder
  (name        : any)   ; Symbol — function name being built
  (instrs      : any)   ; List of IirInstr (stored reversed for O(1) prepend)
  (reg-count   : int)   ; Non-negative; next register index
  (label-count : int))  ; Non-negative; next label id

;;; "With-field" copy helpers — record setters are not generated automatically.
(define (iirbuilder-with-reg-count b n)
  (IirBuilder (iirbuilder-name b) (iirbuilder-instrs b)
              n (iirbuilder-label-count b)))
(define (iirbuilder-with-label-count b n)
  (IirBuilder (iirbuilder-name b) (iirbuilder-instrs b)
              (iirbuilder-reg-count b) n))
(define (iirbuilder-with-instrs b is)
  (IirBuilder (iirbuilder-name b) is
              (iirbuilder-reg-count b) (iirbuilder-label-count b)))

;;; Construct an empty builder for function `name`.
(define (new-builder name)
  (IirBuilder name nil 0 0))

;;; Allocate a fresh register slot.
;;; Returns (cons updated-builder slot-name) — use (car r) and (car (cdr r)).
(define (alloc-slot b)
  (let* ((idx  (iirbuilder-reg-count b))
         (slot (symbol-append 'r (number->string idx)))
         (b2   (iirbuilder-with-reg-count b (+ idx 1))))
    (cons b2 (cons slot nil))))

;;; Allocate a fresh label id.
;;; Returns (cons updated-builder label-id).
(define (alloc-label b)
  (let* ((id (iirbuilder-label-count b))
         (b2 (iirbuilder-with-label-count b (+ id 1))))
    (cons b2 (cons id nil))))

;;; Prepend an instruction (O(1)); finalise with (reverse (iirbuilder-instrs b)).
(define (append-instr b instr)
  (iirbuilder-with-instrs b (cons instr (iirbuilder-instrs b))))
```

#### `main.tw`

The root module imports all sub-modules and exports a `main` function that serves
as the smoke-test entry point.  `(main)` returns `1` (one register slot allocated),
confirming the whole module tree compiles and links correctly.

---

## Integration tests

Six tests in `twig-module-driver` (module `tw05d_tests`), each writing `.tw` files
to a temp directory and running them via `compile_module_tree` + `twig_vm::run`.

| Test | Verifies |
|------|----------|
| `span_make_span_valid_invariant` | `make-span 0 3 7` returns non-nil Span |
| `span_make_span_bad_invariant_returns_nil` | `make-span 0 7 3` returns nil |
| `token_tkinteger_predicate` | `(TkInteger? (TkInteger))` → #t |
| `ast_intlit_match_extracts_value` | `(match (IntLit 99 nil) ((IntLit v s) v) (_ -1))` → 99 |
| `iir_builder_alloc_slot_increments_reg_count` | `new-builder` + `alloc-slot` → reg-count 1 |
| `full_module_tree_smoke_test` | Compile `main.tw`, run `(main)` → 1 |

---

## Version bumps

| Crate | Before | After |
|-------|--------|-------|
| `twig-ir-compiler` | 0.10.0 | 0.11.0 |
| `twig-vm` | 0.14.0 | 0.15.0 |
| `twig-module-driver` | 0.1.0 | 0.2.0 |

---

## Definition of done

- `cargo test -p twig-vm --lib -- tw05d_smoke` — 8 tests pass
- `cargo test -p twig-ir-compiler --lib` — all 72 existing tests still pass
- `cargo test -p twig-module-driver --lib -- tw05d` — 6 integration tests pass
- `cargo build --workspace` — clean build
- `code/twig/compiler/` contains 7 `.tw` files
- `code/specs/LANG57-tw05d-compiler-data-model.md` committed first

---

## Relationship to other specs

| Spec | Relationship |
|------|--------------|
| LANG48 / TW05-A | Adds typed syntax to parser; records/unions/match lowering |
| LANG49 / TW05-B | Base type checker |
| LANG53 / TW05-C | Refinement checker bridge wired into type checker |
| LANG56 | Multi-file module driver enabling this milestone |
| TW05-E (future) | Self-hosted lexer/parser imports from `compiler/token`, `compiler/span` |
| TW05-F (future) | Self-hosted IIR emitter imports from `compiler/iir-builder` |
