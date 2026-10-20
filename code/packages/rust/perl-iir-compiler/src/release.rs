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
//!   None ───────────────────────────────► DEFAULT_RELEASE
//!   Some(id) ──► id in SUPPORTED_RELEASES? ── yes ─► Release::Inventory(id)
//!                     │ no
//!                     ▼
//!              id in INVENTORY?          ── yes ─► ReleaseNotAvailable (pending)
//!                     │ no
//!                     ▼
//!               UnknownRelease { nearest: up to 5 IDs sharing the longest prefix }
//! ```
//!
//! Nothing on the right-hand side ever substitutes a neighbouring release,
//! the unversioned pair, or a "closest" grammar (LANG84 C4.3).

use coding_adventures_perl_parser::{INVENTORY, SUPPORTED_RELEASES};
use std::fmt;

use crate::FrontendError;

/// The longest release text echoed back in a diagnostic. A release ID comes
/// from the caller (a CLI flag, an API argument), so an error must not copy an
/// arbitrarily large string into every message that names it.
const MAX_ECHOED_RELEASE_CHARS: usize = 64;

/// How many inventory IDs an `UnknownRelease` error suggests (LANG84 C4.2).
const MAX_SUGGESTIONS: usize = 5;

/// The grammar pair a compile ran against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Release {
    /// The unversioned `perl.tokens`/`perl.grammar` pair: LANG81's bounded
    /// Perl 5.38 execution grammar. Not an inventory row. It stays the
    /// default until a `perl5.38.0` pair grounded in tag `v5.38.0` replaces
    /// it (LANG84 "Python and Perl: make release selection real"), and it is
    /// reachable only by naming no release.
    Unversioned,
    /// Exactly one non-pending row of `releases.csv`.
    Inventory(&'static str),
}

impl Release {
    /// The inventory ID, or `None` for the unversioned pair.
    pub fn id(self) -> Option<&'static str> {
        match self {
            Release::Unversioned => None,
            Release::Inventory(id) => Some(id),
        }
    }
}

impl fmt::Display for Release {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Release::Unversioned => write!(f, "unversioned LANG81 pair"),
            Release::Inventory(id) => write!(f, "Perl {id}"),
        }
    }
}

/// What a compile uses when the caller names no release (LANG84 C4.4).
///
/// LANG84 phase 4a keeps this on the unversioned pair. Phase 4b moves it to an
/// exact inventory ID once a release pair replaces the LANG81 grammar.
pub const DEFAULT_RELEASE: Release = Release::Unversioned;

/// Resolve a caller's release request to exactly one grammar pair.
///
/// `None` means [`DEFAULT_RELEASE`]. `Some("")` is an error, not a default.
pub fn resolve_release(requested: Option<&str>) -> Result<Release, FrontendError> {
    let Some(requested) = requested else {
        return Ok(DEFAULT_RELEASE);
    };
    if let Some(id) = SUPPORTED_RELEASES.iter().find(|id| **id == requested) {
        return Ok(Release::Inventory(id));
    }
    if inventory_ids().any(|id| id == requested) {
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

/// Every inventory release ID, pending rows included, in inventory order.
/// The generated `INVENTORY` pairs each ID with its `grammar_stage`; release
/// resolution needs only the ID (a pair's presence is SUPPORTED_RELEASES).
fn inventory_ids() -> impl Iterator<Item = &'static str> {
    INVENTORY.iter().map(|(id, _stage)| *id)
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
    let best = inventory_ids().map(shared).max().unwrap_or(0);
    if best == 0 {
        return Vec::new();
    }
    inventory_ids()
        .filter(|id| shared(id) == best)
        .take(MAX_SUGGESTIONS)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_release_is_the_default_and_the_default_is_unversioned() {
        assert_eq!(resolve_release(None).unwrap(), DEFAULT_RELEASE);
        assert_eq!(DEFAULT_RELEASE, Release::Unversioned);
        assert_eq!(DEFAULT_RELEASE.id(), None);
    }

    #[test]
    fn every_supported_release_resolves_to_itself() {
        for id in SUPPORTED_RELEASES {
            let release = resolve_release(Some(id)).unwrap();
            assert_eq!(release.id(), Some(*id));
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
        // 5.38.0 is in the inventory but its pair is a later phase: it must
        // not fall back to 5.38.2, which is a checked pair with equal text.
        assert!(matches!(
            resolve_release(Some("5.38.0")),
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
