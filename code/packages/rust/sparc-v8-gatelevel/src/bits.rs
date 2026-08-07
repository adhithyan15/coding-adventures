//! Bit-vector helpers shared by the ALU, decoder, and CPU.
//!
//! # Representation
//!
//! All multi-bit integers are stored as `Vec<u8>` where **index 0 = LSB**.
//! This matches the ripple-carry adder contract: `adder(&a, &b, carry_in)`
//! propagates carry from index 0 (bit 0) toward the MSB.
//!
//! Example — the number 5 (binary 0101) as a 4-bit vector:
//!
//! ```text
//!  index:  0   1   2   3
//!  value:  1   0   1   0
//!          ^           ^
//!         LSB         MSB
//! ```

use logic_gates::gates::{and_gate, not_gate, or_gate, xor_gate};

/// Convert a `u32` to a 32-element LSB-first `Vec<u8>`.
pub fn u32_to_bits(n: u32) -> Vec<u8> {
    (0..32).map(|i| ((n >> i) & 1) as u8).collect()
}

/// Convert a `u64` to a 64-element LSB-first `Vec<u8>`.
pub fn u64_to_bits(n: u64) -> Vec<u8> {
    (0..64).map(|i| ((n >> i) & 1) as u8).collect()
}

/// Reconstruct a `u32` from a 32-element LSB-first slice.
///
/// # Panics
///
/// Panics if `bits.len() > 32`; a longer slice would shift by ≥ 32, which
/// overflows `u32` and panics in debug or produces garbage in release.
pub fn bits_to_u32(bits: &[u8]) -> u32 {
    assert!(bits.len() <= 32, "bits_to_u32: slice length {} exceeds 32", bits.len());
    bits.iter()
        .enumerate()
        .fold(0u32, |acc, (i, &b)| acc | ((b as u32) << i))
}

/// Reconstruct a `u64` from a 64-element LSB-first slice.
///
/// # Panics
///
/// Panics if `bits.len() > 64`.
pub fn bits_to_u64(bits: &[u8]) -> u64 {
    assert!(bits.len() <= 64, "bits_to_u64: slice length {} exceeds 64", bits.len());
    bits.iter()
        .enumerate()
        .fold(0u64, |acc, (i, &b)| acc | ((b as u64) << i))
}

// ─── Bitwise helpers ──────────────────────────────────────────────────────────

/// Bitwise AND of two 32-bit vectors.
pub fn and_32(a: &[u8], b: &[u8]) -> Vec<u8> {
    a.iter().zip(b.iter()).map(|(&x, &y)| and_gate(x, y)).collect()
}

/// Bitwise OR of two 32-bit vectors.
pub fn or_32(a: &[u8], b: &[u8]) -> Vec<u8> {
    a.iter().zip(b.iter()).map(|(&x, &y)| or_gate(x, y)).collect()
}

/// Bitwise XOR of two 32-bit vectors.
pub fn xor_32(a: &[u8], b: &[u8]) -> Vec<u8> {
    a.iter().zip(b.iter()).map(|(&x, &y)| xor_gate(x, y)).collect()
}

/// Bitwise NOT of a 32-bit vector.
pub fn not_32(a: &[u8]) -> Vec<u8> {
    a.iter().map(|&x| not_gate(x)).collect()
}

/// Bitwise AND-NOT: `a & ~b` (used by ANDN / BIC).
pub fn andn_32(a: &[u8], b: &[u8]) -> Vec<u8> {
    a.iter().zip(b.iter()).map(|(&x, &y)| and_gate(x, not_gate(y))).collect()
}

/// Bitwise OR-NOT: `a | ~b` (used by ORN).
pub fn orn_32(a: &[u8], b: &[u8]) -> Vec<u8> {
    a.iter().zip(b.iter()).map(|(&x, &y)| or_gate(x, not_gate(y))).collect()
}

/// Bitwise XNOR: `~(a ^ b)` (used by XNOR).
pub fn xnor_32(a: &[u8], b: &[u8]) -> Vec<u8> {
    a.iter()
        .zip(b.iter())
        .map(|(&x, &y)| not_gate(xor_gate(x, y)))
        .collect()
}

// ─── Shift helpers ────────────────────────────────────────────────────────────

/// Logical left shift (SLL): zero-fill at low end.
pub fn sll_32(bits: &[u8], shamt: u32) -> Vec<u8> {
    let shamt = (shamt & 0x1f) as usize;
    let mut out = vec![0u8; 32];
    out[shamt..32].copy_from_slice(&bits[..32 - shamt]);
    out
}

