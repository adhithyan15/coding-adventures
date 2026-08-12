# Changelog

## Unreleased

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
