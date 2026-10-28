# Changelog

## 0.1.0 — 2026-10-26

- Drop the `ruff` lint lines from BUILD: ruff is only installed for Python
  packages, not on Rust build runners. The vector generator's `--check` run
  remains in BUILD.
- Add the configuration-pinned LatticeMico32 functional simulator from the
  accepted Layer 07bd specification.
- Model complete mutable GPR, interrupt/debug CSR, memory, and image state;
  all implemented base instructions; DC.RE exception remapping; and all 32
  latched-pending interrupt inputs.
- Add exact full-state and ordered bus-transaction traces, architectural
  exception completion, typed atomic host faults, bounded run/execute, and
  deterministic pre-work and post-candidate allocation failures.
- Check in 308 independently generated manual exact full-state/full-transaction
  transitions and a distinct 183-row RTL-topology corpus whose wire roster and
  control/storage constructs are re-parsed from five hash-pinned source files.
  Add 56 generated lifecycle/fault rollback rows, eight source-hash facts, and
  retain the 157 compact opcode/decode/operand rows that drive 672 extra ALU
  transitions.
- Trace actual replicated RTL bus values for SB/SH, exhaust every legal store
  lane on success and bus error, and accept all ignored-field classes for
  `rcsr`, `sextb`, and `sexth`.
- Define construction as the sole ordinary infallible allocation boundary;
  checked allocation and atomic rollback begin after an instance exists.
- Cover every branch outcome and displacement endpoint, legal top-memory
  access, documented alias, and typed fault under both DC.RE states; expose a
  checked-boundary fetch-address seam so `MisalignedFetch` has specific atomic
  precedence while strict restore continues to reject invalid snapshots.
- Pass 34 release integration tests, strict Clippy/rustdoc, and 94.33%
  measured release line coverage (882/935).
