# Changelog

## 2026-10-01 (hosts can tell an invalid environment from other failures)

- **`INVALID_ENVIRONMENT_DIAGNOSTIC`** (`"invalid Mosaic environmentChanged
  payload"`): how every `RuntimeError::InvalidEnvironment` message begins,
  now exported and used by its `Display`. A native host sees a failure only
  as an ABI status and this text, and the status cannot separate a refused
  environment from a sequence mismatch (both are `ProtocolError`). The hosts
  hold back an identical report only when the runtime refused it as invalid,
  and send it again after any other failure (an app error may be transient);
  `mosaic-app-bindings` writes this constant into each host. A test pins that
  no other `RuntimeError` message begins the same way, walking every variant
  through an exhaustive match, so a new variant does not compile until it is
  covered. No wire or behaviour change: the message text is unchanged.
- **`MosaicApp::environment_changed` documents that answering `None` must
  not change the state `snapshot` saves.** Native hosts persist an ignored
  report only to retry a failed save (UI48 §7.12), so state recorded silently
  would wait for the next event, and be lost if the app were killed first.

## Unreleased

- **The host environment (UI48 ENV1).** The foundation for phone, tablet and
  desktop layouts in every Mosaic app:
  - `StartContext` gains the size class, pointer, hover, orientation and
    reduced-motion axes (`SizeClass`, `Pointer`, `Hover`, `Orientation`,
    `ReducedMotion`), flat on the wire beside `colorScheme` and each defaulting
    (`regular`/`fine`/`hover`/`landscape`/`no-preference`), so earlier hosts
    decode unchanged. In Rust they are one flattened field,
    `environment: EnvironmentAxes`; `full_environment()` adds the color scheme.
  - `environmentChanged` (`ENVIRONMENT_CHANGED`) reports a change as one
    coalesced `Environment`. The runtime intercepts it, validates the payload
    (`RuntimeError::InvalidEnvironment`, consuming nothing), and calls the new
    `MosaicApp::environment_changed`, whose default ignores it — so existing
    apps, which reject event names they do not know, keep working.
  - An app that ignores a change still consumes the sequence, but the update
    keeps the current revision with `props: null`: hosts render nothing for an
    update that is not newer.
  - `MosaicRuntime::environment()` reports the environment last seen.

- **`StartContext.utc_offset_minutes`** (wire `utcOffsetMinutes`, UI38 "Local
  time"): the host's UTC offset at startup, in minutes east of UTC. It lets an
  app decide which local day "now" falls on. It is optional on the wire both
  ways: an earlier host's context decodes to `None`, and `None` is never
  written. Startup rejects an offset outside −840..=840 with the new
  `RuntimeError::InvalidUtcOffset`. `MIN_UTC_OFFSET_MINUTES` and
  `MAX_UTC_OFFSET_MINUTES` are exported.
- Add UI47 protocol-2 effect completion with pending-work checkpoint protection,
  explicit terminal outcomes and shared native/WASM conformance coverage.

## 0.1.0

- Add the `MosaicApp` application contract and stable JSON wire types.
- Add `MosaicRuntime`, which owns lifecycle, protocol-version, event-sequence,
  and render-revision enforcement.
- Keep failed and rejected events from consuming sequence numbers or revisions.
- Reject invalid text scaling before invoking the app, and require application
  errors to be transactional so host retries are safe.
