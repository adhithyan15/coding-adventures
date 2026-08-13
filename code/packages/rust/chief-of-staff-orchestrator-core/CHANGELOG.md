# Changelog

## Unreleased

- Require validated request context for channel create and destroy authorization.
- Add authoritative channel/member tier resolution and Trust Checker composition.
- Bind approvals to a SHA-256 fingerprint of the complete immutable topology mutation.
- Fail tier resolution and approval before any durable channel storage mutation.
- Require the production process composition to inject a host data-plane
  dispatcher while keeping the generic orchestration core payload-blind.
- Require the production process composition to inject a manifest-blind host
  launch-binding authority alongside package trust and process identity.
- Add safe package reload for stopped hosts with absent or exited supervisor authority.
- Atomically write replacement identity and post-reload desired state before reconciliation.

## 0.1.0

- Add bounded register, desired-state, health, reconcile, and safe-deregister APIs.
- Compose the concrete verified process supervisor through a production constructor.
- Add authorized durable channel-definition create, load, and destroy operations.
- Preserve separate durable intent and authoritative process health evidence.
- Reject monotonic clock regression without advancing failed reconciliation state.
- Own the shared storage handle and production trust resources so the complete core is `Send + 'static` for daemon composition.
