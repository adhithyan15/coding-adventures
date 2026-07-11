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
use crate::limits::MAX_IR_DEPTH;
use crate::metadata::Metadata;
use crate::nodes::*;
use crate::types::SirType;

/// Sentinel inserted into the output when recursion exceeds
/// `MAX_IR_DEPTH`.  Keeps the printer safe to call on hostile or
/// pathologically deep modules without panicking.
const TRUNCATION_MARKER: &str = "(<depth-limit>)";

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
    // When a module carries no explicit version, print the version this
    // build implements (rather than a hard-coded literal that silently
    // drifts on every SIR bump — as it did between "0" and "1").
    meta.sir_version
        .clone()
        .unwrap_or_else(|| crate::metadata::CURRENT_SIR_VERSION.into())
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
        // Kinds render with a Ruby-faithful sigil so a round-tripped
        // module preserves the parameter's binding form:
        //   - `Rest`    → `*name`   prefix (slurps positionals)
        //   - `KwRest`  → `**name`  prefix (slurps keywords)
        //   - `Keyword` → `name:`   *suffix* (a named keyword param — the
        //     trailing colon mirrors Ruby `def f(x:)` / `def f(x: 1)`)
        //   - `Required`→ plain `name`
        // A keyword param uses a suffix (not a prefix) so its printed form
        // reads exactly like the Ruby source that produced it.
        let (prefix, suffix) = match p.kind {
            ParamKind::Required => ("", ""),
            ParamKind::Rest => ("*", ""),
            ParamKind::Keyword => ("", ":"),
            ParamKind::KwRest => ("**", ""),
        };
        // A parameter with a default-value expression renders an extra
        // `(default <expr>)` clause inside the param form, e.g.
        // `(a any (default (int 1)))`.  Params with no default keep the
        // original `(name type)` shape, so existing modules round-trip
        // unchanged.  For a keyword param the same clause distinguishes an
        // OPTIONAL keyword (`(x: any (default (int 1)))`, Ruby `x: 1`) from
        // a REQUIRED one (`(x: any)`, Ruby `x:`).
        if let Some(default) = &p.default {
            let _ = write!(
                out,
                "({}{}{} {} (default ",
                prefix,
                p.name,
                suffix,
                type_or_any(p.sir_type.as_ref())
            );
            print_expr_inline_depth(out, default, 0);
            out.push_str("))");
        } else {
            let _ = write!(
                out,
                "({}{}{} {})",
                prefix,
                p.name,
                suffix,
                type_or_any(p.sir_type.as_ref())
            );
        }
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
    t.map(|t| t.to_string())
        .unwrap_or_else(|| "any".to_string())
}

/// Render a block.  `indent` is the column the *contents* should
/// start at (the opening `(block` keyword is emitted by the caller's
/// indentation).
pub fn print_block(out: &mut String, b: &Block, indent: usize) {
    print_block_depth(out, b, indent, 0);
}

fn print_block_depth(out: &mut String, b: &Block, indent: usize, depth: usize) {
    if depth >= MAX_IR_DEPTH {
        out.push_str(TRUNCATION_MARKER);
        return;
    }
    let pad = " ".repeat(indent);
    let _ = write!(out, "(block");
    if b.stmts.is_empty() {
        out.push(' ');
        print_expr_inline_depth(out, &b.value, depth + 1);
    } else {
        for s in &b.stmts {
            let _ = write!(out, "\n{}  ", pad);
            print_stmt(out, s, indent + 2, depth + 1);
        }
        let _ = write!(out, "\n{}  ", pad);
        print_expr_inline_depth(out, &b.value, depth + 1);
    }
    out.push(')');
}

