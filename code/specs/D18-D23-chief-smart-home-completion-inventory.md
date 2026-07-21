# D18-D23 Chief and Smart Home Completion Inventory

## Intent

Chief of Staff and the smart-home platform are related but separate tracks.

Chief of Staff owns reusable agent architecture: hosts, vault, job runtime,
tool contracts, approval policy, and audit. Smart home owns the local device
platform: integrations, normalized state, subscriptions, pairing, command
routing, supervision, automations, dashboards, and Home Assistant migration.

The bridge between them must stay thin. A Chief of Staff job can call D18D
`smart_home.*` tools, but D23 remains the source of truth for device state,
device authorization, pairing, and command execution.

## Current Landing Zone

The merged D18 and D23 work has the core local path in place:

- D18D defines model-facing tool definitions, invocation validation, events,
  terminal results, and execution journals.
- D23 defines protocol-neutral bridge, device, entity, state, event, command,
  capability, grant, and vault-reference types.
- D23 runtime owns registry reads, authorization decisions, event bus
  subscriptions, pairing sessions, command routing, optimistic state, desired
  state, and supervision observation.
- D23 testkit provides Hue-style fixtures, fake event streams, fake MQTT
  publications, fake local HTTP responses, and runtime seeding helpers.
- `chief-of-staff-smart-home-tools` adapts D18D JSON calls into D23 runtime
  requests without owning smart-home policy or state.

## This Slice

This slice closes the Chief-facing discovery gap while keeping the platform
boundary intact:

- `smart-home-runtime` now owns a discovery catalog, records preferred
  discovery results, reconciles them into unpaired bridge candidates, and
  exposes an authorized `execute_discover_tool` read path.
- `smart-home-testkit` now has deterministic Hue discovery fixtures so runtime,
  integration, and Chief bridge tests share the same fake discovery primitive.
- `chief-of-staff-smart-home-tools` now exposes D18D `smart_home.discover`
  without owning discovery policy or smart-home state.
- The bridge end-to-end test now covers list devices, discover, describe
  capabilities, health, subscribe, pair, command, optimistic state, and D18D
  journal/audit records.

This does not replace Home Assistant yet. It gives Chief of Staff a complete
typed route into D23 discovery records; the remaining smart-home work is to run
real network/radio/cloud discovery workers and persist the platform state.

## Current Follow-On Slice

This slice moves the smart-home platform side forward:

- `hue-core` now normalizes Hue mDNS advertisements and Hue cloud-fallback
  bridge observations into D23 `DiscoveryRecord` candidates.
- Hue discovery records preserve the correct split between `source = mdns` or
  `cloud_fallback` and command transport `lan_http`.
- Hue discovery batches can project unpaired `Bridge` candidates for
  `SmartHomeRuntime::record_discovery`.
- Hue discovery records can seed the existing physical-presence pairing plan
  without exposing application keys or raw credentials.
- `smart-home-testkit` now builds its deterministic Hue discovery runtime
  fixture through the canonical Hue mDNS normalization path.

## Current Discovery Worker Slice

This slice turns the discovery ingest shape into a worker handoff contract while
keeping the implementation testable and D23-owned:

- `smart-home-discovery` now has generic discovery worker-run envelopes with
  worker id/kind/status, per-source failures, run duration, and ingest summary
  counts.
- `smart-home-runtime` can ingest a full discovery worker run, record preferred
  catalog results, reconcile accepted records into unpaired bridge candidates,
  and report inserted/replaced/ignored outcomes.
- `hue-core` can wrap Hue mDNS and cloud-fallback observations into the generic
  D23 worker-run envelope, preserving malformed or non-Hue observations as
  worker failures instead of losing the whole scan.
- `smart-home-testkit` now seeds its Hue discovery runtime fixture through the
  worker-run ingest path, so runtime and Chief bridge tests exercise the same
  platform handoff a future LAN scanner will use.

## Current LAN mDNS Scan Slice

This slice attaches real LAN mDNS scan primitives to that worker handoff while
preserving the platform boundary:

- `udp-client` can send an unconnected multicast discovery probe and collect
  replies until a bounded read timeout or response limit is reached.
- `smart-home-discovery` can build mDNS PTR questions, run IPv4/IPv6 mDNS
  scans through the UDP transport, and parse DNS-SD PTR/SRV/TXT/A/AAAA replies
  with compressed names into reusable `MdnsAdvertisement` records.
