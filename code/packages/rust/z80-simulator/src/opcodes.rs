//! Opcode / register / condition-code constant tables for the Zilog Z80 ISA.
//!
//! # The Z80 is an Intel 8080 superset
//!
//! Every valid 8080 opcode is a valid Z80 opcode with **identical**
//! semantics and **identical** byte encoding — the Z80 (1976) was designed
//! for source (and largely binary) compatibility with 8080 software.  This
//! module therefore reuses the exact same 3-bit register field, 2-bit
//! register-pair field, 3-bit ALU-operation field, and 3-bit condition-code
//! field the 8080 uses (compare `intel8080_simulator::opcodes`) — only the
//! *names* differ (Zilog's assembler mnemonics vs Intel's), and a handful
//! of fixed-byte opcodes that are UNDEFINED on a stock 8080 gain new
//! meanings on the Z80 (`EX AF,AF'`, `EXX`, `DJNZ`, `JR` + its four
//! conditional forms) or become the entry point of a brand-new prefixed
//! opcode space (`CB`, `ED`, `DD`, `FD`).
//!
//! ```text
//! bits 7-6: group   (00 = data/ALU setup, 01 = LD r,r'/HALT, 10 = ALU-with-register, 11 = control/stack/branch)
//! bits 5-3: dst / sub-operation
//! bits 2-0: src / sub-operation
//! ```
//!
//! # What's new on the Z80 (relative to `intel8080_simulator::opcodes`)
//!
//! - **Alternate register bank** — `EX AF,AF'` (`0x08`) and `EXX`
//!   (`0xD9`) swap the live register bank; both bytes are UNDEFINED on a
//!   stock 8080.
//! - **Relative jumps** — `DJNZ e` (`0x10`) and `JR [cc,] e` (`0x18`,
//!   `0x20`, `0x28`, `0x30`, `0x38`); all five bytes are UNDEFINED on a
//!   stock 8080.
//! - **Four prefix bytes** — `CB` (bit manipulation + extended
//!   rotate/shift), `ED` (extended: 16-bit ADC/SBC, block ops, I/R
//!   register moves — **not ported**, see `decode::decode_ed`), `DD`/`FD`
//!   (index registers IX/IY, replacing HL in most HL-using
//!   instructions).  All four prefix bytes are themselves UNDEFINED
//!   opcodes on a stock 8080.

// ===========================================================================
// Register codes (3-bit field: bits 5-3 as "dst", bits 2-0 as "src")
// ===========================================================================
//
// Identical encoding to `intel8080_simulator::opcodes` — B=0, C=1, D=2,
// E=3, H=4, L=5, (HL)=6, A=7.

/// Register B.
pub const REG_B: u8 = 0;
/// Register C.
pub const REG_C: u8 = 1;
/// Register D.
pub const REG_D: u8 = 2;
/// Register E.
pub const REG_E: u8 = 3;
/// Register H (high byte of the HL pair).
pub const REG_H: u8 = 4;
/// Register L (low byte of the HL pair).
pub const REG_L: u8 = 5;
/// `(HL)` — pseudo-register; aliases memory at address `(H << 8) | L`.
pub const REG_M: u8 = 6;
/// Accumulator.
pub const REG_A: u8 = 7;

// ===========================================================================
// Register pair codes (2-bit field, used by LD rp,nn / INC rp / DEC rp /
// ADD HL,rp / PUSH rp / POP rp)
// ===========================================================================

/// BC pair (B = high, C = low).
pub const PAIR_BC: u8 = 0;
/// DE pair (D = high, E = low).
pub const PAIR_DE: u8 = 1;
/// HL pair (H = high, L = low).
pub const PAIR_HL: u8 = 2;
/// SP (or AF for `PUSH`/`POP`).
pub const PAIR_SP: u8 = 3;
/// AF pair — only meaningful for `PUSH`/`POP` (shares the bit pattern of
/// `PAIR_SP`; which one applies is determined by the instruction, exactly
/// as on the 8080).
pub const PAIR_AF: u8 = 3;

// ===========================================================================
// ALU operation codes (bits 5-3 of a group-10/group-11 ALU instruction)
// ===========================================================================

