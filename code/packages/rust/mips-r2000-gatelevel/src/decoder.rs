//! decoder.rs — MIPS R2000 instruction decoder using gate-level field extraction.
//!
//! # MIPS R2000 instruction formats
//!
//! Three fixed-width 32-bit formats:
//!
//! ```text
//! R-type (register, op=0):
//!   ┌────────┬─────┬─────┬─────┬───────┬────────┐
//!   │ op (6) │rs(5)│rt(5)│rd(5)│shamt(5)│funct(6)│
//!   └────────┴─────┴─────┴─────┴───────┴────────┘
//!   Bits: 31..26  25..21  20..16  15..11  10..6  5..0
//!
//! I-type (immediate):
//!   ┌────────┬─────┬─────┬──────────────────┐
//!   │ op (6) │rs(5)│rt(5)│    imm16 (16)    │
//!   └────────┴─────┴─────┴──────────────────┘
//!
//! J-type (jump):
//!   ┌────────┬───────────────────────────────┐
//!   │ op (6) │         target26 (26)         │
//!   └────────┴───────────────────────────────┘
//! ```
//!
//! # Gate-level field extraction
//!
//! Real hardware uses AND masks and shift registers (or wire re-ordering)
//! to right-justify each field.  We model this by extracting bit sub-slices
//! from the instruction word's LSB-first bit array, then converting back to
//! integers.
//!
//! # Sign extension of imm16
//!
//! For I-type instructions, imm16 is sign-extended to 32 bits.  In hardware,
//! bit 15 of the immediate is replicated into bits 16..31.  We implement this
//! by checking `bits[15]` and filling upper positions accordingly.
//!
//! The result is returned as an `i32` (may be negative) so that branch offset
//! arithmetic can use it directly.

use crate::bits::{bits_to_u32, int_to_bits32};

/// Decoded MIPS R2000 instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodedInstruction {
    /// Instruction format: 0=R, 1=I, 2=J.
    pub format: InstrFormat,
    /// 6-bit opcode (bits 31..26).
    pub op: u8,
    /// 5-bit source register (bits 25..21).
    pub rs: u8,
    /// 5-bit target register (bits 20..16).
    pub rt: u8,
    /// 5-bit destination register (bits 15..11), R-type only.
    pub rd: u8,
    /// 5-bit shift amount (bits 10..6), R-type only.
    pub shamt: u8,
    /// 6-bit function code (bits 5..0), R-type only.
    pub funct: u8,
    /// Sign-extended 16-bit immediate (bits 15..0), I-type only.
    pub imm16: i32,
    /// 26-bit jump target (bits 25..0), J-type only.
    pub target26: u32,
}

/// Instruction format discriminant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstrFormat {
    /// R-type (op=0): rs, rt, rd, shamt, funct.
    R,
    /// I-type: rs, rt, imm16.
    I,
    /// J-type (op=2 or 3): target26.
    J,
}

/// Decode a 32-bit MIPS instruction word into its constituent fields.
///
/// Uses gate-level bit extraction: the word is converted to an LSB-first
/// bit array, sub-slices are extracted for each field, then converted back
/// to integers.
///
/// ```
/// # use coding_adventures_mips_r2000_gatelevel::decoder::{decode_instruction, InstrFormat};
/// // ADD $t0, $t1, $t2: op=0, rs=9, rt=10, rd=8, shamt=0, funct=0x20
/// let d = decode_instruction(0x012A4020);
/// assert_eq!(d.format, InstrFormat::R);
/// assert_eq!(d.op, 0); assert_eq!(d.rs, 9); assert_eq!(d.rt, 10);
/// assert_eq!(d.rd, 8); assert_eq!(d.funct, 0x20);
///
/// // BEQ $t0, $t1, 3: op=4, rs=8, rt=9, imm16=3
/// let d = decode_instruction(0x11090003);
/// assert_eq!(d.format, InstrFormat::I);
/// assert_eq!(d.op, 4); assert_eq!(d.rs, 8); assert_eq!(d.rt, 9);
/// assert_eq!(d.imm16, 3);
/// ```
pub fn decode_instruction(word: u32) -> DecodedInstruction {
    // Convert to LSB-first bit array — models the 32 instruction wires.
    let bits = int_to_bits32(word);

    // Extract 6-bit opcode from bits[31:26] (indices 26..32 in LSB-first).
    let mut op_bits = [0u8; 32];
    op_bits[..6].copy_from_slice(&bits[26..32]);
    let op = bits_to_u32(op_bits) as u8;

    // Extract rs: bits[25:21] → indices 21..26.
    let mut rs_bits = [0u8; 32];
    rs_bits[..5].copy_from_slice(&bits[21..26]);
    let rs = bits_to_u32(rs_bits) as u8;

    // Extract rt: bits[20:16] → indices 16..21.
    let mut rt_bits = [0u8; 32];
    rt_bits[..5].copy_from_slice(&bits[16..21]);
    let rt = bits_to_u32(rt_bits) as u8;

    match op {
        0 => {
            // R-type
            let mut rd_bits = [0u8; 32];
            rd_bits[..5].copy_from_slice(&bits[11..16]);
            let rd = bits_to_u32(rd_bits) as u8;

            let mut shamt_bits = [0u8; 32];
            shamt_bits[..5].copy_from_slice(&bits[6..11]);
            let shamt = bits_to_u32(shamt_bits) as u8;

            let mut funct_bits = [0u8; 32];
            funct_bits[..6].copy_from_slice(&bits[0..6]);
            let funct = bits_to_u32(funct_bits) as u8;

            DecodedInstruction {
                format: InstrFormat::R,
                op,
                rs,
                rt,
                rd,
                shamt,
                funct,
                imm16: 0,
                target26: 0,
            }
        }
        2 | 3 => {
            // J-type: bits[25:0] → indices 0..26.
            let mut target_bits = [0u8; 32];
            target_bits[..26].copy_from_slice(&bits[0..26]);
            let target26 = bits_to_u32(target_bits);

            DecodedInstruction {
                format: InstrFormat::J,
                op,
                rs,
                rt,
                rd: 0,
                shamt: 0,
                funct: 0,
                imm16: 0,
                target26,
            }
        }
        _ => {
            // I-type: sign-extend imm16 from bits[15:0] → indices 0..16.
            let imm16 = sign_extend_imm16(&bits);
            DecodedInstruction {
                format: InstrFormat::I,
                op,
                rs,
                rt,
                rd: 0,
                shamt: 0,
                funct: 0,
                imm16,
                target26: 0,
            }
        }
    }
}

/// Extract and sign-extend a 16-bit immediate from an instruction's bit array.
///
/// The 16-bit immediate occupies `bits[15:0]` (indices 0..16 in LSB-first).
/// Sign extension copies bit 15 into bits 16..31.
///
/// In hardware, the sign extension circuit is 16 wires from bit 15 fanned
/// out to positions 16–31.  No gates needed — just wire routing.
fn sign_extend_imm16(bits: &[u8; 32]) -> i32 {
    let sign_bit = bits[15]; // bit 15 is the sign of the 16-bit immediate
    let mut extended = [0u8; 32];
    extended[..16].copy_from_slice(&bits[0..16]);
    extended[16..].fill(sign_bit);
    // Convert to signed: if sign_bit=1, the unsigned value >= 0x8000_0000
    let unsigned_val = bits_to_u32(extended);
    unsigned_val as i32
}
