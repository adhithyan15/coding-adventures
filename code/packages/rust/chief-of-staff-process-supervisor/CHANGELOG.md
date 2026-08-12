# Changelog

## Unreleased

- Carry a model-returned call through a separate authenticated D18D execution
  exchange and prove the sixth data-plane operation over the real child pipe.
- Carry tool-aware completion turns through the child stream helper and prove the
  fifth authenticated data-plane operation over a real signed-package child pipe.
- Surface an authenticated `Terminate` received during a child data-plane
  exchange as a distinct graceful-termination condition.
- Accept an optional authenticated data-plane dispatcher, retain the exact host
  registration for each owned child, and automatically send validated responses
  without exposing request payloads to the orchestration core.
- Exercise automatic receive, publish, acknowledge, and completion dispatch
  through a real signed-package child process and encrypted cross-platform pipes.
- Add a production durable launch-binding provider backed by the pipeline
  binding store; every launch revalidates registration, channel claims,
  lifecycle, and directional membership before process creation.
- Require an injected manifest-blind launch-binding provider, authenticate its
  channel UUID and Level 1 model bindings before readiness, and fail closed when
  bindings are unavailable or incompatible with the verified package runtime.
- Deliver the exact relevant public package key, trust class, and tier over the
  fresh child session, removing the test child's hard-coded verification key.
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
