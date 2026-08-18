# Changelog

## Unreleased

- Added `AuditActionV1::PassphraseRotate` (registry code 6, label
  `passphrase_rotate`) so `VLT-PM43-cli-passphrase-rotation.md`'s ceremony can
  be recorded before it takes effect. The action is vault-scoped: it selects no
  item and no revision, and — deliberately — carries no salt, no KDF parameter,
  no generation number, and no bootstrap identifier. An audit chain records
  that a rotation happened, not the shape of the credential it produced.

- Added the closed V1 password-manager operation audit event, including actor,
  trace, action, outcome, item/revision, prior-event, observed-head, and time
  fields.
- Added deterministic canonical encoding, strict decoding, device signing, and
  signature verification without storage, clock, entropy, or host coupling.
- Added a distinct authored conflict-merge action so an event never invents a
  single selected revision for a merge that intentionally retains all parents.
- Added stable lowercase action and outcome labels for explicit redacted audit
  surfaces without changing the canonical signed representation.
- Added a distinct `PortableRestoreVerify` action so independent semantic
  restore comparison never masquerades as another import mutation or generic
  vault verification.
