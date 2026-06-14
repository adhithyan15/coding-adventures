# Changelog — pdp11-simulator

## 0.1.0 (2026-05-04)

Initial release — Layer 07o.

### Added
- `PDP11State` frozen dataclass with 8 registers (R0–R7), PSW, memory, halted flag
- `PDP11Simulator` implementing `Simulator[PDP11State]` (SIM00 protocol)
- Full 8-addressing-mode evaluation (register, deferred, autoincrement,
  autoincrement deferred, autodecrement, autodecrement deferred, index, index deferred)
- PC-relative and absolute addressing via R7 modes 2, 3, 6, 7
- Double-operand instructions: MOV, CMP, BIT, BIC, BIS, ADD, SUB and byte variants
- Single-operand instructions: CLR, COM, INC, DEC, NEG, ADC, SBC, TST, SWAB,
  ROR, ROL, ASR, ASL and byte variants
- All 15 branch instructions: BR, BNE, BEQ, BGE, BLT, BGT, BLE, BPL, BMI,
  BHI, BLOS, BVC, BVS, BCC, BCS
- JMP, JSR (all addressing modes), RTS, SOB, HALT, NOP, RTI
- Condition code computation: N, Z, V, C with correct PDP-11 semantics
- Byte instruction autoincrement/decrement: always 2 for SP (R6) and PC (R7),
  1 for R0–R5
- `>80%` test coverage (target: 90%+)
- Spec: `code/specs/07o-pdp11-simulator.md`
