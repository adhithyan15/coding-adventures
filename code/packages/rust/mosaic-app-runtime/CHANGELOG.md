# Changelog

## Unreleased

- Add UI47 protocol-2 effect completion with pending-work checkpoint protection,
  explicit terminal outcomes and shared native/WASM conformance coverage.

## 0.1.0

- Add the `MosaicApp` application contract and stable JSON wire types.
- Add `MosaicRuntime`, which owns lifecycle, protocol-version, event-sequence,
  and render-revision enforcement.
- Keep failed and rejected events from consuming sequence numbers or revisions.
- Reject invalid text scaling before invoking the app, and require application
  errors to be transactional so host retries are safe.
