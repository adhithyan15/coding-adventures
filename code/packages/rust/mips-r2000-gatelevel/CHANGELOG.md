# Changelog

## [0.1.0] — 2026-06-16

Initial release: gate-level MIPS R2000 (1985) simulator in Rust.

### Added

- `bits.rs` — integer ↔ LSB-first bit-vector conversion (`int_to_bits32`,
  `bits_to_u32`, `int_to_bits64`, `bits_to_u64`); 32-bit ripple-carry adder
  (`add_32bit`) with separate 31-bit and 33-bit sub-adders to extract
  carry_into_bit31 and carry_out_of_bit31 for signed overflow detection;
  64-bit ripple-carry adder (`add_64bit`) for MULT accumulation;
  `invert_32bit` (32 NOT gates in parallel); `shl_32`, `shr_32_logical`,
  `shr_32_arith` (barrel-shifter models via bit-list manipulation);
  `compute_zero` (NOR reduction tree).

- `alu.rs` — `AluResult32` struct (`result`, `carry`, `overflow`, `zero`,
  `negative`); arithmetic: `add32` (32-bit addition via ripple-carry adder;
  overflow via XOR(carry_into_31, carry_out_of_31)), `sub32` (A+NOT(B)+1;
  carry=0 means borrow); bitwise: `and32`, `or32`, `xor32`, `nor32` (32
  gate instances in parallel); comparison: `slt32` (XOR(N,V) from sub32),
  `sltu32` (NOT(carry) from sub32); shifts: `sll32`, `srl32`, `sra32`;
  multiply: `multu32` (shift-and-add, 32 iterations, 64-bit result),
  `mult32` (sign-handled; magnitude via multu32, negate 64-bit if XOR of
  signs); divide: `divu32` (non-restoring long division, 32 iterations),
  `div32` (signed; magnitude via divu32, apply MIPS sign conventions).

- `register_file.rs` — `RegisterFile32` with `gprs: [[u8;32]; 32]` + `hi`,
  `lo`, `pc` as LSB-first bit arrays; `read_reg`/`write_reg` with R0 guard
  (writes to R0 silently discarded, reads always return 0); `read/write_hi`,
  `read/write_lo`, `read/write_pc`; `increment_pc(by)` via gate-level
  `add_32bit`.

- `decoder.rs` — `decode_instruction(word: u32) → DecodedInstruction`;
  gate-level field extraction: all fields extracted from `int_to_bits32` +
  sub-slice + `bits_to_u32`; R-type (op=0) extracts rs, rt, rd, shamt,
  funct from bits[25:21], [20:16], [15:11], [10:6], [5:0]; J-type (op=2,3)
  extracts target26 from bits[25:0]; I-type extracts imm16 sign-extended
  via bit 15 replication; `InstrFormat` enum (R, I, J).

- `cpu.rs` — `CpuMipsR2000` with `rf: RegisterFile32`, `mem: Vec<u8>`
  (64 KB big-endian), `halted: bool`; `new()`/`reset()`/`load()`/`execute()`/
  `step()` lifecycle; full instruction dispatch for all MIPS I instructions:
  R-type (SLL/SRL/SRA/SLLV/SRLV/SRAV, JR/JALR, MFHI/MTHI/MFLO/MTLO,
  MULT/MULTU, DIV/DIVU, ADD/ADDU/SUB/SUBU, AND/OR/XOR/NOR, SLT/SLTU,
  BREAK); J-type (J, JAL); I-type (BLTZ/BGEZ/BLTZAL/BGEZAL, BEQ/BNE/
  BLEZ/BGTZ, ADDI/ADDIU/SLTI/SLTIU, ANDI/ORI/XORI, LUI, LB/LBU/LH/LHU/
  LW/LWL/LWR, SB/SH/SW/SWL/SWR); halt sentinel SYSCALL (op=0,funct=0x0C);
  `MipsError` enum (SignedOverflow, Misalignment, Break, UnknownOpcode);
  33 unit tests + 21 doctests (54 total, 100% pass).

### Architecture notes

- MIPS I ISA: 32-bit fixed-width instructions, three formats (R/I/J),
  32 GPRs plus HI/LO/PC, no condition codes.
- Big-endian memory (64 KB flat); word/halfword accesses must be aligned.
- No delay slots simulated — PC is already past the branch instruction when
  targets are computed.
- ADD/ADDI/SUB trap on signed overflow (`MipsError::SignedOverflow`);
  ADDU/ADDIU/SUBU wrap silently.
- Overflow detection: `XOR(carry_into_bit31, carry_out_of_bit31)` via
  separate 31-bit and 33-bit sub-adders.
- MULT/MULTU: 32-iteration shift-and-add; accumulates 64-bit product via
  `add_64bit`; result placed in HI:LO.
- DIV/DIVU: 32-iteration non-restoring long division; quotient in LO,
  remainder in HI.  Division by zero returns (0xFFFF_FFFF, dividend).
- LWL/LWR/SWL/SWR: unaligned load/store with big-endian byte-merge logic.
- R0 ($zero): reads always return 0; writes are silently discarded.
- Halt: SYSCALL (op=0, funct=0x0C) sets `halted=true`; all register state
  preserved for post-halt inspection.