fn print_stmt(out: &mut String, s: &Stmt, indent: usize, depth: usize) {
    if depth >= MAX_IR_DEPTH {
        out.push_str(TRUNCATION_MARKER);
        return;
    }
    match s {
        Stmt::LetBinding {
            name,
            sir_type,
            value,
            ..
        } => {
            let _ = write!(out, "(let {}", name);
            if let Some(t) = sir_type {
                let _ = write!(out, " {}", t);
            }
            out.push(' ');
            print_expr_inline_depth(out, value, depth + 1);
            out.push(')');
        }
        Stmt::LetStarBinding {
            name,
            sir_type,
            value,
            ..
        } => {
            let _ = write!(out, "(let* {}", name);
            if let Some(t) = sir_type {
                let _ = write!(out, " {}", t);
            }
            out.push(' ');
            print_expr_inline_depth(out, value, depth + 1);
            out.push(')');
        }
        Stmt::ExprStmt { expr, .. } => {
            let _ = write!(out, "(stmt ");
            print_expr_inline_depth(out, expr, depth + 1);
            out.push(')');
        }
        Stmt::Assign {
            name, scope, value, ..
        } => {
            let _ = write!(out, "(assign {} {} ", name, scope.name());
            print_expr_inline_depth(out, value, depth + 1);
            out.push(')');
        }
        Stmt::While { cond, body, .. } => {
            let _ = write!(out, "(while ");
            print_expr_inline_depth(out, cond, depth + 1);
            let _ = write!(out, "\n{}  ", " ".repeat(indent));
            print_block_depth(out, body, indent + 2, depth + 1);
            out.push(')');
        }
        Stmt::ForRange {
            var,
            start,
            stop,
            step,
            body,
            ..
        } => {
            let _ = write!(out, "(for-range {} ", var);
            print_expr_inline_depth(out, start, depth + 1);
            out.push(' ');
            print_expr_inline_depth(out, stop, depth + 1);
            out.push(' ');
            print_expr_inline_depth(out, step, depth + 1);
            let _ = write!(out, "\n{}  ", " ".repeat(indent));
            print_block_depth(out, body, indent + 2, depth + 1);
            out.push(')');
        }
        Stmt::ForEach {
            var, iter, body, ..
        } => {
            let _ = write!(out, "(for-each {} ", var);
            print_expr_inline_depth(out, iter, depth + 1);
            let _ = write!(out, "\n{}  ", " ".repeat(indent));
            print_block_depth(out, body, indent + 2, depth + 1);
            out.push(')');
        }
        Stmt::SeqSet {
            seq, index, value, ..
        } => {
            let _ = write!(out, "(seq-set ");
            print_expr_inline_depth(out, seq, depth + 1);
            out.push(' ');
            print_expr_inline_depth(out, index, depth + 1);
            out.push(' ');
            print_expr_inline_depth(out, value, depth + 1);
            out.push(')');
        }
        Stmt::MapSet {
            map, key, value, ..
        } => {
            let _ = write!(out, "(map-set ");
            print_expr_inline_depth(out, map, depth + 1);
            out.push(' ');
            print_expr_inline_depth(out, key, depth + 1);
            out.push(' ');
            print_expr_inline_depth(out, value, depth + 1);
            out.push(')');
        }
        Stmt::ClassDef {
            name,
            superclass,
            body,
            ..
        } => {
            // `(class-def Name)` for the empty base-class case;
            // `(class-def Name (< Super))` when the class inherits
            // (Ruby Phase 14c `class Foo < Bar`); body statements, if
            // any, are printed one per line after the optional
            // superclass clause.
            let _ = write!(out, "(class-def {}", name);
            if let Some(sup) = superclass {
                let _ = write!(out, " (< {})", sup);
            }
            for inner in body {
                let _ = write!(out, "\n{}  ", " ".repeat(indent));
                print_stmt(out, inner, indent + 2, depth + 1);
            }
            out.push(')');
        }
        Stmt::ModuleDef { name, body, .. } => {
            // `(module-def Name)` for the empty case; body statements
            // (if any) printed one per line (Ruby Phase 14d).  Mirrors
            // the ClassDef printer minus the superclass clause.
            let _ = write!(out, "(module-def {}", name);
            for inner in body {
                let _ = write!(out, "\n{}  ", " ".repeat(indent));
                print_stmt(out, inner, indent + 2, depth + 1);
            }
            out.push(')');
        }
        Stmt::SingletonClassDef { target, body, .. } => {
            // `(singleton-class-def << Target)` head; body statements
            // (if any) printed one per line (Ruby Phase 14e).
            let _ = write!(out, "(singleton-class-def << {}", target);
            for inner in body {
                let _ = write!(out, "\n{}  ", " ".repeat(indent));
                print_stmt(out, inner, indent + 2, depth + 1);
            }
            out.push(')');
        }
        Stmt::TryCatch {
            body,
            rescues,
            ensure_body,
            ..
        } => {
            // `(try-catch <body…> (rescue …) … (ensure …))` — exception
            // handling (Ruby Phase 16a).  Each clause prints on its own
            // indented line.
            out.push_str("(try-catch");
            for inner in body {
                let _ = write!(out, "\n{}  ", " ".repeat(indent));
                print_stmt(out, inner, indent + 2, depth + 1);
            }
            for r in rescues {
                let _ = write!(out, "\n{}  (rescue", " ".repeat(indent));
                if !r.exception_types.is_empty() {
                    let _ = write!(out, " (types {})", r.exception_types.join(" "));
                }
                if let Some(bind) = &r.binding {
                    let _ = write!(out, " (bind {})", bind);
                }
                for inner in &r.body {
                    let _ = write!(out, "\n{}    ", " ".repeat(indent));
                    print_stmt(out, inner, indent + 4, depth + 1);
                }
                out.push(')');
            }
            if let Some(ens) = ensure_body {
                let _ = write!(out, "\n{}  (ensure", " ".repeat(indent));
                for inner in ens {
                    let _ = write!(out, "\n{}    ", " ".repeat(indent));
                    print_stmt(out, inner, indent + 4, depth + 1);
                }
                out.push(')');
            }
            out.push(')');
        }
        // ── SIR22: array/matrix indexed assignment ──────────────────
        Stmt::IndexSet {
            target,
            indices,
            value,
            ..
        } => {
            let _ = write!(out, "(index-set ");
            print_expr_inline_depth(out, target, depth + 1);
            print_index_args(out, indices, depth);
            out.push(' ');
            print_expr_inline_depth(out, value, depth + 1);
            out.push(')');
        }
    }
}

/// Render an expression as a single-line s-expression.  Public for
/// diagnostics.
pub fn print_expr(e: &Expr) -> String {
    let mut out = String::new();
    print_expr_inline_depth(&mut out, e, 0);
    out
}

