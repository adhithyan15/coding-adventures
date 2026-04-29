# twig-lexer

Thin Rust wrapper around the generic [`GrammarLexer`](../lexer) driven by the canonical [`code/grammars/twig.tokens`](../../grammars/twig.tokens) file.  This is the first stage of the Rust [Twig](../../specs/TW00-twig-language.md) pipeline:

```
Twig source --> [twig-lexer] --> tokens --> [twig-parser] --> AST --> [twig-ir-compiler] --> IIRModule
```

Mirrors the same wrapper pattern used by every other Rust language frontend in this repo (`brainfuck`, `dartmouth-basic`, …) and by the Python [`twig` package](../../python/twig). The `.tokens` grammar file is the single source of truth — every Twig implementation reads it.

## Token kinds (per `twig.tokens`)

| Grammar name | `Token.type_`  | `Token.type_name`     | Effective name       |
|--------------|----------------|-----------------------|----------------------|
| `LPAREN`     | `LParen`       | `None`                | `"LPAREN"`           |
| `RPAREN`     | `RParen`       | `None`                | `"RPAREN"`           |
| `QUOTE`      | `Name`         | `Some("QUOTE")`       | `"QUOTE"`            |
| `BOOL_TRUE`  | `Name`         | `Some("BOOL_TRUE")`   | `"BOOL_TRUE"`        |
| `BOOL_FALSE` | `Name`         | `Some("BOOL_FALSE")`  | `"BOOL_FALSE"`       |
| `INTEGER`    | `Name`         | `Some("INTEGER")`     | `"INTEGER"`          |
| `KEYWORD`    | `Keyword`      | `Some("KEYWORD")`     | `"KEYWORD"`          |
| `NAME`       | `Name`         | `None`                | `"NAME"`             |
| (sentinel)   | `Eof`          | `None`                | `"EOF"`              |

The seven Twig keywords (`define`, `lambda`, `let`, `if`, `begin`, `quote`, `nil`) are promoted from `NAME` to `KEYWORD` at lex time. Whitespace and `;`-to-end-of-line comments are silently skipped.

## Usage

```rust
use twig_lexer::tokenize_twig;

let tokens = tokenize_twig("(define x 42)");
// [LParen, Keyword("define"), Name("x"), Integer("42"), RParen, Eof]
```

For incremental tokenisation or custom error handling, use `create_twig_lexer(source)` to get the underlying `GrammarLexer` directly.

## Why a wrapper, not a hand-written tokenizer?

- **Single source of truth.** Every Twig implementation reads `code/grammars/twig.tokens` (Python today; Ruby/Go/etc. in the future). Hand-writing the lexer would fork the grammar into a second implementation that could drift silently.
- **Shared infrastructure tests.** `grammar-tools` and `lexer` have their own test suites; a wrapper inherits that coverage for free.
- **Less code, fewer bugs.** The token grammar is data; the lexer logic is the same for every language.

## Tests

```bash
cargo test -p twig-lexer
```

16 unit tests covering every token kind, position tracking across newlines and comments, keyword promotion, the `-` / negative-integer disambiguation, and realistic shapes (factorial, quoted symbols, let-bindings).
