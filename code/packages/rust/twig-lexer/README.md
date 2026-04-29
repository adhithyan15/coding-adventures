# twig-lexer

Tokenises [Twig](../../specs/TW00-twig-language.md) source — the Lisp-precursor language that runs on the LANG `vm-core` — into a flat stream of typed tokens.

This is the first stage of the Rust Twig pipeline:

```
Twig source --> [twig-lexer] --> tokens --> [twig-parser] --> AST --> [twig-ir-compiler] --> IIRModule
```

## Token kinds

| Kind        | Description                                | Example value |
|-------------|--------------------------------------------|---------------|
| `LParen`    | `(`                                        | `(`           |
| `RParen`    | `)`                                        | `)`           |
| `Quote`     | `'` quote prefix                            | `'`           |
| `BoolTrue`  | `#t` boolean literal                        | `#t`          |
| `BoolFalse` | `#f` boolean literal                        | `#f`          |
| `Integer`   | signed-integer literal `-?[0-9]+`           | `42`, `-7`    |
| `Keyword`   | one of `define`, `lambda`, `let`, `if`, `begin`, `quote`, `nil` | `define` |
| `Name`      | identifier / operator / predicate           | `+`, `null?`  |
| `Eof`       | end-of-input sentinel                       | `""`          |

Whitespace and `;`-to-end-of-line comments are silently skipped — they never reach the parser.

## How Twig differs from generic Lisp at the token level

This crate is a Twig-specific lexer (rather than a wrapper around the generic [`lisp-lexer`](../lisp-lexer)) for three reasons:

1. **Booleans are dedicated tokens.** `#t` and `#f` lex to `BoolTrue` / `BoolFalse`, not to a `Symbol("#t")`. The Twig parser uses these to populate `BoolLit` AST nodes without an extra string check.
2. **Reserved words are promoted to `Keyword`.** A name whose text matches one of the seven keywords becomes `Keyword`; everything else is `Name`. The parser dispatches on token kind, so a typo like `defin` cannot accidentally match the `define` rule.
3. **No string literals, no dotted-pair `.`.** Twig v1 has no strings, and cons cells are constructed via `(cons a b)`, never `(a . b)`. Removing those code paths keeps the lexer to a single screen.

## Position tracking

Every token records the 1-indexed `line` and `column` of its first character. These positions bubble up into the parser's typed AST nodes and into compiler error messages, so users can find the source of an error by line and column without re-running the source through a separate locator.

## Usage

```rust
use twig_lexer::{tokenize, TokenKind};

let tokens = tokenize("(define (square x) (* x x))").unwrap();
assert_eq!(tokens[0].kind, TokenKind::LParen);
assert_eq!(tokens[1].kind, TokenKind::Keyword);
assert_eq!(tokens[1].value, "define");
```

## Tests

```bash
cargo test -p twig-lexer
```

Coverage targets the same surface as the Python `tests/test_lexer.py`: every token kind, position tracking across newlines and comments, keyword promotion, the `-` / negative-integer disambiguation, and the lone-`#` error path.
