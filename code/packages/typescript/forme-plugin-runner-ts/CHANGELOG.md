# Changelog

## Unreleased

## 1.0.0 — 2026-10-05

- Join the aligned stable Forme `1.0.0` package line targeting kernel API v2; see the [`forme-types` migration guide](../forme-types/MIGRATION-v2.md) for the breaking `RenderedPage` provenance and stage/plugin version changes.

- Consume live host-mediated storage watches through explicit bounded stream
  handles, cancelling abandoned iterators and all outstanding capability
  streams when a run ends.

## 0.1.0 — 2026-09-30

- Add the bounded plugin-side Content-Length JSON-RPC peer and strict binary
  wire envelopes.
- Implement handshake, announcement, init, single and streaming runs,
  cooperative cancellation, typed error translation, and disposal.
- Expose host-mediated storage, network, environment, wall-clock, filesystem,
  and shell APIs through an asynchronous wire-backed `StageContext`.
- Bound frames, headers, metadata, mediated bytes, and input stream queues.
- Replace the happy-path hand-written host fixture with a real SDK-driven
  plugin while retaining adversarial low-level fixtures for fault injection.
