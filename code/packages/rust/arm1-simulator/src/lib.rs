//! # ARM1 Behavioral Simulator
//!
//! A complete behavioral simulator for the ARM1 processor — the first ARM chip,
//! designed by Sophie Wilson and Steve Furber at Acorn Computers in 1984-1985.
//!
//! The ARM1 was a 32-bit RISC processor with just 25,000 transistors. It famously
//! worked correctly on its very first power-on (April 26, 1985). Its accidental
//! low power consumption (~0.1W) later made the ARM architecture dominant in
//! mobile computing, with over 250 billion chips shipped.
//!
//! This crate implements the complete ARMv1 instruction set:
//! - 16 data processing operations (AND, EOR, SUB, RSB, ADD, ADC, SBC, RSC,
//!   TST, TEQ, CMP, CMN, ORR, MOV, BIC, MVN)
//! - Load/store (LDR, STR, LDRB, STRB with pre/post-indexed addressing)
//! - Block transfer (LDM, STM with all four stacking modes)
//! - Branch (B, BL)
//! - Software interrupt (SWI)
//! - Conditional execution on every instruction (16 condition codes)
//! - Inline barrel shifter (LSL, LSR, ASR, ROR, RRX)
//! - 4 processor modes with banked registers (USR, FIQ, IRQ, SVC)
//!
//! # Usage
//!
//! ```rust
//! use arm1_simulator::{ARM1, encode_mov_imm, encode_halt, COND_AL};
//!
//! let mut cpu = ARM1::new(4096);
//! let code = vec![
//!     encode_mov_imm(COND_AL, 0, 42),
//!     encode_halt(),
//! ];
//! cpu.load_program_words(&code, 0);
//! let traces = cpu.run(100);
//! assert_eq!(cpu.read_register(0), 42);
//! ```

// =========================================================================
// Processor Modes
// =========================================================================
//
// The ARM1 supports 4 processor modes. Each mode has its own banked copies
// of certain registers, allowing fast context switching (especially for FIQ,
// which banks 7 registers to avoid saving/restoring them in the handler).
//
//   Mode  M1:M0  Banked Registers
//   ----  -----  ----------------
//   USR   0b00   (none -- base set)
//   FIQ   0b01   R8_fiq..R12_fiq, R13_fiq, R14_fiq
//   IRQ   0b10   R13_irq, R14_irq
//   SVC   0b11   R13_svc, R14_svc

pub const MODE_USR: u32 = 0;
pub const MODE_FIQ: u32 = 1;
pub const MODE_IRQ: u32 = 2;
pub const MODE_SVC: u32 = 3;

/// Returns a human-readable name for a processor mode.
pub fn mode_string(mode: u32) -> &'static str {
    match mode {
        MODE_USR => "USR",
        MODE_FIQ => "FIQ",
        MODE_IRQ => "IRQ",
        MODE_SVC => "SVC",
        _ => "???",
    }
}

// =========================================================================
// Condition Codes
// =========================================================================
//
// Every ARM instruction has a 4-bit condition code in bits 31:28.
// The instruction only executes if the condition is met. This is ARM's
// signature feature -- even data processing and load/store instructions
// can be conditional, eliminating many branches.
//
//   Code  Suffix  Meaning                  Test
//   ----  ------  -------                  ----
//   0000  EQ      Equal                    Z == 1
//   0001  NE      Not Equal                Z == 0
//   0010  CS/HS   Carry Set / Unsigned >=  C == 1
//   0011  CC/LO   Carry Clear / Unsigned < C == 0
//   0100  MI      Minus (Negative)         N == 1
//   0101  PL      Plus (Non-negative)      N == 0
//   0110  VS      Overflow Set             V == 1
//   0111  VC      Overflow Clear           V == 0
//   1000  HI      Unsigned Higher          C == 1 AND Z == 0
//   1001  LS      Unsigned Lower or Same   C == 0 OR  Z == 1
//   1010  GE      Signed >=                N == V
//   1011  LT      Signed <                 N != V
//   1100  GT      Signed >                 Z == 0 AND N == V
//   1101  LE      Signed <=                Z == 1 OR  N != V
//   1110  AL      Always                   true
//   1111  NV      Never (reserved)         false

pub const COND_EQ: u32 = 0x0;
pub const COND_NE: u32 = 0x1;
pub const COND_CS: u32 = 0x2;
pub const COND_CC: u32 = 0x3;
pub const COND_MI: u32 = 0x4;
pub const COND_PL: u32 = 0x5;
pub const COND_VS: u32 = 0x6;
pub const COND_VC: u32 = 0x7;
pub const COND_HI: u32 = 0x8;
pub const COND_LS: u32 = 0x9;
pub const COND_GE: u32 = 0xA;
pub const COND_LT: u32 = 0xB;
pub const COND_GT: u32 = 0xC;
pub const COND_LE: u32 = 0xD;
pub const COND_AL: u32 = 0xE;
pub const COND_NV: u32 = 0xF;

/// Returns the assembly-language suffix for a condition code.
pub fn cond_string(cond: u32) -> &'static str {
    match cond {
        COND_EQ => "EQ",
        COND_NE => "NE",
        COND_CS => "CS",
        COND_CC => "CC",
        COND_MI => "MI",
        COND_PL => "PL",
        COND_VS => "VS",
        COND_VC => "VC",
        COND_HI => "HI",
        COND_LS => "LS",
        COND_GE => "GE",
        COND_LT => "LT",
        COND_GT => "GT",
        COND_LE => "LE",
        COND_AL => "",
        COND_NV => "NV",
        _ => "??",
    }
}

// =========================================================================
// ALU Opcodes
// =========================================================================
//
// The ARM1's ALU supports 16 operations, selected by bits 24:21 of a data
// processing instruction. Four of these (TST, TEQ, CMP, CMN) only set flags
// and do not write a result to the destination register.

pub const OP_AND: u32 = 0x0;
pub const OP_EOR: u32 = 0x1;
pub const OP_SUB: u32 = 0x2;
pub const OP_RSB: u32 = 0x3;
pub const OP_ADD: u32 = 0x4;
pub const OP_ADC: u32 = 0x5;
pub const OP_SBC: u32 = 0x6;
pub const OP_RSC: u32 = 0x7;
pub const OP_TST: u32 = 0x8;
pub const OP_TEQ: u32 = 0x9;
pub const OP_CMP: u32 = 0xA;
pub const OP_CMN: u32 = 0xB;
pub const OP_ORR: u32 = 0xC;
pub const OP_MOV: u32 = 0xD;
pub const OP_BIC: u32 = 0xE;
pub const OP_MVN: u32 = 0xF;

/// Returns the mnemonic for an ALU opcode.
pub fn op_string(opcode: u32) -> &'static str {
    const NAMES: [&str; 16] = [
        "AND", "EOR", "SUB", "RSB", "ADD", "ADC", "SBC", "RSC",
        "TST", "TEQ", "CMP", "CMN", "ORR", "MOV", "BIC", "MVN",
    ];
    if (opcode as usize) < 16 {
        NAMES[opcode as usize]
    } else {
        "???"
    }
}

/// Returns true if the ALU opcode is a test-only operation (TST, TEQ, CMP, CMN)
/// that does not write to the destination register.
pub fn is_test_op(opcode: u32) -> bool {
    opcode >= OP_TST && opcode <= OP_CMN
}

/// Returns true if the ALU opcode is a logical operation.
/// For logical ops, the C flag comes from the barrel shifter carry-out
/// rather than the ALU's adder carry.
pub fn is_logical_op(opcode: u32) -> bool {
    matches!(opcode, OP_AND | OP_EOR | OP_TST | OP_TEQ | OP_ORR | OP_MOV | OP_BIC | OP_MVN)
}

// =========================================================================
// Shift Types
// =========================================================================
//
// The barrel shifter supports 4 shift types, encoded in bits 6:5 of the
// operand2 field. The barrel shifter allows one operand to be shifted or
// rotated FOR FREE as part of any data processing instruction.
//
// Example: ADD R0, R1, R2, LSL #3  means  R0 = R1 + (R2 << 3)

pub const SHIFT_LSL: u32 = 0;
pub const SHIFT_LSR: u32 = 1;
pub const SHIFT_ASR: u32 = 2;
pub const SHIFT_ROR: u32 = 3;

/// Returns the mnemonic for a shift type.
pub fn shift_string(shift_type: u32) -> &'static str {
    match shift_type {
        SHIFT_LSL => "LSL",
        SHIFT_LSR => "LSR",
        SHIFT_ASR => "ASR",
        SHIFT_ROR => "ROR",
        _ => "???",
    }
}

// =========================================================================
// R15 bit positions
// =========================================================================
//
// ARMv1's most distinctive architectural feature is that the program counter
// and processor status flags share a single 32-bit register (R15):
//
//   Bit 31: N (Negative)     Bit 27: I (IRQ disable)
//   Bit 30: Z (Zero)         Bit 26: F (FIQ disable)
//   Bit 29: C (Carry)        Bits 25:2: Program Counter (24 bits)
//   Bit 28: V (Overflow)     Bits 1:0: Processor Mode

pub const FLAG_N: u32 = 1 << 31;
pub const FLAG_Z: u32 = 1 << 30;
pub const FLAG_C: u32 = 1 << 29;
pub const FLAG_V: u32 = 1 << 28;
pub const FLAG_I: u32 = 1 << 27;
pub const FLAG_F: u32 = 1 << 26;
pub const PC_MASK: u32 = 0x03FF_FFFC;
pub const MODE_MASK: u32 = 0x3;

/// The SWI comment field we use as a halt instruction.
/// The simulator intercepts SWI with this value to stop execution.
pub const HALT_SWI: u32 = 0x123456;

// =========================================================================
// Flags
// =========================================================================

