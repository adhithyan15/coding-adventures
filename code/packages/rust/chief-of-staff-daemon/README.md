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
interval. Channel and pipeline mutations resolve every referenced agent,
channel, package hash, and selected model through exact `[privilege]` tier maps.
Fully declared Tier 0 mutations can proceed through Trust Checker; missing maps
remain denied. Optional `[privilege].tier_1_notification_command` and
`tier_2_biometric_command`, and `tier_3_hardware_key_command` paths resolve
against the explicit daemon home and launch operator-reviewed helpers directly
through distinct bounded, versioned, environment-cleared stdin/stdout protocols.
The first enables notification approval and the canonical five-second timeout
only for Tier 1. The second accepts only biometric-strength results for Tier 2;
the third accepts only hardware-key-strength results for Tier 3. Their canonical
thirty- and sixty-second timeouts are denial. Each missing helper keeps its tier
fail-closed. Host launch bindings come only from the
shared durable pipeline binding store; absent, stale, destroyed, directionally
unauthorized, or cross-pipeline records fail before process creation.

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

The six-field `onvif_pairing_*` tuple enables one supervised ONVIF credential
worker for one exact installed bridge. It names the bridge, an owner-only
32-byte Vault KEK, and owner-only username and password files with their exact
byte lengths. Chief rejects partial tuples and containerized Vault custody,
selects only an unexpired pending session for that bridge, and delegates native
camera inspection plus recoverable sealed credential handoff to
`smart-home-onvif-pairing-service`. The worker shares the controller used by
HTTP and model tools and participates in the same coordinated failure and
shutdown path.

The six-field `axis_pairing_*` tuple enables one supervised Axis VAPIX
credential worker for one exact installed bridge. It names the bridge, an
owner-only 32-byte Vault KEK, and owner-only username and password files with
their exact byte lengths. Chief rejects partial tuples and containerized Vault
custody, selects only an unexpired pending session for that bridge, and
delegates exact HTTPS endpoint validation, native camera inspection, and
recoverable sealed credential handoff to `smart-home-axis-pairing-service`.
The worker shares the controller used by HTTP and model tools and participates
in the same coordinated failure and shutdown path.

The six-field `zoneminder_pairing_*` tuple enables one supervised ZoneMinder
credential worker for one exact installed NVR bridge. It names the bridge, an
owner-only 32-byte Vault KEK, and owner-only username and password files with
their exact byte lengths. Chief rejects partial tuples and containerized Vault
custody, selects only an unexpired pending session for that bridge, and
delegates exact HTTPS endpoint and installed-monitor validation, authenticated
API 2.0 inspection, and recoverable sealed credential handoff to
`smart-home-zoneminder-pairing-service`. Login tokens remain process-local and
are discarded after inspection; token refresh ownership and event or mutation
hosts remain outside this composition. The worker shares the controller used
by HTTP and model tools and participates in the same coordinated failure and
shutdown path.

The eight-field `reolink_pairing_*` tuple enables one supervised Reolink
credential worker for one exact installed camera or NVR bridge. It names the
bridge, canonical host, pinned socket address, an owner-only 32-byte Vault KEK,
and owner-only username and password files with their exact byte lengths. Chief
rejects partial tuples and containerized Vault custody, selects only an
unexpired pending session for that bridge, and delegates endpoint pinning,
installed camera/channel identity validation, authenticated native inspection,
and recoverable sealed credential handoff to
`smart-home-reolink-pairing-service`. Authentication remains process-local and
only the opaque Vault reference enters durable runtime state. The worker shares
the controller used by HTTP and model tools and participates in the same
coordinated failure and shutdown path.

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

## Agent tools: `net.fetch` and `vault.request_lease`

When `[data_plane] ollama_models` composes a model-tool surface, it has two
sources.

- **Smart home** offers the same tools to every host. It authorizes them by
  capability grant when they are called.
- **Agent tools** (`src/agent_tools.rs`, D18V "Daemon composition") gives each
  host its own surface.

The agent-tools surface is derived from the host's own signed package:

```text
registration ─▶ verify_agent_package ─▶ digest == package_hash
             ─▶ parse_manifest, tier <= signing key ceiling
             ─▶ HostProfile::from_manifest ─▶ check_registration(tool)
                   net.fetch            needs net:connect
                   vault.request_lease  needs a vault + vault_access leased|both
```

A host is offered a tool only if its manifest lists it in `allowed_tools`,
grants its `tool_capabilities`, and declares a high enough `privilege_tier`.
That is Tier 1 for `net.fetch` and Tier 2 for `vault.request_lease`, so a
developer-signed package can fetch but cannot lease. Every call runs as the
registration's host name.

A lease is limited three times over:

1. The manifest's `vault_access` says which secrets the host may lease, and
   for how long.
2. The sealed record says which agents the owner allowed.
3. `consume_for` redeems the lease only for the host it was issued to, and
   only for a destination the record names.

If a package fails verification, this source offers that host nothing. It
leaves smart home alone.

At startup, `load_chief_vault_runtime` opens the vault and registers every
sealed record before anything serves. One corrupt record stops startup. To
rotate a secret, run `chief-of-staff vault put`, then restart the daemon.

## Per-agent channel brokers: `[hosts.broker]`

With `[hosts.broker]` set, each host bound to a channel gets its own
broker process (D18S P2.6d-2b). It holds only that agent's channel keys,
and the host's Receive, Publish and Acknowledge requests go to it rather
than to the daemon.

```text
[data_plane] channel_keys ─▶ broker_key_files    (read: 1 slot, write: 2)
[hosts.broker] sha256     ─▶ VerifiedExecutable  (startup: wrong bytes stop the daemon)
                          ─▶ ChannelBrokers ─▶ ProcessHostSupervisor
```

The daemon opens no key file here. The launcher opens an agent's files,
owner-only or refused, when it launches that agent's broker, and passes
them by descriptor. Off Linux the table is refused at startup
(`BrokerUnsupported`), because there is no verified launch there yet.

In this step the daemon still provisions the same keys for its own channel
path. Step 2c removes that path, and `channel_keys` will then require
`[hosts.broker]`.

## Validation

```sh
sh chief-of-staff-daemon/BUILD
```
