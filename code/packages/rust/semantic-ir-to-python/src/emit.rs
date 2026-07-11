//! Python emitter — walks a SIR module and produces Python 3 source.
//!
//! See SIR14 for the per-node lowering rules.
//!
//! Python is expression-rich-but-statement-shaped: `let` bindings
//! are statements but SIR `Block`s can appear in expression position.
//! The emitter handles this by lifting non-trivial blocks into
//! nested `def __block_<n>():` functions invoked at the source site.

use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use std::fmt::Write;

use semantic_ir::{
    Block, Expr, Feature, Function, Global, IndexArg, Module, ParamKind, Scope, Stmt,
};

use crate::runtime::RUNTIME;

/// True if the module uses any object-orientation feature, in which case the
/// emitted artifact imports `coding-adventures-sir-runtime-oop`.
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
        // Method dispatch (`recv.meth(args)` → `BuiltinCall("__method__", …)`)
        // and scoped lookups (`A::B` → `BuiltinCall("__scope__", …)`) both route
        // through `_sir_oop_call_method`/the OOP runtime even when the module
        // declares no class/module of its own — e.g. `"hi".upcase` or, post-M3,
        // a rest param used as an Array (`def f(*a); a.length; end`). Gate the
        // OOP import on those builtins too, else the emitted call is undefined.
        || module_uses_builtin(m, "__method__")
        || module_uses_builtin(m, "__scope__")
        // `case_eq` (M5) routes through the OOP runtime helper.
        || module_uses_builtin(m, "case_eq")
        // OOP object-model builtins (O1): `Foo.new`, `super`, `self`, and the
        // method-table registrations all resolve to the OOP runtime, so their
        // presence must pull in the import even in a module that declares no
        // class of its own (defensive — the frontend always pairs them with a
        // `ClassDef`, but the gate must not depend on that).
        || module_uses_builtin(m, "__new__")
        || module_uses_builtin(m, "__super__")
        || module_uses_builtin(m, "__def_method__")
        || module_uses_builtin(m, "__def_class_method__")
        // Mixins (MX2): `include`/`extend` directives route to the OOP runtime's
        // included-modules / class-method tables, so they pull in the import.
        || module_uses_builtin(m, "__include__")
        || module_uses_builtin(m, "__extend__")
        // Issue #59 — a class-method CALL (`Foo.bar` on a const receiver)
        // routes to `_sir_oop_call_class_method`, so it needs the OOP import.
        || module_uses_builtin(m, "__class_method__")
        || module_uses_builtin(m, "__self__")
}

/// True if the module uses exception handling, in which case the emitted
/// artifact imports `coding-adventures-sir-runtime-exceptions`.
fn uses_exceptions(m: &Module) -> bool {
    m.manifest.contains(Feature::Exceptions)
}

/// Collect every `class Child < Parent` edge in the module as `(child, parent)`
/// pairs, in source order, de-duplicated (first edge for a name wins).
///
/// **Why (E2).**  A `rescue StandardError` must catch a raised user
/// `MyErr < StandardError`, but the exception runtime only knows the built-in
/// hierarchy — it has no way to learn that `MyErr` descends from
/// `StandardError` unless we *tell* it.  `Stmt::ClassDef` carries exactly that
/// static edge, so we harvest the `superclass`-bearing class defs here and emit
/// a single `register_ancestry({...})` call at program init (see
/// [`emit_module`]).  Classes without a superclass (`class Foo`) contribute no
/// edge — they still match by exact name, unchanged.
///
/// The walk mirrors [`stmt_uses_builtin`]: exhaustive over the statement forms
/// that nest bodies (`ClassDef`/`ModuleDef`/`SingletonClassDef`/`TryCatch` and
/// the loops), so a class defined *inside* a `begin`/loop/class body is still
/// found.  A `ClassDef`'s own nested `body` is walked too — Ruby's frontend
/// hoists method `def`s out, but a nested class declaration would remain.
fn collect_user_ancestry(m: &Module) -> Vec<(String, String)> {
    let mut pairs: Vec<(String, String)> = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for f in &m.functions {
        collect_ancestry_in_block(&f.body, &mut pairs, &mut seen);
    }
    pairs
}

fn collect_ancestry_in_block(
    b: &Block,
    pairs: &mut Vec<(String, String)>,
    seen: &mut BTreeSet<String>,
) {
    collect_ancestry_in_stmts(&b.stmts, pairs, seen);
    collect_ancestry_in_expr(&b.value, pairs, seen);
}

fn collect_ancestry_in_stmts(
    stmts: &[Stmt],
    pairs: &mut Vec<(String, String)>,
    seen: &mut BTreeSet<String>,
) {
    for s in stmts {
        collect_ancestry_in_stmt(s, pairs, seen);
    }
}

fn collect_ancestry_in_stmt(
    s: &Stmt,
    pairs: &mut Vec<(String, String)>,
    seen: &mut BTreeSet<String>,
) {
    match s {
        Stmt::ClassDef {
            name,
            superclass,
            body,
            ..
        } => {
            if let Some(sup) = superclass {
                if seen.insert(name.clone()) {
                    pairs.push((name.clone(), sup.clone()));
                }
            }
            collect_ancestry_in_stmts(body, pairs, seen);
        }
        Stmt::ModuleDef { body, .. } | Stmt::SingletonClassDef { body, .. } => {
            collect_ancestry_in_stmts(body, pairs, seen);
        }
        Stmt::While { body, .. } | Stmt::ForRange { body, .. } | Stmt::ForEach { body, .. } => {
            collect_ancestry_in_block(body, pairs, seen);
        }
        Stmt::TryCatch {
            body,
            rescues,
            ensure_body,
            ..
        } => {
            collect_ancestry_in_stmts(body, pairs, seen);
            for r in rescues {
                collect_ancestry_in_stmts(&r.body, pairs, seen);
            }
            if let Some(e) = ensure_body {
                collect_ancestry_in_stmts(e, pairs, seen);
            }
        }
        // A class declaration only surfaces as a `Stmt::ClassDef`; the remaining
        // statement forms carry expressions, which may embed a block (e.g. an
        // `if` in value position) that in turn holds a class def.  Recurse into
        // any nested expression so those are not missed.
        Stmt::LetBinding { value, .. }
        | Stmt::LetStarBinding { value, .. }
        | Stmt::Assign { value, .. }
        | Stmt::ExprStmt { expr: value, .. } => {
            collect_ancestry_in_expr(value, pairs, seen);
        }
        Stmt::SeqSet {
            seq, index, value, ..
        } => {
            collect_ancestry_in_expr(seq, pairs, seen);
            collect_ancestry_in_expr(index, pairs, seen);
            collect_ancestry_in_expr(value, pairs, seen);
        }
        Stmt::MapSet {
            map, key, value, ..
        } => {
            collect_ancestry_in_expr(map, pairs, seen);
            collect_ancestry_in_expr(key, pairs, seen);
            collect_ancestry_in_expr(value, pairs, seen);
        }
        // SIR22 `target[indices...] = value` — same shape as `SeqSet`/
        // `MapSet` above (an indexed mutation), so recurse into its
        // sub-expressions the same way: a class def cannot itself be one
        // of these, but a nested `if`-with-block (etc.) inside `target`,
        // an index, or `value` could still carry one.
        Stmt::IndexSet {
            target,
            indices,
            value,
            ..
        } => {
            collect_ancestry_in_expr(target, pairs, seen);
            for ix in indices {
                collect_ancestry_in_index_arg(ix, pairs, seen);
            }
            collect_ancestry_in_expr(value, pairs, seen);
        }
    }
}

fn collect_ancestry_in_index_arg(
    ix: &IndexArg,
    pairs: &mut Vec<(String, String)>,
    seen: &mut BTreeSet<String>,
) {
    match ix {
        IndexArg::Scalar(e) | IndexArg::Range(e) => collect_ancestry_in_expr(e, pairs, seen),
        IndexArg::Whole => {}
    }
}

fn collect_ancestry_in_expr(
    e: &Expr,
    pairs: &mut Vec<(String, String)>,
    seen: &mut BTreeSet<String>,
) {
    match e {
        Expr::If {
            then_branch,
            else_branch,
            ..
        } => {
            collect_ancestry_in_block(then_branch, pairs, seen);
            collect_ancestry_in_block(else_branch, pairs, seen);
        }
        Expr::Block(b) => collect_ancestry_in_block(b, pairs, seen),
        // No other expression form can nest a statement block that could hold a
        // class declaration (calls/literals carry only sub-*expressions*), so
        // there is nothing further to descend into for ancestry purposes.
        _ => {}
    }
}

/// True if the module uses cons pairs, in which case the emitted artifact
/// imports `coding-adventures-sir-runtime-pairs` (the `cons`/`car`/`cdr`/
/// `pair?` helpers, extracted from core).
fn uses_pairs(m: &Module) -> bool {
    m.manifest.contains(Feature::Pairs)
}

/// True if the module calls the `regex` builtin (a Ruby `/pat/flags` literal
/// lowers to `BuiltinCall("regex", …)`).  Regex is not a SIR `Feature`, so we
/// detect it by walking for the builtin name; a positive result gates the
/// `coding-adventures-sir-runtime-regex` import.
fn uses_regex(m: &Module) -> bool {
    module_uses_builtin(m, "regex")
}

/// True if the module calls the `backtick` builtin (a Ruby `` `cmd` ``
/// literal lowers to `BuiltinCall("backtick", [cmd])`).  Gates the
/// `coding-adventures-sir-runtime-shell` import.
fn uses_shell(m: &Module) -> bool {
    module_uses_builtin(m, "backtick")
}

/// True if the module calls the `range` builtin (a Ruby `a..b` / `a...b`
/// literal lowers to `BuiltinCall("range", [start, stop, exclusive])`).  Range
/// is not a SIR `Feature`, so we detect it by walking for the builtin name; a
/// positive result gates the `coding-adventures-sir-runtime-range` import.
fn uses_range(m: &Module) -> bool {
    module_uses_builtin(m, "range")
}

