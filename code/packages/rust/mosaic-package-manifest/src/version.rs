//! # Versions and version requirements
//!
//! A `mosaic-package.toml` says two kinds of thing about versions:
//!
//! ```toml
//! [package]
//! version = "0.16.0"          # a VERSION: what this package IS
//!
//! [dependencies]
//! mosaic-pkg-toolkit = "0.16.0"   # a REQUIREMENT: what this package ACCEPTS
//! ```
//!
//! They look identical, and for a long time they were treated identically:
//! the parser checked that both had the right *shape* and nothing ever
//! compared one against the other. A requirement was a comment. Five of the
//! repository's 26 requirement strings drifted out of date without anything
//! noticing (mosaic-package-publishing-v1 §2.2).
//!
//! This module gives them meaning, following Cargo (spec §7.4, which UI29
//! §4.5 promised would be "Cargo-compatible"):
//!
//! | Written     | Kind  | Means                    | Matches                  |
//! |-------------|-------|--------------------------|--------------------------|
//! | `"1.2.3"`   | caret | `>=1.2.3, <2.0.0`        | 1.2.3, 1.9.0             |
//! | `"0.16.0"`  | caret | `>=0.16.0, <0.17.0`      | 0.16.0, 0.16.3           |
//! | `"0.0.3"`   | caret | `>=0.0.3, <0.0.4`        | 0.0.3 only               |
//! | `"=0.16.0"` | exact | `=0.16.0`                | 0.16.0 only              |
//! | `"~0.16.0"`, `">=1"`, `"*"`, `"1.2"`, ranges | — | **rejected** in v1 | — |
//!
//! ## Why caret, and why it moves at the left-most non-zero digit
//!
//! SemVer promises that a release which keeps the *major* number keeps the
//! interface. Below 1.0.0 SemVer promises nothing, so Cargo (and this spec,
//! §7.3) adopt a convention: before 1.0.0 the left-most non-zero component
//! plays the role of "major". A breaking change to a 0.x package bumps the
//! minor number, and a breaking change to a 0.0.x package bumps the patch.
//! Caret matching is the consumer half of that bargain — "anything the
//! author promised is compatible with what I wrote":
//!
//! ```text
//!            the "major" digit        upper bound (exclusive)
//!   1.2.3    1                        2.0.0
//!   0.2.3      2                      0.3.0
//!   0.0.3        3                    0.0.4
//! ```
//!
//! ## Pre-releases: opt in, one release at a time
//!
//! `1.0.0-rc.1` sorts *below* `1.0.0` (SemVer §11), so a plain range like
//! `>=0.9.0, <2.0.0` would happily match it — and a consumer who never asked
//! for an unfinished release would get one. Cargo's rule, which we follow: a
//! version with a pre-release tag matches only a requirement that itself names
//! a pre-release **of the same `major.minor.patch`**.
//!
//! ```text
//!   requirement "1.0.0-rc.1"  matches 1.0.0-rc.1, 1.0.0-rc.2, 1.0.0, 1.4.0
//!                             not     1.1.0-beta   (pre-release of another triple)
//!   requirement "1.0.0"       not     1.1.0-rc.1   (never asked for a pre-release)
//! ```
//!
//! ## No dependencies, on purpose
//!
//! The `semver` crate would do all of this. The manifest parser deliberately
//! keeps a tiny dependency footprint (spec §10 Phase 1), and the subset we
//! need — strict parsing, precedence, caret and exact — is small enough to
//! read in one sitting. That is rather the point of this repository.

use std::cmp::Ordering;
use std::fmt;

// ---------------------------------------------------------------------------
// Version
// ---------------------------------------------------------------------------

/// One dot-separated pre-release identifier: the `rc` and `1` in `1.0.0-rc.1`.
///
/// SemVer §11.4 compares identifiers differently depending on what they are
/// made of, so we keep the distinction in the type rather than re-deriving it
/// at every comparison:
///
/// - all digits → compared **numerically** (`2 < 10`);
/// - anything else → compared **lexically** in ASCII order (`"alpha" < "beta"`);
/// - a numeric identifier always sorts below an alphanumeric one.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PreReleaseId {
    Numeric(u64),
    Alpha(String),
}

