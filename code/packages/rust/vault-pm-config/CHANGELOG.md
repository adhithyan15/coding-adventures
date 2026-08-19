# Changelog

## Unreleased

- Added `StorageKind::Removable` (`kind = "removable"`), VLT-PM00 §12's
  removable/synced-folder backend row. It is a variant of `Filesystem` with
  the identical on-disk immutable object format — `vault-pm-storage-removable`
  is the layer that treats it differently, by scanning for third-party
  sync-tool conflict-copy symptoms. Added `StorageKind::is_local_directory`
  so callers can select the shared filesystem code path for both kinds
  without a second match arm per call site.

## 0.1.0

- Added the closed, bounded V1 vault and storage configuration model.
- Added strict TOML decoding and deterministic canonical TOML rendering.
- Added opaque locator, location, and credential-reference diagnostics.
