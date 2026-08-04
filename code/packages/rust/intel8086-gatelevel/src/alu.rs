//! ALU for the Intel 8086 gate-level simulator.
//!
//! # Architecture
//!
//! The 8086 ALU is a 16-bit ripple-carry design with 8-bit variants for byte operations.
//! Every add/subtract routes through individual `full_adder` gate calls. Logical operations
//! (AND, OR, XOR) use parallel gate arrays. Shifts model the shift register as a bit vector
//! rewiring.
//!
//! ```text
//! Gate count estimate (ALU only):
//!   16-bit ripple adder    ~80  (16 full adders × ~5 gates each)
//!   16-bit NOT (for SUB)    16
//!   16-bit AND/OR/XOR       48  (16 parallel gates each × 3)
//!   Zero NOR tree (16-bit)  ~20
//!   Parity XOR tree (8-bit) ~8
//!   Overflow XOR gate         1
//!   Shifter/rotator         ~64
//!   ─────────────────────  ────
//!   Total ALU estimate     ~237
//! ```
//!
//! # Flag conventions
//!
//! | Flag | ADD/ADC              | SUB/SBB/CMP          | AND/OR/XOR |
//! |------|---------------------|----------------------|-----------|
//! | CF   | adder carry out      | NOT(adder carry out) | 0         |
//! | OF   | XOR(c_in15, c_out15) | XOR(c_in15, c_out15) | 0         |
//! | AF   | carries[3]           | nibble_borrow(a,b,b) | 0         |
//! | ZF   | NOR tree             | NOR tree             | NOR tree  |
//! | SF   | result[MSB]          | result[MSB]          | result[MSB]|
//! | PF   | XOR tree + NOT       | XOR tree + NOT       | XOR tree  |
//!
//! INC/DEC do not modify CF — the caller preserves the old CF value.

use logic_gates::gates::{and_gate, not_gate, or_gate, xor_gate};

use crate::bits::{
    add_8bit, add_8bit_full, add_16bit_full, bits_to_u8, bits_to_u16,
    compute_parity, compute_zero, int_to_bits8, int_to_bits16, invert_8bit, invert_16bit,
    nibble_borrow,
};

// ─── Result type ─────────────────────────────────────────────────────────────

/// Result of an ALU operation on the Intel 8086.
///
/// Contains the computed value plus all flag values the operation can affect.
/// The caller decides which flags to commit to the register file — for example,
/// INC/DEC preserve the old CF.
#[derive(Debug, Clone, Copy)]
pub struct AluResult8086 {
    /// 16-bit (or zero-extended 8-bit) result value.
    pub result: u16,
    /// Carry flag: 1 if unsigned overflow (ADD) or borrow (SUB).
    pub flag_cf: u8,
    /// Overflow flag: 1 if signed overflow.
    pub flag_of: u8,
    /// Sign flag: MSB of result.
    pub flag_sf: u8,
    /// Zero flag: 1 if result == 0.
    pub flag_zf: u8,
    /// Auxiliary carry: carry out of bit 3 (BCD).
    pub flag_af: u8,
    /// Parity flag: 1 if even number of 1-bits in low 8 bits.
    pub flag_pf: u8,
}

// ─── 16-bit arithmetic ────────────────────────────────────────────────────────

/// 16-bit ADD: A + B + carry_in.
///
/// Routes through 16 full-adder stages. OF = XOR(carry_into_bit15, carry_out_of_bit15).
///
/// # Example
/// ```
/// use coding_adventures_intel8086_gatelevel::alu::add16;
/// let r = add16(0x7FFF, 1, 0);
/// assert_eq!(r.result, 0x8000);
/// assert_eq!(r.flag_of, 1); // signed overflow: 32767 + 1 = -32768
/// assert_eq!(r.flag_cf, 0);
/// ```
pub fn add16(a: u16, b: u16, carry_in: u8) -> AluResult8086 {
    let (result, carries) = add_16bit_full(a, b, carry_in);
    let bits_r = int_to_bits16(result);
    AluResult8086 {
        result,
        flag_cf: carries[15],
        flag_of: xor_gate(carries[14], carries[15]),
        flag_sf: bits_r[15],
        flag_zf: compute_zero(&bits_r),
        flag_af: carries[3],
        flag_pf: compute_parity(&bits_r),
    }
}

