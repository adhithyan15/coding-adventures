# Changelog

## [0.1.0] — 2026-06-16

### Added

- Initial gate-level SPARC V8 simulator in Rust.
- `bits.rs`: LSB-first bit-vector helpers (`u32_to_bits`, `bits_to_u32`,
  `u64_to_bits`, `bits_to_u64`), bitwise gate helpers (`and_32`, `or_32`,
  `xor_32`, `not_32`, `andn_32`, `orn_32`, `xnor_32`), shift helpers
  (`sll_32`, `srl_32`, `sra_32`), sign-extension (`sext13`, `sext22`,
  `sext30`), 32-bit ripple-carry arithmetic (`add_32`, `add_32c`, `sub_32`,
  `sub_32b`), signed overflow detection (`overflow_add`, `overflow_sub`),
  and zero-flag helper (`compute_zero`).
- `alu.rs`: Full ALU with condition-code updates.  Arithmetic: ADD, ADDcc,
  ADDX, ADDXcc, SUB, SUBcc, SUBX, SUBXcc.  Logical: AND, ANDcc, ANDN,
  ANDNcc, OR, ORcc, ORN, ORNcc, XOR, XORcc, XNOR, XNORcc.  Shifts: SLL,
  SRL, SRA.  Multiply: UMUL, SMUL (shift-and-add via ripple-carry adder,
  full 64-bit product).  Divide: UDIV (64÷32 with saturation at
  `0xFFFF_FFFF`), SDIV (saturating signed divide).  MULScc (iterative
  multiply step).  SETHI.
- `register_file.rs`: 56-physical-register file with sliding register windows
  (`NWINDOWS=3`).  `virt_to_phys()` maps logical registers to physical for
  any CWP.  `SAVE`/`RESTORE` with save-depth tracking and window overflow
  detection.  PSR condition-code fields (N, Z, V, C).  Y register.
- `decoder.rs`: Decodes all four SPARC instruction formats.  Format 1 (CALL).
  Format 2 (SETHI, Bicc, NOP).  Format 3 register and immediate forms (ALU,
  Load, Store, Ticc/JMPL/SAVE/RESTORE).
- `cpu.rs`: `SparcCpu` struct with 64 KiB flat memory.  `load()` validates
  origin and length before mutating state (returns `Result`, never panics).
  `execute()` runs up to `max_steps`.  `step()` fetches, decodes, dispatches.
  `fetch()` masks each byte individually for safe wrapping at address
  `0xFFFC`–`0xFFFF`.  Halts on `ta 0` (`0x91D0_2000`).  30+ unit tests
  covering: halt, SETHI, ADD/SUB (reg and imm), logical ops, shifts,
  load/store, branches (BA/BE/BNE), JMPL, UMUL/SMUL, UDIV, SAVE/RESTORE,
  g0 discard, load validation, fetch-wrap, NOP.
- `README.md`: Architecture overview, feature list, register window diagram,
  usage example, halt convention, memory model.
- `BUILD`: `cargo test` invocation for the build tool.
- `Cargo.toml`: crate metadata with dependencies on `logic-gates` and
  `arithmetic`.
