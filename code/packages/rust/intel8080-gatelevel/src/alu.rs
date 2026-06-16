//! 8-bit ALU for the Intel 8080 gate-level simulator.
//!
//! # Architecture
//!
//! The 8080's ALU is an 8-bit ripple-carry design. Every add/subtract routes
//! through 8 full-adder stages (same as the 8008 ALU, but with an extra AC flag).
//!
//! ```text
//! Bit 0: full_adder(A[0], B[0], cin)  → (S[0], C[0])
//! Bit 1: full_adder(A[1], B[1], C[0]) → (S[1], C[1])
//! ...
//! Bit 3: full_adder(A[3], B[3], C[2]) → (S[3], C[3])  ← AC = C[3]
//! ...
//! Bit 7: full_adder(A[7], B[7], C[6]) → (S[7], C[7])  ← CY = C[7]
//! ```
//!
//! # Flags
//!
//! | Flag | Source                     |
//! |------|----------------------------|
//! | CY   | carry out of bit 7         |
//! | Z    | NOR(S[0]..S[7])            |
//! | S    | S[7] (MSB of result)       |
//! | P    | XNOR(S[0]..S[7]) = NOT(XOR_N) |
//! | AC   | carry out of bit 3         |
//!
//! # ANA Auxiliary Carry Quirk
//!
//! Per Intel 8080 System Reference Manual, the AND instruction sets AC to the
//! OR of bit 3 of both operands: `AC = OR(A[3], B[3])`. This differs from
//! ADD/SUB (where AC = carry out of bit 3). It is a hardware artefact of the
//! 8080's AND gate wiring.
//!
//! # Gate count estimate
//!
//! | Component          | Gates |
//! |--------------------|-------|
//! | 8-bit ripple adder | ~40   |
//! | 8-bit NOT (SUB)    | 8     |
//! | 8-bit AND/OR/XOR   | 8 each|
//! | Parity XOR tree    | 8     |
//! | Zero NOR tree      | ~8    |
//! | Rotate mux logic   | ~16   |
//! | **Total ALU**      | **~104** |

use logic_gates::gates::{and_gate, not_gate, or_gate, xor_gate};

use crate::bits::{add_8bit, bits_to_u8, compute_parity, compute_zero, int_to_bits8, sub_8bit};

/// All five 8080 condition flags produced by an ALU operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AluFlags {
    /// CY — carry (add) or borrow (sub) flag.
    pub cy: bool,
    /// Z — zero flag: set when result == 0.
    pub zero: bool,
    /// S — sign flag: set when bit 7 of result is 1.
    pub sign: bool,
    /// P — parity flag: set when number of 1-bits is even.
    pub parity: bool,
    /// AC — auxiliary carry: carry out of bit 3 into bit 4.
    pub ac: bool,
}

/// Whether this operation updates the CY flag.
///
/// INR (increment) and DCR (decrement) do NOT touch CY.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AluResult {
    pub value: u8,
    pub flags: AluFlags,
    /// When false, the caller must preserve the existing CY flag.
    pub updates_cy: bool,
}

/// ALU operation codes.
///
/// These map directly to the 3-bit ALU select field in group-10 opcodes
/// (bits 5–3 of the opcode byte).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AluOp {
    Add = 0,
    Adc = 1,
    Sub = 2,
    Sbb = 3,
    Ana = 4,
    Xra = 5,
    Ora = 6,
    Cmp = 7,
}

impl AluOp {
    pub fn from_bits(bits: u8) -> Option<Self> {
        match bits & 7 {
            0 => Some(Self::Add),
            1 => Some(Self::Adc),
            2 => Some(Self::Sub),
            3 => Some(Self::Sbb),
            4 => Some(Self::Ana),
            5 => Some(Self::Xra),
            6 => Some(Self::Ora),
            7 => Some(Self::Cmp),
            _ => None,
        }
    }
}

/// Gate-level 8-bit ALU for the Intel 8080.
///
/// All arithmetic and logical operations route through real gate functions.
/// No host integer arithmetic is used in the execution path — only in `bits.rs`
/// for the int↔bitvector conversions.
pub struct GateAlu8080;

impl GateAlu8080 {
    // ── Arithmetic operations ────────────────────────────────────────────────

    /// ADD: A + B (no carry in).
    pub fn add(a: u8, b: u8) -> AluResult {
        Self::add_inner(a, b, 0, true)
    }

