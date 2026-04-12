# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-04-12

### Added

- Initial implementation of the generic LSP framework for TypeScript
- `LanguageBridge` interface with `tokenize()` and `parse()` as required methods
- 10 optional provider interfaces: `HoverProvider`, `DefinitionProvider`,
  `ReferencesProvider`, `CompletionProvider`, `RenameProvider`,
  `SemanticTokensProvider`, `DocumentSymbolsProvider`, `FoldingRangesProvider`,
  `SignatureHelpProvider`, `FormatProvider`
- Type guard functions for runtime capability detection (e.g., `isHoverProvider()`)
- `DocumentManager` for tracking open file contents with incremental sync
- `ParseCache` for caching parse results by (URI, version) key
- `buildCapabilities()` for dynamic capability advertisement based on bridge
- `encodeSemanticTokens()` for the LSP compact delta encoding
- `semanticTokenLegend()` with standard LSP token types and modifiers
- `LspServer` class wiring together bridge, document manager, parse cache,
  and JSON-RPC server
- All LSP handlers: lifecycle, text document sync, and feature requests
- LSP error code constants matching the specification
- Full type definitions for all LSP data structures
- Comprehensive test suite covering UTF-16 handling, document management,
  parse caching, capabilities, semantic token encoding, and full JSON-RPC
  round-trip integration tests
