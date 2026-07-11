//! Gate-level ALU for the Intel 8051.
//!
//! Every arithmetic and logical operation is implemented by routing bit
//! vectors through individual `and_gate`, `or_gate`, `xor_gate`, `not_gate`
//! calls and `full_adder` chains from the `logic-gates` and `arithmetic`
//! crates.
//!
//! # Operations
//!
//! | Function | Opcode class | Flags updated |
//! |----------|--------------|---------------|
//! | add8     | ADD, ADDC    | CY, AC, OV, P |
//! | subb8    | SUBB         | CY, AC, OV, P |
//! | anl8     | ANL A,…      | P only        |
//! | orl8     | ORL A,…      | P only        |
//! | xrl8     | XRL A,…      | P only        |
//! | inc8     | INC          | P only (no CY/AC/OV) |
//! | dec8     | DEC          | P only (no CY/AC/OV) |
//! | rl8      | RL A         | CY (exiting MSB), P |
//! | rr8      | RR A         | CY (exiting LSB), P |
//! | rlc8     | RLC A        | CY, P |
//! | rrc8     | RRC A        | CY, P |
//! | swap8    | SWAP A       | none |
//! | da8      | DA A         | CY |
//! | mul8     | MUL AB       | CY (always 0), OV |
//! | div8     | DIV AB       | CY (always 0), OV |
//!
//! # SUBB model
//!
//! `SUBB A, B` computes `A − B − borrow_in` using two's complement:
//! ```text
//! A + NOT(B) + NOT(borrow_in)
//! ```
//! - `CY = NOT(carry_out)` — CY=1 means a borrow occurred (A < B + borrow)
//! - `AC = NOT(carries[3])` — AC=1 means lower nibble borrowed from upper
//! - `OV = XOR(carries[6], carries[7])` — signed overflow

use crate::bits::{
    add_16bit_full, add_8bit_full, compute_parity, int_to_bits8, invert_8bit,
};
use logic_gates::gates::{and_gate, not_gate, or_gate, xor_gate};

// ─── Result struct ────────────────────────────────────────────────────────────

/// Result of an 8051 ALU operation.
///
/// The caller decides which flags to commit to PSW based on the instruction:
/// - ADD/ADDC/SUBB update all four flags.
/// - ANL/ORL/XRL only update parity.
/// - INC/DEC do not update CY, AC, or OV.
/// - SWAP does not update any flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AluResult8051 {
    /// 8-bit result of the operation.
    pub result: u8,
    /// Carry flag (CY in PSW bit 7).
    pub cy: u8,
    /// Auxiliary carry (AC in PSW bit 6) — carry out of bit 3.
    pub ac: u8,
    /// Overflow flag (OV in PSW bit 2) — signed overflow.
    pub ov: u8,
    /// Parity: 1 when result has an **odd** number of 1-bits (PSW.P bit 0).
    pub parity: u8,
}

// ─── Private helper ───────────────────────────────────────────────────────────

/// Compute parity of a u8 result value.
fn parity_of(v: u8) -> u8 {
    compute_parity(&int_to_bits8(v))
}

// ─── Arithmetic ──────────────────────────────────────────────────────────────

/// 8-bit addition with carry: `A + B + carry_in`.
///
/// Models 8 full-adder stages.  Used by ADD and ADDC.
///
/// CY = carry out of bit 7; AC = carry out of bit 3; OV = sign overflow.
///
/// # Example
/// ```
/// use coding_adventures_intel8051_gatelevel::alu::add8;
/// let r = add8(0x50, 0x50, 0);  // 0x50 + 0x50 = 0xA0
/// assert_eq!(r.result, 0xA0);
/// assert_eq!(r.ov, 1);          // 0x50 = +80 decimal; +80+80 overflows signed byte
/// assert_eq!(r.cy, 0);
/// ```
pub fn add8(a: u8, b: u8, carry_in: u8) -> AluResult8051 {
    let (result, carries) = add_8bit_full(a, b, carry_in);
    let cy = carries[7];
    let ac = carries[3];
    let ov = xor_gate(carries[6], carries[7]);
    AluResult8051 { result, cy, ac, ov, parity: parity_of(result) }
}

