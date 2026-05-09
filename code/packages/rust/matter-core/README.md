# matter-core

Matter application-layer primitives for the D23 smart-home runtime.

This crate contains no network, Thread, BLE, Wi-Fi, commissioning, CASE, PASE,
or fabric-storage I/O. It owns the small protocol vocabulary that adapters need
before they can project Matter nodes into `smart-home-core` records.

## Current Scope

- Matter fabric, node, endpoint, cluster, attribute, and command identifiers
- canonical cluster ids for the first integration families
- cluster-to-D23 capability projection
- selected Matter attribute reports mapped into `StateDelta`
- deterministic helpers for level, humidity, temperature, occupancy, lock, and
  thermostat values

## Out of Scope

- Matter secure sessions and certificates
- commissioning flows
- Thread border-router operations
- mDNS or DNS-SD discovery
- persistent fabric credential storage

## Development

```bash
bash BUILD
```
