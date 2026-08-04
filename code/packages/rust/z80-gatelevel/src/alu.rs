//! ALUZ80 — 8-bit and 16-bit ALU for the Zilog Z80.
//!
//! Every add/subtract routes through 8 full-adder stages connected in a ripple
//! carry chain. Each stage is a `full_adder(A[i], B[i], carry_in)` gate call.
//!
//! # Z80 flag layout (F register)
//!
//! ```text
//! Bit 7  S   Sign       — bit 7 of result
//! Bit 6  Z   Zero       — result == 0
//! Bit 5  Y   undocumented
//! Bit 4  H   Half-carry — carry from bit 3 (ADD) / NOT(adder_hc) (SUB)
//! Bit 3  X   undocumented
//! Bit 2  P/V Parity (logical) / Overflow (arithmetic)
//! Bit 1  N   Subtract   — 1 after SUB/SBC/DEC/CP/NEG, 0 otherwise
//! Bit 0  C   Carry / NOT(borrow)
//! ```
//!
//! # Key Z80 differences from Intel 8080
//!
//! - **H flag**: half-carry, identical concept to 8080 AC but:
//!   for subtraction → H = NOT(adder_half_carry) (inverted)
//! - **N flag**: new in Z80, marks the last operation as subtract. DAA needs it.
//! - **P/V dual purpose**: parity for AND/OR/XOR; signed overflow for ADD/SUB.
//! - **AND always sets H=1**; OR/XOR always clear H=0 (Z80 manual quirk).
//!
//! # Subtraction via NOT + 1 (two's complement)
//!
//! A - B - borrow = A + NOT(B) + NOT(borrow).
//! When borrow_in=0 → cin=1 → standard two's complement subtract.
//! When borrow_in=1 → cin=0 → subtract with borrow.
//! C flag (output): NOT(adder_carry_out) — borrow is the inverse of carry.
//! H flag (output): NOT(adder_half_carry) — borrow at nibble is inverse.

use logic_gates::gates::{and_gate, not_gate, or_gate, xor_gate};

use crate::bits::{
    add_8bit_full, add_16bit, bits_to_u8, compute_parity, compute_zero,
    int_to_bits8, int_to_bits16, invert_8bit, invert_16bit,
};

/// Result of a Z80 ALU operation.
///
/// Contains the computed value plus all six user-visible Z80 flags.
/// The caller decides which flags to commit (e.g., INC/DEC preserve C;
/// ADD HL,rp preserves S/Z/PV).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AluResultZ80 {
    pub result: u16,   // 8-bit ops fill only low byte; 16-bit ops use full u16
    pub flag_s: u8,    // Sign (bit 7 of result)
    pub flag_z: u8,    // Zero
    pub flag_h: u8,    // Half-carry
    pub flag_pv: u8,   // Parity / Overflow
    pub flag_n: u8,    // Subtract indicator
    pub flag_c: u8,    // Carry / Borrow
}

// ─── 8-bit operations ────────────────────────────────────────────────────────

/// 8-bit addition: A + B + carry_in.
///
/// Uses the 8-stage ripple-carry adder gate chain.
/// Overflow = XOR(carry_into_bit7, carry_out_of_bit7).
/// H = carry out of bit 3.
///
/// # Examples
/// ```
/// use coding_adventures_z80_gatelevel::alu::add8;
/// let r = add8(5, 3, 0);
/// assert_eq!(r.result, 8);
/// assert_eq!(r.flag_n, 0);
/// assert_eq!(r.flag_c, 0);
///
/// let overflow = add8(0x7F, 0x01, 0); // 127 + 1 = -128 signed: overflow
/// assert_eq!(overflow.result, 0x80);
/// assert_eq!(overflow.flag_pv, 1);
/// ```
pub fn add8(a: u8, b: u8, carry_in: u8) -> AluResultZ80 {
    let (result, carries) = add_8bit_full(a, b, carry_in);
    let result_bits = int_to_bits8(result);

    let overflow = xor_gate(carries[6], carries[7]);

    AluResultZ80 {
        result: result as u16,
        flag_s: result_bits[7],
        flag_z: compute_zero(&result_bits),
        flag_h: carries[3],
        flag_pv: overflow,
        flag_n: 0,
        flag_c: carries[7],
    }
}

