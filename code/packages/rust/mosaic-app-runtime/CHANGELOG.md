# Changelog

## Unreleased

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
