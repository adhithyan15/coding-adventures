//! Opcode / op2 / op3 field constant tables for the SPARC V8 ISA.
//!
//! Every SPARC V8 instruction is 32 bits wide.  The top two bits
//! (`op`, bits 31:30) select one of three instruction formats:
//!
//! ```text
//! Format 1 (op=01):  [op:2][disp30:30]                          -- CALL
//! Format 2 (op=00):  [op:2][rd:5][op2:3][imm22:22]               -- SETHI, Bicc, NOP
//! Format 3 (op=10/11): [op:2][rd:5][op3:6][rs1:5][i:1][...]      -- ALU (op=10) / memory (op=11)
//! ```
//!
//! Format 3 has two sub-shapes selected by the `i` bit (bit 13):
//!
//! ```text
//! Format 3r (i=0): [op][rd][op3][rs1][0][asi:8][rs2:5]  -- register operand
//! Format 3i (i=1): [op][rd][op3][rs1][1][simm13:13]     -- sign-extended 13-bit immediate
//! ```
//!
//! This module is a straight transcription of the field tables in
//! `code/packages/python/sparc-v8-simulator/src/sparc_v8_simulator/{state,simulator}.py`
//! — see that module's docstring for the full instruction-format
//! diagram and design rationale.

// ===========================================================================
// Top-level `op` field (bits 31:30)
// ===========================================================================

/// Format 2 — SETHI / Bicc / NOP.
pub const OP_FMT2: u32 = 0b00;
/// Format 1 — CALL.
pub const OP_CALL: u32 = 0b01;
/// Format 3, ALU-class op3 codes (arithmetic/logic/shift/SAVE/RESTORE/Ticc/…).
pub const OP_ALU: u32 = 0b10;
/// Format 3, memory-class op3 codes (LD/ST family).
pub const OP_MEM: u32 = 0b11;

// ===========================================================================
// Format 2 `op2` field (bits 24:22)
// ===========================================================================

/// `Bicc` — branch on integer condition codes.
pub const OP2_BICC: u32 = 0x2;
/// `SETHI rd, imm22` — also the NOP encoding when `rd=0, imm22=0`.
pub const OP2_SETHI: u32 = 0x4;

/// `SETHI %g0, 0` — the canonical SPARC V8 NOP encoding (`0x0100_0000`).
pub const NOP_WORD: u32 = 0x0100_0000;

// ===========================================================================
// Format 3 `op3` field (bits 24:19) -- ALU family (`op == OP_ALU`)
// ===========================================================================

/// `ADD rd, rs1, reg_or_imm`.
pub const OP3_ADD: u32 = 0x00;
/// `AND rd, rs1, reg_or_imm`.
pub const OP3_AND: u32 = 0x01;
/// `OR rd, rs1, reg_or_imm`.
pub const OP3_OR: u32 = 0x02;
/// `XOR rd, rs1, reg_or_imm`.
pub const OP3_XOR: u32 = 0x03;
/// `SUB rd, rs1, reg_or_imm`.
pub const OP3_SUB: u32 = 0x04;
/// `ANDN rd, rs1, reg_or_imm` (AND NOT).
pub const OP3_ANDN: u32 = 0x05;
/// `ORN rd, rs1, reg_or_imm` (OR NOT).
pub const OP3_ORN: u32 = 0x06;
/// `XNOR rd, rs1, reg_or_imm`.
pub const OP3_XNOR: u32 = 0x07;
/// `ADDX` — add with carry-in.
pub const OP3_ADDX: u32 = 0x08;
/// `UMUL` — unsigned 32x32->64 multiply; high word to `Y`.
pub const OP3_UMUL: u32 = 0x0A;
/// `SMUL` — signed 32x32->64 multiply; high word to `Y`.
pub const OP3_SMUL: u32 = 0x0B;
/// `SUBX` — subtract with borrow-in.
pub const OP3_SUBX: u32 = 0x0C;
/// `UDIV` — unsigned 64÷32->32 divide; dividend is `Y:rs1`.
pub const OP3_UDIV: u32 = 0x0E;
/// `SDIV` — signed 64÷32->32 divide; dividend is `Y:rs1`.
pub const OP3_SDIV: u32 = 0x0F;
/// `ADDcc` — `ADD` that also updates PSR N/Z/V/C.
pub const OP3_ADDCC: u32 = 0x10;
/// `ANDcc` — `AND` that also updates PSR N/Z (V=C=0).
pub const OP3_ANDCC: u32 = 0x11;
/// `ORcc` — `OR` that also updates PSR N/Z (V=C=0).
pub const OP3_ORCC: u32 = 0x12;
/// `XORcc` — `XOR` that also updates PSR N/Z (V=C=0).
pub const OP3_XORCC: u32 = 0x13;
/// `SUBcc` — `SUB` that also updates PSR N/Z/V/C.
pub const OP3_SUBCC: u32 = 0x14;
/// `ANDNcc`.
pub const OP3_ANDNCC: u32 = 0x15;
/// `ORNcc`.
pub const OP3_ORNCC: u32 = 0x16;
/// `XNORcc`.
pub const OP3_XNORCC: u32 = 0x17;
/// `ADDXcc`.
pub const OP3_ADDXCC: u32 = 0x18;
/// `SUBXcc`.
pub const OP3_SUBXCC: u32 = 0x1C;
/// `MULScc` — one step of the restoring-multiply algorithm.
pub const OP3_MULSCC: u32 = 0x24;
/// `SLL rd, rs1, reg_or_imm5` — logical shift left.
pub const OP3_SLL: u32 = 0x25;
/// `SRL rd, rs1, reg_or_imm5` — logical shift right.
pub const OP3_SRL: u32 = 0x26;
/// `SRA rd, rs1, reg_or_imm5` — arithmetic shift right.
pub const OP3_SRA: u32 = 0x27;
/// `RDY rd` — read the `Y` register.
pub const OP3_RDY: u32 = 0x28;
/// `WRY rs1, reg_or_imm` — write the `Y` register (`Y = rs1 XOR src2`).
pub const OP3_WRY: u32 = 0x30;
/// `JMPL rd, rs1, reg_or_imm` — jump and link.
pub const OP3_JMPL: u32 = 0x38;
/// `Ticc` — trap on integer condition.  `ta 0` (cond=`COND_BA`) is this
/// simulator's HALT sentinel — see [`HALT_WORD`].
pub const OP3_TICC: u32 = 0x3A;
/// `SAVE rd, rs1, reg_or_imm` — procedure entry; rotates CWP backward.
pub const OP3_SAVE: u32 = 0x3C;
/// `RESTORE rd, rs1, reg_or_imm` — procedure exit; rotates CWP forward.
pub const OP3_RESTORE: u32 = 0x3D;
/// `UMULcc`.
pub const OP3_UMULCC: u32 = 0x5A;
/// `SMULcc`.
pub const OP3_SMULCC: u32 = 0x5B;
/// `UDIVcc`.
pub const OP3_UDIVCC: u32 = 0x5E;
/// `SDIVcc`.
pub const OP3_SDIVCC: u32 = 0x5F;

