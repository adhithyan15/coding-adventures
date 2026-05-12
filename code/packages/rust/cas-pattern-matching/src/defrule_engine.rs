//! Named-rule store for `defrule` / `apply1` / `apply2` style flows.
//!
//! The store intentionally does not compile or validate rules.  Callers compile
//! rule left/right sides with [`crate::MatchDeclareContext`] and then store the
//! resulting `Rule(lhs, rhs)` IR node here.

use std::collections::HashMap;

use symbolic_ir::IRNode;

/// Mutable map from rule name to compiled `Rule` IR node.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuleStore {
    rules: HashMap<String, IRNode>,
}

impl RuleStore {
    /// Create an empty rule store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Install or replace a named rule.
    pub fn store(&mut self, name: impl Into<String>, rule: IRNode) {
        self.rules.insert(name.into(), rule);
    }

    /// Return the rule for `name`, if present.
    pub fn get(&self, name: &str) -> Option<&IRNode> {
        self.rules.get(name)
    }

    /// Remove a named rule, if present.
    pub fn remove(&mut self, name: &str) {
        self.rules.remove(name);
    }

    /// Remove all rules.
    pub fn clear(&mut self) {
        self.rules.clear();
    }

    /// Return all rule names in sorted order.
    pub fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.rules.keys().cloned().collect();
        names.sort();
        names
    }

    /// Number of stored rules.
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// True when no rules are stored.
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// True when `name` is present.
    pub fn contains(&self, name: &str) -> bool {
        self.rules.contains_key(name)
    }
}