/// Walk every function body looking for a `BuiltinCall` named `name`.  Used to
/// gate per-concern runtime imports for builtins that carry no `Feature` flag
/// (e.g. `regex`).  Exhaustive over `Stmt`/`Expr` so a new node can't silently
/// hide a use (the compiler forces every arm to be handled).
fn module_uses_builtin(m: &Module, name: &str) -> bool {
    m.functions
        .iter()
        .any(|f| block_uses_builtin(&f.body, name))
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
        Stmt::ForRange {
            start,
            stop,
            step,
            body,
            ..
        } => {
            expr_uses_builtin(start, name)
                || expr_uses_builtin(stop, name)
                || expr_uses_builtin(step, name)
                || block_uses_builtin(body, name)
        }
        Stmt::ForEach { iter, body, .. } => {
            expr_uses_builtin(iter, name) || block_uses_builtin(body, name)
        }
        Stmt::SeqSet {
            seq, index, value, ..
        } => {
            expr_uses_builtin(seq, name)
                || expr_uses_builtin(index, name)
                || expr_uses_builtin(value, name)
        }
        Stmt::MapSet {
            map, key, value, ..
        } => {
            expr_uses_builtin(map, name)
                || expr_uses_builtin(key, name)
                || expr_uses_builtin(value, name)
        }
        // SIR22 `target[indices...] = value` — same recursion shape as
        // `SeqSet`/`MapSet`: walk into `target`, each index argument, and
        // `value` so a nested builtin call (e.g. inside an index
        // expression) is still detected.
        Stmt::IndexSet {
            target,
            indices,
            value,
            ..
        } => {
            expr_uses_builtin(target, name)
                || indices.iter().any(|ix| index_arg_uses_builtin(ix, name))
                || expr_uses_builtin(value, name)
        }
        Stmt::ClassDef { body, .. }
        | Stmt::ModuleDef { body, .. }
        | Stmt::SingletonClassDef { body, .. } => stmts_use_builtin(body, name),
        Stmt::TryCatch {
            body,
            rescues,
            ensure_body,
            ..
        } => {
            stmts_use_builtin(body, name)
                || rescues.iter().any(|r| stmts_use_builtin(&r.body, name))
                || ensure_body
                    .as_deref()
                    .is_some_and(|e| stmts_use_builtin(e, name))
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
        Expr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
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
        // KW1 compile-compat stub: a `KeywordArg` is a single-child wrapper
        // whose runtime meaning is its inner `value`; recurse into it so this
        // builtin-usage scan stays faithful.  Real support pending KW2–KW6.
        Expr::KeywordArg { value, .. } => expr_uses_builtin(value, name),
        // SIR22 array/matrix expressions are not accepted by this backend
        // yet (see `ACCEPTED_FEATURES` in `lib.rs`), so a validated module
        // never contains them in practice. Still, this scan is exhaustive
        // by design so a new node can't silently hide a builtin use — recurse
        // into every sub-expression the same way the other compound-expr
        // arms above do.
        Expr::ArrayLit { rows, .. } => rows
            .iter()
            .any(|row| row.iter().any(|c| expr_uses_builtin(c, name))),
        Expr::Range {
            start, step, stop, ..
        } => {
            expr_uses_builtin(start, name)
                || step.as_deref().is_some_and(|s| expr_uses_builtin(s, name))
                || expr_uses_builtin(stop, name)
        }
        Expr::MatMul { lhs, rhs, .. } => {
            expr_uses_builtin(lhs, name) || expr_uses_builtin(rhs, name)
        }
        Expr::ElementwiseOp { lhs, rhs, .. } => {
            expr_uses_builtin(lhs, name) || expr_uses_builtin(rhs, name)
        }
        Expr::Transpose { target, .. } => expr_uses_builtin(target, name),
        Expr::IndexGet {
            target, indices, ..
        } => {
            expr_uses_builtin(target, name)
                || indices.iter().any(|ix| index_arg_uses_builtin(ix, name))
        }
        Expr::Convert { value, .. } => expr_uses_builtin(value, name),
        // SIR23 symbolic-expression/pattern nodes are not accepted by this
        // backend yet (see `ACCEPTED_FEATURES` in `lib.rs`), so a validated
        // module never contains them in practice — same rationale as the
        // SIR22 arms above; recurse into every sub-expression regardless.
        Expr::SymSymbol { .. } | Expr::SymRational { .. } => false,
        Expr::SymApply { head, args, .. } => {
            expr_uses_builtin(head, name) || args.iter().any(|a| expr_uses_builtin(a, name))
        }
        Expr::SymPatternBlank { head, .. } => {
            head.as_deref().is_some_and(|h| expr_uses_builtin(h, name))
        }
        Expr::SymPatternNamed { pattern, .. } => expr_uses_builtin(pattern, name),
        Expr::SymRule { lhs, rhs, .. } => {
            expr_uses_builtin(lhs, name) || expr_uses_builtin(rhs, name)
        }
        Expr::SymReplaceAll { expr, rules, .. } => {
            expr_uses_builtin(expr, name) || rules.iter().any(|r| expr_uses_builtin(r, name))
        }
        Expr::IntLit { .. }
        | Expr::FloatLit { .. }
        | Expr::BoolLit { .. }
        | Expr::NilLit { .. }
        | Expr::SymLit { .. }
        | Expr::StrLit { .. }
        | Expr::VarRef { .. } => false,
    }
}

/// Same walk as `expr_uses_builtin`, specialised to a single SIR22
/// [`IndexArg`] subscript: `Scalar`/`Range` each wrap one expression to
/// recurse into, and `Whole` (`:`) carries no expression at all.
fn index_arg_uses_builtin(ix: &IndexArg, name: &str) -> bool {
    match ix {
        IndexArg::Scalar(e) | IndexArg::Range(e) => expr_uses_builtin(e, name),
        IndexArg::Whole => false,
    }
}

/// Emit a SIR module as Python 3 source.  Caller is responsible
/// for prior validation; this function assumes the module is valid.
pub fn emit_module(m: &Module) -> String {
    BLOCK_COUNTER.with(|c| *c.borrow_mut() = 0);
    HOIST.with(|h| h.borrow_mut().clear());
    FN_ARITY.with(|t| {
        let mut t = t.borrow_mut();
        t.clear();
        for f in &m.functions {
            t.insert(f.name.clone(), f.params.len());
        }
    });

    let mut out = String::new();
    emit_banner(&mut out, m);
    out.push_str(RUNTIME);
    // Source-language display convention (SIR display-convention spec): a
    // Ruby-sourced module selects the Ruby boolean form (`true`/`false`) once
    // at startup; every other source language keeps the default Lisp `#t`/`#f`,
    // so existing Twig output is unchanged and non-Ruby modules gain no extra
    // import.
    //
    // SECURITY: the emitted argument is a hardcoded `"ruby"` literal chosen by
    // an exact `== "ruby"` comparison — never text derived from
    // `source_language` or any other source-controlled field — so this can
    // never inject into the emitted Python.
    if m.metadata.source_language.as_deref() == Some("ruby") {
        out.push_str(
            "from coding_adventures_sir_runtime_core import \
             set_display_convention as _sir_set_display_convention\n\
             _sir_set_display_convention(\"ruby\")\n",
        );
    }
    // Only OOP-using modules import the OOP runtime, so a pure arithmetic
    // module gains no dependency on it.
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
    // E2: thread user `class Child < Parent` ancestry into the exception
    // matcher at program init, *before* any function or main body runs, so a
    // `rescue StandardError` catches a raised user `MyErr < StandardError`.
    // Gated on both the exception import (so the helper exists) and the
    // presence of at least one superclass edge (so we never emit an empty,
    // meaningless registration for a module that has classes but no
    // inheritance).
    if uses_exceptions(m) {
        let pairs = collect_user_ancestry(m);
        if !pairs.is_empty() {
            out.push_str("\n_sir_exc_register_ancestry({");
            for (i, (child, parent)) in pairs.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                let _ = write!(
                    out,
                    "{}: {}",
                    quote_py_string(child),
                    quote_py_string(parent)
                );
            }
            out.push_str("})\n");
        }
    }
    emit_globals(&mut out, &m.globals);
    for f in &m.functions {
        out.push('\n');
        emit_function(&mut out, f);
    }
    emit_main(&mut out, m);

    BLOCK_COUNTER.with(|c| *c.borrow_mut() = 0);
    HOIST.with(|h| h.borrow_mut().clear());
    FN_ARITY.with(|t| t.borrow_mut().clear());

    out
}

/// Drain the pending nested-`def` hoist buffer into `out`.
///
/// Python has no multi-statement lambda, so a block that contains a
/// loop and appears in *expression* position cannot be inlined.  Such a
/// block is lifted to a nested `def __block_N(): …` (see
/// [`emit_block_as_expr`]) whose source is queued here, and the call
/// site emits `__block_N()`.  The queued defs must be written out
/// *before* the statement whose value-expression referenced them — every
/// statement-emitting path renders to a scratch buffer, calls this, then
/// appends the scratch buffer, so the defs land in the right place.
fn flush_hoist(out: &mut String) {
    let pending: Vec<String> = HOIST.with(|h| std::mem::take(&mut *h.borrow_mut()));
    for def in pending {
        out.push_str(&def);
    }
}

fn emit_banner(out: &mut String, m: &Module) {
    let _ = writeln!(
        out,
        "# Generated by semantic-ir-to-python v0.1 from SIR module `{}`.",
        sanitize_comment(&m.name)
    );
    if let Some(lang) = &m.metadata.source_language {
        let _ = writeln!(out, "# Source language: {}", sanitize_comment(lang));
    }
    let _ = writeln!(out, "# Do not edit by hand.");
    out.push('\n');
}

fn emit_globals(out: &mut String, globals: &[Global]) {
    if globals.is_empty() {
        return;
    }
    out.push_str("\n# Globals (initialised in _init): ");
    let mut first = true;
    for g in globals {
        if !first {
            out.push_str(", ");
        }
        first = false;
        out.push_str(&sanitize_comment(&g.name));
    }
    out.push('\n');
}

fn emit_main(out: &mut String, m: &Module) {
    out.push('\n');
    if m.functions.iter().any(|f| f.name == "_init") {
        out.push_str("_init()\n");
    }
    if m.functions.iter().any(|f| f.name == "main") {
        // SIR's `main` was renamed to `_sir_user_main` to free up
        // top-level `if __name__ == '__main__':` semantics for
        // consumers who want them.  For v0 we just call it.
        out.push_str("_sir_user_main()\n");
    }
}

// ---------------------------------------------------------------------------
// Functions
// ---------------------------------------------------------------------------

