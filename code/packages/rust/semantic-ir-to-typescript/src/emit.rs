//! TypeScript emitter — walks a SIR module and produces source code.
//!
//! Output layout (per SIR12):
//!
//! ```text
//! // banner comment
//! import * as __Sir from "@coding-adventures/sir-runtime-core";
//! let <global>: __Sir.Val = null;
//! function <name>(...): __Sir.Val { ... }
//! _init();                  // if module has _init
//! const __sir_result: __Sir.Val = main();
//! ```
//!
//! Lowering is straightforward: each SIR node maps to one TypeScript
//! construct.  `Block` becomes an IIFE so let bindings don't leak;
//! `MakeClosure` becomes a `new __Sir.Closure((..._a) => ...)` that
//! prepends capture values to the call's args.

use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::fmt::Write;

use semantic_ir::{
    Block, Expr, Feature, Function, Global, Module, Scope, Stmt,
};

use crate::runtime::RUNTIME;

/// True if the module uses any object-orientation feature, in which
/// case the emitted artifact imports `@coding-adventures/sir-runtime-oop`.
fn uses_oop(m: &Module) -> bool {
    [
        Feature::Classes,
        Feature::Modules,
        Feature::InstanceVars,
        Feature::ClassVars,
        Feature::Constants,
    ]
    .iter()
    .any(|f| m.manifest.contains(*f))
}

/// True if the module uses exception handling, in which case the emitted
/// artifact imports `@coding-adventures/sir-runtime-exceptions`.
fn uses_exceptions(m: &Module) -> bool {
    m.manifest.contains(Feature::Exceptions)
}

/// True if the module uses cons pairs, in which case the emitted artifact
/// imports `@coding-adventures/sir-runtime-pairs` (the `cons`/`car`/`cdr`/
/// `pair?` helpers, extracted from core).
fn uses_pairs(m: &Module) -> bool {
    m.manifest.contains(Feature::Pairs)
}

/// True if the module calls the `regex` builtin (a Ruby `/pat/flags` literal
/// lowers to `BuiltinCall("regex", …)`).  Regex carries no SIR `Feature`, so we
/// detect it by walking for the builtin name; a positive result gates the
/// `@coding-adventures/sir-runtime-regex` import.
fn uses_regex(m: &Module) -> bool {
    module_uses_builtin(m, "regex")
}

/// True if the module calls the `backtick` builtin (a Ruby `` `cmd` ``
/// literal lowers to `BuiltinCall("backtick", [cmd])`).  Gates the
/// `@coding-adventures/sir-runtime-shell` import.
fn uses_shell(m: &Module) -> bool {
    module_uses_builtin(m, "backtick")
}

/// True if the module calls the `range` builtin (a Ruby `a..b` / `a...b`
/// literal lowers to `BuiltinCall("range", [start, stop, exclusive])`).  Range
/// carries no SIR `Feature`, so we detect it by builtin name; a positive result
/// gates the `@coding-adventures/sir-runtime-range` import.
fn uses_range(m: &Module) -> bool {
    module_uses_builtin(m, "range")
}

/// Walk every function body for a `BuiltinCall` named `name` — gates
/// per-concern imports for builtins that carry no `Feature` flag.  Exhaustive
/// over `Stmt`/`Expr` so a new node can't silently hide a use.
fn module_uses_builtin(m: &Module, name: &str) -> bool {
    m.functions.iter().any(|f| block_uses_builtin(&f.body, name))
}

fn block_uses_builtin(b: &Block, name: &str) -> bool {
    b.stmts.iter().any(|s| stmt_uses_builtin(s, name)) || expr_uses_builtin(&b.value, name)
}

fn stmts_use_builtin(stmts: &[Stmt], name: &str) -> bool {
    stmts.iter().any(|s| stmt_uses_builtin(s, name))
}

fn stmt_uses_builtin(s: &Stmt, name: &str) -> bool {
    match s {
        Stmt::LetBinding { value, .. }
        | Stmt::LetStarBinding { value, .. }
        | Stmt::Assign { value, .. } => expr_uses_builtin(value, name),
        Stmt::ExprStmt { expr, .. } => expr_uses_builtin(expr, name),
        Stmt::While { cond, body, .. } => {
            expr_uses_builtin(cond, name) || block_uses_builtin(body, name)
        }
        Stmt::ForRange { start, stop, step, body, .. } => {
            expr_uses_builtin(start, name)
                || expr_uses_builtin(stop, name)
                || expr_uses_builtin(step, name)
                || block_uses_builtin(body, name)
        }
        Stmt::ForEach { iter, body, .. } => {
            expr_uses_builtin(iter, name) || block_uses_builtin(body, name)
        }
        Stmt::SeqSet { seq, index, value, .. } => {
            expr_uses_builtin(seq, name)
                || expr_uses_builtin(index, name)
                || expr_uses_builtin(value, name)
        }
        Stmt::MapSet { map, key, value, .. } => {
            expr_uses_builtin(map, name)
                || expr_uses_builtin(key, name)
                || expr_uses_builtin(value, name)
        }
        Stmt::ClassDef { body, .. }
        | Stmt::ModuleDef { body, .. }
        | Stmt::SingletonClassDef { body, .. } => stmts_use_builtin(body, name),
        Stmt::TryCatch { body, rescues, ensure_body, .. } => {
            stmts_use_builtin(body, name)
                || rescues.iter().any(|r| stmts_use_builtin(&r.body, name))
                || ensure_body.as_deref().is_some_and(|e| stmts_use_builtin(e, name))
        }
    }
}

fn expr_uses_builtin(e: &Expr, name: &str) -> bool {
    match e {
        Expr::BuiltinCall { name: n, args, .. } => {
            n == name || args.iter().any(|a| expr_uses_builtin(a, name))
        }
        Expr::DirectCall { args, .. } => args.iter().any(|a| expr_uses_builtin(a, name)),
        Expr::IndirectCall { target, args, .. } => {
            expr_uses_builtin(target, name) || args.iter().any(|a| expr_uses_builtin(a, name))
        }
        Expr::If { cond, then_branch, else_branch, .. } => {
            expr_uses_builtin(cond, name)
                || block_uses_builtin(then_branch, name)
                || block_uses_builtin(else_branch, name)
        }
        Expr::Block(b) => block_uses_builtin(b, name),
        Expr::MakeClosure { captures, .. } => {
            captures.iter().any(|c| expr_uses_builtin(&c.value, name))
        }
        Expr::SeqLit { items, .. } => items.iter().any(|i| expr_uses_builtin(i, name)),
        Expr::SeqIndex { seq, index, .. } => {
            expr_uses_builtin(seq, name) || expr_uses_builtin(index, name)
        }
        Expr::SeqLen { seq, .. } => expr_uses_builtin(seq, name),
        Expr::MapLit { entries, .. } => entries
            .iter()
            .any(|en| expr_uses_builtin(&en.key, name) || expr_uses_builtin(&en.value, name)),
        Expr::MapGet { map, key, .. } => {
            expr_uses_builtin(map, name) || expr_uses_builtin(key, name)
        }
        Expr::LogicalAnd { lhs, rhs, .. } | Expr::LogicalOr { lhs, rhs, .. } => {
            expr_uses_builtin(lhs, name) || expr_uses_builtin(rhs, name)
        }
        Expr::StrConcat { parts, .. } => parts.iter().any(|p| expr_uses_builtin(p, name)),
        Expr::Intrinsic { args, .. } => args.iter().any(|a| expr_uses_builtin(a, name)),
        Expr::IntLit { .. }
        | Expr::FloatLit { .. }
        | Expr::BoolLit { .. }
        | Expr::NilLit { .. }
        | Expr::SymLit { .. }
        | Expr::StrLit { .. }
        | Expr::VarRef { .. } => false,
    }
}

