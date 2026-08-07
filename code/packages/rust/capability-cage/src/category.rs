//! Capability categories and actions.
//!
//! The taxonomy is fixed by `capability-cage-rust.md`: 8 categories and
//! 14 actions. Not every (category, action) pair is meaningful; the
//! valid pairings are encoded in [`Capability::new`] (see `capability.rs`).

use std::fmt;
use std::str::FromStr;

/// One of the eight category types in the cage taxonomy.
///
/// Adding a category requires a spec amendment and corresponding
/// updates to the JSON schema and every backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    Fs,
    Net,
    Proc,
    Env,
    Ffi,
    Time,
    Stdin,
    Stdout,
}

impl Category {
    /// Lower-case wire form. Stable across versions.
    pub fn as_str(self) -> &'static str {
        match self {
            Category::Fs => "fs",
            Category::Net => "net",
            Category::Proc => "proc",
            Category::Env => "env",
            Category::Ffi => "ffi",
            Category::Time => "time",
            Category::Stdin => "stdin",
            Category::Stdout => "stdout",
        }
    }
}

impl fmt::Display for Category {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Category {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "fs" => Ok(Category::Fs),
            "net" => Ok(Category::Net),
            "proc" => Ok(Category::Proc),
            "env" => Ok(Category::Env),
            "ffi" => Ok(Category::Ffi),
            "time" => Ok(Category::Time),
            "stdin" => Ok(Category::Stdin),
            "stdout" => Ok(Category::Stdout),
            _ => Err(()),
        }
    }
}

/// One of the fourteen action types in the cage taxonomy.
///
/// Adding an action requires a spec amendment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Read,
    Write,
    Create,
    Delete,
    List,
    Connect,
    Listen,
    Dns,
    Exec,
    Fork,
    Signal,
    Call,
    Load,
    Sleep,
}

impl Action {
    pub fn as_str(self) -> &'static str {
        match self {
            Action::Read => "read",
            Action::Write => "write",
            Action::Create => "create",
            Action::Delete => "delete",
            Action::List => "list",
            Action::Connect => "connect",
            Action::Listen => "listen",
            Action::Dns => "dns",
            Action::Exec => "exec",
            Action::Fork => "fork",
            Action::Signal => "signal",
            Action::Call => "call",
            Action::Load => "load",
            Action::Sleep => "sleep",
        }
    }
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Action {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "read" => Ok(Action::Read),
            "write" => Ok(Action::Write),
            "create" => Ok(Action::Create),
            "delete" => Ok(Action::Delete),
            "list" => Ok(Action::List),
            "connect" => Ok(Action::Connect),
            "listen" => Ok(Action::Listen),
            "dns" => Ok(Action::Dns),
            "exec" => Ok(Action::Exec),
            "fork" => Ok(Action::Fork),
            "signal" => Ok(Action::Signal),
            "call" => Ok(Action::Call),
            "load" => Ok(Action::Load),
            "sleep" => Ok(Action::Sleep),
            _ => Err(()),
        }
    }
}

/// Returns true if (category, action) is a meaningful pair per the
/// cage taxonomy.
///
/// The valid pairings (from the spec):
/// ```text
/// fs       read, write, create, delete, list
/// net      connect, listen, dns
/// proc     exec, fork, signal
/// env      read, write
/// ffi      call, load
/// time     read, sleep
/// stdin    read
/// stdout   write
/// ```
pub fn is_valid_combination(category: Category, action: Action) -> bool {
    use Action::*;
    use Category::*;
    matches!(
        (category, action),
        (Fs, Read)
            | (Fs, Write)
            | (Fs, Create)
            | (Fs, Delete)
            | (Fs, List)
            | (Net, Connect)
            | (Net, Listen)
            | (Net, Dns)
            | (Proc, Exec)
            | (Proc, Fork)
            | (Proc, Signal)
            | (Env, Read)
            | (Env, Write)
            | (Ffi, Call)
            | (Ffi, Load)
            | (Time, Read)
            | (Time, Sleep)
            | (Stdin, Read)
            | (Stdout, Write)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_json_value::{parse, JsonNumber, JsonValue};
    use std::collections::HashSet;

    #[test]
    fn category_round_trips() {
        for s in ["fs", "net", "proc", "env", "ffi", "time", "stdin", "stdout"] {
            let cat: Category = s.parse().unwrap();
            assert_eq!(cat.as_str(), s);
            assert_eq!(format!("{cat}"), s);
        }
    }

    #[test]
    fn action_round_trips() {
        for s in [
            "read", "write", "create", "delete", "list", "connect", "listen", "dns", "exec",
            "fork", "signal", "call", "load", "sleep",
        ] {
            let act: Action = s.parse().unwrap();
            assert_eq!(act.as_str(), s);
        }
    }

    #[test]
    fn unknown_category_rejected() {
        assert!("network".parse::<Category>().is_err());
        assert!("".parse::<Category>().is_err());
    }

    #[test]
    fn unknown_action_rejected() {
        assert!("destroy".parse::<Action>().is_err());
        assert!("".parse::<Action>().is_err());
    }

    #[test]
    fn shared_fixture_exhaustively_matches_pair_table() {
        let fixture = parse(include_str!(
            "../../../../specs/fixtures/capability-security-v1/taxonomy.json"
        ))
        .expect("shared taxonomy fixture must parse");
        let JsonValue::Object(root) = fixture else {
            panic!("taxonomy fixture must be an object");
        };
        let lookup = |key: &str| {
            root.iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value)
                .unwrap_or_else(|| panic!("missing fixture field {key}"))
        };
        let JsonValue::Object(categories) = lookup("categories") else {
            panic!("categories must be an object");
        };
        let JsonValue::Array(all_actions) = lookup("all_actions") else {
            panic!("all_actions must be an array");
        };
        let actions: Vec<&str> = all_actions
            .iter()
            .map(|value| match value {
                JsonValue::String(action) => action.as_str(),
                _ => panic!("all_actions entries must be strings"),
            })
            .collect();

        let mut valid_count = 0;
        let mut invalid_count = 0;
        for (category_name, allowed_value) in categories {
            let category: Category = category_name.parse().expect("known fixture category");
            let JsonValue::Array(allowed_values) = allowed_value else {
                panic!("category actions must be arrays");
            };
            let allowed: HashSet<&str> = allowed_values
                .iter()
                .map(|value| match value {
                    JsonValue::String(action) => action.as_str(),
                    _ => panic!("category action entries must be strings"),
                })
                .collect();
            for action_name in &actions {
                let action: Action = action_name.parse().expect("known fixture action");
                let want = allowed.contains(action_name);
                assert_eq!(
                    is_valid_combination(category, action),
                    want,
                    "pair mismatch for {category}:{action}"
                );
                if want {
                    valid_count += 1;
                } else {
                    invalid_count += 1;
                }
            }
        }

        let expected_count = |key: &str| match lookup(key) {
            JsonValue::Number(JsonNumber::Integer(value)) => *value as usize,
            _ => panic!("{key} must be an integer"),
        };
        assert_eq!(valid_count, expected_count("expected_valid_pair_count"));
        assert_eq!(
            invalid_count,
            expected_count("expected_invalid_cross_pair_count")
        );
    }
}
