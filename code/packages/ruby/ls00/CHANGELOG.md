# Changelog

## 0.1.0 — 2026-04-12

### Added

- Initial release of the generic LSP framework for Ruby.
- `LspServer` class wiring JSON-RPC transport, document manager, and parse cache.
- `DocumentManager` for tracking open file contents with incremental change support.
- `ParseCache` for avoiding redundant parses using (uri, version) cache keys.
- Full UTF-16 to UTF-8 byte offset conversion for correct cursor positioning.
- Dynamic capability advertisement based on bridge's `respond_to?` checks.
- Semantic token encoding with delta-based compact integer format.
- All LSP data types as Ruby Struct classes (Position, Range, Location, etc.).
- LSP-specific error code constants.
- Duck-typed LanguageBridge interface documentation.
- Handlers for all standard LSP features:
  - Lifecycle: initialize, initialized, shutdown, exit
  - Text sync: didOpen, didChange, didClose, didSave
  - Features: hover, definition, references, completion, rename, documentSymbol, semanticTokens/full, foldingRange, signatureHelp, formatting
- Comprehensive test suite ported from Go implementation with 35+ test cases.
