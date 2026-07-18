# Changelog

All notable changes to the `coding-adventures-jsdoc-parser` crate will be documented in this file.

## [0.1.1] - 2026-07-18

### Fixed
- **Security hardening**: `create_jsdoc_parser` never called `GrammarParser::with_max_depth`, leaving every caller exposed to a native-stack-overflow DoS from an adversarial `@type {(((...)))}` payload. Added a `MAX_RULE_DEPTH = 200` cap, derived from measuring `jsdoc.grammar`'s one self-referential recursion shape (nested parenthesised type expressions) — safe at 289, crashes at 290. Cap sits ~31% below that. Notable surprise: exceeding the cap does not produce a parse error — `jsdoc.grammar`'s own `unknown_tag` catch-all (documented as a deliberate "unknown tags survive and round-trip" fallback) absorbs the too-deep payload as an unstructured tag instead, so the overall parse still succeeds, just with a different (degraded but harmless) tree shape. 3 new depth-guard regression tests assert "does not crash" rather than "returns Err" for this reason.

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
