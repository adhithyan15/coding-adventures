# smart-home-axis-pairing-service

Explicit, recoverable Axis credential provisioning for one exact installed
camera bridge.

The actor request carries only the D23 principal, pending pairing session,
expected durable runtime revision, and completion time. A host-owned input reads
the username and password once from configured exact-length owner-only files
after authorization succeeds. Neither file paths nor credential bytes enter
actor messages, runtime state, journals, reports, or logs.

Before writing a secret, the service requires exactly one Axis
`https_endpoint` identifier equal to the session bridge address and performs an
authenticated VAPIX inspection through that strict HTTPS endpoint. The observed
serial must match the installed camera when one exists, and camera 1 must be
enabled with VAPIX HTTP version 3 and JPEG support. The service encodes the same
versioned username/password envelope consumed by
`smart-home-axis-snapshot-host`. The opaque reference is committed through
`smart-home-pairing-transaction`. Startup resolves all pending journals before
accepting work, and replacement cleanup remains bound to the captured Vault
revision.

Snapshot delivery remains read-only and never provisions credentials.

## Development

```bash
bash BUILD
```
