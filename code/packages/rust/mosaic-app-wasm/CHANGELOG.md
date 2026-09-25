# Changelog

## Unreleased

### Added — the browser answers the standard `files.*` kinds (UI87 §7)

- `mosaic-file-effects.mjs` answers `files.open` and `files.save`, the kinds
  every Mosaic backend answers, with UI59's `accept` MIME list (the same
  MIME-to-extension table as the Compose library) and `mimeType` in the open
  result. `file.open` / `file.save` stay as aliases with their original results.
- Suggested names follow the shared rule (`isPlainFileName`): no separators,
  `:`, control or format characters, or trailing dot or space; with `accept`
  the name must end in an accepted extension. Checked before any dialog opens.
- Without the File System Access API, `files.save` downloads the bytes and
  reports `ok { name, download: true }`, so an app can say "downloaded" rather
  than claim a durable save. Narrowed after security review, since no dialog
  shows the name: only for a known accepted type with a matching extension
  (never an untyped `.exe`), at most one download per 5 s (one gesture cannot
  start a burst), Blob typed as the accepted MIME type, URL always revoked.
  Names may not contain runs of spaces (the `Invoice.pdf<spaces>.exe`
  disguise). `file.save` keeps its explicit degradation.
- Five new tests (15 total), run in the VisiCalc workflow.

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
