//! Instruction decoder -- combinational logic that maps opcodes to control signals.
//!
//! # How instruction decoding works in hardware
//!
//! The decoder takes an 8-bit instruction byte and produces control signals
//! that tell the rest of the CPU what to do. In the real 4004, this was a
//! combinational logic network -- a forest of AND, OR, and NOT gates that
//! pattern-match the opcode bits.
//!
//! For example, to detect LDM (0xD_):
//!
//! ```text
//! is_ldm = AND(bit7, bit6, NOT(bit5), bit4)  => bits 7654 = 1101
//! ```
//!
//! The decoder doesn't use sequential logic -- it's purely combinational.
//! Given the same input bits, it always produces the same output signals.
//!
//! # Control signals
//!
//! The decoder outputs tell the control unit what to do:
//! - `is_jump`: This is a jump instruction
//! - `is_call`: This is JMS (push return address)
//! - `is_return`: This is BBL (pop and return)
//! - `is_two_byte`: Instruction is 2 bytes
//! - And many more family-specific flags

use logic_gates::gates::{and_gate, not_gate, or_gate};

/// Control signals produced by the instruction decoder.
///
/// Every field represents a wire carrying a 0 or 1 signal, or a
/// multi-bit value extracted from the instruction.
#[derive(Debug, Clone)]
pub struct DecodedInstruction {
    /// Original first instruction byte.
    pub raw: u8,
    /// Optional second byte for 2-byte instructions.
    pub raw2: Option<u8>,

    /// Upper nibble bits [7:4].
    pub upper: u8,
    /// Lower nibble bits [3:0].
    pub lower: u8,

    // --- Instruction family detection (from gate logic) ---
    pub is_nop: u8,
    pub is_hlt: u8,
    pub is_ldm: u8,
    pub is_ld: u8,
    pub is_xch: u8,
    pub is_inc: u8,
    pub is_add: u8,
    pub is_sub: u8,
    pub is_jun: u8,
    pub is_jcn: u8,
    pub is_isz: u8,
    pub is_jms: u8,
    pub is_bbl: u8,
    pub is_fim: u8,
    pub is_src: u8,
    pub is_fin: u8,
    pub is_jin: u8,
    /// 0xE_ range: I/O operations.
    pub is_io: u8,
    /// 0xF_ range: accumulator operations.
    pub is_accum: u8,

    /// Two-byte flag.
    pub is_two_byte: u8,

    // --- Operand extraction ---
    /// Lower nibble (register index).
    pub reg_index: u8,
    /// Lower nibble >> 1 (pair index).
    pub pair_index: u8,
    /// Lower nibble (immediate value).
    pub immediate: u8,
    /// Lower nibble (JCN condition code).
    pub condition: u8,

    // --- For 2-byte instructions ---
    /// 12-bit address (JUN/JMS).
    pub addr12: u16,
    /// 8-bit address/data (JCN/ISZ/FIM).
    pub addr8: u8,
}

