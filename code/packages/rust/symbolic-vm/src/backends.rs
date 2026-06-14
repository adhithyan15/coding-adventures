//! The two reference backends: [`StrictBackend`] and [`SymbolicBackend`].
//!
//! ```text
//! ┌─────────────────────┬─────────────────────────────────────────────┐
//! │                     │ StrictBackend      │ SymbolicBackend         │
//! ├─────────────────────┼────────────────────┼─────────────────────────┤
//! │ Unbound symbol      │ panic!             │ returns symbol as-is    │
//! │ Unknown head        │ panic!             │ returns expr as-is      │
//! │ Arith on symbols    │ panic!             │ folds identities, else  │
//! │                     │                    │ leaves the expression   │
//! └─────────────────────┴────────────────────┴─────────────────────────┘
//! ```
//!
//! Both backends share a common environment + held-head set via
//! [`crate::backend::BaseBackend`].  Subclassing is done here by embedding
//! that shared state and delegating `lookup`/`bind`/`hold_heads` to it.

use std::collections::{HashMap, HashSet};

use symbolic_ir::IRNode;

use crate::backend::{Backend, BaseBackend, Handler};
use crate::handlers::build_handler_table;

// ---------------------------------------------------------------------------
// StrictBackend
// ---------------------------------------------------------------------------

/// Python-style numeric evaluator.
///
/// Every name must be bound; every head must have a handler; every
/// arithmetic operation must be fully numeric.  Panics on unknowns.
///
/// Useful for "calculator mode" — load a program with only numeric inputs
/// and get numeric answers out.
pub struct StrictBackend {
    base: BaseBackend,
    handlers: HashMap<String, Handler>,
}

impl StrictBackend {
    /// Create a new `StrictBackend` with the full numeric handler table.
    pub fn new() -> Self {
        Self {
            base: BaseBackend::new(),
            handlers: build_handler_table(false),
        }
    }
}

impl Default for StrictBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for StrictBackend {
    fn lookup(&self, name: &str) -> Option<IRNode> {
        self.base.env.get(name).cloned()
    }

    fn bind(&mut self, name: &str, value: IRNode) {
        self.base.env.insert(name.to_string(), value);
    }

    fn on_unresolved(&self, name: &str) -> IRNode {
        panic!("undefined symbol: {name:?}");
    }

    fn on_unknown_head(&self, expr: symbolic_ir::IRApply) -> IRNode {
        let head_name = if let IRNode::Symbol(s) = &expr.head {
            s.as_str().to_string()
        } else {
            "?".to_string()
        };
        panic!("no handler for head: {head_name:?}");
    }

    fn handler_for(&self, name: &str) -> Option<&Handler> {
        self.handlers.get(name)
    }

    fn hold_heads(&self) -> &HashSet<String> {
        &self.base.held
    }
}

// ---------------------------------------------------------------------------
// SymbolicBackend
// ---------------------------------------------------------------------------

/// Mathematica-style evaluator.
///
/// Unbound names stay as free symbols; algebraic identities collapse trivial
/// cases; unknown functions pass through untouched.  The result is a tiny
/// computer algebra system: `Add(x, 0)` → `x`, `Pow(x, 0)` → `1`, and
/// `Cos(0)` → `1`, but `Add(x, x)` stays as-is (no polynomial
/// normalisation yet).
pub struct SymbolicBackend {
    base: BaseBackend,
    handlers: HashMap<String, Handler>,
}

impl SymbolicBackend {
    /// Create a new `SymbolicBackend` with the full symbolic handler table.
    pub fn new() -> Self {
        Self {
            base: BaseBackend::new(),
            handlers: build_handler_table(true),
        }
    }

    /// Pre-bind a name to a value in the environment.
    ///
    /// Used by language-specific backends to install constants like `%pi`.
    pub fn pre_bind(&mut self, name: &str, value: IRNode) {
        self.base.env.insert(name.to_string(), value);
    }
}

impl Default for SymbolicBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for SymbolicBackend {
    fn lookup(&self, name: &str) -> Option<IRNode> {
        self.base.env.get(name).cloned()
    }

    fn bind(&mut self, name: &str, value: IRNode) {
        self.base.env.insert(name.to_string(), value);
    }

    fn on_unresolved(&self, name: &str) -> IRNode {
        // Free variables remain as symbols — the Mathematica convention.
        IRNode::Symbol(name.to_string())
    }

    fn on_unknown_head(&self, expr: symbolic_ir::IRApply) -> IRNode {
        // Unevaluated expressions pass through unchanged.
        IRNode::Apply(Box::new(expr))
    }

    fn handler_for(&self, name: &str) -> Option<&Handler> {
        self.handlers.get(name)
    }

    fn hold_heads(&self) -> &HashSet<String> {
        &self.base.held
    }
}
