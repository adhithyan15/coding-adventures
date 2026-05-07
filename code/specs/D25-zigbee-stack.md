# D25 - Zigbee Stack

## Overview

Zigbee is a complete low-power IoT stack built above IEEE 802.15.4. It adds a
network layer, device roles, joining, routing, security, endpoints, clusters,
profiles, and application-level device behavior.

D25 defines a from-scratch Zigbee implementation track. The smart-home runtime
can use vendor bridges when that is practical, but this repository should also
be able to form, inspect, simulate, and eventually operate its own Zigbee
networks.

All D25 implementation packages are written in Rust.

---

## Where It Fits

```text
Smart Home Runtime (D23)
    |
    v
Zigbee Integration
    |
    v
Zigbee Stack (D25)
  - network layer
  - application support sublayer
  - device objects
  - cluster library mapping
  - security services
    |
    v
IEEE 802.15.4 (D24)
```

**Depends on:** D24 IEEE 802.15.4, crypto primitives, state-machine packages,
D23 smart-home entity/capability model.

**Used by:** Zigbee coordinator packages, packet analyzers, protocol simulators,
smart-home integrations, learning tools.

---

## Design Principles

1. **Do not start with a USB stick API.** Start with frames, network messages,
   and state machines.
2. **Separate network behavior from device modeling.** Routing and joining are
   not clusters.
3. **Clusters map upward.** Zigbee clusters become D23 capabilities and entities.
4. **Security is explicit.** Network keys, link keys, counters, and trust-center
   behavior are first-class objects.
5. **Interop later, understanding first.** Early packages should decode, encode,
   simulate, and explain before trying to control real homes.

---

## Layers

```text
zigbee-zcl       Zigbee Cluster Library mapping
zigbee-zdo       Zigbee Device Object behavior
zigbee-aps       Application Support Sublayer frames and binding
zigbee-nwk       network frames, addressing, routing, joining
zigbee-security  keys, counters, encryption, trust center model
ieee802154-*     MAC/PHY foundation from D24
```

---

## Package Roadmap

### `zigbee-nwk`

- network frame parser/encoder
- network addresses
- radius/depth fields
- route discovery records
- neighbor and route tables
- join/leave state machines

**Initial Rust implementation:** `code/packages/rust/zigbee-nwk` now provides
the first NWK byte-boundary primitives: network addresses, frame-control bits,
radius/sequence fields, optional IEEE source/destination addresses, multicast
control, payload round-tripping, and first neighbor/route table primitives for
freshness and next-hop lookup. Route discovery messages, joins, security, APS,
ZDO, and ZCL remain future packages/layers.

### `zigbee-aps`

- endpoint addressing
- group addressing
- binding table model
- APS commands
- fragmentation model

**Initial Rust implementation:** `code/packages/rust/zigbee-aps` now provides
APS frame-control, endpoint/group addressing, cluster/profile id, counter, and
payload parser/encoder primitives. Binding tables, APS commands,
fragmentation, and security remain future layers.

### `zigbee-zdo`

- node descriptors
- simple descriptors
- active endpoint discovery
- bind/unbind requests
- management requests

### `zigbee-zcl`

- cluster ids
- attribute reports
- command frames
- foundation commands
- common smart-home clusters mapped to D23 capabilities

### `zigbee-coordinator`

- network formation
- permit join
- device interview
- trust-center policy
- projection into D23 Bridge/Device/Entity records

### `zigbee-sim`

- virtual coordinator/router/end-device nodes
- deterministic routing and join tests
- sleepy end-device behavior

---

## D23 Mapping

```text
Zigbee coordinator -> Bridge
Zigbee node        -> Device
endpoint           -> entity grouping hint
cluster            -> capability family
attribute report   -> DeviceEvent
cluster command    -> DeviceCommand
network key        -> Vault record
link key           -> Vault record
```

---

## Test Strategy

- network frame fixtures parse and encode exactly
- join/leave flows run in a simulator without hardware
- address allocation and route-table updates are deterministic
- security counters reject replay
- cluster reports map into stable D23 capabilities
- sleepy-device queues preserve ordering and expiry semantics

---

## References

- Connectivity Standards Alliance Zigbee:
  <https://csa-iot.org/all-solutions/zigbee/>
- Zigbee FAQ:
  <https://csa-iot.org/all-solutions/zigbee/zigbee-faq/>
- Zigbee 4.0 announcement:
  <https://csa-iot.org/newsroom/the-connectivity-standards-alliance-announces-zigbee-4-0/>