/// 8-bit subtraction with borrow: `A − B − borrow_in`.
///
/// Implemented as `A + NOT(B) + NOT(borrow_in)` (two's complement).
///
/// - `CY = NOT(carry_out)` → CY=1 means borrow (A < B + borrow_in)
/// - `AC = NOT(carries[3])` → AC=1 means lower nibble borrowed
/// - `OV = XOR(carries[6], carries[7])` → signed overflow
///
/// # Example
/// ```
/// use coding_adventures_intel8051_gatelevel::alu::subb8;
/// // 0x00 - 0x01 - 0 = 0xFF with CY=1 (borrow)
/// let r = subb8(0x00, 0x01, 0);
/// assert_eq!(r.result, 0xFF);
/// assert_eq!(r.cy, 1);
/// ```
pub fn subb8(a: u8, b: u8, borrow_in: u8) -> AluResult8051 {
    let b_inv = invert_8bit(b);
    // NOT(borrow_in): borrow=0 → carry_in=1; borrow=1 → carry_in=0
    let carry_in = not_gate(borrow_in);
    let (result, carries) = add_8bit_full(a, b_inv, carry_in);
    // CY = NOT(carry_out): no carry-out of addition ↔ borrow occurred
    let cy = not_gate(carries[7]);
    // AC = NOT(nibble carry): no carry out of nibble ↔ nibble borrow
    let ac = not_gate(carries[3]);
    let ov = xor_gate(carries[6], carries[7]);
    AluResult8051 { result, cy, ac, ov, parity: parity_of(result) }
}

/// Increment by 1: `A + 1`.
///
/// INC does **not** modify CY, AC, or OV — only the value and parity.
/// The result struct carries cy=0, ac=0, ov=0 for the caller to honour.
///
/// # Example
/// ```
/// use coding_adventures_intel8051_gatelevel::alu::inc8;
/// let r = inc8(0xFF); // wraps to 0
/// assert_eq!(r.result, 0x00);
/// assert_eq!(r.cy, 0); // INC never sets CY
/// ```
pub fn inc8(a: u8) -> AluResult8051 {
    let (result, _carries) = add_8bit_full(a, 1, 0);
    // INC clears CY, AC, OV regardless of arithmetic result
    AluResult8051 { result, cy: 0, ac: 0, ov: 0, parity: parity_of(result) }
}

/// Decrement by 1: `A − 1`.
///
/// DEC does **not** modify CY, AC, or OV — only the value and parity.
///
/// # Example
/// ```
/// use coding_adventures_intel8051_gatelevel::alu::dec8;
/// let r = dec8(0x00); // wraps to 0xFF
/// assert_eq!(r.result, 0xFF);
/// assert_eq!(r.cy, 0); // DEC never sets CY
/// ```
pub fn dec8(a: u8) -> AluResult8051 {
    // A - 1 = A + NOT(1) + 1 = A + 0xFE + 1
    let b_inv = invert_8bit(1u8);
    let (result, _carries) = add_8bit_full(a, b_inv, 1);
    // DEC clears CY, AC, OV
    AluResult8051 { result, cy: 0, ac: 0, ov: 0, parity: parity_of(result) }
}