/// 16-bit SUB: A - B - borrow_in via two's complement.
///
/// Gate path: 16 NOT gates → adder → NOT(carry_out) = CF.
/// CF = 1 means borrow occurred (A < B + borrow_in unsigned).
/// AF uses nibble_borrow() for correct BCD semantics.
///
/// # Example
/// ```
/// use coding_adventures_intel8086_gatelevel::alu::sub16;
/// assert_eq!(sub16(10, 3, 0).result, 7);
/// assert_eq!(sub16(0, 1, 0).flag_cf, 1); // borrow
/// ```
pub fn sub16(a: u16, b: u16, borrow_in: u8) -> AluResult8086 {
    let not_b = invert_16bit(b);
    let c_in = not_gate(borrow_in);
    let (result, carries) = add_16bit_full(a, not_b, c_in);
    let bits_r = int_to_bits16(result);
    AluResult8086 {
        result,
        flag_cf: not_gate(carries[15]),
        flag_of: xor_gate(carries[14], carries[15]),
        flag_sf: bits_r[15],
        flag_zf: compute_zero(&bits_r),
        flag_af: nibble_borrow(a as u8, b as u8, borrow_in),
        flag_pf: compute_parity(&bits_r),
    }
}

/// 16-bit AND: A & B. CF=0, OF=0, AF=0.
///
/// 16 AND gates in parallel.
pub fn and16(a: u16, b: u16) -> AluResult8086 {
    let bits_a = int_to_bits16(a);
    let bits_b = int_to_bits16(b);
    let r: Vec<u8> = (0..16).map(|i| and_gate(bits_a[i], bits_b[i])).collect();
    let result = bits_to_u16(&r);
    AluResult8086 {
        result,
        flag_cf: 0, flag_of: 0,
        flag_sf: r[15],
        flag_zf: compute_zero(&r),
        flag_af: 0,
        flag_pf: compute_parity(&r),
    }
}

/// 16-bit OR: A | B. CF=0, OF=0, AF=0.
pub fn or16(a: u16, b: u16) -> AluResult8086 {
    let bits_a = int_to_bits16(a);
    let bits_b = int_to_bits16(b);
    let r: Vec<u8> = (0..16).map(|i| or_gate(bits_a[i], bits_b[i])).collect();
    let result = bits_to_u16(&r);
    AluResult8086 {
        result,
        flag_cf: 0, flag_of: 0,
        flag_sf: r[15],
        flag_zf: compute_zero(&r),
        flag_af: 0,
        flag_pf: compute_parity(&r),
    }
}

/// 16-bit XOR: A ^ B. CF=0, OF=0, AF=0.
pub fn xor16(a: u16, b: u16) -> AluResult8086 {
    let bits_a = int_to_bits16(a);
    let bits_b = int_to_bits16(b);
    let r: Vec<u8> = (0..16).map(|i| xor_gate(bits_a[i], bits_b[i])).collect();
    let result = bits_to_u16(&r);
    AluResult8086 {
        result,
        flag_cf: 0, flag_of: 0,
        flag_sf: r[15],
        flag_zf: compute_zero(&r),
        flag_af: 0,
        flag_pf: compute_parity(&r),
    }
}

/// 16-bit INC. CF is not modified — caller preserves old CF.
pub fn inc16(a: u16) -> AluResult8086 {
    let mut r = add16(a, 1, 0);
    r.flag_cf = 0; // caller preserves CF
    r
}

/// 16-bit DEC. CF is not modified — caller preserves old CF.
pub fn dec16(a: u16) -> AluResult8086 {
    let mut r = sub16(a, 1, 0);
    r.flag_cf = 0; // caller preserves CF
    r
}

/// 16-bit NEG: 0 - A.
///
/// CF = 1 if A != 0. OF = 1 if A == 0x8000.
pub fn neg16(a: u16) -> AluResult8086 {
    sub16(0, a, 0)
}

/// 16-bit bitwise NOT. No flags affected.
pub fn not16(a: u16) -> u16 {
    invert_16bit(a)
}

// ─── 8-bit arithmetic ─────────────────────────────────────────────────────────

