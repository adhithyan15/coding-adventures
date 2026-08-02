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

## Current Durable Discovery Service Slice

This slice gives the supervised mDNS pass an actor-owned lifecycle and durable
restart boundary:

- `smart-home-discovery-service` owns the D23 runtime, mDNS executor,
  report adapter, and repository-owned `StorageBackend` inside one actor state.
- Typed tick messages drive due runs sequentially, and the runtime still emits
  exactly the selected-interface IPv4/IPv6 requests through the injectable
  executor boundary.
- Every tick persists worker cadence and retry pressure, a compact run journal,
  and service health. Reopening against the same backend restores that state
  before another network request can run.
- Local-folder restart tests prove successful cadence, named-interface binding,
  failed-run backoff, service counters, and durable run audits survive process
  replacement.

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

## Current Hue Pairing Service Slice

This slice runs the physical-presence registration exchange through production
host boundaries and durable secret storage:

- `smart-home-hue-pairing-service` owns the D23 runtime, an injectable Hue
  registration transport, and the repository's sealed Vault store inside one
  actor state.
- The production transport executes bounded HTTP/1 over LAN TCP or the shared
  TLS platform. HTTPS remains certificate-verifying and accepts caller-supplied
  Hue trust roots instead of silently disabling verification.
- A pending D23 session drives the canonical `/api` registration request,
  successful application and client keys are encrypted at rest, and D23
  receives only a random `VaultRef`.
- If runtime completion fails after the Vault write, the service attempts a
  revision-bound rollback so an unusable credential is not left behind.
- Real loopback-network and local-folder restart tests prove the request reaches
  LAN I/O, credentials survive a Vault restart, and raw secrets never enter
  runtime state, event metadata, actor snapshots, or pairing reports.

## Current Hue LAN Integration Slice

This slice turns the transport-neutral Hue client into the production D23
bridge worker path:

- `hue-integration` executes bounded CLIP v2 HTTP/1 over LAN TCP or the shared
  TLS platform, preserving certificate verification and caller-supplied trust
  roots.
- Full resource snapshots project Hue devices, lights, motion sensors, buttons,
  scenes, and bridge health into the existing D23 runtime.
- Authorized D23 light commands route through the native Hue resource endpoint;
  rejected commands cannot reach Vault reads or LAN I/O, and accepted commands
  always publish a final bridge result.
- Bounded Hue Server-Sent Event reads use the incremental parser and project
  native light updates into normalized D23 device events and state.
- Paired application/client keys are decrypted only inside the worker. Actor
  messages, reports, errors, runtime state, and debug output contain no raw
  credentials.
- A real loopback bridge test proves snapshot refresh, authorized command
  dispatch, event-stream ingestion, and normalized state update through the
  production socket transport.

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

## Current Durable Runtime Persistence Slice

This slice makes normalized D23 runtime data restart-safe without moving
process-local worker or subscription ownership into storage:

- `smart-home-core` exposes serde support for the normalized protocol-neutral
  model, and `smart-home-runtime` can produce and restore a versioned durable
  snapshot through its existing validated registry and desired-state paths.
- The snapshot retains bridge, device, entity, scene, state, device-event,
  authorization, capability-grant, runtime-event, command-result, pairing,
  optimistic-state, and desired-state data.
- `smart-home-runtime-store` persists that snapshot through any D18A
  `StorageBackend`, uses compare-and-swap revisions, and keeps validated opaque
  automation definitions beside runtime data for the future rules engine.
- A local-folder restart test closes and reopens the durable backend before
  proving topology, state, event and command history, pairing, desired state,
  and automation definitions are queryable again.
- Live discovery workers and event subscriptions remain process-local and are
  rebuilt by their owners instead of restoring stale network work or consumers.

## Current Durable Local API Slice

This slice closes the local API boundary with a runnable restart-safe
controller instead of another fixture wrapper:

- `smart-home-platform-http` already exposes Home Assistant-compatible and
  native routes for dashboard, mobile, CLI, and Chief clients over the
  normalized runtime, including authorized service and desired-state writes.
