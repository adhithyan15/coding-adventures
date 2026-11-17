//! # `sem`: what one RV32I operation means as a circuit, in two spellings
//!
//! The probe compares two descriptions of the same computation.  This module holds both
//! spellings of every operation the probe's stepper knows:
//!
//! * the **primary** circuit, the straightforward `bit-blaster` construction (`a + b` is a
//!   ripple-carry adder, `a ^ b` is one XOR gate per bit, ...), used by the code side and by
//!   every reference instruction that is *not* diverged; and
//! * the **alternative** circuit, a different construction of the *same function*: the
//!   adder as `a - (-b)` or as `(a ^ b) + ((a & b) << 1)`, a comparison through the borrow
//!   identity of *Hacker's Delight* 2-12, a barrel shifter whose stages run in the opposite
//!   order, a multiplier with its operands swapped.
//!
//! ```text
//!   a + b      primary:   full adder, bit 0 .. bit 31
//!              alt 0:     a - (0 - b)                 two subtractors, different carry chain
//!              alt 1:     (a ^ b) + ((a & b) << 1)    the half-adder identity
//! ```
//!
//! A reference side that uses the alternative for a share `d` of its instructions is the
//! probe's **synthetic divergence** (VOS30 section 8.4, item 2): where `d = 0` the two sides
//! are the *same AIG nodes* and the miter collapses (guard G1 must report it), and the more
//! instructions differ, the more the SAT solver has to *prove* instead of the AIG
//! hash-consing it away.  Every alternative is checked equal to its primary on random
//! inputs by this module's tests; a wrong alternative would turn an UNSAT experiment into
//! a spurious SAT one.

use bit_blaster::aig::{Aig, Edge};
use bit_blaster::bv::{self, BitVec};

/// The register-register operations of the probe's RV32I subset (immediate forms use the
/// same kind with a constant right operand).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Bin {
    /// `add`, `addi`.
    Add,
    /// `sub`.
    Sub,
    /// `sll`, `slli`.
    Sll,
    /// `slt`, `slti`.
    Slt,
    /// `sltu`, `sltiu`.
    Sltu,
    /// `xor`, `xori`.
    Xor,
    /// `srl`, `srli`.
    Srl,
    /// `sra`, `srai`.
    Sra,
    /// `or`, `ori`.
    Or,
    /// `and`, `andi`.
    And,
    /// `mul` (RV32M, the low word of the product).
    Mul,
}

/// A one-bit predicate as a 32-bit word: the bit in position 0, zeros above.
pub fn bool_word(e: Edge) -> BitVec {
    BitVec {
        bits: (0..32)
            .map(|i| if i == 0 { e } else { Edge::FALSE })
            .collect(),
    }
}

/// The five low bits of a word: RV32I shifts use only those.
fn shamt(b: &BitVec) -> BitVec {
    bv::extract(b, 4, 0)
}

/// The shift amount as a full-width word (the barrel shifter wants operands of one width; the
/// zero bits above bit 4 fold away, so no overflow logic is left).
fn shamt32(b: &BitVec) -> BitVec {
    bv::zext(&shamt(b), 27)
}

/// The primary circuit of `k` over two 32-bit words.
pub fn primary(aig: &mut Aig, k: Bin, a: &BitVec, b: &BitVec) -> BitVec {
    match k {
        Bin::Add => bv::add(aig, a, b),
        Bin::Sub => bv::sub(aig, a, b),
        Bin::Sll => bv::shl(aig, a, &shamt32(b)),
        Bin::Srl => bv::lshr(aig, a, &shamt32(b)),
        Bin::Sra => bv::ashr(aig, a, &shamt32(b)),
        Bin::Slt => bool_word(bv::slt(aig, a, b)),
        Bin::Sltu => bool_word(bv::ult(aig, a, b)),
        Bin::Xor => bv::xor(aig, a, b),
        Bin::Or => bv::or(aig, a, b),
        Bin::And => bv::and(aig, a, b),
        Bin::Mul => bv::mul(aig, a, b),
    }
}

/// How many alternative circuits `k` has (0: only the primary).
pub fn alt_count(k: Bin) -> u32 {
    match k {
        Bin::Add => 2,
        _ => 1,
    }
}

/// The alternative to use when the second operand is a constant, if there is one that is not
/// hash-consed back into the primary.
pub fn imm_alternative(k: Bin) -> Option<u32> {
    match k {
        Bin::Add => Some(1),
        Bin::Xor | Bin::Or | Bin::And | Bin::Slt | Bin::Sltu => Some(0),
        _ => None,
    }
}

