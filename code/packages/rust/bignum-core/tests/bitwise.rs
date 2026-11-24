//! Tests for the Lean-`Nat`-compatible bitwise operations on `BigInteger` (KRN-002a).
//!
//! Strategy: every operation is compared against a deliberately naive reference that
//! works one bit at a time on a `Vec<bool>` (index 0 = least significant bit). The
//! reference shares no code with the limb-wise implementation: values cross between the
//! two worlds only through the binary string form (`parse_radix(.., 2)` /
//! `to_str_radix(.., 2)`), which is itself an independent, already-tested code path.

use bignum_core::{BigInteger, BitwiseError};

// ---------------------------------------------------------------------------
//  Naive bit-vector reference
// ---------------------------------------------------------------------------

type Bits = Vec<bool>;

fn trim(mut b: Bits) -> Bits {
    while b.last() == Some(&false) {
        b.pop();
    }
    b
}

fn to_bits(n: &BigInteger) -> Bits {
    let s = n.to_str_radix(2);
    if s == "0" {
        return Vec::new();
    }
    s.chars().rev().map(|c| c == '1').collect()
}

fn from_bits(b: &Bits) -> BigInteger {
    let b = trim(b.clone());
    if b.is_empty() {
        return BigInteger::zero();
    }
    let s: String = b.iter().rev().map(|&x| if x { '1' } else { '0' }).collect();
    BigInteger::parse_radix(&s, 2).unwrap()
}

fn zip(a: &Bits, b: &Bits, f: impl Fn(bool, bool) -> bool) -> Bits {
    let n = a.len().max(b.len());
    (0..n)
        .map(|i| f(*a.get(i).unwrap_or(&false), *b.get(i).unwrap_or(&false)))
        .collect()
}

fn ref_shl(a: &Bits, k: usize) -> Bits {
    if a.is_empty() {
        return Vec::new();
    }
    let mut out = vec![false; k];
    out.extend_from_slice(a);
    out
}

fn ref_shr(a: &Bits, k: usize) -> Bits {
    if k >= a.len() {
        Vec::new()
    } else {
        a[k..].to_vec()
    }
}

fn ref_log2(a: &Bits) -> u64 {
    let a = trim(a.clone());
    if a.is_empty() {
        0
    } else {
        a.len() as u64 - 1
    }
}

// ---------------------------------------------------------------------------
//  Deterministic value generation (no rand crate; reproducible forever)
// ---------------------------------------------------------------------------

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let x = (self.0 ^ (self.0 >> 31)).wrapping_mul(0x2545_F491_4F6C_DD1D);
        x ^ (x >> 29)
    }
    /// A random value with at most `max_bits` bits (and a random actual width, so short
    /// values are common).
    fn value(&mut self, max_bits: usize) -> BigInteger {
        let width = (self.next() as usize) % (max_bits + 1);
        let bits: Bits = (0..width).map(|_| self.next() & 1 == 1).collect();
        from_bits(&bits)
    }
}

fn pow2(k: u32) -> BigInteger {
    BigInteger::from_u64(2).pow(k)
}

/// Zero, one, and 2^k - 1, 2^k, 2^k + 1 around every limb and double-limb boundary.
fn edge_values() -> Vec<BigInteger> {
    let mut v = vec![BigInteger::zero(), BigInteger::one()];
    for &k in &[
        1u32, 2, 8, 31, 32, 33, 63, 64, 65, 95, 96, 97, 127, 128, 129, 200,
    ] {
        v.push(pow2(k).sub(&BigInteger::one()));
        v.push(pow2(k));
        v.push(pow2(k).add(&BigInteger::one()));
    }
    // A value with an all-ones low limb under a high bit, and an alternating pattern.
    v.push(from_bits(&(0..150).map(|i| i % 2 == 0).collect()));
    v.push(from_bits(&(0..150).map(|i| i % 2 == 1).collect()));
    v
}

fn all_samples() -> Vec<BigInteger> {
    let mut v = edge_values();
    let mut rng = Lcg(0xC0FFEE);
    for _ in 0..60 {
        v.push(rng.value(300));
    }
    // A few genuinely large values (thousands of bits).
    for _ in 0..4 {
        v.push(rng.value(4000));
    }
    v
}

