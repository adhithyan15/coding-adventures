# Changelog — coding-adventures-z80-simulator

All notable changes to this package are documented here.

## [0.1.0] — 2026-05-04

### Added

Initial release of the Zilog Z80 behavioral simulator (Layer 07k).

**Core files**

- `src/z80_simulator/state.py` — `Z80State` frozen dataclass with all Z80 registers:
  main bank (A/F/BC/DE/HL), alternate bank (A'/F'/BC'/DE'/HL'), index registers
  (IX/IY), special registers (SP/PC/I/R), interrupt state (IFF1/IFF2/IM),
  halted flag, and 65536-byte memory tuple.  Helper properties: `bc`, `de`, `hl`,
  `f_byte()`.

- `src/z80_simulator/flags.py` — Pure flag computation helpers:
  - `compute_sz(result)` → (S, Z)
  - `compute_parity(value)` → bool (even parity)
  - `compute_overflow_add / compute_overflow_sub`
  - `compute_half_carry_add / compute_half_carry_sub`
  - `pack_f / unpack_f` — pack/unpack the Z80 F register byte
  - `daa(a, flag_n, flag_h, flag_c)` — BCD decimal adjust accumulator

- `src/z80_simulator/simulator.py` — `Z80Simulator(Simulator[Z80State])`:
  Full SIM00 protocol implementation.

  *Unprefixed instructions*: NOP, HALT, LD r,r'/n/(HL), LD rp,nn, LD A,(BC/DE/nn),
  LD (BC/DE/nn),A, LD HL,(nn), LD (nn),HL, LD SP,HL, PUSH/POP rp,
  ADD/ADC/SUB/SBC/AND/OR/XOR/CP (all variants), INC/DEC r/rp,
  ADD HL,rp, RLCA/RRCA/RLA/RRA, DAA, CPL, CCF, SCF,
  JP nn/cc/HL, JR e/cc, DJNZ, CALL nn/cc, RET/cc, RST p,
  IN A,(n), OUT (n),A, DI, EI, EX AF,AF', EXX, EX DE,HL, EX (SP),HL.

  *CB prefix*: RLC/RRC/RL/RR/SLA/SRA/SLL/SRL on all 8 registers;
  BIT b,r; SET b,r; RES b,r.

  *ED prefix*: LD A,I/R; LD I,A; LD R,A; LD rp,(nn); LD (nn),rp;
  ADC HL,rp; SBC HL,rp; NEG; IM 0/1/2; RETI; RETN; RLD; RRD;
  IN r,(C); OUT (C),r;
  LDI/LDD/LDIR/LDDR; CPI/CPD/CPIR/CPDR;
  INI/IND/INIR/INDR; OUTI/OUTD/OTIR/OTDR.

  *DD/FD prefix (IX/IY)*: LD IX/IY,nn; LD IX/IY,(nn); LD (nn),IX/IY;
  LD SP,IX/IY; PUSH/POP IX/IY; ADD IX/IY,rp; INC/DEC IX/IY;
  LD r,(IX/IY+d); LD (IX/IY+d),r; LD (IX/IY+d),n;
  INC/DEC (IX/IY+d); ALU ops with (IX/IY+d); JP (IX/IY); EX (SP),IX/IY.

  *DDCB/FDCB prefix*: rotate/shift on (IX/IY+d);
  BIT/SET/RES on (IX/IY+d).

  *Interrupts*: `interrupt(data)` with IM 0 (RST p), IM 1 (0x0038), IM 2
  (vector table via I register); `nmi()` jumps to 0x0066 regardless of IFF1.

**Tests (14 test files, >80% coverage)**

- `test_flags.py` — unit tests for all flag helpers
- `test_protocol.py` — SIM00 protocol (reset/load/step/execute/get_state/I-O)
- `test_load_store.py` — all LD variants
- `test_arithmetic.py` — ADD/ADC/SUB/SBC/INC/DEC/NEG/DAA/ADC HL/SBC HL
- `test_logical.py` — AND/OR/XOR/CP/CPL/CCF/SCF
- `test_rotate_shift.py` — RLCA/RRCA/RLA/RRA + CB rotates/shifts + RLD/RRD
- `test_bit_ops.py` — BIT/SET/RES + DDCB bit ops
- `test_block_ops.py` — LDI/LDD/LDIR/LDDR/CPI/CPD/CPIR/CPDR
- `test_index_regs.py` — IX/IY load/arithmetic/displacement addressing
- `test_branch.py` — JP/JR/DJNZ/CALL/RET/RST
- `test_exchange.py` — EX AF,AF'/EXX/EX DE,HL/EX (SP),HL/LD A,I
- `test_io.py` — IN/OUT/OTIR/OTDR/INIR/INDR
- `test_interrupts.py` — DI/EI/IM 0-2/interrupt()/nmi()/RETN
- `test_programs.py` — sum 1..N, factorial, Fibonacci, LDIR copy, CPIR search

**Documentation**

- `README.md` — package overview, quick start, architecture highlights
- `CHANGELOG.md` — this file
- `code/specs/07k-z80-simulator.md` — full specification
- `code/specs/CPU-SIMULATOR-ROADMAP.md` — alternating roadmap from Z80 to AArch64/RISC-V