- Its runtime adapter now accepts a live clock and a durable mutation callback.
  Every accepted service or desired-state mutation is saved synchronously; a
  failed save restores the exact pre-request runtime and returns HTTP 503.
- The `smart-home-local-controller` binary restores
  `smart-home-runtime-store` from a D18A local folder, installs and persists
  its local API capability grant, and serves the existing browser/API surface
  through the repository HTTP stack.
- Executable tests prove an HTTP desired-state mutation is queryable after a
  fresh store instance and prove failed persistence leaves no in-memory
  mutation behind.

## Current Automation Runtime Slice

This slice turns the previously opaque automation storage boundary into an
executable, restart-safe rules runtime:

- `smart-home-automation-runtime` owns typed schedule and normalized device
  event triggers, state-equality conditions, direct commands, scene expansion,
  stable per-occurrence idempotency keys, dry-run plans, and bounded audit.
- Every automation mutation delegates to the existing authorized D23 command
  tool. Definitions, consumed trigger occurrences, and automation audit records
  persist atomically beside the normalized runtime snapshot.
- The native local API can create and inspect definitions, preview or execute
  schedule and event evaluations, and read automation audit records.
- The production local controller restores the engine, runs schedule
  evaluation on a local worker, and synchronously persists definition and
  execution mutations with rollback when storage fails.
- Executable tests cover scene planning, conditions, event matching,
  idempotency across snapshot restore, durable state validation, and the full
  create-preview-execute-audit HTTP lifecycle.

## Current Z-Wave Runtime Integration Slice

This slice connects the repository's Z-Wave protocol stack to the normalized
D23 runtime instead of adding another protocol-only layer:

- `smart-home-zwave-integration` installs a Serial API controller and
  interviewed nodes as normalized bridges, devices, entities, capabilities,
  and Z-Wave protocol identifiers.
- Application Command Handler reports flow through `zwave-command-classes`
  into normalized device events and runtime state; report-specific sensor
  capabilities such as temperature are added when first observed.
- Outbound light and lock commands must pass D23 authorization before the
  adapter builds reliable Serial API SendData frames.
- SendData response, callback, failure, and timeout states stay correlated to
  the accepted D23 command.
- Executable tests cover switches, dimmers, locks, battery and multilevel
  sensor reports, rejected commands, successful callbacks, and timeouts.
- Serial port ownership, inclusion, and S2 security remain production host
  concerns above the completed typed runtime adapter boundary.

## Current Z-Wave Serial Host Slice

This slice moves the completed adapter across a real serial byte-stream
boundary:

- `smart-home-zwave-host` opens an OS serial port with bounded timeouts and
  owns Serial API SOF framing, ACK/NAK/CAN handling, retries, checksum
  rejection, and a bounded unsolicited-message queue.
- The host runs the typed version, memory-id, controller-capability, and
  init-data bootstrap, rejects end-device APIs, and installs the discovered
  controller identity into D23.
- Authorized commands cross the real wire boundary, synchronous SendData
  responses and asynchronous callbacks remain correlated, and terminal
  success, failure, and timeout results are published through the runtime.
- Application Command Handler frames can be pumped through the same session
  into normalized runtime state without dropping frames received while another
  request is awaiting its response.
- A runnable one-shot host binary bootstraps a configured serial controller,
  while scripted byte-stream tests prove retries, ACK/NAK behavior, malformed
  frame rejection, queued unsolicited frames, bootstrap, command completion,
  and timeout publication.
- Node inclusion and S2 remain explicit follow-on host state machines.

## Current MQTT Runtime Integration Slice

This slice adds a production broker boundary and turns the existing MQTT
topic/event primitives into normalized D23 devices:

- `smart-home-mqtt-integration` owns a real MQTT 3.1.1 connection, bounded
  polling, reconnect health, QoS subscriptions, retained deliveries, and
  broker-native cursors through `rumqttc`.