impl Ord for PreReleaseId {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Numeric(a), Self::Numeric(b)) => a.cmp(b),
            (Self::Alpha(a), Self::Alpha(b)) => a.cmp(b),
            (Self::Numeric(_), Self::Alpha(_)) => Ordering::Less,
            (Self::Alpha(_), Self::Numeric(_)) => Ordering::Greater,
        }
    }
}

impl PartialOrd for PreReleaseId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for PreReleaseId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Numeric(n) => write!(f, "{n}"),
            Self::Alpha(s) => f.write_str(s),
        }
    }
}

/// A strict SemVer 2.0.0 version, without build metadata.
///
/// "Strict" means exactly what <https://semver.org> §2, §9 and §10 say, and
/// what `code/scripts/taskapp_release.py`'s `SEMVER` pattern checks:
///
/// | Rule                                        | Rejects              |
/// |---------------------------------------------|----------------------|
/// | exactly three numeric components            | `1.2`, `1.2.3.4`     |
/// | ASCII digits only                           | `１.2.3`, `1.٢.3`    |
/// | no leading zero (except `0` itself)         | `01.2.3`, `1.2.3-01` |
/// | pre-release identifiers are non-empty       | `1.2.3-`, `1.2.3-a..b` |
/// | pre-release uses `[0-9A-Za-z-]` only        | `1.2.3-rc_1`         |
/// | no build metadata (`+…`)                    | `1.2.3+sha.abc`      |
///
/// Build metadata is legal SemVer, but it never affects precedence, so two
/// archives that differ only in `+…` would be indistinguishable to the
/// resolver. The manifest has always refused it (spec §7.1).
///
/// Each numeric component must fit in a `u64`. A version larger than that is
/// not a version anyone has published, and refusing it is simpler than
/// arbitrary-precision comparison.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    /// Empty for a release; `["rc", 1]` for `-rc.1`.
    pub pre: Vec<PreReleaseId>,
}

impl Version {
    /// Parse a strict SemVer version. Returns `None` for anything the table
    /// on [`Version`] rejects.
    ///
    /// ```
    /// use mosaic_package_manifest::Version;
    ///
    /// let v = Version::parse("1.0.0-rc.1").unwrap();
    /// assert_eq!((v.major, v.minor, v.patch), (1, 0, 0));
    /// assert!(v.is_prerelease());
    /// assert!(Version::parse("01.0.0").is_none());
    /// assert!(Version::parse("1.0.0+build").is_none());
    /// ```
    pub fn parse(text: &str) -> Option<Version> {
        // Build metadata is refused outright rather than parsed and dropped.
        if text.contains('+') {
            return None;
        }
        // The first `-` ends the numeric core. Later `-`s are legal *inside*
        // pre-release identifiers (`1.0.0-x-y`), so split only once.
        let (core, pre) = match text.split_once('-') {
            Some((core, pre)) => (core, Some(pre)),
            None => (text, None),
        };

        let mut parts = core.split('.');
        let major = parse_numeric(parts.next()?)?;
        let minor = parse_numeric(parts.next()?)?;
        let patch = parse_numeric(parts.next()?)?;
        if parts.next().is_some() {
            return None;
        }

        let pre = match pre {
            None => Vec::new(),
            Some(pre) => pre
                .split('.')
                .map(parse_pre_release_id)
                .collect::<Option<Vec<_>>>()?,
        };

        Some(Version {
            major,
            minor,
            patch,
            pre,
        })
    }

    /// Whether this is a pre-release (`1.0.0-rc.1`) rather than a release.
    pub fn is_prerelease(&self) -> bool {
        !self.pre.is_empty()
    }

    /// The `major.minor.patch` triple, without the pre-release tag.
    fn triple(&self) -> (u64, u64, u64) {
        (self.major, self.minor, self.patch)
    }
}

/// A numeric component: ASCII digits, no leading zero, fits in a `u64`.
///
/// `str::parse::<u64>` alone is not enough: it accepts `+5` and leading
/// zeros, both of which SemVer forbids.
fn parse_numeric(text: &str) -> Option<u64> {
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if text.len() > 1 && text.starts_with('0') {
        return None;
    }
    text.parse().ok()
}

/// One pre-release identifier. All-digit identifiers obey the numeric rules
/// (so `01` is refused); anything else may use letters, digits and `-`.
fn parse_pre_release_id(text: &str) -> Option<PreReleaseId> {
    if text.is_empty() {
        return None;
    }
    if text.bytes().all(|b| b.is_ascii_digit()) {
        return parse_numeric(text).map(PreReleaseId::Numeric);
    }
    if text.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
        return Some(PreReleaseId::Alpha(text.to_string()));
    }
    None
}

