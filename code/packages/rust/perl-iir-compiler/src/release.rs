//! Release resolution: the ONE place this crate looks at a release ID.
//!
//! LANG84 C2 forbids the IIR compiler from parsing, ordering or comparing
//! release IDs (`if version >= "5.004"`). Resolution is the exception the
//! spec allows, and it is deliberately dumb: an ID is either an exact row of
//! `code/grammars/perl/releases.csv` or it is not. There is no semantic
//! version ordering anywhere in here, because Perl's inventory interleaves
//! maintenance branches and development releases, so "5.004_72 < 5.005" is
//! not a question this crate can answer correctly — and does not need to.
//!
//! ```text
//!   None ───────────────────────────────► DEFAULT_RELEASE (5.38.0)
//!   Some(id) ──► id in SUPPORTED_RELEASES? ── yes ─► Release(id)
//!                     │ no
//!                     ▼
//!              id in INVENTORY_RELEASES? ── yes ─► ReleaseNotAvailable (pending)
//!                     │ no
//!                     ▼
//!               UnknownRelease { nearest: up to 5 IDs sharing the longest prefix }
//! ```
//!
//! Nothing on the right-hand side ever substitutes a neighbouring release,
//! the unversioned pair, or a "closest" grammar (LANG84 C4.3).

use coding_adventures_perl_parser::{INVENTORY_RELEASES, SUPPORTED_RELEASES};
use std::fmt;

use crate::FrontendError;

/// The longest release text echoed back in a diagnostic. A release ID comes
/// from the caller (a CLI flag, an API argument), so an error must not copy an
/// arbitrarily large string into every message that names it.
const MAX_ECHOED_RELEASE_CHARS: usize = 64;

/// How many inventory IDs an `UnknownRelease` error suggests (LANG84 C4.2).
const MAX_SUGGESTIONS: usize = 5;

/// The one inventory release a compile runs against (LANG84 C4.1).
///
/// A `Release` can only be obtained from [`resolve_release`] or
/// [`DEFAULT_RELEASE`], so holding one is proof that the ID is a
/// non-pending row of `releases.csv` with compiled tables. There is no
/// "unversioned" value any more: since LANG84 phase 4b the IIR path never
/// touches LANG81's unversioned `perl.tokens`/`perl.grammar` (C4.5). Those
/// files stay for `perl-lexer::tokenize_perl` and `perl-parser::parse_perl`,
/// which are not on the IIR path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Release(&'static str);

impl Release {
    /// The exact inventory ID, such as `"5.38.0"` or `"5.004_72"`.
    pub fn id(self) -> &'static str {
        self.0
    }
}

impl fmt::Display for Release {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Perl {}", self.0)
    }
}

/// The inventory ID a compile uses when the caller names no release.
///
/// `5.38.0` is the first stable release of the line LANG81's pilot was
/// written against, and its pair is grounded in the `v5.38.0` tag's own
/// `perly.y`, `toke.c`, `keywords.h` and `parser.h` blobs (see the header of
/// `code/grammars/perl/perl5.38.0.tokens`). A test pins that this is a
/// non-pending inventory row with compiled tables.
pub const DEFAULT_RELEASE_ID: &str = "5.38.0";

/// What a compile uses when the caller names no release (LANG84 C4.4).
///
/// Phase 4a kept this on the unversioned LANG81 pair; phase 4b moved it to
/// the inventory release [`DEFAULT_RELEASE_ID`].
pub const DEFAULT_RELEASE: Release = Release(DEFAULT_RELEASE_ID);

/// Resolve a caller's release request to exactly one grammar pair.
///
/// `None` means [`DEFAULT_RELEASE`]. `Some("")` is an error, not a default.
pub fn resolve_release(requested: Option<&str>) -> Result<Release, FrontendError> {
    let Some(requested) = requested else {
        return Ok(DEFAULT_RELEASE);
    };
    if let Some(id) = SUPPORTED_RELEASES.iter().find(|id| **id == requested) {
        return Ok(Release(id));
    }
    if INVENTORY_RELEASES.contains(&requested) {
        return Err(FrontendError::ReleaseNotAvailable {
            release: echo(requested),
            stage: "pending",
        });
    }
    Err(FrontendError::UnknownRelease {
        release: echo(requested),
        nearest: nearest_releases(requested),
    })
}

/// Bound and neutralize a caller-supplied release string before it enters a
/// diagnostic. Only its first 64 characters are kept, and each is escaped
/// with `char::escape_debug`, so a control character (a newline that would
/// forge a log line, an ESC that would drive a terminal) arrives as visible
/// text such as `\n` or `\u{1b}` rather than as the character itself.
fn echo(requested: &str) -> String {
    let mut chars = requested.chars();
    let head: String = chars
        .by_ref()
        .take(MAX_ECHOED_RELEASE_CHARS)
        .flat_map(char::escape_debug)
        .collect();
    if chars.next().is_some() {
        format!("{head}…")
    } else {
        head
    }
}

