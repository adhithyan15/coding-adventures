# Changelog

All notable changes to `ls00` will be documented here.

## [0.1.0] — 2026-04-11

### Added

**Initial implementation of the generic LSP server framework.**

Core package structure:

- `types.go` — all shared LSP data types: `Position`, `Range`, `Location`, `Diagnostic`, `Token`, `ASTNode`, `CompletionItem`, `SemanticToken`, `DocumentSymbol`, `FoldingRange`, `TextEdit`, `WorkspaceEdit`, `HoverResult`, `SignatureHelpResult`, and related constants (`DiagnosticSeverity`, `SymbolKind`, `CompletionItemKind`)
- `language_bridge.go` — `LanguageBridge` (required minimum: `Tokenize`, `Parse`) plus 10 optional provider interfaces: `HoverProvider`, `DefinitionProvider`, `ReferencesProvider`, `CompletionProvider`, `RenameProvider`, `SemanticTokensProvider`, `DocumentSymbolsProvider`, `FoldingRangesProvider`, `SignatureHelpProvider`, `FormatProvider`
- `document_manager.go` — `DocumentManager` with UTF-16 offset conversion; handles both full and incremental text sync modes; `ConvertUTF16OffsetToByteOffset` exported for testing
- `parse_cache.go` — `ParseCache` keyed by `(uri, version)` with automatic eviction on document close
- `capabilities.go` — `BuildCapabilities()` using runtime type assertions; `SemanticTokenLegend()` with all 23 standard token types and 10 modifiers; `EncodeSemanticTokens()` implementing the LSP delta encoding
- `lsp_errors.go` — LSP-specific error codes: `ServerNotInitialized`, `RequestFailed`, `ServerCancelled`, `ContentModified`, `RequestCancelled`
- `server.go` — `LspServer` wiring bridge + document manager + parse cache + JSON-RPC server; `sendNotification()` for server-pushed messages; helper functions `positionToLSP`, `rangeToLSP`, `locationToLSP`, `parsePosition`, `parseURI`

Lifecycle handlers:

- `handlers.go` — `initialize` (returns capabilities + serverInfo), `initialized` (no-op), `shutdown` (sets flag, returns null), `exit` (calls `os.Exit(0/1)` based on shutdown flag)

Text document handlers (all in `handler_text_document.go`):

- `textDocument/didOpen` — opens document, parses, pushes diagnostics
- `textDocument/didChange` — applies incremental or full changes, re-parses, pushes diagnostics
- `textDocument/didClose` — removes document and clears diagnostics
- `textDocument/didSave` — re-syncs if full text is sent

Feature handlers:

- `handler_hover.go` — `textDocument/hover` → returns MarkupContent(markdown) with optional range
- `handler_definition.go` — `textDocument/definition` → returns Location or null
- `handler_references.go` — `textDocument/references` → returns Location array (respects `includeDeclaration`)
- `handler_completion.go` — `textDocument/completion` → returns CompletionList with `isIncomplete: false`
- `handler_rename.go` — `textDocument/rename` → returns WorkspaceEdit or error
- `handler_symbols.go` — `textDocument/documentSymbol` → returns DocumentSymbol tree (hierarchical)
- `handler_semantic_tokens.go` — `textDocument/semanticTokens/full` → tokenizes via bridge, encodes to compact integer array
- `handler_folding.go` — `textDocument/foldingRange` → returns FoldingRange array
- `handler_signature_help.go` — `textDocument/signatureHelp` → returns SignatureHelp with active signature/parameter
- `handler_formatting.go` — `textDocument/formatting` → returns TextEdit array

Tests (`ls00_test.go`):

- 9 UTF-16 offset conversion tests (ASCII, BMP, emoji surrogate pairs, CJK, multiline)
- DocumentManager open/change/close/incremental/emoji tests
- ParseCache hit/miss/evict/diagnostics tests
- Capabilities advertisement tests (minimal bridge vs. full bridge)
- Semantic token encoding tests (empty, single, multi-line, unsorted, unknown type, modifier bitmask)
- Integration tests via io.Pipe (full JSON-RPC pipeline) for all 11 handlers
- 79% statement coverage

### Notes

- `textDocumentSync: 2` (incremental) is always advertised
- UTF-16 handling tested with emoji (🎸, U+1F3B8), which requires 2 UTF-16 code units but 4 UTF-8 bytes
- `go vet` clean
- Depends on `github.com/coding-adventures/json-rpc` via local replace directive
