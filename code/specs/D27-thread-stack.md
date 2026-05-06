# D27 - Thread Stack

## Overview

Thread is a secure, low-power IPv6 mesh networking stack built above IEEE
802.15.4. It is not itself a smart-home application model. In modern smart homes,
Matter usually supplies that application layer over Thread.

D27 defines a from-scratch Thread implementation track. The repository should be
able to understand and simulate Thread networking, border routing, commissioning,
mesh link establishment, service discovery interactions, and diagnostics rather
than treating Thread as a black box hidden behind platform APIs.

All D27 implementation packages are written in Rust.

---

## Where It Fits

```text
Smart Home Runtime (D23)
    |
    v
Matter Integration
    |
    v
Thread Diagnostics / Network Path (D27)
    |
    +--> IPv6 / UDP / CoAP / mDNS pieces
    +--> 6LoWPAN adaptation
    +--> Thread MLE
    +--> border router model
    |
    v
IEEE 802.15.4 (D24)
```

**Depends on:** D24 IEEE 802.15.4, IPv6/UDP/network packages, crypto
primitives, state-machine packages, D23 smart-home model.

**Used by:** Thread simulators, Matter-over-Thread diagnostics, border router
experiments, packet analyzers, smart-home reliability tooling.

---

## Design Principles

1. **Thread is networking.** Do not confuse it with Matter clusters or device
   automation.
2. **Use IP-native abstractions where appropriate.** Thread nodes should project
   into IPv6 routes and services, not vendor bridge commands.
3. **Diagnostics are product value.** Border-router health, partitions, multicast
   reachability, and service discovery explain many Matter failures.
4. **Commissioning is security-sensitive.** Mesh credentials and fabric-facing
   credentials belong in Vault-backed records.
5. **Simulate partitions early.** Thread reliability is easiest to understand in
   a deterministic mesh simulator.

---

## Layers

```text
matter-*          application layer above Thread, future spec/package family
thread-diagnostics D23-facing health, topology, and reachability projection
thread-border-router border routing and prefix behavior
thread-mle        mesh link establishment state machines
sixlowpan         IPv6 adaptation over 802.15.4
ieee802154-*      MAC/PHY foundation from D24
```

---

## Package Roadmap

### `sixlowpan`

- IPv6 header compression model
- fragmentation/reassembly
- mesh addressing helpers
- UDP compression

**Initial Rust implementation:** `code/packages/rust/sixlowpan` now provides
dispatch-byte classification, LOWPAN_IPHC first/second byte parsing, and
fragment first/next header parse/encode helpers. Full IPv6/UDP decompression,
reassembly, MLE, commissioning, and border-router behavior remain future
layers.

### `thread-mle`

- roles and neighbor state
- parent/child relationships
- leader/router/end-device behavior
- attach/detach flows
- network data model

**Initial Rust implementation:** `code/packages/rust/thread-mle` now provides
Thread role, MLE command, TLV, scan-mask, mode, message parser/encoder, and
deterministic parent/child attach-state primitives. Neighbor tables, network
data, UDP/CoAP/DTLS, and commissioning remain future layers.

### `thread-commissioning`

- commissioning state model
- joiner lifecycle
- credential custody
- audit events

### `thread-border-router`

- prefix advertisement model
- external route model
- border-router health
- multicast and service-discovery diagnostics

### `thread-diagnostics`

- topology snapshots
- partition detection
- route reachability
- D23 health projection

### `thread-sim`

- virtual mesh nodes
- border-router scenarios
- network partition/merge tests
- sleepy end-device behavior

---

## D23 Mapping

Thread usually maps into D23 through Matter:

```text
Thread border router -> Bridge / network path
Thread partition     -> diagnostic health domain
Matter fabric        -> controller credential set
Matter node          -> Device
Matter endpoint      -> entity grouping hint
Matter cluster       -> capability family
```

Thread-specific state should still be visible in diagnostics:

```text
border router health -> smart_home.health event
route failure        -> smart_home.health event
partition change     -> smart_home.health event
commissioning event  -> audit event
```

---

## Test Strategy

- 6LoWPAN compression fixtures round-trip
- MLE attach/detach flows run without hardware
- partitions and merges are deterministic in simulation
- border-router route advertisements produce expected reachability state
- commissioning stores credentials only through Vault references
- D23 receives health events without needing Thread internals

---

## References

- Thread Group overview:
  <https://threadgroup.org/What-is-Thread>
- Thread in homes:
  <https://www.threadgroup.org/BUILT-FOR-IOT/Home>
- Thread 1.4.0 specification page:
  <https://threadgroup.org/ThreadSpec>
- Thread Network Fundamentals:
  <https://www.threadgroup.org/Portals/0/documents/support/Thread%20Network%20Fundamentals_v3.pdf>
- Matter connectivity transports:
  <https://handbook.buildwithmatter.com/how-it-works/connectivity-transports/>
