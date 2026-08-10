# smart-home-synology-pairing-service

Explicit, recoverable credential provisioning for one exact installed Synology
Surveillance Station server.

The actor request carries only the D23 principal, pending pairing session,
expected durable runtime revision, and completion time. A host-owned input reads
the username and password once from configured exact-length owner-only files
after authorization succeeds. Neither file paths nor credential bytes enter
actor messages, runtime state, journals, reports, or logs.

Before writing a secret, the service binds the pending session bridge to a
host-owned canonical hostname and reviewed socket address. Any installed
Synology `https_endpoint` must equal that credential-free bridge address. The
native verifier retains the hostname for strict certificate verification and
SNI while bypassing fresh DNS, discovers the advertised Synology APIs, opens an
isolated SID-format session, and inspects package permissions plus the
privilege-filtered camera list. A non-empty returned camera set must exactly
match every already installed positive `camera_id`, canonical camera entity,
and `camera.snapshot` capability.

The verifier explicitly logs out after success or authenticated failure. SID,
SynoToken, OTP, and remembered-device material remain process-local and are
never persisted. The service encodes the same versioned username/password
envelope consumed by `smart-home-synology-snapshot-host`; its opaque reference
is committed through `smart-home-pairing-transaction`. Startup resolves all
pending journals before accepting work, and replacement cleanup remains bound
to the captured Vault revision.

Snapshot delivery remains read-only and never provisions credentials.

## Development

```bash
bash BUILD
```