/// BCD decimal adjust after addition.
///
/// The 8051 DA A instruction adjusts ACC after a BCD ADD so that the
/// result is a valid 2-digit BCD number.
///
/// Algorithm (implemented gate-level via comparisons and conditional adds):
/// 1. If `AC=1` or lower nibble > 9, add 6 to ACC, propagate any new carry.
/// 2. If `CY=1` or adjusted value > 0x99, add 0x60 to ACC, set CY=1.
///
/// # Example
/// ```
/// use coding_adventures_intel8051_gatelevel::alu::da8;
/// // BCD: 0x09 + 0x01 = 0x10, DA should give 0x10 with CY=0
/// let r = da8(0x0A, 0, 0); // after adding 9+1=0x0A
/// assert_eq!(r.result, 0x10);
/// assert_eq!(r.cy, 0);
/// ```
pub fn da8(a: u8, cy_in: u8, ac_in: u8) -> AluResult8051 {
    let bits = int_to_bits8(a);
    // lower nibble = bits[3:0], upper nibble = bits[7:4]
    let low = bits_to_nibble_low(&bits);
    let _high = bits_to_nibble_high(&bits);

    // Step 1: check if lower nibble needs BCD correction (add 6)
    // low > 9 → gate-level: nibble comparator
    let low_needs_adj = or_gate(ac_in, nibble_gt9(low));
    let (adj1, cy1) = if low_needs_adj != 0 {
        let (v, carries) = add_8bit_full(a, 0x06, 0);
        (v, carries[7])
    } else {
        (a, 0)
    };
    let any_cy1 = or_gate(cy_in, cy1);

    // Step 2: check upper nibble (re-read after possible low adjustment)
    let bits2 = int_to_bits8(adj1);
    let high2 = bits_to_nibble_high(&bits2);
    let high_needs_adj = or_gate(any_cy1, nibble_gt9(high2));
    let (adj2, cy2) = if high_needs_adj != 0 {
        let (v, carries) = add_8bit_full(adj1, 0x60, 0);
        (v, carries[7])
    } else {
        (adj1, 0)
    };
    let out_cy = or_gate(any_cy1, cy2);

    AluResult8051 {
        result: adj2,
        cy: out_cy,
        ac: 0, // DA does not update AC
        ov: 0,
        parity: parity_of(adj2),
    }
}

/// Extract the lower nibble as a 4-bit integer from an LSB-first bit array.
fn bits_to_nibble_low(bits: &[u8; 8]) -> u8 {
    bits[0] | (bits[1] << 1) | (bits[2] << 2) | (bits[3] << 3)
}

/// Extract the upper nibble as a 4-bit integer from an LSB-first bit array.
fn bits_to_nibble_high(bits: &[u8; 8]) -> u8 {
    bits[4] | (bits[5] << 1) | (bits[6] << 2) | (bits[7] << 3)
}

/// Gate-level check: returns 1 if a 4-bit value is > 9.
///
/// A 4-bit value N > 9 iff:
///   - bit3=1 AND (bit2=1 OR bit1=1), i.e., N ≥ 10
///   - Encoding: 0b1010=10, 0b1011=11, …, 0b1111=15
///
/// Truth: N>9 = b3 AND (b2 OR b1)
fn nibble_gt9(nibble: u8) -> u8 {
    let b3 = (nibble >> 3) & 1;
    let b2 = (nibble >> 2) & 1;
    let b1 = (nibble >> 1) & 1;
    and_gate(b3, or_gate(b2, b1))
}

// ─── Logical ─────────────────────────────────────────────────────────────────

/// 8 AND gates in parallel: `A & B`.
///
/// Logical ops do **not** update CY, AC, or OV (all returned as 0).
///
/// # Example
/// ```
/// use coding_adventures_intel8051_gatelevel::alu::anl8;
/// assert_eq!(anl8(0xF0, 0x0F).result, 0x00);
/// assert_eq!(anl8(0xFF, 0xAA).result, 0xAA);
/// ```
pub fn anl8(a: u8, b: u8) -> AluResult8051 {
    let a_bits = int_to_bits8(a);
    let b_bits = int_to_bits8(b);
    let mut out = [0u8; 8];
    for i in 0..8 {
        out[i] = and_gate(a_bits[i], b_bits[i]);
    }
    let result = crate::bits::bits_to_u8(&out);
    AluResult8051 { result, cy: 0, ac: 0, ov: 0, parity: parity_of(result) }
}

