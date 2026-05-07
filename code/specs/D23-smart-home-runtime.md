# D23 - Smart Home Runtime

## Overview

Smart home systems look like device control from the outside, but the durable
primitive is not a light bulb, a lock, a thermostat, or a bridge. The durable
primitive is a supervised stream of facts and commands:

```text
device event -> normalized state -> policy -> command -> audited side effect
```

This spec defines a repository-owned smart home runtime that can power Home
Assistant-style local control, Homey-style flows, Node-RED-style device graphs,
IFTTT/Zapier-style automations, and Chief of Staff agents that are allowed to
observe or operate devices.

The runtime is intentionally not "the Hue integration." Philips Hue is the first
adapter because it is a good forcing function:

- it has local bridge discovery
- it has a physical pairing ceremony
- it exposes a REST API for snapshots and commands
- it exposes an event stream for proactive state changes
- it has a real credential that belongs in the Vault
- it hides a Zigbee network behind a bridge abstraction

If the Hue adapter is cut correctly, the same substrate can host Zigbee radios,
Z-Wave controllers, Thread border routers, Matter controllers, cloud APIs, and
arbitrary user-written integrations.

D23 is the application substrate. It does not mean Zigbee, Z-Wave, and Thread
should remain opaque forever. Those protocols are low-level enough that this
repository should implement them from scratch as separate learning-oriented
stacks:

- D24 IEEE 802.15.4 for the shared MAC/PHY foundation used by Zigbee and Thread
- D25 Zigbee Stack for network, APS, ZDO, ZCL, coordinator, and simulator work
- D26 Z-Wave Stack for controller, inclusion, command-class, security, and Long
  Range work
- D27 Thread Stack for 6LoWPAN, MLE, commissioning, border routing, and Matter
  diagnostics

The smart-home runtime consumes those stacks through normalized bridge, device,
entity, event, and command interfaces. The protocol packages own frame formats,
state machines, security handshakes, routing, and simulators.

---

## Where It Fits

```text
User / Agent / Workflow / Job
    |
    v
Tool Runtime (D18D)
    |
    +--> smart_home.list_devices
    +--> smart_home.get_state
    +--> smart_home.command
    +--> smart_home.subscribe
    +--> smart_home.pair_bridge
    |
    v
Smart Home Runtime (D23)
  - registry
  - normalized device/entity model
  - event bus
  - command router
  - integration supervisors
  - pairing sessions
  - state cache
  - policy/audit hooks
    |
    +--> Hue Integration
    |      +--> Hue Bridge Actor
    |      +--> Hue Pairing
    |      +--> Hue CLIP v2 Client
    |      +--> Hue Event Stream
    |
    +--> Zigbee Integration
    |      +--> radio/coordinator process
    |      +--> network map
    |      +--> cluster library mapping
    |
    +--> Z-Wave Integration
    |      +--> controller process
    |      +--> command class mapping
    |      +--> secure inclusion state
    |
    +--> Thread / Matter Integration
           +--> border router discovery
           +--> fabric/controller state
           +--> Matter cluster mapping

Infrastructure:
  D19 Actor        -> supervised bridge/integration actors
  D18A Stores      -> durable device registry, state snapshots, event history
  D18C Jobs        -> scheduled automations and maintenance tasks
  D18D Tool API    -> model/workflow-facing command surface
  D21 Cage/Policy  -> capability and tier enforcement
  Vault packages   -> bridge app keys, radio network keys, fabric credentials
  D24 IEEE 802.15.4 -> shared low-level radio foundation for Zigbee/Thread
  D25 Zigbee Stack  -> from-scratch Zigbee implementation track
  D26 Z-Wave Stack  -> from-scratch Z-Wave implementation track
  D27 Thread Stack  -> from-scratch Thread implementation track
```

**Depends on:** D19 Actor, D18A Stores, D18C Job Framework, D18D Tool API,
D21 Capability Cage, Vault packages.

