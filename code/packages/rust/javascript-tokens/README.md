# coding-adventures-javascript-tokens

Backend-agnostic shared types for the JavaScript pipeline: ES version enum,
and (in follow-up PRs) the cross-crate `TokenKind` enum and `Span` type.

This crate sits at the **bottom** of the JS-pipeline dependency graph. It has
no dependencies (not even `serde`) so anything downstream — `javascript-lexer`,
`javascript-parser`, `javascript-ast`, the Closure-Compiler-clone crates, the
future V8-in-Rust clone — can pull it in without creating cycles.

## What's here today

- `EsVersion` — every ECMAScript edition with a grammar file under
  `code/grammars/ecmascript/`: `Es1`, `Es3`, `Es5`, `Es2015` through `Es2025`.
  Includes `latest()`, `as_str()` that matches the grammar file basenames,
  `FromStr`, and `Display`.

## What's coming

Per [CLOC02](../../../specs/CLOC02-javascript-ast.md):

- `TokenKind` — the union enum across every ES version's `.tokens` file.
- `Span` — `{ start: u32, end: u32 }`, decoupled from the CV log.

These ship in their own PRs to keep the diffs small.

## Why this is a separate crate

[CLOC01](../../../specs/CLOC01-closure-compiler-overview.md) requires the JS
frontend to be shared between two backends: the Closure-Compiler-clone and the
future V8 clone. If `EsVersion` or `TokenKind` lived in `javascript-lexer`,
every backend would depend on the lexer for type definitions. Splitting them
out keeps the layering clean.
