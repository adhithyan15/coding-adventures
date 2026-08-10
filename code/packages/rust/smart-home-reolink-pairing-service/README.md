# smart-home-reolink-pairing-service

Explicit, recoverable credential provisioning for one exact direct Reolink RLC
camera or one exact installed Reolink NVR bridge.

The actor request carries only the D23 principal, pending pairing session,
expected durable runtime revision, and completion time. A host-owned input reads
the username and password once from configured exact-length owner-only files
after authorization succeeds. Neither file paths nor credential bytes enter
actor messages, runtime state, journals, reports, or logs.

Before writing a secret, the service requires the pending session's exact
credential-free bridge address to match a host-owned canonical hostname and
reviewed socket address. Production traffic uses strict certificate-verifying
HTTPS, keeps the canonical hostname for SNI, and bypasses fresh name
resolution. An authenticated CGI session must prove a non-empty stable serial,
the exact installed physical-channel set when one exists, and at least one
awake online snapshot-capable channel on a direct `RLC-*` camera. For an NVR,
authenticated `exactType`, NVR model and serial, every installed physical
channel's `typeInfo`, and every channel's supported executable
`abilityChn.snap` value must exactly match durable state. Empty channel slots
are excluded. Query tokens stay process-local and the verifier logs out on
success or failure.

The service encodes the same versioned username/password envelope consumed by
`smart-home-reolink-snapshot-host`. The opaque reference is committed through
`smart-home-pairing-transaction`. Startup resolves all pending journals before
accepting work, and replacement cleanup remains bound to the captured Vault
revision.

Snapshot delivery remains read-only and never provisions credentials.

## Development

```bash
bash BUILD
```
