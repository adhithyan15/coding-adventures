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
//! | 2.2, 2.4, 2.6, 2.7.18                        | `py2`   | below    |
//! | 3.0a1                                        | `py30`  | below    |
//! | 3.5.10, 3.6.0, 3.11.17, 3.12.3, 3.13.16,     | `py3`   | below    |
//! | 3.16.0a1                                     |         |          |
//! | every other row                              | none    | no grammar pair yet |
//!
//! A profile also names the subsets its releases claim (`subsets`): `py3`
//! releases compile `py-int-print` and, since phase 7c slice 3,
//! `py-float-print`; `py30` releases `py-int-print`; `py2` releases
//! `py2-int-print`. Python 2 and Python 3 have different subsets, not one
//! subset under two profiles, because the same text means different things
//! in them (`print(1, 2)` is a call in Python 3 and a tuple in Python 2).
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
//!   is outside an integer subset: [`Slash::True`]. Under [`Floats::Repr`]
//!   it lowers to `f64_div` (`py-float-print`); under [`Floats::None`] it
//!   is refused.
//! - **Integer literals are decimal** in `py-int-print`, with PEP 515 `_`
//!   separators where the release lexes them (3.6.0 and later; 3.0a1 and
//!   3.5.10 lex `1_000` as an error). A leading zero on a non-zero literal
//!   is a SyntaxError in every Python 3. [`IntLiterals::Decimal`].
//!
//! The host interpreters (3.11.17, 3.12.3, 3.13.16) reproduce every
//! accepted corpus case; 3.0a1 and 3.16.0a1 rest on the source evidence
//! above (LANG84 "Pinned historical source").
//!
//! ## Floats: `py3` against `py30` (LANG84 phase 7c slice 3)
//!
//! Everything above holds for 3.0a1 too. The two profiles differ in one
//! switch, [`Floats`], because they differ in what `print` writes for a
//! float, which is `str(x)`:
//!
//! | Release | `float`'s `tp_str` | `print(1/3)` | Profile |
//! |---------|--------------------|--------------|---------|
//! | 3.0a1 | `float_str`: `format_float(…, PREC_STR)`, `PREC_STR` 12 (`Objects/floatobject.c` lines 285–299), i.e. `%.12g` | `0.333333333333` | `py30`, [`Floats::None`] |
//! | 3.5.10 … 3.16.0a1 | `float_repr` (`Objects/floatobject.c` at v3.5.10, `tp_repr` and `tp_str` both, lines 1849/1855): the shortest string that reads back exactly | `0.3333333333333333` | `py3`, [`Floats::Repr`] |
//!
//! LANG83 has one Python float writer, `write_f64_py`, and it writes
//! `repr`. So floats are in the subset only under [`Floats::Repr`]. The
//! difference is semantic, not lexical (LANG84 C2): `7 / 2` lexes in every
//! Python 3 release, so 3.0a1's lexer could never be its only guard.
//!
//! What `py-float-print` relies on, at v3.5.10 and in the host interpreters:
//!
//! - **A float literal is correctly rounded** (`PyOS_string_to_double`
//!   through `dtoa.c`), as Rust's `str::parse::<f64>` is.
//! - **`float + - *` never raise**: `float_add`, `float_sub`, `float_mul`
//!   compute in C `double`, so an overflow is `inf` and `inf - inf` is
//!   `nan`, as IEEE (and LANG83 PR6c's `f64_*`) say.
//! - **`/` raises on a zero divisor** (`float_div`: "float division by
//!   zero"; `long_true_divide`: "division by zero"), including `-0.0`.
//! - **`int / int` is the correctly rounded quotient** of the exact ints;
//!   when both are at most 2^53 in magnitude `long_true_divide` takes the
//!   fast path `(double)a / (double)b` (Objects/longobject.c at v3.5.10,
//!   "Fast path for a and b small"), which is `int_to_real` then `f64_div`.
//! - **Mixing converts the int** with `PyLong_AsDouble` (round half to
//!   even), as `int_to_real` does on every backend.
//!
//! ## The `py2` profile (LANG84 phase 7c slice 1)
//!
//! What `py2-int-print` relies on, at every `py2` tag (2.2, 2.4, 2.6 and
//! 2.7.18; blob IDs in
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
//! - **Integer literals have four spellings and a long suffix** (phase 7c
//!   slice 2). `tok_get` in `Parser/tokenizer.c` lexes a decimal, an octal
//!   `0[0-7]*` (so `010` is 8), a hex `0x…` and, from 2.6 (PEP 3127),
//!   `0o…` and `0b…`; any of them may end in `l`/`L`. `parsenumber`
//!   (`Python/compile.c` at 2.2 and 2.4, `Python/ast.c` at 2.6 and 2.7.18)
//!   hands an `L` literal to `PyLong_FromString(s, 0, 0)` and the rest to
//!   `PyOS_strtoul`/`PyOS_strtol` with base 0, which pick the base from the
//!   prefix. `print` writes `str()` of a long, which has no `L`
//!   (`Objects/longobject.c` `long_str`; only `long_repr` adds it), and a
//!   long that fits `i64` does the same arithmetic as an int
//!   (`long_classic_div` floors through `l_divmod`). So in `i64` the suffix
//!   changes nothing the subset can observe. [`IntLiterals::Python2`].
//!
//!   Which prefixes a release has is decided by its lexer, not here: the
//!   2.2 and 2.4 pairs never produce a `0o17` token. The decoder accepts
//!   every Python 2 spelling because no spelling has two values in two
//!   Python 2 releases inside `i64`. The one spelling that does differ (at
//!   2.2 a hex or octal literal between `LONG_MAX` and `ULONG_MAX` is a
//!   negative int, `(long) PyOS_strtoul`; 2.4 made it a positive long, PEP
//!   237) is above `i64::MAX`, which the decoder refuses.

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