/// 8-bit ADD: A + B + carry_in.
///
/// # Example
/// ```
/// use coding_adventures_intel8086_gatelevel::alu::add8;
/// let r = add8(0x7F, 1, 0);
/// assert_eq!(r.result, 0x80);
/// assert_eq!(r.flag_of, 1); // signed overflow: 127 + 1 = -128
/// ```
pub fn add8(a: u8, b: u8, carry_in: u8) -> AluResult8086 {
    let (result, carries) = add_8bit_full(a, b, carry_in);
    let bits_r = int_to_bits8(result);
    AluResult8086 {
        result: result as u16,
        flag_cf: carries[7],
        flag_of: xor_gate(carries[6], carries[7]),
        flag_sf: bits_r[7],
        flag_zf: compute_zero(&bits_r),
        flag_af: carries[3],
        flag_pf: compute_parity(&bits_r),
    }
}

/// 8-bit SUB: A - B - borrow_in.
///
/// CF = 1 means borrow occurred (A < B + borrow_in unsigned).
pub fn sub8(a: u8, b: u8, borrow_in: u8) -> AluResult8086 {
    let not_b = invert_8bit(b);
    let c_in = not_gate(borrow_in);
    let (result, carries) = add_8bit_full(a, not_b, c_in);
    let bits_r = int_to_bits8(result);
    AluResult8086 {
        result: result as u16,
        flag_cf: not_gate(carries[7]),
        flag_of: xor_gate(carries[6], carries[7]),
        flag_sf: bits_r[7],
        flag_zf: compute_zero(&bits_r),
        flag_af: nibble_borrow(a, b, borrow_in),
        flag_pf: compute_parity(&bits_r),
    }
}

/// 8-bit AND: A & B. CF=0, OF=0, AF=0.
pub fn and8(a: u8, b: u8) -> AluResult8086 {
    let bits_a = int_to_bits8(a);
    let bits_b = int_to_bits8(b);
    let r: Vec<u8> = (0..8).map(|i| and_gate(bits_a[i], bits_b[i])).collect();
    let result = bits_to_u8(&r);
    AluResult8086 {
        result: result as u16,
        flag_cf: 0, flag_of: 0,
        flag_sf: r[7],
        flag_zf: compute_zero(&r),
        flag_af: 0,
        flag_pf: compute_parity(&r),
    }
}

/// 8-bit OR: A | B. CF=0, OF=0, AF=0.
pub fn or8(a: u8, b: u8) -> AluResult8086 {
    let bits_a = int_to_bits8(a);
    let bits_b = int_to_bits8(b);
    let r: Vec<u8> = (0..8).map(|i| or_gate(bits_a[i], bits_b[i])).collect();
    let result = bits_to_u8(&r);
    AluResult8086 {
        result: result as u16,
        flag_cf: 0, flag_of: 0,
        flag_sf: r[7],
        flag_zf: compute_zero(&r),
        flag_af: 0,
        flag_pf: compute_parity(&r),
    }
}

/// 8-bit XOR: A ^ B. CF=0, OF=0, AF=0.
pub fn xor8(a: u8, b: u8) -> AluResult8086 {
    let bits_a = int_to_bits8(a);
    let bits_b = int_to_bits8(b);
    let r: Vec<u8> = (0..8).map(|i| xor_gate(bits_a[i], bits_b[i])).collect();
    let result = bits_to_u8(&r);
    AluResult8086 {
        result: result as u16,
        flag_cf: 0, flag_of: 0,
        flag_sf: r[7],
        flag_zf: compute_zero(&r),
        flag_af: 0,
        flag_pf: compute_parity(&r),
    }
}

/// 8-bit INC. CF not modified — caller preserves it.
pub fn inc8(a: u8) -> AluResult8086 {
    let mut r = add8(a, 1, 0);
    r.flag_cf = 0;
    r
}

/// 8-bit DEC. CF not modified — caller preserves it.
pub fn dec8(a: u8) -> AluResult8086 {
    let mut r = sub8(a, 1, 0);
    r.flag_cf = 0;
    r
}

/// 8-bit NEG: 0 - A.
pub fn neg8(a: u8) -> AluResult8086 {
    sub8(0, a, 0)
}

/// 8-bit bitwise NOT. No flags affected.
pub fn not8(a: u8) -> u8 {
    invert_8bit(a)
}

