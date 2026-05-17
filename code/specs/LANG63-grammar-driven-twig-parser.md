# LANG63 — Grammar-Driven Twig Lexer and CST Parser

**Status:** In Progress
**Branch:** `feat/lang62-tw05i-self-compilation-check`
**Depends on:** LANG62 (TW05-I: first self-compilation check)

---

## Overview

LANG63 replaces the hand-written Twig lexer and recursive-descent parser
(written in LANG58) with versions generated from the formal grammar files
(`twig.tokens` and `twig.grammar`) using the `grammar-tools` CLI program.

Three files change in `code/twig/compiler/`:

1. **`lexer.tw`** — regenerated from `twig.tokens`.  Drops the hand-authored
   dispatch chain; the chain is now produced mechanically by
   `grammar-tools compile-tokens-twig`.

2. **`cst-parser.tw`** (new) — generated from `twig.grammar`.  A
   grammar-driven recursive-descent parser that returns a Concrete Syntax Tree
   (CST) rather than an AST.  Exported functions: `cst-parse-program` and
   `cst-parse-expr`.

3. **`parser.tw`** — rewritten as a CST→AST extraction layer.  The structural
   recognition work is fully delegated to `cst-parser.tw`; this module walks
   the resulting CST and produces typed `Expr` AST nodes.  The public interface
   (`parse-program`, `parse-expr`) is unchanged.

Two new CLI commands are added to `grammar-tools` (the Go program at
`code/programs/go/grammar-tools/`):

- `compile-tokens-twig <file.tokens> [-o output.tw]`
- `compile-grammar-twig <file.grammar> [<file.tokens>] [-o output.tw]`

The corresponding generator functions live in the `grammar-tools` Go library
package (`code/packages/go/grammar-tools/`), in the new `twig_codegen.go`
file.

Finally, `twig-module-driver` (`code/packages/rust/twig-module-driver/`) is
updated to include the new `"cst-parser"` module in every copy-list that
previously included `"parser"`.

---

## Background and Motivation

The project rule (recorded in `MEMORY.md` as `feedback_no_handwritten_lexers_parsers.md`)
states:

> New language frontends MUST wrap GrammarLexer/GrammarParser using .tokens/.grammar
> files in code/grammars/ — never hand-write.

LANG58's hand-written `lexer.tw` and `parser.tw` predate this rule.  LANG63
brings the Twig self-hosted compiler into compliance:

- The lexer becomes a generated dispatch chain, driven by `twig.tokens`.
- The parser splits into two layers: a generated CST parser (mechanical
  structure) and a hand-written CST→AST extractor (semantic meaning).  The
  CST→AST layer is allowed to remain hand-written because it embodies semantic
  decisions (which CST nodes map to which AST variants), not structural
  recognition.

There is a secondary benefit: the grammar files now serve as the single source
of truth for the Twig language syntax.  Any change to the token set or grammar
rules will regenerate `lexer.tw` and `cst-parser.tw` automatically.

---

## Architecture Overview

The parsing pipeline after LANG63 is three layers deep:

```
source string
    │
    ▼  compiler/lexer (generated from twig.tokens)
token list  [Token records]
    │
    ▼  compiler/cst-parser (generated from twig.grammar)
CST node    (cons "rule-name" (list child …)) or Token for atoms
    │
    ▼  compiler/parser (hand-written CST→AST extractor)
AST node    Expr union variant (IntLit, DefExpr, LambdaExpr, …)
```

Callers (including `main.tw` and all `twig-module-driver` tests) use only
the public API of `compiler/parser`, which is unchanged from LANG58.

---

## grammar-tools CLI Commands Added

### `compile-tokens-twig <file.tokens> [-o output.tw]`

Parses `file.tokens`, validates it (unless `--force`), then calls
`GenerateTwigLexer` to produce the full `lexer.tw` content.  Writes to
`output.tw` if `-o` is given, otherwise prints to stdout.

Example:

```
grammar-tools compile-tokens-twig twig.tokens -o code/twig/compiler/lexer.tw
```

### `compile-grammar-twig <file.grammar> [<file.tokens>] [-o output.tw]`

Parses `file.grammar` and, optionally, a companion `file.tokens` (for keyword
and token-kind information).  Calls `GenerateTwigParser` to produce the full
`cst-parser.tw` content.  If `file.tokens` is omitted, the tool looks for a
file with the same base name and a `.tokens` extension in the same directory.

Example:

```
grammar-tools compile-grammar-twig twig.grammar twig.tokens -o code/twig/compiler/cst-parser.tw
```

---

## generator functions (grammar-tools library)

Two new exported functions in `code/packages/go/grammar-tools/twig_codegen.go`:

### `GenerateTwigLexer(grammar *TokenGrammar, sourceFile string) string`

Generates the complete text of `lexer.tw` from a parsed `TokenGrammar`.

Output structure:

1. Module header with `DO NOT EDIT` warning and regeneration command.
2. Character predicates: `lex-digit?`, `lex-whitespace?`, `lex-name-start?`,
   `lex-name-continue?`.
3. Scan helpers: `lex-scan-name`, `lex-scan-digits`, `lex-scan-string`,
   `lex-skip-whitespace`, `lex-skip-comment`.
4. Main loop: `lex-emit` and `lex-loop`.
5. Dispatch chain: one `lex-dispatch-N` function per token definition plus
   one per skip pattern, in grammar-file order.  Each function tail-calls
   `lex-dispatch-(N+1)` on no-match.  The final function calls `lex-unknown`.
6. Error fallback: `lex-unknown` (skips one byte and continues).
7. Public API: `lex-source`.

Token name → Twig TokenKind mapping (from `twigKindFor`):

| Grammar token | Twig kind      |
|---------------|----------------|
| `LPAREN`      | `TkLParen`     |
| `RPAREN`      | `TkRParen`     |
| `QUOTE`       | `TkQuote`      |
| `COLON`       | `TkColon`      |
| `ARROW`       | `TkIdentifier` |
| `BOOL_TRUE`   | `TkBoolean`    |
| `BOOL_FALSE`  | `TkBoolean`    |
| `STRING`      | `TkString`     |
| `INTEGER`     | `TkInteger`    |
| `NAME`        | `TkIdentifier` |

`ARROW` and `NAME` both map to `TkIdentifier`.  The dispatch chain handles
this correctly because `ARROW` appears before `NAME` in the grammar, so `->` is
consumed as a single unit before the name fallback is tried.

### `GenerateTwigParser(pg *ParserGrammar, tg *TokenGrammar, sourceFile string) string`

Generates the complete text of `cst-parser.tw` from a parsed `ParserGrammar`
plus a `TokenGrammar` (used for keyword set and token-kind information).

Output structure:

1. Module header with `DO NOT EDIT` warning and regeneration command.
2. CST node format documentation block.
3. Token-matching helpers: one `cst-match-<Kind>` function per unique Twig
   kind, plus `cst-match-kw` for keyword matching.
4. Grammar rules in file order: one or more Twig functions per rule
   (main function, plus helper functions for repetitions, optionals, and
   alternation sub-sequences).

Function naming convention:

| Pattern                         | Purpose                                    |
|---------------------------------|--------------------------------------------|
| `cst-parse-<rule>`              | Entry point for the named rule             |
| `cst-parse-<rule>-alt<n>`       | Sub-function for the Nth alternation arm   |
| `cst-parse-<rule>-opt<n>`       | Optional element wrapper (always succeeds) |
| `cst-parse-<rule>-rep<n>`       | Zero-or-more accumulator (always succeeds) |

---

## CST Node Format

CST nodes returned by `cst-parser.tw` follow a uniform convention:

| Node type          | Twig representation                                    |
|--------------------|--------------------------------------------------------|
| Non-terminal       | `(cons "rule-name" (list child0 child1 …))`            |
| Terminal (token)   | Raw `Token` record from `compiler/token`               |
| Empty optional     | `nil`                                                  |
| Parse success      | `(cons rest-tokens cst-node)`                          |
| Parse failure      | `nil`                                                  |

Non-terminals carry the grammar rule name as a string in their `car`.  The
`cdr` is a Twig list of child nodes, in the same order as the grammar rule
sequence.  Terminals are passed through as-is from the lexer.

---

## Key Design Insight: Detecting Token Records vs. CST Non-Terminals

The `token?` predicate (generated from the `Token` record definition) expands
to `(pair? v)`, because `token?` checks whether a value is a non-nil cons
pair.  This creates an ambiguity: both Token records and named CST non-terminal
nodes are cons pairs.

The reliable discriminant is the `car`:

- **Token record**: `(cons kind lexeme span)` — `car` is a `TokenKind`
  pair (itself a cons cell).
- **Named CST non-terminal**: `(cons "rule-name" kids)` — `car` is a
  Twig string (not a pair).

Therefore, in `parser.tw`'s `extract-expr` dispatcher:

```scheme
(if (pair? (car cst)) (extract-atom cst)    ; Token record
    (let* ((rule (car cst))) ...))           ; Named CST non-terminal
```

`(pair? (car cst))` returns `#t` for Token records (whose `car` is a
`TokenKind` union value, which is itself a cons pair) and `#f` for named CST
nodes (whose `car` is a string).

This insight replaces the previous approach of calling `token?` directly, which
would match any cons pair and produce incorrect results on named CST nodes.

---

## CST Rule → AST Node Mapping (in parser.tw)