/// 8 OR gates in parallel: `A | B`.
///
/// # Example
/// ```
/// use coding_adventures_intel8051_gatelevel::alu::orl8;
/// assert_eq!(orl8(0xF0, 0x0F).result, 0xFF);
/// ```
pub fn orl8(a: u8, b: u8) -> AluResult8051 {
    let a_bits = int_to_bits8(a);
    let b_bits = int_to_bits8(b);
    let mut out = [0u8; 8];
    for i in 0..8 {
        out[i] = or_gate(a_bits[i], b_bits[i]);
    }
    let result = crate::bits::bits_to_u8(&out);
    AluResult8051 { result, cy: 0, ac: 0, ov: 0, parity: parity_of(result) }
}

/// 8 XOR gates in parallel: `A ^ B`.
///
/// Also used by CPL A (XOR with 0xFF).
///
/// # Example
/// ```
/// use coding_adventures_intel8051_gatelevel::alu::xrl8;
/// assert_eq!(xrl8(0xFF, 0xFF).result, 0x00);
/// ```
pub fn xrl8(a: u8, b: u8) -> AluResult8051 {
    let a_bits = int_to_bits8(a);
    let b_bits = int_to_bits8(b);
    let mut out = [0u8; 8];
    for i in 0..8 {
        out[i] = xor_gate(a_bits[i], b_bits[i]);
    }
    let result = crate::bits::bits_to_u8(&out);
    AluResult8051 { result, cy: 0, ac: 0, ov: 0, parity: parity_of(result) }
}

// ─── Rotates ─────────────────────────────────────────────────────────────────

/// Rotate left without carry: bit 7 → CY, bit 7 → bit 0.
///
/// The 8051 RL instruction (opcode 0x23) rotates ACC left by one position.
/// The bit leaving from the MSB becomes both CY and the new LSB.
///
/// ```text
/// Before:  b7  b6  b5  b4  b3  b2  b1  b0
/// CY ←  b7
/// After:   b6  b5  b4  b3  b2  b1  b0  b7
/// ```
///
/// # Example
/// ```
/// use coding_adventures_intel8051_gatelevel::alu::rl8;
/// let r = rl8(0b10110001);  // MSB=1 rotates into LSB
/// assert_eq!(r.result, 0b01100011);
/// assert_eq!(r.cy, 1);
/// ```
pub fn rl8(a: u8) -> AluResult8051 {
    let bits = int_to_bits8(a);
    // Outgoing bit: bits[7] (MSB in LSB-first storage)
    let out_bit = bits[7];
    // New value: shift left, insert out_bit into bit 0
    let mut out = [0u8; 8];
    out[0] = out_bit;
    out[1..8].copy_from_slice(&bits[..7]);
    let result = crate::bits::bits_to_u8(&out);
    AluResult8051 { result, cy: out_bit, ac: 0, ov: 0, parity: parity_of(result) }
}

/// Rotate right without carry: bit 0 → CY, bit 0 → bit 7.
///
/// The 8051 RR instruction (opcode 0x03).  The bit leaving from the LSB
/// becomes both CY and the new MSB.
///
/// # Example
/// ```
/// use coding_adventures_intel8051_gatelevel::alu::rr8;
/// let r = rr8(0b10110001); // LSB=1 rotates into MSB
/// assert_eq!(r.result, 0b11011000);
/// assert_eq!(r.cy, 1);
/// ```
pub fn rr8(a: u8) -> AluResult8051 {
    let bits = int_to_bits8(a);
    let out_bit = bits[0]; // LSB exits
    let mut out = [0u8; 8];
    out[..7].copy_from_slice(&bits[1..8]);
    out[7] = out_bit; // enters at MSB
    let result = crate::bits::bits_to_u8(&out);
    AluResult8051 { result, cy: out_bit, ac: 0, ov: 0, parity: parity_of(result) }
}

