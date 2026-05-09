# D23A - Smart Home Integration Primitive Catalog

## Overview

D23 defines the normalized smart-home runtime: bridges, devices, entities,
events, commands, state cache, supervision, policy, and audit. D23A defines the
catalog and primitive roadmap that let the runtime grow from "Hue as a trial
run" into a broad smart-home ecosystem without copying any one platform's
architecture.

The architectural shape is:

```text
ecosystem survey
  -> primitive family catalog
  -> integration catalog entry
  -> supervised worker or capability-caged sidecar
  -> normalized D23 bridge/device/entity/event/command model
  -> Chief of Staff tools
```

Home Assistant is only one implementation. It is a useful public map because it
has a large integration index and machine-readable manifests, but the same
primitive families show up in Hubitat, Homey Pro, SmartThings, openHAB,
Homebridge, ioBroker, Domoticz, Jeedom, HomeSeer, and automation systems like
Node-RED, n8n, IFTTT, and Zapier.

D23A's job is not "implement every adapter now." Its job is to identify the
smallest shared primitives that make later adapters boring:

- normalized entity, capability, state, event, command, health, and audit models
- discovery primitives for LAN, radio, USB, MQTT, cloud account, webhook, and
  manual setup
- transport primitives for local HTTP, WebSocket, SSE, MQTT, BLE, serial, radio,
  camera media, cloud APIs, and webhooks
- auth and pairing primitives for bridge buttons, local tokens, OAuth2, API keys,
  certificates, Matter commissioning, HomeKit pairing, radio keys, and MQTT
  credentials
- supervision primitives for pollers, event streams, serial/radio controllers,
  bridge actors, webhook receivers, cloud leases, and calculated workers
- capability cage and Vault primitives so every integration has explicit access
  to secrets, network, filesystem, serial, BLE, radio, camera, and high-risk
  commands

Hue proved the pattern: local discovery, physical pairing, local token storage,
HTTP resource reads, event-stream updates, normalized entity projection, command
mapping, health transitions, and tests can be built once as reusable primitive
families instead of as Hue-only machinery.

---

## Source Snapshot

This spec was refreshed on 2026-05-08 using official or project-owned source
surfaces.

| Platform | Source surface | What it contributes to D23A |
|----------|----------------|-----------------------------|
| Home Assistant | public integration index, developer manifest docs, Core manifests | broad catalog taxonomy, `iot_class`, `integration_type`, virtual aliases |
| Hubitat | supported device list, app/driver docs | local hub, Zigbee/Z-Wave/LAN/cloud driver split, Groovy app sandbox lessons |
| Homey Pro | app store and protocol-rich hub model | app ecosystem, consumer pairing flows, local radios plus cloud apps |
| SmartThings | hub-connected device and Edge driver docs | capability model, Matter/Zigbee/Z-Wave/LAN local execution through hub drivers |
| openHAB | add-ons and bindings reference | bindings as hardware/service adapters, mature protocol-first framing |
| Homebridge | plugin docs and verified plugin program | HomeKit/Matter bridge semantics, plugin quality gates, Node sidecar lessons |
| ioBroker | `sources-dist.json` adapter catalog | adapter-scale ecosystem and admin-installable integration metadata |
| Domoticz | hardware and protocol wiki | hardware gateway view across 433/868/915 MHz, Z-Wave, Zigbee, cameras, Modbus, MQTT |
| Jeedom | smart-home solution and market pages | local-first, plugin-driven, multi-protocol market framing |
| HomeSeer | plugin documentation and hub/software pages | local hub plus commercial plugin ecosystem, Zigbee/Z-Wave/Matter/Hue/ONVIF examples |

### Reference Links

These links are intentionally source links rather than marketing paraphrases.
They give future implementation work stable places to re-check categories,
protocol claims, and ecosystem framing before turning a catalog row into a
worker.

