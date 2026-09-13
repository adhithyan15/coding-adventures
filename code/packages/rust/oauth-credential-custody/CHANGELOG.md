# Changelog

## Unreleased

- Added an audited access-token revocation closure that releases the exact
  credential revision only when no refresh token exists, preventing callers
  from deleting a still-refreshable record after revoking only its access token.
- Added audit-gated metadata and combined refresh-token/revision/metadata
  closures for broker expiry decisions and atomic refresh orchestration.
- Added opaque provider/account keys, zeroizing credential records, a small
  injected compare-and-swap store, audit-before-disclosure access and refresh
  closures, atomic refresh-token rotation, conditional deletion, closed
  diagnostics, and a deterministic in-memory reference store.
