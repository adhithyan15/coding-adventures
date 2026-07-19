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
    Block, ElementwiseOpKind, Expr, Feature, Function, Global, IndexArg, Module, ParamKind, Scope,
    Stmt,
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
        // Method dispatch (`recv.meth(args)` → `BuiltinCall("__method__", …)`)
        // and scoped lookups (`A::B` → `BuiltinCall("__scope__", …)`) both route
        // through the OOP runtime even when the module declares no class/module
        // of its own — e.g. `"hi".upcase` or, post-M3, a rest param used as an
        // Array (`def f(*a); a.length; end`). Gate the OOP import on those
        // builtins too, else the emitted call is undefined.
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
        || module_uses_builtin(m, "__class_method__")
        || module_uses_builtin(m, "__self__")
        // Mixin directives (MX3): `include M` / `extend M` route to the OOP
        // runtime's `includeModule`/`extendModule`, so their presence pulls in
        // the import (the frontend pairs them with module `__def_method__`s,
        // which already gate, but the gate must not depend on that).
        || module_uses_builtin(m, "__include__")
        || module_uses_builtin(m, "__extend__")
}

/// True if the module uses exception handling, in which case the emitted
/// artifact imports `@coding-adventures/sir-runtime-exceptions`.
fn uses_exceptions(m: &Module) -> bool {
    m.manifest.contains(Feature::Exceptions)
}

/// Collect every `class Child < Parent` edge in the module as `(child, parent)`
/// pairs, in source order, de-duplicated (first edge for a name wins).
///
/// **Why (E2).**  A `rescue StandardError` must catch a raised user
/// `MyErr < StandardError`, but the exception runtime only knows the built-in
/// hierarchy — it cannot learn that `MyErr` descends from `StandardError`
/// unless we *tell* it.  `Stmt::ClassDef` carries exactly that static edge, so
/// we harvest the `superclass`-bearing class defs here and emit a single
/// `registerAncestry({...})` call at program init (see [`emit_module`]).
/// Classes without a superclass (`class Foo`) contribute no edge — they still
/// match by exact name, unchanged.
///
/// The walk mirrors the builtin-usage scan: exhaustive over the statement forms
/// that nest bodies (`ClassDef`/`ModuleDef`/`SingletonClassDef`/`TryCatch` and
/// the loops), so a class defined *inside* a `begin`/loop/class body is still
/// found.
fn collect_user_ancestry(m: &Module) -> Vec<(String, String)> {
    let mut pairs: Vec<(String, String)> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for f in &m.functions {
        collect_ancestry_in_block(&f.body, &mut pairs, &mut seen);
    }
    pairs
}

fn collect_ancestry_in_block(
    b: &Block,
    pairs: &mut Vec<(String, String)>,
    seen: &mut HashSet<String>,
) {
    collect_ancestry_in_stmts(&b.stmts, pairs, seen);
    collect_ancestry_in_expr(&b.value, pairs, seen);
}

fn collect_ancestry_in_stmts(
    stmts: &[Stmt],
    pairs: &mut Vec<(String, String)>,
    seen: &mut HashSet<String>,
) {
    for s in stmts {
        collect_ancestry_in_stmt(s, pairs, seen);
    }
}

fn collect_ancestry_in_stmt(
    s: &Stmt,
    pairs: &mut Vec<(String, String)>,
    seen: &mut HashSet<String>,
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
        // SIR22: `target[indices...] = value` carries only sub-*expressions*
        // (target/indices/value), never a statement block, so it can nest no
        // `ClassDef` — same reasoning as `SeqSet`/`MapSet` above, just walked
        // for completeness in case a future frontend surprises us.
        Stmt::IndexSet {
            target,
            indices,
            value,
            ..
        } => {
            collect_ancestry_in_expr(target, pairs, seen);
            for idx in indices {
                collect_ancestry_in_index_arg(idx, pairs, seen);
            }
            collect_ancestry_in_expr(value, pairs, seen);
        }
    }
}

fn collect_ancestry_in_index_arg(
    idx: &IndexArg,
    pairs: &mut Vec<(String, String)>,
    seen: &mut HashSet<String>,
) {
    match idx {
        IndexArg::Scalar(e) | IndexArg::Range(e) => collect_ancestry_in_expr(e, pairs, seen),
        IndexArg::Whole => {}
    }
}

fn collect_ancestry_in_expr(
    e: &Expr,
    pairs: &mut Vec<(String, String)>,
    seen: &mut HashSet<String>,
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
        // class declaration (calls/literals carry only sub-*expressions*).
        _ => {}
    }
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

/// True if the module uses the SIR23 symbolic/pattern domain, in which case
/// the emitted artifact imports `@coding-adventures/sir-runtime-symbolic`.
/// Both `Feature::SymbolicExpr` (`SymSymbol`/`SymApply`) and
/// `Feature::PatternMatching` (`SymPatternBlank`/`SymPatternNamed`/
/// `SymRule`/`SymReplaceAll`) gate the same single import — a module using
/// only bare symbolic values with no pattern/rule ever still needs `sym`/
/// `apply` from this package.
fn uses_symbolic(m: &Module) -> bool {
    m.manifest.contains(Feature::SymbolicExpr) || m.manifest.contains(Feature::PatternMatching)
}

/// True if the module uses the SIR22 array/matrix domain, in which case the
/// emitted artifact imports `@coding-adventures/sir-runtime-array`. Any of
/// the three SIR22 features gates the same single import — a module using
/// only a bare `ArrayLit`/`IndexGet` with no `MatMul`/`ElementwiseOp`/
/// `Transpose` still needs `fromRows`/`indexGet` from this package.
fn uses_array(m: &Module) -> bool {
    m.manifest.contains(Feature::NDArrays)
        || m.manifest.contains(Feature::MatrixOps)
        || m.manifest.contains(Feature::ArrayColumnMajor)
}

/// Walk every function body for a `BuiltinCall` named `name` — gates
/// per-concern imports for builtins that carry no `Feature` flag.  Exhaustive
/// over `Stmt`/`Expr` so a new node can't silently hide a use.
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
        // SIR22: `target[indices...] = value` — recurse into every operand
        // (mirrors `SeqSet`/`MapSet` above) so a builtin nested inside an
        // index expression is still detected.
        Stmt::IndexSet {
            target,
            indices,
            value,
            ..
        } => {
            expr_uses_builtin(target, name)
                || indices.iter().any(|idx| index_arg_uses_builtin(idx, name))
                || expr_uses_builtin(value, name)
        }
    }
}

