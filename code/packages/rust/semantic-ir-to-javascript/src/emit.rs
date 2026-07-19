//! JavaScript emitter — walks a SIR module and produces source code.
//!
//! Output layout (per SIR18):
//!
//! ```text
//! // banner comment
//! "use strict";
//! const __Sir = (() => { … })();   // inlined runtime (runtime.rs)
//! let <global> = null;             // module-level globals
//! function <name>(…) { … }         // one per SIR Function
//! _init();                         // if the module declares _init
//! main();                          // if the module declares main
//! ```
//!
//! The emitter is the TypeScript backend stripped of type annotations,
//! with the runtime *inlined* (a `__Sir` IIFE) instead of imported.
//! Each SIR node maps to one JavaScript construct:
//!
//! - A `Block` in **function-body** position becomes a flat
//!   `{ stmts…; return value; }` (no wrapper — it already sits inside a
//!   `function`).
//! - A `Block` in **expression** position becomes an IIFE
//!   `(() => { stmts…; return value; })()` so its `let` bindings don't
//!   leak into the surrounding scope.
//! - `MakeClosure` becomes `new __Sir.Closure((..._a) => fn(caps…, ..._a))`,
//!   prepending the captured values ahead of the call's runtime args.
//!
//! ## SIR16 nodes (this milestone, D4)
//!
//! As of D4 the emitter covers the **full SIR16 / v1 surface**, all of
//! which JavaScript supports natively:
//!
//! - Floats (`FloatLit`) — native `number` (`NaN`/`Infinity` spelled out).
//! - Short-circuit (`LogicalAnd`/`LogicalOr`) — a truthy-guarded arrow
//!   IIFE so the rhs runs only when the lhs decides, routing the test
//!   through `__Sir.truthy` (only `false`/`nil` are falsy).
//! - Sequences (`SeqLit`/`SeqIndex`/`SeqLen`, `SeqSet`) — native arrays
//!   (`[…]`, `a[i]`, `a.length`, `a[i] = v`).
//! - Maps (`MapLit`/`MapGet`, `MapSet`) — native `Map` (`new Map([[k, v]])`,
//!   `.get(k) ?? null`, `.set(k, v)`).
//! - Mutable bindings (`Assign`) — a plain reassignment; `let` (not
//!   `const`) is already the keyword for every binding, so no pre-pass is
//!   needed (unlike the Rust/TypeScript backends).
//! - Loops (`While`/`ForRange`/`ForEach`) — native `while`/`for`/
//!   `for…of`, with the `while`/`for-range` tests routed through
//!   `__Sir.truthy` and a direction-aware, once-evaluated `for-range`.
//!
//! ## Exceptions (E1, SIR17)
//!
//! `Stmt::TryCatch` lowers to a native `try`/`catch`/`finally` whose catch
//! body is a `__Sir.rescueMatches`-guarded if/else-if chain, and the
//! `raise` builtin lowers to `__Sir.raiseError(cls, msg)` — mirroring the
//! TypeScript backend but against the *inlined* exception runtime.  A
//! `ClassDef`'s inheritance edge is collected into one
//! `__Sir.registerAncestry({ … })` at program init so a `rescue
//! StandardError` catches a `raise MyErr` when `class MyErr < StandardError`
//! (E2's JS half); the class body's non-`def` statements are emitted inline.
//!
//! ## Symbolic expressions + pattern/rewrite (SIR23)
//!
//! A `SymSymbol`/`SymRational`/`SymApply`/`SymPatternBlank`/
//! `SymPatternNamed`/`SymRule`/`SymReplaceAll` node lowers to a call into
//! `__Sir.Symbolic.*` — a plain-JS port of the published
//! `sir-runtime-symbolic`/`symbolic-ir`/`cas-pattern-matching` TypeScript
//! packages (see `runtime.rs`), so the artifact stays self-contained.
//! `emit_sym_operand` wraps a bare `IntLit`/`FloatLit`/`StrLit` child
//! through the matching leaf-term constructor before it can sit inside a
//! term tree. Mirrors the TypeScript backend's SIR23 arms exactly, minus
//! the `import`.
//!
//! ## Deferred nodes (still rejected at the capability check)
//!
//! String interpolation (`StrConcat`), the remaining SIR17 OOP scopes
//! (`ModuleDef`, `SingletonClassDef` dispatch, the `Instance`/`ClassVar`
//! scopes), the SIR22 array/matrix domain, and `Intrinsic` are **not
//! emitted**.  Their `Feature`s are absent from the backend's
//! `accepts_features()` list, so a module that uses them is rejected at
//! the capability check *before* lowering — the `panic!` arms below are
//! defence-in-depth that fire only on a backend bug (the accept-set
//! drifting out of sync with what `emit` handles), never on user input.

use std::cell::Cell;
use std::fmt::Write;

use semantic_ir::{
    Block, ElementwiseOpKind, Expr, Function, Global, IndexArg, Module, Param, ParamKind, Scope,
    Stmt,
};

use crate::runtime::RUNTIME;

thread_local! {
    /// Monotonic counter for synthesised loop temporaries (the
    /// once-evaluated `__sir_stop`/`__sir_step` bounds of a `ForRange`).
    /// Reset at the start of every `emit_module` so output stays
    /// deterministic regardless of how many modules a process compiles.
    static LOOP_COUNTER: Cell<usize> = const { Cell::new(0) };
}

fn fresh_loop_id() -> usize {
    LOOP_COUNTER.with(|c| {
        let n = c.get();
        c.set(n + 1);
        n
    })
}

/// Emit a SIR module as JavaScript source.  The caller is responsible
/// for prior validation and capability checks; this function assumes
/// the module is valid and uses only accepted features.
pub fn emit_module(m: &Module) -> String {
    LOOP_COUNTER.with(|c| c.set(0));
    let mut out = String::new();
    emit_banner(&mut out, m);
    // Strict mode goes first (after the banner comment) so the whole
    // file — including the inlined runtime — runs under strict semantics.
    out.push_str("\"use strict\";\n\n");
    // Substitute the display-convention placeholder (SIR display-convention
    // spec): a Ruby-sourced module renders booleans as `true`/`false`; every
    // other source language keeps the default Lisp `#t`/`#f`, so existing Twig
    // output is unchanged.
    //
    // SECURITY: the replacement value MUST remain a hardcoded literal selected
    // by a boolean — never text derived from `source_language` or any other
    // source-controlled field — so this substitution can never inject into the
    // emitted JavaScript.
    let display_ruby = m.metadata.source_language.as_deref() == Some("ruby");
    out.push_str(&RUNTIME.replace(
        "__SIR_DISPLAY_RUBY__",
        if display_ruby { "true" } else { "false" },
    ));
    emit_ancestry_registration(&mut out, m);
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
        "// Generated by semantic-ir-to-javascript v0.1 from SIR module `{}`.",
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
        // Module-level globals start as `null`; their real value is set
        // by the synthesised `_init` function (see `pick_global_set`).
        // `let` (not `const`) because `_init` reassigns them.
        let _ = writeln!(out, "let {} = null;", sanitize_ident(&g.name));
    }
}

fn emit_module_footer(out: &mut String, m: &Module) {
    out.push('\n');
    if m.functions.iter().any(|f| f.name == "_init") {
        out.push_str("_init();\n");
    }
    if m.functions.iter().any(|f| f.name == "main") {
        out.push_str("main();\n");
    }
}

/// Emit the module's user-defined exception-class ancestry (E2, the JS
/// half) once, at program init, as a single `__Sir.registerAncestry({
/// child: "Super", … })` call — right after the runtime and before any
/// user code runs.  This merges the module's `class Child < Super` edges
/// into the runtime's ancestry table so a `rescue StandardError` matches
/// a `raise MyErr` when `class MyErr < StandardError`.
///
/// We register **eagerly at init** rather than at each `ClassDef` site
/// because a `raise`/`rescue` may lexically precede the class definition
/// (Ruby's classes are resolved by name, not by source order); the merged
/// table must already be complete the first time `rescueMatches` runs.
/// If the module defines no inheriting classes, nothing is emitted (a
/// pure non-OOP module gains no init noise).
fn emit_ancestry_registration(out: &mut String, m: &Module) {
    let mut pairs: Vec<(&str, &str)> = Vec::new();
    for f in &m.functions {
        collect_ancestry(&f.body.stmts, &mut pairs);
    }
    if pairs.is_empty() {
        return;
    }
    out.push_str("\n__Sir.registerAncestry({");
    for (i, (child, sup)) in pairs.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        let _ = write!(out, " {}: {}", quote_js_string(child), quote_js_string(sup));
    }
    out.push_str(" });\n");
}

