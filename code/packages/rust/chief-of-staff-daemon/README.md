# chief-of-staff-daemon

Concrete, dependency-free-at-runtime executable for the D18 Chief of Staff
orchestrator. It composes the repository-owned configuration, credential,
package trust, storage, process supervision, control API, WebSocket runtime,
reconciliation, and shutdown packages without adding new policy.

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

SIGINT, SIGTERM, Ctrl+C, Ctrl+Break, console close, logoff, and system shutdown
request a cooperative listener stop. Dropping the composed process supervisor
reaps every child still owned by this daemon instance.

## Validation

```sh
sh chief-of-staff-daemon/BUILD
```