fn print_expr_inline_depth(out: &mut String, e: &Expr, depth: usize) {
    if depth >= MAX_IR_DEPTH {
        out.push_str(TRUNCATION_MARKER);
        return;
    }
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
        Expr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            let _ = write!(out, "(if ");
            print_expr_inline_depth(out, cond, depth + 1);
            out.push(' ');
            print_block_depth(out, then_branch, 0, depth + 1);
            out.push(' ');
            print_block_depth(out, else_branch, 0, depth + 1);
            out.push(')');
        }
        Expr::Block(b) => {
            print_block_depth(out, b, 0, depth + 1);
        }
        Expr::DirectCall {
            fn_name,
            args,
            effects,
            ..
        } => {
            let _ = write!(out, "(direct-call {} ", fn_name);
            print_effects(out, effects);
            print_args(out, args, depth);
            out.push(')');
        }
        Expr::IndirectCall {
            target,
            args,
            effects,
            ..
        } => {
            let _ = write!(out, "(indirect-call ");
            print_expr_inline_depth(out, target, depth + 1);
            out.push(' ');
            print_effects(out, effects);
            print_args(out, args, depth);
            out.push(')');
        }
        Expr::BuiltinCall {
            name,
            args,
            effects,
            ..
        } => {
            let _ = write!(out, "(builtin-call {} ", name);
            print_effects(out, effects);
            print_args(out, args, depth);
            out.push(')');
        }
        Expr::MakeClosure {
            fn_name, captures, ..
        } => {
            let _ = write!(out, "(make-closure {}", fn_name);
            for c in captures {
                let _ = write!(out, " ({} ", c.name);
                print_expr_inline_depth(out, &c.value, depth + 1);
                out.push(')');
            }
            out.push(')');
        }
        Expr::Intrinsic {
            targets,
            name,
            args,
            return_type,
            effects,
            ..
        } => {
            let _ = write!(
                out,
                "(intrinsic ({}) {} {} ",
                join_strs(targets),
                name,
                return_type
            );
            print_effects(out, effects);
            print_args(out, args, depth);
            out.push(')');
        }

        // ── SIR16 additions ────────────────────────────────────────
        Expr::FloatLit { value, .. } => {
            let _ = write!(out, "(float {})", format_float(*value));
        }
        Expr::SeqLit { items, .. } => {
            let _ = write!(out, "(seq");
            print_args(out, items, depth);
            out.push(')');
        }
        Expr::SeqIndex { seq, index, .. } => {
            let _ = write!(out, "(seq-index ");
            print_expr_inline_depth(out, seq, depth + 1);
            out.push(' ');
            print_expr_inline_depth(out, index, depth + 1);
            out.push(')');
        }
        Expr::SeqLen { seq, .. } => {
            let _ = write!(out, "(seq-len ");
            print_expr_inline_depth(out, seq, depth + 1);
            out.push(')');
        }
        Expr::MapLit { entries, .. } => {
            let _ = write!(out, "(map");
            for entry in entries {
                out.push_str(" (");
                print_expr_inline_depth(out, &entry.key, depth + 1);
                out.push(' ');
                print_expr_inline_depth(out, &entry.value, depth + 1);
                out.push(')');
            }
            out.push(')');
        }
        Expr::MapGet { map, key, .. } => {
            let _ = write!(out, "(map-get ");
            print_expr_inline_depth(out, map, depth + 1);
            out.push(' ');
            print_expr_inline_depth(out, key, depth + 1);
            out.push(')');
        }
        Expr::LogicalAnd { lhs, rhs, .. } => {
            let _ = write!(out, "(and ");
            print_expr_inline_depth(out, lhs, depth + 1);
            out.push(' ');
            print_expr_inline_depth(out, rhs, depth + 1);
            out.push(')');
        }
        Expr::LogicalOr { lhs, rhs, .. } => {
            let _ = write!(out, "(or ");
            print_expr_inline_depth(out, lhs, depth + 1);
            out.push(' ');
            print_expr_inline_depth(out, rhs, depth + 1);
            out.push(')');
        }
        Expr::StrConcat { parts, .. } => {
            let _ = write!(out, "(str-concat");
            print_args(out, parts, depth);
            out.push(')');
        }
        // ── KW1: keyword argument ──────────────────────────────────
        // `(keyword-arg name <value>)` — the head keyword names the
        // concept, then the keyword's name (bare, like a `var-ref`'s name),
        // then the value expression.  It appears inline in a call's arg
        // list, so `f(1, a: 2)` prints as
        // `(direct-call f (effects …) (int 1) (keyword-arg a (int 2)))`.
        Expr::KeywordArg { name, value, .. } => {
            let _ = write!(out, "(keyword-arg {} ", name);
            print_expr_inline_depth(out, value, depth + 1);
            out.push(')');
        }

        // ── SIR22: array/matrix nodes ────────────────────────────────
        // `(array (row (int 1) (int 2)) (row (int 3) (int 4)))` — one
        // `(row ...)` group per row, preserving the row-major literal
        // order the SIR22 spec documents (the frontend reconciles this
        // with the column-major storage convention, not the printer).
        Expr::ArrayLit { rows, .. } => {
            let _ = write!(out, "(array");
            for row in rows {
                let _ = write!(out, " (row");
                for item in row {
                    out.push(' ');
                    print_expr_inline_depth(out, item, depth + 1);
                }
                out.push(')');
            }
            out.push(')');
        }
        // `(range <start> <stop>)` when step is absent (step = 1);
        // `(range <start> <step> <stop>)` when explicit — matching
        // MATLAB's own two-arg-vs-three-arg colon distinction, so a
        // round-tripped `1:5` doesn't grow a spurious `(int 1)` step.
        Expr::Range {
            start, step, stop, ..
        } => {
            let _ = write!(out, "(range ");
            print_expr_inline_depth(out, start, depth + 1);
            out.push(' ');
            if let Some(step) = step {
                print_expr_inline_depth(out, step, depth + 1);
                out.push(' ');
            }
            print_expr_inline_depth(out, stop, depth + 1);
            out.push(')');
        }
        Expr::MatMul { lhs, rhs, .. } => {
            let _ = write!(out, "(matmul ");
            print_expr_inline_depth(out, lhs, depth + 1);
            out.push(' ');
            print_expr_inline_depth(out, rhs, depth + 1);
            out.push(')');
        }
        Expr::ElementwiseOp { op, lhs, rhs, .. } => {
            let _ = write!(out, "(elementwise-op {} ", op.name());
            print_expr_inline_depth(out, lhs, depth + 1);
            out.push(' ');
            print_expr_inline_depth(out, rhs, depth + 1);
            out.push(')');
        }
        // `(transpose <target> conjugate)` / `(transpose <target> plain)`
        // — MATLAB `'` vs `.'` (see Expr::Transpose's doc comment).
        Expr::Transpose {
            target, conjugate, ..
        } => {
            let _ = write!(out, "(transpose ");
            print_expr_inline_depth(out, target, depth + 1);
            let _ = write!(out, " {})", if *conjugate { "conjugate" } else { "plain" });
        }
        Expr::IndexGet {
            target, indices, ..
        } => {
            let _ = write!(out, "(index-get ");
            print_expr_inline_depth(out, target, depth + 1);
            print_index_args(out, indices, depth);
            out.push(')');
        }
        // ── SIR26: integer conversion ──────────────────────────────────
        Expr::Convert { value, to, .. } => {
            // `(convert <to> <value>)`, where <to> is the IntSpec text form
            // (SIR21), e.g. `(convert (int u8 wrap) (var-ref t local))`.
            let _ = write!(out, "(convert {} ", to);
            print_expr_inline_depth(out, value, depth + 1);
            out.push(')');
        }

        // ── SIR23: symbolic expression + pattern/rewrite nodes ─────────
        // Every head keyword below is prefixed `sym-` so it can never
        // collide with an existing form — notably `(sym name)` above is
        // `Expr::SymLit` (a Ruby-style interned `:symbol` literal), a
        // wholly different concept from `SymSymbol`'s symbolic-expression-
        // tree leaf.
        Expr::SymSymbol { name, .. } => {
            let _ = write!(out, "(sym-symbol {})", name);
        }
        // `(sym-rational <numer> <denom>)` — printed as the two integer
        // fields rather than a single `n/d` token so the shape matches
        // every other multi-field node in this file (e.g. `Transpose`,
        // `IndexArg::Scalar`); the IR does not reduce the fraction (see
        // `SymRational`'s doc comment), and neither does the printer.
        Expr::SymRational { numer, denom, .. } => {
            let _ = write!(out, "(sym-rational {} {})", numer, denom);
        }
        Expr::SymApply { head, args, .. } => {
            let _ = write!(out, "(sym-apply ");
            print_expr_inline_depth(out, head, depth + 1);
            print_args(out, args, depth);
            out.push(')');
        }
        // `(sym-pattern-blank)` for Wolfram `_`; `(sym-pattern-blank
        // <head>)` for `_h`.
        Expr::SymPatternBlank { head, .. } => {
            let _ = write!(out, "(sym-pattern-blank");
            if let Some(h) = head {
                out.push(' ');
                print_expr_inline_depth(out, h, depth + 1);
            }
            out.push(')');
        }
        Expr::SymPatternNamed { name, pattern, .. } => {
            let _ = write!(out, "(sym-pattern-named {} ", name);
            print_expr_inline_depth(out, pattern, depth + 1);
            out.push(')');
        }
        // `(sym-rule <lhs> <rhs> eager)` for `->`; `... delayed)` for
        // `:>` — mirroring how `Transpose` renders its boolean flag as a
        // trailing keyword (`conjugate`/`plain`) rather than `true`/`false`.
        Expr::SymRule {
            lhs, rhs, delayed, ..
        } => {
            let _ = write!(out, "(sym-rule ");
            print_expr_inline_depth(out, lhs, depth + 1);
            out.push(' ');
            print_expr_inline_depth(out, rhs, depth + 1);
            let _ = write!(out, " {})", if *delayed { "delayed" } else { "eager" });
        }
        // `(sym-replace-all <expr> (rules <rule>...) once)` for `/.`;
        // `... repeated)` for `//.`.
        Expr::SymReplaceAll {
            expr,
            rules,
            repeated,
            ..
        } => {
            let _ = write!(out, "(sym-replace-all ");
            print_expr_inline_depth(out, expr, depth + 1);
            let _ = write!(out, " (rules");
            print_args(out, rules, depth);
            out.push(')');
            let _ = write!(out, " {})", if *repeated { "repeated" } else { "once" });
        }
    }
}

