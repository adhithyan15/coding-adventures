//! # Fixed-width integer semantics (VER04j WX-B and WX-C)
//!
//! Warrant v1 adds eight fixed-width integer types, `U8 U16 U32 U64` and
//! `I8 I16 I32 I64`, and a family of total functions over them.  This module is
//! the single, host-independent definition of what those functions *mean*; the
//! checker uses it for range tests (`widen`), the ground evaluator uses it for
//! every example, and the tests use it as the model that native Rust is held
//! against.  (The generated evidence harness carries its own copy in
//! `evidence_prelude.rs.txt`, because it must compile inside someone else's
//! crate; the differential tests hold the two copies together.)
//!
//! ## Canonical representation
//!
//! A value of a fixed-width type is stored as the *mathematical integer* it
//! denotes, in an `i128`.  `U64` (max 2^64 - 1) and `I64` (min -2^63) both fit.
//! The stored value is always *canonical*, meaning inside the type's range, so
//! equality of values is plain integer equality.
//!
//! Bitwise functions work on the **bit pattern**: the two's-complement
//! encoding of the value, masked to `w` bits and held in a `u128`.
//!
//! ```text
//!   I8 value  -1   --pattern-->  0xff  (8 one bits)
//!   I8 value  -128 --pattern-->  0x80
//!   U8 value  255  --pattern-->  0xff   (the same pattern as -1i8)
//!   pattern 0xff --canon as I8--> -1     --canon as U8--> 255
//! ```
//!
//! ## Wrapping arithmetic
//!
//! `+ - *` compute the exact result modulo 2^w and then re-read it in the
//! type's range.  Doing the arithmetic on *patterns* in `u128` gives that for
//! free, including for 64-bit multiplication, whose exact product of two `U64`
//! values (up to 2^128 - 2^65 + 1) does not fit an `i128`:
//!
//! ```text
//!   U8:  200 + 100 = 300  -> 300 mod 256 = 44
//!   I8:  100 + 100 = 200  -> pattern 0xc8 -> read signed -> -56
//!   I8: -128 - 1   = -129 -> pattern 0x7f -> 127
//! ```
//!
//! ## Total shifts and rotates
//!
//! Shift counts are mathematical naturals, so `n >= w` is a well-defined input
//! and the result never depends on the host.  `shl(1u8, 8) == 0`, whereas x86
//! would mask the count and give `1`, and C leaves it undefined.

/// One of the eight fixed-width integer types of Warrant v1.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FixedTy {
    U8,
    U16,
    U32,
    U64,
    I8,
    I16,
    I32,
    I64,
}

impl FixedTy {
    /// All eight types, unsigned first, in increasing width.
    pub const ALL: [FixedTy; 8] = [
        FixedTy::U8,
        FixedTy::U16,
        FixedTy::U32,
        FixedTy::U64,
        FixedTy::I8,
        FixedTy::I16,
        FixedTy::I32,
        FixedTy::I64,
    ];

