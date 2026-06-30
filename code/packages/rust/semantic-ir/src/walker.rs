//! Visitor / walker utilities.
//!
//! The walker provides a depth-first traversal of a SIR module.
//! Backends and analyses use it to inspect every node without
//! re-implementing the traversal logic.  The visitor receives an
//! immutable reference to each node and may accumulate state in
//! `self`.
//!
//! Two traversal styles are exposed:
//!
//! - [`Visitor`] — fine-grained per-kind callbacks (override only the
//!   ones you care about).  All `visit_*` methods default to walking
//!   children.
//! - [`walk_*` free functions] — explicit traversal driven from the
//!   outside; useful when you want to compose two visitors over the
//!   same tree.

use crate::nodes::*;

/// A read-only visitor.  Implement only the methods you care about;
/// the defaults walk children automatically.
///
/// Example — count `BuiltinCall` nodes:
/// ```
/// use semantic_ir::{Visitor, Expr, EffectSet, Span, Module, FeatureManifest, Metadata};
///
/// struct CountBuiltins(usize);
/// impl Visitor for CountBuiltins {
///     fn visit_expr(&mut self, e: &Expr) {
///         if let Expr::BuiltinCall { .. } = e {
///             self.0 += 1;
///         }
///         semantic_ir::walker::walk_expr_default(self, e);
///     }
/// }
///
/// let module = Module {
///     name: "demo".into(), manifest: FeatureManifest::new(),
///     imports: vec![], exports: vec![], functions: vec![], globals: vec![],
///     metadata: Metadata::new(), span: Span::synthetic(),
/// };
/// let mut v = CountBuiltins(0);
/// v.visit_module(&module);
/// assert_eq!(v.0, 0);
/// ```
pub trait Visitor: Sized {
    fn visit_module(&mut self, m: &Module) {
        walk_module_default(self, m);
    }

    fn visit_function(&mut self, f: &Function) {
        walk_function_default(self, f);
    }

    fn visit_global(&mut self, _g: &Global) {}

    fn visit_block(&mut self, b: &Block) {
        walk_block_default(self, b);
    }

    fn visit_stmt(&mut self, s: &Stmt) {
        walk_stmt_default(self, s);
    }

    fn visit_expr(&mut self, e: &Expr) {
        walk_expr_default(self, e);
    }
}

/// Default walk for a module: visits globals then functions.
pub fn walk_module_default<V: Visitor>(v: &mut V, m: &Module) {
    for g in &m.globals {
        v.visit_global(g);
    }
    for f in &m.functions {
        v.visit_function(f);
    }
}

/// Default walk for a function: visits each parameter's default-value
/// expression (if any) and then the body block.  Visiting defaults
/// before the body keeps source-ish order (`def f(a = <expr>) …`) and
/// ensures passes that walk the IR observe the default expressions.
pub fn walk_function_default<V: Visitor>(v: &mut V, f: &Function) {
    for p in &f.params {
        if let Some(default) = &p.default {
            v.visit_expr(default);
        }
    }
    v.visit_block(&f.body);
}

/// Default walk for a block: visits each statement then the value expression.
pub fn walk_block_default<V: Visitor>(v: &mut V, b: &Block) {
    for s in &b.stmts {
        v.visit_stmt(s);
    }
    v.visit_expr(&b.value);
}