/// `ADD A,` — A ← A + operand.
pub const ALU_ADD: u8 = 0;
/// `ADC A,` — A ← A + operand + CY.
pub const ALU_ADC: u8 = 1;
/// `SUB` — A ← A - operand.
pub const ALU_SUB: u8 = 2;
/// `SBC A,` — A ← A - operand - CY.
pub const ALU_SBC: u8 = 3;
/// `AND` — A ← A AND operand.
pub const ALU_AND: u8 = 4;
/// `XOR` — A ← A XOR operand.
pub const ALU_XOR: u8 = 5;
/// `OR` — A ← A OR operand.
pub const ALU_OR: u8 = 6;
/// `CP` — set flags as if `A - operand`; A unchanged.
pub const ALU_CP: u8 = 7;

// ===========================================================================
// Condition codes (bits 5-3 of a conditional jump/call/return)
// ===========================================================================

/// Not Zero (`Z == 0`).
pub const COND_NZ: u8 = 0;
/// Zero (`Z == 1`).
pub const COND_Z: u8 = 1;
/// No Carry (`CY == 0`).
pub const COND_NC: u8 = 2;
/// Carry (`CY == 1`).
pub const COND_C: u8 = 3;
/// Parity Odd (`P/V == 0`).
pub const COND_PO: u8 = 4;
/// Parity Even (`P/V == 1`).
pub const COND_PE: u8 = 5;
/// Plus / positive (`S == 0`).
pub const COND_P: u8 = 6;
/// Minus / negative (`S == 1`).
pub const COND_M: u8 = 7;

// ===========================================================================
// Individually addressed (fixed-byte) opcodes — byte-identical to their
// 8080-legacy counterparts (see `code/specs/z80-encoder.md` for the full
// byte-identity table)
// ===========================================================================

/// `NOP` — no operation.
pub const NOP: u8 = 0x00;
/// `HALT` — halt.  Shares its bit pattern with `LD (HL),(HL)` (`01_110_110`);
/// the Z80 special-cases it as a halt rather than a self-move — **byte-
/// identical to 8080 `HLT`** (`0x76`).
pub const HALT: u8 = 0x76;
/// `RET` — unconditional return.  Byte-identical to 8080.
pub const RET: u8 = 0xC9;
/// `JP nn` — unconditional absolute jump (Zilog's name for 8080's `JMP`).
/// Byte-identical to 8080.
pub const JP: u8 = 0xC3;
/// `CALL nn` — unconditional call.  Byte-identical to 8080.
pub const CALL: u8 = 0xCD;
/// `RLCA` — rotate A left circular (Zilog's name for 8080's `RLC`).
/// Byte-identical to 8080.
pub const RLCA: u8 = 0x07;
/// `RRCA` — rotate A right circular (Zilog's `RRC`).  Byte-identical.
pub const RRCA: u8 = 0x0F;
/// `RLA` — rotate A left through carry (Zilog's `RAL`).  Byte-identical.
pub const RLA: u8 = 0x17;
/// `RRA` — rotate A right through carry (Zilog's `RAR`).  Byte-identical.
pub const RRA: u8 = 0x1F;
/// `DAA` — decimal-adjust accumulator.  Byte-identical to 8080.
pub const DAA: u8 = 0x27;
/// `CPL` — complement accumulator (Zilog's `CMA`).  Byte-identical.
pub const CPL: u8 = 0x2F;
/// `SCF` — set carry flag (Zilog's `STC`).  Byte-identical.
pub const SCF: u8 = 0x37;
/// `CCF` — complement carry flag (Zilog's `CMC`).  Byte-identical.
pub const CCF: u8 = 0x3F;
/// `LD (BC),A` — memory[BC] ← A (Zilog's `STAX B`).  Byte-identical.
pub const LD_BC_A: u8 = 0x02;
/// `LD (DE),A` — memory[DE] ← A (Zilog's `STAX D`).  Byte-identical.
pub const LD_DE_A: u8 = 0x12;
/// `LD A,(BC)` — A ← memory[BC] (Zilog's `LDAX B`).  Byte-identical.
pub const LD_A_BC: u8 = 0x0A;
/// `LD A,(DE)` — A ← memory[DE] (Zilog's `LDAX D`).  Byte-identical.
pub const LD_A_DE: u8 = 0x1A;
/// `LD (nn),HL` — memory[nn] ← L; memory[nn+1] ← H (Zilog's `SHLD`).
/// Byte-identical.
pub const LD_NN_HL: u8 = 0x22;
/// `LD HL,(nn)` — L ← memory[nn]; H ← memory[nn+1] (Zilog's `LHLD`).
/// Byte-identical.
pub const LD_HL_NN: u8 = 0x2A;
/// `LD (nn),A` — memory[nn] ← A (Zilog's `STA`).  Byte-identical.
pub const LD_NN_A: u8 = 0x32;
/// `LD A,(nn)` — A ← memory[nn] (Zilog's `LDA`).  Byte-identical.
pub const LD_A_NN: u8 = 0x3A;
/// `EX (SP),HL` — swap top-of-stack with HL (Zilog's name for 8080's
/// `XTHL`; identical semantics).  Byte-identical.
pub const EX_SP_HL: u8 = 0xE3;
/// `LD SP,HL` — SP ← HL (Zilog's `SPHL`).  Byte-identical.
pub const LD_SP_HL: u8 = 0xF9;
/// `EX DE,HL` — swap DE and HL (Zilog's `XCHG`).  Byte-identical.
pub const EX_DE_HL: u8 = 0xEB;
/// `JP (HL)` — PC ← HL (Zilog's `PCHL`; note this jumps to the address
/// *in* HL, not to `(HL)`'s memory contents — the mnemonic is
/// historically confusing on both chips).  Byte-identical.
pub const JP_HL: u8 = 0xE9;
/// `IN A,(n)` — A ← input_port[n].  Byte-identical to 8080.
pub const IN: u8 = 0xDB;
/// `OUT (n),A` — output_port[n] ← A.  Byte-identical to 8080.
pub const OUT: u8 = 0xD3;
/// `EI` — enable interrupts.  Byte-identical to 8080.
pub const EI: u8 = 0xFB;
/// `DI` — disable interrupts.  Byte-identical to 8080.
pub const DI: u8 = 0xF3;