/// Represents the ARM1's four condition flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Flags {
    /// Negative -- set when result's bit 31 is 1.
    pub n: bool,
    /// Zero -- set when result is 0.
    pub z: bool,
    /// Carry -- set on unsigned overflow or shifter carry-out.
    pub c: bool,
    /// Overflow -- set on signed overflow.
    pub v: bool,
}

impl Default for Flags {
    fn default() -> Self {
        Self { n: false, z: false, c: false, v: false }
    }
}

// =========================================================================
// Memory Access
// =========================================================================

/// Records a single memory read or write during instruction execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryAccess {
    pub address: u32,
    pub value: u32,
}

// =========================================================================
// Trace
// =========================================================================

/// Records the state change caused by executing one instruction.
/// Captures the complete before/after snapshot for debugging and
/// cross-language validation.
#[derive(Debug, Clone, PartialEq)]
pub struct Trace {
    pub address: u32,
    pub raw: u32,
    pub mnemonic: String,
    pub condition: String,
    pub condition_met: bool,
    pub regs_before: [u32; 16],
    pub regs_after: [u32; 16],
    pub flags_before: Flags,
    pub flags_after: Flags,
    pub memory_reads: Vec<MemoryAccess>,
    pub memory_writes: Vec<MemoryAccess>,
}

// =========================================================================
// ALU Result
// =========================================================================

/// Holds the output of an ALU operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ALUResult {
    pub result: u32,
    pub n: bool,
    pub z: bool,
    pub c: bool,
    pub v: bool,
    pub write_result: bool,
}

// =========================================================================
// Condition Evaluator
// =========================================================================
//
// Every ARM instruction's 4-bit condition code is tested against the current
// flags. The instruction only executes if the condition is satisfied.
// This is the behavioral equivalent of the ARM1's condition evaluation
// hardware -- a small block of combinational logic.

/// Tests whether the given condition code is satisfied by the current flags.
pub fn evaluate_condition(cond: u32, flags: Flags) -> bool {
    match cond {
        COND_EQ => flags.z,
        COND_NE => !flags.z,
        COND_CS => flags.c,
        COND_CC => !flags.c,
        COND_MI => flags.n,
        COND_PL => !flags.n,
        COND_VS => flags.v,
        COND_VC => !flags.v,
        COND_HI => flags.c && !flags.z,
        COND_LS => !flags.c || flags.z,
        COND_GE => flags.n == flags.v,
        COND_LT => flags.n != flags.v,
        COND_GT => !flags.z && (flags.n == flags.v),
        COND_LE => flags.z || (flags.n != flags.v),
        COND_AL => true,
        COND_NV => false,
        _ => false,
    }
}

// =========================================================================
// Barrel Shifter
// =========================================================================
//
// The barrel shifter is the ARM1's most distinctive hardware feature. On the
// real chip, it was a 32x32 crossbar of pass transistors -- each of the 32
// output bits could be connected to any of the 32 input bits. This allowed
// shifting and rotating a value by any amount in a single clock cycle, at
// zero additional cost.

/// Applies a shift operation to a 32-bit value.
///
/// Returns (result, carry_out).
///
/// Parameters:
/// - `value`: the 32-bit input (from register Rm)
/// - `shift_type`: 0=LSL, 1=LSR, 2=ASR, 3=ROR
/// - `amount`: number of positions to shift
/// - `carry_in`: current carry flag
/// - `by_register`: true if shift amount comes from a register
pub fn barrel_shift(value: u32, shift_type: u32, amount: u32, carry_in: bool, by_register: bool) -> (u32, bool) {
    // When shifting by a register value, if the amount is 0 the value passes
    // through unchanged and the carry flag is unaffected.
    if by_register && amount == 0 {
        return (value, carry_in);
    }

    match shift_type {
        SHIFT_LSL => shift_lsl(value, amount, carry_in, by_register),
        SHIFT_LSR => shift_lsr(value, amount, carry_in, by_register),
        SHIFT_ASR => shift_asr(value, amount, carry_in, by_register),
        SHIFT_ROR => shift_ror(value, amount, carry_in, by_register),
        _ => (value, carry_in),
    }
}

/// Logical Shift Left.
///
///   Before (LSL #3):  [b31 b30 ... b3 b2 b1 b0]
///   After:            [b28 b27 ... b0  0  0  0 ]
///   Carry out:        b29 (the last bit shifted out)
///
/// Special case: LSL #0 means "no shift" -- value unchanged, carry unchanged.
fn shift_lsl(value: u32, amount: u32, carry_in: bool, _by_register: bool) -> (u32, bool) {
    if amount == 0 {
        return (value, carry_in);
    }
    if amount >= 32 {
        if amount == 32 {
            return (0, (value & 1) != 0);
        }
        return (0, false);
    }
    let carry = (value >> (32 - amount)) & 1;
    (value << amount, carry != 0)
}

/// Logical Shift Right.
///
/// Special case: immediate LSR #0 encodes LSR #32 (result = 0, carry = bit 31).
fn shift_lsr(value: u32, amount: u32, carry_in: bool, by_register: bool) -> (u32, bool) {
    if amount == 0 && !by_register {
        return (0, (value >> 31) != 0);
    }
    if amount == 0 {
        return (value, carry_in);
    }
    if amount >= 32 {
        if amount == 32 {
            return (0, (value >> 31) != 0);
        }
        return (0, false);
    }
    let carry = (value >> (amount - 1)) & 1;
    (value >> amount, carry != 0)
}

/// Arithmetic Shift Right (sign-extending).
///
/// The sign bit (bit 31) is replicated into the vacated positions,
/// preserving the sign of a two's complement number.
///
/// Special case: immediate ASR #0 encodes ASR #32:
///   If bit 31 = 0: result = 0x00000000, carry = 0
///   If bit 31 = 1: result = 0xFFFFFFFF, carry = 1
fn shift_asr(value: u32, amount: u32, carry_in: bool, by_register: bool) -> (u32, bool) {
    let sign_bit = (value >> 31) != 0;

    if amount == 0 && !by_register {
        if sign_bit {
            return (0xFFFF_FFFF, true);
        }
        return (0, false);
    }
    if amount == 0 {
        return (value, carry_in);
    }
    if amount >= 32 {
        if sign_bit {
            return (0xFFFF_FFFF, true);
        }
        return (0, false);
    }

    // Arithmetic right shift: cast to signed, shift, cast back.
    let signed = value as i32;
    let result = (signed >> amount) as u32;
    let carry = (value >> (amount - 1)) & 1;
    (result, carry != 0)
}

/// Rotate Right.
///
/// Special case: immediate ROR #0 encodes RRX (Rotate Right Extended):
///   33-bit rotation through carry flag. Old carry -> bit 31, old bit 0 -> new carry.
fn shift_ror(value: u32, amount: u32, carry_in: bool, by_register: bool) -> (u32, bool) {
    if amount == 0 && !by_register {
        // RRX -- Rotate Right Extended (33-bit rotation through carry)
        let carry = (value & 1) != 0;
        let mut result = value >> 1;
        if carry_in {
            result |= 0x8000_0000;
        }
        return (result, carry);
    }
    if amount == 0 {
        return (value, carry_in);
    }

    // Normalize rotation amount to 0-31
    let amount = amount & 31;
    if amount == 0 {
        // ROR by 32 (or multiple of 32): value unchanged, carry = bit 31
        return (value, (value >> 31) != 0);
    }

    let result = (value >> amount) | (value << (32 - amount));
    let carry = (result >> 31) & 1;
    (result, carry != 0)
}

/// Decodes a rotated immediate value from the Operand2 field when the I bit
/// is set (bit 25 = 1).
///
/// The encoding packs a wide range of constants into 12 bits:
///   - Bits 7:0:   8-bit immediate value
///   - Bits 11:8:  4-bit rotation amount (actual rotation = 2 * this value)
///
/// Returns (value, carry_out).
pub fn decode_immediate(imm8: u32, rotate: u32) -> (u32, bool) {
    let rotate_amount = rotate * 2;
    if rotate_amount == 0 {
        return (imm8, false);
    }
    let value = (imm8 >> rotate_amount) | (imm8 << (32 - rotate_amount));
    let carry_out = (value >> 31) != 0;
    (value, carry_out)
}

// =========================================================================
// ALU
// =========================================================================
//
// The ARM1's ALU performs 16 operations. It takes two 32-bit inputs (Rn and
// the barrel-shifted Operand2) and produces a 32-bit result plus four
// condition flags (N, Z, C, V).
//
// Flag computation differs for logical vs arithmetic ops:
//   Arithmetic: C = carry out from 32-bit adder, V = signed overflow
//   Logical: C = carry from barrel shifter, V = unchanged

