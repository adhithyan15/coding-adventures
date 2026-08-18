//! Opcode / funct-field constant tables for the MIPS R2000 ISA.
//!
//! MIPS R2000 instructions are always 32 bits wide and come in three
//! formats:
//!
//! ```text
//! R-type:  op(6) rs(5) rt(5) rd(5) shamt(5) funct(6)
//! I-type:  op(6) rs(5) rt(5) imm16(16)
//! J-type:  op(6) target26(26)
//! ```
//!
//! `op` (bits 31:26) selects the instruction format/family.  For
//! R-type instructions (`op == 0`), `funct` (bits 5:0) selects the
//! specific operation.  `REGIMM` (`op == 1`) is a second dispatch
//! family selected by the `rt` field instead of `funct`.

// ===========================================================================
// Opcodes (bits 31:26)
// ===========================================================================

/// R-type family — operation selected by `funct` (bits 5:0).
pub const OP_RTYPE: u32 = 0x00;
/// REGIMM family — operation selected by `rt` (bits 20:16): BLTZ/BGEZ/BLTZAL/BGEZAL.
pub const OP_REGIMM: u32 = 0x01;
/// `J target` — unconditional jump.
pub const OP_J: u32 = 0x02;
/// `JAL target` — jump and link.
pub const OP_JAL: u32 = 0x03;
/// `BEQ rs, rt, offset`.
pub const OP_BEQ: u32 = 0x04;
/// `BNE rs, rt, offset`.
pub const OP_BNE: u32 = 0x05;
/// `BLEZ rs, offset`.
pub const OP_BLEZ: u32 = 0x06;
/// `BGTZ rs, offset`.
pub const OP_BGTZ: u32 = 0x07;
/// `ADDI rt, rs, imm` — signed add immediate; traps on 32-bit overflow.
pub const OP_ADDI: u32 = 0x08;
/// `ADDIU rt, rs, imm` — unsigned (wrapping) add immediate.
pub const OP_ADDIU: u32 = 0x09;
/// `SLTI rt, rs, imm` — set on signed less-than immediate.
pub const OP_SLTI: u32 = 0x0A;
/// `SLTIU rt, rs, imm` — set on unsigned less-than immediate.
pub const OP_SLTIU: u32 = 0x0B;
/// `ANDI rt, rs, imm` — zero-extended immediate AND.
pub const OP_ANDI: u32 = 0x0C;
/// `ORI rt, rs, imm` — zero-extended immediate OR.
pub const OP_ORI: u32 = 0x0D;
/// `XORI rt, rs, imm` — zero-extended immediate XOR.
pub const OP_XORI: u32 = 0x0E;
/// `LUI rt, imm` — load upper 16 bits, lower 16 bits zeroed.
pub const OP_LUI: u32 = 0x0F;
/// `LB rt, offset(rs)` — load byte, sign-extended.
pub const OP_LB: u32 = 0x20;
/// `LH rt, offset(rs)` — load halfword, sign-extended.
pub const OP_LH: u32 = 0x21;
/// `LW rt, offset(rs)` — load word.
pub const OP_LW: u32 = 0x23;
/// `LBU rt, offset(rs)` — load byte, zero-extended.
pub const OP_LBU: u32 = 0x24;
/// `LHU rt, offset(rs)` — load halfword, zero-extended.
pub const OP_LHU: u32 = 0x25;
/// `SB rt, offset(rs)` — store least-significant byte.
pub const OP_SB: u32 = 0x28;
/// `SH rt, offset(rs)` — store least-significant halfword.
pub const OP_SH: u32 = 0x29;
/// `SW rt, offset(rs)` — store word.
pub const OP_SW: u32 = 0x2B;

// ===========================================================================
// R-type funct codes (bits 5:0, only meaningful when op == OP_RTYPE)
// ===========================================================================

