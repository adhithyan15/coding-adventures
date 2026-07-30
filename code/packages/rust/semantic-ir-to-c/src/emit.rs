//! Node lowering — turns a `semantic_ir::Module` into portable ISO C99.
//!
//! The emitter is **statement-oriented**.  A SIR value is always produced
//! *into a destination* — a `return` ([`emit_tail`]) or an assignment to a
//! declared `SirValue` ([`emit_assign`]) — because portable C has no
//! statement-expression to give a multi-statement block a value.  A "simple"
//! expression (no embedded control flow) is rendered directly as a C
//! expression string by [`emit_expr`]; anything compound is flattened into
//! statements with a temporary.
//!
//! See [SIR24](../../../specs/SIR24-semantic-ir-to-c.md) for the design.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::fmt::Write;

use semantic_ir::{Block, Expr, Function, Global, IntSpec, IntWidth, Module, Scope, Span, Stmt};

use crate::runtime::RUNTIME;

thread_local! {
    /// Per-module counter for fresh temporary / block identifiers.  Reset at
    /// the top of [`emit_module`] so emission is byte-stable (the determinism
    /// test relies on this).
    static TEMP_ID: Cell<u64> = const { Cell::new(0) };

    /// Per-module map from a user function's RAW name to its declared parameter
    /// NAMES in order, snapshotted at the top of [`emit_module`].  Used at a
    /// `DirectCall` for two SIR19 jobs: default-argument padding needs the arity
    /// (the length), and keyword resolution needs the names (to place a
    /// `KeywordArg` at its callee slot by name).  Thread-local (like `TEMP_ID`)
    /// so the deep `emit_expr` / `emit_assign` call tree can read it without
    /// threading a context.
    static SIGNATURES: RefCell<HashMap<String, Vec<String>>> = RefCell::new(HashMap::new());
}

fn fresh_id() -> u64 {
    TEMP_ID.with(|c| {
        let n = c.get();
        c.set(n + 1);
        n
    })
}

/// The declared parameter count of a known user function (by raw name), for
/// call-site default-argument padding.  `None` for an unknown callee.
fn callee_arity(fn_name: &str) -> Option<usize> {
    SIGNATURES.with(|a| a.borrow().get(fn_name).map(|p| p.len()))
}

/// The declared parameter NAMES of a known user function (by raw name), for
/// resolving a `KeywordArg` to its callee slot.  `None` for an unknown callee
/// (which the validator guarantees never receives keyword arguments).
fn callee_param_names(fn_name: &str) -> Option<Vec<String>> {
    SIGNATURES.with(|a| a.borrow().get(fn_name).cloned())
}

/// Append `_sir_missing()` arguments to bring a `DirectCall`'s `provided`
/// argument count up to the callee's declared arity — the SIR19 default-param
/// call-site padding.  A leading comma is written before each pad when the call
/// already has `provided > 0` arguments (or a previous pad) so the argument
/// list stays well-formed.  No-op for an unknown callee or an exact/over-full
/// call.
fn emit_default_padding(out: &mut String, fn_name: &str, provided: usize) {
    if let Some(arity) = callee_arity(fn_name) {
        for i in provided..arity {
            if i > 0 {
                out.push_str(", ");
            }
            out.push_str("_sir_missing()");
        }
    }
}

/// Emit a complete self-contained C source file for `m`.
pub fn emit_module(m: &Module) -> String {
    TEMP_ID.with(|c| c.set(0));
    // Snapshot each user function's declared parameter names for `DirectCall`
    // default padding (needs the count) and keyword resolution (needs the
    // names).  Cleared first so a reused thread starts empty.
    SIGNATURES.with(|a| {
        let mut map = a.borrow_mut();
        map.clear();
        for f in &m.functions {
            map.insert(
                f.name.clone(),
                f.params.iter().map(|p| p.name.clone()).collect(),
            );
        }
    });

    let mut out = String::new();
    emit_banner(&mut out, m);
    // Silence MSVC's CRT deprecation warnings for the ISO functions the
    // runtime uses (snprintf etc.).  Harmless on GCC/Clang (no such macro),
    // and must precede every #include to take effect on MSVC.
    out.push_str("#define _CRT_SECURE_NO_WARNINGS 1\n");
    out.push_str("#include <stdio.h>\n");
    out.push_str("#include <stdlib.h>\n");
    out.push_str("#include <string.h>\n");
    out.push_str("#include <stdarg.h>\n");
    out.push_str("#include <stdint.h>\n");
    // `<math.h>` supplies the C99 `INFINITY` / `NAN` macros used to spell a
    // non-finite `FloatLit` (a finite literal needs no macro). Standard and
    // available on every C99 compiler including MSVC.
    out.push_str("#include <math.h>\n");
    // `<setjmp.h>` supplies `jmp_buf`/`setjmp`/`longjmp` for the SIR17 exception
    // handler stack.
    out.push_str("#include <setjmp.h>\n\n");

    // The runtime, with the display-convention placeholder resolved to a
    // boolean-selected LITERAL (never source text — see the security note in
    // runtime.rs).
    let display_ruby = m.metadata.source_language.as_deref() == Some("ruby");
    out.push_str(&RUNTIME.replace("__SIR_DISPLAY_RUBY__", if display_ruby { "1" } else { "0" }));
    out.push_str("\n\n");

    emit_globals_comment(&mut out, &m.globals);

    // Forward declarations — every user function first, so mutual recursion
    // needs no ordering, then a thunk for each closure target.
    out.push_str("/* forward declarations */\n");
    for f in &m.functions {
        emit_prototype(&mut out, f);
    }
    let thunks = collect_closure_targets(m);
    for name in &thunks {
        let _ = writeln!(
            out,
            "static SirValue {}(SirValue *caps, SirValue *args, int argc);",
            thunk_name(name)
        );
    }
    out.push('\n');

    // Definitions.
    for f in &m.functions {
        emit_function(&mut out, f);
        out.push('\n');
    }
    for name in &thunks {
        emit_thunk(&mut out, m, name);
        out.push('\n');
    }

    // Process entry.
    out.push_str("int main(void) {\n");
    if m.functions.iter().any(|f| f.name == "_init") {
        out.push_str("    sir_user_init();\n");
    }
    if m.functions.iter().any(|f| f.name == "main") {
        out.push_str("    user_main();\n");
    }
    out.push_str("    return 0;\n}\n");

    out
}

fn emit_banner(out: &mut String, m: &Module) {
    out.push_str("/* Generated by semantic-ir-to-c (SIR24) — do not edit. */\n");
    let _ = writeln!(out, "/* module: {} */", sanitize_comment(&m.name));
    if let Some(lang) = &m.metadata.source_language {
        let _ = writeln!(out, "/* source language: {} */", sanitize_comment(lang));
    }
    out.push('\n');
}

fn emit_globals_comment(out: &mut String, globals: &[Global]) {
    if globals.is_empty() {
        return;
    }
    out.push_str("/* globals (held in the runtime store, initialised in _init):");
    for g in globals {
        let _ = write!(out, " {}", sanitize_comment(&g.name));
    }
    out.push_str(" */\n\n");
}

// ---------------------------------------------------------------------------
// Functions
// ---------------------------------------------------------------------------

/// The C name for a SIR function.  `main` and `_init` are renamed so they
/// never collide with C's process entry or a reserved platform symbol.
fn function_emit_name(name: &str) -> String {
    match name {
        "main" => "user_main".to_string(),
        "_init" => "sir_user_init".to_string(),
        other => sanitize_ident(other),
    }
}

/// A closure body's function takes its captures first, then its params — the
/// order [`emit_function`] emits and [`emit_thunk`] unpacks.
fn emit_prototype(out: &mut String, f: &Function) {
    let _ = write!(out, "SirValue {}(", function_emit_name(&f.name));
    emit_param_list(out, f);
    out.push_str(");\n");
}

fn emit_param_list(out: &mut String, f: &Function) {
    let mut first = true;
    for c in &f.captures {
        if !first {
            out.push_str(", ");
        }
        first = false;
        let _ = write!(out, "SirValue {}", sanitize_ident(&c.name));
    }
    for p in &f.params {
        if !first {
            out.push_str(", ");
        }
        first = false;
        let _ = write!(out, "SirValue {}", sanitize_ident(&p.name));
    }
    if first {
        out.push_str("void");
    }
}

fn emit_function(out: &mut String, f: &Function) {
    let _ = writeln!(out, "/* {} */", sanitize_comment(&f.span.to_string()));
    let _ = write!(out, "SirValue {}(", function_emit_name(&f.name));
    emit_param_list(out, f);
    out.push_str(") {\n");
    emit_default_prologue(out, f, 1);
    emit_block_body(out, &f.body, 1);
    out.push_str("}\n");
}

/// Emit the SIR19 default-parameter prologue: for each parameter carrying a
/// default, in declaration order, guard `if (_sir_is_missing(name)) { name =
/// <default>; }`.  A call site pads an omitted trailing defaulted argument with
/// `_sir_missing()` (see [`emit_default_padding`]); this replaces it with the
/// default value BEFORE the body runs, so the body never sees a `SIR_MISSING`.
/// Declaration order lets a later default reference an earlier parameter (whose
/// own default, if any, is already filled) — matching the validator and the
/// Go/Ruby backends.  The parameter is a C function parameter (a mutable
/// lvalue), so it is reassigned in place; a compound default hoists through
/// `emit_assign`.
fn emit_default_prologue(out: &mut String, f: &Function, indent: usize) {
    let pad = indent_str(indent);
    for p in &f.params {
        let Some(default) = &p.default else { continue };
        let name = sanitize_ident(&p.name);
        let _ = writeln!(out, "{pad}if (_sir_is_missing({name})) {{");
        emit_assign(out, &name, default, indent + 1);
        let _ = writeln!(out, "{pad}}}");
    }
}

/// Emit a block in **statement/return** position — its `stmts` run for effect,
/// then its `value` is returned.
fn emit_block_body(out: &mut String, b: &Block, indent: usize) {
    for s in &b.stmts {
        emit_stmt(out, s, indent);
    }
    emit_tail(out, &b.value, indent);
}

/// Emit a thunk that adapts a closure body's fixed C signature to the uniform
/// `SirFn` calling convention.  Missing trailing args read as `nil`
/// (proc-lenient arity).
fn emit_thunk(out: &mut String, m: &Module, name: &str) {
    let f = m.functions.iter().find(|f| f.name == name);
    let _ = writeln!(
        out,
        "static SirValue {}(SirValue *caps, SirValue *args, int argc) {{",
        thunk_name(name)
    );
    if let Some(f) = f {
        let ncap = f.captures.len();
        let nparam = f.params.len();
        let _ = write!(out, "    return {}(", function_emit_name(name));
        let mut first = true;
        for i in 0..ncap {
            if !first {
                out.push_str(", ");
            }
            first = false;
            let _ = write!(out, "caps[{}]", i);
        }
        for i in 0..nparam {
            if !first {
                out.push_str(", ");
            }
            first = false;
            let _ = write!(out, "(argc > {i} ? args[{i}] : _sir_nil())", i = i);
        }
        // A body with no captures and no params is still a valid zero-arg call.
        out.push_str(");\n");
        // Silence unused-parameter warnings when the body takes nothing.
        if ncap == 0 {
            out.push_str("    (void)caps;\n");
        }
        if nparam == 0 {
            out.push_str("    (void)args; (void)argc;\n");
        }
    } else {
        out.push_str("    (void)caps; (void)args; (void)argc;\n    return _sir_nil();\n");
    }
    out.push_str("}\n");
}

fn thunk_name(name: &str) -> String {
    format!("_sir_thunk_{}", sanitize_ident(name))
}

/// The set of function names referenced by a `MakeClosure` anywhere in the
/// module — exactly the functions that need a thunk.  Sorted for determinism.
fn collect_closure_targets(m: &Module) -> Vec<String> {
    let mut set = std::collections::BTreeSet::new();
    for f in &m.functions {
        collect_targets_block(&f.body, &mut set);
    }
    set.into_iter().collect()
}

fn collect_targets_block(b: &Block, set: &mut std::collections::BTreeSet<String>) {
    for s in &b.stmts {
        collect_targets_stmt(s, set);
    }
    collect_targets_expr(&b.value, set);
}