/// Performs one of the 16 ALU operations.
pub fn alu_execute(opcode: u32, a: u32, b: u32, carry_in: bool, shifter_carry: bool, old_v: bool) -> ALUResult {
    let mut result: u32;
    let carry: bool;
    let overflow: bool;
    let write_result = !is_test_op(opcode);

    match opcode {
        // -- Logical operations --
        // C flag comes from barrel shifter, V flag is preserved.
        OP_AND | OP_TST => {
            result = a & b;
            carry = shifter_carry;
            overflow = old_v;
        }
        OP_EOR | OP_TEQ => {
            result = a ^ b;
            carry = shifter_carry;
            overflow = old_v;
        }
        OP_ORR => {
            result = a | b;
            carry = shifter_carry;
            overflow = old_v;
        }
        OP_MOV => {
            result = b;
            carry = shifter_carry;
            overflow = old_v;
        }
        OP_BIC => {
            result = a & !b;
            carry = shifter_carry;
            overflow = old_v;
        }
        OP_MVN => {
            result = !b;
            carry = shifter_carry;
            overflow = old_v;
        }

        // -- Arithmetic operations --
        // C flag comes from the adder carry-out, V detects signed overflow.
        // Subtraction is done via two's complement: A - B = A + NOT(B) + 1
        OP_ADD | OP_CMN => {
            let (r, c, v) = add32(a, b, false);
            result = r;
            carry = c;
            overflow = v;
        }
        OP_ADC => {
            let (r, c, v) = add32(a, b, carry_in);
            result = r;
            carry = c;
            overflow = v;
        }
        OP_SUB | OP_CMP => {
            let (r, c, v) = add32(a, !b, true);
            result = r;
            carry = c;
            overflow = v;
        }
        OP_SBC => {
            let (r, c, v) = add32(a, !b, carry_in);
            result = r;
            carry = c;
            overflow = v;
        }
        OP_RSB => {
            let (r, c, v) = add32(b, !a, true);
            result = r;
            carry = c;
            overflow = v;
        }
        OP_RSC => {
            let (r, c, v) = add32(b, !a, carry_in);
            result = r;
            carry = c;
            overflow = v;
        }
        _ => {
            result = 0;
            carry = false;
            overflow = false;
        }
    }

    // Suppress unused mut warning -- result is assigned in every branch
    let _ = &mut result;

    ALUResult {
        result,
        n: (result >> 31) != 0,
        z: result == 0,
        c: carry,
        v: overflow,
        write_result,
    }
}

/// 32-bit addition with carry-in, computing carry-out and overflow.
///
/// We use 64-bit arithmetic for clarity. The real ARM1 uses a 32-stage
/// ripple-carry adder.
fn add32(a: u32, b: u32, carry_in: bool) -> (u32, bool, bool) {
    let cin: u64 = if carry_in { 1 } else { 0 };
    let sum = (a as u64) + (b as u64) + cin;
    let result = sum as u32;
    let carry = (sum >> 32) != 0;

    // Overflow detection: both operands have same sign, but result differs.
    //   overflow = ((a ^ result) & (b ^ result)) >> 31
    let overflow = (((a ^ result) & (b ^ result)) >> 31) != 0;
    (result, carry, overflow)
}

// =========================================================================
// Instruction Types
// =========================================================================

/// Instruction class, determined by bits 27:25 of the instruction word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstType {
    DataProcessing,
    LoadStore,
    BlockTransfer,
    Branch,
    SWI,
    Coprocessor,
    Undefined,
}

// =========================================================================
// Decoded Instruction
// =========================================================================

/// Holds all fields extracted from a 32-bit ARM instruction word.
///
/// This is the behavioral equivalent of the ARM1's PLA decoder. The real
/// hardware uses combinational gate trees to extract these fields in parallel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedInstruction {
    pub raw: u32,
    pub inst_type: InstType,
    pub cond: u32,

    // Data Processing fields
    pub opcode: u32,
    pub s: bool,
    pub rn: usize,
    pub rd: usize,
    pub immediate: bool,

    // Operand2 -- immediate form
    pub imm8: u32,
    pub rotate: u32,

    // Operand2 -- register form
    pub rm: usize,
    pub shift_type: u32,
    pub shift_by_reg: bool,
    pub shift_imm: u32,
    pub rs: usize,

    // Load/Store fields
    pub load: bool,
    pub byte: bool,
    pub pre_index: bool,
    pub up: bool,
    pub write_back: bool,
    pub offset12: u32,

    // Block Transfer fields
    pub register_list: u16,
    pub force_user: bool,

    // Branch fields
    pub link: bool,
    pub branch_offset: i32,

    // SWI fields
    pub swi_comment: u32,
}

impl Default for DecodedInstruction {
    fn default() -> Self {
        Self {
            raw: 0,
            inst_type: InstType::Undefined,
            cond: 0,
            opcode: 0,
            s: false,
            rn: 0,
            rd: 0,
            immediate: false,
            imm8: 0,
            rotate: 0,
            rm: 0,
            shift_type: 0,
            shift_by_reg: false,
            shift_imm: 0,
            rs: 0,
            load: false,
            byte: false,
            pre_index: false,
            up: false,
            write_back: false,
            offset12: 0,
            register_list: 0,
            force_user: false,
            link: false,
            branch_offset: 0,
            swi_comment: 0,
        }
    }
}

// =========================================================================
// Decoder
// =========================================================================

/// Extracts all fields from a 32-bit ARM instruction.
pub fn decode(instruction: u32) -> DecodedInstruction {
    let mut d = DecodedInstruction {
        raw: instruction,
        cond: (instruction >> 28) & 0xF,
        ..Default::default()
    };

    let bits2726 = (instruction >> 26) & 0x3;
    let bit25 = (instruction >> 25) & 0x1;

    match (bits2726, bit25) {
        (0, _) => {
            d.inst_type = InstType::DataProcessing;
            decode_data_processing(&mut d, instruction);
        }
        (1, _) => {
            d.inst_type = InstType::LoadStore;
            decode_load_store(&mut d, instruction);
        }
        (2, 0) => {
            d.inst_type = InstType::BlockTransfer;
            decode_block_transfer(&mut d, instruction);
        }
        (2, 1) => {
            d.inst_type = InstType::Branch;
            decode_branch(&mut d, instruction);
        }
        (3, _) => {
            if (instruction >> 24) & 0xF == 0xF {
                d.inst_type = InstType::SWI;
                d.swi_comment = instruction & 0x00FF_FFFF;
            } else {
                d.inst_type = InstType::Coprocessor;
            }
        }
        _ => {
            d.inst_type = InstType::Undefined;
        }
    }

    d
}

fn decode_data_processing(d: &mut DecodedInstruction, inst: u32) {
    d.immediate = ((inst >> 25) & 1) == 1;
    d.opcode = (inst >> 21) & 0xF;
    d.s = ((inst >> 20) & 1) == 1;
    d.rn = ((inst >> 16) & 0xF) as usize;
    d.rd = ((inst >> 12) & 0xF) as usize;

    if d.immediate {
        d.imm8 = inst & 0xFF;
        d.rotate = (inst >> 8) & 0xF;
    } else {
        d.rm = (inst & 0xF) as usize;
        d.shift_type = (inst >> 5) & 0x3;
        d.shift_by_reg = ((inst >> 4) & 1) == 1;
        if d.shift_by_reg {
            d.rs = ((inst >> 8) & 0xF) as usize;
        } else {
            d.shift_imm = (inst >> 7) & 0x1F;
        }
    }
}

fn decode_load_store(d: &mut DecodedInstruction, inst: u32) {
    // Note: for LDR/STR, I=1 means REGISTER offset (opposite of data processing!)
    d.immediate = ((inst >> 25) & 1) == 1;
    d.pre_index = ((inst >> 24) & 1) == 1;
    d.up = ((inst >> 23) & 1) == 1;
    d.byte = ((inst >> 22) & 1) == 1;
    d.write_back = ((inst >> 21) & 1) == 1;
    d.load = ((inst >> 20) & 1) == 1;
    d.rn = ((inst >> 16) & 0xF) as usize;
    d.rd = ((inst >> 12) & 0xF) as usize;

    if d.immediate {
        // Register offset
        d.rm = (inst & 0xF) as usize;
        d.shift_type = (inst >> 5) & 0x3;
        d.shift_imm = (inst >> 7) & 0x1F;
    } else {
        // Immediate offset
        d.offset12 = inst & 0xFFF;
    }
}

fn decode_block_transfer(d: &mut DecodedInstruction, inst: u32) {
    d.pre_index = ((inst >> 24) & 1) == 1;
    d.up = ((inst >> 23) & 1) == 1;
    d.force_user = ((inst >> 22) & 1) == 1;
    d.write_back = ((inst >> 21) & 1) == 1;
    d.load = ((inst >> 20) & 1) == 1;
    d.rn = ((inst >> 16) & 0xF) as usize;
    d.register_list = (inst & 0xFFFF) as u16;
}

fn decode_branch(d: &mut DecodedInstruction, inst: u32) {
    d.link = ((inst >> 24) & 1) == 1;

    // Sign-extend 24-bit offset to 32 bits, then shift left by 2.
    let mut offset = inst & 0x00FF_FFFF;
    if (offset >> 23) != 0 {
        offset |= 0xFF00_0000;
    }
    d.branch_offset = (offset as i32) << 2;
}

// =========================================================================
// Disassembly
// =========================================================================

impl DecodedInstruction {
    /// Returns a human-readable assembly string for the instruction.
    pub fn disassemble(&self) -> String {
        let cond = cond_string(self.cond);
        match self.inst_type {
            InstType::DataProcessing => self.disasm_data_processing(cond),
            InstType::LoadStore => self.disasm_load_store(cond),
            InstType::BlockTransfer => self.disasm_block_transfer(cond),
            InstType::Branch => self.disasm_branch(cond),
            InstType::SWI => {
                if self.swi_comment == HALT_SWI {
                    format!("HLT{cond}")
                } else {
                    format!("SWI{cond} #0x{:X}", self.swi_comment)
                }
            }
            InstType::Coprocessor => format!("CDP{cond} (undefined)"),
            InstType::Undefined => format!("UND{cond} #0x{:08X}", self.raw),
        }
    }

    fn disasm_data_processing(&self, cond: &str) -> String {
        let op = op_string(self.opcode);
        let suf = if self.s && !is_test_op(self.opcode) { "S" } else { "" };
        let op2 = self.disasm_operand2();

        if self.opcode == OP_MOV || self.opcode == OP_MVN {
            format!("{op}{cond}{suf} R{}, {op2}", self.rd)
        } else if is_test_op(self.opcode) {
            format!("{op}{cond} R{}, {op2}", self.rn)
        } else {
            format!("{op}{cond}{suf} R{}, R{}, {op2}", self.rd, self.rn)
        }
    }