fn emit_function(out: &mut String, f: &Function) {
    let _ = writeln!(out, "# SIR span: {}", sanitize_comment(&f.span.to_string()));
    let _ = write!(out, "def {}(", function_emit_name(&f.name));

    let mut first = true;
    for c in &f.captures {
        if !first {
            out.push_str(", ");
        }
        first = false;
        out.push_str(&sanitize_ident(&c.name));
    }
    // KW2 keyword-only `*` separator.  A `Keyword` param is Python-native as a
    // *keyword-only* parameter — one that sits AFTER a `*` in the signature and
    // is bound by name only (`def f(a, *, b, c=1)` accepts `f(1, b=2)` but
    // rejects `f(1, 2)`).  Python opens the keyword-only region in one of two
    // ways, and we must emit the separator exactly once:
    //
    //   * an explicit `*args` collector (SIR `Rest`) — everything after it is
    //     already keyword-only, so it *is* the separator; a second bare `*`
    //     would be a `SyntaxError` (`def f(*a, *, b)`).
    //   * a bare `*` with no name — the form to emit when there is NO `Rest`.
    //
    // The validator's def-side ordering (positional → Rest → Keyword* → KwRest)
    // guarantees every `Keyword` param follows any `Rest`, so a single lookahead
    // suffices: does the whole list contain a `Rest`?  If not, we inject the
    // bare `*` immediately before the FIRST `Keyword` param.  `kw_sep_needed`
    // tracks "still owe a bare `*`", and is cleared the moment we emit one (so
    // the second, third … keyword params do not each re-emit it).
    let has_rest = f.params.iter().any(|p| p.kind == ParamKind::Rest);
    let mut kw_sep_needed = !has_rest;
    for p in &f.params {
        // Inject the bare `*` separator right before the first keyword param,
        // when no `*args`/`Rest` already provides it.  The separator is a
        // stand-alone list element, so it obeys the same comma rule as a param.
        if p.kind == ParamKind::Keyword && kw_sep_needed {
            if !first {
                out.push_str(", ");
            }
            out.push('*');
            first = false;
            kw_sep_needed = false;
        }
        if !first {
            out.push_str(", ");
        }
        first = false;
        // M3 variadic kinds map to Python's native variadic forms:
        //   Rest   (`*rest`)  → `*rest`   (collects trailing positionals)
        //   KwRest (`**opts`) → `**opts`  (collects trailing keywords)
        // Both are faithful in Python; only the construction differs.  A
        // `Keyword` param takes no prefix: its name is emitted bare, but because
        // it now sits after the `*` (or after `*args`) it is keyword-only.
        match p.kind {
            ParamKind::Rest => out.push('*'),
            ParamKind::KwRest => out.push_str("**"),
            ParamKind::Required | ParamKind::Keyword => {}
        }
        out.push_str(&sanitize_ident(&p.name));
        // Default parameters (positional P2c *and* KW2 optional keyword params).
        // A defaulted param gets the *sentinel* as its native Python default —
        // `name=_SIR_MISSING` — so callers may omit the trailing/keyword
        // argument.  We deliberately do NOT emit `name=<expr>`: SIR defaults are
        // call-time and may reference earlier params, which Python's def-time
        // defaults cannot express (`def f(a, b=a)` is a NameError).  The real
        // default is resolved in the body prologue below, uniformly for both
        // positional and keyword optionals.  (`*rest`/`**opts` never default.)
        if matches!(p.kind, ParamKind::Required | ParamKind::Keyword) && p.default.is_some() {
            out.push_str("=_SIR_MISSING");
        }
    }
    out.push_str("):\n");

    // M3 Rest-param normalization. Python's `*rest` binds a *tuple*, but SIR
    // sequence semantics (and Ruby's `*rest`, which is an `Array`) require a
    // *list* — every downstream sequence op (`len`, indexing, dispatched
    // `.map`/`.length`, …) is keyed to `list`. So rebind each Rest param to a
    // list in the function prologue. (`**opts` already binds a `dict`, which
    // matches SIR's map representation, so KwRest needs no fixup.)
    let pad = indent_str(1);
    for p in &f.params {
        if p.kind == ParamKind::Rest {
            let name = sanitize_ident(&p.name);
            let _ = writeln!(out, "{pad}{name} = list({name})");
        }
    }

    // P2c default-parameter resolve-prologue.  Emitted in *param order* so an
    // earlier defaulted param is already resolved before a later default that
    // references it.  Each defaulted param is `_SIR_MISSING` exactly when the
    // caller omitted it; we then rebind it to its default expression, which is
    // emitted through the ordinary expr path and runs *here in the body*, where
    // earlier params are in scope — giving call-time, param-scoped semantics:
    //
    //     if <name> is _SIR_MISSING:
    //         <name> = <default expr>
    //
    // A default expr may itself hoist nested defs (see `flush_hoist`), so each
    // prologue entry is rendered into a scratch buffer, its hoists flushed
    // before it, then appended — matching every other statement-emitting path.
    let inner_pad = indent_str(2);
    for p in &f.params {
        let Some(default) = &p.default else { continue };
        let name = sanitize_ident(&p.name);
        let mut tmp = String::new();
        let _ = writeln!(tmp, "{pad}if {name} is _SIR_MISSING:");
        let _ = write!(tmp, "{inner_pad}{name} = ");
        emit_expr(&mut tmp, default, 1);
        tmp.push('\n');
        flush_hoist(out);
        out.push_str(&tmp);
    }

    emit_function_body(out, &f.body, 1);
}

fn function_emit_name(name: &str) -> String {
    if name == "main" {
        "_sir_user_main".to_string()
    } else {
        sanitize_ident(name)
    }
}

fn emit_function_body(out: &mut String, b: &Block, indent: usize) {
    let pad = indent_str(indent);
    for s in &b.stmts {
        emit_stmt(out, s, indent);
    }
    // Render the return expression into a scratch buffer first so any
    // nested defs it hoists are flushed *before* the `return` line.
    let mut tmp = String::new();
    let _ = write!(tmp, "{}return ", pad);
    emit_expr(&mut tmp, &b.value, indent);
    tmp.push('\n');
    flush_hoist(out);
    out.push_str(&tmp);
}

// ---------------------------------------------------------------------------
// Statements
// ---------------------------------------------------------------------------

/// Emit a single statement, flushing any nested-`def` hoists it
/// generates *before* the statement itself.  The statement is rendered
/// into a scratch buffer (so `emit_expr` can queue hoists), then the
/// hoist buffer is drained into `out`, then the scratch buffer appended.
fn emit_stmt(out: &mut String, s: &Stmt, indent: usize) {
    let mut tmp = String::new();
    emit_stmt_inner(&mut tmp, s, indent);
    flush_hoist(out);
    out.push_str(&tmp);
}

