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

This slice gives the supervised mDNS pass an actor-owned lifecycle against the
central durable runtime boundary:

- `smart-home-discovery-service` receives the shared
  `smart-home-controller-runtime` owner and keeps only the mDNS executor,
  report adapter, service-health journal, and run-report journal inside its
  actor state.
- Typed tick messages drive due runs sequentially, and the runtime still emits
  exactly the selected-interface IPv4/IPv6 requests through the injectable
  executor boundary.
- Schedule registration and every tick mutate the central runtime through one
  revision-guarded transaction. The central snapshot now retains worker
  cadence and retry pressure, while the service backend retains compact run
  journals and service health.
- Reopening the central owner restores schedule state before another request
  can run. Legacy service-owned schedules import only when absent, so the
  central record wins conflicts and repeated startup does not churn revisions.
- Local-folder restart tests prove successful cadence, named-interface binding,
  failed-run backoff, service counters, durable run audits, stale-owner CAS
  rejection, and idempotent legacy import survive process replacement.

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
host boundaries, durable secret storage, and the recoverable pairing
transaction coordinator:

- `smart-home-hue-pairing-service` restores its D23 runtime from the durable
  runtime store and resolves every pending pairing journal before creating an
  actor that can accept another request.
- Schema-v2 actor requests carry the exact D23 principal and expected durable
  runtime revision. D23 Human Approval is checked before LAN I/O, journal
  creation, or Vault writes and checked again inside the coordinator.
- The production transport executes bounded HTTP/1 over LAN TCP or the shared
  TLS platform. HTTPS remains certificate-verifying and accepts caller-supplied
  Hue trust roots instead of silently disabling verification.
- A pending authorized D23 session drives the canonical `/api` registration
  request. Successful application and client keys remain zeroizing and
  process-local until the coordinator writes a transaction-owned sealed record;
  the journal, runtime, messages, snapshots, and reports receive only an opaque
  `VaultRef` and non-secret metadata.
- Runtime completion is persisted with expected-revision CAS before actor state
  is replaced from the returned durable snapshot. Replaced credentials are
  removed only at the exact revision captured by the journal.
- Real loopback-network and local-folder tests prove delivery, restart recovery
  around interrupted journal acknowledgements, stale-revision rollback,
  replacement cleanup, denial before I/O or writes, cleanup-drift refusal, and
  absence of raw credentials from runtime, journal, event, snapshot, message,
  and report state.

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

These packages complete many Chief of Staff primitives, but they do not yet
complete the production composition boundary. The runnable Chief daemon owns
host lifecycle and the encrypted Level 1 data plane, while the Home
Assistant-compatible controller, discovery services, pairing services, and
Chief smart-home tool bridge still create or restore separate D23 runtime
instances. The production host data plane also has no provider-neutral tool
call operation. Closing D18 therefore requires one durable D23 mutation owner,
migration of supervised smart-home services onto that owner, a thread-safe
Chief tool adapter, authenticated host tool dispatch, and one executable Chief
to D23 end-to-end path.

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

## Current Central Controller Ownership Slice

This slice replaces the local controller's hand-assembled runtime, automation,
storage, and scheduler ownership with one reusable durable coordinator:

- `smart-home-controller-runtime` restores `SmartHomeRuntime`,
  `SmartHomeAutomationRuntime`, and `SmartHomeRuntimeStore` as one authority.
- Serialized transactions clone the live runtime pair, persist the complete
  candidate before publishing it, and preserve the exact prior live state when
  mutation or compare-and-swap persistence fails.
- The coordinator exposes shared runtime handles only as adapter boundaries for
  the existing Home Assistant HTTP surface and future supervised discovery,
  pairing, and Chief tool services.
- The production local controller uses the coordinator for startup restore,
  synchronous HTTP persistence, startup snapshots, and scheduled automation
  evaluation instead of independently assembling those responsibilities.
- Restart, failed-persistence, and concurrent-mutation tests prove one durable
  owner survives process recreation, does not publish rejected candidates, and
  does not lose serialized updates.

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

## Current Production Hue Discovery Composition Slice

This slice runs the existing Hue discovery stack inside the production local
controller without creating another D23 runtime owner:

- `smart-home-local-controller --hue-mdns-interface NAME` explicitly enables a
  Hue DNS-SD worker for one selected local interface; discovery remains off by
  default so controller startup never assumes a network device name.
- The production process installs `smart-home-discovery-service` in an actor
  system with the UDP mDNS executor and Hue report adapter, while every schedule
  registration and run result commits through the same
  `smart-home-controller-runtime` used by HTTP and automations.
- Repeated startup preserves existing cadence and retry pressure when the
  configured worker is unchanged, while an intentional interface change
  replaces its configuration through a serialized central transaction.
- `RuntimeDurableSnapshot` now retains normalized discovery records as well as
  worker schedules, so accepted Hue observations and their bridge candidates
  survive process replacement. Missing discovery fields still deserialize as
  empty for backward compatibility with older snapshots.

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

## Current HEOS Change Event Slice

This slice upgrades the existing read-only HEOS inspection path to local push
without inventing a media command model prematurely:

- A dedicated bounded TCP connection registers through the documented
  `system/register_for_change_events` command before collecting unsolicited
  HEOS JSON frames.
- Player state, volume, mute, progress, repeat, shuffle, queue, topology, and
  playback-error events become normalized D23 device events with stable player
  identity and refresh-required metadata where the protocol sends no payload.
- D23 subscribe authorization is checked before the event socket opens.
  Account usernames are excluded from event metadata.
- A real loopback TCP test proves registration, event framing, parsing, and D23
  runtime application over the production transport.

## Current HEOS Media Control Slice

This slice turns the credential-free HEOS player path into an authorized local
control surface without borrowing command semantics from unrelated domains:

- D23 now has typed media playback, volume, grouping, and queue operations in
  the canonical device-command envelope, with explicit capability mappings and
  low-risk policy tiers.
- HEOS player entities advertise commandable playback, volume, grouping, and
  queue capabilities alongside their detailed observed player state.
- The production TCP host supports play/pause/stop, next/previous, volume,
  mute, group membership, queue clearing, queue playback, removal, and
  reordering with exact command-response correlation.
- Grouping authorizes every affected installed player entity before opening a
  socket. Invalid player references and malformed queue identifiers fail before
  transport I/O.
- A real loopback TCP test proves all ten native command transfers, while a
  denial test proves unauthorized media commands never reach the transport.

## Current AirGradient Local Control Slice

This slice extends the verified local telemetry path through the monitor's
documented `/config` contract without hiding cloud ownership conflicts:

- D23 now has reusable indicator-mode, indicator-brightness,
  display-brightness, and sensor-calibration command types with canonical
  capability mappings. Calibration requires human-approval policy.
- AirGradient inspection reads the current configuration and installs an
  indicator/display control entity plus a calibration capability on the CO2
  sensor.
- Authorized local commands support LED-bar mode, LED-bar brightness, display
  brightness, and the 400 ppm CO2 calibration trigger. Persistent settings are
  read back and verified after each PUT.
- `configurationControl=cloud` fails with an explicit local-control conflict;
  `both` succeeds with an explicit warning that a later cloud update can
  overwrite the value.
- A real loopback HTTP test proves all four native controls, while denial and
  cloud-conflict tests prove no unauthorized or cloud-rejected PUT reaches the
  monitor.

## Current AirGradient Typed Settings Slice

This slice extends the same authorized `/config` host with documented,
non-credential settings while keeping configuration values strongly typed:

- D23 now exposes a reusable `device.configuration` capability for temperature
  unit, PM display standard, automatic CO2 baseline days, gas learning offsets,
  compensated display, indicator self-test, and correction-profile commands.
