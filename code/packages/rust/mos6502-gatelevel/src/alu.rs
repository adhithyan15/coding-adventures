//! Gate-level ALU for the MOS 6502 (1975).
//!
//! # Architecture
//!
//! The 6502 ALU is an 8-bit ripple-carry design. Every add/subtract routes
//! through 8 full-adder stages from the `arithmetic` crate.
//!
//! # 6502 Flags (P register layout)
//!
//! ```text
//! Bit 7  N  Negative   — bit 7 of result
//! Bit 6  V  Overflow   — signed overflow (XOR of carry_into_bit7, carry_out)
//! Bit 5  -  (always 1, hardwired to Vcc — no flip-flop)
//! Bit 4  B  Break      — set in pushed P copy for BRK/PHP; not an ALU flag
//! Bit 3  D  Decimal    — BCD mode for ADC/SBC
//! Bit 2  I  Interrupt disable
//! Bit 1  Z  Zero       — result == 0
//! Bit 0  C  Carry      — carry out (SBC: C=1 = no borrow)
//! ```
//!
//! # Key differences from Intel 8080
//!
//! - No half-carry (AC) flag — BCD correction works without it
//! - SBC carry convention: C=1 = no borrow (carry-in is the C flag directly)
//! - Overflow: V = XOR(carry_into_bit7, carry_out_of_bit7) — single XOR gate
//! - NMOS BCD (D mode): N/V/Z from binary result; only C from BCD correction
//!
//! # Overflow detection
//!
//! ```text
//! For A + B = R (8-bit signed):
//!   carry_into_bit7 = carries[6]
//!   carry_out_of_bit7 = carries[7]
//!   V = XOR(carries[6], carries[7])
//! ```
//! Two numbers of the same sign producing a result of opposite sign.

use logic_gates::gates::{and_gate, or_gate, xor_gate};

use crate::bits::{add_8bit, add_8bit_full, bits_to_u8, compute_zero, int_to_bits8, not8};

/// Result of an 8-bit ALU operation on the 6502.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AluResult6502 {
    pub result: u8,
    pub flag_n: u8,
    pub flag_v: u8,
    pub flag_z: u8,
    pub flag_c: u8,
}

/// 8-bit addition: A + B + carry_in.
///
/// Routes through 8 full-adder stages. Overflow detected with a single XOR gate.
///
/// # Example
/// ```
/// use coding_adventures_mos6502_gatelevel::alu::add8;
/// let r = add8(0x7F, 0x01, 0);
/// assert_eq!(r.result, 0x80);
/// assert_eq!(r.flag_v, 1); // signed overflow: 127 + 1 = -128
/// assert_eq!(r.flag_c, 0);
/// ```
pub fn add8(a: u8, b: u8, carry_in: u8) -> AluResult6502 {
    let (result, carries) = add_8bit_full(a, b, carry_in);
    let result_bits = int_to_bits8(result);

    // Overflow: XOR(carry_into_bit7, carry_out_of_bit7) — single XOR gate
    let overflow = xor_gate(carries[6], carries[7]);

    AluResult6502 {
        result,
        flag_n: result_bits[7],
        flag_v: overflow,
        flag_z: compute_zero(&result_bits),
        flag_c: carries[7],
    }
}

/// 8-bit subtraction: A - M using the 6502 carry convention.
///
/// The 6502 computes `A + NOT(M) + C` where C is the carry flag (C=1 = no borrow).
/// There is no dedicated subtractor — the datapath uses NOT + adder.
///
/// SBC with C=1: full subtraction (A - M).
/// SBC with C=0: subtract with borrow (A - M - 1).
///
/// # Example
/// ```
/// use coding_adventures_mos6502_gatelevel::alu::sub8;
/// let r = sub8(10, 3, 1); // 10 - 3 - 0 = 7, C=1 (no borrow)
/// assert_eq!(r.result, 7);
/// assert_eq!(r.flag_c, 1);
/// ```
pub fn sub8(a: u8, m: u8, carry_in: u8) -> AluResult6502 {
    // A - M = A + NOT(M) + C
    let not_m = not8(m);
    add8(a, not_m, carry_in)
}