fn emit_stmt_inner(out: &mut String, s: &Stmt, indent: usize) {
    let pad = indent_str(indent);
    match s {
        Stmt::LetBinding { name, value, .. } | Stmt::LetStarBinding { name, value, .. } => {
            let _ = write!(out, "{}{} = ", pad, sanitize_ident(name));
            emit_expr(out, value, indent);
            out.push('\n');
        }
        Stmt::ExprStmt { expr, .. } => {
            // Recognise the synthesised _init global_set pattern and
            // emit a direct _globals[...] = ... assignment for
            // readable output.  Otherwise emit an expression
            // statement.
            if let Some((global, value)) = pick_global_set(expr) {
                let _ = write!(out, "{}_globals[{}] = ", pad, quote_py_string(global));
                emit_expr(out, value, indent);
                out.push('\n');
            } else {
                let _ = write!(out, "{}", pad);
                emit_expr(out, expr, indent);
                out.push('\n');
            }
        }
        // ── SIR16 mutation ──────────────────────────────────────────
        // `Assign` re-binds an already-declared name.  Local/Param/
        // Capture all resolve to a bare identifier; Global writes the
        // module-level `_globals` dict (matching how `_init` and
        // `VarRef::Global` reads are rendered).  Instance/ClassVar/Const
        // belong to features this backend does not accept yet, so they
        // fall through to the panic guard below.
        Stmt::Assign {
            name,
            scope: Scope::Local | Scope::Param | Scope::Capture,
            value,
            ..
        } => {
            let _ = write!(out, "{}{} = ", pad, sanitize_ident(name));
            emit_expr(out, value, indent);
            out.push('\n');
        }
        Stmt::Assign {
            name,
            scope: Scope::Global,
            value,
            ..
        } => {
            let _ = write!(out, "{}_globals[{}] = ", pad, quote_py_string(name));
            emit_expr(out, value, indent);
            out.push('\n');
        }
        // `seq[index] = value`
        Stmt::SeqSet {
            seq, index, value, ..
        } => {
            out.push_str(&pad);
            emit_expr(out, seq, indent);
            out.push('[');
            emit_expr(out, index, indent);
            out.push_str("] = ");
            emit_expr(out, value, indent);
            out.push('\n');
        }
        // `map[key] = value`
        Stmt::MapSet {
            map, key, value, ..
        } => {
            out.push_str(&pad);
            emit_expr(out, map, indent);
            out.push('[');
            emit_expr(out, key, indent);
            out.push_str("] = ");
            emit_expr(out, value, indent);
            out.push('\n');
        }
        // ── SIR16 loops ─────────────────────────────────────────────
        // `while _sir_truthy(cond):` — the test routes through SIR
        // truthiness (only `False`/`None` are falsy), never Python's.
        Stmt::While { cond, body, .. } => {
            out.push_str(&pad);
            out.push_str("while _sir_truthy(");
            emit_expr(out, cond, indent);
            out.push_str("):\n");
            emit_block_as_stmts(out, body, indent + 1);
        }
        // `for var in range(start, stop, step):` — Python's `range` is
        // already half-open (`stop` exclusive) and direction-aware (a
        // negative `step` counts down), matching SIR `ForRange`.
        Stmt::ForRange {
            var,
            start,
            stop,
            step,
            body,
            ..
        } => {
            let _ = write!(out, "{}for {} in range(", pad, sanitize_ident(var));
            emit_expr(out, start, indent);
            out.push_str(", ");
            emit_expr(out, stop, indent);
            out.push_str(", ");
            emit_expr(out, step, indent);
            out.push_str("):\n");
            emit_block_as_stmts(out, body, indent + 1);
        }
        // `for var in iter:` — iterate a Seq.
        Stmt::ForEach {
            var, iter, body, ..
        } => {
            let _ = write!(out, "{}for {} in ", pad, sanitize_ident(var));
            emit_expr(out, iter, indent);
            out.push_str(":\n");
            emit_block_as_stmts(out, body, indent + 1);
        }
        // ── SIR17 scopes (assignment) ───────────────────────────────
        // `@x = v` → current-self instance-variable write via the OOP
        // runtime (no native `self` — methods are receiver-less).
        Stmt::Assign {
            name,
            scope: Scope::Instance,
            value,
            ..
        } => {
            let _ = write!(out, "{}_sir_oop_ivar_set({}, ", pad, quote_py_string(name));
            emit_expr(out, value, indent);
            out.push_str(")\n");
        }
        // `@@x = v` → class-variable store write.
        Stmt::Assign {
            name,
            scope: Scope::ClassVar,
            value,
            ..
        } => {
            let _ = write!(out, "{}_sir_oop_cvar_set({}, ", pad, quote_py_string(name));
            emit_expr(out, value, indent);
            out.push_str(")\n");
        }
        // `CONST = v` → an ordinary module-level binding; reads elsewhere
        // emit the bare identifier (see `emit_var_ref`).
        Stmt::Assign {
            name,
            scope: Scope::Const,
            value,
            ..
        } => {
            let _ = write!(out, "{}{} = ", pad, sanitize_ident(name));
            emit_expr(out, value, indent);
            out.push('\n');
        }
        // `Assign` to a builtin is never produced by any frontend (you
        // cannot rebind `+`); a validated module never reaches here.
        Stmt::Assign {
            scope: Scope::Builtin,
            span,
            ..
        } => {
            panic!(
                "python backend reached an assign to a Builtin-scoped name at {} — invalid SIR",
                span
            );
        }
        // ── SIR17 class / module / singleton declarations ───────────
        // The Ruby→SIR frontend hoists method `def`s to top-level
        // functions, so a `ClassDef` body carries only its non-`def`
        // statements (constant / class-variable assigns).  We register
        // the class in the OOP runtime (for ancestry-aware `is_a?`) and
        // emit the body statements in source order.
        Stmt::ClassDef {
            name,
            superclass,
            body,
            ..
        } => {
            let _ = write!(
                out,
                "{}_sir_oop_define_class({}, ",
                pad,
                quote_py_string(name)
            );
            match superclass {
                Some(sup) => out.push_str(&quote_py_string(sup)),
                None => out.push_str("None"),
            }
            out.push_str(")\n");
            for st in body {
                emit_stmt(out, st, indent);
            }
        }
        // A module is a namespace with no superclass; register it so it
        // can participate in `is_a?`/ancestry, then emit its body.
        Stmt::ModuleDef { name, body, .. } => {
            let _ = writeln!(
                out,
                "{}_sir_oop_define_class({}, None)",
                pad,
                quote_py_string(name)
            );
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
        // `begin … rescue … ensure … end` → native `try: … except Exception
        // as __exc: … finally: …`.  Python's `except` matches by Python class
        // while Ruby has an ordered list of typed `rescue` clauses, so the
        // handler catches broadly and the body is an `if`/`elif` chain asking
        // the exception runtime `rescue_matches(__exc, [class names])` per
        // clause in source order; a `rescue Foo => e` binds `e = __exc`; if no
        // clause matches the original exception is re-`raise`d (Ruby's
        // "propagate when unrescued").  `ensure_body` → a `finally:` block.
        Stmt::TryCatch {
            body,
            rescues,
            ensure_body,
            ..
        } => {
            let _ = writeln!(out, "{}try:", pad);
            emit_stmt_list(out, body, indent + 1);
            if !rescues.is_empty() {
                // Catch broadly (matching the TS backend's catch-all) so a
                // native Python error can still be matched by `rescue
                // StandardError`; the dispatch + re-raise is in the body.
                let _ = writeln!(out, "{}except Exception as __exc:", pad);
                let ipad = indent_str(indent + 1);
                for (i, r) in rescues.iter().enumerate() {
                    let mut types = String::from("[");
                    for (j, t) in r.exception_types.iter().enumerate() {
                        if j > 0 {
                            types.push_str(", ");
                        }
                        types.push_str(&quote_py_string(t));
                    }
                    types.push(']');
                    let kw = if i == 0 { "if" } else { "elif" };
                    let _ = writeln!(
                        out,
                        "{}{} _sir_exc_rescue_matches(__exc, {}):",
                        ipad, kw, types
                    );
                    // `rescue Foo => e` binds the caught value as a local.
                    if let Some(bind) = &r.binding {
                        let bpad = indent_str(indent + 2);
                        let _ = writeln!(out, "{}{} = __exc", bpad, sanitize_ident(bind));
                        emit_stmt_list_allow_only_value(out, &r.body, indent + 2, false);
                    } else {
                        emit_stmt_list(out, &r.body, indent + 2);
                    }
                }
                // No clause matched → propagate the original exception.
                let _ = writeln!(out, "{}else:", ipad);
                let bpad = indent_str(indent + 2);
                let _ = writeln!(out, "{}raise", bpad);
            }
            if let Some(ens) = ensure_body {
                let _ = writeln!(out, "{}finally:", pad);
                emit_stmt_list(out, ens, indent + 1);
            }
        }
        // SIR22 `target[indices...] = value`. This backend does not declare
        // `Feature::NDArrays`/`Feature::MatrixOps` in `ACCEPTED_FEATURES`
        // (see `lib.rs`), so `Backend::check_module` rejects any module
        // using this node before it ever reaches emission — a validated
        // module never reaches this arm.
        Stmt::IndexSet { span, .. } => {
            panic!(
                "python backend reached a deferred SIR22 array/matrix statement (index-set) at {} — not accepted yet",
                span
            );
        }
    }
}

/// Emit a bare statement list (as carried by `TryCatch` bodies / rescue
/// clauses / `ensure`) at `indent`, emitting `pass` when empty so the Python
/// block is non-empty.
fn emit_stmt_list(out: &mut String, stmts: &[Stmt], indent: usize) {
    if stmts.is_empty() {
        let _ = writeln!(out, "{}pass", indent_str(indent));
        return;
    }
    for s in stmts {
        emit_stmt(out, s, indent);
    }
}

/// Like [`emit_stmt_list`] but the caller has already emitted at least one line
/// at `indent` (e.g. a rescue binding), so an *empty* list must NOT add a
/// `pass`.  `_emit_pass_if_empty` is honoured only when the caller passes
/// `true`.
fn emit_stmt_list_allow_only_value(
    out: &mut String,
    stmts: &[Stmt],
    indent: usize,
    emit_pass_if_empty: bool,
) {
    if stmts.is_empty() {
        if emit_pass_if_empty {
            let _ = writeln!(out, "{}pass", indent_str(indent));
        }
        return;
    }
    for s in stmts {
        emit_stmt(out, s, indent);
    }
}

fn pick_global_set(e: &Expr) -> Option<(&str, &Expr)> {
    if let Expr::BuiltinCall { name, args, .. } = e {
        if name == "global_set" && args.len() == 2 {
            if let Expr::SymLit { name: gn, .. } = &args[0] {
                return Some((gn.as_str(), &args[1]));
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
            out.push_str(if *value { "True" } else { "False" });
        }
        Expr::NilLit { .. } => out.push_str("None"),
        Expr::SymLit { name, .. } => {
            let _ = write!(out, "_sir_intern({})", quote_py_string(name));
        }
        Expr::StrLit { value, .. } => {
            out.push_str(&quote_py_string(value));
        }
        Expr::VarRef { name, scope, .. } => emit_var_ref(out, name, *scope),
        Expr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            // Python ternary: (then if cond else else)
            out.push('(');
            emit_block_as_expr(out, then_branch, indent);
            out.push_str(" if _sir_truthy(");
            emit_expr(out, cond, indent);
            out.push_str(") else ");
            emit_block_as_expr(out, else_branch, indent);
            out.push(')');
        }
        Expr::Block(b) => emit_block_as_expr(out, b, indent),
        Expr::DirectCall { fn_name, args, .. } => {
            let _ = write!(out, "{}(", function_emit_name(fn_name));
            emit_args(out, args, indent);
            out.push(')');
        }
        Expr::IndirectCall { target, args, .. } => {
            out.push_str("_sir_apply(");
            emit_expr(out, target, indent);
            out.push_str(", [");
            emit_args(out, args, indent);
            out.push_str("])");
        }
        Expr::BuiltinCall { name, args, .. } => emit_builtin_call(out, name, args, indent),
        Expr::MakeClosure {
            fn_name, captures, ..
        } => {
            let _ = write!(out, "_sir_make_closure({}, [", function_emit_name(fn_name));
            for (i, c) in captures.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                emit_expr(out, &c.value, indent);
            }
            out.push_str("])");
        }
        Expr::Intrinsic { name, span, .. } => {
            panic!(
                "emit reached an Intrinsic `{}` at {} — backend should have rejected it",
                name, span
            );
        }
        // ── SIR16 expression kinds — native Python ──────────────────
        Expr::FloatLit { value, .. } => {
            // `{:?}` keeps a decimal point (e.g. `3.0`, `3.14`) so the
            // literal stays a float, not an int.
            let _ = write!(out, "{:?}", value);
        }
        Expr::SeqLit { items, .. } => {
            out.push('[');
            emit_args(out, items, indent);
            out.push(']');
        }
        Expr::SeqIndex { seq, index, .. } => {
            emit_expr(out, seq, indent);
            out.push('[');
            emit_expr(out, index, indent);
            out.push(']');
        }
        Expr::SeqLen { seq, .. } => {
            out.push_str("len(");
            emit_expr(out, seq, indent);
            out.push(')');
        }
        Expr::MapLit { entries, .. } => {
            out.push('{');
            for (i, entry) in entries.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                emit_expr(out, &entry.key, indent);
                out.push_str(": ");
                emit_expr(out, &entry.value, indent);
            }
            out.push('}');
        }
        Expr::MapGet { map, key, .. } => {
            emit_expr(out, map, indent);
            out.push('[');
            emit_expr(out, key, indent);
            out.push(']');
        }
        // Short-circuit: a lambda keeps the rhs unevaluated until the
        // lhs decides, and routes the test through SIR truthiness (only
        // `False`/`nil` are falsy — not `0`/`""`/`[]`).  The lambda
        // param `__l` gives each occurrence its own scope, so nested
        // `&&`/`||` never collide.
        Expr::LogicalAnd { lhs, rhs, .. } => {
            out.push_str("(lambda __l: (");
            emit_expr(out, rhs, indent);
            out.push_str(") if _sir_truthy(__l) else __l)(");
            emit_expr(out, lhs, indent);
            out.push(')');
        }
        Expr::LogicalOr { lhs, rhs, .. } => {
            out.push_str("(lambda __l: __l if _sir_truthy(__l) else (");
            emit_expr(out, rhs, indent);
            out.push_str("))(");
            emit_expr(out, lhs, indent);
            out.push(')');
        }
        // String interpolation: every part rendered through the SIR
        // display helper (a string part renders to itself) and joined.
        Expr::StrConcat { parts, .. } => {
            out.push('(');
            if parts.is_empty() {
                out.push_str("\"\"");
            }
            for (i, p) in parts.iter().enumerate() {
                if i > 0 {
                    out.push_str(" + ");
                }
                out.push_str("_sir_to_display(");
                emit_expr(out, p, indent);
                out.push(')');
            }
            out.push(')');
        }
        // KW2 keyword argument.  A `KeywordArg { name, value }` reaches the
        // backend only inside a call's `args` vec, after all positionals (the
        // validator enforces both).  Python's native call syntax spells it
        // `name=value` (`f(1, b=2)`), which binds `value` to the callee's
        // keyword-only parameter `name`.  The name is a bare identifier here —
        // not a string literal — so it is emitted through `sanitize_ident`,
        // and the value lowers through the ordinary expression emitter.
        Expr::KeywordArg { name, value, .. } => {
            out.push_str(&sanitize_ident(name));
            out.push('=');
            emit_expr(out, value, indent);
        }
        // SIR22 array/matrix expressions. This backend does not declare
        // `Feature::NDArrays`/`Feature::MatrixOps` in `ACCEPTED_FEATURES`
        // (see `lib.rs`), so `Backend::check_module` rejects any module
        // using these nodes before emission is ever reached — matching the
        // `Expr::Intrinsic` guard above, a validated module never reaches
        // this arm.
        Expr::ArrayLit { .. }
        | Expr::Range { .. }
        | Expr::MatMul { .. }
        | Expr::ElementwiseOp { .. }
        | Expr::Transpose { .. }
        | Expr::IndexGet { .. }
        // SIR26 `Convert` — `Conversions` not accepted; unreachable in a
        // validated module.
        | Expr::Convert { .. } => {
            panic!(
                "python backend reached a deferred SIR22/SIR26 expression ({}) at {} — not accepted yet",
                e.kind_name(),
                e.span()
            );
        }
        // SIR23 symbolic-expression/pattern nodes. This backend does not
        // declare `Feature::SymbolicExpr`/`Feature::PatternMatching` in
        // `ACCEPTED_FEATURES` (see `lib.rs`), so `Backend::check_module`
        // rejects any module using these nodes before emission is ever
        // reached — matching the SIR22/SIR26 guard above.
        Expr::SymSymbol { .. }
        | Expr::SymRational { .. }
        | Expr::SymApply { .. }
        | Expr::SymPatternBlank { .. }
        | Expr::SymPatternNamed { .. }
        | Expr::SymRule { .. }
        | Expr::SymReplaceAll { .. } => {
            panic!(
                "python backend reached a deferred SIR23 expression ({}) at {} — not accepted yet",
                e.kind_name(),
                e.span()
            );
        }
    }
}

