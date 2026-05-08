//! The atomic Capability declaration.

use crate::category::{is_valid_combination, Action, Category};
use crate::errors::InvalidCombination;

/// A single OS-level permission declaration.
///
/// Capabilities are immutable. Construct via [`Capability::new`], which
/// validates the (category, action) pair against the cage taxonomy
/// and rejects empty targets.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Capability {
    pub category: Category,
    pub action: Action,
    pub target: String,
    pub justification: String,
}

impl Capability {
    /// Construct a capability after validating the (category, action)
    /// pair and the target string.
    ///
    /// Returns [`InvalidCombination::UnsupportedPair`] if the pair is
    /// not in the cage taxonomy, or [`InvalidCombination::EmptyTarget`]
    /// if the target is empty.
    pub fn new(
        category: Category,
        action: Action,
        target: impl Into<String>,
        justification: impl Into<String>,
    ) -> Result<Self, InvalidCombination> {
        if !is_valid_combination(category, action) {
            return Err(InvalidCombination::UnsupportedPair { category, action });
        }
        let target = target.into();
        if target.is_empty() {
            return Err(InvalidCombination::EmptyTarget);
        }
        Ok(Self {
            category,
            action,
            target,
            justification: justification.into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_capability_round_trips_fields() {
        let cap = Capability::new(
            Category::Fs,
            Action::Read,
            "./grammars/json.tokens",
            "load lexer DFA",
        )
        .unwrap();
        assert_eq!(cap.category, Category::Fs);
        assert_eq!(cap.action, Action::Read);
        assert_eq!(cap.target, "./grammars/json.tokens");
        assert_eq!(cap.justification, "load lexer DFA");
    }

    #[test]
    fn invalid_pair_rejected() {
        let err = Capability::new(Category::Fs, Action::Connect, "x", "y").unwrap_err();
        assert_eq!(
            err,
            InvalidCombination::UnsupportedPair {
                category: Category::Fs,
                action: Action::Connect,
            }
        );
    }

    #[test]
    fn empty_target_rejected() {
        let err = Capability::new(Category::Fs, Action::Read, "", "y").unwrap_err();
        assert_eq!(err, InvalidCombination::EmptyTarget);
    }

    #[test]
    fn capabilities_compare_by_value() {
        let a = Capability::new(Category::Fs, Action::Read, "x", "j").unwrap();
        let b = Capability::new(Category::Fs, Action::Read, "x", "j").unwrap();
        let c = Capability::new(Category::Fs, Action::Read, "y", "j").unwrap();
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