/// SemVer §11 precedence.
///
/// 1. Compare `major`, `minor`, `patch` numerically.
/// 2. A release sorts **above** any pre-release of the same triple
///    (`1.0.0-rc.1 < 1.0.0`).
/// 3. Two pre-releases compare identifier by identifier; if one runs out
///    first, the shorter one is lower (`1.0.0-alpha < 1.0.0-alpha.1`).
///
/// ```text
///   1.0.0-alpha < 1.0.0-alpha.1 < 1.0.0-alpha.beta < 1.0.0-beta
///               < 1.0.0-beta.2 < 1.0.0-beta.11 < 1.0.0-rc.1 < 1.0.0
/// ```
impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.triple().cmp(&other.triple()).then_with(|| {
            match (self.pre.is_empty(), other.pre.is_empty()) {
                (true, true) => Ordering::Equal,
                (true, false) => Ordering::Greater,
                (false, true) => Ordering::Less,
                // `Vec`'s lexicographic order is exactly rule 3.
                (false, false) => self.pre.cmp(&other.pre),
            }
        })
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        for (index, id) in self.pre.iter().enumerate() {
            f.write_str(if index == 0 { "-" } else { "." })?;
            write!(f, "{id}")?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// VersionReq
// ---------------------------------------------------------------------------

/// A `[dependencies]` value: which versions of a dependency a package accepts.
///
/// v1 has exactly two forms (spec §7.4). Everything else is refused at
/// manifest parse, so a requirement can never mean something the resolver
/// does not implement.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VersionReq {
    /// A bare version, `"0.16.0"`: SemVer-compatible with it.
    Caret(Version),
    /// `"=0.16.0"`: that version and no other.
    Exact(Version),
}

/// Why [`VersionReq::parse`] refused a string.
///
/// The two cases need different advice. A malformed version is a typo; an
/// operator is a deliberate choice that v1 does not support yet, and the
/// person who wrote it should be told what to write instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionReqError {
    /// The string uses an operator, wildcard, range or partial version that
    /// v1 does not support: `~1.2.3`, `^1.2.3`, `>=1`, `*`, `1.x`, `1.2`,
    /// `>=1, <2`, `1 || 2`.
    Unsupported,
    /// The string is neither of the supported forms and does not look like
    /// an operator either: `not-a-version`, `=01.2.3`, `1.2.3+build`.
    Malformed,
}

impl VersionReq {
    /// Parse a requirement string.
    ///
    /// ```
    /// use mosaic_package_manifest::{Version, VersionReq, VersionReqError};
    ///
    /// let caret = VersionReq::parse("0.16.0").unwrap();
    /// assert!(caret.matches(&Version::parse("0.16.3").unwrap()));
    /// assert!(!caret.matches(&Version::parse("0.17.0").unwrap()));
    ///
    /// let exact = VersionReq::parse("=0.16.0").unwrap();
    /// assert!(!exact.matches(&Version::parse("0.16.3").unwrap()));
    ///
    /// assert_eq!(VersionReq::parse("~0.16.0"), Err(VersionReqError::Unsupported));
    /// assert_eq!(VersionReq::parse("sixteen"), Err(VersionReqError::Malformed));
    /// ```
    pub fn parse(text: &str) -> Result<VersionReq, VersionReqError> {
        // `=` followed immediately by a strict version is the exact form.
        // No whitespace is allowed after it: one spelling per meaning.
        if let Some(rest) = text.strip_prefix('=') {
            if let Some(version) = Version::parse(rest) {
                return Ok(VersionReq::Exact(version));
            }
        } else if let Some(version) = Version::parse(text) {
            return Ok(VersionReq::Caret(version));
        }
        if looks_like_unsupported_operator(text) {
            Err(VersionReqError::Unsupported)
        } else {
            Err(VersionReqError::Malformed)
        }
    }

    /// The version the requirement was written against.
    pub fn version(&self) -> &Version {
        match self {
            Self::Caret(version) | Self::Exact(version) => version,
        }
    }