/// 8-bit subtraction: A - B - borrow_in.
///
/// Implemented as A + NOT(B) + NOT(borrow_in) via the ripple-carry adder.
/// C = NOT(adder_carry): borrow is the inverse of carry.
/// H = NOT(adder_half_carry): borrow-at-nibble is the inverse.
///
/// # Examples
/// ```
/// use coding_adventures_z80_gatelevel::alu::sub8;
/// let r = sub8(10, 3, 0);
/// assert_eq!(r.result, 7);
/// assert_eq!(r.flag_n, 1); // subtract
/// assert_eq!(r.flag_c, 0); // no borrow
/// ```
pub fn sub8(a: u8, b: u8, borrow_in: u8) -> AluResultZ80 {
    let not_b = invert_8bit(b);
    let cin = not_gate(borrow_in); // borrow_in=0 → cin=1 (two's complement)

    let (result, carries) = add_8bit_full(a, not_b, cin);
    let result_bits = int_to_bits8(result);

    let overflow = xor_gate(carries[6], carries[7]);

    AluResultZ80 {
        result: result as u16,
        flag_s: result_bits[7],
        flag_z: compute_zero(&result_bits),
        flag_h: not_gate(carries[3]),  // inverted for subtraction
        flag_pv: overflow,
        flag_n: 1,                      // subtraction
        flag_c: not_gate(carries[7]),  // borrow = NOT(carry)
    }
}

/// 8-bit AND: A & B.
///
/// 8 AND gates in parallel.
/// Z80 quirk: H=1 always (vs 8080 where AC = OR of bit-3 of operands).
/// PV = even parity of result.
///
/// # Example
/// ```
/// use coding_adventures_z80_gatelevel::alu::and8;
/// let r = and8(0b10101010, 0b11001100);
/// assert_eq!(r.result, 0b10001000);
/// assert_eq!(r.flag_h, 1); // Z80 AND always sets H
/// assert_eq!(r.flag_n, 0);
/// assert_eq!(r.flag_c, 0);
/// ```
pub fn and8(a: u8, b: u8) -> AluResultZ80 {
    let a_bits = int_to_bits8(a);
    let b_bits = int_to_bits8(b);
    let result_bits: Vec<u8> = (0..8).map(|i| and_gate(a_bits[i], b_bits[i])).collect();
    let result = bits_to_u8(&result_bits);

    AluResultZ80 {
        result: result as u16,
        flag_s: result_bits[7],
        flag_z: compute_zero(&result_bits),
        flag_h: 1,  // Z80 AND: H always 1
        flag_pv: compute_parity(&result_bits),
        flag_n: 0,
        flag_c: 0,
    }
}

/// 8-bit OR: A | B.
///
/// 8 OR gates in parallel. H=0, N=0, C=0 (Z80 manual).
/// PV = even parity of result.
pub fn or8(a: u8, b: u8) -> AluResultZ80 {
    let a_bits = int_to_bits8(a);
    let b_bits = int_to_bits8(b);
    let result_bits: Vec<u8> = (0..8).map(|i| or_gate(a_bits[i], b_bits[i])).collect();
    let result = bits_to_u8(&result_bits);

    AluResultZ80 {
        result: result as u16,
        flag_s: result_bits[7],
        flag_z: compute_zero(&result_bits),
        flag_h: 0,
        flag_pv: compute_parity(&result_bits),
        flag_n: 0,
        flag_c: 0,
    }
}

/// 8-bit XOR: A ^ B.
///
/// 8 XOR gates in parallel. H=0, N=0, C=0 (Z80 manual).
/// PV = even parity of result.
pub fn xor8(a: u8, b: u8) -> AluResultZ80 {
    let a_bits = int_to_bits8(a);
    let b_bits = int_to_bits8(b);
    let result_bits: Vec<u8> = (0..8).map(|i| xor_gate(a_bits[i], b_bits[i])).collect();
    let result = bits_to_u8(&result_bits);

    AluResultZ80 {
        result: result as u16,
        flag_s: result_bits[7],
        flag_z: compute_zero(&result_bits),
        flag_h: 0,
        flag_pv: compute_parity(&result_bits),
        flag_n: 0,
        flag_c: 0,
    }
}

