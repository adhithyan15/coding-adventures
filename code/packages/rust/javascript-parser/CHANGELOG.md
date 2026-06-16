# Changelog

All notable changes to the `coding-adventures-javascript-parser` crate will be documented in this file.

## [0.7.0] - 2026-06-15

### Changed
- Transitive upgrade: `coding-adventures-javascript-lexer` 0.8.0 (via `lexer`
  0.5.0) fixes gap-044b — template literal substitutions with non-identifier
  expressions no longer produce a LexerError.  No API changes in this crate.

## [0.6.0] - 2026-06-14

### Added
- New dependency on `coding-adventures-javascript-ast` for the typed ESTree AST.
- `pub mod bridge` — `GrammarASTNode → javascript_ast::Program` bridge module (CLOC12.136). Converts the generic grammar tree produced by `GrammarParser` into the fully typed AST consumed by all downstream optimization passes.
- `pub fn parse_javascript_program(source, EsVersion) -> Result<Program, String>` — convenience entry point that parses AND bridges in one call.
- `bridge::grammar_to_program(&GrammarASTNode, EsVersion) -> Result<Program, BridgeError>` — the core converter.
- `bridge::BridgeError` — typed error with two variants:
  - `UnsupportedSyntax { rule, location }` — Phase 2+ syntax not yet in the typed AST (async, generators, classes, for-in/of, try-catch, destructuring, template literals, optional chaining, `new` expressions, update expressions, sequence expressions, computed property keys, spread elements). Callers should degrade gracefully to WHITESPACE_ONLY / identity output.
  - `InternalError { msg, rule }` — bug in the bridge (node shape mismatch). Should not occur on valid input.

### Bridge coverage (Phase 1 subset)
**Statements** (12 variants): `block`, `if/else`, `while`, `for`, `continue`, `break`, `return`, `throw`, `switch`/`case`/`default`, `labeled`, `empty`, `expression_statement`, `variable_statement` (`var`), `lexical_declaration` (`let`/`const`), `function_declaration`.

**Expressions** (15 variants): `Identifier`, `NumericLiteral`, `StringLiteral`, `BooleanLiteral` (true/false), `NullLiteral`, `UndefinedLiteral`, `BigIntLiteral`, `BinaryExpression` (all 21 operators), `LogicalExpression` (`&&`/`||`/`??`), `UnaryExpression` (7 prefix operators), `AssignmentExpression` (13 operators), `ConditionalExpression` (ternary), `CallExpression`, `MemberExpression` (dot and computed), `ArrayExpression`, `ObjectExpression` (init properties, shorthand).

