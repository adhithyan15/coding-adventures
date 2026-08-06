//! The rung-0 CAS-wiring [`symbolic_vm::Backend`] (ADJ-FORMULA-LIBRARIES FL-10
//! §3D): the minimal policy object that lets a `symbolic … for <var>` clause
//! reach `cas-solve` through the same `Backend`/`VM` seam every other CAS
//! dialect in this workspace uses (mirroring `macsyma-runtime`'s
//! `MacsymaBackend`/`solve_handler`, the one existing example) — rather than
//! calling `cas-solve`'s solver functions as free functions from `adj-lang`
//! directly.
//!
//! Rung-0 needs exactly one capability — solve a linear equation for one
//! variable — so this backend registers exactly one handler, for the `Solve`
//! head, and holds no other state: no bindings (`lookup` always returns
//! `None`), no other handlers, no rewrite rules. The equation `adj-lang`
//! builds is already fully reduced (every non-target identifier has been
//! evaluated to a plain number through `logic-engine`'s own `compute` path
//! before it ever reaches this module — see `lower.rs`'s `expr_to_irnode`),
//! so nothing here ever needs to resolve a name.

use std::collections::HashSet;
use std::sync::Arc;

use symbolic_ir::{IRApply, IRNode};
use symbolic_vm::{Backend, Handler, VM};

/// Registered under `cas_solve::SOLVE` (`"Solve"`) by [`RungZeroBackend::new`].
const SOLVE_HEAD: &str = cas_solve::SOLVE;

pub struct RungZeroBackend {
    handlers: std::collections::HashMap<String, Handler>,
    held: HashSet<String>,
}

impl Default for RungZeroBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl RungZeroBackend {
    pub fn new() -> Self {
        let mut handlers = std::collections::HashMap::new();
        handlers.insert(SOLVE_HEAD.to_string(), Arc::new(solve_handler) as Handler);
        let held = [SOLVE_HEAD].into_iter().map(str::to_string).collect();
        Self { handlers, held }
    }
}

impl Backend for RungZeroBackend {
    fn lookup(&self, _name: &str) -> Option<IRNode> {
        None
    }

    fn bind(&mut self, _name: &str, _value: IRNode) {}

    fn on_unresolved(&self, name: &str) -> IRNode {
        symbolic_ir::sym(name)
    }

    fn handler_for(&self, head_name: &str) -> Option<&Handler> {
        self.handlers.get(head_name)
    }

    fn hold_heads(&self) -> &HashSet<String> {
        &self.held
    }
}

/// `Solve(Equal(lhs, rhs), target)` — a single equation, a single target
/// symbol. Delegates the actual linear-coefficient extraction and Gaussian
/// elimination to [`cas_solve::solve_linear_system`] (a square system of
/// dimension 1 is the degenerate, still-fully-general case), returning its
/// `Rule(target, value)` node directly (no `List` wrapping — unlike MACSYMA's
/// `solve`, this rung's caller always wants exactly one equation's exactly one
/// answer, never a list of alternative solution sets).
///
/// Falls back to the unevaluated `Solve(...)` expression — exactly like
/// `macsyma-runtime`'s `solve_handler` — when `args` isn't `[equation,
/// symbol]` or the system has no unique solution (singular: no solution, or
/// every value satisfies it — `solve_linear_system` cannot distinguish these,
/// see [`crate::lower::LowerError::SymbolicUnsolvable`]). The caller
/// (`lower.rs`'s `apply_symbolic`) tells "solved" from "fell back" by
/// inspecting the result's head.
fn solve_handler(_vm: &mut VM, expr: IRApply) -> IRNode {
    let fallback = IRNode::Apply(Box::new(expr.clone()));
    if expr.args.len() != 2 {
        return fallback;
    }
    if !matches!(expr.args[1], IRNode::Symbol(_)) {
        return fallback;
    }
    let equations = [expr.args[0].clone()];
    let variables = [expr.args[1].clone()];
    match cas_solve::solve_linear_system(&equations, &variables) {
        Some(rules) if rules.len() == 1 => rules.into_iter().next().unwrap(),
        _ => fallback,
    }
}