fn collect_targets_stmt(s: &Stmt, set: &mut std::collections::BTreeSet<String>) {
    match s {
        Stmt::LetBinding { value, .. }
        | Stmt::LetStarBinding { value, .. }
        | Stmt::ExprStmt { expr: value, .. }
        | Stmt::Assign { value, .. } => collect_targets_expr(value, set),
        _ => {}
    }
}

fn collect_targets_expr(e: &Expr, set: &mut std::collections::BTreeSet<String>) {
    match e {
        Expr::MakeClosure {
            fn_name, captures, ..
        } => {
            set.insert(fn_name.clone());
            for c in captures {
                collect_targets_expr(&c.value, set);
            }
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            collect_targets_expr(cond, set);
            collect_targets_block(then_branch, set);
            collect_targets_block(else_branch, set);
        }
        Expr::Block(b) => collect_targets_block(b, set),
        Expr::DirectCall { args, .. }
        | Expr::IndirectCall { args, .. }
        | Expr::BuiltinCall { args, .. } => {
            for a in args {
                collect_targets_expr(a, set);
            }
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Statements
// ---------------------------------------------------------------------------

fn emit_stmt(out: &mut String, s: &Stmt, indent: usize) {
    let pad = indent_str(indent);
    match s {
        Stmt::LetBinding { name, value, .. } | Stmt::LetStarBinding { name, value, .. } => {
            let n = sanitize_ident(name);
            if is_simple(value) {
                let _ = write!(out, "{pad}SirValue {n} = ");
                emit_expr(out, value, indent);
                out.push_str(";\n");
            } else {
                let _ = writeln!(out, "{pad}SirValue {n};");
                emit_assign(out, &n, value, indent);
            }
        }
        Stmt::ExprStmt { expr, .. } => {
            if is_simple(expr) {
                let _ = write!(out, "{pad}(void)(");
                emit_expr(out, expr, indent);
                out.push_str(");\n");
            } else {
                let tmp = format!("_sir_t{}", fresh_id());
                let _ = writeln!(out, "{pad}SirValue {tmp};");
                emit_assign(out, &tmp, expr, indent);
                let _ = writeln!(out, "{pad}(void){tmp};");
            }
        }
        // SIR16 re-binding: the target `SirValue` is already declared (by an
        // earlier `LetStarBinding`), so this reuses `emit_assign` (which handles
        // a simple value or a compound `If`/`Convert`).
        Stmt::Assign {
            name, scope, value, ..
        } => {
            if matches!(scope, Scope::Const) {
                // OOP slice 1: a `Scope::Const` definition (`PI = 3`) writes the
                // runtime constant table.  The name is a QUOTED C string literal
                // (no injection).  A compound value is hoisted into a temp first.
                if is_simple(value) {
                    let _ = write!(out, "{pad}(void)_sir_const_set({}, ", quote_c_string(name));
                    emit_expr(out, value, indent);
                    out.push_str(");\n");
                } else {
                    let tmp = format!("_sir_t{}", fresh_id());
                    let _ = writeln!(out, "{pad}SirValue {tmp};");
                    emit_assign(out, &tmp, value, indent);
                    let _ = writeln!(
                        out,
                        "{pad}(void)_sir_const_set({}, {tmp});",
                        quote_c_string(name)
                    );
                }
            } else if matches!(scope, Scope::Instance) {
                // OOP slice 3: `@v = …` writes the current receiver's ivar map.
                // The `@`-name is a QUOTED C string literal (no injection).  A
                // compound value is hoisted into a temp first.
                if is_simple(value) {
                    let _ = write!(out, "{pad}(void)_sir_ivar_set({}, ", quote_c_string(name));
                    emit_expr(out, value, indent);
                    out.push_str(");\n");
                } else {
                    let tmp = format!("_sir_t{}", fresh_id());
                    let _ = writeln!(out, "{pad}SirValue {tmp};");
                    emit_assign(out, &tmp, value, indent);
                    let _ = writeln!(
                        out,
                        "{pad}(void)_sir_ivar_set({}, {tmp});",
                        quote_c_string(name)
                    );
                }
            } else if matches!(scope, Scope::ClassVar) {
                // OOP slice 6: `@@x = …` in a METHOD body writes the current
                // class's class-variable storage (resolved via
                // `_sir_current_class`).  The `@@`-name is a QUOTED C string
                // literal (no injection).  (A class-BODY `@@x = 0` initializer is
                // emitted by the `ClassDef` arm with the class named explicitly.)
                if is_simple(value) {
                    let _ = write!(out, "{pad}(void)_sir_cvar_set({}, ", quote_c_string(name));
                    emit_expr(out, value, indent);
                    out.push_str(");\n");
                } else {
                    let tmp = format!("_sir_t{}", fresh_id());
                    let _ = writeln!(out, "{pad}SirValue {tmp};");
                    emit_assign(out, &tmp, value, indent);
                    let _ = writeln!(
                        out,
                        "{pad}(void)_sir_cvar_set({}, {tmp});",
                        quote_c_string(name)
                    );
                }
            } else {
                let n = sanitize_ident(name);
                emit_assign(out, &n, value, indent);
            }
        }
        // SIR16 loop.  Portable shape that re-evaluates a possibly-compound
        // condition every iteration:
        //   for (;;) { SirValue c; <c = cond>; if (!_sir_truthy(c)) break; <body> }
        Stmt::While { cond, body, .. } => {
            let _ = writeln!(out, "{pad}for (;;) {{");
            let inner = indent + 1;
            let ipad = indent_str(inner);
            let ctmp = format!("_sir_w{}", fresh_id());
            let _ = writeln!(out, "{ipad}SirValue {ctmp};");
            emit_assign(out, &ctmp, cond, inner);
            let _ = writeln!(out, "{ipad}if (!_sir_truthy({ctmp})) break;");
            for st in &body.stmts {
                emit_stmt(out, st, inner);
            }
            let _ = writeln!(out, "{pad}}}");
        }
        // `for var in start...stop step step`. Gated by `Feature::Loops` alone,
        // so it is reachable whenever loops are accepted. Counts in native
        // `int64_t` mirroring the Go/Rust backends: `start`/`stop`/`step` are
        // evaluated ONCE into `SirValue` temporaries (they may have side
        // effects), then reduced to `int64_t`. Direction-aware EXCLUSIVE stop
        // (`step >= 0 ? i < stop : i > stop`, so a descending loop works). The
        // outer `{…}` scopes the counter temporaries; `var` is declared INSIDE
        // the loop body, so it (and any body-local) is block-scoped — matching
        // the validator (which rewinds the loop body) and Go's `:=` counter,
        // never clobbering an enclosing same-named local.
        Stmt::ForRange {
            var,
            start,
            stop,
            step,
            body,
            ..
        } => {
            let id = fresh_id();
            let inner = indent + 1;
            let ipad = indent_str(inner);
            let sv = |kind: &str| format!("_sir_fr{kind}v{id}");
            let _ = writeln!(out, "{pad}{{");
            for (kind, e) in [("start", start), ("stop", stop), ("step", step)] {
                let _ = writeln!(out, "{ipad}SirValue {};", sv(kind));
                emit_assign(out, &sv(kind), e, inner);
            }
            let _ = writeln!(
                out,
                "{ipad}int64_t _sir_fri{id} = _sir_as_int({});",
                sv("start")
            );
            let _ = writeln!(
                out,
                "{ipad}int64_t _sir_frstop{id} = _sir_as_int({});",
                sv("stop")
            );
            let _ = writeln!(
                out,
                "{ipad}int64_t _sir_frstep{id} = _sir_as_int({});",
                sv("step")
            );
            let _ = writeln!(
                out,
                "{ipad}while (_sir_frstep{id} >= 0 ? _sir_fri{id} < _sir_frstop{id} : \
                 _sir_fri{id} > _sir_frstop{id}) {{",
            );
            let body_i = inner + 1;
            let bpad = indent_str(body_i);
            let _ = writeln!(
                out,
                "{bpad}SirValue {} = _sir_int(_sir_fri{id});",
                sanitize_ident(var)
            );
            // A loop that never reads its counter (an empty body, or `for _ in
            // 0..n { … }`) would leave `var` unused — `(void)` silences
            // `-Wunused-variable` so a `-Werror` consumer still compiles.
            let _ = writeln!(out, "{bpad}(void){};", sanitize_ident(var));
            for st in &body.stmts {
                emit_stmt(out, st, body_i);
            }
            let _ = writeln!(out, "{bpad}_sir_fri{id} += _sir_frstep{id};");
            let _ = writeln!(out, "{ipad}}}");
            let _ = writeln!(out, "{pad}}}");
        }
        // Other SIR16+ statements (index-set/class/try) require features this
        // backend does not accept, so the capability check rejects them before
        // emit. `ForEach` is the exception — it observes only `Feature::Loops`
        // (accepted), so `compile`'s `first_foreach` pre-pass rejects it
        // cleanly rather than letting it reach this `unreachable!`.
        // `a[i] = v` — mutate the shared sequence box (via `_sir_seq_set`,
        // which traps on an out-of-range index). Operands are hoisted so
        // left-to-right evaluation order holds; the returned value is discarded
        // in statement position.
        Stmt::SeqSet {
            seq, index, value, ..
        } => {
            let _ = writeln!(out, "{pad}{{");
            let inner = indent + 1;
            let ipad = indent_str(inner);
            let names = hoist_operands(out, &[seq, index, value], inner);
            let _ = writeln!(
                out,
                "{ipad}(void)_sir_seq_set({}, {}, {});",
                names[0], names[1], names[2]
            );
            let _ = writeln!(out, "{pad}}}");
        }
        // `for var in iter … end`. `_sir_seq_iter` normalises the iterable to a
        // snapshot sequence (a real sequence is copied, a cons-list flattened),
        // so the body renders ONCE over one array. `var` is declared inside the
        // loop body block → block-scoped (matching the validator's rewind and
        // the Go reference), never clobbering an enclosing same-named local.
        Stmt::ForEach {
            var, iter, body, ..
        } => {
            let id = fresh_id();
            let _ = writeln!(out, "{pad}{{");
            let inner = indent + 1;
            let ipad = indent_str(inner);
            let it = hoist_operands(out, &[iter], inner);
            let _ = writeln!(
                out,
                "{ipad}SirValue _sir_feq{id} = _sir_seq_iter({});",
                it[0]
            );
            let _ = writeln!(
                out,
                "{ipad}int64_t _sir_fen{id} = _sir_feq{id}.as.seq->len;"
            );
            let _ = writeln!(
                out,
                "{ipad}for (int64_t _sir_fei{id} = 0; _sir_fei{id} < _sir_fen{id}; _sir_fei{id}++) {{"
            );
            let body_i = inner + 1;
            let bpad = indent_str(body_i);
            let v = sanitize_ident(var);
            let _ = writeln!(
                out,
                "{bpad}SirValue {v} = _sir_feq{id}.as.seq->items[_sir_fei{id}];"
            );
            let _ = writeln!(out, "{bpad}(void){v};");
            for st in &body.stmts {
                emit_stmt(out, st, body_i);
            }
            let _ = writeln!(out, "{ipad}}}");
            let _ = writeln!(out, "{pad}}}");
        }
        // `h[k] = v` — insert/update the shared map box (via `_sir_map_set`,
        // which traps only on a non-map; a map has no bounds to check). Operands
        // are hoisted so left-to-right evaluation order holds; the returned
        // value is discarded in statement position. Mirrors `SeqSet`.
        Stmt::MapSet {
            map, key, value, ..
        } => {
            let _ = writeln!(out, "{pad}{{");
            let inner = indent + 1;
            let ipad = indent_str(inner);
            let names = hoist_operands(out, &[map, key, value], inner);
            let _ = writeln!(
                out,
                "{ipad}(void)_sir_map_set({}, {}, {});",
                names[0], names[1], names[2]
            );
            let _ = writeln!(out, "{pad}}}");
        }
        // SIR17 `begin … rescue … ensure … end`.  C has no unwinding, so this
        // lowers to a TWO-handler `setjmp`/`longjmp` structure:
        //   - an OUTER "ensure" handler wraps the whole thing, so `ensure` runs
        //     even when a rescue body itself raises (Ruby semantics);
        //   - an INNER "body" handler catches an exception from the guarded body.
        //     It is popped BEFORE the rescue dispatch, so a raise in a rescue
        //     clause (or an unmatched exception) unwinds to the OUTER handler.
        // A `rescue` clause matches by class name against the baked-in ancestry
        // table (`_sir_rescue_matches`); an empty class list is a catch-all.
        // Exception-type names are emitted as QUOTED string literals (via
        // `quote_c_string`), so — unlike a language that emits them as bare
        // constants — no rescue type can inject source.
        Stmt::TryCatch {
            body,
            rescues,
            ensure_body,
            ..
        } => {
            let id = fresh_id();
            let i1 = indent + 1;
            let i2 = indent + 2;
            let i3 = indent + 3;
            let p1 = indent_str(i1);
            let p2 = indent_str(i2);
            let p3 = indent_str(i3);
            let _ = writeln!(out, "{pad}{{");
            // OOP slice 3: snapshot the current `self` — a method that `raise`s
            // inside the guarded body `longjmp`s past `_sir_call_method`'s own
            // restore, so the rescue/ensure paths below re-bind `self` to what it
            // was when this `begin` started (else `@x` would read the raiser's).
            let _ = writeln!(out, "{p1}SirValue _sir_selfsave{id} = _sir_current_self;");
            // OOP slice 6: snapshot the current class too — a method that `raise`s
            // `longjmp`s past the dispatcher's own restore, so `@@x` in the
            // rescue/ensure bodies must resolve against the class active when this
            // `begin` started, not the raiser's.
            let _ = writeln!(
                out,
                "{p1}const char *_sir_classsave{id} = _sir_current_class;"
            );
            let _ = writeln!(out, "{p1}int _sir_eh{id} = _sir_push_handler();");
            let _ = writeln!(out, "{p1}volatile int _sir_esc{id} = 0;");
            let _ = writeln!(
                out,
                "{p1}if (setjmp(_sir_handlers[_sir_eh{id}])) {{ _sir_esc{id} = 1; }}"
            );
            let _ = writeln!(out, "{p1}if (!_sir_esc{id}) {{");
            let _ = writeln!(out, "{p2}int _sir_bh{id} = _sir_push_handler();");
            let _ = writeln!(out, "{p2}volatile int _sir_c{id} = 0;");
            let _ = writeln!(
                out,
                "{p2}if (setjmp(_sir_handlers[_sir_bh{id}])) {{ _sir_c{id} = 1; }}"
            );
            let _ = writeln!(out, "{p2}if (!_sir_c{id}) {{");
            for st in body {
                emit_stmt(out, st, i3);
            }
            let _ = writeln!(out, "{p2}}}");
            let _ = writeln!(out, "{p2}_sir_pop_handler();");
            let _ = writeln!(out, "{p2}if (_sir_c{id}) {{");
            // Restore `self` before a rescue body runs (the body raised & was
            // caught, so `_sir_current_self` may be the raiser's).
            let _ = writeln!(out, "{p3}_sir_current_self = _sir_selfsave{id};");
            let _ = writeln!(out, "{p3}_sir_current_class = _sir_classsave{id};");
            let _ = writeln!(out, "{p3}SirValue _sir_ex{id} = _sir_current_error;");
            for (k, rc) in rescues.iter().enumerate() {
                let kw = if k == 0 { "if" } else { "} else if" };
                if rc.exception_types.is_empty() {
                    let _ = writeln!(
                        out,
                        "{p3}{kw} (_sir_rescue_matches(_sir_ex{id}, NULL, 0)) {{"
                    );
                } else {
                    let lits: Vec<String> = rc
                        .exception_types
                        .iter()
                        .map(|t| quote_c_string(t))
                        .collect();
                    let _ = writeln!(
                        out,
                        "{p3}{kw} (_sir_rescue_matches(_sir_ex{id}, (const char *const[]){{{}}}, {})) {{",
                        lits.join(", "),
                        rc.exception_types.len()
                    );
                }
                if let Some(bind) = &rc.binding {
                    let b = sanitize_ident(bind);
                    let _ = writeln!(
                        out,
                        "{}SirValue {b} = _sir_ex{id}; (void){b};",
                        indent_str(i3 + 1)
                    );
                }
                for st in &rc.body {
                    emit_stmt(out, st, i3 + 1);
                }
            }
            // Unmatched (or no rescue clause): re-raise so it propagates through
            // the outer ensure handler after `ensure` runs.
            if rescues.is_empty() {
                let _ = writeln!(out, "{p3}(void)_sir_raise(_sir_ex{id});");
            } else {
                let _ = writeln!(out, "{p3}}} else {{ (void)_sir_raise(_sir_ex{id}); }}");
            }
            let _ = writeln!(out, "{p2}}}");
            let _ = writeln!(out, "{p1}}}");
            // Snapshot the escaping exception BEFORE `ensure` runs and before
            // popping this handler: an `ensure` body that itself handles an
            // exception would otherwise overwrite the global `_sir_current_error`
            // and propagate the WRONG exception.  (Meaningful only when
            // `_sir_esc` is set; harmlessly nil on the normal path.)
            let _ = writeln!(out, "{p1}SirValue _sir_pend{id} = _sir_current_error;");
            // Restore `self` on the escape path too (reached whether the body
            // completed normally or an exception unwound to here) before `ensure`
            // runs and before control leaves the `begin`.
            let _ = writeln!(out, "{p1}_sir_current_self = _sir_selfsave{id};");
            let _ = writeln!(out, "{p1}_sir_current_class = _sir_classsave{id};");
            let _ = writeln!(out, "{p1}_sir_pop_handler();");
            if let Some(ens) = ensure_body {
                for st in ens {
                    emit_stmt(out, st, i1);
                }
            }
            let _ = writeln!(
                out,
                "{p1}if (_sir_esc{id}) {{ (void)_sir_raise(_sir_pend{id}); }}"
            );
            let _ = writeln!(out, "{pad}}}");
        }
        // OOP slice 1/4/6: a class is just a NAME in the C runtime (an instance
        // carries its class string; there is no class object).  A subclass
        // (`class Dog < Animal`) registers its `sub -> super` edge (slice 4) so
        // method resolution and `rescue`-matching walk the user hierarchy.  A
        // class-BODY `@@x = v` initializer (slice 6) seeds the class's
        // class-variable storage with the class named EXPLICITLY (a method body
        // would resolve `@@x` via `_sir_current_class`, but here `self` is the
        // top-level `main`, so the class must be spelled out).  All names are
        // QUOTED C string literals (no injection).  The scan admits ONLY a
        // ClassVar-assign body; any other class-level code is still rejected.
        Stmt::ClassDef {
            name,
            superclass,
            body,
            ..
        } => {
            if let Some(sup) = superclass {
                let _ = writeln!(
                    out,
                    "{pad}_sir_register_super({}, {});",
                    quote_c_string(name),
                    quote_c_string(sup)
                );
            } else if body.is_empty() {
                let _ = writeln!(out, "{pad}/* class declaration (no class object) */");
            }
            for st in body {
                if let Stmt::Assign {
                    name: var,
                    scope: Scope::ClassVar,
                    value,
                    ..
                } = st
                {
                    // `@@x = <value>` at class `name` → `_sir_cvar_set_in`.  A
                    // compound value is hoisted into a temp first.
                    if is_simple(value) {
                        let _ = write!(
                            out,
                            "{pad}(void)_sir_cvar_set_in({}, {}, ",
                            quote_c_string(name),
                            quote_c_string(var)
                        );
                        emit_expr(out, value, indent);
                        out.push_str(");\n");
                    } else {
                        let tmp = format!("_sir_t{}", fresh_id());
                        let _ = writeln!(out, "{pad}SirValue {tmp};");
                        emit_assign(out, &tmp, value, indent);
                        let _ = writeln!(
                            out,
                            "{pad}(void)_sir_cvar_set_in({}, {}, {tmp});",
                            quote_c_string(name),
                            quote_c_string(var)
                        );
                    }
                }
            }
        }
        // OOP slice 7: a `module M; … end` is, like a class, just a NAME in the C
        // runtime — its methods are registered via `__def_method__` keyed on the
        // module name (so no module object is needed), and `include`/`extend`
        // (separate top-level `__include__`/`__extend__` builtins) record the
        // mixin.  The declaration itself emits only a comment; a non-empty module
        // body (module-level code) is rejected by the scan, as with a class.
        Stmt::ModuleDef { .. } => {
            let _ = writeln!(out, "{pad}/* module declaration (no module object) */");
        }
        other => unreachable!("C backend reached unsupported statement: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Value production: emit_tail (return position) and emit_assign (into dst)
// ---------------------------------------------------------------------------

/// Emit `e` in return position.
fn emit_tail(out: &mut String, e: &Expr, indent: usize) {
    let pad = indent_str(indent);
    if is_simple(e) {
        let _ = write!(out, "{pad}return ");
        emit_expr(out, e, indent);
        out.push_str(";\n");
        return;
    }
    match e {
        Expr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            let c = emit_cond(out, cond, indent);
            let _ = writeln!(out, "{pad}if (_sir_truthy({c})) {{");
            emit_block_body(out, then_branch, indent + 1);
            let _ = writeln!(out, "{pad}}} else {{");
            emit_block_body(out, else_branch, indent + 1);
            let _ = writeln!(out, "{pad}}}");
        }
        Expr::Block(b) => {
            emit_block_body(out, b, indent);
        }
        // A compound call / short-circuit in tail position: compute into a
        // temp, then return it.
        _ => {
            let tmp = format!("_sir_t{}", fresh_id());
            let _ = writeln!(out, "{pad}SirValue {tmp};");
            emit_assign(out, &tmp, e, indent);
            let _ = writeln!(out, "{pad}return {tmp};");
        }
    }
}

/// Emit statements leaving `e`'s value in the already-declared lvalue `dst`.
fn emit_assign(out: &mut String, dst: &str, e: &Expr, indent: usize) {
    let pad = indent_str(indent);
    if is_simple(e) {
        let _ = write!(out, "{pad}{dst} = ");
        emit_expr(out, e, indent);
        out.push_str(";\n");
        return;
    }
    match e {
        Expr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            let c = emit_cond(out, cond, indent);
            let _ = writeln!(out, "{pad}if (_sir_truthy({c})) {{");
            emit_block_assign(out, dst, then_branch, indent + 1);
            let _ = writeln!(out, "{pad}}} else {{");
            emit_block_assign(out, dst, else_branch, indent + 1);
            let _ = writeln!(out, "{pad}}}");
        }
        Expr::Block(b) => {
            let _ = writeln!(out, "{pad}{{");
            emit_block_assign(out, dst, b, indent + 1);
            let _ = writeln!(out, "{pad}}}");
        }
        // Short-circuit `and` / `or` (returns the deciding operand).
        Expr::BuiltinCall { name, args, .. } if name == "and" && args.len() == 2 => {
            emit_assign(out, dst, &args[0], indent);
            let _ = writeln!(out, "{pad}if (_sir_truthy({dst})) {{");
            emit_assign(out, dst, &args[1], indent + 1);
            let _ = writeln!(out, "{pad}}}");
        }
        Expr::BuiltinCall { name, args, .. } if name == "or" && args.len() == 2 => {
            emit_assign(out, dst, &args[0], indent);
            let _ = writeln!(out, "{pad}if (!_sir_truthy({dst})) {{");
            emit_assign(out, dst, &args[1], indent + 1);
            let _ = writeln!(out, "{pad}}}");
        }
        // SIR16 short-circuit `&&` / `||` (`Expr::LogicalAnd` / `LogicalOr`) —
        // distinct nodes from the eager `and`/`or` builtins above, but the same
        // lowering: assign the LEFT operand into `dst`, then conditionally
        // OVERWRITE it with the right. Because the right operand is emitted only
        // inside the `if` body, it is not evaluated when the left already
        // decides (true short-circuit), and `dst` holds the DECIDING OPERAND
        // (not a coerced bool) — `a && b` is `a` when `a` is falsy else `b`;
        // `a || b` is `a` when truthy else `b`. This matches the Go backend's
        // short-circuit IIFE and Ruby's native `&&`/`||`.
        Expr::LogicalAnd { lhs, rhs, .. } => {
            emit_assign(out, dst, lhs, indent);
            let _ = writeln!(out, "{pad}if (_sir_truthy({dst})) {{");
            emit_assign(out, dst, rhs, indent + 1);
            let _ = writeln!(out, "{pad}}}");
        }
        Expr::LogicalOr { lhs, rhs, .. } => {
            emit_assign(out, dst, lhs, indent);
            let _ = writeln!(out, "{pad}if (!_sir_truthy({dst})) {{");
            emit_assign(out, dst, rhs, indent + 1);
            let _ = writeln!(out, "{pad}}}");
        }
        // SIR26 conversion with a compound value: compute the value into `dst`,
        // then reduce it to the target width in place.
        Expr::Convert { value, to, .. } => {
            emit_assign(out, dst, value, indent);
            if let IntWidth::Arbitrary = to.width {
                // identity widen — nothing to mask
            } else {
                let bits = to.width.bits().expect("non-arbitrary width has bits");
                let signed = if to.signed { 1 } else { 0 };
                let _ = writeln!(out, "{pad}{dst} = _sir_convert({dst}, {bits}, {signed});");
            }
        }
        // SIR16 sequences with a compound operand: hoist operands into temps
        // (preserving left-to-right order), then build/read the sequence.
        Expr::SeqLit { items, .. } => {
            let _ = writeln!(out, "{pad}{{");
            let inner = indent + 1;
            let ipad = indent_str(inner);
            let ops: Vec<&Expr> = items.iter().collect();
            let names = hoist_operands(out, &ops, inner);
            let _ = write!(out, "{ipad}{dst} = _sir_seq_lit({}", items.len());
            for n in &names {
                let _ = write!(out, ", {n}");
            }
            out.push_str(");\n");
            let _ = writeln!(out, "{pad}}}");
        }
        Expr::SeqIndex { seq, index, .. } => {
            let _ = writeln!(out, "{pad}{{");
            let inner = indent + 1;
            let ipad = indent_str(inner);
            let names = hoist_operands(out, &[seq.as_ref(), index.as_ref()], inner);
            let _ = writeln!(
                out,
                "{ipad}{dst} = _sir_seq_index({}, {});",
                names[0], names[1]
            );
            let _ = writeln!(out, "{pad}}}");
        }
        Expr::SeqLen { seq, .. } => {
            let _ = writeln!(out, "{pad}{{");
            let inner = indent + 1;
            let ipad = indent_str(inner);
            let names = hoist_operands(out, &[seq.as_ref()], inner);
            let _ = writeln!(out, "{ipad}{dst} = _sir_seq_len({});", names[0]);
            let _ = writeln!(out, "{pad}}}");
        }
        // SIR16 maps with a compound operand: hoist operands into temps
        // (preserving left-to-right order — for a `MapLit`, key/value
        // interleaved: k0, v0, k1, v1, …), then build/read the map.
        Expr::MapLit { entries, .. } => {
            let _ = writeln!(out, "{pad}{{");
            let inner = indent + 1;
            let ipad = indent_str(inner);
            let mut ops: Vec<&Expr> = Vec::with_capacity(entries.len() * 2);
            for e in entries {
                ops.push(&e.key);
                ops.push(&e.value);
            }
            let names = hoist_operands(out, &ops, inner);
            let _ = write!(out, "{ipad}{dst} = _sir_map_lit({}", entries.len());
            for n in &names {
                let _ = write!(out, ", {n}");
            }
            out.push_str(");\n");
            let _ = writeln!(out, "{pad}}}");
        }
        Expr::MapGet { map, key, .. } => {
            let _ = writeln!(out, "{pad}{{");
            let inner = indent + 1;
            let ipad = indent_str(inner);
            let names = hoist_operands(out, &[map.as_ref(), key.as_ref()], inner);
            let _ = writeln!(
                out,
                "{ipad}{dst} = _sir_map_get({}, {});",
                names[0], names[1]
            );
            let _ = writeln!(out, "{pad}}}");
        }
        // A call whose arguments contain control flow: hoist every argument
        // into a temp (left-to-right), then make the call.
        _ => emit_compound_call(out, dst, e, indent),
    }
}

/// Emit a block that leaves its `value` in `dst` (its `stmts` run for effect).
fn emit_block_assign(out: &mut String, dst: &str, b: &Block, indent: usize) {
    for s in &b.stmts {
        emit_stmt(out, s, indent);
    }
    emit_assign(out, dst, &b.value, indent);
}

/// Emit a condition expression, hoisting it into a temp first when it is not
/// simple.  Returns the C text to test with `_sir_truthy(...)`.
fn emit_cond(out: &mut String, cond: &Expr, indent: usize) -> String {
    if is_simple(cond) {
        let mut s = String::new();
        emit_expr(&mut s, cond, indent);
        s
    } else {
        let pad = indent_str(indent);
        let tmp = format!("_sir_c{}", fresh_id());
        let _ = writeln!(out, "{pad}SirValue {tmp};");
        emit_assign(out, &tmp, cond, indent);
        tmp
    }
}

/// Hoist a call's compound arguments into temps, then assign the call to
/// `dst`.  Every argument becomes a temp so left-to-right evaluation order is
/// preserved.
/// Hoist each operand into a fresh `SirValue` temporary (left-to-right, so
/// evaluation order is preserved even with side effects), returning the temp
/// names. A simple operand is assigned directly; a compound one goes through
/// `emit_assign`. Shared by the `Seq*` compound arms below.
fn hoist_operands(out: &mut String, ops: &[&Expr], indent: usize) -> Vec<String> {
    let ipad = indent_str(indent);
    let mut names = Vec::with_capacity(ops.len());
    for a in ops {
        let t = format!("_sir_a{}", fresh_id());
        if is_simple(a) {
            let _ = write!(out, "{ipad}SirValue {t} = ");
            emit_expr(out, a, indent);
            out.push_str(";\n");
        } else {
            let _ = writeln!(out, "{ipad}SirValue {t};");
            emit_assign(out, &t, a, indent);
        }
        names.push(t);
    }
    names
}

fn emit_compound_call(out: &mut String, dst: &str, e: &Expr, indent: usize) {
    let pad = indent_str(indent);
    let args = call_args(e).expect("emit_compound_call on a non-call expr");
    // SIR19 keyword call: a `DirectCall` carrying any `KeywordArg` needs by-name
    // resolution to the callee's declared slots (not the generic left-to-right
    // hoist below), so route it to the dedicated resolver.
    if let Expr::DirectCall { fn_name, .. } = e {
        if args.iter().any(|a| matches!(a, Expr::KeywordArg { .. })) {
            emit_keyword_call(out, dst, fn_name, args, indent);
            return;
        }
    }
    // SIR17 `raise Foo, <msg>` whose MESSAGE is a COMPOUND expression (so the
    // whole call is non-simple and lands here rather than in `emit_builtin_simple`).
    // The `Const` class name must become a raw C string — NOT a hoisted
    // `_sir_const_get`, which the C runtime has no builtin exception-class constant
    // for (see the simple-path arm) — so intercept it here, hoist ONLY the message,
    // and construct the exception directly.  Without this a computed-message
    // `raise ArgumentError, cond ? "a" : "b"` would regress to the same
    // `uninitialized constant` failure.  Mirrors the Go/Rust backends.
    if let Expr::BuiltinCall { name, args, .. } = e {
        if name == "raise" {
            if let Some(Expr::VarRef { name: cn, scope: Scope::Const, .. }) = args.first() {
                let _ = writeln!(out, "{pad}{{");
                let inner = indent + 1;
                let ipad = indent_str(inner);
                let msg = match args.get(1) {
                    Some(m) => {
                        let t = format!("_sir_a{}", fresh_id());
                        if is_simple(m) {
                            let _ = write!(out, "{ipad}SirValue {t} = ");
                            emit_expr(out, m, inner);
                            out.push_str(";\n");
                        } else {
                            let _ = writeln!(out, "{ipad}SirValue {t};");
                            emit_assign(out, &t, m, inner);
                        }
                        t
                    }
                    None => "_sir_nil()".to_string(),
                };
                let _ = writeln!(
                    out,
                    "{ipad}{dst} = _sir_raise(_sir_error({}, {}));",
                    quote_c_string(cn),
                    msg
                );
                let _ = writeln!(out, "{pad}}}");
                return;
            }
        }
    }
    let _ = writeln!(out, "{pad}{{");
    let inner = indent + 1;
    let ipad = indent_str(inner);
    let mut names = Vec::with_capacity(args.len());
    for a in args {
        let t = format!("_sir_a{}", fresh_id());
        if is_simple(a) {
            let _ = write!(out, "{ipad}SirValue {t} = ");
            emit_expr(out, a, inner);
            out.push_str(";\n");
        } else {
            let _ = writeln!(out, "{ipad}SirValue {t};");
            emit_assign(out, &t, a, inner);
        }
        names.push(t);
    }
    let _ = write!(out, "{ipad}{dst} = ");
    emit_call_with_arg_names(out, e, &names);
    out.push_str(";\n");
    let _ = writeln!(out, "{pad}}}");
}

/// Emit a `DirectCall` that carries keyword arguments (SIR19 `KeywordParams`).
///
/// C has no native keyword calls, so — like the Go backend's KW6 — a keyword
/// argument is resolved to the callee's declared parameter SLOT BY NAME at emit
/// time, producing a plain positional C call.  For each callee slot, in
/// declared order, the filler is: the leading positional argument at that index;
/// else the `KeywordArg` naming that parameter; else `_sir_missing()` (an
/// omitted optional slot — the validator guarantees a required parameter is
/// never left out, and the callee's prologue substitutes the default).  Each
/// filler expression is hoisted into a temp first (matching the statement-
/// oriented emitter), so a compound argument is evaluated exactly once; the
/// temps are computed in slot order (matching Go's declared-order evaluation).
fn emit_keyword_call(out: &mut String, dst: &str, fn_name: &str, args: &[Expr], indent: usize) {
    let pad = indent_str(indent);
    let inner = indent + 1;
    let ipad = indent_str(inner);

    // The validator guarantees every keyword argument trails all positionals, so
    // the first `KeywordArg` marks the split.
    let split = args
        .iter()
        .position(|a| matches!(a, Expr::KeywordArg { .. }))
        .unwrap_or(args.len());
    let positionals = &args[..split];
    let keywords: Vec<(&str, &Expr)> = args[split..]
        .iter()
        .map(|a| match a {
            Expr::KeywordArg { name, value, .. } => (name.as_str(), value.as_ref()),
            // Guaranteed by the validator (keywords trail positionals).
            other => unreachable!("keyword-call tail held a non-KeywordArg: {other:?}"),
        })
        .collect();

    // The callee's declared parameter names — present for any function that can
    // receive keywords (the validator rejects keyword calls to unknown callees).
    let param_names = callee_param_names(fn_name).unwrap_or_default();

    let _ = writeln!(out, "{pad}{{");
    // One entry per callee slot: `Some(temp)` for a filled slot, `None` for an
    // omitted optional (rendered as `_sir_missing()`).
    let mut slots: Vec<Option<String>> = Vec::with_capacity(param_names.len());
    for (i, pname) in param_names.iter().enumerate() {
        let fill: Option<&Expr> = if i < positionals.len() {
            Some(&positionals[i])
        } else {
            keywords
                .iter()
                .find(|(kw, _)| *kw == pname)
                .map(|(_, v)| *v)
        };
        match fill {
            Some(expr) => {
                let t = format!("_sir_k{}", fresh_id());
                if is_simple(expr) {
                    let _ = write!(out, "{ipad}SirValue {t} = ");
                    emit_expr(out, expr, inner);
                    out.push_str(";\n");
                } else {
                    let _ = writeln!(out, "{ipad}SirValue {t};");
                    emit_assign(out, &t, expr, inner);
                }
                slots.push(Some(t));
            }
            None => slots.push(None),
        }
    }
    let _ = write!(out, "{ipad}{dst} = {}(", function_emit_name(fn_name));
    for (i, slot) in slots.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        match slot {
            Some(t) => out.push_str(t),
            None => out.push_str("_sir_missing()"),
        }
    }
    out.push_str(");\n");
    let _ = writeln!(out, "{pad}}}");
}

fn call_args(e: &Expr) -> Option<&[Expr]> {
    match e {
        Expr::DirectCall { args, .. }
        | Expr::IndirectCall { args, .. }
        | Expr::BuiltinCall { args, .. } => Some(args),
        _ => None,
    }
}

/// Render a call whose arguments are already computed into the given temp
/// names.  Mirrors [`emit_expr`]'s call arms but substitutes names for args.
fn emit_call_with_arg_names(out: &mut String, e: &Expr, names: &[String]) {
    match e {
        Expr::DirectCall { fn_name, .. } => {
            let _ = write!(out, "{}(", function_emit_name(fn_name));
            push_joined(out, names);
            // SIR19: pad a call that omits trailing defaulted arguments.
            emit_default_padding(out, fn_name, names.len());
            out.push(')');
        }
        Expr::IndirectCall { target, .. } => {
            // target is simple (a compound target would itself be a hoisted
            // arg elsewhere); render it directly.
            let mut t = String::new();
            emit_expr(&mut t, target, 0);
            let _ = write!(out, "_sir_apply({}, {}", t, names.len());
            for n in names {
                let _ = write!(out, ", {n}");
            }
            out.push(')');
        }
        Expr::BuiltinCall { name, .. } => emit_builtin_with_names(out, name, names),
        _ => unreachable!("emit_call_with_arg_names on non-call"),
    }
}

fn push_joined(out: &mut String, names: &[String]) {
    for (i, n) in names.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push_str(n);
    }
}

// ---------------------------------------------------------------------------
// Simple expressions (rendered directly as C expression text)
// ---------------------------------------------------------------------------

/// A "simple" expression has no embedded control flow, so it can be rendered
/// as a single C expression with no statements hoisted out.
fn is_simple(e: &Expr) -> bool {
    match e {
        Expr::IntLit { .. }
        | Expr::FloatLit { .. }
        | Expr::BoolLit { .. }
        | Expr::NilLit { .. }
        | Expr::SymLit { .. }
        | Expr::StrLit { .. }
        | Expr::VarRef { .. } => true,
        Expr::Block(b) => b.stmts.is_empty() && is_simple(&b.value),
        Expr::If { .. } => false,
        // Short-circuit operators must not evaluate both operands eagerly.
        Expr::BuiltinCall { name, args, .. }
            if (name == "and" || name == "or") && args.len() == 2 =>
        {
            false
        }
        Expr::DirectCall { args, .. }
        | Expr::IndirectCall { args, .. }
        | Expr::BuiltinCall { args, .. } => args.iter().all(is_simple),
        Expr::MakeClosure { captures, .. } => captures.iter().all(|c| is_simple(&c.value)),
        // SIR26: a conversion is as simple as its value.
        Expr::Convert { value, .. } => is_simple(value),
        // SIR16 sequences: a `_sir_seq_*` call is simple iff its operands are
        // (they render inline as function arguments); otherwise the operands
        // are hoisted by `emit_assign`.
        Expr::SeqLit { items, .. } => items.iter().all(is_simple),
        Expr::SeqIndex { seq, index, .. } => is_simple(seq) && is_simple(index),
        Expr::SeqLen { seq, .. } => is_simple(seq),
        // SIR16 maps: a `_sir_map_*` call is simple iff every operand is (they
        // render inline as function arguments); otherwise `emit_assign` hoists
        // them. A `MapLit` operand set is every entry's key AND value.
        Expr::MapLit { entries, .. } => entries
            .iter()
            .all(|e| is_simple(&e.key) && is_simple(&e.value)),
        Expr::MapGet { map, key, .. } => is_simple(map) && is_simple(key),
        // SIR16+ nodes / Intrinsic are not accepted in v0 → unreachable after
        // the capability check.  Treat as non-simple defensively.
        _ => false,
    }
}

/// Render a simple `e` as C expression text.  Panics on a compound `e` (the
/// caller must route those through [`emit_assign`]).
fn emit_expr(out: &mut String, e: &Expr, indent: usize) {
    match e {
        Expr::IntLit { value, .. } => emit_int_literal(out, *value),
        Expr::FloatLit { value, .. } => emit_float_literal(out, *value),
        Expr::BoolLit { value, .. } => {
            let _ = write!(out, "_sir_bool({})", if *value { 1 } else { 0 });
        }
        Expr::NilLit { .. } => out.push_str("_sir_nil()"),
        Expr::SymLit { name, .. } => {
            let _ = write!(out, "_sir_sym({})", quote_c_string(name));
        }
        Expr::StrLit { value, .. } => {
            let _ = write!(out, "_sir_str({})", quote_c_string(value));
        }
        Expr::VarRef { name, scope, .. } => emit_var_ref(out, name, *scope),
        Expr::DirectCall { fn_name, args, .. } => {
            let _ = write!(out, "{}(", function_emit_name(fn_name));
            emit_simple_args(out, args, indent);
            // SIR19: pad a call that omits trailing defaulted arguments.
            emit_default_padding(out, fn_name, args.len());
            out.push(')');
        }
        Expr::IndirectCall { target, args, .. } => {
            out.push_str("_sir_apply(");
            emit_expr(out, target, indent);
            let _ = write!(out, ", {}", args.len());
            for a in args {
                out.push_str(", ");
                emit_expr(out, a, indent);
            }
            out.push(')');
        }
        Expr::BuiltinCall { name, args, .. } => emit_builtin_simple(out, name, args, indent),
        Expr::MakeClosure {
            fn_name, captures, ..
        } => {
            let _ = write!(
                out,
                "_sir_make_closure({}, {}",
                thunk_name(fn_name),
                captures.len()
            );
            for c in captures {
                out.push_str(", ");
                emit_expr(out, &c.value, indent);
            }
            out.push(')');
        }
        Expr::Block(b) => {
            // Simple only when stmts empty (guaranteed by is_simple).
            emit_expr(out, &b.value, indent);
        }
        Expr::Convert { value, to, .. } => emit_convert(out, value, to, indent),
        // SIR16 sequences (simple-operand forms; compound operands are hoisted
        // by `emit_assign`). `_sir_seq_lit(N, e0, e1, …)` boxes a fresh array;
        // `_sir_seq_index`/`_sir_seq_len` read it.
        Expr::SeqLit { items, .. } => {
            let _ = write!(out, "_sir_seq_lit({}", items.len());
            for it in items {
                out.push_str(", ");
                emit_expr(out, it, indent);
            }
            out.push(')');
        }
        Expr::SeqIndex { seq, index, .. } => {
            out.push_str("_sir_seq_index(");
            emit_expr(out, seq, indent);
            out.push_str(", ");
            emit_expr(out, index, indent);
            out.push(')');
        }
        Expr::SeqLen { seq, .. } => {
            out.push_str("_sir_seq_len(");
            emit_expr(out, seq, indent);
            out.push(')');
        }
        // SIR16 maps (simple-operand forms; compound operands are hoisted by
        // `emit_assign`). `_sir_map_lit(N, k0, v0, …)` boxes a fresh assoc-array
        // from N key/value pairs; `_sir_map_get(map, key)` reads it.
        Expr::MapLit { entries, .. } => {
            let _ = write!(out, "_sir_map_lit({}", entries.len());
            for e in entries {
                out.push_str(", ");
                emit_expr(out, &e.key, indent);
                out.push_str(", ");
                emit_expr(out, &e.value, indent);
            }
            out.push(')');
        }
        Expr::MapGet { map, key, .. } => {
            out.push_str("_sir_map_get(");
            emit_expr(out, map, indent);
            out.push_str(", ");
            emit_expr(out, key, indent);
            out.push(')');
        }
        other => unreachable!("emit_expr on compound/unsupported node: {other:?}"),
    }
}

/// SIR26 integer conversion → the portable `_sir_convert(v, bits, signed)`
/// runtime helper (two's-complement reduction), or the identity when the
/// target width is `Arbitrary` (a widen into the unbounded integer).
fn emit_convert(out: &mut String, value: &Expr, to: &IntSpec, indent: usize) {
    match to.width {
        IntWidth::Arbitrary => emit_expr(out, value, indent),
        w => {
            let bits = w.bits().expect("non-arbitrary width has bits");
            let signed = if to.signed { 1 } else { 0 };
            out.push_str("_sir_convert(");
            emit_expr(out, value, indent);
            let _ = write!(out, ", {bits}, {signed})");
        }
    }
}

fn emit_simple_args(out: &mut String, args: &[Expr], indent: usize) {
    for (i, a) in args.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        emit_expr(out, a, indent);
    }
}

fn emit_var_ref(out: &mut String, name: &str, scope: Scope) {
    match scope {
        Scope::Local | Scope::Param | Scope::Capture => out.push_str(&sanitize_ident(name)),
        Scope::Global => {
            let _ = write!(out, "_sir_global_get_s({})", quote_c_string(name));
        }
        Scope::Builtin => {
            let _ = write!(out, "_sir_builtin_closure({})", quote_c_string(name));
        }
        // OOP slice 1: a `Scope::Const` reference (`PI`, `Foo::Bar`) reads the
        // runtime constant table.  The name is a QUOTED C string literal (no
        // injection); an undefined constant raises `NameError` at runtime.
        Scope::Const => {
            let _ = write!(out, "_sir_const_get({})", quote_c_string(name));
        }
        // OOP slice 3: a `Scope::Instance` reference (`@v`) reads the current
        // receiver's instance-variable map (nil when unset).  The name (incl. the
        // `@`) is a QUOTED C string literal (no injection).
        Scope::Instance => {
            let _ = write!(out, "_sir_ivar_get({})", quote_c_string(name));
        }
        // OOP slice 6: a `Scope::ClassVar` reference (`@@x`) reads the current
        // class's class-variable storage (resolved via `_sir_current_class`; nil
        // when unset).  The `@@`-name is a QUOTED C string literal (no injection).
        // This is the LAST variable scope — the match is now exhaustive over
        // `Scope`, so an exhaustive match (no catch-all) is the compile-time
        // signal that every scope has a real emit path.
        Scope::ClassVar => {
            let _ = write!(out, "_sir_cvar_get({})", quote_c_string(name));
        }
    }
}

/// A builtin call whose arguments are all simple.
fn emit_builtin_simple(out: &mut String, name: &str, args: &[Expr], indent: usize) {
    // SIR17 `raise`.  The FIRST argument decides the shape (mirroring the
    // Go/Rust/JS/Python backends for cross-backend parity):
    //   • no argument — re-raise the exception currently being handled.
    //   • a `Const` first argument (`raise Foo` / `raise Foo, "msg"`) — the
    //     `Const` is a CLASS NAME, not a runtime value, so construct an exception
    //     of that class carrying the message (nil for a bare `raise Foo`, whose
    //     `#message` then defaults to the class name).  The class name is a
    //     QUOTED C string literal, so this Const is intercepted HERE and never
    //     reaches `emit_var_ref`'s `_sir_const_get` — the C runtime registers no
    //     builtin exception-class CONSTANTS, so `const_get("ArgumentError")` would
    //     (wrongly) raise `NameError: uninitialized constant ArgumentError`, the
    //     `puts(e)` conformance failure this fixes.
    //   • any other first argument (`raise "boom"`, `raise some_exc`) — a VALUE:
    //     an exception object is re-raised as-is, else wrapped in a `RuntimeError`
    //     carrying it as the message.
    // Never returns (longjmp/exit).
    if name == "raise" {
        match args.first() {
            None => out.push_str("_sir_raise(_sir_current_error)"),
            Some(Expr::VarRef { name: cn, scope: Scope::Const, .. }) => {
                let _ = write!(out, "_sir_raise(_sir_error({}, ", quote_c_string(cn));
                match args.get(1) {
                    Some(msg) => emit_expr(out, msg, indent),
                    None => out.push_str("_sir_nil()"),
                }
                out.push_str("))");
            }
            Some(_) => {
                out.push_str("_sir_raise_value(");
                emit_expr(out, &args[0], indent);
                out.push(')');
            }
        }
        return;
    }
    // OOP slice 1: `Foo.new` → `_sir_new_instance("Foo")`.  `args[0]` is the class
    // name (a `StrLit`), emitted as a QUOTED C string literal (no injection); the
    // scan guarantees exactly one `StrLit` arg (constructor args need `initialize`,
    // a later slice), so this arm never reaches the compound path.
    if name == "__new__" {
        if let Some(Expr::StrLit { value, .. }) = args.first() {
            let _ = write!(out, "_sir_new_instance({})", quote_c_string(value));
        } else {
            let _ = write!(out, "_sir_unknown_builtin({})", quote_c_string(name));
        }
        return;
    }
    // OOP slice 2: register an instance method.  `args = [StrLit(class),
    // StrLit(method), MakeClosure(fn)]`.  Class/method are QUOTED C string
    // literals (no injection); the closure is emitted as a `_sir_make_closure`.
    if name == "__def_method__" {
        if let (Some(Expr::StrLit { value: cls, .. }), Some(Expr::StrLit { value: m, .. }), Some(clo)) =
            (args.first(), args.get(1), args.get(2))
        {
            let _ = write!(out, "_sir_def_method({}, {}, ", quote_c_string(cls), quote_c_string(m));
            emit_expr(out, clo, indent);
            out.push(')');
        } else {
            let _ = write!(out, "_sir_unknown_builtin({})", quote_c_string(name));
        }
        return;
    }
    // OOP slice 2: dispatch an instance method.  `args = [recv, StrLit(method),
    // call-args…]`.  The method name is a QUOTED C string literal; dispatch is an
    // explicit `(class,method)` table lookup (anti-RCE — never reflection).
    if name == "__method__" {
        if let (Some(recv), Some(Expr::StrLit { value: m, .. })) = (args.first(), args.get(1)) {
            let call_args = &args[2..];
            // A user-registered method (OOP) dispatches through the class method
            // table; otherwise a KNOWN built-in name (Collections) routes to the
            // runtime dispatcher.  Both take the same `(recv, "m", argc, args…)`
            // shape, so only the helper name differs.  (The scan guarantees the
            // name is one or the other; a truly unknown name is rejected there.)
            let user = DEFINED_METHODS.with(|d| d.borrow().contains(m));
            let helper = if !user && is_builtin_method(m) {
                "_sir_builtin_method"
            } else {
                "_sir_call_method"
            };
            let _ = write!(out, "{helper}(");
            emit_expr(out, recv, indent);
            let _ = write!(out, ", {}, {}", quote_c_string(m), call_args.len());
            for a in call_args {
                out.push_str(", ");
                emit_expr(out, a, indent);
            }
            out.push(')');
        } else {
            let _ = write!(out, "_sir_unknown_builtin({})", quote_c_string(name));
        }
        return;
    }
    // OOP slice 5: register a CLASS method.  `args = [StrLit(class),
    // StrLit(method), MakeClosure(fn)]` — same shape as `__def_method__`, but it
    // populates the SEPARATE class-method (singleton) table.
    if name == "__def_class_method__" {
        if let (Some(Expr::StrLit { value: cls, .. }), Some(Expr::StrLit { value: m, .. }), Some(clo)) =
            (args.first(), args.get(1), args.get(2))
        {
            let _ = write!(out, "_sir_def_class_method({}, {}, ", quote_c_string(cls), quote_c_string(m));
            emit_expr(out, clo, indent);
            out.push(')');
        } else {
            let _ = write!(out, "_sir_unknown_builtin({})", quote_c_string(name));
        }
        return;
    }
    // OOP slice 5: dispatch a CLASS method.  `args = [StrLit(class),
    // StrLit(method), call-args…]` — the receiver is the class NAME (a StrLit),
    // not an instance expression, so both leading args are quoted C literals.
    if name == "__class_method__" {
        if let (Some(Expr::StrLit { value: cls, .. }), Some(Expr::StrLit { value: m, .. })) =
            (args.first(), args.get(1))
        {
            let call_args = &args[2..];
            let _ = write!(
                out,
                "_sir_call_class_method({}, {}, {}",
                quote_c_string(cls),
                quote_c_string(m),
                call_args.len()
            );
            for a in call_args {
                out.push_str(", ");
                emit_expr(out, a, indent);
            }
            out.push(')');
        } else {
            let _ = write!(out, "_sir_unknown_builtin({})", quote_c_string(name));
        }
        return;
    }
    // OOP slice 4: `super` — `args = [StrLit(method), StrLit(definingClass),
    // call-args…]`.  Resolve `method` from the SUPERCLASS of the defining class
    // and apply to the current self.  Method / defining-class are QUOTED C string
    // literals (no injection); the walk is an ancestry table lookup (anti-RCE).
    if name == "__super__" {
        if let (Some(Expr::StrLit { value: m, .. }), Some(Expr::StrLit { value: dc, .. })) =
            (args.first(), args.get(1))
        {
            let call_args = &args[2..];
            let _ = write!(
                out,
                "_sir_call_super({}, {}, {}",
                quote_c_string(m),
                quote_c_string(dc),
                call_args.len()
            );
            for a in call_args {
                out.push_str(", ");
                emit_expr(out, a, indent);
            }
            out.push(')');
        } else {
            let _ = write!(out, "_sir_unknown_builtin({})", quote_c_string(name));
        }
        return;
    }
    // OOP slice 7: `include M` / `extend M` — `args = [StrLit(class),
    // StrLit(module)]`.  Both names are QUOTED C string literals (no injection);
    // each records a `(class, module)` mixin the method resolvers consult.
    if name == "__include__" || name == "__extend__" {
        if let (Some(Expr::StrLit { value: cls, .. }), Some(Expr::StrLit { value: m, .. })) =
            (args.first(), args.get(1))
        {
            let helper = if name == "__include__" {
                "_sir_register_include"
            } else {
                "_sir_register_extend"
            };
            let _ = write!(out, "{}({}, {})", helper, quote_c_string(cls), quote_c_string(m));
        } else {
            let _ = write!(out, "_sir_unknown_builtin({})", quote_c_string(name));
        }
        return;
    }
    // OOP slice 3: a bare `self` → the current receiver (`_sir_self()`).
    if name == "__self__" {
        out.push_str("_sir_self()");
        return;
    }
    // Variadic-shaped builtins take (count, args...).
    if let Some(helper) = variadic_helper(name) {
        let _ = write!(out, "{}({}", helper, args.len());
        for a in args {
            out.push_str(", ");
            emit_expr(out, a, indent);
        }
        out.push(')');
        return;
    }
    if let Some((helper, arity)) = fixed_helper(name) {
        let _ = write!(out, "{helper}(");
        for i in 0..arity {
            if i > 0 {
                out.push_str(", ");
            }
            if let Some(a) = args.get(i) {
                emit_expr(out, a, indent);
            } else {
                out.push_str("_sir_nil()");
            }
        }
        out.push(')');
        return;
    }
    // Unknown builtin: fail loudly at runtime (no v0 program should reach it).
    let _ = write!(out, "_sir_unknown_builtin({})", quote_c_string(name));
}

/// The already-hoisted-args variant used by [`emit_compound_call`].
fn emit_builtin_with_names(out: &mut String, name: &str, names: &[String]) {
    if name == "raise" {
        if names.is_empty() {
            out.push_str("_sir_raise(_sir_current_error)");
        } else {
            let _ = write!(out, "_sir_raise_value({})", names[0]);
        }
        return;
    }
    // `__new__`/`__def_method__`/`__method__` carry a `StrLit` class/method name
    // (and a `MakeClosure`) that must stay a raw C literal, which the compound
    // (already-emitted `names`) path cannot recover.  The scan rejects any such
    // call that is not `is_simple` (a control-flow argument), so these arms are
    // unreachable — emit a clear marker rather than a wrong construction.
    if matches!(name, "__new__" | "__def_method__" | "__method__") {
        let _ = write!(out, "_sir_unknown_builtin({})", quote_c_string(name));
        return;
    }
    // `__self__` takes no arguments, so it is always simple; render it here too.
    if name == "__self__" {
        out.push_str("_sir_self()");
        return;
    }
    if let Some(helper) = variadic_helper(name) {
        let _ = write!(out, "{}({}", helper, names.len());
        for n in names {
            let _ = write!(out, ", {n}");
        }
        out.push(')');
        return;
    }
    if let Some((helper, arity)) = fixed_helper(name) {
        let _ = write!(out, "{helper}(");
        for i in 0..arity {
            if i > 0 {
                out.push_str(", ");
            }
            match names.get(i) {
                Some(n) => out.push_str(n),
                None => out.push_str("_sir_nil()"),
            }
        }
        out.push(')');
        return;
    }
    let _ = write!(out, "_sir_unknown_builtin({})", quote_c_string(name));
}

/// Builtins that take `(int count, ...)`.
fn variadic_helper(name: &str) -> Option<&'static str> {
    Some(match name {
        "+" => "_sir_plus",
        "-" => "_sir_minus",
        "*" => "_sir_times",
        "/" => "_sir_divide",
        // Ruby unary minus (`-x`) lowers to `BuiltinCall("neg", [x])`. It was
        // UNIMPLEMENTED here, so any negative literal made the C backend report
        // an unsupported builtin and skip. Unary minus IS single-argument
        // subtraction, and `_sir_minus_v` already negates its single argument
        // tag-preservingly (a `SIR_FLOAT` stays float, else int) — so `neg`
        // lowers to `_sir_minus(1, x)` with no new runtime code, matching the
        // Go/Rust/Python runtimes that gained `neg` in SIR21 §E3.
        "neg" => "_sir_minus",
        "print" => "_sir_print",
        "puts" => "_sir_puts",
        _ => return None,
    })
}

