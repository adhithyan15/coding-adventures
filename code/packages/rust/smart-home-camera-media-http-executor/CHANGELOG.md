# Changelog

## 0.1.0

- Added a pinned, certificate-verifying HTTPS snapshot executor for the
  `smart-home-camera-media` lease boundary.
- Added bounded JPEG, PNG, and WebP response handling with strict redirect,
  content-encoding, HTTP-framing, and payload-signature rejection.
- Added zeroizing process-local Basic and RFC 7616 Digest credentials, SHA-256
  preference, CSPRNG client nonces, and one refreshed-challenge retry.
- Implemented the shared credential registry and reject replacement until the
  existing entity credentials are explicitly removed.
