# Changelog

## Unreleased

- Provision non-empty typed data-plane declarations into the production daemon's
  exact channel-key and Ollama authorities, with UUID-v7/process-monotonic publish
  metadata and no startup network probe. Empty declarations remain unavailable.
- Compose durable per-request host data-plane authorization.
- Compose the storage-backed durable pipeline launch-binding provider. Host
  starts now require an exact registered package plus current immutable channel
  claims, active membership, and bounded persisted model settings.

## 0.1.0 - 2026-08-03

- Add the concrete cross-platform Chief daemon executable.
- Compose strict configuration, owner-only local authentication, trusted package
  keys, durable registry storage, verified host supervision, authenticated
  WebSocket serving, periodic reconciliation, and cooperative process shutdown.
- Bound and race-check configuration-file loading without following a final
  symlink.