- AirGradient installs a dedicated configuration entity and accepts only the
  documented `c`/`f`, `ugm3`/`us-aqi`, 0-200 day, and 0-720 hour value ranges.
- Correction commands validate sensor-specific algorithms and require complete,
  finite, positive-scale SLR profiles where the native contract requires them.
- Every persistent setting is read back into confirmed runtime state; LED
  self-test remains a non-persistent trigger. Cloud-only ownership still stops
  before command submission or PUT.
- A real loopback HTTP test proves all seven native setting transfers and
  readbacks, while malformed correction-profile tests stop before transport.

## Current Reolink Recording Control Slice

This slice extends the authenticated Reolink CGI inspection host with one
bounded camera/NVR control that the current transport can verify:

- D23 exposes a reusable boolean `camera.recording` capability and typed
  recording command with a human-approval policy tier.
- Online channels probe `GetRecV20`; only channels that return native recording
  state advertise the commandable capability.
- Authorized `SetRecV20` changes use the existing login-token lifecycle and are
  followed by an exact `GetRecV20` readback before runtime state is confirmed.
- Malformed, unsupported, and unauthorized commands fail before credentials or
  transport I/O. Recording changes remain non-optimistic until device readback.
- A real loopback HTTP test proves inspection, capability detection, denial,
  native recording control, readback verification, logout, and D23 state update.

## Current Reolink PTZ Control Slice

This slice adds bounded physical camera movement without claiming position
state that the portable CGI contract cannot read back:

- D23 exposes a reusable `camera.ptz` capability plus distinct preset-recall
  and bounded-movement commands at the human-approval policy tier.
- Online channels probe `GetPtzPreset`; only channels with a successful native
  response advertise PTZ or accept its commands, and disabled presets are not
  valid recall targets.
- Preset recall validates a probed preset ID and a 1-64 speed before using the
  existing authenticated `PtzCtrl` host.
- Directional movement accepts only left/right/up/down, a 1-64 speed, and a
  duration of at most five seconds, then emits an explicit native `Stop` in the
  same login-token session.
- Invalid, unsupported, and unauthorized requests stop before credentials or
  transport I/O. The runtime does not invent optimistic orientation state.
- A real loopback HTTP test proves probing, preset filtering, denial, recall,
  bounded start/stop movement, logout, and exact native request shapes.

## Current Axis VAPIX Inspection Slice

This slice adds a second vendor-specific camera/NVR runtime using Axis's
documented local discovery and authenticated JSON APIs:

- The shared production mDNS scanner targets `_axis-video._tcp.local` and
  `_axis-nvr._tcp.local`, preserving advertisements as credential-required
  candidates until authenticated identity verifies them.
- Production configuration requires a credential-free HTTPS origin and a
  `VaultRef`; Basic credentials are materialized only inside the bounded TLS
  transport. Plain HTTP is restricted to loopback transport tests.
- The host calls `basicdeviceinfo.cgi` and `apidiscovery.cgi`, rejects VAPIX
  errors, bounds response sizes, and normalizes product identity, firmware, and
  the device's sorted public API inventory.
- D23 authorizes the read before credentials or network transport are touched,
  then installs one confirmed camera entity and a paired Axis bridge without
  exposing secrets in request plans, runtime metadata, or debug output.
- A real loopback HTTP test proves both JSON requests, Basic-auth materialization,
  response parsing, runtime installation, and denial before transport.

## Current Axis VAPIX PTZ Control Slice

This slice extends the authenticated Axis host with capability-probed physical
camera control while preserving the device's native arbitration boundary:

- Devices advertising `ptz-control` are inspected through documented `info=1`,
  `query=position`, `query=presetposall`, and `CtlQueueing` requests for VAPIX
  camera 1. Confirmed pan, tilt, zoom, and enabled server presets become runtime
  state without implying broader channel coverage.
- `camera.ptz` is installed only when native speed control plus continuous
  pan/tilt or at least one server preset are proven. Unknown queue settings fail
  closed instead of silently bypassing device arbitration.
- Preset recall accepts only probed IDs and a 1-100 native speed. Directional
  movement accepts left/right/up/down, a 1-100 speed, and at most five seconds,
  then emits an explicit `continuouspantiltmove=0,0` stop.
- D23 applies the existing human-approval command tier before credentials or
  transport I/O. Commands acquire and release `ptzqueue.cgi` control when
  required, while the queue cookie exists only inside the bounded transport.
- Commands do not invent optimistic orientation after movement. A real loopback
  test proves probing, denial and argument rejection before I/O, queue-cookie
  isolation, preset recall, bounded start/stop movement, and queue release.

## Current Blue Iris Inspection and Control Slice

This slice adds an authenticated local NVR host and the bounded camera controls
that its documented JSON interface can verify:

- Explicit local HTTPS configuration and Vault-backed credentials feed Blue
  Iris's `/json` challenge-response login. Credentials, hashes, sessions, and
  the returned license value remain transport-private, and each operation uses
  a fresh session with explicit logout.
- Authorized `camlist` inspection installs confirmed camera health, activity,
  recording, and PTZ-support state. Plain HTTP remains loopback-test-only.
- A session granting `clipcreate` exposes the existing typed
  `camera.recording` command. Manual recording changes remain unconfirmed until
  exact `camlist.isManRec` readback matches the request.
- A session granting `ptz` plus a PTZ-capable camera exposes preset recall for
  IDs 1-20 and left/right/up/down movement with 1-100 speed and at most five
  seconds of duration. Movement ends with native Stop, including a best-effort
  stop after an ambiguous start response.
- D23 human approval occurs before control transport I/O. Runtime and real
  loopback tests prove denied and malformed requests stop before transport,
  recording readback, preset recall, bounded movement, Stop, and logout.

## Current Frigate Inspection Slice

This slice adds a first-party authenticated local Frigate NVR health host while
keeping session and media boundaries explicit:

- Explicit local HTTPS configuration and Vault-backed credentials feed the
  documented `/api/login` flow. The JWT cookie exists only inside zeroizing
  transport memory, never enters request plans or normalized state, and each
  successful inspection ends at `/api/logout`.
- D23 authorizes `smart_home.read` before credentials or network I/O. The host
  then reads only `/api/version` and role-filtered `/api/stats`; it deliberately
  avoids the broader configuration response even though Frigate redacts known
  credential fields there.
- Confirmed camera entities expose processing and detection FPS, detection
  enablement, native connection quality, expected FPS, and recent reconnect and
  stall counts. Native unusable/stopped processing maps offline, while poor or
  recently unstable connections map degraded.
- Production requires Frigate's authenticated HTTPS origin. Plain HTTP remains
  loopback-test-only, and an exact protocol test proves login-body isolation,
  cookie-only authenticated reads, bounded parsing, and redirecting logout.

## Current Frigate Snapshot Slice

This slice composes the installed Frigate camera identity with the camera-media
authorization and lease boundary without persisting a JWT or reusable secret:

- Exact D23 Human Approval runs before Vault resolution or transport I/O. The
  host then revalidates the installed Frigate bridge, credential reference,
  native camera-name identifier, snapshot capability, and reviewed pinned
  address before registering one process-local endpoint.
- One bounded Vault credential envelope is installed only in the dedicated
  executor for one delivery. The executor logs in at `/api/login`, sends the
  validated JWT cookie only to the same-origin `/api/{camera}/latest.jpg`,
  accepts one bounded JPEG, and explicitly calls `/api/logout` after every
  authenticated success or failure.
- Credentials, cookies, and endpoint state are zeroized or removed after the
  delivery attempt. Strict HTTPS uses the reviewed address with the canonical
  host retained for certificate validation; HTTP remains loopback-test-only.
- Tests prove denial before Vault, exact-target and malformed-envelope refusal,
  repeated sealed-Vault deliveries, cleanup after executor failure, percent-
  encoded camera names, pinned TLS, cookie-only image authentication, and
  logout after both successful and invalid-image outcomes.