/// Walk a statement list (recursing into every nested body) collecting
/// `(childClass, superclassName)` pairs from each `Stmt::ClassDef` whose
/// `superclass` is `Some`.  Base classes (`class Foo`, no `< Bar`) carry
/// no edge and are skipped.  The recursion covers class bodies, loop /
/// try / rescue / ensure bodies, and both branches of an `If` statement,
/// so a class declared inside any of them is still registered.
fn collect_ancestry<'a>(stmts: &'a [Stmt], pairs: &mut Vec<(&'a str, &'a str)>) {
    for s in stmts {
        match s {
            Stmt::ClassDef {
                name,
                superclass,
                body,
                ..
            } => {
                if let Some(sup) = superclass {
                    pairs.push((name.as_str(), sup.as_str()));
                }
                collect_ancestry(body, pairs);
            }
            Stmt::ModuleDef { body, .. } | Stmt::SingletonClassDef { body, .. } => {
                collect_ancestry(body, pairs);
            }
            Stmt::TryCatch {
                body,
                rescues,
                ensure_body,
                ..
            } => {
                collect_ancestry(body, pairs);
                for r in rescues {
                    collect_ancestry(&r.body, pairs);
                }
                if let Some(ens) = ensure_body {
                    collect_ancestry(ens, pairs);
                }
            }
            Stmt::While { body, .. } | Stmt::ForRange { body, .. } | Stmt::ForEach { body, .. } => {
                collect_ancestry(&body.stmts, pairs)
            }
            _ => {}
        }
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
    // lowerer's MakeClosure convention (`fn(cap0, cap1, …args)`).
    let mut first = true;
    for c in &f.captures {
        if !first {
            out.push_str(", ");
        }
        first = false;
        out.push_str(&sanitize_ident(&c.name));
    }
    // JavaScript has no native keyword-argument call form, so `Keyword`
    // params (`def f(a:)` / `def f(a: 1)`) are lowered — exactly as the
    // TypeScript backend does (spec §4) — to a single trailing
    // **options-object** parameter (`__kw`).  The individual keyword names
    // are recovered in the body prologue by destructuring that object (see
    // `emit_keyword_prologue`).  So here we emit every *non-keyword* param
    // in the signature and, iff the function declares any keyword params, a
    // final `__kw`.
    //
    // Why `__kw` is collision-safe: like the backend's other synthetic
    // names (`__Sir`, `__l`, `__sir_stop_*`), it relies on the convention
    // that SIR-source identifiers do not begin with the `__` runtime
    // prefix — `sanitize_ident` never *produces* a leading `__` (it prefixes
    // `_$`), so no user parameter can sanitize to `__kw`.
    let has_keyword_params = f.params.iter().any(|p| p.kind == ParamKind::Keyword);
    for p in &f.params {
        match p.kind {
            // Keyword params are not emitted positionally; they are folded
            // into the single trailing `__kw` object emitted after the loop.
            ParamKind::Keyword => continue,
            // `*rest` → native JS rest parameter.
            ParamKind::Rest => {
                if !first {
                    out.push_str(", ");
                }
                first = false;
                out.push_str("...");
                out.push_str(&sanitize_ident(&p.name));
            }
            // `**opts` has no native JS form (no keyword-argument call);
            // v0 binds it as a trailing ordinary parameter — but this
            // backend does not accept the features that produce KwRest
            // yet, so in practice only `Required` is reached here.
            ParamKind::Required | ParamKind::KwRest => {
                if !first {
                    out.push_str(", ");
                }
                first = false;
                out.push_str(&sanitize_ident(&p.name));
                // P2d: a defaulted param (`Param { default: Some(e) }`)
                // becomes a native JS default parameter `name = <e>`.
                //
                // JavaScript's default-parameter semantics are *exactly*
                // SIR's: the default expression is evaluated **at call
                // time**, only when the argument is omitted, in **param
                // scope** — so a later param's default may reference an
                // earlier param by its bare name (valid JS, since earlier
                // params are already in scope left-to-right).  No runtime
                // helper is needed; the strategy is a direct native inline.
                if let Some(default) = &p.default {
                    out.push_str(" = ");
                    emit_expr(out, default, 2);
                }
            }
        }
    }
    if has_keyword_params {
        if !first {
            out.push_str(", ");
        }
        out.push_str("__kw");
    }
    out.push_str(") {\n");
    if has_keyword_params {
        emit_keyword_prologue(out, &f.params, 2);
    }
    emit_function_body(out, &f.body, 2);
    out.push_str("}\n");
}

/// Emit the body prologue that unpacks the trailing `__kw` options object
/// into the function's keyword-param locals (KW4).
///
/// For `def f(a, b:, c: 1)` this emits, at the top of the body:
///
/// ```js
///   const { b, c = 1 } = __kw ?? {};
/// ```
///
/// - A **required** keyword (`Keyword`, `default == None`) destructures to a
///   bare `name` — the validator has already guaranteed the caller supplied
///   it, so the key is always present.
/// - An **optional** keyword (`Keyword`, `default == Some(e)`) destructures
///   with a JS default `name = <e>`, which fills in when the caller omits
///   the key (property is `undefined`).  JS destructuring defaults fire on
///   `undefined` exactly like SIR optional-keyword semantics.
/// - The `?? {}` guard means a callee with only optional keywords still
///   works when the caller passes no options object at all (`f(1)`).
///
/// The **object key** is the raw source `name` (it must match the key the
/// call site writes); the **bound local** is `sanitize_ident(name)`.  When
/// they differ we emit the explicit `{ key: local }` rename form.
fn emit_keyword_prologue(out: &mut String, params: &[Param], indent: usize) {
    let pad = " ".repeat(indent);
    let _ = write!(out, "{pad}const {{ ");
    let mut first = true;
    for p in params.iter().filter(|p| p.kind == ParamKind::Keyword) {
        if !first {
            out.push_str(", ");
        }
        first = false;
        let local = sanitize_ident(&p.name);
        if local == p.name {
            // Shorthand `{ name }` / `{ name = default }` — key and binding
            // coincide because the source name is already a valid JS ident.
            out.push_str(&local);
        } else {
            // Rename `{ "raw key": local }` — the object key stays the raw
            // source name so it lines up with the call site, but the local
            // binding is the sanitized identifier.
            out.push_str(&quote_js_string(&p.name));
            out.push_str(": ");
            out.push_str(&local);
        }
        if let Some(default) = &p.default {
            out.push_str(" = ");
            emit_expr(out, default, indent);
        }
    }
    out.push_str(" } = __kw ?? {};\n");
}

/// Emit a block as a **function body** — a flat statement list followed
/// by `return <value>;`.  No IIFE wrapper: it already lives inside a
/// `function { … }`.
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
        // `let <name> = <value>;`.  Both `let` and `let*` bindings emit
        // a JS `let` — the lowerer already ordered `let*`'s sequential
        // dependencies, so a top-down emission is faithful.  We use
        // `let` (not `const`) uniformly so a later `Assign` over the same
        // name reassigns a mutable binding (the `MutableBindings` feature)
        // without any const→let pre-pass — JS `let` is reassignable, so
        // unlike the Rust/TypeScript backends this backend needs none.
        Stmt::LetBinding { name, value, .. } | Stmt::LetStarBinding { name, value, .. } => {
            let _ = write!(out, "{}let {} = ", pad, sanitize_ident(name));
            emit_expr(out, value, indent);
            out.push_str(";\n");
        }
        Stmt::ExprStmt { expr, .. } => {
            // Recognise the synthesised `_init` global-set pattern and
            // emit a direct assignment for nicer output.
            if let Some((global, value)) = pick_global_set(expr) {
                let _ = write!(out, "{}{} = ", pad, sanitize_ident(global));
                emit_expr(out, value, indent);
                out.push_str(";\n");
            } else {
                out.push_str(&pad);
                emit_expr(out, expr, indent);
                out.push_str(";\n");
            }
        }
        // ── SIR16: mutation (MutableBindings) ───────────────────────
        // `Assign` re-binds an already-declared name.  Local/Param/
        // Capture/Global all resolve to a bare identifier in JS (globals
        // are module-level `let`s), so a plain reassignment is faithful.
        // The Instance/ClassVar/Const scopes are SIR17 features this
        // backend does not accept, so they fall through to the panic guard.
        Stmt::Assign {
            name,
            scope: Scope::Local | Scope::Param | Scope::Capture | Scope::Global,
            value,
            ..
        } => {
            let _ = write!(out, "{}{} = ", pad, sanitize_ident(name));
            emit_expr(out, value, indent);
            out.push_str(";\n");
        }
        // ── OOP (O3): instance / class variable writes ──────────────
        // `@x = v` / `@@x = v` store onto the current `self`'s (or its
        // class's) prototype-less bag via the OOP runtime.  The `@`/`@@`
        // sigil is preserved in `name` and used verbatim as the bag key.
        Stmt::Assign {
            name,
            scope: Scope::Instance,
            value,
            ..
        } => {
            let _ = write!(out, "{}__Sir.ivarSet({}, ", pad, quote_js_string(name));
            emit_expr(out, value, indent);
            out.push_str(");\n");
        }
        Stmt::Assign {
            name,
            scope: Scope::ClassVar,
            value,
            ..
        } => {
            let _ = write!(out, "{}__Sir.cvarSet({}, ", pad, quote_js_string(name));
            emit_expr(out, value, indent);
            out.push_str(");\n");
        }
        Stmt::Assign { scope, span, .. } => {
            panic!(
                "javascript backend reached a deferred `Assign` scope `{}` at {span} — not accepted yet",
                scope.name()
            );
        }
        // ── SIR16: loops (Loops) ────────────────────────────────────
        // `while (truthy(cond)) { body }` — the test routes through SIR
        // truthiness (only `false`/`nil` are falsy), never JS truthiness.
        Stmt::While { cond, body, .. } => {
            out.push_str(&pad);
            out.push_str("while (__Sir.truthy(");
            emit_expr(out, cond, indent);
            out.push_str(")) {\n");
            emit_block_as_stmts(out, body, indent + 2);
            let _ = writeln!(out, "{pad}}}");
        }
        // `for (var = start; …; var += step) { body }` — half-open
        // (`stop` exclusive).  `stop`/`step` are evaluated ONCE into
        // block-scoped temporaries (matching Python's `range`), and the
        // loop condition is direction-aware so a negative `step` counts
        // down correctly.  JS has one numeric type, so no casts are
        // needed (unlike the TypeScript backend's `as number`).
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
            let _ = writeln!(out, "{pad}{{");
            let _ = write!(out, "{inner_pad}let {v} = ");
            emit_expr(out, start, inner);
            out.push_str(";\n");
            let _ = write!(out, "{inner_pad}const __sir_stop_{id} = ");
            emit_expr(out, stop, inner);
            out.push_str(";\n");
            let _ = write!(out, "{inner_pad}const __sir_step_{id} = ");
            emit_expr(out, step, inner);
            out.push_str(";\n");
            let _ = writeln!(
                out,
                "{inner_pad}while (__sir_step_{id} >= 0 ? {v} < __sir_stop_{id} : {v} > __sir_stop_{id}) {{"
            );
            emit_block_as_stmts(out, body, inner + 2);
            let _ = writeln!(out, "{}{v} = {v} + __sir_step_{id};", " ".repeat(inner + 2));
            let _ = writeln!(out, "{inner_pad}}}");
            let _ = writeln!(out, "{pad}}}");
        }
        // `for (const var of iter) { body }` — iterate a Seq.  `let` is
        // used so a body that reassigns the loop variable still works.
        Stmt::ForEach {
            var, iter, body, ..
        } => {
            let _ = write!(out, "{}for (let {} of ", pad, sanitize_ident(var));
            emit_expr(out, iter, indent);
            out.push_str(") {\n");
            emit_block_as_stmts(out, body, indent + 2);
            let _ = writeln!(out, "{pad}}}");
        }
        // ── SIR16: indexed assignment (Sequences / Maps) ────────────
        // `seq[index] = value;` — native array element write.
        Stmt::SeqSet {
            seq, index, value, ..
        } => {
            out.push_str(&pad);
            out.push('(');
            emit_expr(out, seq, indent);
            out.push_str(")[");
            emit_expr(out, index, indent);
            out.push_str("] = ");
            emit_expr(out, value, indent);
            out.push_str(";\n");
        }
        // `map.set(key, value);` — native `Map.set`.
        Stmt::MapSet {
            map, key, value, ..
        } => {
            out.push_str(&pad);
            out.push('(');
            emit_expr(out, map, indent);
            out.push_str(").set(");
            emit_expr(out, key, indent);
            out.push_str(", ");
            emit_expr(out, value, indent);
            out.push_str(");\n");
        }
        // ── SIR17: class / module / singleton declarations ─────────
        // The Ruby→SIR frontend hoists method `def`s to top-level
        // functions, so a `ClassDef` body carries only its non-`def`
        // statements (constant / class-variable assigns).  This backend
        // does not model OOP method dispatch or instantiation; a class is
        // accepted only for its *ancestry edge* (E2) — the `superclass`
        // pair is collected separately (see `collect_ancestry`) and
        // emitted once as `__Sir.registerAncestry({ … })` at program init.
        // Here we just emit the body statements in source order.
        Stmt::ClassDef { body, .. }
        | Stmt::ModuleDef { body, .. }
        | Stmt::SingletonClassDef { body, .. } => {
            for st in body {
                emit_stmt(out, st, indent);
            }
        }
        // `begin … rescue … ensure … end` → native `try { … } catch
        // (__exc) { … } finally { … }`.  A native `catch` binds *one*
        // variable and catches *everything*, while Ruby has an ordered
        // list of typed `rescue` clauses, so the catch body is an
        // if/else-if chain that asks the runtime `rescueMatches(exc,
        // [class names])` for each clause in source order and re-`throw`s
        // if none match (matching Ruby's "propagate when unrescued").
        // Mirrors the TypeScript backend's `TryCatch` arm exactly, minus
        // the type annotation on the `=> e` binding.
        Stmt::TryCatch {
            body,
            rescues,
            ensure_body,
            ..
        } => {
            let _ = writeln!(out, "{pad}try {{");
            for st in body {
                emit_stmt(out, st, indent + 2);
            }
            let _ = write!(out, "{pad}}}");
            if !rescues.is_empty() {
                out.push_str(" catch (__exc) {\n");
                let inner = indent + 2;
                let ipad = " ".repeat(inner);
                for (i, r) in rescues.iter().enumerate() {
                    // Build the `["Foo", "Bar"]` class-name array; an empty
                    // list is a bare `rescue` (catch-all) → `[]`.
                    let mut types = String::from("[");
                    for (j, t) in r.exception_types.iter().enumerate() {
                        if j > 0 {
                            types.push_str(", ");
                        }
                        types.push_str(&quote_js_string(t));
                    }
                    types.push(']');
                    let kw = if i == 0 { "if" } else { "} else if" };
                    let _ = writeln!(out, "{ipad}{kw} (__Sir.rescueMatches(__exc, {types})) {{");
                    // `rescue Foo => e` binds the caught value as a local.
                    if let Some(bind) = &r.binding {
                        let _ = writeln!(out, "{ipad}  const {} = __exc;", sanitize_ident(bind));
                    }
                    for st in &r.body {
                        emit_stmt(out, st, inner + 2);
                    }
                }
                // No clause matched → propagate the original exception.
                let _ = writeln!(out, "{ipad}}} else {{");
                let _ = writeln!(out, "{ipad}  throw __exc;");
                let _ = writeln!(out, "{ipad}}}");
                let _ = write!(out, "{pad}}}");
            }
            if let Some(ens) = ensure_body {
                out.push_str(" finally {\n");
                for st in ens {
                    emit_stmt(out, st, indent + 2);
                }
                let _ = write!(out, "{pad}}}");
            }
            out.push('\n');
        }
        // ── SIR22: array/matrix indexed assignment ──────────────────
        // `target[indices...] = value;` — mutates in place via
        // `__Sir.Array.indexSet` (see `runtime.rs`'s "array/matrix
        // domain" section), matching the SIR22 spec's own note that
        // `IndexSet` is a `Stmt`, not a pure `Expr`, for exactly this
        // in-place-mutation reason.
        Stmt::IndexSet {
            target,
            indices,
            value,
            ..
        } => {
            out.push_str(&pad);
            out.push_str("__Sir.Array.indexSet(");
            emit_expr(out, target, indent);
            out.push_str(", [");
            emit_index_args(out, indices, indent);
            out.push_str("], ");
            emit_expr(out, value, indent);
            out.push_str(");\n");
        }
    }
}

/// The JS string `__Sir.Array.elementwise`'s `applyOp` switches on —
/// exact `ElementwiseOpKind` variant names, not `.name()`'s lowercase
/// forms (`"add"`, etc., used elsewhere for e.g. debug/display), since
/// this string is a real runtime dispatch key, not a cosmetic label.
fn elementwise_op_js_name(op: ElementwiseOpKind) -> &'static str {
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

/// Emit one `IndexArg` as the JS object literal `__Sir.Array.indexGet`/
/// `indexSet` expect: `{ kind: "scalar", value }` / `{ kind: "whole" }` /
/// `{ kind: "range", indices: <NDArray> }`. The `Range` case reuses
/// `emit_expr` on the inner `Expr::Range` node directly — that node's own
/// `Expr::Range` arm already emits a call into `__Sir.Array.range(...)`,
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