**Used by:** smart home integrations, workflow automation, agent runtimes, local
UIs, mobile clients, diagnostic tools, and future Home Assistant/Homey/Node-RED
style products.

---

## Design Principles

1. **Local first.** Prefer local LAN/radio control over cloud APIs whenever the
   device ecosystem allows it.
2. **Events are facts.** Device reports are immutable observations, not commands
   smuggled backward.
3. **Commands are explicit side effects.** Every command is validated, authorized,
   routed, retried, and audited.
4. **Credentials never live in adapters.** Integrations request Vault leases and
   never receive ambient secret access.
5. **Protocols are adapters, not the core.** Zigbee, Z-Wave, Thread, Matter, Hue,
   and cloud APIs all project into the same entity/event/command model.
6. **Supervision is the default.** Bridges, radios, event streams, and guest
   integrations are supervised actors with restart policy and health state.
7. **State is cached, but truth is external.** The cache represents the latest
   known state plus freshness metadata. It is never treated as omniscient.
8. **Rust implementation is the rule.** Repository-owned integrations and
   protocol stacks are written in Rust and isolated by actors/processes rather
   than mixed implementation languages.

---

## Key Concepts

### Integration

An Integration is a package that knows how to talk to one device ecosystem.

Examples:

- `hue`
- `zigbee`
- `zwave`
- `thread`
- `matter`
- `homekit`
- `mqtt`
- `shelly`
- `tplink.kasa`

An integration owns protocol-specific details and exports repository-owned
interfaces:

```text
Integration
|-- integration_id
|-- display_name
|-- version
|-- runtime_kind        in_process_rust | rust_worker_process
|-- capabilities[]
|-- discovery_handlers[]
|-- pairing_handlers[]
|-- bridge_actor_factory
|-- entity_mapper
|-- command_mapper
|-- diagnostics
```

Integrations do not expose raw sockets, raw serial ports, or raw secrets to
agents. They expose normalized tools and event streams through the runtime.

### Bridge

A Bridge is a local control point for a device network or product ecosystem.

Examples:

- Philips Hue Bridge / Bridge Pro
- Zigbee USB coordinator
- Z-Wave USB controller
- Thread border router
- Matter controller service
- MQTT broker
- vendor cloud account proxy

```text
Bridge
|-- bridge_id
|-- integration_id
|-- transport          lan_http | mdns | serial | ble | cloud | local_process
|-- address?
|-- hardware_model?
|-- firmware_version?
|-- auth_ref?          Vault secret reference, never the secret value
|-- health
|-- last_seen_at
|-- metadata
```

Each bridge is represented by a supervised `BridgeActor`.

### Device

A Device is a physical or logical thing reported by a bridge.

```text
Device
|-- device_id
|-- bridge_id
|-- manufacturer
|-- model
|-- name
|-- serial?
|-- firmware_version?
|-- room_id?
|-- entities[]
|-- identifiers[]
|-- health
|-- metadata
```

A Hue bulb, a Z-Wave lock, a Zigbee motion sensor, and a Matter thermostat all
become Devices.

### Entity

An Entity is one controllable or observable facet of a device.

Examples:

- a light on/off switch
- brightness level
- color temperature
- color coordinates
- occupancy sensor
- contact sensor
- lock state
- battery percentage
- thermostat setpoint
- scene

```text
Entity
|-- entity_id
|-- device_id
|-- kind               light | switch | sensor | lock | thermostat | scene | ...
|-- name
|-- capabilities[]
|-- state
|-- freshness
|-- metadata
```

Entity ids are repository-owned stable ids. Protocol ids remain in metadata and
bridge-private mapping tables.

### Capability

A Capability describes what an entity can do or report.

```text
Capability
|-- capability_id      light.on_off | light.brightness | sensor.occupancy
|-- mode               observe | command | observe_and_command
|-- value_schema
|-- unit?
|-- min?
|-- max?
|-- step?
```