| CST rule name          | AST node produced                                          |
|------------------------|------------------------------------------------------------|
| `Token` (any atom)     | `IntLit`, `BoolLit`, `StrLit`, `SymLit`, `NilLit`, `VarRef` |
| `"quoted"`             | `SymLit` (`'foo` → the symbol string)                      |
| `"if-form"`            | `IfExpr`                                                   |
| `"let-form"`           | `LetExpr`                                                  |
| `"let-star-form"`      | `LetExpr` (same representation as `let`)                   |
| `"begin-form"`         | `BeginExpr`                                                |
| `"lambda-form"`        | `LambdaExpr`                                               |
| `"quote-form"`         | `SymLit`                                                   |
| `"define"`             | `DefExpr` (+ optional `LambdaExpr` for function shorthand) |
| `"apply"`              | `CallExpr`                                                 |
| `"match-form"`         | `CallExpr "match"` (best-effort encoding)                  |
| `"module-form"`        | `CallExpr "module"` (skipped by emitter)                   |
| `"record-def"`         | `CallExpr "record"` (skipped by emitter)                   |
| `"union-def"`          | `CallExpr "union"` (skipped by emitter)                    |
| `"type-alias"`         | `CallExpr "type"` (skipped by emitter)                     |

Meta-forms (`module-form`, `record-def`, `union-def`, `type-alias`) produce
named `CallExpr` nodes so that future consumers can still identify the form by
its keyword, but the emitter's `emit-top-level-form` skips everything that is
not a `DefExpr(LambdaExpr)`.

Multi-body forms (`let`, `let*`, `lambda`, function-shorthand `define`) wrap
multiple body expressions in a `BeginExpr` using the `make-body` helper, which
passes a single expression through unchanged.

---

## Public Interface (Unchanged)

`compiler/parser` exports the same two functions as before LANG63:

```scheme
; Parse one expression from the token stream.
; Returns (cons rest-tokens ast-node) on success, nil on failure.
(parse-expr tokens)

; Parse all top-level forms; return a flat list of Expr AST nodes.
(parse-program tokens)
```

All callers are unaffected by the internal refactoring.  The `main.tw` smoke
test and all `twig-module-driver` integration tests continue to call
`lex-source`, `parse-program`, and `emit-program` in the same way.

---

## Files Changed

| File | Change |
|------|--------|
| `code/packages/go/grammar-tools/twig_codegen.go` | New — `GenerateTwigLexer` + `GenerateTwigParser` + helpers |
| `code/programs/go/grammar-tools/main.go` | Added `compile-tokens-twig` and `compile-grammar-twig` CLI commands and handler functions |
| `code/twig/compiler/lexer.tw` | Regenerated from `twig.tokens` (replaces LANG58 hand-written version) |
| `code/twig/compiler/cst-parser.tw` | New — generated CST parser from `twig.grammar` |
| `code/twig/compiler/parser.tw` | Rewritten as CST→AST extraction layer; now imports `compiler/cst-parser` |
| `code/packages/rust/twig-module-driver/src/lib.rs` | Added `"cst-parser"` to all `copy_all_tw_modules` and module copy lists in tw05e/f/g/h/i tests |

Grammar source files (not modified by LANG63, but consumed by the generators):

| File | Used by |
|------|---------|
| `code/grammars/twig.tokens` | `compile-tokens-twig` → `lexer.tw` |
| `code/grammars/twig.grammar` | `compile-grammar-twig` → `cst-parser.tw` |

---

## twig-module-driver: copy-list Updates

Every `copy_all_tw_modules` helper in `twig-module-driver/src/lib.rs` that
previously copied `["span", "token", "ast", "lexer", "parser", …]` now also
copies `"cst-parser"`.  This applies to all five test modules:

- `tw05e_tests::copy_all_tw_modules`
- `tw05f_tests::copy_all_tw_modules`
- `tw05g_tests::copy_all_tw_modules`
- `tw05h_tests::copy_all_tw_modules`
- `tw05i_tests::copy_all_tw_modules`

Without `"cst-parser"`, the Twig module driver cannot resolve
`(import compiler/cst-parser)` in `parser.tw`, and all integration tests that
call `parse-program` or `parse-expr` would fail with an unresolved-import
error.

---

## Verification

All existing `twig-module-driver` tests continue to pass unchanged after
LANG63.  The public API of `compiler/parser` is identical to LANG58:

- Lexer tests (`tw05e_tests`): `lex-source` output format is identical — the
  generated dispatch chain produces the same `Token` records as the hand-written
  version.
- Parser tests (`tw05e_tests` through `tw05i_tests`): `parse-program` output
  is semantically identical to the LANG58 hand-written parser — the CST→AST
  extraction layer maps every rule to the same `Expr` variant.
- Full-pipeline tests (all `full_lex_parse_emit_*`): `(main) = 2` result is
  unchanged.

---

## What Comes Next (TW05-J)

TW05-J will expand self-compilation checks to `diagnostic.tw` and `token.tw`
— modules with richer union/record structures.  LANG63's grammar-driven
architecture is a prerequisite: the grammar files must stay accurate as new
language constructs are added, and the generators enforce that the lexer and
CST parser always reflect the authoritative grammar.
