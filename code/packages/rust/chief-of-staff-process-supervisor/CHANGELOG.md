# Changelog

## 0.1.0

- Add exact package re-verification before every host spawn.
- Add bounded pipe framing and fresh secure-channel bootstrap.
- Add authenticated readiness, heartbeat, and graceful termination handling.
- Add owned child reaping with hard-kill fallback and drop cleanup.
- Implement the D18 service reconciler's authoritative supervisor contract.
- Own shared keyring and zeroizing identity handles and require movable session sources for daemon composition.
