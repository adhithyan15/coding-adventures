//! bits.rs — 32-bit integer ↔ LSB-first bit-vector bridge for MIPS R2000.
//!
//! # Design principle
//!
//! Python integers are used only at the boundary (encoding/decoding).
//! Every other module operates on `[u8; N]` bit arrays where index 0 is the
//! least-significant bit.  This mirrors real hardware: the pin interface
//! converts voltages to logic levels; internal logic never "knows" about
//! decimal.
//!
//! # Bit ordering
//!
//! All bit arrays are LSB-first: index 0 = bit 0 = least-significant.
//!
//! ```
//! // int_to_bits32(5) → [1, 0, 1, 0, 0, ..., 0]
//! //                     ↑ bit 0 (value 1)
//! //                           ↑ bit 2 (value 4)
//! ```
//!
//! # Overflow detection for add_32bit
//!
//! Signed overflow occurs when two same-sign operands produce an
//! opposite-sign result.  Equivalently:
//!   `overflow = XOR(carry_into_bit31, carry_out_of_bit31)`
//!
//! We extract carry_out_of_bit31 from a 33-bit ripple-carry adder (bit 32
//! of the sum, since both inputs have bit 32 = 0 — the 33rd full-adder just
//! passes the carry through as its sum bit).
//!
//! We extract carry_into_bit31 by running a separate 31-bit ripple-carry
//! adder over bits[0..31] and reading its carry_out.

use arithmetic::adders::ripple_carry_adder_with_carry;
use logic_gates::gates::{not_gate, or_gate, xor_gate};

// ── Integer ↔ bit vector ──────────────────────────────────────────────────────

/// Convert a 32-bit unsigned integer to an LSB-first 32-bit array.
///
/// Bit index 0 is the least-significant bit (value 2⁰ = 1).
///
/// ```
/// # use coding_adventures_mips_r2000_gatelevel::bits::int_to_bits32;
/// let b = int_to_bits32(5);
/// assert_eq!(b[0], 1); // bit 0 = 1
/// assert_eq!(b[1], 0); // bit 1 = 0
/// assert_eq!(b[2], 1); // bit 2 = 1
/// ```
pub fn int_to_bits32(value: u32) -> [u8; 32] {
    let mut bits = [0u8; 32];
    for (i, bit) in bits.iter_mut().enumerate() {
        *bit = ((value >> i) & 1) as u8;
    }
    bits
}

/// Convert an LSB-first 32-bit array to a `u32`.
///
/// ```
/// # use coding_adventures_mips_r2000_gatelevel::bits::{int_to_bits32, bits_to_u32};
/// assert_eq!(bits_to_u32(int_to_bits32(42)), 42);
/// assert_eq!(bits_to_u32(int_to_bits32(0)), 0);
/// assert_eq!(bits_to_u32(int_to_bits32(0xFFFF_FFFF)), 0xFFFF_FFFF);
/// ```
pub fn bits_to_u32(bits: [u8; 32]) -> u32 {
    let mut v = 0u32;
    for (i, &bit) in bits.iter().enumerate() {
        v |= (bit as u32) << i;
    }
    v
}

/// Convert a 64-bit unsigned integer to an LSB-first 64-bit array.
pub fn int_to_bits64(value: u64) -> [u8; 64] {
    let mut bits = [0u8; 64];
    for (i, bit) in bits.iter_mut().enumerate() {
        *bit = ((value >> i) & 1) as u8;
    }
    bits
}

/// Convert an LSB-first 64-bit array to a `u64`.
pub fn bits_to_u64(bits: [u8; 64]) -> u64 {
    let mut v = 0u64;
    for (i, &bit) in bits.iter().enumerate() {
        v |= (bit as u64) << i;
    }
    v
}

// ── 32-bit addition ───────────────────────────────────────────────────────────