    /// ADC: A + B + CY.
    pub fn adc(a: u8, b: u8, cy: bool) -> AluResult {
        Self::add_inner(a, b, cy as u8, true)
    }

    /// SUB: A - B via two's complement adder.
    pub fn sub(a: u8, b: u8) -> AluResult {
        Self::sub_inner(a, b, 0)
    }

    /// SBB: A - B - CY.
    pub fn sbb(a: u8, b: u8, cy: bool) -> AluResult {
        Self::sub_inner(a, b, cy as u8)
    }

    /// INR: A + 1. CY flag is **not** updated.
    pub fn inr(a: u8) -> AluResult {
        let (result, _cy, ac) = add_8bit(a, 1, 0);
        let bits = int_to_bits8(result);
        AluResult {
            value: result,
            flags: AluFlags {
                cy: false, // placeholder; caller preserves existing CY
                zero: compute_zero(&bits) != 0,
                sign: bits[7] != 0,
                parity: compute_parity(&bits) != 0,
                ac: ac != 0,
            },
            updates_cy: false,
        }
    }

    /// DCR: A - 1. CY flag is **not** updated.
    pub fn dcr(a: u8) -> AluResult {
        let (result, _borrow, ac_borrow) = sub_8bit(a, 1, 0);
        let bits = int_to_bits8(result);
        AluResult {
            value: result,
            flags: AluFlags {
                cy: false, // placeholder
                zero: compute_zero(&bits) != 0,
                sign: bits[7] != 0,
                parity: compute_parity(&bits) != 0,
                ac: ac_borrow != 0,
            },
            updates_cy: false,
        }
    }

    // ── Logical operations ───────────────────────────────────────────────────

    /// ANA: A & B. CY=0; AC = OR(A[3], B[3]) per 8080 spec quirk.
    pub fn ana(a: u8, b: u8) -> AluResult {
        let a_bits = int_to_bits8(a);
        let b_bits = int_to_bits8(b);
        // 8 AND gates in parallel
        let result_bits: Vec<u8> = (0..8).map(|i| and_gate(a_bits[i], b_bits[i])).collect();
        let result = bits_to_u8(&result_bits);
        // 8080 ANA quirk: AC = OR(bit3(A), bit3(B))
        let ac = or_gate(a_bits[3], b_bits[3]) != 0;
        AluResult {
            value: result,
            flags: AluFlags {
                cy: false,
                zero: compute_zero(&result_bits) != 0,
                sign: result_bits[7] != 0,
                parity: compute_parity(&result_bits) != 0,
                ac,
            },
            updates_cy: true,
        }
    }

    /// XRA: A ^ B. CY=0, AC=0.
    pub fn xra(a: u8, b: u8) -> AluResult {
        let a_bits = int_to_bits8(a);
        let b_bits = int_to_bits8(b);
        let result_bits: Vec<u8> = (0..8).map(|i| xor_gate(a_bits[i], b_bits[i])).collect();
        let result = bits_to_u8(&result_bits);
        AluResult {
            value: result,
            flags: AluFlags {
                cy: false,
                zero: compute_zero(&result_bits) != 0,
                sign: result_bits[7] != 0,
                parity: compute_parity(&result_bits) != 0,
                ac: false,
            },
            updates_cy: true,
        }
    }

    /// ORA: A | B. CY=0, AC=0.
    pub fn ora(a: u8, b: u8) -> AluResult {
        let a_bits = int_to_bits8(a);
        let b_bits = int_to_bits8(b);
        let result_bits: Vec<u8> = (0..8).map(|i| or_gate(a_bits[i], b_bits[i])).collect();
        let result = bits_to_u8(&result_bits);
        AluResult {
            value: result,
            flags: AluFlags {
                cy: false,
                zero: compute_zero(&result_bits) != 0,
                sign: result_bits[7] != 0,
                parity: compute_parity(&result_bits) != 0,
                ac: false,
            },
            updates_cy: true,
        }
    }

    /// CMP: same as SUB but result is discarded (only flags matter).
    pub fn cmp(a: u8, b: u8) -> AluResult {
        Self::sub_inner(a, b, 0)
    }

    // ── Rotate operations ────────────────────────────────────────────────────

    /// RLC: rotate A left circular. A7→CY, A7→A0.
    pub fn rlc(a: u8) -> AluResult {
        let bits = int_to_bits8(a);
        let msb = bits[7];
        // Shift left: new[0]=old[7], new[1]=old[0], ..., new[7]=old[6]
        let mut new_bits = vec![msb];
        new_bits.extend_from_slice(&bits[..7]);
        AluResult {
            value: bits_to_u8(&new_bits),
            flags: AluFlags { cy: msb != 0, ..AluFlags::default() },
            updates_cy: true,
        }
    }