// ─── Shift and rotate operations ─────────────────────────────────────────────
//
// All shift functions model the shift register as a bit-vector rewiring:
// bits are physically moved (wired) to new positions.  `count & 0x1F` caps
// at 31 (matching the 8086's behaviour for the CL-sourced shift count).
//
// Return type: `(result, cf)` where `result` is masked to `width` bits.

/// Logical left shift. Returns `(result, cf)`.
///
/// CF = last bit shifted out = `bits[width - count]` (before shift).
/// For count=1: CF = old MSB.
///
/// # Example
/// ```
/// use coding_adventures_intel8086_gatelevel::alu::shl;
/// assert_eq!(shl(0b10000000, 1, 8), (0, 1)); // MSB shifts out to CF
/// ```
pub fn shl(value: u16, count: u8, width: u8) -> (u16, u8) {
    let w = width as usize;
    let mask: u16 = if width == 16 { 0xFFFF } else { 0xFF };
    let cnt = (count & 0x1F) as usize;
    if cnt == 0 { return (value & mask, 0); }
    let bits: Vec<u8> = (0..w).map(|i| ((value >> i) & 1) as u8).collect();
    if cnt >= w { return (0, 0); }
    let cf = bits[w - cnt];
    let mut rb = vec![0u8; w];
    rb[cnt..w].copy_from_slice(&bits[0..w - cnt]);
    let result = rb.iter().enumerate().fold(0u16, |acc, (i, &b)| acc | ((b as u16) << i));
    (result & mask, cf)
}

/// Logical right shift. Returns `(result, cf)`.
///
/// CF = `bits[count - 1]` (last bit shifted out on the right).
pub fn shr(value: u16, count: u8, width: u8) -> (u16, u8) {
    let w = width as usize;
    let mask: u16 = if width == 16 { 0xFFFF } else { 0xFF };
    let cnt = (count & 0x1F) as usize;
    if cnt == 0 { return (value & mask, 0); }
    let bits: Vec<u8> = (0..w).map(|i| ((value >> i) & 1) as u8).collect();
    if cnt >= w { return (0, 0); }
    let cf = bits[cnt - 1];
    let mut rb = vec![0u8; w];
    rb[0..w - cnt].copy_from_slice(&bits[cnt..w]);
    let result = rb.iter().enumerate().fold(0u16, |acc, (i, &b)| acc | ((b as u16) << i));
    (result & mask, cf)
}

/// Arithmetic right shift (sign-extending). Returns `(result, cf)`.
///
/// The sign bit replicates into vacated positions.
pub fn sar(value: u16, count: u8, width: u8) -> (u16, u8) {
    let w = width as usize;
    let mask: u16 = if width == 16 { 0xFFFF } else { 0xFF };
    let cnt = (count & 0x1F) as usize;
    if cnt == 0 { return (value & mask, 0); }
    let bits: Vec<u8> = (0..w).map(|i| ((value >> i) & 1) as u8).collect();
    let sign = bits[w - 1];
    let (cf, rb) = if cnt >= w {
        (sign, vec![sign; w])
    } else {
        let cf = bits[cnt - 1];
        let mut rb = vec![sign; w];
        rb[0..w - cnt].copy_from_slice(&bits[cnt..w]);
        (cf, rb)
    };
    let result = rb.iter().enumerate().fold(0u16, |acc, (i, &b)| acc | ((b as u16) << i));
    (result & mask, cf)
}

/// Rotate left (not through carry). Returns `(result, cf)`.
///
/// CF = new bit 0 = old bit `width - count` (the bit that wrapped around).
/// For count=1: CF = old MSB.
pub fn rol(value: u16, count: u8, width: u8) -> (u16, u8) {
    let w = width as usize;
    let mask: u16 = if width == 16 { 0xFFFF } else { 0xFF };
    let cnt = (count as usize) % w;
    let bits: Vec<u8> = (0..w).map(|i| ((value >> i) & 1) as u8).collect();
    if cnt == 0 { return (value & mask, bits[0]); }
    // result_bits = bits[w - cnt ..] ++ bits[.. w - cnt]
    let mut rb = Vec::with_capacity(w);
    rb.extend_from_slice(&bits[w - cnt..]);
    rb.extend_from_slice(&bits[..w - cnt]);
    let cf = rb[0]; // new bit 0
    let result = rb.iter().enumerate().fold(0u16, |acc, (i, &b)| acc | ((b as u16) << i));
    (result & mask, cf)
}

