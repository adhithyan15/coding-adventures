# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-04-12

### Added

- Initial release of the Python ls00 LSP framework
- Port of Go implementation at `code/packages/go/ls00/`
- `LanguageBridge` protocol with `tokenize()` and `parse()` as required methods
- 10 optional provider protocols using `@runtime_checkable`:
  - `HoverProvider` -- hover tooltips
  - `DefinitionProvider` -- Go to Definition (F12)
  - `ReferencesProvider` -- Find All References
  - `CompletionProvider` -- autocomplete suggestions
  - `RenameProvider` -- symbol rename (F2)
  - `SemanticTokensProvider` -- semantic syntax highlighting
  - `DocumentSymbolsProvider` -- document outline panel
  - `FoldingRangesProvider` -- code folding
  - `SignatureHelpProvider` -- function signature hints
  - `FormatProvider` -- document formatting
- `DocumentManager` with UTF-16 offset conversion for LSP compatibility
- `ParseCache` with `(uri, version)` keyed caching
- Dynamic capability advertisement via `isinstance()` checks
- Semantic token encoding in LSP's compact delta format
- Full LSP lifecycle: initialize -> didOpen -> features -> shutdown -> exit
- All LSP types as Python dataclasses with full type annotations
- LSP-specific error codes (ServerNotInitialized, RequestFailed, etc.)
- Comprehensive test suite with 40+ tests covering:
  - UTF-16 offset conversion (10 test cases including emoji, CJK, BMP)
  - DocumentManager lifecycle (7 tests)
  - ParseCache hit/miss behavior (5 tests)
  - Capability advertisement (7 tests)
  - Semantic token encoding (7 tests)
  - Full JSON-RPC round-trip handler tests (15+ tests)