/// Collections: the built-in *method* names the C runtime's `_sir_builtin_method`
/// dispatches.  A `__method__` call to one of these — when the module has NOT
/// defined a user method of the same name — routes to the runtime dispatcher
/// instead of the user method table, and the structural scan accepts it (rather
/// than deferring it to a later Collections batch).  Slice 1 = 0-arity String
/// methods (`length`/`size`/`empty?` polymorphic over String/Array/Hash); slice 2
/// = 1-arg String queries (`include?`/`start_with?`/`end_with?`/`index`, the arg
/// a String).  The dispatcher raises `NoMethodError` on an unsupported receiver
/// type, matching Ruby.
fn is_builtin_method(name: &str) -> bool {
    matches!(
        name,
        // slice 1 — 0-arity
        "length" | "size" | "upcase" | "downcase" | "reverse" | "empty?" | "to_s"
        // slice 2 — 1-arg String queries (the arg is a String)
        | "include?" | "start_with?" | "end_with?" | "index"
    )
}

/// The builtins the v0 emitter can lower directly.  Anything else — the
/// `__method__` collection-dispatch protocol, the OOP/exception builtins
/// (`__new__`, `raise`, …) — belongs to a later batch and is rejected up front
/// by [`first_unsupported_builtin`] rather than emitted as a call that fails at
/// runtime.
fn is_supported_builtin(name: &str) -> bool {
    variadic_helper(name).is_some()
        || fixed_helper(name).is_some()
        // OOP slice 1 `__new__` (construct an instance) + slice 2 instance
        // methods: `__def_method__` (register a `(class,method)` closure) and
        // `__method__` (dispatch it).  (`__super__`/`__self__`/`__class_method__`/
        // … stay unsupported — later slices.)
        // Slice 3 adds `__self__` (a bare `self`); slice 4 adds `__super__`
        // (`super`); slice 5 adds class methods `__def_class_method__` /
        // `__class_method__`.  (`@@class` vars and modules stay unsupported —
        // later slices.)
        || matches!(
            name,
            "and" | "or"
                | "raise"
                | "__new__"
                | "__def_method__"
                | "__method__"
                | "__self__"
                | "__super__"
                | "__def_class_method__"
                | "__class_method__"
                | "__include__"
                | "__extend__"
        )
}

