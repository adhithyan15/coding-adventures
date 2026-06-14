# LANG58 — TW05-E: Self-Hosted Twig Lexer + Parser

**Status:** In Progress
**Branch:** `feat/lang58-tw05e-self-hosted-lexer-parser`
**Depends on:** LANG57 (TW05-D: compiler data model)

---

## Overview

TW05-E is the first real *compilation phase* of the self-hosted Twig compiler: turning
source text into an Abstract Syntax Tree (AST).  The deliverable is two new Twig modules:

- `code/twig/compiler/lexer.tw` — `lex-source : String → List[Token]`
- `code/twig/compiler/parser.tw` — `parse-program : List[Token] → List[Expr]`

Both modules build on the data model introduced in TW05-D (LANG57):
`Span`, `Token`, `TokenKind`, and `Expr` from `compiler/ast`.

A smoke-test update to `main.tw` round-trips `"42"` through the full pipeline and
returns the integer value `42`, exercising the module graph end-to-end.

---

## Why self-host the lexer and parser?

The long-term goal (TW05-G) is for the Twig compiler to compile itself.  The lexer and
parser are the first two phases in any compiler pipeline.  Writing them in Twig gives us:

1. A real workout for the module system and string-processing builtins
2. An executable specification that doubles as regression tests
3. The foundation for the full self-hosting pipeline

---

## Language features used

| Feature | Provided by |
|---------|-------------|
| `string-ref`, `string-length` | LANG47 (lispy-runtime), wired in LANG58 |
| `char->integer`, `integer->char` | LANG47, wired in LANG58 |
| `substring`, `string-append` | LANG47, wired in LANG58 |
| `string->number`, `string=?`, `string<?`, `string>?` | LANG47, wired in LANG58 |
| `char-alphabetic?`, `char-numeric?`, `char-whitespace?` | LANG47, wired in LANG58 |
| `let*`, `and`, `or`, `not` | LANG52 |
| `cons`, `car`, `cdr`, `list`, `reverse`, `null?` | LANG52 |
| Multi-file modules, `(import ...)` | LANG56 |
| Records, unions, predicate functions | LANG48 / LANG57 |

---

## Key design constraints

### 1. No `(match ...)` across module boundaries

The module driver does not propagate `variant_tags` across module boundaries.  Pattern
matching on `TokenKind` or `Expr` variants imported from another module fails at runtime
because the discriminant is unknown.

**Workaround:** Use the generated predicate functions instead:
```scheme
; WRONG — match on imported union variant:
(match kind ((TkInteger) ...) ((TkLParen) ...))

; CORRECT — predicate dispatch:
(if (TkInteger? kind) ... (if (TkLParen? kind) ...))
```

### 2. No mutable state

Twig has no mutable state.  The lexer is a tail-recursive function that threads an
accumulator list.  The parser returns `(cons rest-tokens parsed-expr)` pairs.

### 3. ASCII integer dispatch

Characters are integers in Twig's encoding.  The lexer compares code-point constants
rather than character literals:
```scheme
(define ASCII-LPAREN 40)   ; (
(define ASCII-RPAREN 41)   ; )
```

---

## Module: `compiler/lexer`

### Exported API

```scheme
(define (lex-source src) ...)
; src : String → List[Token]
; Returns a token list including TkEOF as the final sentinel.
```

### Algorithm

```
lex-source(src):
  lex-loop(src, 0, string-length(src), '())

lex-loop(src, pos, len, acc):
  if pos >= len:
    reverse(cons(EOF-token(pos), acc))
  else:
    c = char->integer(string-ref(src, pos))
    dispatch on c → produce token, recurse
```

Character categories handled:

| Input | Token produced |
|-------|----------------|
| `(` (40) | `TkLParen` |
| `)` (41) | `TkRParen` |
| `'` (39) | `TkQuote` |
| `.` (46) | `TkDot` |
| `:` (58) | `TkColon` |
| `"` (34) | scan string → `TkString` |
| `;` (59) | skip to end of line, recurse |
| `#` (35) | scan `#t` or `#f` → `TkBoolean` |
| digit (48–57) or `-` (45) followed by digit | scan integer → `TkInteger` |
| whitespace (9, 10, 13, 32) | skip, recurse |
| anything else | scan identifier → `TkIdentifier` |