fn index_arg_uses_builtin(idx: &IndexArg, name: &str) -> bool {
    match idx {
        IndexArg::Scalar(e) | IndexArg::Range(e) => expr_uses_builtin(e, name),
        IndexArg::Whole => false,
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
        // KW1 compile-compat stub: recurse into a `KeywordArg`'s inner `value`
        // (its runtime meaning) so this builtin-usage scan stays faithful.
        // Real support pending KW2–KW6.
        Expr::KeywordArg { value, .. } => expr_uses_builtin(value, name),
        // SIR22 compile-compat stubs: this backend does not accept
        // `Feature::NDArrays`/`Feature::MatrixOps` (see `accepts_features` in
        // lib.rs), so `check_module` rejects any module using these nodes
        // before emission — but the scan still recurses into every operand
        // so it stays faithful if that ever changes.
        Expr::ArrayLit { rows, .. } => rows
            .iter()
            .any(|row| row.iter().any(|e| expr_uses_builtin(e, name))),
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
                || indices.iter().any(|idx| index_arg_uses_builtin(idx, name))
        }
        // SIR22 addendum compile-compat stubs: same rationale as the base
        // SIR22 stubs above — this backend doesn't accept `MatrixOps`/
        // `NDArrays`/`ArrayColumnMajor` either, so these never reach
        // emission, but the scan still recurses faithfully.
        Expr::Reduce { target, .. } => expr_uses_builtin(target, name),
        Expr::Scan { target, .. } => expr_uses_builtin(target, name),
        Expr::OuterProduct { lhs, rhs, .. } => {
            expr_uses_builtin(lhs, name) || expr_uses_builtin(rhs, name)
        }
        Expr::Shape { target, .. } => expr_uses_builtin(target, name),
        Expr::Reshape { shape, target, .. } => {
            expr_uses_builtin(shape, name) || expr_uses_builtin(target, name)
        }
        Expr::IndexGenerator { count, .. } => expr_uses_builtin(count, name),
        Expr::IndexOf {
            haystack, needle, ..
        } => expr_uses_builtin(haystack, name) || expr_uses_builtin(needle, name),
        Expr::Ravel { target, .. } => expr_uses_builtin(target, name),
        Expr::Catenate { lhs, rhs, .. } => {
            expr_uses_builtin(lhs, name) || expr_uses_builtin(rhs, name)
        }
        Expr::Convert { value, .. } => expr_uses_builtin(value, name),
        // SIR23 compile-compat stubs: this backend does not accept
        // `Feature::SymbolicExpr`/`Feature::PatternMatching` (see
        // `accepts_features` in lib.rs), so `check_module` rejects any
        // module using these nodes before emission — same rationale as the
        // SIR22 stubs above.
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
    // Source-language display convention (SIR display-convention spec): a
    // Ruby-sourced module selects the Ruby boolean form (`true`/`false`) once
    // at startup via the namespace-imported `__Sir`; every other source
    // language keeps the default Lisp `#t`/`#f`, so existing Twig output is
    // unchanged.
    //
    // SECURITY: the emitted argument is a hardcoded `"ruby"` literal chosen by
    // an exact `== "ruby"` comparison — never text derived from
    // `source_language` or any other source-controlled field — so this can
    // never inject into the emitted TypeScript.
    if m.metadata.source_language.as_deref() == Some("ruby") {
        out.push_str("__Sir.setDisplayConvention(\"ruby\");\n");
    }
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
    // Only symbolic/pattern-using modules import the SIR23 runtime.
    if uses_symbolic(m) {
        out.push_str(crate::runtime::RUNTIME_SYM);
    }
    // Only array/matrix-using modules import the SIR22 runtime.
    if uses_array(m) {
        out.push_str(crate::runtime::RUNTIME_ARRAY);
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
            out.push_str("\n__SirExc.registerAncestry({");
            for (i, (child, parent)) in pairs.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                let _ = write!(
                    out,
                    "{}: {}",
                    quote_ts_string(child),
                    quote_ts_string(parent)
                );
            }
            out.push_str("});\n");
        }
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
    let _ = writeln!(
        out,
        "// SIR span: {}",
        sanitize_comment(&f.span.to_string())
    );
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
    // KW3 — keyword parameters lower to a single trailing "options object".
    //
    // TypeScript (like JavaScript) has NO native keyword-argument syntax: a
    // caller cannot write `f(y: 2)` and have `y` bound by name to a formal
    // parameter.  The idiomatic, zero-runtime equivalent is the *options
    // object* — one trailing parameter that carries a bag of named values,
    // destructured on entry.  So for a definition
    //
    //     def f(a, x:, y: 1)          # a positional, x required-kw, y opt-kw
    //
    // we emit
    //
    //     function f(a: __Sir.Val, __kw: __Sir.Val): __Sir.Val {
    //       const { x, y = 1 } = (__kw ?? {}) as { [k: string]: __Sir.Val };
    //       …
    //     }
    //
    // The destructure carries each *optional* keyword's default (`y = 1`) and
    // omits it for a *required* one (`x`) — matching the call side, which for
    // a validator-accepted call always supplies every required keyword.  We
    // split the param list rather than interleave: all `Keyword` params
    // collapse into the ONE `__kw` object, appended after the positionals, so
    // the arity of the JS parameter list stays `positionals + 1`.
    //
    // Why `__kw`?  The `__`-prefix is this backend's reserved namespace for
    // synthesized bindings (`__Sir`, `__SirOop`, `__l`, `__sir_result`, …).
    // A user keyword name never round-trips through the frontends as `__kw`,
    // and `sanitize_ident` passes real user idents through verbatim, so the
    // options-object parameter cannot shadow a user binding in practice.
    let keyword_params: Vec<&semantic_ir::Param> = f.keyword_params();
    for p in &f.params {
        // Keyword params are NOT emitted inline — they fold into the trailing
        // `__kw` object handled after this loop.  Skip them here.
        if p.kind == ParamKind::Keyword {
            continue;
        }
        if !first {
            out.push_str(", ");
        }
        first = false;
        // M3 variadic kinds in TypeScript:
        //   Rest   (`*rest`)  → `...rest: __Sir.Val[]`  (native JS rest)
        //   KwRest (`**opts`) → `opts: __Sir.Val`        (v0 object fallback)
        // JavaScript has no keyword-argument call form, so a KwRest def
        // parameter has no faithful native declaration. v0: emit it as a
        // trailing ordinary object parameter — the call side (Q10f) already
        // collapses `**h` into a single merged trailing object, so this binds
        // that object. (Documented limitation; mirrors the TS double-splat
        // call-position treatment.)
        match p.kind {
            ParamKind::Rest => {
                let _ = write!(out, "...{}: __Sir.Val[]", sanitize_ident(&p.name));
            }
            // Keyword handled above (skipped); this arm covers positionals and
            // the KwRest object fallback.
            ParamKind::Required | ParamKind::KwRest | ParamKind::Keyword => {
                let _ = write!(out, "{}: __Sir.Val", sanitize_ident(&p.name));
                // P2b default parameters.  A SIR default is evaluated
                // per-call in the callee's parameter scope and may reference
                // EARLIER params — exactly TypeScript's native default-value
                // semantics.  So we emit the default expression verbatim
                // through the ordinary expression emitter (which renders an
                // earlier param's `VarRef` as a plain identifier, valid in
                // TS):  `name: __Sir.Val = <default>`.  The call side never
                // pads omitted trailing args (the validator allows omitting
                // them), so these native defaults are what fill them in.
                //
                // Only `Required` params carry a meaningful default: a `Rest`
                // param matched the arm above, and a `KwRest` (`**opts`) has
                // no default surface form, so its `default` (if any) is
                // ignored here — consistent with the v0 object fallback.
                if p.kind == ParamKind::Required {
                    if let Some(default) = &p.default {
                        out.push_str(" = ");
                        emit_expr(out, default, 2);
                    }
                }
            }
        }
    }
    // Append the single trailing options-object parameter iff the function has
    // any keyword params.  (No keyword params → no `__kw`, so ordinary
    // positional functions are byte-for-byte unchanged.)
    if !keyword_params.is_empty() {
        if !first {
            out.push_str(", ");
        }
        out.push_str("__kw: __Sir.Val");
    }
    out.push_str("): __Sir.Val {\n");

    // Body prologue: destructure the options object into the keyword bindings.
    // Each required keyword (`default: None`) is a bare name; each optional
    // (`default: Some(e)`) carries its default expression, so an omitted
    // optional falls back to `e`.  `__kw ?? {}` tolerates a caller that
    // supplied NO keywords at all (the object is then absent → `undefined`),
    // which happens when every keyword is optional and the call omits them.
    if !keyword_params.is_empty() {
        out.push_str("  const { ");
        for (i, p) in keyword_params.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            out.push_str(&sanitize_ident(&p.name));
            if let Some(default) = &p.default {
                out.push_str(" = ");
                emit_expr(out, default, 2);
            }
        }
        // Cast the untyped `Val` to an index signature so the destructure
        // typechecks under `strict` — the runtime shape is a plain object.
        out.push_str(" } = (__kw ?? {}) as { [k: string]: __Sir.Val };\n");
    }

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
        Stmt::Assign {
            name,
            scope: Scope::Local,
            value,
            ..
        } => {
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
        Stmt::ForRange {
            start,
            stop,
            step,
            body,
            ..
        } => {
            collect_expr_assigned(start, out);
            collect_expr_assigned(stop, out);
            collect_expr_assigned(step, out);
            collect_assigned_locals(body, out);
        }
        Stmt::ForEach { iter, body, .. } => {
            collect_expr_assigned(iter, out);
            collect_assigned_locals(body, out);
        }
        Stmt::SeqSet {
            seq, index, value, ..
        } => {
            collect_expr_assigned(seq, out);
            collect_expr_assigned(index, out);
            collect_expr_assigned(value, out);
        }
        Stmt::MapSet {
            map, key, value, ..
        } => {
            collect_expr_assigned(map, out);
            collect_expr_assigned(key, out);
            collect_expr_assigned(value, out);
        }
        // A try/rescue/ensure carries bare statement lists that may reassign
        // an outer local, so descend into every one.
        Stmt::TryCatch {
            body,
            rescues,
            ensure_body,
            ..
        } => {
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
        // SIR22 compile-compat stub: rejected by `check_module` before this
        // backend ever emits (no `Feature::NDArrays`/`Feature::MatrixOps` in
        // `accepts_features`), but recurse into every operand — mirroring
        // `SeqSet`/`MapSet` above — so the scan stays faithful regardless.
        Stmt::IndexSet {
            target,
            indices,
            value,
            ..
        } => {
            collect_expr_assigned(target, out);
            for idx in indices {
                collect_index_arg_assigned(idx, out);
            }
            collect_expr_assigned(value, out);
        }
    }
}