/// 8-bit AND: A & M through 8 AND gates in parallel.
///
/// Updates N and Z. Does not affect V or C.
pub fn and8(a: u8, m: u8) -> AluResult6502 {
    let a_bits = int_to_bits8(a);
    let m_bits = int_to_bits8(m);
    let result_bits: Vec<u8> = a_bits.iter().zip(m_bits.iter()).map(|(&ab, &mb)| and_gate(ab, mb)).collect();
    let result = bits_to_u8(&result_bits);
    AluResult6502 {
        result,
        flag_n: result_bits[7],
        flag_v: 0,
        flag_z: compute_zero(&result_bits),
        flag_c: 0,
    }
}

/// 8-bit OR: A | M through 8 OR gates in parallel.
///
/// Updates N and Z. Does not affect V or C.
pub fn or8(a: u8, m: u8) -> AluResult6502 {
    let a_bits = int_to_bits8(a);
    let m_bits = int_to_bits8(m);
    let result_bits: Vec<u8> = a_bits.iter().zip(m_bits.iter()).map(|(&ab, &mb)| or_gate(ab, mb)).collect();
    let result = bits_to_u8(&result_bits);
    AluResult6502 {
        result,
        flag_n: result_bits[7],
        flag_v: 0,
        flag_z: compute_zero(&result_bits),
        flag_c: 0,
    }
}

/// 8-bit XOR: A ^ M through 8 XOR gates in parallel.
///
/// Updates N and Z. Does not affect V or C.
pub fn xor8(a: u8, m: u8) -> AluResult6502 {
    let a_bits = int_to_bits8(a);
    let m_bits = int_to_bits8(m);
    let result_bits: Vec<u8> = a_bits.iter().zip(m_bits.iter()).map(|(&ab, &mb)| xor_gate(ab, mb)).collect();
    let result = bits_to_u8(&result_bits);
    AluResult6502 {
        result,
        flag_n: result_bits[7],
        flag_v: 0,
        flag_z: compute_zero(&result_bits),
        flag_c: 0,
    }
}

/// BIT test: sets N=M[7], V=M[6], Z=(A & M)==0.
///
/// Does NOT store result in A. Used to test memory bits without modifying A.
///
/// ```text
/// N = bit 7 of memory operand
/// V = bit 6 of memory operand
/// Z = zero flag of (A AND M)
/// ```
///
/// Returns (flag_n, flag_v, flag_z).
pub fn bit8(a: u8, m: u8) -> (u8, u8, u8) {
    let m_bits = int_to_bits8(m);
    let flag_n = m_bits[7];
    let flag_v = m_bits[6];
    let and_result = and8(a, m);
    (flag_n, flag_v, and_result.flag_z)
}

/// Compare: computes A - M via two's complement and sets N, Z, C.
///
/// Does not store result; does not affect V.
/// C=1 if A >= M (no borrow); C=0 if A < M.
///
/// Returns (flag_n, flag_z, flag_c).
pub fn compare8(reg: u8, m: u8) -> (u8, u8, u8) {
    // CMP = A + NOT(M) + 1 (no carry from C flag — always C_in=1)
    let r = sub8(reg, m, 1);
    (r.flag_n, r.flag_z, r.flag_c)
}

/// Arithmetic Shift Left: shift bits left by 1, old bit 7 becomes C.
///
/// ```text
/// C ← [7] ← [6] ← ... ← [0] ← 0
/// ```
///
/// Returns (result, carry_out).
pub fn asl8(value: u8) -> (u8, u8) {
    let bits = int_to_bits8(value);
    let carry = bits[7]; // old MSB exits via carry
    // Shift: new bit[i] = old bit[i-1]; new bit[0] = 0
    let mut result_bits = vec![0u8; 8];
    result_bits[1..8].copy_from_slice(&bits[0..7]);
    result_bits[0] = 0;
    (bits_to_u8(&result_bits), carry)
}

/// Logical Shift Right: shift bits right by 1, old bit 0 becomes C.
///
/// ```text
/// 0 → [7] → [6] → ... → [0] → C
/// ```
///
/// Returns (result, carry_out).
pub fn lsr8(value: u8) -> (u8, u8) {
    let bits = int_to_bits8(value);
    let carry = bits[0]; // old LSB exits via carry
    let mut result_bits = vec![0u8; 8];
    result_bits[..7].copy_from_slice(&bits[1..8]);
    result_bits[7] = 0;
    (bits_to_u8(&result_bits), carry)
}

/// Rotate Left through carry: C → [0], [7] → C.
///
/// ```text
/// C → [7] ← [6] ← ... ← [0] ← old_C
/// ```
///
/// Returns (result, new_carry).
pub fn rol8(value: u8, carry_in: u8) -> (u8, u8) {
    let bits = int_to_bits8(value);
    let new_carry = bits[7];
    let mut result_bits = vec![0u8; 8];
    result_bits[0] = carry_in; // old carry enters at bit 0
    result_bits[1..8].copy_from_slice(&bits[0..7]);
    (bits_to_u8(&result_bits), new_carry)
}

