# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-12

### Added

- Initial release of the ls00 LSP framework for Swift.
- `LanguageBridge` protocol with required `tokenize()` and `parse()` methods.
- 10 optional provider protocols: HoverProvider, DefinitionProvider, ReferencesProvider, CompletionProvider, RenameProvider, SemanticTokensProvider, DocumentSymbolsProvider, FoldingRangesProvider, SignatureHelpProvider, FormatProvider.
- All LSP types: Position, Range, Location, Diagnostic, Token, TextEdit, WorkspaceEdit, HoverResult, CompletionItem, SemanticToken, DocumentSymbol, FoldingRange, SignatureHelpResult.
- `DocumentManager` for tracking open files with incremental change support and UTF-16 offset conversion.
- `ParseCache` for avoiding redundant parses (keyed by URI + version).
- Dynamic capability builder that inspects bridge protocol conformance at runtime.
- Semantic token encoder producing LSP's compact delta-format integer array.
- LSP-specific error codes (ServerNotInitialized, RequestFailed, etc.).
- Full server with all LSP handler methods wired to JSON-RPC dispatch.
- Comprehensive test suite covering UTF-16 conversion, document management, parse caching, semantic token encoding, capabilities, and server lifecycle.
