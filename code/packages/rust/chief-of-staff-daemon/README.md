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

The exported production data-plane composition boundary is also used by the real
Level 1 host integration test. That test supplies owner-only key files and a
loopback Ollama fixture, then proves encrypted receive, completion, encrypted
publish, and input acknowledgement through the same dispatcher used by `run`.

SIGINT, SIGTERM, Ctrl+C, Ctrl+Break, console close, logoff, and system shutdown
request a cooperative listener stop. Dropping the composed process supervisor
reaps every child still owned by this daemon instance.

## Validation

```sh
sh chief-of-staff-daemon/BUILD
```