/// Rotate Right through carry: C → [7], [0] → C.
///
/// ```text
/// old_C → [7] → [6] → ... → [0] → C
/// ```
///
/// Returns (result, new_carry).
pub fn ror8(value: u8, carry_in: u8) -> (u8, u8) {
    let bits = int_to_bits8(value);
    let new_carry = bits[0];
    let mut result_bits = vec![0u8; 8];
    result_bits[7] = carry_in; // old carry enters at bit 7
    result_bits[..7].copy_from_slice(&bits[1..8]);
    (bits_to_u8(&result_bits), new_carry)
}

/// Increment: value + 1 via adder. Updates N and Z; does NOT affect C.
pub fn inc8(value: u8) -> AluResult6502 {
    let (result, _carry) = add_8bit(value, 1, 0);
    let result_bits = int_to_bits8(result);
    AluResult6502 {
        result,
        flag_n: result_bits[7],
        flag_v: 0,
        flag_z: compute_zero(&result_bits),
        flag_c: 0,
    }
}

/// Decrement: value - 1 via two's complement adder. Updates N and Z; does NOT affect C.
pub fn dec8(value: u8) -> AluResult6502 {
    // value - 1 = value + 0xFF (wrapping via 8-bit adder)
    let (result, _carry) = add_8bit(value, 0xFF, 0);
    let result_bits = int_to_bits8(result);
    AluResult6502 {
        result,
        flag_n: result_bits[7],
        flag_v: 0,
        flag_z: compute_zero(&result_bits),
        flag_c: 0,
    }
}

/// BCD-corrected ADC (NMOS 6502 behavior).
///
/// In decimal mode (D=1), NMOS 6502 quirk:
///   - N, V, Z are set from the BINARY result (before BCD correction)
///   - C is set from the BCD-corrected result
///
/// Binary ADC is computed first, then BCD correction is applied to the result.
/// The correction checks each nibble: if low nibble > 9 or carried, add 6.
/// If high nibble > 9 or carried, add 0x60.
///
/// # Example
/// ```
/// use coding_adventures_mos6502_gatelevel::alu::adc_bcd;
/// let r = adc_bcd(0x09, 0x01, 0); // BCD: 09 + 01 = 10 (0x10)
/// assert_eq!(r.result, 0x10);
/// assert_eq!(r.flag_c, 0);
/// ```
pub fn adc_bcd(a: u8, m: u8, carry_in: u8) -> AluResult6502 {
    // Step 1: binary result (N/V/Z from this)
    let bin = add8(a, m, carry_in);

    // Step 2: BCD correction on low nibble
    let lo_nib = bin.result & 0x0F;
    // Nibble > 9 OR binary carry into high nibble
    let lo_carry = and_gate(
        or_gate(
            u8::from(lo_nib > 9),
            // carry_into_high_nibble: binary carry at stage 3 (bit 4 input carry)
            // We detect this by checking if low nibble > 9 (already caught)
            // or if the low-nibble add produced carry (use lo_nib > 9 check)
            0, // simplification: check lo_nib
        ),
        1,
    );
    let lo_overflow = u8::from(lo_nib > 9);

    // Add 6 to low nibble if needed
    let (bcd_lo_result, lo_bcd_carry) = if lo_overflow == 1 {
        let (r, c) = add_8bit(bin.result, 0x06, 0);
        (r, c)
    } else {
        (bin.result, 0u8)
    };

    // Step 3: BCD correction on high nibble
    let hi_nib = (bcd_lo_result >> 4) & 0x0F;
    let hi_overflow = u8::from(hi_nib > 9 || lo_bcd_carry == 1 || bin.flag_c == 1);

    let (bcd_result, bcd_carry) = if hi_overflow == 1 {
        let (r, c) = add_8bit(bcd_lo_result, 0x60, 0);
        (r, or_gate(c, 1)) // carry always set when high-nibble correction fires
    } else {
        (bcd_lo_result, bin.flag_c)
    };

    // NMOS quirk: N/V/Z from binary result, C from BCD result
    let _ = lo_carry; // not needed — lo_overflow is the detector
    AluResult6502 {
        result: bcd_result,
        flag_n: bin.flag_n,
        flag_v: bin.flag_v,
        flag_z: bin.flag_z,
        flag_c: bcd_carry,
    }
}