/// Up to five inventory IDs that share the longest common prefix with
/// `requested`, in inventory order; empty when nothing shares even one
/// character. This is string-prefix matching, not version arithmetic:
/// `5.38` suggests `5.38.0-RC1`, `5.38.0-RC2`, `5.38.0`, … because they
/// start with the same text.
fn nearest_releases(requested: &str) -> Vec<&'static str> {
    let shared = |id: &str| {
        id.chars()
            .zip(requested.chars())
            .take_while(|(a, b)| a == b)
            .count()
    };
    let best = INVENTORY_RELEASES
        .iter()
        .map(|id| shared(id))
        .max()
        .unwrap_or(0);
    if best == 0 {
        return Vec::new();
    }
    INVENTORY_RELEASES
        .iter()
        .copied()
        .filter(|id| shared(id) == best)
        .take(MAX_SUGGESTIONS)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_release_is_the_default_and_the_default_is_an_inventory_row() {
        assert_eq!(resolve_release(None).unwrap(), DEFAULT_RELEASE);
        assert_eq!(DEFAULT_RELEASE.id(), "5.38.0");
        // LANG84 C4.4: the default is an exact, checked inventory ID, and
        // naming it explicitly resolves to the very same release.
        assert!(SUPPORTED_RELEASES.contains(&DEFAULT_RELEASE_ID));
        assert_eq!(resolve_release(Some(DEFAULT_RELEASE_ID)).unwrap(), DEFAULT_RELEASE);
        assert_eq!(DEFAULT_RELEASE.to_string(), "Perl 5.38.0");
    }

    #[test]
    fn every_supported_release_resolves_to_itself() {
        for id in SUPPORTED_RELEASES {
            let release = resolve_release(Some(id)).unwrap();
            assert_eq!(release.id(), *id);
            assert_eq!(release.to_string(), format!("Perl {id}"));
        }
    }

    #[test]
    fn a_pending_release_is_known_but_not_available() {
        let error = resolve_release(Some("1.001")).unwrap_err();
        assert_eq!(
            error,
            FrontendError::ReleaseNotAvailable {
                release: "1.001".into(),
                stage: "pending"
            }
        );
        // 5.38.1 sits between two checked pairs (5.38.0 and 5.38.2) whose
        // toke.c blobs are identical, yet it is pending: it must not fall
        // back to either neighbour.
        assert!(matches!(
            resolve_release(Some("5.38.1")),
            Err(FrontendError::ReleaseNotAvailable { .. })
        ));
    }

    #[test]
    fn an_empty_or_unknown_release_is_an_error_with_suggestions() {
        let FrontendError::UnknownRelease { release, nearest } =
            resolve_release(Some("")).unwrap_err()
        else {
            panic!("empty release must be UnknownRelease");
        };
        assert_eq!(release, "");
        assert!(nearest.is_empty());

        let FrontendError::UnknownRelease { nearest, .. } =
            resolve_release(Some("5.38")).unwrap_err()
        else {
            panic!("a release-line shorthand is not an inventory ID");
        };
        assert_eq!(nearest.len(), MAX_SUGGESTIONS);
        assert!(
            nearest.iter().all(|id| id.starts_with("5.38")),
            "{nearest:?}"
        );

        let FrontendError::UnknownRelease { nearest, .. } =
            resolve_release(Some("ruby")).unwrap_err()
        else {
            panic!("a non-Perl ID is unknown");
        };
        assert!(nearest.is_empty());
    }

    #[test]
    fn an_oversized_release_is_truncated_in_the_error() {
        let huge = "9".repeat(100_000);
        let FrontendError::UnknownRelease { release, .. } =
            resolve_release(Some(&huge)).unwrap_err()
        else {
            panic!("expected UnknownRelease");
        };
        assert_eq!(release.chars().count(), MAX_ECHOED_RELEASE_CHARS + 1);
        assert!(release.ends_with('…'));
    }

    #[test]
    fn control_characters_in_a_release_are_escaped_in_the_error() {
        let FrontendError::UnknownRelease { release, .. } =
            resolve_release(Some("x\nINFO: ok\u{1b}[2J")).unwrap_err()
        else {
            panic!("expected UnknownRelease");
        };
        assert!(!release.chars().any(char::is_control), "{release:?}");
        assert_eq!(release, "x\\nINFO: ok\\u{1b}[2J");
        let message = resolve_release(Some("a\rb")).unwrap_err().to_string();
        assert!(!message.chars().any(char::is_control), "{message:?}");
    }
}