/// Increment A by 1 (INC instruction).
///
/// Uses adder. C flag is NOT affected (caller preserves it).
/// N=0 (increment is addition). Overflow at 0x7F → 0x80.
pub fn inc8(a: u8) -> AluResultZ80 {
    let mut r = add8(a, 1, 0);
    r.flag_n = 0;
    r
}

/// Decrement A by 1 (DEC instruction).
///
/// Uses adder (via subtract). C flag is NOT affected (caller preserves it).
/// N=1 (decrement is subtraction). Overflow at 0x80 → 0x7F.
pub fn dec8(a: u8) -> AluResultZ80 {
    let mut r = sub8(a, 1, 0);
    r.flag_n = 1;
    r
}

/// Negate accumulator: A = 0 - A (NEG instruction).
///
/// Equivalent to sub8(0, a, 0). C=1 unless A=0. Overflow only at A=0x80.
pub fn neg8(a: u8) -> AluResultZ80 {
    sub8(0, a, 0)
}

/// Complement accumulator: A = NOT(A) (CPL instruction).
///
/// 8 NOT gates. H=1, N=1 always. S/Z/PV/C are NOT changed (caller preserves).
/// The caller must use only result and ignore flag_s/z/pv/c from this.
pub fn cpl8(a: u8) -> AluResultZ80 {
    let bits_a = int_to_bits8(a);
    let result_bits: Vec<u8> = bits_a.iter().map(|&b| not_gate(b)).collect();
    let result = bits_to_u8(&result_bits);

    AluResultZ80 {
        result: result as u16,
        flag_s: 0,   // caller preserves
        flag_z: 0,   // caller preserves
        flag_h: 1,   // CPL sets H
        flag_pv: 0,  // caller preserves
        flag_n: 1,   // CPL sets N
        flag_c: 0,   // caller preserves
    }
}

/// Decimal Adjust Accumulator (DAA instruction).
///
/// BCD correction after ADD or SUB. The N flag tells us which operation
/// preceded DAA (only the Z80 supports DAA after subtraction; the 8080 doesn't).
///
/// After ADD (N=0):
///   - If (A & 0x0F) > 9 or H=1: add 0x06
///   - If A > 0x99 or C=1: add 0x60, set C
///
/// After SUB (N=1):
///   - If H=1: subtract 0x06
///   - If C=1: subtract 0x60
///
/// The correction routes through the ripple-carry adder (same gate chain as
/// the main ALU).
pub fn daa8(a: u8, flag_n: u8, flag_h: u8, flag_c: u8) -> AluResultZ80 {
    let mut correction = 0u8;
    let mut new_c = flag_c;

    let result;
    let new_h;

    if flag_n == 0 {
        // After addition
        if (a & 0x0F) > 9 || flag_h != 0 {
            correction |= 0x06;
        }
        let temp = a.wrapping_add(correction);
        if temp > 0x99 || flag_c != 0 {
            correction |= 0x60;
            new_c = 1;
        }
        let (r, _, hc) = crate::bits::add_8bit(a, correction, 0);
        result = r;
        new_h = hc;
    } else {
        // After subtraction
        if flag_h != 0 {
            correction |= 0x06;
        }
        if flag_c != 0 {
            correction |= 0x60;
            new_c = 1;
        }
        if correction != 0 {
            let (r, _, hc_raw) = crate::bits::add_8bit(a, invert_8bit(correction), 1);
            result = r;
            new_h = not_gate(hc_raw);
        } else {
            result = a;
            new_h = 0;
        }
    }

    let result_bits = int_to_bits8(result);
    AluResultZ80 {
        result: result as u16,
        flag_s: result_bits[7],
        flag_z: compute_zero(&result_bits),
        flag_h: new_h,
        flag_pv: compute_parity(&result_bits),
        flag_n,  // preserved from previous operation
        flag_c: new_c,
    }
}

