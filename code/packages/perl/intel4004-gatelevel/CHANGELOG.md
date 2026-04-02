# Changelog

## [0.01] - 2026-03-31

### Added

- `CodingAdventures::Intel4004GateLevel` — gate-level Intel 4004 simulator
  where every arithmetic and logical operation routes through gate functions
  from `CodingAdventures::LogicGates` and `CodingAdventures::Arithmetic`.

- **Bit conversion helpers** — `_int_to_bits($value, $width)` (LSB-first) and
  `_bits_to_int(\@bits)` for bridging between integers and gate-level bit arrays.

- **ALU helpers**:
  - `_gate_add($a, $b, $carry_in)` — 4-bit addition via `ripple_carry_adder`.
  - `_gate_not4($a)` — 4-bit bitwise NOT via four NOT gates.

- **Flip-flop register model** — all registers (accumulator, R0–R15, program
  counter, 3-level stack) stored as arrays of `new_flip_flop_state()` hashrefs.
  Two-phase clock writes (clock=0 captures master, clock=1 latches slave).

- **PC increment** — implemented as a 12-bit half-adder chain, modelling the
  real ripple-carry incrementer in the 4004.

- **ISZ and INC** — increment via 4-bit half-adder chain (same circuit used for
  the PC incrementer, applied to register values).

- **IAC** — implemented as `gate_add(A, 0, carry_in=1)`.

- **CMC** — implemented using the `NOT` gate from logic_gates.

- **CMA** — four NOT gates applied to each accumulator bit.

- **DAA** — BCD adjust via `gate_add(A, 6, 0)` when A > 9 or carry is set.

- **RAM** — flat keyed hash of flip-flop state arrays; WRM/RDM/SBM/ADM all
  read through the gate-level state; WR0–WR3/RD0–RD3 for status characters.

- `gate_count()` — returns a hash of approximate gate counts per component,
  totalling 716 (close to the 4004's ~786 estimated logic gates).

- Comprehensive Test2::V0 test suite in `t/test_intel4004_gatelevel.t` covering
  every instruction that differs meaningfully from the behavioral path — including
  gate-level ADD, SUB, INC, ISZ, IAC, CMC, CMA, RAL, RAR, DAA, WRM/RDM, JMS/BBL
  stack operations, and the 3×4 ISZ loop integration test.
