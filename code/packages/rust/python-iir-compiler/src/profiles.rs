//! # Release profiles (LANG84 "Release profiles")
//!
//! Two CPython releases can parse a program into the *same* tree and still
//! mean different things by it. The tree says "divide these two integers";
//! only the release knows whether `/` floors (Python 2's classic division on
//! ints) or produces a float (Python 3's true division, PEP 238). LANG84 puts
//! every such difference in one place, a [`ReleaseProfile`]: a plain struct
//! of semantic switches. Each row of `code/grammars/python/releases.csv` names
//! its profile in the `profile` column, and many releases share one.
//!
//! The compiler may vary by release in exactly two ways (LANG84 C2): by the
//! *shape* of the tree the release's grammar produced, and by the fields of
//! this struct. It never looks at a release ID's digits. That is why this
//! file maps IDs to profiles with a flat `match`, never with a comparison
//! such as `release >= "3.0"`: CPython's inventory interleaves maintenance
//! lines (3.13.16 shipped a day before 3.11.17), so "greater than" is not a
//! well-defined question for release IDs.
//!
//! ## Which releases have a profile
//!
//! A release gets a profile when its row claims an IIR subset (`iir_status`
//! is not `-`). `release_inventory.py check` enforces "profile is `-` exactly
//! when `iir_status` is `-`", and `tests/release_subsets.rs` pins this table
//! against the CSV.
//!
//! | Release(s)                                   | Profile | Evidence |
//! |----------------------------------------------|---------|----------|
//! | 3.0a1, 3.11.17, 3.12.3, 3.13.16, 3.16.0a1     | `py3`   | below    |
//! | every other row                              | none    | no grammar pair yet |
//!
//! ## The `py3` profile
//!
//! What `py-int-print` relies on, and where each tagged release says so
//! (git blob IDs in `code/grammars/python/pair-evidence.csv`):
//!
//! - **`int` is one unbounded type.** 3.0a1 has no `Objects/intobject.c`;
//!   `Python/bltinmodule.c` binds `int` to `PyLong_Type`
//!   (`SETBUILTIN("int", &PyLong_Type)`). Every later release keeps that.
//! - **`//` and `%` floor.** At 3.0a1, `Objects/longobject.c` `long_div`
//!   (`nb_floor_divide`) and `long_mod` both go through `l_divmod`, which
//!   adjusts a truncated quotient down and adds the divisor to the remainder
//!   when their signs differ. 3.11.17 to 3.16.0a1 keep `l_divmod` and add
//!   the single-digit fast paths `fast_floor_div` / `fast_mod`, which floor
//!   too. So `-7 // 2 == -4` and `-7 % 2 == 1` everywhere.
//! - **A zero divisor raises `ZeroDivisionError`.** 3.0a1: `long_divrem`
//!   ("integer division or modulo by zero"); later: "division by zero"
//!   (`long_div` / `long_mod` paths).
//! - **`print` is a builtin function that writes `str(x)` and a newline.**
//!   3.0a1: `builtin_print` with keyword parameters `sep`, `end`, `file`
//!   (bltinmodule.c line 1179), registered as `"print"`; later:
//!   `builtin_print_impl`. `print()` writes only the newline.
//! - **`/` is true division** (PEP 238 is the Python 3 default): 3.0a1's
//!   `nb_true_divide` is `long_true_divide`, which returns a float. So `/`
//!   is outside an integer subset: [`Slash::True`].
//!
//! The host interpreters (3.11.17, 3.12.3, 3.13.16) reproduce every
//! accepted corpus case; 3.0a1 and 3.16.0a1 rest on the source evidence
//! above (LANG84 "Pinned historical source").

/// What `/` means on two integers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slash {
    /// True division: the result is a float (Python 3, PEP 238). Outside
    /// every integer subset, so the compiler refuses it as `Unsupported`.
    True,
    /// Classic division: floor division on ints (Python 2 without `from
    /// __future__ import division`). It would lower to `i64_div_floor`. No
    /// profile uses it yet; a Python 2 release needs its own pair (its
    /// `print` is a statement) before it can.
    Classic,
}

/// The semantic switches one release (or many) compiles under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReleaseProfile {
    /// The name the `profile` column of `releases.csv` uses.
    pub name: &'static str,
    /// What `/` means on integers.
    pub slash: Slash,
}

/// Python 3: unbounded `int`, floor `//` and `%`, true `/`, `print()`.
pub const PY3: ReleaseProfile = ReleaseProfile {
    name: "py3",
    slash: Slash::True,
};

/// Every profile this crate defines, for tooling and tests.
pub const PROFILES: &[ReleaseProfile] = &[PY3];

/// The profile an inventory release compiles under, or `None` when the
/// release claims no IIR subset. Exact IDs only; nothing here orders or
/// compares versions.
pub fn profile_for(release: &str) -> Option<&'static ReleaseProfile> {
    match release {
        "3.0a1" | "3.11.17" | "3.12.3" | "3.13.16" | "3.16.0a1" => Some(&PY3),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_7a_releases_share_py3_and_nothing_else_has_a_profile() {
        for release in ["3.0a1", "3.11.17", "3.12.3", "3.13.16", "3.16.0a1"] {
            assert_eq!(profile_for(release), Some(&PY3), "{release}");
        }
        for release in ["3.0", "3.12", "3.13.15", "2.7.18", "", "py3"] {
            assert_eq!(profile_for(release), None, "{release:?}");
        }
        assert_eq!(PY3.slash, Slash::True);
        assert!(PROFILES.iter().all(|p| !p.name.is_empty()));
    }
}