// ─── Rotate / Shift operations ────────────────────────────────────────────────
//
// CB-prefixed rotates/shifts affect all flags (S, Z, H=0, P/V=parity, N=0, C).
// Accumulator rotates (RLCA/RRCA/RLA/RRA) only affect H=0, N=0, C.

/// RLC: Rotate Left Circular — bit 7 → bit 0 and → C.
///
/// ```text
/// Circuit: new = {A[6]..A[0], A[7]}  new_C = A[7]
/// ```
pub fn rlc8(a: u8) -> AluResultZ80 {
    let bits = int_to_bits8(a);
    let msb = bits[7];
    let mut new_bits = vec![msb];
    new_bits.extend_from_slice(&bits[..7]);
    let result = bits_to_u8(&new_bits);
    AluResultZ80 {
        result: result as u16,
        flag_s: new_bits[7],
        flag_z: compute_zero(&new_bits),
        flag_h: 0,
        flag_pv: compute_parity(&new_bits),
        flag_n: 0,
        flag_c: msb,
    }
}

/// RRC: Rotate Right Circular — bit 0 → bit 7 and → C.
///
/// ```text
/// Circuit: new = {A[0], A[7]..A[1]}  new_C = A[0]
/// ```
pub fn rrc8(a: u8) -> AluResultZ80 {
    let bits = int_to_bits8(a);
    let lsb = bits[0];
    let mut new_bits = bits[1..].to_vec();
    new_bits.push(lsb);
    let result = bits_to_u8(&new_bits);
    AluResultZ80 {
        result: result as u16,
        flag_s: new_bits[7],
        flag_z: compute_zero(&new_bits),
        flag_h: 0,
        flag_pv: compute_parity(&new_bits),
        flag_n: 0,
        flag_c: lsb,
    }
}

/// RL: Rotate Left through carry — old_C → bit 0, bit 7 → C.
pub fn rl8(a: u8, carry_in: u8) -> AluResultZ80 {
    let bits = int_to_bits8(a);
    let msb = bits[7];
    let mut new_bits = vec![carry_in];
    new_bits.extend_from_slice(&bits[..7]);
    let result = bits_to_u8(&new_bits);
    AluResultZ80 {
        result: result as u16,
        flag_s: new_bits[7],
        flag_z: compute_zero(&new_bits),
        flag_h: 0,
        flag_pv: compute_parity(&new_bits),
        flag_n: 0,
        flag_c: msb,
    }
}

/// RR: Rotate Right through carry — old_C → bit 7, bit 0 → C.
pub fn rr8(a: u8, carry_in: u8) -> AluResultZ80 {
    let bits = int_to_bits8(a);
    let lsb = bits[0];
    let mut new_bits = bits[1..].to_vec();
    new_bits.push(carry_in);
    let result = bits_to_u8(&new_bits);
    AluResultZ80 {
        result: result as u16,
        flag_s: new_bits[7],
        flag_z: compute_zero(&new_bits),
        flag_h: 0,
        flag_pv: compute_parity(&new_bits),
        flag_n: 0,
        flag_c: lsb,
    }
}

/// SLA: Shift Left Arithmetic — 0 → bit 0, bit 7 → C.
pub fn sla8(a: u8) -> AluResultZ80 {
    let bits = int_to_bits8(a);
    let msb = bits[7];
    let mut new_bits = vec![0u8];
    new_bits.extend_from_slice(&bits[..7]);
    let result = bits_to_u8(&new_bits);
    AluResultZ80 {
        result: result as u16,
        flag_s: new_bits[7],
        flag_z: compute_zero(&new_bits),
        flag_h: 0,
        flag_pv: compute_parity(&new_bits),
        flag_n: 0,
        flag_c: msb,
    }
}

