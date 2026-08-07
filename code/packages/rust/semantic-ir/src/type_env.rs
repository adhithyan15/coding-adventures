//! Shared operand-type lookup for consulting [`crate::op_select`]'s rules
//! from a backend emitter (SIR21 T3c-3 prerequisite).
//!
//! [`op_select::resolve_binary`]/[`op_select::resolve_numeric`] need each
//! operand's *carried* static type — but that type does not live on every
//! `Expr` occurrence. Looking at [`crate::nodes`], `sir_type: Option<SirType>`
//! is a field on **declaration sites** only: [`Param`], [`Capture`],
//! [`Stmt::LetBinding`]/[`Stmt::LetStarBinding`]. A bare `Expr::VarRef`
//! carries just a name and a [`Scope`] tag; a literal (`IntLit`, `FloatLit`,
//! …) carries no type field at all. So "what is this operand's static type?"
//! is not a lookup *on* the expression — it requires knowing which
//! declaration, if any, bound the name a `VarRef` refers to.
//!
//! This module is that lookup, factored out once so every backend consults
//! the *same* rule rather than re-deriving it six times (the same "shared
//! reusable engine, not six ad-hoc copies" discipline `op_select` itself
//! follows). It is deliberately small: a name → type map, populated by
//! walking a function's `Param`s/`Capture`s once, then updated incrementally
//! as a caller walks a `Block`'s statements in order (so a later
//! `LetBinding` of the same name correctly shadows an earlier one — SIR's
//! `Local` scope is ordinary lexical shadowing, not SSA). It does **not**
//! walk control flow itself (`If` branches, loops, nested `Block`s): each
//! backend already has its own statement-walking loop with its own
//! block-scoping rules, and duplicating that here would be exactly the kind
//! of premature, unexercised machinery this repo's own lessons warn against.
//! A caller updates the environment at the same point it already visits each
//! statement.
//!
//! No inference happens anywhere in this module — consistent with SIR10's
//! "disambiguation is the frontend's job" and `op_select`'s own discipline.
//! An expression that is not a direct `VarRef` to a declared name (a
//! literal, a call result, a nested arithmetic expression) resolves to
//! `None` (Dynamic), exactly as it does today with no [`TypeEnv`] consulted
//! at all — this module can only make `resolve_binary`/`resolve_numeric`
//! *reachable*, never *more permissive* than what a frontend explicitly
//! declared.
//!
//! **Not yet consulted by any backend.** No frontend populates `sir_type` on
//! any node today (every operand is `Dynamic`), so wiring a backend's
//! emitter to build and consult a [`TypeEnv`] is currently inert — it would
//! resolve to `RuntimeDispatch` on every real program, identically to not
//! consulting it at all. This module exists so that wiring, when it lands
//! per backend, has one correct, tested primitive to call rather than six
//! independent reimplementations.

use std::collections::HashMap;

use crate::nodes::{Capture, Expr, Function, Param, Scope, Stmt};
use crate::types::SirType;

/// A name → declared-type map for the locals, parameters, and captures
/// visible at some point in a function body.
#[derive(Debug, Default, Clone)]
pub struct TypeEnv {
    bindings: HashMap<String, Option<SirType>>,
}

impl TypeEnv {
    /// An empty environment — every name resolves to `None` (Dynamic).
    pub fn new() -> Self {
        Self::default()
    }

    /// Seed an environment from a function's parameters and captures (the
    /// bindings visible at the top of its body, before any statement runs).
    pub fn from_function(function: &Function) -> Self {
        let mut env = Self::new();
        for param in &function.params {
            env.declare_param(param);
        }
        for capture in &function.captures {
            env.declare_capture(capture);
        }
        env
    }

    /// Declare (or shadow) a name directly.
    pub fn declare(&mut self, name: &str, sir_type: Option<SirType>) {
        self.bindings.insert(name.to_string(), sir_type);
    }

    fn declare_param(&mut self, param: &Param) {
        self.declare(&param.name, param.sir_type.clone());
    }

    fn declare_capture(&mut self, capture: &Capture) {
        self.declare(&capture.name, capture.sir_type.clone());
    }