    /// Whether `candidate` satisfies this requirement.
    ///
    /// Two gates, in order:
    ///
    /// 1. **The pre-release gate.** A pre-release candidate is considered
    ///    only if the requirement names a pre-release of the same triple.
    /// 2. **The range.** Exact: equal. Caret: `version <= candidate < upper`,
    ///    where `upper` bumps the left-most non-zero component.
    ///
    /// Gate 1 is what makes the caret upper bound safe to compare without a
    /// pre-release tag: `2.0.0-alpha` sorts below `2.0.0`, but it cannot
    /// reach the range check for requirement `1.2.3`, because `2.0.0` is not
    /// `1.2.3`'s triple.
    pub fn matches(&self, candidate: &Version) -> bool {
        let wanted = self.version();
        if candidate.is_prerelease()
            && !(wanted.is_prerelease() && wanted.triple() == candidate.triple())
        {
            return false;
        }
        match self {
            Self::Exact(version) => candidate == version,
            Self::Caret(version) => {
                candidate >= version
                    && match caret_upper_bound(version) {
                        Some(upper) => candidate.triple() < upper,
                        // `u64::MAX` overflowed: there is no higher version.
                        None => true,
                    }
            }
        }
    }

    /// The requirement as an explicit range, for diagnostics:
    /// `>=0.15.0, <0.16.0` for `"0.15.0"`, `=0.16.0` for `"=0.16.0"`.
    pub fn range(&self) -> String {
        match self {
            Self::Exact(version) => format!("={version}"),
            Self::Caret(version) => match caret_upper_bound(version) {
                Some((major, minor, patch)) => {
                    format!(">={version}, <{major}.{minor}.{patch}")
                }
                None => format!(">={version}"),
            },
        }
    }
}

/// The exclusive upper bound of a caret requirement, as a release triple.
///
/// Bump the left-most non-zero component and zero everything after it. For
/// `0.0.0` there is no non-zero component, so the patch is the one that moves
/// (`^0.0.0` is `>=0.0.0, <0.0.1`, as in Cargo).
///
/// `None` only when that component is `u64::MAX` and cannot be bumped.
fn caret_upper_bound(version: &Version) -> Option<(u64, u64, u64)> {
    if version.major > 0 {
        Some((version.major.checked_add(1)?, 0, 0))
    } else if version.minor > 0 {
        Some((0, version.minor.checked_add(1)?, 0))
    } else {
        Some((0, 0, version.patch.checked_add(1)?))
    }
}

/// Whether a string that failed to parse was written in an operator,
/// wildcard, range or partial-version form that v1 deliberately does not
/// support — as opposed to being a plain typo.
///
/// The difference only changes the error message. Both are refused.
fn looks_like_unsupported_operator(text: &str) -> bool {
    // Comparison and caret/tilde operators, wildcards, and the punctuation
    // that joins comparators into ranges. None of these can appear in a
    // strict version, so their presence is unambiguous.
    const OPERATOR_CHARS: &[char] = &['^', '~', '<', '>', '*', ',', '|'];
    if text.contains(OPERATOR_CHARS) || text.contains(char::is_whitespace) {
        return true;
    }
    // `==1.2.3` is a second operator after the supported one.
    let unprefixed = text.strip_prefix('=').unwrap_or(text);
    if unprefixed.starts_with('=') {
        return true;
    }
    // A wildcard or partial core (`1.x`, `1.2.X`, `1`, `1.2`) is a range in
    // Cargo's grammar. Only the numeric core is inspected: `x` is a perfectly
    // good pre-release identifier (`1.0.0-x`).
    let core = unprefixed.split(['-', '+']).next().unwrap_or("");
    let components: Vec<&str> = core.split('.').collect();
    let wildcard = components.iter().any(|part| matches!(*part, "x" | "X"));
    let partial = components.len() < 3
        && components
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()));
    wildcard || partial
}

impl fmt::Display for VersionReq {
    /// The requirement as written: `0.16.0` or `=0.16.0`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Caret(version) => write!(f, "{version}"),
            Self::Exact(version) => write!(f, "={version}"),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn v(text: &str) -> Version {
        Version::parse(text).unwrap_or_else(|| panic!("{text} should parse"))
    }

    fn req(text: &str) -> VersionReq {
        VersionReq::parse(text).unwrap_or_else(|e| panic!("{text} should parse: {e:?}"))
    }

    // ---- Version::parse ----