## Current Synology Surveillance Station Inspection Slice

This slice adds a first-party authenticated local Synology NVR health host while
preserving the device's advertised Web API boundary:

- Explicit local HTTPS configuration and Vault-backed credentials first query
  `SYNO.API.Info`. Advertised paths are constrained to the same `/webapi`
  origin, and authentication, package-info, and camera APIs must expose the
  versions needed by this host before any login begins.
- D23 authorizes `smart_home.read` before credentials or transport I/O. The host
  opens an isolated SID-format `SurveillanceStation` session with SynoToken
  support, reads only package information and a bounded, privilege-filtered
  camera list, and explicitly logs out.
- Credentials, login bodies, SID values, and SynoToken values stay in zeroizing
  transport memory and never enter request plans, normalized state, metadata,
  or debug output. OTP and remembered-device flows are intentionally outside
  this non-interactive username/password slice.
- Confirmed camera entities expose the documented native status, channel,
  vendor, and model. Normal and ready states map online, transitional states
  map degraded, and connection, authorization, stream, storage, disabled, or
  missing-video states map offline.
- Production requires an authenticated HTTPS origin. Plain HTTP remains
  loopback-test-only, and an exact protocol test proves API discovery, login
  payload isolation, role-filtered reads, session-token confinement, and
  logout.

## Current UniFi Network Inspection Slice

This slice promotes the cataloged UniFi Network integration to a first-party
local health host over Ubiquiti's official integration API:

- Explicit local UniFi OS HTTPS configuration and a Vault-backed API key target
  the fixed /proxy/network/integration/v1 boundary. Plain HTTP is accepted only
  for loopback protocol tests.
- D23 authorizes smart_home.read before transport I/O. The host reads only
  application information, bounded paginated local sites, and bounded paginated
  adopted-device summaries.
- The API key remains in zeroizing transport memory and is materialized as
  X-API-Key only while encoding each request. Request plans retain only the
  Vault reference and normalized state never contains the key.
- Confirmed network-diagnostic entities expose the documented site, device,
  model, MAC, IP, features, and native state. Online maps online; update,
  readiness, and adoption states map degraded; offline, interrupted, isolated,
  and deleting states map offline.
- A real loopback test proves the exact application, site, and per-site device
  requests, bounded pagination, API-key header materialization, and the absence
  of Vault references from wire traffic.

## Current UniFi Connected-Client Presence Slice

This slice closes the safely governable current-presence portion of Ubiquiti's
official connected-client API without retaining native client identity:

- The local authenticated host adds bounded pagination over the documented
  `GET /v1/sites/{siteId}/clients` endpoint after application, site, and adopted
  device inspection.
- D23 read authorization, an exact ephemeral device-identifier grant, and a
  separate exact five-minute presence grant all succeed before credentials or
  network I/O. Both grants bind the principal, bridge client resource, local
  destination, declared purpose, consent receipt, retention, and validity
  window.
- A distinct Vault-leased 32-byte key derives domain-separated 128-bit
  host-scoped client pseudonyms. Native client IDs, names, MAC addresses, IP
  addresses, and connection timestamps live only in zeroizing response storage
  and never enter runtime identity, state, metadata, errors, or debug output.
- Confirmed client entities expose only pseudonymous current presence,
  connection type, and optional access type/authorization. Their state expires
  five minutes after observation instead of making an indefinite presence
  claim.
- Real loopback coverage proves the exact client path and API-key header,
  consent denial before transport, raw-field exclusion, and bounded state
  expiry.

## Current UniFi Live Device Statistics Slice

This slice closes bounded live health metrics for explicitly selected adopted
devices over Ubiquiti's official local latest-statistics endpoint:

- The authenticated local host calls only
  `GET /v1/sites/{siteId}/devices/{deviceId}/statistics/latest` for an explicit
  set of already-installed devices. Site/device path segments, target
  uniqueness, response count, and target correspondence are validated before
  runtime mutation.
- D23 read authorization and an exact local operational-telemetry grant with
  two-minute retention both succeed before credentials or network I/O. The
  reusable governance contract remains deny-by-default for retention
  near-matches.
- Each poll is capped at 64 unique targets and an integration instance refuses
  another statistics poll for one minute. Rate-limit, authorization, target,
  and consent failures all occur before statistics transport I/O.
- Confirmed network-diagnostic statistics expose bounded CPU and memory
  utilization, load averages, uptime, optional uplink rates, and at most 64
  radio frequency/retry readings. State expires after two minutes; native last
  and next heartbeat timestamps are deliberately discarded.
- Real loopback coverage proves the exact single-device path and API-key
  header. Focused tests prove numeric/radio bounds, exact consent, installed-
  target enforcement, expiration, and pre-I/O rate limiting.

## Current Enphase IQ Gateway Inspection Slice

This slice promotes the cataloged Enphase Envoy entry to an authenticated local
energy-telemetry host using Enphase's documented IQ Gateway API:

- Explicit local HTTPS configuration, a known gateway serial, and a Vault-backed
  pre-generated access token target only `/ivp/meters` and
  `/ivp/meters/readings`. Plain HTTP remains loopback-test-only.
- D23 authorizes `smart_home.read` before credentials or network I/O. The
  bearer token remains in zeroizing transport memory and is materialized only
  while encoding the `Authorization` header.
- Meter inventory and readings are bounded, duplicate native EIDs are rejected,
  and every reading must match exactly one advertised meter before runtime
  state can change.
- Confirmed sensor entities expose aggregate delivered and received energy,
  demand, active/apparent/reactive power, power factor, voltage, current,
  frequency, phase shape, and native meter health. Disabled meters map offline;
  non-normal status or status flags map degraded.
- Production TLS remains certificate-verifying and supports caller-supplied
  trust roots for gateway certificates. A real loopback test proves both exact
  bearer-authenticated requests and that Vault references never reach the wire.

## Current Enphase Per-Inverter Production Slice

This slice closes the documented, safely governable per-microinverter read path
without retaining Enphase's native serial numbers:

- The production host adds the documented authenticated
  `GET /api/v1/production/inverters` request alongside meter inventory and
  readings. Inverter arrays are bounded, native serials must be decimal and
  unique, and power fields are finite and non-negative.
- D23 read authorization and an exact host-owned device-identifier grant both
  succeed before credentials or network I/O. The grant binds the principal,
  gateway inverter resource, inspection operation, local destination, declared
  purpose, consent receipt, validity window, and ephemeral raw-identifier
  retention.
- A separate Vault-leased 32-byte key derives domain-separated 128-bit
  host-scoped pseudonyms. Raw microinverter serials live only in a zeroizing
  response tree and never enter runtime entity IDs, names, metadata, state,
  errors, request plans, or debug output.
- Confirmed inverter sensors expose only pseudonymous identity, device type,
  last-report time, last reported active power, and maximum reported active
  power. Aggregate meter behavior remains unchanged.
- Real loopback coverage proves the exact three bearer-authenticated requests,
  consent denial before transport, stable key-scoped identity, and the absence
  of raw inverter serials from requests and installed entities.

## Current ZoneMinder Inspection Slice

This slice adds a first-party authenticated local ZoneMinder NVR host using the
documented API 2.0 contract:

- Explicit local HTTPS origin or path-prefix configuration and Vault-backed
  username/password credentials target only `/api/host/login.json`,
  `/api/host/getVersion.json`, and `/api/monitors.json`. Plain HTTP remains
  loopback-test-only.
- D23 authorizes `smart_home.read` before credentials or network I/O. Login is
  form encoded, API version 2.0 is required, and the short-lived access JWT is
  kept in zeroizing transport memory while token-bearing GET targets are built.
  Refresh tokens and token-bearing URLs do not enter request plans, normalized
  state, or debug output.