// The structural gate below (`first_unsupported_builtin`) reports the first
// `BuiltinCall` the v0 emitter cannot lower — some builtins (notably `__method__`)
// are not gated by an unaccepted feature, so a module can pass the capability
// check yet still contain an unlowerable builtin — plus the out-of-slice OOP
// shapes.  The thread-local below carries its `__method__` allowlist.
thread_local! {
    // The set of method names the module registers via `__def_method__` — the
    // CLOSED set that `__method__` dispatch may target (OOP slice 2). A dispatch to
    // any OTHER name is a built-in-method call (the Collections batch), rejected
    // until then. Populated once per `first_unsupported_builtin` run (compilation
    // is single-threaded per module), read in the scan. (Dispatch is already
    // anti-RCE by construction — an explicit (class,method) table lookup — so this
    // allowlist is purely for clean COMPILE-TIME rejection.)
    static DEFINED_METHODS: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
    // OOP slice 5: the same closed-set idea for CLASS methods — the names the
    // module registers via `__def_class_method__`, the only names a
    // `__class_method__` dispatch may target (else it is a built-in class method,
    // the Collections batch, rejected until then).
    static DEFINED_CLASS_METHODS: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
}

/// Collect the `__def_method__("Class", "m", …)` method names anywhere in `m`
/// (a full recursive walk, so a dispatch is never wrongly rejected for a
/// registration nested in a loop/branch/closure).
fn collect_defined_methods(m: &Module, out: &mut (HashSet<String>, HashSet<String>)) {
    // `out.0` = instance-method names (`__def_method__`); `out.1` = class-method
    // names (`__def_class_method__`).  Both are collected in ONE walk so the two
    // dispatch allowlists (instance `__method__`, class `__class_method__`) stay
    // in lockstep with what the module actually registers.
    fn from_expr(e: &Expr, out: &mut (HashSet<String>, HashSet<String>)) {
        match e {
            Expr::BuiltinCall { name, args, .. } => {
                if name == "__def_method__" {
                    if let Some(Expr::StrLit { value, .. }) = args.get(1) {
                        out.0.insert(value.clone());
                    }
                } else if name == "__def_class_method__" {
                    if let Some(Expr::StrLit { value, .. }) = args.get(1) {
                        out.1.insert(value.clone());
                    }
                }
                args.iter().for_each(|a| from_expr(a, out));
            }
            Expr::DirectCall { args, .. } => args.iter().for_each(|a| from_expr(a, out)),
            Expr::IndirectCall { target, args, .. } => {
                from_expr(target, out);
                args.iter().for_each(|a| from_expr(a, out));
            }
            Expr::KeywordArg { value, .. } => from_expr(value, out),
            Expr::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                from_expr(cond, out);
                from_block(then_branch, out);
                from_block(else_branch, out);
            }
            Expr::Block(b) => from_block(b, out),
            Expr::MakeClosure { captures, .. } => {
                captures.iter().for_each(|c| from_expr(&c.value, out))
            }
            Expr::Convert { value, .. } => from_expr(value, out),
            Expr::SeqLit { items, .. } => items.iter().for_each(|i| from_expr(i, out)),
            Expr::SeqIndex { seq, index, .. } => {
                from_expr(seq, out);
                from_expr(index, out);
            }
            Expr::SeqLen { seq, .. } => from_expr(seq, out),
            Expr::MapLit { entries, .. } => entries.iter().for_each(|e| {
                from_expr(&e.key, out);
                from_expr(&e.value, out);
            }),
            Expr::MapGet { map, key, .. } => {
                from_expr(map, out);
                from_expr(key, out);
            }
            Expr::LogicalAnd { lhs, rhs, .. } | Expr::LogicalOr { lhs, rhs, .. } => {
                from_expr(lhs, out);
                from_expr(rhs, out);
            }
            _ => {}
        }
    }
    fn from_stmt(s: &Stmt, out: &mut (HashSet<String>, HashSet<String>)) {
        match s {
            Stmt::LetBinding { value, .. }
            | Stmt::LetStarBinding { value, .. }
            | Stmt::ExprStmt { expr: value, .. }
            | Stmt::Assign { value, .. } => from_expr(value, out),
            Stmt::While { cond, body, .. } => {
                from_expr(cond, out);
                from_block(body, out);
            }
            Stmt::ForRange {
                start,
                stop,
                step,
                body,
                ..
            } => {
                from_expr(start, out);
                from_expr(stop, out);
                from_expr(step, out);
                from_block(body, out);
            }
            Stmt::ForEach { iter, body, .. } => {
                from_expr(iter, out);
                from_block(body, out);
            }
            Stmt::SeqSet {
                seq, index, value, ..
            } => {
                from_expr(seq, out);
                from_expr(index, out);
                from_expr(value, out);
            }
            Stmt::MapSet {
                map, key, value, ..
            } => {
                from_expr(map, out);
                from_expr(key, out);
                from_expr(value, out);
            }
            Stmt::TryCatch {
                body,
                rescues,
                ensure_body,
                ..
            } => {
                body.iter().for_each(|s| from_stmt(s, out));
                rescues
                    .iter()
                    .for_each(|rc| rc.body.iter().for_each(|s| from_stmt(s, out)));
                if let Some(e) = ensure_body {
                    e.iter().for_each(|s| from_stmt(s, out));
                }
            }
            _ => {}
        }
    }
    fn from_block(b: &Block, out: &mut (HashSet<String>, HashSet<String>)) {
        b.stmts.iter().for_each(|s| from_stmt(s, out));
        from_expr(&b.value, out);
    }
    for f in &m.functions {
        for p in &f.params {
            if let Some(d) = &p.default {
                from_expr(d, out);
            }
        }
        from_block(&f.body, out);
    }
}

