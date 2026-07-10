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
        Stmt::ForRange {
            start,
            stop,
            step,
            body,
            ..
        } => {
            v.visit_expr(start);
            v.visit_expr(stop);
            v.visit_expr(step);
            v.visit_block(body);
        }
        Stmt::ForEach { iter, body, .. } => {
            v.visit_expr(iter);
            v.visit_block(body);
        }
        Stmt::SeqSet {
            seq, index, value, ..
        } => {
            v.visit_expr(seq);
            v.visit_expr(index);
            v.visit_expr(value);
        }
        Stmt::MapSet {
            map, key, value, ..
        } => {
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
        Stmt::TryCatch {
            body,
            rescues,
            ensure_body,
            ..
        } => {
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
        Stmt::IndexSet {
            target,
            indices,
            value,
            ..
        } => {
            // SIR22: `target[indices...] = value`. Recurse into the
            // target, every index-arg subexpression, and the value —
            // same shape as SeqSet/MapSet above.
            v.visit_expr(target);
            walk_index_args(v, indices);
            v.visit_expr(value);
        }
    }
}

/// Shared helper: visit every `Expr` nested inside a slice of
/// [`IndexArg`]s (SIR22).  `Whole` has no children; `Scalar`/`Range`
/// each carry exactly one nested expression.  Used by both
/// [`walk_stmt_default`] (for `Stmt::IndexSet`) and
/// [`walk_expr_default`] (for `Expr::IndexGet`) so the two index-arg
/// walks can't drift apart.
fn walk_index_args<V: Visitor>(v: &mut V, indices: &[IndexArg]) {
    for arg in indices {
        match arg {
            IndexArg::Scalar(e) => v.visit_expr(e),
            IndexArg::Whole => {}
            IndexArg::Range(e) => v.visit_expr(e),
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

        Expr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
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
        // ── KW1: keyword argument ──────────────────────────────────
        // A keyword argument has exactly one child, its `value`.  We
        // recurse into it so a visitor sees the argument expression (e.g.
        // the `1` in `f(a: 1)`).  The recursion is depth-bounded like the
        // rest of the walk: the same `visit_expr` dispatch that guards
        // every other child applies here — we add no bypass.
        Expr::KeywordArg { value, .. } => {
            v.visit_expr(value);
        }

        // ── SIR22: array/matrix nodes ───────────────────────────────
        Expr::ArrayLit { rows, .. } => {
            for row in rows {
                for item in row {
                    v.visit_expr(item);
                }
            }
        }
        Expr::Range {
            start, step, stop, ..
        } => {
            v.visit_expr(start);
            if let Some(step) = step {
                v.visit_expr(step);
            }
            v.visit_expr(stop);
        }
        Expr::MatMul { lhs, rhs, .. } => {
            v.visit_expr(lhs);
            v.visit_expr(rhs);
        }
        Expr::ElementwiseOp { lhs, rhs, .. } => {
            v.visit_expr(lhs);
            v.visit_expr(rhs);
        }
        Expr::Transpose { target, .. } => {
            v.visit_expr(target);
        }
        Expr::IndexGet {
            target, indices, ..
        } => {
            v.visit_expr(target);
            walk_index_args(v, indices);
        }

        // ── SIR26 ──────────────────────────────────────────────────
        Expr::Convert { value, .. } => {
            v.visit_expr(value);
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
                    Expr::IntLit {
                        value: 1,
                        span: s(),
                    },
                    Expr::IntLit {
                        value: 2,
                        span: s(),
                    },
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
        let mut c = Counter {
            builtins: 0,
            ints: 0,
        };
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
                value: Expr::IntLit {
                    value: 5,
                    span: s(),
                },
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
        let mut c = Counter {
            builtins: 0,
            ints: 0,
        };
        c.visit_module(&m);
        assert_eq!(c.ints, 1); // the literal 5 in the let RHS
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
                default: Some(Box::new(Expr::IntLit {
                    value: 7,
                    span: s(),
                })),
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
        let mut c = Counter {
            builtins: 0,
            ints: 0,
        };
        c.visit_module(&m);
        assert_eq!(
            c.ints, 1,
            "walker should visit the default-value expression"
        );
    }

    #[test]
    fn visitor_visits_keyword_arg_value() {
        // KW1: the walker must recurse into a KeywordArg's value, so the
        // IntLit `2` inside `f(a: 2)` is counted.
        let body = Block {
            stmts: vec![],
            value: Expr::DirectCall {
                fn_name: "f".into(),
                args: vec![Expr::KeywordArg {
                    name: "a".into(),
                    value: Box::new(Expr::IntLit {
                        value: 2,
                        span: s(),
                    }),
                    span: s(),
                }],
                effects: EffectSet::PURE,
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
        let mut c = Counter {
            builtins: 0,
            ints: 0,
        };
        c.visit_module(&m);
        assert_eq!(c.ints, 1, "walker should visit the keyword-arg value");
    }

    #[test]
    fn visitor_visits_if_branches() {
        let then_block = Block {
            stmts: vec![],
            value: Expr::IntLit {
                value: 1,
                span: s(),
            },
            span: s(),
        };
        let else_block = Block {
            stmts: vec![],
            value: Expr::IntLit {
                value: 2,
                span: s(),
            },
            span: s(),
        };
        let body = Block {
            stmts: vec![],
            value: Expr::If {
                cond: Box::new(Expr::BoolLit {
                    value: true,
                    span: s(),
                }),
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
        let mut c = Counter {
            builtins: 0,
            ints: 0,
        };
        c.visit_module(&m);
        assert_eq!(c.ints, 2); // both branches counted
    }

    // ── SIR22: array/matrix walker tests ─────────────────────────────

    fn module_with_body_value(value: Expr) -> Module {
        let f = Function {
            name: "f".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![],
                value,
                span: s(),
            },
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
    fn visitor_walks_array_lit_rows() {
        // [1 2; 3 4] — all four IntLits must be visited.
        let m = module_with_body_value(Expr::ArrayLit {
            rows: vec![
                vec![
                    Expr::IntLit {
                        value: 1,
                        span: s(),
                    },
                    Expr::IntLit {
                        value: 2,
                        span: s(),
                    },
                ],
                vec![
                    Expr::IntLit {
                        value: 3,
                        span: s(),
                    },
                    Expr::IntLit {
                        value: 4,
                        span: s(),
                    },
                ],
            ],
            span: s(),
        });
        let mut c = Counter {
            builtins: 0,
            ints: 0,
        };
        c.visit_module(&m);
        assert_eq!(c.ints, 4);
    }

    #[test]
    fn visitor_walks_range_with_and_without_step() {
        // 1:5 — no step: two ints (start, stop).
        let m = module_with_body_value(Expr::Range {
            start: Box::new(Expr::IntLit {
                value: 1,
                span: s(),
            }),
            step: None,
            stop: Box::new(Expr::IntLit {
                value: 5,
                span: s(),
            }),
            span: s(),
        });
        let mut c = Counter {
            builtins: 0,
            ints: 0,
        };
        c.visit_module(&m);
        assert_eq!(c.ints, 2);

        // 0:2:10 — with step: three ints.
        let m2 = module_with_body_value(Expr::Range {
            start: Box::new(Expr::IntLit {
                value: 0,
                span: s(),
            }),
            step: Some(Box::new(Expr::IntLit {
                value: 2,
                span: s(),
            })),
            stop: Box::new(Expr::IntLit {
                value: 10,
                span: s(),
            }),
            span: s(),
        });
        let mut c2 = Counter {
            builtins: 0,
            ints: 0,
        };
        c2.visit_module(&m2);
        assert_eq!(c2.ints, 3);
    }

    #[test]
    fn visitor_walks_matmul_and_elementwise_op_operands() {
        let matmul = module_with_body_value(Expr::MatMul {
            lhs: Box::new(Expr::IntLit {
                value: 1,
                span: s(),
            }),
            rhs: Box::new(Expr::IntLit {
                value: 2,
                span: s(),
            }),
            span: s(),
        });
        let mut c = Counter {
            builtins: 0,
            ints: 0,
        };
        c.visit_module(&matmul);
        assert_eq!(c.ints, 2);

        let ew = module_with_body_value(Expr::ElementwiseOp {
            op: ElementwiseOpKind::Add,
            lhs: Box::new(Expr::IntLit {
                value: 3,
                span: s(),
            }),
            rhs: Box::new(Expr::IntLit {
                value: 4,
                span: s(),
            }),
            span: s(),
        });
        let mut c2 = Counter {
            builtins: 0,
            ints: 0,
        };
        c2.visit_module(&ew);
        assert_eq!(c2.ints, 2);
    }

    #[test]
    fn visitor_walks_transpose_target() {
        let m = module_with_body_value(Expr::Transpose {
            target: Box::new(Expr::IntLit {
                value: 7,
                span: s(),
            }),
            conjugate: true,
            span: s(),
        });
        let mut c = Counter {
            builtins: 0,
            ints: 0,
        };
        c.visit_module(&m);
        assert_eq!(c.ints, 1);
    }

    #[test]
    fn visitor_walks_index_get_target_and_index_args() {
        // a(i, :, 1:3) — target `a` isn't an IntLit, but the Scalar `i`
        // index and the Range's start/stop are, so 3 ints total.
        let m = module_with_body_value(Expr::IndexGet {
            target: Box::new(Expr::VarRef {
                name: "a".into(),
                scope: Scope::Local,
                span: s(),
            }),
            indices: vec![
                IndexArg::Scalar(Box::new(Expr::IntLit {
                    value: 0,
                    span: s(),
                })),
                IndexArg::Whole,
                IndexArg::Range(Box::new(Expr::Range {
                    start: Box::new(Expr::IntLit {
                        value: 0,
                        span: s(),
                    }),
                    step: None,
                    stop: Box::new(Expr::IntLit {
                        value: 3,
                        span: s(),
                    }),
                    span: s(),
                })),
            ],
            span: s(),
        });
        let mut c = Counter {
            builtins: 0,
            ints: 0,
        };
        c.visit_module(&m);
        assert_eq!(c.ints, 3);
    }

    #[test]
    fn visitor_walks_index_set_stmt() {
        // a(1) = 9 — Stmt::IndexSet is a statement, not the block's value
        // expr, so it's exercised via a non-empty `stmts` list this time.
        let f = Function {
            name: "f".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![Stmt::IndexSet {
                    target: Box::new(Expr::VarRef {
                        name: "a".into(),
                        scope: Scope::Local,
                        span: s(),
                    }),
                    indices: vec![IndexArg::Scalar(Box::new(Expr::IntLit {
                        value: 0,
                        span: s(),
                    }))],
                    value: Box::new(Expr::IntLit {
                        value: 9,
                        span: s(),
                    }),
                    span: s(),
                }],
                value: Expr::NilLit { span: s() },
                span: s(),
            },
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
        let mut c = Counter {
            builtins: 0,
            ints: 0,
        };
        c.visit_module(&m);
        // The index-arg IntLit(0) and the value IntLit(9) — 2 total.
        assert_eq!(c.ints, 2);
    }
}