    /// Update the environment for one statement. Call this in the same
    /// order a caller already walks a `Block`'s `stmts` — a `LetBinding`/
    /// `LetStarBinding` of a name already in scope correctly overwrites the
    /// prior entry (lexical shadowing, matching SIR's `Local` scope
    /// semantics), and every other statement kind is a no-op here (it binds
    /// nothing new at this scope level, or binds something this module
    /// deliberately doesn't track yet — e.g. loop induction variables,
    /// which vary in shape per backend and are out of scope until a real
    /// wiring PR needs them).
    pub fn observe_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::LetBinding { name, sir_type, .. }
            | Stmt::LetStarBinding { name, sir_type, .. } => {
                self.declare(name, sir_type.clone());
            }
            _ => {}
        }
    }

    /// Resolve an expression's statically-known type, or `None` (Dynamic)
    /// if it isn't a direct reference to a name this environment knows
    /// about. Only `Local`/`Param`/`Capture` scopes are looked up —
    /// `Global`/`Instance`/`ClassVar`/`Const`/`Builtin` are out of scope for
    /// this first slice (each has its own declaration/mutation shape a
    /// future wiring PR can extend this module for, on demand).
    pub fn expr_type<'a>(&'a self, expr: &Expr) -> Option<&'a SirType> {
        match expr {
            Expr::VarRef {
                name,
                scope: Scope::Local | Scope::Param | Scope::Capture,
                ..
            } => self.bindings.get(name).and_then(|t| t.as_ref()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::EffectSet;
    use crate::metadata::Metadata;
    use crate::nodes::{Capture, Param, ParamKind};
    use crate::span::Span;
    use crate::types::{IntWidth, Overflow};

    fn span() -> Span {
        Span::synthetic()
    }

    fn i32_ty() -> SirType {
        SirType::int(IntWidth::W32, true, Overflow::Wrap)
    }

    fn var(name: &str, scope: Scope) -> Expr {
        Expr::VarRef {
            name: name.to_string(),
            scope,
            span: span(),
        }
    }

    #[test]
    fn empty_env_resolves_everything_to_dynamic() {
        let env = TypeEnv::new();
        assert_eq!(env.expr_type(&var("x", Scope::Local)), None);
        assert_eq!(env.expr_type(&Expr::IntLit { value: 5, span: span() }), None);
    }

    #[test]
    fn declared_local_resolves_to_its_type() {
        let mut env = TypeEnv::new();
        env.declare("x", Some(i32_ty()));
        assert_eq!(env.expr_type(&var("x", Scope::Local)), Some(&i32_ty()));
    }

    #[test]
    fn undeclared_name_is_dynamic_even_with_a_populated_env() {
        let mut env = TypeEnv::new();
        env.declare("x", Some(i32_ty()));
        assert_eq!(env.expr_type(&var("y", Scope::Local)), None);
    }

    #[test]
    fn declared_but_untyped_local_is_dynamic() {
        // A frontend can declare a name without supplying a static type
        // (today's universal case) — that's still Dynamic, not "unknown
        // whether it's declared".
        let mut env = TypeEnv::new();
        env.declare("x", None);
        assert_eq!(env.expr_type(&var("x", Scope::Local)), None);
    }

    #[test]
    fn non_varref_expressions_are_always_dynamic() {
        // No inference: a literal or nested expression never resolves to a
        // type, even when its "obvious" value would be typeable.
        let mut env = TypeEnv::new();
        env.declare("x", Some(i32_ty()));
        assert_eq!(env.expr_type(&Expr::IntLit { value: 1, span: span() }), None);
        assert_eq!(
            env.expr_type(&Expr::BuiltinCall {
                name: "+".into(),
                args: vec![var("x", Scope::Local), var("x", Scope::Local)],
                effects: EffectSet::default(),
                span: span(),
            }),
            None
        );
    }

    #[test]
    fn global_instance_and_const_scopes_are_not_looked_up() {
        // Deliberately out of scope for this slice (see module docs) — even
        // if a name of the same spelling is declared as a Local, a
        // different-scoped VarRef to it must not accidentally match.
        let mut env = TypeEnv::new();
        env.declare("x", Some(i32_ty()));
        assert_eq!(env.expr_type(&var("x", Scope::Global)), None);
        assert_eq!(env.expr_type(&var("x", Scope::Instance)), None);
        assert_eq!(env.expr_type(&var("x", Scope::Const)), None);
    }

    #[test]
    fn from_function_seeds_params_and_captures() {
        let f = Function {
            name: "f".into(),
            params: vec![Param {
                name: "a".into(),
                sir_type: Some(i32_ty()),
                kind: ParamKind::Required,
                default: None,
                span: span(),
            }],
            return_type: None,
            captures: vec![Capture {
                name: "b".into(),
                sir_type: Some(SirType::Float),
            }],
            body: crate::nodes::Block {
                stmts: vec![],
                value: Expr::NilLit { span: span() },
                span: span(),
            },
            effects: EffectSet::default(),
            metadata: Metadata::default(),
            span: span(),
        };
        let env = TypeEnv::from_function(&f);
        assert_eq!(env.expr_type(&var("a", Scope::Param)), Some(&i32_ty()));
        assert_eq!(env.expr_type(&var("b", Scope::Capture)), Some(&SirType::Float));
    }

    #[test]
    fn observe_stmt_declares_let_bindings_in_order() {
        let mut env = TypeEnv::new();
        // x: Dynamic, then reassigned to a typed i32 — later observation wins,
        // matching lexical shadowing.
        env.observe_stmt(&Stmt::LetBinding {
            name: "x".into(),
            sir_type: None,
            value: Expr::IntLit { value: 1, span: span() },
            span: span(),
        });
        assert_eq!(env.expr_type(&var("x", Scope::Local)), None);

        env.observe_stmt(&Stmt::LetStarBinding {
            name: "x".into(),
            sir_type: Some(i32_ty()),
            value: Expr::IntLit { value: 2, span: span() },
            span: span(),
        });
        assert_eq!(env.expr_type(&var("x", Scope::Local)), Some(&i32_ty()));
    }

    #[test]
    fn observe_stmt_ignores_non_binding_statements() {
        let mut env = TypeEnv::new();
        env.declare("x", Some(i32_ty()));
        env.observe_stmt(&Stmt::ExprStmt {
            expr: Expr::NilLit { span: span() },
            span: span(),
        });
        // Unrelated statement kinds leave the environment untouched.
        assert_eq!(env.expr_type(&var("x", Scope::Local)), Some(&i32_ty()));
    }
}
