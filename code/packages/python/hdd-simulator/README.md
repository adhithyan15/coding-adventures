# hdd-simulator

A dependency-free, synchronous teaching model for hard-disk addressing and
mechanical timing. The package has one fixed geometry: two surfaces, four
tracks per surface, four sectors per track, and eight bytes per sector. It
starts with zero-filled media and exposes one-sector LBA reads and writes.

## Where it fits

This is the first executable media layer in the database and hardware learning
sequence. It gives later storage engines a precise sector interface and a small
clock model. It does not model a filesystem, controller, OS cache, queue depth,
concurrent requests, crash recovery, or a physical drive. Each call completes
immediately in model time, and caller order is FIFO order.

## Example

```python
from hdd_simulator import HddSimulator

disk = HddSimulator()
print(disk.read_sector(0).data)  # b'\x00' * 8
timing = disk.write_sector(3, b"learn-db")
print(timing.total_ticks)        # 3 ticks
print(disk.read_sector(3).data) # b"learn-db"
print(disk.get_state().tick)     # 8 ticks after the read-back
```

A seek costs three ticks per track crossed. Rotation continues during seeking;
sector phase is `clock % 4`. Waiting for the target phase and the one-tick
transfer are included in the immutable `IoTiming`. Ticks are abstract integers,
not seconds. See [DB02](../../../specs/DB02-hdd-simulator.md) for the full contract
and [shared vectors](../../../specs/fixtures/database-media-v1/hdd-simulator.json)
for portable exact examples.

Invalid LBA values and malformed write payloads raise `InvalidLbaError` and
`InvalidSectorDataError`. An operation that would exceed `MAX_SAFE_TICK` raises
`ClockOverflowError`. These errors leave the complete disk state unchanged.
Snapshots and timing records are frozen values, and returned sector data is
immutable `bytes`.

## Checking the contract

`tests/test_hdd_simulator.py` is arranged around contract invariants:

- Shared JSON vectors pin exact address, data, timing, state, and error behavior
  for cross-language reimplementation.
- Initialization and snapshot tests pin zero-filled media, profile dimensions,
  and immutable snapshot ownership.
- Boundary and malformed-input tests pin valid LBA/data limits and prove failed
  requests preserve clock, head, and all sectors.
- Bounded exhaustive checks enumerate all 32 LBAs and every phase/head/target/
  sector placement, with a tick-by-tick oracle independent of the timing formula.
- A fixed-seed generated sequence checks data, timing, and state recurrence over
  250 reads and writes; fixed-seed malformed cases exercise input rejection.

These checks are bounded examples, not a proof for arbitrary geometries or a
physical timing guarantee. They are useful conformance evidence for a future
formal model, not a substitute for one.

## Install

```sh
pip install coding-adventures-hdd-simulator
```