/// Add two 32-bit values via a gate-level ripple-carry adder.
///
/// Returns `(result, carry_out, overflow)` where:
/// - `result` is the 32-bit unsigned sum
/// - `carry_out` is the carry out of bit 31 (unsigned overflow indicator)
/// - `overflow` is 1 if signed two's-complement overflow occurred
///
/// Overflow detection uses `XOR(carry_into_bit31, carry_out_of_bit31)`.
/// Both carries are extracted by running separate ripple-carry sub-adders
/// (a 31-bit one for carry_into_bit31, a 33-bit one for carry_out_of_bit31).
///
/// ```
/// # use coding_adventures_mips_r2000_gatelevel::bits::add_32bit;
/// let (r, c, ov) = add_32bit(1, 2, 0);
/// assert_eq!(r, 3); assert_eq!(c, 0); assert_eq!(ov, 0);
///
/// // Unsigned wrap-around: carry_out = 1
/// let (r, c, _) = add_32bit(0xFFFF_FFFF, 1, 0);
/// assert_eq!(r, 0); assert_eq!(c, 1);
///
/// // Signed overflow: MAX_INT + 1
/// let (_, _, ov) = add_32bit(0x7FFF_FFFF, 1, 0);
/// assert_eq!(ov, 1);
///
/// // No overflow: -1 + 1 = 0
/// let (_, _, ov) = add_32bit(0xFFFF_FFFF, 1, 0);
/// assert_eq!(ov, 0);
/// ```
pub fn add_32bit(a: u32, b: u32, carry_in: u8) -> (u32, u8, u8) {
    // 33-bit adder: a[32]=0, b[32]=0.
    // sum[32] = carry_out_of_bit31 (since full_adder(0,0,c)=(c,0)).
    let mut a33 = [0u8; 33];
    let mut b33 = [0u8; 33];
    for i in 0..32 {
        a33[i] = ((a >> i) & 1) as u8;
        b33[i] = ((b >> i) & 1) as u8;
    }
    // a33[32] = b33[32] = 0 already

    let res33 = ripple_carry_adder_with_carry(&a33, &b33, carry_in);
    let sum33 = res33.sum;

    let mut result_bits = [0u8; 32];
    result_bits.copy_from_slice(&sum33[..32]);
    let result = bits_to_u32(result_bits);
    let carry_out_of_31 = sum33[32]; // carry_out of bit 31 = bit 32 of 33-bit sum

    // 31-bit adder: carry_out = carry_into_bit31
    let mut a31 = [0u8; 31];
    let mut b31 = [0u8; 31];
    for i in 0..31 {
        a31[i] = ((a >> i) & 1) as u8;
        b31[i] = ((b >> i) & 1) as u8;
    }
    let res31 = ripple_carry_adder_with_carry(&a31, &b31, carry_in);
    let carry_into_31 = res31.carry_out;

    let overflow = xor_gate(carry_into_31, carry_out_of_31);

    (result, carry_out_of_31, overflow)
}

// ── 64-bit addition ───────────────────────────────────────────────────────────

/// Add two 64-bit values via a 64-stage gate-level ripple-carry adder.
///
/// Used by MULT/MULTU to accumulate 64-bit partial products.
///
/// Returns `(result, carry_out)`.
///
/// ```
/// # use coding_adventures_mips_r2000_gatelevel::bits::add_64bit;
/// let (r, c) = add_64bit(0xFFFF_FFFF_FFFF_FFFFu64, 1, 0);
/// assert_eq!(r, 0); assert_eq!(c, 1);
/// let (r, _) = add_64bit(100, 200, 0);
/// assert_eq!(r, 300);
/// ```
pub fn add_64bit(a: u64, b: u64, carry_in: u8) -> (u64, u8) {
    let a_bits = int_to_bits64(a);
    let b_bits = int_to_bits64(b);
    let res = ripple_carry_adder_with_carry(&a_bits, &b_bits, carry_in);
    let mut result_arr = [0u8; 64];
    result_arr.copy_from_slice(&res.sum);
    (bits_to_u64(result_arr), res.carry_out)
}

// ── Bitwise NOT ────────────────────────────────────────────────────────────────

/// Bitwise NOT of a 32-bit value via 32 NOT gates in parallel.
///
/// ```
/// # use coding_adventures_mips_r2000_gatelevel::bits::invert_32bit;
/// assert_eq!(invert_32bit(0x0000_0000), 0xFFFF_FFFF);
/// assert_eq!(invert_32bit(0xFFFF_FFFF), 0x0000_0000);
/// assert_eq!(invert_32bit(0xAAAA_AAAA), 0x5555_5555);
/// ```
pub fn invert_32bit(value: u32) -> u32 {
    let bits = int_to_bits32(value);
    let mut inv = [0u8; 32];
    for i in 0..32 {
        inv[i] = not_gate(bits[i]);
    }
    bits_to_u32(inv)
}

// ── Zero detection ────────────────────────────────────────────────────────────

