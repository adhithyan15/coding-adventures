//! S-expression printer for SIR modules.
//!
//! The output is deterministic: every input renders the same way
//! every time.  Whitespace is not significant for SIR semantics, but
//! the printer chooses a single canonical layout so byte-level
//! golden tests are reliable.
//!
//! ## Layout choices
//!
//! - Two-space indentation.
//! - One node per line at the top of a form, with children indented.
//! - Atoms (`(int 42)`, `(nil)`, `(var-ref x param)`) on a single
//!   line.
//! - Effects rendered as `(effects pure)` or `(effects may-print
//!   may-allocate)` — always present on call / function nodes for
//!   consistency, even when pure.
//! - Spans are rendered as a single inline form `(span file L1:C1-L2:C2)`
//!   attached to the node *footer*; this keeps the head keyword
//!   reading naturally on the first line.

use std::fmt::Write;

use crate::effects::EffectSet;
use crate::metadata::Metadata;
use crate::nodes::*;
use crate::types::SirType;

/// Render a module to canonical SIR text.
pub fn print_module(m: &Module) -> String {
    let mut out = String::new();
    let _ = write!(out, "(sir-module {} v{}", m.name, sir_version(&m.metadata));
    if !m.manifest.is_empty() {
        let _ = write!(out, "\n  (manifest {})", m.manifest);
    }
    if !m.metadata.is_empty() {
        let _ = write!(out, "\n  {}", m.metadata);
    }
    for imp in &m.imports {
        let _ = write!(out, "\n  ");
        print_import(&mut out, imp);
    }
    for ex in &m.exports {
        let _ = write!(out, "\n  (export {})", ex.name);
    }
    for g in &m.globals {
        let _ = write!(out, "\n  ");
        print_global(&mut out, g);
    }
    for f in &m.functions {
        let _ = write!(out, "\n\n  ");
        print_function_indented(&mut out, f, 2);
    }
    out.push_str(")\n");
    out
}

fn sir_version(meta: &Metadata) -> String {
    meta.sir_version.clone().unwrap_or_else(|| "0".into())
}

fn print_import(out: &mut String, imp: &Import) {
    let _ = write!(out, "(import {}", imp.module_path);
    for n in &imp.names {
        if n.source_name == n.local_name {
            let _ = write!(out, " {}", n.source_name);
        } else {
            let _ = write!(out, " ({} {})", n.source_name, n.local_name);
        }
    }
    out.push(')');
}

fn print_global(out: &mut String, g: &Global) {
    let _ = write!(out, "(global {}", g.name);
    if let Some(t) = &g.sir_type {
        let _ = write!(out, " {}", t);
    }
    let _ = write!(out, " (init-function {}))", g.init_function);
}

/// Top-level helper exposed publicly — useful for diagnostics that
/// want to render one function.
pub fn print_function(f: &Function) -> String {
    let mut out = String::new();
    print_function_indented(&mut out, f, 0);
    out.push('\n');
    out
}

fn print_function_indented(out: &mut String, f: &Function, indent: usize) {
    let pad = " ".repeat(indent);
    let _ = write!(out, "(function {} (", f.name);
    for (i, p) in f.params.iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        let _ = write!(out, "({} {})", p.name, type_or_any(p.sir_type.as_ref()));
    }
    let _ = write!(out, ") {}", type_or_any(f.return_type.as_ref()));
    if !f.captures.is_empty() {
        let _ = write!(out, " (captures");
        for c in &f.captures {
            let _ = write!(out, " ({} {})", c.name, type_or_any(c.sir_type.as_ref()));
        }
        out.push(')');
    }
    let _ = write!(out, " (effects {})\n{}  ", f.effects, pad);
    print_block(out, &f.body, indent + 2);
    out.push(')');
}

fn type_or_any(t: Option<&SirType>) -> String {
    t.map(|t| t.to_string()).unwrap_or_else(|| "any".to_string())
}

/// Render a block.  `indent` is the column the *contents* should
/// start at (the opening `(block` keyword is emitted by the caller's
/// indentation).
pub fn print_block(out: &mut String, b: &Block, indent: usize) {
    let pad = " ".repeat(indent);
    let _ = write!(out, "(block");
    if b.stmts.is_empty() {
        out.push(' ');
        print_expr_inline(out, &b.value);
    } else {
        for s in &b.stmts {
            let _ = write!(out, "\n{}  ", pad);
            print_stmt(out, s, indent + 2);
        }
        let _ = write!(out, "\n{}  ", pad);
        print_expr_inline(out, &b.value);
    }
    out.push(')');
}

