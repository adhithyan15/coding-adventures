# Changelog

All notable changes to `ibm-system360-simulator` are documented here.

## 0.1.0 — 2026-10-24

### Added

- Complete checked CPU lifecycle with owned architectural state, strict raw-byte
  program transport, bounded execution, exact traces, and typed atomic faults.
- Historical RR, RX, RS, SI, and SS instruction decoding with 24-bit effective
  addresses and big-endian halfword, fullword, and doubleword storage.
- Non-privileged fixed-point, logical, character, storage-to-storage, branch,
  register-pair, and shift instruction families.
- Four raw 64-bit floating registers and exact integer-fraction implementations
  of short and long System/360 hexadecimal floating-point operations.
- Fifteen focused tests with 100% function and 93.20% line coverage.