    fn disasm_operand2(&self) -> String {
        if self.immediate {
            let (val, _) = decode_immediate(self.imm8, self.rotate);
            return format!("#{val}");
        }
        if !self.shift_by_reg && self.shift_imm == 0 && self.shift_type == SHIFT_LSL {
            return format!("R{}", self.rm);
        }
        if self.shift_by_reg {
            return format!("R{}, {} R{}", self.rm, shift_string(self.shift_type), self.rs);
        }
        let mut amount = self.shift_imm;
        if amount == 0 {
            match self.shift_type {
                SHIFT_LSR | SHIFT_ASR => amount = 32,
                SHIFT_ROR => return format!("R{}, RRX", self.rm),
                _ => {}
            }
        }
        format!("R{}, {} #{amount}", self.rm, shift_string(self.shift_type))
    }

    fn disasm_load_store(&self, cond: &str) -> String {
        let op = if self.load { "LDR" } else { "STR" };
        let b_suf = if self.byte { "B" } else { "" };

        let offset = if self.immediate {
            let mut s = format!("R{}", self.rm);
            if self.shift_imm != 0 {
                s += &format!(", {} #{}", shift_string(self.shift_type), self.shift_imm);
            }
            s
        } else {
            format!("#{}", self.offset12)
        };

        let sign = if !self.up { "-" } else { "" };

        if self.pre_index {
            let wb = if self.write_back { "!" } else { "" };
            format!("{op}{cond}{b_suf} R{}, [R{}, {sign}{offset}]{wb}", self.rd, self.rn)
        } else {
            format!("{op}{cond}{b_suf} R{}, [R{}], {sign}{offset}", self.rd, self.rn)
        }
    }

    fn disasm_block_transfer(&self, cond: &str) -> String {
        let op = if self.load { "LDM" } else { "STM" };
        let mode = match (self.pre_index, self.up) {
            (false, true) => "IA",
            (true, true) => "IB",
            (false, false) => "DA",
            (true, false) => "DB",
        };
        let wb = if self.write_back { "!" } else { "" };
        let regs = disasm_reg_list(self.register_list);
        format!("{op}{cond}{mode} R{}{wb}, {{{regs}}}", self.rn)
    }

    fn disasm_branch(&self, cond: &str) -> String {
        let op = if self.link { "BL" } else { "B" };
        format!("{op}{cond} #{}", self.branch_offset)
    }
}

fn disasm_reg_list(list: u16) -> String {
    let mut parts = Vec::new();
    for i in 0..16 {
        if (list >> i) & 1 == 1 {
            match i {
                15 => parts.push("PC".to_string()),
                14 => parts.push("LR".to_string()),
                13 => parts.push("SP".to_string()),
                _ => parts.push(format!("R{i}")),
            }
        }
    }
    parts.join(", ")
}

// =========================================================================
// ARM1 CPU Simulator
// =========================================================================
//
// This is the top-level ARM1 CPU simulator. It implements the complete
// ARMv1 instruction set as designed by Sophie Wilson and Steve Furber.
//
// Architecture:
//   - 16 visible registers (R0-R15), 25 physical (banked for FIQ/IRQ/SVC)
//   - R15 = combined Program Counter + Status Register
//   - 3-stage pipeline: Fetch -> Decode -> Execute

/// The top-level simulator for the first ARM processor.
pub struct ARM1 {
    /// Register file: 27 physical 32-bit registers.
    ///
    /// Layout:
    ///   [0..15]  = R0-R15 (User/System mode base registers)
    ///   [16..22] = R8_fiq..R14_fiq
    ///   [23..24] = R13_irq, R14_irq
    ///   [25..26] = R13_svc, R14_svc
    regs: [u32; 27],

    /// Memory -- byte-addressable, little-endian.
    memory: Vec<u8>,

    /// Has the CPU been halted?
    halted: bool,
}

impl ARM1 {
    /// Creates a new ARM1 simulator with the given memory size.
    ///
    /// On power-on, the ARM1 enters Supervisor mode with IRQs and FIQs disabled,
    /// and begins executing from address 0x00000000 (the Reset vector).
    pub fn new(memory_size: usize) -> Self {
        let memory_size = if memory_size == 0 { 1024 * 1024 } else { memory_size };
        let mut cpu = Self {
            regs: [0; 27],
            memory: vec![0u8; memory_size],
            halted: false,
        };
        cpu.reset();
        cpu
    }

    /// Restores the CPU to its power-on state.
    pub fn reset(&mut self) {
        self.regs = [0; 27];
        // SVC mode, IRQ/FIQ disabled
        self.regs[15] = FLAG_I | FLAG_F | MODE_SVC;
        self.halted = false;
    }

    // =====================================================================
    // Register access
    // =====================================================================

    /// Reads a register (R0-R15), respecting mode banking.
    pub fn read_register(&self, index: usize) -> u32 {
        self.regs[self.physical_reg(index)]
    }

    /// Writes a register (R0-R15), respecting mode banking.
    pub fn write_register(&mut self, index: usize, value: u32) {
        let phys = self.physical_reg(index);
        self.regs[phys] = value;
    }

    /// Maps a logical register index (0-15) to a physical register index (0-26)
    /// based on the current processor mode.
    fn physical_reg(&self, index: usize) -> usize {
        let mode = self.mode();
        match (mode, index) {
            (MODE_FIQ, 8..=14) => 16 + (index - 8),
            (MODE_IRQ, 13..=14) => 23 + (index - 13),
            (MODE_SVC, 13..=14) => 25 + (index - 13),
            _ => index,
        }
    }

    /// Returns the current program counter (26-bit address).
    pub fn pc(&self) -> u32 {
        self.regs[15] & PC_MASK
    }

    /// Sets the PC portion of R15 without changing flags/mode.
    pub fn set_pc(&mut self, addr: u32) {
        self.regs[15] = (self.regs[15] & !PC_MASK) | (addr & PC_MASK);
    }

    /// Returns the current condition flags.
    pub fn flags(&self) -> Flags {
        let r15 = self.regs[15];
        Flags {
            n: (r15 & FLAG_N) != 0,
            z: (r15 & FLAG_Z) != 0,
            c: (r15 & FLAG_C) != 0,
            v: (r15 & FLAG_V) != 0,
        }
    }

    /// Updates the condition flags in R15.
    pub fn set_flags(&mut self, f: Flags) {
        let mut r15 = self.regs[15] & !(FLAG_N | FLAG_Z | FLAG_C | FLAG_V);
        if f.n { r15 |= FLAG_N; }
        if f.z { r15 |= FLAG_Z; }
        if f.c { r15 |= FLAG_C; }
        if f.v { r15 |= FLAG_V; }
        self.regs[15] = r15;
    }

    /// Returns the current processor mode.
    pub fn mode(&self) -> u32 {
        self.regs[15] & MODE_MASK
    }

    /// Returns true if the CPU has been halted.
    pub fn halted(&self) -> bool {
        self.halted
    }

    /// Returns the raw R15 register value.
    pub fn r15_raw(&self) -> u32 {
        self.regs[15]
    }

    // =====================================================================
    // Memory access
    // =====================================================================

    /// Reads a 32-bit word from memory (little-endian, word-aligned).
    pub fn read_word(&self, addr: u32) -> u32 {
        let addr = (addr & PC_MASK) as usize;
        let a = addr & !3; // word-align
        if a + 3 >= self.memory.len() {
            return 0;
        }
        u32::from_le_bytes([
            self.memory[a],
            self.memory[a + 1],
            self.memory[a + 2],
            self.memory[a + 3],
        ])
    }

    /// Writes a 32-bit word to memory (little-endian, word-aligned).
    pub fn write_word(&mut self, addr: u32, value: u32) {
        let addr = (addr & PC_MASK) as usize;
        let a = addr & !3;
        if a + 3 >= self.memory.len() {
            return;
        }
        let bytes = value.to_le_bytes();
        self.memory[a..a + 4].copy_from_slice(&bytes);
    }

    /// Reads a single byte from memory.
    pub fn read_byte(&self, addr: u32) -> u8 {
        let a = (addr & PC_MASK) as usize;
        if a >= self.memory.len() {
            return 0;
        }
        self.memory[a]
    }

    /// Writes a single byte to memory.
    pub fn write_byte(&mut self, addr: u32, value: u8) {
        let a = (addr & PC_MASK) as usize;
        if a >= self.memory.len() {
            return;
        }
        self.memory[a] = value;
    }

    /// Returns a reference to the raw memory array.
    pub fn memory(&self) -> &[u8] {
        &self.memory
    }

    /// Loads raw bytes into memory at the given start address.
    pub fn load_program(&mut self, code: &[u8], start_addr: u32) {
        for (i, &b) in code.iter().enumerate() {
            let addr = start_addr as usize + i;
            if addr < self.memory.len() {
                self.memory[addr] = b;
            }
        }
    }

    /// Loads a program from u32 instruction words (convenience helper).
    pub fn load_program_words(&mut self, instructions: &[u32], start_addr: u32) {
        let mut code = Vec::with_capacity(instructions.len() * 4);
        for &inst in instructions {
            code.extend_from_slice(&inst.to_le_bytes());
        }
        self.load_program(&code, start_addr);
    }

    // =====================================================================
    // Execution
    // =====================================================================

