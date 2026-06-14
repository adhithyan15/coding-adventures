# Changelog

All notable changes to `coding-adventures-z80-gatelevel` are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [0.1.0] — 2026-05-15

### Added

**Package: `coding-adventures-z80-gatelevel` (Layer 07k2)**

Initial release of the Z80 gate-level simulator. Every ALU operation
routes through primitive gate functions (`AND`, `OR`, `XOR`, `NOT`,
`full_adder`, `ripple_carry_adder`) from `coding-adventures-logic-gates`
and `coding-adventures-arithmetic`. No Python integer arithmetic appears
in the ALU path.

#### Architecture

- **`bits.py`** — Bit array helpers: `int_to_bits`, `bits_to_int`,
  `compute_parity`, `compute_zero`, `add_8bit`, `add_16bit`,
  `invert_8bit`, `invert_16bit`. All use gate primitives only.

- **`alu.py`** — Gate-level 8/16-bit ALU:
  - `add8`, `sub8`, `and8`, `or8`, `xor8`, `inc8`, `dec8`
  - `neg8`, `cpl8`, `daa8`
  - `rlc8`, `rrc8`, `rl8`, `rr8`, `sla8`, `sra8`, `srl8`
  - `rlca8`, `rrca8`, `rla8`, `rra8` (accumulator rotate variants)
  - `bit_test`, `set_bit`, `res_bit`
  - `add16`, `adc16`, `sbc16` (16-bit ADC/SBC for `ED` prefix)
  - All operations return `ALUResultZ80` with result + all Z80 flags

- **`register_file.py`** — Register storage via D flip-flop arrays:
  - `Register8` — 8 `register()` cells from `logic_gates`
  - `Register16` — 16 `register()` cells; inc/dec via gate-level adder
  - `RegisterFile` — Main bank (A–L+F), alt bank (A'–L'+F'), IX, IY
  - `pack_f` / `unpack_f` — F register: `S Z 0 H 0 PV N C`
  - `exchange_af()`, `exchange_bank()` — EX AF,AF' and EXX support

- **`decoder.py`** — Combinational instruction decoder using AND/NOT gates:
  - Group detection for unprefixed opcodes via `AND(NOT(b7), NOT(b6))`
  - `decode_cb`, `decode_ed`, `decode_dd_fd` for prefixed instructions
  - `DecoderOutput` frozen dataclass with decoded fields

- **`simulator.py`** — `Z80GateLevelSimulator` implementing
  `Simulator[Z80State]`:
  - All unprefixed Z80 instructions
  - `CB` prefix: RLC/RRC/RL/RR/SLA/SRA/SRL r, BIT/SET/RES b,r
  - `ED` prefix: ADC/SBC HL,rp; NEG; LD A,I/R; LD I/R,A; IM 0/1/2;
    RETI/RETN; LDI/LDD/LDIR/LDDR; CPI/CPD/CPIR/CPDR; IN r,(C); OUT (C),r
  - `DD`/`FD` prefix: full IX/IY instruction set including indexed
    load/store, ALU, INC/DEC, PUSH/POP, JP (IX), EX (SP),IX
  - `DDCB`/`FDCB` prefix: BIT/SET/RES/rotate on `(IX+d)`/`(IY+d)`
  - Block memory copy (LDIR/LDDR) and search (CPIR/CPDR)
  - I/O ports: IN A,(n), OUT (n),A, IN r,(C), OUT (C),r
  - R register auto-increment (low 7 bits) on each instruction fetch
  - Z80 power-on state: F = 0xFF (all flags set)
  - `execute()` preserves I/O port state across internal reset

#### Testing

- **355 tests** across 7 test modules
- **96.16% line coverage** (well above 80% requirement)
- `test_bits.py` — bit helper functions
- `test_alu.py` — all ALU operations with flag verification
- `test_register_file.py` — Register8/16/RegisterFile including exchanges
- `test_decoder.py` — decoder for all prefix types
- `test_programs.py` — end-to-end programs (loops, stack, 16-bit arithmetic)
- `test_equivalence.py` — cross-validates against behavioral `Z80Simulator`
  for identical register/flag output on the same bytecode
- `test_simulator_coverage.py` — targeted coverage of IX/IY, ED, DDCB,
  block ops, conditional branches, I/O, exchange, and error paths

#### Key design notes

- **Two's complement subtraction**: `A - B = A + NOT(B) + 1`. The H flag
  is inverted from the adder's half-carry; C flag is inverted from
  carry-out. This mirrors real Z80 silicon.

- **Z80 vs Intel 8080 flags**: Z80 adds N flag (1 after subtraction) and
  uses P/V dually for parity (logical ops) and overflow (arithmetic ops).

- **Gate count for ADD A, B**: ~55 gate operations total (6 decode +
  40 full-adder + 8 zero-detect + 1 overflow XOR). A behavioral simulator
  executes the same instruction in 1 Python bytecode.

- **Cross-compatibility**: `Z80GateLevelSimulator` and `Z80Simulator`
  both implement `Simulator[Z80State]` and produce bit-for-bit identical
  output for all supported instructions.

#### Dependencies

- `coding-adventures-logic-gates` — AND, OR, XOR, NOT, register()
- `coding-adventures-arithmetic` — full_adder, ripple_carry_adder
- `coding-adventures-simulator-protocol` — Simulator, ExecutionResult, StepTrace
- `coding-adventures-z80-simulator` — Z80State (shared output type)