- Home Assistant MQTT discovery records dynamically install lights, switches,
  binary sensors, numeric sensors, and thermostats as normalized bridges,
  devices, entities, capabilities, and protocol identifiers.
- Discovered state and availability topics flow into normalized state, health,
  event history, and stream checkpoints, including scalar JSON value-template
  extraction for common sensor payloads.
- Light, switch, and thermostat commands must pass D23 authorization before
  MQTT publication; accepted commands carry command/correlation metadata and
  broker queue failures publish terminal command audit.
- Broker credentials remain ephemeral host values while durable bridge records
  retain only an opaque `VaultRef`.
- A runnable bounded host binary and a scripted TCP broker test prove real
  CONNECT, SUBSCRIBE/SUBACK, retained discovery, dynamic state subscriptions,
  normalized state, authorization, command publication planning, delivery
  cursors, and transport failure audit.

## Current Zigbee Runtime Integration Slice

This slice connects the repository's Zigbee application protocol stack to the
normalized runtime without pretending to own a coordinator transport:

- `smart-home-zigbee-integration` installs a serial coordinator with an opaque
  radio-network-key reference and projects ZDO-interviewed Home Automation
  endpoints into normalized devices, entities, and ZCL-derived capabilities.
- Inbound APS bytes are validated against the interviewed source endpoint,
  profile, and cluster before ZCL attribute reports become confirmed runtime
  state events.
- Outbound light commands pass through D23 authorization, idempotency, and
  command audit before the adapter emits round-trippable APS/ZCL bytes.
- Coordinator radio ownership, joining, network-key leasing, APS security,
  delivery retries, and acknowledgements remain an explicit production host
  boundary.

## Current Matter Runtime Integration Slice

This slice connects the repository's typed Matter application model to D23
while keeping the absent commissioning and secure-session host explicit:

- `smart-home-matter-integration` installs an opaque fabric/controller
  boundary and projects externally commissioned node endpoint clusters into
  normalized devices, entities, identifiers, and capabilities.
- Typed Matter attribute reports are checked against installed endpoint
  clusters before becoming confirmed runtime state events.
- Light and lock commands pass authorization and command audit before the
  adapter creates a typed Matter invocation for a secure-session host.
- Durable topology retains only a `VaultRef`; PASE/CASE, certificate
  validation, fabric key storage, Interaction Model encoding, subscriptions,
  and network I/O remain production host work.

## Current Home Assistant Migration Slice

This slice creates an executable, review-first boundary for retiring Home
Assistant without silently changing behavior:

- `smart-home-home-assistant-migration` accepts a versioned export of areas,
  devices, entities, current state, scenes, and a bounded automation subset.
- The planner assigns deterministic source-prefixed identifiers, preserves
  Home Assistant registry identifiers and metadata, maps known domains to D23
  capabilities, and retains unknown domains as observe-only entities.
- Area, device, entity, scene, condition, and action references are validated
  before apply. Unsupported or unresolved automation behavior is a blocking
  diagnostic instead of a partial import.
- Apply upserts topology, state, scenes, and durable automation definitions
  through the existing D23 runtime and automation APIs. Stable source
  fingerprints and receipts make reruns idempotent and auditable.
- The CLI emits either a dry-run plan or an applied runtime/automation snapshot
  through an atomic file replacement. Unit and process-level tests prove dry
  runs, scene and automation mapping, blocking diagnostics, repeat apply, and
  artifact round trips.

## Current Home Assistant Live Export Slice

This slice connects a running Home Assistant instance to the review-first
migration boundary:

- `smart-home-home-assistant-export` opens Home Assistant's WebSocket API,
  performs the `auth_required` / token / `auth_ok` handshake, and keeps the
  long-lived access token out of arguments, artifacts, and error output.
- The collector requests the area, device, and entity registries plus all
  current states, then projects the responses into the versioned migration
  export contract in deterministic identifier order.
- State records absent from the entity registry receive explicit synthetic
  entity records instead of being silently discarded. Duplicate identifiers,
  failed commands, malformed payloads, and premature disconnects fail the
  collection.
