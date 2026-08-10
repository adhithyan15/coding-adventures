# Changelog

## 0.4.1

- Move username and password fields into zeroizing storage before validation so
  rejected credential input is also cleared on drop.

## 0.4.0

- Probe the bounded VAPIX image parameter inventory during authenticated
  inspection.
- Advertise `camera.snapshot` only for an enabled camera-1 channel with VAPIX
  HTTP version 3 and native JPEG support.

## 0.3.0

- Probe Axis endpoints without credentials and select advertised Basic or
  Digest authentication instead of sending Basic preemptively.
- Add SHA-256/MD5 Digest challenge handling with CSPRNG client nonces,
  nonce-count reuse, and one bounded retry for refreshed or stale challenges.
- Keep all credentials and generated authorization values transport-private and
  zeroizing while preserving certificate-verifying production HTTPS.

## 0.2.0

- Add capability-probed Axis position and preset inspection.
- Add authorized preset recall and five-second-bounded directional movement.
- Honor native PTZ control queues with transport-only cookies and explicit
  queue release and movement stop.

## 0.1.0

- Add Axis video/NVR mDNS discovery records.
- Add HTTPS Basic-auth VAPIX device and API inspection with Vault-backed plans.
- Add authorized D23 runtime installation and real loopback HTTP transport proof.