Capabilities let automations and agents reason about devices without hardcoding
Hue resources, Zigbee clusters, Z-Wave command classes, or Matter clusters.

### DeviceEvent

A DeviceEvent is an immutable observation from the outside world.

```text
DeviceEvent
|-- event_id
|-- bridge_id
|-- device_id?
|-- entity_id?
|-- observed_at
|-- received_at
|-- event_type         discovered | updated | removed | unavailable | error
|-- state_delta?
|-- raw_ref?           pointer to raw protocol payload in ArtifactStore
|-- correlation_id?
|-- metadata
```

Raw protocol payloads can be stored as artifacts for diagnostics, but the common
path consumes normalized deltas.

### DeviceCommand

A DeviceCommand is an explicit request to change the outside world.

```text
DeviceCommand
|-- command_id
|-- entity_id
|-- command_type       turn_on | turn_off | set_brightness | set_color | ...
|-- arguments
|-- requested_by
|-- idempotency_key?
|-- required_tier
|-- required_capabilities[]
|-- timeout_ms
|-- correlation_id
```

Commands are never sent directly to adapters by models. They pass through Tool
Runtime validation, policy, and audit first.

### Scene

A Scene is a named bundle of desired entity states. Some ecosystems, including
Hue, expose native scenes. Others need repository-owned scenes.

```text
Scene
|-- scene_id
|-- scope              room | zone | home | bridge | custom
|-- native_ref?
|-- actions[]
|-- metadata
```

The runtime should prefer native scenes when they are semantically equivalent,
because vendor bridges often apply them more atomically and efficiently.

---

## Runtime Layers

### SmartHomeRegistry

The registry stores normalized Bridges, Devices, Entities, Scenes, and their
protocol mapping tables.

Backends:

- D18A Store layer for repository-owned records
- ArtifactStore for raw discovery snapshots and diagnostics
- Vault references for credentials

The registry is the source of identity, not necessarily the source of latest
state. Latest state belongs to the state cache.

### State Cache

The state cache stores the latest known entity state plus freshness information.

```text
StateSnapshot
|-- entity_id
|-- value
|-- source             event_stream | poll | optimistic_command | manual
|-- observed_at
|-- received_at
|-- expires_at?
|-- confidence         confirmed | optimistic | stale | unknown
```

Optimistic command state must be replaced by confirmed state from the bridge or
marked stale after a timeout.

### Event Bus

The event bus is a D19 channel family for smart-home observations.

Suggested channels:

- `smart_home.discovery`
- `smart_home.state`
- `smart_home.command`
- `smart_home.health`
- `smart_home.audit`

The bus supports replay, so a UI, workflow engine, or agent can rebuild local
state from a checkpoint.

### Command Router

The command router maps a normalized command to the owning bridge actor.

```text
Tool call
  -> validate input
  -> resolve entity
  -> authorize requested side effect
  -> create DeviceCommand
  -> route to BridgeActor
  -> adapter maps to protocol request
  -> bridge response becomes CommandResult
  -> later event stream confirms actual state
```

The router should separate "request accepted by bridge" from "device definitely
changed state." Smart home networks are lossy and eventually consistent.

### Integration Supervisor

Each integration runs under a supervisor tree.

```text
SmartHomeSupervisor
|-- IntegrationSupervisor(hue)
|   |-- DiscoveryActor
|   |-- PairingActor
|   |-- BridgeActor(hue_bridge_1)
|   |   |-- HttpClient
|   |   |-- EventStreamActor
|   |   |-- PollFallbackActor
|   |   |-- HealthActor
|   |-- BridgeActor(hue_bridge_2)
|
|-- IntegrationSupervisor(zigbee)
|   |-- CoordinatorActor
|   |-- NetworkMapActor
|   |-- InterviewActor
|
|-- IntegrationSupervisor(zwave)
|   |-- ControllerActor
|   |-- InclusionActor
|   |-- CommandClassMapper
```

Restarting an event stream must not restart the whole bridge unless the bridge
actor cannot recover its own invariant.

