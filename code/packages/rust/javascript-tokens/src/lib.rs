//! Shared JavaScript tokens and ES version enum.
//!
//! # What this crate is for
//!
//! `javascript-tokens` holds the **backend-agnostic** types used across the
//! JavaScript pipeline — anything that more than one crate downstream needs
//! to agree on. It deliberately has no dependencies (not even `serde` for
//! v1) so it sits at the bottom of the dependency graph and can be pulled in
//! by everything else without creating cycles.
//!
//! Per [CLOC02](../../specs/CLOC02-javascript-ast.md), the JS frontend is
//! shared between two backends:
//!
//! 1. The Closure-Compiler-clone (JS → optimised JS).
//! 2. The future V8-in-Rust clone (JS → bytecode → VM).
//!
//! Both backends consume the same parser AST, which in turn references the
//! types here.
//!
//! # What's in v1
//!
//! Just [`EsVersion`] — the enum naming every ECMAScript edition that has a
//! grammar file under `code/grammars/ecmascript/`. The lexer and parser
//! today take version strings like `"es2025"`; this enum is the typed
//! replacement that downstream crates will migrate to.
//!
//! A full `TokenKind` enum (covering every token kind from `NAME` through
//! `OPTIONAL_CHAIN` and the template-literal groups) is a follow-up PR; it
//! will share the same crate.

use std::fmt;
use std::str::FromStr;

/// Every ECMAScript edition that has a grammar file under
/// `code/grammars/ecmascript/`.
///
/// ES2 and ES5.1 are not separate variants — per the
/// `versioned-ecmascript-typescript-grammars.md` spec they alias ES1 and ES5
/// respectively (editorial changes only), and the grammar tree omits them.
///
/// The string form (returned by [`EsVersion::as_str`]) matches the grammar
/// file basenames exactly: `"es1"`, `"es3"`, `"es5"`, `"es2015"` through
/// `"es2025"`. That's the same identifier the lexer's `SUPPORTED_VERSIONS`
/// list uses, so call sites can move between the two without renaming.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EsVersion {
    Es1,
    Es3,
    Es5,
    Es2015,
    Es2016,
    Es2017,
    Es2018,
    Es2019,
    Es2020,
    Es2021,
    Es2022,
    Es2023,
    Es2024,
    Es2025,
}

impl EsVersion {
    /// The most recent edition we have a grammar for.
    ///
    /// Used as the default when callers don't specify a version. This is
    /// `Es2025` today; future grammar additions bump it.
    pub const fn latest() -> Self {
        EsVersion::Es2025
    }

    /// The basename of the matching grammar file under
    /// `code/grammars/ecmascript/`. Matches what the lexer/parser
    /// `SUPPORTED_VERSIONS` arrays contain — call sites can pass this
    /// directly to `tokenize_javascript` / `parse_javascript`.
    pub const fn as_str(self) -> &'static str {
        match self {
            EsVersion::Es1 => "es1",
            EsVersion::Es3 => "es3",
            EsVersion::Es5 => "es5",
            EsVersion::Es2015 => "es2015",
            EsVersion::Es2016 => "es2016",
            EsVersion::Es2017 => "es2017",
            EsVersion::Es2018 => "es2018",
            EsVersion::Es2019 => "es2019",
            EsVersion::Es2020 => "es2020",
            EsVersion::Es2021 => "es2021",
            EsVersion::Es2022 => "es2022",
            EsVersion::Es2023 => "es2023",
            EsVersion::Es2024 => "es2024",
            EsVersion::Es2025 => "es2025",
        }
    }

    /// Every variant in chronological order — useful for iterating across
    /// the cascade in tests.
    pub const ALL: &'static [EsVersion] = &[
        EsVersion::Es1,
        EsVersion::Es3,
        EsVersion::Es5,
        EsVersion::Es2015,
        EsVersion::Es2016,
        EsVersion::Es2017,
        EsVersion::Es2018,
        EsVersion::Es2019,
        EsVersion::Es2020,
        EsVersion::Es2021,
        EsVersion::Es2022,
        EsVersion::Es2023,
        EsVersion::Es2024,
        EsVersion::Es2025,
    ];
}