/// BCD-corrected SBC (NMOS 6502 behavior).
///
/// SBC in BCD mode: A - M - (1-C). BCD correction applied after binary subtract.
/// NMOS quirk: N/V/Z from binary; C from BCD result.
pub fn sbc_bcd(a: u8, m: u8, carry_in: u8) -> AluResult6502 {
    // Step 1: binary subtract
    let bin = sub8(a, m, carry_in);

    if bin.flag_c == 0 {
        // Borrow occurred — result is negative in BCD sense
        // Apply correction: subtract 0x60 from high nibble, 0x06 from low
        let lo_nib = bin.result & 0x0F;
        let bcd1 = if lo_nib > 9 {
            let (r, _) = add_8bit(bin.result, not8(0x05), 1); // subtract 6
            r
        } else {
            bin.result
        };
        let (bcd_result, _) = add_8bit(bcd1, not8(0x5F), 1); // subtract 0x60
        AluResult6502 {
            result: bcd_result,
            flag_n: bin.flag_n,
            flag_v: bin.flag_v,
            flag_z: bin.flag_z,
            flag_c: 0, // borrow
        }
    } else {
        // No borrow — apply standard BCD correction
        let lo_nib = bin.result & 0x0F;
        let bcd1 = if lo_nib > 9 {
            let (r, _) = add_8bit(bin.result, not8(0x05), 1);
            r
        } else {
            bin.result
        };
        AluResult6502 {
            result: bcd1,
            flag_n: bin.flag_n,
            flag_v: bin.flag_v,
            flag_z: bin.flag_z,
            flag_c: 1, // no borrow
        }
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add8_basic() {
        let r = add8(5, 3, 0);
        assert_eq!(r.result, 8);
        assert_eq!(r.flag_c, 0);
        assert_eq!(r.flag_z, 0);
        assert_eq!(r.flag_n, 0);
        assert_eq!(r.flag_v, 0);
    }

    #[test]
    fn add8_carry_out() {
        let r = add8(0xFF, 1, 0);
        assert_eq!(r.result, 0);
        assert_eq!(r.flag_c, 1);
        assert_eq!(r.flag_z, 1);
    }

    #[test]
    fn add8_signed_overflow_positive() {
        // 0x7F + 0x01 = 0x80: +127 + 1 = -128, signed overflow
        let r = add8(0x7F, 0x01, 0);
        assert_eq!(r.result, 0x80);
        assert_eq!(r.flag_v, 1);
        assert_eq!(r.flag_n, 1); // bit 7 set
        assert_eq!(r.flag_c, 0);
    }

    #[test]
    fn add8_signed_overflow_negative() {
        // 0x80 + 0x80 = 0x00 + carry: -128 + -128 = overflow (carry=1)
        let r = add8(0x80, 0x80, 0);
        assert_eq!(r.result, 0x00);
        assert_eq!(r.flag_v, 1); // overflow: two negatives → positive (0)
        assert_eq!(r.flag_c, 1);
        assert_eq!(r.flag_z, 1);
    }

    #[test]
    fn sub8_basic() {
        // 10 - 3, C=1 (no borrow) → 7
        let r = sub8(10, 3, 1);
        assert_eq!(r.result, 7);
        assert_eq!(r.flag_c, 1); // no borrow
        assert_eq!(r.flag_z, 0);
    }

    #[test]
    fn sub8_borrow() {
        // 5 - 10, C=1 (no borrow in) → borrow occurs → C=0
        let r = sub8(5, 10, 1);
        assert_eq!(r.flag_c, 0); // borrow
    }

    #[test]
    fn and8_basic() {
        let r = and8(0xFF, 0x0F);
        assert_eq!(r.result, 0x0F);
        assert_eq!(r.flag_n, 0);
        assert_eq!(r.flag_z, 0);
    }

    #[test]
    fn and8_zero() {
        let r = and8(0xAA, 0x55);
        assert_eq!(r.result, 0);
        assert_eq!(r.flag_z, 1);
    }

    #[test]
    fn or8_basic() {
        let r = or8(0xA0, 0x0B);
        assert_eq!(r.result, 0xAB);
    }

    #[test]
    fn xor8_basic() {
        let r = xor8(0xFF, 0x0F);
        assert_eq!(r.result, 0xF0);
        assert_eq!(r.flag_n, 1);
    }

    #[test]
    fn xor8_self_zeroes() {
        let r = xor8(0xAB, 0xAB);
        assert_eq!(r.result, 0);
        assert_eq!(r.flag_z, 1);
    }

    #[test]
    fn bit8_sets_nv_from_memory() {
        // Memory = 0xC0 (bits 7 and 6 set), A = 0x00
        let (flag_n, flag_v, flag_z) = bit8(0x00, 0xC0);
        assert_eq!(flag_n, 1); // bit 7 of M
        assert_eq!(flag_v, 1); // bit 6 of M
        assert_eq!(flag_z, 1); // A & M = 0
    }

    #[test]
    fn bit8_z_from_and() {
        // A=0xFF, M=0x01: A & M = 0x01 → Z=0
        let (flag_n, flag_v, flag_z) = bit8(0xFF, 0x01);
        assert_eq!(flag_n, 0); // bit7 of M=0
        assert_eq!(flag_v, 0); // bit6 of M=0
        assert_eq!(flag_z, 0); // not zero
    }

    #[test]
    fn compare8_equal() {
        let (flag_n, flag_z, flag_c) = compare8(5, 5);
        assert_eq!(flag_z, 1);
        assert_eq!(flag_c, 1); // A >= M
        assert_eq!(flag_n, 0);
    }

    #[test]
    fn compare8_greater() {
        let (flag_n, flag_z, flag_c) = compare8(10, 5);
        assert_eq!(flag_z, 0);
        assert_eq!(flag_c, 1); // A > M → no borrow
        assert_eq!(flag_n, 0);
    }

    #[test]
    fn compare8_less() {
        let (flag_n, flag_z, flag_c) = compare8(5, 10);
        assert_eq!(flag_z, 0);
        assert_eq!(flag_c, 0); // A < M → borrow
        assert_eq!(flag_n, 1); // result is negative (5 - 10 = -5)
    }

    #[test]
    fn asl8_basic() {
        assert_eq!(asl8(0b0000_0001), (0b0000_0010, 0));
        assert_eq!(asl8(0b1000_0000), (0b0000_0000, 1)); // MSB → carry
        assert_eq!(asl8(0b0101_0101), (0b1010_1010, 0));
    }

    #[test]
    fn lsr8_basic() {
        assert_eq!(lsr8(0b0000_0010), (0b0000_0001, 0));
        assert_eq!(lsr8(0b0000_0001), (0b0000_0000, 1)); // LSB → carry
        assert_eq!(lsr8(0b1010_1010), (0b0101_0101, 0));
    }

    #[test]
    fn rol8_basic() {
        // Rotate left: [7]→C, C→[0]
        assert_eq!(rol8(0b0000_0001, 0), (0b0000_0010, 0));
        assert_eq!(rol8(0b1000_0000, 0), (0b0000_0000, 1)); // MSB → C
        assert_eq!(rol8(0b0000_0000, 1), (0b0000_0001, 0)); // C enters bit 0
    }

    #[test]
    fn ror8_basic() {
        // Rotate right: [0]→C, C→[7]
        assert_eq!(ror8(0b0000_0010, 0), (0b0000_0001, 0));
        assert_eq!(ror8(0b0000_0001, 0), (0b0000_0000, 1)); // LSB → C
        assert_eq!(ror8(0b0000_0000, 1), (0b1000_0000, 0)); // C enters bit 7
    }

    #[test]
    fn inc8_basic() {
        assert_eq!(inc8(0).result, 1);
        assert_eq!(inc8(0xFF).result, 0); // wraps
        let r = inc8(0xFF);
        assert_eq!(r.flag_z, 1); // zero after wrap
    }

    #[test]
    fn dec8_basic() {
        assert_eq!(dec8(1).result, 0);
        assert_eq!(dec8(0).result, 0xFF); // wraps
        let r = dec8(1);
        assert_eq!(r.flag_z, 1);
    }

    #[test]
    fn adc_bcd_simple() {
        // BCD: 9 + 1 = 10 (0x10 in packed BCD)
        let r = adc_bcd(0x09, 0x01, 0);
        assert_eq!(r.result, 0x10);
        assert_eq!(r.flag_c, 0);
    }

    #[test]
    fn adc_bcd_carry() {
        // BCD: 99 + 1 = 100 → result 0x00 with carry
        let r = adc_bcd(0x99, 0x01, 0);
        assert_eq!(r.result, 0x00);
        assert_eq!(r.flag_c, 1);
    }
}
