//! The [`VM`] — a tree-walking evaluator over [`symbolic_ir`] nodes.
//!
//! ## Evaluation algorithm
//!
//! ```text
//! eval(node):
//!   atom (Symbol)
//!     → backend.lookup(name)          if bound: re-evaluate binding
//!     → backend.on_unresolved(name)   if unbound
//!   atom (Integer | Rational | Float | Str)
//!     → return unchanged
//!   Apply(head, args):
//!     if head is held:   new_args = args           (unevaluated)
//!     else:              new_args = [eval(a) ...]  (applicative order)
//!     for (pred, xform) in backend.rules():
//!       if pred(expr): return eval(xform(expr))
//!     if handler = backend.handler_for(head_name):
//!       return handler(vm, expr)
//!     if backend.lookup(head) == Define(name, params, body):
//!       return eval(substitute(body, params → new_args))
//!     return backend.on_unknown_head(expr)
//! ```
//!
//! The "self-loop guard" in symbol evaluation prevents `x := x` from
//! recursing forever.

use symbolic_ir::{IRApply, IRNode, DEFINE, LIST};

use crate::backend::Backend;

/// The symbolic tree evaluator.
///
/// Create one with `VM::new(backend)` then call `vm.eval(node)` or
/// `vm.eval_program(statements)`.
pub struct VM {
    /// The evaluation policy — language-specific behaviour lives here.
    pub backend: Box<dyn Backend>,
}

impl VM {
    /// Create a new VM backed by `backend`.
    pub fn new(backend: Box<dyn Backend>) -> Self {
        Self { backend }
    }

    // ------------------------------------------------------------------
    // Public entry points
    // ------------------------------------------------------------------

    /// Evaluate `node` and return the result.
    pub fn eval(&mut self, node: IRNode) -> IRNode {
        match node {
            IRNode::Symbol(ref name) => self.eval_symbol(name.clone(), node),
            IRNode::Apply(apply) => self.eval_apply(*apply),
            // Numeric and string literals pass through unchanged.
            other => other,
        }
    }

    /// Evaluate a sequence of statements; return the value of the last one.
    ///
    /// Mirrors a MACSYMA REPL: each statement is evaluated in order against
    /// the same backend environment.  An empty program returns `None`.
    pub fn eval_program(&mut self, statements: Vec<IRNode>) -> Option<IRNode> {
        let mut result = None;
        for stmt in statements {
            result = Some(self.eval(stmt));
        }
        result
    }

    // ------------------------------------------------------------------
    // Internal evaluation
    // ------------------------------------------------------------------

    fn eval_symbol(&mut self, name: String, original: IRNode) -> IRNode {
        // Clone the binding out of the backend (releases borrow) before
        // recursing into eval so borrow checker is satisfied.
        let value = self.backend.lookup(&name).map(|v| v.clone());
        match value {
            None => self.backend.on_unresolved(&name),
            Some(bound) => {
                // Self-loop guard: x := x would recurse forever without this.
                if bound == original {
                    return original;
                }
                self.eval(bound)
            }
        }
    }