/// Whether the release path has floats, and how `print` writes one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Floats {
    /// No floats: a `FLOAT` token and a true `/` are refused. 3.0a1
    /// (`%.12g` for `str(float)`) and Python 2 (the same).
    None,
    /// Floats are in the subset and `print` writes `repr` (LANG83
    /// `write_f64_py`): the `py3` profile, 3.5.10 and later.
    Repr,
}

/// How an `INT` token's text becomes a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntLiterals {
    /// Decimal only, with optional single `_` separators between digits
    /// (PEP 515; the lexer decides whether the release has them). A
    /// non-zero literal with a leading zero is refused. The `py3` profile.
    Decimal,
    /// Python 2: decimal, octal `0[0-7]*`, `0x`/`0o`/`0b` prefixes, and an
    /// optional `l`/`L` long suffix that changes no value. No `_`. The
    /// `py2` profile.
    Python2,
}

/// The semantic switches one release (or many) compiles under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReleaseProfile {
    /// The name the `profile` column of `releases.csv` uses.
    pub name: &'static str,
    /// What `/` means on integers.
    pub slash: Slash,
    /// How integer literals are spelled.
    pub int_literals: IntLiterals,
    /// Whether floats exist, and how `print` writes them.
    pub floats: Floats,
    /// The subsets this profile's releases claim, as `subsets.csv` names
    /// them, narrowest first; `iir_status` lists them joined with `;`.
    pub subsets: &'static [&'static str],
}

impl ReleaseProfile {
    /// The widest subset this profile compiles (the last of `subsets`),
    /// for diagnostics.
    pub fn subset(&self) -> &'static str {
        self.subsets.last().copied().unwrap_or("")
    }
}

/// Python 3 from 3.2: unbounded `int`, floor `//` and `%`, true `/`,
/// `print()`, and floats printed as `repr`.
pub const PY3: ReleaseProfile = ReleaseProfile {
    name: "py3",
    slash: Slash::True,
    int_literals: IntLiterals::Decimal,
    floats: Floats::Repr,
    subsets: &["py-int-print", "py-float-print"],
};

/// Python 3.0: as [`PY3`], but `str(float)` is `%.12g`, which no LANG83
/// writer produces, so the integer subset only.
pub const PY30: ReleaseProfile = ReleaseProfile {
    name: "py30",
    slash: Slash::True,
    int_literals: IntLiterals::Decimal,
    floats: Floats::None,
    subsets: &["py-int-print"],
};