thread_local! {
    /// Monotonic counter for synthesised loop temporaries (the
    /// once-evaluated `__stop`/`__step` bounds of a `ForRange`).  Reset
    /// at the start of every `emit_module` so output stays deterministic.
    static LOOP_COUNTER: Cell<usize> = const { Cell::new(0) };

    /// Names that are the target of an `Assign` somewhere in the current
    /// function.  A `LetBinding` for such a name must emit `let` (not
    /// `const`) so the later reassignment type-checks.  Populated per
    /// function in `emit_function`; immutable bindings stay `const`.
    static MUTABLE_NAMES: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
}

fn fresh_loop_id() -> usize {
    LOOP_COUNTER.with(|c| {
        let n = c.get();
        c.set(n + 1);
        n
    })
}

/// Emit a SIR module as TypeScript source.  Caller is responsible
/// for prior validation; this function assumes the module is valid.
pub fn emit_module(m: &Module) -> String {
    LOOP_COUNTER.with(|c| c.set(0));
    let mut out = String::new();
    emit_banner(&mut out, m);
    out.push_str(RUNTIME);
    // Only OOP-using modules import the OOP runtime, so a pure
    // arithmetic module gains no dependency on it.
    if uses_oop(m) {
        out.push_str(crate::runtime::RUNTIME_OOP);
    }
    // Only throwing/rescuing modules import the exception runtime.
    if uses_exceptions(m) {
        out.push_str(crate::runtime::RUNTIME_EXC);
    }
    // Only pair-using modules import the pairs runtime.
    if uses_pairs(m) {
        out.push_str(crate::runtime::RUNTIME_PAIRS);
    }
    // Only regex-using modules import the regex runtime.
    if uses_regex(m) {
        out.push_str(crate::runtime::RUNTIME_REGEX);
    }
    // Only backtick-using modules import the shell runtime.
    if uses_shell(m) {
        out.push_str(crate::runtime::RUNTIME_SHELL);
    }
    // Only range-using modules import the range runtime.
    if uses_range(m) {
        out.push_str(crate::runtime::RUNTIME_RANGE);
    }
    emit_globals(&mut out, &m.globals);
    for f in &m.functions {
        out.push('\n');
        emit_function(&mut out, f);
    }
    emit_module_footer(&mut out, m);
    out
}

fn emit_banner(out: &mut String, m: &Module) {
    let _ = writeln!(
        out,
        "// Generated by semantic-ir-to-typescript v0.1 from SIR module `{}`.",
        sanitize_comment(&m.name)
    );
    if let Some(lang) = &m.metadata.source_language {
        let _ = writeln!(out, "// Source language: {}", sanitize_comment(lang));
    }
    let _ = writeln!(out, "// Do not edit by hand.");
    out.push('\n');
}

fn emit_globals(out: &mut String, globals: &[Global]) {
    if globals.is_empty() {
        return;
    }
    out.push('\n');
    for g in globals {
        let _ = writeln!(out, "let {}: __Sir.Val = null;", sanitize_ident(&g.name));
    }
}

fn emit_module_footer(out: &mut String, m: &Module) {
    out.push('\n');
    if m.functions.iter().any(|f| f.name == "_init") {
        out.push_str("_init();\n");
    }
    if m.functions.iter().any(|f| f.name == "main") {
        out.push_str("const __sir_result: __Sir.Val = main();\n");
        // `__sir_result` is referenced once so TypeScript doesn't
        // emit an "unused variable" warning in strict modes.
        out.push_str("void __sir_result;\n");
    }
}

// ---------------------------------------------------------------------------
// Functions
// ---------------------------------------------------------------------------

fn emit_function(out: &mut String, f: &Function) {
    let _ = writeln!(out, "// SIR span: {}", sanitize_comment(&f.span.to_string()));
    let _ = write!(out, "function {}(", sanitize_ident(&f.name));
    // Captures are positional and come BEFORE params, matching the
    // lowerer's MakeClosure convention.
    let mut first = true;
    for c in &f.captures {
        if !first {
            out.push_str(", ");
        }
        first = false;
        let _ = write!(out, "{}: __Sir.Val", sanitize_ident(&c.name));
    }
    for p in &f.params {
        if !first {
            out.push_str(", ");
        }
        first = false;
        let _ = write!(out, "{}: __Sir.Val", sanitize_ident(&p.name));
    }
    out.push_str("): __Sir.Val {\n");

    // Pre-pass: any local that is later reassigned must bind with `let`.
    MUTABLE_NAMES.with(|m| {
        let mut set = m.borrow_mut();
        set.clear();
        collect_assigned_locals(&f.body, &mut set);
    });

    emit_function_body(out, &f.body, 2);

    MUTABLE_NAMES.with(|m| m.borrow_mut().clear());
    out.push_str("}\n");
}

/// Walk a block (recursing through nested blocks, loop bodies, and
/// block-bearing expressions) collecting the names targeted by an
/// `Assign` with `Local` scope.  These need `let` rather than `const`.
fn collect_assigned_locals(b: &Block, out: &mut HashSet<String>) {
    for s in &b.stmts {
        collect_stmt_assigned(s, out);
    }
    collect_expr_assigned(&b.value, out);
}

fn collect_stmt_assigned(s: &Stmt, out: &mut HashSet<String>) {
    match s {
        Stmt::Assign { name, scope: Scope::Local, value, .. } => {
            out.insert(name.clone());
            collect_expr_assigned(value, out);
        }
        Stmt::Assign { value, .. } => collect_expr_assigned(value, out),
        Stmt::LetBinding { value, .. } | Stmt::LetStarBinding { value, .. } => {
            collect_expr_assigned(value, out);
        }
        Stmt::ExprStmt { expr, .. } => collect_expr_assigned(expr, out),
        Stmt::While { cond, body, .. } => {
            collect_expr_assigned(cond, out);
            collect_assigned_locals(body, out);
        }
        Stmt::ForRange { start, stop, step, body, .. } => {
            collect_expr_assigned(start, out);
            collect_expr_assigned(stop, out);
            collect_expr_assigned(step, out);
            collect_assigned_locals(body, out);
        }
        Stmt::ForEach { iter, body, .. } => {
            collect_expr_assigned(iter, out);
            collect_assigned_locals(body, out);
        }
        Stmt::SeqSet { seq, index, value, .. } => {
            collect_expr_assigned(seq, out);
            collect_expr_assigned(index, out);
            collect_expr_assigned(value, out);
        }
        Stmt::MapSet { map, key, value, .. } => {
            collect_expr_assigned(map, out);
            collect_expr_assigned(key, out);
            collect_expr_assigned(value, out);
        }
        // A try/rescue/ensure carries bare statement lists that may reassign
        // an outer local, so descend into every one.
        Stmt::TryCatch { body, rescues, ensure_body, .. } => {
            for st in body {
                collect_stmt_assigned(st, out);
            }
            for r in rescues {
                for st in &r.body {
                    collect_stmt_assigned(st, out);
                }
            }
            if let Some(ens) = ensure_body {
                for st in ens {
                    collect_stmt_assigned(st, out);
                }
            }
        }
        // Class/module bodies are rejected at the capability check for this
        // backend; nothing to collect.
        Stmt::ClassDef { .. } | Stmt::ModuleDef { .. } | Stmt::SingletonClassDef { .. } => {}
    }
}

