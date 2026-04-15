# Changelog

All notable changes to the `coding-adventures-ls00` package will be documented in this file.

## [0.1.0] - 2026-04-12

### Added

- Initial release of the generic LSP server framework for Lua.
- **Types**: Position, Range, Location, Diagnostic, Token, TextEdit, WorkspaceEdit, HoverResult, CompletionItem, SemanticToken, DocumentSymbol, FoldingRange, ParameterInformation, SignatureInformation, SignatureHelpResult — all as constructor functions returning plain tables.
- **UTF-16 conversion**: `convert_utf16_offset_to_byte_offset()` for converting LSP's UTF-16 character offsets to Lua's 1-based byte offsets. Handles ASCII, BMP codepoints (2-3 byte UTF-8), and non-BMP codepoints (4-byte UTF-8 / surrogate pairs).
- **DocumentManager**: Tracks open file contents with version numbers. Supports full replacement and incremental (range-based) changes via `apply_changes()`.
- **ParseCache**: Caches parse results keyed by (uri, version) to avoid redundant parsing. Automatic eviction when version changes or document closes.
- **Capabilities**: `build_capabilities(bridge)` dynamically builds the LSP capabilities object by checking which functions the bridge table provides (`if bridge.hover ~= nil`).
- **Semantic token encoding**: `encode_semantic_tokens()` converts SemanticToken tables to LSP's compact delta-encoded integer array format. Handles sorting, unknown types, and modifier bitmasks.
- **Semantic token legend**: `semantic_token_legend()` returns the standard 23 token types and 10 token modifiers.
- **LspServer**: Full LSP server that wires together the bridge, DocumentManager, ParseCache, and JSON-RPC server. Handles all lifecycle events (initialize, shutdown, exit) and all feature requests (hover, definition, references, completion, rename, documentSymbol, semanticTokens, foldingRange, signatureHelp, formatting).
- **Bridge design**: Plain Lua table with function fields. Required: `tokenize`, `parse`. Optional: `hover`, `definition`, `references`, `completion`, `rename`, `semantic_tokens`, `document_symbols`, `folding_ranges`, `signature_help`, `format`.
- **Diagnostics publishing**: Automatically pushes diagnostics to the editor on didOpen and didChange.
- **Tests**: Comprehensive test suite covering UTF-16 conversion, DocumentManager, ParseCache, capabilities, semantic token encoding, and full server integration.
