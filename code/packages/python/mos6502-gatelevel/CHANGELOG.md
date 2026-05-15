# Changelog — coding-adventures-mos6502-gatelevel

All notable changes to this package are documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [1.0.0] — 2026-05-15

### Added

- `bits.py` — `int_to_bits`, `bits_to_int`, `compute_zero`, `add_8bit`,
  `add_16bit`, `invert_8bit`; all routing through `ripple_carry_adder`
  from the `arithmetic` package.

- `alu.py` — `ALUResult6502` dataclass and full 8-bit ALU operations:
  - `add8`, `sub8` — addition and subtraction via full-adder chain
  - `and8`, `or8`, `xor8` — logical operations via 8 parallel gates
  - `asl8`, `lsr8` — shift operations via shift-register model
  - `rol8`, `ror8` — rotate-through-carry operations
  - `inc8`, `dec8` — increment/decrement via adder chain
  - `compare8` — CMP/CPX/CPY without storing result
  - `bit8` — BIT instruction (N and V from M[7:6], Z from A & M)
  - `daa_adc`, `daa_sbc` — NMOS BCD correction (N/V/Z from binary,
    C from BCD-corrected result — the NMOS quirk)

- `register_file.py` — `Register8`, `Register16`, `FlagRegister`, and
  `RegisterFile6502`; registers stored as D flip-flop arrays via the
  `register` function from `logic-gates`.

- `decoder.py` — `Decoder6502` with AND/NOT gate group decode (2-to-4
  decoder on cc bits) and a 151-entry PLA lookup table for all official
  NMOS 6502 opcodes.  `DecodedInstruction` dataclass.

- `simulator.py` — `MOS6502GateLevelSimulator` implementing the full
  `Simulator[MOS6502State]` protocol:
  - All 151 official NMOS 6502 opcodes
  - `reset()`, `load()`, `step()`, `execute()`, `get_state()`
  - `set_input_port()`, `get_output_port()` for memory-mapped I/O
  - `interrupt()` (maskable IRQ) and `nmi()` (non-maskable interrupt)
  - JMP indirect page-crossing bug accurately replicated
  - BCD mode (NMOS behavior: N/V/Z from binary, C from BCD)
  - BRK halts execution (sets `halted=True`); pushes PC+2 and P(B=1)
  - NMI loads 0xFFFA/B; IRQ loads 0xFFFE/F

- `tests/test_bits.py` — 40+ tests for bit conversion helpers
- `tests/test_alu.py` — 80+ tests for all ALU operations
- `tests/test_register_file.py` — 40+ tests for registers and flags
- `tests/test_decoder.py` — 40+ tests for instruction decode
- `tests/test_equivalence.py` — 50+ cross-validation programs against
  the behavioral `MOS6502Simulator`
- `tests/test_programs.py` — end-to-end programs: loops, subroutines,
  stack, BCD, indexed addressing, I/O
- `tests/test_simulator_coverage.py` — BRK/NMI/IRQ, JMP indirect bug,
  all addressing modes, flag edge cases

### Architecture notes

- All data-path arithmetic uses `full_adder` / `ripple_carry_adder` from
  the `arithmetic` package — no Python `+` or `-` on 8-bit data values.
- Stack pointer arithmetic (S ± 1) also routes through `add_8bit`.
- Zero-page address wrapping routes through `add_8bit` to stay gate-level.
- Only address bus operations (forming 0x0100 | S, checking I/O range)
  use Python bitwise operations — these are bus connections, not ALU ops.
