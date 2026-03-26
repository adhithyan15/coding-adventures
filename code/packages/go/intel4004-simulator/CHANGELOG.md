# Changelog

## [0.2.0] - 2026-03-21

### Added
- Implemented all 46 Intel 4004 instructions (up from 5 MVP instructions)
- Jump instructions: JUN, JCN, JMS, JIN, ISZ with proper 2-byte encoding
- Subroutine support: JMS/BBL with 3-level hardware call stack (wraps mod 3)
- Register pair operations: FIM, SRC, FIN for 8-bit data handling
- Register operations: LD, INC for direct register access
- I/O instructions: WRM, WMP, WRR, WPM, WR0-WR3, SBM, RDM, RDR, ADM, RD0-RD3
- Accumulator operations: CLB, CLC, IAC, CMC, CMA, RAL, RAR, TCC, DAC, TCS, STC, DAA, KBP, DCL
- Full RAM subsystem: 4 banks x 4 registers x (16 main + 4 status) nibbles
- RAM output ports (one per bank, written by WMP)
- ROM I/O port (WRR/RDR)
- RAM bank selection via DCL instruction
- RAM addressing via SRC instruction (register + character selection)
- Hardware call stack with 3-level depth and silent overflow wrapping
- NOP instruction support
- Reset() method to clear all CPU state
- Raw2 field in Intel4004Trace for 2-byte instruction tracing
- Comprehensive encoder helpers for all instruction types
- 98.6% test coverage with tests for every instruction

### Changed
- SUB now uses correct complement-add semantics: A + ~Rn + borrow_in (carry=true means NO borrow)
- ADD now includes carry-in for proper multi-digit BCD chaining
- Registers changed from slice to fixed [16]int array
- Intel4004Simulator struct expanded with HwStack, StackPointer, RAM, RAMStatus, RAMOutput, RAMBank, RAMRegister, RAMCharacter, ROMPort fields
- Step() now detects and fetches 2-byte instructions automatically
- LoadProgram() now clears ROM before loading

## [0.1.0] - Unreleased

### Added
- Developed `Intel4004Simulator` disconnected structurally from explicit CPU generics due to inherent architecture differences (i.e constraints towards 4-bits solely).
- Implemented core Accumulator mapping arrays `LDM`, `XCH`, `ADD`, `SUB`.
- Traces fully reveal Accumulator manipulations alongside standard `Carry` indicators allowing observation of numeric rollovers.
- Documentation natively emphasizes how limiting logic boundaries dictated 6 unique execution steps comparative to more modern optimizations evaluating within single execution cycles for mathematical evaluations.
