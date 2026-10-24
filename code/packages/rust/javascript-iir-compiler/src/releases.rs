//! # Which ECMAScript edition a compile uses (LANG84 phase 6d)
//!
//! LANG84 makes every published ECMAScript edition a selectable thing:
//! `compile_source_at(src, Some("es2017"), …)` lexes and parses with
//! `es2017.tokens` / `es2017.grammar` and nothing else.
//!
//! ```text
//!   request ──► resolve_release ──► (inventory ID, EsVersion)
//!                                         │
//!            parse_javascript_cst_bounded(source, EsVersion)
//! ```
//!
//! ## No silent fallback (LANG84 C4)
//!
//! A request resolves to exactly one `releases.csv` row or to a typed error:
//!
//! | Request                       | Result                                   |
//! |-------------------------------|------------------------------------------|
//! | `None`                        | [`DEFAULT_RELEASE`] (`es2020`)           |
//! | `Some("")`                    | `UnknownRelease` (empty is not default)  |
//! | `Some("es2017")` (claims)     | that edition's pair (es2016 … es2025)    |
//! | `Some("es5")` (row, no claim) | `ReleaseNotAvailable`                    |
//! | `Some("js1.5")` (no pair)     | `ReleaseNotAvailable`                    |
//! | `Some("ES2017")`, `Some("2017")` | `UnknownRelease` + suggestions        |
//!
//! A neighbouring edition, the unversioned `ecmascript` pair, or "the
//! latest" is never substituted. IDs are compared byte for byte; ordering
//! comes from the inventory, never from parsing the ID.
//!
//! ## Why the tables below are written out
//!
//! Ruby and Python generate their inventory tables from `releases.csv` with
//! grammar-tools. The ECMAScript lexer and parser predate that generator and
//! key their compiled tables by [`EsVersion`], so this crate lists the rows
//! itself and `tests/release_subsets.rs` checks the lists against the CSV:
//! a row added, removed or re-claimed there fails the test until this file
//! agrees.

use coding_adventures_javascript_tokens::EsVersion;

use crate::frontend_error::FrontendError;
use crate::profiles::{ReleaseProfile, ECMASCRIPT};

/// The subset the release path lowers, as `code/grammars/ecmascript/subsets.csv`
/// names it.
pub const SUBSET: &str = "es-number-log";

/// The release used when a caller names none. An exact inventory ID.
///
/// es2020 is the edition the IIR path parsed before releases were
/// selectable, so `compile_source` behaves exactly as it always has. Its
/// row claims `es-number-log` (LANG84 C4.4).
pub const DEFAULT_RELEASE: &str = "es2020";

/// Every row of `releases.csv`, in inventory order.
pub(crate) const INVENTORY: &[&str] = &[
    "js1.0", "js1.1", "es1", "js1.2", "es2", "js1.3", "js1.4", "es3", "js1.5", "js1.6", "js1.7",
    "js1.8", "js1.8.1", "es5", "js1.8.2", "js1.8.5", "es5.1", "es2015", "es2016", "es2017",
    "es2018", "es2019", "es2020", "es2021", "es2022", "es2023", "es2024", "es2025", "es2026",
];

/// The rows that claim [`SUBSET`], in inventory order, with the edition
/// whose pair each one selects.
const CLAIMING: &[(&str, EsVersion)] = &[
    ("es2016", EsVersion::Es2016),
    ("es2017", EsVersion::Es2017),
    ("es2018", EsVersion::Es2018),
    ("es2019", EsVersion::Es2019),
    ("es2020", EsVersion::Es2020),
    ("es2021", EsVersion::Es2021),
    ("es2022", EsVersion::Es2022),
    ("es2023", EsVersion::Es2023),
    ("es2024", EsVersion::Es2024),
    ("es2025", EsVersion::Es2025),
];

/// The claiming release IDs, in inventory order.
const CLAIMING_IDS: &[&str] = &[
    "es2016", "es2017", "es2018", "es2019", "es2020", "es2021", "es2022", "es2023",
    "es2024", "es2025",
];