/// Rotate left through carry: 9-bit rotate [CY | ACC] left by 1.
///
/// RLC A (opcode 0x33):
/// ```text
/// CY_new ← bit7;   bit0 ← CY_old
/// ```
///
/// # Example
/// ```
/// use coding_adventures_intel8051_gatelevel::alu::rlc8;
/// // 0b10000000 with CY=0: bit7=1 → CY_new=1, bit0=CY_old=0
/// let r = rlc8(0b10000000, 0);
/// assert_eq!(r.result, 0b00000000);
/// assert_eq!(r.cy, 1);
/// ```
pub fn rlc8(a: u8, cy_in: u8) -> AluResult8051 {
    let bits = int_to_bits8(a);
    let out_bit = bits[7]; // bit7 exits to CY
    let mut out = [0u8; 8];
    out[0] = cy_in; // CY enters at bit0
    out[1..8].copy_from_slice(&bits[..7]);
    let result = crate::bits::bits_to_u8(&out);
    AluResult8051 { result, cy: out_bit, ac: 0, ov: 0, parity: parity_of(result) }
}

/// Rotate right through carry: 9-bit rotate [ACC | CY] right by 1.
///
/// RRC A (opcode 0x13):
/// ```text
/// CY_new ← bit0;   bit7 ← CY_old
/// ```
///
/// # Example
/// ```
/// use coding_adventures_intel8051_gatelevel::alu::rrc8;
/// // 0b00000001 with CY=0: bit0=1 → CY_new=1, bit7=CY_old=0
/// let r = rrc8(0b00000001, 0);
/// assert_eq!(r.result, 0b00000000);
/// assert_eq!(r.cy, 1);
/// ```
pub fn rrc8(a: u8, cy_in: u8) -> AluResult8051 {
    let bits = int_to_bits8(a);
    let out_bit = bits[0]; // bit0 exits to CY
    let mut out = [0u8; 8];
    out[..7].copy_from_slice(&bits[1..8]);
    out[7] = cy_in; // CY enters at bit7
    let result = crate::bits::bits_to_u8(&out);
    AluResult8051 { result, cy: out_bit, ac: 0, ov: 0, parity: parity_of(result) }
}

/// Swap upper and lower nibbles of ACC: `{high, low} → {low, high}`.
///
/// SWAP A (opcode 0xC4) does not update any PSW flags — not even parity.
/// The result struct carries zeros in all flag fields.
///
/// # Example
/// ```
/// use coding_adventures_intel8051_gatelevel::alu::swap8;
/// assert_eq!(swap8(0xAB).result, 0xBA);
/// assert_eq!(swap8(0x12).result, 0x21);
/// ```
pub fn swap8(a: u8) -> AluResult8051 {
    let bits = int_to_bits8(a);
    // Wire swap: bits[0..4] ↔ bits[4..8]. `out` and `bits` are distinct
    // arrays, so the two half-copies are independent of ordering.
    let mut out = [0u8; 8];
    out[..4].copy_from_slice(&bits[4..8]); // low ← old high
    out[4..8].copy_from_slice(&bits[..4]); // high ← old low
    let result = crate::bits::bits_to_u8(&out);
    // SWAP does NOT update parity
    AluResult8051 { result, cy: 0, ac: 0, ov: 0, parity: 0 }
}

// ─── MUL / DIV ───────────────────────────────────────────────────────────────