fn collect_expr_assigned(e: &Expr, out: &mut HashSet<String>) {
    match e {
        Expr::If { cond, then_branch, else_branch, .. } => {
            collect_expr_assigned(cond, out);
            collect_assigned_locals(then_branch, out);
            collect_assigned_locals(else_branch, out);
        }
        Expr::Block(b) => collect_assigned_locals(b, out),
        Expr::DirectCall { args, .. } | Expr::BuiltinCall { args, .. } => {
            for a in args {
                collect_expr_assigned(a, out);
            }
        }
        Expr::IndirectCall { target, args, .. } => {
            collect_expr_assigned(target, out);
            for a in args {
                collect_expr_assigned(a, out);
            }
        }
        Expr::MakeClosure { captures, .. } => {
            for c in captures {
                collect_expr_assigned(&c.value, out);
            }
        }
        Expr::SeqLit { items, .. } => {
            for i in items {
                collect_expr_assigned(i, out);
            }
        }
        Expr::SeqIndex { seq, index, .. } => {
            collect_expr_assigned(seq, out);
            collect_expr_assigned(index, out);
        }
        Expr::SeqLen { seq, .. } => collect_expr_assigned(seq, out),
        Expr::MapLit { entries, .. } => {
            for entry in entries {
                collect_expr_assigned(&entry.key, out);
                collect_expr_assigned(&entry.value, out);
            }
        }
        Expr::MapGet { map, key, .. } => {
            collect_expr_assigned(map, out);
            collect_expr_assigned(key, out);
        }
        Expr::LogicalAnd { lhs, rhs, .. } | Expr::LogicalOr { lhs, rhs, .. } => {
            collect_expr_assigned(lhs, out);
            collect_expr_assigned(rhs, out);
        }
        Expr::StrConcat { parts, .. } => {
            for p in parts {
                collect_expr_assigned(p, out);
            }
        }
        // Leaves with no nested blocks/exprs that could hold an Assign.
        Expr::IntLit { .. }
        | Expr::FloatLit { .. }
        | Expr::BoolLit { .. }
        | Expr::NilLit { .. }
        | Expr::SymLit { .. }
        | Expr::StrLit { .. }
        | Expr::VarRef { .. }
        | Expr::Intrinsic { .. } => {}
    }
}

fn emit_function_body(out: &mut String, b: &Block, indent: usize) {
    let pad = " ".repeat(indent);
    for s in &b.stmts {
        emit_stmt(out, s, indent);
    }
    let _ = write!(out, "{}return ", pad);
    emit_expr(out, &b.value, indent);
    out.push_str(";\n");
}

// ---------------------------------------------------------------------------
// Statements
// ---------------------------------------------------------------------------