    /// Executes one instruction and returns a trace of what happened.
    pub fn step(&mut self) -> Trace {
        let pc = self.pc();
        let mut regs_before = [0u32; 16];
        for i in 0..16 {
            regs_before[i] = self.read_register(i);
        }
        let flags_before = self.flags();

        // Fetch
        let instruction = self.read_word(pc);

        // Decode
        let decoded = decode(instruction);

        // Evaluate condition
        let cond_met = evaluate_condition(decoded.cond, flags_before);

        let mut trace = Trace {
            address: pc,
            raw: instruction,
            mnemonic: decoded.disassemble(),
            condition: cond_string(decoded.cond).to_string(),
            condition_met: cond_met,
            regs_before,
            regs_after: [0u32; 16],
            flags_before,
            flags_after: Flags::default(),
            memory_reads: Vec::new(),
            memory_writes: Vec::new(),
        };

        // Advance PC (default: next instruction)
        self.set_pc(pc + 4);

        if cond_met {
            match decoded.inst_type {
                InstType::DataProcessing => self.execute_data_processing(&decoded, &mut trace),
                InstType::LoadStore => self.execute_load_store(&decoded, &mut trace),
                InstType::BlockTransfer => self.execute_block_transfer(&decoded, &mut trace),
                InstType::Branch => self.execute_branch(&decoded, &mut trace),
                InstType::SWI => self.execute_swi(&decoded, &mut trace),
                InstType::Coprocessor | InstType::Undefined => self.trap_undefined(pc),
            }
        }

        // Capture state after execution
        for i in 0..16 {
            trace.regs_after[i] = self.read_register(i);
        }
        trace.flags_after = self.flags();

        trace
    }

    /// Executes instructions until halted or max_steps reached.
    pub fn run(&mut self, max_steps: usize) -> Vec<Trace> {
        let mut traces = Vec::with_capacity(max_steps.min(1024));
        for _ in 0..max_steps {
            if self.halted {
                break;
            }
            traces.push(self.step());
        }
        traces
    }

    // =====================================================================
    // Data Processing execution
    // =====================================================================

    fn execute_data_processing(&mut self, d: &DecodedInstruction, _trace: &mut Trace) {
        // Get first operand (Rn)
        let a = if d.opcode != OP_MOV && d.opcode != OP_MVN {
            self.read_reg_for_exec(d.rn)
        } else {
            0
        };

        // Get second operand through barrel shifter
        let flags = self.flags();
        let (b, shifter_carry) = if d.immediate {
            let (val, sc) = decode_immediate(d.imm8, d.rotate);
            if d.rotate == 0 {
                (val, flags.c)
            } else {
                (val, sc)
            }
        } else {
            let rm_val = self.read_reg_for_exec(d.rm);
            let shift_amount = if d.shift_by_reg {
                self.read_reg_for_exec(d.rs) & 0xFF
            } else {
                d.shift_imm
            };
            barrel_shift(rm_val, d.shift_type, shift_amount, flags.c, d.shift_by_reg)
        };

        // Execute ALU operation
        let result = alu_execute(d.opcode, a, b, flags.c, shifter_carry, flags.v);

        // Write result to Rd (unless test-only)
        if result.write_result {
            if d.rd == 15 {
                if d.s {
                    // MOVS PC, LR -- restore PC and flags
                    self.regs[15] = result.result;
                } else {
                    self.set_pc(result.result & PC_MASK);
                }
            } else {
                self.write_register(d.rd, result.result);
            }
        }

        // Update flags if S bit set (and Rd != R15, handled above)
        if d.s && d.rd != 15 {
            self.set_flags(Flags { n: result.n, z: result.z, c: result.c, v: result.v });
        }
        // Test-only ops always update flags
        if is_test_op(d.opcode) {
            self.set_flags(Flags { n: result.n, z: result.z, c: result.c, v: result.v });
        }
    }

    /// Reads a register as it would appear during execution.
    /// For R15, returns PC + 8 (accounting for the 3-stage pipeline).
    fn read_reg_for_exec(&self, index: usize) -> u32 {
        if index == 15 {
            // R15 reads as PC + 8. We already advanced PC by 4 in step(), so add 4 more.
            self.regs[15].wrapping_add(4)
        } else {
            self.read_register(index)
        }
    }

    // =====================================================================
    // Load/Store execution
    // =====================================================================

    fn execute_load_store(&mut self, d: &DecodedInstruction, trace: &mut Trace) {
        let offset = if d.immediate {
            // Register offset (with optional shift)
            let mut rm_val = self.read_reg_for_exec(d.rm);
            if d.shift_imm != 0 {
                let (shifted, _) = barrel_shift(rm_val, d.shift_type, d.shift_imm, self.flags().c, false);
                rm_val = shifted;
            }
            rm_val
        } else {
            d.offset12
        };

        let base = self.read_reg_for_exec(d.rn);
        let addr = if d.up { base.wrapping_add(offset) } else { base.wrapping_sub(offset) };
        let transfer_addr = if d.pre_index { addr } else { base };

        if d.load {
            let value = if d.byte {
                self.read_byte(transfer_addr) as u32
            } else {
                let mut v = self.read_word(transfer_addr);
                // ARM1 quirk: unaligned word loads rotate the data
                let rotation = (transfer_addr & 3) * 8;
                if rotation != 0 {
                    v = (v >> rotation) | (v << (32 - rotation));
                }
                v
            };
            trace.memory_reads.push(MemoryAccess { address: transfer_addr, value });
            if d.rd == 15 {
                self.regs[15] = value;
            } else {
                self.write_register(d.rd, value);
            }
        } else {
            let value = self.read_reg_for_exec(d.rd);
            if d.byte {
                self.write_byte(transfer_addr, (value & 0xFF) as u8);
            } else {
                self.write_word(transfer_addr, value);
            }
            trace.memory_writes.push(MemoryAccess { address: transfer_addr, value });
        }

        // Write-back
        if d.write_back || !d.pre_index {
            if d.rn != 15 {
                self.write_register(d.rn, addr);
            }
        }
    }

    // =====================================================================
    // Block Transfer execution (LDM/STM)
    // =====================================================================

    fn execute_block_transfer(&mut self, d: &DecodedInstruction, trace: &mut Trace) {
        let base = self.read_register(d.rn);
        let reg_list = d.register_list;

        let count: u32 = (0..16).filter(|i| (reg_list >> i) & 1 == 1).count() as u32;
        if count == 0 {
            return;
        }

        let start_addr = match (d.pre_index, d.up) {
            (false, true) => base,              // IA
            (true, true) => base + 4,           // IB
            (false, false) => base - (count * 4) + 4, // DA
            (true, false) => base - (count * 4),      // DB
        };

        let mut addr = start_addr;
        for i in 0..16usize {
            if (reg_list >> i) & 1 == 0 {
                continue;
            }

            if d.load {
                let value = self.read_word(addr);
                trace.memory_reads.push(MemoryAccess { address: addr, value });
                if i == 15 {
                    self.regs[15] = value;
                } else {
                    self.write_register(i, value);
                }
            } else {
                let value = if i == 15 {
                    self.regs[15].wrapping_add(4) // PC + 8 but we already added 4
                } else {
                    self.read_register(i)
                };
                self.write_word(addr, value);
                trace.memory_writes.push(MemoryAccess { address: addr, value });
            }
            addr += 4;
        }

        if d.write_back {
            let new_base = if d.up {
                base + (count * 4)
            } else {
                base - (count * 4)
            };
            self.write_register(d.rn, new_base);
        }
    }

    // =====================================================================
    // Branch execution
    // =====================================================================

    fn execute_branch(&mut self, d: &DecodedInstruction, _trace: &mut Trace) {
        let branch_base = self.pc().wrapping_add(4);

        if d.link {
            let return_addr = self.regs[15];
            self.write_register(14, return_addr);
        }

        let target = (branch_base as i32).wrapping_add(d.branch_offset) as u32;
        self.set_pc(target & PC_MASK);
    }

    // =====================================================================
    // SWI execution
    // =====================================================================

    fn execute_swi(&mut self, d: &DecodedInstruction, _trace: &mut Trace) {
        if d.swi_comment == HALT_SWI {
            self.halted = true;
            return;
        }

        // Real SWI: enter Supervisor mode
        self.regs[25] = self.regs[15]; // R13_svc
        self.regs[26] = self.regs[15]; // R14_svc

        let mut r15 = self.regs[15];
        r15 = (r15 & !MODE_MASK) | MODE_SVC;
        r15 |= FLAG_I;
        self.regs[15] = r15;
        self.set_pc(0x08);
    }

    // =====================================================================
    // Exception handling
    // =====================================================================

    fn trap_undefined(&mut self, _instr_addr: u32) {
        self.regs[26] = self.regs[15]; // Save R15 to R14_svc
        let mut r15 = self.regs[15];
        r15 = (r15 & !MODE_MASK) | MODE_SVC;
        r15 |= FLAG_I;
        self.regs[15] = r15;
        self.set_pc(0x04);
    }
}

// =========================================================================
// Encoding helpers
// =========================================================================
//
// These functions create instruction words, useful for writing test programs
// without an assembler.

/// Creates a data processing instruction word.
pub fn encode_data_processing(cond: u32, opcode: u32, s: u32, rn: u32, rd: u32, operand2: u32) -> u32 {
    (cond << 28) | operand2 | (opcode << 21) | (s << 20) | (rn << 16) | (rd << 12)
}

/// Creates a MOV immediate instruction.
pub fn encode_mov_imm(cond: u32, rd: u32, imm8: u32) -> u32 {
    encode_data_processing(cond, OP_MOV, 0, 0, rd, (1 << 25) | imm8)
}

/// Creates a data processing instruction with a register operand.
pub fn encode_alu_reg(cond: u32, opcode: u32, s: u32, rd: u32, rn: u32, rm: u32) -> u32 {
    encode_data_processing(cond, opcode, s, rn, rd, rm)
}

/// Creates a Branch or Branch-with-Link instruction.
pub fn encode_branch(cond: u32, link: bool, offset: i32) -> u32 {
    let mut inst = (cond << 28) | 0x0A00_0000;
    if link {
        inst |= 0x0100_0000;
    }
    let encoded = ((offset >> 2) as u32) & 0x00FF_FFFF;
    inst |= encoded;
    inst
}