pub fn first_unsupported_builtin(m: &Module) -> Option<(String, Span)> {
    let mut defined = (HashSet::new(), HashSet::new());
    collect_defined_methods(m, &mut defined);
    DEFINED_METHODS.with(|d| *d.borrow_mut() = defined.0);
    DEFINED_CLASS_METHODS.with(|d| *d.borrow_mut() = defined.1);
    for f in &m.functions {
        // A SIR19 parameter default is an expression the prologue evaluates at
        // call time, so a deferred builtin nested in one must be pre-checked
        // too — otherwise it would slip past the body scan and reach the
        // emitter's `unreachable!`.
        for p in &f.params {
            if let Some(default) = &p.default {
                if let Some(hit) = scan_expr_for_builtin(default) {
                    return Some(hit);
                }
            }
        }
        if let Some(hit) = scan_block_for_builtin(&f.body) {
            return Some(hit);
        }
    }
    None
}

fn scan_block_for_builtin(b: &Block) -> Option<(String, Span)> {
    b.stmts
        .iter()
        .find_map(scan_stmt_for_builtin)
        .or_else(|| scan_expr_for_builtin(&b.value))
}

fn scan_stmt_for_builtin(s: &Stmt) -> Option<(String, Span)> {
    match s {
        Stmt::LetBinding { value, .. }
        | Stmt::LetStarBinding { value, .. }
        | Stmt::ExprStmt { expr: value, .. }
        | Stmt::Assign { value, .. } => scan_expr_for_builtin(value),
        // Loop bodies must be scanned too, or an unsupported builtin hidden
        // in a `while`/`for` body escapes the pre-check and hits the
        // emitter's `unreachable!`. (`While` was a pre-existing scan hole.)
        Stmt::While { cond, body, .. } => {
            scan_expr_for_builtin(cond).or_else(|| scan_block_for_builtin(body))
        }
        Stmt::ForRange {
            start,
            stop,
            step,
            body,
            ..
        } => scan_expr_for_builtin(start)
            .or_else(|| scan_expr_for_builtin(stop))
            .or_else(|| scan_expr_for_builtin(step))
            .or_else(|| scan_block_for_builtin(body)),
        // Sequence write / iteration bodies likewise carry sub-expressions.
        Stmt::SeqSet {
            seq, index, value, ..
        } => scan_expr_for_builtin(seq)
            .or_else(|| scan_expr_for_builtin(index))
            .or_else(|| scan_expr_for_builtin(value)),
        Stmt::MapSet {
            map, key, value, ..
        } => scan_expr_for_builtin(map)
            .or_else(|| scan_expr_for_builtin(key))
            .or_else(|| scan_expr_for_builtin(value)),
        Stmt::ForEach { iter, body, .. } => {
            scan_expr_for_builtin(iter).or_else(|| scan_block_for_builtin(body))
        }
        // A `begin … rescue … ensure … end` — scan the guarded body, every
        // rescue clause body, and the ensure body, so an unsupported builtin
        // hidden in any of them is caught by the pre-check, not the emitter.
        Stmt::TryCatch {
            body,
            rescues,
            ensure_body,
            ..
        } => body
            .iter()
            .find_map(scan_stmt_for_builtin)
            .or_else(|| {
                rescues
                    .iter()
                    .find_map(|rc| rc.body.iter().find_map(scan_stmt_for_builtin))
            })
            .or_else(|| {
                ensure_body
                    .as_ref()
                    .and_then(|e| e.iter().find_map(scan_stmt_for_builtin))
            }),
        // OOP slice 1: a `Stmt::SingletonClassDef` (`class << self`) ALSO observes
        // `Feature::Classes`, so accepting `Classes` obligates rejecting it here —
        // else it reaches the emitter's `unreachable!` (a DoS on a hand-built
        // module).  Deferred to a later slice.  (`Stmt::ClassDef` is NOT rejected
        // — it emits a comment / mixin registrations.)
        Stmt::SingletonClassDef { span, .. } => {
            Some(("class << self (singleton class)".to_string(), span.clone()))
        }
        // OOP slice 7: a `Stmt::ModuleDef` emits only a comment (its methods are
        // registered separately via `__def_method__`), so a NON-EMPTY module body
        // (module-level code) would be silently dropped — reject it cleanly.
        Stmt::ModuleDef { body, span, .. } if !body.is_empty() => Some((
            "a module with a non-empty body (module-level code is not yet lowered)".to_string(),
            span.clone(),
        )),
        // OOP slice 6: a `Stmt::ClassDef` body may contain ONLY `@@x = …`
        // class-variable initializers (emitted as `_sir_cvar_set_in`).  Any other
        // class-level statement would be silently dropped by the emit, so reject
        // it cleanly; each accepted initializer's VALUE is scanned so a deferred
        // builtin nested in it is still reported.
        Stmt::ClassDef { body, span, .. } => {
            for st in body {
                match st {
                    Stmt::Assign {
                        scope: Scope::ClassVar,
                        value,
                        ..
                    } => {
                        if let Some(hit) = scan_expr_for_builtin(value) {
                            return Some(hit);
                        }
                    }
                    _ => {
                        return Some((
                            "class-level code other than a `@@x` initializer \
                             (a later OOP slice)"
                                .to_string(),
                            span.clone(),
                        ));
                    }
                }
            }
            None
        }
        _ => None,
    }
}