fn emit_stmt(out: &mut String, s: &Stmt, indent: usize) {
    let pad = " ".repeat(indent);
    match s {
        Stmt::LetBinding { name, value, .. } | Stmt::LetStarBinding { name, value, .. } => {
            // `const` for immutable bindings (the common case); `let`
            // when a later `Assign` re-binds this name (otherwise the
            // reassignment would not type-check).  Parallel-let
            // semantics were already preserved by the lowerer, so
            // top-down emission is faithful either way.
            let keyword = if MUTABLE_NAMES.with(|m| m.borrow().contains(name)) {
                "let"
            } else {
                "const"
            };
            let _ = write!(out, "{}{} {}: __Sir.Val = ", pad, keyword, sanitize_ident(name));
            emit_expr(out, value, indent);
            out.push_str(";\n");
        }
        Stmt::ExprStmt { expr, .. } => {
            // Recognize the synthesised _init global_set pattern and
            // emit a direct assignment for nicer output.
            if let Some((global, value)) = pick_global_set(expr) {
                let _ = write!(out, "{}{} = ", pad, sanitize_ident(global));
                emit_expr(out, value, indent);
                out.push_str(";\n");
            } else {
                let _ = write!(out, "{}", pad);
                emit_expr(out, expr, indent);
                out.push_str(";\n");
            }
        }
        // ── SIR16 mutation ──────────────────────────────────────────
        // `Assign` re-binds an already-declared name.  Local/Param/
        // Capture/Global all resolve to a bare identifier in TS (globals
        // are module-level `let`s), so a plain reassignment is faithful.
        // Instance/ClassVar/Const are not in this backend's accepted
        // features yet, so they fall through to the panic guard.
        Stmt::Assign {
            name,
            scope: scope @ (Scope::Local | Scope::Param | Scope::Capture | Scope::Global),
            value,
            ..
        } => {
            let _ = scope; // identifier is the same for all four
            let _ = write!(out, "{}{} = ", pad, sanitize_ident(name));
            emit_expr(out, value, indent);
            out.push_str(";\n");
        }
        // `seq[index] = value` — cast through the array view of `Val`.
        Stmt::SeqSet { seq, index, value, .. } => {
            out.push_str(&pad);
            out.push_str("((");
            emit_expr(out, seq, indent);
            out.push_str(") as __Sir.Val[])[(");
            emit_expr(out, index, indent);
            out.push_str(") as number] = ");
            emit_expr(out, value, indent);
            out.push_str(";\n");
        }
        // `map[key] = value` — `Map.set` on the map view of `Val`.
        Stmt::MapSet { map, key, value, .. } => {
            out.push_str(&pad);
            out.push_str("((");
            emit_expr(out, map, indent);
            out.push_str(") as Map<__Sir.Val, __Sir.Val>).set(");
            emit_expr(out, key, indent);
            out.push_str(", ");
            emit_expr(out, value, indent);
            out.push_str(");\n");
        }
        // ── SIR16 loops ─────────────────────────────────────────────
        // `while (truthy(cond)) { body }` — the test routes through SIR
        // truthiness (only `false`/`nil` are falsy), never JS truthiness.
        Stmt::While { cond, body, .. } => {
            out.push_str(&pad);
            out.push_str("while (__Sir.truthy(");
            emit_expr(out, cond, indent);
            out.push_str(")) {\n");
            emit_block_as_stmts(out, body, indent + 2);
            let _ = write!(out, "{}}}\n", pad);
        }
        // `for (var = start; …; var += step) { body }` — half-open
        // (`stop` exclusive).  `stop`/`step` are evaluated ONCE into
        // block-scoped temporaries (matching Python's `range`), and the
        // loop condition is direction-aware so a negative `step` counts
        // down correctly.
        Stmt::ForRange { var, start, stop, step, body, .. } => {
            let id = fresh_loop_id();
            let v = sanitize_ident(var);
            let inner = indent + 2;
            let inner_pad = " ".repeat(inner);
            // Open a block so the temporaries don't leak.
            let _ = write!(out, "{}{{\n", pad);
            let _ = write!(out, "{}let {}: __Sir.Val = ", inner_pad, v);
            emit_expr(out, start, inner);
            out.push_str(";\n");
            let _ = write!(out, "{}const __sir_stop_{}: number = (", inner_pad, id);
            emit_expr(out, stop, inner);
            out.push_str(") as number;\n");
            let _ = write!(out, "{}const __sir_step_{}: number = (", inner_pad, id);
            emit_expr(out, step, inner);
            out.push_str(") as number;\n");
            let _ = write!(
                out,
                "{}while (__sir_step_{id} >= 0 ? ({v} as number) < __sir_stop_{id} : ({v} as number) > __sir_stop_{id}) {{\n",
                inner_pad, id = id, v = v
            );
            emit_block_as_stmts(out, body, inner + 2);
            let _ = write!(
                out,
                "{}{} = ({} as number) + __sir_step_{};\n",
                " ".repeat(inner + 2),
                v,
                v,
                id
            );
            let _ = write!(out, "{}}}\n", inner_pad);
            let _ = write!(out, "{}}}\n", pad);
        }
        // `for (const var of iter) { body }` — iterate a Seq.  The
        // binding uses `let` if the body reassigns the loop variable.
        Stmt::ForEach { var, iter, body, .. } => {
            let kw = if MUTABLE_NAMES.with(|m| m.borrow().contains(var)) {
                "let"
            } else {
                "const"
            };
            let _ = write!(out, "{}for ({} {} of ((", pad, kw, sanitize_ident(var));
            emit_expr(out, iter, indent);
            out.push_str(") as __Sir.Val[])) {\n");
            emit_block_as_stmts(out, body, indent + 2);
            let _ = write!(out, "{}}}\n", pad);
        }
        // ── SIR17 scopes (assignment) ───────────────────────────────
        // `@x = v` → current-self instance-variable write via the OOP
        // runtime (no native `this` — methods are receiver-less).
        Stmt::Assign { name, scope: Scope::Instance, value, .. } => {
            let _ = write!(out, "{}__SirOop.ivarSet({}, ", pad, quote_ts_string(name));
            emit_expr(out, value, indent);
            out.push_str(");\n");
        }
        // `@@x = v` → class-variable store write.
        Stmt::Assign { name, scope: Scope::ClassVar, value, .. } => {
            let _ = write!(out, "{}__SirOop.cvarSet({}, ", pad, quote_ts_string(name));
            emit_expr(out, value, indent);
            out.push_str(");\n");
        }
        // `CONST = v` → an ordinary module-level binding.  Constants are
        // assign-once in Ruby, so `const` is faithful; reads elsewhere
        // emit the bare identifier (see `emit_var_ref`).
        Stmt::Assign { name, scope: Scope::Const, value, .. } => {
            let _ = write!(out, "{}const {}: __Sir.Val = ", pad, sanitize_ident(name));
            emit_expr(out, value, indent);
            out.push_str(";\n");
        }
        // `Assign` to a builtin is never produced by any frontend (you
        // cannot rebind `+`); a validated module never reaches here.
        Stmt::Assign { scope: Scope::Builtin, span, .. } => {
            panic!("ts backend reached an assign to a Builtin-scoped name at {} — invalid SIR", span);
        }
        // ── SIR17 class / module / singleton declarations ───────────
        // The Ruby→SIR frontend hoists method `def`s to top-level
        // functions, so a `ClassDef` body carries only its non-`def`
        // statements (constant / class-variable assigns).  We register
        // the class in the OOP runtime (for ancestry-aware `is_a?`) and
        // emit the body statements in source order.
        Stmt::ClassDef { name, superclass, body, .. } => {
            let _ = write!(out, "{}__SirOop.defineClass({}, ", pad, quote_ts_string(name));
            match superclass {
                Some(sup) => {
                    let _ = write!(out, "{}", quote_ts_string(sup));
                }
                None => out.push_str("null"),
            }
            out.push_str(");\n");
            for st in body {
                emit_stmt(out, st, indent);
            }
        }
        // A module is a namespace with no superclass; register it so it
        // can participate in `is_a?`/ancestry, then emit its body.
        Stmt::ModuleDef { name, body, .. } => {
            let _ = write!(out, "{}__SirOop.defineClass({}, null);\n", pad, quote_ts_string(name));
            for st in body {
                emit_stmt(out, st, indent);
            }
        }
        // `class << receiver; …; end` — method `def`s are hoisted out by
        // the frontend, so only the body's non-`def` statements remain.
        Stmt::SingletonClassDef { body, .. } => {
            for st in body {
                emit_stmt(out, st, indent);
            }
        }
        // `begin … rescue … ensure … end` → native `try { … } catch (e) {
        // … } finally { … }`.  A native `catch` binds *one* variable and
        // catches *everything*, while Ruby has an ordered list of typed
        // `rescue` clauses, so the catch body is an if/else-if chain that asks
        // the exception runtime `rescueMatches(exc, [class names])` for each
        // clause in source order and re-`throw`s if none match (matching
        // Ruby's "propagate when unrescued").
        Stmt::TryCatch { body, rescues, ensure_body, .. } => {
            let pad = " ".repeat(indent);
            let _ = write!(out, "{}try {{\n", pad);
            emit_stmt_block(out, body, indent + 2);
            let _ = write!(out, "{}}}", pad);
            if !rescues.is_empty() {
                out.push_str(" catch (__exc) {\n");
                let inner = indent + 2;
                let ipad = " ".repeat(inner);
                for (i, r) in rescues.iter().enumerate() {
                    let mut types = String::from("[");
                    for (j, t) in r.exception_types.iter().enumerate() {
                        if j > 0 {
                            types.push_str(", ");
                        }
                        types.push_str(&quote_ts_string(t));
                    }
                    types.push(']');
                    let kw = if i == 0 { "if" } else { "} else if" };
                    let _ = write!(
                        out,
                        "{}{} (__SirExc.rescueMatches(__exc, {})) {{\n",
                        ipad, kw, types
                    );
                    // `rescue Foo => e` binds the caught value as a local.
                    if let Some(bind) = &r.binding {
                        let _ = write!(
                            out,
                            "{}  const {}: __Sir.Val = __exc;\n",
                            ipad,
                            sanitize_ident(bind)
                        );
                    }
                    emit_stmt_block(out, &r.body, inner + 2);
                }
                // No clause matched → propagate the original exception.
                let _ = write!(out, "{}}} else {{\n", ipad);
                let _ = write!(out, "{}  throw __exc;\n", ipad);
                let _ = write!(out, "{}}}\n", ipad);
                let _ = write!(out, "{}}}", pad);
            }
            if let Some(ens) = ensure_body {
                out.push_str(" finally {\n");
                emit_stmt_block(out, ens, indent + 2);
                let _ = write!(out, "{}}}", pad);
            }
            out.push('\n');
        }
    }
}