    /// The Warrant spelling of the type (`"U32"`).
    pub fn name(self) -> &'static str {
        match self {
            FixedTy::U8 => "U8",
            FixedTy::U16 => "U16",
            FixedTy::U32 => "U32",
            FixedTy::U64 => "U64",
            FixedTy::I8 => "I8",
            FixedTy::I16 => "I16",
            FixedTy::I32 => "I32",
            FixedTy::I64 => "I64",
        }
    }

    /// Parse the Warrant spelling of a type name.
    pub fn from_name(name: &str) -> Option<FixedTy> {
        FixedTy::ALL.into_iter().find(|t| t.name() == name)
    }

    /// The literal suffix (`"u32"`), also the Rust spelling of the native type.
    pub fn suffix(self) -> &'static str {
        match self {
            FixedTy::U8 => "u8",
            FixedTy::U16 => "u16",
            FixedTy::U32 => "u32",
            FixedTy::U64 => "u64",
            FixedTy::I8 => "i8",
            FixedTy::I16 => "i16",
            FixedTy::I32 => "i32",
            FixedTy::I64 => "i64",
        }
    }

    /// Parse a literal suffix.
    pub fn from_suffix(suffix: &str) -> Option<FixedTy> {
        FixedTy::ALL.into_iter().find(|t| t.suffix() == suffix)
    }

    /// Width in bits.
    pub fn bits(self) -> u32 {
        match self {
            FixedTy::U8 | FixedTy::I8 => 8,
            FixedTy::U16 | FixedTy::I16 => 16,
            FixedTy::U32 | FixedTy::I32 => 32,
            FixedTy::U64 | FixedTy::I64 => 64,
        }
    }

    /// Width in bytes (`bits / 8`), the `N` of `Bytes<N>` for endian conversion.
    pub fn bytes(self) -> u64 {
        u64::from(self.bits() / 8)
    }

    /// Is the type signed (`I8 .. I64`)?
    pub fn signed(self) -> bool {
        matches!(
            self,
            FixedTy::I8 | FixedTy::I16 | FixedTy::I32 | FixedTy::I64
        )
    }

    /// The smallest value of the type.
    pub fn min(self) -> i128 {
        if self.signed() {
            -(1i128 << (self.bits() - 1))
        } else {
            0
        }
    }

    /// The largest value of the type.
    pub fn max(self) -> i128 {
        if self.signed() {
            (1i128 << (self.bits() - 1)) - 1
        } else {
            (1i128 << self.bits()) - 1
        }
    }

    /// Is the mathematical value `v` inside the type's range?
    pub fn contains(self, v: i128) -> bool {
        self.min() <= v && v <= self.max()
    }

    /// Is every value of `self` also a value of `other`?  This is the
    /// `widen<other>(x: self)` admission rule: `U8 -> U32` yes, `I8 -> U32` no
    /// (negative values), `U32 -> I32` no (values above `i32::MAX`).
    pub fn widens_to(self, other: FixedTy) -> bool {
        other.min() <= self.min() && self.max() <= other.max()
    }

    /// The mask of the low `w` bits.
    pub fn mask(self) -> u128 {
        (1u128 << self.bits()) - 1
    }

    /// The two's-complement bit pattern of `v`, masked to `w` bits.  Works for
    /// any mathematical `v`, in or out of range, which is exactly `wrap`.
    pub fn pattern(self, v: i128) -> u128 {
        (v as u128) & self.mask()
    }

    /// Read a bit pattern (already masked, or not: it is masked here) back as a
    /// canonical value of the type.
    pub fn canon(self, pattern: u128) -> i128 {
        let p = pattern & self.mask();
        if self.signed() && p >> (self.bits() - 1) == 1 {
            (p as i128) - (1i128 << self.bits())
        } else {
            p as i128
        }
    }

    /// `wrap<T>(v)`: the mathematical value reduced modulo 2^w and read in the
    /// type's range.  `wrap<I8>(255) == -1`, `wrap<U8>(-1) == 255`.
    pub fn wrap(self, v: i128) -> i128 {
        self.canon(self.pattern(v))
    }

    /// `try<T>(v)`: `Some(v)` exactly when `v` is in range.
    pub fn try_from_int(self, v: i128) -> Option<i128> {
        self.contains(v).then_some(v)
    }
}

// ---------------------------------------------------------------------------
// WX-B: arithmetic
// ---------------------------------------------------------------------------

/// Wrapping `a + b`.
pub fn add(ty: FixedTy, a: i128, b: i128) -> i128 {
    ty.canon(ty.pattern(a).wrapping_add(ty.pattern(b)))
}

/// Wrapping `a - b`.
pub fn sub(ty: FixedTy, a: i128, b: i128) -> i128 {
    ty.canon(ty.pattern(a).wrapping_sub(ty.pattern(b)))
}

/// Wrapping `a * b` (pattern product, so `U64 * U64` never overflows the host).
pub fn mul(ty: FixedTy, a: i128, b: i128) -> i128 {
    ty.canon(ty.pattern(a).wrapping_mul(ty.pattern(b)))
}