/// Python 2: classic `/` on ints, floor `//` and `%`, octal, hex and long
/// literals, the `print`
/// statement (the statement is a grammar shape, not a switch: only the
/// `py2` pairs have a `print_stmt` rule).
pub const PY2: ReleaseProfile = ReleaseProfile {
    name: "py2",
    slash: Slash::Classic,
    int_literals: IntLiterals::Python2,
    floats: Floats::None,
    subsets: &["py2-int-print"],
};

/// Every profile this crate defines, for tooling and tests.
pub const PROFILES: &[ReleaseProfile] = &[PY2, PY30, PY3];

/// CST rules a profile's grammars add to [`crate::DISPATCHED_RULES`]
/// (LANG84 C8): `cst_contract` checks each one exists in every release of
/// that profile, and only there.
pub const PROFILE_RULES: &[(&str, &[&str])] = &[("py2", &["print_stmt"])];

/// Token types a profile's grammars add to [`crate::DISPATCHED_TOKENS`]
/// (LANG84 C8, phase 7c slice 3): `cst_contract` checks each one is defined
/// and referenced in every release of that profile, and in no other. Only
/// the `py3` pairs lex a `FLOAT`.
pub const PROFILE_TOKENS: &[(&str, &[&str])] = &[("py3", &["FLOAT"])];

/// The profile an inventory release compiles under, or `None` when the
/// release claims no IIR subset. Exact IDs only; nothing here orders or
/// compares versions.
pub fn profile_for(release: &str) -> Option<&'static ReleaseProfile> {
    match release {
        "2.2" | "2.4" | "2.6" | "2.7.18" => Some(&PY2),
        "3.0a1" => Some(&PY30),
        "3.5.10" | "3.6.0" | "3.11.17" | "3.12.3" | "3.13.16" | "3.16.0a1" => Some(&PY3),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paired_releases_have_their_profile_and_nothing_else_has_one() {
        for release in ["3.5.10", "3.6.0", "3.11.17", "3.12.3", "3.13.16", "3.16.0a1"] {
            assert_eq!(profile_for(release), Some(&PY3), "{release}");
        }
        assert_eq!(profile_for("3.0a1"), Some(&PY30));
        for release in ["2.2", "2.4", "2.6", "2.7.18"] {
            assert_eq!(profile_for(release), Some(&PY2), "{release}");
        }
        // The legacy minor lines (2.7, 3.0, 3.12) and unpaired rows have none.
        for release in ["3.0", "3.12", "3.13.15", "3.5.9", "3.6.1", "2.7", "2.7.17", "2.6.9", "2.4.6", "2.2a3", "", "py3", "py2"] {
            assert_eq!(profile_for(release), None, "{release:?}");
        }
        assert_eq!(PY3.slash, Slash::True);
        assert_eq!(PY2.slash, Slash::Classic);
        assert_eq!(PY3.int_literals, IntLiterals::Decimal);
        assert_eq!(PY2.int_literals, IntLiterals::Python2);
        // py30 differs from py3 in exactly one switch.
        assert_eq!(PY30, ReleaseProfile { name: "py30", floats: Floats::None, subsets: &["py-int-print"], ..PY3 });
        assert_eq!((PY3.floats, PY30.floats, PY2.floats), (Floats::Repr, Floats::None, Floats::None));
        assert!(PROFILES.iter().all(|p| !p.name.is_empty()));
    }

    #[test]
    fn each_profile_names_its_own_subset() {
        assert_eq!(PY3.subsets, ["py-int-print", "py-float-print"]);
        assert_eq!(PY3.subset(), "py-float-print");
        assert_eq!(PY30.subset(), "py-int-print");
        assert_eq!(PY2.subset(), "py2-int-print");
        for (profile, names) in PROFILE_RULES.iter().chain(PROFILE_TOKENS) {
            assert!(PROFILES.iter().any(|p| p.name == *profile), "{profile}");
            assert!(!names.is_empty());
        }
        // A profile lexes floats exactly when it prints them.
        for profile in PROFILES {
            let lexes_floats = PROFILE_TOKENS.iter().any(|(p, names)| *p == profile.name && names.contains(&"FLOAT"));
            assert_eq!(lexes_floats, profile.floats == Floats::Repr, "{}", profile.name);
        }
    }
}
