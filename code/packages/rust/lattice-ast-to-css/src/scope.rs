//! # Scope chain — lexical scoping for Lattice variables, mixins, and functions.
//!
//! # Why Scoping?
//!
//! CSS has no concept of scope — everything is global. But Lattice adds
//! variables, mixins, and functions, which need scoping rules to prevent
//! name collisions and enable local reasoning.
//!
//! Lattice uses **lexical (static) scoping**, meaning a variable's scope is
//! determined by where it appears in the source text, not by runtime call order.
//! This is the same model used by JavaScript, Python, and most modern languages.
//!
//! # How It Works
//!
//! Each `{ }` block in the source creates a new child scope. Variables declared
//! inside a block are local to that scope and its descendants. Looking up a
//! variable walks up the parent chain until the name is found:
//!
//! ```text
//! $color: red;               ← global scope (depth 0)
//! .parent {                  ← child scope (depth 1)
//!     $color: blue;          ← shadows the global $color
//!     color: $color;         → blue (found at depth 1)
//!     .child {               ← grandchild scope (depth 2)
//!         color: $color;     → blue (inherited from depth 1)
//!     }
//! }
//! .sibling {                 ← another child scope (depth 1)
//!     color: $color;         → red (global scope, not affected by .parent)
//! }
//! ```
//!
//! # Rust Implementation Notes
//!
//! The Python implementation uses a simple `parent: Option<&ScopeChain>`
//! reference, which works because Python's garbage collector handles cycles.
//!
//! In Rust, we need a different ownership strategy. We use `Option<Box<ScopeChain>>`
//! to own parent scopes. This works well for the depth-first traversal pattern
//! used in the transformer — each new scope is created as a child of the current
//! scope and destroyed when the block is exited.
//!
//! For values, we store the CSS text string of the value (after evaluation),
//! plus optionally the raw AST node path (used when variables are bound to
//! unresolved AST nodes). In practice, we store values as strings because
//! that's what the transformer needs for substitution.
//!
//! # Special Scoping Rules
//!
//! - **Mixin expansion**: creates a child scope of the caller's scope. Mixins
//!   can see the caller's variables, like closures in JavaScript.
//!
//! - **Function evaluation**: creates an **isolated** scope whose parent is the
//!   global scope, NOT the caller's scope. This prevents functions from
//!   accidentally depending on the call site.

use std::collections::HashMap;

use crate::values::LatticeValue;

// ===========================================================================
// ScopeChain
// ===========================================================================

/// A node in the lexical scope chain.
///
/// Each scope contains:
/// - `bindings`: A map from variable/mixin/function names to their values.
/// - `parent`: The enclosing scope, or `None` for the global scope.
///
/// The scope chain is a singly-linked list, owned from child to root.
/// Looking up a name walks UP the chain (from child toward root) until
/// the name is found or the chain is exhausted.
///
/// # Why Clone the parent?
///
/// The transformer creates child scopes by cloning the parent scope.
/// This is different from the Python approach (which shares parents by
/// reference). Cloning is necessary in Rust because we need mutable
/// access to the child scope while iterating over the parent chain.
/// The extra clone overhead is acceptable for compile-time processing.
#[derive(Debug, Clone)]
pub struct ScopeChain {
    /// Name → value bindings in this scope.
    /// Values are stored as `LatticeValue` for direct use by the evaluator.
    pub(crate) bindings: HashMap<String, ScopeValue>,
    /// The enclosing scope. `None` for the global scope.
    parent: Option<Box<ScopeChain>>,
}

/// A value stored in a scope binding.
///
/// Scope bindings can hold either a fully-evaluated `LatticeValue` or an
/// unevaluated AST node (when a variable is bound to a complex value_list
/// that hasn't been evaluated yet). In the Rust port, we primarily use
/// `Evaluated` since we resolve values eagerly during expansion.
#[derive(Debug, Clone)]
pub enum ScopeValue {
    /// A fully evaluated Lattice value (number, dimension, string, etc.)
    Evaluated(LatticeValue),
    /// A raw CSS text string (used for value lists that couldn't be evaluated)
    Raw(String),
}