/// Rotate right (not through carry). Returns `(result, cf)`.
///
/// CF = new MSB = old bit `count - 1`.
pub fn ror(value: u16, count: u8, width: u8) -> (u16, u8) {
    let w = width as usize;
    let mask: u16 = if width == 16 { 0xFFFF } else { 0xFF };
    let cnt = (count as usize) % w;
    let bits: Vec<u8> = (0..w).map(|i| ((value >> i) & 1) as u8).collect();
    if cnt == 0 { return (value & mask, bits[w - 1]); }
    // result_bits = bits[cnt ..] ++ bits[.. cnt]
    let mut rb = Vec::with_capacity(w);
    rb.extend_from_slice(&bits[cnt..]);
    rb.extend_from_slice(&bits[..cnt]);
    let cf = rb[w - 1]; // new MSB
    let result = rb.iter().enumerate().fold(0u16, |acc, (i, &b)| acc | ((b as u16) << i));
    (result & mask, cf)
}

/// Rotate left through carry. Returns `(result, new_cf)`.
///
/// The (width+1)-bit value `[value_bits, cf_in]` is rotated left by `count`.
/// The carry occupies position `width` in the extended vector.
pub fn rcl(value: u16, count: u8, width: u8, cf_in: u8) -> (u16, u8) {
    let w = width as usize;
    let mask: u16 = if width == 16 { 0xFFFF } else { 0xFF };
    let total = w + 1;
    let cnt = (count as usize) % total;
    let mut bits: Vec<u8> = (0..w).map(|i| ((value >> i) & 1) as u8).collect();
    bits.push(cf_in);
    if cnt == 0 {
        let result = bits[..w].iter().enumerate().fold(0u16, |acc, (i, &b)| acc | ((b as u16) << i));
        return (result & mask, cf_in);
    }
    let mut rb = Vec::with_capacity(total);
    rb.extend_from_slice(&bits[total - cnt..]);
    rb.extend_from_slice(&bits[..total - cnt]);
    let new_cf = rb[w];
    let result = rb[..w].iter().enumerate().fold(0u16, |acc, (i, &b)| acc | ((b as u16) << i));
    (result & mask, new_cf)
}

/// Rotate right through carry. Returns `(result, new_cf)`.
pub fn rcr(value: u16, count: u8, width: u8, cf_in: u8) -> (u16, u8) {
    let w = width as usize;
    let mask: u16 = if width == 16 { 0xFFFF } else { 0xFF };
    let total = w + 1;
    let cnt = (count as usize) % total;
    let mut bits: Vec<u8> = (0..w).map(|i| ((value >> i) & 1) as u8).collect();
    bits.push(cf_in);
    if cnt == 0 {
        let result = bits[..w].iter().enumerate().fold(0u16, |acc, (i, &b)| acc | ((b as u16) << i));
        return (result & mask, cf_in);
    }
    let mut rb = Vec::with_capacity(total);
    rb.extend_from_slice(&bits[cnt..]);
    rb.extend_from_slice(&bits[..cnt]);
    let new_cf = rb[w];
    let result = rb[..w].iter().enumerate().fold(0u16, |acc, (i, &b)| acc | ((b as u16) << i));
    (result & mask, new_cf)
}

// ─── BCD operations ───────────────────────────────────────────────────────────

/// DAA — Decimal Adjust AL after Addition.
///
/// Routes correction adds through `add_8bit`. Returns `(new_al, new_af, new_cf)`.
pub fn daa(al: u8, flag_af: u8, flag_cf: u8) -> (u8, u8, u8) {
    let old_al = al;
    let mut new_al = al;
    let new_cf: u8;
    let new_af: u8;
    if (old_al & 0xF) > 9 || flag_af != 0 {
        let (r, _, _) = add_8bit(new_al, 6, 0);
        new_al = r;
        new_af = 1u8;
    } else {
        new_af = 0u8;
    }
    if old_al > 0x99 || flag_cf != 0 {
        let (r, _, _) = add_8bit(new_al, 0x60, 0);
        new_al = r;
        new_cf = 1;
    } else {
        new_cf = 0;
    }
    (new_al, new_af, new_cf)
}