    /// RRC: rotate A right circular. A0→CY, A0→A7.
    pub fn rrc(a: u8) -> AluResult {
        let bits = int_to_bits8(a);
        let lsb = bits[0];
        let mut new_bits = bits[1..].to_vec();
        new_bits.push(lsb);
        AluResult {
            value: bits_to_u8(&new_bits),
            flags: AluFlags { cy: lsb != 0, ..AluFlags::default() },
            updates_cy: true,
        }
    }

    /// RAL: rotate A left through carry. A7→CY, old_CY→A0.
    pub fn ral(a: u8, cy: bool) -> AluResult {
        let bits = int_to_bits8(a);
        let msb = bits[7];
        let mut new_bits = vec![cy as u8];
        new_bits.extend_from_slice(&bits[..7]);
        AluResult {
            value: bits_to_u8(&new_bits),
            flags: AluFlags { cy: msb != 0, ..AluFlags::default() },
            updates_cy: true,
        }
    }

    /// RAR: rotate A right through carry. A0→CY, old_CY→A7.
    pub fn rar(a: u8, cy: bool) -> AluResult {
        let bits = int_to_bits8(a);
        let lsb = bits[0];
        let mut new_bits = bits[1..].to_vec();
        new_bits.push(cy as u8);
        AluResult {
            value: bits_to_u8(&new_bits),
            flags: AluFlags { cy: lsb != 0, ..AluFlags::default() },
            updates_cy: true,
        }
    }

    // ── Special operations ───────────────────────────────────────────────────

    /// CMA: complement accumulator. 8 NOT gates in parallel. Flags unchanged.
    pub fn cma(a: u8) -> u8 {
        let bits = int_to_bits8(a);
        let inv: Vec<u8> = bits.iter().map(|&b| not_gate(b)).collect();
        bits_to_u8(&inv)
    }

    /// STC: set carry. CY=1. No other flags affected.
    pub fn stc() -> AluFlags {
        AluFlags { cy: true, ..AluFlags::default() }
    }

    /// CMC: complement carry. CY = NOT(CY). No other flags affected.
    pub fn cmc(cy: bool) -> bool {
        not_gate(cy as u8) != 0
    }

    /// DAA: decimal adjust accumulator (two-step BCD correction).
    ///
    /// After a binary addition of two BCD numbers, DAA corrects the
    /// result to valid BCD (each nibble 0–9).
    ///
    /// Step 1 — low nibble: if (A & 0x0F) > 9 or AC==1, add 0x06
    /// Step 2 — high nibble: if A > 0x99 or CY==1, add 0x60, set CY
    ///
    /// Both correction additions route through the 8-bit adder gate chain.
    pub fn daa(a: u8, cy: bool, ac: bool) -> AluResult {
        let mut correction: u8 = 0;
        let mut new_cy = cy;

        // Step 1: low nibble correction
        let low = a & 0x0F;
        if low > 9 || ac {
            correction |= 0x06;
        }

        // Step 2: high nibble correction (check after tentative step-1 apply)
        let temp = a.wrapping_add(correction);
        let high = temp >> 4;
        if high > 9 || cy {
            correction |= 0x60;
            new_cy = true;
        }

        // Apply correction through adder
        let (result, final_cy, ac_out) = add_8bit(a, correction, 0);
        let final_cy_bool = new_cy || (final_cy != 0);
        let result_bits = int_to_bits8(result);

        AluResult {
            value: result,
            flags: AluFlags {
                cy: final_cy_bool,
                zero: compute_zero(&result_bits) != 0,
                sign: result_bits[7] != 0,
                parity: compute_parity(&result_bits) != 0,
                ac: ac_out != 0,
            },
            updates_cy: true,
        }
    }

    // ── Dispatch ─────────────────────────────────────────────────────────────

    /// Dispatch a group-10 ALU register operation.
    pub fn dispatch(op: AluOp, a: u8, b: u8, cy: bool) -> AluResult {
        match op {
            AluOp::Add => Self::add(a, b),
            AluOp::Adc => Self::adc(a, b, cy),
            AluOp::Sub => Self::sub(a, b),
            AluOp::Sbb => Self::sbb(a, b, cy),
            AluOp::Ana => Self::ana(a, b),
            AluOp::Xra => Self::xra(a, b),
            AluOp::Ora => Self::ora(a, b),
            AluOp::Cmp => Self::cmp(a, b),
        }
    }

