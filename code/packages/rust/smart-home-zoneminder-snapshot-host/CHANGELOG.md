# Changelog

## 0.1.0

- Add exact D23 Human Approval preflight before credential resolution, login,
  endpoint registration, or media I/O.
- Resolve a bounded versioned credential envelope from a dedicated sealed-Vault
  namespace and acquire one transport-private ZoneMinder API 2.0 token per
  approved operation.
- Validate the explicit `nph-zms` endpoint against the installed bridge origin
  and reviewed pinned connection target.
- Deliver only `mode=single` JPEG snapshots for exact installed monitor IDs,
  with bounded TTL and payload handling inherited from camera-media.
- Keep token-bearing endpoint state zeroizing and process-local, and remove it
  after every delivery outcome.
- Cover denial-before-secrets, invalid targets and payloads, endpoint cleanup,
  repeated sealed-record use, exact token placement, and strict native HTTPS
  login-to-snapshot composition.
- Leave streams, recordings, export, playback, token reuse, refresh, and
  automated credential provisioning prerequisite-gated.