- `smart-home-discovery` keeps malformed datagrams as per-packet scan failures,
  which lets discovery workers report partial LAN results instead of dropping a
  whole scan.
- `hue-core` can convert a generic `MdnsScanResult` into the D23 Hue discovery
  worker-run envelope.
- `smart-home-testkit` now seeds deterministic Hue discovery worker fixtures
  through the mDNS scan envelope, so runtime tests exercise the scanner handoff
  shape without opening sockets.

## Current Supervised Discovery Schedule Slice

This slice lets the D23 runtime own discovery cadence and scan scope without
moving socket work into the Chief bridge:

- `smart-home-runtime` can register scheduled discovery workers with
  integration id, worker kind, discovery sources, selected network interfaces,
  run timeout, interval, next due time, and metadata such as mDNS service type.
- Runtime read snapshots and non-mutating supervision plans now include due
  discovery worker runs alongside pairing expiry, state refresh, desired-state
  drift, and bridge-worker restart pressure.
- `SmartHomeRuntime::record_scheduled_discovery_worker_run` validates that a
  reported run matches the registered worker id, integration, and kind before
  ingesting records and advancing the next scheduled due time.
- Scheduled discovery workers track last run status, record/failure counts,
  accepted catalog changes, consecutive failures, and unhealthy status after
  partial or failed runs.
- `smart-home-testkit` now has a deterministic Hue scheduled-worker fixture, so
  runtime and integration tests can exercise the scheduled handoff without
  opening sockets or inventing another fake scheduler.

## Current Discovery Execution Handoff Slice

This slice gives a future supervised discovery actor an executable mDNS work
contract while keeping socket execution and runtime mutation separate:

- `smart-home-discovery` now has per-interface IPv4/IPv6 mDNS scan requests,
  scan plans, and aggregate scan reports that preserve both packet-level parse
  failures and interface-level transport failures.
- `smart-home-runtime` can project due mDNS discovery worker schedules into
  executable scan requests without mutating scheduler state, and mDNS schedules
  must now name the DNS-SD service type they plan to scan.
- `hue-core` can convert an aggregate mDNS worker scan report into the existing
  Hue D23 discovery worker-run envelope, preserving interface failure context
  as worker failure metadata.
- `smart-home-testkit` now seeds Hue discovery worker fixtures through the scan
  report path, so runtime and Chief-facing tests exercise the same handoff a
  supervised worker will report back.

## Current Discovery Runner Slice

This slice wires the mDNS execution contract into a reusable runner boundary
without moving platform state or socket supervision into the Chief bridge:

- `smart-home-discovery` now has an injectable `MdnsWorkerScanExecutor`,
  default UDP-backed request execution, and grouped scan-plan runners that turn
  runtime-produced mDNS requests into worker scan reports.
- The runner preserves per-interface successes, packet parse failures, and
  transport failures while grouping reports by worker, integration, and DNS-SD
  service type.
- `smart-home-testkit` now provides a scripted mDNS worker executor that can run
  scheduled runtime scan plans deterministically without opening sockets.
- The fixture path now proves schedule projection, scan execution, Hue report
  conversion, and scheduled runtime ingest end to end while keeping Chief of
  Staff on the existing thin `smart_home.*` tool bridge.

## Current Discovery Supervisor Run Slice

This slice lets D23 coordinate one deterministic supervised mDNS pass without
turning runtime into a Hue integration or process manager:

- `smart-home-runtime` now has a supervised mDNS discovery run helper that
  marks due workers started, executes due scan plans through an injectable mDNS
  runner, adapts reports into discovery worker runs, records scheduled ingest,
  and returns compact run/catalog outcome counts.
- Runtime keeps vendor-specific interpretation outside the scheduler through an
  injected `MdnsDiscoveryRunAdapter`, so Hue report conversion stays in
  `hue-core` or callers that compose Hue.
- Adapter failures are converted into deterministic failed discovery-worker
  runs, which advances schedule state and preserves failure pressure for
  supervisor status instead of leaving workers stuck as running.
- The new tests prove both a completed supervised mDNS pass and a failed adapter
  path using local fake runners, without opening sockets or moving the Chief
  bridge boundary.

## Current Discovery Observability Slice

