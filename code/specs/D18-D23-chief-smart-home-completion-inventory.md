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

## Chief Of Staff Remaining Work

These items are Chief of Staff architecture, not smart-home platform work:

- Load D18D tool catalogs into a host/orchestrator profile instead of only
  using in-memory tests.
- Run a Chief job end to end through scheduler or job framework, tool runtime,
  approval checks, result journal, and final user-visible report.
- Add approval UX and policy wiring for Tier2 or stronger actions such as
  bridge pairing, locks, cameras, alarms, and safety devices.
- Wire vault leasing so tools receive opaque `VaultRef` handles and never raw
  smart-home secrets.
- Persist tool execution journals and expose compact audit summaries for jobs,
  hosts, sessions, and user review.
- Package at least one reusable Chief job such as "home status brief",
  "goodnight check", or "device health triage".

## Smart Home Remaining Work

These items move toward retiring an existing Home Assistant install:

- Run the LAN mDNS scanner from a supervised discovery worker across selected
  interfaces and feed scheduled runs into durable D23 runtime state.
- Complete real Hue pairing: start session, user presence/link button, vault
  credential write, bridge health update, and no-secret audit trail.
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
