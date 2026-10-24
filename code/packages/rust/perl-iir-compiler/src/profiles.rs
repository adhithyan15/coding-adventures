//! # Release profiles (LANG84 "Release profiles")
//!
//! Two Perl releases can parse `print(-0);` into the *same* tree and still
//! print different things. The tree says "negate the integer 0"; only the
//! release knows whether that integer is a C `double` (whose negation is the
//! IEEE negative zero `-0.0`) or an integer (whose negation is just `0`).
//! LANG84 keeps every such difference in one place, a [`ReleaseProfile`]: a
//! plain struct of semantic switches. Each row of
//! `code/grammars/perl/releases.csv` names its profile in the `profile`
//! column, and many releases share one.
//!
//! The compiler may vary by release in exactly two ways (LANG84 C2): by the
//! *shape* of the tree the release's grammar produced, and by the fields of
//! this struct. It never looks at a release ID's digits. That is why
//! [`profile_for`] is a lookup of exact IDs in two lists and never a
//! comparison such as `release >= "5.006"`: Perl's inventory interleaves
//! maintenance tracks (5.004_04-m4 shipped after 5.004_65), so "greater
//! than" is not even a well-defined question for release IDs.
//!
//! ## The profiles
//!
//! | Profile   | Releases | `arithmetic` | Printed zero |
//! |-----------|----------|--------------|--------------|
//! | `perl_nv` | the 114 checked releases from 1.000 to 5.004_05 (and 1.0.15, 1.0_16) | [`Arithmetic::Double`] | `-0.0` is refused |
//! | `perl_iv` | 5.38.0, 5.38.2, 5.44.0, 5.45.3 | [`Arithmetic::Integer`] | always `0` |
//!
//! Every pending row has no profile (no grammar pair, so nothing compiles).
//!
//! ## What the subset needs to know
//!
//! The `pl-int-print` subset is `print(<expr>);` over decimal literals,
//! `+`, `-`, `*`, unary minus and parentheses, with every value inside
//! the signed 32-bit range. Two facts decide what such a program prints:
//!
//! 1. **A non-zero value, or a positive zero, prints as its exact decimal
//!    integer in every checked release.** Every value is an integer below
//!    2^31 in magnitude, which a C `double` holds exactly, and every
//!    printing path renders it exactly: `sprintf("%.20g")` in Perl 1–4,
//!    `Gconvert(x, DBL_DIG, ...)` (15 significant digits) or `"%ld"` in
//!    Perl 5, and Perl's `print` adds no separator or newline because `$,`,
//!    `$\` and `$#` are unset by default.
//! 2. **A negative zero is where releases differ.** `-0`, `0 * -3` and
//!    `-0 - 0` are the IEEE `-0.0` when Perl computes them in C doubles.
//!    Whether `print` then shows `0` or `-0` depends on the release's
//!    constant folding (5.001–5.004 fold an integral `double` constant to an
//!    integer, 5.000 does not, and 5.005 stops doing it for unary minus) and
//!    on the platform: Perl 1–4 print with `%.20g` except on Apollo, and Perl
//!    5.000–5.004 strip a `-0` only where a hints file defines
//!    `FIXNEGATIVEZERO` (no `perl.h` does). A pre-ANSI `printf` that tests
//!    `x < 0` prints `0`; glibc prints `-0`. The release does not pin the
//!    output down, so under [`Arithmetic::Double`] the compiler tracks each
//!    value's zero sign by IEEE rules and **refuses** to print a negative
//!    zero, with a typed `Unsupported` error. A refusal is never a
//!    miscompile; printing `0` for a release that prints `-0` would be.
//!
//! In the IV-preserving releases (Perl 5.8 onward, and so all of the 5.38+
//! rows) integer operands give integer results, and the host `perl`
//! v5.38.2 oracle prints `0` for `-0`, `0 * -1` and `-0.0` alike.
//!
//! ## Pinned sources (read 2026-10-20 from `Perl/perl5` tags)
//!
//! | Tag (commit) | File:line | What it shows |
//! |---|---|---|
//! | `perl-1.0` (`8d063cd8450e59ea1c611a2f4f5a21059a2804f1`) | `arg.c`:1464–1482, 1571 | `O_MULTIPLY`, `O_ADD`, `O_SUBTRACT` and `O_NEGATE` compute in a `double value` |
//! | `perl-1.0` | `arg.c`:723–735, `perl.h`:141 | `print` writes each element's `str_get` text; `$#` (`ofmt`) defaults to `Nullch` |
//! | `perl-1.0` | `str.c`:70 | a number's text is `sprintf(s,"%.20g",...)` |
//! | `perl-1.0.15` (`de64de47f7150f9a23ee0bb6d23c1082e8c5e551`), `perl-1.0.16` (`5fe6fe9e744e5dd360822dc892a626b8193eea40`) | `str.c`:73 | the same `%.20g` |
//! | `perl-2.0` (`378cc40b38293ffc7298c6a7ed3cd740ad79be52`) | `str.c`:81–86, `eval.c`:549 | `0.0` prints as `"0"` only `#ifdef apollo`; `O_NEGATE` on doubles |
//! | `perl-3.000` (`a687059cbaf2c6fdccb5e0fae2aee80ec15625a8`) | `str.c`:125–130, `eval.c`:402 | as 2.0 |
//! | `perl-4.0.36` (`e334a159a5616cab575044bafaf68f75b7bb3a16`) | `str.c`:170–175, `eval.c`:918 | as 2.0 |
//! | `perl-5.000` (`a0d0e21ea6ea90a22318550944fe6cb09ae10cda`) | `pp.c`:907 (`pp_negate`), 581 (`pp_multiply`) | `SETn(-SvNV(sv))`, `SETn(left * right)`: doubles |
//! | `perl-5.000` | `sv.c`:1282–1284, `op.c`:1261 | `Gconvert(SvNVX(sv), DBL_DIG, 0, s)`, `-0` stripped only `#ifdef FIXNEGATIVEZERO`; `fold_constants` keeps the folded `double` |
//! | `perl-5.004` (`84c476bd7dc489e0b953be0e097d1a3adfd610d4`) | `op.c`:1583, `pp.c` `pp_negate`, `sv.c`:1698–1701 | an integral folded `double` becomes an IV ("can we smush double to int"); `pp_negate` gains an IV path; `FIXNEGATIVEZERO` unchanged |
//! | `perl-5.005` (`2eac638167584a95d64b751a89a1df475c16cacd`) | `op.c`:1722–1724 | "don't smush -2.0 to -2": `OP_NEGATE` results stay `double` |
//!
//! The untagged development and maintenance releases between those tags
//! (5.004_5x, 5.004_04-mN, ...) are covered by fact 1 alone, which holds at
//! every tag on both sides of them; the profile does not depend on which
//! side of the folding change a release falls, because it refuses every
//! negative zero.