---

## Tool Surface

The initial D18D tool catalog should be small and generic:

```text
smart_home.discover
smart_home.pair_bridge
smart_home.list_bridges
smart_home.list_devices
smart_home.get_state
smart_home.command
smart_home.subscribe
smart_home.describe_capabilities
smart_home.get_health
```

### `smart_home.command`

```text
input
|-- entity_id
|-- command_type
|-- arguments
|-- idempotency_key?

output
|-- command_id
|-- status             accepted | rejected | timed_out | failed
|-- bridge_id
|-- correlation_id
|-- message?
```

Side effects: `external`

Required policy:

- read access to target entity
- command access to requested capability
- stronger tier for locks, garage doors, alarms, cameras, and safety devices
- optional human confirmation for high-risk commands

### `smart_home.subscribe`

Subscriptions are runtime-level streams over normalized events. The tool should
not expose protocol-native SSE, serial frames, or radio packets to models.

```text
input
|-- filter
|   |-- bridge_ids[]?
|   |-- device_ids[]?
|   |-- entity_ids[]?
|   |-- capability_ids[]?
|-- from_checkpoint?

output
|-- subscription_id
|-- checkpoint
```

Side effects: `read`

---

## Philips Hue Adapter

Hue is a bridge-mediated integration. The repository talks to the Hue Bridge on
the local network; the bridge talks to lights and accessories, mostly over
Zigbee. The runtime should not need to understand the private Zigbee network
inside a Hue bridge in order to control Hue resources.

### Discovery

Hue bridge discovery should support:

1. mDNS on the local network
2. the Hue cloud discovery endpoint as a fallback
3. manual IP entry

Discovery creates a candidate Bridge record with `health = unpaired` and no
credential.

### Pairing

Hue local pairing is a physical-presence ceremony:

1. User asks to pair a discovered bridge.
2. Runtime starts a `PairingSession`.
3. UI prompts the user to press the bridge link button.
4. Pairing actor calls the Hue bridge registration endpoint with an app/device
   identifier.
5. Bridge returns an application key / username.
6. Runtime stores the key in the Vault and records only an `auth_ref` on the
   Bridge.

The pairing session is short-lived and auditable.

### Local API Shape

The Hue CLIP v2 local API uses HTTPS to the bridge and an application key header.
The adapter should isolate the bridge certificate behavior inside the Hue client
instead of leaking TLS exceptions to the generic runtime.

Common resource paths:

```text
GET /clip/v2/resource
GET /clip/v2/resource/bridge
GET /clip/v2/resource/device
GET /clip/v2/resource/light
GET /clip/v2/resource/light/{id}
PUT /clip/v2/resource/light/{id}
GET /clip/v2/resource/grouped_light
PUT /clip/v2/resource/grouped_light/{id}
GET /clip/v2/resource/room
GET /clip/v2/resource/zone
GET /clip/v2/resource/scene
PUT /clip/v2/resource/scene/{id}
```

Typical request shape:

```text
https://<bridge-ip>/clip/v2/resource/light/{id}
hue-application-key: <leased app key>
```

The app key must be acquired through a Vault lease for each client session and
zeroized when no longer needed.

### Event Stream

Hue v2 exposes proactive local state changes through an event stream. The Hue
adapter should prefer the event stream for state updates and use polling only for
startup snapshots, missed-event recovery, and health checks.

```text
GET https://<bridge-ip>/eventstream/clip/v2
Accept: text/event-stream
hue-application-key: <leased app key>
```

The `HueEventStreamActor` responsibilities:

- open the stream
- parse event batches
- convert Hue resource deltas to `DeviceEvent`
- update the state cache
- emit health transitions on disconnect/reconnect
- restart with backoff under supervision
- trigger snapshot reconciliation after suspected gaps

### Mapping

Hue resources map into the normalized model:

