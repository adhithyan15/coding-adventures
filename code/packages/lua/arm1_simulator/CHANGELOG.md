# Changelog — coding-adventures-arm1-simulator (Lua)

## 0.1.0 — 2026-03-31

### Added
- Complete ARM1 (ARMv1) behavioral instruction set simulator in Lua 5.4
- All 16 condition codes with correct Boolean logic
- Barrel shifter: LSL, LSR (with #0=32 special case), ASR (with #0=32 special case), ROR, RRX
- Full 16-opcode ALU: AND, EOR, SUB, RSB, ADD, ADC, SBC, RSC, TST, TEQ, CMP, CMN, ORR, MOV, BIC, MVN
- 27-physical-register file with mode banking: USR/FIQ (R8-R14)/IRQ (R13-R14)/SVC (R13-R14)
- R15 as combined PC + NZCVIF status register (26-bit address space)
- ARM1 3-stage pipeline: PC = instruction_addr + 8 during execute
- Data processing, load/store, block transfer (LDM/STM), branch (B/BL), SWI instructions
- Rotated 8-bit immediate operand decoding
- Encoding helpers: encode_mov_imm, encode_alu_reg, encode_branch, encode_halt, encode_ldr, encode_str, encode_ldm, encode_stm
- Comprehensive test suite with >95% coverage