/// DAS — Decimal Adjust AL after Subtraction.
///
/// Correction subtractions route through `add_8bit(x, NOT(n), 1)` (two's complement).
pub fn das(al: u8, flag_af: u8, flag_cf: u8) -> (u8, u8, u8) {
    let old_al = al;
    let mut result = al;
    let new_cf: u8;
    let new_af: u8;
    if (old_al & 0xF) > 9 || flag_af != 0 {
        let (r, _, _) = add_8bit(result, invert_8bit(6), 1); // result - 6
        result = r;
        new_af = 1;
    } else {
        new_af = 0;
    }
    if old_al > 0x99 || flag_cf != 0 {
        let (r, _, _) = add_8bit(result, invert_8bit(0x60), 1); // result - 0x60
        result = r;
        new_cf = 1;
    } else {
        new_cf = 0;
    }
    (result, new_af, new_cf)
}

/// AAA — ASCII Adjust after Addition.
///
/// Returns `(new_al, new_ah, af_cf)`.
pub fn aaa(al: u8, ah: u8, flag_af: u8) -> (u8, u8, u8) {
    if (al & 0xF) > 9 || flag_af != 0 {
        let (al_out, _, _) = add_8bit(al, 6, 0);
        let (ah_out, _, _) = add_8bit(ah, 1, 0);
        (al_out & 0x0F, ah_out, 1)
    } else {
        (al & 0x0F, ah, 0)
    }
}

/// AAS — ASCII Adjust after Subtraction.
///
/// Returns `(new_al, new_ah, af_cf)`.
pub fn aas(al: u8, ah: u8, flag_af: u8) -> (u8, u8, u8) {
    if (al & 0xF) > 9 || flag_af != 0 {
        let (al_out, _, _) = add_8bit(al, invert_8bit(6), 1); // al - 6
        let (ah_out, _, _) = add_8bit(ah, invert_8bit(1), 1); // ah - 1
        (al_out & 0x0F, ah_out, 1)
    } else {
        (al & 0x0F, ah, 0)
    }
}

/// AAM — ASCII Adjust after Multiply.
///
/// Returns `(new_ah, new_al)`. AH = AL ÷ base, AL = AL mod base.
/// Note: host division used (gate-level divider out of scope).
pub fn aam(al: u8, base: u8) -> (u8, u8) {
    if base == 0 { return (0, 0); }
    (al / base, al % base)
}

/// AAD — ASCII Adjust before Division.
///
/// AL = AH × base + AL (AH set to 0 by caller). Returns new AL.
pub fn aad(ah: u8, al: u8, base: u8) -> u8 {
    let product = ((ah as u16) * (base as u16)) & 0xFF;
    let (result, _, _) = add_8bit(product as u8, al, 0);
    result
}

// ─── Multiply / Divide (host arithmetic) ─────────────────────────────────────
//
// A gate-level 16×16-bit multiplier requires ~1000 gates and is out of scope
// for this educational simulator. Host integer arithmetic is used instead.

/// Unsigned 8-bit multiply: `AX = AL × operand`. Returns `(ax, cf_of)`.
pub fn mul8(al: u8, operand: u8) -> (u16, u8) {
    let ax = (al as u16) * (operand as u16);
    let cf_of = if (ax >> 8) != 0 { 1 } else { 0 };
    (ax, cf_of)
}

/// Unsigned 16-bit multiply: `DX:AX = AX × operand`. Returns `(dx, ax, cf_of)`.
pub fn mul16(ax: u16, operand: u16) -> (u16, u16, u8) {
    let r32 = (ax as u32) * (operand as u32);
    let new_ax = (r32 & 0xFFFF) as u16;
    let new_dx = ((r32 >> 16) & 0xFFFF) as u16;
    let cf_of = if new_dx != 0 { 1 } else { 0 };
    (new_dx, new_ax, cf_of)
}

