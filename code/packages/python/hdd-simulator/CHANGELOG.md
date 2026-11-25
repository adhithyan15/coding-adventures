# Changelog — hdd-simulator

## 0.1.0

- Add fixed v1 HDD geometry with zero-filled sectors and LBA translation.
- Model deterministic seek, continuously advancing platter rotation, and
  one-tick sector transfer for synchronous reads and writes.
- Expose immutable timing results and full state snapshots; reject invalid
  requests without changing state.
- Add shared database-media v1 timing vectors and bounded exhaustive checks.