fn collect_index_arg_assigned(idx: &IndexArg, out: &mut HashSet<String>) {
    match idx {
        IndexArg::Scalar(e) | IndexArg::Range(e) => collect_expr_assigned(e, out),
        IndexArg::Whole => {}
    }
}

fn collect_expr_assigned(e: &Expr, out: &mut HashSet<String>) {
    match e {
        Expr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
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
        // KW1 compile-compat stub: recurse into a `KeywordArg`'s inner `value`
        // — its runtime meaning — so assigned-local collection stays faithful.
        // Real support pending KW2–KW6.
        Expr::KeywordArg { value, .. } => collect_expr_assigned(value, out),
        // SIR22 compile-compat stubs: rejected by `check_module` before this
        // backend ever emits (no `Feature::NDArrays`/`Feature::MatrixOps` in
        // `accepts_features`), but recurse into every operand so the scan
        // stays faithful regardless.
        Expr::ArrayLit { rows, .. } => {
            for row in rows {
                for e in row {
                    collect_expr_assigned(e, out);
                }
            }
        }
        Expr::Range {
            start, step, stop, ..
        } => {
            collect_expr_assigned(start, out);
            if let Some(s) = step {
                collect_expr_assigned(s, out);
            }
            collect_expr_assigned(stop, out);
        }
        Expr::MatMul { lhs, rhs, .. } | Expr::ElementwiseOp { lhs, rhs, .. } => {
            collect_expr_assigned(lhs, out);
            collect_expr_assigned(rhs, out);
        }
        Expr::Transpose { target, .. } => collect_expr_assigned(target, out),
        Expr::IndexGet {
            target, indices, ..
        } => {
            collect_expr_assigned(target, out);
            for idx in indices {
                collect_index_arg_assigned(idx, out);
            }
        }
        // SIR22 addendum compile-compat stubs: same rationale as the base
        // SIR22 stubs above.
        Expr::Reduce { target, .. }
        | Expr::Scan { target, .. }
        | Expr::Shape { target, .. }
        | Expr::Ravel { target, .. } => collect_expr_assigned(target, out),
        Expr::OuterProduct { lhs, rhs, .. } | Expr::Catenate { lhs, rhs, .. } => {
            collect_expr_assigned(lhs, out);
            collect_expr_assigned(rhs, out);
        }
        Expr::Reshape { shape, target, .. } => {
            collect_expr_assigned(shape, out);
            collect_expr_assigned(target, out);
        }
        Expr::IndexGenerator { count, .. } => collect_expr_assigned(count, out),
        Expr::IndexOf {
            haystack, needle, ..
        } => {
            collect_expr_assigned(haystack, out);
            collect_expr_assigned(needle, out);
        }
        Expr::Convert { value, .. } => collect_expr_assigned(value, out),
        // SIR23 compile-compat stubs: rejected by `check_module` before this
        // backend ever emits (no `Feature::SymbolicExpr`/`Feature::PatternMatching`
        // in `accepts_features`), but recurse into every operand so the scan
        // stays faithful regardless.
        Expr::SymSymbol { .. } | Expr::SymRational { .. } => {}
        Expr::SymApply { head, args, .. } => {
            collect_expr_assigned(head, out);
            for a in args {
                collect_expr_assigned(a, out);
            }
        }
        Expr::SymPatternBlank { head, .. } => {
            if let Some(h) = head {
                collect_expr_assigned(h, out);
            }
        }
        Expr::SymPatternNamed { pattern, .. } => collect_expr_assigned(pattern, out),
        Expr::SymRule { lhs, rhs, .. } => {
            collect_expr_assigned(lhs, out);
            collect_expr_assigned(rhs, out);
        }
        Expr::SymReplaceAll { expr, rules, .. } => {
            collect_expr_assigned(expr, out);
            for r in rules {
                collect_expr_assigned(r, out);
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
            let _ = write!(
                out,
                "{}{} {}: __Sir.Val = ",
                pad,
                keyword,
                sanitize_ident(name)
            );
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
        Stmt::SeqSet {
            seq, index, value, ..
        } => {
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
        Stmt::MapSet {
            map, key, value, ..
        } => {
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
            let _ = writeln!(out, "{}}}", pad);
        }
        // `for (var = start; …; var += step) { body }` — half-open
        // (`stop` exclusive).  `stop`/`step` are evaluated ONCE into
        // block-scoped temporaries (matching Python's `range`), and the
        // loop condition is direction-aware so a negative `step` counts
        // down correctly.
        Stmt::ForRange {
            var,
            start,
            stop,
            step,
            body,
            ..
        } => {
            let id = fresh_loop_id();
            let v = sanitize_ident(var);
            let inner = indent + 2;
            let inner_pad = " ".repeat(inner);
            // Open a block so the temporaries don't leak.
            let _ = writeln!(out, "{}{{", pad);
            let _ = write!(out, "{}let {}: __Sir.Val = ", inner_pad, v);
            emit_expr(out, start, inner);
            out.push_str(";\n");
            let _ = write!(out, "{}const __sir_stop_{}: number = (", inner_pad, id);
            emit_expr(out, stop, inner);
            out.push_str(") as number;\n");
            let _ = write!(out, "{}const __sir_step_{}: number = (", inner_pad, id);
            emit_expr(out, step, inner);
            out.push_str(") as number;\n");
            let _ = writeln!(
                out,
                "{}while (__sir_step_{id} >= 0 ? ({v} as number) < __sir_stop_{id} : ({v} as number) > __sir_stop_{id}) {{",
                inner_pad, id = id, v = v
            );
            emit_block_as_stmts(out, body, inner + 2);
            let _ = writeln!(
                out,
                "{}{} = ({} as number) + __sir_step_{};",
                " ".repeat(inner + 2),
                v,
                v,
                id
            );
            let _ = writeln!(out, "{}}}", inner_pad);
            let _ = writeln!(out, "{}}}", pad);
        }
        // `for (const var of iter) { body }` — iterate a Seq.  The
        // binding uses `let` if the body reassigns the loop variable.
        Stmt::ForEach {
            var, iter, body, ..
        } => {
            let kw = if MUTABLE_NAMES.with(|m| m.borrow().contains(var)) {
                "let"
            } else {
                "const"
            };
            let _ = write!(out, "{}for ({} {} of ((", pad, kw, sanitize_ident(var));
            emit_expr(out, iter, indent);
            out.push_str(") as __Sir.Val[])) {\n");
            emit_block_as_stmts(out, body, indent + 2);
            let _ = writeln!(out, "{}}}", pad);
        }
        // ── SIR17 scopes (assignment) ───────────────────────────────
        // `@x = v` → current-self instance-variable write via the OOP
        // runtime (no native `this` — methods are receiver-less).
        Stmt::Assign {
            name,
            scope: Scope::Instance,
            value,
            ..
        } => {
            let _ = write!(out, "{}__SirOop.ivarSet({}, ", pad, quote_ts_string(name));
            emit_expr(out, value, indent);
            out.push_str(");\n");
        }
        // `@@x = v` → class-variable store write.
        Stmt::Assign {
            name,
            scope: Scope::ClassVar,
            value,
            ..
        } => {
            let _ = write!(out, "{}__SirOop.cvarSet({}, ", pad, quote_ts_string(name));
            emit_expr(out, value, indent);
            out.push_str(");\n");
        }
        // `CONST = v` → an ordinary module-level binding.  Constants are
        // assign-once in Ruby, so `const` is faithful; reads elsewhere
        // emit the bare identifier (see `emit_var_ref`).
        Stmt::Assign {
            name,
            scope: Scope::Const,
            value,
            ..
        } => {
            let _ = write!(out, "{}const {}: __Sir.Val = ", pad, sanitize_ident(name));
            emit_expr(out, value, indent);
            out.push_str(";\n");
        }
        // `Assign` to a builtin is never produced by any frontend (you
        // cannot rebind `+`); a validated module never reaches here.
        Stmt::Assign {
            scope: Scope::Builtin,
            span,
            ..
        } => {
            panic!(
                "ts backend reached an assign to a Builtin-scoped name at {} — invalid SIR",
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
                "{}__SirOop.defineClass({}, ",
                pad,
                quote_ts_string(name)
            );
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
            let _ = writeln!(
                out,
                "{}__SirOop.defineClass({}, null);",
                pad,
                quote_ts_string(name)
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
        // `begin … rescue … ensure … end` → native `try { … } catch (e) {
        // … } finally { … }`.  A native `catch` binds *one* variable and
        // catches *everything*, while Ruby has an ordered list of typed
        // `rescue` clauses, so the catch body is an if/else-if chain that asks
        // the exception runtime `rescueMatches(exc, [class names])` for each
        // clause in source order and re-`throw`s if none match (matching
        // Ruby's "propagate when unrescued").
        Stmt::TryCatch {
            body,
            rescues,
            ensure_body,
            ..
        } => {
            let pad = " ".repeat(indent);
            let _ = writeln!(out, "{}try {{", pad);
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
                    let _ = writeln!(
                        out,
                        "{}{} (__SirExc.rescueMatches(__exc, {})) {{",
                        ipad, kw, types
                    );
                    // `rescue Foo => e` binds the caught value as a local.
                    if let Some(bind) = &r.binding {
                        let _ = writeln!(
                            out,
                            "{}  const {}: __Sir.Val = __exc;",
                            ipad,
                            sanitize_ident(bind)
                        );
                    }
                    emit_stmt_block(out, &r.body, inner + 2);
                }
                // No clause matched → propagate the original exception.
                let _ = writeln!(out, "{}}} else {{", ipad);
                let _ = writeln!(out, "{}  throw __exc;", ipad);
                let _ = writeln!(out, "{}}}", ipad);
                let _ = write!(out, "{}}}", pad);
            }
            if let Some(ens) = ensure_body {
                out.push_str(" finally {\n");
                emit_stmt_block(out, ens, indent + 2);
                let _ = write!(out, "{}}}", pad);
            }
            out.push('\n');
        }
        // SIR22: array/matrix indexed assignment — `target[indices...] =
        // value;`, mutating in place via `__SirArray.indexSet` (the
        // imported `@coding-adventures/sir-runtime-array` package, gated by
        // `uses_array` — see `emit_module`). Matches the SIR22 spec's own
        // note that `IndexSet` is a `Stmt`, not a pure `Expr`, for exactly
        // this in-place-mutation reason.
        Stmt::IndexSet {
            target,
            indices,
            value,
            ..
        } => {
            out.push_str(&pad);
            out.push_str("__SirArray.indexSet(");
            emit_expr(out, target, indent);
            out.push_str(", [");
            emit_index_args(out, indices, indent);
            out.push_str("], ");
            emit_expr(out, value, indent);
            out.push_str(");\n");
        }
    }
}

/// The `@coding-adventures/sir-runtime-array` string `elementwise`'s
/// `applyOp` switches on — exact `ElementwiseOpKind` variant names, not
/// `.name()`'s lowercase forms, since this string is a real runtime
/// dispatch key, not a cosmetic label.
fn elementwise_op_ts_name(op: ElementwiseOpKind) -> &'static str {
    match op {
        ElementwiseOpKind::Add => "Add",
        ElementwiseOpKind::Sub => "Sub",
        ElementwiseOpKind::Mul => "Mul",
        ElementwiseOpKind::Div => "Div",
        ElementwiseOpKind::Pow => "Pow",
        ElementwiseOpKind::Max => "Max",
        ElementwiseOpKind::Min => "Min",
        ElementwiseOpKind::Eq => "Eq",
        ElementwiseOpKind::Ne => "Ne",
        ElementwiseOpKind::Lt => "Lt",
        ElementwiseOpKind::Le => "Le",
        ElementwiseOpKind::Ge => "Ge",
        ElementwiseOpKind::Gt => "Gt",
    }
}

/// Emit one `IndexArg` as the object literal `__SirArray.indexGet`/
/// `indexSet` expect: `{ kind: "scalar", value }` / `{ kind: "whole" }` /
/// `{ kind: "range", indices: <NDArray> }`. The `Range` case reuses
/// `emit_expr` on the inner `Expr::Range` node directly — that node's own
/// `Expr::Range` arm already emits a call into `__SirArray.range(...)`,
/// which returns exactly the `NDArray` shape `indices` needs, so no
/// separate handling is needed here.
fn emit_index_arg(out: &mut String, arg: &IndexArg, indent: usize) {
    match arg {
        IndexArg::Scalar(e) => {
            out.push_str("{ kind: \"scalar\", value: ");
            emit_expr(out, e, indent);
            out.push_str(" }");
        }
        IndexArg::Whole => {
            out.push_str("{ kind: \"whole\" }");
        }
        IndexArg::Range(e) => {
            out.push_str("{ kind: \"range\", indices: ");
            emit_expr(out, e, indent);
            out.push_str(" }");
        }
    }
}

fn emit_index_args(out: &mut String, indices: &[IndexArg], indent: usize) {
    for (i, arg) in indices.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        emit_index_arg(out, arg, indent);
    }
}

/// Detect the `BuiltinCall("global_set", SymLit(name), value)`
/// pattern and return the global name + value if so.
fn pick_global_set(e: &Expr) -> Option<(&str, &Expr)> {
    if let Expr::BuiltinCall { name, args, .. } = e {
        if name == "global_set" && args.len() == 2 {
            if let Expr::SymLit {
                name: global_name, ..
            } = &args[0]
            {
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
        Expr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
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
            emit_call_args(out, args, indent);
            out.push(')');
        }
        Expr::IndirectCall { target, args, .. } => {
            out.push_str("__Sir.apply(");
            emit_expr(out, target, indent);
            out.push_str(", [");
            emit_call_args(out, args, indent);
            out.push_str("])");
        }
        Expr::BuiltinCall { name, args, .. } => emit_builtin_call(out, name, args, indent),
        Expr::MakeClosure {
            fn_name, captures, ..
        } => {
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
        // KW3 — a `KeywordArg` is NOT a first-class value: it only ever
        // appears inside a call's `args`, where `emit_call_args` collapses the
        // trailing run of them into one options-object literal (never routing
        // them through `emit_expr`).  The validator (spec rule 6) rejects a
        // `KeywordArg` in any other position, so reaching this arm means the
        // call path bypassed `emit_call_args` — an internal backend bug.  A
        // positioned panic surfaces it (mirrors the `Intrinsic` guard).
        Expr::KeywordArg { span, .. } => {
            panic!(
                "typescript backend reached a bare keyword-arg expression at {} — a KeywordArg must be collapsed by emit_call_args, never emitted as a value",
                span
            );
        }
        // ── SIR22: array/matrix nodes (base cut) ──────────────────────
        // Real codegen: calls into the imported `@coding-adventures/sir-runtime-array`
        // package (bound as `__SirArray`, gated by `uses_array` — see
        // `emit_module`), mirroring how the SIR23 `Sym*` arms above call
        // into `__SirSym.*`. `rows` is row-major in the literal syntax (per
        // the SIR22 spec); `__SirArray.fromRows` reconciles that with
        // column-major storage, so the emitter just nests the row/element
        // expressions unchanged.
        Expr::ArrayLit { rows, .. } => {
            out.push_str("__SirArray.fromRows([");
            for (i, row) in rows.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push('[');
                emit_args(out, row, indent);
                out.push(']');
            }
            out.push_str("])");
        }
        // `__SirArray.range(start, stop, step)` — note the argument ORDER:
        // the SIR node's own field order is `start, step, stop`, but
        // `sir-runtime-array`'s `range(start, stop, step = 1)` takes `stop`
        // before `step`.
        Expr::Range {
            start, step, stop, ..
        } => {
            out.push_str("__SirArray.range(");
            emit_expr(out, start, indent);
            out.push_str(", ");
            emit_expr(out, stop, indent);
            out.push_str(", ");
            match step {
                Some(step) => emit_expr(out, step, indent),
                None => out.push('1'),
            }
            out.push(')');
        }
        Expr::MatMul { lhs, rhs, .. } => {
            out.push_str("__SirArray.matmul(");
            emit_expr(out, lhs, indent);
            out.push_str(", ");
            emit_expr(out, rhs, indent);
            out.push(')');
        }
        // The op name must match `elementwise`'s `applyOp` switch (in
        // `sir-runtime-array`) exactly (`"Add"`, not `.name()`'s lowercase
        // `"add"`).
        Expr::ElementwiseOp { op, lhs, rhs, .. } => {
            let _ = write!(
                out,
                "__SirArray.elementwise({}, ",
                quote_ts_string(elementwise_op_ts_name(*op))
            );
            emit_expr(out, lhs, indent);
            out.push_str(", ");
            emit_expr(out, rhs, indent);
            out.push(')');
        }
        Expr::Transpose {
            target, conjugate, ..
        } => {
            out.push_str("__SirArray.transpose(");
            emit_expr(out, target, indent);
            let _ = write!(out, ", {conjugate})");
        }
        Expr::IndexGet {
            target, indices, ..
        } => {
            out.push_str("__SirArray.indexGet(");
            emit_expr(out, target, indent);
            out.push_str(", [");
            emit_index_args(out, indices, indent);
            out.push_str("])");
        }
        // ── SIR22 addendum: APL primitive operators — real codegen ────
        // Each of the nine maps 1:1 onto a call into the published
        // `@coding-adventures/sir-runtime-array` package's own SIR22-
        // addendum port (see that package's `reduce.ts`/`outer.ts`/
        // `shape.ts`/`iota.ts`/`ravel.ts`) — the SAME `__SirArray` import
        // the base cut above already gates in via `uses_array`, so no new
        // import-gating logic is needed here. `Reduce`/`Scan`/
        // `OuterProduct` carry an `ElementwiseOpKind` and so reuse
        // `elementwise_op_ts_name` exactly like `ElementwiseOp` above does;
        // the remaining six have no `op` field at all (they are "bespoke,
        // not BinOp-shaped" per the SIR22 spec addendum and
        // `apl_runtime::builtins`'s own doc comment) and just recurse into
        // their operand(s).
        Expr::Reduce { op, target, .. } => {
            let _ = write!(
                out,
                "__SirArray.reduce({}, ",
                quote_ts_string(elementwise_op_ts_name(*op))
            );
            emit_expr(out, target, indent);
            out.push(')');
        }
        Expr::Scan { op, target, .. } => {
            let _ = write!(
                out,
                "__SirArray.scan({}, ",
                quote_ts_string(elementwise_op_ts_name(*op))
            );
            emit_expr(out, target, indent);
            out.push(')');
        }
        Expr::OuterProduct { op, lhs, rhs, .. } => {
            let _ = write!(
                out,
                "__SirArray.outer({}, ",
                quote_ts_string(elementwise_op_ts_name(*op))
            );
            emit_expr(out, lhs, indent);
            out.push_str(", ");
            emit_expr(out, rhs, indent);
            out.push(')');
        }
        Expr::Shape { target, .. } => {
            out.push_str("__SirArray.shape(");
            emit_expr(out, target, indent);
            out.push(')');
        }
        // Field order here is `shape, target` (per the SIR22 spec: the
        // shape vector is not interchangeable with the data being
        // reshaped, so the node spells out the roles instead of reusing
        // `lhs`/`rhs` — see `semantic_ir::Expr::Reshape`'s own doc comment
        // in `nodes.rs`) — `sir-runtime-array`'s `reshape(shapeArg, target)`
        // (`shape.ts`) takes that SAME order, so no argument reordering is
        // needed at this call site (contrast `Expr::Range`'s `start, step,
        // stop` vs. `range`'s `start, stop, step` just above, which DOES
        // reorder — checked both directly rather than assumed, since this
        // crate's own base-cut `Range` arm already proves field order can
        // legitimately differ between a SIR node and its runtime callee).
        Expr::Reshape { shape, target, .. } => {
            out.push_str("__SirArray.reshape(");
            emit_expr(out, shape, indent);
            out.push_str(", ");
            emit_expr(out, target, indent);
            out.push(')');
        }
        Expr::IndexGenerator { count, .. } => {
            out.push_str("__SirArray.indexGenerator(");
            emit_expr(out, count, indent);
            out.push(')');
        }
        Expr::IndexOf {
            haystack, needle, ..
        } => {
            out.push_str("__SirArray.indexOf(");
            emit_expr(out, haystack, indent);
            out.push_str(", ");
            emit_expr(out, needle, indent);
            out.push(')');
        }
        Expr::Ravel { target, .. } => {
            out.push_str("__SirArray.ravel(");
            emit_expr(out, target, indent);
            out.push(')');
        }
        Expr::Catenate { lhs, rhs, .. } => {
            out.push_str("__SirArray.catenate(");
            emit_expr(out, lhs, indent);
            out.push_str(", ");
            emit_expr(out, rhs, indent);
            out.push(')');
        }
        // SIR26 `Convert` — `Conversions` not accepted; unreachable in a
        // validated module.
        Expr::Convert { span, .. } => {
            panic!(
                "typescript backend reached a deferred SIR26 expression ({}) at {span} — not accepted yet",
                e.kind_name()
            );
        }
        // SIR23 symbolic-expression/pattern nodes — construct/consume a
        // tagged term-tree value at runtime via `sir-runtime-symbolic`,
        // imported as `__SirSym` (gated by `uses_symbolic`, see
        // `emit_module`).  See the SIR23 spec's "Backend impact" and
        // `emit_sym_operand`'s doc comment for why a plain `IntLit`/
        // `FloatLit`/`StrLit` child needs wrapping but every other child
        // expression does not.
        Expr::SymSymbol { name, .. } => {
            out.push_str("__SirSym.sym(");
            out.push_str(&quote_ts_string(name));
            out.push(')');
        }
        Expr::SymRational { numer, denom, .. } => {
            let _ = write!(out, "__SirSym.rational({}, {})", numer, denom);
        }
        Expr::SymApply { head, args, .. } => {
            out.push_str("__SirSym.apply(");
            emit_sym_operand(out, head, indent);
            out.push_str(", [");
            for (i, a) in args.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                emit_sym_operand(out, a, indent);
            }
            out.push_str("])");
        }
        Expr::SymPatternBlank { head: None, .. } => {
            out.push_str("__SirSym.blank()");
        }
        Expr::SymPatternBlank {
            head: Some(head), ..
        } => match head.as_ref() {
            Expr::SymSymbol { name, .. } => {
                out.push_str("__SirSym.blankTyped(");
                out.push_str(&quote_ts_string(name));
                out.push(')');
            }
            _ => panic!(
                "typescript backend: SymPatternBlank's head-constraint must be a SymSymbol, got {} at {}",
                head.kind_name(),
                head.span()
            ),
        },
        Expr::SymPatternNamed { name, pattern, .. } => {
            out.push_str("__SirSym.named(");
            out.push_str(&quote_ts_string(name));
            out.push_str(", ");
            emit_sym_operand(out, pattern, indent);
            out.push(')');
        }
        Expr::SymRule {
            lhs, rhs, delayed, ..
        } => {
            out.push_str(if *delayed {
                "__SirSym.ruleDelayed("
            } else {
                "__SirSym.rule("
            });
            emit_sym_operand(out, lhs, indent);
            out.push_str(", ");
            emit_sym_operand(out, rhs, indent);
            out.push(')');
        }
        Expr::SymReplaceAll {
            expr,
            rules,
            repeated,
            ..
        } => {
            out.push_str("__SirSym.unwrap(");
            out.push_str(if *repeated {
                "__SirSym.replaceRepeated("
            } else {
                "__SirSym.replaceAll("
            });
            emit_sym_operand(out, expr, indent);
            out.push_str(", [");
            for (i, r) in rules.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                emit_sym_operand(out, r, indent);
            }
            out.push_str("]))");
        }
    }
}