fn emit_var_ref(out: &mut String, name: &str, scope: Scope) {
    match scope {
        Scope::Local | Scope::Param | Scope::Capture => {
            out.push_str(&sanitize_ident(name));
        }
        Scope::Global => {
            let _ = write!(out, "_sir_global_get_static({})", quote_py_string(name));
        }
        Scope::Builtin => {
            let _ = write!(out, "_sir_builtin_closure({})", quote_py_string(name));
        }
        // SIR17 scopes — read via the OOP runtime, since the Ruby→SIR
        // frontend hoists methods to receiver-less top-level functions
        // (no native `self` to read attributes from).
        Scope::Instance => {
            // `@x` → current-self instance-variable read.
            let _ = write!(out, "_sir_oop_ivar_get({})", quote_py_string(name));
        }
        Scope::ClassVar => {
            // `@@x` → class-variable store read.
            let _ = write!(out, "_sir_oop_cvar_get({})", quote_py_string(name));
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
        emit_arg(out, a, indent);
    }
}

/// Emit one argument / sequence element, expanding the splat markers into
/// Python's native spread syntax.
///
/// Ruby `*x` / `**x` reach the backend as `BuiltinCall("splat", [x])` /
/// `BuiltinCall("double_splat", [x])` — sitting as a trailing call argument or
/// as an array element.  Python has faithful native forms for both:
///
/// | SIR marker (Ruby) | Python emitted | meaning |
/// |---|---|---|
/// | `splat` (`f(*a)`, `[1, *a, 3]`) | `*a` | splice a sequence's items |
/// | `double_splat` (`f(**h)`) | `**h` | splice a map's entries as kwargs |
///
/// Anything that is not a splat marker emits as an ordinary expression.  (A
/// `double_splat` only ever appears in keyword-argument position in the SIR the
/// Ruby frontend produces, so `**h` lands where Python accepts it; it is never
/// emitted into a list literal.)
fn emit_arg(out: &mut String, a: &Expr, indent: usize) {
    if let Expr::BuiltinCall { name, args, .. } = a {
        if name == "splat" && args.len() == 1 {
            out.push('*');
            emit_expr(out, &args[0], indent);
            return;
        }
        if name == "double_splat" && args.len() == 1 {
            out.push_str("**");
            emit_expr(out, &args[0], indent);
            return;
        }
    }
    if try_emit_block_pass(out, a, indent) {
        return;
    }
    emit_expr(out, a, indent);
}

/// Emit a `&expr` block-pass argument that survived frontend normalization
/// (M2).  The Ruby frontend wraps a `&`-prefixed block argument as
/// `BuiltinCall("block_pass", [inner])`.  Q9f unwraps it at *user-method*
/// `DirectCall` sites, but a block-pass to a **method-dispatch** call
/// (`recv.map(&:to_s)`) reaches the backend intact inside the `__method__`
/// envelope.  Two inner shapes matter:
///
/// | inner | emitted | meaning |
/// |---|---|---|
/// | `SymLit("m")` (`&:m`) | `_sir_oop_sym_to_proc(intern("m"))` | `Symbol#to_proc` — a block calling `recv.m(*rest)` |
/// | any other (`&proc`)   | `<inner>` (unwrapped) | the operand already *is* the proc/block value |
///
/// Returns `true` when it handled a `block_pass` envelope (so the caller does
/// not also `emit_expr` it).  A malformed envelope (not exactly one operand)
/// is left for the generic path.
fn try_emit_block_pass(out: &mut String, a: &Expr, indent: usize) -> bool {
    if let Expr::BuiltinCall { name, args, .. } = a {
        if name == "block_pass" && args.len() == 1 {
            if let Expr::SymLit { .. } = &args[0] {
                out.push_str("_sir_oop_sym_to_proc(");
                emit_expr(out, &args[0], indent);
                out.push(')');
            } else {
                emit_expr(out, &args[0], indent);
            }
            return true;
        }
    }
    false
}

fn emit_builtin_call(out: &mut String, name: &str, args: &[Expr], indent: usize) {
    // Reflective method dispatch: the Ruby→SIR frontend lowers
    // `recv.meth(args…)` to `BuiltinCall("__method__", [recv, "meth",
    // args…])`.  Route it through the OOP runtime's `call_method`.  For
    // the class-predicate methods, a `Const`-scoped class operand (e.g.
    // `Integer`) is passed as its *name string* so the predicate works
    // without a binding for the built-in class name.
    if name == "__method__" && args.len() >= 2 {
        if let Expr::StrLit { value: meth, .. } = &args[1] {
            out.push_str("_sir_oop_call_method(");
            emit_expr(out, &args[0], indent);
            let _ = write!(out, ", {}", quote_py_string(meth));
            let is_class_pred = matches!(meth.as_str(), "is_a?" | "kind_of?" | "instance_of?");
            for (i, a) in args[2..].iter().enumerate() {
                out.push_str(", ");
                match a {
                    Expr::VarRef {
                        name: cn,
                        scope: Scope::Const,
                        ..
                    } if is_class_pred && i == 0 => {
                        out.push_str(&quote_py_string(cn));
                    }
                    // A `&:sym` / `&proc` block argument on a dispatched call
                    // (`recv.map(&:to_s)`) survives as a `block_pass` envelope
                    // (Q9f only unwraps these at user-method DirectCalls).
                    _ if try_emit_block_pass(out, a, indent) => {}
                    _ => emit_expr(out, a, indent),
                }
            }
            out.push(')');
            return;
        }
    }
    // OOP object-model builtins (O1).  The Ruby→SIR frontend (O2) emits these
    // for user-defined classes; each routes to the OOP runtime's explicit
    // method-table helpers (never reflection — the C3 RCE lesson).  Class and
    // method names arrive as `StrLit` args and are emitted through the normal
    // expression path (`quote_py_string`), so no source-derived name is ever
    // interpolated raw.
    //
    //   __new__(class, ...ctor_args)          → _sir_oop_call_new(class, args…)
    //   __super__(method, class, ...args)     → _sir_oop_call_super(method, class, args…)
    //   __def_method__(class, method, fn)     → _sir_oop_def_method(class, method, fn)
    //   __def_class_method__(class, meth, fn) → _sir_oop_def_class_method(class, meth, fn)
    //   __self__()                            → _sir_oop_current_self()
    //
    // All args are ordinary SIR `Expr`s (`StrLit` for the names, `MakeClosure`
    // for the method body), so a plain `emit_args` is correct and safe.
    if name == "__new__" && !args.is_empty() {
        out.push_str("_sir_oop_call_new(");
        emit_args(out, args, indent);
        out.push(')');
        return;
    }
    if name == "__super__" && args.len() >= 2 {
        out.push_str("_sir_oop_call_super(");
        emit_args(out, args, indent);
        out.push(')');
        return;
    }
    // Issue #59 — class-method call `Foo.bar(args)` (const receiver) →
    // `_sir_oop_call_class_method("Foo", "bar", args…)`.  The class + method
    // names arrive as `StrLit`s and are emitted via the normal expression path
    // (`quote_py_string`), so no source-derived name is interpolated raw.
    if name == "__class_method__" && args.len() >= 2 {
        out.push_str("_sir_oop_call_class_method(");
        emit_args(out, args, indent);
        out.push(')');
        return;
    }
    if name == "__def_method__" && args.len() == 3 {
        out.push_str("_sir_oop_def_method(");
        emit_args(out, args, indent);
        out.push(')');
        return;
    }
    if name == "__def_class_method__" && args.len() == 3 {
        out.push_str("_sir_oop_def_class_method(");
        emit_args(out, args, indent);
        out.push(')');
        return;
    }
    // Mixin directives (MX2).  `include M` / `extend M` in a class or module
    // body lower to `__include__("Owner", "M")` / `__extend__("Owner", "M")`,
    // both carrying two `StrLit` name args.  They route to the OOP runtime's
    // explicit tables: `include` appends `M` to the owner's included-modules
    // list (consulted by the MRO walk); `extend` copies `M`'s instance methods
    // into the owner's class-method table.  Dispatch stays table-driven — the
    // names are emitted via `quote_py_string`, never interpolated raw (the C3
    // RCE lesson).
    if name == "__include__" && args.len() == 2 {
        out.push_str("_sir_oop_include_module(");
        emit_args(out, args, indent);
        out.push(')');
        return;
    }
    if name == "__extend__" && args.len() == 2 {
        out.push_str("_sir_oop_extend_module(");
        emit_args(out, args, indent);
        out.push(')');
        return;
    }
    if name == "__self__" && args.is_empty() {
        out.push_str("_sir_oop_current_self()");
        return;
    }
    // `case_eq` (M5) — Ruby case-equality `pattern === value`, emitted by a
    // `when` clause for range/regex/literal patterns (the class case lowers to
    // `is_a?` via `__method__` instead).  Routes to the OOP runtime helper,
    // which dispatches Range→membership, Regexp→match, else `==`.
    if name == "case_eq" && args.len() == 2 {
        out.push_str("_sir_oop_case_eq(");
        emit_expr(out, &args[0], indent);
        out.push_str(", ");
        emit_expr(out, &args[1], indent);
        out.push(')');
        return;
    }
    // `raise` → raise a SIR exception via the exception runtime.  The first
    // argument decides the shape:
    //   • a `Const` class name (`raise Foo` / `raise Foo, "msg"`) → the class
    //     name is passed as a *string* (no binding needed for a built-in
    //     class), with the optional message second;
    //   • any other first arg (`raise "msg"`) → an implicit `RuntimeError`
    //     carrying that value as the message (matching Ruby);
    //   • no args (bare `raise`) → a generic re-raise (`RuntimeError`).
    if name == "raise" {
        out.push_str("_sir_exc_raise_error(");
        match args.first() {
            None => {}
            Some(Expr::VarRef {
                name: cn,
                scope: Scope::Const,
                ..
            }) => {
                out.push_str(&quote_py_string(cn));
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
    // Args are `[pattern, flags]` (both string literals); routes to the
    // dedicated package's `compile`, gated by `uses_regex`.
    if name == "regex" {
        out.push_str("_sir_regex_compile(");
        emit_args(out, args, indent);
        out.push(')');
        return;
    }
    // `backtick` (a Ruby `` `cmd` `` literal) → run via the shell runtime,
    // returning the command's stdout.  Gated by `uses_shell`.
    if name == "backtick" {
        out.push_str("_sir_shell_backtick(");
        emit_args(out, args, indent);
        out.push(')');
        return;
    }
    // `range` (a Ruby `a..b` / `a...b` literal) → construct a first-class SIR
    // `Range` via the range runtime.  Args are `[start, stop, exclusive]`
    // (start/stop may be `NilLit` for the begin/endless forms).  Gated by
    // `uses_range`.
    if name == "range" {
        out.push_str("_sir_range(");
        emit_args(out, args, indent);
        out.push(')');
        return;
    }
    // Ruby `&&`/`and` and `||`/`or` lower to `BuiltinCall("and"/"or", [lhs,
    // rhs])`.  They must **short-circuit** (rhs not evaluated when lhs decides
    // it) and use SIR truthiness, so they emit the same truthy-guarded
    // immediately-invoked lambda as `Expr::LogicalAnd`/`LogicalOr` rather than
    // routing through the eager `call_builtin` dispatch (which would evaluate
    // both operands and lose Ruby semantics).
    if name == "and" && args.len() == 2 {
        out.push_str("(lambda __l: (");
        emit_expr(out, &args[1], indent);
        out.push_str(") if _sir_truthy(__l) else __l)(");
        emit_expr(out, &args[0], indent);
        out.push(')');
        return;
    }
    if name == "or" && args.len() == 2 {
        out.push_str("(lambda __l: __l if _sir_truthy(__l) else (");
        emit_expr(out, &args[1], indent);
        out.push_str("))(");
        emit_expr(out, &args[0], indent);
        out.push(')');
        return;
    }
    // `!`/`not` → SIR-truthiness negation, always a bool (never Python's
    // operand-returning `not`).  `-x` (unary minus) → numeric negation.
    if name == "not" && args.len() == 1 {
        out.push_str("(not _sir_truthy(");
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
    // directly (which renders `_sir_make_closure(...)`) rather than routing
    // through the eager `call_builtin` dispatch — there is no separate
    // "lambda" runtime helper to call, the closure already is the result.
    //
    // Proc-vs-lambda arity (Q10g): a `MakeClosure` is proc-lenient by default
    // (`apply` adjusts a block's arguments to its arity), but a Ruby lambda is
    // **strict**.  Wrap the closure in `_sir_as_lambda(...)`, which flips its
    // strict flag so `apply` passes arguments through unadjusted (a mismatch
    // then raises, the analogue of Ruby's `ArgumentError`).
    if name == "lambda" && args.len() == 1 {
        out.push_str("_sir_as_lambda(");
        emit_expr(out, &args[0], indent);
        out.push(')');
        return;
    }
    // `defined?(x)` lowers to `BuiltinCall("defined?", [operand])`.  The single
    // most important Ruby contract here is that `defined?` **never evaluates its
    // operand** — `defined?(expensive_call)` must not call it.  So we inspect
    // the operand's SIR *shape at emit time* and emit a constant description
    // string; the operand expression is never rendered, so it cannot run.
    //
    // | operand shape | emitted | Ruby `defined?` returns |
    // |---|---|---|
    // | local / param / capture VarRef | `"local-variable"` | "local-variable" |
    // | `Const` VarRef                 | `"constant"`       | "constant" |
    // | `Instance` (`@x`) VarRef       | `"instance-variable"` | desc, or nil if unset |
    // | `ClassVar` (`@@x`) VarRef      | `"class variable"` | desc, or nil if unset |
    // | `Global` (`$x`) VarRef         | `"global-variable"`| desc, or nil if unset |
    // | builtin-name VarRef            | `"method"`         | "method" |
    // | `recv.meth` (`__method__` env) | `"method"`         | "method", or nil if absent |
    // | any other expr (literal, call) | `"expression"`     | "expression"/"method"/… |
    //
    // v0 simplification (documented in `code/specs/sir-runtime.md`): for an
    // instance/class/global variable we emit the static description rather than
    // performing the runtime presence check Ruby uses to return `nil` when the
    // variable is unset (the per-concern runtimes expose no presence predicate
    // yet).  Q10h: a method-call operand `recv.meth` (the `__method__` dispatch
    // envelope) now reports `"method"` — Ruby's category when the method
    // resolves — instead of the generic `"expression"`; the runtime
    // respond_to?-presence check that would yield `nil` for an absent method is
    // the documented method-dispatch boundary.  The non-evaluation contract
    // holds for every shape (a constant string is emitted; the operand — and so
    // the receiver and the call — is never rendered).
    if name == "defined?" && args.len() == 1 {
        let desc = match &args[0] {
            Expr::VarRef { scope, .. } => match scope {
                Scope::Local | Scope::Param | Scope::Capture => "local-variable",
                Scope::Const => "constant",
                Scope::Instance => "instance-variable",
                Scope::ClassVar => "class variable",
                Scope::Global => "global-variable",
                Scope::Builtin => "method",
            },
            Expr::BuiltinCall { name: inner, .. } if inner == "__method__" => "method",
            _ => "expression",
        };
        out.push_str(&quote_py_string(desc));
        return;
    }
    let helper = match name {
        "+" => "_sir_plus",
        "-" => "_sir_minus",
        "*" => "_sir_times",
        "/" => "_sir_divide",
        "=" => "_sir_eq",
        "<" => "_sir_lt",
        ">" => "_sir_gt",
        "cons" => "_sir_cons",
        "car" => "_sir_car",
        "cdr" => "_sir_cdr",
        "null?" => "_sir_is_null",
        "pair?" => "_sir_is_pair",
        "number?" => "_sir_is_number",
        "symbol?" => "_sir_is_symbol",
        "print" => "_sir_print",
        // `_sir_puts` is variadic (`sir_puts(*args)`), so `_sir_puts(a, b)`
        // forwards every argument (Ruby `puts a, b`).
        "puts" => "_sir_puts",
        "global_set" => "_sir_global_set",
        "global_get" => "_sir_global_get",
        _ => {
            let _ = write!(out, "_sir_call_builtin({}, [", quote_py_string(name));
            emit_args(out, args, indent);
            out.push_str("])");
            return;
        }
    };
    let _ = write!(out, "{}(", helper);
    emit_args(out, args, indent);
    out.push(')');
}

fn emit_block_as_expr(out: &mut String, b: &Block, indent: usize) {
    if b.stmts.is_empty() {
        emit_expr(out, &b.value, indent);
        return;
    }
    // A block holding a loop in expression position cannot be expressed
    // as a walrus tuple (Python has no multi-statement expression), so
    // it is lifted to a nested `def` (queued in the hoist buffer) and
    // the call site emits `__block_N()`.
    if block_has_loop(b) {
        emit_block_as_lifted_def(out, b, indent);
        return;
    }
    // Otherwise: render the block as a left-to-right tuple of assignment
    // expressions whose last element is the block's value (Python 3.8+
    // walrus form).  `(...)[-1]` yields that last element.
    //
    // - LetBinding `x = e`        → `(x := e)`
    // - Assign{Local} `x = e`     → `(x := e)` (re-bind; same form)
    // - SeqSet `s[i] = v`         → `(s.__setitem__(i, v))` (returns None)
    // - MapSet `m[k] = v`         → `(m.__setitem__(k, v))`
    // - ExprStmt `e`              → `(e)` (value discarded)
    out.push('(');
    for s in &b.stmts {
        match s {
            Stmt::LetBinding { name, value, .. }
            | Stmt::LetStarBinding { name, value, .. }
            | Stmt::Assign {
                name,
                scope: Scope::Local | Scope::Param | Scope::Capture,
                value,
                ..
            } => {
                let _ = write!(out, "({} := ", sanitize_ident(name));
                emit_expr(out, value, indent);
                out.push_str("), ");
            }
            Stmt::Assign {
                name,
                scope: Scope::Global,
                value,
                ..
            } => {
                let _ = write!(out, "(_globals.__setitem__({}, ", quote_py_string(name));
                emit_expr(out, value, indent);
                out.push_str(")), ");
            }
            Stmt::ExprStmt { expr, .. } => {
                out.push('(');
                emit_expr(out, expr, indent);
                out.push_str("), ");
            }
            Stmt::SeqSet {
                seq, index, value, ..
            } => {
                out.push('(');
                emit_expr(out, seq, indent);
                out.push_str(".__setitem__(");
                emit_expr(out, index, indent);
                out.push_str(", ");
                emit_expr(out, value, indent);
                out.push_str(")), ");
            }
            Stmt::MapSet {
                map, key, value, ..
            } => {
                out.push('(');
                emit_expr(out, map, indent);
                out.push_str(".__setitem__(");
                emit_expr(out, key, indent);
                out.push_str(", ");
                emit_expr(out, value, indent);
                out.push_str(")), ");
            }
            // `@x = v` / `@@x = v` → OOP-store writes (return the value,
            // so they compose in the walrus tuple).
            Stmt::Assign {
                name,
                scope: Scope::Instance,
                value,
                ..
            } => {
                let _ = write!(out, "(_sir_oop_ivar_set({}, ", quote_py_string(name));
                emit_expr(out, value, indent);
                out.push_str(")), ");
            }
            Stmt::Assign {
                name,
                scope: Scope::ClassVar,
                value,
                ..
            } => {
                let _ = write!(out, "(_sir_oop_cvar_set({}, ", quote_py_string(name));
                emit_expr(out, value, indent);
                out.push_str(")), ");
            }
            // `CONST = v` → walrus binding (the const name is a plain
            // identifier, so `(NAME := v)` both binds and yields it).
            Stmt::Assign {
                name,
                scope: Scope::Const,
                value,
                ..
            } => {
                let _ = write!(out, "({} := ", sanitize_ident(name));
                emit_expr(out, value, indent);
                out.push_str("), ");
            }
            // Loops were diverted to the lifted-def path above.
            Stmt::While { span, .. } | Stmt::ForRange { span, .. } | Stmt::ForEach { span, .. } => {
                panic!(
                    "python backend (walrus path) reached a loop at {} — should have been lifted",
                    span
                );
            }
            // `Assign` to a builtin, and class/module/singleton/try
            // declarations, have no walrus-expression form; the former is
            // invalid SIR and the latter are emitted only in statement
            // position (a block-as-expr never contains them).
            Stmt::Assign { span, .. }
            | Stmt::ClassDef { span, .. }
            | Stmt::ModuleDef { span, .. }
            | Stmt::SingletonClassDef { span, .. }
            | Stmt::TryCatch { span, .. } => {
                panic!(
                    "python backend (walrus path) reached an unwalrusable statement at {}",
                    span
                );
            }
            // SIR22 `target[indices...] = value`. This backend does not
            // declare `Feature::NDArrays`/`Feature::MatrixOps` in
            // `ACCEPTED_FEATURES` (see `lib.rs`), so `Backend::check_module`
            // rejects any module using this node before emission is ever
            // reached — a validated module never reaches this arm.
            Stmt::IndexSet { span, .. } => {
                panic!(
                    "python backend reached a deferred SIR22 array/matrix statement (index-set) at {} — not accepted yet",
                    span
                );
            }
        }
    }
    emit_expr(out, &b.value, indent);
    out.push_str(")[-1]");
}

/// Lift a statement-bearing block to a nested `def __block_N(): …` whose
/// source is queued in the hoist buffer; emit `__block_N()` at the call
/// site.  Used for expression-position blocks containing a loop, which
/// cannot be inlined as a Python expression.
fn emit_block_as_lifted_def(out: &mut String, b: &Block, indent: usize) {
    let name = fresh_block_name();
    let pad = indent_str(indent);
    let mut def = String::new();
    let _ = writeln!(def, "{}def {}():", pad, name);
    // Names re-bound (Assign{Local}) but bound in an enclosing scope must
    // be declared `nonlocal` so the assignment mutates the outer binding
    // rather than shadowing it.  Names introduced by a let / loop var in
    // this block are local and excluded.
    let nonlocals = collect_nonlocals(b);
    let inner = indent + 1;
    let inner_pad = indent_str(inner);
    for n in &nonlocals {
        let _ = writeln!(def, "{}nonlocal {}", inner_pad, sanitize_ident(n));
    }
    // Body statements, then `return <value>` — emitted via the normal
    // statement path so any further nested loops hoist correctly.
    emit_block_as_stmts_with_return(&mut def, b, inner);
    HOIST.with(|h| h.borrow_mut().push(def));
    let _ = write!(out, "{}()", name);
}

/// Emit a block's statements followed by `return <value>` at `indent`
/// (used for a lifted nested `def` body).
fn emit_block_as_stmts_with_return(out: &mut String, b: &Block, indent: usize) {
    for s in &b.stmts {
        emit_stmt(out, s, indent);
    }
    let pad = indent_str(indent);
    let mut tmp = String::new();
    let _ = write!(tmp, "{}return ", pad);
    emit_expr(&mut tmp, &b.value, indent);
    tmp.push('\n');
    flush_hoist(out);
    out.push_str(&tmp);
}

/// Emit a block in **statement context** — used for loop bodies, whose
/// trailing value is discarded.  Each statement is emitted in order; a
/// non-nil trailing value becomes an expression statement so its side
/// effect still fires.  An otherwise-empty body emits `pass`.
fn emit_block_as_stmts(out: &mut String, b: &Block, indent: usize) {
    let pad = indent_str(indent);
    let has_value = !matches!(b.value, Expr::NilLit { .. });
    if b.stmts.is_empty() && !has_value {
        let _ = writeln!(out, "{}pass", pad);
        return;
    }
    for s in &b.stmts {
        emit_stmt(out, s, indent);
    }
    if has_value {
        let mut tmp = String::new();
        tmp.push_str(&pad);
        emit_expr(&mut tmp, &b.value, indent);
        tmp.push('\n');
        flush_hoist(out);
        out.push_str(&tmp);
    }
}

/// True if any top-level statement of `b` is a loop.  Loops cannot be
/// expressed as walrus tuples, so such a block must be lifted to a
/// nested `def` when it appears in expression position.  (Nested blocks
/// handle their own loops recursively, so this check is shallow.)
fn block_has_loop(b: &Block) -> bool {
    b.stmts.iter().any(|s| {
        matches!(
            s,
            // A `try`/`except`/`finally` is a compound statement that, like a
            // loop, cannot be expressed as a walrus tuple — so a block holding
            // one must also be lifted to a nested `def` in expression position.
            Stmt::While { .. }
                | Stmt::ForRange { .. }
                | Stmt::ForEach { .. }
                | Stmt::TryCatch { .. }
        )
    })
}

/// Collect the names a lifted nested `def` must declare `nonlocal`: every
/// `Assign{Local}` target reachable through the block's own statements
/// and its inline loop bodies, minus names introduced locally (by a
/// let / let* binding or as a loop variable).  Expression-position
/// sub-blocks are *not* traversed — they become their own nested defs.
fn collect_nonlocals(b: &Block) -> BTreeSet<String> {
    let mut assigned = BTreeSet::new();
    let mut bound = BTreeSet::new();
    collect_nonlocals_block(b, &mut assigned, &mut bound);
    assigned.difference(&bound).cloned().collect()
}

fn collect_nonlocals_block(
    b: &Block,
    assigned: &mut BTreeSet<String>,
    bound: &mut BTreeSet<String>,
) {
    for s in &b.stmts {
        match s {
            Stmt::LetBinding { name, .. } | Stmt::LetStarBinding { name, .. } => {
                bound.insert(name.clone());
            }
            Stmt::Assign {
                name,
                scope: Scope::Local | Scope::Param | Scope::Capture,
                ..
            } => {
                assigned.insert(name.clone());
            }
            Stmt::While { body, .. } => collect_nonlocals_block(body, assigned, bound),
            Stmt::ForRange { var, body, .. } | Stmt::ForEach { var, body, .. } => {
                bound.insert(var.clone());
                collect_nonlocals_block(body, assigned, bound);
            }
            // A try/rescue/ensure carries bare statement lists; descend into
            // each so an outer local reassigned inside the `begin` is declared
            // `nonlocal` in the lifted def.  A rescue binding (`=> e`) is a
            // freshly-introduced local, so it counts as `bound`.
            Stmt::TryCatch {
                body,
                rescues,
                ensure_body,
                ..
            } => {
                let synthetic = Block {
                    stmts: body.clone(),
                    value: Expr::NilLit {
                        span: s.span().clone(),
                    },
                    span: s.span().clone(),
                };
                collect_nonlocals_block(&synthetic, assigned, bound);
                for r in rescues {
                    if let Some(bind) = &r.binding {
                        bound.insert(bind.clone());
                    }
                    let rb = Block {
                        stmts: r.body.clone(),
                        value: Expr::NilLit {
                            span: s.span().clone(),
                        },
                        span: s.span().clone(),
                    };
                    collect_nonlocals_block(&rb, assigned, bound);
                }
                if let Some(ens) = ensure_body {
                    let eb = Block {
                        stmts: ens.clone(),
                        value: Expr::NilLit {
                            span: s.span().clone(),
                        },
                        span: s.span().clone(),
                    };
                    collect_nonlocals_block(&eb, assigned, bound);
                }
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Fresh-name and arity helpers
// ---------------------------------------------------------------------------

thread_local! {
    static BLOCK_COUNTER: RefCell<usize> = const { RefCell::new(0) };
    static FN_ARITY: RefCell<HashMap<String, usize>> = RefCell::new(HashMap::new());
    /// Pending nested-`def` sources, awaiting flush before the current
    /// statement (see [`flush_hoist`]).
    static HOIST: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

fn fresh_block_name() -> String {
    BLOCK_COUNTER.with(|c| {
        let mut c = c.borrow_mut();
        let n = *c;
        *c += 1;
        format!("__block_{}", n)
    })
}

// ---------------------------------------------------------------------------
// Lexical helpers
// ---------------------------------------------------------------------------

fn indent_str(level: usize) -> String {
    "    ".repeat(level)
}

/// Sanitize a SIR identifier for Python.
///
/// Python identifiers match `[A-Za-z_][A-Za-z0-9_]*` plus Unicode
/// letters; we restrict to ASCII for simplicity.  Reserved words
/// get an underscore suffix; other invalid characters are encoded
/// as `_<hex>` forms.  Empty input yields `"_sir_empty"`.
pub fn sanitize_ident(s: &str) -> String {
    if s.is_empty() {
        return "_sir_empty".to_string();
    }
    if is_valid_py_ident(s) && !is_python_keyword(s) {
        return s.to_string();
    }
    if is_python_keyword(s) {
        return format!("{}_", s);
    }
    let mut out = String::with_capacity(s.len() + 4);
    out.push('_');
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch);
        } else {
            let _ = write!(out, "_{:x}", ch as u32);
        }
    }
    out
}

fn is_valid_py_ident(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let mut chars = s.chars();
    let first = chars.next().unwrap();
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    for ch in chars {
        if !(ch.is_ascii_alphanumeric() || ch == '_') {
            return false;
        }
    }
    true
}

fn is_python_keyword(s: &str) -> bool {
    matches!(
        s,
        "False"
            | "None"
            | "True"
            | "and"
            | "as"
            | "assert"
            | "async"
            | "await"
            | "break"
            | "class"
            | "continue"
            | "def"
            | "del"
            | "elif"
            | "else"
            | "except"
            | "finally"
            | "for"
            | "from"
            | "global"
            | "if"
            | "import"
            | "in"
            | "is"
            | "lambda"
            | "nonlocal"
            | "not"
            | "or"
            | "pass"
            | "raise"
            | "return"
            | "try"
            | "while"
            | "with"
            | "yield"
            | "match"
            | "case"
    )
}

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

fn quote_py_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\x{:02x}", c as u32);
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
    fn sanitize_idents() {
        assert_eq!(sanitize_ident("hello"), "hello");
        assert_eq!(sanitize_ident("def"), "def_");
        assert_eq!(sanitize_ident("class"), "class_");
        assert_eq!(sanitize_ident(""), "_sir_empty");
        let r = sanitize_ident("null?");
        assert!(r.starts_with('_'));
        assert!(r.contains("null"));
    }

    #[test]
    fn quote_py_string_basic() {
        assert_eq!(quote_py_string("hi"), r#""hi""#);
        assert_eq!(quote_py_string("a\"b\\c"), r#""a\"b\\c""#);
    }

    #[test]
    fn quote_py_string_control_char_escape() {
        let q = quote_py_string("\u{0001}");
        assert!(q.contains(r"\x01"), "got: {}", q);
    }

    #[test]
    fn sanitize_comment_strips_terminators() {
        let r = sanitize_comment("a\nb");
        assert!(!r.contains('\n'));
        let r2 = sanitize_comment(&"x\u{2028}y\u{2029}z".to_string());
        assert!(!r2.contains('\u{2028}'));
        assert!(!r2.contains('\u{2029}'));
    }

    #[test]
    fn function_emit_name_renames_main() {
        assert_eq!(function_emit_name("main"), "_sir_user_main");
        assert_eq!(function_emit_name("id"), "id");
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
            params: vec![Param {
                name: "x".into(),
                sir_type: None,
                kind: ParamKind::Required,
                default: None,
                span: s(),
            }],
            return_type: None,
            captures: vec![],
            body,
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        };
        let mut out = String::new();
        emit_function(&mut out, &f);
        assert!(out.contains("def id(x):"));
        assert!(out.contains("return x"));
    }

    #[test]
    fn emit_variadic_params_native_star_forms() {
        // M3: `def f(a, *rest, **opts); end` → Python `def f(a, *rest, **opts):`
        // — both variadic kinds have faithful native Python forms.
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
            body: Block {
                stmts: vec![],
                value: Expr::NilLit { span: s() },
                span: s(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        };
        let mut out = String::new();
        emit_function(&mut out, &f);
        assert!(out.contains("def f(a, *rest, **opts):"), "got: {out}");
    }

    #[test]
    fn emit_default_param_uses_sentinel_and_prologue() {
        // P2c: `def f(a, b = a + 1); a + b; end`.  `b` is defaulted, so the
        // signature must bind it to the sentinel (`b=_SIR_MISSING`) and the body
        // must open with the resolve-prologue that rewrites a still-sentinel `b`
        // to `a + 1` — emitted in the body, where the earlier param `a` is in
        // scope (call-time, param-referencing default semantics).
        let default_b = Expr::BuiltinCall {
            name: "+".into(),
            args: vec![
                Expr::VarRef {
                    name: "a".into(),
                    scope: Scope::Param,
                    span: s(),
                },
                Expr::IntLit {
                    value: 1,
                    span: s(),
                },
            ],
            effects: EffectSet::PURE,
            span: s(),
        };
        let body = Block {
            stmts: vec![],
            value: Expr::BuiltinCall {
                name: "+".into(),
                args: vec![
                    Expr::VarRef {
                        name: "a".into(),
                        scope: Scope::Param,
                        span: s(),
                    },
                    Expr::VarRef {
                        name: "b".into(),
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
                    name: "b".into(),
                    sir_type: None,
                    kind: ParamKind::Required,
                    default: Some(Box::new(default_b)),
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
        let mut out = String::new();
        emit_function(&mut out, &f);
        // Signature: `a` plain, `b` defaulted to the sentinel.
        assert!(out.contains("def f(a, b=_SIR_MISSING):"), "got:\n{out}");
        // Prologue: sentinel check + rebind to the param-referencing default.
        assert!(out.contains("    if b is _SIR_MISSING:"), "got:\n{out}");
        assert!(out.contains("        b = _sir_plus(a, 1)"), "got:\n{out}");
        // The prologue precedes the body's `return`.
        let prologue_at = out.find("if b is _SIR_MISSING:").unwrap();
        let return_at = out.find("return ").unwrap();
        assert!(
            prologue_at < return_at,
            "prologue must precede return; got:\n{out}"
        );
        // A param with no default must NOT gain a sentinel default.
        assert!(!out.contains("a=_SIR_MISSING"), "got:\n{out}");
    }

    #[test]
    fn emit_module_minimal() {
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
                    value: Expr::IntLit {
                        value: 42,
                        span: s(),
                    },
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
        assert!(out.contains("# Generated by semantic-ir-to-python"));
        assert!(out.contains("from coding_adventures_sir_runtime_core import"));
        assert!(out.contains("def _sir_user_main():"));
        assert!(out.contains("return 42"));
        assert!(out.contains("_sir_user_main()"));
        // A non-Ruby module must NOT emit the display-convention setter.
        assert!(
            !out.contains("_sir_set_display_convention"),
            "twig module must keep the default Lisp display; got:\n{out}"
        );
    }

    #[test]
    fn emit_display_convention_ruby_selects_ruby_booleans() {
        // A Ruby-sourced module emits the display-convention setter once so
        // `puts true` renders `true`; a non-Ruby module (above) does not.
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
                    value: Expr::NilLit { span: s() },
                    span: s(),
                },
                effects: EffectSet::PURE,
                metadata: Metadata::new(),
                span: s(),
            }],
            globals: vec![],
            metadata: Metadata::new()
                .with_source_language("ruby")
                .with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
            span: s(),
        };
        let out = emit_module(&m);
        assert!(
            out.contains("_sir_set_display_convention(\"ruby\")"),
            "ruby module must select the Ruby display convention; got:\n{out}"
        );
    }

    #[test]
    fn emit_block_walrus_strategy() {
        // (let ((x 1)) (+ x 2))
        let b = Block {
            stmts: vec![Stmt::LetBinding {
                name: "x".into(),
                sir_type: None,
                value: Expr::IntLit {
                    value: 1,
                    span: s(),
                },
                span: s(),
            }],
            value: Expr::BuiltinCall {
                name: "+".into(),
                args: vec![
                    Expr::VarRef {
                        name: "x".into(),
                        scope: Scope::Local,
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
        let mut out = String::new();
        emit_block_as_expr(&mut out, &b, 0);
        // Walrus form: ((x := 1), _sir_plus(x, 2))[-1]
        assert!(out.contains("(x := 1)"));
        assert!(out.contains("_sir_plus(x, 2)"));
        assert!(out.ends_with(")[-1]"));
    }

    // M2 — `&:sym` symbol-to-proc on a method-dispatch call.

    fn method_call(recv: Expr, meth: &str, extra: Vec<Expr>) -> Expr {
        let mut args = vec![
            recv,
            Expr::StrLit {
                value: meth.into(),
                span: s(),
            },
        ];
        args.extend(extra);
        Expr::BuiltinCall {
            name: "__method__".into(),
            args,
            effects: EffectSet::PURE,
            span: s(),
        }
    }

    fn block_pass(inner: Expr) -> Expr {
        Expr::BuiltinCall {
            name: "block_pass".into(),
            args: vec![inner],
            effects: EffectSet::PURE,
            span: s(),
        }
    }

    // ── O1: OOP object-model builtin emit arms ───────────────────────────────

    fn str_lit(v: &str) -> Expr {
        Expr::StrLit {
            value: v.into(),
            span: s(),
        }
    }

    fn builtin(name: &str, args: Vec<Expr>) -> Expr {
        Expr::BuiltinCall {
            name: name.into(),
            args,
            effects: EffectSet::PURE,
            span: s(),
        }
    }

    #[test]
    fn oop_new_emits_call_new() {
        // Dog.new("Rex") → _sir_oop_call_new("Dog", "Rex").
        let e = builtin("__new__", vec![str_lit("Dog"), str_lit("Rex")]);
        let mut out = String::new();
        emit_expr(&mut out, &e, 0);
        assert_eq!(out, r#"_sir_oop_call_new("Dog", "Rex")"#);
    }

    #[test]
    fn oop_super_emits_call_super() {
        // super in Cat#describe → _sir_oop_call_super("describe", "Cat").
        let e = builtin("__super__", vec![str_lit("describe"), str_lit("Cat")]);
        let mut out = String::new();
        emit_expr(&mut out, &e, 0);
        assert_eq!(out, r#"_sir_oop_call_super("describe", "Cat")"#);
    }

    #[test]
    fn oop_def_method_emits_registration() {
        // __def_method__("Dog", "speak", MakeClosure(speak)) →
        // _sir_oop_def_method("Dog", "speak", _sir_make_closure(speak, [])).
        let closure = Expr::MakeClosure {
            fn_name: "speak".into(),
            captures: vec![],
            span: s(),
        };
        let e = builtin(
            "__def_method__",
            vec![str_lit("Dog"), str_lit("speak"), closure],
        );
        let mut out = String::new();
        emit_expr(&mut out, &e, 0);
        assert_eq!(
            out,
            r#"_sir_oop_def_method("Dog", "speak", _sir_make_closure(speak, []))"#
        );
    }

    #[test]
    fn oop_def_class_method_emits_registration() {
        let closure = Expr::MakeClosure {
            fn_name: "zero".into(),
            captures: vec![],
            span: s(),
        };
        let e = builtin(
            "__def_class_method__",
            vec![str_lit("Counter"), str_lit("zero"), closure],
        );
        let mut out = String::new();
        emit_expr(&mut out, &e, 0);
        assert_eq!(
            out,
            r#"_sir_oop_def_class_method("Counter", "zero", _sir_make_closure(zero, []))"#
        );
    }

    #[test]
    fn oop_self_emits_current_self() {
        let e = builtin("__self__", vec![]);
        let mut out = String::new();
        emit_expr(&mut out, &e, 0);
        assert_eq!(out, "_sir_oop_current_self()");
    }

    #[test]
    fn oop_include_emits_include_module() {
        // include Greetable in class Robot → __include__("Robot", "Greetable")
        // → _sir_oop_include_module("Robot", "Greetable").
        let e = builtin("__include__", vec![str_lit("Robot"), str_lit("Greetable")]);
        let mut out = String::new();
        emit_expr(&mut out, &e, 0);
        assert_eq!(out, r#"_sir_oop_include_module("Robot", "Greetable")"#);
    }

    #[test]
    fn oop_extend_emits_extend_module() {
        // extend Counting in class Widget → __extend__("Widget", "Counting")
        // → _sir_oop_extend_module("Widget", "Counting").
        let e = builtin("__extend__", vec![str_lit("Widget"), str_lit("Counting")]);
        let mut out = String::new();
        emit_expr(&mut out, &e, 0);
        assert_eq!(out, r#"_sir_oop_extend_module("Widget", "Counting")"#);
    }

    #[test]
    fn sym_block_pass_on_dispatch_emits_sym_to_proc() {
        // arr.map(&:to_s) → callMethod(arr, "map", sym_to_proc(intern("to_s")))
        let e = method_call(
            Expr::VarRef {
                name: "arr".into(),
                scope: Scope::Local,
                span: s(),
            },
            "map",
            vec![block_pass(Expr::SymLit {
                name: "to_s".into(),
                span: s(),
            })],
        );
        let mut out = String::new();
        emit_expr(&mut out, &e, 0);
        assert_eq!(
            out,
            r#"_sir_oop_call_method(arr, "map", _sir_oop_sym_to_proc(_sir_intern("to_s")))"#
        );
    }

    #[test]
    fn proc_block_pass_on_dispatch_unwraps_to_value() {
        // arr.each(&p) → callMethod(arr, "each", p) — the proc IS the block.
        let e = method_call(
            Expr::VarRef {
                name: "arr".into(),
                scope: Scope::Local,
                span: s(),
            },
            "each",
            vec![block_pass(Expr::VarRef {
                name: "p".into(),
                scope: Scope::Local,
                span: s(),
            })],
        );
        let mut out = String::new();
        emit_expr(&mut out, &e, 0);
        assert_eq!(out, r#"_sir_oop_call_method(arr, "each", p)"#);
    }

    #[test]
    fn sym_block_pass_as_plain_arg_emits_sym_to_proc() {
        // The general emit_arg path also handles a surviving block_pass.
        let mut out = String::new();
        emit_arg(
            &mut out,
            &block_pass(Expr::SymLit {
                name: "upcase".into(),
                span: s(),
            }),
            0,
        );
        assert_eq!(out, r#"_sir_oop_sym_to_proc(_sir_intern("upcase"))"#);
    }
}
