//! Node lowering — turns a `semantic_ir::Module` into self-contained Ruby.
//!
//! Ruby is **expression-oriented**: `if`/`begin…end` yield values and a method
//! returns its last expression.  So — unlike the Go/C backends — every SIR node
//! renders **directly** as a Ruby expression with no statement-hoisting, and
//! [`emit_expr`] is total (it handles `If`/`Block` too).
//!
//! See [SIR25](../../../specs/SIR25-semantic-ir-to-ruby.md) for the design.

use std::collections::HashSet;
use std::fmt::Write;
use std::sync::atomic::{AtomicUsize, Ordering};

use semantic_ir::{Block, Expr, Function, Global, IntWidth, Module, ParamKind, Scope, Stmt};

use crate::runtime::RUNTIME;

/// Monotonic counter for unique loop-temporary names (`sir_for_i_N`, …). A
/// `ForRange` desugars to a `while` with three temporaries that must not
/// collide across NESTED range loops. The `sir_` prefix is guarded by
/// [`sanitize_ident`] (a user variable spelled the same is renamed away), so
/// these can never clash with a program's own names.
static LOOP_TEMP_ID: AtomicUsize = AtomicUsize::new(0);

/// Builtins the v0 backend lowers.  A `BuiltinCall` to anything else is
/// rejected by [`first_scan_issue`] (mirroring the C backend), so a
/// module using e.g. the `__method__` collection-dispatch protocol fails
/// cleanly rather than emitting a call with no lowering.
const SUPPORTED_BUILTINS: &[&str] = &[
    "+",
    "-",
    "*",
    "/",
    "%",
    "neg",
    // Bitwise / shift (SIR27 milestone 5) — native Ruby Integer operators.
    "&",
    "|",
    "^",
    "~",
    "<<",
    ">>",
    "u>>",
    // Truncating division / remainder (SIR27 milestone 6) — distinct from the
    // flooring `/`/`%` because C truncates toward zero; `u`-variants for the
    // unsigned common type.
    "tdiv",
    "tmod",
    "utdiv",
    "utmod",
    // Numeric conversions (SIR27 milestone 9, floating point): `to_f` widens an
    // integer to `double`, `to_i` truncates a `double` toward zero to an integer
    // (the frontend then masks it to the target width with a `Convert`).
    "to_f",
    "to_i",
    // Faithful `printf` float formatting (SIR27 milestone 10): `fmt_float(value,
    // precision, kind)` renders a `double` exactly as C's `printf` `%f`/`%e`/`%g`
    // (and their uppercase forms) would.
    "fmt_float",
    "=",
    "==",
    "!=",
    "<",
    ">",
    "<=",
    ">=",
    "not",
    "and",
    "or",
    "cons",
    "car",
    "cdr",
    "null?",
    "pair?",
    "number?",
    "symbol?",
    "print",
    "puts",
    "global_get",
    "global_set",
    // SIR17 exceptions (`Feature::Exceptions`): `raise` (re-raise / raise a
    // message or exception) and `retry` (restart the `begin` body) render as
    // the native Ruby keywords.
    "raise",
    "retry",
    // OOP classes slice 1 (`Feature::Classes`): `__new__` constructs an instance
    // — the frontend's lowering of `Foo.new(args…)`.  Its first argument is the
    // class name (a `StrLit`), emitted verbatim as the `.new` receiver after
    // constant-path validation in the scan (so no source can inject); the rest
    // are constructor arguments.
    "__new__",
    // OOP classes slice 2 (instance methods): `__def_method__("Class", "m",
    // MakeClosure(fn))` registers a hoisted method on the class, and
    // `__method__(recv, "m", args…)` dispatches to it.  Both render through a
    // reserved `sir_um_` method-name PREFIX (`define_method`/`public_send`), so
    // dispatch is CLOSED — no reflection/eval built-in (none named `sir_um_*`)
    // is reachable (anti-RCE).  The other OOP builtins (`__super__`, `__self__`,
    // `__class_method__`, `__def_class_method__`, …) are still absent — a module
    // using them is rejected until their slice.
    "__def_method__",
    "__method__",
    // OOP classes slice 3 (instance variables): `__self__` — the frontend's
    // lowering of a bare `self` — renders as the native `self` keyword.  (Its
    // sibling `@ivar` read/write rides on `Scope::Instance`, not a builtin.)
    "__self__",
    // OOP classes slice 4 (inheritance): `__super__("m", "Class", args…)` — a
    // `super` call — dispatches the superclass's `sir_um_m` on `self`.  (The
    // subclass relation itself rides on `Stmt::ClassDef { superclass }`.)
    "__super__",
    // OOP classes slice 5 (class methods): `def self.m` registers via
    // `__def_class_method__("Class", "m", MakeClosure(fn))` (a singleton method
    // on the class), and `Class.m(args…)` dispatches via
    // `__class_method__("Class", "m", args…)`.  Both use the SAME reserved
    // `sir_um_` prefix as instance methods (a class's singleton method table is
    // separate from its instance methods, so the shared prefix cannot collide),
    // keeping dispatch CLOSED (anti-RCE).
    "__def_class_method__",
    "__class_method__",
    // OOP classes slice 7 (modules / mixins): `include M` / `extend M` render as
    // Ruby's native `Class.include(Module)` / `Class.extend(Module)`.  Both
    // operands are bare constant references (validated).  A module's own methods
    // are hoisted + registered via `__def_method__`, reusing the slice-2 machinery.
    "__include__",
    "__extend__",
];

/// Emit a complete self-contained Ruby source file for `m`.
pub fn emit_module(m: &Module) -> String {
    let mut out = String::new();
    emit_banner(&mut out, m);

    let display_ruby = m.metadata.source_language.as_deref() == Some("ruby");
    out.push_str(&RUNTIME.replace(
        "__SIR_DISPLAY_RUBY__",
        if display_ruby { "true" } else { "false" },
    ));
    out.push_str("\n\n");

    emit_globals_comment(&mut out, &m.globals);

    for f in &m.functions {
        emit_function(&mut out, f);
        out.push('\n');
    }

    if m.functions.iter().any(|f| f.name == "_init") {
        out.push_str("sir_user_init\n");
    }
    if m.functions.iter().any(|f| f.name == "main") {
        out.push_str("sir_user_main\n");
    }

    out
}

fn emit_banner(out: &mut String, m: &Module) {
    out.push_str("# Generated by semantic-ir-to-ruby (SIR25) — do not edit.\n");
    let _ = writeln!(out, "# module: {}", sanitize_comment(&m.name));
    if let Some(lang) = &m.metadata.source_language {
        let _ = writeln!(out, "# source language: {}", sanitize_comment(lang));
    }
    out.push('\n');
}

fn emit_globals_comment(out: &mut String, globals: &[Global]) {
    if globals.is_empty() {
        return;
    }
    out.push_str("# globals (held in $sir_globals, initialised in _init):");
    for g in globals {
        let _ = write!(out, " {}", sanitize_comment(&g.name));
    }
    out.push_str("\n\n");
}

/// A problem the pre-emit scan found — reported by `compile` as a clean
/// rejection.  Both live in ONE traversal ([`first_scan_issue`]) so the check
/// can never drift out of coverage with the emitter (a lesson from a rescue-type
/// injection: a hand-picked subset walk missed positions the emitter reaches).
pub enum ScanHit {
    /// A `BuiltinCall` whose name the v0 backend cannot lower.
    Builtin(String, semantic_ir::Span),
    /// A `rescue` clause exception type that is not a valid Ruby constant path
    /// (would inject source if emitted verbatim).
    RescueType(String, semantic_ir::Span),
    /// A constant name/path emitted VERBATIM as a Ruby constant that is not a
    /// valid constant path — so a metacharacter would inject source.  Covers
    /// every such position: a `Stmt::ClassDef` name, a `__new__` call's class
    /// name, a `Scope::Const` `VarRef` (`PI`, `Foo::Bar`), and a `Scope::Const`
    /// `Stmt::Assign` target.  Same injection guard as `RescueType`.
    ConstantName(String, semantic_ir::Span),
    /// A construct that is well-formed but beyond this slice's support — a class
    /// superclass (inheritance), a non-empty class body, or a namespaced
    /// (`Foo::Bar`) class/constant *definition* (which `const_set` cannot name).
    /// Carries a human-readable reason.  Deferred to a later slice; rejected
    /// cleanly rather than mis-emitted.
    Unsupported(String, semantic_ir::Span),
    /// A `Scope::Instance` name (`@v`) that is not a valid Ruby instance-variable
    /// name — it is emitted VERBATIM (a bare `@v` read / write), so a
    /// metacharacter would inject source.  Same injection guard as `ConstantName`,
    /// at the instance-variable positions.
    InstanceVarName(String, semantic_ir::Span),
    /// A `Scope::ClassVar` name (`@@x`) that is not a valid Ruby class-variable
    /// name (`@@<identifier>`) — injection guard at the class-variable positions.
    ClassVarName(String, semantic_ir::Span),
}