/// How a release evaluates the subset's `+`, `-`, `*` and unary minus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arithmetic {
    /// IV-preserving integer arithmetic (Perl 5.8 onward): an integer result
    /// stays an integer, so every zero is `0` and prints `0`.
    Integer,
    /// C `double` arithmetic (Perl 1–4 and 5.000–5.005): IEEE signed zero.
    /// The compiler refuses to print a value that is `-0.0` here, because
    /// the release prints it as `0` or `-0` depending on folding and the
    /// platform (see the module docs).
    ///
    /// | expression | IEEE result | printed by the release |
    /// |------------|-------------|------------------------|
    /// | `0`        | `+0.0`      | `0` |
    /// | `-0`       | `-0.0`      | `0` or `-0` (refused) |
    /// | `0 * -3`   | `-0.0`      | `0` or `-0` (refused) |
    /// | `-0 + 0`   | `+0.0`      | `0` |
    /// | `3 - 3`    | `+0.0`      | `0` |
    Double,
}

/// The semantic switches one release (or many) compiles under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReleaseProfile {
    /// The name the `profile` column of `releases.csv` uses.
    pub name: &'static str,
    /// How the subset's arithmetic is evaluated.
    pub arithmetic: Arithmetic,
}

/// Perl 1.000 through 5.004_05: arithmetic in C doubles.
pub const PERL_NV: ReleaseProfile = ReleaseProfile {
    name: "perl_nv",
    arithmetic: Arithmetic::Double,
};

/// Perl 5.38.0 onward: IV-preserving integer arithmetic.
pub const PERL_IV: ReleaseProfile = ReleaseProfile {
    name: "perl_iv",
    arithmetic: Arithmetic::Integer,
};

/// Every profile this crate defines, for tooling and tests.
pub const PROFILES: &[ReleaseProfile] = &[PERL_NV, PERL_IV];