This slice makes supervised discovery runs inspectable through the existing
D23 runtime and D18D bridge surfaces:

- `smart-home-runtime` now snapshots scheduled discovery workers with due
  status, next due time, last run status, record/failure counts, catalog change
  counts, total run count, and consecutive failure pressure.
- `RuntimeSupervisionObservation` now carries discovery scheduler counts and
  per-worker snapshots alongside the existing non-mutating supervision plan and
  bridge-worker heartbeat schedule.
- `chief-of-staff-smart-home-tools` now includes those discovery worker details
  in `smart_home.observe_supervision`, so Chief jobs and dashboards can triage
  scheduled discovery health without owning D23 scheduler policy.
- The bridge end-to-end test now proves the D18D handler can surface scheduled
  Hue mDNS worker health while the runtime remains the source of truth.

## Current Discovery Retry Policy Slice

This slice keeps retry/backoff decisions in the D23 scheduler while exposing
the resulting state through existing read-side surfaces:

- `ScheduledDiscoveryWorker` now owns retry delay, maximum retry delay, and
  retry multiplier settings alongside its normal interval and timeout.
- Failed or partial scheduled discovery runs advance the next due time by the
  capped retry delay for the current consecutive failure count; completed runs
  reset failure pressure and return to the normal interval.
- Runtime supervision snapshots now report the configured retry policy and the
  current retry delay when a worker is under failure pressure.
- `smart_home.observe_supervision` carries those runtime fields through the
  Chief bridge without adding Chief-owned scheduler policy.
- The runtime retry test proves capped backoff and recovery reset behavior, and
  the Chief end-to-end fixture proves the D18D tool output includes the policy
  fields.

## Current Hue Pairing Exchange Slice

This slice connects the existing D23 pairing session model to Hue's local
physical-presence registration exchange without moving Vault ownership or
network I/O into the Chief bridge:

- `hue-core` can turn a discovered-bridge pairing plan into a local HTTP
  registration request plan for `/api`, including the Hue `devicetype` payload
  and user-presence metadata.
- `hue-core` can parse Hue registration success responses into application-key
  credentials, parse link-button rejection responses as structured errors, and
  produce a Vault secret payload for the component that owns secret storage.
- `hue-core` then projects only a `VaultRef` and non-secret metadata into a
  pairing handoff; raw Hue application keys and client keys are not copied into
  runtime audit metadata.
- `smart-home-runtime` now accepts metadata-bearing pairing completions, stores
  the `VaultRef`, marks the bridge online, and includes the non-secret metadata
  in the bridge-health audit event.
- `smart-home-testkit` proves the no-socket path end to end with a fake local
  HTTP response, simulated Vault handoff, runtime session completion, and an
  assertion that raw Hue credentials are absent from audit metadata.

## Chief Of Staff Remaining Work

The Chief host-profile slice now provides JSON orchestrator profiles, isolated
host tool ownership, privilege ceilings, capability coverage checks, catalog
completeness gates, and executable routing. The Weather Agent uses three host
profiles for fetch, classify, and write instead of wiring an unrestricted tool
runtime directly.

The supervised-host slice now connects those profiles to the repo-owned stdio
process pool and generic job protocol. Activation requires exactly one process
specification per declared host; RPC follows profile tool ownership; process
snapshots expose live-worker and shutdown state; and bounded restart is proven
by a host that crashes one in-flight call and services the next call after
restart.

The signed-package gate now hashes sealed agent contents in deterministic,
length-framed path order, verifies raw Ed25519 signatures with the repo-owned
crypto crates, rejects symlinks and byte tampering, resolves `PUBKEY_ID` through
a typed trusted keyring, and enforces signer privilege ceilings before a
supervised process can launch. Developer keys are capped at Tier 1.

The deny-all Deno slice now re-verifies the sealed package at activation, derives
literal no-prompt and deny flags in Rust, launches the signed
`code/agent_runtime.ts`, and carries the existing host RPC envelope over stdio.
The executable worker proves environment, filesystem read/write, subprocess,
and network access are denied; post-signing entrypoint tampering prevents launch.

The host-capability slice now makes the signed `launch.sh` and runtime process
arguments come from one canonical deny-all launch plan. Launch-time verification
rejects any script drift. Subprocess-originated RPC is parsed into the canonical
D18D invocation contract and executed only by the Rust host's profile-gated,
capability-checked handler catalog; unknown tools never reach a handler.

