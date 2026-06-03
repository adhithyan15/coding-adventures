# mccarthy-lisp-lexer

Tokenizer for **McCarthy's 1960 Lisp** (Lisp 1.0).

L1 of the McCarthy Lisp implementation — see
[`MCCARTHY-LISP-PLAN.md`](../../../specs/MCCARTHY-LISP-PLAN.md).

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

## Token kinds

| Variant | Source form        | Example  |
|---------|--------------------|----------|
| `LParen`| `(`                | `(`      |
| `RParen`| `)`                | `)`      |
| `Quote` | `'` (sugar)        | `'X`     |
| `Dot`   | `.` (cons sep)     | `.`      |
| `Symbol`| `[A-Z][A-Z0-9-]*`  | `CAR`    |
| `Int`   | `-?[0-9]+`         | `42`     |