/// Wrapping negation of a **signed** value; `-min == min`.
///
/// ```text
///   -(-128i8) = 128 -> pattern 0x80 -> -128
/// ```
pub fn neg(ty: FixedTy, a: i128) -> i128 {
    ty.canon(ty.pattern(a).wrapping_neg())
}

/// `checked_add` etc.: `None` iff the exact result leaves the range.
pub fn checked(op: char, ty: FixedTy, a: i128, b: i128) -> Option<i128> {
    let exact = match op {
        '+' => a.checked_add(b),
        '-' => a.checked_sub(b),
        _ => a.checked_mul(b),
    }?;
    ty.try_from_int(exact)
}

/// `div(a, b)` on unsigned values: `None` iff `b == 0`.
pub fn div_unsigned(a: i128, b: i128) -> Option<i128> {
    (b != 0).then(|| a / b)
}

/// `rem(a, b)` on unsigned values: `None` iff `b == 0`.
pub fn rem_unsigned(a: i128, b: i128) -> Option<i128> {
    (b != 0).then(|| a % b)
}

/// `div_trunc` on signed values, truncating toward zero.  `None` iff `b == 0`
/// or the one overflowing case `min / -1`.
pub fn div_trunc(ty: FixedTy, a: i128, b: i128) -> Option<i128> {
    if b == 0 || (b == -1 && a == ty.min()) {
        None
    } else {
        Some(a / b)
    }
}

/// `rem_trunc` on signed values (sign of the dividend).  `None` iff `b == 0`
/// or `min / -1` (checked-style, as the specification fixes).
pub fn rem_trunc(ty: FixedTy, a: i128, b: i128) -> Option<i128> {
    if b == 0 || (b == -1 && a == ty.min()) {
        None
    } else {
        Some(a % b)
    }
}

/// Floor division of mathematical integers (`div` on `Int`/`Nat`).
pub fn floor_div(a: i128, b: i128) -> Option<i128> {
    if b == 0 || (a == i128::MIN && b == -1) {
        return None;
    }
    let q = a / b;
    Some(if a % b != 0 && ((a < 0) != (b < 0)) {
        q - 1
    } else {
        q
    })
}

/// Floor modulo of mathematical integers (`rem` on `Int`/`Nat`): the result
/// has the sign of the divisor, so it is non-negative for `b > 0`.
pub fn floor_mod(a: i128, b: i128) -> Option<i128> {
    if b == 0 {
        return None;
    }
    if b == -1 {
        return Some(0);
    }
    let r = a % b;
    Some(if r != 0 && ((r < 0) != (b < 0)) {
        r + b
    } else {
        r
    })
}

// ---------------------------------------------------------------------------
// WX-C: bitwise operations
// ---------------------------------------------------------------------------
//
// Truth table for the per-bit functions (one bit of each operand):
//
//     a b | and or xor      a | not
//     0 0 |  0   0   0      0 |  1
//     0 1 |  0   1   1      1 |  0
//     1 0 |  0   1   1
//     1 1 |  1   1   0

/// `bit_and(a, b)`.
pub fn bit_and(ty: FixedTy, a: i128, b: i128) -> i128 {
    ty.canon(ty.pattern(a) & ty.pattern(b))
}

/// `bit_or(a, b)`.
pub fn bit_or(ty: FixedTy, a: i128, b: i128) -> i128 {
    ty.canon(ty.pattern(a) | ty.pattern(b))
}

/// `bit_xor(a, b)`.
pub fn bit_xor(ty: FixedTy, a: i128, b: i128) -> i128 {
    ty.canon(ty.pattern(a) ^ ty.pattern(b))
}

/// `bit_not(a)`: flip all `w` bits.  `bit_not(0u8) == 255`, `bit_not(0i8) == -1`.
pub fn bit_not(ty: FixedTy, a: i128) -> i128 {
    ty.canon(!ty.pattern(a))
}

/// `shl(a, n)`: shift left, zero fill, high bits dropped; `n >= w` gives `0`.
///
/// ```text
///   shl(0x81u8, 1) = 0x02      shl(1u8, 7) = 128      shl(1u8, 8) = 0
/// ```
pub fn shl(ty: FixedTy, a: i128, n: u128) -> i128 {
    if n >= u128::from(ty.bits()) {
        0
    } else {
        ty.canon(ty.pattern(a) << n)
    }
}