- The CLI performs atomic output replacement. Protocol tests use a real local
  WebSocket server to prove authentication, command ordering, normalization,
  failed authorization, and token redaction.
- Scene and automation definitions are not inferred from
  registry/current-state payloads; their live collection is delegated to the
  reviewed definition-enrichment stage below.

## Current Home Assistant Historical State Slice

This slice carries source history into D23's durable replay log instead of
leaving it as a detached archive:

- `smart-home-home-assistant-history` authenticates to Home Assistant's
  WebSocket API and requests `history/history_during_period` in bounded entity
  batches derived from the reviewed topology migration plan.
- The collector validates returned entity identities and RFC3339 timestamps,
  keeps full source state and attributes, and sorts records deterministically.
- The planner maps historical states to topology-backed D23 capabilities,
  preserves source payload details as event metadata, emits diagnostics for
  lossy values, and creates stable content-derived event identifiers.
- Apply routes chronological events through `SmartHomeRuntime`, skips
  identical events on repeat apply, restores the topology export's newer
  current state after replay, and emits a durable snapshot containing registry
  and runtime event history.
- Dry-run and applied CLI artifacts are written atomically. Real WebSocket and
  process tests prove batched collection, durable replay, repeat idempotency,
  current-state preservation, and token redaction.

## Current Home Assistant Definition Collection Slice

This slice retrieves executable source definitions without approximating
unsupported Home Assistant behavior:

- `smart-home-home-assistant-definitions` consumes the reviewed topology
  export, authenticates as an administrator, and requests each automation's
  raw configuration through `automation/config` over the WebSocket API.
- Editable Home Assistant scenes are retrieved from
  `/api/config/scene/config/{config-id}` over bounded HTTP or HTTPS using the
  entity registry's stable configuration identifier.
- The collector accepts only the importer's executable subset: state and
  simple time-pattern triggers, state conditions, and bounded scene, light,
  switch, lock, and thermostat actions. Templates, delays, multi-trigger
  semantics, device/area targets, unsupported services, and non-editable scene
  platforms are skipped with durable diagnostics instead of being guessed.
- The enriched artifact keeps definitions and a source-fingerprinted
  collection report in deterministic order. Its extra report field is
  backward-compatible with the existing migration reader, so the same file
  can be planned and applied without a conversion step.
- Real local WebSocket, HTTP, chunked-response, and CLI process tests prove
  authenticated collection, migration compatibility, deterministic reruns,
  partial-definition diagnostics, atomic output, and token redaction.

## Current Home Assistant Dashboard Migration Slice

This slice migrates concrete Lovelace definitions without executing custom
frontend code or guessing at unsupported behavior:

- `smart-home-home-assistant-dashboard-migration` authenticates to Home
  Assistant and uses `lovelace/dashboards/list`, `lovelace/config`, and
  `lovelace/resources/list` over the WebSocket API.
- Standard entity, light, thermostat, sensor, tile, button, entities, glance,
  and history cards become a deterministic native dashboard manifest. Layout
  stacks, grids, and section views are flattened in source order.
- Entity references are accepted only when they are enabled in the reviewed
  topology export and use the same `ha:<entity-id>` identifiers as the
  executable runtime migration.
- Custom cards and resources, unsupported actions, malformed rows, and unknown
  entities produce durable review diagnostics instead of approximations. A
  listed dashboard whose configuration cannot be fetched blocks applied
  migration while remaining inspectable in dry-run output.
- Real local WebSocket and CLI process tests prove authenticated multi-dashboard
  collection, resource capture, deterministic source fingerprints, blocked
  apply behavior, atomic output, and token redaction.

## Current Operational Dashboard Slice

This slice turns the durable local API and migrated dashboard artifacts into a
complete browser-operated control surface:

- `smart-home-dashboard-core` owns and validates the native dashboard manifest
  shared by Home Assistant migration and the local controller.
- The durable controller loads a raw manifest or applied migration artifact
  before binding and exposes it through
  `/api/smart_home/dashboard_manifest`.
