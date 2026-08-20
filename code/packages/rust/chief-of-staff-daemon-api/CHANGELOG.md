# Changelog

## Unreleased

- Add independently session-authorized `wire_host_pipeline` and
  `unwire_host_pipeline` operations plus typed client calls.
- Bind topology approval context to the authenticated requester identity and
  protocol request ID rather than accepting caller-supplied identity fields.
- Parse and emit exact bounded pipeline, package, agent, channel, and optional
  model bindings before invoking the Trust-Checker-backed core mutation.
- Add an independently authorized `reload_host` operation and typed client call.
- Preserve stopped/inactive reload conflicts as stable public conflict responses.

## 0.1.0

- Add a bounded, versioned JSON request/response protocol for Chief host lifecycle operations.
- Require connection-local authentication and per-operation authorization.
- Bind the protocol to `chief-of-staff-orchestrator-core` and the repository WebSocket runtime.
- Preserve separate durable and authoritative health evidence with precision-safe JSON encoding.
- Add a typed blocking WebSocket client with strict response-ID and envelope validation.
- Accept the owned lifetime-free orchestrator core at the threaded daemon boundary.
- Add a local serialized reconciliation boundary for the fail-closed daemon scheduler.
- Report a quarantine's `permanent` and `expired` flags, and its `boot_id`
  alongside `until_ns` when it lifts, so a client cannot mistake *that* reading
  for its own clock. The observation's `started_at_ns`, `last_heartbeat_ns` and
  `last_restart_ns` are still reported bare.
- Report `failed` as a reconcile action, with a `failure` string naming why that
  host could not be reconciled.
