# Changelog

## Unreleased

- Add an exactly-once pre-ready `LaunchBindings` record with canonical bounded
  channel name-to-UUID mappings and optional bounded Level 1 model settings.
- Add a bounded authenticated `PackageTrust` record that must be delivered
  exactly once before a child can announce independently verified readiness.
- Add authenticated channel receive, publish, and acknowledge requests plus
  provider-neutral text completion requests and responses.
- Enforce bounded binary fields, canonical UUID-v7 identities, one in-flight
  request, monotonic request IDs, exact correlation, response-kind matching,
  and redacted stable failure codes.

## 0.1.0

- Add strict bounded `D18C` readiness, heartbeat, and termination records.
- Bind readiness to the immutable registered package hash.
- Attach trusted orchestrator receipt time to authenticated child events.
- Enforce channel roles and lifecycle ordering with fail-closed peer handling.
