## 0.1.2

- `BUILD` and `BUILD_windows` run `rust/warrant-corpus-gen/tests/fixtures/dump_oracle_enumeration.py
  --check` after the lowering parity check: the oracle's `emit` at `full` depth (counts, `wraps`,
  descriptors, v1 and v2 digests, slim base set and every chunk line) of ct-compare and two
  synthetic documents is compared with the snapshots the Rust generator (CV2-S3b) reproduces
  byte for byte. A change to the oracle's enumeration turns this check red. No oracle source
  changed.
