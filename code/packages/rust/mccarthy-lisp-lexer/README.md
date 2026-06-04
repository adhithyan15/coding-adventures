# mccarthy-lisp-lexer

Tokenizer for **McCarthy's 1960 Lisp** (Lisp 1.0).

L1 of the McCarthy Lisp implementation — see
[`MCCARTHY-LISP-PLAN.md`](../../../specs/MCCARTHY-LISP-PLAN.md).

## Grammar-driven — the lexer comes "for free"

This crate does **not** hand-write a tokenizer.  Every lexing rule
lives in [`code/grammars/mccarthy_lisp.tokens`](../../../grammars/mccarthy_lisp.tokens),
which `build.rs` compiles to Rust at build time.  The crate is a thin
wrapper that hands that grammar to the shared
[`GrammarLexer`](../lexer).  This is the same pattern used by
`twig-lexer`, `nib-lexer`, and `oct-lexer` — there is exactly one
lexer engine in the repo, and McCarthy Lisp joins it rather than
forking a second implementation that could silently drift.

## Why a distinct crate (vs the existing `lisp-lexer`)

The existing `lisp-lexer` targets a modern Scheme-ish Lisp: lowercase
symbols, strings, decimals, operator symbols (`+`, `-`, `*`, `<=`, …).
McCarthy's 1960 Lisp predates almost all of that:

| Feature              | Lisp 1.0 (this crate) | Modern Scheme (`lisp-lexer`) |
|----------------------|-----------------------|------------------------------|
| Symbol case          | all-uppercase         | mixed case                   |
| Strings              | none                  | yes (`"foo"`)                |
| Decimal numbers      | integers only         | yes                          |
| Operator symbols     | none                  | yes (`+`, `<=`, etc.)        |
| Comments             | `;` to end of line *  | `;` to end of line           |
| Quote sugar          | `'X`                  | `'X`                         |
| Dotted pair          | `(A . B)`             | `(A . B)`                    |

\* Comments weren't in McCarthy's 1958–1960 paper but were standardised
in Lisp 1.5 (1962).  We accept them in v0.1.0 for ergonomic test
sources.

The dialect restrictions are enforced entirely by the token regexes,
so they fall out of the grammar at no extra cost:

- `SYMBOL = /[A-Z][A-Z0-9-]*/` rejects lowercase source.
- `INTEGER = /-?[0-9]+/` requires a digit, so a bare `-` (an operator
  symbol, which Lisp 1.0 has none of) matches nothing and is a lex
  error.
- There is no `STRING` rule, so `"…"` is a lex error.

## Token kinds (per `mccarthy_lisp.tokens`)

| Grammar token | Source form        | Example  |
|---------------|--------------------|----------|
| `LPAREN`      | `(`                | `(`      |
| `RPAREN`      | `)`                | `)`      |
| `QUOTE`       | `'` (sugar)        | `'X`     |
| `DOT`         | `.` (cons sep)     | `.`      |
| `SYMBOL`      | `[A-Z][A-Z0-9-]*`  | `CAR`    |
| `INTEGER`     | `-?[0-9]+`         | `42`     |

## API

```rust
use mccarthy_lisp_lexer::tokenize_mccarthy;

let tokens = tokenize_mccarthy("(CAR '(A B C))").unwrap();
// Vec<lexer::token::Token>, ending with an EOF token.
// Inspect tokens via `t.effective_type_name()` and `t.value`.
```

- `tokenize_mccarthy(src) -> Result<Vec<Token>, LexerError>` — the
  whole token stream (incl. trailing `EOF`).
- `create_mccarthy_lexer(src) -> GrammarLexer` — the lexer object, for
  streaming / incremental use.
- `mccarthy_token_grammar_spec() -> &'static TokenGrammar` — the
  build-time-compiled grammar, for tooling (LSP, highlighters).
