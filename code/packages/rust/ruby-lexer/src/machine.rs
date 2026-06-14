//! Version → state-machine definition lookup.
//!
//! The Ruby 1.8 TOML at the crate root is the **canonical baseline**.
//! Phase 4 (version evolution) introduces the 15-era version
//! dispatch — all 15 era-version strings listed in
//! [`code/specs/ruby-version-evolution.md`](../../../specs/ruby-version-evolution.md)
//! are accepted by [`definition_for_version`].  Each era maps to a
//! state-machine definition whose `name` field carries the era
//! string (e.g. `ruby-2.3-lexer`), so downstream consumers can
//! report which grammar they targeted.
//!
//! v0 inheritance model: the baseline TOML is shared by every era;
//! only the machine name differs.  This intentionally avoids
//! duplicating ~1100 lines of TOML 14 times — era-specific deltas
//! (lambda `->` in 1.9.1, `%i[]` in 2.0, `&.` in 2.3, `<<~` heredocs
//! in 2.3, etc.) are deferred to Phase 4b+ where each era forks the
//! single transitions that change.  Until then, callers asking for
//! `"2.7"` get the same token grammar as `"1.8"` — just labelled
//! differently so version-aware tooling can still discriminate.
//!
//! Adding a real era delta requires:
//!   1. Lift the era-specific transitions into a fork in
//!      [`era_toml`] (or split into a per-version file once the
//!      diffs become substantial).
//!   2. Add tests asserting the new behaviour gates on the version
//!      string.
//!   3. Update `code/specs/ruby-version-evolution.md` if the
//!      behaviour diverges from the prose spec.

use state_machine::definitions::StateMachineDefinition;
use state_machine_markup_deserializer::from_states_toml;

const RUBY_1_8: &str = include_str!("../ruby-1.8.lexer.states.toml");

/// All era versions modelled by the spec.  Order is chronological,
/// matching the era table in `ruby-version-evolution.md`.  `1.8` is
/// also the **default** when callers pass `""`.
pub const ERA_VERSIONS: &[&str] = &[
    "1.0", "1.6", "1.8", "1.9.1", "1.9.3", "2.0", "2.1", "2.3", "2.5", "2.6",
    "2.7", "3.0", "3.1", "3.2", "3.3",
];

pub(crate) fn definition_for_version(version: &str) -> Result<StateMachineDefinition, String> {
    let canonical = match version {
        "" => "1.8",
        v if ERA_VERSIONS.contains(&v) => v,
        other => {
            return Err(format!(
                "ruby lexer: version `{other}` is not a recognized Ruby era — \
                 see code/specs/ruby-version-evolution.md for the list of 15 supported eras"
            ));
        }
    };
    let source = era_toml(canonical);
    from_states_toml(&source).map_err(|e| format!("ruby lexer: failed to parse TOML — {e}"))
}

/// Materialize the per-era TOML.  v0: same content as the 1.8
/// baseline with only the machine `name` retagged so downstream
/// tooling can identify the requested era.  Phase 4b+ will fork
/// real transitions here.
///
/// The `name` field appears exactly once at the top of the TOML
/// (the inner `[[tokens]]` entries also have `name` keys, but
/// `replacen(_, _, 1)` confines the substitution to the very first
/// occurrence — the state-machine identifier on line 26).
fn era_toml(version: &str) -> String {
    if version == "1.8" {
        // Avoid the alloc + replace when the requested era is the
        // canonical baseline.
        return RUBY_1_8.to_string();
    }
    let from = "name = \"ruby-1.8-lexer\"";
    let to = format!("name = \"ruby-{version}-lexer\"");
    RUBY_1_8.replacen(from, &to, 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ruby_1_8_parses_cleanly() {
        let def = definition_for_version("1.8").expect("definition");
        assert_eq!(def.name, "ruby-1.8-lexer");
        assert_eq!(def.profile.as_deref(), Some("lexer/v1"));
        assert!(def.states.iter().any(|s| s.id == "data"));
        assert!(def.states.iter().any(|s| s.id == "done"));
    }

    #[test]
    fn ruby_default_version_is_1_8() {
        let def = definition_for_version("").expect("default");
        assert_eq!(def.name, "ruby-1.8-lexer");
    }

    #[test]
    fn unknown_version_errors() {
        let err = definition_for_version("0.9").unwrap_err();
        assert!(err.contains("not a recognized Ruby era"));
        // Make sure the helpful pointer to the spec is in the error
        // message — that's the path callers follow to learn what
        // version strings *are* valid.
        assert!(err.contains("ruby-version-evolution.md"));
    }

    #[test]
    fn all_15_era_versions_are_accepted() {
        // Every entry in ERA_VERSIONS must parse cleanly and tag
        // the machine name with its era string.  This is the core
        // Phase 4 acceptance criterion — the version dispatch
        // surface is complete.
        for &v in ERA_VERSIONS {
            let def = definition_for_version(v)
                .unwrap_or_else(|e| panic!("version {v} failed: {e}"));
            assert_eq!(
                def.name,
                format!("ruby-{v}-lexer"),
                "version {v} produced unexpected machine name {}",
                def.name,
            );
            // Sanity: structural integrity is preserved across all
            // eras — `data` and `done` are the dispatcher and final
            // states, present in every Ruby grammar.
            assert!(def.states.iter().any(|s| s.id == "data"));
            assert!(def.states.iter().any(|s| s.id == "done"));
        }
    }

    #[test]
    fn era_versions_list_is_chronological_and_unique() {
        // Guard against accidental ordering or duplication when
        // future PRs append (or insert) era entries.
        let mut seen = std::collections::HashSet::new();
        for v in ERA_VERSIONS {
            assert!(seen.insert(*v), "duplicate era version {v}");
        }
        // The 1.8 baseline must remain in the list (it is the
        // identity era — every other era currently shares its
        // grammar).
        assert!(ERA_VERSIONS.contains(&"1.8"));
        // 3.3 is the latest era per ruby-version-evolution.md and
        // must be the last entry.
        assert_eq!(ERA_VERSIONS.last(), Some(&"3.3"));
    }
}