| Source | Reference | D23A reading |
|--------|-----------|--------------|
| Home Assistant | [Integrations](https://www.home-assistant.io/integrations/) | Public integration index spans device classes, service connectors, helpers, system integrations, and virtual/product aliases. |
| Hubitat | [Hubitat products](https://hubitat.com/products/) and [compatible devices](https://docs2.hubitat.com/en/devices/list-of-compatible-devices) | Local hub first: Matter, Z-Wave, Zigbee, Bluetooth, Ring, LAN, drivers, integration apps, and Thread border-router requirements for Matter over Thread. |
| Homey Pro | [Homey apps](https://homey.app/en-us/apps/homey-pro/) and [installing Homey apps](https://support.homey.app/hc/en-us/articles/360012079034-Installing-Homey-apps) | Apps are the user-visible integration unit; official and community apps cover radios, local hubs, cloud services, flows, scripts, energy, media, security, and even Home Assistant bridging. |
| SmartThings | [Devices in SmartThings](https://support.smartthings.com/hc/en-us/articles/360052390111-Devices-in-SmartThings) and [Matter for SmartThings](https://partners.smartthings.com/matter) | Hub-mediated devices, partner devices, regional support, Works with SmartThings certification, Matter, Zigbee, Z-Wave, LAN, and cloud linked services. |
| openHAB | [Add-ons](https://www.openhab.org/addons/) and [Add-on Store](https://www.openhab.org/docs/mainui/addons) | Add-ons split into bindings, automation, system integrations, persistence, transformations, voice, and UI; bindings are the hardware/service adapter analogue. |
| Apple Home | [Apple Home developer page](https://developer.apple.com/apple-home/) | HomeKit, Matter, ThreadNetwork, EnergyKit, MFi, Works with Apple Home, and Matter certification are separate primitives, not one adapter. |
| Google Home | [Matter supported device types](https://developers.home.google.com/matter/supported-devices) | Matter support is device-type and control-surface specific; Matter clusters need a capability projection layer and cannot be assumed uniformly supported. |
| Amazon Alexa | [Matter device categories](https://developer.amazon.com/en-US/docs/alexa/smarthome/supported-matter-device-categories.html) | Alexa is both a cloud/voice ecosystem and a Matter controller surface; device categories and security restrictions must be explicit in capability policy. |
| Z-Wave Alliance | [Command Classes](https://z-wavealliance.org/development-resources-overview/z-wave-command-classes/) | Command classes are the core application primitive for Z-Wave capability mapping, interviews, reports, and command routing. |
| Thread Group | [Thread with Matter](https://threadgroup.org/Newsroom/Blog/thread-with-matter-better-connections-smarter-homes) | Thread is the IP mesh/network primitive; Matter is the application layer. Border-router health and network diagnostics are first-class runtime facts. |

### Survey Takeaways

The platforms disagree on packaging but converge on the same underlying
primitive families:

- Home Assistant and openHAB expose very broad adapter catalogs. They prove the
  catalog must separate devices, hubs, services, helpers, system integrations,
  virtual aliases, and automation add-ons.
- Hubitat and Homey Pro highlight a consumer hub shape: local radios,
  installable apps/drivers, community extensions, and guided setup. D23 should
  support app-like catalog entries without adopting app-local authority.
- SmartThings, Apple Home, Google Home, Alexa, and Matter ecosystems are both
  integration targets and bridge surfaces. D23 needs product aliases and bridge
  entries that route to Matter, HomeKit, cloud account, or standard-protocol
  workers.
- Thread, Zigbee, Z-Wave, MQTT, Matter, HomeKit, and BLE are durable protocol
  primitives. Vendor integrations should consume these rather than reimplement
  their own discovery, credential, command, and supervision machinery.
- Camera/media, energy, climate, safety, locks, alarms, covers, valves, and
  irrigation require stronger privacy and side-effect policy than ordinary
  lighting. The catalog must preserve that risk gradient for Chief of Staff
  agents.

Machine-readable checks performed:

```text
home-assistant/core branch: dev
home-assistant/core commit: 27969c3
home-assistant/core commit_time: 2026-05-09T00:24:51+03:00
home-assistant/core manifest_count: 1461

ioBroker sources-dist.json adapter_count: 607
```

Home Assistant manifest `iot_class` distribution in that snapshot:

| IoT class | Count | Runtime primitive implication |
|-----------|------:|-------------------------------|
| `local_polling` | 417 | local transport plus scheduled refresh and stale-state deadlines |
| `cloud_polling` | 389 | cloud API, token lease, quota-aware scheduler, internet dependency |
| `local_push` | 250 | local subscription/event stream with fast health transitions |
| missing | 241 | helper, platform, internal, legacy, or metadata-gap entries |
| `cloud_push` | 116 | cloud webhook/subscription, token lease, outage-aware health |
| `calculated` | 32 | internal calculated state worker, no external transport |
| `assumed_state` | 16 | command path without trustworthy state feedback |

Home Assistant manifest `integration_type` distribution in that snapshot:

| Integration type | Count | Catalog implication |
|------------------|------:|---------------------|
| missing | 391 | legacy or uncategorized metadata |
| `device` | 267 | one device or device family |
| `hub` | 264 | bridge, controller, gateway, account, or ecosystem |
| `service` | 255 | external data/API service |
| `virtual` | 121 | product alias that routes to a real implementation or standard |
| `system` | 83 | host/system/runtime integration |
| `entity` | 46 | entity-level platform or synthetic entity |
| `helper` | 28 | helper or calculated entity |
| `hardware` | 6 | hardware adapter, dongle, or appliance support |

---

## Core Interpretation

### 1. Integration Is Not A Primitive

"Integration" is a packaging word. The reusable substrate underneath it is more
stable and should be modeled first.

| Integration shape | Examples across platforms | Primitive families |
|-------------------|---------------------------|--------------------|
| Protocol standard | Matter, MQTT, Zigbee, Z-Wave, Thread, HomeKit Controller, ESPHome, Tasmota, KNX, Modbus, BTHome | protocol model, discovery, pairing, transport, command mapping |
| Local hub | Hue, deCONZ, IKEA, UniFi, Sonos, Bond, Broadlink | bridge actor, local auth, event stream/polling, child device projection |
| Local device | Shelly, TP-Link/Kasa/Tapo, WLED, LIFX, Govee local, Roku, Wemo | LAN discovery, local HTTP/UDP/TCP, state cache, command idempotency |
| Bluetooth profile | SwitchBot, BTHome, Govee BLE, trackers, buttons, meters | BLE scanning, GATT, rate limits, host adapter health |
| Cloud account/hub | Tuya, SmartThings, Nest, Ring, Roborock, Alexa/Google ecosystems | OAuth/API key, webhook/polling, cloud outage state, quota policy |
| Camera/media | ONVIF, RTSP, Reolink, Frigate, UniFi Protect, Sonos, Cast | media/camera transport, privacy policy, stream lease, snapshot audit |
| Energy/climate/water | Ecobee, Nest, Enphase, Fronius, Tesla Powerwall, Opower, DSMR/P1, Rachio | telemetry model, forecast/rate state, high-risk setpoint/valve policy |
| Notification/human channel | mobile push, Telegram, Discord, email, Slack, Alexa announcements | D18D service connector, outbound message policy, audit |
| Data service sensor | weather, transit, calendar, finance, health, mail | D18A/D18D service connector, freshness and provenance |
| Helper/calculated entity | template, group, min/max, utility meter, derived presence | calculated worker, dependency graph, replay/cursor |
| Virtual alias | Tapo through TP-Link, Symfonisk through Sonos, appliance brands through Home Connect | catalog alias, guided setup route, no worker |

### 2. Connectivity Class Becomes Supervision

The catalog's connectivity field is not display metadata. It tells the runtime
how to supervise the worker.

| Connectivity | Worker shape |
|--------------|--------------|
| `local_push` | long-lived local stream/socket/subscription plus restart and stale-state deadlines |
| `local_polling` | scheduled local refresh with adaptive backoff and stale-state deadlines |
| `cloud_push` | webhook/subscription worker with token refresh and cloud outage state |
| `cloud_polling` | scheduled cloud refresh with quota policy and explicit internet dependency |
| `calculated` | internal worker fed by event/state dependencies |
| `assumed_state` | command audit plus low-confidence state projection |

### 3. Virtual Entries Are User Experience, Not Runtime Work

Many platforms expose aliases because people search by product or brand, not by
underlying standard. D23A should preserve this without launching duplicate
workers.

Examples:

- Tapo routes to TP-Link/Kasa/Tapo primitives.
- SYMFONISK routes to Sonos/media primitives.
- Ultraloq often routes through Z-Wave.
- ESPHome product lines route through ESPHome or MQTT.
- Appliance brands may route through Home Connect.
- Utility providers may route through Opower-like energy-service primitives.

### 4. Automation Products Are Adjacent, Not Device Truth

Node-RED, n8n, IFTTT, Zapier, and similar systems are flow/workflow engines.
They are important integration surfaces, but they should not be the primary
source of smart-home truth in D23. They belong in D18D as tool/service
connectors or outbound automation bridges. D23 owns device/entity state,
supervision, and command audit.

---

## Primitive Families

### Normalized Model

Required for every integration:

- bridge, device, endpoint/entity, capability, scene, group, event, command,
  health, audit, and provenance identifiers
- stable mapping from vendor resources into D23 entity kinds
- source confidence and state freshness metadata
- alias routing for product names that map to another implementation

### Discovery

Discovery must be reusable and observable:

| Primitive | Examples |
|-----------|----------|
| mDNS/DNS-SD | Hue, Matter, HomeKit, ESPHome, Shelly, WLED, Sonos |
| SSDP/UPnP | media devices, some LAN hubs, discovery fallbacks |
| DHCP/router observation | TP-Link/Kasa/Tapo, LIFX-style LAN devices, static-IP candidates |
| Bluetooth advertisement scan | BTHome, SwitchBot, Govee BLE, trackers |
| USB enumeration | Zigbee coordinators, Z-Wave sticks, Thread border-router adapters |
| MQTT discovery | Home Assistant MQTT discovery, Tasmota, Zigbee2MQTT, custom sensors |
| cloud account inventory | Tuya, SmartThings, Nest, Ring, Roborock |
| webhook registration | cloud push integrations, local callback integrations |
| manual/file config | KNX, Modbus, camera URLs, edge cases |

Every discovery observation should record source, timestamp, confidence,
network/interface, pairing status, and the catalog entry it appears to satisfy.

### Transport

| Primitive | Examples |
|-----------|----------|
| local HTTP/HTTPS | Hue, Shelly, WLED, ESPHome, cameras, energy gateways |
| WebSocket | ESPHome, some local APIs, bidirectional streams |
| Server-Sent Events | Hue CLIP v2 event stream |
| MQTT | Tasmota, Zigbee2MQTT, sensors, user-defined automations |
| CoAP/UDP/TCP | Matter-adjacent and LAN device protocols, vendor APIs |
| serial | Zigbee, Z-Wave, Thread adapters, Modbus RTU |
| BLE GATT | SwitchBot, sensors, locks, meters |
| radio controller | 802.15.4, Z-Wave, 433/868/915 MHz via gateways |
| camera/media | RTSP, ONVIF events, snapshots, stream leases |
| cloud API | OAuth/API-key REST/GraphQL/vendor SDK calls |
| webhook receiver | cloud push, local callbacks, D18D flow bridges |

### Auth And Pairing

| Primitive | Examples |
|-----------|----------|
| physical link button | Hue, some bridges |
| local token/API key | Hue, Shelly, UniFi, cameras, energy gateways |
| OAuth2 | Ecobee, Nest, Ring, SmartThings-like accounts |
| username/password | local controllers, cameras, legacy hubs |
| certificate pairing | Matter, mTLS-capable local services |
| HomeKit pairing | HomeKit Controller/HAP devices |
| Matter commissioning | Matter over Thread/Wi-Fi/Ethernet |
| radio network key | Zigbee, Z-Wave, Thread |
| MQTT credentials | broker username/password/cert |

Secrets are Vault records. Workers receive leases, not ambient access.

### State, Command, And Audit

The shared runtime needs:

- canonical state snapshots and event deltas
- replay cursors and subscription cursors
- stale-state deadlines and state-confidence levels
- idempotent command envelopes
- command outcome events, including "accepted but state unknown"
- high-risk command classification for locks, alarms, covers, valves, cameras,
  irrigation, thermostat setpoints, and garage doors
- per-command audit records with actor, capability grant, target, desired state,
  result, and correlation id

### Supervision

| Worker kind | Used by |
|-------------|---------|
| `poller` | local/cloud polling integrations |
| `event_stream` | Hue SSE, WebSocket streams, cloud push subscriptions |
| `bridge_actor` | hubs that multiplex many child devices |
| `serial_controller` | Zigbee, Z-Wave, Thread, Modbus RTU |
| `radio_controller` | BLE, 802.15.4, 433/868/915 MHz via adapters |
| `webhook_receiver` | cloud and local callbacks |
| `calculated_worker` | templates, groups, utility meters, derived state |
| `sidecar` | non-Rust SDKs inside a capability cage |

Supervision must own restart policy, health state, exponential backoff,
heartbeat deadlines, credential renewal, and degraded/offline transitions.

### Capability Cage

Each primitive exposes the minimum capabilities needed:

```text
net.lan.http
net.lan.mdns
net.lan.ssdp
net.cloud.https
net.webhook.receive
secret.vault.lease
serial.open
ble.scan
ble.gatt
radio.802154
radio.zwave
camera.snapshot
camera.stream
filesystem.config.read
smart_home.read
smart_home.pair
smart_home.manage_network
smart_home.manage_credentials
smart_home.command.*
```

---

## Cross-Platform Integration Matrix

This matrix is intentionally primitive-oriented. It names integration families
that recur across Home Assistant, Hubitat, Homey Pro, SmartThings, openHAB,
Homebridge, ioBroker, Domoticz, Jeedom, and HomeSeer.

| Family | Why it matters | First primitive focus |
|--------|----------------|-----------------------|
| Matter | cross-vendor standard across Wi-Fi, Ethernet, and Thread | commissioning, certificates, mDNS, fabric metadata |
| Thread | low-power IP mesh used by Matter | border-router discovery, diagnostics, network credentials |
| Zigbee | large installed base and common hub radio | coordinator serial API, network keys, ZCL projection |
| Z-Wave | locks, switches, sensors, energy, mature mesh | serial API, command-class projection, inclusion/exclusion |
| MQTT | universal local bus and automation bridge | broker client, discovery topics, retained state, command topics |
| HomeKit/HAP | local pairing and bridge interoperability | pairing, mDNS, accessory model, bridge projection |
| ESPHome | DIY/local devices and voice/sensor surfaces | mDNS, local API, entity projection |
| Tasmota | MQTT-native flashed devices | MQTT device/entity profile |
| BLE/BTHome | cheap sensors, buttons, trackers | scan/GATT primitives and host health |
| Modbus/KNX | HVAC, energy, industrial/building automation | register/group-address mapping, TCP/RTU transports |
| RF/IR bridges | Broadlink, Bond, 433/868 MHz ecosystems | learned command storage, one-way/assumed-state model |
| Hue/deCONZ/IKEA | bridge-based lighting and sensors | local bridge actor, child entities, pairing |
| Shelly/TP-Link/WLED/LIFX | high-leverage local LAN device families | LAN discovery, local HTTP/UDP, command mapping |
| Cameras/NVR | ONVIF, RTSP, Reolink, Frigate, UniFi Protect | camera privacy policy, event stream, snapshot/stream lease |
| Media | Sonos, Cast, Roku, Apple TV, Android TV | media entity model, discovery, command surface |
| Energy/climate | Ecobee, Nest, Enphase, Fronius, Powerwall, Opower | telemetry freshness, cloud/local split, setpoint policy |
| Cloud long tail | Tuya, SmartThings, Ring, Roborock, Alexa/Google | OAuth/API keys, cloud outage, rate limits, webhooks |
| Flow engines | Node-RED, n8n, IFTTT, Zapier | D18D bridge, webhook/action connector, no device truth ownership |

---

## Catalog Entry Model

Each integration catalog entry should describe:

```text
IntegrationCatalogEntry
|-- integration_id
|-- display_name
|-- summary
|-- category
|-- connectivity_class
|-- runtime_kind
|-- implementation_status
|-- priority
|-- discovery_mechanisms[]
|-- auth_modes[]
|-- required_capabilities[]
|-- target_entity_kinds[]
|-- supported_protocols[]
|-- depends_on_integrations[]
|-- virtual_target?
|-- virtual_iot_standards[]
|-- required_primitives[]
|-- source_refs[]
|-- notes[]
```

Recommended enums:

```text
IntegrationCategory
  protocol_standard
  local_hub
  local_device
  bluetooth_profile
  cloud_hub
  camera_media
  energy_climate
  notification_channel
  data_service
  helper_calculated
  virtual_alias
  system_hardware

ConnectivityClass
  local_push
  local_polling
  cloud_push
  cloud_polling
  calculated
  assumed_state

ImplementationStatus
  cataloged
  specified
  scaffolded
  simulated
  first_party_runtime
  production_ready
  delegated_to_standard
  unsupported

PrimitiveFamily
  normalized_model
  discovery_index
  mdns
  ssdp
  dhcp
  local_http
  websocket
  server_sent_events
  mqtt
  bluetooth_low_energy
  usb
  serial_controller
  radio_802154
  zwave_serial_api
  matter_commissioning
  homekit_pairing
  cloud_api
  webhook
  oauth2
  local_pairing
  local_token
  certificate_pairing
  radio_network_key
  mqtt_credentials
  camera_media
  energy_telemetry
  calculated_state
  command_mapping
  capability_policy
  vault_lease
  supervision
  test_simulator
```

---

## Chief Of Staff Mapping

D23A integrations should surface through D18D tools, D18A stores, and D21
capability policy.

Read tools:

```text
smart_home.list_integrations
smart_home.describe_integration
smart_home.list_primitives
smart_home.describe_primitive
smart_home.discover
smart_home.list_bridges
smart_home.list_devices
smart_home.get_state
smart_home.get_health
smart_home.subscribe
```

Management tools:

```text
smart_home.pair_bridge
smart_home.configure_integration
smart_home.manage_network
smart_home.rotate_credentials
smart_home.run_diagnostics
```

Command capabilities:

```text
smart_home.command.light
smart_home.command.switch
smart_home.command.climate
smart_home.command.cover
smart_home.command.lock
smart_home.command.alarm
smart_home.command.camera
smart_home.command.media
smart_home.command.vacuum
smart_home.command.irrigation
smart_home.command.valve
smart_home.command.energy
smart_home.command.scene
```

High-risk command examples:

- unlock a lock
- open a garage door
- disable an alarm
- move or expose a camera
- change thermostat setpoints aggressively
- start irrigation
- open a water/gas valve

The catalog should declare default risk for each command surface so the Tool API
can require human approval, biometric auth, or a hardware-key challenge.

---

## Storage Model

D18A should persist:

```text
integration_catalog_entries
primitive_catalog_entries
integration_aliases
integration_installations
integration_workers
integration_credentials
integration_discovery_observations
integration_pairing_sessions
integration_health_events
integration_capability_grants
integration_source_refs
```

The catalog is mostly static. Installations are local to the user's home. One
catalog entry can have many installations: two Hue bridges, several MQTT
brokers, multiple UniFi sites, and multiple cloud accounts are normal.

---

## Runtime Model

Each installed integration becomes one or more supervised workers:

```text
IntegrationSupervisor
|-- catalog_entry
|-- required_primitives[]
|-- installation
|-- worker_kind
|-- transport lease
|-- Vault lease
|-- state cursor
|-- event stream cursor
|-- command queue
|-- health reporter
|-- audit sink
```

The supervisor never trusts an adapter to be self-healing. It owns restart
policy, health state, backoff, worker lease renewal, and stale-state deadlines.

---

## Rollout Waves

### Wave 0 - Trial Run And Existing Substrate

| Area | Packages |
|------|----------|
| Core model | `smart-home-core` |
| Registry | `smart-home-registry` |
| Runtime | `smart-home-runtime` |
| Discovery | `smart-home-discovery` |
| Hue trial run | `hue-core`, `hue-client`, future `hue-integration` |
| Zigbee substrate | `zigbee-nwk`, `zigbee-aps`, `zigbee-zdo`, `zigbee-zcl` |
| Z-Wave substrate | `zwave-core`, `zwave-serial-api`, `zwave-command-classes` |
| Thread substrate | `thread-mle`, future 6LoWPAN/Thread packages |

Hue's purpose is to validate the primitive shape: discovery, physical pairing,
local token, HTTP reads, SSE events, command mapping, health, audit, and tests.

### Wave 1 - Primitive Multipliers

These should be implemented before most vendor-specific adapters:

| Primitive package direction | Unlocks |
|-----------------------------|---------|
| `smart-home-integration-catalog` | integration/primitive metadata, aliases, roadmap, D18D descriptors |
| `smart-home-mqtt` | Tasmota, Zigbee2MQTT, sensors, local automation bridges |
| `smart-home-ble` and `bthome-core` | SwitchBot, BTHome, BLE sensors/buttons |
| `smart-home-local-http` | Hue, Shelly, WLED, ESPHome, cameras, energy gateways |
| `smart-home-event-streams` | Hue SSE, WebSocket workers, subscription health |
| `smart-home-usb-serial` | Zigbee, Z-Wave, Thread, Modbus RTU |
| `smart-home-camera-media` | ONVIF, RTSP, snapshots, privacy-sensitive state |
| `smart-home-cloud-account` | OAuth/API-key cloud integrations and webhook registration |
| `smart-home-testkit` | fake bridges, fake brokers, fake event streams, replay fixtures |

### Wave 2 - Standards

| Standard | Why |
|----------|-----|
| MQTT | smallest local bus multiplier |
| Matter | modern cross-vendor standard |
| HomeKit Controller | local pairing for many consumer devices |
| ESPHome | DIY/local ecosystem |
| Tasmota | MQTT-native device ecosystem |
| Modbus | HVAC, energy, industrial |
| KNX | building automation |

### Wave 3 - High-Leverage Local Hubs And Devices

| Family | Surfaces |
|--------|----------|
| Hue/deCONZ/IKEA | lights, groups, scenes, sensors, buttons |
| Shelly | relays, switches, covers, sensors, energy |
| TP-Link/Kasa/Tapo | plugs, lights, switches, cameras |
| WLED/LIFX/Govee local | lighting |
| SwitchBot BLE | buttons, curtains, locks, meters |
| UniFi | presence, network, Protect-adjacent surfaces |
| Sonos/Cast/Roku/Apple TV/Android TV | media control and announcements |

### Wave 4 - Cameras, Energy, Climate, Water

| Family | Notes |
|--------|-------|
| ONVIF/RTSP/Reolink/Frigate/UniFi Protect | require camera privacy policy and stream leases |
| Enphase/Fronius/Powerwall/DSMR/P1 | energy telemetry freshness and provenance |
| Ecobee/Nest/tado/Home Connect | climate/appliance cloud-local split |
| Rachio/irrigation/valves | high-risk command approval |

### Wave 5 - Cloud Long Tail And Flow Bridges

| Family | Notes |
|--------|-------|
| Tuya/SmartThings/Ring/Roborock | broad product coverage, cloud dependency and rate limits |
| Alexa/Google assistant bridges | voice/control projection, not core truth source |
| Node-RED/n8n/IFTTT/Zapier | D18D flow connector or webhook/action bridge |
| weather/transit/finance/mail/calendar | D18D data/service connectors, projected into D23 only when device-like |

---

## Package Plan

### `smart-home-integration-catalog`

Pure Rust catalog model and seed data.

Responsibilities:

- integration catalog entry structs/enums
- primitive-family metadata
- connectivity and supervision hints
- first-party rollout seed entries
- alias entries for virtual/product integrations
- query helpers by id, category, capability, primitive, connectivity, status,
  and priority

No network, filesystem, Vault, radio, serial, or runtime I/O.

### `smart-home-primitive-kit`

Future shared primitives package set.

Responsibilities:

- reusable discovery observation records
- transport lease descriptors
- pairing session descriptors
- worker-kind descriptors
- testkit traits for fake brokers, bridges, streams, cloud APIs, and radios

### `smart-home-integration-importer`

Later offline tool for importing external catalog metadata.

Responsibilities:

- read Home Assistant manifests, ioBroker adapter metadata, openHAB binding
  metadata, and similar source catalogs
- produce stable catalog JSON/CBOR artifacts
- normalize connectivity, category, discovery, auth, virtual aliases, and source
  refs
- mark generated entries as reference/catalog-only until first-party support
  exists

### `smart-home-integration-host`

Later supervised worker host.

Responsibilities:

- launch integration workers or capability-caged sidecars
- mediate Vault/network/serial/BLE/radio/camera capabilities
- emit health and audit events
- connect workers to D23 runtime commands/events

---

## Acceptance Criteria

D23A is useful when:

1. A Chief of Staff agent can ask which integrations and primitive families are
   supported, planned, or catalog-only.
2. A user searching for a product alias can be routed to the real integration or
   standard.
3. The runtime can choose poller, event stream, bridge actor, serial/radio
   controller, webhook receiver, or calculated worker from catalog metadata.
4. Security policy can distinguish read-only services, ordinary commands, and
   high-risk commands before an adapter exists.
5. The backlog is driven by standards and high-leverage primitives before cloud
   long-tail work.
6. New integrations can be added as small Rust packages or capability-caged
   sidecars with shared discovery, pairing, auth, state, command, supervision,
   and testkit primitives.

---

## Immediate Implementation Sequence

1. Add `smart-home-integration-catalog` with primitive requirements and seed
   entries.
2. Add D18D descriptors for `smart_home.list_integrations`,
   `smart_home.describe_integration`, `smart_home.list_primitives`, and
   `smart_home.describe_primitive`.
3. Add catalog-backed discovery hints to `smart-home-discovery`.
4. Add the MQTT primitive package before vendor-specific MQTT integrations.
5. Add shared local HTTP/event-stream primitives, reusing Hue as the proof.
6. Add BLE/BTHome, USB/serial, and camera-media primitive packages.
7. Add Matter/HomeKit/ESPHome/Tasmota catalog entries and primitive descriptors
   before full adapters.
8. Add importer tooling that can refresh external catalog snapshots into a
   checked artifact.

---

## Open Questions

1. Should external catalog snapshots be vendored as generated artifacts, or
   remain developer-only research inputs?
2. Should cloud integrations live in D23 if they represent devices, or should
   all cloud services be D18D service connectors projected into D23 only when
   they expose physical devices?
3. Should camera/media capabilities be part of `smart_home.*`, or split into
   `camera.*` and `media.*` tool namespaces with D23 entity projection?
4. Should Matter become a direct first-party stack immediately, or should we
   first support a local Matter server/bridge while Thread and commissioning
   primitives mature?
5. Which runtime should host non-Rust adapters: WASI, process sandbox, Firecracker
   microVM, macOS sandbox, container, or a narrower custom capability cage?

---

## References

- Home Assistant integrations index:
  <https://www.home-assistant.io/integrations/>
- Home Assistant integration manifest documentation:
  <https://developers.home-assistant.io/docs/creating_integration_manifest/>
- Home Assistant Core integration manifests:
  <https://github.com/home-assistant/core/tree/dev/homeassistant/components>
- Hubitat app overview:
  <https://docs.hubitat.com/index.php?title=App_Overview>
- Hubitat supported devices:
  <https://docs.hubitat.com/index.php?title=List_of_Supported_Devices_v1>
- Homey app store:
  <https://homey.app/en-us/apps/>
- SmartThings hub-connected device docs:
  <https://developer.smartthings.com/docs/devices/hub-connected/get-started/>
- openHAB add-ons:
  <https://www.openhab.org/addons/>
- Homebridge developer docs:
  <https://developers.homebridge.io/homebridge/>
- Homebridge plugin verification:
  <https://github.com/homebridge/plugins>
- ioBroker adapter source catalog:
  <https://download.iobroker.net/sources-dist.json>
- Domoticz hardware and protocols:
  <https://wiki.domoticz.com/Hardware>
  <https://wiki.domoticz.com/Integrations_and_Protocols>
- Jeedom smart-home overview:
  <https://jeedom.com/solutions/smart-home>
- HomeSeer plugins:
  <https://docs.homeseer.com/products/plugins>
- D23 Smart Home Runtime: `code/specs/D23-smart-home-runtime.md`
- D18 Chief of Staff: `code/specs/D18-chief-of-staff.md`
- D18D Chief of Staff Tool API: `code/specs/D18D-chief-of-staff-tool-api.md`
- D21 Capability Cage: `code/specs/D21-capability-cage.md`