/// Longest release ID echoed back in an error; longer requests are cut so a
/// hostile string cannot be reflected at length.
const MAX_RELEASE_ID_BYTES: usize = 64;

/// The releases that claim [`SUBSET`], in inventory order.
pub fn supported_releases() -> &'static [&'static str] {
    CLAIMING_IDS
}

/// The profile a release compiles under, or `None` for a row that claims no
/// subset. Exact IDs only.
pub fn profile_for(release: &str) -> Option<&'static ReleaseProfile> {
    CLAIMING_IDS.contains(&release).then_some(&ECMASCRIPT)
}

/// Resolve a request to the exact inventory ID and the edition it selects.
pub fn resolve_release(release: Option<&str>) -> Result<(&'static str, EsVersion), FrontendError> {
    let requested = release.unwrap_or(DEFAULT_RELEASE);
    if requested.len() > MAX_RELEASE_ID_BYTES {
        return Err(FrontendError::UnknownRelease {
            requested: echo(requested),
            suggestions: Vec::new(),
        });
    }
    if let Some(&(id, version)) = CLAIMING.iter().find(|(id, _)| *id == requested) {
        return Ok((id, version));
    }
    match INVENTORY.iter().find(|id| **id == requested) {
        Some(id) => Err(FrontendError::ReleaseNotAvailable {
            release: (*id).to_string(),
            stage: not_available_reason(id).to_string(),
        }),
        None => Err(FrontendError::UnknownRelease {
            requested: echo(requested),
            suggestions: suggestions(requested),
        }),
    }
}

/// Why an inventory row that claims nothing cannot compile the subset.
fn not_available_reason(release: &str) -> &'static str {
    match release {
        "es1" => "no es-number-log claim: es1 has no let/const (grammar_stage none)",
        "es2015" => {
            "no es-number-log claim: its source evidence is pending (the Ecma PDF's \
             SHA-256 is not recorded, so the pair cannot reach stage G; grammar_stage none)"
        }
        "es3" | "es5" => {
            "no es-number-log claim: its .tokens has no F10 regex/division table yet \
             (grammar_stage none, LANG84 phase 6d)"
        }
        _ => "no grammar pair: grammar_stage none",
    }
}

/// Up to five inventory IDs sharing the longest common prefix with the
/// request, in inventory order; none when nothing shares even one byte.
fn suggestions(requested: &str) -> Vec<String> {
    let common = |id: &str| {
        id.bytes()
            .zip(requested.bytes())
            .take_while(|(a, b)| a == b)
            .count()
    };
    let best = INVENTORY.iter().map(|id| common(id)).max().unwrap_or(0);
    if best == 0 {
        return Vec::new();
    }
    INVENTORY
        .iter()
        .filter(|id| common(id) == best)
        .take(5)
        .map(|id| (*id).to_string())
        .collect()
}

