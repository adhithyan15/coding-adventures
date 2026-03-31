# Changelog — CodingAdventures::ARM1Simulator (Perl)

## 0.1.0 — 2026-03-31

### Added
- Complete ARMv1 behavioral instruction set simulator in Pure Perl
- All 16 data processing instructions (AND, EOR, SUB, RSB, ADD, ADC, SBC, RSC, TST, TEQ, CMP, CMN, ORR, MOV, BIC, MVN)
- Barrel shifter: LSL, LSR, ASR, ROR (immediate and register-controlled), RRX (rotate right through carry)
- All 16 condition codes (EQ, NE, CS, CC, MI, PL, VS, VC, HI, LS, GE, LT, GT, LE, AL, NV)
- Single data transfer: LDR, STR with pre/post indexing and writeback
- Block data transfer: LDM, STM in all four modes (IA, IB, DA, DB) with register list
- Branch (B) and Branch with Link (BL) using 24-bit signed offset
- Software Interrupt (SWI) with mode switching; SWI 0x123456 as halt sentinel
- 4 processor modes with banked registers: USR, FIQ (R8–R14 banked), IRQ (R13–R14 banked), SVC (R13–R14 banked)
- ARM1 pipeline model: PC = PC+8 during execution (3-stage: Fetch/Decode/Execute)
- Unaligned word load rotation (ARM1 hardware quirk)
- Execution trace capturing instruction address, mnemonic, before/after register state, flags, memory accesses
- Encoding helpers: encode_mov_imm, encode_alu_reg, encode_alu_reg_shift, encode_branch, encode_ldr, encode_str, encode_ldm, encode_stm, encode_halt
- Comprehensive test suite: construction, reset, memory, all ALU ops, barrel shifter, all condition codes, load/store, block transfer, branches, SWI/halt, mode banking, end-to-end programs
