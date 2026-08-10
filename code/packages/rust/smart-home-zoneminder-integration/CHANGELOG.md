# Changelog

## 0.1.1

- Move supplied usernames and passwords into zeroizing storage before all
  validation error paths.

## 0.1.0

- Add manual local-HTTPS endpoint intake and Vault-backed ZoneMinder credentials.
- Add transport-private API 2.0 access-token handling with no persisted refresh
  token or token-bearing request plan.
- Add authorized, bounded version and monitor-health inspection through the
  documented ZoneMinder API.
- Add normalized bridge, camera device, entity, capability, and confirmed state
  installation with an exact loopback protocol proof.
- Bound username and password inputs, expose a redacted one-shot access-token
  result from the existing native login transport, and declare the
  Human Approval `camera.snapshot` capability for the dedicated snapshot host.