fn scan_expr_for_builtin(e: &Expr) -> Option<(String, Span)> {
    match e {
        Expr::BuiltinCall {
            name, args, span, ..
        } => {
            if !is_supported_builtin(name) {
                return Some((name.clone(), span.clone()));
            }
            // OOP slice 1: `__new__` must be `[StrLit(class)]` — exactly one
            // constant class name, no constructor arguments (those need
            // `initialize`, a later slice).  Anything else is rejected cleanly so
            // the emitter's single-`StrLit` assumption holds.
            if name == "__new__"
                && !matches!(args.as_slice(), [Expr::StrLit { .. }])
            {
                return Some((
                    "__new__ with constructor arguments or a non-constant class name".to_string(),
                    span.clone(),
                ));
            }
            // OOP slice 2: `__def_method__` must be `[StrLit(class), StrLit(method),
            // MakeClosure]` — the emitter reads all three; a malformed shape would
            // otherwise emit a runtime-failing marker instead of rejecting.
            if name == "__def_method__"
                && !matches!(
                    args.as_slice(),
                    [Expr::StrLit { .. }, Expr::StrLit { .. }, Expr::MakeClosure { .. }]
                )
            {
                return Some(("a malformed __def_method__ registration".to_string(), span.clone()));
            }
            // `__method__` must have a receiver and a `StrLit` method name.
            if name == "__method__"
                && !matches!((args.first(), args.get(1)), (Some(_), Some(Expr::StrLit { .. })))
            {
                return Some(("a malformed __method__ dispatch".to_string(), span.clone()));
            }
            // OOP slice 4: `__super__` must be `[StrLit(method), StrLit(class), …]`
            // — the emitter reads both names as raw C literals.
            if name == "__super__"
                && !matches!(
                    (args.first(), args.get(1)),
                    (Some(Expr::StrLit { .. }), Some(Expr::StrLit { .. }))
                )
            {
                return Some(("a malformed __super__ call".to_string(), span.clone()));
            }
            // OOP slice 7: `__include__`/`__extend__` are `[StrLit(class),
            // StrLit(module)]` — both names emitted as raw C literals.
            if matches!(name.as_str(), "__include__" | "__extend__")
                && !matches!(
                    (args.first(), args.get(1)),
                    (Some(Expr::StrLit { .. }), Some(Expr::StrLit { .. }))
                )
            {
                return Some((format!("a malformed {name} call"), span.clone()));
            }
            // OOP slice 5: `__def_class_method__` mirrors `__def_method__` —
            // `[StrLit(class), StrLit(method), MakeClosure]`.
            if name == "__def_class_method__"
                && !matches!(
                    args.as_slice(),
                    [Expr::StrLit { .. }, Expr::StrLit { .. }, Expr::MakeClosure { .. }]
                )
            {
                return Some((
                    "a malformed __def_class_method__ registration".to_string(),
                    span.clone(),
                ));
            }
            // OOP slice 5: `__class_method__` is `[StrLit(class), StrLit(method), …]`
            // — the receiver is the class NAME (a `StrLit`), not an instance expr.
            if name == "__class_method__"
                && !matches!(
                    (args.first(), args.get(1)),
                    (Some(Expr::StrLit { .. }), Some(Expr::StrLit { .. }))
                )
            {
                return Some(("a malformed __class_method__ dispatch".to_string(), span.clone()));
            }
            // These OOP builtins carry `StrLit` class/method names that must stay
            // raw C literals.  The compound (control-flow-argument) path cannot
            // recover them, so reject a call that is not `is_simple` — deferred,
            // not mis-emitted.
            if matches!(
                name.as_str(),
                "__def_method__"
                    | "__method__"
                    | "__super__"
                    | "__def_class_method__"
                    | "__class_method__"
                    | "__include__"
                    | "__extend__"
            ) && !is_simple(e)
            {
                return Some((
                    format!("a `{name}` call with a control-flow argument"),
                    span.clone(),
                ));
            }
            // `__method__(recv, "m", …)` dispatch: accepted if `m` is a
            // user-registered method (OOP, via the class table) OR a built-in
            // method the Collections runtime dispatches (`is_builtin_method`).  A
            // name that is NEITHER is a built-in method not in this slice yet —
            // rejected cleanly (dispatch is anti-RCE by construction; this keeps
            // the clean compile-time rejection for the not-yet-lowered names).
            if name == "__method__" {
                if let Some(Expr::StrLit { value, .. }) = args.get(1) {
                    let known = DEFINED_METHODS.with(|d| d.borrow().contains(value))
                        || is_builtin_method(value);
                    if !known {
                        return Some((
                            format!(
                                "a call to the built-in method `{value}` \
                                 (not lowered by the C backend yet)"
                            ),
                            span.clone(),
                        ));
                    }
                }
            }
            // OOP slice 5: same allowlist for a `__class_method__` dispatch — a
            // name the module never registers via `__def_class_method__` is a
            // built-in class method (`Foo.name`, the Collections batch), rejected
            // cleanly.  (The method name is args[1]; args[0] is the class NAME.)
            if name == "__class_method__" {
                if let Some(Expr::StrLit { value, .. }) = args.get(1) {
                    // A class-method dispatch may target a registered class method
                    // OR — via OOP slice 7 `extend M` — one of a module's INSTANCE
                    // methods (registered through `__def_method__`).  So the
                    // allowlist is the UNION; a name in neither is a built-in class
                    // method (`Foo.name`, the Collections batch), rejected cleanly.
                    let known = DEFINED_CLASS_METHODS.with(|d| d.borrow().contains(value))
                        || DEFINED_METHODS.with(|d| d.borrow().contains(value));
                    if !known {
                        return Some((
                            format!(
                                "a call to the built-in class method `{value}` (only \
                                 user-defined class methods dispatch this slice)"
                            ),
                            span.clone(),
                        ));
                    }
                }
            }
            args.iter().find_map(scan_expr_for_builtin)
        }
        Expr::DirectCall { args, .. } => args.iter().find_map(scan_expr_for_builtin),
        Expr::IndirectCall { target, args, .. } => {
            scan_expr_for_builtin(target).or_else(|| args.iter().find_map(scan_expr_for_builtin))
        }
        // A keyword argument (in a `DirectCall`'s arg list) carries its value as
        // a sub-expression — scan it so a deferred builtin in `f(x: foo())` is
        // reported cleanly.
        Expr::KeywordArg { value, .. } => scan_expr_for_builtin(value),
        Expr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => scan_expr_for_builtin(cond)
            .or_else(|| scan_block_for_builtin(then_branch))
            .or_else(|| scan_block_for_builtin(else_branch)),
        Expr::Block(b) => scan_block_for_builtin(b),
        Expr::MakeClosure { captures, .. } => captures
            .iter()
            .find_map(|c| scan_expr_for_builtin(&c.value)),
        Expr::Convert { value, .. } => scan_expr_for_builtin(value),
        Expr::SeqLit { items, .. } => items.iter().find_map(scan_expr_for_builtin),
        Expr::SeqIndex { seq, index, .. } => {
            scan_expr_for_builtin(seq).or_else(|| scan_expr_for_builtin(index))
        }
        Expr::SeqLen { seq, .. } => scan_expr_for_builtin(seq),
        Expr::MapLit { entries, .. } => entries.iter().find_map(|e| {
            scan_expr_for_builtin(&e.key).or_else(|| scan_expr_for_builtin(&e.value))
        }),
        Expr::MapGet { map, key, .. } => {
            scan_expr_for_builtin(map).or_else(|| scan_expr_for_builtin(key))
        }
        Expr::LogicalAnd { lhs, rhs, .. } | Expr::LogicalOr { lhs, rhs, .. } => {
            scan_expr_for_builtin(lhs).or_else(|| scan_expr_for_builtin(rhs))
        }
        _ => None,
    }
}