    #[test]
    fn parses_strict_versions() {
        assert_eq!(
            v("1.2.3"),
            Version {
                major: 1,
                minor: 2,
                patch: 3,
                pre: vec![]
            }
        );
        assert_eq!(
            v("1.0.0-rc.1").pre,
            vec![PreReleaseId::Alpha("rc".into()), PreReleaseId::Numeric(1)]
        );
        // `-` is legal inside an identifier, and an identifier may start with
        // a digit as long as it is not all digits.
        assert_eq!(
            v("1.0.0-x-y.0a").pre,
            vec![
                PreReleaseId::Alpha("x-y".into()),
                PreReleaseId::Alpha("0a".into())
            ]
        );
        assert_eq!(v("0.0.0").to_string(), "0.0.0");
        assert_eq!(v("10.20.30-alpha.0").to_string(), "10.20.30-alpha.0");
    }

    #[test]
    fn rejects_everything_strict_semver_rejects() {
        for bad in [
            "",
            "1",
            "1.2",
            "1.2.3.4",
            "01.2.3",
            "1.02.3",
            "1.2.03",
            "1.2.3-",
            "1.2.3-01",
            "1.2.3-a..b",
            "1.2.3-rc_1",
            "1.2.3+build",
            "1.2.3-rc.1+build",
            "v1.2.3",
            " 1.2.3",
            "1.2.3 ",
            "+1.2.3",
            "1.2.-3",
            "１.2.3",
            "1.٢.3",
            "18446744073709551616.0.0",
        ] {
            assert!(Version::parse(bad).is_none(), "{bad:?} should be rejected");
        }
        // The largest representable component is still accepted.
        assert_eq!(v("18446744073709551615.0.0").major, u64::MAX);
    }

    // ---- Version ordering (SemVer §11) ----

    #[test]
    fn precedence_follows_semver_section_11() {
        let ordered = [
            "1.0.0-alpha",
            "1.0.0-alpha.1",
            "1.0.0-alpha.beta",
            "1.0.0-beta",
            "1.0.0-beta.2",
            "1.0.0-beta.11",
            "1.0.0-rc.1",
            "1.0.0",
            "1.0.1",
            "1.1.0",
            "2.0.0",
            "10.0.0",
        ];
        for pair in ordered.windows(2) {
            assert!(v(pair[0]) < v(pair[1]), "{} < {}", pair[0], pair[1]);
        }
        assert_eq!(v("1.0.0").cmp(&v("1.0.0")), Ordering::Equal);
    }

    // ---- VersionReq::parse ----

    #[test]
    fn a_bare_version_is_caret_and_equals_is_exact() {
        assert_eq!(req("0.16.0"), VersionReq::Caret(v("0.16.0")));
        assert_eq!(req("=0.16.0"), VersionReq::Exact(v("0.16.0")));
        assert_eq!(req("=1.0.0-rc.1"), VersionReq::Exact(v("1.0.0-rc.1")));
        // Display round-trips the written form.
        assert_eq!(req("0.16.0").to_string(), "0.16.0");
        assert_eq!(req("=0.16.0").to_string(), "=0.16.0");
        assert_eq!(req("=0.16.0").version(), &v("0.16.0"));
    }

    #[test]
    fn operators_wildcards_and_ranges_are_unsupported() {
        for unsupported in [
            "^0.16.0",
            "~0.16.0",
            ">=0.16.0",
            ">0.16.0",
            "<1.0.0",
            "<=1.0.0",
            "*",
            "0.*",
            "1.x",
            "1.2.X",
            "1",
            "1.2",
            "=1.2",
            ">=1.0.0, <2.0.0",
            "1.0.0 || 2.0.0",
            "= 1.0.0",
            "==1.0.0",
            " 1.0.0",
        ] {
            assert_eq!(
                VersionReq::parse(unsupported),
                Err(VersionReqError::Unsupported),
                "{unsupported:?}"
            );
        }
    }

    #[test]
    fn malformed_strings_are_malformed() {
        for malformed in [
            "",
            "not-a-version",
            "01.2.3",
            "=01.2.3",
            "1.2.3+build",
            "1.2.3-",
            "=",
            "v1.2.3",
            "1.2.3.4",
        ] {
            assert_eq!(
                VersionReq::parse(malformed),
                Err(VersionReqError::Malformed),
                "{malformed:?}"
            );
        }
    }

