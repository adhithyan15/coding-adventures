# cobol-lexer (coding-adventures-cobol-lexer)

Lexer for **COBOL-60** (CODASYL, the 1960 report) — the lexical layer of the
COBOL frontend and the descendant of the FLOW-MATIC lexer. It loads the compiled
`cobol.tokens` grammar (`code/grammars/cobol/cobol.tokens`) and feeds it to
the generic `lexer::GrammarLexer`; no tokenization is hand-written.

Implements the lexical layer of [PL07](../../../specs/PL07-cobol-60.md).

## API

```rust
use coding_adventures_cobol_lexer::{tokenize_cobol, strip_cobol_columns};

// Carded (80-column) source is handled automatically by the pre-tokenize hook:
let tokens = tokenize_cobol("000100 PROCEDURE DIVISION.\n000200     STOP RUN.");
```

## What's COBOL-specific here

Two things FLOW-MATIC didn't need:

1. **The fixed-column card format** — handled by the `strip_cobol_columns`
   **pre-tokenize hook** (a pure `String -> String` function registered via
   `GrammarLexer::add_pre_tokenize`). It drops the sequence area (cols 1–6) and
   identification area (cols 73–80), removes `*`/`/` comment lines, splices `-`
   continuation lines, and keeps the code area (cols 8–72). It is exported and
   unit-tested independently of the grammar.
2. **PICTURE strings** — context-sensitive. `X(20)` looks like a name, so it is
   lexed only after a `PIC`/`PICTURE` keyword, using the grammar's declarative
   **mode transition** (F10): the keyword switches into a `picture` group that
   matches one `PIC_STRING`, then switches back. The core picture pattern
   excludes the period, so `PIC X(20).` lexes as `PIC_STRING("X(20)")` then
   `DOT`.

Everything else reuses the FLOW-MATIC machinery: hyphenated `NAME`s, English
reserved words as case-insensitive `KEYWORD`s, numeric/quoted literals, and the
`. ( )` punctuation. Level numbers (`01`/`77`/…) lex as `NUMBER`; the parser
recognises a level by value and position. Note `STRING` is a reserved word (the
verb) *and* the type name of the quoted string-literal token — the two never
collide: keyword promotion only rewrites bare `NAME` words, while a quoted `"…"`
always lexes as the literal token regardless of the keyword list. `UNSTRING`
(with `END-UNSTRING`) is reserved the same way for the inverse verb. `INSPECT`
(with `TALLYING`, `REPLACING`, `LEADING`, `CHARACTERS`, `BEFORE`, `AFTER`, `FOR`,
and `END-INSPECT`) is reserved for the character-scan verb; `FIRST`/`INITIAL` stay
unreserved so common data names keep working (they are only needed by the
still-deferred `REPLACING FIRST` / `BEFORE INITIAL`).

## Scope

A historically faithful first cut: the column-strip hook, a focused reserved-word
subset, core PICTURE symbols (`9 X A V S P` + repetition), literals, and the
punctuation the verbs need — including `COLON` (`:`), which separates the start
and length in a reference modification (`WS-NAME(2:3)`). Enough to lex a complete
four-division program. Editing PICTUREs, the full ~300-word reserved list, and
Area A/B enforcement are future work (see PL07).

## Regenerating the grammar

`src/_grammar.rs` is generated from `code/grammars/cobol/cobol.tokens`:

```
cargo run --release --manifest-path code/programs/rust/grammar-tools/Cargo.toml \
  -- generate-rust-compiled-grammars cobol
```
