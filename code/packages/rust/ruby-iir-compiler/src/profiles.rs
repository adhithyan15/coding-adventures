//! # Release profiles (LANG84 "Release profiles")
//!
//! Two Ruby releases can parse a program into the *same* tree and still mean
//! different things by it. The tree says "divide these two integers"; only
//! the release knows whether that division rounds toward negative infinity
//! or toward zero. LANG84 puts every such difference in one place, a
//! [`ReleaseProfile`]: a plain struct of semantic switches. Each row of
//! `code/grammars/ruby/releases.csv` names its profile in the `profile`
//! column, and many releases share one.
//!
//! The compiler may vary by release in exactly two ways (LANG84 C2): by the
//! *shape* of the tree the release's grammar produced, and by the fields of
//! this struct. It never looks at a release ID's digits. That is why this
//! file maps IDs to profiles with a flat `match` and never with a comparison
//! such as `release >= "1.9"`: Ruby's inventory interleaves maintenance
//! lines (1.8.7-p374 shipped after 2.0.0-p0), so "greater than" is not even a
//! well-defined question for release IDs.
//!
//! ## Which releases have a profile
//!
//! A release gets a profile when its row claims an IIR subset
//! (`iir_status` is not `-`); `release_inventory.py check` enforces "profile
//! is `-` exactly when `iir_status` is `-`", and the `profiles_match_the_inventory`
//! test in `tests/release_subsets.rs` pins this table against the CSV.
//!
//! | Release(s)              | Profile   | Why                                   |
//! |-------------------------|-----------|---------------------------------------|
//! | 3.3.5, 3.3.6, 3.3.7     | `ruby33`  | checked against host ruby 3.3.6 and the tag sources below |
//! | 0.49, 0.50              | none      | `puts` is not callable bare (see below) |
//! | every other row         | none      | no grammar pair yet                   |
//!
//! ## The 3.3 line (`ruby33`)
//!
//! Evidence (tag `v3_3_5`; `v3_3_6` and `v3_3_7` carry byte-identical files):
//!
//! - `numeric.c` (blob `03b336d9b720f128cf2c9d97f81e368521538b68`):
//!   `fix_divide` and `fix_mod` call `rb_num_zerodiv()` on a zero divisor
//!   (lines 4167 and 4271) and otherwise use the floor helpers
//!   `rb_fix_div_fix` / `rb_fix_mod_fix`, so `-7 / 2 == -4` and
//!   `-7 % 2 == 1`. A result outside the machine word becomes a Bignum.
//! - `io.c` (blob `9d651e016a99fa4accee1ba69f230cb604ef80c7`): line 15309
//!   `rb_define_global_function("puts", rb_f_puts, -1)` — `puts` is a
//!   private Kernel method, callable with no receiver.
//!
//! ## Why 0.49 and 0.50 have no profile
//!
//! Their grammars parse `puts(1 + 2)` (LANG84 R0, stage P), but the program
//! does not *run* there:
//!
//! - `io.c` at `v0_49` defines `puts` only as an IO method
//!   (`rb_define_method(C_IO, "puts", Fio_puts, 1)`, line 1196; `v0_50`
//!   line 1258). A bare `puts(...)` at top level has the main object as its
//!   receiver, which is not an IO, so it raises a NameError.
//! - `numeric.c` at `v0_49` (blob `34d82134b4b53924e8b723ea49e6fb2550762678`)
//!   divides fixnums with C's `/` and `%` on `int` (`Ffix_div`, `Ffix_mod`),
//!   which truncate toward zero: `-7 / 2 == -3` there.
//!
//! So the rb-int-puts subset is not part of those releases' language, and
//! [`profile_for`] answers `None`; the compiler turns that into a typed
//! `ReleaseNotAvailable` error instead of compiling 3.3 semantics under a
//! 1994 label. The truncating division is still recorded as an
//! [`IntDiv::Truncate`] switch so that the first early release that does get
//! a profile has a field to set (and the compiler refuses it until LANG83's
//! `i64_div_trunc` leaves `Later`).

/// How `Integer#/` and `Integer#%` round a quotient that is not exact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntDiv {
    /// Toward negative infinity, remainder takes the divisor's sign
    /// (Ruby 3.3, Python `//`). Lowers to LANG83 `i64_div_floor` /
    /// `i64_mod_floor`.
    ///
    /// | a  | b  | a / b | a % b |
    /// |----|----|-------|-------|
    /// |  7 |  2 |   3   |   1   |
    /// | -7 |  2 |  -4   |   1   |
    /// |  7 | -2 |  -4   |  -1   |
    /// | -7 | -2 |   3   |  -1   |
    Floor,
    /// Toward zero, remainder takes the dividend's sign (C `/` and `%`, as in
    /// Ruby 0.49's `Ffix_div`). No profile uses it yet; LANG83's
    /// `i64_div_trunc` is still `Later`, so the compiler refuses it.
    Truncate,
}

/// The semantic switches one release (or many) compiles under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReleaseProfile {
    /// The name the `profile` column of `releases.csv` uses.
    pub name: &'static str,
    /// Rounding of integer `/` and `%`.
    pub int_div: IntDiv,
}

/// The Ruby 3.3 line: floor division, Kernel `puts`.
pub const RUBY33: ReleaseProfile = ReleaseProfile {
    name: "ruby33",
    int_div: IntDiv::Floor,
};

/// Every profile this crate defines, for tooling and tests.
pub const PROFILES: &[ReleaseProfile] = &[RUBY33];

/// The profile an inventory release compiles under, or `None` when the
/// release claims no IIR subset (see the module docs for why 0.49 and 0.50
/// do not). Exact IDs only; nothing here orders or compares versions.
pub fn profile_for(release: &str) -> Option<&'static ReleaseProfile> {
    match release {
        "3.3.5" | "3.3.6" | "3.3.7" => Some(&RUBY33),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_33_line_shares_one_profile_and_1994_has_none() {
        for release in ["3.3.5", "3.3.6", "3.3.7"] {
            assert_eq!(profile_for(release), Some(&RUBY33), "{release}");
        }
        for release in ["0.49", "0.50", "3.3.4", "3.3", "", "ruby33"] {
            assert_eq!(profile_for(release), None, "{release:?}");
        }
        assert_eq!(RUBY33.int_div, IntDiv::Floor);
        assert!(PROFILES.iter().all(|p| !p.name.is_empty()));
    }
}