/// Detect the `BuiltinCall("global_set", SymLit(name), value)` pattern
/// and return the global name + value if so.  The `_init` function
/// builds module globals through this builtin; we render it as a direct
/// `name = value;` assignment instead of a runtime call.
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
            let _ = write!(out, "{value}");
        }
        Expr::FloatLit { value, .. } => {
            // A Ruby `Float` literal is minted through `mkFloat`, which boxes
            // an integral value (`7.0` — otherwise indistinguishable from the
            // Integer `7`) and leaves a non-integral one native (`3.5`).  This
            // is where the Integer-vs-Float tag is BORN; every downstream
            // numeric helper unwraps via `numOf` and re-tags via `mkFloat`.
            let _ = write!(out, "__Sir.mkFloat({})", format_float(*value));
        }
        Expr::BoolLit { value, .. } => {
            out.push_str(if *value { "true" } else { "false" });
        }
        Expr::NilLit { .. } => out.push_str("null"),
        Expr::SymLit { name, .. } => {
            let _ = write!(out, "__Sir.intern({})", quote_js_string(name));
        }
        Expr::StrLit { value, .. } => {
            out.push_str(&quote_js_string(value));
        }
        Expr::VarRef { name, scope, .. } => emit_var_ref(out, name, *scope),
        // `(__Sir.truthy(cond) ? then : else)` — the test routes through
        // SIR truthiness (only `false`/`nil` are falsy), never JS's.
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
        // A statically-known call.  P2d: the validator allows a caller to
        // omit trailing defaulted args (arity ≥ `required_param_count`), so
        // we emit ONLY the args that are present and do NOT pad — native JS
        // default parameters (emitted on the callee in `emit_function`)
        // fill the omitted trailing params at call time.  IndirectCall and
        // closure defaults are unchanged / deferred.
        Expr::DirectCall { fn_name, args, .. } => {
            let _ = write!(out, "{}(", sanitize_ident(fn_name));
            emit_call_args(out, args, indent);
            out.push(')');
        }
        // `__Sir.applyClosure(target, [args…])` — invokes a first-class
        // closure value, distinct from a statically-known `DirectCall`.
        Expr::IndirectCall { target, args, .. } => {
            out.push_str("__Sir.applyClosure(");
            emit_expr(out, target, indent);
            out.push_str(", [");
            emit_call_args(out, args, indent);
            out.push_str("])");
        }
        Expr::BuiltinCall { name, args, .. } => emit_builtin_call(out, name, args, indent),
        // `new __Sir.Closure((..._a) => fn(cap0, cap1, ..._a))`.
        Expr::MakeClosure {
            fn_name, captures, ..
        } => {
            out.push_str("new __Sir.Closure((..._a) => ");
            let _ = write!(out, "{}(", sanitize_ident(fn_name));
            for c in captures {
                emit_expr(out, &c.value, indent);
                out.push_str(", ");
            }
            out.push_str("..._a))");
        }
        // ── SIR16: sequences (Sequences) — native arrays ──────────────
        Expr::SeqLit { items, .. } => {
            out.push('[');
            emit_args(out, items, indent);
            out.push(']');
        }
        Expr::SeqIndex { seq, index, .. } => {
            out.push('(');
            emit_expr(out, seq, indent);
            out.push_str(")[");
            emit_expr(out, index, indent);
            out.push(']');
        }
        Expr::SeqLen { seq, .. } => {
            out.push('(');
            emit_expr(out, seq, indent);
            out.push_str(").length");
        }
        // ── SIR16: maps (Maps) — native `Map` ─────────────────────────
        Expr::MapLit { entries, .. } => {
            out.push_str("new Map([");
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
        // `map.get(key) ?? null` — a missing key reads as nil (`null`),
        // matching the spec's target-defined miss behaviour.
        Expr::MapGet { map, key, .. } => {
            out.push_str("((");
            emit_expr(out, map, indent);
            out.push_str(").get(");
            emit_expr(out, key, indent);
            out.push_str(") ?? null)");
        }
        // ── SIR16: short-circuit (ShortCircuit) ───────────────────────
        // An arrow IIFE keeps the rhs unevaluated until the lhs decides,
        // routing the test through SIR truthiness (only `false`/`nil` are
        // falsy — not `0`/`""`).  The param `__l` gives each occurrence
        // its own scope, so nested `&&`/`||` never collide.
        Expr::LogicalAnd { lhs, rhs, .. } => {
            out.push_str("((__l) => __Sir.truthy(__l) ? (");
            emit_expr(out, rhs, indent);
            out.push_str(") : __l)(");
            emit_expr(out, lhs, indent);
            out.push(')');
        }
        Expr::LogicalOr { lhs, rhs, .. } => {
            out.push_str("((__l) => __Sir.truthy(__l) ? __l : (");
            emit_expr(out, rhs, indent);
            out.push_str("))(");
            emit_expr(out, lhs, indent);
            out.push(')');
        }
        // ── Deferred expression kinds (rejected at capability check) ──
        // `StrConcat` (SIR18 string interpolation) is not accepted yet.
        Expr::StrConcat { span, .. } => {
            panic!(
                "javascript backend reached a deferred `StrConcat` at {span} — not accepted yet"
            );
        }
        Expr::Intrinsic { name, span, .. } => {
            // The v0 backend accepts no intrinsics; `check_module`
            // rejects them before lowering.  Panic here to catch a
            // backend bug if that check ever drifts.
            panic!(
                "emit reached an Intrinsic `{name}` at {span} — backend should have rejected it"
            );
        }
        // A keyword argument (`f(a: 1)`) is **not** a first-class value: the
        // validator guarantees it appears only inside a call's `args` vec,
        // and `emit_call_args` peels every `KeywordArg` off into the trailing
        // options object *before* recursing into `emit_expr`.  Reaching this
        // arm therefore means a backend bug (a `KeywordArg` somewhere it was
        // never meant to be), so we panic with the offending span.
        Expr::KeywordArg { span, .. } => {
            panic!("javascript backend reached a `KeywordArg` outside call-argument position at {span} — this is a backend bug (keyword args are collapsed by emit_call_args)");
        }
        // ── SIR22: array/matrix nodes (base cut) ──────────────────────
        // Real codegen: calls into the inlined `__Sir.Array.*` sub-runtime
        // (see `runtime.rs`'s "array/matrix domain" section) — a plain-JS
        // port of `sir-runtime-array`, mirroring how the SIR23 `Sym*` arms
        // above call into `__Sir.Symbolic.*`. `rows` is row-major in the
        // literal syntax (per the SIR22 spec); `__Sir.Array.fromRows`
        // reconciles that with column-major storage, so the emitter just
        // nests the row/element expressions unchanged.
        Expr::ArrayLit { rows, .. } => {
            out.push_str("__Sir.Array.fromRows([");
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
        // `__Sir.Array.range(start, stop, step)` — note the argument
        // ORDER: the SIR node's own field order is `start, step, stop`,
        // but `sir-runtime-array`'s `range(start, stop, step = 1)`
        // (and this runtime's port of it) takes `stop` before `step`.
        Expr::Range {
            start, step, stop, ..
        } => {
            out.push_str("__Sir.Array.range(");
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
            out.push_str("__Sir.Array.matmul(");
            emit_expr(out, lhs, indent);
            out.push_str(", ");
            emit_expr(out, rhs, indent);
            out.push(')');
        }
        // The op name must match `applyOp`'s switch in `runtime.rs`
        // exactly (`"Add"`, not `.name()`'s lowercase `"add"`).
        Expr::ElementwiseOp { op, lhs, rhs, .. } => {
            let _ = write!(
                out,
                "__Sir.Array.elementwise({}, ",
                quote_js_string(elementwise_op_js_name(*op))
            );
            emit_expr(out, lhs, indent);
            out.push_str(", ");
            emit_expr(out, rhs, indent);
            out.push(')');
        }
        Expr::Transpose {
            target, conjugate, ..
        } => {
            out.push_str("__Sir.Array.transpose(");
            emit_expr(out, target, indent);
            let _ = write!(out, ", {conjugate})");
        }
        Expr::IndexGet {
            target, indices, ..
        } => {
            out.push_str("__Sir.Array.indexGet(");
            emit_expr(out, target, indent);
            out.push_str(", [");
            emit_index_args(out, indices, indent);
            out.push_str("])");
        }
        // ── SIR22 addendum: APL primitive operators (still deferred) ──
        // `Reduce`/`Scan`/`OuterProduct`/`Shape`/`Reshape`/
        // `IndexGenerator`/`IndexOf`/`Ravel`/`Catenate` observe the SAME
        // `NDArrays`/`MatrixOps`/`ArrayColumnMajor` features as the base
        // cut above (the SIR22 addendum gives them no feature flag of
        // their own), so accepting those features for the base cut also
        // lets a module using these nine past the capability check —
        // unlike every other still-deferred arm in this function, these
        // nine are NOT purely a "capability check ever drifts" guard.
        // This is safe today only because no frontend crate emits any of
        // these nine `Expr` variants yet (`apl-to-semantic-ir` does not
        // lower APL's reduce/scan/outer-product/shape/reshape/iota/
        // index-of/ravel/catenate operators to them — see the SIR22 spec's
        // addendum and `sir-runtime-array`'s own README, which scoped them
        // out of the runtime package for the identical reason). Wiring
        // real codegen for these requires porting `array_runtime::ops::
        // {reduce,scan,outer}` and `apl-runtime::builtins`'s bespoke
        // shape/reshape/iota/index-of/ravel/catenate logic into this
        // runtime first — a natural, cleanly-scoped follow-up once a
        // frontend actually needs them, not part of this PR.
        Expr::Reduce { span, .. }
        | Expr::Scan { span, .. }
        | Expr::OuterProduct { span, .. }
        | Expr::Shape { span, .. }
        | Expr::Reshape { span, .. }
        | Expr::IndexGenerator { span, .. }
        | Expr::IndexOf { span, .. }
        | Expr::Ravel { span, .. }
        | Expr::Catenate { span, .. }
        // SIR26 `Convert` — `Conversions` not accepted; unreachable in a
        // validated module (capability check rejects it).
        | Expr::Convert { span, .. } => {
            panic!(
                "javascript backend reached a deferred SIR22/SIR26 expression ({}) at {span} — not accepted yet",
                e.kind_name()
            );
        }
        // ── SIR23: symbolic expression + pattern/rewrite nodes ────────
        // Mirrors the TypeScript backend's SIR23 arms exactly, but calls
        // into the INLINED `__Sir.Symbolic.*` runtime (runtime.rs) rather
        // than an imported `@coding-adventures/sir-runtime-symbolic`.
        Expr::SymSymbol { name, .. } => {
            out.push_str("__Sir.Symbolic.sym(");
            out.push_str(&quote_js_string(name));
            out.push(')');
        }
        Expr::SymRational { numer, denom, .. } => {
            let _ = write!(out, "__Sir.Symbolic.rational({numer}, {denom})");
        }
        Expr::SymApply { head, args, .. } => {
            out.push_str("__Sir.Symbolic.apply(");
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
            out.push_str("__Sir.Symbolic.blank()");
        }
        Expr::SymPatternBlank {
            head: Some(head), ..
        } => match head.as_ref() {
            Expr::SymSymbol { name, .. } => {
                out.push_str("__Sir.Symbolic.blankTyped(");
                out.push_str(&quote_js_string(name));
                out.push(')');
            }
            _ => panic!(
                "javascript backend: SymPatternBlank's head-constraint must be a SymSymbol, got {} at {}",
                head.kind_name(),
                head.span()
            ),
        },
        Expr::SymPatternNamed { name, pattern, .. } => {
            out.push_str("__Sir.Symbolic.named(");
            out.push_str(&quote_js_string(name));
            out.push_str(", ");
            emit_sym_operand(out, pattern, indent);
            out.push(')');
        }
        Expr::SymRule {
            lhs, rhs, delayed, ..
        } => {
            out.push_str(if *delayed {
                "__Sir.Symbolic.ruleDelayed("
            } else {
                "__Sir.Symbolic.rule("
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
            out.push_str("__Sir.Symbolic.unwrap(");
            out.push_str(if *repeated {
                "__Sir.Symbolic.replaceRepeated("
            } else {
                "__Sir.Symbolic.replaceAll("
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

/// Wrap a `SymApply`/`SymRule`/`SymReplaceAll` operand that is a bare
/// literal (`IntLit`/`FloatLit`/`StrLit`) through the matching
/// `__Sir.Symbolic.*` leaf-term constructor — a raw JS number/string is
/// never a valid Symbolic term, so it must become one before it can sit
/// inside a term tree. Any other operand (already a Symbolic-producing
/// expression, e.g. a nested `SymApply` or a `VarRef`) emits unchanged.
/// Mirrors the TypeScript backend's identically-named helper exactly.
fn emit_sym_operand(out: &mut String, e: &Expr, indent: usize) {
    match e {
        Expr::IntLit { .. } => {
            out.push_str("__Sir.Symbolic.int(");
            emit_expr(out, e, indent);
            out.push(')');
        }
        Expr::FloatLit { value, .. } => {
            // The Symbolic constructors want a RAW number to wrap into a term
            // (`{kind:"float", value}`), not a tagged-float box — so emit the
            // bare literal here rather than routing through `mkFloat`.
            let _ = write!(out, "__Sir.Symbolic.numberNode({})", format_float(*value));
        }
        Expr::StrLit { .. } => {
            out.push_str("__Sir.Symbolic.stringNode(");
            emit_expr(out, e, indent);
            out.push(')');
        }
        _ => emit_expr(out, e, indent),
    }
}

fn emit_var_ref(out: &mut String, name: &str, scope: Scope) {
    match scope {
        // Local, Param, Capture, and Global all resolve to a bare
        // identifier (globals are module-level `let`s).
        Scope::Local | Scope::Param | Scope::Capture | Scope::Global => {
            out.push_str(&sanitize_ident(name));
        }
        // A builtin referenced as a *value* (not called) → a closure
        // wrapping the dispatch-table entry.
        Scope::Builtin => {
            let _ = write!(out, "__Sir.builtinClosure({})", quote_js_string(name));
        }
        // A `Const` reference (`Feature::Constants`) resolves to a bare
        // identifier — a top-level `const`/`let` binding the frontend
        // emitted for the constant.  In practice the dominant `Const` use
        // reaching this backend is an exception *class name* as the first
        // argument of `raise`, which the `raise` builtin arm consumes as a
        // *string* (it never calls `emit_expr` on that Const), so a class
        // name never needs a real binding.  Any other `Const` — a genuine
        // named constant read for its value — emits its bare name.
        Scope::Const => {
            out.push_str(&sanitize_ident(name));
        }
        // ── OOP (O3): instance / class variable reads ────────────────
        // `@x` and `@@x` read from the current `self` (the receiver a
        // running method pushed).  The `@`/`@@` sigil is preserved in
        // `name`, so it becomes the literal key string the runtime bag
        // is keyed on.  A read outside any method reads nil (`null`),
        // matching Ruby's "no prior declaration" rule for these scopes.
        Scope::Instance => {
            let _ = write!(out, "__Sir.ivarGet({})", quote_js_string(name));
        }
        Scope::ClassVar => {
            let _ = write!(out, "__Sir.cvarGet({})", quote_js_string(name));
        }
    }
}

/// Emit a comma-separated argument / element list.
fn emit_args(out: &mut String, args: &[Expr], indent: usize) {
    for (i, a) in args.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        emit_expr(out, a, indent);
    }
}

/// Emit a **call**'s argument list (KW4), splitting positional args from
/// keyword args.
///
/// A call's `args` vec holds positionals first, then zero or more
/// `Expr::KeywordArg` (the validator enforces this ordering and rejects
/// duplicate keyword names).  JavaScript has no keyword-call syntax, so —
/// mirroring the def-side `__kw` options object and the TypeScript backend
/// (spec §4) — every `KeywordArg` collapses into a single trailing object
/// literal:
///
/// ```text
///   f(1, a: 2, b: 3)   →   f(1, { a: 2, b: 3 })
///   f(1)               →   f(1)                    // no keyword args → no object
///   f(a: 2)            →   f({ a: 2 })
/// ```
///
/// The object key is the raw keyword `name` — the exact string the callee's
/// destructuring prologue (`emit_keyword_prologue`) reads — so the two sides
/// line up regardless of identifier sanitisation.
fn emit_call_args(out: &mut String, args: &[Expr], indent: usize) {
    // Positionals are every arg that is not a keyword.  Because the
    // validator forbids a positional after a keyword, this is exactly the
    // leading run of the vec; a `filter` is equivalent and order-preserving.
    let positionals = args
        .iter()
        .filter(|a| !matches!(a, Expr::KeywordArg { .. }));
    let mut wrote_any = false;
    for a in positionals {
        if wrote_any {
            out.push_str(", ");
        }
        wrote_any = true;
        emit_expr(out, a, indent);
    }

    let mut first_kw = true;
    for a in args {
        if let Expr::KeywordArg { name, value, .. } = a {
            if first_kw {
                // Open the single trailing options object, preceded by a
                // comma iff any positional args were already written.
                if wrote_any {
                    out.push_str(", ");
                }
                out.push_str("{ ");
                first_kw = false;
            } else {
                out.push_str(", ");
            }
            // `key: value`.  A raw source name that is a valid JS ident can
            // be a bare property key; otherwise quote it.  Either form is
            // read back identically by the destructuring prologue.
            if is_valid_js_ident(name) {
                out.push_str(name);
            } else {
                out.push_str(&quote_js_string(name));
            }
            out.push_str(": ");
            emit_expr(out, value, indent);
        }
    }
    if !first_kw {
        out.push_str(" }");
    }
}

/// Emit a `BuiltinCall`, specialising the common arithmetic /
/// comparison / IO builtins to native JavaScript and routing anything
/// else through the runtime dispatch table.
///
/// Per the spec's "Builtin specialisation" table:
///
/// | builtin (2-arg) | native JS |
/// |-----------------|-----------|
/// | `+ *`           | `__Sir.plus(a, b)` / `__Sir.times(a, b)` (polymorphic) |
/// | `- / %`         | `(a - b)` … |
/// | `= != < > <= >=`| `(a === b)` … |
/// | `not` (1-arg)   | `(!__Sir.truthy(a))` |
/// | `neg` (1-arg)   | `(-(a))` |
/// | `len` (1-arg)   | `(a).length` |
/// | `print` (1-arg) | `__Sir.print` helper |
///
/// `+` and `*` are Ruby-POLYMORPHIC: besides numeric add/mul they do
/// String/Array concat, repeat, and join (see the
/// sir-polymorphic-operators spec).  Native JS infix would be *wrong* for
/// the collection arms (`[1] + [2]` is the string `"1,2"`, `"ab" * 3` is
/// `NaN`), so even the 2-arg form routes through the inlined
/// `__Sir.plus` / `__Sir.times` helpers, which dispatch on the first
/// operand's runtime type and preserve the exact old numeric fold.
///
/// A variadic arithmetic operator (`(+ 1 2 3)`) has *more* than two
/// args, so it falls through to `__Sir.callBuiltin("+", […])` — the
/// dispatch table folds it (and now dispatches polymorphically too).  `global_set`/`global_get` route to their
/// runtime helpers; an unknown builtin lands on `callBuiltin` so a new
/// builtin needs no backend change to *run* (only to be idiomatic).
/// Emit a class- or method-*name* operand for an OOP builtin
/// (`__new__` / `__super__` / `__def_method__` / …).  The frontend
/// passes these as a `StrLit` (the literal name) or — for a class
/// operand written as a bare constant like `Dog.new` — a `Const`-scoped
/// `VarRef`.  In BOTH cases we emit the name as a *string literal* (via
/// [`quote_js_string`]): the OOP runtime keys its method tables and its
/// ancestry map on the class/method *name string*, never on a live
/// binding, so a class name never needs (or gets) a real JS variable.
/// Any other expression is emitted normally as a fallback (e.g. a
/// computed name), so the builtin still lowers to *something* runnable.
fn emit_oop_name_arg(out: &mut String, e: &Expr) {
    match e {
        Expr::StrLit { value, .. } => out.push_str(&quote_js_string(value)),
        Expr::VarRef {
            name,
            scope: Scope::Const,
            ..
        } => out.push_str(&quote_js_string(name)),
        other => emit_expr(out, other, 0),
    }
}

fn emit_builtin_call(out: &mut String, name: &str, args: &[Expr], indent: usize) {
    // Method dispatch (`recv.meth(args…)` → `BuiltinCall("__method__",
    // [recv, StrLit("meth"), args…])`, produced by the Ruby/JS frontends)
    // routes to the runtime's `callMethod(recv, name, args…)`, which applies
    // the JS-native method (arrays' `push`/`map`/… or strings'
    // `toUpperCase`/…) and unwraps any `Closure` callback argument.  The
    // method name is always a `StrLit` at `args[1]`.
    if name == "__method__" && args.len() >= 2 {
        out.push_str("__Sir.callMethod(");
        emit_expr(out, &args[0], indent); // receiver
        for a in &args[1..] {
            out.push_str(", ");
            emit_expr(out, a, indent); // StrLit(method), then call args
        }
        out.push(')');
        return;
    }
    // ── user-defined-class OOP (O3) ─────────────────────────────────
    // The Ruby→SIR frontend lowers class OOP to a small family of
    // builtins whose *first* argument is a class- or method-name string
    // literal.  Each routes to the matching inlined `__Sir` OOP-runtime
    // helper; the runtime dispatches by explicit `Map` key (never
    // reflection), so a class/method named `constructor` is inert data.
    //
    //   `Klass.new(args…)`          → `__new__("Klass", args…)`
    //                                 → `__Sir.callNew("Klass", args…)`
    //   `super(args…)` in `C#m`     → `__super__("m", "C", args…)`
    //                                 → `__Sir.callSuper("m", "C", args…)`
    //   `def m …` in `class C`      → `__def_method__("C", "m", <closure>)`
    //                                 → `__Sir.defMethod("C", "m", <closure>)`
    //   `def self.m …`              → `__def_class_method__("C", "m", …)`
    //                                 → `__Sir.defClassMethod("C", "m", …)`
    //   `self`                      → `__self__()` → `__Sir.currentSelf()`
    if name == "__new__" && !args.is_empty() {
        out.push_str("__Sir.callNew(");
        emit_oop_name_arg(out, &args[0]);
        for a in &args[1..] {
            out.push_str(", ");
            emit_expr(out, a, indent);
        }
        out.push(')');
        return;
    }
    if name == "__super__" && args.len() >= 2 {
        // args: [method-name, class-name, call-args…].
        out.push_str("__Sir.callSuper(");
        emit_oop_name_arg(out, &args[0]);
        out.push_str(", ");
        emit_oop_name_arg(out, &args[1]);
        for a in &args[2..] {
            out.push_str(", ");
            emit_expr(out, a, indent);
        }
        out.push(')');
        return;
    }
    if name == "__def_method__" && args.len() == 3 {
        // args: [class-name, method-name, closure].
        out.push_str("__Sir.defMethod(");
        emit_oop_name_arg(out, &args[0]);
        out.push_str(", ");
        emit_oop_name_arg(out, &args[1]);
        out.push_str(", ");
        emit_expr(out, &args[2], indent);
        out.push(')');
        return;
    }
    if name == "__def_class_method__" && args.len() == 3 {
        out.push_str("__Sir.defClassMethod(");
        emit_oop_name_arg(out, &args[0]);
        out.push_str(", ");
        emit_oop_name_arg(out, &args[1]);
        out.push_str(", ");
        emit_expr(out, &args[2], indent);
        out.push(')');
        return;
    }
    if name == "__self__" {
        out.push_str("__Sir.currentSelf()");
        return;
    }
    // ── mixins (MX4): include / extend / class-method call ───────────
    // The frontend lowers Ruby mixin surface to three more OOP builtins,
    // each keyed by class/module *name-string* literals (never a live
    // object), so the runtime dispatches by explicit `Map` key — a module
    // named `constructor` is inert data, same bar as the method tables.
    //
    //   `include M` in `class C` → `__include__("C", "M")`
    //                              → `__Sir.includeModule("C", "M")`
    //   `extend  M` in `class C` → `__extend__("C", "M")`
    //                              → `__Sir.extendModule("C", "M")`
    //   `Klass.m(args…)`         → `__class_method__("Klass", "m", args…)`
    //                              → `__Sir.callClassMethod("Klass", "m", args…)`
    if name == "__include__" && args.len() == 2 {
        // args: [owner-name, module-name].
        out.push_str("__Sir.includeModule(");
        emit_oop_name_arg(out, &args[0]);
        out.push_str(", ");
        emit_oop_name_arg(out, &args[1]);
        out.push(')');
        return;
    }
    if name == "__extend__" && args.len() == 2 {
        // args: [owner-name, module-name].
        out.push_str("__Sir.extendModule(");
        emit_oop_name_arg(out, &args[0]);
        out.push_str(", ");
        emit_oop_name_arg(out, &args[1]);
        out.push(')');
        return;
    }
    if name == "__class_method__" && args.len() >= 2 {
        // args: [class-name, method-name, call-args…].
        out.push_str("__Sir.callClassMethod(");
        emit_oop_name_arg(out, &args[0]);
        out.push_str(", ");
        emit_oop_name_arg(out, &args[1]);
        for a in &args[2..] {
            out.push_str(", ");
            emit_expr(out, a, indent);
        }
        out.push(')');
        return;
    }
    // `raise` (SIR17) → throw a SIR exception via the inlined runtime.
    // Mirrors the TypeScript backend's `raise` arm; the first argument
    // decides the shape:
    //   • a `Const` class name (`raise Foo` / `raise Foo, "msg"`) → the
    //     class name is passed as a *string* (built-in classes need no
    //     binding), with the optional message second →
    //     `__Sir.raiseError("Foo"[, <msg>])`;
    //   • any other first arg (`raise "msg"`) → an implicit `RuntimeError`
    //     carrying that value as the message (matching Ruby) →
    //     `__Sir.raiseError("RuntimeError", <arg>)`;
    //   • no args (bare `raise`) → a generic re-raise →
    //     `__Sir.raiseError()` (the runtime defaults to `RuntimeError`).
    if name == "raise" {
        out.push_str("__Sir.raiseError(");
        match args.first() {
            None => {}
            Some(Expr::VarRef {
                name: cn,
                scope: Scope::Const,
                ..
            }) => {
                out.push_str(&quote_js_string(cn));
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
    // Ruby `&&`/`and` and `||`/`or` lower to `BuiltinCall("and"/"or",
    // [lhs, rhs])` — the frontend folds BOTH the 2-operand `a || b` form and a
    // multi-value `when 1, 2, 3` chain through them.  They MUST short-circuit
    // (rhs not evaluated once lhs decides) and use SIR truthiness, returning the
    // deciding OPERAND (not a bare bool).  So they emit the same truthy-guarded
    // arrow IIFE as `Expr::LogicalOr`/`LogicalAnd` rather than routing through
    // the eager `callBuiltin` dispatch — which has no `or`/`and` entry and would
    // evaluate both operands, losing Ruby semantics.  The `__l` param scopes each
    // occurrence so nested `&&`/`||` never collide.
    if name == "and" && args.len() == 2 {
        out.push_str("((__l) => __Sir.truthy(__l) ? (");
        emit_expr(out, &args[1], indent);
        out.push_str(") : __l)(");
        emit_expr(out, &args[0], indent);
        out.push(')');
        return;
    }
    if name == "or" && args.len() == 2 {
        out.push_str("((__l) => __Sir.truthy(__l) ? __l : (");
        emit_expr(out, &args[1], indent);
        out.push_str("))(");
        emit_expr(out, &args[0], indent);
        out.push(')');
        return;
    }
    // 2-argument `+` / `*` are POLYMORPHIC in Ruby (numeric add/mul, but
    // also String/Array concat, repeat, and join — see the
    // sir-polymorphic-operators spec).  Native JS infix would be *wrong*
    // for the collection arms (`[1] + [2]` yields the string `"1,2"`, and
    // `"ab" * 3` yields `NaN`), so they route through the inlined runtime
    // helpers `__Sir.plus` / `__Sir.times`, which dispatch on the first
    // operand's runtime type and fall back to the exact old numeric fold.
    if args.len() == 2 {
        let poly = match name {
            "+" => Some("__Sir.plus"),
            "*" => Some("__Sir.times"),
            // `/` routes through the runtime `divide` helper, which ADDS
            // the Ruby zero-divisor check (native JS `/` yields `Infinity`,
            // not a `ZeroDivisionError`) AND picks Integer#/ floor vs Float#/
            // true-division from the operand tags.  See `runtime::divide`.
            "/" => Some("__Sir.divide"),
            // `-` and `%` route through runtime helpers (not native infix)
            // because their result must be RE-TAGGED: a boxed Float operand
            // yields a boxed Float result (`7.0 - 1 == 6.0`), which native
            // `-`/`%` on a `SirFloat` object cannot produce.  See
            // `runtime::minus` / `runtime::mod`.
            "-" => Some("__Sir.minus"),
            "%" => Some("__Sir.mod"),
            _ => None,
        };
        if let Some(helper) = poly {
            let _ = write!(out, "{helper}(");
            emit_expr(out, &args[0], indent);
            out.push_str(", ");
            emit_expr(out, &args[1], indent);
            out.push(')');
            return;
        }
    }
    // 2-argument comparisons route through thin runtime helpers that unwrap
    // a tagged Float via `numOf` before comparing.  `numOf` is the IDENTITY
    // for every non-`SirFloat` value, so `eq`/`lt`/… are byte-identical to
    // the old native `===`/`<`/… for all existing values, and additionally
    // correct for a boxed Float (`7.0 == 7` is true; `7.0 < 8` works — a
    // native `<` on a `SirFloat` object would coerce to `NaN`).
    if args.len() == 2 {
        let cmp = match name {
            "=" => Some("__Sir.eq"),
            "!=" => Some("__Sir.ne"),
            "<" => Some("__Sir.lt"),
            ">" => Some("__Sir.gt"),
            "<=" => Some("__Sir.le"),
            ">=" => Some("__Sir.ge"),
            _ => None,
        };
        if let Some(helper) = cmp {
            let _ = write!(out, "{helper}(");
            emit_expr(out, &args[0], indent);
            out.push_str(", ");
            emit_expr(out, &args[1], indent);
            out.push(')');
            return;
        }
    }
    // 1-argument unary operators.
    if args.len() == 1 {
        match name {
            "not" => {
                out.push_str("(!__Sir.truthy(");
                emit_expr(out, &args[0], indent);
                out.push_str("))");
                return;
            }
            // MATLAB/Octave-family truthiness ("nonzero is true") for a
            // boolean-context operand that may be a genuine JS boolean
            // (an already-lowered comparison/`~`/`&&`/`||`) or a bare
            // number — `matlabTruthy` handles both correctly, unlike the
            // canonical `truthy()` above, which implements the unrelated
            // Ruby/Lisp convention (`0` is truthy there). See
            // `matlab-to-semantic-ir::lower::to_matlab_condition`, the
            // sole emitter of this builtin.
            "matlab_truthy" => {
                out.push_str("__Sir.matlabTruthy(");
                emit_expr(out, &args[0], indent);
                out.push(')');
                return;
            }
            "neg" => {
                // Unary minus re-tags: `-(7.0)` is the boxed Float `-7.0`,
                // which native `-` on a `SirFloat` object cannot produce.
                out.push_str("__Sir.neg(");
                emit_expr(out, &args[0], indent);
                out.push(')');
                return;
            }
            "len" => {
                // Works for arrays and strings alike (`.length`).
                out.push('(');
                emit_expr(out, &args[0], indent);
                out.push_str(").length");
                return;
            }
            _ => {}
        }
    }
    // Named helpers that always route through the inlined runtime.
    // `print` is exposed on the `__Sir` object directly for readable
    // output (`__Sir.print(x)`); the predicates and pair constructors
    // live in the bracket-indexed dispatch table.
    let helper = match name {
        "print" => Some("__Sir.print"),
        "puts" => Some("__Sir.puts"),
        "cons" => Some("__Sir.builtins[\"cons\"]"),
        "car" => Some("__Sir.builtins[\"car\"]"),
        "cdr" => Some("__Sir.builtins[\"cdr\"]"),
        "pair?" => Some("__Sir.builtins[\"pair?\"]"),
        "null?" => Some("__Sir.builtins[\"null?\"]"),
        "number?" => Some("__Sir.builtins[\"number?\"]"),
        "symbol?" => Some("__Sir.builtins[\"symbol?\"]"),
        _ => None,
    };
    if let Some(h) = helper {
        let _ = write!(out, "{h}(");
        emit_args(out, args, indent);
        out.push(')');
        return;
    }
    // Unknown / variadic builtin → runtime dispatch table.  This lets a
    // future builtin (or a variadic `+`) run without a backend change.
    let _ = write!(out, "__Sir.callBuiltin({}, [", quote_js_string(name));
    emit_args(out, args, indent);
    out.push_str("])");
}

/// Emit a block in **expression context** — an IIFE so its `let`
/// bindings stay private.  An empty-statement block renders its value
/// directly (no wrapper needed), keeping simple `if`-branches terse.
fn emit_block_as_expr(out: &mut String, b: &Block, indent: usize) {
    if b.stmts.is_empty() {
        emit_expr(out, &b.value, indent);
        return;
    }
    out.push_str("(() => {\n");
    let inner = indent + 2;
    for s in &b.stmts {
        emit_stmt(out, s, inner);
    }
    let ipad = " ".repeat(inner);
    let _ = write!(out, "{ipad}return ");
    emit_expr(out, &b.value, inner);
    out.push_str(";\n");
    let opad = " ".repeat(indent);
    let _ = write!(out, "{opad}}})()");
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

/// Sanitize a SIR identifier for JavaScript.
///
/// JavaScript identifiers match `[A-Za-z_$][A-Za-z0-9_$]*` and must not
/// be reserved words.  SIR names can contain `?`, `!`, `-`, `+`, `*`,
/// etc. (Lisp/Ruby conventions), so we rewrite anything that does not
/// fit:
///
/// | input        | output      | rule |
/// |--------------|-------------|------|
/// | `hello`      | `hello`     | already valid → unchanged |
/// | `class`      | `_$class`   | reserved word → `_$` prefix |
/// | `null?`      | `_$null_3f` | invalid char → `_$` + hex-encoded |
/// | `""` (empty) | `_$empty`   | empty → sentinel |
///
/// The `_$` prefix guarantees the result starts with a legal leading
/// character (never a digit) and never collides with the bare valid
/// form of another name (a valid name never starts with `_$` unless the
/// source did, in which case it passes through unchanged — and a source
/// `_$class` is itself valid, so no collision).
pub fn sanitize_ident(s: &str) -> String {
    if s.is_empty() {
        return "_$empty".to_string();
    }
    if is_valid_js_ident(s) {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len() + 4);
    out.push('_');
    out.push('$');
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '$' {
            out.push(ch);
        } else {
            // Hex-encode the codepoint to avoid collisions between
            // different invalid characters.
            let _ = write!(out, "_{:x}", ch as u32);
        }
    }
    out
}

/// Render `s` safe inside a single-line `//` comment.  Strips the
/// characters that terminate a line comment (`\n`, `\r`, U+0085, U+2028,
/// U+2029) so a hostile `Span.file` / `Module.name` can't inject code
/// after the comment marker.
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

fn is_valid_js_ident(s: &str) -> bool {
    let mut chars = s.chars();
    let first = match chars.next() {
        Some(c) => c,
        None => return false,
    };
    if !(first.is_ascii_alphabetic() || first == '_' || first == '$') {
        return false;
    }
    for ch in chars {
        if !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '$') {
            return false;
        }
    }
    !is_js_reserved(s)
}

fn is_js_reserved(s: &str) -> bool {
    // ECMAScript reserved words + a few future-reserved / contextual
    // ones that are unsafe as bindings.  `sanitize_ident` prefixes `_$`
    // for any of these.
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
            | "await"
            | "async"
            | "implements"
            | "interface"
            | "package"
            | "private"
            | "protected"
            | "public"
    )
}

/// Quote a string as a JavaScript double-quoted literal, escaping the
/// characters that would otherwise break the literal or the parser.
fn quote_js_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            // U+2028 / U+2029 are legal in ES2019+ string literals but
            // break older parsers and source-map tooling; escape them.
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

/// Format an `f64` as a JavaScript numeric literal.  Integer-valued
/// floats get an explicit `.0` so the emitted token is unambiguously a
/// float in the *source* (JS has one numeric type, but the decimal
/// point documents intent and matches the spec).  Non-finite values map
/// to their JS spellings (`NaN`, `Infinity`, `-Infinity`), since a bare
/// `NaN`/`Infinity` literal is valid JS.
fn format_float(v: f64) -> String {
    if v.is_nan() {
        return "NaN".to_string();
    }
    if v.is_infinite() {
        return if v < 0.0 {
            "-Infinity".to_string()
        } else {
            "Infinity".to_string()
        };
    }
    // `{:?}` on an f64 yields the shortest round-tripping decimal and
    // always includes a decimal point for integer-valued floats
    // (e.g. `3.0`), which is exactly the spec's requirement.
    format!("{v:?}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use semantic_ir::{CaptureValue, EffectSet, Metadata, Param, RescueClause, Span};

    fn s() -> Span {
        Span::synthetic()
    }

    // ── sanitize_ident ────────────────────────────────────────────

    #[test]
    fn sanitize_empty_ident_returns_sentinel() {
        assert_eq!(sanitize_ident(""), "_$empty");
    }

    #[test]
    fn sanitize_passes_through_valid_idents() {
        assert_eq!(sanitize_ident("hello"), "hello");
        assert_eq!(sanitize_ident("foo_bar"), "foo_bar");
        assert_eq!(sanitize_ident("$x"), "$x");
        assert_eq!(sanitize_ident("main"), "main");
    }

    #[test]
    fn sanitize_avoids_reserved_words() {
        assert_eq!(sanitize_ident("class"), "_$class");
        assert!(sanitize_ident("function").starts_with("_$"));
        assert!(sanitize_ident("await").starts_with("_$"));
    }

    #[test]
    fn sanitize_rewrites_invalid_chars() {
        let r = sanitize_ident("null?");
        assert!(r.starts_with("_$"));
        assert!(r.contains("null"));
        // `?` (U+003F) hex-encodes to `_3f`.
        assert!(r.contains("_3f"), "got {r}");
    }

    #[test]
    fn sanitize_hex_encodes_distinctly() {
        // `+` and `-` must not collide.
        assert_ne!(sanitize_ident("+"), sanitize_ident("-"));
    }

    // ── string quoting & float formatting ─────────────────────────

    #[test]
    fn quote_js_string_basic_and_escapes() {
        assert_eq!(quote_js_string("hi"), r#""hi""#);
        assert_eq!(quote_js_string("a\"b\nc\\d"), r#""a\"b\nc\\d""#);
    }

    #[test]
    fn quote_js_string_low_control_and_separators() {
        // A control char below 0x20 becomes a \u00XX escape.
        assert!(quote_js_string("\u{0001}").contains("\\u0001"));
        // U+2028 / U+2029 become explicit   /   escapes.
        let q = quote_js_string("a\u{2028}b\u{2029}c");
        assert!(q.contains("\\u2028") && q.contains("\\u2029"), "got {q}");
        assert!(!q.contains('\u{2028}') && !q.contains('\u{2029}'));
    }

    #[test]
    fn format_float_has_decimal_point_and_specials() {
        assert_eq!(format_float(3.0), "3.0");
        assert_eq!(format_float(3.25), "3.25");
        assert_eq!(format_float(f64::INFINITY), "Infinity");
        assert_eq!(format_float(f64::NEG_INFINITY), "-Infinity");
        assert_eq!(format_float(f64::NAN), "NaN");
    }

    #[test]
    fn sanitize_comment_strips_newlines_and_separators() {
        let safe = sanitize_comment("x\n*/console.log('pwn');//");
        assert!(!safe.contains('\n') && !safe.contains('\r'));
        let safe2 = sanitize_comment("a\u{2028}b\u{2029}c\u{0085}d");
        assert!(!safe2.contains('\u{2028}'));
        assert!(!safe2.contains('\u{2029}'));
        assert!(!safe2.contains('\u{0085}'));
    }

    // ── per-node emit ─────────────────────────────────────────────

    fn emit_e(e: &Expr) -> String {
        let mut out = String::new();
        emit_expr(&mut out, e, 0);
        out
    }

    #[test]
    fn emit_int_float_bool_nil() {
        assert_eq!(
            emit_e(&Expr::IntLit {
                value: 42,
                span: s()
            }),
            "42"
        );
        assert_eq!(
            emit_e(&Expr::FloatLit {
                value: 2.5,
                span: s()
            }),
            "__Sir.mkFloat(2.5)"
        );
        assert_eq!(
            emit_e(&Expr::BoolLit {
                value: true,
                span: s()
            }),
            "true"
        );
        assert_eq!(
            emit_e(&Expr::BoolLit {
                value: false,
                span: s()
            }),
            "false"
        );
        assert_eq!(emit_e(&Expr::NilLit { span: s() }), "null");
    }

    #[test]
    fn emit_symbol_and_string() {
        assert_eq!(
            emit_e(&Expr::SymLit {
                name: "foo".into(),
                span: s()
            }),
            r#"__Sir.intern("foo")"#
        );
        assert_eq!(
            emit_e(&Expr::StrLit {
                value: "hi".into(),
                span: s()
            }),
            r#""hi""#
        );
    }

    #[test]
    fn emit_var_ref_local_param_capture_global_are_bare() {
        for scope in [Scope::Local, Scope::Param, Scope::Capture, Scope::Global] {
            assert_eq!(
                emit_e(&Expr::VarRef {
                    name: "x".into(),
                    scope,
                    span: s()
                }),
                "x"
            );
        }
    }

    #[test]
    fn emit_var_ref_builtin_uses_closure() {
        assert_eq!(
            emit_e(&Expr::VarRef {
                name: "+".into(),
                scope: Scope::Builtin,
                span: s()
            }),
            r#"__Sir.builtinClosure("+")"#
        );
    }

    fn bc(name: &str, args: Vec<Expr>) -> Expr {
        Expr::BuiltinCall {
            name: name.into(),
            args,
            effects: EffectSet::PURE,
            span: s(),
        }
    }

    #[test]
    fn emit_builtin_arithmetic_routes_through_retagging_helpers() {
        let two = || {
            vec![
                Expr::IntLit {
                    value: 1,
                    span: s(),
                },
                Expr::IntLit {
                    value: 2,
                    span: s(),
                },
            ]
        };
        // `-`/`%` route through runtime helpers (not native infix) so a boxed
        // Float operand yields a boxed Float result (`7.0 - 1 == 6.0`).
        assert_eq!(emit_e(&bc("-", two())), "__Sir.minus(1, 2)");
        assert_eq!(emit_e(&bc("%", two())), "__Sir.mod(1, 2)");
    }

    #[test]
    fn emit_builtin_divide_routes_through_runtime_helper() {
        // `/` routes through `__Sir.divide`, which adds the Ruby
        // zero-divisor check (native JS `/` yields `Infinity`, not a
        // `ZeroDivisionError`).  The numeric result is otherwise identical.
        let two = || {
            vec![
                Expr::IntLit {
                    value: 1,
                    span: s(),
                },
                Expr::IntLit {
                    value: 2,
                    span: s(),
                },
            ]
        };
        assert_eq!(emit_e(&bc("/", two())), "__Sir.divide(1, 2)");
    }

    #[test]
    fn emit_builtin_plus_times_are_polymorphic_runtime_calls() {
        // `+`/`*` are Ruby-polymorphic (numeric add/mul, String/Array
        // concat/repeat/join), so the 2-arg form routes through the inlined
        // runtime dispatch helpers rather than native infix.  Native `[1] +
        // [2]` would be the wrong string `"1,2"`, and `"ab" * 3` would be
        // `NaN` — the helpers fix both while preserving the numeric path.
        let two = || {
            vec![
                Expr::IntLit {
                    value: 1,
                    span: s(),
                },
                Expr::IntLit {
                    value: 2,
                    span: s(),
                },
            ]
        };
        assert_eq!(emit_e(&bc("+", two())), "__Sir.plus(1, 2)");
        assert_eq!(emit_e(&bc("*", two())), "__Sir.times(1, 2)");
    }

    #[test]
    fn emit_builtin_comparison_routes_through_numof_helpers() {
        let two = || {
            vec![
                Expr::IntLit {
                    value: 1,
                    span: s(),
                },
                Expr::IntLit {
                    value: 2,
                    span: s(),
                },
            ]
        };
        // Comparisons route through thin `numOf`-unwrapping helpers — for
        // plain numbers these are exactly the old `===`/`<`/…, and additionally
        // correct for a boxed Float (`7.0 == 7`, `7.0 < 8`).
        assert_eq!(emit_e(&bc("=", two())), "__Sir.eq(1, 2)");
        assert_eq!(emit_e(&bc("!=", two())), "__Sir.ne(1, 2)");
        assert_eq!(emit_e(&bc("<", two())), "__Sir.lt(1, 2)");
        assert_eq!(emit_e(&bc(">", two())), "__Sir.gt(1, 2)");
        assert_eq!(emit_e(&bc("<=", two())), "__Sir.le(1, 2)");
        assert_eq!(emit_e(&bc(">=", two())), "__Sir.ge(1, 2)");
    }

    #[test]
    fn emit_builtin_unary_not_neg_len() {
        assert_eq!(
            emit_e(&bc(
                "not",
                vec![Expr::BoolLit {
                    value: true,
                    span: s()
                }]
            )),
            "(!__Sir.truthy(true))"
        );
        assert_eq!(
            emit_e(&bc(
                "neg",
                vec![Expr::IntLit {
                    value: 5,
                    span: s()
                }]
            )),
            "__Sir.neg(5)"
        );
        assert_eq!(
            emit_e(&bc(
                "len",
                vec![Expr::VarRef {
                    name: "a".into(),
                    scope: Scope::Local,
                    span: s()
                }]
            )),
            "(a).length"
        );
    }

    #[test]
    fn emit_builtin_print_routes_to_runtime() {
        assert_eq!(
            emit_e(&bc(
                "print",
                vec![Expr::IntLit {
                    value: 7,
                    span: s()
                }]
            )),
            "__Sir.print(7)"
        );
    }

    /// `puts` routes to the variadic `__Sir.puts(...)` runtime helper.  All
    /// arguments are forwarded (Ruby `puts a, b`), and a bare `puts` becomes
    /// `__Sir.puts()`.
    #[test]
    fn emit_builtin_puts_routes_to_runtime() {
        assert_eq!(
            emit_e(&bc(
                "puts",
                vec![Expr::IntLit {
                    value: 7,
                    span: s()
                }]
            )),
            "__Sir.puts(7)"
        );
        assert_eq!(
            emit_e(&bc(
                "puts",
                vec![
                    Expr::IntLit {
                        value: 1,
                        span: s()
                    },
                    Expr::IntLit {
                        value: 2,
                        span: s()
                    },
                ],
            )),
            "__Sir.puts(1, 2)"
        );
        assert_eq!(emit_e(&bc("puts", vec![])), "__Sir.puts()");
    }

    #[test]
    fn emit_method_dispatch_routes_to_call_method() {
        // `arr.push(1)` → BuiltinCall("__method__", [arr, "push", 1]) →
        // `__Sir.callMethod(arr, "push", 1)`.  Receiver first, method name
        // (a StrLit) second, call args after.
        let e = bc(
            "__method__",
            vec![
                Expr::VarRef {
                    name: "arr".into(),
                    scope: Scope::Local,
                    span: s(),
                },
                Expr::StrLit {
                    value: "push".into(),
                    span: s(),
                },
                Expr::IntLit {
                    value: 1,
                    span: s(),
                },
            ],
        );
        assert_eq!(emit_e(&e), r#"__Sir.callMethod(arr, "push", 1)"#);
    }

    #[test]
    fn emit_zero_arg_method_dispatch() {
        // `s.toUpperCase()` → __method__(s, "toUpperCase") → no trailing args.
        let e = bc(
            "__method__",
            vec![
                Expr::VarRef {
                    name: "s".into(),
                    scope: Scope::Local,
                    span: s(),
                },
                Expr::StrLit {
                    value: "toUpperCase".into(),
                    span: s(),
                },
            ],
        );
        assert_eq!(emit_e(&e), r#"__Sir.callMethod(s, "toUpperCase")"#);
    }

    // ── OOP builtin emit shape (O3) ───────────────────────────────

    fn strlit(v: &str) -> Expr {
        Expr::StrLit {
            value: v.into(),
            span: s(),
        }
    }

    #[test]
    fn emit_new_routes_to_call_new() {
        // `Dog.new("Rex")` → __new__("Dog", "Rex") → __Sir.callNew(...).
        let e = bc("__new__", vec![strlit("Dog"), strlit("Rex")]);
        assert_eq!(emit_e(&e), r#"__Sir.callNew("Dog", "Rex")"#);
        // Zero-arg construction: just the class name.
        assert_eq!(
            emit_e(&bc("__new__", vec![strlit("Dog")])),
            r#"__Sir.callNew("Dog")"#
        );
    }

    #[test]
    fn emit_new_accepts_const_class_operand() {
        // `Dog.new` where `Dog` is a bare Const → still emitted as the
        // *name string*, never a live binding.
        let e = bc(
            "__new__",
            vec![Expr::VarRef {
                name: "Dog".into(),
                scope: Scope::Const,
                span: s(),
            }],
        );
        assert_eq!(emit_e(&e), r#"__Sir.callNew("Dog")"#);
    }

    #[test]
    fn emit_super_routes_to_call_super() {
        // super("x") inside Cat#describe → __super__("describe","Cat","x").
        let e = bc(
            "__super__",
            vec![strlit("describe"), strlit("Cat"), strlit("x")],
        );
        assert_eq!(emit_e(&e), r#"__Sir.callSuper("describe", "Cat", "x")"#);
    }

    #[test]
    fn emit_def_method_routes_to_def_method() {
        // def speak; end in class Dog → __def_method__("Dog","speak",<closure>).
        let closure = Expr::MakeClosure {
            fn_name: "Dog__speak".into(),
            captures: vec![],
            span: s(),
        };
        let e = bc(
            "__def_method__",
            vec![strlit("Dog"), strlit("speak"), closure],
        );
        assert_eq!(
            emit_e(&e),
            r#"__Sir.defMethod("Dog", "speak", new __Sir.Closure((..._a) => Dog__speak(..._a)))"#
        );
    }

    #[test]
    fn emit_def_class_method_routes_to_def_class_method() {
        let closure = Expr::MakeClosure {
            fn_name: "Dog__count".into(),
            captures: vec![],
            span: s(),
        };
        let e = bc(
            "__def_class_method__",
            vec![strlit("Dog"), strlit("count"), closure],
        );
        assert_eq!(
            emit_e(&e),
            r#"__Sir.defClassMethod("Dog", "count", new __Sir.Closure((..._a) => Dog__count(..._a)))"#
        );
    }

    #[test]
    fn emit_self_routes_to_current_self() {
        assert_eq!(emit_e(&bc("__self__", vec![])), "__Sir.currentSelf()");
    }

    // ── mixins (MX4): include / extend / class-method call ───────────

    #[test]
    fn emit_include_routes_to_include_module() {
        // `include Greet` in `class Robot` → __include__("Robot", "Greet")
        //                                   → __Sir.includeModule("Robot", "Greet").
        let e = bc("__include__", vec![strlit("Robot"), strlit("Greet")]);
        assert_eq!(emit_e(&e), r#"__Sir.includeModule("Robot", "Greet")"#);
    }

    #[test]
    fn emit_extend_routes_to_extend_module() {
        // `extend Counter` in `class Widget` → __extend__("Widget", "Counter")
        //                                     → __Sir.extendModule("Widget", "Counter").
        let e = bc("__extend__", vec![strlit("Widget"), strlit("Counter")]);
        assert_eq!(emit_e(&e), r#"__Sir.extendModule("Widget", "Counter")"#);
    }

    #[test]
    fn emit_class_method_call_routes_to_call_class_method() {
        // `Widget.tally(1)` → __class_method__("Widget", "tally", 1)
        //                    → __Sir.callClassMethod("Widget", "tally", 1).
        let e = bc(
            "__class_method__",
            vec![
                strlit("Widget"),
                strlit("tally"),
                Expr::IntLit {
                    value: 1,
                    span: s(),
                },
            ],
        );
        assert_eq!(emit_e(&e), r#"__Sir.callClassMethod("Widget", "tally", 1)"#);
    }

    #[test]
    fn emit_class_method_call_no_args() {
        // `Widget.count` (no call args) → __class_method__("Widget", "count").
        let e = bc("__class_method__", vec![strlit("Widget"), strlit("count")]);
        assert_eq!(emit_e(&e), r#"__Sir.callClassMethod("Widget", "count")"#);
    }

    #[test]
    fn emit_ivar_read_routes_to_ivar_get() {
        // `@name` (Scope::Instance, sigil preserved) → __Sir.ivarGet("@name").
        assert_eq!(
            emit_e(&Expr::VarRef {
                name: "@name".into(),
                scope: Scope::Instance,
                span: s()
            }),
            r#"__Sir.ivarGet("@name")"#
        );
    }

    #[test]
    fn emit_cvar_read_routes_to_cvar_get() {
        assert_eq!(
            emit_e(&Expr::VarRef {
                name: "@@count".into(),
                scope: Scope::ClassVar,
                span: s()
            }),
            r#"__Sir.cvarGet("@@count")"#
        );
    }

    #[test]
    fn emit_ivar_write_routes_to_ivar_set() {
        // `@name = "Rex"` → __Sir.ivarSet("@name", "Rex");
        let st = Stmt::Assign {
            name: "@name".into(),
            scope: Scope::Instance,
            value: strlit("Rex"),
            span: s(),
        };
        assert_eq!(emit_s(&st), "__Sir.ivarSet(\"@name\", \"Rex\");\n");
    }

    #[test]
    fn emit_cvar_write_routes_to_cvar_set() {
        let st = Stmt::Assign {
            name: "@@count".into(),
            scope: Scope::ClassVar,
            value: Expr::IntLit {
                value: 0,
                span: s(),
            },
            span: s(),
        };
        assert_eq!(emit_s(&st), "__Sir.cvarSet(\"@@count\", 0);\n");
    }

    #[test]
    fn emit_variadic_plus_falls_back_to_dispatch() {
        let three = bc(
            "+",
            vec![
                Expr::IntLit {
                    value: 1,
                    span: s(),
                },
                Expr::IntLit {
                    value: 2,
                    span: s(),
                },
                Expr::IntLit {
                    value: 3,
                    span: s(),
                },
            ],
        );
        assert_eq!(emit_e(&three), r#"__Sir.callBuiltin("+", [1, 2, 3])"#);
    }

    #[test]
    fn emit_unknown_builtin_falls_back_to_dispatch() {
        assert_eq!(
            emit_e(&bc(
                "frobnicate",
                vec![Expr::IntLit {
                    value: 1,
                    span: s()
                }]
            )),
            r#"__Sir.callBuiltin("frobnicate", [1])"#
        );
    }

    #[test]
    fn emit_pairs_route_through_runtime_table() {
        let cons = bc(
            "cons",
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
        );
        assert_eq!(emit_e(&cons), r#"__Sir.builtins["cons"](1, 2)"#);
        assert_eq!(
            emit_e(&bc("car", vec![cons.clone()])),
            r#"__Sir.builtins["car"](__Sir.builtins["cons"](1, 2))"#
        );
    }

    #[test]
    fn emit_direct_and_indirect_calls() {
        let dc = Expr::DirectCall {
            fn_name: "id".into(),
            args: vec![Expr::IntLit {
                value: 7,
                span: s(),
            }],
            effects: EffectSet::PURE,
            span: s(),
        };
        assert_eq!(emit_e(&dc), "id(7)");
        let ic = Expr::IndirectCall {
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
        };
        assert_eq!(emit_e(&ic), "__Sir.applyClosure(f, [5])");
    }

    #[test]
    fn emit_if_uses_truthy_ternary() {
        let if_e = Expr::If {
            cond: Box::new(Expr::BoolLit {
                value: true,
                span: s(),
            }),
            then_branch: Box::new(Block {
                stmts: vec![],
                value: Expr::IntLit {
                    value: 1,
                    span: s(),
                },
                span: s(),
            }),
            else_branch: Box::new(Block {
                stmts: vec![],
                value: Expr::IntLit {
                    value: 2,
                    span: s(),
                },
                span: s(),
            }),
            span: s(),
        };
        assert_eq!(emit_e(&if_e), "(__Sir.truthy(true) ? (1) : (2))");
    }

    #[test]
    fn emit_make_closure_prepends_captures() {
        let mc = Expr::MakeClosure {
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
        assert_eq!(
            emit_e(&mc),
            "new __Sir.Closure((..._a) => __lambda_0(5, ..._a))"
        );
    }

    #[test]
    fn emit_block_in_expr_position_is_iife() {
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
            value: Expr::VarRef {
                name: "x".into(),
                scope: Scope::Local,
                span: s(),
            },
            span: s(),
        };
        let out = emit_e(&Expr::Block(Box::new(b)));
        assert!(out.starts_with("(() => {"), "got {out}");
        assert!(out.contains("let x = 1;"));
        assert!(out.contains("return x;"));
        assert!(out.trim_end().ends_with("})()"));
    }

    // ── SIR16 expressions ─────────────────────────────────────────

    fn var(name: &str) -> Expr {
        Expr::VarRef {
            name: name.into(),
            scope: Scope::Local,
            span: s(),
        }
    }

    fn int(v: i64) -> Expr {
        Expr::IntLit {
            value: v,
            span: s(),
        }
    }

    #[test]
    fn emit_float_lit_decimal_and_specials() {
        // A Float literal is minted through `__Sir.mkFloat`, which boxes an
        // integral value and leaves a non-integral / non-finite one native.
        assert_eq!(
            emit_e(&Expr::FloatLit {
                value: 3.0,
                span: s()
            }),
            "__Sir.mkFloat(3.0)"
        );
        assert_eq!(
            emit_e(&Expr::FloatLit {
                value: 2.5,
                span: s()
            }),
            "__Sir.mkFloat(2.5)"
        );
        assert_eq!(
            emit_e(&Expr::FloatLit {
                value: f64::INFINITY,
                span: s()
            }),
            "__Sir.mkFloat(Infinity)"
        );
        assert_eq!(
            emit_e(&Expr::FloatLit {
                value: f64::NEG_INFINITY,
                span: s()
            }),
            "__Sir.mkFloat(-Infinity)"
        );
        assert_eq!(
            emit_e(&Expr::FloatLit {
                value: f64::NAN,
                span: s()
            }),
            "__Sir.mkFloat(NaN)"
        );
    }

    #[test]
    fn emit_seq_lit_is_native_array() {
        let seq = Expr::SeqLit {
            items: vec![int(1), int(2), int(3)],
            span: s(),
        };
        assert_eq!(emit_e(&seq), "[1, 2, 3]");
        let empty = Expr::SeqLit {
            items: vec![],
            span: s(),
        };
        assert_eq!(emit_e(&empty), "[]");
    }

    #[test]
    fn emit_seq_index_is_native_subscript() {
        let e = Expr::SeqIndex {
            seq: Box::new(var("xs")),
            index: Box::new(int(0)),
            span: s(),
        };
        assert_eq!(emit_e(&e), "(xs)[0]");
    }

    #[test]
    fn emit_seq_len_is_native_length() {
        let e = Expr::SeqLen {
            seq: Box::new(var("xs")),
            span: s(),
        };
        assert_eq!(emit_e(&e), "(xs).length");
    }

    #[test]
    fn emit_map_lit_is_native_map() {
        let e = Expr::MapLit {
            entries: vec![
                semantic_ir::nodes::MapEntry {
                    key: Expr::StrLit {
                        value: "a".into(),
                        span: s(),
                    },
                    value: int(1),
                },
                semantic_ir::nodes::MapEntry {
                    key: Expr::StrLit {
                        value: "b".into(),
                        span: s(),
                    },
                    value: int(2),
                },
            ],
            span: s(),
        };
        assert_eq!(emit_e(&e), r#"new Map([["a", 1], ["b", 2]])"#);
        let empty = Expr::MapLit {
            entries: vec![],
            span: s(),
        };
        assert_eq!(emit_e(&empty), "new Map([])");
    }

    #[test]
    fn emit_map_get_uses_get_with_nil_default() {
        let e = Expr::MapGet {
            map: Box::new(var("d")),
            key: Box::new(Expr::StrLit {
                value: "k".into(),
                span: s(),
            }),
            span: s(),
        };
        assert_eq!(emit_e(&e), r#"((d).get("k") ?? null)"#);
    }

    #[test]
    fn emit_or_and_builtins_short_circuit() {
        // Ruby `a || b` / `a && b` lower to `BuiltinCall("or"/"and", [a, b])` and
        // must emit the SAME truthy-guarded short-circuit IIFE as
        // `Expr::LogicalOr`/`LogicalAnd` — NOT the eager `__Sir.callBuiltin`
        // fallback (which has no `or`/`and` entry and would evaluate both
        // operands, throwing `unknown builtin` at runtime).
        let a = Expr::StrLit {
            value: "a".into(),
            span: s(),
        };
        let b = Expr::StrLit {
            value: "b".into(),
            span: s(),
        };
        let or = Expr::BuiltinCall {
            name: "or".into(),
            args: vec![a.clone(), b.clone()],
            effects: EffectSet::PURE,
            span: s(),
        };
        assert_eq!(
            emit_e(&or),
            r#"((__l) => __Sir.truthy(__l) ? __l : ("b"))("a")"#
        );
        assert!(!emit_e(&or).contains("callBuiltin"));
        let and = Expr::BuiltinCall {
            name: "and".into(),
            args: vec![a, b],
            effects: EffectSet::PURE,
            span: s(),
        };
        assert_eq!(
            emit_e(&and),
            r#"((__l) => __Sir.truthy(__l) ? ("b") : __l)("a")"#
        );
    }

    #[test]
    fn emit_logical_and_short_circuits_via_truthy() {
        let e = Expr::LogicalAnd {
            lhs: Box::new(var("a")),
            rhs: Box::new(var("b")),
            span: s(),
        };
        assert_eq!(emit_e(&e), "((__l) => __Sir.truthy(__l) ? (b) : __l)(a)");
    }

    #[test]
    fn emit_logical_or_short_circuits_via_truthy() {
        let e = Expr::LogicalOr {
            lhs: Box::new(var("a")),
            rhs: Box::new(var("b")),
            span: s(),
        };
        assert_eq!(emit_e(&e), "((__l) => __Sir.truthy(__l) ? __l : (b))(a)");
    }

    // ── SIR16 statements ──────────────────────────────────────────

    fn emit_s(st: &Stmt) -> String {
        let mut out = String::new();
        emit_stmt(&mut out, st, 0);
        out
    }

    #[test]
    fn emit_assign_is_bare_reassignment() {
        for scope in [Scope::Local, Scope::Param, Scope::Capture, Scope::Global] {
            let st = Stmt::Assign {
                name: "x".into(),
                scope,
                value: int(7),
                span: s(),
            };
            assert_eq!(emit_s(&st), "x = 7;\n");
        }
    }

    #[test]
    fn emit_seq_set_is_native_element_write() {
        let st = Stmt::SeqSet {
            seq: var("xs"),
            index: int(0),
            value: int(9),
            span: s(),
        };
        assert_eq!(emit_s(&st), "(xs)[0] = 9;\n");
    }

    #[test]
    fn emit_map_set_is_native_map_set() {
        let st = Stmt::MapSet {
            map: var("d"),
            key: Expr::StrLit {
                value: "k".into(),
                span: s(),
            },
            value: int(1),
            span: s(),
        };
        assert_eq!(emit_s(&st), r#"(d).set("k", 1);"#.to_string() + "\n");
    }

    #[test]
    fn emit_while_routes_cond_through_truthy() {
        let st = Stmt::While {
            cond: Expr::BuiltinCall {
                name: "<".into(),
                args: vec![var("i"), int(3)],
                effects: EffectSet::PURE,
                span: s(),
            },
            body: Block {
                stmts: vec![Stmt::Assign {
                    name: "i".into(),
                    scope: Scope::Local,
                    value: Expr::BuiltinCall {
                        name: "+".into(),
                        args: vec![var("i"), int(1)],
                        effects: EffectSet::PURE,
                        span: s(),
                    },
                    span: s(),
                }],
                value: Expr::NilLit { span: s() },
                span: s(),
            },
            span: s(),
        };
        let out = emit_s(&st);
        assert!(
            out.starts_with("while (__Sir.truthy(__Sir.lt(i, 3))) {\n"),
            "got {out}"
        );
        assert!(out.contains("i = __Sir.plus(i, 1);"));
        // The trailing nil body value is dropped.
        assert!(!out.contains("null;"), "got {out}");
        assert!(out.trim_end().ends_with('}'));
    }

    #[test]
    fn emit_for_range_is_direction_aware_c_style() {
        // Reset the counter so the temporary names are predictable.
        LOOP_COUNTER.with(|c| c.set(0));
        let st = Stmt::ForRange {
            var: "i".into(),
            start: int(0),
            stop: int(5),
            step: int(1),
            body: Block {
                stmts: vec![Stmt::ExprStmt {
                    expr: bc("print", vec![var("i")]),
                    span: s(),
                }],
                value: Expr::NilLit { span: s() },
                span: s(),
            },
            span: s(),
        };
        let out = emit_s(&st);
        assert!(out.contains("let i = 0;"), "got {out}");
        assert!(out.contains("const __sir_stop_0 = 5;"), "got {out}");
        assert!(out.contains("const __sir_step_0 = 1;"), "got {out}");
        assert!(
            out.contains("while (__sir_step_0 >= 0 ? i < __sir_stop_0 : i > __sir_stop_0) {"),
            "got {out}"
        );
        assert!(out.contains("__Sir.print(i);"), "got {out}");
        assert!(out.contains("i = i + __sir_step_0;"), "got {out}");
    }

    #[test]
    fn emit_for_each_is_for_of() {
        let st = Stmt::ForEach {
            var: "x".into(),
            iter: var("xs"),
            body: Block {
                stmts: vec![Stmt::ExprStmt {
                    expr: bc("print", vec![var("x")]),
                    span: s(),
                }],
                value: Expr::NilLit { span: s() },
                span: s(),
            },
            span: s(),
        };
        let out = emit_s(&st);
        assert!(out.starts_with("for (let x of xs) {\n"), "got {out}");
        assert!(out.contains("__Sir.print(x);"), "got {out}");
    }

    #[test]
    fn emit_nested_loop_temporaries_are_distinct() {
        // Two for-ranges in one module must get distinct temp ids so the
        // emitted code is well-formed (no shadow collision).
        LOOP_COUNTER.with(|c| c.set(0));
        let mk = || Stmt::ForRange {
            var: "i".into(),
            start: int(0),
            stop: int(2),
            step: int(1),
            body: Block {
                stmts: vec![],
                value: Expr::NilLit { span: s() },
                span: s(),
            },
            span: s(),
        };
        let a = emit_s(&mk());
        let b = emit_s(&mk());
        assert!(a.contains("__sir_stop_0"), "got {a}");
        assert!(b.contains("__sir_stop_1"), "got {b}");
    }

    // ── statements ────────────────────────────────────────────────

    #[test]
    fn emit_let_binding_uses_let() {
        let st = Stmt::LetBinding {
            name: "x".into(),
            sir_type: None,
            value: Expr::IntLit {
                value: 9,
                span: s(),
            },
            span: s(),
        };
        let mut out = String::new();
        emit_stmt(&mut out, &st, 0);
        assert_eq!(out, "let x = 9;\n");
    }

    #[test]
    fn emit_global_set_pattern_is_direct_assignment() {
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
        let mut out = String::new();
        emit_stmt(&mut out, &Stmt::ExprStmt { expr: e, span: s() }, 0);
        assert!(out.contains("counter = 0;"));
        // The SymLit form is suppressed in favour of the direct assign.
        assert!(!out.contains("intern"));
    }

    #[test]
    fn emit_expr_stmt_terminates_with_semicolon() {
        let st = Stmt::ExprStmt {
            expr: bc(
                "print",
                vec![Expr::IntLit {
                    value: 1,
                    span: s(),
                }],
            ),
            span: s(),
        };
        let mut out = String::new();
        emit_stmt(&mut out, &st, 0);
        assert_eq!(out, "__Sir.print(1);\n");
    }

    // ── functions & module ────────────────────────────────────────

    fn fun(name: &str, params: Vec<Param>, body: Block) -> Function {
        Function {
            name: name.into(),
            params,
            return_type: None,
            captures: vec![],
            body,
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        }
    }

    #[test]
    fn emit_simple_function_flat_body() {
        let f = fun(
            "id",
            vec![Param {
                name: "x".into(),
                sir_type: None,
                kind: ParamKind::Required,
                default: None,
                span: s(),
            }],
            Block {
                stmts: vec![],
                value: Expr::VarRef {
                    name: "x".into(),
                    scope: Scope::Param,
                    span: s(),
                },
                span: s(),
            },
        );
        let mut out = String::new();
        emit_function(&mut out, &f);
        assert!(out.contains("function id(x) {"));
        assert!(out.contains("  return x;"));
        // No IIFE wrapper for a function body.
        assert!(!out.contains("(() => {"));
    }

    #[test]
    fn emit_rest_param_is_native_spread() {
        let f = fun(
            "f",
            vec![Param {
                name: "rest".into(),
                sir_type: None,
                kind: ParamKind::Rest,
                default: None,
                span: s(),
            }],
            Block {
                stmts: vec![],
                value: Expr::NilLit { span: s() },
                span: s(),
            },
        );
        let mut out = String::new();
        emit_function(&mut out, &f);
        assert!(out.contains("function f(...rest) {"), "got {out}");
    }

    #[test]
    fn emit_defaulted_param_is_native_js_default() {
        // P2d: `f(a, b = a + 1)` emits `function f(a, b = __Sir.plus(a, 1)) {`.
        // The default is a native JS default param referencing the earlier
        // param `a` by name (legal — earlier params are in scope); the `+`
        // itself lowers to the polymorphic runtime helper.
        let default = bc(
            "+",
            vec![
                Expr::VarRef {
                    name: "a".into(),
                    scope: Scope::Param,
                    span: s(),
                },
                int(1),
            ],
        );
        let f = fun(
            "f",
            vec![
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
                    default: Some(Box::new(default)),
                    span: s(),
                },
            ],
            Block {
                stmts: vec![],
                value: Expr::VarRef {
                    name: "b".into(),
                    scope: Scope::Param,
                    span: s(),
                },
                span: s(),
            },
        );
        let mut out = String::new();
        emit_function(&mut out, &f);
        assert!(
            out.contains("function f(a, b = __Sir.plus(a, 1)) {"),
            "got {out}"
        );
    }

    #[test]
    fn emit_direct_call_omitting_defaulted_arg_does_not_pad() {
        // P2d: a DirectCall that supplies fewer args than params emits ONLY
        // the present args — native JS defaults fill the rest, no padding.
        let dc = Expr::DirectCall {
            fn_name: "f".into(),
            args: vec![int(5)],
            effects: EffectSet::PURE,
            span: s(),
        };
        assert_eq!(emit_e(&dc), "f(5)");
    }

    // ── KW4: keyword parameters & arguments ───────────────────────

    fn kw_param(name: &str, default: Option<Expr>) -> Param {
        Param {
            name: name.into(),
            sir_type: None,
            kind: ParamKind::Keyword,
            default: default.map(Box::new),
            span: s(),
        }
    }

    fn req_param(name: &str) -> Param {
        Param {
            name: name.into(),
            sir_type: None,
            kind: ParamKind::Required,
            default: None,
            span: s(),
        }
    }

    #[test]
    fn emit_keyword_params_become_trailing_options_object() {
        // def f(a, b:, c: 1)  →
        //   function f(a, __kw) { const { b, c = 1 } = __kw ?? {}; … }
        // `b` (no default) destructures bare; `c` (default 1) carries a JS
        // destructuring default; both fold into the single trailing `__kw`.
        let f = fun(
            "f",
            vec![
                req_param("a"),
                kw_param("b", None),
                kw_param("c", Some(int(1))),
            ],
            Block {
                stmts: vec![],
                value: Expr::VarRef {
                    name: "b".into(),
                    scope: Scope::Param,
                    span: s(),
                },
                span: s(),
            },
        );
        let mut out = String::new();
        emit_function(&mut out, &f);
        assert!(out.contains("function f(a, __kw) {"), "signature: {out}");
        assert!(
            out.contains("const { b, c = 1 } = __kw ?? {};"),
            "prologue: {out}"
        );
    }

    #[test]
    fn emit_keyword_only_function_has_no_positional_params() {
        // def f(x:)  →  function f(__kw) { const { x } = __kw ?? {}; … }
        // A required keyword with no positionals: `__kw` is the *only*
        // parameter, and `x` destructures without a default.
        let f = fun(
            "f",
            vec![kw_param("x", None)],
            Block {
                stmts: vec![],
                value: Expr::VarRef {
                    name: "x".into(),
                    scope: Scope::Param,
                    span: s(),
                },
                span: s(),
            },
        );
        let mut out = String::new();
        emit_function(&mut out, &f);
        assert!(out.contains("function f(__kw) {"), "signature: {out}");
        assert!(out.contains("const { x } = __kw ?? {};"), "prologue: {out}");
    }

    #[test]
    fn emit_function_without_keyword_params_has_no_kw_object() {
        // A function with only positionals must be byte-for-byte unchanged:
        // no `__kw` param, no destructuring prologue.
        let f = fun(
            "g",
            vec![req_param("a")],
            Block {
                stmts: vec![],
                value: int(0),
                span: s(),
            },
        );
        let mut out = String::new();
        emit_function(&mut out, &f);
        assert!(out.contains("function g(a) {"), "got {out}");
        assert!(!out.contains("__kw"), "no options object expected: {out}");
    }

    /// Build a `KeywordArg` call-argument.
    fn kw_arg(name: &str, value: Expr) -> Expr {
        Expr::KeywordArg {
            name: name.into(),
            value: Box::new(value),
            span: s(),
        }
    }

    #[test]
    fn emit_call_collapses_keyword_args_into_trailing_object() {
        // f(1, b: 2, c: 3)  →  f(1, { b: 2, c: 3 })
        let dc = Expr::DirectCall {
            fn_name: "f".into(),
            args: vec![int(1), kw_arg("b", int(2)), kw_arg("c", int(3))],
            effects: EffectSet::PURE,
            span: s(),
        };
        assert_eq!(emit_e(&dc), "f(1, { b: 2, c: 3 })");
    }

    #[test]
    fn emit_call_with_only_keyword_args_has_no_leading_comma() {
        // f(a: 2)  →  f({ a: 2 })  — no positionals, so no leading comma.
        let dc = Expr::DirectCall {
            fn_name: "f".into(),
            args: vec![kw_arg("a", int(2))],
            effects: EffectSet::PURE,
            span: s(),
        };
        assert_eq!(emit_e(&dc), "f({ a: 2 })");
    }

    #[test]
    fn emit_call_without_keyword_args_emits_no_object() {
        // f(1, 2)  →  f(1, 2)  — plain positional call, unchanged.
        let dc = Expr::DirectCall {
            fn_name: "f".into(),
            args: vec![int(1), int(2)],
            effects: EffectSet::PURE,
            span: s(),
        };
        assert_eq!(emit_e(&dc), "f(1, 2)");
    }

    #[test]
    fn emit_indirect_call_keyword_args_go_into_the_arg_array() {
        // Closure application routes args through `[…]`; the keyword object
        // is the last element of that array: applyClosure(t, [1, { b: 2 }]).
        let ic = Expr::IndirectCall {
            target: Box::new(Expr::VarRef {
                name: "t".into(),
                scope: Scope::Local,
                span: s(),
            }),
            args: vec![int(1), kw_arg("b", int(2))],
            effects: EffectSet::PURE,
            span: s(),
        };
        assert_eq!(emit_e(&ic), "__Sir.applyClosure(t, [1, { b: 2 }])");
    }

    #[test]
    fn emit_full_module_has_banner_strict_runtime_and_main() {
        let m = Module {
            name: "demo".into(),
            manifest: semantic_ir::FeatureManifest::new(),
            imports: vec![],
            exports: vec![],
            functions: vec![fun(
                "main",
                vec![],
                Block {
                    stmts: vec![],
                    value: Expr::IntLit {
                        value: 42,
                        span: s(),
                    },
                    span: s(),
                },
            )],
            globals: vec![],
            metadata: Metadata::new()
                .with_source_language("twig")
                .with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
            span: s(),
        };
        let out = emit_module(&m);
        assert!(out.contains("// Generated by semantic-ir-to-javascript"));
        assert!(out.contains("// Source language: twig"));
        assert!(out.contains("\"use strict\";"));
        assert!(out.contains("const __Sir = (() => {"));
        assert!(out.contains("function main() {"));
        assert!(out.contains("main();\n"));
        assert!(out.ends_with('\n'));
    }

    // ── E1: exceptions (raise / TryCatch) ─────────────────────────

    /// `raise Foo, "msg"` (a `Const` class + message) → the class name is
    /// a *string literal*, the message follows.
    #[test]
    fn emit_raise_const_class_with_message() {
        let e = bc(
            "raise",
            vec![
                Expr::VarRef {
                    name: "ArgumentError".into(),
                    scope: Scope::Const,
                    span: s(),
                },
                Expr::StrLit {
                    value: "boom".into(),
                    span: s(),
                },
            ],
        );
        assert_eq!(emit_e(&e), r#"__Sir.raiseError("ArgumentError", "boom")"#);
    }

    /// `raise Foo` (no message) → just the class-name string.
    #[test]
    fn emit_raise_const_class_no_message() {
        let e = bc(
            "raise",
            vec![Expr::VarRef {
                name: "RuntimeError".into(),
                scope: Scope::Const,
                span: s(),
            }],
        );
        assert_eq!(emit_e(&e), r#"__Sir.raiseError("RuntimeError")"#);
    }

    /// Bare `raise` (no args) → `raiseError()`, which the runtime defaults
    /// to a generic `RuntimeError` re-raise.
    #[test]
    fn emit_raise_bare_reraises() {
        assert_eq!(emit_e(&bc("raise", vec![])), "__Sir.raiseError()");
    }

    /// `raise "msg"` (a non-`Const` first arg) → an implicit `RuntimeError`
    /// carrying the value as the message, matching Ruby and the TS backend.
    #[test]
    fn emit_raise_non_const_is_runtime_error_with_message() {
        let e = bc(
            "raise",
            vec![Expr::StrLit {
                value: "oops".into(),
                span: s(),
            }],
        );
        assert_eq!(emit_e(&e), r#"__Sir.raiseError("RuntimeError", "oops")"#);
    }

    /// A `TryCatch` with one typed, bound rescue emits a native
    /// `try { … } catch (__exc) { if (rescueMatches(…)) { … } else { throw
    /// __exc; } }` — the rescueMatches-guarded else-chain.
    #[test]
    fn emit_try_catch_emits_rescue_matches_else_chain() {
        let st = Stmt::TryCatch {
            body: vec![Stmt::ExprStmt {
                expr: bc(
                    "print",
                    vec![Expr::StrLit {
                        value: "try".into(),
                        span: s(),
                    }],
                ),
                span: s(),
            }],
            rescues: vec![RescueClause {
                exception_types: vec!["StandardError".into()],
                binding: Some("e".into()),
                body: vec![Stmt::ExprStmt {
                    expr: bc(
                        "print",
                        vec![Expr::StrLit {
                            value: "caught".into(),
                            span: s(),
                        }],
                    ),
                    span: s(),
                }],
                span: s(),
            }],
            ensure_body: None,
            span: s(),
        };
        let out = emit_s(&st);
        assert!(out.contains("try {"), "got:\n{out}");
        assert!(out.contains("catch (__exc) {"), "got:\n{out}");
        assert!(
            out.contains(r#"if (__Sir.rescueMatches(__exc, ["StandardError"])) {"#),
            "got:\n{out}"
        );
        assert!(out.contains("const e = __exc;"), "got:\n{out}");
        assert!(out.contains("} else {"), "got:\n{out}");
        assert!(out.contains("throw __exc;"), "got:\n{out}");
    }

    /// A bare `rescue` (empty `exception_types`) emits the catch-all
    /// `rescueMatches(__exc, [])`; multiple clauses chain with `else if`;
    /// an `ensure` becomes a `finally`.
    #[test]
    fn emit_try_catch_bare_rescue_chain_and_finally() {
        let st = Stmt::TryCatch {
            body: vec![],
            rescues: vec![
                RescueClause {
                    exception_types: vec!["TypeError".into()],
                    binding: None,
                    body: vec![],
                    span: s(),
                },
                RescueClause {
                    exception_types: vec![], // bare `rescue`
                    binding: None,
                    body: vec![],
                    span: s(),
                },
            ],
            ensure_body: Some(vec![]),
            span: s(),
        };
        let out = emit_s(&st);
        assert!(
            out.contains(r#"if (__Sir.rescueMatches(__exc, ["TypeError"])) {"#),
            "got:\n{out}"
        );
        assert!(
            out.contains("} else if (__Sir.rescueMatches(__exc, [])) {"),
            "got:\n{out}"
        );
        assert!(out.contains("finally {"), "got:\n{out}");
    }

    // ── E2 (JS half): user-defined class ancestry ─────────────────

    /// A `class MyErr < StandardError` inside a function body is collected
    /// into a single `__Sir.registerAncestry({ … })` emitted at program
    /// init (after the runtime, before user functions run).
    #[test]
    fn emit_module_registers_user_class_ancestry() {
        let class_def = Stmt::ClassDef {
            name: "MyErr".into(),
            superclass: Some("StandardError".into()),
            body: vec![],
            span: s(),
        };
        let m = Module {
            name: "demo".into(),
            manifest: semantic_ir::FeatureManifest::new(),
            imports: vec![],
            exports: vec![],
            functions: vec![fun(
                "main",
                vec![],
                Block {
                    stmts: vec![class_def],
                    value: Expr::NilLit { span: s() },
                    span: s(),
                },
            )],
            globals: vec![],
            metadata: Metadata::new().with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
            span: s(),
        };
        let out = emit_module(&m);
        assert!(
            out.contains(r#"__Sir.registerAncestry({ "MyErr": "StandardError" });"#),
            "got:\n{out}"
        );
        // Registration precedes the user's `main` so the table is complete
        // before any `rescueMatches` runs.
        let reg = out.find("__Sir.registerAncestry({").unwrap();
        let main = out.find("function main(").unwrap();
        assert!(
            reg < main,
            "registerAncestry call must precede user functions"
        );
    }

    /// A base class (`class Foo`, no superclass) carries no ancestry edge,
    /// so a module with only base classes emits no `registerAncestry`.
    #[test]
    fn emit_module_omits_registration_without_inheritance() {
        let class_def = Stmt::ClassDef {
            name: "Foo".into(),
            superclass: None,
            body: vec![],
            span: s(),
        };
        let m = Module {
            name: "demo".into(),
            manifest: semantic_ir::FeatureManifest::new(),
            imports: vec![],
            exports: vec![],
            functions: vec![fun(
                "main",
                vec![],
                Block {
                    stmts: vec![class_def],
                    value: Expr::NilLit { span: s() },
                    span: s(),
                },
            )],
            globals: vec![],
            metadata: Metadata::new().with_sir_version(semantic_ir::CURRENT_SIR_VERSION),
            span: s(),
        };
        // The runtime *defines* `registerAncestry`, so only the *call*
        // `__Sir.registerAncestry({` marks a real registration — and that
        // must be absent for a module with no inheriting classes.
        assert!(!emit_module(&m).contains("__Sir.registerAncestry({"));
    }
}