/// Emit `e` as a value usable as a `symbolic-ir` `IRNode` term — used for
/// the `head`/`args`/`lhs`/`rhs`/`pattern`/`expr`/rule-list children of
/// `SymApply`/`SymPatternBlank`/`SymPatternNamed`/`SymRule`/
/// `SymReplaceAll`, where the SIR23 spec requires a real term-tree node,
/// not a bare host value.
///
/// The three literal kinds SIR23 "reuses directly" instead of defining new
/// leaf nodes for (`IntLit`/`FloatLit`/`StrLit`, per the spec's "New Expr
/// variants" section) need wrapping into the matching `__SirSym`
/// constructor here — a bare JS number or string is never a valid
/// `IRNode`. Every other expression (a `SymSymbol`/`SymRational`/
/// `SymApply`/pattern/rule node, which already constructs a term via the
/// ordinary `emit_expr` arms above, or a `VarRef`/call whose runtime value
/// is already a term by the frontend's own convention) emits unchanged.
fn emit_sym_operand(out: &mut String, e: &Expr, indent: usize) {
    match e {
        Expr::IntLit { .. } => {
            out.push_str("__SirSym.int(");
            emit_expr(out, e, indent);
            out.push(')');
        }
        Expr::FloatLit { .. } => {
            out.push_str("__SirSym.numberNode(");
            emit_expr(out, e, indent);
            out.push(')');
        }
        Expr::StrLit { .. } => {
            out.push_str("__SirSym.stringNode(");
            emit_expr(out, e, indent);
            out.push(')');
        }
        _ => emit_expr(out, e, indent),
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
        emit_arg(out, a, indent);
    }
}

