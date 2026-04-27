# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-04-12

### Added

- Complete Intel 8008 behavioral simulator implementing all 48 instruction groups:
  - **MOV** (01 DDD SSS) — register-to-register copy including M pseudo-register
  - **MVI** (00 DDD 110) — move immediate byte into register or memory
  - **INR/DCR** — increment/decrement register (updates Z, S, P; preserves CY)
  - **ALU register** (10 OOO SSS) — ADD, ADC, SUB, SBB, ANA, XRA, ORA, CMP
  - **ALU immediate** (11 OOO 100) — ADI, ACI, SUI, SBI, ANI, XRI, ORI, CPI
  - **Rotates** — RLC, RRC, RAL, RAR (circular and through-carry)
  - **Jumps** — JMP (unconditional), JFC/JTC/JFZ/JTZ/JFS/JTS/JFP/JTP (conditional)
  - **Calls** — CAL (unconditional), CFC/CTC/CFZ/CTZ/CFS/CTS/CFP/CTP (conditional)
  - **Returns** — RET (unconditional), RFC/RTC/RFZ/RTZ/RFS/RTS/RFP/RTP (conditional)
  - **RST** — 1-byte restart instructions (interrupt vectors at 0x00, 0x08, …, 0x38)
  - **IN/OUT** — I/O port instructions (8 input ports, 24 output ports)
  - **HLT** — processor halt (0x76 and 0xFF encodings)
- 8-level push-down hardware stack modeled correctly:
  - Stack entry[0] is always the live program counter
  - CALL rotates stack down; RETURN rotates up
  - Maximum 7 active nested calls (8th level silently overwrites oldest)
- 14-bit program counter and 16 KiB address space
- M pseudo-register for indirect memory access via (H & 0x3F) << 8 | L
- `Flags` struct with carry, zero, sign, parity fields
- `Trace` struct capturing full before/after state for each instruction
- `Simulator::run()` convenience method: loads program, resets state, collects traces
- 31 unit tests covering arithmetic, logic, rotates, jumps, calls, memory, I/O, flags
- Documented encoding quirks where control flow opcodes overlap MOV patterns
  (0x76=HLT, 0x7E=CAL, 0x7C=JMP, 0x79=IN 7)
