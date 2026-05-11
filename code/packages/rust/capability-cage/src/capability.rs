//! The atomic Capability declaration.

use crate::category::{is_valid_combination, Action, Category};
use crate::errors::InvalidCombination;
use read_write_separation::{Capability as ReadWriteCapability, CapabilityFlavor, CapabilityTrust};

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
    pub flavor: Option<CapabilityFlavor>,
    pub trust: Option<CapabilityTrust>,
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
            flavor: None,
            trust: None,
        })
    }

    /// Annotate the capability for read/write separation checks.
    pub fn with_flavor(mut self, flavor: CapabilityFlavor) -> Self {
        self.flavor = Some(flavor);
        self
    }

    /// Annotate the capability trust boundary for read/write separation checks.
    pub fn with_trust(mut self, trust: CapabilityTrust) -> Self {
        self.trust = Some(trust);
        self
    }

    pub(crate) fn to_read_write_capability(&self) -> ReadWriteCapability {
        let mut capability = ReadWriteCapability::new(
            self.category.as_str(),
            self.action.as_str(),
            self.target.clone(),
        )
        .with_justification(self.justification.clone());

        if let Some(flavor) = self.flavor {
            capability = capability.with_flavor(flavor);
        }
        if let Some(trust) = self.trust {
            capability = capability.with_trust(trust);
        }

        capability
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
        assert_eq!(cap.flavor, None);
        assert_eq!(cap.trust, None);
    }

    #[test]
    fn read_write_annotations_are_preserved() {
        let cap = Capability::new(
            Category::Net,
            Action::Connect,
            "imap.example.test:993",
            "mail",
        )
        .unwrap()
        .with_flavor(CapabilityFlavor::Ingestion)
        .with_trust(CapabilityTrust::Untrusted);
        let rws = cap.to_read_write_capability();

        assert_eq!(cap.flavor, Some(CapabilityFlavor::Ingestion));
        assert_eq!(cap.trust, Some(CapabilityTrust::Untrusted));
        assert_eq!(rws.flavor, Some(CapabilityFlavor::Ingestion));
        assert_eq!(rws.trust, Some(CapabilityTrust::Untrusted));
        assert_eq!(rws.justification.as_deref(), Some("mail"));
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
