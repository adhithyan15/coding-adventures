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
//! | 2.2, 2.7.18                                  | `py2`   | below    |
//! | 3.0a1, 3.11.17, 3.12.3, 3.13.16, 3.16.0a1     | `py3`   | below    |
//! | every other row                              | none    | no grammar pair yet |
//!
//! A profile also names the subset its releases claim (`subset`):
//! `py3` releases compile `py-int-print`, `py2` releases `py2-int-print`.
//! The two are different subsets, not one subset under two profiles,
//! because the same text means different things in them (`print(1, 2)` is
//! a call in Python 3 and a tuple in Python 2).
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
//!
//! ## The `py2` profile (LANG84 phase 7c slice 1)
//!
//! What `py2-int-print` relies on, at both tags (2.2 and 2.7.18; blob IDs in
//! `pair-evidence.csv`, columns `intobject_c_blob`, `ceval_c_blob`,
//! `pythonrun_c_blob`). There is no host `python2`, so this source is the
//! oracle for every case:
//!
//! - **`/` is classic division, and on two ints it floors.**
//!   `Objects/intobject.c` registers `int_classic_div` as `nb_divide` and
//!   `int_div` as `nb_floor_divide`; both call `i_divmod`, which computes
//!   the C quotient and then, when the remainder's sign differs from the
//!   divisor's, adds the divisor to the remainder and decrements the
//!   quotient. So `-7 / 2 == -7 // 2 == -4` and `-7 % 2 == 1`. (Only
//!   `from __future__ import division` or `-Qnew` makes `/` true division;
//!   neither is in the subset.) [`Slash::Classic`].
//! - **A zero divisor raises `ZeroDivisionError`** ("integer division or
//!   modulo by zero", `i_divmod`).
//! - **`int` is bounded, but promotes.** A result outside the C `long` is
//!   handed to `PyLong_Type` (`int_add` … call its `nb_add`; `i_divmod`
//!   returns `DIVMOD_OVERFLOW` for `LONG_MIN / -1`), so CPython prints a
//!   bigger number. The subset traps `Trap.Overflow` instead, the same
//!   stated divergence as `py3`.
//! - **`print` is a statement.** `print x` compiles to the expression, then
//!   `PRINT_ITEM` (writes `str(x)`) and `PRINT_NEWLINE` (writes `\n` and
//!   clears the stream's softspace); a bare `print` is `PRINT_NEWLINE` only
//!   (`Python/ceval.c`). A trap in `x` therefore writes nothing for that
//!   statement.
//! - **Why `print a, b` and `print a,` are refused.** `PRINT_ITEM` sets
//!   softspace, and before an uncaught exception's traceback CPython 2 calls
//!   `Py_FlushLine`, which writes `\n` if softspace is set
//!   (`Python/pythonrun.c`). `print 1, 1/0` therefore writes `1\n`, a
//!   newline the program never asked for, which only a trap handler could
//!   reproduce. The compiler refuses both forms (`Unsupported`).

/// What `/` means on two integers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slash {
    /// True division: the result is a float (Python 3, PEP 238). Outside
    /// every integer subset, so the compiler refuses it as `Unsupported`.
    True,
    /// Classic division: floor division on ints (Python 2 without `from
    /// __future__ import division`). It lowers exactly like `//`, to
    /// `i64_div_floor` (with the same `-1` divisor fold). The `py2` profile.
    Classic,
}

/// The semantic switches one release (or many) compiles under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReleaseProfile {
    /// The name the `profile` column of `releases.csv` uses.
    pub name: &'static str,
    /// What `/` means on integers.
    pub slash: Slash,
    /// The subset this profile's releases claim, as `subsets.csv` names it.
    pub subset: &'static str,
}

/// Python 3: unbounded `int`, floor `//` and `%`, true `/`, `print()`.
pub const PY3: ReleaseProfile = ReleaseProfile {
    name: "py3",
    slash: Slash::True,
    subset: "py-int-print",
};

/// Python 2: classic `/` on ints, floor `//` and `%`, the `print`
/// statement (the statement is a grammar shape, not a switch: only the
/// `py2` pairs have a `print_stmt` rule).
pub const PY2: ReleaseProfile = ReleaseProfile {
    name: "py2",
    slash: Slash::Classic,
    subset: "py2-int-print",
};

/// Every profile this crate defines, for tooling and tests.
pub const PROFILES: &[ReleaseProfile] = &[PY2, PY3];

/// CST rules a profile's grammars add to [`crate::DISPATCHED_RULES`]
/// (LANG84 C8): `cst_contract` checks each one exists in every release of
/// that profile, and only there.
pub const PROFILE_RULES: &[(&str, &[&str])] = &[("py2", &["print_stmt"])];

/// The profile an inventory release compiles under, or `None` when the
/// release claims no IIR subset. Exact IDs only; nothing here orders or
/// compares versions.
pub fn profile_for(release: &str) -> Option<&'static ReleaseProfile> {
    match release {
        "2.2" | "2.7.18" => Some(&PY2),
        "3.0a1" | "3.11.17" | "3.12.3" | "3.13.16" | "3.16.0a1" => Some(&PY3),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paired_releases_have_their_profile_and_nothing_else_has_one() {
        for release in ["3.0a1", "3.11.17", "3.12.3", "3.13.16", "3.16.0a1"] {
            assert_eq!(profile_for(release), Some(&PY3), "{release}");
        }
        for release in ["2.2", "2.7.18"] {
            assert_eq!(profile_for(release), Some(&PY2), "{release}");
        }
        // The legacy minor lines (2.7, 3.0, 3.12) and unpaired rows have none.
        for release in ["3.0", "3.12", "3.13.15", "2.7", "2.7.17", "2.2a3", "", "py3", "py2"] {
            assert_eq!(profile_for(release), None, "{release:?}");
        }
        assert_eq!(PY3.slash, Slash::True);
        assert_eq!(PY2.slash, Slash::Classic);
        assert!(PROFILES.iter().all(|p| !p.name.is_empty()));
    }

    #[test]
    fn each_profile_names_its_own_subset() {
        assert_eq!(PY3.subset, "py-int-print");
        assert_eq!(PY2.subset, "py2-int-print");
        for (profile, rules) in PROFILE_RULES {
            assert!(PROFILES.iter().any(|p| p.name == *profile), "{profile}");
            assert!(!rules.is_empty());
        }
    }
}
