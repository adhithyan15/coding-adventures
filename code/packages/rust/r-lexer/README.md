# R Lexer

A grammar-driven lexer (tokenizer) for the
[R language](https://en.wikipedia.org/wiki/R_(programming_language)) — "an
implementation of the S language" (Ihaka & Gentleman, 1993).

## What it does

Tokenizes R source text. Like every language frontend in this repo it does not
hand-write tokenization: it loads the compiled `r.tokens` grammar and feeds it
to the generic `GrammarLexer` from the `lexer` crate, then applies one
post-tokenize hook for R's context-sensitive newline rule.

## Built as a sibling of `s-lexer`

R's lexical structure is ~98% that of historical S, so this crate mirrors
`s-lexer`. The differences (and they are the whole point) are:

| | S | R |
|---|---|---|
| `_` | the **assignment** operator (not allowed in names) | an ordinary **name character** (`data_frame` is one name) |
| Right super-assign | — | `->>` |
| Typed `NA` | — | `NA_integer_`, `NA_real_`, `NA_character_` |

Everything else — `<-`/`<<-`/`->`, comparisons, arithmetic, `:`, `%op%`, `$`,
brackets, keywords, strings, `#` comments, and the bracket-interior newline
rule — is shared with S.

## Usage

```rust
use coding_adventures_r_lexer::tokenize_r;

let tokens = tokenize_r("data_frame <- 1\n");
assert_eq!(tokens[0].value, "data_frame"); // one name in R
```

Use `try_tokenize_r` for a `Result` instead of a panic.

## Regenerating the embedded grammar

`src/_grammar.rs` is generated from `code/grammars/r.tokens` with the
grammar-tools CLI (`grammar-tools compile-tokens code/grammars/r.tokens -o
src/_grammar.rs`) — never hand-edit it.

## Testing

```sh
cargo test -p coding-adventures-r-lexer
```

See [`code/specs/R00-r-language.md`](../../../specs/R00-r-language.md).
