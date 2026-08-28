//! Opcode / register / condition-code constant tables for the Intel 8080 ISA.
//!
//! The 8080 uses a regular 8-bit opcode space split into four groups by the
//! top two bits:
//!
//! ```text
//! bits 7-6: group   (00 = data/ALU setup, 01 = MOV, 10 = ALU-with-register, 11 = control/stack/branch)
//! bits 5-3: dst / sub-operation
//! bits 2-0: src / sub-operation
//! ```
//!
//! Register-Direct-Addressing instructions carry a 3-bit register code in
//! either (or both) of the low six bits; register-pair instructions carry a
//! 2-bit pair code.  This module declares the constants `decode.rs` and
//! `encoding.rs` both need so the bit layout has exactly one source of
//! truth, mirroring `mips_r2000_simulator::opcodes`.

// ===========================================================================
// Register codes (3-bit field: bits 5-3 as "dst", bits 2-0 as "src")
// ===========================================================================

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
/// M — pseudo-register; aliases memory at address `(H << 8) | L`.
pub const REG_M: u8 = 6;
/// Accumulator.
pub const REG_A: u8 = 7;

// ===========================================================================
// Register pair codes (2-bit field, used by LXI/INX/DCX/DAD/PUSH/POP)
// ===========================================================================

/// BC pair (B = high, C = low).
pub const PAIR_B: u8 = 0;
/// DE pair (D = high, E = low).
pub const PAIR_D: u8 = 1;
/// HL pair (H = high, L = low).
pub const PAIR_H: u8 = 2;
/// SP (or PSW for `PUSH`/`POP`).
pub const PAIR_SP: u8 = 3;

// ===========================================================================
// ALU operation codes (bits 5-3 of a group-10/group-11 ALU instruction)
// ===========================================================================

/// `ADD` — A ← A + operand.
pub const ALU_ADD: u8 = 0;
/// `ADC` — A ← A + operand + CY.
pub const ALU_ADC: u8 = 1;
/// `SUB` — A ← A - operand.
pub const ALU_SUB: u8 = 2;
/// `SBB` — A ← A - operand - CY.
pub const ALU_SBB: u8 = 3;
/// `ANA` — A ← A AND operand.
pub const ALU_ANA: u8 = 4;
/// `XRA` — A ← A XOR operand.
pub const ALU_XRA: u8 = 5;
/// `ORA` — A ← A OR operand.
pub const ALU_ORA: u8 = 6;
/// `CMP` — set flags as if `A - operand`; A unchanged.
pub const ALU_CMP: u8 = 7;

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
/// Parity Odd (`P == 0`).
pub const COND_PO: u8 = 4;
/// Parity Even (`P == 1`).
pub const COND_PE: u8 = 5;
/// Plus / positive (`S == 0`).
pub const COND_P: u8 = 6;
/// Minus / negative (`S == 1`).
pub const COND_M: u8 = 7;

// ===========================================================================
// Individually addressed (fixed-byte) opcodes
// ===========================================================================

/// `NOP` — no operation.
pub const NOP: u8 = 0x00;
/// `HLT` — halt.  Shares its bit pattern with `MOV M,M` (`01_110_110`);
/// the 8080 special-cases it as a halt rather than a self-move.
pub const HLT: u8 = 0x76;
/// `RET` — unconditional return.
pub const RET: u8 = 0xC9;
/// `JMP addr` — unconditional jump.
pub const JMP: u8 = 0xC3;
/// `CALL addr` — unconditional call.
pub const CALL: u8 = 0xCD;
/// `RLC` — rotate A left circular.
pub const RLC: u8 = 0x07;
/// `RRC` — rotate A right circular.
pub const RRC: u8 = 0x0F;
/// `RAL` — rotate A left through carry.
pub const RAL: u8 = 0x17;
/// `RAR` — rotate A right through carry.
pub const RAR: u8 = 0x1F;
/// `DAA` — decimal-adjust accumulator.
pub const DAA: u8 = 0x27;
/// `CMA` — complement accumulator.
pub const CMA: u8 = 0x2F;
/// `STC` — set carry.
pub const STC: u8 = 0x37;
/// `CMC` — complement carry.
pub const CMC: u8 = 0x3F;
/// `STAX B` — `memory[BC]` ← A.
pub const STAX_B: u8 = 0x02;
/// `STAX D` — `memory[DE]` ← A.
pub const STAX_D: u8 = 0x12;
/// `LDAX B` — A ← `memory[BC]`.
pub const LDAX_B: u8 = 0x0A;
/// `LDAX D` — A ← `memory[DE]`.
pub const LDAX_D: u8 = 0x1A;
/// `SHLD addr` — `memory[addr]` ← L; `memory[addr+1]` ← H.
pub const SHLD: u8 = 0x22;
/// `LHLD addr` — L ← `memory[addr]`; H ← `memory[addr+1]`.
pub const LHLD: u8 = 0x2A;
/// `STA addr` — `memory[addr]` ← A.
pub const STA: u8 = 0x32;
/// `LDA addr` — A ← `memory[addr]`.
pub const LDA: u8 = 0x3A;
/// `XTHL` — L ↔ `memory[SP]`; H ↔ `memory[SP+1]`.
pub const XTHL: u8 = 0xE3;
/// `SPHL` — SP ← HL.
pub const SPHL: u8 = 0xF9;
/// `XCHG` — HL ↔ DE.
pub const XCHG: u8 = 0xEB;
/// `PCHL` — PC ← HL.
pub const PCHL: u8 = 0xE9;
/// `IN port` — A ← input_port\[port\].
pub const IN: u8 = 0xDB;
/// `OUT port` — output_port\[port\] ← A.
pub const OUT: u8 = 0xD3;
/// `EI` — enable interrupts.
pub const EI: u8 = 0xFB;
/// `DI` — disable interrupts.
pub const DI: u8 = 0xF3;
