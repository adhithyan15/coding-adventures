//! The [`Backend`] trait — the policy seam for the VM.
//!
//! Everything that makes one CAS dialect different from another is encoded
//! here:
//!
//! | Question | Method |
//! |----------|--------|
//! | How do I look up a name? | [`Backend::lookup`] / [`Backend::bind`] |
//! | What if a name is unbound? | [`Backend::on_unresolved`] |
//! | What if no handler exists? | [`Backend::on_unknown_head`] |
//! | Are there cheap rewrite rules? | [`Backend::rules`] |
//! | How do I evaluate a given head? | [`Backend::handler_for`] |
//! | Which heads must NOT have their args pre-evaluated? | [`Backend::hold_heads`] |
//!
//! ## Handler type
//!
//! A [`Handler`] is an `Arc<dyn Fn(&mut VM, IRApply) -> IRNode>`.  Using
//! `Arc` rather than `Box` lets multiple backends share sub-handlers and,
//! more importantly, allows the VM to **clone** the handler out of the
//! backend's map before calling it, which resolves the borrow-checker
//! conflict between "holding a reference into `self.backend`" and
//! "passing `&mut self` into the handler call".
//!
//! ## Rewrite rules
//!
//! Rules are `(predicate, transform)` pairs.  The VM tests each predicate
//! (in order) on the already-arg-evaluated `IRApply`; the first matching
//! rule's transform is applied and its result is re-evaluated.  Rules run
//! before head handlers and are intended for cheap syntactic rewrites.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use symbolic_ir::{IRApply, IRNode};

use crate::vm::VM;

/// A head handler: evaluates `IRApply(head, args)` and returns the result.
///
/// The VM has already evaluated `args` (unless the head is held) before
/// dispatching here.  The handler may call `vm.eval(...)` to evaluate any
/// sub-expressions it constructs.
///
/// `Arc` makes the type cheaply cloneable so the VM can copy the handler
/// out of the backend's table before invoking it (avoiding a double-borrow
/// of `self`).
pub type Handler = Arc<dyn Fn(&mut VM, IRApply) -> IRNode + Send + Sync>;

/// A predicate over an `IRApply`.  Returns `true` if the rule should fire.
pub type RulePredicate = Arc<dyn Fn(&IRApply) -> bool + Send + Sync>;

/// A transform applied to an `IRApply` whose predicate fired.
/// Produces the rewritten node (before re-evaluation by the VM).
pub type RuleTransform = Arc<dyn Fn(IRApply) -> IRNode + Send + Sync>;

/// A rewrite rule: `(predicate, transform)`.
pub type Rule = (RulePredicate, RuleTransform);

/// Policy object that the [`VM`] consults for every evaluation decision.
///
/// Implement this trait to create a new CAS dialect.  The two reference
/// implementations in this crate are [`crate::StrictBackend`] and
/// [`crate::SymbolicBackend`].
pub trait Backend: Send {
    // ------------------------------------------------------------------
    // Name binding
    // ------------------------------------------------------------------

    /// Return the current binding for `name`, or `None` if unbound.
    fn lookup(&self, name: &str) -> Option<IRNode>;

    /// Install or update a binding for `name`.
    fn bind(&mut self, name: &str, value: IRNode);

    // ------------------------------------------------------------------
    // Evaluation policy
    // ------------------------------------------------------------------

    /// What to return when `name` has no binding.
    ///
    /// - **Strict** backends `panic!` with a `NameError` message.
    /// - **Symbolic** backends return `IRNode::Symbol(name.to_string())`
    ///   so unbound names act as free variables.
    fn on_unresolved(&self, name: &str) -> IRNode;

    /// What to return when no handler exists for `expr`'s head.
    ///
    /// The default implementation returns `IRNode::Apply(Box::new(expr))`,
    /// leaving the expression unevaluated.  Strict backends override this
    /// to panic.
    fn on_unknown_head(&self, expr: IRApply) -> IRNode {
        IRNode::Apply(Box::new(expr))
    }

    /// Rewrite rules to try before dispatching to a head handler.
    ///
    /// Returns a slice of `(predicate, transform)` pairs.  The VM tests
    /// them in order on the already-arg-evaluated `IRApply`; the first
    /// matching rule's transform is applied and its result re-evaluated.
    fn rules(&self) -> &[Rule] {
        &[]
    }

    /// Return the handler registered for `head_name`, if any.
    ///
    /// The VM clones the `Arc<Handler>` before calling it, which lets it
    /// release its borrow of `self` before passing `&mut self` to the
    /// handler.
    fn handler_for(&self, head_name: &str) -> Option<&Handler>;

    /// Head names whose arguments the VM must **not** evaluate before
    /// dispatching.
    ///
    /// The canonical held heads are `"Assign"`, `"Define"`, and `"If"`:
    /// - `Assign(name, rhs)` — the lhs is a name, not an expression.
    /// - `Define(name, params, body)` — the body is stored unevaluated.
    /// - `If(cond, then, else)` — only the chosen branch is evaluated.
    fn hold_heads(&self) -> &HashSet<String>;
}

// ---------------------------------------------------------------------------
// Convenience: build a handler from a plain function pointer
// ---------------------------------------------------------------------------

/// Wrap a `fn(&mut VM, IRApply) -> IRNode` in an `Arc` so it can be stored
/// as a [`Handler`].
///
/// ```rust
/// use symbolic_vm::backend::{handler_fn, Handler};
/// use symbolic_vm::VM;
/// use symbolic_ir::{IRApply, IRNode};
///
/// fn my_handler(_vm: &mut VM, expr: IRApply) -> IRNode {
///     IRNode::Apply(Box::new(expr))
/// }
///
/// let h: Handler = handler_fn(my_handler);
/// ```
pub fn handler_fn(f: fn(&mut VM, IRApply) -> IRNode) -> Handler {
    Arc::new(f)
}

// ---------------------------------------------------------------------------
// BaseBackend — shared environment + hold-heads for StrictBackend /
// SymbolicBackend
// ---------------------------------------------------------------------------

/// Shared state for the two reference backends.
///
/// Holds the name-binding environment and the set of held heads.  The two
/// reference backends embed a `BaseBackend` and delegate `lookup`/`bind`/
/// `hold_heads` to it.
pub struct BaseBackend {
    pub env: HashMap<String, IRNode>,
    pub held: HashSet<String>,
}

impl BaseBackend {
    /// Create a `BaseBackend` with the standard held heads (`Assign`,
    /// `Define`, `If`) and pre-bound boolean constants.
    pub fn new() -> Self {
        let mut env = HashMap::new();
        // Pre-bind True/False so they don't trigger on_unresolved.
        env.insert("True".to_string(), IRNode::Symbol("True".to_string()));
        env.insert("False".to_string(), IRNode::Symbol("False".to_string()));

        let held = ["Assign", "Define", "If"]
            .iter()
            .map(|s| s.to_string())
            .collect();

        Self { env, held }
    }
}

impl Default for BaseBackend {
    fn default() -> Self {
        Self::new()
    }
}
