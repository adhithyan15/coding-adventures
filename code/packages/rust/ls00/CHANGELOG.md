# Changelog

## v0.1.0 -- Initial Release

### Added

- `LanguageBridge` trait with required `tokenize` and `parse` methods, plus
  10 optional provider methods (`hover`, `definition`, `references`,
  `completion`, `rename`, `semantic_tokens`, `document_symbols`,
  `folding_ranges`, `signature_help`, `format`).
- `LspServer` struct that reads JSON-RPC messages, dispatches to handlers,
  and writes responses. Handles the full LSP lifecycle (initialize, shutdown,
  exit) and all document synchronization notifications.
- `DocumentManager` for tracking open file contents with incremental edit
  support and correct UTF-16 to UTF-8 offset conversion.
- `ParseCache` for caching parse results keyed by (URI, version) to avoid
  redundant parses on cursor movement.
- `build_capabilities()` for dynamic capability advertisement based on which
  `supports_*` methods the bridge overrides.
- Semantic token encoding/decoding with the full LSP delta format and standard
  token type/modifier legends.
- LSP error code constants matching the specification.
- 53 integration tests covering UTF-16 conversion, document management, parse
  caching, capability building, semantic token encoding, and full JSON-RPC
  round-trip handler tests for all supported LSP methods.