/// Emit one argument / sequence element, expanding the `splat` marker into
/// JavaScript's native spread syntax.
///
/// Ruby `*x` reaches the backend as `BuiltinCall("splat", [x])` — a trailing
/// call argument or an array element — and maps cleanly to JS `...` (array /
/// argument spread).
///
/// | SIR marker (Ruby) | TS emitted | meaning |
/// |---|---|---|
/// | `splat` (`f(*a)`, `[1, *a, 3]`) | `...a` | spread an iterable's items |
///
/// `double_splat` (`**h`) is **not** handled here: it has no per-argument JS
/// form (JavaScript has no keyword-argument call), so the *call-argument* layer
/// ([`emit_call_args`]) collapses a contiguous run of `**` markers into one
/// merged keyword-map argument instead.  Anything that is not a `splat` marker
/// emits as an ordinary expression.
fn emit_arg(out: &mut String, a: &Expr, indent: usize) {
    if let Expr::BuiltinCall { name, args, .. } = a {
        if name == "splat" && args.len() == 1 {
            out.push_str("...");
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
/// | `SymLit("m")` (`&:m`) | `__SirOop.symToProc(intern("m"))` | `Symbol#to_proc` — a block calling `recv.m(...rest)` |
/// | any other (`&proc`)   | `<inner>` (unwrapped) | the operand already *is* the proc/block value |
///
/// Returns `true` when it handled a `block_pass` envelope (so the caller does
/// not also `emit_expr` it).  A malformed envelope (not exactly one operand)
/// is left for the generic path.
fn try_emit_block_pass(out: &mut String, a: &Expr, indent: usize) -> bool {
    if let Expr::BuiltinCall { name, args, .. } = a {
        if name == "block_pass" && args.len() == 1 {
            if let Expr::SymLit { .. } = &args[0] {
                out.push_str("__SirOop.symToProc(");
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

/// Is this argument a `**h` double-splat marker (`BuiltinCall("double_splat",
/// [operand])`)?
fn is_double_splat(a: &Expr) -> bool {
    matches!(a, Expr::BuiltinCall { name, args, .. }
        if name == "double_splat" && args.len() == 1)
}

/// Emit a *call's* argument list, collapsing every contiguous run of `**`
/// double-splat markers into a single merged keyword-map argument.
///
/// Ruby `f(**h1, **h2)` splices each map's entries as keyword arguments (later
/// keys winning).  Python emits a native `**`; **JavaScript has no
/// keyword-argument call form**, so there is no faithful per-argument spread.
/// The v0 strategy (see `code/specs/sir-runtime.md`) is the conventional JS
/// "options object": collapse the trailing/contiguous run of `**` markers into
/// ONE argument built by the runtime merge helper —
/// `__Sir.doubleSplatMerge(h1, h2)` — which returns a fresh `Map` with
/// left-to-right precedence.  A callee compiled from `def f(**opts)` then
/// receives that map as its final positional parameter.
///
/// Plain (`splat`-or-ordinary) arguments pass straight through [`emit_arg`], so
/// `f(a, *b, **h)` becomes `f(a, ...b, __Sir.doubleSplatMerge(h))`.  Runs are
/// collapsed *in place*, which keeps a trailing block argument (appended by the
/// frontend's block-param ABI) after the merged map — e.g. `f(**h) { … }`
/// emits `f(__Sir.doubleSplatMerge(h), <block>)`.
///
/// v0 cut-line: mixing inline `key: value` pairs with `**h` at one call site is
/// not modelled — only explicit `**map` operands are merged.
///
/// **KW3 — keyword arguments (`f(1, y: 2)`).**  A named keyword argument
/// reaches the backend as an [`Expr::KeywordArg`] inside `args`, always AFTER
/// every positional (the validator enforces the ordering).  The def side folds
/// keyword *parameters* into a trailing `__kw` options object, so the call side
/// must supply that object: every `KeywordArg` in this call collapses into ONE
/// trailing object literal `{ name: value, … }`.  With no keyword args the
/// object is omitted entirely, so an ordinary positional call is unchanged:
///
/// | SIR `args`                                   | emitted            |
/// |----------------------------------------------|--------------------|
/// | `[Int(1)]`                                    | `f(1)`             |
/// | `[Int(1), KeywordArg{y, Int(2)}]`             | `f(1, { y: 2 })`   |
/// | `[KeywordArg{x,…}, KeywordArg{y,…}]`          | `f({ x: …, y: … })`|
fn emit_call_args(out: &mut String, args: &[Expr], indent: usize) {
    // Split positionals from the trailing run of keyword arguments.  Because
    // the validator guarantees every `KeywordArg` follows all positionals, the
    // keyword args are exactly the trailing suffix — we partition once.
    let n_positional = args.iter().take_while(|a| !is_keyword_arg(a)).count();
    let positional = &args[..n_positional];
    let keyword = &args[n_positional..];

    let mut first = true;
    let mut i = 0;
    while i < positional.len() {
        if !first {
            out.push_str(", ");
        }
        first = false;
        if is_double_splat(&positional[i]) {
            // Gather this maximal contiguous run of `**` markers.
            let start = i;
            while i < positional.len() && is_double_splat(&positional[i]) {
                i += 1;
            }
            out.push_str("__Sir.doubleSplatMerge(");
            for (j, a) in positional[start..i].iter().enumerate() {
                if j > 0 {
                    out.push_str(", ");
                }
                if let Expr::BuiltinCall { args: inner, .. } = a {
                    emit_expr(out, &inner[0], indent);
                }
            }
            out.push(')');
        } else {
            emit_arg(out, &positional[i], indent);
            i += 1;
        }
    }

    // Collapse the trailing keyword arguments into one options-object literal
    // that binds the callee's `__kw` parameter (see `emit_function`).  Each
    // `KeywordArg { name, value }` becomes an object entry `name: <value>`;
    // the value lowers through the ordinary expression emitter.
    if !keyword.is_empty() {
        if !first {
            out.push_str(", ");
        }
        out.push_str("{ ");
        for (j, a) in keyword.iter().enumerate() {
            if j > 0 {
                out.push_str(", ");
            }
            if let Expr::KeywordArg { name, value, .. } = a {
                let _ = write!(out, "{}: ", sanitize_ident(name));
                emit_expr(out, value, indent);
            }
        }
        out.push_str(" }");
    }
}

/// Is this argument a keyword argument (`f(y: 2)` → `Expr::KeywordArg`)?
fn is_keyword_arg(a: &Expr) -> bool {
    matches!(a, Expr::KeywordArg { .. })
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
            let is_class_pred = matches!(meth.as_str(), "is_a?" | "kind_of?" | "instance_of?");
            for (i, a) in args[2..].iter().enumerate() {
                out.push_str(", ");
                match a {
                    // First operand of a class predicate, given as a
                    // constant class name → emit the name as a string.
                    Expr::VarRef {
                        name: cn,
                        scope: Scope::Const,
                        ..
                    } if is_class_pred && i == 0 => {
                        out.push_str(&quote_ts_string(cn));
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
    // method names arrive as `StrLit` args and emit through the normal
    // expression path (`quote_ts_string`), so no source-derived name is ever
    // interpolated raw.
    //
    //   __new__(class, ...ctor_args)          → __SirOop.callNew(class, args…)
    //   __super__(method, class, ...args)     → __SirOop.callSuper(method, class, args…)
    //   __def_method__(class, method, fn)     → __SirOop.defMethod(class, method, fn)
    //   __def_class_method__(class, meth, fn) → __SirOop.defClassMethod(class, meth, fn)
    //   __self__()                            → __SirOop.currentSelfVal()
    //
    // All args are ordinary SIR `Expr`s (`StrLit` for the names, `MakeClosure`
    // for the method body), so a plain `emit_args` is correct and safe.
    if name == "__new__" && !args.is_empty() {
        out.push_str("__SirOop.callNew(");
        emit_args(out, args, indent);
        out.push(')');
        return;
    }
    if name == "__super__" && args.len() >= 2 {
        out.push_str("__SirOop.callSuper(");
        emit_args(out, args, indent);
        out.push(')');
        return;
    }
    if name == "__def_method__" && args.len() == 3 {
        out.push_str("__SirOop.defMethod(");
        emit_args(out, args, indent);
        out.push(')');
        return;
    }
    if name == "__def_class_method__" && args.len() == 3 {
        out.push_str("__SirOop.defClassMethod(");
        emit_args(out, args, indent);
        out.push(')');
        return;
    }
    // Class-method CALL dispatch `Foo.bar(args)` (const receiver) →
    // `__SirOop.callClassMethod("Foo", "bar", args…)` (issue #59, mirrored from
    // the Python backend).  Needed for `extend M`'s methods (registered as class
    // methods) to be callable as `Owner.method`.  The class + method names
    // arrive as `StrLit`s through the normal expression path — never a raw
    // source-derived name (the C3 RCE lesson).
    if name == "__class_method__" && args.len() >= 2 {
        out.push_str("__SirOop.callClassMethod(");
        emit_args(out, args, indent);
        out.push(')');
        return;
    }
    // Mixin directives (MX3).  The Ruby frontend (MX1) lowers `include M` /
    // `extend M` in a class/module body to these two builtins, both carrying the
    // owner and module names as `StrLit`s (never a source-derived value that is
    // interpolated raw — the C3 RCE lesson):
    //
    //   __include__("Owner", "M")  → __SirOop.includeModule("Owner", "M")
    //   __extend__("Owner", "M")   → __SirOop.extendModule("Owner", "M")
    //
    // `includeModule` appends `M` to the owner's include-order list (consulted
    // by the method-resolution walk); `extendModule` copies `M`'s instance
    // methods into the owner's class-method table (`Owner.method`).
    if name == "__include__" && args.len() == 2 {
        out.push_str("__SirOop.includeModule(");
        emit_args(out, args, indent);
        out.push(')');
        return;
    }
    if name == "__extend__" && args.len() == 2 {
        out.push_str("__SirOop.extendModule(");
        emit_args(out, args, indent);
        out.push(')');
        return;
    }
    if name == "__self__" && args.is_empty() {
        out.push_str("__SirOop.currentSelfVal()");
        return;
    }
    // `case_eq` (M5) — Ruby case-equality `pattern === value`, emitted by a
    // `when` clause for range/regex/literal patterns (the class case lowers to
    // `is_a?` via `__method__` instead).  Routes to the OOP runtime helper.
    if name == "case_eq" && args.len() == 2 {
        out.push_str("__SirOop.caseEq(");
        emit_expr(out, &args[0], indent);
        out.push_str(", ");
        emit_expr(out, &args[1], indent);
        out.push(')');
        return;
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
            Some(Expr::VarRef {
                name: cn,
                scope: Scope::Const,
                ..
            }) => {
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
    // `defined?(x)` lowers to `BuiltinCall("defined?", [operand])`.  Ruby's
    // contract: `defined?` **never evaluates its operand** — so we inspect the
    // operand's SIR shape at emit time and emit a constant description string;
    // the operand is never rendered, so it cannot run.  Same shape→description
    // table and v0 simplifications as the Python backend (see its comment and
    // `code/specs/sir-runtime.md`): instance/class/global vars report their
    // static description rather than the runtime nil-when-unset.  Q10h: a
    // method-call operand `recv.meth` (the `__method__` dispatch envelope)
    // reports `"method"` — Ruby's category when the method resolves — instead of
    // the generic `"expression"`; the respond_to?-presence check that would
    // return `nil` for an absent method is the documented method-dispatch
    // boundary.  The non-evaluation contract holds for every shape.
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
        out.push_str(&quote_ts_string(desc));
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
        // `puts` is variadic in the runtime (`puts(...args)`), so a direct
        // `__Sir.puts(a, b)` forwards every argument (Ruby `puts a, b`).
        "puts" => "__Sir.puts",
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
        let with_sep = "a\u{2028}b\u{2029}c\u{0085}d".to_string();
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
        let s = "a\u{2028}b\u{2029}c".to_string();
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
            value: Expr::IntLit {
                value: 0,
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
        emit_expr(
            &mut out,
            &Expr::IntLit {
                value: 42,
                span: s(),
            },
            0,
        );
        assert_eq!(out, "42");
    }

    #[test]
    fn emit_bool_and_nil() {
        let mut a = String::new();
        emit_expr(
            &mut a,
            &Expr::BoolLit {
                value: true,
                span: s(),
            },
            0,
        );
        let mut b = String::new();
        emit_expr(
            &mut b,
            &Expr::BoolLit {
                value: false,
                span: s(),
            },
            0,
        );
        let mut c = String::new();
        emit_expr(&mut c, &Expr::NilLit { span: s() }, 0);
        assert_eq!(a, "true");
        assert_eq!(b, "false");
        assert_eq!(c, "null");
    }

    #[test]
    fn emit_symbol_interns() {
        let mut out = String::new();
        emit_expr(
            &mut out,
            &Expr::SymLit {
                name: "foo".into(),
                span: s(),
            },
            0,
        );
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
                args: vec![Expr::IntLit {
                    value: 7,
                    span: s(),
                }],
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
                args: vec![Expr::IntLit {
                    value: 5,
                    span: s(),
                }],
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
                    value: Expr::IntLit {
                        value: 5,
                        span: s(),
                    },
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
        assert!(out.contains("function id(x: __Sir.Val): __Sir.Val"));
        assert!(out.contains("return x;"));
    }

    #[test]
    fn emit_default_param_referencing_earlier_param() {
        // P2b: `def f(a, b = a + 1); b; end` → TS
        // `function f(a: __Sir.Val, b: __Sir.Val = __Sir.add(a, 1)): __Sir.Val`.
        // The default is inlined natively and references the EARLIER param
        // `a` (valid TS), so omitted trailing args are filled at call time.
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
            body: Block {
                stmts: vec![],
                value: Expr::VarRef {
                    name: "b".into(),
                    scope: Scope::Param,
                    span: s(),
                },
                span: s(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        };
        let mut out = String::new();
        emit_function(&mut out, &f);
        assert!(
            out.contains("function f(a: __Sir.Val, b: __Sir.Val = __Sir.add(a, 1)): __Sir.Val"),
            "got:\n{}",
            out
        );
    }

    #[test]
    fn emit_variadic_params_rest_native_kwrest_object_fallback() {
        // M3: `def f(a, *rest, **opts); end` → TS
        // `function f(a: __Sir.Val, ...rest: __Sir.Val[], opts: __Sir.Val)`.
        // Rest is native JS rest; KwRest has no faithful JS form (no kwargs),
        // so v0 emits it as a trailing ordinary object parameter.
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
        assert!(
            out.contains("function f(a: __Sir.Val, ...rest: __Sir.Val[], opts: __Sir.Val)"),
            "got: {out}"
        );
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
        assert!(out.contains("// Generated by semantic-ir-to-typescript"));
        assert!(out.contains(r#"import * as __Sir from "@coding-adventures/sir-runtime-core";"#));
        assert!(out.contains("function main()"));
        assert!(out.contains("const __sir_result: __Sir.Val = main();"));
        // A non-Ruby module must NOT emit the display-convention setter.
        assert!(
            !out.contains("setDisplayConvention"),
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
            out.contains(r#"__Sir.setDisplayConvention("ruby");"#),
            "ruby module must select the Ruby display convention; got:\n{out}"
        );
    }

    #[test]
    fn emit_pick_global_set_pattern() {
        // The init-statement pattern (BuiltinCall("global_set",
        // SymLit, value)) should render as a direct assignment.
        let e = Expr::BuiltinCall {
            name: "global_set".into(),
            args: vec![
                Expr::SymLit {
                    name: "counter".into(),
                    span: s(),
                },
                Expr::IntLit {
                    value: 0,
                    span: s(),
                },
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

    fn ts_str_lit(v: &str) -> Expr {
        Expr::StrLit {
            value: v.into(),
            span: s(),
        }
    }

    fn ts_builtin(name: &str, args: Vec<Expr>) -> Expr {
        Expr::BuiltinCall {
            name: name.into(),
            args,
            effects: EffectSet::PURE,
            span: s(),
        }
    }

    #[test]
    fn oop_new_emits_call_new_ts() {
        // Dog.new("Rex") → __SirOop.callNew("Dog", "Rex").
        let e = ts_builtin("__new__", vec![ts_str_lit("Dog"), ts_str_lit("Rex")]);
        let mut out = String::new();
        emit_expr(&mut out, &e, 0);
        assert_eq!(out, r#"__SirOop.callNew("Dog", "Rex")"#);
    }

    #[test]
    fn oop_super_emits_call_super_ts() {
        let e = ts_builtin("__super__", vec![ts_str_lit("describe"), ts_str_lit("Cat")]);
        let mut out = String::new();
        emit_expr(&mut out, &e, 0);
        assert_eq!(out, r#"__SirOop.callSuper("describe", "Cat")"#);
    }

    #[test]
    fn oop_def_method_emits_registration_ts() {
        // __def_method__("Dog", "speak", MakeClosure(Dog_speak)) →
        // __SirOop.defMethod("Dog", "speak", new __Sir.Closure(...)).
        let closure = Expr::MakeClosure {
            fn_name: "Dog_speak".into(),
            captures: vec![],
            span: s(),
        };
        let e = ts_builtin(
            "__def_method__",
            vec![ts_str_lit("Dog"), ts_str_lit("speak"), closure],
        );
        let mut out = String::new();
        emit_expr(&mut out, &e, 0);
        assert!(
            out.starts_with(r#"__SirOop.defMethod("Dog", "speak", new __Sir.Closure("#),
            "got: {out}"
        );
    }

    #[test]
    fn oop_def_class_method_emits_registration_ts() {
        let closure = Expr::MakeClosure {
            fn_name: "Counter_zero".into(),
            captures: vec![],
            span: s(),
        };
        let e = ts_builtin(
            "__def_class_method__",
            vec![ts_str_lit("Counter"), ts_str_lit("zero"), closure],
        );
        let mut out = String::new();
        emit_expr(&mut out, &e, 0);
        assert!(
            out.starts_with(r#"__SirOop.defClassMethod("Counter", "zero", new __Sir.Closure("#),
            "got: {out}"
        );
    }

    #[test]
    fn oop_self_emits_current_self_ts() {
        let e = ts_builtin("__self__", vec![]);
        let mut out = String::new();
        emit_expr(&mut out, &e, 0);
        assert_eq!(out, "__SirOop.currentSelfVal()");
    }

    #[test]
    fn class_method_call_emits_call_class_method_ts() {
        // __class_method__("Counter", "zero") → __SirOop.callClassMethod("Counter", "zero").
        let e = ts_builtin(
            "__class_method__",
            vec![ts_str_lit("Counter"), ts_str_lit("zero")],
        );
        let mut out = String::new();
        emit_expr(&mut out, &e, 0);
        assert_eq!(out, r#"__SirOop.callClassMethod("Counter", "zero")"#);
    }

    #[test]
    fn mixin_include_emits_include_module_ts() {
        // __include__("Greeter", "Loud") → __SirOop.includeModule("Greeter", "Loud").
        let e = ts_builtin(
            "__include__",
            vec![ts_str_lit("Greeter"), ts_str_lit("Loud")],
        );
        let mut out = String::new();
        emit_expr(&mut out, &e, 0);
        assert_eq!(out, r#"__SirOop.includeModule("Greeter", "Loud")"#);
    }

    #[test]
    fn mixin_extend_emits_extend_module_ts() {
        // __extend__("Widget", "Describable") → __SirOop.extendModule("Widget", "Describable").
        let e = ts_builtin(
            "__extend__",
            vec![ts_str_lit("Widget"), ts_str_lit("Describable")],
        );
        let mut out = String::new();
        emit_expr(&mut out, &e, 0);
        assert_eq!(out, r#"__SirOop.extendModule("Widget", "Describable")"#);
    }

    #[test]
    fn sym_block_pass_on_dispatch_emits_sym_to_proc() {
        // arr.map(&:to_s) → callMethod(arr, "map", symToProc(intern("to_s")))
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
            r#"__SirOop.callMethod(arr, "map", __SirOop.symToProc(__Sir.intern("to_s")))"#
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
        assert_eq!(out, r#"__SirOop.callMethod(arr, "each", p)"#);
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
        assert_eq!(out, r#"__SirOop.symToProc(__Sir.intern("upcase"))"#);
    }
}
