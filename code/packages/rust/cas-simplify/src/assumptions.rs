//! Per-context assumptions for sign-aware simplification.

use std::collections::{HashMap, HashSet};

use symbolic_ir::{IRNode, EQUAL, GREATER, GREATER_EQUAL, LESS, LESS_EQUAL, NOT_EQUAL};

// Short operator strings used to canonicalise compound relations.
// Mirrors the Python ``_RELATION_HEAD_TO_OP`` map and gives us a
// stable, hashable middle component for the ``(lhs, op, rhs)`` triple
// stored in ``general_relations``.  ``&'static str`` keeps the triple
// type cheap to clone.
const OP_GT: &str = ">";
const OP_LT: &str = "<";
const OP_GE: &str = ">=";
const OP_LE: &str = "<=";
const OP_EQ: &str = "=";
const OP_NE: &str = "!=";

/// Map a relational head name to the canonical operator string, or
/// `None` when the head isn't one of the six relational operators.
fn head_to_op(head: &str) -> Option<&'static str> {
    match head {
        GREATER => Some(OP_GT),
        LESS => Some(OP_LT),
        GREATER_EQUAL => Some(OP_GE),
        LESS_EQUAL => Some(OP_LE),
        EQUAL => Some(OP_EQ),
        NOT_EQUAL => Some(OP_NE),
        _ => None,
    }
}

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
    /// Track G2 — compound relations as canonicalised `(lhs, op, rhs)`
    /// triples.  `op` is one of the six static strings declared above.
    /// `IRNode` implements `Hash + Eq` so the set deduplicates
    /// structurally.  See [`Self::assume_relation`] for the canonical
    /// form rules — `<` is always rewritten to `>`, `<=` to `>=`,
    /// `=`/`!=` ordered by display key.
    general_relations: HashSet<(IRNode, &'static str, IRNode)>,
}