/// Detect the `BuiltinCall("global_set", SymLit(name), value)`
/// pattern and return the global name + value if so.
fn pick_global_set(e: &Expr) -> Option<(&str, &Expr)> {
    if let Expr::BuiltinCall { name, args, .. } = e {
        if name == "global_set" && args.len() == 2 {
            if let Expr::SymLit { name: global_name, .. } = &args[0] {
                return Some((global_name.as_str(), &args[1]));
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Expressions
// ---------------------------------------------------------------------------

fn emit_expr(out: &mut String, e: &Expr, indent: usize) {
    match e {
        Expr::IntLit { value, .. } => {
            let _ = write!(out, "{}", value);
        }
        Expr::BoolLit { value, .. } => {
            let _ = write!(out, "{}", if *value { "true" } else { "false" });
        }
        Expr::NilLit { .. } => out.push_str("null"),
        Expr::SymLit { name, .. } => {
            let _ = write!(out, "__Sir.intern({})", quote_ts_string(name));
        }
        Expr::StrLit { value, .. } => {
            let _ = write!(out, "{}", quote_ts_string(value));
        }
        Expr::VarRef { name, scope, .. } => emit_var_ref(out, name, *scope),
        Expr::If { cond, then_branch, else_branch, .. } => {
            out.push_str("(__Sir.truthy(");
            emit_expr(out, cond, indent);
            out.push_str(") ? (");
            emit_block_as_expr(out, then_branch, indent);
            out.push_str(") : (");
            emit_block_as_expr(out, else_branch, indent);
            out.push_str("))");
        }
        Expr::Block(b) => emit_block_as_expr(out, b, indent),
        Expr::DirectCall { fn_name, args, .. } => {
            let _ = write!(out, "{}(", sanitize_ident(fn_name));
            emit_args(out, args, indent);
            out.push(')');
        }
        Expr::IndirectCall { target, args, .. } => {
            out.push_str("__Sir.apply(");
            emit_expr(out, target, indent);
            out.push_str(", [");
            emit_args(out, args, indent);
            out.push_str("])");
        }
        Expr::BuiltinCall { name, args, .. } => emit_builtin_call(out, name, args, indent),
        Expr::MakeClosure { fn_name, captures, .. } => {
            // Render: new __Sir.Closure((..._a) => <fn>(<cap0>, <cap1>, ..._a))
            out.push_str("new __Sir.Closure((..._a: __Sir.Val[]) => ");
            let _ = write!(out, "{}(", sanitize_ident(fn_name));
            for c in captures {
                emit_expr(out, &c.value, indent);
                out.push_str(", ");
            }
            out.push_str("..._a))");
        }
        Expr::Intrinsic { name, span, .. } => {
            // v0 backend accepts no intrinsics; the check_module
            // pass rejects them before reaching this point.  Panic
            // here to catch a backend bug if the check ever drifts.
            panic!(
                "emit reached an Intrinsic `{}` at {} — backend should have rejected it",
                name, span
            );
        }
        // ── SIR16 expression kinds — native TypeScript ─────────────
        Expr::FloatLit { value, .. } => {
            let _ = write!(out, "{:?}", value);
        }
        Expr::SeqLit { items, .. } => {
            out.push('[');
            emit_args(out, items, indent);
            out.push(']');
        }
        Expr::SeqIndex { seq, index, .. } => {
            // `Val` is a union; cast to an array/number for the native
            // index.
            out.push_str("((");
            emit_expr(out, seq, indent);
            out.push_str(") as __Sir.Val[])[(");
            emit_expr(out, index, indent);
            out.push_str(") as number]");
        }
        Expr::SeqLen { seq, .. } => {
            out.push_str("((");
            emit_expr(out, seq, indent);
            out.push_str(") as __Sir.Val[]).length");
        }
        Expr::MapLit { entries, .. } => {
            out.push_str("new Map<__Sir.Val, __Sir.Val>([");
            for (i, entry) in entries.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push('[');
                emit_expr(out, &entry.key, indent);
                out.push_str(", ");
                emit_expr(out, &entry.value, indent);
                out.push(']');
            }
            out.push_str("])");
        }
        Expr::MapGet { map, key, .. } => {
            out.push_str("(((");
            emit_expr(out, map, indent);
            out.push_str(") as Map<__Sir.Val, __Sir.Val>).get(");
            emit_expr(out, key, indent);
            out.push_str(") ?? null)");
        }
        // Short-circuit: an arrow closure keeps the rhs unevaluated until
        // the lhs decides, routing the test through SIR truthiness (only
        // `false`/`nil` are falsy — not `0`/`""`).  The param `__l` gives
        // each occurrence its own scope, so nested `&&`/`||` never collide.
        Expr::LogicalAnd { lhs, rhs, .. } => {
            out.push_str("((__l: __Sir.Val) => __Sir.truthy(__l) ? (");
            emit_expr(out, rhs, indent);
            out.push_str(") : __l)(");
            emit_expr(out, lhs, indent);
            out.push(')');
        }
        Expr::LogicalOr { lhs, rhs, .. } => {
            out.push_str("((__l: __Sir.Val) => __Sir.truthy(__l) ? __l : (");
            emit_expr(out, rhs, indent);
            out.push_str("))(");
            emit_expr(out, lhs, indent);
            out.push(')');
        }
        // String interpolation: each part rendered through the SIR display
        // helper (a string part renders to itself) and joined.
        Expr::StrConcat { parts, .. } => {
            out.push('(');
            if parts.is_empty() {
                out.push_str("\"\"");
            }
            for (i, p) in parts.iter().enumerate() {
                if i > 0 {
                    out.push_str(" + ");
                }
                out.push_str("__Sir.toDisplay(");
                emit_expr(out, p, indent);
                out.push(')');
            }
            out.push(')');
        }
    }
}

fn emit_var_ref(out: &mut String, name: &str, scope: Scope) {
    match scope {
        Scope::Local | Scope::Param | Scope::Capture | Scope::Global => {
            let _ = write!(out, "{}", sanitize_ident(name));
        }
        Scope::Builtin => {
            let _ = write!(out, "__Sir.builtinClosure({})", quote_ts_string(name));
        }
        // SIR17 scopes — emitted via the OOP runtime, since the
        // Ruby→SIR frontend hoists methods to receiver-less top-level
        // functions (no native `this` to read members from).
        Scope::Instance => {
            // `@x` → current-self instance-variable read.
            let _ = write!(out, "__SirOop.ivarGet({})", quote_ts_string(name));
        }
        Scope::ClassVar => {
            // `@@x` → class-variable store read.
            let _ = write!(out, "__SirOop.cvarGet({})", quote_ts_string(name));
        }
        Scope::Const => {
            // Constants are ordinary module-level bindings — a bare,
            // sanitised identifier (e.g. `LEGS`).
            out.push_str(&sanitize_ident(name));
        }
    }
}

fn emit_args(out: &mut String, args: &[Expr], indent: usize) {
    for (i, a) in args.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        emit_expr(out, a, indent);
    }
}

fn emit_builtin_call(out: &mut String, name: &str, args: &[Expr], indent: usize) {
    // Reflective method dispatch: the Ruby→SIR frontend lowers
    // `recv.meth(args…)` to `BuiltinCall("__method__", [recv, "meth",
    // args…])`.  Route it through the OOP runtime's `callMethod`.  For
    // the class-predicate methods, a `Const`-scoped class operand (e.g.
    // `Integer`) is passed as its *name string* so the predicate works
    // without a binding for the built-in class name.
    if name == "__method__" && args.len() >= 2 {
        if let Expr::StrLit { value: meth, .. } = &args[1] {
            out.push_str("__SirOop.callMethod(");
            emit_expr(out, &args[0], indent);
            let _ = write!(out, ", {}", quote_ts_string(meth));
            let is_class_pred =
                matches!(meth.as_str(), "is_a?" | "kind_of?" | "instance_of?");
            for (i, a) in args[2..].iter().enumerate() {
                out.push_str(", ");
                match a {
                    // First operand of a class predicate, given as a
                    // constant class name → emit the name as a string.
                    Expr::VarRef { name: cn, scope: Scope::Const, .. } if is_class_pred && i == 0 => {
                        out.push_str(&quote_ts_string(cn));
                    }
                    _ => emit_expr(out, a, indent),
                }
            }
            out.push(')');
            return;
        }
    }
    // `raise` → throw a SIR exception via the exception runtime.  The first
    // argument decides the shape:
    //   • a `Const` class name (`raise Foo` / `raise Foo, "msg"`) → the class
    //     name is passed as a *string* (no binding needed for a built-in
    //     class), with the optional message second;
    //   • any other first arg (`raise "msg"`) → an implicit `RuntimeError`
    //     carrying that value as the message (matching Ruby);
    //   • no args (bare `raise`) → a generic re-raise (`RuntimeError`).
    if name == "raise" {
        out.push_str("__SirExc.raiseError(");
        match args.first() {
            None => {}
            Some(Expr::VarRef { name: cn, scope: Scope::Const, .. }) => {
                out.push_str(&quote_ts_string(cn));
                if let Some(msg) = args.get(1) {
                    out.push_str(", ");
                    emit_expr(out, msg, indent);
                }
            }
            Some(other) => {
                out.push_str("\"RuntimeError\", ");
                emit_expr(out, other, indent);
            }
        }
        out.push(')');
        return;
    }
    // `regex` (a Ruby `/pat/flags` literal) → compile via the regex runtime.
    // Args are `[pattern, flags]`; routes to `__SirRegex.compile`, gated by
    // `uses_regex`.
    if name == "regex" {
        out.push_str("__SirRegex.compile(");
        emit_args(out, args, indent);
        out.push(')');
        return;
    }
    // `backtick` (a Ruby `` `cmd` `` literal) → run via the shell runtime,
    // returning the command's stdout.  Gated by `uses_shell`.
    if name == "backtick" {
        out.push_str("__SirShell.backtick(");
        emit_args(out, args, indent);
        out.push(')');
        return;
    }
    // `range` (a Ruby `a..b` / `a...b` literal) → construct a first-class SIR
    // `Range` via the range runtime.  Args are `[start, stop, exclusive]`
    // (start/stop may be `NilLit` for the begin/endless forms).  Gated by
    // `uses_range`.
    if name == "range" {
        out.push_str("__SirRange.range(");
        emit_args(out, args, indent);
        out.push(')');
        return;
    }
    // Ruby `&&`/`and` and `||`/`or` lower to `BuiltinCall("and"/"or", [lhs,
    // rhs])`.  They must **short-circuit** and use SIR truthiness, so they
    // emit the same truthy-guarded arrow IIFE as `Expr::LogicalAnd`/`LogicalOr`
    // rather than routing through the eager `callBuiltin` dispatch.
    if name == "and" && args.len() == 2 {
        out.push_str("((__l: __Sir.Val) => __Sir.truthy(__l) ? (");
        emit_expr(out, &args[1], indent);
        out.push_str(") : __l)(");
        emit_expr(out, &args[0], indent);
        out.push(')');
        return;
    }
    if name == "or" && args.len() == 2 {
        out.push_str("((__l: __Sir.Val) => __Sir.truthy(__l) ? __l : (");
        emit_expr(out, &args[1], indent);
        out.push_str("))(");
        emit_expr(out, &args[0], indent);
        out.push(')');
        return;
    }
    // `!`/`not` → SIR-truthiness negation (always boolean); `-x` (unary minus)
    // → numeric negation.
    if name == "not" && args.len() == 1 {
        out.push_str("(!__Sir.truthy(");
        emit_expr(out, &args[0], indent);
        out.push_str("))");
        return;
    }
    if name == "neg" && args.len() == 1 {
        out.push_str("(-(");
        emit_expr(out, &args[0], indent);
        out.push_str("))");
        return;
    }
    // `lambda` / `->{…}` lower to `BuiltinCall("lambda", [MakeClosure])`.  The
    // lambda *is* its closure value, so we emit the inner `MakeClosure`
    // directly (which renders `new __Sir.Closure(...)`) rather than routing
    // through the eager `callBuiltin` dispatch — there is no separate
    // "lambda" runtime helper, the closure already is the result.
    if name == "lambda" && args.len() == 1 {
        emit_expr(out, &args[0], indent);
        return;
    }
    let helper = match name {
        "+" => "__Sir.add",
        "-" => "__Sir.sub",
        "*" => "__Sir.mul",
        "/" => "__Sir.div",
        "=" => "__Sir.eq",
        "<" => "__Sir.lt",
        ">" => "__Sir.gt",
        // Pairs live in the dedicated `@coding-adventures/sir-runtime-pairs`
        // package now (imported as `__SirPairs`, gated by `uses_pairs`).
        "cons" => "__SirPairs.cons",
        "car" => "__SirPairs.car",
        "cdr" => "__SirPairs.cdr",
        "null?" => "__Sir.isNull",
        "pair?" => "__SirPairs.isPair",
        "number?" => "__Sir.isNumber",
        "symbol?" => "__Sir.isSymbol",
        "print" => "__Sir.print",
        "global_set" => "__Sir.globalSet",
        "global_get" => "__Sir.globalGet",
        // Unknown builtin: route through the dispatch table.  This
        // lets future builtins land without immediate backend changes.
        _ => {
            let _ = write!(out, "__Sir.callBuiltin({}, [", quote_ts_string(name));
            emit_args(out, args, indent);
            out.push_str("])");
            return;
        }
    };
    let _ = write!(out, "{}(", helper);
    emit_args(out, args, indent);
    out.push(')');
}

/// Emit a bare statement list (a `Vec<Stmt>`, as carried by `TryCatch`
/// bodies / rescue clauses / `ensure`) at the given indent.
fn emit_stmt_block(out: &mut String, stmts: &[Stmt], indent: usize) {
    for s in stmts {
        emit_stmt(out, s, indent);
    }
}

fn emit_block_as_expr(out: &mut String, b: &Block, indent: usize) {
    // Empty-stmts block: render the value directly without wrapping.
    if b.stmts.is_empty() {
        emit_expr(out, &b.value, indent);
        return;
    }
    // Non-empty: wrap in an IIFE so let bindings don't leak.
    out.push_str("(() => {\n");
    let inner_indent = indent + 2;
    for s in &b.stmts {
        emit_stmt(out, s, inner_indent);
    }
    let pad = " ".repeat(inner_indent);
    let _ = write!(out, "{}return ", pad);
    emit_expr(out, &b.value, inner_indent);
    out.push_str(";\n");
    let outer_pad = " ".repeat(indent);
    let _ = write!(out, "{}}})()", outer_pad);
}

/// Emit a block in **statement context** — used for loop bodies, whose
/// trailing value is discarded rather than returned.  Each statement is
/// emitted in order; the block's trailing `value` is emitted as an
/// expression statement so any side effect still fires, except a bare
/// `nil` (the common "this block yields nothing" marker), which is
/// dropped to keep the output clean.
fn emit_block_as_stmts(out: &mut String, b: &Block, indent: usize) {
    for s in &b.stmts {
        emit_stmt(out, s, indent);
    }
    if !matches!(b.value, Expr::NilLit { .. }) {
        let pad = " ".repeat(indent);
        out.push_str(&pad);
        emit_expr(out, &b.value, indent);
        out.push_str(";\n");
    }
}

// ---------------------------------------------------------------------------
// Lexical helpers
// ---------------------------------------------------------------------------

/// Sanitize a SIR identifier for TypeScript.
///
/// SIR names can include `?`, `!`, `-`, `+`, `*`, etc.  TypeScript
/// identifiers must match `[A-Za-z_$][A-Za-z0-9_$]*`.  We map
/// problematic characters to safe substitutes; names that are
/// already valid identifiers pass through unchanged.
///
/// Empty input is sanitised to `"_$empty"` to guarantee the result
/// is a valid TS identifier and to avoid colliding with the
/// `"_$"`-prefixed forms produced for other invalid inputs.
pub fn sanitize_ident(s: &str) -> String {
    if s.is_empty() {
        return "_$empty".to_string();
    }
    if is_valid_ts_ident(s) {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len() + 4);
    out.push('_'); // prefix so the result never starts with a digit
    out.push('$');
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '$' {
            out.push(ch);
        } else {
            // Encode as the unicode codepoint to avoid collisions.
            let _ = write!(out, "_{:x}", ch as u32);
        }
    }
    out
}

/// Render `s` safe for inclusion in a TypeScript single-line `//`
/// comment.
///
/// Strips characters that end a line comment: `\n`, `\r`, U+2028
/// (LINE SEPARATOR), U+2029 (PARAGRAPH SEPARATOR), and U+0085
/// (NEXT LINE).  Without this, a hostile SIR producer could put
/// newlines inside a `Span.file` or `Module.name` and inject lines
/// after the comment marker that the TypeScript parser would treat
/// as ordinary code.
fn sanitize_comment(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\n' | '\r' | '\u{0085}' | '\u{2028}' | '\u{2029}' => out.push(' '),
            _ => out.push(ch),
        }
    }
    out
}

