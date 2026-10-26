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
- Check in 265 independently generated manual and 265 separately reproducible
  pinned-RTL exact full-state/full-transaction transitions, 52 generated
  lifecycle/fault rollback rows, eight source-hash facts, and retain the 157
  compact opcode/decode/operand rows that drive 672 extra ALU transitions.
- Trace actual replicated RTL bus values for SB/SH, exhaust every legal store
  lane on success and bus error, and accept all ignored-field classes for
  `rcsr`, `sextb`, and `sexth`.
- Define construction as the sole ordinary infallible allocation boundary;
  checked allocation and atomic rollback begin after an instance exists.
- Pass 32 release integration tests, strict Clippy/rustdoc, and 94.18%
  measured release line coverage (874/928).