Negative integers are supported: `-` followed immediately by a digit starts an integer scan.

### Helpers

| Function | Purpose |
|----------|---------|
| `scan-integer src pos len start-pos` | Scan digits; return `(cons end-pos lexeme)` |
| `scan-identifier src pos len start-pos` | Scan until delimiter; return `(cons end-pos lexeme)` |
| `scan-string src pos len start-pos` | Scan until closing `"` (skipping `\"`); return `(cons end-pos lexeme)` |
| `is-delimiter? c` | `(`, `)`, `;`, whitespace, EOF |
| `build-lexeme src start end` | `(substring src start end)` |

---

## Module: `compiler/parser`

### Exported API

```scheme
(define (parse-program tokens) ...)
; tokens : List[Token] → List[Expr]
; Parses all top-level expressions until TkEOF.

(define (parse-expr tokens) ...)
; tokens : List[Token] → (cons List[Token] Expr)
; Returns (remaining-tokens . parsed-expr).
```

### Algorithm

`parse-expr` dispatches on the kind of the head token:

| Token kind | AST node produced |
|------------|------------------|
| `TkInteger` | `(IntLit (string->number lexeme) span)` |
| `TkBoolean` | `(BoolLit (string=? lexeme "#t") span)` |
| `TkString` | `(StrLit lexeme span)` |
| `TkIdentifier` `"nil"` | `(NilLit span)` |
| `TkIdentifier` otherwise | `(VarRef lexeme span)` |
| `TkLParen` | `parse-list` (see below) |
| `TkQuote` | `(CallExpr (VarRef "quote" sp) (list inner) sp)` |

`parse-list` reads forms until `TkRParen`:

| Head identifier | AST node produced |
|-----------------|------------------|
| `"define"` | `(DefExpr name body sp)` |
| `"if"` | `(IfExpr cond then else sp)` |
| `"begin"` | `(BeginExpr exprs sp)` |
| `"let"` or `"let*"` | `(LetExpr bindings body sp)` (bindings as cons-pair list) |
| anything else | `(CallExpr fn-expr args sp)` |

`parse-program-loop` accumulates top-level expressions until `TkEOF`.

---

## Updated `main.tw`

```scheme
(define (main)
  ; Round-trip "42" through the lexer + parser
  (let* ((tokens (lex-source "42"))
         (exprs  (parse-program tokens))
         (first  (car exprs)))
    (intlit-value first)))   ; → 42
```

---

## Integration tests (`twig-module-driver` 0.2.0 → 0.3.0)

| Test | Verifies |
|------|----------|
| `lexer_single_integer_token` | `(token-lexeme (car (lex-source "42")))` → `"42"` |
| `lexer_parens_and_identifier` | `(lex-source "(foo)")` → 4 tokens (LP, Ident, RP, EOF) |
| `lexer_skips_whitespace` | `(lex-source "  42  ")` → TkInteger + TkEOF |
| `lexer_skips_comment` | `(lex-source "; hi\n42")` → TkInteger + TkEOF |
| `parser_integer_literal` | parse `[TkInteger "99" sp]` → `intlit-value = 99` |
| `parser_nested_call` | parse `(+ 1 2)` → `CallExpr` with 2 args |
| `full_lex_parse_roundtrip` | Compile all 8 modules, run `(main)` → 42 |

---

## Version bumps

| Package | Before | After |
|---------|--------|-------|
| `twig-ir-compiler` | 0.12.0 | 0.13.0 |
| `twig-module-driver` | 0.2.0 | 0.3.0 |

---

## Commit sequence

1. `docs(specs)` — this file
2. `feat(twig-ir-compiler)` — wire 13 string/char builtins into BUILTINS, bump 0.13.0
3. `feat(twig)` — `token.tw` (TkColon), `lexer.tw`, `parser.tw`, updated `main.tw`
4. `test(twig-module-driver)` — 7 tw05e integration tests, bump 0.3.0

---

## Divergence from plan

None — implementation matches the approved LANG58 plan exactly.

---

## What comes next (TW05-F)

TW05-F will add the resolver + IIR emitter in Twig: walking the `Expr` AST and
emitting `IirInstr` nodes via the `IirBuilder` API.  That completes the pipeline from
source text to IR, setting up TW05-G self-compilation.