- The embedded dashboard consumes native manifest views to scope entities and
  provides direct sections for health, rooms, devices, current state,
  automations and audit, pairing sessions, state history, runtime events,
  command results, authorization decisions, and capability grants.
- Pairing list/detail routes expose session state and opaque vault references
  without disclosing credential material.
- Rust route tests plus desktop and mobile browser verification prove the
  manifest, automation, pairing, and audit workflows over the fixture runtime.

## Current ONVIF Camera Integration Slice

This slice adds the first executable production camera path without placing
privacy-sensitive media endpoints in durable D23 state:

- `smart-home-onvif-integration` sends bounded ONVIF WS-Discovery probes over
  UDP, parses namespace-aware ProbeMatch responses, and emits normalized D23
  discovery records.
- Authenticated ONVIF SOAP calls use WS-Security UsernameToken password digests
  over bounded HTTP/1.1 or certificate-verifying HTTPS to collect device
  information, media profiles, snapshot endpoints, and RTSP stream endpoints.
- ONVIF cameras project to first-class normalized `Camera` entities and the
  `Onvif` protocol family. Runtime state contains profile metadata but no media
  URI, password, nonce, or credential material.
- `smart-home-camera-media` keeps endpoints process-local. Short-lived,
  principal-bound, single-use leases backed by active Human Approval grants for
  `camera.snapshot` or `camera.stream` authorize one trusted-host delivery; the
  lease holder never receives the snapshot or stream endpoint URI.
- Real loopback UDP and TCP tests prove discovery, five authenticated SOAP
  exchanges, runtime installation, capability authorization, media redemption,
  and redaction boundaries over actual host transports.

## Current Shelly Gen2/Gen3 Integration Slice

This slice adds a production broader-device path for local relays, lights,
inputs, sensors, and energy monitors:

- `_shelly._tcp.local` mDNS advertisements become verified D23 discovery
  records using the official generation TXT marker.
- Bounded HTTP/1.1 requests inspect `/shelly` and `Shelly.GetStatus`, then
  project supported components into normalized devices, entities,
  capabilities, and confirmed state.
- D23 authorization gates `Switch.Set` and `Light.Set` RPC mutations, including
  brightness control, before the host sends a device request.
- Authentication-enabled devices fail closed with an explicit pairing boundary;
  credentials are not accepted or persisted by this first host slice.
- Real loopback TCP tests prove inspection, runtime installation, authorization,
  and command transfer over the production transport.

## Current WLED Integration Slice

This slice adds a production broader-device path for local addressable-light
controllers:

- `_wled._tcp.local` mDNS advertisements become verified, no-pairing D23
  discovery records using WLED's advertised MAC when present.
- Bounded HTTP/1.1 requests inspect the documented `/json/si` state and device
  information endpoint, then project a master light plus capability-aware
  segment lights into normalized devices, entities, and confirmed state.
- WLED color capability bits determine whether each segment exposes RGB and
  color-temperature commands; effect identifiers remain observed state rather
  than an unsupported generic command.
- D23 authorization gates power, brightness, RGB, and mirek-to-Kelvin color
  temperature mutations before the host posts to `/json/state`.
- Real loopback TCP tests prove inspection, runtime installation, authorization,
  and command transfer over the production transport. This first host uses
  polling and does not claim WebSocket push support.

## Current Camera Media Security Hardening Slice

The camera-media broker is a portable policy boundary, not a transport host:

- An access request carries target, media kind, purpose, and bounded TTL only.
  The trusted host supplies the authenticated principal, monotonic current time,
  and collision-resistant nonce source; callers cannot backdate or impersonate
  those fields through the request DTO.
- A lease records the endpoint generation it authorized. Registering or rotating
  an endpoint advances that generation, invalidating every older lease before
  transport execution.
- Redemption rechecks the current D23 Human Approval grant at trusted current
  time, atomically consumes the lease, and lends the endpoint only to a trusted
  media executor. Snapshot bytes are bounded before release. The executor yields
  an owned stream resource; the service mints the public session ID and retains
  the resource through explicit close or trusted-time expiry. Failed teardown
  remains owned, reported, and retryable rather than disappearing from state.