// ===========================================================================
// Z80-only fixed-byte opcodes — UNDEFINED on a stock 8080
// ===========================================================================

/// `EX AF,AF'` — swap the main and alternate AF register pairs.  `0x08` is
/// an undefined/reserved byte on a stock 8080.
pub const EX_AF_AF: u8 = 0x08;
/// `EXX` — swap BC/DE/HL with BC'/DE'/HL' (the alternate bank).  `0xD9` is
/// undefined on a stock 8080.
pub const EXX: u8 = 0xD9;
/// `DJNZ e` — decrement B; if nonzero, jump PC-relative by signed `e`.
/// `0x10` is undefined on a stock 8080.
pub const DJNZ: u8 = 0x10;
/// `JR e` — unconditional PC-relative jump.  `0x18` is undefined on a
/// stock 8080.
pub const JR: u8 = 0x18;
/// `JR NZ,e`.  `0x20` is undefined on a stock 8080.
pub const JR_NZ: u8 = 0x20;
/// `JR Z,e`.  `0x28` is undefined on a stock 8080.
pub const JR_Z: u8 = 0x28;
/// `JR NC,e`.  `0x30` is undefined on a stock 8080.
pub const JR_NC: u8 = 0x30;
/// `JR C,e`.  `0x38` is undefined on a stock 8080.
pub const JR_C: u8 = 0x38;

// ===========================================================================
// Prefix bytes — each opens a distinct secondary opcode space, all
// UNDEFINED as standalone bytes on a stock 8080
// ===========================================================================

/// `CB` prefix — bit manipulation (`BIT`/`SET`/`RES`) and extended
/// rotate/shift (`RLC`/`RRC`/`RL`/`RR`/`SLA`/`SRA`/`SLL`/`SRL`) on any of
/// the 8 `r`-coded operands.  See `decode::decode_cb`.
pub const CB_PREFIX: u8 = 0xCB;
/// `ED` prefix — extended instructions (16-bit `ADC`/`SBC HL,rp`, block
/// transfer/compare/I-O `LDIR`/`CPIR`/`INIR`/…, `LD A,I`/`LD A,R`, `NEG`,
/// interrupt-mode selection, …).  **Not ported** in this v0.1.0 — see the
/// module docs on `decode::decode_ed` for the deliberate scope cut.
pub const ED_PREFIX: u8 = 0xED;
/// `DD` prefix — replaces `HL` with `IX` throughout most instructions
/// that follow.  Only `LD IX,nn` and `INC IX` are ported in this
/// v0.1.0 — see `decode::decode_ddfd`.
pub const DD_PREFIX: u8 = 0xDD;
/// `FD` prefix — replaces `HL` with `IY` throughout most instructions
/// that follow.  Only `LD IY,nn` and `INC IY` are ported in this
/// v0.1.0 — see `decode::decode_ddfd`.
pub const FD_PREFIX: u8 = 0xFD;