/// Unsigned 8×8 multiply: `A × B`.
///
/// Implemented as a shift-and-add loop (8 iterations), mirroring the
/// hardware's repeated-addition approach.
///
/// Returns `(hi, lo, ov)`:
/// - `lo` → written to A
/// - `hi` → written to B
/// - `ov = 1` if result > 255 (i.e., hi ≠ 0)
/// - CY = 0 always after MUL
///
/// # Example
/// ```
/// use coding_adventures_intel8051_gatelevel::alu::mul8;
/// let (hi, lo, ov) = mul8(0x10, 0x20); // 16 × 32 = 512 = 0x0200
/// assert_eq!(hi, 0x02);
/// assert_eq!(lo, 0x00);
/// assert_eq!(ov, 1);
/// ```
pub fn mul8(a: u8, b: u8) -> (u8, u8, u8) {
    let b_bits = int_to_bits8(b);
    let mut product_lo = 0u16;
    let mut product_hi = 0u16;
    let partial = a as u16;

    // Shift-and-add over 8 iterations. `i` both selects `b_bits[i]` and drives
    // the `partial << i` shift, so the explicit index is needed.
    #[allow(clippy::needless_range_loop)]
    for i in 0..8 {
        if b_bits[i] != 0 {
            // Add partial product (shifted) into accumulator
            // We use a 16-bit add to accumulate
            let shifted = partial << i;
            let lo_part = (shifted & 0xFF) as u8;
            let hi_part = ((shifted >> 8) & 0xFF) as u8;
            let (new_lo, cy_lo) = add_16bit_full(product_lo, lo_part as u16, 0);
            let _ = cy_lo;
            product_lo = new_lo & 0xFF;
            let carry_into_hi = (new_lo >> 8) as u8;
            let (new_hi, _) = add_16bit_full(product_hi, hi_part as u16 + carry_into_hi as u16, 0);
            product_hi = new_hi & 0xFF;
        }
    }

    let lo = product_lo as u8;
    let hi = product_hi as u8;
    // OV = 1 if result > 0xFF (i.e., high byte ≠ 0)
    let hi_bits = crate::bits::int_to_bits8(hi);
    let ov = u8::from(!crate::bits::compute_zero(&hi_bits));
    (hi, lo, ov)
}