/// SLL (undocumented): Shift Left Logical — 1 → bit 0, bit 7 → C.
///
/// Not in official Z80 docs but the chip executes it (CB 0x30–0x37).
pub fn sll8(a: u8) -> AluResultZ80 {
    let bits = int_to_bits8(a);
    let msb = bits[7];
    let mut new_bits = vec![1u8];
    new_bits.extend_from_slice(&bits[..7]);
    let result = bits_to_u8(&new_bits);
    AluResultZ80 {
        result: result as u16,
        flag_s: new_bits[7],
        flag_z: compute_zero(&new_bits),
        flag_h: 0,
        flag_pv: compute_parity(&new_bits),
        flag_n: 0,
        flag_c: msb,
    }
}

/// SRA: Shift Right Arithmetic — bit 7 preserved (sign extension), bit 0 → C.
pub fn sra8(a: u8) -> AluResultZ80 {
    let bits = int_to_bits8(a);
    let lsb = bits[0];
    let msb = bits[7];
    let mut new_bits = bits[1..].to_vec();
    new_bits.push(msb);  // sign extension
    let result = bits_to_u8(&new_bits);
    AluResultZ80 {
        result: result as u16,
        flag_s: new_bits[7],
        flag_z: compute_zero(&new_bits),
        flag_h: 0,
        flag_pv: compute_parity(&new_bits),
        flag_n: 0,
        flag_c: lsb,
    }
}

/// SRL: Shift Right Logical — 0 → bit 7, bit 0 → C.
pub fn srl8(a: u8) -> AluResultZ80 {
    let bits = int_to_bits8(a);
    let lsb = bits[0];
    let mut new_bits = bits[1..].to_vec();
    new_bits.push(0u8);
    let result = bits_to_u8(&new_bits);
    AluResultZ80 {
        result: result as u16,
        flag_s: new_bits[7],  // always 0 (SRL clears sign)
        flag_z: compute_zero(&new_bits),
        flag_h: 0,
        flag_pv: compute_parity(&new_bits),
        flag_n: 0,
        flag_c: lsb,
    }
}

// ─── Accumulator rotate variants (only C affected; S/Z/PV unchanged) ─────────

/// RLCA: Rotate Left Circular Accumulator (unprefixed 0x07).
/// Same as RLC but H=0, N=0, C updated; S/Z/PV unchanged (caller preserves).
pub fn rlca8(a: u8) -> AluResultZ80 {
    let bits = int_to_bits8(a);
    let msb = bits[7];
    let mut new_bits = vec![msb];
    new_bits.extend_from_slice(&bits[..7]);
    let result = bits_to_u8(&new_bits);
    AluResultZ80 {
        result: result as u16,
        flag_s: 0, flag_z: 0, flag_h: 0, flag_pv: 0, flag_n: 0,
        flag_c: msb,
    }
}

/// RRCA: Rotate Right Circular Accumulator (unprefixed 0x0F).
pub fn rrca8(a: u8) -> AluResultZ80 {
    let bits = int_to_bits8(a);
    let lsb = bits[0];
    let mut new_bits = bits[1..].to_vec();
    new_bits.push(lsb);
    let result = bits_to_u8(&new_bits);
    AluResultZ80 {
        result: result as u16,
        flag_s: 0, flag_z: 0, flag_h: 0, flag_pv: 0, flag_n: 0,
        flag_c: lsb,
    }
}

/// RLA: Rotate Left through carry Accumulator (unprefixed 0x17).
pub fn rla8(a: u8, carry_in: u8) -> AluResultZ80 {
    let bits = int_to_bits8(a);
    let msb = bits[7];
    let mut new_bits = vec![carry_in];
    new_bits.extend_from_slice(&bits[..7]);
    let result = bits_to_u8(&new_bits);
    AluResultZ80 {
        result: result as u16,
        flag_s: 0, flag_z: 0, flag_h: 0, flag_pv: 0, flag_n: 0,
        flag_c: msb,
    }
}

/// RRA: Rotate Right through carry Accumulator (unprefixed 0x1F).
pub fn rra8(a: u8, carry_in: u8) -> AluResultZ80 {
    let bits = int_to_bits8(a);
    let lsb = bits[0];
    let mut new_bits = bits[1..].to_vec();
    new_bits.push(carry_in);
    let result = bits_to_u8(&new_bits);
    AluResultZ80 {
        result: result as u16,
        flag_s: 0, flag_z: 0, flag_h: 0, flag_pv: 0, flag_n: 0,
        flag_c: lsb,
    }
}