/// The checked releases that compile under [`PERL_IV`], in inventory order.
pub const PERL_IV_RELEASES: &[&str] = &["5.38.0", "5.38.2", "5.44.0", "5.45.3"];

/// The checked releases that compile under [`PERL_NV`], in inventory order.
pub const PERL_NV_RELEASES: &[&str] = &[
    "1.000", "2.000", "2.001", "3.000", "3.044", "4.000", "4.036", "5.000", "5.001",
    "5.001n", "5.002", "5.002_01", "5.003", "5.003_01", "5.003_02", "5.003_03",
    "5.003_04", "5.003_05", "5.003_06", "5.003_07", "5.003_08", "5.003_09", "5.003_10",
    "5.003_11", "5.003_12", "5.003_13", "5.003_14", "5.003_15", "5.003_16", "5.003_17",
    "5.003_18", "5.003_19", "5.003_20", "5.003_21", "5.003_22", "5.003_23", "5.003_24",
    "5.003_25", "5.003_26", "5.003_27", "5.003_28", "5.003_90", "5.003_91", "5.003_92",
    "5.003_93", "5.003_94", "5.003_95", "5.003_96", "5.003_97", "5.003_97a",
    "5.003_97b", "5.003_97c", "5.003_97d", "5.003_97e", "5.003_97f", "5.003_97g",
    "5.003_97h", "5.003_97i", "5.003_97j", "5.003_98", "5.003_99", "5.003_99a",
    "p54rc1", "p54rc2", "5.004", "5.004_01-t2", "5.004_01", "5.004_01_01",
    "5.004_01_02", "5.004_01_03", "5.004_02", "5.004_02_01", "5.004_03-t2", "5.004_03",
    "5.004_50", "5.004_04-t1", "5.004_04-t2", "5.004_51", "5.004_04-t3", "5.004_04-t4",
    "5.004_04", "5.004_52", "5.004_53", "5.004_54", "5.004_55", "5.004_56", "5.004_57",
    "5.004_58", "5.004_59", "5.004_60", "5.004_61", "5.004_04-m1", "5.004_62",
    "5.004_63", "5.004_64", "5.004_04-m2", "5.004_04-m3", "5.004_65", "5.004_04-m4",
    "5.004_66", "5.004_67", "5.004_68", "5.004_69", "5.004_70", "5.004_71", "5.004_72",
    "5.004_05-MT5", "5.004_05-MT6", "5.004_05-MT7", "5.004_05-MT8", "5.004_05-MT9",
    "5.004_05", "1.0.15", "1.0_16",
];

/// The profile an inventory release compiles under, or `None` for a release
/// with no reviewed profile (every pending row). Exact IDs only: nothing here
/// orders or compares versions, and a release that gains a grammar pair gets
/// no profile until someone adds it to a list above.
pub fn profile_for(release: &str) -> Option<&'static ReleaseProfile> {
    if PERL_IV_RELEASES.contains(&release) {
        Some(&PERL_IV)
    } else if PERL_NV_RELEASES.contains(&release) {
        Some(&PERL_NV)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_perl_parser::SUPPORTED_RELEASES;

    #[test]
    fn every_checked_release_has_exactly_one_profile() {
        for release in SUPPORTED_RELEASES {
            let lists = usize::from(PERL_IV_RELEASES.contains(release))
                + usize::from(PERL_NV_RELEASES.contains(release));
            assert_eq!(lists, 1, "{release}");
            assert!(profile_for(release).is_some(), "{release}");
        }
        assert_eq!(
            PERL_IV_RELEASES.len() + PERL_NV_RELEASES.len(),
            SUPPORTED_RELEASES.len(),
            "a profile list names a release with no checked pair"
        );
    }

    #[test]
    fn profiles_are_looked_up_by_exact_id() {
        assert_eq!(profile_for("5.38.0"), Some(&PERL_IV));
        assert_eq!(profile_for("1.000"), Some(&PERL_NV));
        assert_eq!(profile_for("5.004_05"), Some(&PERL_NV));
        for release in ["", "5.38", "5.38.1", "v5.38.0", "5.004_72 ", "perl_iv", "5.006"] {
            assert_eq!(profile_for(release), None, "{release:?}");
        }
        assert!(PROFILES.iter().all(|p| !p.name.is_empty()));
        assert_ne!(PERL_NV.arithmetic, PERL_IV.arithmetic);
    }
}