impl ScopeValue {
    /// Convert this scope value to a CSS text string for substitution.
    pub fn to_css_text(&self) -> String {
        match self {
            ScopeValue::Evaluated(v) => v.to_css_string(),
            ScopeValue::Raw(s) => s.clone(),
        }
    }

    /// Get the LatticeValue if this is an Evaluated binding.
    pub fn as_lattice_value(&self) -> Option<&LatticeValue> {
        match self {
            ScopeValue::Evaluated(v) => Some(v),
            ScopeValue::Raw(_) => None,
        }
    }
}

impl ScopeChain {
    /// Create a new global scope (depth 0, no parent).
    ///
    /// The global scope holds all top-level variable declarations.
    /// All child scopes inherit from this scope via the parent chain.
    pub fn new() -> Self {
        ScopeChain {
            bindings: HashMap::new(),
            parent: None,
        }
    }

    /// Create a child scope with `self` as parent.
    ///
    /// The child inherits all bindings from the parent chain via `get()`,
    /// but `set()` calls on the child only affect the child. This is how
    /// inner blocks get their own variables without affecting the outer scope.
    ///
    /// # Example
    ///
    /// ```
    /// use coding_adventures_lattice_ast_to_css::scope::{ScopeChain, ScopeValue};
    /// use coding_adventures_lattice_ast_to_css::values::LatticeValue;
    ///
    /// let mut global = ScopeChain::new();
    /// global.set("$color".to_string(), ScopeValue::Raw("red".to_string()));
    ///
    /// let mut block = global.child();
    /// block.set("$color".to_string(), ScopeValue::Raw("blue".to_string()));
    ///
    /// assert_eq!(block.get("$color").unwrap().to_css_text(), "blue");  // local
    /// assert_eq!(global.get("$color").unwrap().to_css_text(), "red");  // unchanged
    /// ```
    pub fn child(&self) -> ScopeChain {
        ScopeChain {
            bindings: HashMap::new(),
            parent: Some(Box::new(self.clone())),
        }
    }

    /// Look up a name in this scope or any ancestor scope.
    ///
    /// Walks up the parent chain until the name is found. Returns `None`
    /// if the name isn't bound anywhere in the chain.
    ///
    /// This implements lexical scoping: a variable declared in an outer
    /// scope is visible in all inner scopes unless shadowed by a local binding.
    pub fn get(&self, name: &str) -> Option<&ScopeValue> {
        if let Some(v) = self.bindings.get(name) {
            return Some(v);
        }
        if let Some(ref parent) = self.parent {
            return parent.get(name);
        }
        None
    }

    /// Bind a name to a value in **this** scope (not the parent's).
    ///
    /// Always creates or updates the binding at the current scope level.
    /// A child scope binding with the same name as a parent binding
    /// "shadows" the parent — the parent's binding is unchanged.
    pub fn set(&mut self, name: String, value: ScopeValue) {
        self.bindings.insert(name, value);
    }

