//! RFLAGS computation and condition-code evaluation.
//!
//! These mirror the rules in `07w-x86-64-simulator.md`: `add`/`sub` (and the
//! `cmp` that is a throw-away `sub`) compute CF/ZF/SF/OF/PF/AF, and the `jcc`/
//! `setcc`/`cmovcc` family read those flags through [`condition_holds`].
//!
//! All arithmetic is done at 64-bit width (the only width the backend uses for
//! register values).

use crate::state::Flags;

/// `dst + src` with full 64-bit flag computation. Returns `(result, flags)`.
pub fn add_with_flags(dst: u64, src: u64) -> (u64, Flags) {
    let (res, carry) = dst.overflowing_add(src);
    let mut f = Flags { cf: carry, ..Flags::default() };
    f.zf = res == 0;
    f.sf = (res >> 63) & 1 == 1;
    // Signed overflow: both operands same sign, result differs.
    f.of = ((dst ^ res) & (src ^ res)) >> 63 & 1 == 1;
    f.af = ((dst ^ src ^ res) >> 4) & 1 == 1;
    f.pf = parity(res);
    (res, f)
}

/// `dst - src` with full 64-bit flag computation (also used by `cmp`).
pub fn sub_with_flags(dst: u64, src: u64) -> (u64, Flags) {
    let (res, borrow) = dst.overflowing_sub(src);
    let mut f = Flags { cf: borrow, ..Flags::default() };
    f.zf = res == 0;
    f.sf = (res >> 63) & 1 == 1;
    // Signed overflow on subtract: operands differ in sign and result sign
    // differs from the minuend.
    f.of = ((dst ^ src) & (dst ^ res)) >> 63 & 1 == 1;
    f.af = ((dst ^ src ^ res) >> 4) & 1 == 1;
    f.pf = parity(res);
    (res, f)
}

/// Flags for a logical result (AND/OR/XOR/TEST): CF=OF=0, ZF/SF/PF from the value.
pub fn logic_flags(res: u64) -> Flags {
    Flags {
        cf: false,
        of: false,
        zf: res == 0,
        sf: (res >> 63) & 1 == 1,
        pf: parity(res),
        af: false,
    }
}

/// Even-parity of the low 8 bits (x86 PF is computed over the low byte only).
fn parity(v: u64) -> bool {
    (v as u8).count_ones() & 1 == 0
}

/// x86 condition codes (the `tttn` nibble of `Jcc`/`SETcc`/`CMOVcc`). The
/// numeric value matches the opcode's low nibble so the decoder can pass it
/// straight through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum Cond {
    O = 0x0, No = 0x1, B = 0x2, Ae = 0x3, E = 0x4, Ne = 0x5, Be = 0x6, A = 0x7,
    S = 0x8, Ns = 0x9, P = 0xA, Np = 0xB, L = 0xC, Ge = 0xD, Le = 0xE, G = 0xF,
}

impl Cond {
    /// Decode the `tttn` condition nibble.
    pub fn from_nibble(n: u8) -> Cond {
        match n & 0xF {
            0x0 => Cond::O, 0x1 => Cond::No, 0x2 => Cond::B, 0x3 => Cond::Ae,
            0x4 => Cond::E, 0x5 => Cond::Ne, 0x6 => Cond::Be, 0x7 => Cond::A,
            0x8 => Cond::S, 0x9 => Cond::Ns, 0xA => Cond::P, 0xB => Cond::Np,
            0xC => Cond::L, 0xD => Cond::Ge, 0xE => Cond::Le, _ => Cond::G,
        }
    }
}

/// Does the given condition hold under `f`? (The ARM ARM-style truth table for
/// x86 condition codes — see 07w §"Condition Codes".)
pub fn condition_holds(c: Cond, f: &Flags) -> bool {
    match c {
        Cond::O => f.of,
        Cond::No => !f.of,
        Cond::B => f.cf,                       // unsigned <
        Cond::Ae => !f.cf,                     // unsigned >=
        Cond::E => f.zf,                        // ==
        Cond::Ne => !f.zf,                      // !=
        Cond::Be => f.cf || f.zf,              // unsigned <=
        Cond::A => !f.cf && !f.zf,             // unsigned >
        Cond::S => f.sf,
        Cond::Ns => !f.sf,
        Cond::P => f.pf,
        Cond::Np => !f.pf,
        Cond::L => f.sf != f.of,               // signed <
        Cond::Ge => f.sf == f.of,              // signed >=
        Cond::Le => f.zf || (f.sf != f.of),    // signed <=
        Cond::G => !f.zf && (f.sf == f.of),    // signed >
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_sets_zero_and_carry() {
        let (r, f) = add_with_flags(u64::MAX, 1);
        assert_eq!(r, 0);
        assert!(f.zf && f.cf);
        assert!(!f.of); // -1 + 1 = 0, no signed overflow
    }

    #[test]
    fn sub_borrow_and_sign() {
        let (r, f) = sub_with_flags(0, 1);
        assert_eq!(r, u64::MAX);
        assert!(f.cf && f.sf && !f.zf);
    }

    #[test]
    fn signed_overflow_on_add() {
        // i64::MAX + 1 → signed overflow.
        let (_, f) = add_with_flags(i64::MAX as u64, 1);
        assert!(f.of && f.sf);
    }

    #[test]
    fn unsigned_and_signed_compares() {
        // cmp 3, 5  →  3 - 5: CF (unsigned <) and SF!=OF (signed <).
        let (_, f) = sub_with_flags(3, 5);
        assert!(condition_holds(Cond::B, &f));   // 3 <u 5
        assert!(condition_holds(Cond::L, &f));   // 3 <s 5
        assert!(!condition_holds(Cond::Ae, &f));
        assert!(condition_holds(Cond::Ne, &f));
    }

    #[test]
    fn negative_index_is_unsigned_above() {
        // The E5 bounds check: idx = -1 (huge unsigned) vs len = 3.
        let (_, f) = sub_with_flags((-1i64) as u64, 3);
        assert!(!condition_holds(Cond::B, &f)); // NOT unsigned-below → traps
    }
}
