# Changelog

## [0.1.0] — 2026-05-14

### Added

Initial release of the x86-64 (AMD64) behavioral simulator (Layer 07w).

**Core architecture:**
- 16 × 64-bit GPRs (RAX RCX RDX RBX RSP RBP RSI RDI R8–R15)
- RIP (instruction pointer) and RFLAGS (CF/PF/ZF/SF/OF tracked)
- 64 KiB flat little-endian byte-addressed memory (wraps modulo 65 536)
- RSP initialised to 0xFFF8 on reset (stack-ready out of the box)
- HLT (0xF4) sentinel halts simulation

**Instruction encoding decoder:**
- Legacy prefix skip (F0/F2/F3/66/67/segment overrides)
- REX prefix (0x40–0x4F) with W/R/X/B bit field decoding
- Single-byte and 0x0F two-byte opcodes
- Full ModRM + SIB + displacement + immediate decoding
- All mod/rm addressing modes: reg–reg, [reg], [reg+disp8/32],
  [SIB+disp], [RIP+disp32]

**Supported instructions:**

| Group | Instructions |
|-------|-------------|
| Data transfer | MOV (r/m↔r, imm→r, imm32→r/m), MOVSX, MOVZX, MOVSXD, XCHG, LEA, PUSH/POP |
| Arithmetic | ADD, ADC, SUB, SBB, IMUL (2/3-operand), MUL, IDIV, DIV, NEG, INC, DEC, CMP |
| Logical | AND, OR, XOR, NOT, TEST |
| Shift/rotate | SHL/SAL, SHR, SAR, ROL, ROR (×1, ×CL, ×imm8) |
| Control flow | JMP (rel8/32/r/m), CALL (rel32/r/m), RET, all 16 Jcc (rel8/rel32), LOOP/LOOPE/LOOPNE, JRCXZ |
| String | REP STOSQ / REP STOSD |
| Bit ops | BSF, BSR, BT (r/imm), BSWAP |
| Cond move | CMOVcc (all 16 conditions) |
| Cond set | SETcc (all 16 conditions) |
| Misc | NOP, HLT |

**RFLAGS:**
- ADD/ADC: CF=unsigned overflow, OF=signed overflow, SF/ZF/PF from result
- SUB/SBB/CMP/NEG: borrow-complement CF convention
- AND/OR/XOR/TEST: CF=OF=0; SF/ZF/PF from result
- INC/DEC: SF/ZF/PF/OF updated; CF preserved
- Shifts: CF=last bit shifted out; correct OF for 1-bit shifts

**SIM00 protocol:**
- `reset()`, `load()`, `step()`, `execute()`, `get_state()`
- `set_input_port()`, `get_output_port()`, `interrupt()`, `nmi()` (all no-ops)