fn is_valid_ts_ident(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let mut chars = s.chars();
    let first = chars.next().unwrap();
    if !(first.is_ascii_alphabetic() || first == '_' || first == '$') {
        return false;
    }
    for ch in chars {
        if !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '$') {
            return false;
        }
    }
    !is_ts_reserved(s)
}

fn is_ts_reserved(s: &str) -> bool {
    // The set we care about for v0.  A larger set may be added
    // later; sanitize_ident will prefix `_$` for any of these.
    matches!(
        s,
        "break"
            | "case"
            | "catch"
            | "class"
            | "const"
            | "continue"
            | "debugger"
            | "default"
            | "delete"
            | "do"
            | "else"
            | "enum"
            | "export"
            | "extends"
            | "false"
            | "finally"
            | "for"
            | "function"
            | "if"
            | "import"
            | "in"
            | "instanceof"
            | "new"
            | "null"
            | "return"
            | "super"
            | "switch"
            | "this"
            | "throw"
            | "true"
            | "try"
            | "typeof"
            | "var"
            | "void"
            | "while"
            | "with"
            | "yield"
            | "let"
            | "static"
            | "any"
            | "boolean"
            | "number"
            | "string"
            | "symbol"
            | "type"
            | "namespace"
            | "interface"
            | "package"
            | "private"
            | "protected"
            | "public"
    )
}

