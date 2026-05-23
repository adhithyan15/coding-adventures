# Changelog

All notable changes to the `coding-adventures-jsdoc-parser` crate will be documented in this file.

## [0.1.0] - 2026-05-22

### Added
- New crate per CLOC05 Phase 1.
- `create_jsdoc_parser(source) -> GrammarParser` and `parse_jsdoc(source) -> Result<GrammarASTNode, String>` factory + convenience functions. Output rule: `document`.
- `_grammar.rs` auto-generated from `code/grammars/jsdoc/jsdoc.grammar` via `grammar-tools compile-grammar`. v1 grammar covers `@type`, `@param`, `@returns`/`@return` as named tags; every other `@xxx` tag falls through to an `unknown_tag` rule that round-trips the payload tokens until the next `NEWLINE`.
- Type-expression rules: nominal types with dotted paths (`Foo.Bar`), array suffix (`Foo[]`), nullable (`?Foo`), non-nullable (`!Foo`), variadic (`...Foo`), optional (`Foo=`), parenthesised grouping, and the wildcard `*`.
- 10 tests covering: empty document, type tag, returns tag, param tag with name, multi-tag documents, nullable type, array type, dotted nominal type, unknown tag tolerance, factory path.

### Notes
- Output is the generic `GrammarASTNode` tree. A typed `jsdoc-ast` crate per CLOC05 is a follow-up.
- Deferred grammar features: object records, generic args, union/intersection inside type expressions, template literal types, and the long tail of named JSDoc tags.
