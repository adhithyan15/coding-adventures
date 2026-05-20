//! Version → state-machine definition lookup.
//!
//! The TOML state-machine source lives at the crate root (one file
//! per Ruby version era).  We embed the bytes via `include_str!`
//! and parse them lazily on first use through
//! `state_machine_markup_deserializer::from_states_toml`.
//!
//! Adding a new version requires:
//!   1. Drop a `ruby-<ver>.lexer.states.toml` at the crate root.
//!   2. Add it to the match arm in [`definition_for_version`].
//!   3. Update `code/specs/ruby-version-evolution.md`.

use state_machine::definitions::StateMachineDefinition;
use state_machine_markup_deserializer::from_states_toml;

const RUBY_1_8: &str = include_str!("../ruby-1.8.lexer.states.toml");

pub(crate) fn definition_for_version(version: &str) -> Result<StateMachineDefinition, String> {
    let source = match version {
        "1.8" | "" => RUBY_1_8,
        // Phase 1 only ships the 1.8 baseline.  Later phases add
        // 1.0 / 1.6 (forward-derived from 1.8) and the 1.9.1 / 2.0 / ... era files.
        other => {
            return Err(format!(
                "ruby lexer: version `{other}` is not yet supported (Phase 1 ships only 1.8)"
            ));
        }
    };
    from_states_toml(source).map_err(|e| format!("ruby lexer: failed to parse TOML — {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ruby_1_8_parses_cleanly() {
        let def = definition_for_version("1.8").expect("definition");
        assert_eq!(def.name, "ruby-1.8-lexer");
        assert_eq!(def.profile.as_deref(), Some("lexer/v1"));
        // Quick sanity: the dispatcher `data` state must exist.
        assert!(def.states.iter().any(|s| s.id == "data"));
        // And so must `done`.
        assert!(def.states.iter().any(|s| s.id == "done"));
    }

    #[test]
    fn ruby_default_version_is_1_8() {
        let def = definition_for_version("").expect("default");
        assert_eq!(def.name, "ruby-1.8-lexer");
    }

    #[test]
    fn unknown_version_errors() {
        let err = definition_for_version("2.0").unwrap_err();
        assert!(err.contains("not yet supported"));
    }
}
