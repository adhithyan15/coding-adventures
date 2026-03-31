# Changelog — coding-adventures-intel4004-gatelevel (Lua)

## 0.1.0 — 2026-03-31

Initial release. Lua port of the Elixir intel4004_gatelevel package.

### Added

- Gate-level Intel 4004 simulator where all arithmetic routes through logic gates
- ADD instruction uses ripple_carry_adder → full_adder chains → XOR/AND gates
- SUB uses complement-add: NOT gates on operand, then ripple_carry_adder
- INC/IAC use half-adder chains for incrementing
- CMA uses NOT gates on all 4 accumulator bits
- RAL/RAR use bit array manipulation (models actual shift logic)
- Registers, accumulator, carry, PC, and stack stored in D flip-flop states
  via Register() from logic_gates.sequential
- PC incremented via chain of 12 half-adders (models real hardware)
- 3-level hardware stack via flip-flop states
- RAM via flip-flop states (simulated Register storage)
- gate_count() returns educational gate estimates per component
- Cross-validation tests vs behavioral simulator
- Complete test suite with 95%+ coverage
