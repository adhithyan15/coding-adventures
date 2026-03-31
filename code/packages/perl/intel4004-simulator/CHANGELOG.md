# Changelog

## [0.01] - 2026-03-31

### Added

- `CodingAdventures::Intel4004Simulator` — complete behavioral simulator for the
  Intel 4004 microprocessor implementing all 46 real instructions plus `HLT`.

- **Data movement**: `LDM`, `LD`, `XCH`, `FIM`, `SRC`, `FIN`, `JIN`.

- **Arithmetic**: `ADD` (with carry-in), `SUB` (complement-add method),
  `INC`, `IAC`, `DAC`, `ADM`, `SBM`.

- **Carry / flag ops**: `CLB`, `CLC`, `STC`, `CMC`, `TCC`, `TCS`.

- **Shifts and complement**: `RAL` (rotate left through carry), `RAR` (rotate
  right through carry), `CMA` (complement accumulator).

- **BCD**: `DAA` (decimal adjust), `KBP` (keyboard process / 1-hot to binary).

- **Flow control**: `NOP`, `HLT`, `JUN` (unconditional 12-bit jump), `JCN`
  (conditional jump with 4-bit condition code), `JMS` (subroutine call),
  `BBL` (return and load), `ISZ` (increment and skip if zero).

- **RAM I/O**: `WRM`/`RDM` (main characters), `WMP` (output port),
  `WR0–WR3`/`RD0–RD3` (status characters), `ADM`/`SBM` (RAM arithmetic).

- **ROM I/O**: `WRR`/`RDR` (ROM port), `WPM` (write program RAM, stub).

- **Bank select**: `DCL` (designate command line — selects RAM bank 0–3).

- `run($program, $max_steps)` — loads and executes a program, returns arrayref
  of trace hashrefs (address, mnemonic, before/after accumulator and carry).

- `step()` — single-step execution returning one trace hashref.

- `reset()` — restores all state to power-on defaults.

- Comprehensive Test2::V0 test suite in `t/test_intel4004_simulator.t` covering
  every instruction, carry semantics, BCD operations, subroutine call/return,
  ISZ loop patterns, FIN/JIN indirect addressing, and integration programs
  (1+2=3, 3×4=12 via ISZ loop).
