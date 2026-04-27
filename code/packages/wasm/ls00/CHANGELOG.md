# Changelog

All notable changes to the ls00-wasm package will be documented in this file.

## [0.1.0] - 2026-04-12

### Added

- Initial release of the WASM LSP server wrapper.
- `WasmLanguageBridge` struct for providing tokenize/parse logic from JavaScript callbacks.
- `WasmLspServer` struct with callback-based message-passing API (no stdio).
- `handleMessage(json)` method for processing incoming JSON-RPC messages.
- `isInitialized()` and `isShutdown()` state query methods.
- Full LSP request dispatch: initialize, shutdown, hover, definition, references, completion, rename, documentSymbol, semanticTokens/full, foldingRange, signatureHelp, formatting.
- Full LSP notification dispatch: initialized, didOpen, didChange, didClose, didSave.
- Server-initiated notifications (publishDiagnostics) sent via JS callback.
- Document manager and parse cache reused from the core ls00 crate.
- Native Rust unit tests guarded with `#[cfg(not(target_arch = "wasm32"))]`.
- BUILD file for build-tool integration.