impl AssumptionContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn assume_relation(&mut self, expr: &IRNode) {
        let Some((lhs, op, rhs)) = parse_relation(expr) else {
            return;
        };
        // Plain-symbol-vs-zero path: fold into the per-symbol fact
        // table (Phase 21 behaviour).
        if let (Some(name), true) = (sym_name(lhs), rhs == &IRNode::Integer(0)) {
            match op {
                OP_GT => self.add(name, POSITIVE),
                OP_LT => self.add(name, NEGATIVE),
                OP_GE => self.add(name, NONNEG),
                OP_LE => self.add(name, NONPOS),
                OP_EQ => self.add(name, ZERO),
                OP_NE => self.add(name, NONZERO),
                _ => {}
            }
            return;
        }
        // Track G2 — compound-relation path.  Canonicalise so commuted
        // and dual surface forms collapse to the same triple.
        let triple = canon_relation(lhs.clone(), op, rhs.clone());
        self.general_relations.insert(triple);
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
        let Some((lhs, op, rhs)) = parse_relation(expr) else {
            return;
        };
        if let (Some(name), true) = (sym_name(lhs), rhs == &IRNode::Integer(0)) {
            match op {
                OP_GT => self.remove(name, POSITIVE),
                OP_LT => self.remove(name, NEGATIVE),
                OP_GE => self.remove(name, NONNEG),
                OP_LE => self.remove(name, NONPOS),
                OP_EQ => self.remove(name, ZERO),
                OP_NE => self.remove(name, NONZERO),
                _ => {}
            }
            return;
        }
        let triple = canon_relation(lhs.clone(), op, rhs.clone());
        self.general_relations.remove(&triple);
    }

    pub fn forget_all(&mut self) {
        self.facts.clear();
        self.general_relations.clear();
    }

    pub fn is_positive(&self, sym_name: &str) -> Option<bool> {
        let facts = self.fact_set(sym_name);
        if facts.contains(POSITIVE) {
            Some(true)
        } else if facts.contains(NEGATIVE) || facts.contains(ZERO) {
            Some(false)
        } else {
            None
        }
    }

    pub fn is_negative(&self, sym_name: &str) -> Option<bool> {
        let facts = self.fact_set(sym_name);
        if facts.contains(NEGATIVE) {
            Some(true)
        } else if facts.contains(POSITIVE) || facts.contains(ZERO) || facts.contains(NONNEG) {
            Some(false)
        } else {
            None
        }
    }

    pub fn is_nonneg(&self, sym_name: &str) -> Option<bool> {
        let facts = self.fact_set(sym_name);
        if facts.contains(NONNEG) || facts.contains(POSITIVE) || facts.contains(ZERO) {
            Some(true)
        } else if facts.contains(NEGATIVE) {
            Some(false)
        } else {
            None
        }
    }

    pub fn is_integer(&self, sym_name: &str) -> bool {
        self.fact_set(sym_name).contains(INTEGER)
    }

    pub fn sign_of(&self, sym_name: &str) -> Option<i8> {
        let facts = self.fact_set(sym_name);
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
        let (lhs, op, rhs) = parse_relation(expr)?;

        // Path 1 — plain-symbol-vs-zero (Phase 21 behaviour).
        if let (Some(name), true) = (sym_name(lhs), rhs == &IRNode::Integer(0)) {
            if let Some(answer) = self.is_true_plain(name, op) {
                return Some(answer);
            }
            // Fall through to the compound path so an explicit
            // verbatim assertion with a symbol-vs-zero shape can still
            // match (mirrors the Python helper).
        }

        // Path 2 — Track G2 compound-relation lookup.  Canonicalise the
        // query and probe the structural set.
        let triple = canon_relation(lhs.clone(), op, rhs.clone());
        if self.general_relations.contains(&triple) {
            Some(true)
        } else {
            None
        }
    }

    /// Plain-symbol-vs-zero branch of [`Self::is_true_relation`].
    /// Returns `None` when nothing in the fact table speaks to the
    /// query, letting the caller fall through to the compound-relation
    /// lookup.
    fn is_true_plain(&self, sym_name: &str, op: &str) -> Option<bool> {
        let facts = self.fact_set(sym_name);
        match op {
            OP_GT => self.is_positive(sym_name),
            OP_LT => self.is_negative(sym_name),
            OP_GE => {
                if facts.contains(POSITIVE) || facts.contains(ZERO) || facts.contains(NONNEG) {
                    Some(true)
                } else if facts.contains(NEGATIVE) {
                    Some(false)
                } else {
                    None
                }
            }
            OP_LE => {
                if facts.contains(NEGATIVE) || facts.contains(ZERO) || facts.contains(NONPOS) {
                    Some(true)
                } else if facts.contains(POSITIVE) {
                    Some(false)
                } else {
                    None
                }
            }
            OP_EQ => {
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
            OP_NE => {
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

    pub fn facts_for(&self, sym_name: &str) -> Vec<&'static str> {
        let mut facts: Vec<_> = self
            .facts
            .get(sym_name)
            .map(|facts| facts.iter().copied().collect())
            .unwrap_or_default();
        facts.sort_unstable();
        facts
    }

    pub fn symbols_with_facts(&self) -> Vec<String> {
        let mut names: Vec<_> = self
            .facts
            .iter()
            .filter(|(_, facts)| !facts.is_empty())
            .map(|(name, _)| name.clone())
            .collect();
        names.sort();
        names
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

    fn fact_set(&self, sym_name: &str) -> HashSet<&'static str> {
        self.facts.get(sym_name).cloned().unwrap_or_default()
    }
}

/// Parse a relational `Apply(head, [lhs, rhs])` and return
/// `(lhs, op, rhs)` with `op` mapped to the canonical operator string.
/// Returns `None` for non-relational nodes (head not in the six
/// recognised relational operators, wrong arity, etc.).
fn parse_relation(expr: &IRNode) -> Option<(&IRNode, &'static str, &IRNode)> {
    let IRNode::Apply(apply) = expr else {
        return None;
    };
    if apply.args.len() != 2 {
        return None;
    }
    let IRNode::Symbol(head) = &apply.head else {
        return None;
    };
    let op = head_to_op(head.as_str())?;
    Some((&apply.args[0], op, &apply.args[1]))
}

/// Canonicalise a `(lhs, op, rhs)` relation triple.  Rules mirror the
/// Python `_canon_relation`:
///
/// - `a < b` is rewritten to `(b, ">", a)` — every strict inequality
///   becomes `>`.
/// - `a <= b` becomes `(b, ">=", a)`.
/// - `a = b` / `a != b` are commutative — ordered by display-string key
///   so duplicates from either operand order collapse.
/// - `a > b` and `a >= b` pass through verbatim.
fn canon_relation(lhs: IRNode, op: &'static str, rhs: IRNode) -> (IRNode, &'static str, IRNode) {
    match op {
        OP_LT => (rhs, OP_GT, lhs),
        OP_LE => (rhs, OP_GE, lhs),
        OP_EQ | OP_NE => {
            if node_key(&lhs) <= node_key(&rhs) {
                (lhs, op, rhs)
            } else {
                (rhs, op, lhs)
            }
        }
        _ => (lhs, op, rhs),
    }
}

/// Deterministic ordering key for the commutativity tiebreak in
/// [`canon_relation`].  Uses `Display` since every IR node has a
/// structural `Display` impl.
fn node_key(node: &IRNode) -> String {
    format!("{node}")
}

fn sym_name(node: &IRNode) -> Option<&str> {
    match node {
        IRNode::Symbol(name) => Some(name),
        _ => None,
    }
}