- Monitor arrays are bounded, native IDs must be positive and unique, and only
  documented monitor configuration and `Monitor_Status` health fields are
  normalized.
- Confirmed camera entities expose enablement, capture, analysis, recording,
  native status, capture/analysis FPS, and capture bandwidth. Disabled,
  non-capturing, stopped, or no-signal monitors map offline; unknown or
  zero-capture states map degraded.
- Production TLS remains certificate-verifying. A real loopback test proves the
  exact path-prefixed login, version, and monitor requests plus transport-private
  credential and JWT handling.

## Current Axis HTTP Digest Authentication Slice

This slice closes the reusable HTTP Digest prerequisite and wires it into the
existing Axis VAPIX production transport:

- `http-digest-auth` parses bounded RFC 7616 challenges and builds zeroizing
  MD5, MD5-sess, SHA-256, and SHA-256-sess authorization values for `qop=auth`
  or the legacy no-`qop` form.
- Duplicate, oversized, malformed, unsupported-algorithm, `auth-int`-only,
  unsupported-charset, `userhash=true`, and header-injection inputs fail closed.
- The Axis transport now follows the documented unauthenticated-request then
  `401 WWW-Authenticate` flow, prefers supported SHA-256 Digest over MD5 and
  Basic, and keeps the selected challenge and nonce count only in transport
  memory.
- Digest client nonces come from the OS CSPRNG. Credentials, A1 material,
  derived responses, Basic values, Digest values, and encoded request bytes are
  zeroized and never enter request plans, debug output, or normalized state.
- Authentication is retried at most once per request. A real loopback exchange
  proves the exact unauthenticated probe, authenticated retry, preemptive nonce
  count, and `stale=true` nonce refresh while production TLS remains
  certificate-verifying.

## Current AirGradient Data Governance Slice

This slice closes the privacy, telemetry-egress, and operator-consent
prerequisite for AirGradient's documented country and vendor-cloud controls:

- `smart-home-data-governance` adds a bounded, deny-by-default host policy that
  binds one authenticated principal and governed resource to a data category,
  operation, exact destination, declared purpose, consent receipt, and validity
  window. Model-facing command arguments cannot create or widen grants.
- D23 adds human-approved country and cloud-upload command types. The runtime
  keeps both non-optimistic until native device readback.
- AirGradient accepts only assigned ISO 3166 alpha-2 country codes and requires
  an exact coarse-location configuration grant before any transport I/O.
  Normalized state records only that a country is configured, never its value.
- Enabling upload requires a separate environmental-telemetry egress grant
  bound to `https://api.airgradient.com`. Disabling upload is
  privacy-protective and does not require a consent grant, though D23 still
  requires human approval for the mutation.
- Both persistent controls use the documented local `PUT /config` contract and
  exact `GET /config` readback. Real loopback tests prove wire shapes,
  authorization ordering, consent denial before I/O, and verified shutdown.
- Local HTTP request-plan debug output now reports only body length, preventing
  country or future configuration bodies from leaking through derived debug.

## Current AirGradient Custom Egress Slice

This slice closes the safely executable part of AirGradient's documented MQTT
broker and custom HTTP routing surface:

- D23 adds human-approved `device_set_mqtt_broker` and
  `device_set_http_domain` contracts. The runtime keeps both non-optimistic
  until exact native readback.
- MQTT command values accept only credential-free `mqtt://` or `mqtts://`
  broker URIs with an explicit port. Custom HTTP routing accepts only a fully
  qualified DNS name and is governed as its exact HTTPS origin.
- Enabling either route requires an active environmental-telemetry grant bound
  to the exact principal, configuration entity, operation, and destination
  before any device I/O. Disabling an existing route remains
  privacy-protective without a consent grant.
- The production local HTTP transport performs the documented `PUT /config`
  mutation and exact `GET /config` readback. Tests prove consent denial before
  transport, while real loopback tests prove wire shapes and verified
  enable/disable flows.
- Normalized state exposes only whether MQTT and custom HTTP routing are
  configured. Destination identities and pre-existing MQTT userinfo are
  redacted from debug and never enter normalized state.
- AirGradient's `httpDomain` applies to telemetry, remote configuration, and OTA
  together; this slice treats the coupled HTTPS origin as one governed route.
- Credential-bearing MQTT remains blocked: current upstream firmware logs the
  parsed username and password, so host-side Vault leasing cannot prevent the
  device-side disclosure. Command input rejects MQTT userinfo.

## Current Retained Identity Migration Slice

This slice closes the shared runtime prerequisite for safe pseudonym-key
rotation without starting either vendor-specific rotation workflow:

- `smart-home-runtime` accepts host-only whole-device identity replacements
  that cover every retained child entity exactly once. It rejects missing
  children, destination collisions, no-op identities, bridge changes, and
  capability-shape drift before mutating runtime state.
- A successful migration atomically rewrites device/entity topology, embedded
  and current state, scenes, registry and runtime event history,
  entity-scoped grants, command authorization decisions, optimistic and
  desired state, live entity subscription filters, and queued deliveries.
- Replacement devices and entities carry the destination pseudonym metadata;
  opaque metadata is never heuristically edited. The runtime preserves the
  source state and history under the supplied destination identities.
- `smart-home-runtime-store` builds the migrated runtime as a candidate,
  persists its complete durable snapshot with an expected-revision
  compare-and-swap, and replaces the caller's live runtime only after storage
  succeeds. A stale revision leaves both live and durable state unchanged.
  Supplied automation definitions and execution state must already use the
  destination identities; exact source-ID references fail before mutation.
- Enphase and bounded UniFi connected-client key rotation now use this path in
  the completed slices below. Multi-site or paginated UniFi rotation remains
  prerequisite-gated because one-shot keys cannot safely span a partially
  persisted multi-response operation.

## Current Enphase Identifier-Key Rotation Slice

This slice closes Enphase pseudonym-key rotation over the shared retained
identity migration path:

- `vault-leases` can atomically consume a distinct batch of one-shot leases,
  validating every lease before removing any payload so a failed batch leaves
  the complete key set untouched.
- The Enphase host authorizes the D23 read and exact ephemeral
  device-identifier grant before consuming either key or reaching transport.
  Source and destination leases must be distinct and each payload must contain
  exactly 32 bytes.
- One bounded authenticated `/api/v1/production/inverters` response derives
  exact source/destination pseudonym pairs under both keys. Raw serials remain
  in the zeroizing response tree, and both payloads and key objects are dropped
  before runtime migration begins.
- The response must correspond exactly to the currently installed inverter
  pseudonyms. Rotation constructs one complete gateway replacement, including
  deterministic destination identities for every meter and inverter child,
  while preserving capability and retained-state shape.
- `smart-home-runtime-store` persists the replacement with the caller's
  expected revision and swaps live state only after durable success. Opaque
  automation definitions and state must already use destination identities or
  prove absence of source references through the store's fail-closed scan.
- Real loopback coverage proves one bearer-authenticated inverter request,
  atomic two-key consumption, full live and restart identity replacement, and
  exclusion of native inverter serials from runtime debug state. Consent denial
  occurs before either lease or transport is touched.

## Current UniFi Connected-Client Identifier-Key Rotation Slice

This slice closes bounded UniFi connected-client pseudonym-key rotation over
the shared retained-identity migration path:

- The host supplies one explicit safe site ID and two distinct one-shot
  32-byte key leases. D23 read authorization and the exact ephemeral
  device-identifier plus five-minute presence grants succeed before either
  lease is consumed or transport is reached.
- Both leases are consumed atomically. Exactly one authenticated first-page
  client response must be non-empty, complete, and contain no more than 100
  clients; hidden pagination fails closed.