/// The alternative circuit number `variant` of `k` (`variant < alt_count(k)`), or `None`
/// if `k` has none.  Same function as [`primary`], different gates.
pub fn alternative(aig: &mut Aig, k: Bin, variant: u32, a: &BitVec, b: &BitVec) -> Option<BitVec> {
    let msb_word = |e: Edge| bool_word(e);
    Some(match k {
        Bin::Add if variant.is_multiple_of(2) => {
            let nb = bv::neg(aig, b);
            bv::sub(aig, a, &nb)
        }
        Bin::Add => {
            let x = bv::xor(aig, a, b);
            let c = bv::and(aig, a, b);
            let c2 = bv::shl_const(&c, 1);
            bv::add(aig, &x, &c2)
        }
        Bin::Sub => {
            let nb = bv::neg(aig, b);
            bv::add(aig, a, &nb)
        }
        Bin::Xor => {
            // a ^ b = (a + b) - 2 (a & b); the AIG's own XOR is (a | b) & !(a & b), so the
            // textbook "or-and-not" spelling would hash-cons into it and collapse.
            let s = bv::add(aig, a, b);
            let n = bv::and(aig, a, b);
            let n2 = bv::shl_const(&n, 1);
            bv::sub(aig, &s, &n2)
        }
        Bin::Or => {
            let s = bv::add(aig, a, b);
            let n = bv::and(aig, a, b);
            bv::sub(aig, &s, &n)
        }
        Bin::And => {
            let s = bv::add(aig, a, b);
            let o = bv::or(aig, a, b);
            bv::sub(aig, &s, &o)
        }
        Bin::Sltu => {
            // borrow(a - b) = msb( (~a & b) | ((~a | b) & (a - b)) )   (Hacker's Delight 2-12)
            let na = bv::not(a);
            let d = bv::sub(aig, a, b);
            let t1 = bv::and(aig, &na, b);
            let t2o = bv::or(aig, &na, b);
            let t2 = bv::and(aig, &t2o, &d);
            let r = bv::or(aig, &t1, &t2);
            msb_word(r.msb())
        }
        Bin::Slt => {
            // a < b signed = msb( d ^ ((a ^ b) & (d ^ a)) ) with d = a - b   (Hacker's Delight 2-12)
            let d = bv::sub(aig, a, b);
            let x = bv::xor(aig, a, b);
            let y = bv::xor(aig, &d, a);
            let z = bv::and(aig, &x, &y);
            let r = bv::xor(aig, &d, &z);
            msb_word(r.msb())
        }
        Bin::Sll | Bin::Srl | Bin::Sra => {
            // The same barrel shifter with its five stages applied from the top (shift by 16 first).
            let s = shamt(b);
            let mut x = a.clone();
            for k2 in (0..5).rev() {
                let moved = match k {
                    Bin::Sll => bv::shl_const(&x, 1 << k2),
                    Bin::Srl => bv::lshr_const(&x, 1 << k2),
                    _ => bv::ashr_const(&x, 1 << k2),
                };
                x = bv::ite(aig, s.bits[k2], &moved, &x);
            }
            x
        }
        Bin::Mul => bv::mul(aig, b, a),
    })
}

/// Evaluate a circuit word on one concrete assignment (every AIG input given), as a `u32`.
pub fn eval_word(aig: &Aig, w: &BitVec, inputs: &[bool]) -> u32 {
    let words: Vec<u64> = inputs.iter().map(|&b| b as u64).collect();
    let values = aig.simulate(&words);
    let mut v = 0u32;
    for (i, e) in w.bits.iter().enumerate().take(32) {
        if bit_blaster::aig::edge_value(&values, *e) & 1 == 1 {
            v |= 1 << i;
        }
    }
    v
}

