# Changelog

## 0.3.0

- Retain an optional reviewed canonical host and pinned socket with registered
  endpoints and expose that binding only to the trusted executor during lease
  redemption, preventing origin-reviewed integrations from losing DNS-rebinding
  defenses at the camera-media boundary.
- Reject IP-literal endpoint registrations whose reviewed pinned address does
  not equal the URI literal.

## 0.2.0

- Replaced raw endpoint redemption with a host-owned `CameraMediaService` that
  installs identity, clock, nonce, and executor authority once; operation calls
  cannot substitute caller-asserted security dependencies.
- Return bounded zeroizing snapshot bytes and retain executor-owned stream
  resources behind broker-minted session IDs with explicit close, grant-aware
  expiry, surfaced teardown failures, and retryable retained cleanup.
- Removed principal and timestamp fields from untrusted access requests; issue,
  delivery, expiry sweeps, and audit now use host-supplied identity and time.
- Recheck active Human Approval grants at delivery and clamp lease expiry to the
  authorizing grant's expiry.
- Added endpoint generations, checked expiry arithmetic, collision-failing
  injected nonce IDs, bounded endpoint/lease state, and closed delivery errors.
- Removed bearer IDs from audit records; redacted and zeroized public IDs; reject
  userinfo, fragments, default plaintext, replay, stale generations, oversized
  snapshots, wrong-kind executor results, and invalid deliveries.
- Added global, per-principal, stream, endpoint, lease, and audit bounds; expired
  leases are pruned before quota evaluation.
- Removed direct OS randomness and declared the resulting in-memory policy core's
  explicit empty capability profile, with a committed schema-validation gate.

## 0.1.0

- Added process-local camera endpoint registration with redacted debug output.
- Added capability-grant-backed snapshot and stream lease issuance.
- Added short-lived, principal-bound, single-use redemption and bounded audit.
