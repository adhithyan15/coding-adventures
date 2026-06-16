# Changelog — coding-adventures-z80-gatelevel

## [0.1.0] — 2026-06-15

### Added

- `bits.rs`: `int_to_bits8`, `int_to_bits16`, `bits_to_u8`, `bits_to_u16`,
  `compute_zero` (NOR tree), `compute_parity` (XOR tree + NOT),
  `invert_8bit` / `invert_16bit` (N NOT gates in parallel for two's complement),
  `add_8bit` (returns result, carry, half-carry at bit 3),
  `add_8bit_full` (returns full carry chain for overflow detection),
  `add_16bit` (returns result, carry, half-carry at bit 11)
- `alu.rs`: `AluResultZ80` with all six Z80 flag fields, plus:
  - `add8` / `sub8` — 8-bit add/subtract; overflow via XOR(carries[6], carries[7]);
    H = carry_3 (add) / NOT(carry_3) (sub); C = NOT(carry_7) for sub
  - `and8` (H=1 always — Z80 quirk), `or8` (H=0), `xor8` (H=0)
  - `inc8`, `dec8` — via adder; caller preserves C flag
  - `neg8` (0 - A), `cpl8` (NOT A; caller preserves S/Z/PV/C)
  - `daa8` — BCD correction for both ADD and SUB (unique to Z80; uses N flag)
  - `rlc8`, `rrc8`, `rl8`, `rr8`, `sla8`, `sll8` (undocumented), `sra8`, `srl8` (CB prefix)
  - `rlca8`, `rrca8`, `rla8`, `rra8` (accumulator rotates; only C affected)
  - `bit_test`, `set_bit`, `res_bit` — CB bit manipulation
  - `add16` (ADD HL,rp: only H/N/C affected), `adc16`, `sbc16` (ED: all flags)
- `registers.rs`: `RegisterFile` with:
  - Main 8-register bank (`regs[8]`; codes 0–7 with 6 = (HL) pseudo)
  - Alternate bank (`alt[8]`) for EXX/EX AF,AF'
  - Separate `f` and `f_prime` bytes (F register layout: S Z _ H _ PV N C)
  - `ix`, `iy` index registers (u16)
  - `read16_pair` / `write16_pair` for BC/DE/HL/SP
  - `exchange_af()` (EX AF,AF') and `exchange_bank()` (EXX)
  - `pack_f` / `unpack_f` free functions for F register encoding
- `cpu.rs`: `GateLevelCpuZ80` implementing the complete Z80 instruction set:
  - **Unprefixed**: LD (all modes), ALU, INC/DEC, ADD HL,rp, INC/DEC rp,
    PUSH/POP, CALL/RET/RST, JP/JR/DJNZ, EX variants, RLCA/RRCA/RLA/RRA,
    DAA, CPL, CCF, SCF, IN/OUT, DI/EI, NOP, HALT
  - **CB prefix**: 8 rotates × 8 registers + SLL (undocumented); BIT/SET/RES
  - **ED prefix**: ADC HL/SBC HL, LD rp/(nn), NEG, IM 0/1/2, RETI/RETN,
    LDI/LDD/LDIR/LDDR, CPI/CPD/CPIR/CPDR, IN r,(C)/OUT (C),r, LD A,I/R/LD I,A/LD R,A
  - **DD/FD prefix**: IX/IY load/store, ALU with (IX+d), INC/DEC (IX+d),
    ADD IX/IY, EX (SP),IX, JP (IX)
  - **DDCB/FDCB prefix**: all bit ops on (IX+d)/(IY+d), with optional copy to register
  - `Z80State` snapshot struct, `StepTrace` per-instruction trace
  - 256-port I/O arrays (input_ports / output_ports)
  - R register auto-increment (low 7 bits per fetch, bit 7 preserved)
- 58 unit tests + 8 doc tests; zero compiler warnings
