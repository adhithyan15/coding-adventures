# Changelog

## Unreleased

## 1.0.0 — 2026-10-05

- Join the aligned stable Forme `1.0.0` package line targeting kernel API v2; see the [`forme-types` migration guide](../forme-types/MIGRATION-v2.md) for the breaking `RenderedPage` provenance and stage/plugin version changes.

- Require every runner to start a live storage-watch handle, receive a pushed
  change, and cancel the handle after early iterator return.

## 0.1.0 — 2026-09-30

- Add the language-neutral FM02 subprocess driver and canonical runner vectors.
- Cover lifecycle, wire values, all mediated context calls, streaming and
  hybrid shapes, cancellation, typed errors, malformed peers, and bounds.
- Prove the reusable corpus against the TypeScript reference runner.
- Declare runner execution/signaling authority and bound aggregate output,
  notification, message, and queued-work retention.
- Validate exact capability parameters, typed capability-denial translation,
  identity mismatches, malformed response envelopes, and TERM-to-KILL cleanup.