    // ── Internal helpers ─────────────────────────────────────────────────────

    fn add_inner(a: u8, b: u8, cin: u8, updates_cy: bool) -> AluResult {
        let (result, cy, ac) = add_8bit(a, b, cin);
        let bits = int_to_bits8(result);
        AluResult {
            value: result,
            flags: AluFlags {
                cy: cy != 0,
                zero: compute_zero(&bits) != 0,
                sign: bits[7] != 0,
                parity: compute_parity(&bits) != 0,
                ac: ac != 0,
            },
            updates_cy,
        }
    }

    fn sub_inner(a: u8, b: u8, borrow_in: u8) -> AluResult {
        let (result, borrow, ac_borrow) = sub_8bit(a, b, borrow_in);
        let bits = int_to_bits8(result);
        AluResult {
            value: result,
            flags: AluFlags {
                cy: borrow != 0,
                zero: compute_zero(&bits) != 0,
                sign: bits[7] != 0,
                parity: compute_parity(&bits) != 0,
                ac: ac_borrow != 0,
            },
            updates_cy: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_basic() {
        let r = GateAlu8080::add(10, 5);
        assert_eq!(r.value, 15);
        assert!(!r.flags.cy);
        assert!(!r.flags.zero);
    }

    #[test]
    fn add_overflow() {
        let r = GateAlu8080::add(0xFF, 1);
        assert_eq!(r.value, 0);
        assert!(r.flags.cy);
        assert!(r.flags.zero);
    }

    #[test]
    fn sub_basic() {
        let r = GateAlu8080::sub(10, 5);
        assert_eq!(r.value, 5);
        assert!(!r.flags.cy);
    }

    #[test]
    fn sub_borrow() {
        let r = GateAlu8080::sub(5, 10);
        assert_eq!(r.value, 0xFB);
        assert!(r.flags.cy); // borrow occurred
    }

    #[test]
    fn ana_ac_quirk() {
        // ANA with bit 3 set in A: AC = OR(1,0) = 1
        let r = GateAlu8080::ana(0x08, 0x08); // 0b00001000 & 0b00001000
        assert_eq!(r.value, 0x08);
        assert!(r.flags.ac); // OR(bit3(A), bit3(B)) = OR(1,1) = 1
        assert!(!r.flags.cy);
    }

    #[test]
    fn xra_self() {
        let r = GateAlu8080::xra(0xAB, 0xAB);
        assert_eq!(r.value, 0);
        assert!(r.flags.zero);
        assert!(!r.flags.cy);
        assert!(!r.flags.ac);
    }

    #[test]
    fn rlc_msb_set() {
        let r = GateAlu8080::rlc(0x80); // 1000_0000
        assert_eq!(r.value, 0x01); // MSB wraps to LSB
        assert!(r.flags.cy);
    }

    #[test]
    fn rrc_lsb_set() {
        let r = GateAlu8080::rrc(0x01); // 0000_0001
        assert_eq!(r.value, 0x80); // LSB wraps to MSB
        assert!(r.flags.cy);
    }

    #[test]
    fn ral_through_carry() {
        let r = GateAlu8080::ral(0x55, true); // 0101_0101, CY=1
        // Shift left: 0xAA, old CY (1) → bit 0, old bit7 (0) → new CY
        assert_eq!(r.value, 0xAB); // 1010_1011
        assert!(!r.flags.cy); // old bit7 was 0
    }

    #[test]
    fn cma_inverts() {
        assert_eq!(GateAlu8080::cma(0xAA), 0x55);
        assert_eq!(GateAlu8080::cma(0x00), 0xFF);
    }

    #[test]
    fn inr_no_cy_update() {
        let r = GateAlu8080::inr(0xFF);
        assert_eq!(r.value, 0x00);
        assert!(r.flags.zero);
        assert!(!r.updates_cy);
    }

    #[test]
    fn dcr_basic() {
        let r = GateAlu8080::dcr(5);
        assert_eq!(r.value, 4);
        assert!(!r.updates_cy);
    }

    #[test]
    fn parity_flag() {
        let r = GateAlu8080::add(0x03, 0x00); // 0x03 = 2 ones → even parity
        assert!(r.flags.parity);
        let r2 = GateAlu8080::add(0x01, 0x00); // 1 one → odd parity
        assert!(!r2.flags.parity);
    }
}
