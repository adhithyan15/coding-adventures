//! MACSYMA-style `matchdeclare` state and pattern compilation.
//!
//! A [`MatchDeclareContext`] records which bare symbols should be treated as
//! pattern variables.  [`MatchDeclareContext::compile_pattern`] then walks an
//! arbitrary `IRNode` tree and replaces declared symbols with
//! `Pattern(name, Blank(...))` nodes understood by the matcher.

use std::collections::HashMap;

use symbolic_ir::{IRApply, IRNode};

use crate::nodes::{blank, blank_typed, named};

/// Per-VM store of `matchdeclare` declarations.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MatchDeclareContext {
    declarations: HashMap<String, String>,
}

impl MatchDeclareContext {
    /// Create an empty declaration context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record `sym_name` as a pattern variable constrained by `pred_tag`.
    ///
    /// Predicate tags are stored lowercased.  Unknown tags are preserved for
    /// query purposes and compile to an unconstrained `Blank()` fallback.
    pub fn declare(&mut self, sym_name: impl Into<String>, pred_tag: impl AsRef<str>) {
        self.declarations
            .insert(sym_name.into(), pred_tag.as_ref().to_ascii_lowercase());
    }

    /// Remove one declaration, if present.
    pub fn forget(&mut self, sym_name: &str) {
        self.declarations.remove(sym_name);
    }

    /// Remove all declarations.
    pub fn forget_all(&mut self) {
        self.declarations.clear();
    }

    /// True when `sym_name` is declared as a pattern variable.
    pub fn is_declared(&self, sym_name: &str) -> bool {
        self.declarations.contains_key(sym_name)
    }

    /// Return the normalized predicate tag for `sym_name`, if declared.
    pub fn get_predicate(&self, sym_name: &str) -> Option<&str> {
        self.declarations.get(sym_name).map(String::as_str)
    }

    /// Compile a raw pattern tree into matcher-ready pattern nodes.
    ///
    /// Every declared `Symbol(name)` becomes `Pattern(name, Blank(...))`.
    /// Compound apply heads and arguments are both walked recursively.
    pub fn compile_pattern(&self, pattern: &IRNode) -> IRNode {
        self.walk(pattern)
    }

    fn walk(&self, node: &IRNode) -> IRNode {
        match node {
            IRNode::Symbol(name) => {
                if let Some(pred_tag) = self.declarations.get(name) {
                    named(name, blank_for_predicate(pred_tag))
                } else {
                    node.clone()
                }
            }
            IRNode::Apply(apply) => IRNode::Apply(Box::new(IRApply {
                head: self.walk(&apply.head),
                args: apply.args.iter().map(|arg| self.walk(arg)).collect(),
            })),
            _ => node.clone(),
        }
    }
}

fn blank_for_predicate(pred_tag: &str) -> IRNode {
    match predicate_blank_head(pred_tag) {
        Some(head) => blank_typed(head),
        None => blank(),
    }
}

fn predicate_blank_head(pred_tag: &str) -> Option<&'static str> {
    match pred_tag {
        "integerp" => Some("Integer"),
        "symbolp" => Some("Symbol"),
        "floatp" => Some("Float"),
        "rationalp" => Some("Rational"),
        "listp" => Some("List"),
        "stringp" => Some("String"),
        "true" | "all" | "any" | "numberp" => None,
        _ => None,
    }
}
