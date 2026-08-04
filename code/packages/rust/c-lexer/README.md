# c-lexer (coding-adventures-c-lexer)

Lexer for the **C integer-core subset** (SIR27) — the lexical layer of the
`c-to-semantic-ir` frontend.  It loads the compiled `c.tokens` grammar
(`code/grammars/c/c.tokens`) and feeds it to the generic `lexer::GrammarLexer`;
no tokenization is hand-written.

Implements the lexical part of
[SIR27](../../../specs/SIR27-c-to-semantic-ir.md).

## API

```rust
use coding_adventures_c_lexer::{tokenize_c, try_tokenize_c, create_c_lexer};
let tokens = tokenize_c("int32_t x = 5;");
```

## Subset notes

No context-sensitive hooks are needed: whole preprocessor lines (`#…`) are
dropped by the grammar's `skip:` section, and the `<stdint.h>` fixed-width type
names + `size_t` are lexed as **keywords** — so the two features that make full
C context-sensitive (the preprocessor and the typedef/identifier ambiguity)
never arise in v1.

## Regenerating the grammar

`src/_grammar.rs` is generated from `code/grammars/c/c.tokens`:

```
grammar-tools compile-tokens code/grammars/c/c.tokens -o code/packages/rust/c-lexer/src/_grammar.rs
```