**Grammar routing**: handles the `optional_chain_expression` intermediate rule (the grammar's general suffix-chain node for dot access, bracket access, and call expressions — not just `?.` chains), the `new_expression` pass-through, and binary expression left-fold for precedence chains (`additive`, `multiplicative`, `shift`, etc.).

### Notes
- v1: all produced nodes carry `cv: None`. Per-node CV threading (source-byte → IR → engine-clause provenance) is CLOC12.137.
- Standalone assignment expressions (`x = y;`) are not yet parseable by the underlying grammar parser (ordered alternation matches `conditional_expression` first). This is a grammar-level gap, not a bridge limitation.
- Phase 1 unsupported constructs return `Err(UnsupportedSyntax)` rather than panicking, allowing `closurec` to degrade to identity output for files containing them.

### Tests
30 tests total (20 bridge + 10 existing parser tests):
- Literals: `empty_program`, `numeric_literal`, `string_literal`, `boolean_literal_true`, `null_literal`
- Declarations: `var_declaration`, `let_declaration`, `const_declaration`
- Expressions: `binary_add`, `logical_and`, `call_expression_roundtrip`
- Statements: `if_statement_no_else`, `if_statement_with_else`, `while_statement_bridge`, `switch_statement_bridge`
- Functions: `function_declaration`, `return_with_value`
- Error paths: `do_while_is_unsupported`

## [0.5.0] - 2026-05-21

### Added
- New dependencies on `coding_adventures_correlation_vector` (for `CVLog`, `Origin`) and `serde_json` (for contribution `meta` JSON values).
- `pub struct ProgramWithCv { pub ast: GrammarASTNode, pub cv: String }` — packages a parsed AST with its program-root CV identifier.
- `parse_javascript_with_cv(source, source_file, EsVersion, &mut CVLog) -> Result<ProgramWithCv, String>` — full CV-plumbed parse per CLOC03 §"Stage 2 — Parser" (v1: root-only). Behavior:
  - Tokenizes via `tokenize_javascript_with_cv` so every token gets its own CV ID.
  - Runs the underlying `GrammarParser` on the unwrapped tokens.
  - Mints the program-root CV via `cv.merge(all_token_cv_ids, Origin{source: source_file, location: "0:0", …})` so the program CV has every token as an ancestor.
  - Appends `Contribution { source: "parser", tag: "constructed", meta: { rule: <root rule name>, version: <es version> } }` per CLOC03.
- Module docs added a "Correlation-vector plumbing" section linking to CLOC03 and noting that v1 is root-only.
- 5 new tests:
  - `parse_with_cv_assigns_a_program_id`
  - `parse_with_cv_program_id_resolves_in_log` — `cv.get(id)` returns an entry whose `Origin.source = source_file` and `Origin.location = "0:0"`.
  - `parse_with_cv_appends_constructed_contribution` — `cv.history(id)` contains a `(source="parser", tag="constructed")` entry whose meta carries the correct `rule` and `version`.
  - `parse_with_cv_program_has_token_ancestors` — `cv.ancestors(id)` is non-empty (the merge step worked).
  - `parse_with_cv_disabled_log_still_returns_ast` — `CVLog::new(false)` keeps the API shape; the parser does not panic and still returns a valid AST.

### Notes
- All existing APIs (string-based, typed, no-CV) are untouched. This PR is purely additive.
- v1 is **root-only**: per-AST-node CV propagation requires deeper plumbing into `GrammarParser` (which today produces a generic `GrammarASTNode` tree, not the typed `javascript-ast::Program`). That work happens in a follow-up PR alongside the AST-typed parser output.
- The merge approach (program CV inherits from all tokens) gives source-map generators a reasonable starting point even with root-only plumbing: every output byte that comes from the program node resolves to the leftmost token's `Origin`.

## [0.4.0] - 2026-05-21

### Added
- New dependency on `coding-adventures-javascript-tokens` for the shared `EsVersion` enum.
- `create_javascript_parser_typed(source, EsVersion) -> Result<GrammarParser, String>` — typed constructor; no unknown-version error path.
- `parse_javascript_typed(source, EsVersion) -> Result<GrammarASTNode, String>` — typed parser.
- `pub const DEFAULT_ES_VERSION: EsVersion = EsVersion::Es2025;` — typed default.
- New tests covering the typed APIs: `parse_typed_es2015`, `default_es_version_constant_is_es2025`, `all_typed_versions_load`, `create_parser_typed`.

### Notes
- The existing `&str`-based APIs are kept for backwards compatibility. Typed APIs are the preferred surface going forward.
- The typed parser delegates to `javascript-lexer`'s `tokenize_javascript_typed`, so token/grammar versions are guaranteed to come from the same ECMAScript edition.

## [0.3.0] - 2026-05-20

### Removed
- Dropped support for the empty-string `""` "generic" version that pointed at the stub `code/grammars/javascript.grammar`. The full ES1 through ES2025 grammars under `code/grammars/ecmascript/` supersede it.
- Removed the embedded `mod generic` block (~103 lines) from `_grammar.rs`.

### Changed
- Crate docstring no longer mentions the "generic" grammar.

### Migration
- Replace `parse_javascript(source, "")` with `parse_javascript(source, "es2025")` (or another explicit ES version).

### Notes
- Rust-only first step of CLOC01 Phase 1 stub retirement. Other language ports (Go, Python, TypeScript, Ruby) get equivalent follow-up PRs; the stub `.grammar` source file is preserved until all ports migrate.

## [0.2.0] - 2026-04-05

### Changed
- `create_javascript_parser(source, version)` now accepts a `version: &str` parameter and returns `Result<GrammarParser, String>` instead of panicking.
- `parse_javascript(source, version)` now accepts a `version: &str` parameter and returns `Result<GrammarASTNode, String>` instead of panicking.

### Added
- Version-aware grammar selection: pass `""` for the generic grammar or one of `"es1"`, `"es3"`, `"es5"`, `"es2015"`–`"es2025"` for versioned ECMAScript grammars stored in `grammars/ecmascript/`.
- `grammar_root()` helper that uses `PathBuf` navigation from `env!("CARGO_MANIFEST_DIR")`.
- Returns `Err(String)` for unrecognised version strings instead of panicking on a missing file.
- The lexer is called with the same version string so tokens and grammar are always from the same ECMAScript edition.
- New tests: `test_versioned_es2015`, `test_all_versioned_grammars`, `test_unknown_version_returns_err`, `test_create_parser_unknown_version`.

## [0.1.0] - 2026-03-21

### Added
- `create_javascript_parser(source)` — factory function that loads `javascript.grammar` and returns a configured `GrammarParser`.
- `parse_javascript(source)` — convenience function that parses JavaScript source and returns a `GrammarASTNode`.
- Loads grammar from `javascript.grammar` using `env!("CARGO_MANIFEST_DIR")` for reliable path resolution.
- Test suite covering variable declarations, expressions, function declarations, if/else, while loops, for loops, multiple statements, empty programs, function calls, and the factory function.