/// Logical right shift (SRL): zero-fill at high end.
pub fn srl_32(bits: &[u8], shamt: u32) -> Vec<u8> {
    let shamt = (shamt & 0x1f) as usize;
    let mut out = vec![0u8; 32];
    out[..32 - shamt].copy_from_slice(&bits[shamt..32]);
    out
}

/// Arithmetic right shift (SRA): sign-extend from bit 31.
pub fn sra_32(bits: &[u8], shamt: u32) -> Vec<u8> {
    let shamt = (shamt & 0x1f) as usize;
    let sign = bits[31];
    let mut out = vec![sign; 32];
    out[..32 - shamt].copy_from_slice(&bits[shamt..32]);
    out
}

// ─── Sign extension ───────────────────────────────────────────────────────────

/// Sign-extend a 13-bit immediate (bits 12:0 of `word`) to 32 bits.
pub fn sext13(word: u32) -> u32 {
    let raw = word & 0x1FFF;
    // Bit 12 is the sign bit.
    if (raw >> 12) & 1 == 1 {
        // Fill upper 19 bits with 1s.
        raw | 0xFFFF_E000
    } else {
        raw
    }
}

/// Sign-extend a 22-bit immediate (bits 21:0) to 32 bits.
pub fn sext22(word: u32) -> u32 {
    let raw = word & 0x003F_FFFF;
    if (raw >> 21) & 1 == 1 {
        raw | 0xFFC0_0000
    } else {
        raw
    }
}

/// Sign-extend a 30-bit displacement (bits 29:0) to 32 bits, used in CALL.
pub fn sext30(word: u32) -> u32 {
    let raw = word & 0x3FFF_FFFF;
    if (raw >> 29) & 1 == 1 {
        raw | 0xC000_0000
    } else {
        raw
    }
}

// ─── 32-bit addition / subtraction via ripple-carry adder ────────────────────

use arithmetic::adders::ripple_carry_adder_with_carry;

/// 32-bit addition: returns (sum_bits, carry_out).
pub fn add_32(a: &[u8], b: &[u8]) -> (Vec<u8>, u8) {
    let r = ripple_carry_adder_with_carry(a, b, 0);
    (r.sum, r.carry_out)
}

/// 32-bit addition with carry-in: returns (sum_bits, carry_out).
pub fn add_32c(a: &[u8], b: &[u8], c_in: u8) -> (Vec<u8>, u8) {
    let r = ripple_carry_adder_with_carry(a, b, c_in);
    (r.sum, r.carry_out)
}

/// 32-bit subtraction via two's complement: `a - b = a + ~b + 1`.
///
/// Returns `(difference_bits, borrow_out)` where borrow = `(a < b)`.
/// In hardware, the carry-out of the adder is the *inverted* borrow:
/// borrow = NOT(carry_out).
pub fn sub_32(a: &[u8], b: &[u8]) -> (Vec<u8>, u8) {
    let b_inv = not_32(b);
    let r = ripple_carry_adder_with_carry(a, &b_inv, 1);
    let borrow = not_gate(r.carry_out);
    (r.sum, borrow)
}

/// 32-bit subtraction with borrow-in.
pub fn sub_32b(a: &[u8], b: &[u8], borrow_in: u8) -> (Vec<u8>, u8) {
    // `a - b - borrow_in = a + ~b + (1 - borrow_in)`
    let b_inv = not_32(b);
    let carry_in = not_gate(borrow_in);
    let r = ripple_carry_adder_with_carry(a, &b_inv, carry_in);
    let borrow_out = not_gate(r.carry_out);
    (r.sum, borrow_out)
}

/// Detect signed overflow for addition: V = carry_into_bit31 XOR carry_out_of_bit31.
///
/// We compute this by running two partial adders:
/// - 31-bit adder over bits 0..30 → gives carry into bit 31
/// - 32-bit full adder → gives carry out of bit 31
///
/// V = XOR(c31_in, carry_out)
pub fn overflow_add(a: &[u8], b: &[u8], c_in: u8) -> u8 {
    let r31 = ripple_carry_adder_with_carry(&a[..31], &b[..31], c_in);
    let c31_in = r31.carry_out;
    let r32 = ripple_carry_adder_with_carry(a, b, c_in);
    let c31_out = r32.carry_out;
    xor_gate(c31_in, c31_out)
}

/// Detect signed overflow for subtraction (a - b).
pub fn overflow_sub(a: &[u8], b: &[u8]) -> u8 {
    let b_inv = not_32(b);
    overflow_add(a, &b_inv, 1)
}

/// Check whether all bits in the slice are zero (for Z flag).
pub fn compute_zero(bits: &[u8]) -> u8 {
    // NOR-tree: zero iff no bit is set.
    let any_set = bits.iter().fold(0u8, |acc, &b| or_gate(acc, b));
    not_gate(any_set)
}
