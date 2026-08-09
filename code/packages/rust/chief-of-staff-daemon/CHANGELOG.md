# Changelog

## Unreleased

- Compose a fail-closed host launch-binding provider until durable pipeline
  wiring is connected; hosts cannot start with ambient or invented bindings.

## 0.1.0 - 2026-08-03

- Add the concrete cross-platform Chief daemon executable.
- Compose strict configuration, owner-only local authentication, trusted package
  keys, durable registry storage, verified host supervision, authenticated
  WebSocket serving, periodic reconciliation, and cooperative process shutdown.
- Bound and race-check configuration-file loading without following a final
  symlink.