/// Decode an instruction byte into control signals using logic gates.
///
/// In real hardware, this is a combinational circuit -- no clock needed.
/// The input bits propagate through AND/OR/NOT gate trees to produce
/// the output control signals.
pub fn decode(raw: u8, raw2: Option<u8>) -> DecodedInstruction {
    // Extract individual bits
    let b7 = (raw >> 7) & 1;
    let b6 = (raw >> 6) & 1;
    let b5 = (raw >> 5) & 1;
    let b4 = (raw >> 4) & 1;
    let b3 = (raw >> 3) & 1;
    let b2 = (raw >> 2) & 1;
    let b1 = (raw >> 1) & 1;
    let b0 = raw & 1;

    let upper = (raw >> 4) & 0xF;
    let lower = raw & 0xF;

    // --- Instruction family detection ---
    // Each family is detected by AND-ing the upper nibble bits.
    // Using NOT for inverted bits.

    // NOP = 0x00: all bits zero
    let is_nop = and_gate(
        and_gate(
            and_gate(not_gate(b7), not_gate(b6)),
            and_gate(
                and_gate(not_gate(b5), not_gate(b4)),
                and_gate(not_gate(b3), not_gate(b2)),
            ),
        ),
        and_gate(not_gate(b1), not_gate(b0)),
    );

    // HLT = 0x01: only b0 is 1
    let is_hlt = and_gate(
        and_gate(
            and_gate(not_gate(b7), not_gate(b6)),
            and_gate(
                and_gate(not_gate(b5), not_gate(b4)),
                and_gate(not_gate(b3), not_gate(b2)),
            ),
        ),
        and_gate(not_gate(b1), b0),
    );

    // 0x1_ = 0001 : JCN
    let is_jcn_family =
        and_gate(and_gate(not_gate(b7), not_gate(b6)), and_gate(not_gate(b5), b4));

    // 0x2_ = 0010 : FIM (even b0) or SRC (odd b0)
    let is_2x = and_gate(and_gate(not_gate(b7), not_gate(b6)), and_gate(b5, not_gate(b4)));
    let is_fim = and_gate(is_2x, not_gate(b0));
    let is_src = and_gate(is_2x, b0);

    // 0x3_ = 0011 : FIN (even b0) or JIN (odd b0)
    let is_3x = and_gate(and_gate(not_gate(b7), not_gate(b6)), and_gate(b5, b4));
    let is_fin = and_gate(is_3x, not_gate(b0));
    let is_jin = and_gate(is_3x, b0);

    // 0x4_ = 0100 : JUN
    let is_jun_family =
        and_gate(and_gate(not_gate(b7), b6), and_gate(not_gate(b5), not_gate(b4)));

    // 0x5_ = 0101 : JMS
    let is_jms_family =
        and_gate(and_gate(not_gate(b7), b6), and_gate(not_gate(b5), b4));

    // 0x6_ = 0110 : INC
    let is_inc_family = and_gate(and_gate(not_gate(b7), b6), and_gate(b5, not_gate(b4)));

    // 0x7_ = 0111 : ISZ
    let is_isz_family = and_gate(and_gate(not_gate(b7), b6), and_gate(b5, b4));

    // 0x8_ = 1000 : ADD
    let is_add_family =
        and_gate(and_gate(b7, not_gate(b6)), and_gate(not_gate(b5), not_gate(b4)));

    // 0x9_ = 1001 : SUB
    let is_sub_family =
        and_gate(and_gate(b7, not_gate(b6)), and_gate(not_gate(b5), b4));

    // 0xA_ = 1010 : LD
    let is_ld_family = and_gate(and_gate(b7, not_gate(b6)), and_gate(b5, not_gate(b4)));

    // 0xB_ = 1011 : XCH
    let is_xch_family = and_gate(and_gate(b7, not_gate(b6)), and_gate(b5, b4));

    // 0xC_ = 1100 : BBL
    let is_bbl_family =
        and_gate(and_gate(b7, b6), and_gate(not_gate(b5), not_gate(b4)));

    // 0xD_ = 1101 : LDM
    let is_ldm_family = and_gate(and_gate(b7, b6), and_gate(not_gate(b5), b4));

    // 0xE_ = 1110 : I/O operations
    let is_io_family = and_gate(and_gate(b7, b6), and_gate(b5, not_gate(b4)));

    // 0xF_ = 1111 : accumulator operations
    let is_accum_family = and_gate(and_gate(b7, b6), and_gate(b5, b4));

    // Two-byte detection
    let is_two_byte = or_gate(
        or_gate(is_jcn_family, is_jun_family),
        or_gate(
            or_gate(is_jms_family, is_isz_family),
            is_fim,
        ),
    );

    // Operand extraction
    let reg_index = lower;
    let pair_index = lower >> 1;
    let immediate = lower;
    let condition = lower;

    // 12-bit address for JUN/JMS
    let second = raw2.unwrap_or(0);
    let addr12 = ((lower as u16) << 8) | (second as u16);
    let addr8 = second;

    DecodedInstruction {
        raw,
        raw2,
        upper,
        lower,
        is_nop,
        is_hlt,
        is_ldm: is_ldm_family,
        is_ld: is_ld_family,
        is_xch: is_xch_family,
        is_inc: is_inc_family,
        is_add: is_add_family,
        is_sub: is_sub_family,
        is_jun: is_jun_family,
        is_jcn: is_jcn_family,
        is_isz: is_isz_family,
        is_jms: is_jms_family,
        is_bbl: is_bbl_family,
        is_fim,
        is_src,
        is_fin,
        is_jin,
        is_io: is_io_family,
        is_accum: is_accum_family,
        is_two_byte,
        reg_index,
        pair_index,
        immediate,
        condition,
        addr12,
        addr8,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_nop() {
        let d = decode(0x00, None);
        assert_eq!(d.is_nop, 1);
        assert_eq!(d.is_hlt, 0);
    }

    #[test]
    fn test_decode_hlt() {
        let d = decode(0x01, None);
        assert_eq!(d.is_hlt, 1);
        assert_eq!(d.is_nop, 0);
    }

    #[test]
    fn test_decode_ldm() {
        let d = decode(0xD5, None);
        assert_eq!(d.is_ldm, 1);
        assert_eq!(d.immediate, 5);
    }

    #[test]
    fn test_decode_ld() {
        let d = decode(0xA3, None);
        assert_eq!(d.is_ld, 1);
        assert_eq!(d.reg_index, 3);
    }

    #[test]
    fn test_decode_add() {
        let d = decode(0x87, None);
        assert_eq!(d.is_add, 1);
        assert_eq!(d.reg_index, 7);
    }

    #[test]
    fn test_decode_jun() {
        let d = decode(0x41, Some(0x23));
        assert_eq!(d.is_jun, 1);
        assert_eq!(d.is_two_byte, 1);
        assert_eq!(d.addr12, 0x123);
    }

    #[test]
    fn test_decode_jms() {
        let d = decode(0x52, Some(0x00));
        assert_eq!(d.is_jms, 1);
        assert_eq!(d.is_two_byte, 1);
        assert_eq!(d.addr12, 0x200);
    }

    #[test]
    fn test_decode_bbl() {
        let d = decode(0xC3, None);
        assert_eq!(d.is_bbl, 1);
        assert_eq!(d.immediate, 3);
    }

    #[test]
    fn test_decode_fim() {
        let d = decode(0x20, Some(0xAB));
        assert_eq!(d.is_fim, 1);
        assert_eq!(d.is_two_byte, 1);
        assert_eq!(d.pair_index, 0);
        assert_eq!(d.addr8, 0xAB);
    }

    #[test]
    fn test_decode_src() {
        let d = decode(0x21, None);
        assert_eq!(d.is_src, 1);
        assert_eq!(d.is_two_byte, 0);
    }

    #[test]
    fn test_decode_io() {
        let d = decode(0xE0, None);
        assert_eq!(d.is_io, 1);
    }

    #[test]
    fn test_decode_accum() {
        let d = decode(0xF2, None);
        assert_eq!(d.is_accum, 1);
    }
}