/// Render a slice of `IndexArg`s, one space-prefixed `(…)` form per
/// entry, shared by `Expr::IndexGet` and `Stmt::IndexSet` (SIR22).
///
/// ```text
/// Scalar(e)  →  " (idx-scalar <e>)"
/// Whole      →  " (idx-whole)"
/// Range(e)   →  " (idx-range <e>)"
/// ```
fn print_index_args(out: &mut String, indices: &[IndexArg], depth: usize) {
    for arg in indices {
        match arg {
            IndexArg::Scalar(e) => {
                let _ = write!(out, " (idx-scalar ");
                print_expr_inline_depth(out, e, depth + 1);
                out.push(')');
            }
            IndexArg::Whole => {
                out.push_str(" (idx-whole)");
            }
            IndexArg::Range(e) => {
                let _ = write!(out, " (idx-range ");
                print_expr_inline_depth(out, e, depth + 1);
                out.push(')');
            }
        }
    }
}

/// Render a float deterministically.  We emit a `.` to disambiguate
/// from integer literals — `3` would parse as `(int)`, so we print
/// `3.0` instead.  Non-finite values use a stable textual form.
fn format_float(v: f64) -> String {
    if v.is_nan() {
        "nan".into()
    } else if v.is_infinite() {
        if v.is_sign_negative() {
            "-inf".into()
        } else {
            "inf".into()
        }
    } else if v == v.trunc() && v.abs() < 1e16 {
        format!("{:.1}", v)
    } else {
        format!("{}", v)
    }
}

fn print_effects(out: &mut String, e: &EffectSet) {
    let _ = write!(out, "(effects {})", e);
}