// ─── Bit manipulation ─────────────────────────────────────────────────────────

/// BIT b, r — test bit n of A via AND gate.
///
/// Z = NOT(A[n]) — Z=1 means the bit is 0.
/// H=1, N=0 always. S/PV/C caller-preserved.
/// The register value is NOT changed (result=0 indicates read-only).
pub fn bit_test(a: u8, bit_n: u8) -> AluResultZ80 {
    let bits_a = int_to_bits8(a);
    let tested = and_gate(bits_a[bit_n as usize], 1);
    let z = not_gate(tested);
    AluResultZ80 {
        result: 0,  // BIT doesn't write back
        flag_s: if bit_n == 7 { tested } else { 0 },
        flag_z: z,
        flag_h: 1,
        flag_pv: compute_parity(&bits_a),
        flag_n: 0,
        flag_c: 0,  // caller preserves
    }
}

/// SET b, r — set bit n of A to 1 using OR gate. Returns new value.
pub fn set_bit(a: u8, bit_n: u8) -> u8 {
    let mut bits_a = int_to_bits8(a);
    bits_a[bit_n as usize] = or_gate(bits_a[bit_n as usize], 1);
    bits_to_u8(&bits_a)
}

/// RES b, r — reset bit n of A to 0 using AND gate. Returns new value.
pub fn res_bit(a: u8, bit_n: u8) -> u8 {
    let mut bits_a = int_to_bits8(a);
    bits_a[bit_n as usize] = and_gate(bits_a[bit_n as usize], 0);
    bits_to_u8(&bits_a)
}

// ─── 16-bit operations ────────────────────────────────────────────────────────

/// ADD HL, rp — 16-bit addition (unprefixed).
///
/// Only H, N, C are affected. S/Z/PV are UNCHANGED (caller preserves).
pub fn add16(hl: u16, rp: u16) -> AluResultZ80 {
    let (result, cout, hc16) = add_16bit(hl, rp, 0);
    AluResultZ80 {
        result,
        flag_s: 0,    // not affected
        flag_z: 0,    // not affected
        flag_h: hc16, // carry from bit 11
        flag_pv: 0,   // not affected
        flag_n: 0,    // addition
        flag_c: cout,
    }
}

/// ADC HL, rp — 16-bit add with carry (ED prefix). All flags affected.
pub fn adc16(hl: u16, rp: u16, carry_in: u8) -> AluResultZ80 {
    let (result, cout, hc16) = add_16bit(hl, rp, carry_in);
    let result_bits = int_to_bits16(result);

    let hl_sign = ((hl >> 15) & 1) as u8;
    let rp_sign = ((rp >> 15) & 1) as u8;
    let res_sign = result_bits[15];
    // Overflow: same signs of inputs, different sign of result
    let overflow = and_gate(
        not_gate(xor_gate(hl_sign, rp_sign)),
        xor_gate(hl_sign, res_sign),
    );

    AluResultZ80 {
        result,
        flag_s: result_bits[15],
        flag_z: compute_zero(&result_bits),
        flag_h: hc16,
        flag_pv: overflow,
        flag_n: 0,
        flag_c: cout,
    }
}

