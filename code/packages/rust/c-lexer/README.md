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

The lexer exposes `#`, `##`, and `.` so PREP01 can receive directive tokens,
including local and system header names. It still skips whitespace and comments.
The legacy source-input C parser ignores directive lines until the PREP01 C
dialect is connected; its token-input API expects directive-free tokens.
Fixed-width type names and `size_t` remain **keywords** in this subset.

## Regenerating the grammar

`src/_grammar.rs` is generated from `code/grammars/c/c.tokens`:

```
grammar-tools compile-tokens code/grammars/c/c.tokens -o code/packages/rust/c-lexer/src/_grammar.rs
```