```text
Hue bridge         -> Bridge
Hue device         -> Device
Hue light          -> Entity(kind = light)
Hue grouped_light  -> Entity(kind = light_group)
Hue room           -> Location / grouping metadata
Hue zone           -> Location / grouping metadata
Hue scene          -> Scene(native_ref = hue scene id)
Hue motion sensor  -> Entity(kind = sensor, capability = sensor.occupancy)
Hue button         -> Entity(kind = input, capability = input.button)
```

### Bridge Pro

Hue Bridge Pro should be represented as the same integration with different
capabilities:

- larger bridge capacity
- faster bridge responses
- Wi-Fi bridge transport in addition to Ethernet
- MotionAware resources where available

The adapter should discover Bridge Pro features from bridge resources and
feature flags rather than hardcoding product assumptions.

---

## Protocol Adapter Strategy

### Zigbee

Zigbee is usually exposed through either a vendor bridge, such as Hue, or a local
coordinator, such as a USB radio. Direct Zigbee support should be its own
integration because it owns radio lifecycle, network formation, joining,
interviews, binding, reporting, and cluster mapping.

The smart-home core should not embed Zigbee concepts. The Zigbee adapter maps:

```text
Zigbee network       -> Bridge
IEEE address         -> Device identifier
endpoint             -> entity grouping hint
cluster              -> capability family
attribute report     -> DeviceEvent
cluster command      -> DeviceCommand mapping
binding/reporting    -> adapter-private configuration
network key          -> Vault secret
```

The implementation should be repository-owned and layered through D24/D25:
frame parsing first, deterministic simulation second, hardware adapters third.
USB coordinator support is an adapter around our Zigbee stack, not the
definition of the stack.

### Z-Wave

Z-Wave is controller-mediated. It should also be a separate integration because
it owns inclusion/exclusion, node interviews, command classes, security classes,
and regional radio constraints.

Mapping:

```text
Z-Wave controller    -> Bridge
node id              -> Device identifier scoped to controller
command class        -> capability family
value report         -> DeviceEvent
set command          -> DeviceCommand mapping
S2 keys              -> Vault secrets
```

The runtime must understand that some Z-Wave commands are slow, sleepy,
queued, or require secure inclusion before they are available.

The implementation should be repository-owned and layered through D26. Existing
controller APIs are useful references and hardware access paths, but the stack
should model frames, inclusion, command classes, security, node interviews, and
Long Range topology explicitly.

### Thread and Matter

Thread is an IPv6 mesh network, not a smart-home application model by itself.
Matter is the application layer most smart-home products will use over Thread.
Therefore Thread support should normally enter the runtime through a Matter
controller integration.

Mapping:

```text
Thread border router -> Bridge / network path
Matter fabric        -> Controller credential set
Matter node          -> Device
endpoint             -> entity grouping hint
cluster              -> capability family
attribute report     -> DeviceEvent
command              -> DeviceCommand mapping
fabric credentials   -> Vault secrets
```

Thread diagnostics are still valuable: border-router health, network partition
state, route reachability, and multicast/service-discovery health affect Matter
reliability.

The implementation should be repository-owned and layered through D24/D27.
Thread is networking, not device automation. Matter-over-Thread should consume
Thread reachability and diagnostics while keeping Matter clusters in the
application layer.

---

## Package Roadmap

### `smart-home-core`

Repository-owned types:

- `Bridge`
- `Device`
- `Entity`
- `Capability`
- `DeviceEvent`
- `DeviceCommand`
- `CommandResult`
- `Scene`
- `StateSnapshot`
- `IntegrationDescriptor`

No protocol dependencies.

**Initial Rust implementation:** `code/packages/rust/smart-home-core` now owns
the normalized D23 data model plus D18D-style smart-home tool descriptors. It is
pure data and performs no I/O.

### `smart-home-registry`

Durable storage for bridge/device/entity identity and protocol mapping tables.

Uses D18A stores and Vault references.