/// Builtins with a fixed arity.
fn fixed_helper(name: &str) -> Option<(&'static str, usize)> {
    Some(match name {
        "=" => ("_sir_eq", 2),
        "case_eq" => ("_sir_case_eq", 2),
        "<" => ("_sir_lt", 2),
        ">" => ("_sir_gt", 2),
        // Milestone 2 comparisons (the C frontend emits the operator spellings).
        "<=" => ("_sir_le", 2),
        ">=" => ("_sir_ge", 2),
        "==" => ("_sir_eq", 2),
        "!=" => ("_sir_ne", 2),
        // Milestone 4 logical negation (`&&`/`||` short-circuit via emit_assign).
        "not" => ("_sir_not", 1),
        // Milestone 5 bitwise / shift.
        "&" => ("_sir_band", 2),
        "|" => ("_sir_bor", 2),
        "^" => ("_sir_bxor", 2),
        "~" => ("_sir_bnot", 1),
        "<<" => ("_sir_shl", 2),
        ">>" => ("_sir_shr", 2),
        "u>>" => ("_sir_lshr", 2),
        // Milestone 6 truncating division / remainder (signed + unsigned).
        "tdiv" => ("_sir_itdiv", 2),
        "tmod" => ("_sir_itmod", 2),
        "utdiv" => ("_sir_utdiv", 2),
        "utmod" => ("_sir_utmod", 2),
        // Milestone 9 numeric conversions: `to_f` (int → double) and `to_i`
        // (double → int, truncating toward zero, matching C's `(int)double`).
        "to_f" => ("_sir_to_f", 1),
        "to_i" => ("_sir_to_i", 1),
        // Milestone 10 faithful printf: `fmt_float(value, precision, kind)`
        // renders a double exactly as C's printf `%f`/`%e`/`%g` (+ uppercase).
        "fmt_float" => ("_sir_fmt_float_c", 3),
        "cons" => ("_sir_cons", 2),
        "car" => ("_sir_car", 1),
        "cdr" => ("_sir_cdr", 1),
        "null?" => ("_sir_is_null", 1),
        "pair?" => ("_sir_is_pair", 1),
        "number?" => ("_sir_is_number", 1),
        "symbol?" => ("_sir_is_symbol", 1),
        "global_get" => ("_sir_global_get", 1),
        "global_set" => ("_sir_global_set", 2),
        _ => return None,
    })
}

