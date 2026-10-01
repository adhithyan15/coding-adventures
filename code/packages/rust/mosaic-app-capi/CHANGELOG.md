# Changelog

## 2026-10-01 (no diagnostic reads as an invalid environment)

- Test only: native hosts hold back an environment report only when the
  failure's diagnostic begins with `mosaic-app-runtime`'s
  `INVALID_ENVIRONMENT_DIAGNOSTIC` (UI48 §7.12). A new test checks that none
  of this layer's own diagnostics -- panics (string, `String` and opaque
  payloads), decode and encode failures, null input/output/handle pointers,
  and a poisoned handle -- begins that way. No behaviour change.

## Unreleased

- Add UI47 protocol-2 effect completion with pending-work checkpoint protection,
  explicit terminal outcomes and shared native/WASM conformance coverage.

## 0.1.0

- Add fixed C layouts for input bytes, owned output buffers, opaque app handles,
  and status codes.
- Add a reusable export macro for create, dispatch, snapshot, restore, destroy,
  and buffer-free symbols.
- Contain Rust panics, poison runtimes after a panic, and return bounded UTF-8
  diagnostics through the normal output buffer.
- Test lifecycle, retry, protocol failures, application failures, panic poisoning,
  invalid pointers, snapshots, restores, and buffer ownership.