/// The concrete semantics of [`Bin`] on `u32`, written independently of every circuit above:
/// plain Rust integer arithmetic (the *independent concrete evaluator* of guard G3 uses it).
pub fn concrete(k: Bin, a: u32, b: u32) -> u32 {
    match k {
        Bin::Add => a.wrapping_add(b),
        Bin::Sub => a.wrapping_sub(b),
        Bin::Sll => a << (b & 31),
        Bin::Srl => a >> (b & 31),
        Bin::Sra => ((a as i32) >> (b & 31)) as u32,
        Bin::Slt => ((a as i32) < (b as i32)) as u32,
        Bin::Sltu => (a < b) as u32,
        Bin::Xor => a ^ b,
        Bin::Or => a | b,
        Bin::And => a & b,
        Bin::Mul => a.wrapping_mul(b),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tiny deterministic generator for the tests.
    fn next(s: &mut u64) -> u32 {
        *s = s
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (*s >> 32) as u32
    }

    const KINDS: [Bin; 11] = [
        Bin::Add,
        Bin::Sub,
        Bin::Sll,
        Bin::Slt,
        Bin::Sltu,
        Bin::Xor,
        Bin::Srl,
        Bin::Sra,
        Bin::Or,
        Bin::And,
        Bin::Mul,
    ];

    #[test]
    fn primary_and_every_alternative_match_the_concrete_semantics() {
        let mut seed = 7u64;
        for k in KINDS {
            let mut g = Aig::new();
            let a = BitVec::input(&mut g, 32);
            let b = BitVec::input(&mut g, 32);
            let p = primary(&mut g, k, &a, &b);
            let alts: Vec<BitVec> = (0..alt_count(k))
                .filter_map(|v| alternative(&mut g, k, v, &a, &b))
                .collect();
            assert_eq!(alts.len() as u32, alt_count(k));
            for round in 0..40 {
                let (x, y) = match round {
                    0 => (0, 0),
                    1 => (u32::MAX, 1),
                    2 => (0x8000_0000, 31),
                    3 => (0x8000_0000, 0x8000_0000),
                    _ => (next(&mut seed), next(&mut seed)),
                };
                let mut inputs = Vec::new();
                for i in 0..32 {
                    inputs.push(x >> i & 1 == 1);
                }
                for i in 0..32 {
                    inputs.push(y >> i & 1 == 1);
                }
                let want = concrete(k, x, y);
                assert_eq!(
                    eval_word(&g, &p, &inputs),
                    want,
                    "{k:?} primary {x:#x} {y:#x}"
                );
                for (v, alt) in alts.iter().enumerate() {
                    assert_eq!(
                        eval_word(&g, alt, &inputs),
                        want,
                        "{k:?} alt {v} {x:#x} {y:#x}"
                    );
                }
            }
        }
    }

    #[test]
    fn constant_operand_alternatives_are_distinct_circuits() {
        // `imm_alternative` names the alternative a constant operand may use; each must not
        // hash-cons into the primary.  (Add and sub alternative 0 do, and a constant shift's
        // always does: its barrel stages have constant selects and fold to the same wiring.)
        for (k, imm) in [
            (Bin::Add, 7u32),
            (Bin::Add, 0xFFFF_FFFF),
            (Bin::Xor, 1234),
            (Bin::And, 0xFF),
            (Bin::Or, 9),
            (Bin::Slt, 5),
            (Bin::Sltu, 5),
        ] {
            let v = imm_alternative(k).expect("these have one");
            let mut g = Aig::new();
            let a = BitVec::input(&mut g, 32);
            let c = BitVec::constant(32, imm as u128);
            let p = primary(&mut g, k, &a, &c);
            let alt = alternative(&mut g, k, v, &a, &c).unwrap();
            assert_ne!(p, alt, "{k:?} {imm:#x} alt {v}");
        }
        for k in [Bin::Sll, Bin::Srl, Bin::Sra] {
            assert_eq!(imm_alternative(k), None);
            let mut g = Aig::new();
            let a = BitVec::input(&mut g, 32);
            let c = BitVec::constant(32, 3);
            let p = primary(&mut g, k, &a, &c);
            let alt = alternative(&mut g, k, 0, &a, &c).unwrap();
            assert_eq!(p, alt, "{k:?} by a constant folds to the same wiring");
        }
    }

    #[test]
    fn an_alternative_is_a_different_circuit_from_the_primary() {
        for k in KINDS {
            let mut g = Aig::new();
            let a = BitVec::input(&mut g, 32);
            let b = BitVec::input(&mut g, 32);
            let p = primary(&mut g, k, &a, &b);
            for v in 0..alt_count(k) {
                let alt = alternative(&mut g, k, v, &a, &b).unwrap();
                assert_ne!(
                    p, alt,
                    "{k:?} alt {v} hash-consed into the primary: it would collapse"
                );
            }
        }
    }
}