/// Creates our pseudo-halt instruction (SWI 0x123456).
pub fn encode_halt() -> u32 {
    (COND_AL << 28) | 0x0F00_0000 | HALT_SWI
}

/// Creates a Load Register instruction with immediate offset.
pub fn encode_ldr(cond: u32, rd: u32, rn: u32, offset: i32, pre_index: bool) -> u32 {
    let mut inst = (cond << 28) | 0x0410_0000;
    inst |= rd << 12;
    inst |= rn << 16;
    if pre_index {
        inst |= 1 << 24;
    }
    if offset >= 0 {
        inst |= 1 << 23;
        inst |= (offset as u32) & 0xFFF;
    } else {
        inst |= ((-offset) as u32) & 0xFFF;
    }
    inst
}

/// Creates a Store Register instruction with immediate offset.
pub fn encode_str(cond: u32, rd: u32, rn: u32, offset: i32, pre_index: bool) -> u32 {
    let mut inst = (cond << 28) | 0x0400_0000;
    inst |= rd << 12;
    inst |= rn << 16;
    if pre_index {
        inst |= 1 << 24;
    }
    if offset >= 0 {
        inst |= 1 << 23;
        inst |= (offset as u32) & 0xFFF;
    } else {
        inst |= ((-offset) as u32) & 0xFFF;
    }
    inst
}

/// Creates a Load Multiple instruction.
pub fn encode_ldm(cond: u32, rn: u32, reg_list: u16, write_back: bool, mode: &str) -> u32 {
    let mut inst = (cond << 28) | 0x0810_0000;
    inst |= rn << 16;
    inst |= reg_list as u32;
    if write_back {
        inst |= 1 << 21;
    }
    match mode {
        "IA" => { inst |= 1 << 23; }
        "IB" => { inst |= 1 << 24; inst |= 1 << 23; }
        "DA" => {}
        "DB" => { inst |= 1 << 24; }
        _ => {}
    }
    inst
}

/// Creates a Store Multiple instruction.
pub fn encode_stm(cond: u32, rn: u32, reg_list: u16, write_back: bool, mode: &str) -> u32 {
    let mut inst = encode_ldm(cond, rn, reg_list, write_back, mode);
    inst &= !(1 << 20); // Clear L bit
    inst
}

// =========================================================================
// Display
// =========================================================================