/// Signed 8-bit multiply: `AX = AL_signed × operand_signed`. Returns `(ax, cf_of)`.
pub fn imul8(al: u8, operand: u8) -> (u16, u8) {
    let a = (al as i8) as i16;
    let b = (operand as i8) as i16;
    let result = (a * b) as u16;
    let expected_hi = if (result & 0x80) != 0 { 0xFF_u16 } else { 0 };
    let cf_of = if ((result >> 8) & 0xFF) != expected_hi { 1 } else { 0 };
    (result, cf_of)
}

/// Signed 16-bit multiply: `DX:AX = AX_signed × operand_signed`. Returns `(dx, ax, cf_of)`.
pub fn imul16(ax: u16, operand: u16) -> (u16, u16, u8) {
    let a = (ax as i16) as i32;
    let b = (operand as i16) as i32;
    let r32 = (a * b) as u32;
    let new_ax = (r32 & 0xFFFF) as u16;
    let new_dx = ((r32 >> 16) & 0xFFFF) as u16;
    let expected_hi = if (new_ax & 0x8000) != 0 { 0xFFFF_u16 } else { 0 };
    let cf_of = if new_dx != expected_hi { 1 } else { 0 };
    (new_dx, new_ax, cf_of)
}

/// Unsigned 8-bit divide: `AL = AX ÷ operand, AH = AX mod operand`.
///
/// Returns `None` if operand is zero (INT 0 in real hardware).
pub fn div8(ax: u16, operand: u8) -> Option<(u8, u8)> {
    if operand == 0 { return None; }
    let q = ax / operand as u16;
    let r = ax % operand as u16;
    Some((q as u8, r as u8))
}

/// Unsigned 16-bit divide: `AX = DX:AX ÷ operand, DX = DX:AX mod operand`.
pub fn div16(dx_ax: u32, operand: u16) -> Option<(u16, u16)> {
    if operand == 0 { return None; }
    let q = dx_ax / operand as u32;
    let r = dx_ax % operand as u32;
    Some((q as u16, r as u16))
}

/// Signed 8-bit divide. Returns `None` on division by zero.
pub fn idiv8(ax: u16, operand: u8) -> Option<(u8, u8)> {
    if operand == 0 { return None; }
    let dividend = ax as i16;
    let divisor = (operand as i8) as i16;
    let q = dividend / divisor;
    let r = dividend % divisor;
    Some((q as u8, r as u8))
}