// ===========================================================================
// Format 3 `op3` field (bits 24:19) -- memory family (`op == OP_MEM`)
// ===========================================================================

/// `LD rd, [ea]` — load word.
pub const OP3_LD: u32 = 0x00;
/// `LDUB rd, [ea]` — load unsigned byte.
pub const OP3_LDUB: u32 = 0x01;
/// `LDUH rd, [ea]` — load unsigned halfword.
pub const OP3_LDUH: u32 = 0x02;
/// `ST rd, [ea]` — store word.
pub const OP3_ST: u32 = 0x04;
/// `STB rd, [ea]` — store byte.
pub const OP3_STB: u32 = 0x05;
/// `STH rd, [ea]` — store halfword.
pub const OP3_STH: u32 = 0x06;
/// `LDSB rd, [ea]` — load signed byte.
pub const OP3_LDSB: u32 = 0x09;
/// `LDSH rd, [ea]` — load signed halfword.
pub const OP3_LDSH: u32 = 0x0A;

// ===========================================================================
// Bicc / Ticc condition-code field (4 bits, SPARC V8 manual §A.7 / §A.4)
// ===========================================================================

/// `BN`/never — branch never taken.
pub const COND_BN: u32 = 0x0;
/// `BE`/equal — `Z=1`.
pub const COND_BE: u32 = 0x1;
/// `BLE`/less-or-equal — `Z=1 or N!=V`.
pub const COND_BLE: u32 = 0x2;
/// `BL`/less — `N!=V`.
pub const COND_BL: u32 = 0x3;
/// `BLEU`/less-or-equal-unsigned — `C=1 or Z=1`.
pub const COND_BLEU: u32 = 0x4;
/// `BCS`/carry-set (unsigned less) — `C=1`.
pub const COND_BCS: u32 = 0x5;
/// `BNEG`/negative — `N=1`.
pub const COND_BNEG: u32 = 0x6;
/// `BVS`/overflow-set — `V=1`.
pub const COND_BVS: u32 = 0x7;
/// `BA`/always — branch always taken.  Also the Ticc "trap always"
/// condition used by [`HALT_WORD`].
pub const COND_BA: u32 = 0x8;
/// `BNE`/not-equal — `Z=0`.
pub const COND_BNE: u32 = 0x9;
/// `BG`/greater — `Z=0 and N=V`.
pub const COND_BG: u32 = 0xA;
/// `BGE`/greater-or-equal — `N=V`.
pub const COND_BGE: u32 = 0xB;
/// `BGU`/greater-unsigned — `C=0 and Z=0`.
pub const COND_BGU: u32 = 0xC;
/// `BCC`/carry-clear (unsigned greater-or-equal) — `C=0`.
pub const COND_BCC: u32 = 0xD;
/// `BPOS`/positive — `N=0`.
pub const COND_BPOS: u32 = 0xE;
/// `BVC`/overflow-clear — `V=0`.
pub const COND_BVC: u32 = 0xF;

// ===========================================================================
// HALT sentinel
// ===========================================================================

/// `ta 0` — "trap always", software trap #0.  This simulator's HALT
/// convention (matches the Python original and the SPARC/Solaris
/// `ta 0`-as-debugger-breakpoint idiom, distinct from `ta 1` = `sys_exit`
/// on real SPARC Linux/SunOS ABIs).
///
/// Encoding (Format 3i, `op=OP_ALU`, `op3=OP3_TICC`): `rd` field carries
/// the trap condition (`COND_BA` = 8 = "always"), `rs1=0`, `i=1`,
/// `simm13=0`:
///
/// ```text
/// (OP_ALU << 30) | (COND_BA << 25) | (OP3_TICC << 19) | (1 << 13)
///   = (0b10 << 30) | (0x8 << 25) | (0x3A << 19) | (1 << 13)
///   = 0x91D0_2000
/// ```
///
/// See `code/packages/python/sparc-v8-simulator/src/sparc_v8_simulator/state.py`
/// (`HALT_WORD`) for the matching Python derivation this Rust port mirrors
/// byte-for-byte.
pub const HALT_WORD: u32 = 0x91D0_2000;
