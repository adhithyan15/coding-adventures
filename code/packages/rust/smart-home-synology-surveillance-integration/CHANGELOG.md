# Changelog

## 0.1.1

- Add reviewed-socket pinning for loopback tests and certificate-verifying
  HTTPS while retaining the canonical hostname for SNI.
- Move credentials into zeroizing storage before validation so rejected input
  is cleared on drop.

## 0.1.0

- Add manual local-HTTPS endpoint intake and Vault-backed Synology credentials.
- Discover supported Web API paths and versions before opening a SID-format
  Surveillance Station session.
- Add authorized, bounded package and privilege-filtered camera-health
  inspection with transport-private SID and SynoToken handling.
- Add normalized bridge, camera device, entity, capability, and confirmed state
  installation with an exact loopback protocol proof and explicit logout.
- Project `camera.snapshot` only when package information explicitly allows it.
- Add an operation-scoped snapshot session that revalidates one exact
  privilege-filtered camera, keeps its SID/SynoToken endpoint zeroizing and
  process-local, and supports explicit logout.
