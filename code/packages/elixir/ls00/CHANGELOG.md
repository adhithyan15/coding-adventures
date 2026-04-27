# Changelog

## 0.1.0 -- 2026-04-12

### Added

- Initial release: generic LSP server framework for Elixir.
- `Ls00.LanguageBridge` behaviour with 2 required callbacks (`tokenize/1`, `parse/1`) and 10 optional callbacks (hover, definition, references, completion, rename, semantic_tokens, document_symbols, folding_ranges, signature_help, format).
- `Ls00.DocumentManager` with UTF-16 code unit to UTF-8 byte offset conversion, supporting ASCII, BMP codepoints, emoji/surrogate pairs, CJK characters, and multiline documents.
- `Ls00.ParseCache` with (uri, version) keyed caching and URI-based eviction.
- `Ls00.Capabilities` with runtime capability detection via `function_exported?/3`, semantic token legend, and delta-encoded semantic token serialization.
- `Ls00.LspErrors` with all LSP-specific error code constants.
- `Ls00.Handlers` implementing all LSP lifecycle (initialize, initialized, shutdown, exit), text document sync (didOpen, didChange, didClose, didSave), and feature request handlers.
- `Ls00.Server` wiring JSON-RPC server to handler state via Agent for mutable state threading.
- 61 tests covering UTF-16 conversion (10), document manager (8), parse cache (5), capabilities (5), semantic tokens (8), and server/handler integration (25).
- Literate programming style with comprehensive `@moduledoc` and `@doc` annotations.
