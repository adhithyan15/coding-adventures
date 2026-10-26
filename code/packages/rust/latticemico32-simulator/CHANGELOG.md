# Changelog

## 0.1.0 — 2026-10-26

- Add the configuration-pinned LatticeMico32 functional simulator from the
  accepted Layer 07bd specification.
- Model complete mutable GPR, interrupt/debug CSR, memory, and image state;
  all implemented base instructions; DC.RE exception remapping; and all 32
  latched-pending interrupt inputs.
- Add exact full-state and ordered bus-transaction traces, architectural
  exception completion, typed atomic host faults, bounded run/execute, and
  deterministic pre-work and post-candidate allocation failures.
- Check in the independent 64-row manual primary-opcode matrix, 61-row
  pinned-RTL decode evidence corpus, and 32 operand seeds that drive 672
  complete-state manual-oracle transitions.
- Pass 26 release integration tests, strict Clippy/rustdoc, and 93.83%
  measured release line coverage (867/924).