// ---------------------------------------------------------------------------
//  Differential tests
// ---------------------------------------------------------------------------

#[test]
fn reference_round_trips() {
    // Guard the reference itself: bits -> value -> bits is the identity on trimmed bits.
    for n in all_samples() {
        assert_eq!(from_bits(&to_bits(&n)), n);
    }
}

#[test]
fn and_or_xor_match_reference_on_all_pairs() {
    let samples = all_samples();
    for a in &samples {
        for b in &samples {
            let (ba, bb) = (to_bits(a), to_bits(b));
            assert_eq!(
                a.and(b).unwrap(),
                from_bits(&zip(&ba, &bb, |x, y| x & y)),
                "and {a} {b}"
            );
            assert_eq!(
                a.or(b).unwrap(),
                from_bits(&zip(&ba, &bb, |x, y| x | y)),
                "or {a} {b}"
            );
            assert_eq!(
                a.xor(b).unwrap(),
                from_bits(&zip(&ba, &bb, |x, y| x ^ y)),
                "xor {a} {b}"
            );
        }
    }
}

#[test]
fn shifts_match_reference() {
    let amounts: [u64; 14] = [0, 1, 2, 7, 31, 32, 33, 63, 64, 65, 96, 100, 129, 5000];
    for a in &all_samples() {
        let ba = to_bits(a);
        for &k in &amounts {
            assert_eq!(
                a.shl(k).unwrap(),
                from_bits(&ref_shl(&ba, k as usize)),
                "shl {a} {k}"
            );
            assert_eq!(
                a.shr(k).unwrap(),
                from_bits(&ref_shr(&ba, k as usize)),
                "shr {a} {k}"
            );
        }
    }
}

#[test]
fn log2_matches_reference() {
    for a in &all_samples() {
        assert_eq!(a.log2().unwrap(), ref_log2(&to_bits(a)), "log2 {a}");
    }
}

// ---------------------------------------------------------------------------
//  Hand-checked unit tests (the Lean definitions, case by case)
// ---------------------------------------------------------------------------

#[test]
fn small_known_values() {
    let n = BigInteger::from_u64;
    assert_eq!(n(0b1100).and(&n(0b1010)).unwrap(), n(0b1000));
    assert_eq!(n(0b1100).or(&n(0b1010)).unwrap(), n(0b1110));
    assert_eq!(n(0b1100).xor(&n(0b1010)).unwrap(), n(0b0110));
    assert_eq!(n(1).shl(100).unwrap(), pow2(100));
    assert_eq!(pow2(100).shr(100).unwrap(), n(1));
    assert_eq!(pow2(100).shr(101).unwrap(), n(0));
}

#[test]
fn identities_and_zero() {
    let z = BigInteger::zero();
    for a in &all_samples() {
        assert_eq!(a.and(&z).unwrap(), z);
        assert_eq!(a.or(&z).unwrap(), *a);
        assert_eq!(a.xor(&z).unwrap(), *a);
        assert!(a.xor(a).unwrap().is_zero());
        assert_eq!(a.and(a).unwrap(), *a);
        assert_eq!(a.or(a).unwrap(), *a);
        assert_eq!(a.shl(0).unwrap(), *a);
        assert_eq!(a.shr(0).unwrap(), *a);
        // (a << k) >> k == a, and a << k == a * 2^k
        assert_eq!(a.shl(77).unwrap().shr(77).unwrap(), *a);
        assert_eq!(a.shl(77).unwrap(), a.mul(&pow2(77)));
        // a >> k == floor(a / 2^k)
        assert_eq!(a.shr(45).unwrap(), a.div(&pow2(45)));
    }
}

#[test]
fn results_are_normalized() {
    // XOR of equal high limbs and AND of disjoint high limbs must not leave a trailing
    // zero limb; equality (derived from the representation) proves normalization.
    let hi = pow2(200).add(&BigInteger::one());
    let lo = BigInteger::one();
    assert_eq!(hi.xor(&pow2(200)).unwrap(), lo);
    assert_eq!(hi.and(&lo).unwrap(), lo);
    assert_eq!(hi.and(&pow2(200)).unwrap(), pow2(200));
    assert_eq!(pow2(64).shr(64).unwrap().num_limbs(), 1);
    assert_eq!(pow2(64).shr(33).unwrap(), pow2(31));
    assert_eq!(pow2(32).shl(0).unwrap().num_limbs(), 2);
}