fn print_stmt(out: &mut String, s: &Stmt, indent: usize) {
    match s {
        Stmt::LetBinding { name, sir_type, value, .. } => {
            let _ = write!(out, "(let {}", name);
            if let Some(t) = sir_type {
                let _ = write!(out, " {}", t);
            }
            out.push(' ');
            print_expr_inline(out, value);
            out.push(')');
        }
        Stmt::LetStarBinding { name, sir_type, value, .. } => {
            let _ = write!(out, "(let* {}", name);
            if let Some(t) = sir_type {
                let _ = write!(out, " {}", t);
            }
            out.push(' ');
            print_expr_inline(out, value);
            out.push(')');
        }
        Stmt::ExprStmt { expr, .. } => {
            let _ = write!(out, "(stmt ");
            print_expr_inline(out, expr);
            out.push(')');
        }
    }
    // Suppress unused indent warning when stmts are short — keep for
    // future multi-line variants.
    let _ = indent;
}

/// Render an expression as a single-line s-expression.  Public for
/// diagnostics.
pub fn print_expr(e: &Expr) -> String {
    let mut out = String::new();
    print_expr_inline(&mut out, e);
    out
}

fn print_expr_inline(out: &mut String, e: &Expr) {
    match e {
        Expr::IntLit { value, .. } => {
            let _ = write!(out, "(int {})", value);
        }
        Expr::BoolLit { value, .. } => {
            let _ = write!(out, "(bool {})", value);
        }
        Expr::NilLit { .. } => {
            let _ = write!(out, "(nil)");
        }
        Expr::SymLit { name, .. } => {
            let _ = write!(out, "(sym {})", name);
        }
        Expr::StrLit { value, .. } => {
            let _ = write!(out, "(str {})", quote_string(value));
        }
        Expr::VarRef { name, scope, .. } => {
            let _ = write!(out, "(var-ref {} {})", name, scope.name());
        }
        Expr::If { cond, then_branch, else_branch, .. } => {
            let _ = write!(out, "(if ");
            print_expr_inline(out, cond);
            out.push(' ');
            print_block(out, then_branch, 0);
            out.push(' ');
            print_block(out, else_branch, 0);
            out.push(')');
        }
        Expr::Block(b) => {
            print_block(out, b, 0);
        }
        Expr::DirectCall { fn_name, args, effects, .. } => {
            let _ = write!(out, "(direct-call {} ", fn_name);
            print_effects(out, effects);
            print_args(out, args);
            out.push(')');
        }
        Expr::IndirectCall { target, args, effects, .. } => {
            let _ = write!(out, "(indirect-call ");
            print_expr_inline(out, target);
            out.push(' ');
            print_effects(out, effects);
            print_args(out, args);
            out.push(')');
        }
        Expr::BuiltinCall { name, args, effects, .. } => {
            let _ = write!(out, "(builtin-call {} ", name);
            print_effects(out, effects);
            print_args(out, args);
            out.push(')');
        }
        Expr::MakeClosure { fn_name, captures, .. } => {
            let _ = write!(out, "(make-closure {}", fn_name);
            for c in captures {
                let _ = write!(out, " ({} ", c.name);
                print_expr_inline(out, &c.value);
                out.push(')');
            }
            out.push(')');
        }
        Expr::Intrinsic { targets, name, args, return_type, effects, .. } => {
            let _ = write!(out, "(intrinsic ({}) {} {} ", join_strs(targets), name, return_type);
            print_effects(out, effects);
            print_args(out, args);
            out.push(')');
        }
    }
}

fn print_effects(out: &mut String, e: &EffectSet) {
    let _ = write!(out, "(effects {})", e);
}

fn print_args(out: &mut String, args: &[Expr]) {
    for a in args {
        out.push(' ');
        print_expr_inline(out, a);
    }
}

fn join_strs(items: &[String]) -> String {
    items.join(" ")
}

