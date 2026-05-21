# coding-adventures-javascript-tokens

Backend-agnostic shared types for the JavaScript pipeline: ES version enum,
byte-range span, and the cross-crate `TokenKind` enum.

This crate sits at the **bottom** of the JS-pipeline dependency graph. It has
no dependencies (not even `serde`) so anything downstream — `javascript-lexer`,
`javascript-parser`, `javascript-ast`, the Closure-Compiler-clone crates, the
future V8-in-Rust clone — can pull it in without creating cycles.

## What's here today

- `EsVersion` — every ECMAScript edition with a grammar file under
  `code/grammars/ecmascript/`: `Es1`, `Es3`, `Es5`, `Es2015` through `Es2025`.
  Includes `latest()`, `as_str()` that matches the grammar file basenames,
  `FromStr`, and `Display`.
- `Span { start: u32, end: u32 }` — half-open `[start, end)` byte range.
  `const fn` constructors and accessors. Used by the lexer to record where
  tokens came from in the source, and by `correlation-vector` `Origin`
  records to anchor everything back to bytes.
- `TokenKind` — broad cross-version classification: `Name`, `Number`,
  `String`, `Regex`, `TemplateNoSub`/`Head`/`Middle`/`Tail`, `BigInt`,
  `PrivateName`, `Keyword`, `Operator`, `Punctuation`, `Comment`,
  `Whitespace`, `Newline`, `Hashbang`, `Error`, `Eof`, plus
  `Other(String)` for grammar-driven token names that need to round-trip
  (e.g. `"OPTIONAL_CHAIN"`, `"STAR_STAR_EQUALS"`). Methods: `is_trivia()`,
  `is_eof()`.

## Why this is a separate crate

[CLOC01](../../../specs/CLOC01-closure-compiler-overview.md) requires the JS
frontend to be shared between two backends: the Closure-Compiler-clone and the
future V8 clone. If `EsVersion` or `TokenKind` lived in `javascript-lexer`,
every backend would depend on the lexer for type definitions. Splitting them
out keeps the layering clean.
