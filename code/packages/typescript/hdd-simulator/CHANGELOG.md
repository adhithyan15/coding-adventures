# Changelog — hdd-simulator

## 0.1.0

- Add fixed v1 HDD geometry, zero-filled sectors, and LBA translation.
- Model synchronous sector I/O with exact seek, rotational wait, and transfer
  ticks, exposing immutable timing and deep snapshots.
- Copy Uint8Array data at read/write boundaries and reject invalid inputs
  without state changes.
- Consume shared cross-language vectors and add exhaustive and seeded tests.
