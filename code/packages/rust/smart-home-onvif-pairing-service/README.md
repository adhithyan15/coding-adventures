# smart-home-onvif-pairing-service

Explicit, recoverable ONVIF credential provisioning for one exact installed
camera bridge.

The actor request carries only the D23 principal, pending pairing session,
expected durable runtime revision, and completion time. A host-owned input reads
the username and password once from configured exact-length owner-only files
after authorization succeeds. Neither file paths nor credential bytes enter
actor messages, runtime state, journals, reports, or logs.

Before writing a secret, the service requires exactly one ONVIF endpoint
reference on the session bridge and performs authenticated inspection through
the bridge's reviewed, address-pinned HTTPS endpoint. It then encodes the same
versioned envelope consumed by `smart-home-onvif-snapshot-host` and commits its
opaque reference through `smart-home-pairing-transaction` against the shared
`SmartHomeControllerRuntime` authority. Startup resolves all pending journals
before accepting work, replacement cleanup remains bound to the captured Vault
revision, and successful commits are immediately visible to every controller
consumer without an actor-private runtime copy.

Snapshot delivery remains read-only and never provisions credentials.

## Development

```bash
bash BUILD
```
