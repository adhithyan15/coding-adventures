# Changelog — coding-adventures-mos6502-gatelevel

## [0.1.0] — 2026-06-15

### Added

- `bits.rs`: `int_to_bits8`, `int_to_bits16`, `bits_to_u8`, `bits_to_u16`,
  `compute_zero` (NOR tree), `not8` (8 NOT gates in parallel),
  `add_8bit_full` (full 8-stage adder returning all carries for overflow detection),
  `add_8bit` (convenience wrapper), `add_16bit` (16-stage adder for PC/address arithmetic)
- `alu.rs`: `GateAlu6502` operations:
  - `add8` / `sub8` — 8-bit add/subtract with N/V/Z/C flags; overflow via XOR(carries[6], carries[7])
  - `and8`, `or8`, `xor8` — bitwise ops through 8 gate-per-bit chains
  - `bit8` — BIT test: N=M[7], V=M[6], Z=(A&M)==0
  - `compare8` — CMP/CPX/CPY via A + NOT(M) + 1; sets N/Z/C
  - `asl8`, `lsr8` — shift left/right; `rol8`, `ror8` — rotate through carry
  - `inc8`, `dec8` — increment/decrement via adder; no C flag update
  - `adc_bcd`, `sbc_bcd` — NMOS BCD correction (N/V/Z from binary; C from BCD)
- `decoder.rs`: 2-to-4 AND/NOT group decoder + full 151-opcode PLA lookup table;
  `decode()`, `is_legal()`, `is_branch()` functions; 13 addressing mode constants
- `registers.rs`: `Register8` (8 D flip-flops), `Register16` (16 D flip-flops, PC),
  `FlagRegister` (7 separate flag flip-flops; `pack/unpack` with B-override for BRK/PHP),
  `RegisterFile6502` with stack push/pull via 8-bit adder (page 0x01xx)
- `cpu.rs`: `GateLevelCpu` implementing all 151 NMOS 6502 instructions:
  - All 13 addressing modes: IMM, ZP, ZPX, ZPY, ABS, ABX, ABY, INX, INY, IMP, ACC, REL, IND
  - NMOS JMP indirect page-wrap bug (high byte from `$xx00` not `$xx01`)
  - BCD mode for ADC/SBC (D flag)
  - IRQ and NMI interrupt handlers
  - Memory-mapped I/O: reads from 0xFF00–0xFFEF → input_ports; writes → output_ports
- `CpuState` snapshot struct, `StepTrace` per-instruction trace
- 81 unit tests + 7 doc-tests
