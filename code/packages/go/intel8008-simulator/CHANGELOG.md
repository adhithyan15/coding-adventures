# Changelog

## [0.1.0] - 2026-04-12

### Added

- Implemented complete Intel 8008 behavioral simulator (`Simulator` type)
- All instruction groups fully supported:
  - Group 0 (`00xxxxxx`): HLT, MVI, INR, DCR, RLC, RRC, RAL, RAR, OUT
  - Group 1 (`01xxxxxx`): MOV, IN, unconditional JMP/CAL, all 8 conditional JMP/CAL, HLT (0x76)
  - Group 2 (`10xxxxxx`): ADD, ADC, SUB, SBB, ANA, XRA, ORA, CMP (register and M pseudo-register)
  - Group 3 (`11xxxxxx`): ADI, SUI, ANI, XRI, ORI, CPI (immediate), RST, RET, conditional RET, HLT (0xFF)
- 8-level hardware push-down stack where entry 0 is always the current PC
- 14-bit program counter with correct wrapping
- M pseudo-register: reads/writes memory at address `((H & 0x3F) << 8) | L`
- 4 CPU flags: Carry, Zero, Sign, Parity (even parity → P=1)
- Parity computed as NOT(XOR of all 8 result bits)
- SUB/SBB borrow convention: CY=1 means borrow occurred
- Encoding conflict resolution matching MCS-8 manual:
  - `0x76` → HLT (not MOV M,M)
  - `0xFF` → HLT (not RST 7)
  - `0x7E` → CAL unconditional (not MOV A,M)
  - `0x7C` → JMP unconditional (not MOV A,H)
  - SSS=001 in group 01 → IN port (not MOV D,C)
  - Opcodes `0x40`–`0x5E` (even) → conditional JMP/CAL
- `Trace` type capturing PC, opcode, mnemonic, and decoded fields per instruction
- `Run()` method: calls `Reset()`, `LoadProgram()`, then loops calling `Step()` until halted or step limit reached
- `Reset()` clears all registers, flags, stack, and halted state
- `GetOutputPort(n)` reads the output latch for port n (written by OUT instruction)
- `PC()`, `Stack()`, `StackDepth()` accessors for external inspection
- 86.2% test coverage across 58+ test cases
- Tests cover: all ALU operations, all flag behaviors, all jump/call/return variants, M pseudo-register, IN/OUT, example programs (1+2, 4×5 multiply, absolute value via subroutine)