fn quote_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::Effect;
    use crate::manifest::{Feature, FeatureManifest};
    use crate::metadata::{Metadata, CURRENT_SIR_VERSION};
    use crate::nodes::Scope;
    use crate::span::Span;

    fn s() -> Span {
        Span::synthetic()
    }

    fn module_with(fns: Vec<Function>, manifest: FeatureManifest) -> Module {
        Module {
            name: "demo".into(),
            manifest,
            imports: vec![],
            exports: vec![],
            functions: fns,
            globals: vec![],
            metadata: Metadata::new().with_sir_version(CURRENT_SIR_VERSION),
            span: s(),
        }
    }

    #[test]
    fn print_empty_module() {
        let m = module_with(vec![], FeatureManifest::new());
        let t = print_module(&m);
        assert!(t.starts_with("(sir-module demo v0"));
        assert!(t.contains("(metadata"));
        assert!(t.ends_with(")\n"));
    }

    #[test]
    fn print_module_is_deterministic() {
        let m = module_with(vec![], FeatureManifest::new());
        assert_eq!(print_module(&m), print_module(&m));
    }

    #[test]
    fn print_integer_literal() {
        let e = Expr::IntLit { value: 42, span: s() };
        assert_eq!(print_expr(&e), "(int 42)");
    }

    #[test]
    fn print_negative_integer() {
        let e = Expr::IntLit { value: -7, span: s() };
        assert_eq!(print_expr(&e), "(int -7)");
    }

    #[test]
    fn print_booleans_and_nil() {
        assert_eq!(print_expr(&Expr::BoolLit { value: true, span: s() }), "(bool true)");
        assert_eq!(print_expr(&Expr::BoolLit { value: false, span: s() }), "(bool false)");
        assert_eq!(print_expr(&Expr::NilLit { span: s() }), "(nil)");
    }

    #[test]
    fn print_var_ref_carries_scope() {
        let e = Expr::VarRef {
            name: "x".into(),
            scope: Scope::Local,
            span: s(),
        };
        assert_eq!(print_expr(&e), "(var-ref x local)");
    }

    #[test]
    fn print_builtin_call_with_pure() {
        let e = Expr::BuiltinCall {
            name: "+".into(),
            args: vec![
                Expr::IntLit { value: 1, span: s() },
                Expr::IntLit { value: 2, span: s() },
            ],
            effects: EffectSet::PURE,
            span: s(),
        };
        assert_eq!(
            print_expr(&e),
            "(builtin-call + (effects pure) (int 1) (int 2))"
        );
    }

    #[test]
    fn print_builtin_call_with_effects() {
        let e = Expr::BuiltinCall {
            name: "print".into(),
            args: vec![Expr::IntLit { value: 99, span: s() }],
            effects: EffectSet::PURE.with(Effect::MayPrint),
            span: s(),
        };
        assert_eq!(
            print_expr(&e),
            "(builtin-call print (effects may-print) (int 99))"
        );
    }

    #[test]
    fn print_string_escapes() {
        let e = Expr::StrLit {
            value: "hello \"world\"\n".into(),
            span: s(),
        };
        assert_eq!(print_expr(&e), r#"(str "hello \"world\"\n")"#);
    }

    #[test]
    fn print_make_closure_captures() {
        let e = Expr::MakeClosure {
            fn_name: "__lambda_0".into(),
            captures: vec![CaptureValue {
                name: "n".into(),
                value: Expr::IntLit { value: 5, span: s() },
            }],
            span: s(),
        };
        assert_eq!(
            print_expr(&e),
            "(make-closure __lambda_0 (n (int 5)))"
        );
    }

    #[test]
    fn print_full_module_with_function() {
        let body = Block {
            stmts: vec![],
            value: Expr::BuiltinCall {
                name: "+".into(),
                args: vec![
                    Expr::VarRef {
                        name: "x".into(),
                        scope: Scope::Param,
                        span: s(),
                    },
                    Expr::VarRef {
                        name: "y".into(),
                        scope: Scope::Param,
                        span: s(),
                    },
                ],
                effects: EffectSet::PURE,
                span: s(),
            },
            span: s(),
        };
        let f = Function {
            name: "add".into(),
            params: vec![
                Param { name: "x".into(), sir_type: None, span: s() },
                Param { name: "y".into(), sir_type: None, span: s() },
            ],
            return_type: None,
            captures: vec![],
            body,
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        };
        let m = module_with(
            vec![f],
            FeatureManifest::from_features(&[Feature::DynamicTyping]),
        );
        let text = print_module(&m);
        assert!(text.contains("(function add ((x any) (y any)) any (effects pure)"));
        assert!(text.contains("(builtin-call + (effects pure) (var-ref x param) (var-ref y param))"));
    }

    #[test]
    fn print_let_binding() {
        let s_ = s();
        let block = Block {
            stmts: vec![Stmt::LetBinding {
                name: "x".into(),
                sir_type: None,
                value: Expr::IntLit { value: 1, span: s_.clone() },
                span: s_.clone(),
            }],
            value: Expr::VarRef {
                name: "x".into(),
                scope: Scope::Local,
                span: s_.clone(),
            },
            span: s_,
        };
        let mut out = String::new();
        print_block(&mut out, &block, 0);
        assert!(out.contains("(let x (int 1))"));
        assert!(out.contains("(var-ref x local)"));
    }
}