- Endpoint, active-lease, per-principal lease, stream, and audit tables are bounded
  by policy. URL userinfo and fragments fail closed. Plaintext schemes are denied
  by default and require an explicit loopback-fixture opt-in; secure query tokens
  remain confined to the trusted executor. Audit rows contain no bearer ID.
- Clock, authenticated identity, nonce generation, and media I/O are installed
  once in the host-owned service and cannot be substituted per request. The
  deterministic policy core therefore declares an explicit empty package
  capability profile; the later native ONVIF host owns and must obtain approval
  for its nonempty authority.

## Current Govee LAN Integration Slice

This slice adds a production local-UDP path for LAN-enabled Govee lights:

- Govee multicast scans on `239.255.255.250:4001` collect bounded replies on
  UDP 4002, validate each response against its source address, and emit
  verified, no-pairing D23 discovery records.
- Bounded `devStatus` requests inspect fixed device endpoints on UDP 4003 and
  project power, brightness, RGB, and color-temperature state into normalized
  light entities.
- D23 authorization gates `turn`, `brightness`, and `colorwc` mutations before
  native transfer, and a post-command status query verifies and records the
  confirmed device state.
- Real loopback UDP tests prove discovery, inspection, runtime installation,
  authorization, command transfer, and post-command verification over the
  production transport. LAN Control must be enabled on the device; this path
  does not accept Govee cloud credentials.

## Current LIFX LAN Integration Slice

This slice adds a production local-UDP path for LIFX lights:

- Binary `GetService` probes use IPv4 UDP broadcast on port 56700, collect
  bounded replies, validate packet size, protocol flags, source correlation,
  service type, and device serial, and emit verified no-pairing D23 records.
- Direct `GetColor` inspection correlates source, sequence, target, and packet
  type before projecting power, brightness, RGB, and color-temperature state
  into normalized light entities.
- D23 authorization gates `SetLightPower` and `SetColor` mutations. Power,
  brightness, RGB, and color-temperature commands are followed by a fresh
  `GetColor` query that verifies and records the confirmed device state.
- Real loopback UDP tests prove binary packet handling, discovery, inspection,
  runtime installation, authorization, native transfer, and post-command
  verification over the production transport. This path does not accept LIFX
  cloud credentials.

## Current TP-Link Kasa Legacy LAN Integration Slice

This slice adds a production local-UDP path for credential-free legacy Kasa
plugs, switches, and lights:

- XOR-obfuscated `get_sysinfo` probes use bounded IPv4 UDP broadcast on port
  9999, validate response JSON and stable device identity, and emit verified,
  no-pairing D23 discovery records.
- Direct device inspection distinguishes relay devices from bulbs and projects
  model-aware power, brightness, RGB, and color-temperature state and
  capabilities into normalized entities.
- D23 authorization gates native relay and bulb transition mutations before
  wire transfer. Every accepted command is followed by a fresh `get_sysinfo`
  query that verifies and records confirmed state.
- Real loopback UDP tests prove obfuscation, discovery, inspection, runtime
  installation, authorization, command transfer, and post-command verification
  over the production transport. This path accepts no cloud credentials and
  does not claim newer authenticated KLAP/Tapo devices.

## Smart Home Remaining Work

These items move toward retiring an existing Home Assistant install:

- Continue platform integrations beyond Hue, MQTT, ONVIF, Shelly Gen2/Gen3,
  WLED, Govee LAN, LIFX LAN, Kasa legacy LAN, and the Z-Wave, Zigbee, and Matter
  runtime adapters: ONVIF PullPoint camera events, RTSP media transfer and
  recording, vendor-specific camera/NVR integrations, authenticated KLAP/Tapo
  devices and other broader device families, a production Matter
  commissioning/secure-session/network host, a Thread border-router host, a
  production Zigbee coordinator/join/security host, and production Z-Wave
  inclusion and S2.

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