/// `shr(a, n)`: *logical* right shift of the pattern, zero fill; `n >= w` gives
/// `0`.  On a signed type the pattern is shifted and then re-read signed:
/// `shr(-1i8, 1) == 127`.
pub fn shr(ty: FixedTy, a: i128, n: u128) -> i128 {
    if n >= u128::from(ty.bits()) {
        0
    } else {
        ty.canon(ty.pattern(a) >> n)
    }
}

/// `sar(a, n)`: *arithmetic* right shift (sign fill) of a signed value;
/// `n >= w` gives `-1` for a negative `a` and `0` otherwise.
///
/// ```text
///   sar(-128i8, 1) = -64      sar(-128i8, 8) = -1      sar(100i8, 9) = 0
/// ```
pub fn sar(ty: FixedTy, a: i128, n: u128) -> i128 {
    if n >= u128::from(ty.bits()) {
        if a < 0 {
            -1
        } else {
            0
        }
    } else {
        a >> n
    }
}

/// `rotl(a, n)`: rotate left by `n mod w`.  `rotl(a, w) == a`.
///
/// ```text
///   rotl(0x81u8, 1) = 0x03      rotl(0x81u8, 9) = 0x03
/// ```
pub fn rotl(ty: FixedTy, a: i128, n: u128) -> i128 {
    let w = u128::from(ty.bits());
    let k = n % w;
    let p = ty.pattern(a);
    if k == 0 {
        return a;
    }
    ty.canon((p << k) | (p >> (w - k)))
}

/// `rotr(a, n)`: rotate right by `n mod w`; `rotr(a, n) == rotl(a, (w - n mod w) mod w)`.
///
/// ```text
///   rotr(0x03u8, 1) = 0x81
/// ```
pub fn rotr(ty: FixedTy, a: i128, n: u128) -> i128 {
    let w = u128::from(ty.bits());
    rotl(ty, a, (w - n % w) % w)
}

/// `popcount(a)`: number of one bits of the width-`w` pattern.
/// `popcount(-1i8) == 8` (not the sign-extended host word).
pub fn popcount(ty: FixedTy, a: i128) -> u128 {
    u128::from(ty.pattern(a).count_ones())
}

/// `clz(a)`: leading zero bits of the pattern; `w` for zero.
pub fn clz(ty: FixedTy, a: i128) -> u128 {
    let p = ty.pattern(a);
    if p == 0 {
        u128::from(ty.bits())
    } else {
        u128::from(p.leading_zeros() - (128 - ty.bits()))
    }
}

/// `ctz(a)`: trailing zero bits of the pattern; `w` for zero.
pub fn ctz(ty: FixedTy, a: i128) -> u128 {
    let p = ty.pattern(a);
    if p == 0 {
        u128::from(ty.bits())
    } else {
        u128::from(p.trailing_zeros())
    }
}

/// `bit(a, i)`: bit `i` (LSB = 0); `false` for `i >= w`.
pub fn bit(ty: FixedTy, a: i128, i: u128) -> bool {
    i < u128::from(ty.bits()) && (ty.pattern(a) >> i) & 1 == 1
}

/// `be_bytes(x)`: big-endian two's-complement bytes (`be_bytes(0x01020304u32)`
/// is `01 02 03 04`).
pub fn be_bytes(ty: FixedTy, a: i128) -> Vec<u8> {
    let n = ty.bytes() as usize;
    let p = ty.pattern(a);
    (0..n).rev().map(|i| (p >> (8 * i)) as u8).collect()
}

/// `le_bytes(x)`: little-endian two's-complement bytes (`le_bytes(-2i16)` is
/// `fe ff`).
pub fn le_bytes(ty: FixedTy, a: i128) -> Vec<u8> {
    let mut v = be_bytes(ty, a);
    v.reverse();
    v
}