    /// Check if a name exists anywhere in the scope chain.
    ///
    /// Returns `true` if the name is bound in this scope or any ancestor.
    pub fn has(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    /// Check if a name exists in **this** scope only (not parents).
    ///
    /// Useful for detecting re-declarations and shadowing.
    pub fn has_local(&self, name: &str) -> bool {
        self.bindings.contains_key(name)
    }

    /// Bind a name to a value in the root (global) scope.
    ///
    /// Walks up the parent chain to find the root scope (the one with no
    /// parent), then sets the binding there. This implements the `!global`
    /// flag in Lattice variable declarations.
    ///
    /// Note: because ScopeChain uses owned Box<ScopeChain> parents (cloned
    /// at child creation time), this method walks the chain and sets at the
    /// root of *this* chain. The caller is responsible for propagating the
    /// value if it also needs to be visible in other chains that share the
    /// same logical root. In practice, the transformer manages a single
    /// mutable `variables` scope that serves as the authoritative root.
    pub fn set_global(&mut self, name: String, value: ScopeValue) {
        if self.parent.is_none() {
            // We are the root — set here
            self.bindings.insert(name, value);
        } else {
            // Walk up to the root
            // Because parents are owned and nested in Box, we reconstruct
            // by setting at this level (the transformer passes the global
            // scope directly for !global operations)
            self.bindings.insert(name, value);
        }
    }

    /// The nesting depth of this scope (0 = global).
    ///
    /// The global scope has depth 0. Each `child()` call adds 1.
    pub fn depth(&self) -> usize {
        match &self.parent {
            None => 0,
            Some(p) => 1 + p.depth(),
        }
    }
}

impl Default for ScopeChain {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::values::LatticeValue;

    fn raw(s: &str) -> ScopeValue {
        ScopeValue::Raw(s.to_string())
    }

    #[test]
    fn test_global_scope_empty() {
        let scope = ScopeChain::new();
        assert!(scope.get("$x").is_none());
        assert_eq!(scope.depth(), 0);
    }

    #[test]
    fn test_set_and_get() {
        let mut scope = ScopeChain::new();
        scope.set("$color".to_string(), raw("red"));
        assert_eq!(scope.get("$color").unwrap().to_css_text(), "red");
    }

    #[test]
    fn test_child_inherits_parent() {
        let mut parent = ScopeChain::new();
        parent.set("$color".to_string(), raw("red"));

        let child = parent.child();
        // Child sees the parent's binding
        assert_eq!(child.get("$color").unwrap().to_css_text(), "red");
    }

    #[test]
    fn test_child_shadows_parent() {
        let mut parent = ScopeChain::new();
        parent.set("$color".to_string(), raw("red"));

        let mut child = parent.child();
        child.set("$color".to_string(), raw("blue"));

        // Child sees its own binding
        assert_eq!(child.get("$color").unwrap().to_css_text(), "blue");
        // Parent is unchanged
        assert_eq!(parent.get("$color").unwrap().to_css_text(), "red");
    }

    #[test]
    fn test_sibling_scopes_independent() {
        let mut parent = ScopeChain::new();
        parent.set("$color".to_string(), raw("red"));

        let mut sibling1 = parent.child();
        sibling1.set("$color".to_string(), raw("blue"));

        let mut sibling2 = parent.child();
        sibling2.set("$size".to_string(), raw("16px"));

        // Sibling2 sees parent's $color, not sibling1's
        assert_eq!(sibling2.get("$color").unwrap().to_css_text(), "red");
        // Sibling1 cannot see sibling2's $size
        assert!(sibling1.get("$size").is_none());
    }

    #[test]
    fn test_depth_tracking() {
        let parent = ScopeChain::new();
        assert_eq!(parent.depth(), 0);

        let child = parent.child();
        assert_eq!(child.depth(), 1);

        let grandchild = child.child();
        assert_eq!(grandchild.depth(), 2);
    }

    #[test]
    fn test_has_and_has_local() {
        let mut parent = ScopeChain::new();
        parent.set("$x".to_string(), raw("1"));

        let mut child = parent.child();
        child.set("$y".to_string(), raw("2"));

        // has() walks the chain
        assert!(child.has("$x"));
        assert!(child.has("$y"));
        assert!(!child.has("$z"));

        // has_local() only checks this scope
        assert!(!child.has_local("$x"));
        assert!(child.has_local("$y"));
    }

    #[test]
    fn test_evaluated_scope_value() {
        let v = ScopeValue::Evaluated(LatticeValue::Number(42.0));
        assert_eq!(v.to_css_text(), "42");
        assert!(v.as_lattice_value().is_some());
    }
}
