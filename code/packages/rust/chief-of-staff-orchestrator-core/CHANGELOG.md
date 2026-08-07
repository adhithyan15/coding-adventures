# Changelog

## Unreleased

- Add safe package reload for stopped hosts with absent or exited supervisor authority.
- Atomically write replacement identity and post-reload desired state before reconciliation.

## 0.1.0

- Add bounded register, desired-state, health, reconcile, and safe-deregister APIs.
- Compose the concrete verified process supervisor through a production constructor.
- Add authorized durable channel-definition create, load, and destroy operations.
- Preserve separate durable intent and authoritative process health evidence.
- Reject monotonic clock regression without advancing failed reconciliation state.
- Own the shared storage handle and production trust resources so the complete core is `Send + 'static` for daemon composition.
