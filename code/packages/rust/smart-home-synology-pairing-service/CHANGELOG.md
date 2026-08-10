# Changelog

## 0.1.0

- Add D23-authorized Synology credential provisioning from one-shot owner-only
  secret files.
- Verify the exact installed server through the pending session, canonical
  hostname, reviewed socket address, strict certificate-verifying HTTPS, API
  discovery, package permissions, and privilege-filtered cameras.
- Require exact installed positive camera identifiers, canonical camera
  entities, and snapshot capabilities before replacement.
- Persist only the versioned username/password envelope; SID, SynoToken, OTP,
  and remembered-device material remain process-local.
- Explicitly log out the isolated Synology session after successful inspection
  or authenticated failure.
- Install only transaction-owned opaque references through recoverable runtime
  CAS and revision-bound replacement cleanup.
- Resolve all pending journals before actor startup and keep secret material out
  of messages, runtime state, journals, reports, and logs.
