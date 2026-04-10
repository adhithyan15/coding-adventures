# Changelog -- DartmouthBasicParser (Swift)

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