/// `SLL rd, rt, shamt` — logical left shift by immediate.  `SLL $zero, $zero, 0`
/// (encoded word `0x0000_0000`) is the canonical MIPS NOP.
pub const FUNCT_SLL: u32 = 0x00;
/// `SRL rd, rt, shamt` — logical right shift by immediate (zero-fill).
pub const FUNCT_SRL: u32 = 0x02;
/// `SRA rd, rt, shamt` — arithmetic right shift by immediate (sign-fill).
pub const FUNCT_SRA: u32 = 0x03;
/// `SLLV rd, rt, rs` — logical left shift by register (`rs & 31`).
pub const FUNCT_SLLV: u32 = 0x04;
/// `SRLV rd, rt, rs` — logical right shift by register.
pub const FUNCT_SRLV: u32 = 0x06;
/// `SRAV rd, rt, rs` — arithmetic right shift by register.
pub const FUNCT_SRAV: u32 = 0x07;
/// `JR rs` — jump to register.
pub const FUNCT_JR: u32 = 0x08;
/// `JALR rd, rs` — jump and link register.
pub const FUNCT_JALR: u32 = 0x09;
/// `SYSCALL` — our HALT sentinel (matches MIPS Linux convention where
/// `$v0` carries a syscall number; here any `SYSCALL` halts the sim).
pub const FUNCT_SYSCALL: u32 = 0x0C;
/// `BREAK` — software breakpoint.  Treated as a fault (halts the
/// simulator) rather than the "program done" SYSCALL sentinel.
pub const FUNCT_BREAK: u32 = 0x0D;
/// `MFHI rd` — move from HI.
pub const FUNCT_MFHI: u32 = 0x10;
/// `MTHI rs` — move to HI.
pub const FUNCT_MTHI: u32 = 0x11;
/// `MFLO rd` — move from LO.
pub const FUNCT_MFLO: u32 = 0x12;
/// `MTLO rs` — move to LO.
pub const FUNCT_MTLO: u32 = 0x13;
/// `MULT rs, rt` — signed 32x32->64 multiply; result in HI:LO.
pub const FUNCT_MULT: u32 = 0x18;
/// `MULTU rs, rt` — unsigned 32x32->64 multiply; result in HI:LO.
pub const FUNCT_MULTU: u32 = 0x19;
/// `DIV rs, rt` — signed divide; LO = quotient, HI = remainder.
pub const FUNCT_DIV: u32 = 0x1A;
/// `DIVU rs, rt` — unsigned divide.
pub const FUNCT_DIVU: u32 = 0x1B;
/// `ADD rd, rs, rt` — signed add; traps on 32-bit overflow.
pub const FUNCT_ADD: u32 = 0x20;
/// `ADDU rd, rs, rt` — unsigned (wrapping) add.
pub const FUNCT_ADDU: u32 = 0x21;
/// `SUB rd, rs, rt` — signed subtract; traps on 32-bit overflow.
pub const FUNCT_SUB: u32 = 0x22;
/// `SUBU rd, rs, rt` — unsigned (wrapping) subtract.
pub const FUNCT_SUBU: u32 = 0x23;
/// `AND rd, rs, rt` — bitwise AND.
pub const FUNCT_AND: u32 = 0x24;
/// `OR rd, rs, rt` — bitwise OR.
pub const FUNCT_OR: u32 = 0x25;
/// `XOR rd, rs, rt` — bitwise XOR.
pub const FUNCT_XOR: u32 = 0x26;
/// `NOR rd, rs, rt` — bitwise NOR (complement of OR).
pub const FUNCT_NOR: u32 = 0x27;
/// `SLT rd, rs, rt` — set if `signed(rs) < signed(rt)`.
pub const FUNCT_SLT: u32 = 0x2A;
/// `SLTU rd, rs, rt` — set if `unsigned(rs) < unsigned(rt)`.
pub const FUNCT_SLTU: u32 = 0x2B;

// ===========================================================================
// REGIMM rt-field codes (bits 20:16, only meaningful when op == OP_REGIMM)
// ===========================================================================

/// `BLTZ rs, offset` — branch if `signed(rs) < 0`.
pub const REGIMM_BLTZ: u32 = 0x00;
/// `BGEZ rs, offset` — branch if `signed(rs) >= 0`.
pub const REGIMM_BGEZ: u32 = 0x01;
/// `BLTZAL rs, offset` — `$ra = pc+4`; branch if `signed(rs) < 0`.
pub const REGIMM_BLTZAL: u32 = 0x10;
/// `BGEZAL rs, offset` — `$ra = pc+4`; branch if `signed(rs) >= 0`.
pub const REGIMM_BGEZAL: u32 = 0x11;

// ===========================================================================
// HALT sentinel
// ===========================================================================

/// `SYSCALL` instruction word (all-zero except `funct = 0x0C`):
/// `op=0, rs=0, rt=0, rd=0, shamt=0, funct=0x0C`.  Halts the simulator —
/// see the Python original (`mips_r2000_simulator/simulator.py`) for the
/// same HALT-via-SYSCALL convention.
pub const HALT_OPCODE_WORD: u32 = 0x0000_000C;
