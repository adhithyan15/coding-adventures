# Changelog

## 0.2.0

- Require the exact D23 principal and expected durable runtime revision in
  actor pairing requests.
- Recover every pending pairing transaction before actor startup and replace
  live state only from the coordinator's committed durable snapshot.
- Move Hue credential installation and replacement cleanup onto the
  recoverable sealed-Vault/runtime-store transaction coordinator.
- Cover authorization-before-I/O, restart recovery, stale-revision rollback,
  exact replacement cleanup, and cleanup revision drift.

## 0.1.0

- Add actor-owned Hue LAN registration execution.
- Seal application and client keys in the durable Vault store.
- Complete D23 pairing sessions with opaque `VaultRef` handles only.
- Add real loopback HTTP and local-folder Vault restart coverage.
