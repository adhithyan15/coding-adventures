# D26 - Z-Wave Stack

## Overview

Z-Wave is a sub-GHz smart-home protocol with its own radio, MAC/network behavior,
controller model, node inclusion, security classes, and application command
classes. Unlike Zigbee and Thread, it is not built on IEEE 802.15.4.

D26 defines a from-scratch Z-Wave implementation track. The goal is to understand
classic Z-Wave, Z-Wave Plus, S2 security, SmartStart-style provisioning, and
Z-Wave Long Range as protocol machinery rather than only through existing
controller libraries.

All D26 implementation packages are written in Rust.

---

## Where It Fits

```text
Smart Home Runtime (D23)
    |
    v
Z-Wave Integration
    |
    v
Z-Wave Stack (D26)
  - serial/controller API boundary
  - frame parser/encoder
  - node table
  - inclusion/exclusion
  - command classes
  - S0/S2 security model
  - Long Range mode
```

**Depends on:** crypto primitives, state-machine packages, serial/USB adapter
packages, D23 smart-home entity/capability model.

**Used by:** Z-Wave controller packages, simulators, packet analyzers, smart-home
integrations, learning tools.

---

## Design Principles

1. **Model the protocol, not just the controller library.**
2. **Command classes map upward.** Z-Wave command classes become D23
   capabilities and entities.
3. **Regional radio constraints are data, not conditionals scattered through the
   stack.**
4. **Security inclusion is a lifecycle.** Keys and granted command classes are
   part of node identity.
5. **Long Range is not just bigger mesh.** It has different topology and scaling
   assumptions, so model it explicitly.

---

## Package Roadmap

### `zwave-core`

- frame types
- home id and node id model
- controller/node identifiers
- regional profile metadata
- parse errors

**Initial Rust implementation:** `code/packages/rust/zwave-core` now provides
Home ID, classic node id, Long Range node id, region profile, command-class id,
and Serial API frame parse/encode primitives with checksum validation. The
controller state machine, inclusion, callbacks, command classes, and S2 remain
future layers.

### `zwave-serial-api`

- serial framing
- host/controller request-response correlation
- callbacks
- controller capabilities

**Initial Rust implementation:** `code/packages/rust/zwave-serial-api` now
provides function ids, request/response/callback classification, controller
capability decoding, Memory Get ID parsing, and request tracking with callback
correlation and timeout expiry. Real serial-port I/O and controller loops remain
future layers.

### `zwave-command-classes`

- command-class registry
- value reports
- set/get commands
- interview descriptors
- mapping into D23 capabilities

### `zwave-security`

- S0 model for historical understanding
- S2 key classes
- nonce lifecycle
- inclusion state machine
- replay and counter checks

### `zwave-controller`

- inclusion/exclusion
- node interview
- route and health state
- sleepy node queues
- projection into D23 Bridge/Device/Entity records

### `zwave-lr`

- Long Range topology model
- 12-bit addressing
- direct gateway-to-node assumptions
- LR-specific inclusion and health tracking

### `zwave-sim`

- virtual controller
- virtual nodes
- sleepy devices
- secure inclusion fixtures
- command class behavior tests

---

## D23 Mapping

```text
Z-Wave controller -> Bridge
node id           -> Device identifier scoped to controller
command class     -> capability family
value report      -> DeviceEvent
set command       -> DeviceCommand
S2 key            -> Vault record
```

---

## Test Strategy

- serial frames round-trip through parser and encoder
- controller request/callback correlation is deterministic
- inclusion/exclusion state machines survive interruption
- S2 replay protection rejects stale messages
- sleepy-node command queues expire correctly
- command classes map into stable D23 capabilities
- Long Range nodes do not accidentally use classic mesh assumptions

---

## References

- Z-Wave developer specification resources:
  <https://z-wavealliance.org/development-resources-overview/specification-for-developers/>
- Z-Wave specification overview:
  <https://z-wavealliance.org/certification-overview/specification/>
- Z-Wave Long Range:
  <https://z-wavealliance.org/z-wave-long-range-technology/>
- Z-Wave 2025B announcement:
  <https://z-wavealliance.org/introducing-the-z-wave-2025b-specification-support-for-self-powered-devices-smarter-automation-and-streamlined-certification/>