impl fmt::Display for EsVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Default for EsVersion {
    fn default() -> Self {
        EsVersion::latest()
    }
}

/// Parsed back from the same strings [`EsVersion::as_str`] emits.
///
/// Anything else (`""`, `"es2"`, `"es5.1"`, `"latest"`, …) returns
/// [`UnknownEsVersion`]. We deliberately do not accept the empty string —
/// the legacy "generic" version retired by PR #3785 is not a valid
/// `EsVersion` and never will be.
impl FromStr for EsVersion {
    type Err = UnknownEsVersion;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "es1" => Ok(EsVersion::Es1),
            "es3" => Ok(EsVersion::Es3),
            "es5" => Ok(EsVersion::Es5),
            "es2015" => Ok(EsVersion::Es2015),
            "es2016" => Ok(EsVersion::Es2016),
            "es2017" => Ok(EsVersion::Es2017),
            "es2018" => Ok(EsVersion::Es2018),
            "es2019" => Ok(EsVersion::Es2019),
            "es2020" => Ok(EsVersion::Es2020),
            "es2021" => Ok(EsVersion::Es2021),
            "es2022" => Ok(EsVersion::Es2022),
            "es2023" => Ok(EsVersion::Es2023),
            "es2024" => Ok(EsVersion::Es2024),
            "es2025" => Ok(EsVersion::Es2025),
            other => Err(UnknownEsVersion(other.to_string())),
        }
    }
}

/// Error returned by [`EsVersion::from_str`] when the input doesn't match
/// any known ES edition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownEsVersion(pub String);

impl fmt::Display for UnknownEsVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unknown ECMAScript version {:?}; valid values are {}",
            self.0,
            EsVersion::ALL
                .iter()
                .map(|v| format!("{:?}", v.as_str()))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

impl std::error::Error for UnknownEsVersion {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latest_is_es2025() {
        assert_eq!(EsVersion::latest(), EsVersion::Es2025);
    }

    #[test]
    fn default_is_latest() {
        assert_eq!(EsVersion::default(), EsVersion::latest());
    }

    #[test]
    fn as_str_matches_grammar_filenames() {
        // The lexer's SUPPORTED_VERSIONS array (post-PR #3785) — verbatim.
        let expected = [
            "es1", "es3", "es5", "es2015", "es2016", "es2017", "es2018",
            "es2019", "es2020", "es2021", "es2022", "es2023", "es2024",
            "es2025",
        ];
        let actual: Vec<&str> = EsVersion::ALL.iter().map(|v| v.as_str()).collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn round_trip_through_strings() {
        for v in EsVersion::ALL {
            assert_eq!(EsVersion::from_str(v.as_str()).unwrap(), *v);
        }
    }

    #[test]
    fn empty_string_is_rejected() {
        // The legacy "generic" version retired by PR #3785 must not parse.
        assert!(EsVersion::from_str("").is_err());
    }

    #[test]
    fn unknown_strings_are_rejected() {
        for bad in &["es2", "es5.1", "latest", "ES2025", "es2026", " es2025"] {
            assert!(
                EsVersion::from_str(bad).is_err(),
                "expected {:?} to be rejected",
                bad
            );
        }
    }

    #[test]
    fn display_matches_as_str() {
        assert_eq!(format!("{}", EsVersion::Es2020), "es2020");
    }

    #[test]
    fn unknown_error_message_includes_valid_set() {
        let err = EsVersion::from_str("nope").unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("\"nope\""), "msg = {msg}");
        assert!(msg.contains("\"es2025\""), "msg = {msg}");
    }

    #[test]
    fn ord_is_chronological() {
        assert!(EsVersion::Es1 < EsVersion::Es3);
        assert!(EsVersion::Es5 < EsVersion::Es2015);
        assert!(EsVersion::Es2015 < EsVersion::Es2025);
    }
}
