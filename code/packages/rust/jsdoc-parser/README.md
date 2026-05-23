# coding-adventures-jsdoc-parser

Grammar-driven parser for the **interior** of a `/** ... */` JSDoc block
comment. Per [CLOC05](../../../specs/CLOC05-jsdoc-sub-pipeline.md).

The parser grammar lives at
[`code/grammars/jsdoc/jsdoc.grammar`](../../../grammars/jsdoc/jsdoc.grammar)
and is compiled to native Rust at build time via
`grammar-tools compile-grammar`, embedded as `mod _grammar`.

## What's here (v1)

- `create_jsdoc_parser(source)` → `GrammarParser`
- `parse_jsdoc(source)` → `Result<GrammarASTNode, String>`
- v1 recognises `@type`, `@param`, `@returns` (and `@return`) explicitly,
  with a fallback `unknown_tag` rule for everything else (so `@throws`,
  `@template`, etc. parse without errors and round-trip).
- Type-expression coverage: nominal references with dotted paths, array
  suffix `Foo[]`, nullable `?Foo`, non-nullable `!Foo`, variadic `...Foo`,
  optional `Foo=`, parens, wildcard `*`.

## What's coming (follow-up PRs)

Per CLOC05:
- Object record types `{ k: T, l: U }`.
- Generic type arguments `<T, U>`.
- Union (`A|B`) / intersection (`A&B`) inside type expressions.
- Template literal types.
- Named-tag rules for the rest of the JSDoc tag set (`@template`,
  `@typedef`, `@callback`, `@property`, `@implements`, `@extends`,
  `@constructor`, `@enum`, `@deprecated`, `@public`/`@private`,
  `@override`, `@readonly`, `@const`, `@pure`/`@nosideeffects`,
  `@suppress`, etc.).
- `jsdoc-ast` typed AST crate (the current output is the generic
  `GrammarASTNode`).
- `jsdoc-comment-extractor` (pulls comment ranges from a
  `javascript-ast::Program`).
- `jsdoc-types-extractor` (walks the JSDoc AST → emits
  `type-sidecar::Sidecar` records per CLOC04).