/// Signed 16-bit divide. Returns `None` on division by zero.
pub fn idiv16(dx_ax: u32, operand: u16) -> Option<(u16, u16)> {
    if operand == 0 { return None; }
    let dividend = dx_ax as i32;
    let divisor = (operand as i16) as i32;
    let q = dividend / divisor;
    let r = dividend % divisor;
    Some((q as u16, r as u16))
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add16_basic() {
        let r = add16(5, 3, 0);
        assert_eq!(r.result, 8);
        assert_eq!(r.flag_cf, 0);
        assert_eq!(r.flag_of, 0);
        assert_eq!(r.flag_zf, 0);
    }

    #[test]
    fn add16_carry() {
        let r = add16(0xFFFF, 1, 0);
        assert_eq!(r.result, 0);
        assert_eq!(r.flag_cf, 1);
        assert_eq!(r.flag_zf, 1);
    }

    #[test]
    fn add16_signed_overflow() {
        let r = add16(0x7FFF, 1, 0);
        assert_eq!(r.result, 0x8000);
        assert_eq!(r.flag_of, 1);
        assert_eq!(r.flag_sf, 1); // now negative
    }

    #[test]
    fn sub16_basic() {
        let r = sub16(10, 3, 0);
        assert_eq!(r.result, 7);
        assert_eq!(r.flag_cf, 0);
        assert_eq!(r.flag_of, 0);
    }

    #[test]
    fn sub16_borrow() {
        let r = sub16(0, 1, 0);
        assert_eq!(r.result, 0xFFFF);
        assert_eq!(r.flag_cf, 1);
    }

    #[test]
    fn sub16_signed_overflow() {
        let r = sub16(0x8000, 1, 0); // -32768 - 1 = +32767: signed overflow
        assert_eq!(r.result, 0x7FFF);
        assert_eq!(r.flag_of, 1);
    }

    #[test]
    fn and16_basic() {
        let r = and16(0xFF00, 0x0FF0);
        assert_eq!(r.result, 0x0F00);
        assert_eq!(r.flag_cf, 0);
        assert_eq!(r.flag_of, 0);
    }

    #[test]
    fn or16_basic() {
        let r = or16(0xFF00, 0x00FF);
        assert_eq!(r.result, 0xFFFF);
    }

    #[test]
    fn xor16_basic() {
        let r = xor16(0xAAAA, 0x5555);
        assert_eq!(r.result, 0xFFFF);
    }

    #[test]
    fn add8_basic() {
        let r = add8(5, 3, 0);
        assert_eq!(r.result, 8);
    }

    #[test]
    fn add8_signed_overflow() {
        let r = add8(0x7F, 1, 0);
        assert_eq!(r.result, 0x80);
        assert_eq!(r.flag_of, 1);
    }

    #[test]
    fn sub8_borrow() {
        let r = sub8(0, 1, 0);
        assert_eq!(r.result, 0xFF);
        assert_eq!(r.flag_cf, 1);
    }

    #[test]
    fn inc_dec_do_not_modify_cf() {
        let r = inc16(0xFFFF);
        assert_eq!(r.result, 0);
        assert_eq!(r.flag_cf, 0); // caller preserves CF separately
        let r = dec8(0);
        assert_eq!(r.result as u8, 0xFF);
        assert_eq!(r.flag_cf, 0);
    }

    #[test]
    fn neg16_basic() {
        let r = neg16(1);
        assert_eq!(r.result, 0xFFFF);
        assert_eq!(r.flag_cf, 1); // borrow from 0 - 1
    }

    #[test]
    fn shl_basic() {
        assert_eq!(shl(1, 1, 8), (2, 0));
        assert_eq!(shl(0x80, 1, 8), (0, 1)); // MSB shifts out
    }

    #[test]
    fn shr_basic() {
        assert_eq!(shr(2, 1, 8), (1, 0));
        assert_eq!(shr(1, 1, 8), (0, 1)); // LSB shifts out
    }

    #[test]
    fn sar_sign_extends() {
        assert_eq!(sar(0x80, 1, 8), (0xC0, 0)); // 0b10000000 → 0b11000000
        assert_eq!(sar(0x81, 1, 8), (0xC0, 1)); // CF = bit that shifted out
    }

    #[test]
    fn rol_wraps() {
        assert_eq!(rol(0x80, 1, 8), (1, 1)); // MSB wraps to bit 0, CF=1
    }

    #[test]
    fn ror_wraps() {
        assert_eq!(ror(1, 1, 8), (0x80, 1)); // bit 0 wraps to MSB, CF=1
    }

    #[test]
    fn rcl_through_carry() {
        assert_eq!(rcl(0x80, 1, 8, 0), (0, 1)); // MSB goes to CF; CF-in=0 enters bit 0
        assert_eq!(rcl(0x00, 1, 8, 1), (1, 0)); // CF=1 enters bit 0
    }

    #[test]
    fn rcr_through_carry() {
        assert_eq!(rcr(1, 1, 8, 0), (0, 1)); // bit 0 → CF; CF-in=0 enters MSB
        assert_eq!(rcr(0, 1, 8, 1), (0x80, 0)); // CF=1 enters MSB
    }

    #[test]
    fn daa_basic() {
        // 0x09 + 0x09 = 0x12 in BCD
        let (r, _af, _cf) = daa(0x12, 0, 0);
        assert_eq!(r, 0x12); // already valid BCD, no adjust needed
        // Force correction: 0x0A (10) should adjust to 0x10 (BCD for 10)
        let (r2, _, _) = daa(0x0A, 0, 0);
        assert_eq!(r2, 0x10);
    }

    #[test]
    fn mul8_basic() {
        let (ax, cf_of) = mul8(10, 5);
        assert_eq!(ax, 50);
        assert_eq!(cf_of, 0);
    }

    #[test]
    fn mul8_overflow() {
        let (ax, cf_of) = mul8(0xFF, 0xFF);
        assert_eq!(ax, 0xFE01);
        assert_eq!(cf_of, 1);
    }

    #[test]
    fn div8_basic() {
        let (q, r) = div8(100, 7).unwrap();
        assert_eq!(q, 14);
        assert_eq!(r, 2);
    }

    #[test]
    fn div8_zero() {
        assert!(div8(100, 0).is_none());
    }
}
