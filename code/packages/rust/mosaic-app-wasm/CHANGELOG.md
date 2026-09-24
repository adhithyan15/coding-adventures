# Changelog

## Unreleased

### Added — the loader sends the browser's UTC offset

`mosaic-host.mjs` adds `utcOffsetMinutes` (`-getTimezoneOffset()`) to the
default start context. It is left out when implausible, so an app never fails
to start over it, and a caller's explicit context overrides it.

- Add reusable browser file.open/file.save handlers with explicit outcomes,
  gesture-bound pickers, bounded opaque bytes and completion retry without I/O replay.

- Add UI47 protocol-2 effect completion with pending-work checkpoint protection,
  explicit terminal outcomes and shared native/WASM conformance coverage.

## 0.1.0

- Add scalar WASM lifecycle exports and a reusable JavaScript host over MosaicRuntime.
- Validate compiled conformance and VisiCalc adapters through the shared protocol.
