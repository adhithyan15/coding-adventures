# Changelog -- DartmouthBasicParser (Swift)

## [0.2.0] -- 2026-07-13

### Changed
- `loadGrammar()` now returns the grammar embedded at compile time in the
  generated `_Grammar.swift` instead of reading `code/grammars/**` via
  `#filePath` at run time. The lexer/parser no longer depends on the
  monorepo layout and works when published standalone. Grammar source of
  truth is unchanged; regenerate via `swift/grammar-tools`' `grammar-tools-embed`.

## [0.1.2] -- 2026-07-12

### Fixed
- `loadGrammar()` now reads the grammar from
  `grammars/dartmouth_basic/dartmouth_basic.grammar` after the per-grammar
  subdirectory move in PR #7475. The old flat path would have failed once the
  companion lexer's own grammar-path fix let this package build again.

## [0.1.1] -- 2026-04-10

### Fixed
- Removed the `relabelJumpTargets` pre-parse hook that was incorrectly
  converting `NUMBER` tokens to `LINE_NUM` for GOTO/GOSUB/IF-THEN targets.
  The grammar uses `NUMBER` (not `LINE_NUM`) for those positions, so the
  hook caused parse errors: `"Parse error: Expected NUMBER, got '50'"`.
  Jump targets like "50" in `GOTO 50` correctly remain as `NUMBER` tokens;
  the grammar matches them directly.

## [0.1.0] -- 2026-04-10

### Added

- Initial implementation of `DartmouthBasicParser`.
- `parse(_:)` -- parses Dartmouth BASIC (1964) source text into an `ASTNode` tree.
- `parseTokens(_:)` -- parses a pre-lexed `[Token]` array into an `ASTNode` tree.
- `loadGrammar()` -- loads and parses `dartmouth_basic.grammar` from the monorepo.
- `relabelJumpTargets(_:)` -- pre-parse hook that promotes NUMBER tokens after
  GOTO/GOSUB/THEN keywords to LINE_NUM, bridging the lexer/grammar mismatch for
  jump-target positions.
- Grammar-driven parsing via `GrammarParser` from the `Parser` package, with
  full coverage of all 17 Dartmouth BASIC 1964 statement types.
- Comprehensive XCTest suite covering all statement types, expression precedence,
  right-associative exponentiation, empty lines, and multi-statement programs.
- `BUILD` and `BUILD_windows` scripts.
- `.gitignore` with `.build/`.
- `required_capabilities.json` declaring `filesystem:read`.
