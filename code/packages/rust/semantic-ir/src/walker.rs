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

use crate::limits::MAX_IR_DEPTH;
use crate::nodes::*;

/// A read-only visitor.  Implement only the methods you care about;
/// the defaults walk children automatically.
///
/// `visit_block`/`visit_stmt`/`visit_expr` carry an explicit `depth`
/// counter: `Block`/`Stmt`/`Expr` are mutually self-nesting (an `Expr`
/// can contain a `Block`, which contains `Stmt`s and an `Expr`, and so
/// on), so without a depth cap a pathologically deep tree could
/// recurse until it overflows the host stack — an uncatchable process
/// abort, not a normal Rust panic. The `walk_*_default` free functions
/// bound that recursion at [`crate::limits::MAX_IR_DEPTH`] by silently
/// truncating (matching `validator.rs`/`backend.rs`/`text::printer`'s
/// own depth guards): `Visitor` has no error-reporting channel of its
/// own, so — like `backend.rs`'s `walk_intrinsics_in_expr` — its job
/// is just "don't panic"; a validator pass is the right place to flag
/// pathologic nesting as a diagnostic.
///
/// Example — count `BuiltinCall` nodes:
/// ```
/// use semantic_ir::{Visitor, Expr, EffectSet, Span, Module, FeatureManifest, Metadata};
///
/// struct CountBuiltins(usize);
/// impl Visitor for CountBuiltins {
///     fn visit_expr(&mut self, e: &Expr, depth: usize) {
///         if let Expr::BuiltinCall { .. } = e {
///             self.0 += 1;
///         }
///         semantic_ir::walker::walk_expr_default(self, e, depth);
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

    fn visit_block(&mut self, b: &Block, depth: usize) {
        walk_block_default(self, b, depth);
    }

    fn visit_stmt(&mut self, s: &Stmt, depth: usize) {
        walk_stmt_default(self, s, depth);
    }

    fn visit_expr(&mut self, e: &Expr, depth: usize) {
        walk_expr_default(self, e, depth);
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
///
/// This is the entry point into the depth-counted `Block`/`Stmt`/`Expr`
/// walk, so it starts the count at `0`.
pub fn walk_function_default<V: Visitor>(v: &mut V, f: &Function) {
    for p in &f.params {
        if let Some(default) = &p.default {
            v.visit_expr(default, 0);
        }
    }
    v.visit_block(&f.body, 0);
}

/// Default walk for a block: visits each statement then the value
/// expression.
///
/// Depth-bounded by [`crate::limits::MAX_IR_DEPTH`]: once `depth`
/// reaches the cap this returns immediately without visiting the
/// block's contents, so a pathologically deep `Block`/`Stmt`/`Expr`
/// tree can't recurse the host stack into overflow. Silent
/// truncation, no error reported — see the [`Visitor`] docs.
pub fn walk_block_default<V: Visitor>(v: &mut V, b: &Block, depth: usize) {
    if depth >= MAX_IR_DEPTH {
        return;
    }
    for s in &b.stmts {
        v.visit_stmt(s, depth + 1);
    }
    v.visit_expr(&b.value, depth + 1);
}

/// Default walk for a statement.
///
/// Depth-bounded the same way as [`walk_block_default`]; see its docs.
pub fn walk_stmt_default<V: Visitor>(v: &mut V, s: &Stmt, depth: usize) {
    if depth >= MAX_IR_DEPTH {
        return;
    }
    match s {
        Stmt::LetBinding { value, .. } => v.visit_expr(value, depth + 1),
        Stmt::LetStarBinding { value, .. } => v.visit_expr(value, depth + 1),
        Stmt::ExprStmt { expr, .. } => v.visit_expr(expr, depth + 1),
        Stmt::Assign { value, .. } => v.visit_expr(value, depth + 1),
        Stmt::While { cond, body, .. } => {
            v.visit_expr(cond, depth + 1);
            v.visit_block(body, depth + 1);
        }
        Stmt::ForRange {
            start,
            stop,
            step,
            body,
            ..
        } => {
            v.visit_expr(start, depth + 1);
            v.visit_expr(stop, depth + 1);
            v.visit_expr(step, depth + 1);
            v.visit_block(body, depth + 1);
        }
        Stmt::ForEach { iter, body, .. } => {
            v.visit_expr(iter, depth + 1);
            v.visit_block(body, depth + 1);
        }
        Stmt::SeqSet {
            seq, index, value, ..
        } => {
            v.visit_expr(seq, depth + 1);
            v.visit_expr(index, depth + 1);
            v.visit_expr(value, depth + 1);
        }
        Stmt::MapSet {
            map, key, value, ..
        } => {
            v.visit_expr(map, depth + 1);
            v.visit_expr(key, depth + 1);
            v.visit_expr(value, depth + 1);
        }
        Stmt::ClassDef { body, .. } => {
            // Class body is a list of statements; recurse so visitors
            // see nested declarations.  Phase 14a always lowers an
            // empty body for `class Foo; end`, but later phases will
            // populate it; the walker is forward-compatible.
            for stmt in body {
                v.visit_stmt(stmt, depth + 1);
            }
        }
        Stmt::ModuleDef { body, .. } => {
            // Module body is a list of statements — same recursion as
            // ClassDef (Ruby Phase 14d).
            for stmt in body {
                v.visit_stmt(stmt, depth + 1);
            }
        }
        Stmt::SingletonClassDef { body, .. } => {
            // Singleton-class body — same recursion (Ruby Phase 14e).
            for stmt in body {
                v.visit_stmt(stmt, depth + 1);
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
                v.visit_stmt(stmt, depth + 1);
            }
            for r in rescues {
                for stmt in &r.body {
                    v.visit_stmt(stmt, depth + 1);
                }
            }
            if let Some(ens) = ensure_body {
                for stmt in ens {
                    v.visit_stmt(stmt, depth + 1);
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
            v.visit_expr(target, depth + 1);
            walk_index_args(v, indices, depth + 1);
            v.visit_expr(value, depth + 1);
        }
    }
}

/// Shared helper: visit every `Expr` nested inside a slice of
/// [`IndexArg`]s (SIR22).  `Whole` has no children; `Scalar`/`Range`
/// each carry exactly one nested expression.  Used by both
/// [`walk_stmt_default`] (for `Stmt::IndexSet`) and
/// [`walk_expr_default`] (for `Expr::IndexGet`) so the two index-arg
/// walks can't drift apart.
///
/// `depth` is passed through **unchanged** to the nested
/// `visit_expr` calls, not incremented again: index args are siblings
/// of the target/value at the same nesting level, not one level
/// deeper — the caller (`walk_stmt_default`/`walk_expr_default`) has
/// already applied `depth + 1` before calling in here, mirroring
/// `backend.rs`'s `walk_intrinsics_in_index_args`. No guard check of
/// its own: this helper never recurses back into `Block`/`Stmt`, so it
/// can't itself grow the call stack — the depth check on the
/// `visit_expr` calls it makes is enough.
fn walk_index_args<V: Visitor>(v: &mut V, indices: &[IndexArg], depth: usize) {
    for arg in indices {
        match arg {
            IndexArg::Scalar(e) => v.visit_expr(e, depth),
            IndexArg::Whole => {}
            IndexArg::Range(e) => v.visit_expr(e, depth),
        }
    }
}

/// Default walk for an expression — recurses into every sub-expression.
///
/// Depth-bounded the same way as [`walk_block_default`]; see its docs.
pub fn walk_expr_default<V: Visitor>(v: &mut V, e: &Expr, depth: usize) {
    if depth >= MAX_IR_DEPTH {
        return;
    }
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
            v.visit_expr(cond, depth + 1);
            v.visit_block(then_branch, depth + 1);
            v.visit_block(else_branch, depth + 1);
        }

        Expr::Block(b) => {
            v.visit_block(b, depth + 1);
        }

        Expr::DirectCall { args, .. } => {
            for a in args {
                v.visit_expr(a, depth + 1);
            }
        }

        Expr::IndirectCall { target, args, .. } => {
            v.visit_expr(target, depth + 1);
            for a in args {
                v.visit_expr(a, depth + 1);
            }
        }

        Expr::BuiltinCall { args, .. } => {
            for a in args {
                v.visit_expr(a, depth + 1);
            }
        }

        Expr::MakeClosure { captures, .. } => {
            for c in captures {
                v.visit_expr(&c.value, depth + 1);
            }
        }

        Expr::Intrinsic { args, .. } => {
            for a in args {
                v.visit_expr(a, depth + 1);
            }
        }

        // ── SIR16 additions ────────────────────────────────────────
        Expr::FloatLit { .. } => {}
        Expr::SeqLit { items, .. } => {
            for i in items {
                v.visit_expr(i, depth + 1);
            }
        }
        Expr::SeqIndex { seq, index, .. } => {
            v.visit_expr(seq, depth + 1);
            v.visit_expr(index, depth + 1);
        }
        Expr::SeqLen { seq, .. } => {
            v.visit_expr(seq, depth + 1);
        }
        Expr::MapLit { entries, .. } => {
            for entry in entries {
                v.visit_expr(&entry.key, depth + 1);
                v.visit_expr(&entry.value, depth + 1);
            }
        }
        Expr::MapGet { map, key, .. } => {
            v.visit_expr(map, depth + 1);
            v.visit_expr(key, depth + 1);
        }
        Expr::LogicalAnd { lhs, rhs, .. } | Expr::LogicalOr { lhs, rhs, .. } => {
            v.visit_expr(lhs, depth + 1);
            v.visit_expr(rhs, depth + 1);
        }
        // ── SIR18: string interpolation ────────────────────────────
        Expr::StrConcat { parts, .. } => {
            for p in parts {
                v.visit_expr(p, depth + 1);
            }
        }
        // ── KW1: keyword argument ──────────────────────────────────
        // A keyword argument has exactly one child, its `value`.  We
        // recurse into it so a visitor sees the argument expression (e.g.
        // the `1` in `f(a: 1)`).  The recursion is depth-bounded like the
        // rest of the walk: the same `visit_expr` dispatch that guards
        // every other child applies here — we add no bypass.
        Expr::KeywordArg { value, .. } => {
            v.visit_expr(value, depth + 1);
        }

        // ── SIR22: array/matrix nodes ───────────────────────────────
        Expr::ArrayLit { rows, .. } => {
            for row in rows {
                for item in row {
                    v.visit_expr(item, depth + 1);
                }
            }
        }
        Expr::Range {
            start, step, stop, ..
        } => {
            v.visit_expr(start, depth + 1);
            if let Some(step) = step {
                v.visit_expr(step, depth + 1);
            }
            v.visit_expr(stop, depth + 1);
        }
        Expr::MatMul { lhs, rhs, .. } => {
            v.visit_expr(lhs, depth + 1);
            v.visit_expr(rhs, depth + 1);
        }
        Expr::ElementwiseOp { lhs, rhs, .. } => {
            v.visit_expr(lhs, depth + 1);
            v.visit_expr(rhs, depth + 1);
        }
        Expr::Transpose { target, .. } => {
            v.visit_expr(target, depth + 1);
        }
        Expr::IndexGet {
            target, indices, ..
        } => {
            v.visit_expr(target, depth + 1);
            walk_index_args(v, indices, depth + 1);
        }

        // ── SIR22 addendum: APL primitive operators ─────────────────
        Expr::Reduce { target, .. } => {
            v.visit_expr(target, depth + 1);
        }
        Expr::Scan { target, .. } => {
            v.visit_expr(target, depth + 1);
        }
        Expr::OuterProduct { lhs, rhs, .. } => {
            v.visit_expr(lhs, depth + 1);
            v.visit_expr(rhs, depth + 1);
        }
        Expr::Shape { target, .. } => {
            v.visit_expr(target, depth + 1);
        }
        Expr::Reshape { shape, target, .. } => {
            v.visit_expr(shape, depth + 1);
            v.visit_expr(target, depth + 1);
        }
        Expr::IndexGenerator { count, .. } => {
            v.visit_expr(count, depth + 1);
        }
        Expr::IndexOf {
            haystack, needle, ..
        } => {
            v.visit_expr(haystack, depth + 1);
            v.visit_expr(needle, depth + 1);
        }
        Expr::Ravel { target, .. } => {
            v.visit_expr(target, depth + 1);
        }
        Expr::Catenate { lhs, rhs, .. } => {
            v.visit_expr(lhs, depth + 1);
            v.visit_expr(rhs, depth + 1);
        }

        // ── SIR26 ──────────────────────────────────────────────────
        Expr::Convert { value, .. } => {
            v.visit_expr(value, depth + 1);
        }

        // ── SIR23: symbolic expression + pattern/rewrite nodes ──────
        Expr::SymSymbol { .. } => {}
        Expr::SymRational { .. } => {}
        Expr::SymApply { head, args, .. } => {
            v.visit_expr(head, depth + 1);
            for a in args {
                v.visit_expr(a, depth + 1);
            }
        }
        Expr::SymPatternBlank { head, .. } => {
            if let Some(h) = head {
                v.visit_expr(h, depth + 1);
            }
        }
        Expr::SymPatternNamed { pattern, .. } => {
            v.visit_expr(pattern, depth + 1);
        }
        Expr::SymRule { lhs, rhs, .. } => {
            v.visit_expr(lhs, depth + 1);
            v.visit_expr(rhs, depth + 1);
        }
        Expr::SymReplaceAll { expr, rules, .. } => {
            v.visit_expr(expr, depth + 1);
            for r in rules {
                v.visit_expr(r, depth + 1);
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
        fn visit_expr(&mut self, e: &Expr, depth: usize) {
            match e {
                Expr::BuiltinCall { .. } => self.builtins += 1,
                Expr::IntLit { .. } => self.ints += 1,
                _ => {}
            }
            walk_expr_default(self, e, depth);
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

    // ── SIR22 addendum: APL primitive operator walker tests ──────────

    #[test]
    fn visitor_walks_reduce_target() {
        let m = module_with_body_value(Expr::Reduce {
            op: ElementwiseOpKind::Add,
            target: Box::new(Expr::IntLit {
                value: 7,
                span: s(),
            }),
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
    fn visitor_walks_scan_target() {
        let m = module_with_body_value(Expr::Scan {
            op: ElementwiseOpKind::Add,
            target: Box::new(Expr::IntLit {
                value: 7,
                span: s(),
            }),
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
    fn visitor_walks_outer_product_lhs_and_rhs() {
        let m = module_with_body_value(Expr::OuterProduct {
            op: ElementwiseOpKind::Mul,
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
        c.visit_module(&m);
        assert_eq!(c.ints, 2);
    }

    #[test]
    fn visitor_walks_shape_target() {
        let m = module_with_body_value(Expr::Shape {
            target: Box::new(Expr::IntLit {
                value: 1,
                span: s(),
            }),
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
    fn visitor_walks_reshape_shape_and_target() {
        let m = module_with_body_value(Expr::Reshape {
            shape: Box::new(Expr::IntLit {
                value: 2,
                span: s(),
            }),
            target: Box::new(Expr::IntLit {
                value: 1,
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
    }

    #[test]
    fn visitor_walks_index_generator_count() {
        let m = module_with_body_value(Expr::IndexGenerator {
            count: Box::new(Expr::IntLit {
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
        assert_eq!(c.ints, 1);
    }

    #[test]
    fn visitor_walks_index_of_haystack_and_needle() {
        let m = module_with_body_value(Expr::IndexOf {
            haystack: Box::new(Expr::IntLit {
                value: 1,
                span: s(),
            }),
            needle: Box::new(Expr::IntLit {
                value: 2,
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
    }

    #[test]
    fn visitor_walks_ravel_target() {
        let m = module_with_body_value(Expr::Ravel {
            target: Box::new(Expr::IntLit {
                value: 1,
                span: s(),
            }),
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
    fn visitor_walks_catenate_lhs_and_rhs() {
        let m = module_with_body_value(Expr::Catenate {
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
        c.visit_module(&m);
        assert_eq!(c.ints, 2);
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

    // ── SIR23: symbolic expression + pattern/rewrite walker tests ────

    #[test]
    fn visitor_walks_sym_apply_head_and_args() {
        // f(1, 2) as symbolic data: head `f` (not an IntLit) plus two
        // IntLit args.
        let m = module_with_body_value(Expr::SymApply {
            head: Box::new(Expr::SymSymbol {
                name: "f".into(),
                span: s(),
            }),
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
            span: s(),
        });
        let mut c = Counter {
            builtins: 0,
            ints: 0,
        };
        c.visit_module(&m);
        assert_eq!(c.ints, 2);
    }

    #[test]
    fn visitor_walks_sym_apply_computed_head() {
        // f[x][y] — the outer SymApply's head is itself a SymApply whose
        // own args contain an IntLit; the walker must recurse into it.
        let m = module_with_body_value(Expr::SymApply {
            head: Box::new(Expr::SymApply {
                head: Box::new(Expr::SymSymbol {
                    name: "f".into(),
                    span: s(),
                }),
                args: vec![Expr::IntLit {
                    value: 1,
                    span: s(),
                }],
                span: s(),
            }),
            args: vec![Expr::IntLit {
                value: 2,
                span: s(),
            }],
            span: s(),
        });
        let mut c = Counter {
            builtins: 0,
            ints: 0,
        };
        c.visit_module(&m);
        assert_eq!(c.ints, 2);
    }

    #[test]
    fn visitor_walks_sym_pattern_blank_head() {
        // `_h` — the head-constrained blank's `head` must be visited.
        // Use an IntLit stand-in (not a realistic head, but sufficient to
        // prove the walker recurses).
        let m = module_with_body_value(Expr::SymPatternBlank {
            head: Some(Box::new(Expr::IntLit {
                value: 1,
                span: s(),
            })),
            span: s(),
        });
        let mut c = Counter {
            builtins: 0,
            ints: 0,
        };
        c.visit_module(&m);
        assert_eq!(c.ints, 1);

        // Bare `_` (head: None) has no children to visit.
        let m2 = module_with_body_value(Expr::SymPatternBlank {
            head: None,
            span: s(),
        });
        let mut c2 = Counter {
            builtins: 0,
            ints: 0,
        };
        c2.visit_module(&m2);
        assert_eq!(c2.ints, 0);
    }

    #[test]
    fn visitor_walks_sym_pattern_named_pattern() {
        let m = module_with_body_value(Expr::SymPatternNamed {
            name: "x".into(),
            pattern: Box::new(Expr::SymPatternBlank {
                head: Some(Box::new(Expr::IntLit {
                    value: 7,
                    span: s(),
                })),
                span: s(),
            }),
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
    fn visitor_walks_sym_rule_lhs_and_rhs() {
        let m = module_with_body_value(Expr::SymRule {
            lhs: Box::new(Expr::IntLit {
                value: 1,
                span: s(),
            }),
            rhs: Box::new(Expr::IntLit {
                value: 2,
                span: s(),
            }),
            delayed: true,
            span: s(),
        });
        let mut c = Counter {
            builtins: 0,
            ints: 0,
        };
        c.visit_module(&m);
        assert_eq!(c.ints, 2);
    }

    #[test]
    fn visitor_walks_sym_replace_all_expr_and_rules() {
        // expr /. {rule1, rule2} — the target expr plus every rule in the
        // rules vec must be visited.
        let rule1 = Expr::SymRule {
            lhs: Box::new(Expr::IntLit {
                value: 1,
                span: s(),
            }),
            rhs: Box::new(Expr::IntLit {
                value: 2,
                span: s(),
            }),
            delayed: false,
            span: s(),
        };
        let rule2 = Expr::SymRule {
            lhs: Box::new(Expr::IntLit {
                value: 3,
                span: s(),
            }),
            rhs: Box::new(Expr::IntLit {
                value: 4,
                span: s(),
            }),
            delayed: false,
            span: s(),
        };
        let m = module_with_body_value(Expr::SymReplaceAll {
            expr: Box::new(Expr::IntLit {
                value: 0,
                span: s(),
            }),
            rules: vec![rule1, rule2],
            repeated: true,
            span: s(),
        });
        let mut c = Counter {
            builtins: 0,
            ints: 0,
        };
        c.visit_module(&m);
        // expr(0) + rule1(1,2) + rule2(3,4) = 5 IntLits total.
        assert_eq!(c.ints, 5);
    }

    #[test]
    fn visitor_sym_symbol_and_sym_rational_have_no_children() {
        let m = module_with_body_value(Expr::SymSymbol {
            name: "x".into(),
            span: s(),
        });
        let mut c = Counter {
            builtins: 0,
            ints: 0,
        };
        c.visit_module(&m);
        assert_eq!(c.ints, 0);

        let m2 = module_with_body_value(Expr::SymRational {
            numer: 1,
            denom: 2,
            span: s(),
        });
        let mut c2 = Counter {
            builtins: 0,
            ints: 0,
        };
        c2.visit_module(&m2);
        assert_eq!(c2.ints, 0);
    }

    // ── MAX_IR_DEPTH guard tests ─────────────────────────────────────
    //
    // `walk_block_default`/`walk_stmt_default`/`walk_expr_default` are
    // mutually recursive across `Block`/`Stmt`/`Expr`, so a
    // pathologically deep tree (attacker-controlled or accidental) has
    // to be capped or it can overflow the host stack — an uncatchable
    // process abort, not a `Result::Err`. These tests build such a
    // tree directly (bypassing any frontend) to exercise the walker's
    // own guard in isolation.

    /// Build a chain of `depth` nested `Expr::Block`s wrapping a single
    /// `IntLit` at the bottom.
    ///
    /// `Expr::Block(Box<Block>)` is the mechanically simplest
    /// self-nesting shape this IR offers: each level is exactly one
    /// `Expr` wrapping one `Block` wrapping one child `Expr`, so a
    /// plain loop can build it without any recursion of its own
    /// (unlike e.g. nested `Expr::If`, which would need two child
    /// `Block`s constructed per level for no added value here — one
    /// self-nesting child per level is enough to drive the depth
    /// counter through both `walk_block_default` and
    /// `walk_expr_default`).
    ///
    /// Built iteratively (not recursively) so constructing the fixture
    /// itself never risks overflowing the *test* thread's stack — only
    /// the walk under test is meant to be at risk of that, which is
    /// exactly what these tests check.
    fn build_deep_block_chain(depth: usize) -> Expr {
        let mut e = Expr::IntLit {
            value: 0,
            span: s(),
        };
        for _ in 0..depth {
            e = Expr::Block(Box::new(Block {
                stmts: vec![],
                value: e,
                span: s(),
            }));
        }
        e
    }

    /// A `Visitor` that does nothing but use the trait's own defaults —
    /// i.e. it exercises `walk_expr_default`/`walk_block_default`
    /// end-to-end with no overrides at all.
    struct NoOp;
    impl Visitor for NoOp {}

    #[test]
    fn deep_expr_tree_does_not_crash_default_visitor() {
        // Depth guard aside, a several-thousand-level `Box<Expr>` chain
        // also has a compiler-generated *recursive* `Drop` impl (each
        // `Box<Expr>` drop calls into the `Box<Block>` it owns, which
        // calls into the next `Box<Expr>`, ...) that this file's own
        // guard has no say over. We run on a dedicated thread with a
        // generous stack — the same pattern `validator.rs`'s
        // `depth_overflow_is_reported_not_panicked` test uses — and
        // leak the tree afterwards to skip that recursive teardown
        // entirely, so this test is purely about the walk itself.
        let handle = std::thread::Builder::new()
            .stack_size(64 * 1024 * 1024)
            .name("deep-walker-no-crash".into())
            .spawn(|| {
                let deep = build_deep_block_chain(5_000);
                let mut v = NoOp;
                v.visit_expr(&deep, 0);
                Box::leak(Box::new(deep));
            })
            .expect("spawn test thread");
        handle
            .join()
            .expect("walking a 5,000-deep Expr tree must not panic/abort");
    }

    #[test]
    fn deep_expr_tree_walk_is_truncated_at_max_ir_depth() {
        // Prove the guard is load-bearing: a `Visitor` that counts
        // every `Expr` node it visits, walked over a tree far deeper
        // than `MAX_IR_DEPTH`, must see a bounded number of nodes —
        // not the full input depth.
        struct DepthCounter(usize);
        impl Visitor for DepthCounter {
            fn visit_expr(&mut self, e: &Expr, depth: usize) {
                self.0 += 1;
                walk_expr_default(self, e, depth);
            }
        }

        let handle = std::thread::Builder::new()
            .stack_size(64 * 1024 * 1024)
            .name("deep-walker-truncation".into())
            .spawn(|| {
                let deep = build_deep_block_chain(MAX_IR_DEPTH + 500);
                let mut v = DepthCounter(0);
                v.visit_expr(&deep, 0);
                let visited = v.0;
                Box::leak(Box::new(deep));
                visited
            })
            .expect("spawn test thread");
        let visited = handle.join().expect("walk must not panic/abort");

        // Work out the exact expected count. Each level of this
        // `Expr::Block` chain costs *two* units of depth budget, not
        // one: `walk_expr_default` sees `Expr::Block(b)` and calls
        // `v.visit_block(b, depth + 1)`, and `walk_block_default` then
        // calls `v.visit_expr(&b.value, depth + 1)` again — so the
        // `Expr` at chain level `k` is visited with
        // `depth == 2 * k`. `DepthCounter::visit_expr` increments the
        // counter unconditionally before delegating to
        // `walk_expr_default`, so level `k` is always counted once it
        // is reached; `walk_expr_default`'s own guard
        // (`depth >= MAX_IR_DEPTH`) is what stops the chain from being
        // *expanded* any further, i.e. it prevents level `k + 1` from
        // ever being reached (and hence counted) once
        // `2 * k >= MAX_IR_DEPTH`.
        //
        // Levels 0..=(MAX_IR_DEPTH / 2) all get reached and counted
        // (integer division floors, which matches this recursion
        // exactly regardless of whether MAX_IR_DEPTH is even or odd —
        // verified by hand for MAX_IR_DEPTH in {1, 2, 3, 4, 5}), so the
        // total is `MAX_IR_DEPTH / 2 + 1` — well under the
        // `MAX_IR_DEPTH + 500` levels actually present in the input
        // tree, proving the walk was truncated rather than exhausting
        // the tree.
        let expected = MAX_IR_DEPTH / 2 + 1;
        assert_eq!(
            visited, expected,
            "expected the walk to be truncated at {expected} visited nodes \
             (MAX_IR_DEPTH / 2 + 1), got {visited}"
        );
        assert!(
            visited < MAX_IR_DEPTH + 500,
            "walk should not have reached the full {}-level input tree",
            MAX_IR_DEPTH + 500
        );
    }
}
