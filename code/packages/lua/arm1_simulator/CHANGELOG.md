# Changelog — coding-adventures-arm1-simulator (Lua)

## 0.1.0 — 2026-03-31

### Added
- Complete ARMv1 behavioral instruction set simulator
- All 16 data processing instructions (AND, EOR, SUB, RSB, ADD, ADC, SBC, RSC, TST, TEQ, CMP, CMN, ORR, MOV, BIC, MVN)
- Barrel shifter: LSL, LSR, ASR, ROR (immediate and register), RRX
- All 16 condition codes (EQ, NE, CS, CC, MI, PL, VS, VC, HI, LS, GE, LT, GT, LE, AL, NV)
- Single data transfer: LDR, STR, LDRB, STRB with pre/post indexing, add/subtract, writeback
- Block data transfer: LDM, STM in all four modes (IA, IB, DA, DB)
- Branch (B) and Branch with Link (BL)
- Software Interrupt (SWI) with mode switching; SWI 0x123456 as halt
- 4 processor modes with banked registers: USR, FIQ (R8-R14 banked), IRQ (R13-R14 banked), SVC (R13-R14 banked)
- ARM1 pipeline model: PC = PC+8 during execution (3-stage: Fetch/Decode/Execute)
- Unaligned word load rotation (ARM1 quirk)
- Execution trace capturing before/after register state, flags, memory accesses
- Encoding helpers: encode_mov_imm, encode_alu_reg, encode_alu_reg_shift, encode_branch, encode_ldr, encode_str, encode_ldm, encode_stm, encode_halt
- Comprehensive test suite: unit tests for each instruction category + end-to-end programs
