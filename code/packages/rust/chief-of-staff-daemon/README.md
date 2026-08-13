# chief-of-staff-daemon

Concrete, dependency-free-at-runtime executable for the D18 Chief of Staff
orchestrator. It composes the repository-owned configuration, credential,
package trust, storage, process supervision, exact data-plane authorities,
control API, WebSocket runtime, reconciliation, and shutdown packages without
adding new policy.

Run with the spec-default config path:

```sh
chief-of-staff-daemon
```

The process resolves `~/.chief-of-staff/config.toml` from `HOME` on Unix or
`USERPROFILE` on Windows. One explicit absolute config path may be supplied:

```sh
chief-of-staff-daemon /absolute/path/to/config.toml
```

The configured credential parent and trusted public-key files must already
exist. The daemon creates or initializes the configured state directory, binds
only the loopback address accepted by `chief-of-staff-daemon-config`, performs
one reconciliation before serving, and then reconciles at the configured health
interval. Channel topology mutation remains denied until the Trust Checker is
implemented. Host launch bindings come only from the shared durable pipeline
binding store; absent, stale, destroyed, directionally unauthorized, or
cross-pipeline records fail before process creation.

A non-empty optional `[data_plane]` table is provisioned before serving. Raw
32-byte channel secrets are loaded through the owner-only no-link reader, model
tags resolve only to their explicitly configured Ollama clients, and publishes
receive fresh UUID-v7 identities plus process-monotonic timestamps. Startup does
not probe model endpoints. An absent or empty table preserves the fail-closed
unavailable service for existing control-plane-only deployments.

When an Ollama model is configured, the daemon also restores the central durable
smart-home controller and injects a bounded core `smart_home.*` D18D catalog.
Model-offered definitions must match that catalog exactly; returned calls run
through D18D with the authenticated host identity and return structured results.
Grant activation and expiry plus durable authorization audit records use an
injected Unix-millisecond clock sampled immediately before each dispatch. A
missing, pre-epoch, or unrepresentable production timestamp fails closed before
the tool can run.

Operators may provision exact Chief host access with additive
`smart_home_tool_grants` entries in `[data_plane]`. Startup accepts only tools in
the daemon's installed ten-tool catalog, converts each declaration to a
tool-scoped least-privilege D23 grant, and commits changed records through the
central durable controller before serving. Identical records do not create a new
revision. The grant ledger is durable governance history: deleting a config row
does not erase an already committed grant; set the same `grant_id` to
`status = "revoked"` to disable it durably. Unknown tools, persistence failures,
future issuance times, or an unavailable provisioning clock fail startup without
publishing a partial in-memory policy.

An optional `[smart_home]` table makes this daemon the Home Assistant-compatible
local-controller process as well. Chief restores the durable D23 controller
exactly once, shares that live owner with both D18D model tools and the HTTP
adapter, provisions the adapter's stable local full-access principal through the
same serialized transaction boundary, and binds both loopback listeners before
either begins serving. A bind failure releases both listeners. Native shutdown,
an HTTP server failure, or a Chief server failure stops the peer listener and
joins it before control-plane teardown. The standalone
`smart-home-local-controller` remains available for deployments that do not opt
into Chief composition, but it must not point at the same state directory while
this table is enabled.

Setting `hue_mdns_interface` in that table also makes Chief the supervised Hue
discovery owner. Chief durably installs the canonical Hue mDNS schedule into
the same controller used by HTTP and model tools, starts the actor worker only
after both listeners bind, and stops and joins it during every normal or failure
shutdown. A worker clock or actor failure stops both listeners instead of
leaving a partially live daemon. Reapplying an identical interface is
idempotent; changing it durably replaces the worker configuration.

Setting `hue_pairing_kek_path` in the same table also makes Chief the supervised
Hue physical-presence pairing owner. The path names an existing owner-only
32-byte injected KEK; the daemon initializes or unseals the configured Vault
without placing key bytes in TOML, messages, snapshots, reports, or logs. This
opt-in is rejected while `[vault].container = true`. The worker watches pending
Hue sessions on the shared controller, preserves the requesting principal and
exact durable revision, and delegates registration, sealed credential storage,
transaction recovery, and central completion to
`smart-home-hue-pairing-service`. Link-button rejection remains retryable while
the session is pending. Clock or actor failure stops both listeners, and normal
shutdown joins the worker before the controller and unsealed Vault are dropped.

The shared HTTP adapter receives the same fallible Unix-millisecond clock as the
model-tool dispatcher. It samples that source once for every matched request
and reuses the result for grant activation/expiry, authorization audit,
persistence, freshness, and response generation. Clock unavailability returns
HTTP 503 before the handler runs; the daemon never substitutes timestamp zero.

The exported production data-plane composition boundary is also used by the real
Level 1 host integration test. That test supplies owner-only key files and a
loopback Ollama fixture, then proves encrypted receive, completion, encrypted
publish, and input acknowledgement through the same dispatcher used by `run`.

SIGINT, SIGTERM, Ctrl+C, Ctrl+Break, console close, logoff, and system shutdown
request a cooperative stop of every configured listener. Dropping the composed
process supervisor reaps every child still owned by this daemon instance.

## Validation

```sh
sh chief-of-staff-daemon/BUILD
```