- The zeroizing native response derives exact source/destination pseudonym
  correspondence under both keys. Raw client IDs and both key objects are
  disposed before retained-identity migration begins.
- The response must correspond exactly to the installed pseudonymous client
  set. Every replacement preserves its existing bounded presence state and
  expiry while changing the complete client device/entity identity pair.
- `smart-home-runtime-store` persists the replacements with the caller's
  expected revision and swaps live state only after durable success. Opaque
  automation definitions and state must already use destination identities or
  prove absence of source references.
- Real loopback coverage proves one API-key-authenticated client request,
  atomic two-key consumption, live and restart identity replacement, and raw
  identifier exclusion. Consent denial occurs before leases or transport.

## Current Camera Media HTTPS Snapshot Executor Slice

This slice supplies the concrete native snapshot transport behind the existing
camera-media lease boundary without moving transport authority into the policy
broker:

- `smart-home-camera-media-http-executor` accepts endpoints only during trusted
  lease redemption and requires the broker-retained canonical host plus pinned
  socket. It connects directly to the reviewed address while preserving that
  host for HTTP `Host`, TLS SNI, and forced strict certificate verification.
- Production delivery is HTTPS-only. A separate explicit loopback fixture mode
  is the only plaintext path; redirects, user information, fragments, origin
  mismatches, unpinned destinations, and non-snapshot media fail before useful
  bytes are released.
- Optional Basic and RFC 7616 Digest credentials remain zeroizing and
  process-local by camera entity. Delivery probes without credentials, prefers
  advertised SHA-256 Digest over MD5 and Basic, uses CSPRNG client nonces, and
  permits one bounded refreshed Digest challenge retry.
- HTTP response heads, wire bytes, content framing, and decoded payloads are
  bounded. Ambiguous length/transfer framing, content encoding, redirects,
  non-image media types, and JPEG/PNG/WebP signature mismatches fail closed.
- Real loopback coverage proves one public Human Approval-backed camera-media
  lease reaches exactly one pinned snapshot request and returns bounded
  zeroizing image bytes without exposing the endpoint through public delivery,
  audit, error, or debug surfaces.
- Supervised streams, recording downloads, exports, and playback remain a
  separate resource-lifecycle prerequisite; this executor deliberately rejects
  stream delivery rather than inventing teardown or retention semantics.

## Current ONVIF HTTPS Snapshot Host Slice

This slice composes the installed ONVIF camera topology with the completed
camera-media policy and native HTTPS executor in one concrete production host:

- `smart-home-onvif-snapshot-host` performs the current authenticated
  principal's exact Human Approval check before resolving credentials or
  reaching network I/O. Invalid requests and missing process-local snapshot
  endpoints fail at the same pre-I/O boundary.
- The host implements the existing camera endpoint registry, so ONVIF profile
  installation registers its reviewed canonical snapshot URI and pinned socket
  directly into the host without exposing either through durable runtime state.
- An installed bridge retains only an opaque stable Vault reference. The
  production credential source reads its bounded, versioned credential envelope
  from the sealed store into zeroizing memory for one authorized delivery.
- Basic or Digest credentials are registered for exactly the selected entity,
  one short-lived camera-media lease is issued and redeemed, and credentials are
  removed on every success or error return path. Duplicate registration fails
  closed instead of replacing another active host-owned credential lifetime.
- The production constructor supplies trusted system time, OS CSPRNG lease
  nonces, and the strict pinned HTTPS executor. Tests cover denial before Vault
  resolution, removal after failed delivery, repeated independently authorized
  reads from one sealed record, and a real Basic-authenticated loopback JPEG.
- Automated ONVIF credential provisioning remains a pairing responsibility; a
  host must write the versioned envelope into the dedicated sealed-Vault
  namespace before installing its opaque reference. RTSP and other streams
  remain blocked on a supervised resource owner and teardown lifecycle.

## Current ZoneMinder HTTPS Snapshot Host Slice

This slice composes installed ZoneMinder monitor identities with the completed
camera-media policy and pinned native HTTPS executor:

- `smart-home-zoneminder-snapshot-host` performs exact Human Approval preflight
  before resolving credentials, logging in, registering an endpoint, or
  reaching media I/O. The target must be the exact camera entity derived from a
  positive installed ZoneMinder monitor identifier.
- The configured credential-free `nph-zms` endpoint must share the installed
  bridge's HTTPS origin and retain a reviewed canonical host plus pinned socket.
  The host requests only documented `mode=single` delivery at fixed scale 100;
  it does not open the MJPEG streaming mode.
- One bounded, versioned sealed-Vault credential envelope is materialized for
  each approved operation. The existing API 2.0 transport obtains one
  short-lived access token, then credentials and the login response are
  disposed before media delivery.
- The token-bearing snapshot URI exists only in zeroizing process-local broker
  state. It is removed after success, lease failure, or executor failure and
  never enters normalized runtime state, errors, audit records, or debug output.
- Tests cover denial before Vault/login, malformed payload redaction, exact
  target and endpoint validation, one token per operation, cleanup after failed
  delivery, repeated sealed-record use, and strict native HTTPS login-to-JPEG
  composition.
- Automated credential provisioning remains an explicit pairing-flow
  responsibility. Token reuse and refresh remain blocked on a supervised
  session lifecycle and refresh-token Vault policy; streams, recordings,
  exports, and playback remain blocked on supervised resource ownership.

## Current Synology Surveillance Station Snapshot Host Slice

This slice composes installed Synology camera identities with the completed
camera-media policy and pinned native HTTPS executor:

- `smart-home-synology-snapshot-host` performs exact Human Approval preflight
  before Vault access, session setup, endpoint registration, or media I/O. The
  target must be the exact camera entity and native camera identifier installed
  by the authenticated Synology inspection host.
- One bounded, versioned sealed-Vault credential envelope opens one isolated
  `SurveillanceStation` SID/SynoToken session. The host repeats API discovery,
  confirms `allowSnapshot`, and proves the requested camera is still present in
  the bounded privilege-filtered list.
- The documented version-9 `GetSnapshot` request uses only the exact camera id
  and fixed high-quality profile. Its token-bearing URI exists only in
  zeroizing process-local state and is registered against the reviewed
  canonical host and pinned socket for one bounded JPEG delivery.
- Endpoint removal and explicit logout are attempted after success, lease
  failure, executor failure, and endpoint-registration failure. Session setup
  failures after login also attempt logout before returning.
- Tests cover denial before Vault/session I/O, permission-bound capability
  projection, malformed credential redaction, repeated sealed-record use,
  endpoint and logout failure paths, setup-failure logout, exact camera
  correspondence, strict native TLS requests, pinned JPEG delivery, and final
  endpoint removal.
- Automated credential provisioning remains an explicit pairing-flow
  responsibility. Reusable sessions, OTP and remembered-device authentication,
  events, streams, recordings, export, playback, PTZ, and configuration
  mutations remain prerequisite-gated.

## Current Axis VAPIX Snapshot Host Slice

This slice composes the installed Axis camera-1 identity with the completed
camera-media policy and pinned native HTTPS executor:

- Authenticated Axis inspection now probes the bounded VAPIX image parameter
  inventory and advertises `camera.snapshot` only when camera 1 is enabled,
  VAPIX HTTP version 3 is present, and JPEG is supported.
- `smart-home-axis-snapshot-host` performs exact Human Approval preflight
  before Vault access, endpoint registration, credential registration, or
  media I/O. The target must be the exact camera entity installed by the
  reviewed Axis bridge and carry the probed camera-1 metadata and capability.
- One bounded, versioned sealed-Vault credential envelope is decoded into
  zeroizing process-local state. The host registers the documented
  `/axis-cgi/jpg/image.cgi?camera=1` endpoint against the reviewed canonical
  host and pinned socket, then registers only temporary Basic or Digest
  credentials for one bounded delivery.
