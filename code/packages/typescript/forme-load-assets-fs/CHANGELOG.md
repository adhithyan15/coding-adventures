# Changelog

## Unreleased

### Added

- A deterministic `Stream<ContentNode> -> Stream<Asset>` collector that loads
  each unique resolved filesystem reference once.
- Canonical-root and asset `realpath` containment, including explicit symlink
  escape rejection and regular-file enforcement.
- Binary revision hashing, MIME signature/extension detection, defensive byte
  copies, cancellation, and source/identity/role collision diagnostics.
