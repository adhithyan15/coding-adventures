# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-21

### Added
- Initial release of the Intel 4004 gate-level simulator in Ruby.
- All computation routes through real logic gates (NOT, AND, OR, XOR).
- All state stored in D flip-flop registers via the logic_gates package.
- ALU operations (add, subtract, complement, increment, decrement) via the arithmetic package.
- 16 x 4-bit register file with pair read/write support.
- 4-bit accumulator and 1-bit carry flag.
- 12-bit program counter with half-adder incrementer.
- 3-level x 12-bit hardware call stack with mod-3 wrapping.
- RAM: 4 banks x 4 registers x (16 main + 4 status) nibbles.
- Instruction decoder using combinational gate logic.
- Full Intel 4004 instruction set support:
  - NOP, HLT
  - LDM, LD, XCH, INC
  - ADD, SUB
  - JUN, JCN, ISZ, JMS, BBL
  - FIM, SRC, FIN, JIN
  - I/O: WRM, WMP, WRR, WPM, WR0-WR3, SBM, RDM, RDR, ADM, RD0-RD3
  - Accumulator: CLB, CLC, IAC, CMC, CMA, RAL, RAR, TCC, DAC, TCS, STC, DAA, KBP, DCL
- Gate trace output for each instruction execution.
- Cross-validation test suite against behavioral simulator.
- Estimated gate count: ~1,014 gates (~4,056 transistors).