- Credentials and the endpoint are removed after success, lease failure,
  executor failure, and registration failure. Neither credential material nor
  the authenticated endpoint enters runtime state, errors, audit records, or
  debug output.
- Tests cover denial before Vault, exact installed-target correspondence,
  malformed credential redaction, repeated sealed-record use, cleanup after
  failure, and a strict pinned-TLS Basic challenge-to-JPEG exchange.
- Automated credential provisioning remains an explicit pairing-flow
  responsibility. Event streams, source enumeration, recordings, playback,
  exports, and broader media transfer remain prerequisite-gated.

## Current Reolink HTTPS Snapshot Host Slice

This slice composes the exact installed Reolink physical-channel identity with
the completed camera-media policy and pinned native HTTPS executor:

- Authenticated Reolink inspection advertises `camera.snapshot` only for an
  awake, online `RLC-*` channel. NVR, battery-camera, logical-channel, and
  unsupported-family snapshot claims remain excluded.
- `smart-home-reolink-snapshot-host` performs exact Human Approval preflight
  before Vault access, endpoint registration, or media I/O. The target must be
  the exact camera entity installed by the reviewed bridge, remain online and
  awake, and retain its exact physical-channel metadata and capability.
- One bounded, versioned sealed-Vault credential envelope is decoded into
  zeroizing process-local state. The host percent-encodes the credentials into
  the documented `/cgi-bin/api.cgi?cmd=Snap` HTTPS request and registers that
  endpoint only against the reviewed canonical host and pinned socket.
- The credential envelope is disposed before delivery and the complete
  token-bearing endpoint is removed after success, lease failure, executor
  failure, or registration failure. It never enters normalized state, errors,
  audit records, or debug output.
- Tests cover denial before Vault, exact installed-target correspondence,
  malformed credential redaction, repeated sealed-record use, cleanup after
  failure, and a strict pinned-TLS JPEG request with percent-encoded query
  credentials.
- Automated credential provisioning remains an explicit pairing-flow
  responsibility. Streams, recordings, playback, NVR channels, logical
  channels, and broader media transfer remain prerequisite-gated.

## Current Revision-Guarded Pairing Completion Slice

This slice closes the durable-runtime half of credential pairing without
claiming cross-store atomicity:

- `smart-home-runtime-store` accepts an already sealed opaque `VaultRef`, a D23
  principal, one exact pending pairing completion, and the caller's expected
  runtime revision.
- Completion runs against a cloned runtime through the existing
  `smart_home.complete_pairing` Human Approval authorization path. The complete
  candidate snapshot is persisted with compare-and-swap before live state is
  replaced.
- Authorization denial, invalid pairing state, encoding failure, or a stale
  storage revision leaves the live pairing session and bridge reference
  unchanged. Successful restart recovery restores the completed session,
  bridge reference, authorization decision, health event, and metadata.
- The API returns the previous opaque bridge `VaultRef` after commit so a
  credential host can distinguish first installation from replacement without
  reading credential material.
- Tests prove authorized replacement and restart recovery, denial without a
  durable candidate, and stale-revision rollback with both live and durable
  pairing state still pending.
- Sealed-Vault write/rollback and old-record deletion are deliberately not
  presented as atomic with runtime persistence. A recoverable transaction
  journal remains required before automated credential provisioning can ship.

## Current Recoverable Pairing Transaction Slice

This slice closes the reusable cross-store recovery prerequisite without yet
claiming that an existing vendor pairing actor has adopted it:

- `smart-home-pairing-transaction` writes a secret-free durable journal before
  credential creation and exposes stable paginated discovery so a restart host
  can enumerate every pending transaction.
- Fresh credentials use transaction-owned opaque references and Vault keys
  with sealed-Vault `put_if_absent`; generated address collisions cannot
  overwrite an existing record. Journal records retain only D23 principal and
  pairing identity, opaque Vault references, exact Vault and runtime revisions,
  timestamps, metadata, and transaction state.
- D23 `CompletePairing` authorization is proven against a cloned durable
  runtime before journal or Vault writes. Runtime completion then uses the
  expected-revision persist-before-swap path.
- Recovery distinguishes prepared, Vault-written, runtime-committed, and
  cleanup-complete states. It detects a Vault write or runtime commit whose
  following journal acknowledgement was interrupted.
- An uncommitted credential is deleted only at the exact revision returned by
  its original Vault creation. After runtime commit, the previous credential is
  deleted only at the exact revision captured during preparation; replacement
  drift or a partial runtime reference leaves the journal pending instead of
  deleting a newer or potentially referenced record.
- Tests prove first-install and replacement commit, no credential bytes in the
  journal, restart discovery, recovery on both sides of runtime CAS, stale
  runtime rollback, cleanup revision-conflict and partial-reference retention,
  and authorization denial before any journal or Vault write.
- The Hue pairing actor now uses this coordinator as the first production
  composition. Other vendor credential-provisioning paths must reuse this
  durable protocol and prove their own exact pairing and secret-input boundary.

## Current ONVIF Credential Pairing Slice

This slice makes ONVIF the first externally supplied camera credential to use
the completed pairing transaction composition:

- `smart-home-onvif-pairing-service` accepts only the exact pending pairing
  session, D23 principal, expected durable runtime revision, and completion
  time. Usernames, passwords, secret paths, and Vault references never enter
  actor messages.
- The production input is configured by its host for one exact bridge and reads
  each credential once through the race-resistant, owner-only, exact-length
  secret-file boundary. Returned bytes and parsed strings remain zeroizing and
  the input refuses reuse or another bridge.
- Human Approval succeeds before either secret file is opened, network I/O is
  attempted, or durable state is written. The pending session must name an
  ONVIF bridge with one exact WS-Discovery endpoint-reference identifier and a
  stored device-service address.
- The native verifier re-reviews that address, pins its resolved LAN socket,
  requires certificate-verifying HTTPS, and completes authenticated ONVIF
  device and media inspection before the credential envelope can be sealed.
- The service writes the same bounded versioned envelope consumed by the
  read-only ONVIF snapshot host, but only at a transaction-owned opaque
  reference. Runtime completion uses expected-revision CAS, startup resolves
  every pending journal, successful commits replace actor state from the
  durable snapshot, and replacement cleanup remains bound to the captured
  Vault revision.
- Tests prove successful envelope installation without durable secret
  exposure, denial and stale-revision failure before secret input, one-shot
  owner-only file handling, secret-free actor messages, interrupted-commit
  restart recovery, and cleanup-drift refusal.
- Snapshot delivery remains strictly read-only and cannot implicitly create or
  replace credentials. Axis, Reolink, and Synology must each prove
  their own exact pairing identity and input lifetime before reusing this
  composition.

## Current ZoneMinder Credential Pairing Slice

This slice extends the recoverable camera credential composition to one exact
ZoneMinder NVR bridge without persisting its API session tokens:

- `smart-home-zoneminder-pairing-service` accepts only the exact pending
  pairing session, D23 principal, expected durable runtime revision, and
  completion time. The actor message cannot carry credential paths, bytes,
  tokens, or Vault references.
- One host-owned input is bridge-bound, exact-length, owner-only, one-shot, and
  zeroizing. Human Approval succeeds before it opens either credential file,
  reaches LAN I/O, or creates a transaction journal or Vault record.
- Pairing requires one vendor-protocol `https_endpoint` identifier equal to the
  bridge's strict HTTPS address. Authenticated API 2.0 login, version, and
  monitor inspection complete before any credential write; a non-empty
  observed monitor set must exactly equal every already installed positive
  `monitor_id`.
- Only the snapshot host's bounded versioned username/password envelope enters
  the transaction-owned Vault record. Access and refresh tokens stay inside
  the bounded verifier and never enter actor state, runtime state, journals,
  reports, or the pairing envelope.