/// Default walk for a statement.
pub fn walk_stmt_default<V: Visitor>(v: &mut V, s: &Stmt) {
    match s {
        Stmt::LetBinding { value, .. } => v.visit_expr(value),
        Stmt::LetStarBinding { value, .. } => v.visit_expr(value),
        Stmt::ExprStmt { expr, .. } => v.visit_expr(expr),
        Stmt::Assign { value, .. } => v.visit_expr(value),
        Stmt::While { cond, body, .. } => {
            v.visit_expr(cond);
            v.visit_block(body);
        }
        Stmt::ForRange { start, stop, step, body, .. } => {
            v.visit_expr(start);
            v.visit_expr(stop);
            v.visit_expr(step);
            v.visit_block(body);
        }
        Stmt::ForEach { iter, body, .. } => {
            v.visit_expr(iter);
            v.visit_block(body);
        }
        Stmt::SeqSet { seq, index, value, .. } => {
            v.visit_expr(seq);
            v.visit_expr(index);
            v.visit_expr(value);
        }
        Stmt::MapSet { map, key, value, .. } => {
            v.visit_expr(map);
            v.visit_expr(key);
            v.visit_expr(value);
        }
        Stmt::ClassDef { body, .. } => {
            // Class body is a list of statements; recurse so visitors
            // see nested declarations.  Phase 14a always lowers an
            // empty body for `class Foo; end`, but later phases will
            // populate it; the walker is forward-compatible.
            for stmt in body {
                v.visit_stmt(stmt);
            }
        }
        Stmt::ModuleDef { body, .. } => {
            // Module body is a list of statements — same recursion as
            // ClassDef (Ruby Phase 14d).
            for stmt in body {
                v.visit_stmt(stmt);
            }
        }
        Stmt::SingletonClassDef { body, .. } => {
            // Singleton-class body — same recursion (Ruby Phase 14e).
            for stmt in body {
                v.visit_stmt(stmt);
            }
        }
        Stmt::TryCatch { body, rescues, ensure_body, .. } => {
            // Exception handling (Ruby Phase 16a): recurse into the try
            // body, each rescue clause's body, and the optional ensure
            // body, so visitors see every nested statement.
            for stmt in body {
                v.visit_stmt(stmt);
            }
            for r in rescues {
                for stmt in &r.body {
                    v.visit_stmt(stmt);
                }
            }
            if let Some(ens) = ensure_body {
                for stmt in ens {
                    v.visit_stmt(stmt);
                }
            }
        }
    }
}