    // ---- VersionReq::matches: the §7.4 caret table ----

    #[test]
    fn caret_at_or_above_one_keeps_the_major() {
        let r = req("1.2.3");
        for yes in ["1.2.3", "1.2.4", "1.9.0", "1.99.99"] {
            assert!(r.matches(&v(yes)), "1.2.3 should match {yes}");
        }
        for no in ["1.2.2", "1.0.0", "2.0.0", "0.9.0"] {
            assert!(!r.matches(&v(no)), "1.2.3 should not match {no}");
        }
        assert_eq!(r.range(), ">=1.2.3, <2.0.0");
    }

    #[test]
    fn caret_below_one_keeps_the_minor() {
        let r = req("0.16.0");
        for yes in ["0.16.0", "0.16.3"] {
            assert!(r.matches(&v(yes)), "0.16.0 should match {yes}");
        }
        for no in ["0.17.0", "0.15.9", "1.0.0", "0.11.0"] {
            assert!(!r.matches(&v(no)), "0.16.0 should not match {no}");
        }
        assert_eq!(r.range(), ">=0.16.0, <0.17.0");

        // The stale pin this phase exists to catch.
        assert!(!req("0.11.0").matches(&v("0.16.0")));
    }

    #[test]
    fn caret_below_zero_point_one_keeps_the_patch() {
        let r = req("0.0.3");
        assert!(r.matches(&v("0.0.3")));
        for no in ["0.0.4", "0.0.2", "0.1.0"] {
            assert!(!r.matches(&v(no)), "0.0.3 should not match {no}");
        }
        assert_eq!(r.range(), ">=0.0.3, <0.0.4");

        // `^0.0.0` is `=0.0.0` in practice, as in Cargo.
        assert!(req("0.0.0").matches(&v("0.0.0")));
        assert!(!req("0.0.0").matches(&v("0.0.1")));
    }

    #[test]
    fn exact_matches_only_itself() {
        let r = req("=0.16.0");
        assert!(r.matches(&v("0.16.0")));
        for no in ["0.16.1", "0.15.0", "0.17.0", "0.16.0-rc.1"] {
            assert!(!r.matches(&v(no)), "=0.16.0 should not match {no}");
        }
        assert_eq!(r.range(), "=0.16.0");
    }

    #[test]
    fn a_caret_bound_at_u64_max_does_not_overflow() {
        let max = format!("{}.0.0", u64::MAX);
        let r = req(&max);
        assert!(r.matches(&v(&max)));
        assert_eq!(r.range(), format!(">={max}"));
        let max_minor = format!("0.{}.0", u64::MAX);
        assert!(req(&max_minor).matches(&v(&format!("0.{}.7", u64::MAX))));
        let max_patch = format!("0.0.{}", u64::MAX);
        assert!(req(&max_patch).matches(&v(&max_patch)));
    }

    // ---- Pre-releases (Cargo's rule) ----

    #[test]
    fn a_release_requirement_never_matches_a_pre_release() {
        let r = req("1.0.0");
        assert!(
            !r.matches(&v("1.1.0-rc.1")),
            "within range, but never asked for"
        );
        assert!(!r.matches(&v("1.0.0-rc.1")), "below the requirement");
        assert!(
            !r.matches(&v("2.0.0-alpha")),
            "below the upper bound, other triple"
        );
        assert!(!req("0.16.0").matches(&v("0.16.1-beta")));
    }

    #[test]
    fn a_pre_release_requirement_opts_into_its_own_triple_only() {
        let r = req("1.0.0-rc.1");
        for yes in ["1.0.0-rc.1", "1.0.0-rc.2", "1.0.0", "1.4.0"] {
            assert!(r.matches(&v(yes)), "1.0.0-rc.1 should match {yes}");
        }
        for no in ["1.0.0-beta", "1.1.0-beta", "2.0.0", "0.9.0"] {
            assert!(!r.matches(&v(no)), "1.0.0-rc.1 should not match {no}");
        }
        assert_eq!(r.range(), ">=1.0.0-rc.1, <2.0.0");

        let exact = req("=1.0.0-rc.1");
        assert!(exact.matches(&v("1.0.0-rc.1")));
        assert!(!exact.matches(&v("1.0.0-rc.2")));
        assert!(!exact.matches(&v("1.0.0")));
    }
}