#[test]
fn log2_values() {
    let n = BigInteger::from_u64;
    // Lean: Nat.log2 0 = 0 (defined, not an error).
    assert_eq!(n(0).log2().unwrap(), 0);
    assert_eq!(n(1).log2().unwrap(), 0);
    assert_eq!(n(2).log2().unwrap(), 1);
    assert_eq!(n(3).log2().unwrap(), 1);
    assert_eq!(n(255).log2().unwrap(), 7);
    assert_eq!(n(256).log2().unwrap(), 8);
    assert_eq!(pow2(1000).log2().unwrap(), 1000);
    assert_eq!(pow2(1000).sub(&n(1)).log2().unwrap(), 999);
}

#[test]
fn huge_right_shift_is_zero_without_allocating() {
    assert!(pow2(200).shr(u64::MAX).unwrap().is_zero());
    assert!(BigInteger::zero().shr(u64::MAX).unwrap().is_zero());
}

// ---------------------------------------------------------------------------
//  Refusals
// ---------------------------------------------------------------------------

#[test]
fn negative_operands_are_refused() {
    let neg = BigInteger::from_i64(-5);
    let pos = BigInteger::from_u64(5);
    let e = Err(BitwiseError::NegativeOperand);
    assert_eq!(neg.and(&pos), e);
    assert_eq!(pos.and(&neg), e);
    assert_eq!(neg.or(&pos), e);
    assert_eq!(pos.or(&neg), e);
    assert_eq!(neg.xor(&pos), e);
    assert_eq!(pos.xor(&neg), e);
    assert_eq!(neg.shl(1), e);
    assert_eq!(neg.shr(1), e);
    assert_eq!(neg.try_shl(1, 100), e);
    assert_eq!(neg.log2(), Err(BitwiseError::NegativeOperand));
}

#[test]
fn bounded_shift_checks_projected_size() {
    let a = BigInteger::from_u64(3); // 2 bits
    assert_eq!(a.try_shl(10, 12).unwrap(), a.shl(10).unwrap());
    assert_eq!(
        a.try_shl(11, 12),
        Err(BitwiseError::ResultTooLarge {
            projected_bits: 13,
            max_bits: 12
        })
    );
    // An absurd shift is refused up front, in O(1), without allocating.
    assert!(matches!(
        a.try_shl(u64::MAX, 1 << 20),
        Err(BitwiseError::ResultTooLarge { .. })
    ));
    // Zero shifted by anything is zero, even past the ceiling.
    assert!(BigInteger::zero().try_shl(u64::MAX, 1).unwrap().is_zero());
}

#[test]
fn error_display_is_informative() {
    assert!(BitwiseError::NegativeOperand
        .to_string()
        .contains("non-negative"));
    let e = BitwiseError::ResultTooLarge {
        projected_bits: 13,
        max_bits: 12,
    };
    let s = e.to_string();
    assert!(s.contains("13") && s.contains("12"));
}

#[test]
fn unrepresentable_shift_is_a_typed_error_without_allocating() {
    // u64::MAX bits is ~2^59 limbs: never allocatable. On 64-bit targets the guard that
    // fires is try_shl's ceiling; on 32-bit it is shl's own usize conversion check.
    // Either way: a typed error, no allocation, no wrap.
    let a = BigInteger::from_u64(3);
    assert!(matches!(
        a.try_shl(u64::MAX, u64::MAX / 2),
        Err(BitwiseError::ResultTooLarge { .. })
    ));
    if usize::BITS == 32 {
        assert!(matches!(
            a.shl(u64::MAX),
            Err(BitwiseError::ResultTooLarge { .. })
        ));
        assert!(matches!(
            a.shl(1u64 << 37),
            Err(BitwiseError::ResultTooLarge { .. })
        ));
    }
    // zero never allocates whatever the shift
    assert!(BigInteger::zero().shl(u64::MAX).unwrap().is_zero());
}