/// SBC HL, rp — 16-bit subtract with borrow (ED prefix). All flags affected.
///
/// Implemented as HL + NOT(rp) + NOT(borrow_in).
/// C = NOT(adder_carry). H = NOT(adder_hc16) (borrow at bit 12).
pub fn sbc16(hl: u16, rp: u16, borrow_in: u8) -> AluResultZ80 {
    let not_rp = invert_16bit(rp);
    let cin = not_gate(borrow_in);

    let (result, cout, hc16) = add_16bit(hl, not_rp, cin);
    let result_bits = int_to_bits16(result);

    let hl_sign = ((hl >> 15) & 1) as u8;
    let rp_sign = ((rp >> 15) & 1) as u8;
    let res_sign = result_bits[15];
    // Subtraction overflow: opposite signs of inputs, result differs from hl
    let overflow = and_gate(
        xor_gate(hl_sign, rp_sign),
        xor_gate(hl_sign, res_sign),
    );

    AluResultZ80 {
        result,
        flag_s: result_bits[15],
        flag_z: compute_zero(&result_bits),
        flag_h: not_gate(hc16),  // inverted for subtraction
        flag_pv: overflow,
        flag_n: 1,
        flag_c: not_gate(cout),  // borrow = NOT(carry)
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
        assert_eq!(r.flag_n, 0);
        assert_eq!(r.flag_c, 0);
        assert_eq!(r.flag_z, 0);
        assert_eq!(r.flag_s, 0);
    }

    #[test]
    fn add8_overflow() {
        // 0x7F + 0x01 = 0x80 (signed overflow: positive + positive = negative)
        let r = add8(0x7F, 0x01, 0);
        assert_eq!(r.result, 0x80);
        assert_eq!(r.flag_pv, 1);
        assert_eq!(r.flag_s, 1);
        assert_eq!(r.flag_c, 0);
    }

    #[test]
    fn add8_carry() {
        let r = add8(0xFF, 0x01, 0);
        assert_eq!(r.result, 0);
        assert_eq!(r.flag_c, 1);
        assert_eq!(r.flag_z, 1);
    }

    #[test]
    fn add8_half_carry() {
        let r = add8(0x0F, 0x01, 0);
        assert_eq!(r.result, 0x10);
        assert_eq!(r.flag_h, 1);
        assert_eq!(r.flag_c, 0);
    }

    #[test]
    fn sub8_basic() {
        let r = sub8(10, 3, 0);
        assert_eq!(r.result, 7);
        assert_eq!(r.flag_n, 1);
        assert_eq!(r.flag_c, 0); // no borrow
    }

    #[test]
    fn sub8_borrow() {
        // 3 - 10 = -7 = 0xF9 unsigned, borrow occurs
        let r = sub8(3, 10, 0);
        assert_eq!(r.result, 0xF9);
        assert_eq!(r.flag_c, 1); // borrow
        assert_eq!(r.flag_s, 1); // negative
    }

    #[test]
    fn sub8_half_carry() {
        // 0x10 - 0x01 = 0x0F: borrow from bit 4 → H=1
        let r = sub8(0x10, 0x01, 0);
        assert_eq!(r.result, 0x0F);
        assert_eq!(r.flag_h, 1);
    }

    #[test]
    fn and8_sets_h() {
        let r = and8(0xFF, 0x0F);
        assert_eq!(r.result, 0x0F);
        assert_eq!(r.flag_h, 1); // Z80 AND always H=1
        assert_eq!(r.flag_c, 0);
        assert_eq!(r.flag_n, 0);
    }

    #[test]
    fn or8_clears_h() {
        let r = or8(0xF0, 0x0F);
        assert_eq!(r.result, 0xFF);
        assert_eq!(r.flag_h, 0);
        assert_eq!(r.flag_s, 1);
        assert_eq!(r.flag_z, 0);
    }

    #[test]
    fn xor8_clears_h() {
        let r = xor8(0xAA, 0xAA);
        assert_eq!(r.result, 0);
        assert_eq!(r.flag_z, 1);
        assert_eq!(r.flag_h, 0);
    }

    #[test]
    fn inc8_dec8() {
        let r = inc8(0x7F);
        assert_eq!(r.result, 0x80);
        assert_eq!(r.flag_pv, 1); // overflow: 0x7F → 0x80
        assert_eq!(r.flag_n, 0);

        let r2 = dec8(0x80);
        assert_eq!(r2.result, 0x7F);
        assert_eq!(r2.flag_pv, 1); // overflow: 0x80 → 0x7F
        assert_eq!(r2.flag_n, 1);
    }

    #[test]
    fn neg8_test() {
        let r = neg8(0x01);
        assert_eq!(r.result, 0xFF);
        assert_eq!(r.flag_c, 1); // borrow (not zero)
        assert_eq!(r.flag_n, 1);

        let r0 = neg8(0x00);
        assert_eq!(r0.result, 0x00);
        assert_eq!(r0.flag_c, 0); // NEG 0: C=0 per Z80 manual
        assert_eq!(r0.flag_z, 1);
    }

    #[test]
    fn cpl8_test() {
        let r = cpl8(0xAA);
        assert_eq!(r.result, 0x55);
        assert_eq!(r.flag_h, 1);
        assert_eq!(r.flag_n, 1);
    }

    #[test]
    fn rotates() {
        // RLC: 0b10000001 → 0b00000011, C=1
        let r = rlc8(0x81);
        assert_eq!(r.result, 0x03);
        assert_eq!(r.flag_c, 1);

        // RRC: 0b00000011 → 0b10000001, C=1
        let r2 = rrc8(0x03);
        assert_eq!(r2.result, 0x81);
        assert_eq!(r2.flag_c, 1);

        // RL through carry=1: 0b10000000 → 0b00000001 (old carry→bit0), C=1 (old bit7)
        let r3 = rl8(0x80, 1);
        assert_eq!(r3.result, 0x01);
        assert_eq!(r3.flag_c, 1);
    }

    #[test]
    fn shifts() {
        // SLA: 0b10000001 → 0b00000010, C=1
        let r = sla8(0x81);
        assert_eq!(r.result, 0x02);
        assert_eq!(r.flag_c, 1);

        // SRA: 0b10000001 → 0b11000000, C=1 (sign extended)
        let r2 = sra8(0x81);
        assert_eq!(r2.result, 0xC0);
        assert_eq!(r2.flag_c, 1);

        // SRL: 0b10000001 → 0b01000000, C=1 (no sign extension)
        let r3 = srl8(0x81);
        assert_eq!(r3.result, 0x40);
        assert_eq!(r3.flag_c, 1);
    }

    #[test]
    fn bit_operations() {
        assert_eq!(bit_test(0b10101010, 1).flag_z, 0); // bit 1 is 1 → Z=0
        assert_eq!(bit_test(0b10101010, 0).flag_z, 1); // bit 0 is 0 → Z=1
        assert_eq!(bit_test(0xFF, 7).flag_h, 1);

        assert_eq!(set_bit(0b00000000, 3), 0b00001000);
        assert_eq!(res_bit(0b11111111, 3), 0b11110111);
    }

    #[test]
    fn add16_only_hnc() {
        // ADD HL,rp: S/Z/PV not changed
        let r = add16(0x1234, 0xABCD);
        assert_eq!(r.result, 0xBE01);
        assert_eq!(r.flag_s, 0);   // not updated
        assert_eq!(r.flag_z, 0);   // not updated
        assert_eq!(r.flag_n, 0);
    }

    #[test]
    fn adc16_sbc16() {
        // ADC HL,HL: 0x0001 + 0x0001 + C=0 = 0x0002
        let r = adc16(0x0001, 0x0001, 0);
        assert_eq!(r.result, 0x0002);
        assert_eq!(r.flag_z, 0);
        assert_eq!(r.flag_n, 0);

        // SBC HL,HL: 0x0002 - 0x0002 - C=0 = 0
        let r2 = sbc16(0x0002, 0x0002, 0);
        assert_eq!(r2.result, 0x0000);
        assert_eq!(r2.flag_z, 1);
        assert_eq!(r2.flag_n, 1);
        assert_eq!(r2.flag_c, 0);
    }

    #[test]
    fn daa8_after_add() {
        // 0x09 + 0x01 (BCD 9+1=10, which is BCD 0x10)
        // Binary: 0x09 + 0x01 = 0x0A; DAA corrects to 0x10
        let r = daa8(0x0A, 0, 0, 0);
        assert_eq!(r.result, 0x10);
    }

    #[test]
    fn daa8_after_sub() {
        // DAA after SUB: 0x09 - 0x01 = 0x08 (already valid BCD; no correction)
        let r = daa8(0x08, 1, 0, 0);
        assert_eq!(r.result, 0x08);
        assert_eq!(r.flag_n, 1);
    }
}