- Expected-revision runtime CAS, startup recovery, durable actor-state
  replacement, and revision-bound prior-record cleanup reuse
  `smart-home-pairing-transaction` unchanged. Snapshot delivery remains a
  read-only consumer.
- Tests cover denial and stale revision before input, exact bridge and monitor
  identity, owner-only one-shot files, a real API 2.0 login-to-monitor loopback,
  secret-free durable state, interrupted-commit restart recovery, exact
  replacement cleanup, and cleanup-drift refusal.

## Current Axis Credential Pairing Slice

This slice extends recoverable camera credential provisioning to one exact
Axis VAPIX camera without weakening snapshot delivery into a credential writer:

- `smart-home-axis-pairing-service` accepts only the exact pending pairing
  session, D23 principal, expected durable runtime revision, and completion
  time. Actor messages cannot carry credential paths, bytes, or Vault
  references.
- One host-owned username/password input is bridge-bound, exact-length,
  owner-only, one-shot, and zeroizing. Human Approval succeeds before either
  file is opened, LAN I/O begins, or a transaction journal or Vault record is
  created.
- The pending bridge must expose exactly one reviewed `https_endpoint`
  identifier equal to its credential-free address. Authenticated VAPIX
  inspection then proves a non-empty stable serial plus enabled camera 1,
  VAPIX HTTP version 3, and JPEG support. When a camera is already installed,
  its exact serial, sole camera entity, channel metadata, and snapshot
  capability must correspond.
- Verification reuses the production Basic/SHA-256 Digest transport and only
  the snapshot host's versioned username/password envelope enters the
  transaction-owned Vault record. Axis credential fields move into zeroizing
  storage before validation, including rejection paths.
- Expected-revision runtime CAS, startup recovery, durable actor-state
  replacement, and revision-bound prior-record cleanup reuse
  `smart-home-pairing-transaction` unchanged. Snapshot delivery remains a
  read-only consumer.
- Tests cover denial and stale revision before input, exact HTTPS and installed
  camera identity, one-shot owner-only files, a real authenticated VAPIX
  camera-1 loopback, secret-free durable state, interrupted-commit recovery,
  exact replacement cleanup, and cleanup-drift refusal.

## Current Reolink Credential Pairing Slice

This slice extends recoverable camera credential provisioning to one exact
direct Reolink RLC camera without turning snapshot delivery into a writer:

- `smart-home-reolink-pairing-service` accepts only the exact pending pairing
  session, D23 principal, expected durable runtime revision, and completion
  time. Actor messages cannot carry credential paths, bytes, session tokens, or
  Vault references.
- One host-owned username/password input is bridge-bound, exact-length,
  owner-only, one-shot, and zeroizing. Human Approval succeeds before either
  file is opened, LAN I/O begins, or a transaction journal or Vault record is
  created.
- The pending bridge's exact credential-free address must match a host-owned
  canonical hostname and reviewed socket address. Production inspection keeps
  the hostname for SNI and strict certificate verification while connecting
  only to the reviewed address.
- Authenticated CGI inspection proves a non-empty stable serial, exact equality
  with every already installed physical channel, and at least one awake online
  snapshot-capable channel on an `RLC-*` camera. Session query tokens remain
  process-local and explicit logout runs on every successful or failed
  inspection path.
- Only the snapshot host's bounded versioned username/password envelope enters
  the transaction-owned Vault record. Expected-revision runtime CAS, startup
  recovery, durable actor-state replacement, and revision-bound prior-record
  cleanup reuse `smart-home-pairing-transaction` unchanged.
- Tests cover denial and stale revision before input, exact installed identity,
  one-shot owner-only files, reviewed-address-pinned strict TLS, a real
  login-to-logout CGI loopback, secret-free durable state, interrupted-commit
  recovery, exact replacement cleanup, and cleanup-drift refusal.
- The NVR extension below adds the separately documented per-channel identity
  and snapshot-ability proof without weakening this direct-camera boundary.

## Current Reolink NVR Credential Pairing Slice

This slice extends the completed recoverable transaction composition to one
exact installed Reolink NVR without inferring capabilities from the NVR model:

- Authenticated `GetDevInfo` must report exact product type `NVR` plus the exact
  durable NVR model and serial. `GetChannelstatus.typeInfo` must be non-empty
  and exactly equal to the durable device model for every installed physical
  channel; empty NVR slots are excluded from normalized devices.
- Pairing-only authenticated `GetAbility` inspection requires a positive
  documented `abilityChn[channel].snap.ver` and execute permission for every
  installed channel. A
  missing channel entry, missing field, zero version, offline channel, sleeping
  channel, non-`RLC-*` model, or any durable-set mismatch fails before the
  transaction writes a journal or credential.
- The existing exact D23 principal, one-shot owner-only zeroizing input,
  reviewed-address-pinned strict HTTPS, transaction-owned opaque reference,
  expected-revision runtime CAS, startup recovery, and revision-bound
  replacement cleanup remain unchanged.
- CGI query tokens remain operation-scoped and process-local. Explicit logout
  runs after every authenticated success or failure, and snapshot delivery
  remains a read-only consumer of the bounded versioned credential envelope.
- Tests cover exact NVR serial/model/channel correspondence, empty-slot
  exclusion, documented per-channel snapshot ability, and explicit logout in
  addition to the existing denial, rollback, recovery, and cleanup suite.

## Current Synology Credential Pairing Slice

This slice completes recoverable credential provisioning for one exact
Synology Surveillance Station server without persisting session material:

- `smart-home-synology-pairing-service` accepts only the exact pending pairing
  session, D23 principal, expected durable runtime revision, and completion
  time. Actor messages cannot carry credential paths, bytes, session material,
  or Vault references.
- One host-owned username/password input is bridge-bound, exact-length,
  owner-only, one-shot, and zeroizing. Human Approval succeeds before either
  file is opened, LAN I/O begins, or a transaction journal or Vault record is
  created.
- The pending bridge's credential-free address must match a host-owned
  canonical hostname and reviewed socket address. Production inspection keeps
  the hostname for SNI and strict certificate verification while connecting
  only to the reviewed address.
- Authenticated API discovery, isolated SID-format login, package-permission
  inspection, and privilege-filtered camera listing must prove a non-empty set
  equal to every installed positive `camera_id`, canonical camera entity, and
  snapshot capability. Explicit logout runs after success or authenticated
  failure.
- Only the snapshot host's bounded versioned username/password envelope enters
  the transaction-owned Vault record. SID, SynoToken, OTP, and
  remembered-device material remain process-local and are never persisted.
- Expected-revision runtime CAS, startup recovery, durable actor-state
  replacement, and revision-bound prior-record cleanup reuse
  `smart-home-pairing-transaction` unchanged. Snapshot delivery remains a
  read-only consumer.
- Tests cover denial and stale revision before input, exact installed server
  and camera identity, one-shot owner-only files, reviewed-address-pinned
  strict TLS, a real discovery-to-logout loopback, secret-free durable state,
  interrupted-commit recovery, exact replacement cleanup, and cleanup-drift
  refusal.

## Smart Home Remaining Work

The remaining backlog is ordered by the strongest executable production path
and then by prerequisite readiness:

The reusable central owner and the discovery service's transactional migration
are complete. The remaining central-composition backlog takes priority over
adding another isolated integration or Chief read model:

1. Compose the existing Hue mDNS worker and discovery actor into the local
   controller so discoveries are visible through the same Home Assistant HTTP
   runtime.
2. Migrate Hue pairing and then the remaining pairing/snapshot services so they
   transact against the same live revision instead of restoring private runtime
   copies.
3. Replace the `Rc<RefCell<SmartHomeRuntime>>` Chief bridge with a thread-safe
   service adapter against the controller authority.