fn print_args(out: &mut String, args: &[Expr], depth: usize) {
    for a in args {
        out.push(' ');
        print_expr_inline_depth(out, a, depth + 1);
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
        assert!(t.starts_with("(sir-module demo v3"));
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
        let e = Expr::IntLit {
            value: 42,
            span: s(),
        };
        assert_eq!(print_expr(&e), "(int 42)");
    }

    #[test]
    fn print_negative_integer() {
        let e = Expr::IntLit {
            value: -7,
            span: s(),
        };
        assert_eq!(print_expr(&e), "(int -7)");
    }

    #[test]
    fn print_str_concat() {
        // Phase 20b — `StrConcat` renders as `(str-concat <parts…>)`,
        // each part printed inline like a builtin-call's args.
        let e = Expr::StrConcat {
            parts: vec![
                Expr::StrLit {
                    value: "hi ".into(),
                    span: s(),
                },
                Expr::VarRef {
                    name: "name".into(),
                    scope: Scope::Local,
                    span: s(),
                },
            ],
            span: s(),
        };
        assert_eq!(
            print_expr(&e),
            "(str-concat (str \"hi \") (var-ref name local))"
        );
    }

    #[test]
    fn print_booleans_and_nil() {
        assert_eq!(
            print_expr(&Expr::BoolLit {
                value: true,
                span: s()
            }),
            "(bool true)"
        );
        assert_eq!(
            print_expr(&Expr::BoolLit {
                value: false,
                span: s()
            }),
            "(bool false)"
        );
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
    fn print_var_ref_instance_scope() {
        // Phase 15a — an instance-var ref prints with the `instance`
        // scope tag and the `@`-sigil name preserved verbatim.
        let e = Expr::VarRef {
            name: "@x".into(),
            scope: Scope::Instance,
            span: s(),
        };
        assert_eq!(print_expr(&e), "(var-ref @x instance)");
    }

    #[test]
    fn print_var_ref_class_var_scope() {
        // Phase 15b — a class-var ref prints with the `class-var`
        // scope tag and the `@@`-sigil name preserved verbatim.
        let e = Expr::VarRef {
            name: "@@count".into(),
            scope: Scope::ClassVar,
            span: s(),
        };
        assert_eq!(print_expr(&e), "(var-ref @@count class-var)");
    }

    #[test]
    fn print_var_ref_const_scope() {
        // Phase 15c — a constant ref prints with the `const` scope tag
        // and the (uppercase-initial) name preserved verbatim.
        let e = Expr::VarRef {
            name: "MAX".into(),
            scope: Scope::Const,
            span: s(),
        };
        assert_eq!(print_expr(&e), "(var-ref MAX const)");
    }

    #[test]
    fn print_builtin_call_with_pure() {
        let e = Expr::BuiltinCall {
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
            args: vec![Expr::IntLit {
                value: 99,
                span: s(),
            }],
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
                value: Expr::IntLit {
                    value: 5,
                    span: s(),
                },
            }],
            span: s(),
        };
        assert_eq!(print_expr(&e), "(make-closure __lambda_0 (n (int 5)))");
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
                Param {
                    name: "x".into(),
                    sir_type: None,
                    kind: ParamKind::Required,
                    default: None,
                    span: s(),
                },
                Param {
                    name: "y".into(),
                    sir_type: None,
                    kind: ParamKind::Required,
                    default: None,
                    span: s(),
                },
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
        assert!(
            text.contains("(builtin-call + (effects pure) (var-ref x param) (var-ref y param))")
        );
    }

    #[test]
    fn print_variadic_params_render_splat_prefix() {
        // M3: a Rest param renders `*name`, a KwRest param renders `**name`,
        // so round-tripping preserves splat-ness. `def f(a, *rest, **opts)`.
        let body = Block {
            stmts: vec![],
            value: Expr::NilLit { span: s() },
            span: s(),
        };
        let f = Function {
            name: "f".into(),
            params: vec![
                Param {
                    name: "a".into(),
                    sir_type: None,
                    kind: ParamKind::Required,
                    default: None,
                    span: s(),
                },
                Param {
                    name: "rest".into(),
                    sir_type: None,
                    kind: ParamKind::Rest,
                    default: None,
                    span: s(),
                },
                Param {
                    name: "opts".into(),
                    sir_type: None,
                    kind: ParamKind::KwRest,
                    default: None,
                    span: s(),
                },
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
        assert!(
            text.contains("(function f ((a any) (*rest any) (**opts any)) any"),
            "got: {text}"
        );
    }

    #[test]
    fn print_param_default_renders_default_clause() {
        // SIR19: `def f(a = 1)` — the param carries a default literal.
        // The printer renders an extra `(default (int 1))` clause inside
        // the param form, while a defaultless param keeps `(name type)`.
        let body = Block {
            stmts: vec![],
            value: Expr::NilLit { span: s() },
            span: s(),
        };
        let f = Function {
            name: "f".into(),
            params: vec![
                Param {
                    name: "a".into(),
                    sir_type: None,
                    kind: ParamKind::Required,
                    default: Some(Box::new(Expr::IntLit {
                        value: 1,
                        span: s(),
                    })),
                    span: s(),
                },
                Param {
                    name: "b".into(),
                    sir_type: None,
                    kind: ParamKind::Required,
                    default: None,
                    span: s(),
                },
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
            FeatureManifest::from_features(&[Feature::DynamicTyping, Feature::DefaultParams]),
        );
        let text = print_module(&m);
        assert!(
            text.contains("(function f ((a any (default (int 1))) (b any)) any"),
            "got: {text}"
        );
    }

    #[test]
    fn print_keyword_params_render_colon_suffix() {
        // KW1: `def f(x:, y: 1)` → a REQUIRED keyword `x` renders `(x: any)`
        // and an OPTIONAL keyword `y` renders `(y: any (default (int 1)))`.
        let body = Block {
            stmts: vec![],
            value: Expr::NilLit { span: s() },
            span: s(),
        };
        let f = Function {
            name: "f".into(),
            params: vec![
                Param {
                    name: "x".into(),
                    sir_type: None,
                    kind: ParamKind::Keyword,
                    default: None,
                    span: s(),
                },
                Param {
                    name: "y".into(),
                    sir_type: None,
                    kind: ParamKind::Keyword,
                    default: Some(Box::new(Expr::IntLit {
                        value: 1,
                        span: s(),
                    })),
                    span: s(),
                },
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
            FeatureManifest::from_features(&[Feature::DynamicTyping, Feature::KeywordParams]),
        );
        let text = print_module(&m);
        assert!(
            text.contains("(function f ((x: any) (y: any (default (int 1)))) any"),
            "got: {text}"
        );
    }

    #[test]
    fn print_keyword_arg_renders_head_name_value() {
        // KW1: `f(1, a: 2)` → a keyword argument prints as
        // `(keyword-arg a (int 2))`, inline in the call's arg list after
        // the positional `(int 1)`.
        let e = Expr::DirectCall {
            fn_name: "f".into(),
            args: vec![
                Expr::IntLit {
                    value: 1,
                    span: s(),
                },
                Expr::KeywordArg {
                    name: "a".into(),
                    value: Box::new(Expr::IntLit {
                        value: 2,
                        span: s(),
                    }),
                    span: s(),
                },
            ],
            effects: EffectSet::PURE,
            span: s(),
        };
        assert_eq!(
            print_expr(&e),
            "(direct-call f (effects pure) (int 1) (keyword-arg a (int 2)))"
        );
    }

    // `3.14` is an arbitrary float literal exercised by the printer, not PI.
    #[allow(clippy::approx_constant)]
    #[test]
    fn print_float_lit_emits_decimal_form() {
        // Integer-valued floats render with explicit `.0` so the
        // round-trip parser can distinguish them from int literals.
        let e = Expr::FloatLit {
            value: 3.0,
            span: s(),
        };
        assert_eq!(print_expr(&e), "(float 3.0)");
        let e = Expr::FloatLit {
            value: 3.14,
            span: s(),
        };
        assert_eq!(print_expr(&e), "(float 3.14)");
    }

    #[test]
    fn print_float_lit_handles_non_finite() {
        let nan = Expr::FloatLit {
            value: f64::NAN,
            span: s(),
        };
        assert_eq!(print_expr(&nan), "(float nan)");
        let posinf = Expr::FloatLit {
            value: f64::INFINITY,
            span: s(),
        };
        assert_eq!(print_expr(&posinf), "(float inf)");
        let neginf = Expr::FloatLit {
            value: f64::NEG_INFINITY,
            span: s(),
        };
        assert_eq!(print_expr(&neginf), "(float -inf)");
    }

    #[test]
    fn print_seq_and_map_literals() {
        let seq = Expr::SeqLit {
            items: vec![
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
        };
        assert_eq!(print_expr(&seq), "(seq (int 1) (int 2))");
        let map = Expr::MapLit {
            entries: vec![MapEntry {
                key: Expr::StrLit {
                    value: "a".into(),
                    span: s(),
                },
                value: Expr::IntLit {
                    value: 1,
                    span: s(),
                },
            }],
            span: s(),
        };
        assert_eq!(print_expr(&map), r#"(map ((str "a") (int 1)))"#);
    }

    #[test]
    fn print_logical_short_circuit() {
        let e = Expr::LogicalAnd {
            lhs: Box::new(Expr::BoolLit {
                value: true,
                span: s(),
            }),
            rhs: Box::new(Expr::BoolLit {
                value: false,
                span: s(),
            }),
            span: s(),
        };
        assert_eq!(print_expr(&e), "(and (bool true) (bool false))");
    }

    #[test]
    fn print_let_binding() {
        let s_ = s();
        let block = Block {
            stmts: vec![Stmt::LetBinding {
                name: "x".into(),
                sir_type: None,
                value: Expr::IntLit {
                    value: 1,
                    span: s_.clone(),
                },
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

    // ── SIR17: class declarations ──────────────────────────────────

    #[test]
    fn print_empty_class_def() {
        // `class Foo; end` → `(class-def Foo)` (no body lines).
        let s_ = s();
        let block = Block {
            stmts: vec![Stmt::ClassDef {
                name: "Foo".into(),
                superclass: None,
                body: vec![],
                span: s_.clone(),
            }],
            value: Expr::NilLit { span: s_.clone() },
            span: s_,
        };
        let mut out = String::new();
        print_block(&mut out, &block, 0);
        assert!(
            out.contains("(class-def Foo)"),
            "expected `(class-def Foo)` in output, got:\n{}",
            out
        );
    }

    #[test]
    fn print_class_def_with_body_stmt() {
        // Forward-compat: a populated body prints each statement
        // indented under the class-def head.  Phase 14a never emits
        // this shape, but the printer supports it.
        let s_ = s();
        let block = Block {
            stmts: vec![Stmt::ClassDef {
                name: "Bar".into(),
                superclass: None,
                body: vec![Stmt::LetBinding {
                    name: "y".into(),
                    sir_type: None,
                    value: Expr::IntLit {
                        value: 2,
                        span: s_.clone(),
                    },
                    span: s_.clone(),
                }],
                span: s_.clone(),
            }],
            value: Expr::NilLit { span: s_.clone() },
            span: s_,
        };
        let mut out = String::new();
        print_block(&mut out, &block, 0);
        assert!(out.contains("(class-def Bar"));
        assert!(out.contains("(let y (int 2))"));
    }

    #[test]
    fn print_empty_module_def() {
        // `module M; end` → `(module-def M)` (no body lines).
        let s_ = s();
        let block = Block {
            stmts: vec![Stmt::ModuleDef {
                name: "M".into(),
                body: vec![],
                span: s_.clone(),
            }],
            value: Expr::NilLit { span: s_.clone() },
            span: s_,
        };
        let mut out = String::new();
        print_block(&mut out, &block, 0);
        assert!(
            out.contains("(module-def M)"),
            "expected `(module-def M)` in output, got:\n{}",
            out
        );
    }

    #[test]
    fn print_module_def_with_body_stmt() {
        // A populated module body prints each statement indented under
        // the module-def head.
        let s_ = s();
        let block = Block {
            stmts: vec![Stmt::ModuleDef {
                name: "Config".into(),
                body: vec![Stmt::LetBinding {
                    name: "v".into(),
                    sir_type: None,
                    value: Expr::IntLit {
                        value: 3,
                        span: s_.clone(),
                    },
                    span: s_.clone(),
                }],
                span: s_.clone(),
            }],
            value: Expr::NilLit { span: s_.clone() },
            span: s_,
        };
        let mut out = String::new();
        print_block(&mut out, &block, 0);
        assert!(out.contains("(module-def Config"));
        assert!(out.contains("(let v (int 3))"));
    }

    #[test]
    fn print_singleton_class_def() {
        // `class << self; end` → `(singleton-class-def << self)`.
        let s_ = s();
        let block = Block {
            stmts: vec![Stmt::SingletonClassDef {
                target: "self".into(),
                body: vec![],
                span: s_.clone(),
            }],
            value: Expr::NilLit { span: s_.clone() },
            span: s_,
        };
        let mut out = String::new();
        print_block(&mut out, &block, 0);
        assert!(
            out.contains("(singleton-class-def << self)"),
            "expected `(singleton-class-def << self)` in output, got:\n{}",
            out
        );
    }

    #[test]
    fn print_class_def_with_superclass() {
        // Ruby Phase 14c: `class Foo < Bar` prints the superclass
        // clause `(< Bar)` right after the class name.
        let s_ = s();
        let block = Block {
            stmts: vec![Stmt::ClassDef {
                name: "Foo".into(),
                superclass: Some("Bar".into()),
                body: vec![],
                span: s_.clone(),
            }],
            value: Expr::NilLit { span: s_.clone() },
            span: s_,
        };
        let mut out = String::new();
        print_block(&mut out, &block, 0);
        assert!(
            out.contains("(class-def Foo (< Bar))"),
            "expected `(class-def Foo (< Bar))` in output, got:\n{}",
            out
        );
    }

    #[test]
    fn print_try_catch_with_rescue_and_ensure() {
        // Ruby Phase 16a: `begin … rescue Foo => e … ensure … end` prints
        // a `(try-catch …)` head with `(rescue (types …) (bind …) …)` and
        // `(ensure …)` clauses.
        let s_ = s();
        let lb = |name: &str| Stmt::LetBinding {
            name: name.into(),
            sir_type: None,
            value: Expr::IntLit {
                value: 1,
                span: s_.clone(),
            },
            span: s_.clone(),
        };
        let block = Block {
            stmts: vec![Stmt::TryCatch {
                body: vec![lb("x")],
                rescues: vec![crate::nodes::RescueClause {
                    exception_types: vec!["StandardError".into()],
                    binding: Some("e".into()),
                    body: vec![lb("y")],
                    span: s_.clone(),
                }],
                ensure_body: Some(vec![lb("z")]),
                span: s_.clone(),
            }],
            value: Expr::NilLit { span: s_.clone() },
            span: s_,
        };
        let mut out = String::new();
        print_block(&mut out, &block, 0);
        assert!(
            out.contains("(try-catch"),
            "expected try-catch head, got:\n{}",
            out
        );
        assert!(
            out.contains("(rescue (types StandardError) (bind e)"),
            "expected rescue clause, got:\n{}",
            out
        );
        assert!(
            out.contains("(ensure"),
            "expected ensure clause, got:\n{}",
            out
        );
    }

    // ── SIR22: array/matrix printer tests ────────────────────────────

    #[test]
    fn print_array_lit_renders_rows() {
        // [1 2; 3 4]
        let e = Expr::ArrayLit {
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
        };
        assert_eq!(
            print_expr(&e),
            "(array (row (int 1) (int 2)) (row (int 3) (int 4)))"
        );
    }

    #[test]
    fn print_array_lit_empty() {
        let e = Expr::ArrayLit {
            rows: vec![],
            span: s(),
        };
        assert_eq!(print_expr(&e), "(array)");
    }

    #[test]
    fn print_range_without_step() {
        // 1:5 — step absent means step = 1; the printer must not
        // synthesise one.
        let e = Expr::Range {
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
        };
        assert_eq!(print_expr(&e), "(range (int 1) (int 5))");
    }

    #[test]
    fn print_range_with_step() {
        // 0:2:10
        let e = Expr::Range {
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
        };
        assert_eq!(print_expr(&e), "(range (int 0) (int 2) (int 10))");
    }

    #[test]
    fn print_matmul() {
        let e = Expr::MatMul {
            lhs: Box::new(Expr::VarRef {
                name: "a".into(),
                scope: Scope::Local,
                span: s(),
            }),
            rhs: Box::new(Expr::VarRef {
                name: "b".into(),
                scope: Scope::Local,
                span: s(),
            }),
            span: s(),
        };
        assert_eq!(
            print_expr(&e),
            "(matmul (var-ref a local) (var-ref b local))"
        );
    }

    #[test]
    fn print_elementwise_op_all_kinds() {
        // Every ElementwiseOpKind renders its kebab-case op name.
        for (kind, name) in [
            (ElementwiseOpKind::Add, "add"),
            (ElementwiseOpKind::Sub, "sub"),
            (ElementwiseOpKind::Mul, "mul"),
            (ElementwiseOpKind::Div, "div"),
            (ElementwiseOpKind::Pow, "pow"),
        ] {
            let e = Expr::ElementwiseOp {
                op: kind,
                lhs: Box::new(Expr::IntLit {
                    value: 1,
                    span: s(),
                }),
                rhs: Box::new(Expr::IntLit {
                    value: 2,
                    span: s(),
                }),
                span: s(),
            };
            assert_eq!(
                print_expr(&e),
                format!("(elementwise-op {} (int 1) (int 2))", name)
            );
        }
    }

    #[test]
    fn print_transpose_conjugate_vs_plain() {
        // MATLAB `'` (conjugate) vs `.'` (plain) — see Expr::Transpose.
        let tick = Expr::Transpose {
            target: Box::new(Expr::VarRef {
                name: "a".into(),
                scope: Scope::Local,
                span: s(),
            }),
            conjugate: true,
            span: s(),
        };
        assert_eq!(print_expr(&tick), "(transpose (var-ref a local) conjugate)");
        let dot_tick = Expr::Transpose {
            target: Box::new(Expr::VarRef {
                name: "a".into(),
                scope: Scope::Local,
                span: s(),
            }),
            conjugate: false,
            span: s(),
        };
        assert_eq!(print_expr(&dot_tick), "(transpose (var-ref a local) plain)");
    }

    #[test]
    fn print_index_get_scalar_whole_and_range_args() {
        // a(i, :, 1:3)
        let e = Expr::IndexGet {
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
        };
        assert_eq!(
            print_expr(&e),
            "(index-get (var-ref a local) (idx-scalar (int 0)) (idx-whole) (idx-range (range (int 0) (int 3))))"
        );
    }

    #[test]
    fn print_index_get_no_indices() {
        let e = Expr::IndexGet {
            target: Box::new(Expr::VarRef {
                name: "a".into(),
                scope: Scope::Local,
                span: s(),
            }),
            indices: vec![],
            span: s(),
        };
        assert_eq!(print_expr(&e), "(index-get (var-ref a local))");
    }

    #[test]
    fn print_index_set_stmt() {
        // a(1) = 9 — printed via print_block since IndexSet is a Stmt.
        let block = Block {
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
        };
        let mut out = String::new();
        print_block(&mut out, &block, 0);
        assert!(
            out.contains("(index-set (var-ref a local) (idx-scalar (int 0)) (int 9))"),
            "got:\n{}",
            out
        );
    }

    // ── SIR23: symbolic expression + pattern/rewrite printer tests ──

    #[test]
    fn print_sym_symbol() {
        let e = Expr::SymSymbol {
            name: "Plus".into(),
            span: s(),
        };
        assert_eq!(print_expr(&e), "(sym-symbol Plus)");
    }

    #[test]
    fn print_sym_rational_unreduced() {
        // The printer does not reduce the fraction — 2/4 prints verbatim.
        let e = Expr::SymRational {
            numer: 2,
            denom: 4,
            span: s(),
        };
        assert_eq!(print_expr(&e), "(sym-rational 2 4)");
    }

    #[test]
    fn print_sym_apply_with_computed_head() {
        // f[x][y] — head is itself a SymApply, not a bare name.
        let e = Expr::SymApply {
            head: Box::new(Expr::SymApply {
                head: Box::new(Expr::SymSymbol {
                    name: "f".into(),
                    span: s(),
                }),
                args: vec![Expr::SymSymbol {
                    name: "x".into(),
                    span: s(),
                }],
                span: s(),
            }),
            args: vec![Expr::SymSymbol {
                name: "y".into(),
                span: s(),
            }],
            span: s(),
        };
        assert_eq!(
            print_expr(&e),
            "(sym-apply (sym-apply (sym-symbol f) (sym-symbol x)) (sym-symbol y))"
        );
    }

    #[test]
    fn print_sym_apply_no_args() {
        let e = Expr::SymApply {
            head: Box::new(Expr::SymSymbol {
                name: "f".into(),
                span: s(),
            }),
            args: vec![],
            span: s(),
        };
        assert_eq!(print_expr(&e), "(sym-apply (sym-symbol f))");
    }

    #[test]
    fn print_sym_pattern_blank_bare_and_head_constrained() {
        // Wolfram `_` vs `_h`.
        let bare = Expr::SymPatternBlank {
            head: None,
            span: s(),
        };
        assert_eq!(print_expr(&bare), "(sym-pattern-blank)");
        let constrained = Expr::SymPatternBlank {
            head: Some(Box::new(Expr::SymSymbol {
                name: "Integer".into(),
                span: s(),
            })),
            span: s(),
        };
        assert_eq!(
            print_expr(&constrained),
            "(sym-pattern-blank (sym-symbol Integer))"
        );
    }

    #[test]
    fn print_sym_pattern_named() {
        // Wolfram `x_`.
        let e = Expr::SymPatternNamed {
            name: "x".into(),
            pattern: Box::new(Expr::SymPatternBlank {
                head: None,
                span: s(),
            }),
            span: s(),
        };
        assert_eq!(
            print_expr(&e),
            "(sym-pattern-named x (sym-pattern-blank))"
        );
    }

    #[test]
    fn print_sym_rule_eager_vs_delayed() {
        // `->` (Rule) vs `:>` (RuleDelayed).
        let eager = Expr::SymRule {
            lhs: Box::new(Expr::SymSymbol {
                name: "x".into(),
                span: s(),
            }),
            rhs: Box::new(Expr::IntLit {
                value: 1,
                span: s(),
            }),
            delayed: false,
            span: s(),
        };
        assert_eq!(
            print_expr(&eager),
            "(sym-rule (sym-symbol x) (int 1) eager)"
        );
        let delayed = Expr::SymRule {
            lhs: Box::new(Expr::SymSymbol {
                name: "x".into(),
                span: s(),
            }),
            rhs: Box::new(Expr::IntLit {
                value: 1,
                span: s(),
            }),
            delayed: true,
            span: s(),
        };
        assert_eq!(
            print_expr(&delayed),
            "(sym-rule (sym-symbol x) (int 1) delayed)"
        );
    }

    #[test]
    fn print_sym_replace_all_once_vs_repeated() {
        // `/.` (ReplaceAll) vs `//.` (ReplaceRepeated).
        let rule = Expr::SymRule {
            lhs: Box::new(Expr::SymSymbol {
                name: "x".into(),
                span: s(),
            }),
            rhs: Box::new(Expr::IntLit {
                value: 0,
                span: s(),
            }),
            delayed: false,
            span: s(),
        };
        let once = Expr::SymReplaceAll {
            expr: Box::new(Expr::SymSymbol {
                name: "x".into(),
                span: s(),
            }),
            rules: vec![rule.clone()],
            repeated: false,
            span: s(),
        };
        assert_eq!(
            print_expr(&once),
            "(sym-replace-all (sym-symbol x) (rules (sym-rule (sym-symbol x) (int 0) eager)) once)"
        );
        let repeated = Expr::SymReplaceAll {
            expr: Box::new(Expr::SymSymbol {
                name: "x".into(),
                span: s(),
            }),
            rules: vec![rule],
            repeated: true,
            span: s(),
        };
        assert_eq!(
            print_expr(&repeated),
            "(sym-replace-all (sym-symbol x) (rules (sym-rule (sym-symbol x) (int 0) eager)) repeated)"
        );
    }

    #[test]
    fn print_sym_replace_all_no_rules() {
        let e = Expr::SymReplaceAll {
            expr: Box::new(Expr::SymSymbol {
                name: "x".into(),
                span: s(),
            }),
            rules: vec![],
            repeated: false,
            span: s(),
        };
        assert_eq!(print_expr(&e), "(sym-replace-all (sym-symbol x) (rules) once)");
    }
}
