# Changelog

## 0.01 — 2026-04-12

### Added

- Initial implementation of the Perl ls00 LSP framework, ported from Go
- `CodingAdventures::Ls00::Types` -- LSP data types as constructor functions with constants for severity, completion kind, and symbol kind
- `CodingAdventures::Ls00::LanguageBridge` -- Bridge interface documentation (duck typing via `can()`)
- `CodingAdventures::Ls00::DocumentManager` -- Open document tracking with incremental change support and UTF-16 to byte offset conversion
- `CodingAdventures::Ls00::ParseCache` -- Version-keyed parse result cache with URI-based eviction
- `CodingAdventures::Ls00::Capabilities` -- Dynamic capability building from bridge introspection, semantic token legend, and compact token encoding
- `CodingAdventures::Ls00::LspErrors` -- LSP-specific error code constants
- `CodingAdventures::Ls00::Handlers` -- All LSP request and notification handlers (lifecycle, text document sync, hover, definition, references, completion, rename, document symbols, semantic tokens, folding ranges, signature help, formatting)
- `CodingAdventures::Ls00::Server` -- Main coordinator wiring bridge, document manager, parse cache, and JSON-RPC server
- Comprehensive test suite with 6 test files covering UTF-16 conversion, document management, parse caching, capability detection, semantic token encoding, and full server integration
