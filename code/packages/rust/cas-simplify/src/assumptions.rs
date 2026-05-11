//! Per-context assumptions for sign-aware simplification.

use std::collections::{HashMap, HashSet};

use symbolic_ir::{IRNode, EQUAL, GREATER, GREATER_EQUAL, LESS, LESS_EQUAL, NOT_EQUAL};

const POSITIVE: &str = "positive";
const NEGATIVE: &str = "negative";
const ZERO: &str = "zero";
const NONZERO: &str = "nonzero";
const NONNEG: &str = "nonneg";
const NONPOS: &str = "nonpos";
const INTEGER: &str = "integer";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AssumptionContext {
    facts: HashMap<String, HashSet<&'static str>>,
}

impl AssumptionContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn assume_relation(&mut self, expr: &IRNode) {
        if let Some((sym_name, head)) = relation_against_zero(expr) {
            match head {
                GREATER => self.add(sym_name, POSITIVE),
                LESS => self.add(sym_name, NEGATIVE),
                GREATER_EQUAL => self.add(sym_name, NONNEG),
                LESS_EQUAL => self.add(sym_name, NONPOS),
                EQUAL => self.add(sym_name, ZERO),
                NOT_EQUAL => self.add(sym_name, NONZERO),
                _ => {}
            }
        }
    }

    pub fn assume_property(&mut self, sym: &IRNode, prop: &IRNode) {
        let Some(symbol_name) = sym_name(sym) else {
            return;
        };
        let Some(prop_name) = sym_name(prop) else {
            return;
        };
        let fact = match prop_name.to_ascii_lowercase().as_str() {
            "positive" | "pos" => POSITIVE,
            "negative" | "neg" => NEGATIVE,
            "zero" => ZERO,
            "nonzero" => NONZERO,
            "nonneg" | "nonnegative" => NONNEG,
            "nonpos" | "nonpositive" => NONPOS,
            "integer" | "integerp" => INTEGER,
            _ => return,
        };
        self.add(symbol_name, fact);
    }

    pub fn forget_relation(&mut self, expr: &IRNode) {
        if let Some((sym_name, head)) = relation_against_zero(expr) {
            match head {
                GREATER => self.remove(sym_name, POSITIVE),
                LESS => self.remove(sym_name, NEGATIVE),
                GREATER_EQUAL => self.remove(sym_name, NONNEG),
                LESS_EQUAL => self.remove(sym_name, NONPOS),
                EQUAL => self.remove(sym_name, ZERO),
                NOT_EQUAL => self.remove(sym_name, NONZERO),
                _ => {}
            }
        }
    }

    pub fn forget_all(&mut self) {
        self.facts.clear();
    }

    pub fn is_positive(&self, sym_name: &str) -> Option<bool> {
        let facts = self.facts_for(sym_name);
        if facts.contains(POSITIVE) {
            Some(true)
        } else if facts.contains(NEGATIVE) || facts.contains(ZERO) {
            Some(false)
        } else {
            None
        }
    }

    pub fn is_negative(&self, sym_name: &str) -> Option<bool> {
        let facts = self.facts_for(sym_name);
        if facts.contains(NEGATIVE) {
            Some(true)
        } else if facts.contains(POSITIVE) || facts.contains(ZERO) || facts.contains(NONNEG) {
            Some(false)
        } else {
            None
        }
    }

    pub fn is_nonneg(&self, sym_name: &str) -> Option<bool> {
        let facts = self.facts_for(sym_name);
        if facts.contains(NONNEG) || facts.contains(POSITIVE) || facts.contains(ZERO) {
            Some(true)
        } else if facts.contains(NEGATIVE) {
            Some(false)
        } else {
            None
        }
    }

    pub fn is_integer(&self, sym_name: &str) -> bool {
        self.facts_for(sym_name).contains(INTEGER)
    }

    pub fn sign_of(&self, sym_name: &str) -> Option<i8> {
        let facts = self.facts_for(sym_name);
        if facts.contains(POSITIVE) {
            Some(1)
        } else if facts.contains(NEGATIVE) {
            Some(-1)
        } else if facts.contains(ZERO) {
            Some(0)
        } else {
            None
        }
    }

    pub fn is_true_relation(&self, expr: &IRNode) -> Option<bool> {
        let (sym_name, head) = relation_against_zero(expr)?;
        let facts = self.facts_for(sym_name);
        match head {
            GREATER => self.is_positive(sym_name),
            LESS => self.is_negative(sym_name),
            GREATER_EQUAL => {
                if facts.contains(POSITIVE) || facts.contains(ZERO) || facts.contains(NONNEG) {
                    Some(true)
                } else if facts.contains(NEGATIVE) {
                    Some(false)
                } else {
                    None
                }
            }
            LESS_EQUAL => {
                if facts.contains(NEGATIVE) || facts.contains(ZERO) || facts.contains(NONPOS) {
                    Some(true)
                } else if facts.contains(POSITIVE) {
                    Some(false)
                } else {
                    None
                }
            }
            EQUAL => {
                if facts.contains(ZERO) {
                    Some(true)
                } else if facts.contains(POSITIVE)
                    || facts.contains(NEGATIVE)
                    || facts.contains(NONZERO)
                {
                    Some(false)
                } else {
                    None
                }
            }
            NOT_EQUAL => {
                if facts.contains(NONZERO) || facts.contains(POSITIVE) || facts.contains(NEGATIVE) {
                    Some(true)
                } else if facts.contains(ZERO) {
                    Some(false)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn has_any_facts(&self, sym_name: &str) -> bool {
        self.facts
            .get(sym_name)
            .is_some_and(|facts| !facts.is_empty())
    }

    fn add(&mut self, sym_name: &str, fact: &'static str) {
        self.facts
            .entry(sym_name.to_string())
            .or_default()
            .insert(fact);
    }

    fn remove(&mut self, sym_name: &str, fact: &'static str) {
        if let Some(facts) = self.facts.get_mut(sym_name) {
            facts.remove(fact);
            if facts.is_empty() {
                self.facts.remove(sym_name);
            }
        }
    }

    fn facts_for(&self, sym_name: &str) -> HashSet<&'static str> {
        self.facts.get(sym_name).cloned().unwrap_or_default()
    }
}

fn relation_against_zero(expr: &IRNode) -> Option<(&str, &str)> {
    let IRNode::Apply(apply) = expr else {
        return None;
    };
    if apply.args.len() != 2 || apply.args[1] != IRNode::Integer(0) {
        return None;
    }
    let sym_name = sym_name(&apply.args[0])?;
    let IRNode::Symbol(head) = &apply.head else {
        return None;
    };
    Some((sym_name, head.as_str()))
}

fn sym_name(node: &IRNode) -> Option<&str> {
    match node {
        IRNode::Symbol(name) => Some(name),
        _ => None,
    }
}