4. Add provider-neutral model tool declarations/results, authenticated host
   tool dispatch, and production Chief daemon injection.
5. Prove one executable Chief host to `smart_home.*` to central D23 owner path,
   including durable audit/state and Home Assistant API readback.

The protocol- and vendor-specific backlog below remains valid after those
central ownership steps:

No additional camera snapshot slice is currently executable without a concrete
authentication prerequisite. Blue Iris documents `/image/{camera}` and secure
JSON sessions independently, but does not document how a secure session binds
to that image request; the only documented direct URL credentials require
disabling secure sessions and are rejected. Revision-guarded D23 pairing
completion, its recoverable cross-store journal, and production Hue, ONVIF,
ZoneMinder, Axis, direct Reolink RLC, Reolink NVR, Synology, and Frigate
compositions are now available. The camera pairing services prove bounded
host-owned secret input, D23 principal propagation, authenticated exact-bridge
inspection, durable runtime revision ownership, startup recovery, actor-state
replacement, and revision-bound cleanup without turning snapshot delivery into
a writer. The strongest remaining prerequisite is authenticated AirGradient
MQTT after firmware no longer logs plaintext credentials.

1. Add authenticated AirGradient MQTT only after official firmware removes
   plaintext credential logging and one-shot Vault-leased credential injection
   can be proven without request-plan or normalized-state exposure.
2. Add independent AirGradient telemetry, remote-configuration, and OTA
   destinations only if firmware exposes separate settings; the current
   `httpDomain` contract intentionally governs all three as one HTTPS origin.
3. Add authenticated HEOS source browsing and queue insertion only after the
   account/session and Vault-leasing prerequisites are concrete.
4. Automate Enphase access-token acquisition or renewal only after Enphase
   account authentication, cloud-session handling, operator consent, and
   Vault-leased credential policy are concrete; the current host accepts a
   pre-generated token.
5. Add automatic Enphase IQ Gateway discovery only if Enphase documents a
   stable LAN advertisement; the current production path uses explicit local
   HTTPS endpoint and gateway-serial configuration.
6. Add Enphase live battery, relay, generator, grid, and system-topology state
   only after the normalized energy topology and retention semantics are
   concrete.
7. Add Enphase relay, grid-services, or configuration controls only with
   operation-specific D23 contracts, explicit safety approval, bounded native
   semantics, and readable postcondition verification.
8. Add expiration-aware ZoneMinder access-token reuse and refresh only after a
   supervised session lifecycle and Vault policy for refresh-token residency are
   concrete; the current isolated inspection drops both tokens after each read.
9. Add ZoneMinder event push only after a concrete authenticated event host and
    supervised subscription lifecycle exist.
10. Add ZoneMinder PTZ, monitor configuration, recording-mode, or administrative
    mutations only with operation-specific D23 contracts, least-privilege user
    checks, bounded semantics, and readable postcondition verification.
11. Add UniFi connected-client key rotation across multiple sites or more than
    one 100-client page only after a resumable protocol can prove exact global
    correspondence and all-or-none persistence without extending native-ID or
    one-shot key residency across partial responses.
12. Add UniFi connected-client native details only after field-specific
   minimization and retention are approved; current presence intentionally
   excludes names, native IDs, MACs, IPs, and connection timestamps.
13. Add UniFi historical device statistics, heartbeat-time correlation, or
   broader fleet polling only after durable time-series schema, query access,
   clock semantics, and retention/deletion policy are concrete; current live
   statistics are explicit-target, 64-device bounded, one-minute rate-limited,
   and expire after two minutes.
14. Add remote UniFi Site Manager inspection only after telemetry-egress,
   destination, and operator-consent policy are concrete; keep the current host
   local-only.
15. Add UniFi Network push or change events only after a concrete authenticated
   event host and supervised subscription lifecycle exist.
16. Add UniFi adoption, guest authorization, port actions, or configuration
   mutations only with operation-specific D23 contracts, least-privilege API
   keys, bounded semantics, and readable postcondition verification.
17. Add Synology Surveillance Station OTP and remembered-device authentication
   only after an interactive challenge lifecycle and Vault-leased device-token
   policy are concrete.
18. Add Synology Surveillance Station events only after a concrete authenticated
   event host and supervised subscription lifecycle exist.
19. Add Synology Surveillance Station PTZ, external recording, or configuration
   mutations only with operation-specific D23 contracts, least-privilege API
   checks, bounded semantics, and readable postcondition verification.
20. Add Frigate event and review push only after a concrete authenticated event
   or WebSocket host and supervised subscription lifecycle exist.
21. Add Frigate commands or configuration mutations only with operation-specific
   D23 contracts, least-privilege role checks, and readable postcondition
   verification.
22. Add Frigate recordings, export, and playback only after a supervised
   resource executor owns bounded transfer, cancellation, retention, and
   resource lifecycle semantics.
23. Add bounded Blue Iris snapshots only after an official interface documents
    how an isolated secure JSON session authenticates `/image/{camera}`, or a
    concrete cookie/session-bound media executor exists. Never disable secure
    sessions or place reusable Blue Iris credentials in a URL. Alert/clip
    search, export, and playback still require a supervised resource executor.
24. Add broader Blue Iris `camconfig` or administrative mutations only with
   operation-specific D23 contracts, least-privilege permissions, and readable
   postcondition verification; do not persist the license value returned at
   login.
25. Add automatic Blue Iris discovery only if the server exposes a documented,
   stable LAN advertisement; the current production path is explicit local
   HTTPS endpoint configuration.
26. Add Blue Iris focus, iris, digital-I/O, preset-setting, or broader PTZ
   controls only when each operation has a specific native capability probe,
   bounded semantics, and readable verification where the device exposes it.
27. Add Axis event streaming only after the existing WebSocket protocol core has
   a concrete authenticated host using the completed Digest primitive or a
   short-lived session token, plus subscription supervision.
28. Enumerate Axis video sources/channels before extending PTZ beyond the current
    capability-probed VAPIX camera 1 boundary.
29. Add Axis absolute/relative zoom, guard-tour, or advanced preset management
    only when each operation has a specific capability probe and readable state.
30. Add Reolink current-position, zoom, guard-point, or patrol controls only when
    each operation has a capability-specific probe and the firmware exposes the
    native state needed to avoid invented orientation claims.
31. Add Reolink push events only after a concrete webhook or event-stream host
    and subscription lifecycle exist.
32. Add authenticated KLAP/Tapo devices and other broader-device families only
    after their authentication and session prerequisites are concrete.
33. Add ONVIF PullPoint events once a concrete event host and subscription
    lifecycle exist.
34. Add RTSP media transfer and recording once concrete media transfer and
    recorder host primitives exist.
35. Add a production Matter commissioning, secure-session, and network host only
    after certificate, fabric, Interaction Model encoding, subscription, and
    transport prerequisites exist.
36. Add a Thread border-router host only after an actual host transport exists.
37. Add a production Zigbee coordinator, join, and security host only after
    concrete coordinator transport and security primitives exist.
38. Add production Z-Wave inclusion and S2 only after concrete host transport
    and security primitives exist.

## End-To-End Definition

The first meaningful "working end to end" target should be:

1. A D18 Chief job starts from a host/orchestrator profile.
1. The job calls D18D `smart_home.*` tools.
2. The tools reach the D23 runtime.
3. D23 authorizes reads, subscribe, pair, and low-risk light commands.
4. A Hue fixture or local Hue bridge returns normalized state and health.
5. The job writes a D18D execution journal.
6. The user receives a compact home status or action report.

The first real-home target after that should be:

1. Discover one Hue bridge on LAN.
1. Pair it through a vault-backed credential path.
2. List lights and health.
3. Subscribe to normalized events.
4. Turn one light on and off.
5. Persist state and command audit.
6. Show the result through a local API or dashboard.
