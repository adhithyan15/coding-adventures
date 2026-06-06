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

- Implement real D23 discovery workers, starting with LAN and mDNS Hue
  discovery, that feed the runtime discovery catalog.
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