/// Scan the module — in a SINGLE traversal shared with the unsupported-builtin
/// check — for the first thing `compile` must reject: an unlowerable builtin, an
/// injectable `rescue`/constant name, an out-of-slice class shape, or (OOP slice
/// 2) an instance-method dispatch (`__method__`) to a method the module never
/// defines.  Returns it (with a span) or `None`.
///
/// The traversal is a [`Scan`] carrying the set of method names the module
/// registers via `__def_method__` — the CLOSED set `__method__` may dispatch to.
/// A first collection pass gathers that set (a dispatch can textually precede
/// its registration), then the single scan validates against it.
pub fn first_scan_issue(m: &Module) -> Option<ScanHit> {
    let registered = collect_registered_methods(m);
    let scan = Scan {
        registered_methods: registered.instance,
        registered_class_methods: registered.class_methods,
    };
    for f in &m.functions {
        // A SIR19 parameter default is an expression evaluated at call time, so
        // a builtin nested in it (`def g(x = foo())`) must be pre-checked too —
        // otherwise it would slip past the body scan and reach the emitter's
        // `unreachable!`.  Scan each default before the body.
        for p in &f.params {
            if let Some(default) = &p.default {
                if let Some(hit) = scan.expr(default) {
                    return Some(hit);
                }
            }
        }
        if let Some(hit) = scan.block(&f.body) {
            return Some(hit);
        }
    }
    None
}

