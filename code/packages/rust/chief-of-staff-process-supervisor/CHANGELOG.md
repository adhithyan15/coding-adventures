# Changelog

## Unreleased

- Pass the authenticated package runtime to the single configured host program
  as the reserved final `--package-runtime deno|skill` argument pair.
- Carry bounded correlated channel and completion exchanges over the established
  secure child pipe, with child request helpers and supervisor-side pending-request
  and response hooks.
- Exercise receive, publish, acknowledge, and completion failure through a real
  signed-package child process on the platform-neutral integration path.

## 0.1.0

- Add exact package re-verification before every host spawn.
- Add bounded pipe framing and fresh secure-channel bootstrap.
- Add authenticated readiness, heartbeat, and graceful termination handling.
- Add owned child reaping with hard-kill fallback and drop cleanup.
- Implement the D18 service reconciler's authoritative supervisor contract.
- Own shared keyring and zeroizing identity handles and require movable session sources for daemon composition.