**Initial Rust implementation:** `code/packages/rust/smart-home-registry` now
provides an in-memory registry for normalized bridge/device/entity/scene
records, protocol-native identifier indexes, state snapshots, event logs, and
selector-based device/entity queries over bridge, health, kind, capability, and
state freshness. It is deliberately pure; durable D18A storage and Vault-backed
credential resolution remain later layers.

### `smart-home-runtime`

Actor supervisors, command router, event bus, state cache, health model, and
integration lifecycle.

Uses D19 actors and D18D tools.

**Initial Rust implementation:** `code/packages/rust/smart-home-runtime` now
provides a synchronous runtime core over the normalized registry: event-bus
subscriptions, command validation against entity capabilities, accepted command
results with optimistic state expiry, device-event replay into the state cache,
bridge health updates, and supervised bridge-worker heartbeat/restart signals.

### `smart-home-discovery`

Reusable discovery helpers:

- mDNS
- SSDP if needed for legacy ecosystems
- manual address records
- cloud-discovery fallback hooks

### `hue-core`

Hue-specific types and mapping code. No network I/O.

**Initial Rust implementation:** `code/packages/rust/hue-core` now owns CLIP v2
resource/id/path primitives, structured Hue command intents, and bridge/device/
light projection into `smart-home-core`. HTTPS, TLS policy, Vault leases, and
event-stream transport remain for `hue-client` and `hue-integration`.

### `hue-client`

Hue CLIP v2 HTTP client:

- bridge registration
- resource snapshots
- resource commands
- event stream connection
- bridge diagnostics

### `hue-integration`

Runtime adapter:

- discovery handler
- pairing handler
- Hue bridge actor
- Hue event stream actor
- Hue resource mapper
- Hue command mapper

### `smart-home-testkit`

Fake bridges and deterministic event streams for tests:

- fake Hue bridge
- fake event stream with disconnects/gaps
- command acceptance vs later state confirmation
- stale-state scenarios
- policy rejection scenarios

---

## Failure Model

Smart homes fail in mundane but important ways:

- bridge goes offline
- device is unplugged
- sleepy battery device misses a command
- radio network is partitioned
- event stream disconnects
- optimistic state diverges from real state
- credentials are revoked
- firmware changes a resource shape
- user renames/moves a device in a vendor app

D23 treats each of these as normal operational states, not exceptional crashes.

Bridge actors should crash only when their own invariant is broken. Network
failures, device unavailability, and denied commands should become health events
or command results.

Recommended health states:

```text
unknown
discoverable
unpaired
online
degraded
offline
auth_failed
unsupported
removed
```

---

## Security Model

### Credentials

All smart-home credentials are Vault records:

- Hue app keys
- Zigbee network keys
- Z-Wave S2 keys
- Matter fabric credentials
- cloud API tokens
- MQTT credentials

Adapters receive leased secret material only for the minimum time needed. The
registry stores `auth_ref`, not secret bytes.

### Capability Policy

The policy layer should distinguish at least:

```text
smart_home.read
smart_home.command.light
smart_home.command.switch
smart_home.command.climate
smart_home.command.lock
smart_home.command.alarm
smart_home.command.camera
smart_home.pair
smart_home.manage_network
smart_home.manage_credentials
```

Turning on a lamp is not the same risk as unlocking a door or disabling an
alarm. The Tool API should expose that difference directly.

### Audit

Audit records should answer:

- who requested the command
- which tool call produced it
- which entity was targeted
- which bridge executed it
- what policy allowed or denied it
- whether the bridge accepted it
- whether later state confirmed it

Raw protocol payloads should be optional diagnostic artifacts, not mandatory
audit log content.

---

## Rust Integration Runtime

All repository-owned smart-home integration code is written in Rust. That
includes Hue, Zigbee, Z-Wave, Thread, Matter, MQTT, protocol simulators,
hardware adapters, and diagnostic tools.

The runtime can still preserve a process boundary between the smart-home host
and an integration package. That boundary is for supervision, capability
control, restart isolation, and memory accounting, not for allowing arbitrary
implementation languages inside the core stack.