impl std::fmt::Display for ARM1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mode = mode_string(self.mode());
        let flags = self.flags();
        let flag_str = format!(
            "{}{}{}{}",
            if flags.n { "N" } else { "n" },
            if flags.z { "Z" } else { "z" },
            if flags.c { "C" } else { "c" },
            if flags.v { "V" } else { "v" },
        );
        writeln!(f, "ARM1 [{mode}] {flag_str} PC={:08X}", self.pc())?;
        for i in (0..16).step_by(4) {
            writeln!(
                f,
                "  R{:<2}={:08X}  R{:<2}={:08X}  R{:<2}={:08X}  R{:<2}={:08X}",
                i, self.read_register(i),
                i + 1, self.read_register(i + 1),
                i + 2, self.read_register(i + 2),
                i + 3, self.read_register(i + 3),
            )?;
        }
        Ok(())
    }
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =====================================================================
    // Types and Constants
    // =====================================================================

    #[test]
    fn test_mode_string() {
        assert_eq!(mode_string(MODE_USR), "USR");
        assert_eq!(mode_string(MODE_FIQ), "FIQ");
        assert_eq!(mode_string(MODE_IRQ), "IRQ");
        assert_eq!(mode_string(MODE_SVC), "SVC");
        assert_eq!(mode_string(99), "???");
    }

    #[test]
    fn test_op_string() {
        assert_eq!(op_string(OP_ADD), "ADD");
        assert_eq!(op_string(OP_MOV), "MOV");
        assert_eq!(op_string(99), "???");
    }

    #[test]
    fn test_is_test_op() {
        assert!(is_test_op(OP_TST));
        assert!(is_test_op(OP_CMP));
        assert!(!is_test_op(OP_ADD));
    }

    #[test]
    fn test_is_logical_op() {
        assert!(is_logical_op(OP_AND));
        assert!(is_logical_op(OP_MOV));
        assert!(!is_logical_op(OP_ADD));
    }

    // =====================================================================
    // Condition Evaluator
    // =====================================================================

    #[test]
    fn test_evaluate_condition_comprehensive() {
        let cases: Vec<(&str, u32, Flags, bool)> = vec![
            ("EQ when Z set", COND_EQ, Flags { z: true, ..Default::default() }, true),
            ("EQ when Z clear", COND_EQ, Flags::default(), false),
            ("NE when Z clear", COND_NE, Flags::default(), true),
            ("NE when Z set", COND_NE, Flags { z: true, ..Default::default() }, false),
            ("CS when C set", COND_CS, Flags { c: true, ..Default::default() }, true),
            ("CC when C clear", COND_CC, Flags::default(), true),
            ("MI when N set", COND_MI, Flags { n: true, ..Default::default() }, true),
            ("PL when N clear", COND_PL, Flags::default(), true),
            ("VS when V set", COND_VS, Flags { v: true, ..Default::default() }, true),
            ("VC when V clear", COND_VC, Flags::default(), true),
            ("HI when C=1,Z=0", COND_HI, Flags { c: true, ..Default::default() }, true),
            ("HI when C=1,Z=1", COND_HI, Flags { c: true, z: true, ..Default::default() }, false),
            ("LS when C=0", COND_LS, Flags::default(), true),
            ("LS when Z=1", COND_LS, Flags { c: true, z: true, ..Default::default() }, true),
            ("GE when N=V=0", COND_GE, Flags::default(), true),
            ("GE when N=V=1", COND_GE, Flags { n: true, v: true, ..Default::default() }, true),
            ("GE when N!=V", COND_GE, Flags { n: true, ..Default::default() }, false),
            ("LT when N!=V", COND_LT, Flags { n: true, ..Default::default() }, true),
            ("LT when N=V", COND_LT, Flags::default(), false),
            ("GT when Z=0,N=V", COND_GT, Flags::default(), true),
            ("GT when Z=1", COND_GT, Flags { z: true, ..Default::default() }, false),
            ("LE when Z=1", COND_LE, Flags { z: true, ..Default::default() }, true),
            ("LE when N!=V", COND_LE, Flags { n: true, ..Default::default() }, true),
            ("AL always", COND_AL, Flags::default(), true),
            ("NV never", COND_NV, Flags::default(), false),
        ];
        for (name, cond, flags, want) in &cases {
            assert_eq!(evaluate_condition(*cond, *flags), *want, "case: {name}");
        }
    }

    // =====================================================================
    // Barrel Shifter
    // =====================================================================

    #[test]
    fn test_barrel_shift_lsl() {
        let cases = vec![
            ("LSL #0", 0xFFu32, 0u32, 0xFFu32, false),
            ("LSL #1", 0xFF, 1, 0x1FE, false),
            ("LSL #4", 0xFF, 4, 0xFF0, false),
            ("LSL #31", 1, 31, 0x8000_0000, false),
            ("LSL #32", 1, 32, 0, true),
            ("LSL #33", 1, 33, 0, false),
        ];
        for (name, val, amt, want_val, want_c) in cases {
            let (v, c) = barrel_shift(val, SHIFT_LSL, amt, false, false);
            assert_eq!(v, want_val, "{name}: value");
            assert_eq!(c, want_c, "{name}: carry");
        }
    }

    #[test]
    fn test_barrel_shift_lsr() {
        let (v, c) = barrel_shift(0xFF, SHIFT_LSR, 1, false, false);
        assert_eq!(v, 0x7F);
        assert!(c); // bit 0 was 1

        let (v, c) = barrel_shift(0xFF00, SHIFT_LSR, 8, false, false);
        assert_eq!(v, 0xFF);
        assert!(!c);

        // LSR #0 encodes #32
        let (v, c) = barrel_shift(0x8000_0000, SHIFT_LSR, 0, false, false);
        assert_eq!(v, 0);
        assert!(c);

        // LSR #32 by register
        let (v, c) = barrel_shift(0x8000_0000, SHIFT_LSR, 32, false, true);
        assert_eq!(v, 0);
        assert!(c);
    }

    #[test]
    fn test_barrel_shift_asr() {
        let (v, _) = barrel_shift(0x7FFF_FFFE, SHIFT_ASR, 1, false, false);
        assert_eq!(v, 0x3FFF_FFFF);

        let (v, _) = barrel_shift(0x8000_0000, SHIFT_ASR, 1, false, false);
        assert_eq!(v, 0xC000_0000);

        // ASR #0 encodes #32, negative
        let (v, c) = barrel_shift(0x8000_0000, SHIFT_ASR, 0, false, false);
        assert_eq!(v, 0xFFFF_FFFF);
        assert!(c);

        // ASR #0 encodes #32, positive
        let (v, c) = barrel_shift(0x7FFF_FFFF, SHIFT_ASR, 0, false, false);
        assert_eq!(v, 0);
        assert!(!c);
    }

    #[test]
    fn test_barrel_shift_ror() {
        let (v, _) = barrel_shift(0x0000_000F, SHIFT_ROR, 4, false, false);
        assert_eq!(v, 0xF000_0000);
    }

    #[test]
    fn test_barrel_shift_rrx() {
        let (v, c) = barrel_shift(0x0000_0001, SHIFT_ROR, 0, true, false);
        assert_eq!(v, 0x8000_0000);
        assert!(c); // old bit 0 was 1
    }

    #[test]
    fn test_barrel_shift_by_register_zero() {
        // When shift amount comes from register and is 0, value passes through
        let (v, c) = barrel_shift(0xDEAD_BEEF, SHIFT_LSL, 0, true, true);
        assert_eq!(v, 0xDEAD_BEEF);
        assert!(c); // carry unchanged
    }

    // =====================================================================
    // Decode Immediate
    // =====================================================================

    #[test]
    fn test_decode_immediate() {
        let (v, _) = decode_immediate(0xFF, 0);
        assert_eq!(v, 0xFF);

        let (v, _) = decode_immediate(0xFF, 4);
        assert_eq!(v, 0xFF00_0000);
    }

    // =====================================================================
    // ALU
    // =====================================================================

    #[test]
    fn test_alu_add() {
        let r = alu_execute(OP_ADD, 1, 2, false, false, false);
        assert_eq!(r.result, 3);
        assert!(!r.n);
        assert!(!r.z);
        assert!(!r.c);
        assert!(!r.v);
    }

    #[test]
    fn test_alu_sub_zero() {
        let r = alu_execute(OP_SUB, 5, 5, false, false, false);
        assert_eq!(r.result, 0);
        assert!(r.z);
        assert!(r.c); // no borrow
    }

    #[test]
    fn test_alu_sub_negative() {
        let r = alu_execute(OP_SUB, 3, 5, false, false, false);
        assert!(r.n); // result is negative
        assert!(!r.c); // borrow occurred
    }

    #[test]
    fn test_alu_logical_ops() {
        let r = alu_execute(OP_AND, 0xFF00_FF00, 0x0FF0_0FF0, false, false, false);
        assert_eq!(r.result, 0x0F00_0F00);

        let r = alu_execute(OP_EOR, 0xFF00_FF00, 0x0FF0_0FF0, false, false, false);
        assert_eq!(r.result, 0xF0F0_F0F0);

        let r = alu_execute(OP_ORR, 0xFF00_FF00, 0x0FF0_0FF0, false, false, false);
        assert_eq!(r.result, 0xFFF0_FFF0);

        let r = alu_execute(OP_MOV, 0, 42, false, false, false);
        assert_eq!(r.result, 42);

        let r = alu_execute(OP_MVN, 0, 0, false, false, false);
        assert_eq!(r.result, 0xFFFF_FFFF);

        let r = alu_execute(OP_BIC, 0xFFFF_FFFF, 0x0000_00FF, false, false, false);
        assert_eq!(r.result, 0xFFFF_FF00);
    }

    #[test]
    fn test_alu_test_ops_dont_write() {
        let r = alu_execute(OP_TST, 0xFF, 0x0F, false, false, false);
        assert!(!r.write_result);
        assert_eq!(r.result, 0x0F);

        let r = alu_execute(OP_CMP, 5, 5, false, false, false);
        assert!(!r.write_result);
        assert!(r.z);
    }

    #[test]
    fn test_alu_adc_sbc() {
        // ADC: 0 + 0 + carry
        let r = alu_execute(OP_ADC, 0, 0, true, false, false);
        assert_eq!(r.result, 1);

        // SBC: 5 - 3 - !carry = 5 + !3 + carry = 5 + !3 + 0 when carry=false
        let r = alu_execute(OP_SBC, 5, 3, false, false, false);
        // 5 + (!3) + 0 = 5 + 0xFFFFFFFC + 0 = 1
        assert_eq!(r.result, 1);
    }

    #[test]
    fn test_alu_rsb_rsc() {
        // RSB: B - A = 10 - 3 = 7
        let r = alu_execute(OP_RSB, 3, 10, false, false, false);
        assert_eq!(r.result, 7);

        // RSC: B - A - !C = 10 + !3 + C
        let r = alu_execute(OP_RSC, 3, 10, true, false, false);
        assert_eq!(r.result, 7);
    }

    #[test]
    fn test_alu_overflow() {
        // 0x7FFFFFFF + 1 = 0x80000000, signed overflow
        let r = alu_execute(OP_ADD, 0x7FFF_FFFF, 1, false, false, false);
        assert!(r.v);
        assert!(r.n);
    }

    // =====================================================================
    // Instruction Decoder
    // =====================================================================

    #[test]
    fn test_decode_mov_imm() {
        let inst = encode_mov_imm(COND_AL, 0, 42);
        let d = decode(inst);
        assert_eq!(d.inst_type, InstType::DataProcessing);
        assert_eq!(d.opcode, OP_MOV);
        assert!(d.immediate);
        assert_eq!(d.imm8, 42);
        assert_eq!(d.rd, 0);
    }

    #[test]
    fn test_decode_add_reg() {
        let inst = encode_alu_reg(COND_AL, OP_ADD, 1, 2, 0, 1);
        let d = decode(inst);
        assert_eq!(d.inst_type, InstType::DataProcessing);
        assert_eq!(d.opcode, OP_ADD);
        assert!(d.s);
        assert_eq!(d.rd, 2);
        assert_eq!(d.rn, 0);
        assert_eq!(d.rm, 1);
    }

    #[test]
    fn test_decode_branch() {
        let inst = encode_branch(COND_NE, false, -16);
        let d = decode(inst);
        assert_eq!(d.inst_type, InstType::Branch);
        assert!(!d.link);
        assert_eq!(d.branch_offset, -16);
    }

    #[test]
    fn test_decode_branch_link() {
        let inst = encode_branch(COND_AL, true, 8);
        let d = decode(inst);
        assert_eq!(d.inst_type, InstType::Branch);
        assert!(d.link);
    }

    #[test]
    fn test_decode_halt() {
        let inst = encode_halt();
        let d = decode(inst);
        assert_eq!(d.inst_type, InstType::SWI);
        assert_eq!(d.swi_comment, HALT_SWI);
    }

    #[test]
    fn test_decode_ldr_str() {
        let inst = encode_ldr(COND_AL, 0, 1, 4, true);
        let d = decode(inst);
        assert_eq!(d.inst_type, InstType::LoadStore);
        assert!(d.load);
        assert!(d.pre_index);
        assert!(d.up);
        assert_eq!(d.rd, 0);
        assert_eq!(d.rn, 1);
        assert_eq!(d.offset12, 4);

        let inst = encode_str(COND_AL, 2, 3, -8, true);
        let d = decode(inst);
        assert!(!d.load);
        assert!(!d.up);
        assert_eq!(d.offset12, 8);
    }

    #[test]
    fn test_decode_ldm_stm() {
        let inst = encode_ldm(COND_AL, 13, 0x000F, true, "IA");
        let d = decode(inst);
        assert_eq!(d.inst_type, InstType::BlockTransfer);
        assert!(d.load);
        assert!(d.write_back);
        assert_eq!(d.register_list, 0x000F);
    }

    // =====================================================================
    // Disassembly
    // =====================================================================

    #[test]
    fn test_disassemble() {
        let d = decode(encode_mov_imm(COND_AL, 0, 42));
        assert_eq!(d.disassemble(), "MOV R0, #42");

        let d = decode(encode_halt());
        assert_eq!(d.disassemble(), "HLT");
    }

    // =====================================================================
    // CPU -- Power-on and Reset
    // =====================================================================

    #[test]
    fn test_new_cpu_state() {
        let cpu = ARM1::new(1024);
        assert_eq!(cpu.mode(), MODE_SVC);
        assert_eq!(cpu.pc(), 0);
        assert!(!cpu.halted());
    }

    #[test]
    fn test_reset() {
        let mut cpu = ARM1::new(1024);
        cpu.write_register(0, 42);
        cpu.set_pc(100);
        cpu.reset();
        assert_eq!(cpu.read_register(0), 0);
        assert_eq!(cpu.pc(), 0);
        assert_eq!(cpu.mode(), MODE_SVC);
    }

    // =====================================================================
    // CPU -- Register banking
    // =====================================================================

    #[test]
    fn test_register_banking() {
        let mut cpu = ARM1::new(1024);
        // In SVC mode, R13 and R14 are banked
        cpu.write_register(13, 0xAAAA);
        assert_eq!(cpu.read_register(13), 0xAAAA);

        // Switch to USR mode by modifying R15
        let r15 = cpu.regs[15];
        cpu.regs[15] = (r15 & !MODE_MASK) | MODE_USR;
        assert_eq!(cpu.mode(), MODE_USR);
        // R13 in USR mode should be different (0, since we never wrote to it)
        assert_eq!(cpu.read_register(13), 0);
    }

    // =====================================================================
    // CPU -- Memory
    // =====================================================================

    #[test]
    fn test_memory_word_rw() {
        let mut cpu = ARM1::new(4096);
        cpu.write_word(0x100, 0xDEAD_BEEF);
        assert_eq!(cpu.read_word(0x100), 0xDEAD_BEEF);
    }

    #[test]
    fn test_memory_byte_rw() {
        let mut cpu = ARM1::new(4096);
        cpu.write_byte(0x50, 0xAB);
        assert_eq!(cpu.read_byte(0x50), 0xAB);
    }

    #[test]
    fn test_memory_out_of_bounds() {
        let cpu = ARM1::new(256);
        assert_eq!(cpu.read_word(0x1000), 0);
        assert_eq!(cpu.read_byte(0x1000), 0);
    }

    // =====================================================================
    // CPU -- Full program execution
    // =====================================================================

    #[test]
    fn test_mov_imm_and_halt() {
        let mut cpu = ARM1::new(4096);
        cpu.load_program_words(&[
            encode_mov_imm(COND_AL, 0, 42),
            encode_halt(),
        ], 0);
        let traces = cpu.run(100);
        assert_eq!(traces.len(), 2);
        assert_eq!(cpu.read_register(0), 42);
        assert!(cpu.halted());
    }

    #[test]
    fn test_add_two_registers() {
        let mut cpu = ARM1::new(4096);
        cpu.load_program_words(&[
            encode_mov_imm(COND_AL, 0, 10),
            encode_mov_imm(COND_AL, 1, 20),
            encode_alu_reg(COND_AL, OP_ADD, 0, 2, 0, 1),
            encode_halt(),
        ], 0);
        cpu.run(100);
        assert_eq!(cpu.read_register(2), 30);
    }

    #[test]
    fn test_subs_with_flags() {
        let mut cpu = ARM1::new(4096);
        cpu.load_program_words(&[
            encode_mov_imm(COND_AL, 0, 5),
            encode_mov_imm(COND_AL, 1, 5),
            encode_alu_reg(COND_AL, OP_SUB, 1, 2, 0, 1), // SUBS R2, R0, R1
            encode_halt(),
        ], 0);
        cpu.run(100);
        assert_eq!(cpu.read_register(2), 0);
        assert!(cpu.flags().z);
    }

    #[test]
    fn test_conditional_execution() {
        let mut cpu = ARM1::new(4096);
        cpu.load_program_words(&[
            encode_mov_imm(COND_AL, 0, 5),
            encode_mov_imm(COND_AL, 1, 5),
            encode_alu_reg(COND_AL, OP_SUB, 1, 2, 0, 1), // SUBS: sets Z flag
            encode_mov_imm(COND_NE, 3, 99),  // should NOT execute (Z is set)
            encode_mov_imm(COND_EQ, 4, 42),  // should execute (Z is set)
            encode_halt(),
        ], 0);
        cpu.run(100);
        assert_eq!(cpu.read_register(3), 0);  // skipped
        assert_eq!(cpu.read_register(4), 42); // executed
    }

    #[test]
    fn test_branch() {
        let mut cpu = ARM1::new(4096);
        cpu.load_program_words(&[
            encode_mov_imm(COND_AL, 0, 1),
            encode_branch(COND_AL, false, 4), // skip next instruction
            encode_mov_imm(COND_AL, 0, 99),   // should be skipped
            encode_halt(),
        ], 0);
        cpu.run(100);
        assert_eq!(cpu.read_register(0), 1);
    }

    #[test]
    fn test_loop_sum_1_to_10() {
        let mut cpu = ARM1::new(4096);
        cpu.load_program_words(&[
            encode_mov_imm(COND_AL, 0, 0),   // R0 = sum = 0
            encode_mov_imm(COND_AL, 1, 10),  // R1 = counter = 10
            // loop:
            encode_alu_reg(COND_AL, OP_ADD, 0, 0, 0, 1), // R0 += R1
            encode_data_processing(COND_AL, OP_SUB, 1, 1, 1, (1 << 25) | 1), // SUBS R1, R1, #1
            encode_branch(COND_NE, false, -16), // if not zero, loop
            encode_halt(),
        ], 0);
        cpu.run(200);
        assert_eq!(cpu.read_register(0), 55); // 1+2+...+10 = 55
    }

    #[test]
    fn test_ldr_str() {
        let mut cpu = ARM1::new(4096);
        // MOV R1, a high address using rotated immediate
        // 1 rotated right by 2 = 0x40000000... but we need something in memory range.
        // Let's use a different approach: store at offset from current location.
        cpu.load_program_words(&[
            encode_mov_imm(COND_AL, 0, 42),  // R0 = 42
            // MOV R1, #256 (using rotated immediate: 1 rotated right by 24 = 1 << 8 = 256)
            encode_data_processing(COND_AL, OP_MOV, 0, 0, 1, (1 << 25) | (12 << 8) | 1),
            encode_str(COND_AL, 0, 1, 0, true),  // STR R0, [R1]
            encode_mov_imm(COND_AL, 0, 0),   // R0 = 0
            encode_ldr(COND_AL, 0, 1, 0, true),  // LDR R0, [R1]
            encode_halt(),
        ], 0);
        cpu.run(100);
        assert_eq!(cpu.read_register(0), 42);
    }

    #[test]
    fn test_stm_ldm() {
        let mut cpu = ARM1::new(4096);
        cpu.load_program_words(&[
            encode_mov_imm(COND_AL, 0, 10),
            encode_mov_imm(COND_AL, 1, 20),
            encode_mov_imm(COND_AL, 2, 30),
            encode_mov_imm(COND_AL, 3, 40),
            // Set R5 to a memory address (256 = 1 ROR 24)
            encode_data_processing(COND_AL, OP_MOV, 0, 0, 5, (1 << 25) | (12 << 8) | 1),
            encode_stm(COND_AL, 5, 0x000F, true, "IA"), // STM R5!, {R0-R3}
            encode_mov_imm(COND_AL, 0, 0),
            encode_mov_imm(COND_AL, 1, 0),
            encode_mov_imm(COND_AL, 2, 0),
            encode_mov_imm(COND_AL, 3, 0),
            // Reset R5 to 256
            encode_data_processing(COND_AL, OP_MOV, 0, 0, 5, (1 << 25) | (12 << 8) | 1),
            encode_ldm(COND_AL, 5, 0x000F, true, "IA"), // LDM R5!, {R0-R3}
            encode_halt(),
        ], 0);
        cpu.run(100);
        assert_eq!(cpu.read_register(0), 10);
        assert_eq!(cpu.read_register(1), 20);
        assert_eq!(cpu.read_register(2), 30);
        assert_eq!(cpu.read_register(3), 40);
    }

    #[test]
    fn test_branch_and_link() {
        let mut cpu = ARM1::new(4096);
        cpu.load_program_words(&[
            encode_mov_imm(COND_AL, 0, 7),          // addr 0
            encode_branch(COND_AL, true, 4),         // addr 4: BL +4 -> addr 16
            encode_halt(),                           // addr 8
            0,                                       // addr 12
            encode_alu_reg(COND_AL, OP_ADD, 0, 0, 0, 0), // addr 16: ADD R0, R0, R0
            // MOV PC, LR (return)
            encode_data_processing(COND_AL, OP_MOV, 1, 0, 15, 14),
        ], 0);
        cpu.run(20);
        assert_eq!(cpu.read_register(0), 14); // 7 + 7
    }

    #[test]
    fn test_barrel_shifter_in_instruction() {
        // ADD R1, R0, R0, LSL #2 (multiply by 5: R0 + R0*4)
        let add_with_shift = (COND_AL << 28) |
            (OP_ADD << 21) |
            (0 << 16) |   // Rn = R0
            (1 << 12) |   // Rd = R1
            (2 << 7) |    // shift amount = 2
            (SHIFT_LSL << 5) |
            0;             // Rm = R0

        let mut cpu = ARM1::new(4096);
        cpu.load_program_words(&[
            encode_mov_imm(COND_AL, 0, 7),
            add_with_shift,
            encode_halt(),
        ], 0);
        cpu.run(100);
        assert_eq!(cpu.read_register(1), 35); // 7 + 7*4 = 35
    }

    #[test]
    fn test_cmp_and_conditional_branch() {
        // Compare and branch: if R0 > R1, set R2=1 else R2=0
        let mut cpu = ARM1::new(4096);
        cpu.load_program_words(&[
            encode_mov_imm(COND_AL, 0, 10),   // R0 = 10
            encode_mov_imm(COND_AL, 1, 5),    // R1 = 5
            // CMP R0, R1
            encode_alu_reg(COND_AL, OP_CMP, 1, 0, 0, 1),
            encode_mov_imm(COND_GT, 2, 1),    // R2 = 1 if R0 > R1
            encode_mov_imm(COND_LE, 2, 0),    // R2 = 0 otherwise
            encode_halt(),
        ], 0);
        cpu.run(100);
        assert_eq!(cpu.read_register(2), 1);
    }

    #[test]
    fn test_display() {
        let cpu = ARM1::new(1024);
        let s = format!("{cpu}");
        assert!(s.contains("ARM1"));
        assert!(s.contains("SVC"));
    }

    #[test]
    fn test_trace_memory_tracking() {
        let mut cpu = ARM1::new(4096);
        cpu.load_program_words(&[
            encode_mov_imm(COND_AL, 0, 99),
            encode_data_processing(COND_AL, OP_MOV, 0, 0, 1, (1 << 25) | (12 << 8) | 1), // R1=256
            encode_str(COND_AL, 0, 1, 0, true),
            encode_halt(),
        ], 0);
        let traces = cpu.run(100);
        // The STR instruction should have a memory write
        let str_trace = &traces[2];
        assert!(!str_trace.memory_writes.is_empty());
        assert_eq!(str_trace.memory_writes[0].value, 99);
    }

    #[test]
    fn test_encode_helpers() {
        // Verify encoding round-trips through decoder
        let inst = encode_mov_imm(COND_AL, 5, 100);
        let d = decode(inst);
        assert_eq!(d.rd, 5);
        assert_eq!(d.imm8, 100);

        let inst = encode_branch(COND_NE, true, 24);
        let d = decode(inst);
        assert_eq!(d.cond, COND_NE);
        assert!(d.link);
        assert_eq!(d.branch_offset, 24);
    }

    #[test]
    fn test_byte_load_store() {
        let mut cpu = ARM1::new(4096);
        cpu.write_byte(0x100, 0xAB);
        assert_eq!(cpu.read_byte(0x100), 0xAB);
        // Word read should include the byte
        assert_eq!(cpu.read_word(0x100) & 0xFF, 0xAB);
    }

    #[test]
    fn test_swi_non_halt() {
        // A SWI that is not our halt pseudo-instruction should enter SVC mode
        // and vector to 0x08
        let mut cpu = ARM1::new(4096);
        // Put a halt at the SWI vector
        cpu.load_program_words(&[encode_halt()], 0x08);
        // SWI with a different comment
        let swi_inst = (COND_AL << 28) | 0x0F00_0000 | 0x42;
        cpu.load_program_words(&[swi_inst], 0);
        cpu.run(10);
        assert!(cpu.halted());
    }

    #[test]
    fn test_pc_wraps_26bit() {
        let mut cpu = ARM1::new(4096);
        cpu.set_pc(0xFFFF_FFFC);
        // PC should be masked to 26 bits
        assert_eq!(cpu.pc(), 0x03FF_FFFC);
    }
}
