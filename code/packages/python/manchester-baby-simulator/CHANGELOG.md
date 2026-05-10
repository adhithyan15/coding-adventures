# Changelog — coding-adventures-manchester-baby-simulator

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [0.1.0] — 2026-05-04

### Added

- `BabyState` frozen dataclass: 32-word store (tuple), 32-bit accumulator,
  5-bit CI, halted flag, plus `acc_signed` and `present_instruction` helpers.
- `BabySimulator` implementing the `Simulator[BabyState]` SIM00 protocol:
  - `reset()` — CI←31, A←0, store←zeros, halted←False
  - `load(program, origin=0)` — decode 4-byte little-endian chunks into
    store words; origin is in *word* units (0–31)
  - `step()` — pre-increment CI, fetch store[CI], decode S/F, execute,
    return `StepTrace`
  - `execute(program, max_steps=10_000)` — reset + load + step loop
  - `get_state()` — return immutable `BabyState` snapshot
- Full 7-instruction ISA: JMP, JRP, LDN, STO, SUB (opcodes 100 and 101),
  CMP (skip-if-negative), STP (halt)
- All arithmetic is 32-bit two's-complement with silent overflow (mod 2³²)
- Three test modules covering:
  - `test_protocol.py` — SIM00 contract (reset, load, step, execute, get_state)
  - `test_instructions.py` — all 7 opcodes with edge cases
  - `test_programs.py` — multi-instruction programs (negate, sum, loop,
    countdown, first-program divisor search excerpt)
- `py.typed` marker for PEP 561 inline types
