# hdd-simulator

A dependency-free TypeScript implementation of the deterministic HDD teaching
model from [DB02](../../../specs/DB02-hdd-simulator.md). It uses two surfaces,
four tracks per surface, four sectors per track, and eight bytes per sector.
All 32 sectors begin as zero bytes.

## Use

```ts
import {HddSimulator} from "@coding-adventures/hdd-simulator";

const disk = new HddSimulator();
console.log(disk.readSector(0).data); // Uint8Array(8)
const timing = disk.writeSector(3, new TextEncoder().encode("learn-db"));
console.log(timing.totalTicks); // 3
console.log(new TextDecoder().decode(disk.readSector(3).data)); // learn-db
```

Clock values are nonnegative safe integers in abstract ticks, never seconds.
The geometry is fixed; seek costs three ticks per crossed track, rotation keeps
advancing during seek, and transfer takes one tick. Reads return a fresh
`Uint8Array`; writes copy the caller's array. `getState()` returns frozen,
deep-copied numeric arrays so a caller cannot mutate the model through a
snapshot. Invalid requests throw `InvalidLbaError` or
`InvalidSectorDataError`. A timing operation that would exceed
`Number.MAX_SAFE_INTEGER` raises `ClockOverflowError`. These errors leave all
state unchanged.

## Model limits

Each call completes immediately in model time, and call order is FIFO order.
This is not a physical-drive model and says nothing about RPM, actual timing,
OS caches, a filesystem, controller queues, concurrent requests, crash
recovery, or durability. The [shared JSON vectors](../../../specs/fixtures/database-media-v1/hdd-simulator.json)
pin exact behavior across languages.

## Contract evidence

- Vector tests check portable geometry, returned data, timing breakdown, state
  recurrence, and atomic errors.
- Initialization, buffer-copy, and validation checks pin media ownership and
  API boundaries.
- Bounded enumeration checks all 32 addresses and every four-phase/head/target/
  sector placement against a tick-by-tick oracle.
- Fixed-seed generated operations compare 250 accesses against an independent
  tick/media model and replay; fixed-seed malformed fuzz requests check atomic
  failure.

These finite checks support later formal verification but do not establish
correctness for other geometries or physical hardware.