/// Return 1 if all 32 bits are 0, else 0 (NOR reduction tree).
///
/// In hardware: a tree of NOR gates reduces 32 bits to 1.
/// Logically: zero = NOT(OR(b0, b1, ..., b31))
///
/// ```
/// # use coding_adventures_mips_r2000_gatelevel::bits::compute_zero;
/// assert_eq!(compute_zero(0x0000_0000), 1);
/// assert_eq!(compute_zero(0x0000_0001), 0);
/// assert_eq!(compute_zero(0xFFFF_FFFF), 0);
/// assert_eq!(compute_zero(0x8000_0000), 0);
/// ```
pub fn compute_zero(value: u32) -> u8 {
    let bits = int_to_bits32(value);
    let mut combined = 0u8;
    for b in bits {
        combined = or_gate(combined, b);
    }
    not_gate(combined)
}

// ── Shift operations ──────────────────────────────────────────────────────────

/// Shift left logical by `shamt` bits (0–31).
///
/// In hardware, a barrel shifter is a cross-bar of multiplexers.
/// We model it as direct bit-list manipulation: shift the LSB-first array
/// toward higher indices (multiplying by 2^shamt), filling vacated
/// low indices with 0.
///
/// ```
/// # use coding_adventures_mips_r2000_gatelevel::bits::shl_32;
/// assert_eq!(shl_32(1, 1), 2);
/// assert_eq!(shl_32(1, 31), 0x8000_0000);
/// assert_eq!(shl_32(0xFFFF_FFFF, 1), 0xFFFF_FFFE);
/// assert_eq!(shl_32(0, 0), 0);
/// ```
pub fn shl_32(value: u32, shamt: u32) -> u32 {
    if shamt == 0 {
        return value;
    }
    if shamt >= 32 {
        return 0;
    }
    let bits = int_to_bits32(value);
    let s = shamt as usize;
    let mut shifted = [0u8; 32];
    // LSB-first: bit[i] → bit[i+s]. Low s positions become 0.
    shifted[s..].copy_from_slice(&bits[..32 - s]);
    bits_to_u32(shifted)
}

/// Shift right logical by `shamt` bits (zero-fill from MSB).
///
/// ```
/// # use coding_adventures_mips_r2000_gatelevel::bits::shr_32_logical;
/// assert_eq!(shr_32_logical(4, 1), 2);
/// assert_eq!(shr_32_logical(0x8000_0000, 31), 1);
/// assert_eq!(shr_32_logical(0xFFFF_FFFF, 16), 0x0000_FFFF);
/// ```
pub fn shr_32_logical(value: u32, shamt: u32) -> u32 {
    if shamt == 0 {
        return value;
    }
    if shamt >= 32 {
        return 0;
    }
    let bits = int_to_bits32(value);
    let s = shamt as usize;
    let mut shifted = [0u8; 32];
    // LSB-first: bit[i+s] → bit[i]. High s positions become 0.
    shifted[..32 - s].copy_from_slice(&bits[s..]);
    bits_to_u32(shifted)
}

/// Shift right arithmetic by `shamt` bits (sign-fill from MSB).
///
/// The sign bit (bit 31) is replicated into the vacated high positions,
/// preserving the two's-complement sign.
///
/// ```
/// # use coding_adventures_mips_r2000_gatelevel::bits::shr_32_arith;
/// assert_eq!(shr_32_arith(4, 1), 2);
/// // -2147483648 >> 1 = 0xC000_0000
/// assert_eq!(shr_32_arith(0x8000_0000, 1), 0xC000_0000);
/// // Fully sign-extended
/// assert_eq!(shr_32_arith(0x8000_0000, 31), 0xFFFF_FFFF);
/// assert_eq!(shr_32_arith(0x4000_0000, 31), 0);
/// ```
pub fn shr_32_arith(value: u32, shamt: u32) -> u32 {
    let bits = int_to_bits32(value);
    let sign_bit = bits[31];
    if shamt == 0 {
        return value;
    }
    if shamt >= 32 {
        // Fully sign-extended
        let fill = [sign_bit; 32];
        return bits_to_u32(fill);
    }
    let s = shamt as usize;
    let mut shifted = [0u8; 32];
    shifted[..32 - s].copy_from_slice(&bits[s..]);
    shifted[(32 - s)..].fill(sign_bit);
    bits_to_u32(shifted)
}
