# Changelog

All notable changes to this project will be documented in this file.

## Unreleased

### Added

- Exact 1,428-DFF persistent-state topology and owned full-machine snapshots.
- A public container-derived storage inventory whose independently counted
  banks sum to the exact topology constant.
- Checked complete-state restore, transactional late-run rollback, and five
  compiler-checked hostile full-crate fixtures for the HALT authority boundary.
- Functional-oracle lockstep for every specified encoding and representative
  memory, port, branch, indirect, and stack workloads.
- Typed atomic failures for oversized ROM images, illegal opcodes, truncated
  two-byte instructions, and halted steps.

### Changed

- RAM output ports, selectors, ROM port, halt state, and stack pointer now use
  D flip-flops instead of host fields.
- HALT Q is now the sole authority across observation, snapshot, reset, program
  load, restore, step/run guards, `HLT`, and rollback; the private Boolean is a
  repairable diagnostic observer only.
- `load_program`, `step`, and `run` now return checked `Result` values.
- SRC masks its register selector to the architectural two bits.
- The differential covers all 256 first-byte classifications (240 valid and 16
  identical typed atomic faults). Strict checks pass and package-source line
  coverage is 96.69% (1,607/1,662).

### Lessons

- Structural mutation fixtures must anchor replacements to the complete owning
  method when an identical statement sequence can appear in multiple methods.
- Post-commit integrity checks must count every transition-helper call, not
  only calls carrying the expected value, so a later overwrite cannot hide
  behind a valid first clock.

## [0.1.0] - 2026-03-21

### Added

- Initial implementation of gate-level Intel 4004 CPU simulator in Rust.
- `bits` module: `int_to_bits()` and `bits_to_int()` for LSB-first bit vector conversion.
- `gate_alu` module: 4-bit ALU with add, subtract, complement, increment, decrement, bitwise AND/OR -- all routing through the arithmetic crate's real gate-level ALU.
- `registers` module: 16x4-bit RegisterFile, 4-bit Accumulator, and 1-bit CarryFlag, all built from D flip-flops via the logic-gates crate.
- `decoder` module: combinational instruction decoder using AND/OR/NOT gates to pattern-match all 4004 opcode families.
- `pc` module: 12-bit ProgramCounter with half-adder chain incrementer.
- `stack` module: 3-level x 12-bit HardwareStack with circular pointer and silent overflow (matching real 4004 behavior).
- `ram` module: 4 banks x 4 registers x 20 nibbles of RAM, each nibble stored in 4 D flip-flops.
- `cpu` module: Intel4004GateLevel top-level CPU implementing all 46 instructions with GateTrace execution logging.
- 88 unit tests + 4 doc tests covering all modules and instruction families.
- BUILD file for CI integration.