Every integration process speaks the same host protocol:

```text
host.discover
host.pair
host.snapshot
host.command
host.subscribe
host.health
```

The host grants capabilities, Vault leases, network access, serial access, radio
access, and filesystem access explicitly. A million logical integrations can
exist as records; real integration workers are supervised Rust processes or
actors with explicit resource budgets.

---

## Product Targets

D23 should be sufficient to build:

1. A Home Assistant-style local smart-home controller
2. A Homey-style flow builder
3. A Node-RED-style visual graph over device events and commands
4. An IFTTT/Zapier-style automation service
5. A Chief of Staff smart-home agent that can observe and act through policy
6. A diagnostic console for bridge/radio/network health

The same packages should compose differently for each product. The runtime does
not own the UI.

---

## Test Strategy

### Core tests

- registry round-trips preserve stable ids and protocol-private mappings
- state cache handles confirmed, optimistic, stale, and unknown state
- command router rejects commands for missing entities or unsupported
  capabilities
- policy layer distinguishes read, low-risk command, and high-risk command
- event bus replay rebuilds state from a checkpoint

### Hue tests

- discovery records an unpaired bridge without credentials
- pairing session stores only a Vault reference in the registry
- resource snapshots map Hue bridge/device/light/room/zone/scene resources into
  normalized records
- event stream deltas update state without requiring polling
- event stream disconnects restart with backoff and mark bridge health degraded
- command acceptance is separate from later state confirmation

### Adapter testkit

The `smart-home-testkit` package should provide deterministic fake bridges and
scriptable event streams. Integration authors should be able to test:

- bridge offline/online transitions
- missed events and reconciliation
- stale optimistic state
- denied credentials
- unknown resource shapes
- slow or sleepy devices
- duplicate events and idempotent command handling

### Security tests

- adapters cannot read credentials except through explicit Vault leases
- command tools emit audit records on allow and deny
- high-risk commands require stronger policy than ordinary lighting commands
- integration workers receive only declared host capabilities
- raw diagnostic payload retention follows configured policy

---

## Future Extensions

- Matter controller integration over Thread, Wi-Fi, and Ethernet
- MQTT integration for power users and bridge-style device aggregators
- visual workflow graph runtime over normalized smart-home events
- radio diagnostics UI for Zigbee, Z-Wave, and Thread networks
- local-only automation bundles that can run without cloud connectivity
- bridge migration tools, especially for multi-bridge Hue and Bridge Pro homes
- shared homes with multi-user authorization and per-room policy
- simulation mode for testing automations before they affect real devices

---

## Open Questions

1. Should Matter be a first-class D23 integration immediately, or should it wait
   until Hue and one direct-radio adapter prove the normalized model?
2. Should the first implementation use in-process Rust actors, separate Rust
   worker processes, or both depending on adapter risk?
3. How much raw protocol payload should be retained by default for debugging?
4. Should automations consume only normalized events, or should expert workflows
   be allowed to subscribe to protocol-native events?
5. What is the minimum Rust host protocol required for integration workers to be
   useful without exposing ambient authority?

---

## References

- Philips Hue Developer Program: <https://developers.meethue.com/>
- Hue API v2 announcement and event-stream guidance:
  <https://developers.meethue.com/new-hue-api/>
- Hue getting-started guide for bridge discovery and local pairing:
  <https://developers.meethue.com/develop/get-started-2/>
- OpenHue API, a community OpenAPI description of Hue CLIP v2:
  <https://github.com/openhue/openhue-api>
- Connectivity Standards Alliance Zigbee materials:
  <https://csa-iot.org/all-solutions/zigbee/>
- Z-Wave Alliance specification resources:
  <https://z-wavealliance.org/development-resources-overview/specification-for-developers/>
- Z-Wave Long Range overview:
  <https://z-wavealliance.org/z-wave-long-range-technology/>
- Thread Group overview:
  <https://www.threadgroup.org/What-is-Thread/Overview/home>