// ---------------------------------------------------------------------------
// Literals / identifiers / escaping
// ---------------------------------------------------------------------------

fn emit_int_literal(out: &mut String, value: i64) {
    if value == i64::MIN {
        // The literal -9223372036854775808 would be parsed as unary-minus of a
        // value that overflows a signed 64-bit int, so build it safely.
        out.push_str("_sir_int((-9223372036854775807LL - 1))");
    } else {
        let _ = write!(out, "_sir_int({value}LL)");
    }
}

/// Render a `FloatLit` as a `_sir_float(<C double>)` constructor call.
///
/// - **Non-finite** values have no C floating literal (a program cannot write
///   `inf` / `nan` as tokens), so they use the C99 `<math.h>` macros
///   `INFINITY` / `NAN` (mirroring the Ruby backend's `Float::INFINITY` /
///   `Float::NAN`). These arise only from a hand-built module — normal float
///   arithmetic produces them at runtime, where `_sir_divide_v` already yields
///   IEEE `inf`/`nan` without any literal.
/// - **Finite** values use Rust's `{:?}` (Debug) form, whose shortest
///   round-tripping spelling always carries a decimal point or exponent
///   (`7.0`, `-0.0`, `1e300`) — each a valid C `double` literal that `strtod`
///   parses back to the identical bit pattern. (A plain `{}` would drop the
///   point on an integral value, but C would still read `7` as a `double` in
///   this context; `{:?}` is used for parity with the value model and to keep
///   the emitted text unambiguously floating-point.)
fn emit_float_literal(out: &mut String, value: f64) {
    if value.is_nan() {
        out.push_str("_sir_float(NAN)");
    } else if value.is_infinite() {
        out.push_str(if value > 0.0 {
            "_sir_float(INFINITY)"
        } else {
            "_sir_float(-INFINITY)"
        });
    } else {
        let _ = write!(out, "_sir_float({value:?})");
    }
}

/// Map a SIR identifier into a valid C identifier: non-`[A-Za-z0-9_]` chars are
/// escaped, a leading digit is escaped, C keywords get a trailing `_`, and the
/// runtime's own namespace is kept clear.
pub fn sanitize_ident(s: &str) -> String {
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        let ok = ch.is_ascii_alphanumeric() || ch == '_';
        if ok && !(i == 0 && ch.is_ascii_digit()) {
            out.push(ch);
        } else {
            let _ = write!(out, "_u{:04x}_", ch as u32);
        }
    }
    if out.is_empty() {
        out.push('_');
    }
    if is_c_keyword(&out)
        || is_reserved_macro(&out)
        || out.starts_with("_sir")
        || out == "main"
        || out == "user_main"
        || out == "sir_user_init"
    {
        out.push('_');
    }
    out
}

/// Standard-library / platform *function-like macros* that are not C keywords
/// but would wreck a same-named function definition.  `<stdlib.h>` on MSVC/UCRT
/// (and `<windows.h>`) define `min`/`max` as macros, so `SirValue min(SirValue
/// a, SirValue b)` expands to garbage under clang-cl / MSVC — a portability trap
/// the three-compiler mandate must dodge.  Escaping them (a trailing `_`) keeps
/// a user function called `min` compiling everywhere.
fn is_reserved_macro(s: &str) -> bool {
    matches!(s, "min" | "max")
}

fn is_c_keyword(s: &str) -> bool {
    matches!(
        s,
        "auto"
            | "break"
            | "case"
            | "char"
            | "const"
            | "continue"
            | "default"
            | "do"
            | "double"
            | "else"
            | "enum"
            | "extern"
            | "float"
            | "for"
            | "goto"
            | "if"
            | "inline"
            | "int"
            | "long"
            | "register"
            | "restrict"
            | "return"
            | "short"
            | "signed"
            | "sizeof"
            | "static"
            | "struct"
            | "switch"
            | "typedef"
            | "union"
            | "unsigned"
            | "void"
            | "volatile"
            | "while"
            | "_Bool"
            | "_Complex"
            | "_Imaginary"
            | "bool"
            | "true"
            | "false"
            | "NULL"
    )
}

/// Escape a Rust string into a C double-quoted string literal (including the
/// surrounding quotes).  Non-printable / non-ASCII bytes become octal escapes,
/// so no source text can break out of the literal.
fn quote_c_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for b in s.bytes() {
        match b {
            b'"' => out.push_str("\\\""),
            b'\\' => out.push_str("\\\\"),
            b'\n' => out.push_str("\\n"),
            b'\r' => out.push_str("\\r"),
            b'\t' => out.push_str("\\t"),
            // Escape `?` as `\?` to defeat C **trigraphs**: `??/` would be
            // rewritten to `\` in translation phase 1 (before string lexing) —
            // GCC enables trigraphs under `-std=c99` — and a stray `\` before
            // our escaped closing quote could break out of the literal.  `\?`
            // is `?`, so no `??` pair can ever survive.  (Source-injection
            // defence — the emitted string must stay a string.)
            b'?' => out.push_str("\\?"),
            0x20..=0x7e => out.push(b as char),
            _ => {
                let _ = write!(out, "\\{b:03o}");
            }
        }
    }
    out.push('"');
    out
}

/// Strip line terminators from text destined for a `/* … */` comment so it
/// cannot break out of the comment or inject code.
fn sanitize_comment(s: &str) -> String {
    s.replace(['\n', '\r'], " ").replace("*/", "* /")
}

fn indent_str(level: usize) -> String {
    "    ".repeat(level)
}
