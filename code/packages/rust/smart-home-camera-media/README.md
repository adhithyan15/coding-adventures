# smart-home-camera-media

`smart-home-camera-media` keeps privacy-sensitive camera endpoints out of the
durable D23 model. Integrations register snapshot and stream URIs in a
process-local `CameraMediaService`. Authorized principals receive opaque, short-lived,
single-use leases whose audit records contain no URI, raw bearer ID, or
credential material.

The broker uses D23 capability grants directly:

- `camera.snapshot` requires a Human Approval grant.
- `camera.stream` requires a Human Approval grant.
- entity-scoped, capability-scoped, and all-smart-home grants are supported.
- expired, pending, revoked, or insufficient-tier grants are rejected.
- current authorization is checked again immediately before delivery.
- lease expiry is clamped to the active grant's expiry.

Snapshot and stream URIs are wrapped in the repository's zeroizing secret type
and are lent only to a trusted `CameraMediaExecutor` during one atomic delivery.
Snapshot bytes are bounded before release. For streams, the executor returns an
owned resource, the service mints the public session ID, and the service retains
the resource until explicit close or trusted-time expiry. The executor cannot
smuggle an endpoint URI into the public session ID. `reconcile` also closes a
stream when its current grant is no longer active. Failed native teardown keeps
broker ownership, is reported, and remains retryable; post-open failures without
a public session are retained in a bounded cleanup queue.

The security context is installed once in `CameraMediaService` by the native host:

- `CameraMediaClock` provides trusted issue, delivery, expiry, and audit time.
- `CameraMediaNonceSource` provides collision-resistant 128-bit lease IDs.
- `CameraMediaPrincipalSource` derives the authenticated `AgentId` from the
  active host/session, not the untrusted access request or each operation call.
- `CameraMediaExecutor` owns the actual network/media operation.

The policy core itself performs no filesystem, network, process, environment, FFI,
clock, standard-I/O, or entropy operation. Endpoint and lease tables are bounded;
endpoint generations invalidate pre-rotation leases; URL userinfo and fragments
fail closed. Plaintext endpoints are disabled by default and require an explicit
loopback-fixture policy opt-in. Query tokens are permitted only on otherwise
accepted secure endpoints and remain inside the trusted executor boundary.