The bidirectional transport slice now runs a signed deny-all Deno agent while
the active Rust host services agent-originated `host.*` requests over the
versioned stdio envelope. The host re-verifies the package and signer tier,
requires envelope and call ids to agree, routes allowed calls through the real
D18D handler catalog, and returns typed rejection frames for denied tools. A
real-Deno test proves one allowed call reaches Rust and one undeclared call does
not reach a handler.

The first reusable Chief job slice now takes the existing Weather Agent through
the in-process D18C job plan and executor, isolated host tool runtime, centralized
write-approval policy, D18D execution journal, artifact write, validated
`JobRunReceipt`, and compact user-visible umbrella report. The approved path
records one granted write and three completed calls; the unapproved path stops
at the policy gate and never writes the recommendation.

That job now persists each canonical payload-free D18D audit row through
`chief-of-staff-tool-audit-store` and the D18A local-folder backend before
returning actor failures. A fresh store instance reloads the rows and emits a
compact summary keyed to job, run, host profile, session, and user. Executable
tests prove both the successful three-call run and the approval-blocked write
survive a runtime restart without storing arguments, outputs, or credentials.

The Tier2 approval slice now gives D18D a canonical user-visible challenge and
explicit-consent, biometric, and hardware-key assurance levels. Host policy can
require approval at a privilege threshold; Tier2 grants must be biometric and
bound to the active call challenge before a handler runs, while Tier3 requires
a hardware key. The scheduled Weather Agent job proves pending, weak-denied,
and biometric-approved paths through the real host runtime, execution journal,
user report, and restarted durable audit reader.

The vault-leasing slice adds a reusable Chief host runtime over the zeroizing
`vault-leases` manager. `vault.request_lease` now requires a challenge-bound
Tier2 approval and returns only a random `VaultRef`; the scheduled Weather Agent
passes that handle to its fetch host, which atomically consumes the lease and
keeps the raw credential out of model-visible values, reports, and durable
audit rows. Executable tests cover approved and approval-denied jobs plus
one-shot, revoked, malformed, and unknown lease behavior.

These items are Chief of Staff architecture, not smart-home platform work:

- No remaining items within the D18 Chief architecture completion boundary.

## Smart Home Remaining Work

These items move toward retiring an existing Home Assistant install:

- Connect the supervised mDNS runtime pass to an actor or process that manages
  lifecycle, OS interface binding, and persistence for schedules, results, and
  runtime state across restarts. Retry/backoff policy now exists in the runtime
  scheduler, but an external actor still needs to drive durable process
  lifecycle.
- Finish production Hue pairing by connecting the local HTTP registration plan
  to the worker that presses through real LAN I/O and durable Vault writes. The
  typed request/response/VaultRef handoff and runtime no-secret audit trail now
  exist.
- Add real Hue local HTTP command/read workers and Hue event-stream workers
  behind the existing runtime surfaces.
- Persist registry, state cache, event history, command history, pairing
  sessions, desired state, and automation definitions.
- Add a local API surface for dashboard, mobile, CLI, and Chief of Staff jobs.
- Add an automation/rules engine with schedules, triggers, conditions, scenes,
  idempotency, dry-run planning, and audit.
- Add platform integrations beyond Hue: MQTT, Matter/Thread, Zigbee, Z-Wave,
  cameras, locks, thermostats, and sensors.
- Build Home Assistant migration tools for devices, rooms, scenes,
  automations, dashboards, and historical state export where feasible.
- Provide a dashboard that can inspect devices, rooms, state, health,
  automations, event history, pairing, and command audit.

## End-To-End Definition

The first meaningful "working end to end" target should be:

1. A D18 Chief job starts from a host/orchestrator profile.
2. The job calls D18D `smart_home.*` tools.
3. The tools reach the D23 runtime.
4. D23 authorizes reads, subscribe, pair, and low-risk light commands.
5. A Hue fixture or local Hue bridge returns normalized state and health.
6. The job writes a D18D execution journal.
7. The user receives a compact home status or action report.

The first real-home target after that should be:

1. Discover one Hue bridge on LAN.
2. Pair it through a vault-backed credential path.
3. List lights and health.
4. Subscribe to normalized events.
5. Turn one light on and off.
6. Persist state and command audit.
7. Show the result through a local API or dashboard.