/// Default walk for an expression — recurses into every sub-expression.
pub fn walk_expr_default<V: Visitor>(v: &mut V, e: &Expr) {
    match e {
        // Atoms and references have no children.
        Expr::IntLit { .. }
        | Expr::BoolLit { .. }
        | Expr::NilLit { .. }
        | Expr::SymLit { .. }
        | Expr::StrLit { .. }
        | Expr::VarRef { .. } => {}

        Expr::If { cond, then_branch, else_branch, .. } => {
            v.visit_expr(cond);
            v.visit_block(then_branch);
            v.visit_block(else_branch);
        }

        Expr::Block(b) => {
            v.visit_block(b);
        }

        Expr::DirectCall { args, .. } => {
            for a in args {
                v.visit_expr(a);
            }
        }

        Expr::IndirectCall { target, args, .. } => {
            v.visit_expr(target);
            for a in args {
                v.visit_expr(a);
            }
        }

        Expr::BuiltinCall { args, .. } => {
            for a in args {
                v.visit_expr(a);
            }
        }

        Expr::MakeClosure { captures, .. } => {
            for c in captures {
                v.visit_expr(&c.value);
            }
        }

        Expr::Intrinsic { args, .. } => {
            for a in args {
                v.visit_expr(a);
            }
        }

        // ── SIR16 additions ────────────────────────────────────────
        Expr::FloatLit { .. } => {}
        Expr::SeqLit { items, .. } => {
            for i in items {
                v.visit_expr(i);
            }
        }
        Expr::SeqIndex { seq, index, .. } => {
            v.visit_expr(seq);
            v.visit_expr(index);
        }
        Expr::SeqLen { seq, .. } => {
            v.visit_expr(seq);
        }
        Expr::MapLit { entries, .. } => {
            for entry in entries {
                v.visit_expr(&entry.key);
                v.visit_expr(&entry.value);
            }
        }
        Expr::MapGet { map, key, .. } => {
            v.visit_expr(map);
            v.visit_expr(key);
        }
        Expr::LogicalAnd { lhs, rhs, .. } | Expr::LogicalOr { lhs, rhs, .. } => {
            v.visit_expr(lhs);
            v.visit_expr(rhs);
        }
        // ── SIR18: string interpolation ────────────────────────────
        Expr::StrConcat { parts, .. } => {
            for p in parts {
                v.visit_expr(p);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::EffectSet;
    use crate::manifest::FeatureManifest;
    use crate::metadata::Metadata;
    use crate::span::Span;

    fn s() -> Span {
        Span::synthetic()
    }

    struct Counter {
        builtins: usize,
        ints: usize,
    }

    impl Visitor for Counter {
        fn visit_expr(&mut self, e: &Expr) {
            match e {
                Expr::BuiltinCall { .. } => self.builtins += 1,
                Expr::IntLit { .. } => self.ints += 1,
                _ => {}
            }
            walk_expr_default(self, e);
        }
    }

    fn sample_module() -> Module {
        // (define (f) (+ 1 2))
        let body = Block {
            stmts: vec![],
            value: Expr::BuiltinCall {
                name: "+".into(),
                args: vec![
                    Expr::IntLit { value: 1, span: s() },
                    Expr::IntLit { value: 2, span: s() },
                ],
                effects: EffectSet::PURE,
                span: s(),
            },
            span: s(),
        };
        let f = Function {
            name: "f".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body,
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        };
        Module {
            name: "m".into(),
            manifest: FeatureManifest::new(),
            imports: vec![],
            exports: vec![],
            functions: vec![f],
            globals: vec![],
            metadata: Metadata::new(),
            span: s(),
        }
    }

    #[test]
    fn visitor_walks_full_tree() {
        let m = sample_module();
        let mut c = Counter { builtins: 0, ints: 0 };
        c.visit_module(&m);
        assert_eq!(c.builtins, 1);
        assert_eq!(c.ints, 2);
    }

    #[test]
    fn visitor_visits_let_binding_value() {
        // (define (g) (let ((x 5)) x))
        let body = Block {
            stmts: vec![Stmt::LetBinding {
                name: "x".into(),
                sir_type: None,
                value: Expr::IntLit { value: 5, span: s() },
                span: s(),
            }],
            value: Expr::VarRef {
                name: "x".into(),
                scope: Scope::Local,
                span: s(),
            },
            span: s(),
        };
        let f = Function {
            name: "g".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body,
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        };
        let m = Module {
            name: "m".into(),
            manifest: FeatureManifest::new(),
            imports: vec![],
            exports: vec![],
            functions: vec![f],
            globals: vec![],
            metadata: Metadata::new(),
            span: s(),
        };
        let mut c = Counter { builtins: 0, ints: 0 };
        c.visit_module(&m);
        assert_eq!(c.ints, 1);  // the literal 5 in the let RHS
    }

    #[test]
    fn visitor_visits_param_default_expr() {
        // def f(a = 7) { nil } — the walker should visit the default
        // expression `7`, so the IntLit counter sees it.
        let body = Block {
            stmts: vec![],
            value: Expr::NilLit { span: s() },
            span: s(),
        };
        let f = Function {
            name: "f".into(),
            params: vec![Param {
                name: "a".into(),
                sir_type: None,
                kind: ParamKind::Required,
                default: Some(Box::new(Expr::IntLit { value: 7, span: s() })),
                span: s(),
            }],
            return_type: None,
            captures: vec![],
            body,
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        };
        let m = Module {
            name: "m".into(),
            manifest: FeatureManifest::new(),
            imports: vec![],
            exports: vec![],
            functions: vec![f],
            globals: vec![],
            metadata: Metadata::new(),
            span: s(),
        };
        let mut c = Counter { builtins: 0, ints: 0 };
        c.visit_module(&m);
        assert_eq!(c.ints, 1, "walker should visit the default-value expression");
    }

    #[test]
    fn visitor_visits_if_branches() {
        let then_block = Block {
            stmts: vec![],
            value: Expr::IntLit { value: 1, span: s() },
            span: s(),
        };
        let else_block = Block {
            stmts: vec![],
            value: Expr::IntLit { value: 2, span: s() },
            span: s(),
        };
        let body = Block {
            stmts: vec![],
            value: Expr::If {
                cond: Box::new(Expr::BoolLit { value: true, span: s() }),
                then_branch: Box::new(then_block),
                else_branch: Box::new(else_block),
                span: s(),
            },
            span: s(),
        };
        let f = Function {
            name: "h".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body,
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        };
        let m = Module {
            name: "m".into(),
            manifest: FeatureManifest::new(),
            imports: vec![],
            exports: vec![],
            functions: vec![f],
            globals: vec![],
            metadata: Metadata::new(),
            span: s(),
        };
        let mut c = Counter { builtins: 0, ints: 0 };
        c.visit_module(&m);
        assert_eq!(c.ints, 2);  // both branches counted
    }
}
