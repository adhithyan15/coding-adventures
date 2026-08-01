# smart-home-camera-media

`smart-home-camera-media` keeps privacy-sensitive camera endpoints out of the
durable D23 model. Integrations register snapshot and stream URIs in a
process-local broker. Authorized principals receive opaque, short-lived,
single-use leases whose audit records contain no URI or credential material.

The broker uses D23 capability grants directly:

- `camera.snapshot` requires a Human Approval grant.
- `camera.stream` requires a Human Approval grant.
- entity-scoped, capability-scoped, and all-smart-home grants are supported.
- expired, pending, revoked, or insufficient-tier grants are rejected.

Snapshot and stream URIs are wrapped in the repository's zeroizing secret type
and are exposed only when a matching lease is redeemed by its principal.
