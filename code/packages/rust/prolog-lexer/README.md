# prolog-lexer (Rust)

ISO/Core Prolog tokenizer. Thin glue around the grammar-driven
[`lexer::GrammarLexer`], sourced from `code/grammars/prolog/iso.tokens`.

## What This Is

This crate is the Rust mirror of `code/packages/python/iso-prolog-lexer`.
Both implementations use the *same* `iso.tokens` grammar file via the
same `grammar-tools` + `GrammarLexer` pipeline; their token streams
agree by construction.

```text
   code/grammars/prolog/iso.tokens          (canonical, shared with Python)
        │
        │  cargo run -p prolog-lexer --example regenerate_grammar
        ▼
   src/_grammar.rs                          (auto-generated embedding)
        │
        ▼
   lexer::GrammarLexer                      (generic recognition engine)
        │
        ▼
   Vec<lexer::Token>                        (with trailing TokenType::Eof)
```

## Where It Fits

```text
   Prolog source text
        │
        ▼
   prolog-lexer            ← this crate
        │
        ▼
   prolog-parser           (next, consumes lexer::Token)
        │
        ▼
   prolog-loader (KB)
        │
        ▼
   logic-engine (LP19)
```

## Token Names (from iso.tokens)

`Token::effective_type_name()` returns the canonical grammar name for
every token:

| Token name | What it covers |
|---|---|
| `DCG` | `-->` |
| `QUERY` | `?-` |
| `RULE` | `:-` |
| `LPAREN` / `RPAREN` | parentheses |
| `LBRACKET` / `RBRACKET` | list brackets |
| `LCURLY` / `RCURLY` | curly braces |
| `BAR` | `\|` |
| `COMMA` | `,` |
| `SEMICOLON` | `;` |
| `CUT` | `!` |
| `DOT` | `.` |
| `INTEGER` | `42` |
| `FLOAT` | `3.14`, `2.5e-3` |
| `STRING` | `"..."` with escapes |
| `ATOM` | lowercase-led identifiers, quoted atoms, and symbolic atoms (aliased) |
| `VARIABLE` | uppercase- or `_`-led identifiers |
| `ANON_VAR` | a single `_` |

Whitespace and `%`-style comments are skipped before tokenization.

## API at a Glance

```rust
use prolog_lexer::tokenize_iso_prolog;
use lexer::token::TokenType;

let toks = tokenize_iso_prolog("father(homer, bart).");
for t in &toks {
    if t.type_ == TokenType::Eof {
        break;
    }
    println!("{} = {:?}", t.effective_type_name(), t.value);
}
```

For recoverable errors, use:

```rust
let mut lex = prolog_lexer::create_iso_prolog_lexer(source);
match lex.tokenize() {
    Ok(toks) => { /* ... */ }
    Err(e)   => { /* handle */ }
}
```

## Rust-vs-Python Grammar Adjustments

The Python `re` module supports negative look-ahead; the Rust `regex`
crate does not. The canonical `iso.tokens` uses look-ahead for
`ANON_VAR` (`_(?![A-Za-z0-9_])`). To keep the canonical grammar
pristine, `examples/regenerate_grammar.rs` applies two post-parse
transformations:

1. `ANON_VAR`'s pattern is rewritten to plain `_`.
2. `VARIABLE` is reordered to come before `ANON_VAR`, exploiting the
   lexer's first-match-wins semantics.

`_State` matches `VARIABLE`; `_` alone matches `ANON_VAR`. Semantically
identical to the look-ahead version. Both transformations are
documented in the generator.

## Regenerating the Embedded Grammar

```sh
cargo run -p prolog-lexer --example regenerate_grammar
```

The generated `src/_grammar.rs` is checked into the repo so this
crate's runtime has no file I/O. Re-run the example whenever
`iso.tokens` changes.

## Status

Experimental. Covers the ISO/Core Prolog subset needed for facts,
rules, top-level queries, lists, and symbolic operators. SWI-Prolog
dialect-specific extensions (e.g. `0'c` char literals, `0x1A` hex)
live in the SWI grammar and a separate `swi-prolog-lexer` follow-up.