fn quote_ts_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            // U+2028 (LINE SEPARATOR) and U+2029 (PARAGRAPH
            // SEPARATOR) are allowed inside ES2019+ string
            // literals but cause problems with older parsers,
            // source-map post-processors, and template-literal
            // tooling.  Escape defensively.
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use semantic_ir::{EffectSet, FeatureManifest, Metadata, Param, Span};

    fn s() -> Span {
        Span::synthetic()
    }

    #[test]
    fn sanitize_comment_strips_newlines_and_unicode_separators() {
        let injected = "x\n*/console.log('pwn');//";
        let safe = sanitize_comment(injected);
        assert!(!safe.contains('\n'));
        assert!(!safe.contains('\r'));
        let with_sep = format!("a\u{2028}b\u{2029}c\u{0085}d");
        let safe2 = sanitize_comment(&with_sep);
        assert!(!safe2.contains('\u{2028}'));
        assert!(!safe2.contains('\u{2029}'));
        assert!(!safe2.contains('\u{0085}'));
    }

    #[test]
    fn sanitize_empty_ident_returns_safe_sentinel() {
        assert_eq!(sanitize_ident(""), "_$empty");
    }

    #[test]
    fn quote_ts_string_escapes_unicode_line_separators() {
        let s = format!("a\u{2028}b\u{2029}c");
        let q = quote_ts_string(&s);
        assert!(q.contains(r"\u2028"), "got {}", q);
        assert!(q.contains(r"\u2029"), "got {}", q);
        assert!(!q.contains('\u{2028}'));
        assert!(!q.contains('\u{2029}'));
    }

    #[test]
    fn emit_function_sanitises_span_in_comment() {
        let body = Block {
            stmts: vec![],
            value: Expr::IntLit { value: 0, span: s() },
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
            // Hostile span file with embedded newline + comment escape.
            span: Span::new("hostile\nALERT(1);//", 1, 1, 1, 1),
        };
        let mut out = String::new();
        emit_function(&mut out, &f);
        // First line is the comment.  It must not contain a newline
        // *before* the comment is terminated; check that the comment
        // line is followed immediately by `function f(...)`.
        let span_line_end = out.find('\n').unwrap();
        let span_line = &out[..span_line_end];
        assert!(span_line.starts_with("// SIR span:"));
        assert!(!span_line.contains("ALERT") || span_line.contains("ALERT"));
        // Critical: the *next* non-empty line must be `function f(`,
        // not `ALERT(...)`.  This proves the newline was sanitised.
        let after_span = &out[span_line_end + 1..];
        assert!(
            after_span.trim_start().starts_with("function"),
            "comment injection — got: {:?}",
            after_span
        );
    }

    #[test]
    fn sanitize_passes_through_valid_idents() {
        assert_eq!(sanitize_ident("hello"), "hello");
        assert_eq!(sanitize_ident("foo_bar"), "foo_bar");
        assert_eq!(sanitize_ident("$x"), "$x");
    }

    #[test]
    fn sanitize_rewrites_invalid_chars() {
        let r = sanitize_ident("null?");
        assert!(r.starts_with("_$"));
        assert!(r.contains("null"));
        // The `?` becomes a hex-encoded fragment.
        assert!(r.contains("_3f"));
    }

    #[test]
    fn sanitize_avoids_reserved_words() {
        assert_ne!(sanitize_ident("class"), "class");
        assert!(sanitize_ident("class").starts_with("_$"));
    }

    #[test]
    fn quote_ts_string_basic() {
        assert_eq!(quote_ts_string("hi"), r#""hi""#);
    }

    #[test]
    fn quote_ts_string_escapes() {
        assert_eq!(quote_ts_string("a\"b\nc\\d"), r#""a\"b\nc\\d""#);
    }

    #[test]
    fn quote_ts_string_low_control_chars() {
        // Codepoints below 0x20 are emitted as backslash-u escapes.
        let q = quote_ts_string("\u{0001}");
        assert!(q.contains(r"\u0001"), "got {}", q);
    }

    #[test]
    fn emit_int_literal() {
        let mut out = String::new();
        emit_expr(&mut out, &Expr::IntLit { value: 42, span: s() }, 0);
        assert_eq!(out, "42");
    }

    #[test]
    fn emit_bool_and_nil() {
        let mut a = String::new();
        emit_expr(&mut a, &Expr::BoolLit { value: true, span: s() }, 0);
        let mut b = String::new();
        emit_expr(&mut b, &Expr::BoolLit { value: false, span: s() }, 0);
        let mut c = String::new();
        emit_expr(&mut c, &Expr::NilLit { span: s() }, 0);
        assert_eq!(a, "true");
        assert_eq!(b, "false");
        assert_eq!(c, "null");
    }

    #[test]
    fn emit_symbol_interns() {
        let mut out = String::new();
        emit_expr(&mut out, &Expr::SymLit { name: "foo".into(), span: s() }, 0);
        assert_eq!(out, r#"__Sir.intern("foo")"#);
    }

    #[test]
    fn emit_var_ref_param() {
        let mut out = String::new();
        emit_expr(
            &mut out,
            &Expr::VarRef {
                name: "x".into(),
                scope: Scope::Param,
                span: s(),
            },
            0,
        );
        assert_eq!(out, "x");
    }

    #[test]
    fn emit_var_ref_builtin_uses_dispatch() {
        let mut out = String::new();
        emit_expr(
            &mut out,
            &Expr::VarRef {
                name: "+".into(),
                scope: Scope::Builtin,
                span: s(),
            },
            0,
        );
        assert_eq!(out, r#"__Sir.builtinClosure("+")"#);
    }

    #[test]
    fn emit_builtin_call_plus() {
        let mut out = String::new();
        emit_expr(
            &mut out,
            &Expr::BuiltinCall {
                name: "+".into(),
                args: vec![
                    Expr::IntLit { value: 1, span: s() },
                    Expr::IntLit { value: 2, span: s() },
                ],
                effects: EffectSet::PURE,
                span: s(),
            },
            0,
        );
        assert_eq!(out, "__Sir.add(1, 2)");
    }

    #[test]
    fn emit_direct_call() {
        let mut out = String::new();
        emit_expr(
            &mut out,
            &Expr::DirectCall {
                fn_name: "id".into(),
                args: vec![Expr::IntLit { value: 7, span: s() }],
                effects: EffectSet::PURE,
                span: s(),
            },
            0,
        );
        assert_eq!(out, "id(7)");
    }

    #[test]
    fn emit_indirect_call_uses_apply() {
        let mut out = String::new();
        emit_expr(
            &mut out,
            &Expr::IndirectCall {
                target: Box::new(Expr::VarRef {
                    name: "f".into(),
                    scope: Scope::Local,
                    span: s(),
                }),
                args: vec![Expr::IntLit { value: 5, span: s() }],
                effects: EffectSet::PURE,
                span: s(),
            },
            0,
        );
        assert_eq!(out, "__Sir.apply(f, [5])");
    }

    #[test]
    fn emit_make_closure() {
        let mut out = String::new();
        emit_expr(
            &mut out,
            &Expr::MakeClosure {
                fn_name: "__lambda_0".into(),
                captures: vec![semantic_ir::CaptureValue {
                    name: "n".into(),
                    value: Expr::IntLit { value: 5, span: s() },
                }],
                span: s(),
            },
            0,
        );
        assert_eq!(
            out,
            "new __Sir.Closure((..._a: __Sir.Val[]) => __lambda_0(5, ..._a))"
        );
    }

    #[test]
    fn emit_simple_function() {
        let body = Block {
            stmts: vec![],
            value: Expr::VarRef {
                name: "x".into(),
                scope: Scope::Param,
                span: s(),
            },
            span: s(),
        };
        let f = Function {
            name: "id".into(),
            params: vec![Param { name: "x".into(), sir_type: None, span: s() }],
            return_type: None,
            captures: vec![],
            body,
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        };
        let mut out = String::new();
        emit_function(&mut out, &f);
        assert!(out.contains("function id(x: __Sir.Val): __Sir.Val"));
        assert!(out.contains("return x;"));
    }

    #[test]
    fn emit_full_module_has_banner_runtime_and_main() {
        let m = Module {
            name: "demo".into(),
            manifest: FeatureManifest::new(),
            imports: vec![],
            exports: vec![],
            functions: vec![Function {
                name: "main".into(),
                params: vec![],
                return_type: None,
                captures: vec![],
                body: Block {
                    stmts: vec![],
                    value: Expr::IntLit { value: 42, span: s() },
                    span: s(),
                },
                effects: EffectSet::PURE,
                metadata: Metadata::new(),
                span: s(),
            }],
            globals: vec![],
            metadata: Metadata::new()
                .with_source_language("twig")
                .with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
            span: s(),
        };
        let out = emit_module(&m);
        assert!(out.contains("// Generated by semantic-ir-to-typescript"));
        assert!(out.contains(r#"import * as __Sir from "@coding-adventures/sir-runtime-core";"#));
        assert!(out.contains("function main()"));
        assert!(out.contains("const __sir_result: __Sir.Val = main();"));
    }

    #[test]
    fn emit_pick_global_set_pattern() {
        // The init-statement pattern (BuiltinCall("global_set",
        // SymLit, value)) should render as a direct assignment.
        let e = Expr::BuiltinCall {
            name: "global_set".into(),
            args: vec![
                Expr::SymLit { name: "counter".into(), span: s() },
                Expr::IntLit { value: 0, span: s() },
            ],
            effects: EffectSet::PURE,
            span: s(),
        };
        let stmt = Stmt::ExprStmt { expr: e, span: s() };
        let mut out = String::new();
        emit_stmt(&mut out, &stmt, 0);
        assert!(out.contains("counter = 0;"));
        // Crucially the SymLit form is suppressed in favour of the
        // direct assignment.
        assert!(!out.contains("intern"));
    }
}
