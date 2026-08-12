# smart-home-zoneminder-pairing-service

Explicit, recoverable ZoneMinder credential provisioning for one exact installed
NVR bridge.

The actor request carries only the D23 principal, pending pairing session,
expected durable runtime revision, and completion time. A host-owned input reads
the username and password once from configured exact-length owner-only files
after authorization succeeds. Neither file paths nor credential bytes enter
actor messages, runtime state, journals, reports, or logs.

Before writing a secret, the service requires exactly one ZoneMinder
`https_endpoint` identifier equal to the session bridge address and performs an
authenticated API 2.0 inspection through that strict HTTPS endpoint. A
non-empty returned monitor set must exactly match every already installed
positive `monitor_id`. The service encodes the same versioned username/password
envelope consumed by `smart-home-zoneminder-snapshot-host`; access and refresh
tokens remain process-local and are never persisted. The opaque reference is
committed through `smart-home-pairing-transaction` against the shared
`SmartHomeControllerRuntime` authority. Startup resolves all pending journals
before accepting work, replacement cleanup remains bound to the captured Vault
revision, and successful commits are immediately visible to every controller
consumer without an actor-private runtime copy.

Snapshot delivery remains read-only and never provisions credentials.

## Development

```bash
bash BUILD
```