/// Walk the whole module and collect the method names registered via
/// `__def_method__("Class", "method", closure)` — the closed set that
/// `__method__` dispatch may target (OOP slice 2).  A `__method__` call to any
/// OTHER name is a built-in-method call (the separate Collections batch),
/// rejected until then.  (The `sir_um_` dispatch prefix is the SECURITY
/// guarantee against reflection-RCE; this allowlist is for clean COMPILE-TIME
/// rejection of not-yet-supported built-in dispatch.)
fn collect_registered_methods(m: &Module) -> Registered {
    fn from_expr(e: &Expr, out: &mut Registered) {
        match e {
            Expr::BuiltinCall { name, args, .. } => {
                // A registration's method name is `args[1]`.  Route an instance
                // method (`__def_method__`) and a class method
                // (`__def_class_method__`) into their SEPARATE allowlists — an
                // instance dispatch and a class dispatch are distinct namespaces.
                if name == "__def_method__" {
                    if let Some(Expr::StrLit { value, .. }) = args.get(1) {
                        out.instance.insert(value.clone());
                    }
                } else if name == "__def_class_method__" {
                    if let Some(Expr::StrLit { value, .. }) = args.get(1) {
                        out.class_methods.insert(value.clone());
                    }
                }
                for a in args {
                    from_expr(a, out);
                }
            }
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
            Expr::DirectCall { args, .. } => args.iter().for_each(|a| from_expr(a, out)),
            Expr::IndirectCall { target, args, .. } => {
                from_expr(target, out);
                args.iter().for_each(|a| from_expr(a, out));
            }
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
            Expr::KeywordArg { value, .. } => from_expr(value, out),
            _ => {}
        }
    }
    fn from_stmt(s: &Stmt, out: &mut Registered) {
        match s {
            Stmt::LetBinding { value, .. }
            | Stmt::LetStarBinding { value, .. }
            | Stmt::ExprStmt { expr: value, .. }
            | Stmt::Assign { value, .. } => from_expr(value, out),
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
            Stmt::While { cond, body, .. } => {
                from_expr(cond, out);
                from_block(body, out);
            }
            Stmt::ForEach { iter, body, .. } => {
                from_expr(iter, out);
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
    fn from_block(b: &Block, out: &mut Registered) {
        b.stmts.iter().for_each(|s| from_stmt(s, out));
        from_expr(&b.value, out);
    }
    let mut out = Registered::default();
    for f in &m.functions {
        for p in &f.params {
            if let Some(d) = &p.default {
                from_expr(d, &mut out);
            }
        }
        from_block(&f.body, &mut out);
    }
    out
}

/// The module-wide sets of method names registered via `__def_method__` (instance
/// methods) and `__def_class_method__` (class methods) — the two CLOSED sets that
/// `__method__` / `__class_method__` dispatch may target.  A dispatch to any other
/// name is a built-in-method call (the Collections batch), rejected until then.
#[derive(Default)]
struct Registered {
    instance: HashSet<String>,
    class_methods: HashSet<String>,
}

/// The pre-emit scan, carrying the module's registered-method allowlists so the
/// single co-total traversal can validate instance- and class-method dispatch.
struct Scan {
    registered_methods: HashSet<String>,
    registered_class_methods: HashSet<String>,
}

impl Scan {
    fn block(&self, b: &Block) -> Option<ScanHit> {
        for s in &b.stmts {
            if let Some(hit) = self.stmt(s) {
                return Some(hit);
            }
        }
        self.expr(&b.value)
    }

    fn stmt(&self, s: &Stmt) -> Option<ScanHit> {
        match s {
        Stmt::LetBinding { value, .. }
        | Stmt::LetStarBinding { value, .. }
        | Stmt::ExprStmt { expr: value, .. } => self.expr(value),
        // An `Stmt::Assign` to a `Scope::Const` target (`PI = 3`) emits the name
        // VERBATIM as a Ruby constant, so validate it as a constant path (the
        // same injection guard as a const reference); then scan the value. A
        // non-const assignment routes the name through `sanitize_ident` (safe).
        Stmt::Assign {
            name,
            scope,
            value,
            span,
        } => {
            if matches!(scope, Scope::Const) {
                if !is_valid_constant_path(name) {
                    Some(ScanHit::ConstantName(name.clone(), span.clone()))
                } else if name.contains("::") {
                    // `const_set` cannot name a `Foo::Bar` path.  Deferred.
                    Some(ScanHit::Unsupported(
                        "a namespaced constant assignment (`Foo::Bar = …`)".to_string(),
                        span.clone(),
                    ))
                } else {
                    self.expr(value)
                }
            } else if matches!(scope, Scope::Instance) {
                // A `Scope::Instance` target (`@v = …`) emits the name VERBATIM,
                // so validate it as `@<identifier>` at this position (the same
                // injection guard as an ivar reference); then scan the value.
                if !is_valid_ivar_name(name) {
                    Some(ScanHit::InstanceVarName(name.clone(), span.clone()))
                } else {
                    self.expr(value)
                }
            } else if matches!(scope, Scope::ClassVar) {
                // A `Scope::ClassVar` target (`@@x = …`) emits the name into a
                // `class_variable_set` symbol, so validate it as `@@<identifier>`
                // at this position; then scan the value.
                if !is_valid_classvar_name(name) {
                    Some(ScanHit::ClassVarName(name.clone(), span.clone()))
                } else {
                    self.expr(value)
                }
            } else {
                self.expr(value)
            }
        }
        // A sequence write / iteration has sub-expressions that may themselves
        // hide an unsupported builtin — scan them (and a ForEach body) so the
        // graceful pre-check catches it rather than the emitter.
        Stmt::SeqSet {
            seq, index, value, ..
        } => self.expr(seq)
            .or_else(|| self.expr(index))
            .or_else(|| self.expr(value)),
        Stmt::MapSet {
            map, key, value, ..
        } => self.expr(map)
            .or_else(|| self.expr(key))
            .or_else(|| self.expr(value)),
        // Compound-statement bodies must be scanned too, or an unsupported
        // builtin hidden in a loop body survives the pre-check and reaches the
        // emitter's `unreachable!`. (`While` was a pre-existing scan hole.)
        Stmt::While { cond, body, .. } => self.expr(cond).or_else(|| self.block(body)),
        Stmt::ForEach { iter, body, .. } => self.expr(iter).or_else(|| self.block(body)),
        Stmt::ForRange {
            start,
            stop,
            step,
            body,
            ..
        } => self.expr(start)
            .or_else(|| self.expr(stop))
            .or_else(|| self.expr(step))
            .or_else(|| self.block(body)),
        // A `begin … rescue … ensure … end`.  Check each rescue clause's
        // exception-type NAMES here (this arm is reached by the SAME complete
        // traversal as the builtin scan, so EVERY `TryCatch` the emitter reaches
        // is validated — no separate, drift-prone walk).  Then scan the guarded
        // body, every rescue clause body, and the ensure body for builtins.
        Stmt::TryCatch {
            body,
            rescues,
            ensure_body,
            span,
        } => rescues
            .iter()
            .flat_map(|rc| rc.exception_types.iter())
            .find(|t| !is_valid_constant_path(t))
            .map(|t| ScanHit::RescueType(t.clone(), span.clone()))
            .or_else(|| body.iter().find_map(|s| self.stmt(s)))
            .or_else(|| rescues.iter().find_map(|rc| rc.body.iter().find_map(|s| self.stmt(s))))
            .or_else(|| {
                ensure_body
                    .as_ref()
                    .and_then(|e| e.iter().find_map(|s| self.stmt(s)))
            }),
        // A `Stmt::ClassDef` (`Feature::Classes`).  Slice 1 supports ONLY an
        // empty-bodied base class → native `class Name\nend`.  At exactly the
        // position the emitter reaches, validate that:
        //   - the class NAME is a valid Ruby constant path — it is emitted
        //     verbatim as the `class` name, so a metacharacter would inject;
        //   - there is NO superclass (inheritance is a later slice);
        //   - the body is EMPTY (class-level code / constants are a later slice).
        // Anything else is rejected cleanly here, never reaching the emitter's
        // `unreachable!`.  (This is why the emitter's `ClassDef` arm may ignore
        // `superclass`/`body`: the scan guarantees they are `None`/empty.)
        Stmt::ClassDef {
            name,
            superclass,
            body,
            span,
        } => {
            if !is_valid_constant_path(name) {
                Some(ScanHit::ConstantName(name.clone(), span.clone()))
            } else if name.contains("::") {
                // `const_set` names a constant in ONE namespace by a bare symbol;
                // it cannot define a `Foo::Bar` path.  Deferred.
                Some(ScanHit::Unsupported(
                    "a namespaced class name (`class Foo::Bar`)".to_string(),
                    span.clone(),
                ))
            } else if superclass
                .as_deref()
                .is_some_and(|s| !is_valid_constant_path(s))
            {
                // OOP slice 4: a superclass (`class Dog < Animal`) is emitted as
                // the `Class.new(<superclass>)` argument — a bare constant
                // REFERENCE — so validate it as a constant path (a `::` path IS
                // allowed here: it references, not defines).  A crafted name is
                // rejected so it cannot inject source.
                Some(ScanHit::ConstantName(
                    superclass.clone().unwrap_or_default(),
                    span.clone(),
                ))
            } else {
                // OOP slice 6: a class BODY may contain `@@x = <init>` class-
                // variable initializers — the emitter renders each as
                // `<Class>.class_variable_set(:"@@x", …)`.  Admit ONLY those (with a
                // validated `@@`-name and a scanned value); any OTHER body content
                // (class-level code / constants) stays deferred, rejected cleanly.
                body.iter().find_map(|st| match st {
                    Stmt::Assign {
                        name: cv,
                        scope: Scope::ClassVar,
                        value,
                        span: aspan,
                    } => {
                        if !is_valid_classvar_name(cv) {
                            Some(ScanHit::ClassVarName(cv.clone(), aspan.clone()))
                        } else {
                            self.expr(value)
                        }
                    }
                    _ => Some(ScanHit::Unsupported(
                        "a class body with content other than `@@class` variable \
                         initializers (class-level code / constants)"
                            .to_string(),
                        span.clone(),
                    )),
                })
            }
        }
        // A `Stmt::SingletonClassDef` (`class << self`) ALSO observes
        // `Feature::Classes` in the validator (a singleton class is a
        // class-opening construct, not its own feature) — so accepting `Classes`
        // obligates handling it too, or a hand-built module carrying it would
        // pass validation + the capability check and reach the emitter's
        // `unreachable!` (a DoS).  It is deferred to a later OOP slice; reject it
        // cleanly here.
        Stmt::SingletonClassDef { span, .. } => Some(ScanHit::Unsupported(
            "a singleton class (`class << self`)".to_string(),
            span.clone(),
        )),
        // A `Stmt::ModuleDef` (`module M; end`, `Feature::Modules`, OOP slice 7) —
        // the sole observer of `Feature::Modules`, so accepting it obligates
        // handling `ModuleDef` here (else it would reach the emitter's
        // `unreachable!`).  Emitted as `Object.const_set(:M, Module.new)`, so
        // validate the name as a single-segment constant (like a `ClassDef`); a
        // non-empty module body (class-level code) is deferred — a method-only
        // module has an EMPTY body (its methods are hoisted + registered via
        // `__def_method__`).
        Stmt::ModuleDef { name, body, span } => {
            if !is_valid_constant_path(name) {
                Some(ScanHit::ConstantName(name.clone(), span.clone()))
            } else if name.contains("::") {
                Some(ScanHit::Unsupported(
                    "a namespaced module name (`module Foo::Bar`)".to_string(),
                    span.clone(),
                ))
            } else if !body.is_empty() {
                Some(ScanHit::Unsupported(
                    "a non-empty module body (class-level code)".to_string(),
                    span.clone(),
                ))
            } else {
                None
            }
        }
        _ => None,
    }
    }
}

/// Whether `name` is a syntactically valid Ruby constant path (`Foo`,
/// `Foo::Bar`) — a non-empty `::`-separated list of identifier segments, each
/// starting with a letter or `_` and continuing with `[A-Za-z0-9_]`.  A `rescue`
/// clause's exception type is emitted verbatim, so this gates out any name
/// carrying a metacharacter (space, `;`, `(`, quote, newline, …) that could
/// inject source.
fn is_valid_constant_path(name: &str) -> bool {
    !name.is_empty()
        && name.split("::").all(|seg| {
            let mut chars = seg.chars();
            matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
                && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        })
}

/// Whether `name` is a syntactically valid Ruby instance-variable name — a `@`
/// followed by an identifier (`@v`, `@_x1`).  The frontend puts the `@` in the
/// node's `name`, and the emitter renders `@v` VERBATIM (a bare local/ivar write
/// or read), so this gates out any name carrying a metacharacter that could
/// inject source.  (A `@@class` variable is `Scope::ClassVar`, a later slice.)
fn is_valid_ivar_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next() == Some('@')
        && matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Whether `name` is a syntactically valid Ruby class-variable name — `@@`
/// followed by an identifier (`@@count`).  The frontend puts the `@@` in the
/// node's `name`; the emitter renders it through a quoted symbol in
/// `class_variable_get/set`, but a `class_variable_*` still requires a
/// `@@`-prefixed name and this gates any metacharacter (defence in depth).
fn is_valid_classvar_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next() == Some('@')
        && chars.next() == Some('@')
        && matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

impl Scan {
    fn expr(&self, e: &Expr) -> Option<ScanHit> {
        match e {
        Expr::BuiltinCall {
            name, args, span, ..
        } => {
            if !SUPPORTED_BUILTINS.contains(&name.as_str()) {
                return Some(ScanHit::Builtin(name.clone(), span.clone()));
            }
            // `__new__`'s first argument is the class name, emitted VERBATIM as
            // the `.new` receiver (`Foo.new(…)`).  Validate it here — at exactly
            // that emitter position — as a `StrLit` holding a valid Ruby constant
            // path, so a crafted name cannot inject source.  A malformed shape
            // (missing / non-string class name) is reported as an unlowerable
            // builtin rather than silently mis-emitted.
            if name == "__new__" {
                match args.first() {
                    Some(Expr::StrLit { value, span }) if !is_valid_constant_path(value) => {
                        return Some(ScanHit::ConstantName(value.clone(), span.clone()));
                    }
                    Some(Expr::StrLit { .. }) => {}
                    _ => return Some(ScanHit::Builtin(name.clone(), span.clone())),
                }
            }
            // OOP slice 2 — instance-method definition/dispatch.
            // `__def_method__("Class", "method", closure)` emits the CLASS name
            // verbatim (a `const_set` receiver), so validate it as a constant
            // path; the method name and closure are rendered safely (a quoted
            // symbol / a scanned closure).
            if name == "__def_method__" {
                match args.first() {
                    Some(Expr::StrLit { value, span }) if !is_valid_constant_path(value) => {
                        return Some(ScanHit::ConstantName(value.clone(), span.clone()));
                    }
                    Some(Expr::StrLit { .. }) => {}
                    _ => return Some(ScanHit::Builtin(name.clone(), span.clone())),
                }
                // Require the method-name `StrLit` (args[1]) AND a `MakeClosure`
                // (args[2], rendered as `a[2]` and installed with `&(<closure>)`).
                // A malformed shape — missing the closure, or a non-closure that
                // would emit `&(<expr>)` and fail at Ruby runtime — is reported as
                // an unlowerable builtin here, so the emitter never indexes past
                // the end nor emits a value that cannot become a method body.
                if !matches!(args.get(1), Some(Expr::StrLit { .. }))
                    || !matches!(args.get(2), Some(Expr::MakeClosure { .. }))
                {
                    return Some(ScanHit::Builtin(name.clone(), span.clone()));
                }
            }
            // `__method__(recv, "method", args…)` dispatches to a method by name.
            // The name is rendered as a `sir_um_`-PREFIXED quoted symbol via
            // `public_send`, so no source can inject AND no reflection/eval
            // built-in (none of which is named `sir_um_*`) is reachable — the
            // anti-RCE guarantee.  Additionally reject, cleanly, a dispatch to a
            // method the module never registers via `__def_method__`: that is a
            // BUILT-IN method call (`.upcase`, …), deferred to the Collections
            // batch, so it must not compile-then-`NoMethodError` at runtime.
            if name == "__method__" {
                match args.get(1) {
                    Some(Expr::StrLit { value, span }) => {
                        if !self.registered_methods.contains(value) {
                            return Some(ScanHit::Unsupported(
                                format!(
                                    "a call to the built-in method `{value}` (only \
                                     user-defined methods dispatch this slice; \
                                     built-in methods are the Collections batch)"
                                ),
                                span.clone(),
                            ));
                        }
                    }
                    _ => return Some(ScanHit::Builtin(name.clone(), span.clone())),
                }
            }
            // `__super__("m", "Class", args…)` (OOP slice 4) dispatches the
            // superclass's method.  args[0] is the method NAME (rendered as a
            // `sir_um_`-prefixed quoted symbol — safe), args[1] is the DEFINING
            // class emitted VERBATIM as a bare constant (`<Class>.superclass…`),
            // so validate it as a constant path here (injection guard); the
            // forwarded args (args[2..]) are scanned below.  A malformed shape is
            // reported as an unlowerable builtin.
            if name == "__super__" {
                match (args.first(), args.get(1)) {
                    (Some(Expr::StrLit { .. }), Some(Expr::StrLit { value, span })) => {
                        if !is_valid_constant_path(value) {
                            return Some(ScanHit::ConstantName(value.clone(), span.clone()));
                        }
                    }
                    _ => return Some(ScanHit::Builtin(name.clone(), span.clone())),
                }
            }
            // `__def_class_method__("Class", "m", closure)` (OOP slice 5) — like
            // `__def_method__` but the class NAME (args[0], emitted as the
            // `define_singleton_method` receiver) is a bare constant, so validate
            // it as a constant path; require args[1] StrLit + args[2] MakeClosure.
            if name == "__def_class_method__" {
                match args.first() {
                    Some(Expr::StrLit { value, span }) if !is_valid_constant_path(value) => {
                        return Some(ScanHit::ConstantName(value.clone(), span.clone()));
                    }
                    Some(Expr::StrLit { .. }) => {}
                    _ => return Some(ScanHit::Builtin(name.clone(), span.clone())),
                }
                if !matches!(args.get(1), Some(Expr::StrLit { .. }))
                    || !matches!(args.get(2), Some(Expr::MakeClosure { .. }))
                {
                    return Some(ScanHit::Builtin(name.clone(), span.clone()));
                }
            }
            // `__class_method__("Class", "m", args…)` (OOP slice 5) dispatches a
            // class method: `(<Class>).public_send(:sir_um_m, …)`.  The class name
            // (args[0]) is emitted verbatim as the bare-constant receiver, so
            // validate it as a constant path; the method name (args[1]) rides the
            // same `sir_um_` prefix (anti-RCE) but must be a method the module
            // REGISTERS via `__def_class_method__` — else it is a built-in class
            // method (`Foo.name`, …), the Collections batch, rejected cleanly.
            if name == "__class_method__" {
                match (args.first(), args.get(1)) {
                    (Some(Expr::StrLit { value: cls, span: cspan }), Some(Expr::StrLit { value: m, span: mspan })) => {
                        if !is_valid_constant_path(cls) {
                            return Some(ScanHit::ConstantName(cls.clone(), cspan.clone()));
                        }
                        if !self.registered_class_methods.contains(m) {
                            return Some(ScanHit::Unsupported(
                                format!(
                                    "a call to the built-in class method `{m}` (only \
                                     user-defined class methods dispatch this slice; \
                                     built-in methods are the Collections batch)"
                                ),
                                mspan.clone(),
                            ));
                        }
                    }
                    _ => return Some(ScanHit::Builtin(name.clone(), span.clone())),
                }
            }
            // `__include__("Class", "Module")` / `__extend__(…)` (OOP slice 7) mix
            // a module into a class: `(<Class>).include(<Module>)`.  BOTH operands
            // are emitted verbatim as bare constant references, so validate each as
            // a constant path here (injection guard).
            if name == "__include__" || name == "__extend__" {
                match (args.first(), args.get(1)) {
                    (
                        Some(Expr::StrLit { value: cls, span: cspan }),
                        Some(Expr::StrLit { value: m, span: mspan }),
                    ) => {
                        if !is_valid_constant_path(cls) {
                            return Some(ScanHit::ConstantName(cls.clone(), cspan.clone()));
                        }
                        if !is_valid_constant_path(m) {
                            return Some(ScanHit::ConstantName(m.clone(), mspan.clone()));
                        }
                    }
                    _ => return Some(ScanHit::Builtin(name.clone(), span.clone())),
                }
            }
            args.iter().find_map(|a| self.expr(a))
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => self
            .expr(cond)
            .or_else(|| self.block(then_branch))
            .or_else(|| self.block(else_branch)),
        Expr::Block(b) => self.block(b),
        Expr::DirectCall { args, .. } => args.iter().find_map(|a| self.expr(a)),
        // An `IndirectCall` renders its `target` too (`sir_apply(<target>, …)`),
        // so a deferred builtin hidden in the callee position — not just the
        // args — must be pre-checked, or it would reach the emitter's
        // `unreachable!`.  (Found by security review while wiring the param
        // default scan, which routes through here.)
        Expr::IndirectCall { target, args, .. } => {
            self.expr(target).or_else(|| args.iter().find_map(|a| self.expr(a)))
        }
        Expr::MakeClosure { captures, .. } => captures.iter().find_map(|c| self.expr(&c.value)),
        Expr::Convert { value, .. } => self.expr(value),
        // A sequence literal's items are themselves expressions — scan each so
        // an unsupported builtin nested in `[foo(), bar()]` is caught by the
        // graceful pre-check rather than reaching the emitter.
        Expr::SeqLit { items, .. } => items.iter().find_map(|i| self.expr(i)),
        Expr::SeqIndex { seq, index, .. } => self.expr(seq).or_else(|| self.expr(index)),
        Expr::SeqLen { seq, .. } => self.expr(seq),
        Expr::MapLit { entries, .. } => entries
            .iter()
            .find_map(|e| self.expr(&e.key).or_else(|| self.expr(&e.value))),
        Expr::MapGet { map, key, .. } => self.expr(map).or_else(|| self.expr(key)),
        Expr::LogicalAnd { lhs, rhs, .. } | Expr::LogicalOr { lhs, rhs, .. } => {
            self.expr(lhs).or_else(|| self.expr(rhs))
        }
        // A keyword argument carries its value as a sub-expression — scan it so
        // an unsupported builtin in `f(x: foo())` is reported cleanly.
        Expr::KeywordArg { value, .. } => self.expr(value),
        // A `Scope::Const` reference (`Feature::Constants`) is emitted VERBATIM
        // as a Ruby constant (`PI`, `Foo::Bar`) by `emit_var_ref`, so validate it
        // as a constant path here — at that emitter position — to bar injection.
        // (Non-const scopes go through `sanitize_ident`, which is already safe.)
        Expr::VarRef {
            name,
            scope: Scope::Const,
            span,
        } if !is_valid_constant_path(name) => Some(ScanHit::ConstantName(name.clone(), span.clone())),
        // A `Scope::Instance` reference (`@v`, `Feature::InstanceVars`) is emitted
        // VERBATIM by `emit_var_ref`, so validate it as `@<identifier>` here — at
        // that emitter position — to bar injection.
        Expr::VarRef {
            name,
            scope: Scope::Instance,
            span,
        } if !is_valid_ivar_name(name) => Some(ScanHit::InstanceVarName(name.clone(), span.clone())),
        // A `Scope::ClassVar` reference (`@@x`, `Feature::ClassVars`) emits its name
        // into a `class_variable_get` symbol, so validate it as `@@<identifier>`.
        Expr::VarRef {
            name,
            scope: Scope::ClassVar,
            span,
        } if !is_valid_classvar_name(name) => {
            Some(ScanHit::ClassVarName(name.clone(), span.clone()))
        }
        _ => None,
    }
    }
}

// ---------------------------------------------------------------------------
// Functions
// ---------------------------------------------------------------------------

/// The Ruby name for a SIR function.  `main`/`_init` are renamed so the entry
/// points are explicit and uniform with the other backends.
fn function_emit_name(name: &str) -> String {
    match name {
        "main" => "sir_user_main".to_string(),
        "_init" => "sir_user_init".to_string(),
        other => sanitize_ident(other),
    }
}

fn emit_function(out: &mut String, f: &Function) {
    let _ = writeln!(out, "# {}", sanitize_comment(&f.span.to_string()));
    let _ = write!(out, "def {}(", function_emit_name(&f.name));
    let mut first = true;
    for c in &f.captures {
        if !first {
            out.push_str(", ");
        }
        first = false;
        out.push_str(&sanitize_ident(&c.name));
    }
    for p in &f.params {
        if !first {
            out.push_str(", ");
        }
        first = false;
        let name = sanitize_ident(&p.name);
        // Every `ParamKind` has a native Ruby spelling — the canonical order the
        // validator enforces (`Required* Rest? Keyword* KwRest?`) is exactly
        // Ruby's, so emitting each in place yields a valid signature.
        match p.kind {
            // `x` / `x = <default>` — a positional parameter, optionally with a
            // SIR19 default (`Feature::DefaultParams`).  Ruby evaluates the
            // default at call time when the argument is omitted, left to right,
            // so it may reference an earlier parameter (`def f(a, b = a)`).
            ParamKind::Required => {
                out.push_str(&name);
                if let Some(default) = &p.default {
                    let _ = write!(out, " = {}", emit_expr(default));
                }
            }
            // `x:` / `x: <default>` — a native keyword parameter
            // (`Feature::KeywordParams`): required when it has no default, an
            // optional keyword when it does (a keyword default rides on
            // `KeywordParams`, not `DefaultParams`).  Matched by NAME at the
            // call site, which Ruby handles natively.
            ParamKind::Keyword => {
                let _ = write!(out, "{name}:");
                if let Some(default) = &p.default {
                    let _ = write!(out, " {}", emit_expr(default));
                }
            }
            // `*rest` — a native rest parameter collecting trailing positionals
            // into an `Array`.  (Rest/KwRest carry no default.)
            ParamKind::Rest => {
                let _ = write!(out, "*{name}");
            }
            // `**opts` — a native keyword-rest parameter collecting unmatched
            // keywords into a `Hash`.
            ParamKind::KwRest => {
                let _ = write!(out, "**{name}");
            }
        }
    }
    out.push_str(")\n");
    // Body: statements, then the block's value expression (Ruby returns it).
    for s in &f.body.stmts {
        let _ = writeln!(out, "  {}", emit_stmt(s));
    }
    let _ = writeln!(out, "  {}", emit_expr(&f.body.value));
    out.push_str("end\n");
}

// ---------------------------------------------------------------------------
// Statements
// ---------------------------------------------------------------------------

fn emit_stmt(s: &Stmt) -> String {
    match s {
        Stmt::LetBinding { name, value, .. } | Stmt::LetStarBinding { name, value, .. } => {
            format!("{} = {}", sanitize_ident(name), emit_expr(value))
        }
        Stmt::ExprStmt { expr, .. } => emit_expr(expr),
        // SIR16 re-binding / SIR-constant definition.
        // A `Scope::Const` target (`PI = 3`, `Feature::Constants`) CANNOT be a
        // bare `PI = 3`: the frontend wraps top-level code in `main`, and a
        // constant assignment inside a method body is a Ruby error ("dynamic
        // constant assignment").  So define it REFLECTIVELY with `const_set`
        // (legal anywhere, executes in place), exactly as a class does.  The
        // scan guarantees a single-segment constant here (valid path, no `::`),
        // so `:name` is a safe symbol literal.  Every other scope is a mutable
        // local — a plain `=` after `sanitize_ident`.
        Stmt::Assign {
            name, scope, value, ..
        } => {
            if matches!(scope, Scope::Const) {
                format!("Object.const_set(:{}, {})", name, emit_expr(value))
            } else if matches!(scope, Scope::Instance) {
                // A `Scope::Instance` target (`@v = …`, `Feature::InstanceVars`,
                // OOP slice 3): the name already INCLUDES the `@`, emitted VERBATIM
                // (the scan validated it as `@<identifier>`, so no injection).
                // Inside a `define_method`-installed method `self` is the receiver,
                // so this writes the instance's own variable.
                format!("{} = {}", name, emit_expr(value))
            } else if matches!(scope, Scope::ClassVar) {
                // A `Scope::ClassVar` target (`@@x = …`, `Feature::ClassVars`, OOP
                // slice 6) IN A METHOD BODY: a bare `@@x = …` is a toplevel error in
                // a hoisted function, so write it via `class_variable_set` on the
                // owner (`sir_cvar_owner(self)`).  (The class-BODY initializer takes
                // a different path — see the `ClassDef` emit, where the owner is the
                // class by name.)  The `@@`-name is a validated, safely-quoted symbol.
                format!(
                    "sir_cvar_owner(self).class_variable_set({}, {})",
                    emit_symbol(name),
                    emit_expr(value)
                )
            } else {
                format!("{} = {}", sanitize_ident(name), emit_expr(value))
            }
        }
        // SIR16 loop: Ruby's `while` re-tests the (already-bool) condition each
        // iteration.  The body's statements run for effect; its value is nil and
        // is discarded (a loop yields nothing).
        Stmt::While { cond, body, .. } => {
            let mut s = format!("while sir_truthy({})\n", emit_expr(cond));
            for st in &body.stmts {
                s.push_str(&emit_stmt(st));
                s.push('\n');
            }
            s.push_str("end");
            s
        }
        // `a[i] = v` (indexed write). The SIR reference (`_sir_seq_set`) treats
        // ONLY `0 <= i < len` as valid and RAISES on a negative or out-of-range
        // index — unlike Ruby's native `[]=`, which silently extends the array
        // with nils or counts negatives from the end. The `sir_seq_set` runtime
        // helper enforces the reference rule and returns the assigned value.
        Stmt::SeqSet {
            seq, index, value, ..
        } => format!(
            "sir_seq_set({}, {}, {})",
            emit_expr(seq),
            emit_expr(index),
            emit_expr(value)
        ),
        // `h[k] = v` — native `Hash#[]=`, which inserts or updates and mutates
        // the shared Hash (matching `_sir_map_set`). Unlike a sequence, a map
        // has no bounds, so no guard helper is needed; the assignment evaluates
        // to `v`, but here it stands in statement position.
        Stmt::MapSet {
            map, key, value, ..
        } => format!(
            "({})[{}] = {}",
            emit_expr(map),
            emit_expr(key),
            emit_expr(value)
        ),
        // `iter.each { |var| … }` — a BLOCK, so `var` and any body-local are
        // block-scoped, matching the SIR validator (which `env.add_local`s the
        // var then `env.rewind`s the whole loop body — body-scoped, NOT
        // surrounding-scope) and the Go reference (`for _, x := range …`, whose
        // `:=` var is block-local). A leaking `for … in` would instead clobber
        // an enclosing local that shares the var's name. Safe as a block
        // because SIR has no loop break/next/return that a block would reroute.
        Stmt::ForEach {
            var, iter, body, ..
        } => {
            let mut s = format!("({}).each do |{}|\n", emit_expr(iter), sanitize_ident(var));
            for st in &body.stmts {
                s.push_str(&emit_stmt(st));
                s.push('\n');
            }
            s.push_str("end");
            s
        }
        // `for var in start...stop step step`. Gated by `Feature::Loops` alone,
        // so it is reachable whenever loops are accepted. Desugared to a
        // `while` (rather than a Ruby Range, whose `step`/exclusivity is fiddly
        // for a negative step) that mirrors the Go/Rust backends EXACTLY:
        //   - `start`/`stop`/`step` are evaluated ONCE (they may have side
        //     effects), into `sir_`-prefixed temporaries unique per loop
        //     (nesting-safe, and collision-proof — `sanitize_ident` renames any
        //     user variable out of the `sir_` namespace);
        //   - the stop is EXCLUSIVE and the direction follows the step's sign
        //     (`step >= 0 ? i < stop : i > stop`), so a descending loop works;
        //   - `var` and any body-local are BLOCK-scoped: the body runs inside a
        //     hoisted `->(var) { … }` lambda, so — like `ForEach`'s block and
        //     the Go reference's `:=` counter — they never clobber an enclosing
        //     same-named local (the validator rewinds the loop body).
        Stmt::ForRange {
            var,
            start,
            stop,
            step,
            body,
            ..
        } => {
            let id = LOOP_TEMP_ID.fetch_add(1, Ordering::Relaxed);
            let v = sanitize_ident(var);
            let mut s = String::new();
            // The body runs inside a lambda taking `var`, so — like `ForEach`'s
            // block and the Go reference's `:=` counter — `var` and any
            // body-local are block-scoped (the validator rewinds them), never
            // clobbering an enclosing same-named local. Hoisted once (not
            // re-created per iteration). Safe because SIR loop bodies have no
            // break/next/return a lambda would reroute.
            let _ = writeln!(s, "sir_for_body_{id} = ->({v}) {{");
            for st in &body.stmts {
                s.push_str(&emit_stmt(st));
                s.push('\n');
            }
            s.push_str("}\n");
            // `start`/`stop`/`step` are evaluated ONCE, into `sir_`-namespaced
            // temporaries (guarded from user names by `sanitize_ident`) unique
            // per loop for nesting. Direction-aware EXCLUSIVE stop mirrors the
            // Go/Rust backends: count up while `step >= 0` (`i < stop`), down
            // otherwise (`i > stop`).
            let _ = writeln!(s, "sir_for_i_{id} = ({})", emit_expr(start));
            let _ = writeln!(s, "sir_for_stop_{id} = ({})", emit_expr(stop));
            let _ = writeln!(s, "sir_for_step_{id} = ({})", emit_expr(step));
            let _ = writeln!(
                s,
                "while (sir_for_step_{id} >= 0 ? sir_for_i_{id} < sir_for_stop_{id} : \
                 sir_for_i_{id} > sir_for_stop_{id})"
            );
            let _ = writeln!(s, "sir_for_body_{id}.call(sir_for_i_{id})");
            let _ = writeln!(s, "sir_for_i_{id} += sir_for_step_{id}");
            s.push_str("end");
            s
        }
        // SIR17 `begin … rescue … ensure … end`.  Ruby handles exceptions
        // natively, so each part renders directly.  A `rescue` clause lists its
        // exception classes by name (validated as constant paths before emit, so
        // no source can inject), optionally binds the caught exception to a
        // `sanitize_ident`-safe local, and runs its body; an empty class list is
        // a bare catch-all `rescue`.  `ensure`, when present, runs afterwards.
        Stmt::TryCatch {
            body,
            rescues,
            ensure_body,
            ..
        } => {
            let mut s = String::from("begin\n");
            for st in body {
                s.push_str(&emit_stmt(st));
                s.push('\n');
            }
            for rc in rescues {
                s.push_str("rescue");
                if !rc.exception_types.is_empty() {
                    let _ = write!(s, " {}", rc.exception_types.join(", "));
                }
                if let Some(bind) = &rc.binding {
                    let _ = write!(s, " => {}", sanitize_ident(bind));
                }
                s.push('\n');
                for st in &rc.body {
                    s.push_str(&emit_stmt(st));
                    s.push('\n');
                }
            }
            if let Some(ens) = ensure_body {
                s.push_str("ensure\n");
                for st in ens {
                    s.push_str(&emit_stmt(st));
                    s.push('\n');
                }
            }
            s.push_str("end");
            s
        }
        // OOP classes slice 1 (`Feature::Classes`): an empty base-class
        // declaration.  It CANNOT be a native `class Foo; end` block: the
        // frontend wraps a program's top-level code in `main`, and Ruby forbids
        // BOTH a `class` definition and a constant assignment inside a method
        // body.  So define the class REFLECTIVELY — `Object.const_set(:Foo,
        // Class.new)` — which is legal anywhere, names the class (`Foo.name ==
        // "Foo"`, so `Foo.new` and `x.is_a?(Foo)` work), and executes in place
        // (no fragile hoisting / reordering).  The scan guarantees, at this
        // position, that `name` is a single-segment constant (valid path, no
        // `::`), `superclass` is `None`, and `body` is empty — so `:name` is a
        // safe symbol literal and `Class.new` takes no base / body.  (This
        // dynamic construction also composes with the next slice's
        // `define_method` for the frontend's hoisted, separately-registered
        // methods.)  OOP slice 4: a superclass (`class Dog < Animal`) becomes
        // `Class.new(Animal)` — the superclass is a bare constant REFERENCE (the
        // scan validated it as a constant path), so the subclass inherits its
        // ancestry natively (`Dog.new.is_a?(Animal)`, and `super` resolves up it).
        Stmt::ClassDef {
            name,
            superclass,
            body,
            ..
        } => {
            let mut s = match superclass {
                Some(sup) => format!("Object.const_set(:{name}, Class.new({sup}))"),
                None => format!("Object.const_set(:{name}, Class.new)"),
            };
            // OOP slice 6: a class-BODY `@@x = <init>` (`Scope::ClassVar` Assign,
            // the ONLY body content the scan admits) initialises a class variable.
            // It runs where `self` is `main`, NOT the class, so it CANNOT use the
            // method-body `sir_cvar_owner(self)` path — write it on the class by
            // NAME instead.  The `@@`-name is a validated, safely-quoted symbol.
            for st in body {
                if let Stmt::Assign {
                    name: cv,
                    scope: Scope::ClassVar,
                    value,
                    ..
                } = st
                {
                    let _ = write!(
                        s,
                        "\n{name}.class_variable_set({}, {})",
                        emit_symbol(cv),
                        emit_expr(value)
                    );
                }
            }
            s
        }
        // OOP classes slice 7: a `module M; end` — defined REFLECTIVELY like a
        // class (`const_set` is legal anywhere; a native `module` block is a Ruby
        // error inside the `main` method), naming the module `M`.  The scan
        // guarantees a single-segment constant name and an empty body, so this is
        // a bare `Object.const_set(:M, Module.new)`.  Its methods are hoisted and
        // registered separately with `__def_method__` (reusing slice-2 machinery),
        // and a class mixes it in with the native `include`/`extend` below.
        Stmt::ModuleDef { name, .. } => format!("Object.const_set(:{name}, Module.new)"),
        // Other not-yet-supported statements (e.g. index-set, singleton defs) are
        // rejected by the capability check / scan before emit.
        other => unreachable!("Ruby backend reached unsupported statement: {other:?}"),
    }
}

/// A block used *inside an expression* (an `if` branch or a `begin…end`):
/// statements and the value joined with `; ` (valid Ruby, avoids managing
/// nested indentation).
fn emit_block_inline(b: &Block) -> String {
    let mut parts: Vec<String> = b.stmts.iter().map(emit_stmt).collect();
    parts.push(emit_expr(&b.value));
    parts.join("; ")
}

/// Render an `f64` as a Ruby **Float** literal that round-trips to the same
/// value.  Two things a naive `value.to_string()` gets wrong:
///
/// - **Integral floats lose their point.**  Rust's `f64::to_string` renders
///   `7.0` as `"7"`, which Ruby would parse as an *Integer* — a different type
///   with different `/` (floor vs true divide) and display (`7` vs `7.0`).
///   Rust's `{:?}` (Debug) instead always emits a decimal point or an exponent
///   (`7.0`, `-0.0`, `1e300`), each a valid Ruby *Float* literal, using the
///   shortest round-tripping form.
/// - **Non-finite values have no numeric literal.**  Ruby has no `inf`/`nan`
///   token (those would be method calls / bare identifiers); the values are
///   named `Float::INFINITY` / `Float::NAN`.  A `FloatLit` carrying one is rare
///   (it usually arises at runtime from `1.0 / 0.0`), but must still emit a
///   parseable expression.
///
/// The runtime's `sir_fmt_float` renders every float through Ruby's own
/// `to_s`/`nan?`/`infinite?`, so *display* is native regardless of how the
/// literal was spelled — this helper only has to preserve the numeric value.
fn float_to_ruby_literal(value: f64) -> String {
    if value.is_nan() {
        "Float::NAN".to_string()
    } else if value.is_infinite() {
        if value > 0.0 {
            "Float::INFINITY".to_string()
        } else {
            "-Float::INFINITY".to_string()
        }
    } else {
        // `{:?}` guarantees a `.`/exponent for every finite f64 → a Ruby Float.
        format!("{value:?}")
    }
}

// ---------------------------------------------------------------------------
// Expressions (Ruby is expression-oriented → emit_expr is total)
// ---------------------------------------------------------------------------

fn emit_expr(e: &Expr) -> String {
    match e {
        Expr::IntLit { value, .. } => value.to_string(),
        Expr::FloatLit { value, .. } => float_to_ruby_literal(*value),
        Expr::BoolLit { value, .. } => value.to_string(),
        Expr::NilLit { .. } => "nil".to_string(),
        Expr::SymLit { name, .. } => emit_symbol(name),
        Expr::StrLit { value, .. } => quote_ruby_string(value),
        Expr::VarRef { name, scope, .. } => emit_var_ref(name, *scope),
        Expr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => format!(
            "(if sir_truthy({}) then {} else {} end)",
            emit_expr(cond),
            emit_block_inline(then_branch),
            emit_block_inline(else_branch)
        ),
        Expr::Block(b) => {
            if b.stmts.is_empty() {
                emit_expr(&b.value)
            } else {
                format!("(begin; {}; end)", emit_block_inline(b))
            }
        }
        Expr::DirectCall { fn_name, args, .. } => {
            format!("{}({})", function_emit_name(fn_name), emit_args(args))
        }
        Expr::IndirectCall { target, args, .. } => {
            format!("sir_apply({}, {})", emit_expr(target), emit_args(args))
        }
        Expr::BuiltinCall { name, args, .. } => emit_builtin(name, args),
        Expr::MakeClosure {
            fn_name, captures, ..
        } => {
            // A closure body's fn takes (captures…, params…).  The lambda binds
            // the capture VALUES now and splats the call args at call time.
            let mut fixed: Vec<String> = captures.iter().map(|c| emit_expr(&c.value)).collect();
            fixed.push("*__sir_args".to_string());
            format!(
                "->(*__sir_args) {{ {}({}) }}",
                function_emit_name(fn_name),
                fixed.join(", ")
            )
        }
        // SIR16 sequence literal (`[1, 2, 3]`).  Ruby has native arrays, so a
        // `SeqLit` maps directly to an array-literal expression — no runtime
        // helper (unlike the Go/Rust backends, whose tagged-value runtimes need
        // `_sir_seq_lit` to box a heap sequence).  Each item is itself an
        // expression, emitted recursively.  Structural `==` (Array#==) then
        // makes `[1, 2] == [1, 2]` true, matching every other backend.
        Expr::SeqLit { items, .. } => {
            let elems: Vec<String> = items.iter().map(emit_expr).collect();
            format!("[{}]", elems.join(", "))
        }
        // `a[i]` (read). Ruby's `Array#[]` matches the SIR reference EXACTLY:
        // a negative index counts from the end (`a[-1]` is the last element),
        // and an index outside `0 .. len-1` returns `nil` (it does not raise —
        // that is `fetch`). This is the same rule `_sir_seq_index` documents on
        // the Go/Rust/Python backends, so no helper is needed. The receiver is
        // parenthesised so a compound `seq` expression indexes correctly.
        Expr::SeqIndex { seq, index, .. } => {
            format!("({})[{}]", emit_expr(seq), emit_expr(index))
        }
        // `a.length` — native `Array#length`, matching `_sir_seq_len`.
        Expr::SeqLen { seq, .. } => format!("({}).length", emit_expr(seq)),
        // SIR16 map literal (`{k => v, …}`) → a native Ruby Hash — no runtime
        // helper (unlike the Go/Rust backends, whose tagged-value runtimes box
        // a `*Map`/assoc-list). Each key and value is an expression, emitted
        // recursively. Ruby's Hash preserves insertion order and compares keys
        // structurally (`eql?`/`hash`), so a composite key like `[1, 2]` works.
        Expr::MapLit { entries, .. } => {
            let pairs: Vec<String> = entries
                .iter()
                .map(|e| format!("{} => {}", emit_expr(&e.key), emit_expr(&e.value)))
                .collect();
            format!("{{{}}}", pairs.join(", "))
        }
        // `h[k]` (read). Native `Hash#[]`: a missing key yields nil (no raise),
        // matching `_sir_map_get`. The receiver is parenthesised so a compound
        // `map` expression indexes correctly.
        Expr::MapGet { map, key, .. } => {
            format!("({})[{}]", emit_expr(map), emit_expr(key))
        }
        // SIR26: integer conversion → a mask helper chosen by target width +
        // signedness.  A target width of `Arbitrary` is the identity (a widen
        // into Ruby's already-unbounded Integer), so no helper wraps it.
        Expr::Convert { value, to, .. } => match to.width {
            IntWidth::Arbitrary => emit_expr(value),
            w => {
                let bits = w.bits().expect("non-arbitrary width has bits");
                let sign = if to.signed { 'i' } else { 'u' };
                format!("sir_{sign}{bits}({})", emit_expr(value))
            }
        },
        // SIR16 short-circuit `&&` / `||` (distinct from the eager `and`/`or`
        // builtins). Ruby's native operators ARE the SIR semantics exactly:
        // they short-circuit (the rhs is not evaluated when the lhs decides) and
        // yield the DECIDING OPERAND (not a coerced bool) — `a && b` is `a` when
        // `a` is falsy else `b`; `a || b` is `a` when truthy else `b`. And Ruby
        // truthiness is the SIR/Lisp convention (only `nil`/`false` are falsy),
        // so no `sir_truthy` wrapper is needed — unlike the Go/C backends, which
        // must lift to an IIFE / hoisted `if` to return the operand value.
        Expr::LogicalAnd { lhs, rhs, .. } => {
            format!("({} && {})", emit_expr(lhs), emit_expr(rhs))
        }
        Expr::LogicalOr { lhs, rhs, .. } => {
            format!("({} || {})", emit_expr(lhs), emit_expr(rhs))
        }
        // A SIR19 keyword argument (`Feature::KeywordParams`): `name: <value>`
        // in a call's argument list.  Ruby matches it to the callee's keyword
        // parameter by name natively, so no positional resolution is needed
        // (unlike the Go/C backends).  The label is sanitised identically to the
        // keyword parameter it binds, so the two always agree.
        Expr::KeywordArg { name, value, .. } => {
            format!("{}: {}", sanitize_ident(name), emit_expr(value))
        }
        other => unreachable!("Ruby backend reached unsupported expr: {other:?}"),
    }
}

fn emit_args(args: &[Expr]) -> String {
    args.iter().map(emit_expr).collect::<Vec<_>>().join(", ")
}

fn emit_var_ref(name: &str, scope: Scope) -> String {
    match scope {
        Scope::Local | Scope::Param | Scope::Capture => sanitize_ident(name),
        Scope::Global => format!("sir_global_get({})", quote_ruby_string(name)),
        Scope::Builtin => format!("sir_builtin_closure({})", quote_ruby_string(name)),
        // A `Scope::Const` reference (`Feature::Constants`) is a Ruby constant —
        // emitted VERBATIM as `PI` / `Foo::Bar` (NOT through `sanitize_ident`,
        // which would lowercase-prefix an uppercase name away from constant-hood).
        // `first_scan_issue` has already validated the name as a constant path at
        // this position, so it carries no injectable metacharacter.
        Scope::Const => name.to_string(),
        // A `Scope::Instance` reference (`Feature::InstanceVars`, OOP slice 3) is
        // a Ruby instance variable — the name already INCLUDES the `@` (`@v`),
        // emitted VERBATIM (NOT through `sanitize_ident`, which would mangle the
        // `@`).  `first_scan_issue` validated it as `@<identifier>` at this
        // position, so it carries no injectable metacharacter.  Inside a
        // `define_method`-installed method (slice 2) `self` IS the receiver, so
        // `@v` reads the instance's own variable — no runtime plumbing.
        Scope::Instance => name.to_string(),
        // A `Scope::ClassVar` reference (`@@x`, `Feature::ClassVars`, OOP slice 6).
        // A method body runs in a hoisted top-level function, where a bare `@@x`
        // is a Ruby error ("class variable access from toplevel"), so read it via
        // `class_variable_get` on the owner resolved by `sir_cvar_owner(self)` (the
        // class in both instance- and class-method contexts).  The name (incl.
        // `@@`) is validated as `@@<identifier>` and emitted as a safe quoted
        // symbol (no injection).
        Scope::ClassVar => {
            format!(
                "sir_cvar_owner(self).class_variable_get({})",
                emit_symbol(name)
            )
        }
        // Every `Scope` is now handled; a new variant is a compile error here
        // (the totality signal), so no `unreachable!` catch-all is needed.
    }
}

fn emit_builtin(name: &str, args: &[Expr]) -> String {
    let a: Vec<String> = args.iter().map(emit_expr).collect();
    match name {
        // n-ary arithmetic folds to a native chained operator.
        "+" => format!("({})", join_op(&a, " + ", "0")),
        "*" => format!("({})", join_op(&a, " * ", "1")),
        "/" => format!("({})", join_op(&a, " / ", "1")),
        "-" => {
            if a.len() == 1 {
                format!("(-{})", a[0])
            } else {
                format!("({})", join_op(&a, " - ", "0"))
            }
        }
        "%" => format!("({} % {})", arg(&a, 0), arg(&a, 1)),
        "neg" => format!("(-{})", arg(&a, 0)),
        // Bitwise / shift — Ruby's Integer supports all of these natively on
        // arbitrary-precision two's-complement, so the width is enforced by the
        // surrounding `sir_uN`/`sir_iN` mask (a `Convert`).  `>>` on a negative
        // value arithmetic-shifts (a signed operand arrives negative); on a
        // masked non-negative unsigned value it is a logical shift.
        "&" => format!("({} & {})", arg(&a, 0), arg(&a, 1)),
        "|" => format!("({} | {})", arg(&a, 0), arg(&a, 1)),
        "^" => format!("({} ^ {})", arg(&a, 0), arg(&a, 1)),
        "~" => format!("(~{})", arg(&a, 0)),
        "<<" => format!("({} << {})", arg(&a, 0), arg(&a, 1)),
        // Both `>>` and the unsigned `u>>` render the same: a Ruby unsigned
        // value is a masked non-negative Integer, so `>>` is already logical
        // there (the distinction only matters for the C backend's signed int64).
        ">>" | "u>>" => format!("({} >> {})", arg(&a, 0), arg(&a, 1)),
        // Truncating (C-style) division / remainder via the runtime helpers.
        // The unsigned variants reuse them: a Ruby unsigned value is a
        // non-negative Integer, for which truncation and flooring coincide.
        "tdiv" | "utdiv" => format!("sir_tdiv({}, {})", arg(&a, 0), arg(&a, 1)),
        "tmod" | "utmod" => format!("sir_tmod({}, {})", arg(&a, 0), arg(&a, 1)),
        // `to_f`/`to_i`: Ruby's native `Numeric#to_f` (int → Float) and
        // `Float#to_i` (truncates toward zero, matching C's `(int)double` cast).
        "to_f" => format!("({}).to_f", arg(&a, 0)),
        "to_i" => format!("({}).to_i", arg(&a, 0)),
        // `fmt_float(value, precision, kind)` → the runtime C-printf-faithful
        // formatter (Ruby's `sprintf` is C-compatible; the runtime switches on
        // the fixed `kind` character so no format string is source-derived).
        "fmt_float" => format!(
            "sir_fmt_float_c({}, {}, {})",
            arg(&a, 0),
            arg(&a, 1),
            arg(&a, 2)
        ),
        "not" => format!("(!sir_truthy({}))", arg(&a, 0)),
        "<" => format!("({} < {})", arg(&a, 0), arg(&a, 1)),
        ">" => format!("({} > {})", arg(&a, 0), arg(&a, 1)),
        "<=" => format!("({} <= {})", arg(&a, 0), arg(&a, 1)),
        ">=" => format!("({} >= {})", arg(&a, 0), arg(&a, 1)),
        "=" | "==" => format!("sir_eq({}, {})", arg(&a, 0), arg(&a, 1)),
        "!=" => format!("(!sir_eq({}, {}))", arg(&a, 0), arg(&a, 1)),
        // Short-circuit: Ruby &&/|| use nil/false truthiness and return the
        // deciding operand — exactly the SIR `and`/`or` semantics.
        "and" => format!("({} && {})", arg(&a, 0), arg(&a, 1)),
        "or" => format!("({} || {})", arg(&a, 0), arg(&a, 1)),
        "cons" => format!("sir_cons({}, {})", arg(&a, 0), arg(&a, 1)),
        "car" => format!("sir_car({})", arg(&a, 0)),
        "cdr" => format!("sir_cdr({})", arg(&a, 0)),
        "null?" => format!("sir_is_null({})", arg(&a, 0)),
        "pair?" => format!("sir_is_pair({})", arg(&a, 0)),
        "number?" => format!("sir_is_number({})", arg(&a, 0)),
        "symbol?" => format!("sir_is_symbol({})", arg(&a, 0)),
        "print" => format!("sir_print({})", a.join(", ")),
        "puts" => format!("sir_puts({})", a.join(", ")),
        "global_get" => format!("sir_global_get({})", arg(&a, 0)),
        "global_set" => format!("sir_global_set({}, {})", arg(&a, 0), arg(&a, 1)),
        // SIR17 exceptions.  `raise` with no argument re-raises the exception
        // being handled (`$!`); with a message string it raises a `RuntimeError`
        // (`raise "boom"`); with an exception object it re-raises that.  Each
        // argument is an already-emitted expression (a `StrLit` is quoted, so no
        // source can inject).  `retry` restarts the enclosing `begin` body.
        "raise" => {
            if a.is_empty() {
                "raise".to_string()
            } else {
                format!("raise({})", a.join(", "))
            }
        }
        "retry" => "retry".to_string(),
        // OOP classes slice 1: `Foo.new(args…)`.  `args[0]` is a `StrLit`
        // holding the class name, emitted VERBATIM as the constant receiver
        // (the scan validated it as a constant path at this position, so no
        // source can inject); `args[1..]` (already in `a`) are the constructor
        // arguments.
        //
        // Route through the `sir_new` runtime helper, NOT a native `Class.new`.
        // A `def initialize` is registered (like every method, OOP slice 2)
        // under the reserved `sir_um_` prefix as `sir_um_initialize` — a name
        // Ruby's own `new`/`initialize` never calls — so a native `.new` would
        // allocate an instance whose constructor body (its `@ivar` initialisers)
        // NEVER runs, leaving every `@ivar` nil (the `counter_state` conformance
        // failure: `@n + 1` on a nil `@n`).  `sir_new` mirrors the Go/C/Rust
        // runtimes: allocate, then explicitly invoke the registered constructor
        // with these args (see the runtime preamble).
        "__new__" => {
            let class = match args.first() {
                Some(Expr::StrLit { value, .. }) => value.clone(),
                // The scan rejects a malformed `__new__` before emit.
                _ => unreachable!("__new__ requires a string class-name first argument"),
            };
            let mut parts = vec![class];
            parts.extend_from_slice(&a[1..]);
            format!("sir_new({})", parts.join(", "))
        }
        // OOP classes slice 2 — register a hoisted instance method on the class.
        // `args[0]` = class name (`StrLit`, a bare validated constant receiver),
        // `args[1]` = method name (`StrLit`), `args[2]` = a `MakeClosure` (already
        // emitted in `a[2]` as `->(*__sir_args){ Class__m(*__sir_args) }`).  The
        // method name is rendered under a RESERVED `sir_um_` prefix as a quoted
        // symbol via `emit_symbol` (so it cannot inject and never collides with a
        // built-in), then installed with `define_method`.  `define_method` binds
        // `self` to the receiver at call time, so the hoisted body sees the
        // instance (its `@ivars`, once slice 3 lands).
        "__def_method__" => {
            let class = str_arg(args, 0);
            let sym = emit_symbol(&format!("sir_um_{}", str_arg(args, 1)));
            format!("{class}.define_method({sym}, &({}))", a[2])
        }
        // Dispatch an instance method: `(recv).public_send(:sir_um_<m>, args…)`.
        // The `sir_um_` prefix makes this a CLOSED dispatch — `public_send` can
        // only reach methods installed by `__def_method__` (no built-in is named
        // `sir_um_*`), so a crafted method name cannot reach `instance_eval` /
        // `send` / any reflection sink.  `args[0]` = receiver (`a[0]`), `args[1]`
        // = method name, `args[2..]` (in `a`) = call arguments.
        "__method__" => {
            let sym = emit_symbol(&format!("sir_um_{}", str_arg(args, 1)));
            let mut parts = vec![sym];
            parts.extend_from_slice(&a[2..]);
            format!("({}).public_send({})", a[0], parts.join(", "))
        }
        // OOP classes slice 3: a bare `self` → the native `self` keyword.  Inside
        // a `define_method`-installed method (slice 2) `self` is the receiver.
        "__self__" => "self".to_string(),
        // OOP classes slice 4: a `super` call.  `args[0]` = method name, `args[1]`
        // = the DEFINING class (a bare validated constant), `args[2..]` (in `a`) =
        // the forwarded arguments.  A method body lives in a hoisted top-level
        // function (not a real method context), so native `super` cannot be used;
        // dispatch EXPLICITLY up the ancestry: fetch the superclass's method as an
        // `UnboundMethod`, bind it to `self` (the receiver, inherited via slice
        // 2's `define_method`), and call it.  The method name is `sir_um_`-prefixed
        // (a quoted symbol), so `instance_method` can only fetch a user method —
        // never a reflection built-in (anti-RCE, as with `__method__` dispatch).
        "__super__" => {
            let class = str_arg(args, 1);
            let sym = emit_symbol(&format!("sir_um_{}", str_arg(args, 0)));
            format!(
                "({class}).superclass.instance_method({sym}).bind(self).call({})",
                a[2..].join(", ")
            )
        }
        // OOP classes slice 5 — register a class (singleton) method.  `args[0]` =
        // class name (a bare validated constant), `args[1]` = method name,
        // `args[2]` = the `MakeClosure` (in `a[2]`).  `define_singleton_method`
        // installs it on the class's singleton, under the SAME reserved `sir_um_`
        // prefix as instance methods (separate method tables, so no collision) —
        // keeping class-method dispatch closed (anti-RCE).
        "__def_class_method__" => {
            let class = str_arg(args, 0);
            let sym = emit_symbol(&format!("sir_um_{}", str_arg(args, 1)));
            format!("{class}.define_singleton_method({sym}, &({}))", a[2])
        }
        // Dispatch a class method: `(<Class>).public_send(:sir_um_<m>, args…)`.
        // The class NAME (`args[0]`) is the bare-constant receiver; the method
        // name (`args[1]`) is `sir_um_`-prefixed so `public_send` can only reach a
        // registered class method (anti-RCE).  `args[2..]` (in `a`) are the call
        // arguments.
        "__class_method__" => {
            let class = str_arg(args, 0);
            let sym = emit_symbol(&format!("sir_um_{}", str_arg(args, 1)));
            let mut parts = vec![sym];
            parts.extend_from_slice(&a[2..]);
            format!("({class}).public_send({})", parts.join(", "))
        }
        // OOP classes slice 7 — mix a module into a class.  `args[0]` = the class,
        // `args[1]` = the module, BOTH bare validated constants.  `include` adds
        // the module's methods as INSTANCE methods (resolved through the ancestry
        // by the existing `__method__`/`public_send` dispatch); `extend` adds them
        // as SINGLETON (class) methods.  Native Ruby — no runtime helper.
        "__include__" => format!("({}).include({})", str_arg(args, 0), str_arg(args, 1)),
        "__extend__" => format!("({}).extend({})", str_arg(args, 0), str_arg(args, 1)),
        // Unreachable: first_scan_issue rejected anything else.
        other => unreachable!("v0 Ruby backend reached unsupported builtin: {other}"),
    }
}

/// Join operands with a binary operator, or fall back to `empty` for 0 args.
fn join_op(a: &[String], op: &str, empty: &str) -> String {
    if a.is_empty() {
        empty.to_string()
    } else {
        a.join(op)
    }
}

/// The i-th argument, or `nil` if the frontend under-supplied (defensive).
fn arg(a: &[String], i: usize) -> String {
    a.get(i).cloned().unwrap_or_else(|| "nil".to_string())
}

/// The i-th argument's raw `StrLit` value — for an OOP builtin whose class /
/// method name is a compile-time string the emitter renders specially (a bare
/// constant receiver, or a `sir_um_`-prefixed symbol).  The pre-emit scan
/// guarantees this position is a `StrLit`, so a non-string is unreachable.
fn str_arg(args: &[Expr], i: usize) -> String {
    match args.get(i) {
        Some(Expr::StrLit { value, .. }) => value.clone(),
        _ => unreachable!("OOP builtin requires a string argument at position {i}"),
    }
}

// ---------------------------------------------------------------------------
// Literals / identifiers / escaping
// ---------------------------------------------------------------------------

/// Render a SIR symbol as a Ruby symbol literal.  A clean identifier becomes
/// `:name`; anything else becomes the quoted form `:"…"`.
fn emit_symbol(name: &str) -> String {
    let clean = !name.is_empty()
        && name
            .chars()
            .enumerate()
            .all(|(i, c)| c == '_' || c.is_ascii_alphanumeric() && !(i == 0 && c.is_ascii_digit()));
    if clean {
        format!(":{name}")
    } else {
        format!(":{}", quote_ruby_string(name))
    }
}

/// Map a SIR identifier to a valid Ruby local/method identifier.  Ruby locals
/// may not start with an uppercase letter (that is a constant) or a digit, and
/// keywords / the runtime `sir_` namespace are suffixed with `_`.
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
    // A leading uppercase would make it a constant; prefix an underscore.
    if out.chars().next().is_some_and(|c| c.is_ascii_uppercase()) {
        out.insert(0, '_');
    }
    if is_ruby_keyword(&out) || out.starts_with("sir_") {
        out.push('_');
    }
    out
}

fn is_ruby_keyword(s: &str) -> bool {
    matches!(
        s,
        "BEGIN"
            | "END"
            | "alias"
            | "and"
            | "begin"
            | "break"
            | "case"
            | "class"
            | "def"
            | "defined?"
            | "do"
            | "else"
            | "elsif"
            | "end"
            | "ensure"
            | "false"
            | "for"
            | "if"
            | "in"
            | "module"
            | "next"
            | "nil"
            | "not"
            | "or"
            | "redo"
            | "rescue"
            | "retry"
            | "return"
            | "self"
            | "super"
            | "then"
            | "true"
            | "undef"
            | "unless"
            | "until"
            | "when"
            | "while"
            | "yield"
            | "__FILE__"
            | "__LINE__"
            // `__ENCODING__` is the third of Ruby's three magic-constant
            // keywords (alongside `__FILE__`/`__LINE__` above) — a real
            // lexical keyword, not a plain identifier: `__ENCODING__ = 5`
            // is a `SyntaxError` (verified against MRI/Ruby 3.4), exactly
            // like the other two. It was missing from this list even
            // though its two siblings were already here.
            | "__ENCODING__"
            | "__method__"
            | "lambda"
            | "proc"
    )
}

/// Escape a Rust string into a double-quoted Ruby string literal (with quotes).
/// `#` is escaped so no `#{…}` / `#@` / `#$` interpolation can fire, and control
/// bytes become `\xHH`, so no source text can break out or inject.
fn quote_ruby_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '#' => out.push_str("\\#"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\x{:02X}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Strip line terminators from text destined for a `# …` comment.
fn sanitize_comment(s: &str) -> String {
    s.replace(['\n', '\r'], " ")
}