/// `from_be<T>(b)`: the inverse of [`be_bytes`].  `bytes.len()` must be
/// `ty.bytes()` (the checker guarantees it; `None` otherwise).
pub fn from_be(ty: FixedTy, bytes: &[u8]) -> Option<i128> {
    if bytes.len() as u64 != ty.bytes() {
        return None;
    }
    let p = bytes
        .iter()
        .fold(0u128, |acc, b| (acc << 8) | u128::from(*b));
    Some(ty.canon(p))
}

/// `from_le<T>(b)`: the inverse of [`le_bytes`].
pub fn from_le(ty: FixedTy, bytes: &[u8]) -> Option<i128> {
    let rev: Vec<u8> = bytes.iter().rev().copied().collect();
    from_be(ty, &rev)
}

// ---------------------------------------------------------------------------
// Evidence domains (VER04j section 2.6)
// ---------------------------------------------------------------------------

/// The *edge* domain of a fixed-width type: the biased boundary set the
/// evidence generator enumerates where the full range is too large, in the
/// fixed order that keeps generated output byte-stable.
///
/// Unsigned width `w`: `0..=255` (clipped to the range), `1 << i` for
/// `8 <= i < w`, `MAX`, then `MAX ^ (1 << i)` for `0 <= i < w`.  For `U64`
/// that is exactly the 377 values of VER04h section 3; `U16` has
/// 256 + 8 + 1 + 16 = 281 and `U32` 256 + 24 + 1 + 32 = 313.
///
/// Signed width `w`: those patterns read as signed, together with
/// `-1 ..= -255` (clipped), deduplicated and in ascending numeric order
/// (`I16` 527 values, `I32` 559, `I64` 623).
pub fn edge_domain(ty: FixedTy) -> Vec<i128> {
    let w = ty.bits();
    let umax = ty.mask();
    let mut u: Vec<u128> = (0..=255u128).filter(|v| *v <= umax).collect();
    u.extend((8..w).map(|i| 1u128 << i));
    u.push(umax);
    u.extend((0..w).map(|i| umax ^ (1u128 << i)));
    if !ty.signed() {
        return u.into_iter().map(|p| p as i128).collect();
    }
    let mut s: Vec<i128> = u.into_iter().map(|p| ty.canon(p)).collect();
    s.extend((1..=255i128).map(|k| -k).filter(|v| ty.contains(*v)));
    s.sort_unstable();
    s.dedup();
    s
}