    fn eval_apply(&mut self, node: IRApply) -> IRNode {
        let head_name = head_name(&node.head);

        // 1. Evaluate arguments unless the head holds them.
        let held = self.backend.hold_heads().contains(&head_name);
        let new_args: Vec<IRNode> = if held {
            node.args
        } else {
            node.args.into_iter().map(|a| self.eval(a)).collect()
        };
        let expr = IRApply {
            head: node.head,
            args: new_args,
        };

        // 2. Try rewrite rules (clone the rule Arc first to release the
        //    immutable borrow of self.backend before calling eval again).
        let rules: Vec<_> = self.backend.rules().iter().cloned().collect();
        for (pred, xform) in &rules {
            if pred(&expr) {
                let rewritten = xform(expr.clone());
                return self.eval(rewritten);
            }
        }

        // 3. Dispatch to a head-specific handler.
        //    Clone the Arc handler to release the borrow before calling.
        let handler = self
            .backend
            .handler_for(&head_name)
            .map(|h| h.clone());
        if let Some(handler) = handler {
            return handler(self, expr);
        }

        // 4. User-defined function? Check if the head symbol is bound to a
        //    Define record, and if so, inline-substitute and evaluate.
        if let IRNode::Symbol(ref sym_name) = expr.head {
            let bound = self.backend.lookup(sym_name).map(|v| v.clone());
            if let Some(definition) = bound {
                if is_define_record(&definition) {
                    if let IRNode::Apply(def) = definition {
                        // Clone expr.args before passing so expr stays whole for
                        // the on_unknown_head fallback.
                        let args = expr.args.clone();
                        let result = self.apply_user_function(*def, args);
                        if let Some(node) = result {
                            return self.eval(node);
                        }
                    }
                }
            }
        }

        // 5. No handler, no user function — fall back per backend policy.
        self.backend.on_unknown_head(expr)
    }

    // ------------------------------------------------------------------
    // User-defined function application
    // ------------------------------------------------------------------

    /// Substitute params → args in a `Define` body and return the body.
    ///
    /// The `Define` record has the shape:
    /// `Apply(sym("Define"), [name, List(param1, param2, …), body])`.
    ///
    /// Returns `None` if the record is malformed.
    fn apply_user_function(
        &self,
        definition: IRApply,
        args: Vec<IRNode>,
    ) -> Option<IRNode> {
        // definition.args == [name, List(params...), body]
        if definition.args.len() != 3 {
            return None;
        }
        let params_ir = &definition.args[1];
        let body = definition.args[2].clone();

        // Extract parameter names from List(p1, p2, …).
        let param_names: Vec<String> = match params_ir {
            IRNode::Apply(apply) if is_list_head(&apply.head) => apply
                .args
                .iter()
                .filter_map(|p| {
                    if let IRNode::Symbol(s) = p {
                        Some(s.clone())
                    } else {
                        None
                    }
                })
                .collect(),
            _ => return None,
        };

        if param_names.len() != args.len() {
            return None; // arity mismatch
        }

        let mapping: std::collections::HashMap<String, IRNode> =
            param_names.into_iter().zip(args).collect();
        Some(substitute(body, &mapping))
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Return the name of a symbol head, or `""` for non-symbol heads.
fn head_name(head: &IRNode) -> String {
    if let IRNode::Symbol(s) = head {
        s.clone()
    } else {
        String::new()
    }
}

/// True if `node` is an `Apply(sym("Define"), …)` stored binding.
fn is_define_record(node: &IRNode) -> bool {
    if let IRNode::Apply(a) = node {
        if let IRNode::Symbol(h) = &a.head {
            return h == DEFINE;
        }
    }
    false
}

/// True if `head` is the `List` symbol.
fn is_list_head(head: &IRNode) -> bool {
    if let IRNode::Symbol(s) = head {
        s == LIST
    } else {
        false
    }
}

/// Replace free occurrences of names in `node` with values from `mapping`.
///
/// Walks the tree structurally.  Symbols whose names appear in the mapping
/// are replaced; everything else passes through unchanged.  Both head and
/// args are substituted so that `f(x)` where `f` is in the mapping works.
pub fn substitute(node: IRNode, mapping: &std::collections::HashMap<String, IRNode>) -> IRNode {
    match node {
        IRNode::Symbol(ref name) => {
            if let Some(replacement) = mapping.get(name) {
                replacement.clone()
            } else {
                node
            }
        }
        IRNode::Apply(apply) => {
            let new_head = substitute(apply.head, mapping);
            let new_args: Vec<IRNode> = apply
                .args
                .into_iter()
                .map(|a| substitute(a, mapping))
                .collect();
            IRNode::Apply(Box::new(IRApply {
                head: new_head,
                args: new_args,
            }))
        }
        // Literals pass through unchanged.
        other => other,
    }
}