/// Escape control characters (and quotes) so an echoed request cannot
/// forge terminal output or log lines, then bound the *escaped* text.
///
/// Escaping comes first because it grows the text: one ESC byte becomes the
/// six bytes `\u{1b}`. Truncating first and escaping after (the old order)
/// let 64 ESC bytes echo as 384. Each character's escape is kept or dropped
/// whole, so a cut never leaves half an escape like `\u{1`:
///
/// | request             | echoed                                  |
/// |---------------------|-----------------------------------------|
/// | `es201`             | `es201`                                 |
/// | 100 × ESC           | 10 × `\u{1b}` then `...` (63 + 3 bytes) |
/// | 200 × `é`           | 32 × `é` then `...` (64 + 3 bytes)      |
fn echo(text: &str) -> String {
    let mut shown = String::new();
    for c in text.chars() {
        let escaped: String = c.escape_debug().collect();
        if shown.len() + escaped.len() > MAX_RELEASE_ID_BYTES {
            shown.push_str("...");
            return shown;
        }
        shown.push_str(&escaped);
    }
    shown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claiming_tables_agree_and_follow_inventory_order() {
        let ids: Vec<&str> = CLAIMING.iter().map(|(id, _)| *id).collect();
        assert_eq!(ids, CLAIMING_IDS);
        for (id, version) in CLAIMING {
            // The ID is the edition's own spelling: no mapping can drift.
            assert_eq!(*id, version.as_str());
        }
        let positions: Vec<usize> = ids
            .iter()
            .map(|id| INVENTORY.iter().position(|row| row == id).unwrap())
            .collect();
        assert!(positions.windows(2).all(|w| w[0] < w[1]), "{positions:?}");
        assert!(CLAIMING_IDS.contains(&DEFAULT_RELEASE));
    }

    #[test]
    fn requests_resolve_exactly_or_fail_typed() {
        assert_eq!(resolve_release(None).unwrap(), ("es2020", EsVersion::Es2020));
        assert_eq!(resolve_release(Some("es2017")).unwrap(), ("es2017", EsVersion::Es2017));
        for unclaimed in ["es1", "es3", "es5", "es5.1", "es2", "es2015", "es2026", "js1.5", "js1.8.5"] {
            let error = resolve_release(Some(unclaimed)).unwrap_err();
            assert_eq!(error.category(), "ReleaseNotAvailable", "{unclaimed}");
            assert_eq!(error.release(), Some(unclaimed));
        }
        for unknown in ["", "ES2017", "2017", "es2017 ", " es2017", "es20170", "latest", "es4"] {
            let error = resolve_release(Some(unknown)).unwrap_err();
            assert_eq!(error.category(), "UnknownRelease", "{unknown:?}");
            assert_eq!(error.release(), None);
        }
    }

    #[test]
    fn unknown_requests_are_escaped_truncated_and_suggest_neighbours() {
        let error = resolve_release(Some("es201")).unwrap_err();
        let FrontendError::UnknownRelease { suggestions, .. } = &error else {
            panic!("{error}");
        };
        assert_eq!(suggestions, &["es2015", "es2016", "es2017", "es2018", "es2019"]);
        // Suggestions list inventory rows, claiming or not: es2015 is a row.
        let error = resolve_release(Some("e\u{1b}[2J\n")).unwrap_err();
        let shown = error.to_string();
        assert!(!shown.contains('\u{1b}') && !shown.contains('\n'), "{shown}");
        let long = "é".repeat(200);
        let error = resolve_release(Some(&long)).unwrap_err();
        let FrontendError::UnknownRelease { requested, suggestions } = &error else {
            panic!("{error}");
        };
        assert!(requested.len() <= MAX_RELEASE_ID_BYTES + 3, "{requested}");
        assert!(suggestions.is_empty());
        assert!(suggestions_for_nothing_shared());
    }

    /// Review L2: escaping grows text, so the bound applies to the escaped
    /// form, and no escape sequence is split.
    #[test]
    fn echoed_requests_are_bounded_after_escaping() {
        for request in ["\u{1b}".repeat(100), "\u{1b}".repeat(64), "\n\u{0}".repeat(30), "é".repeat(200)] {
            let error = resolve_release(Some(&request)).unwrap_err();
            let FrontendError::UnknownRelease { requested, .. } = &error else {
                panic!("{error}");
            };
            assert!(requested.len() <= MAX_RELEASE_ID_BYTES + 3, "{} bytes: {requested}", requested.len());
            assert!(!requested.contains('\u{1b}') && !requested.contains('\n'), "{requested}");
            // Every `\u{` that was started is closed: no half escapes.
            assert_eq!(requested.matches("\\u{").count(), requested.matches('}').count(), "{requested}");
            assert!(requested.ends_with("..."), "{requested}");
        }
        assert_eq!(echo("es201"), "es201");
    }

    fn suggestions_for_nothing_shared() -> bool {
        suggestions("zzz").is_empty()
    }
}