/// Every value of a type, ascending (only sensible for the narrow types
/// `U8 I8 U16 I16`, whose full domain the generator may enumerate).
pub fn full_domain(ty: FixedTy) -> Vec<i128> {
    (ty.min()..=ty.max()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_match_native_limits() {
        assert_eq!(FixedTy::U8.max(), u8::MAX as i128);
        assert_eq!(FixedTy::U16.max(), u16::MAX as i128);
        assert_eq!(FixedTy::U32.max(), u32::MAX as i128);
        assert_eq!(FixedTy::U64.max(), u64::MAX as i128);
        assert_eq!(FixedTy::I8.min(), i8::MIN as i128);
        assert_eq!(FixedTy::I16.max(), i16::MAX as i128);
        assert_eq!(FixedTy::I32.min(), i32::MIN as i128);
        assert_eq!(FixedTy::I64.min(), i64::MIN as i128);
        assert_eq!(FixedTy::I64.max(), i64::MAX as i128);
        for t in FixedTy::ALL {
            assert_eq!(FixedTy::from_name(t.name()), Some(t));
            assert_eq!(FixedTy::from_suffix(t.suffix()), Some(t));
            assert_eq!(t.bytes() * 8, u64::from(t.bits()));
        }
        assert_eq!(FixedTy::from_name("U128"), None);
        assert_eq!(FixedTy::from_suffix("u128"), None);
    }

    #[test]
    fn widening_is_range_containment() {
        assert!(FixedTy::U8.widens_to(FixedTy::U32));
        assert!(FixedTy::U8.widens_to(FixedTy::I16));
        assert!(!FixedTy::U8.widens_to(FixedTy::I8));
        assert!(!FixedTy::I8.widens_to(FixedTy::U32));
        assert!(!FixedTy::U32.widens_to(FixedTy::I32));
        assert!(FixedTy::U32.widens_to(FixedTy::I64));
        assert!(!FixedTy::U64.widens_to(FixedTy::I64));
        assert!(FixedTy::I8.widens_to(FixedTy::I64));
    }

    #[test]
    fn specification_truth_tables_for_width_eight() {
        use FixedTy::*;
        assert_eq!(shl(U8, 0x81, 1), 0x02);
        assert_eq!(shl(U8, 1, 7), 128);
        assert_eq!(shl(U8, 1, 8), 0);
        assert_eq!(shr(U8, 0x81, 1), 0x40);
        assert_eq!(shr(I8, -1, 1), 127);
        assert_eq!(sar(I8, -128, 1), -64);
        assert_eq!(sar(I8, -128, 8), -1);
        assert_eq!(sar(I8, 100, 9), 0);
        assert_eq!(rotl(U8, 0x81, 1), 0x03);
        assert_eq!(rotl(U8, 0x81, 9), 0x03);
        assert_eq!(rotr(U8, 0x03, 1), 0x81);
        assert_eq!(popcount(U8, 0xff), 8);
        assert_eq!(popcount(I8, -1), 8);
        assert_eq!(clz(U8, 0), 8);
        assert_eq!(ctz(U8, 0x80), 7);
        assert!(!bit(U8, 5, 8));
        assert!(bit(U8, 5, 2));
        assert_eq!(bit_not(U8, 0), 255);
        assert_eq!(bit_not(I8, 0), -1);
        assert_eq!(be_bytes(U32, 0x0102_0304), vec![1, 2, 3, 4]);
        assert_eq!(le_bytes(I16, -2), vec![0xfe, 0xff]);
        assert_eq!(from_be(U32, &[1, 2, 3, 4]), Some(0x0102_0304));
        assert_eq!(from_le(I16, &[0xfe, 0xff]), Some(-2));
        assert_eq!(from_be(U32, &[1, 2]), None);
    }

    #[test]
    fn arithmetic_edge_cases() {
        use FixedTy::*;
        assert_eq!(add(U8, 200, 100), 44);
        assert_eq!(add(I8, 100, 100), -56);
        assert_eq!(sub(I8, -128, 1), 127);
        assert_eq!(neg(I8, -128), -128);
        assert_eq!(mul(U64, u64::MAX as i128, u64::MAX as i128), 1);
        assert_eq!(neg(I64, i64::MIN as i128), i64::MIN as i128);
        assert_eq!(checked('+', U8, 200, 100), None);
        assert_eq!(checked('*', U64, u64::MAX as i128, u64::MAX as i128), None);
        assert_eq!(checked('-', U8, 0, 1), None);
        assert_eq!(checked('-', I8, 0, 1), Some(-1));
        assert_eq!(div_unsigned(7, 0), None);
        assert_eq!(rem_unsigned(7, 2), Some(1));
        assert_eq!(div_trunc(I8, -128, -1), None);
        assert_eq!(rem_trunc(I8, -128, -1), None);
        assert_eq!(div_trunc(I8, -7, 2), Some(-3));
        assert_eq!(rem_trunc(I8, -7, 2), Some(-1));
        assert_eq!(floor_div(-7, 2), Some(-4));
        assert_eq!(floor_div(7, -2), Some(-4));
        assert_eq!(floor_mod(-7, 2), Some(1));
        assert_eq!(floor_mod(7, -2), Some(-1));
        assert_eq!(floor_mod(5, 0), None);
        assert_eq!(floor_mod(i128::MIN, -1), Some(0));
        assert_eq!(floor_div(i128::MIN, -1), None);
        assert_eq!(U8.wrap(-1), 255);
        assert_eq!(I8.wrap(255), -1);
        assert_eq!(U8.try_from_int(256), None);
        assert_eq!(I8.try_from_int(-129), None);
        assert_eq!(I8.try_from_int(-128), Some(-128));
    }
}
