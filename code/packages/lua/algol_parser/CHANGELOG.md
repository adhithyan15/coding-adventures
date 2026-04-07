# Changelog — coding-adventures-algol-parser (Lua)

All notable changes to this package are documented here.

## [0.1.0] — 2026-04-06

### Added

- Initial implementation of `coding_adventures.algol_parser`.
- `parse(source)` — parses an ALGOL 60 string using the shared `algol.grammar`
  grammar and the grammar-driven `GrammarParser` from `coding-adventures-parser`.
  Tokenizes input via `coding-adventures-algol-lexer`.
- `create_parser(source)` — tokenizes source and returns an initialized
  `GrammarParser` without immediately parsing; useful for trace-mode debugging.
- `get_grammar()` — returns the cached `ParserGrammar` for direct use.
- Grammar is read from `code/grammars/algol.grammar` once and cached.
- Path navigation uses `debug.getinfo` to locate the grammar file relative to
  the installed module, avoiding hardcoded absolute paths.
- Supports the full ALGOL 60 grammar:
  - Top-level: `program` → `block`
  - Block structure with declarations before statements
  - Declarations: `type_decl`, `array_decl`, `switch_decl`, `procedure_decl`
  - Statements: `assign_stmt`, `cond_stmt`, `for_stmt`, `goto_stmt`,
    `proc_stmt`, `compound_stmt`, `empty_stmt`, labeled statements
  - Dangling-else resolution: `unlabeled_stmt` excludes conditionals
  - Expressions: `arith_expr`, `bool_expr`, `desig_expr` with full precedence
  - Left-associative exponentiation (per ALGOL 60 report)
- Comprehensive busted test suite covering:
  - Minimal program: `begin integer x; x := 42 end`
  - Assignment statements
  - If/then and if/then/else
  - For loops (step/until, while, simple)
  - Nested blocks
  - Boolean expressions
  - Grammar inspection (rule names, rule count)
  - `create_parser` API
  - Error handling for invalid programs
- `required_capabilities.json` declaring `filesystem:read`.
- `BUILD` and `BUILD_windows` scripts with transitive dependency installation
  in leaf-to-root order (includes `algol_lexer` as an extra dependency).
