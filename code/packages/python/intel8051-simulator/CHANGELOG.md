# Changelog — intel8051-simulator

## 0.1.0 (2026-05-04)

Initial release — Layer 07p.

### Added
- `I8051State` frozen dataclass with full Harvard memory model:
  256-byte internal RAM (includes SFRs at 0x80–0xFF), 64 KB code memory,
  64 KB external data memory
- PSW flag properties: `.cy`, `.ac`, `.ov`, `.parity`, `.bank`
- `I8051Simulator` implementing `Simulator[I8051State]` (SIM00 protocol)
- All 8051 addressing modes: register, direct, register-indirect (@Ri),
  immediate (#data), indexed (MOVC @A+DPTR, @A+PC), external (MOVX)
- Four register banks (bank 0–3 via PSW.RS1:RS0)
- Bit-addressable memory area (RAM 0x20–0x2F) and SFR bit addressing
- Double-operand data transfer: MOV in all 16 source/destination combinations,
  MOVX @DPTR/@Ri, MOVC @A+DPTR/@A+PC, PUSH/POP, XCH, XCHD
- Arithmetic: ADD, ADDC, SUBB (with borrow), INC/DEC (no flags), MUL AB,
  DIV AB, DA A (BCD decimal adjust)
- Logic: ANL, ORL, XRL (all forms), CLR A, CPL A, RL/RR/RLC/RRC/SWAP A
- Bit manipulation: CLR/SETB/CPL C, CLR/SETB/CPL bit, ANL/ORL C,bit,
  ANL/ORL C,/bit, MOV C,bit, MOV bit,C
- Branches: JZ, JNZ, JC, JNC, JB, JNB, JBC, CJNE (4 forms), DJNZ (2 forms)
- Jumps: LJMP, AJMP (11-bit page-relative), SJMP, JMP @A+DPTR
- Subroutines: LCALL, ACALL, RET, RETI, NOP
- HALT sentinel: opcode 0xA5 (undefined on real 8051)
- Parity (PSW.P) automatically recomputed after every ACC change
- Spec: `code/specs/07p-intel-8051-simulator.md`