/// Unsigned 8-bit divide: `A / B`.
///
/// Implemented as repeated subtraction, mirroring the hardware.
///
/// Returns `(quotient, remainder, ov)`:
/// - `quotient` → written to A
/// - `remainder` → written to B
/// - `ov = 1` if B = 0 (divide-by-zero); result is undefined in that case
/// - CY = 0 always after DIV
///
/// # Example
/// ```
/// use coding_adventures_intel8051_gatelevel::alu::div8;
/// let (q, r, ov) = div8(0x10, 0x03); // 16 / 3 = 5 remainder 1
/// assert_eq!(q, 5);
/// assert_eq!(r, 1);
/// assert_eq!(ov, 0);
/// // Divide by zero
/// let (_, _, ov0) = div8(0x10, 0x00);
/// assert_eq!(ov0, 1);
/// ```
pub fn div8(a: u8, b: u8) -> (u8, u8, u8) {
    // OV = 1 when divisor = 0
    let b_bits = crate::bits::int_to_bits8(b);
    if crate::bits::compute_zero(&b_bits) {
        return (0, 0, 1);
    }

    let mut quotient = 0u8;
    let mut remainder = a;

    // Maximum subtractions = 255 (0xFF / 0x01, the worst case for an 8-bit
    // dividend divided by the smallest non-zero divisor).  The loop cap of
    // 256 is therefore sufficient; the CY break fires on or before iteration
    // 255, so iteration 256 is never reached in correct operation.
    for _ in 0..256u32 {
        // Compare: remainder < b ?  Use subb8: if CY=1, remainder < b
        let cmp = subb8(remainder, b, 0);
        if cmp.cy != 0 {
            break; // remainder < b, done
        }
        remainder = cmp.result;
        // quotient is bounded by 0xFF/1 = 255 — it cannot overflow here
        debug_assert!(quotient < 255, "div8: quotient overflow; loop bound violated");
        let q_res = inc8(quotient);
        quotient = q_res.result;
    }

    (quotient, remainder, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add8_basic() {
        let r = add8(1, 2, 0);
        assert_eq!(r.result, 3);
        assert_eq!(r.cy, 0);
        assert_eq!(r.ac, 0);
        assert_eq!(r.ov, 0);
    }

    #[test]
    fn add8_carry() {
        let r = add8(0xFF, 1, 0);
        assert_eq!(r.result, 0);
        assert_eq!(r.cy, 1);
    }

    #[test]
    fn add8_ac() {
        let r = add8(0x0F, 0x01, 0);
        assert_eq!(r.result, 0x10);
        assert_eq!(r.ac, 1);
    }

    #[test]
    fn add8_overflow() {
        // +127 + +1 = +128 → but as signed, +128 overflows to -128 → OV=1
        let r = add8(0x7F, 0x01, 0);
        assert_eq!(r.result, 0x80);
        assert_eq!(r.ov, 1);
    }

    #[test]
    fn subb8_no_borrow() {
        let r = subb8(5, 3, 0);
        assert_eq!(r.result, 2);
        assert_eq!(r.cy, 0);
    }

    #[test]
    fn subb8_borrow() {
        let r = subb8(0, 1, 0);
        assert_eq!(r.result, 0xFF);
        assert_eq!(r.cy, 1);
    }

    #[test]
    fn subb8_nibble_borrow() {
        // 0x10 - 0x01: lower nibble 0 < 1 → borrow from upper → AC=1
        let r = subb8(0x10, 0x01, 0);
        assert_eq!(r.result, 0x0F);
        assert_eq!(r.ac, 1);
    }

    #[test]
    fn inc_dec_no_flags() {
        let r = inc8(0xFF);
        assert_eq!(r.result, 0x00);
        assert_eq!(r.cy, 0);
        let r2 = dec8(0x00);
        assert_eq!(r2.result, 0xFF);
        assert_eq!(r2.cy, 0);
    }

    #[test]
    fn logical_ops() {
        assert_eq!(anl8(0xF0, 0xFF).result, 0xF0);
        assert_eq!(orl8(0x0F, 0xF0).result, 0xFF);
        assert_eq!(xrl8(0xFF, 0x0F).result, 0xF0);
    }

    #[test]
    fn rotate_ops() {
        let r = rl8(0b10000000);
        assert_eq!(r.result, 0b00000001);
        assert_eq!(r.cy, 1);

        let r2 = rr8(0b00000001);
        assert_eq!(r2.result, 0b10000000);
        assert_eq!(r2.cy, 1);

        let r3 = rlc8(0b10000000, 0);
        assert_eq!(r3.result, 0b00000000);
        assert_eq!(r3.cy, 1);

        let r4 = rrc8(0b00000001, 0);
        assert_eq!(r4.result, 0b00000000);
        assert_eq!(r4.cy, 1);
    }

    #[test]
    fn swap_nibbles() {
        assert_eq!(swap8(0xAB).result, 0xBA);
        assert_eq!(swap8(0x12).result, 0x21);
        // SWAP does not set parity
        assert_eq!(swap8(0x01).parity, 0);
    }

    #[test]
    fn mul8_test() {
        let (hi, lo, ov) = mul8(0x10, 0x20); // 16*32=512
        assert_eq!(hi, 0x02);
        assert_eq!(lo, 0x00);
        assert_eq!(ov, 1);

        let (hi2, lo2, ov2) = mul8(5, 10); // 50, fits in one byte
        assert_eq!(hi2, 0);
        assert_eq!(lo2, 50);
        assert_eq!(ov2, 0);
    }

    #[test]
    fn div8_test() {
        let (q, r, ov) = div8(0x10, 0x03);
        assert_eq!(q, 5);
        assert_eq!(r, 1);
        assert_eq!(ov, 0);

        let (_, _, ov0) = div8(0x10, 0x00);
        assert_eq!(ov0, 1);
    }

    #[test]
    fn da8_test() {
        // BCD: 09 + 01 = 0x0A → DA → 0x10
        let r = da8(0x0A, 0, 0);
        assert_eq!(r.result, 0x10);
        // BCD: 99 + 01 = 0x9A → DA → 0x00 with CY=1
        let r2 = da8(0x9A, 0, 0);
        assert_eq!(r2.result, 0x00);
        assert_eq!(r2.cy, 1);
    }

    #[test]
    fn parity_in_results() {
        // 0x01 → 1 set bit → odd → parity=1
        assert_eq!(add8(0, 1, 0).parity, 1);
        // 0x03 → 2 set bits → even → parity=0
        assert_eq!(add8(1, 2, 0).parity, 0);
    }
}
