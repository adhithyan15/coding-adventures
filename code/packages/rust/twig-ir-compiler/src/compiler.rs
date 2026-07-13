//! Twig → InterpreterIR (IIR) compiler.
//!
//! This module turns a [`twig_parser::Program`] into an
//! [`interpreter_ir::IIRModule`].  The lowering follows the design laid out
//! in `code/specs/TW00-twig-language.md` and mirrors the Python reference
//! implementation at `code/packages/python/twig/src/twig/compiler.py`.
//!
//! ## What gets emitted
//!
//! The output module always contains:
//!
//! 1. **One `IIRFunction` per `(define (name args...) body+)` form.**
//!    Parameters lower 1-to-1 to IIR params.  They stay `"any"` unless an
//!    explicit static annotation or a conservative LANG-FULL E4 call-site proof
//!    can stamp a concrete hint such as `"str"`; the function body is the
//!    lowered body expressions plus a final `ret`.
//! 2. **One `IIRFunction` per anonymous `(lambda ...)` expression.**
//!    Synthetic name (`__lambda_0`, `__lambda_1`, …); captured variables
//!    appear as the *leading* parameters, in the order produced by
//!    [`free_vars`](super::free_vars::free_vars).
//! 3. **A synthesised `main` function.**  Holds, in source order:
//!    - top-level value defines (each emitted as
//!      `call_builtin "global_set" <name> <rhs>`),
//!    - bare top-level expressions.
//!
//!    The value of the *last* bare expression becomes `main`'s return.
//!    Programs with no bare expressions return `nil` via
//!    `call_builtin "make_nil"`.
//!
//! Twig remains dynamically typed, so functions keep `type_status = Untyped`
//! and dynamic paths still carry `type_hint = "any"`.  The LANG-FULL fast paths
//! stamp concrete hints only where source-local evidence makes the type
//! unambiguous.
//! The vm-core profiler will fill in observed types at runtime; the JIT can
//! specialise from those observations.
//!
//! ## Apply-site dispatch (compile-time)
//!
//! The compiler decides at compile time whether each `(fn args...)` is a
//! direct call, a builtin, or an indirect closure call:
//!
//! | Function position           | Emitted IIR                                 |
//! |-----------------------------|---------------------------------------------|
//! | Top-level user fn name      | `call <name>, ...args`                      |
//! | Builtin name (`+`, `cons`)  | `call_builtin <name>, ...args`              |
//! | Anything else (locals etc.) | `call_closure h, ...args` (LANG34)          |
//!
//! Top-level recursion stays on the fast `call` path; only locals
//! holding closures pay the indirect cost.
//!
//! ## LANG34 closure opcodes
//!
//! Since LANG34 the compiler emits first-class closure opcodes instead of
//! routing through `call_builtin`:
//!
//! - Lambda allocation: `alloc_closure(Str(fn_name), cap0, cap1, …) : "closure"`
//!   The function name is an inline `Operand::Str` — no preceding `const`
//!   instruction is needed.  The old `string_arg` helper is retained for
//!   `global_set`/`global_get` / `make_symbol` uses.
//! - Indirect calls: `call_closure(handle, arg0, arg1, …) : "any"`
//!   Replaces the former `call_builtin "apply_closure" handle args…` form.
//!
//! ## Encoding string operands (for globals and symbols)
//!
//! `Operand::Str(literal)` is now the canonical way to embed a compile-time
//! string in an instruction.  The `string_arg` helper (which emits a `const`
//! with `Operand::Var(literal)`) is retained for operations that still need
//! the old convention: `global_set`, `global_get`, `make_symbol`.  See the
//! LANG32 spec for details on the `global_load`/`global_store` lowering pass.

use std::collections::{HashMap, HashSet};

use interpreter_ir::{
    function::{FunctionTypeStatus, IIRFunction},
    instr::{IIRInstr, Operand},
    module::IIRModule,
    module_exports::IIRExport,
    SourceLoc,
};
use lang_refined_types::{Kind, Predicate, RefinedType};

use twig_parser::{
    Apply, Begin, BoolLit, Expr, Form, If, IntLit, Lambda, Let,
    // LANG52: sequential let* bindings
    LetStar,
    Match, MatchPat, NilLit, Program,
    RecordDef, StrLit, SymLit, TypeAnnotation, TypeExpr, UnionDef, VarRef,
};

use crate::errors::TwigCompileError;
use crate::free_vars::free_vars;

// ---------------------------------------------------------------------------
// LANG23 PR 23-E — TypeAnnotation → RefinedType conversion
// ---------------------------------------------------------------------------

/// Convert a parsed [`TypeAnnotation`] into a [`RefinedType`] that the IIR
/// carries and the refinement checker reads.
///
/// This is the bridge between the syntactic form (what the Twig parser
/// produces) and the semantic form (what `lang-refinement-checker` understands).
///
/// # Mapping
///
/// | `TypeAnnotation`          | `RefinedType`                                       |
/// |---------------------------|-----------------------------------------------------|
/// | `UnrefinedInt`            | `RefinedType::unrefined(Kind::Int)`                 |
/// | `Any`                     | `RefinedType::unrefined(Kind::Any)`                 |
/// | `UnrefinedBool`           | `RefinedType::unrefined(Kind::Bool)`                |
/// | `RangeInt { lo, hi }`     | `RefinedType::refined(Int, Range{lo,hi,excl_hi})`   |
/// | `MembershipInt { values }`| `RefinedType::refined(Int, Membership{values})`     |
fn type_annotation_to_refined_type(ann: &TypeAnnotation) -> RefinedType {
    match ann {
        TypeAnnotation::UnrefinedInt => RefinedType::unrefined(Kind::Int),
        TypeAnnotation::Any => RefinedType::unrefined(Kind::Any),
        TypeAnnotation::UnrefinedBool => RefinedType::unrefined(Kind::Bool),
        TypeAnnotation::RangeInt { lo, hi } => RefinedType::refined(
            Kind::Int,
            Predicate::Range {
                lo: Some(*lo),
                hi: Some(*hi),
                inclusive_hi: false, // Twig v1 always uses exclusive upper bound
            },
        ),
        TypeAnnotation::MembershipInt { values } => RefinedType::refined(
            Kind::Int,
            Predicate::Membership { values: values.clone() },
        ),
        // TW05-A / LANG48: Opaque annotations (non-LANG23 TW05 type expressions)
        // are erased to `any` in TW05-A.  The TW05-B type checker will interpret
        // them; for now they're treated as unconstrained.
        TypeAnnotation::Opaque(_) => RefinedType::unrefined(Kind::Any),
    }
}

fn type_annotation_static_iir_hint(ann: &TypeAnnotation) -> Option<&'static str> {
    match ann {
        TypeAnnotation::Opaque(TypeExpr::Name(name))
            if matches!(name.as_str(), "str" | "Str" | "string" | "String") =>
        {
            Some("str")
        }
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StaticParamEvidence {
    Unknown,
    Str,
    Conflict,
}

impl StaticParamEvidence {
    fn observe(self, is_str: bool) -> Self {
        match (self, is_str) {
            (StaticParamEvidence::Unknown, true) | (StaticParamEvidence::Str, true) => {
                StaticParamEvidence::Str
            }
            _ => StaticParamEvidence::Conflict,
        }
    }
}

/// Maximum AST-nesting depth the compiler will descend.
///
/// Mirrors `twig_parser::MAX_NESTING_DEPTH`.  The parser already caps
/// nesting on its way in, so well-behaved inputs never hit this; the
/// extra check protects against `compile_program` being called with a
/// hand-built AST that bypasses the parser, and against future grammar
/// changes that might decouple parse depth from compile depth (e.g. a
/// macro-expansion pass that synthesises deeper trees).
pub const MAX_COMPILE_DEPTH: usize = 256;

// ---------------------------------------------------------------------------
// Builtins
// ---------------------------------------------------------------------------
//
// Names that resolve to host-side callables registered with vm-core's
// `BuiltinRegistry`.  Keep this list in sync with the Python
// `twig.compiler.BUILTINS` set — the surface contract for what counts
// as a builtin is identical across language frontends.
// ---------------------------------------------------------------------------

const BUILTINS: &[&str] = &[
    // Arithmetic / comparison (TW00 core)
    "+", "-", "*", "/", "=", "<", ">",
    // LANG52: extended comparisons and arithmetic
    "<=", ">=", "modulo", "remainder", "quotient",
    // Cons cells
    "cons", "car", "cdr",
    // Predicates (TW00 core)
    "null?", "pair?", "number?", "symbol?",
    // LANG52: boolean and type predicates
    "not", "boolean?",
    // LANG52: structural equality
    "equal?",
    // LANG52: list stdlib
    "list", "length", "append", "reverse", "list-ref", "assoc", "list?",
    // LANG52: symbol utilities
    "symbol-append",
    // LANG57: string ↔ number ↔ symbol conversions (already in lispy-runtime LANG47+)
    "number->string", "string->symbol", "symbol->string",
    // LANG58: string and character operations (lispy-runtime LANG47, wired here for TW05-E)
    "string-length", "string-ref", "substring", "string-append",
    "string->number", "string=?", "string<?", "string>?",
    "char->integer", "integer->char",
    "char-alphabetic?", "char-numeric?", "char-whitespace?",
    // I/O
    "print",
    // Host I/O (LANG52) — these dispatch via call_builtin to exec_host_call
    "host/write_string", "host/read_line", "host/read_file",
    // LANG55: higher-order list operations — dispatch via special-cased
    // exec_hof_* handlers in twig-vm that can recurse into `dispatch`.
    "map", "filter", "fold-left", "fold-right",
];

fn is_builtin(name: &str) -> bool {
    BUILTINS.contains(&name)
}

/// Path-A increment 2: map an arithmetic / comparison builtin name to the
/// typed CIR mnemonic that the IIR-to-* backends accept directly.
///
/// Returns `Some(&'static str)` when the builtin has a typed-CIR analog;
/// `None` for builtins that have no direct typed equivalent yet
/// (`cons`, `length`, `host/*`, the higher-order list ops, etc.).
///
/// The caller's responsibility is to verify that the operand types are
/// concrete before substituting the typed mnemonic — see the
/// `compile_apply` arm for `is_builtin(&v.name)`.  This table is the
/// pure name → name lookup; type-check happens at the call site.
///
/// Mirrors the same pattern used by `nib-iir-compiler::cir_op_for`
/// (PR #3903) and `oct-iir-compiler::compile_binary`.
fn typed_arith_op_for(name: &str) -> Option<&'static str> {
    match name {
        // Arithmetic — all four basic operators map to typed CIR mnemonics.
        "+" => Some("add"),
        "-" => Some("sub"),
        "*" => Some("mul"),
        "/" => Some("div"),
        // Comparison — Twig uses Scheme-style names; CIR uses C-style.
        // `=` is Twig's equality (works on numbers, symbols, booleans,
        // strings, etc.).  For increment 2 we only fire when both operands
        // are `i64`, in which case `=` reduces to integer equality —
        // which `cmp_eq` (with type `i64`) is exactly.
        "="  => Some("cmp_eq"),
        "<"  => Some("cmp_lt"),
        ">"  => Some("cmp_gt"),
        "<=" => Some("cmp_le"),
        ">=" => Some("cmp_ge"),
        _    => None,
    }
}

// ---------------------------------------------------------------------------
// Per-function compilation context
// ---------------------------------------------------------------------------

/// Mutable state while lowering one [`IIRFunction`].
///
/// `instrs` accumulates the body in emission order; `locals` records
/// names introduced at this function level (parameters + active `let`
/// bindings) so [`Compiler::compile_var_ref`] can distinguish locals
/// from globals; the two counters generate fresh register and label
/// names that won't collide.
struct FnCtx {
    instrs: Vec<IIRInstr>,
    /// Per-instruction source positions, kept in **lockstep** with
    /// [`Self::instrs`] (`source_map[i]` = position of `instrs[i]`).
    /// See [`interpreter_ir::SourceLoc`] for indexing conventions.
    source_map: Vec<SourceLoc>,
    locals: HashSet<String>,
    /// Lexical names backed by an already-materialised register. This keeps
    /// string-producing E4 bindings out of backend-unsupported `mov [str]` IR.
    local_aliases: HashMap<String, String>,
    var_counter: usize,
    label_counter: usize,
    /// Current AST-nesting depth.  Incremented on every entry to
    /// `compile_expr` and checked against [`MAX_COMPILE_DEPTH`] to
    /// guard against stack-overflow on adversarial input.
    depth: usize,
    /// LANG-Twig increment-1 local type inference.
    ///
    /// Tracks the *statically-known* type of each destination variable
    /// produced by the current function.  Populated only for sites where
    /// the type is unambiguous from the source code alone (integer
    /// literals → `"i64"`, boolean literals → `"bool"`, string literals
    /// → `"str"`).  Dynamic / call_builtin destinations are intentionally
    /// **not** recorded — the absence of an entry means "the type is
    /// genuinely `any` at static-analysis time".
    ///
    /// Used by `ret` emission sites to upgrade their `type_hint` from
    /// the legacy `"any"` to the precise type, which lets the IIR-to-*
    /// backends (wasm/jvm/clr/beam) actually accept the result.
    /// See [`crate::lib`]'s "Twig path A" notes for the larger story.
    var_types: HashMap<String, String>,
}

impl FnCtx {
    fn new() -> Self {
        FnCtx {
            instrs: Vec::new(),
            source_map: Vec::new(),
            locals: HashSet::new(),
            local_aliases: HashMap::new(),
            var_counter: 0,
            label_counter: 0,
            depth: 0,
            var_types: HashMap::new(),
        }
    }

    fn fresh_var(&mut self, prefix: &str) -> String {
        self.var_counter += 1;
        format!("_{prefix}{}", self.var_counter)
    }

    fn fresh_label(&mut self, prefix: &str) -> String {
        self.label_counter += 1;
        format!("_{prefix}{}", self.label_counter)
    }

    /// Push an instruction + its source position in lockstep.  Every
    /// IR-emit site goes through this — the lockstep invariant
    /// (`source_map.len() == instrs.len()`) is maintained by
    /// construction.  See [`SourceLoc::SYNTHETIC`] for instructions
    /// the compiler synthesises with no real source counterpart.
    fn emit(&mut self, instr: IIRInstr, loc: SourceLoc) {
        self.instrs.push(instr);
        self.source_map.push(loc);
    }

    /// Record that `var` was statically inferred to have type `ty`.
    ///
    /// Only literal-defining sites (`IntLit`, `BoolLit`, `StrLit`) call
    /// this; dynamically-typed sites (call_builtin, ret of unknown
    /// register, etc.) leave the entry absent, signalling "type is
    /// genuinely `any` at static-analysis time".
    fn record_type(&mut self, var: &str, ty: &str) {
        self.var_types.insert(var.to_string(), ty.to_string());
    }

    fn emit_str_const(&mut self, literal: &str, loc: SourceLoc) -> String {
        let v = self.fresh_var("s");
        self.emit_str_const_to(&v, literal, loc);
        v
    }

    fn emit_str_const_to(&mut self, dest: &str, literal: &str, loc: SourceLoc) {
        self.emit(
            IIRInstr::new(
                "str_const",
                Some(dest.to_string()),
                vec![Operand::Str(literal.to_string())],
                "str",
            ),
            loc,
        );
        self.record_type(dest, "str");
    }

    /// Look up the inferred type of `var`, returning the matching
    /// `&'static str` if known.  Returns `"any"` when nothing was
    /// inferred — which is the legacy default and a valid type hint
    /// (just one that the IIR-to-* backends reject).
    fn type_of(&self, var: &str) -> &str {
        self.var_types.get(var).map(|s| s.as_str()).unwrap_or("any")
    }

    /// Path-A increment 3: emit a typed `mov dst = src` instruction,
    /// propagating the source's inferred type to the destination.
    ///
    /// Replaces the legacy `call_builtin "_move" src` emission pattern
    /// that the IIR-to-* backend validators all reject (because
    /// `_move` is not in their `CALL_BUILTIN_SUPPORTED_NAMES` whitelist).
    /// The typed `mov` IR opcode is accepted by every backend
    /// (vm-core dispatch fix #3888, iir-to-beam mov lowering #3898,
    /// and the iir-to-wasm / iir-to-jvm / iir-to-cil backends all
    /// have native `"mov"` arms in their `lower.rs` match tables).
    ///
    /// The `type_hint` carried on the `mov` is the source variable's
    /// type — `"any"` for dynamically-typed sources, `"i64"` / `"bool"`
    /// for sources statically inferred by increments 1 and 2.
    ///
    /// When the source's type is known, the destination's type is
    /// recorded so downstream consumers (e.g. the outer `ret` in
    /// `main`) can propagate further.
    fn emit_move(&mut self, dst: &str, src: &str, loc: SourceLoc) {
        let ty = self.type_of(src).to_string();
        self.emit(
            IIRInstr::new(
                "mov",
                Some(dst.to_string()),
                vec![Operand::Var(src.to_string())],
                ty.as_str(),
            ),
            loc,
        );
        if ty != "any" {
            self.record_type(dst, ty.as_str());
        }
    }
}

// ---------------------------------------------------------------------------
// Compiler
// ---------------------------------------------------------------------------

/// Walks a [`Program`] and accumulates [`IIRFunction`]s.
///
/// One instance compiles one program.  Pre-pass classifies top-level
/// defines into `fn_globals` / `value_globals`; the main pass walks
/// every form and emits IR.  Anonymous lambdas append to
/// [`Self::functions`] as they are encountered (depth-first).
pub struct Compiler {
    /// Names of top-level defines whose RHS is a `Lambda` — direct
    /// callables.
    fn_globals: HashSet<String>,
    /// Statically-known return types for top-level functions already lowered in
    /// source order. Direct calls that appear after such a definition can carry
    /// the concrete IIR type instead of falling back to `any`.
    fn_return_types: HashMap<String, String>,
    /// Statically-known parameter types for top-level functions already lowered
    /// in source order. Used only to keep later direct string arguments on the
    /// E4 `str_const`/string-expression path when a callee parameter is `str`.
    fn_param_types: HashMap<String, Vec<String>>,
    /// Conservative call-site-derived parameter hints for top-level functions.
    /// A hint appears only when `main`-level direct calls provide static E4
    /// string evidence and no conflicting evidence for that parameter.
    fn_inferred_param_types: HashMap<String, Vec<String>>,
    /// Names of top-level defines whose RHS is *not* a lambda — looked
    /// up through `global_get` at use sites.
    value_globals: HashSet<String>,
    /// TW2: value-globals that are captured by a lambda somewhere and so MUST
    /// stay on the host global table (`global_set` / `global_get`).  Computed
    /// once in the pre-pass by [`free_vars::lambda_captured_globals`].
    escaping_value_globals: HashSet<String>,
    /// TW2: a *non-escaping*, statically-typed value-global's main-`fn` register.
    /// Reads of these names return the register directly (a typed value the
    /// code-gen backends accept) instead of emitting a dynamic `global_get`.
    value_global_locals: HashMap<String, String>,
    /// TW2: value-globals read *before* their `define` (a top-level forward
    /// reference).  Such a name already emitted a `global_get`, so its later
    /// `define` must emit the matching `global_set` rather than the typed-local
    /// form — keeping behaviour byte-identical to the pre-TW2 dynamic path.
    forced_global_set: HashSet<String>,
    /// Cumulative function table.  Top-level fns are appended in
    /// source order; anonymous lambdas append as the compiler
    /// encounters them, with `main` appended last.
    functions: Vec<IIRFunction>,
    /// Counter for synthesising lambda names (`__lambda_0`,
    /// `__lambda_1`, …).
    lambda_counter: usize,
    /// TW05-A / LANG48: integer tag for each union variant constructor.
    ///
    /// Populated during the pre-pass when `Form::UnionDef` forms are
    /// encountered.  Consulted when lowering `Expr::Match` arms — a
    /// pattern whose name is in this map is a variant pattern;
    /// otherwise it is a bare-name binding.
    variant_tags: HashMap<String, usize>,
}

impl Compiler {
    pub fn new() -> Self {
        Compiler {
            fn_globals: HashSet::new(),
            fn_return_types: HashMap::new(),
            fn_param_types: HashMap::new(),
            fn_inferred_param_types: HashMap::new(),
            value_globals: HashSet::new(),
            escaping_value_globals: HashSet::new(),
            value_global_locals: HashMap::new(),
            forced_global_set: HashSet::new(),
            functions: Vec::new(),
            lambda_counter: 0,
            variant_tags: HashMap::new(),
        }
    }

    /// Pre-populate `fn_globals` with extern function names from other modules.
    ///
    /// Called by `twig-module-driver` (LANG56) before compilation so that
    /// cross-module calls compile to `call` instructions rather than failing
    /// with "unbound name".  The linker resolves the actual call targets.
    ///
    /// # Example
    ///
    /// ```
    /// use twig_ir_compiler::Compiler;
    ///
    /// let c = Compiler::new().with_extern_fns(&["double", "triple"]);
    /// // Now the compiler will accept `(double x)` even without a local define.
    /// ```
    pub fn with_extern_fns(mut self, extern_fns: &[&str]) -> Self {
        for name in extern_fns {
            self.fn_globals.insert((*name).to_string());
        }
        self
    }

    // ------------------------------------------------------------------
    // Top-level driver
    // ------------------------------------------------------------------

    /// Compile a [`Program`] into an [`IIRModule`].  Consumes `self`.
    pub fn compile(mut self, program: &Program, module_name: &str) -> Result<IIRModule, TwigCompileError> {
        // ── Pre-pass: classify top-level defines + register typed forms ────
        //
        // Free-variable analysis at lambda sites needs to know which names are
        // globals (and therefore *not* free) before we walk any bodies, so we
        // do this in one pre-pass.
        //
        // TW05-A / LANG48 additions:
        // - `Form::RecordDef` generates constructor + accessors — all registered
        //   as `fn_globals` so direct calls reach the fast `call` path.
        // - `Form::UnionDef` generates constructors + predicates + accessors *and*
        //   populates `self.variant_tags` so `match` lowering can look up tags.
        for form in &program.forms {
            match form {
                Form::Define(def) => {
                    if matches!(def.expr, Expr::Lambda(_)) {
                        self.fn_globals.insert(def.name.clone());
                    } else {
                        self.value_globals.insert(def.name.clone());
                    }
                }
                Form::RecordDef(rec) => {
                    // Constructor name (same as type name, e.g. "Span").
                    self.fn_globals.insert(rec.name.clone());
                    // Accessor names: <lower(RecordName)>-<field-name>.
                    let prefix = rec.name.to_lowercase();
                    for f in &rec.fields {
                        self.fn_globals.insert(format!("{prefix}-{}", f.name));
                    }
                    // Type predicate: <lower(RecordName)>?
                    self.fn_globals.insert(format!("{prefix}?"));
                }
                Form::UnionDef(union) => {
                    for (idx, variant) in union.variants.iter().enumerate() {
                        // Constructor name (same as variant name, e.g. "IntLit").
                        self.fn_globals.insert(variant.name.clone());
                        // Predicate: <VariantName>?
                        self.fn_globals.insert(format!("{}?", variant.name));
                        // Accessor names: <lower(VariantName)>-<field-name>.
                        let vprefix = variant.name.to_lowercase();
                        for f in &variant.fields {
                            self.fn_globals.insert(format!("{vprefix}-{}", f.name));
                        }
                        // Register the integer tag for match-arm lowering.
                        self.variant_tags.insert(variant.name.clone(), idx);
                    }
                }
                _ => {}
            }
        }

        // TW2: decide which value-globals may be lowered to typed `main`
        // locals.  A value-global captured by any lambda must stay on the host
        // global table (the closure compiles to a separate function); the rest
        // are read only from `main` and can live in a register.
        self.escaping_value_globals =
            crate::free_vars::lambda_captured_globals(&program.forms, &self.value_globals);

        self.infer_main_direct_string_param_types(&program.forms);

        // ── Main pass: lower every form ──────────────────────────────
        let mut main_ctx = FnCtx::new();
        let mut last_main_value: Option<String> = None;

        for form in &program.forms {
            match form {
                Form::Define(def) if matches!(def.expr, Expr::Lambda(_)) => {
                    // (define (f ...) ...) or (define f (lambda ...))
                    let lam = match &def.expr {
                        Expr::Lambda(l) => l,
                        _ => unreachable!("guarded by matches! above"),
                    };
                    self.compile_top_level_lambda(&def.name, lam)?;
                }
                Form::Define(def) => {
                    // (define x value-expr) — evaluate at top level.
                    let loc = SourceLoc::new(def.line, def.column);
                    if let Expr::StrLit(StrLit { value, .. }) = &def.expr {
                        if !self.escaping_value_globals.contains(&def.name)
                            && !self.forced_global_set.contains(&def.name)
                        {
                            let v = main_ctx.emit_str_const(value, loc);
                            self.value_global_locals.insert(def.name.clone(), v);
                            last_main_value = None;
                            continue;
                        }
                    }

                    let v = self.compile_expr(&def.expr, &mut main_ctx)?;

                    // TW2: when the value is statically typed (`i64` / `bool`
                    // / main-only E4 `str`)
                    // and the name is neither captured by a lambda nor already
                    // forward-referenced, keep it in the register `v` and skip
                    // the dynamic `global_set` entirely.  Reads in `main` then
                    // return that register (see `compile_var_ref`), so the whole
                    // of `main` stays typed and clears every backend validator.
                    let ty = main_ctx.type_of(&v);
                    let typed = matches!(ty, "i64" | "bool" | "str");
                    if typed
                        && !self.escaping_value_globals.contains(&def.name)
                        && !self.forced_global_set.contains(&def.name)
                    {
                        self.value_global_locals.insert(def.name.clone(), v);
                    } else {
                        // Captured / dynamically-typed / forward-referenced:
                        // store on the host global table as before.
                        let name_reg = self.string_arg(&mut main_ctx, &def.name, loc);
                        main_ctx.emit(IIRInstr::new(
                            "call_builtin",
                            None,
                            vec![
                                Operand::Var("global_set".into()),
                                Operand::Var(name_reg),
                                Operand::Var(v),
                            ],
                            "void",
                        ), loc);
                    }
                    last_main_value = None;
                }
                Form::Expr(e) => {
                    last_main_value = Some(self.compile_expr(e, &mut main_ctx)?);
                }

                // ── TW05-A / LANG48 typed-syntax forms ────────────────
                //
                // `type_alias`: compile-time only; no IIR emitted.
                Form::TypeAlias(_) => {}

                // `record_def`: erased to constructor + accessor IIR fns.
                Form::RecordDef(rec) => {
                    let rec = rec.clone();
                    self.emit_record_def(&rec)?;
                }

                // `union_def`: erased to integer-tagged constructors + predicates + accessors.
                Form::UnionDef(union) => {
                    let union = union.clone();
                    self.emit_union_def(&union)?;
                }
            }
        }

        // ── Synthesise `main` ────────────────────────────────────────
        if let Some(reg) = last_main_value {
            // Twig path-A increment 1: propagate the source var's inferred
            // type to the `ret` instruction.  `_n1` returned from `42`
            // becomes `ret _n1 [i64]` instead of `ret _n1 [any]`, which
            // every IIR-to-* backend validator now accepts.
            let ret_ty = main_ctx.type_of(&reg).to_string();
            main_ctx.emit(IIRInstr::new(
                "ret",
                None,
                vec![Operand::Var(reg)],
                ret_ty.as_str(),
            ), SourceLoc::SYNTHETIC);
        } else {
            // No final value-producing expression → return nil.
            //
            // Path A increment 6a: emit `const 0 [ref<LispyPair>]`
            // instead of `call_builtin "make_nil" [any]`.  This matches
            // the Phase 2 heap-lowering convention already used by the
            // IIR-to-{wasm,jvm,clr,beam} backends — nil is the null
            // LispyPair reference (represented as 0).  The typed `const`
            // is accepted by every backend, so programs whose only
            // return path is the implicit nil now flow through every
            // backend without going through the `call_builtin` fallback.
            let nil_var = main_ctx.fresh_var("nil");
            main_ctx.emit(IIRInstr::new(
                "const",
                Some(nil_var.clone()),
                vec![Operand::Int(0)],
                "ref<LispyPair>",
            ), SourceLoc::SYNTHETIC);
            main_ctx.record_type(&nil_var, "ref<LispyPair>");
            main_ctx.emit(IIRInstr::new(
                "ret",
                None,
                vec![Operand::Var(nil_var)],
                "ref<LispyPair>",
            ), SourceLoc::SYNTHETIC);
        }

        // Twig path-A increment 1: derive `main`'s `return_type` from
        // the trailing `ret` instruction's `type_hint`.  This lets the
        // IIR-to-* backends emit a typed return (`ret_i64`, `ret_bool`,
        // …) instead of falling back to the untyped fallback path.
        // Functions whose ret source is genuinely dynamic still carry
        // `"any"` and continue to flow through the existing dynamic path.
        let main_return_type = main_ctx
            .instrs
            .iter()
            .rev()
            .find(|i| i.op == "ret")
            .map(|i| i.type_hint.clone())
            .unwrap_or_else(|| "any".to_string());

        let main_fn = IIRFunction {
            name: "main".into(),
            params: vec![],
            return_type: main_return_type,
            register_count: count_registers(&main_ctx.instrs),
            instructions: main_ctx.instrs,
            type_status: FunctionTypeStatus::Untyped,
            call_count: 0,
            feedback_slots: std::collections::HashMap::new(),
            source_map: main_ctx.source_map,
            param_refinements: Vec::new(),
            return_refinement: None,
        };
        self.functions.push(main_fn);

        // ── LANG56: populate exports from module_info ─────────────────────────
        //
        // When the program carries a `(module name (export f1 f2 ...))` clause,
        // register each exported name as an `IIRExport` — but only for names
        // that were actually compiled as top-level functions.  Value-defines,
        // builtins, and undeclared names are silently skipped rather than
        // erroring; the type-checker enforces that exported names exist.
        //
        // Non-root modules (those without a top-level expression body) will
        // have their `entry_point` cleared by `compile_module_tree` in
        // `twig-module-driver`.  We always produce `entry_point = Some("main")`
        // here; the driver overwrites it for library modules.
        let exports: Vec<IIRExport> = program
            .module_info
            .as_ref()
            .map(|mi| {
                mi.exports
                    .iter()
                    .filter(|name| self.fn_globals.contains(*name))
                    .map(IIRExport::new)
                    .collect()
            })
            .unwrap_or_default();

        Ok(IIRModule {
            name: module_name.to_string(),
            functions: self.functions,
            entry_point: Some("main".to_string()),
            language: "twig".to_string(),
            exports,
            imports: Vec::new(),
        })
    }

    fn infer_main_direct_string_param_types(&mut self, forms: &[Form]) {
        let mut evidence: HashMap<String, Vec<StaticParamEvidence>> = HashMap::new();
        let mut known_static_string_values: HashSet<String> = HashSet::new();
        let mut defined_value_globals: HashSet<String> = HashSet::new();
        let mut forward_referenced_values: HashSet<String> = HashSet::new();

        for form in forms {
            if let Form::Define(def) = form {
                if let Expr::Lambda(lam) = &def.expr {
                    evidence.insert(
                        def.name.clone(),
                        vec![StaticParamEvidence::Unknown; lam.params.len()],
                    );
                }
            }
        }

        for form in forms {
            match form {
                Form::Expr(expr) => {
                    self.record_main_forward_value_refs(
                        expr,
                        &defined_value_globals,
                        &mut forward_referenced_values,
                        &HashSet::new(),
                    );
                    self.collect_main_direct_string_call_evidence(
                        expr,
                        &mut evidence,
                        &known_static_string_values,
                    );
                }
                Form::Define(def) if matches!(def.expr, Expr::Lambda(_)) => {
                    known_static_string_values.remove(&def.name);
                }
                Form::Define(def) => {
                    self.record_main_forward_value_refs(
                        &def.expr,
                        &defined_value_globals,
                        &mut forward_referenced_values,
                        &HashSet::new(),
                    );
                    self.collect_main_direct_string_call_evidence(
                        &def.expr,
                        &mut evidence,
                        &known_static_string_values,
                    );
                    if !self.escaping_value_globals.contains(&def.name)
                        && !forward_referenced_values.contains(&def.name)
                        && Self::is_syntax_static_e4_string_expr(
                            &def.expr,
                            &known_static_string_values,
                        )
                    {
                        known_static_string_values.insert(def.name.clone());
                    } else {
                        known_static_string_values.remove(&def.name);
                    }
                    defined_value_globals.insert(def.name.clone());
                }
                _ => {}
            }
        }

        self.fn_inferred_param_types = evidence
            .into_iter()
            .map(|(name, slots)| {
                let types = slots
                    .into_iter()
                    .map(|e| {
                        if e == StaticParamEvidence::Str {
                            "str"
                        } else {
                            "any"
                        }
                        .to_string()
                    })
                    .collect();
                (name, types)
            })
            .collect();
    }

    fn record_main_forward_value_refs(
        &self,
        expr: &Expr,
        defined_value_globals: &HashSet<String>,
        forward_referenced_values: &mut HashSet<String>,
        shadowed: &HashSet<String>,
    ) {
        match expr {
            Expr::VarRef(v) => {
                if self.value_globals.contains(&v.name)
                    && !defined_value_globals.contains(&v.name)
                    && !shadowed.contains(&v.name)
                {
                    forward_referenced_values.insert(v.name.clone());
                }
            }
            Expr::If(i) => {
                self.record_main_forward_value_refs(&i.cond, defined_value_globals, forward_referenced_values, shadowed);
                self.record_main_forward_value_refs(&i.then_branch, defined_value_globals, forward_referenced_values, shadowed);
                self.record_main_forward_value_refs(&i.else_branch, defined_value_globals, forward_referenced_values, shadowed);
            }
            Expr::Begin(Begin { exprs, .. }) => {
                for e in exprs {
                    self.record_main_forward_value_refs(e, defined_value_globals, forward_referenced_values, shadowed);
                }
            }
            Expr::Let(l) => {
                for (_, rhs) in &l.bindings {
                    self.record_main_forward_value_refs(rhs, defined_value_globals, forward_referenced_values, shadowed);
                }
                let mut body_shadowed = shadowed.clone();
                for (name, _) in &l.bindings {
                    body_shadowed.insert(name.clone());
                }
                for e in &l.body {
                    self.record_main_forward_value_refs(e, defined_value_globals, forward_referenced_values, &body_shadowed);
                }
            }
            Expr::LetStar(l) => {
                let mut scoped_shadowed = shadowed.clone();
                for (name, rhs) in &l.bindings {
                    self.record_main_forward_value_refs(rhs, defined_value_globals, forward_referenced_values, &scoped_shadowed);
                    scoped_shadowed.insert(name.clone());
                }
                for e in &l.body {
                    self.record_main_forward_value_refs(e, defined_value_globals, forward_referenced_values, &scoped_shadowed);
                }
            }
            Expr::Apply(apply) => {
                self.record_main_forward_value_refs(apply.fn_expr.as_ref(), defined_value_globals, forward_referenced_values, shadowed);
                for arg in &apply.args {
                    self.record_main_forward_value_refs(arg, defined_value_globals, forward_referenced_values, shadowed);
                }
            }
            Expr::Match(m) => {
                self.record_main_forward_value_refs(&m.scrutinee, defined_value_globals, forward_referenced_values, shadowed);
                for arm in &m.arms {
                    let mut arm_shadowed = shadowed.clone();
                    match &arm.pat {
                        MatchPat::Variant { bindings, .. } => {
                            for name in bindings {
                                arm_shadowed.insert(name.clone());
                            }
                        }
                        MatchPat::Binding(name) => {
                            arm_shadowed.insert(name.clone());
                        }
                        MatchPat::Wildcard => {}
                    }
                    for e in &arm.body {
                        self.record_main_forward_value_refs(e, defined_value_globals, forward_referenced_values, &arm_shadowed);
                    }
                }
            }
            Expr::Lambda(_) => {}
            Expr::IntLit(_)
            | Expr::BoolLit(_)
            | Expr::NilLit(_)
            | Expr::SymLit(_)
            | Expr::StrLit(_) => {}
        }
    }

    fn collect_main_direct_string_call_evidence(
        &self,
        expr: &Expr,
        evidence: &mut HashMap<String, Vec<StaticParamEvidence>>,
        known_static_string_values: &HashSet<String>,
    ) {
        match expr {
            Expr::If(i) => {
                self.collect_main_direct_string_call_evidence(&i.cond, evidence, known_static_string_values);
                self.collect_main_direct_string_call_evidence(&i.then_branch, evidence, known_static_string_values);
                self.collect_main_direct_string_call_evidence(&i.else_branch, evidence, known_static_string_values);
            }
            Expr::Begin(Begin { exprs, .. }) => {
                for e in exprs {
                    self.collect_main_direct_string_call_evidence(e, evidence, known_static_string_values);
                }
            }
            Expr::Let(l) => {
                for (_, rhs) in &l.bindings {
                    self.collect_main_direct_string_call_evidence(rhs, evidence, known_static_string_values);
                }
                let mut body_static_string_values = known_static_string_values.clone();
                for (name, _) in &l.bindings {
                    body_static_string_values.remove(name);
                }
                for (name, rhs) in &l.bindings {
                    if Self::is_syntax_static_e4_string_expr(rhs, known_static_string_values) {
                        body_static_string_values.insert(name.clone());
                    }
                }
                for e in &l.body {
                    self.collect_main_direct_string_call_evidence(e, evidence, &body_static_string_values);
                }
            }
            Expr::LetStar(l) => {
                let mut scoped_static_string_values = known_static_string_values.clone();
                for (name, rhs) in &l.bindings {
                    self.collect_main_direct_string_call_evidence(rhs, evidence, &scoped_static_string_values);
                    if Self::is_syntax_static_e4_string_expr(rhs, &scoped_static_string_values) {
                        scoped_static_string_values.insert(name.clone());
                    } else {
                        scoped_static_string_values.remove(name);
                    }
                }
                for e in &l.body {
                    self.collect_main_direct_string_call_evidence(e, evidence, &scoped_static_string_values);
                }
            }
            Expr::Apply(apply) => {
                if let Expr::VarRef(f) = apply.fn_expr.as_ref() {
                    if let Some(slots) = evidence.get_mut(&f.name) {
                        if apply.args.len() == slots.len() {
                            for (slot, arg) in slots.iter_mut().zip(apply.args.iter()) {
                                *slot = slot.observe(Self::is_syntax_static_e4_string_expr(
                                    arg,
                                    known_static_string_values,
                                ));
                            }
                        } else {
                            for slot in slots {
                                *slot = StaticParamEvidence::Conflict;
                            }
                        }
                    }
                }
                self.collect_main_direct_string_call_evidence(
                    apply.fn_expr.as_ref(),
                    evidence,
                    known_static_string_values,
                );
                for arg in &apply.args {
                    self.collect_main_direct_string_call_evidence(arg, evidence, known_static_string_values);
                }
            }
            // Lambdas and match bodies can depend on dynamic closure/match-time
            // values, so this prepass deliberately does not infer from them.
            Expr::Lambda(_) | Expr::Match(_) => {}
            Expr::IntLit(_)
            | Expr::BoolLit(_)
            | Expr::NilLit(_)
            | Expr::SymLit(_)
            | Expr::StrLit(_)
            | Expr::VarRef(_) => {}
        }
    }

    fn is_syntax_static_e4_string_expr(
        expr: &Expr,
        known_static_string_values: &HashSet<String>,
    ) -> bool {
        match expr {
            Expr::StrLit(_) => true,
            Expr::VarRef(v) => known_static_string_values.contains(&v.name),
            Expr::Apply(apply) => {
                let Expr::VarRef(f) = apply.fn_expr.as_ref() else {
                    return false;
                };
                match f.name.as_str() {
                    "string-append" => {
                        apply.args.len() == 2
                            && Self::is_syntax_static_e4_string_expr(&apply.args[0], known_static_string_values)
                            && Self::is_syntax_static_e4_string_expr(&apply.args[1], known_static_string_values)
                    }
                    "substring" => {
                        apply.args.len() == 3
                            && Self::is_syntax_static_e4_string_expr(&apply.args[0], known_static_string_values)
                            && Self::is_syntax_static_e4_index_expr(&apply.args[1], known_static_string_values)
                            && Self::is_syntax_static_e4_index_expr(&apply.args[2], known_static_string_values)
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    fn is_syntax_static_e4_index_expr(
        expr: &Expr,
        known_static_string_values: &HashSet<String>,
    ) -> bool {
        match expr {
            Expr::IntLit(_) => true,
            Expr::Apply(apply) => {
                let Expr::VarRef(f) = apply.fn_expr.as_ref() else {
                    return false;
                };
                match f.name.as_str() {
                    "string-length" => {
                        apply.args.len() == 1
                            && Self::is_syntax_static_e4_string_expr(&apply.args[0], known_static_string_values)
                    }
                    "+" | "-" | "*" | "/" => {
                        apply.args.len() >= 2
                            && apply
                                .args
                                .iter()
                                .all(|arg| Self::is_syntax_static_e4_index_expr(arg, known_static_string_values))
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    fn param_static_iir_hint(
        &self,
        ann: Option<&TypeAnnotation>,
        inferred_param_types: &[String],
        idx: usize,
    ) -> Option<&'static str> {
        if let Some(ann) = ann {
            return type_annotation_static_iir_hint(ann);
        }
        if inferred_param_types.get(idx).map(|ty| ty.as_str()) == Some("str") {
            Some("str")
        } else {
            None
        }
    }

    // ------------------------------------------------------------------
    // Top-level fn (define (name args...) body+)
    // ------------------------------------------------------------------

    fn compile_top_level_lambda(&mut self, name: &str, lam: &Lambda) -> Result<(), TwigCompileError> {
        let mut ctx = FnCtx::new();
        let lam_loc = SourceLoc::new(lam.line, lam.column);
        let inferred_param_types = self
            .fn_inferred_param_types
            .get(name)
            .cloned()
            .unwrap_or_default();
        for (idx, (p, ann)) in lam
            .params
            .iter()
            .zip(lam.param_annotations.iter())
            .enumerate()
        {
            ctx.locals.insert(p.clone());
            if let Some(ty) = self.param_static_iir_hint(ann.as_ref(), &inferred_param_types, idx)
            {
                ctx.record_type(p, ty);
            }
        }

        let mut last: Option<String> = None;
        for e in &lam.body {
            last = Some(self.compile_expr(e, &mut ctx)?);
        }
        let last = last.ok_or_else(|| TwigCompileError {
            message: format!("function {name:?} has empty body"),
            line: lam.line,
            column: lam.column,
        })?;
        let return_type = ctx.type_of(&last).to_string();
        ctx.emit(
            IIRInstr::new("ret", None, vec![Operand::Var(last)], return_type.as_str()),
            lam_loc,
        );

        let params: Vec<(String, String)> = lam
            .params
            .iter()
            .zip(lam.param_annotations.iter())
            .enumerate()
            .map(|(idx, (p, ann))| {
                let ty = self
                    .param_static_iir_hint(ann.as_ref(), &inferred_param_types, idx)
                    .unwrap_or("any");
                (p.clone(), ty.to_string())
            })
            .collect();

        // LANG23 PR 23-E — lower TypeAnnotation → RefinedType for every param
        // and for the return type.  Unannotated params stay as `None`.
        let param_refinements: Vec<Option<RefinedType>> = lam
            .param_annotations
            .iter()
            .map(|ann| ann.as_ref().map(type_annotation_to_refined_type))
            .collect();

        let return_refinement: Option<RefinedType> = lam
            .return_annotation
            .as_ref()
            .map(type_annotation_to_refined_type);

        self.functions.push(IIRFunction {
            name: name.to_string(),
            params: params.clone(),
            return_type: return_type.clone(),
            register_count: count_registers(&ctx.instrs),
            instructions: ctx.instrs,
            type_status: FunctionTypeStatus::Untyped,
            call_count: 0,
            feedback_slots: std::collections::HashMap::new(),
            source_map: ctx.source_map,
            param_refinements,
            return_refinement,
        });
        self.fn_return_types.insert(name.to_string(), return_type);
        self.fn_param_types.insert(
            name.to_string(),
            params.into_iter().map(|(_, ty)| ty).collect(),
        );
        Ok(())
    }

    // ------------------------------------------------------------------
    // Anonymous lambda — closure construction
    // ------------------------------------------------------------------

    fn compile_anonymous_lambda(
        &mut self,
        lam: &Lambda,
        outer: &mut FnCtx,
    ) -> Result<String, TwigCompileError> {
        let lam_loc = SourceLoc::new(lam.line, lam.column);
        // 1. Compute free variables using the union of all globals + builtins.
        let mut globals: HashSet<String> = HashSet::new();
        globals.extend(self.fn_globals.iter().cloned());
        globals.extend(self.value_globals.iter().cloned());
        for b in BUILTINS {
            globals.insert((*b).to_string());
        }
        let captures = free_vars(lam, &globals);

        // Every capture must be currently bound in `outer`; otherwise the
        // user wrote a name that doesn't resolve to anything reachable
        // from the lambda site.
        for c in &captures {
            if !outer.locals.contains(c) {
                return Err(TwigCompileError {
                    message: format!(
                        "unbound name {c:?} captured by lambda — \
                         did you forget a (define) or a (let ...) binding?"
                    ),
                    line: lam.line,
                    column: lam.column,
                });
            }
        }

        // 2. Build the inner function: captures ++ params for its parameter list.
        let fn_name = format!("__lambda_{}", self.lambda_counter);
        self.lambda_counter += 1;

        let mut inner = FnCtx::new();
        for c in &captures {
            inner.locals.insert(c.clone());
        }
        for p in &lam.params {
            inner.locals.insert(p.clone());
        }

        let mut last: Option<String> = None;
        for e in &lam.body {
            last = Some(self.compile_expr(e, &mut inner)?);
        }
        let last = last.ok_or_else(|| TwigCompileError {
            message: "lambda has empty body".into(),
            line: lam.line,
            column: lam.column,
        })?;
        inner.emit(
            IIRInstr::new("ret", None, vec![Operand::Var(last)], "any"),
            lam_loc,
        );

        let mut params: Vec<(String, String)> =
            captures.iter().map(|c| (c.clone(), "any".to_string())).collect();
        params.extend(lam.params.iter().map(|p| (p.clone(), "any".to_string())));

        self.functions.push(IIRFunction {
            name: fn_name.clone(),
            params,
            return_type: "any".into(),
            register_count: count_registers(&inner.instrs),
            instructions: inner.instrs,
            type_status: FunctionTypeStatus::Untyped,
            call_count: 0,
            feedback_slots: std::collections::HashMap::new(),
            source_map: inner.source_map,
            param_refinements: Vec::new(),
            return_refinement: None,
        });

        // 3. Emit `alloc_closure` at the call site (LANG34).
        //
        // The function name is now an inline `Operand::Str` — no preceding
        // `const` instruction is needed.  This is cleaner than the old
        // `string_arg` / `call_builtin "make_closure"` convention and
        // makes the callee name statically visible in the IR for analysis
        // passes and the iir-to-* backends.
        let dest = outer.fresh_var("clos");
        let mut srcs: Vec<Operand> = vec![Operand::Str(fn_name.clone())];
        for c in &captures {
            srcs.push(Operand::Var(c.clone()));
        }
        outer.emit(
            IIRInstr::new("alloc_closure", Some(dest.clone()), srcs, "closure"),
            lam_loc,
        );
        Ok(dest)
    }

    // ------------------------------------------------------------------
    // Expression compilation
    // ------------------------------------------------------------------

    fn compile_expr(&mut self, expr: &Expr, ctx: &mut FnCtx) -> Result<String, TwigCompileError> {
        // Depth-bound the recursion at the single chokepoint.  Every
        // compound form lowers by recursing through compile_expr on
        // its children, so wrapping it here covers if / let / begin /
        // lambda / apply / quoted-symbol-construction in one place.
        ctx.depth += 1;
        if ctx.depth > MAX_COMPILE_DEPTH {
            let (line, column) = expr.pos();
            return Err(TwigCompileError {
                message: format!(
                    "AST nesting exceeds MAX_COMPILE_DEPTH ({MAX_COMPILE_DEPTH}) — \
                     refusing to recurse further to avoid stack overflow"
                ),
                line,
                column,
            });
        }
        let result = self.compile_expr_inner(expr, ctx);
        ctx.depth = ctx.depth.saturating_sub(1);
        result
    }

    fn compile_expr_inner(&mut self, expr: &Expr, ctx: &mut FnCtx) -> Result<String, TwigCompileError> {
        let (line, column) = expr.pos();
        let loc = SourceLoc::new(line, column);
        match expr {
            Expr::IntLit(IntLit { value, .. }) => {
                let v = ctx.fresh_var("n");
                // Twig path-A increment 1: integer literals lower to typed
                // `const_i64`-compatible IR by stamping `type_hint = "i64"`.
                // The IIR-to-* validators (wasm/jvm/clr/beam) reject `"any"`,
                // so this is the difference between "Twig compiles to a
                // .wasm" and "Twig fails validation at every backend".
                //
                // `var_types[v]` is recorded so downstream `ret` emission
                // can propagate the type instead of falling back to `"any"`.
                ctx.emit(IIRInstr::new(
                    "const",
                    Some(v.clone()),
                    vec![Operand::Int(*value)],
                    "i64",
                ), loc);
                ctx.record_type(&v, "i64");
                Ok(v)
            }

            Expr::BoolLit(BoolLit { value, .. }) => {
                let v = ctx.fresh_var("b");
                // Twig path-A increment 1: boolean literals lower with
                // `type_hint = "bool"` (the IIR-to-* validators all accept
                // it; the JVM/CLR backends map it to `i32` 0/1).
                ctx.emit(IIRInstr::new(
                    "const",
                    Some(v.clone()),
                    vec![Operand::Bool(*value)],
                    "bool",
                ), loc);
                ctx.record_type(&v, "bool");
                Ok(v)
            }

            Expr::NilLit(NilLit { .. }) => {
                // Path A increment 6a: emit `const 0 [ref<LispyPair>]`
                // instead of `call_builtin "make_nil" [any]`.  Every
                // IIR-to-* backend accepts the typed const, and twig-vm
                // dispatches it as a nil `LispyPair` reference.
                let v = ctx.fresh_var("nil");
                ctx.emit(IIRInstr::new(
                    "const",
                    Some(v.clone()),
                    vec![Operand::Int(0)],
                    "ref<LispyPair>",
                ), loc);
                ctx.record_type(&v, "ref<LispyPair>");
                Ok(v)
            }

            Expr::SymLit(SymLit { name, .. }) => {
                // E6d-4 (symbols): a quote literal (`'a` / `(quote a)`) whose name
                // is known at compile time lowers to `const Var(name) : symbol` —
                // the SAME interned-const form McCarthy Lisp's `emit_symbol` emits,
                // rather than the runtime `make_symbol` string path (which needs
                // data-section string emission the code-gen backends lack). This
                // rides the existing `intern_symbols` / `intern_symbols_structural`
                // passes: each distinct name gets one module-wide id, so `equal?`
                // on symbols is bit-equality (`(equal? 'a 'a)` #t, `(equal? 'a 'b)`
                // #f) on all five code-gen backends — with no new value type. On
                // twig-vm the `const Var(name)` dispatch already interns the text to
                // a symbol, so the VM is unaffected. (Runtime symbol *creation* —
                // `string->symbol` over a runtime string — keeps `make_symbol`.)
                let v = ctx.fresh_var("sym");
                ctx.emit(IIRInstr::new(
                    "const",
                    Some(v.clone()),
                    vec![Operand::Var(name.clone())],
                    "symbol",
                ), loc);
                ctx.record_type(&v, "symbol");
                Ok(v)
            }

            // LANG51: double-quoted string literal — `"hello"`.
            //
            // Lower to a single `const(Operand::Str(value))` instruction with
            // type_hint `"str"`.  The VM's `exec_const` (twig-vm/src/dispatch.rs)
            // already handles `Operand::Str` by calling `alloc_string` and
            // wrapping the result as a heap `LispyValue` — no VM changes needed.
            //
            // We use `Operand::Str` (the LANG32-canonical compile-time string form)
            // rather than the older `string_arg` helper that emits `Operand::Var`
            // because that helper was designed for internal builtin-call scaffolding,
            // not for user-visible string data.  `Operand::Str` is cleaner and
            // propagates the `"str"` type_hint automatically through LANG50 inference.
            Expr::StrLit(StrLit { value, .. }) => {
                let v = ctx.fresh_var("s");
                ctx.emit(IIRInstr::new(
                    "const",
                    Some(v.clone()),
                    vec![Operand::Str(value.clone())],
                    "str",
                ), loc);
                ctx.record_type(&v, "str");
                Ok(v)
            }

            Expr::VarRef(v) => self.compile_var_ref(v, ctx),

            Expr::If(i) => self.compile_if(i, ctx),

            Expr::Begin(Begin { exprs, .. }) => {
                // Parser guarantees at least one body expr.
                let mut last: Option<String> = None;
                for e in exprs {
                    last = Some(self.compile_expr(e, ctx)?);
                }
                Ok(last.expect("parser rejects empty (begin)"))
            }

            Expr::Let(l) => self.compile_let(l, ctx),

            // LANG52: sequential let* — compile via compile_let_star.
            Expr::LetStar(l) => self.compile_let_star(l, ctx),

            Expr::Lambda(l) => self.compile_anonymous_lambda(l, ctx),

            Expr::Apply(a) => self.compile_apply(a, ctx),

            // TW05-A / LANG48: match expression — lowered to if/let chain.
            Expr::Match(m) => self.compile_match(m, ctx),
        }
    }

    fn compile_var_ref(&mut self, v: &VarRef, ctx: &mut FnCtx) -> Result<String, TwigCompileError> {
        let loc = SourceLoc::new(v.line, v.column);
        // Locals (params + lets) — return the name directly; the next
        // instruction that reads it resolves through the register file.
        if ctx.locals.contains(&v.name) {
            if let Some(alias) = ctx.local_aliases.get(&v.name) {
                return Ok(alias.clone());
            }
            return Ok(v.name.clone());
        }

        // Top-level function — wrap in a 0-capture closure handle so
        // the value can be passed around or applied later.
        //
        // LANG34: emit `alloc_closure(Str(fn_name))` instead of the old
        // `string_arg + call_builtin "make_closure"` form.  A 0-capture
        // closure is perfectly valid; exec_alloc_closure handles it.
        if self.fn_globals.contains(&v.name) {
            let dest = ctx.fresh_var("fnref");
            ctx.emit(IIRInstr::new(
                "alloc_closure",
                Some(dest.clone()),
                vec![Operand::Str(v.name.clone())],
                "closure",
            ), loc);
            return Ok(dest);
        }

        // TW2: a non-escaping, statically-typed value-global lives in a `main`
        // register — return it directly (the value, with its inferred type, is
        // already live in `main_ctx`).  This is only ever reached from `main`,
        // because escaping names never land in `value_global_locals`.
        if let Some(reg) = self.value_global_locals.get(&v.name) {
            return Ok(reg.clone());
        }

        // Top-level value — look up via the host global table.
        if self.value_globals.contains(&v.name) {
            // A read that reaches here for a value-global that is *not* in
            // `value_global_locals` is either captured (kept on the table by
            // design) or a forward reference whose `define` has not run yet.
            // Flag it so that `define` emits the matching `global_set`.
            self.forced_global_set.insert(v.name.clone());
            let name_reg = self.string_arg(ctx, &v.name, loc);
            let dest = ctx.fresh_var("g");
            ctx.emit(IIRInstr::new(
                "call_builtin",
                Some(dest.clone()),
                vec![Operand::Var("global_get".into()), Operand::Var(name_reg)],
                "any",
            ), loc);
            return Ok(dest);
        }

        // Builtin — wrap in a 0-capture builtin-closure handle so users
        // can pass `+` etc. into higher-order positions.
        if is_builtin(&v.name) {
            let name_reg = self.string_arg(ctx, &v.name, loc);
            let dest = ctx.fresh_var("bref");
            ctx.emit(IIRInstr::new(
                "call_builtin",
                Some(dest.clone()),
                vec![
                    Operand::Var("make_builtin_closure".into()),
                    Operand::Var(name_reg),
                ],
                "any",
            ), loc);
            return Ok(dest);
        }

        Err(TwigCompileError {
            message: format!(
                "unbound name {:?} (no local, define, or builtin matches)",
                v.name
            ),
            line: v.line,
            column: v.column,
        })
    }

    fn compile_if(&mut self, expr: &If, ctx: &mut FnCtx) -> Result<String, TwigCompileError> {
        let loc = SourceLoc::new(expr.line, expr.column);
        let cond = self.compile_expr(&expr.cond, ctx)?;
        let else_label = ctx.fresh_label("else");
        let end_label = ctx.fresh_label("endif");
        let result = ctx.fresh_var("ifv");

        ctx.emit(IIRInstr::new(
            "jmp_if_false",
            None,
            vec![Operand::Var(cond), Operand::Var(else_label.clone())],
            "void",
        ), loc);

        // Then branch — compile and copy into `result` via typed `mov`.
        // Path-A increment 3: replaces the legacy `call_builtin "_move"`
        // with a typed `mov` IR opcode.  If both arms produce the same
        // statically-known type, `result` ends up typed and propagates
        // through downstream `ret` instructions; programs like
        // `(if (< x 0) -1 1)` now flow through every IIR-to-* backend.
        let then_v = self.compile_expr(&expr.then_branch, ctx)?;
        let then_ty = ctx.type_of(&then_v).to_string();
        let then_loc = SourceLoc::new(expr.then_branch.pos().0, expr.then_branch.pos().1);
        ctx.emit_move(&result, &then_v, then_loc);
        ctx.emit(IIRInstr::new(
            "jmp",
            None,
            vec![Operand::Var(end_label.clone())],
            "void",
        ), loc);

        // Else branch — same shape.
        ctx.emit(IIRInstr::new(
            "label",
            None,
            vec![Operand::Var(else_label)],
            "void",
        ), loc);
        let else_v = self.compile_expr(&expr.else_branch, ctx)?;
        let else_ty = ctx.type_of(&else_v).to_string();
        let else_loc = SourceLoc::new(expr.else_branch.pos().0, expr.else_branch.pos().1);
        // `emit_move` for the else arm would overwrite `result`'s
        // recorded type from the then-arm's `emit_move`.  Compute the
        // consensus type up front so the recorded type reflects both
        // arms' agreement (or `"any"` when they disagree).
        ctx.emit(IIRInstr::new(
            "mov",
            Some(result.clone()),
            vec![Operand::Var(else_v)],
            else_ty.as_str(),
        ), else_loc);
        // Consensus: if both arms agree on a concrete type, that's the
        // type of the `if` expression.  Otherwise it's `any`.
        let consensus_ty = if then_ty == else_ty && then_ty != "any" {
            then_ty
        } else {
            "any".to_string()
        };
        if consensus_ty != "any" {
            ctx.record_type(&result, &consensus_ty);
        } else {
            // Clear any record from the then-arm's emit_move so
            // downstream lookups don't return a stale type.
            ctx.var_types.remove(&result);
        }

        ctx.emit(IIRInstr::new(
            "label",
            None,
            vec![Operand::Var(end_label)],
            "void",
        ), loc);
        Ok(result)
    }

    fn compile_let(&mut self, expr: &Let, ctx: &mut FnCtx) -> Result<String, TwigCompileError> {
        let loc = SourceLoc::new(expr.line, expr.column);
        enum BindingValue {
            Reg(String),
            StrLiteral(String, SourceLoc),
        }

        // Compile RHSs in the OUTER scope (Scheme `let`, not `let*`).
        let mut binding_values: Vec<(String, BindingValue)> = Vec::new();
        for (name, rhs) in &expr.bindings {
            let v = if let Expr::StrLit(StrLit { value, .. }) = rhs {
                BindingValue::StrLiteral(value.clone(), SourceLoc::new(rhs.pos().0, rhs.pos().1))
            } else {
                BindingValue::Reg(self.compile_expr(rhs, ctx)?)
            };
            binding_values.push((name.clone(), v));
        }

        // Bind each name into `locals_` so the binding name exists in the
        // lexical frame.
        // Path-A increment 4: typed `mov` propagates the RHS's
        // inferred type to the binding name, so subsequent expressions
        // that reference `name` see the concrete type. E4 string RHSs alias
        // their producer register instead, because the current code-gen
        // backends accept string producers but not generic string copies.
        let mut added: Vec<String> = Vec::new();
        let mut saved_aliases: Vec<(String, Option<String>)> = Vec::new();
        for (name, src) in &binding_values {
            if ctx.locals.insert(name.clone()) {
                added.push(name.clone());
            }
            match src {
                BindingValue::Reg(src) if ctx.type_of(src) == "str" => {
                    let previous = ctx.local_aliases.insert(name.clone(), src.clone());
                    saved_aliases.push((name.clone(), previous));
                    ctx.record_type(name, "str");
                }
                BindingValue::Reg(src) => ctx.emit_move(name, src, loc),
                BindingValue::StrLiteral(value, value_loc) => {
                    ctx.emit_str_const_to(name, value, *value_loc);
                }
            }
        }

        // Compile body — at least one expression (parser-enforced).
        let mut last: Option<String> = None;
        for e in &expr.body {
            last = Some(self.compile_expr(e, ctx)?);
        }
        let last = last.expect("parser rejects empty let body");

        // Pop let names back out so subsequent peers don't see them
        // bound at this lexical position.
        for n in added {
            ctx.locals.remove(&n);
        }
        for (name, previous) in saved_aliases.into_iter().rev() {
            if let Some(alias) = previous {
                ctx.local_aliases.insert(name, alias);
            } else {
                ctx.local_aliases.remove(&name);
            }
        }
        Ok(last)
    }

    /// LANG52: compile `(let* ((x e1) (y e2) ...) body+)`.
    ///
    /// Unlike `compile_let`, each binding's RHS is compiled AFTER the previous
    /// binding is added to locals — so each name is in scope for all subsequent
    /// RHSs.  This is Scheme `let*` semantics.
    ///
    /// ```text
    /// ; (let* ((a 1) (b (+ a 1))) b)
    /// const  %n0  = 1              ; compile a=1 in outer scope
    /// _move  a    ← %n0           ; bind 'a' into locals (now in scope)
    /// call_builtin +, a, 1 → %n1  ; compile b=(+ a 1) — 'a' is visible
    /// _move  b    ← %n1           ; bind 'b' into locals
    /// ret    b
    /// ```
    fn compile_let_star(&mut self, expr: &LetStar, ctx: &mut FnCtx) -> Result<String, TwigCompileError> {
        let loc = SourceLoc::new(expr.line, expr.column);
        let mut added: Vec<String> = Vec::new();
        let mut saved_aliases: Vec<(String, Option<String>)> = Vec::new();

        for (name, rhs) in &expr.bindings {
            // Compile the RHS in the current scope (which already includes
            // all prior let* bindings).
            if let Expr::StrLit(StrLit { value, .. }) = rhs {
                if ctx.locals.insert(name.clone()) {
                    added.push(name.clone());
                }
                ctx.emit_str_const_to(name, value, SourceLoc::new(rhs.pos().0, rhs.pos().1));
                continue;
            }

            let v = self.compile_expr(rhs, ctx)?;

            // Bind the name into locals BEFORE compiling the next binding.
            // Path-A increment 4: typed `mov` propagates scalar RHS types.
            // E4 string RHSs mirror compile_let by aliasing the producer
            // register instead of emitting backend-unsupported `mov [str]`.
            if ctx.locals.insert(name.clone()) {
                added.push(name.clone());
            }
            if ctx.type_of(&v) == "str" {
                let previous = ctx.local_aliases.insert(name.clone(), v);
                saved_aliases.push((name.clone(), previous));
                ctx.record_type(name, "str");
            } else {
                ctx.emit_move(name, &v, loc);
            }
        }

        // Compile body — parser-enforced at least one expression.
        let mut last: Option<String> = None;
        for e in &expr.body {
            last = Some(self.compile_expr(e, ctx)?);
        }
        let last = last.expect("parser rejects empty let* body");

        // Remove bindings so they don't leak into enclosing scope peers.
        for n in added {
            ctx.locals.remove(&n);
        }
        for (name, previous) in saved_aliases.into_iter().rev() {
            if let Some(alias) = previous {
                ctx.local_aliases.insert(name, alias);
            } else {
                ctx.local_aliases.remove(&name);
            }
        }
        Ok(last)
    }

    // ── LANG52: and / or compile-time special forms ───────────────────────────

    /// Compile `(and args…)` with short-circuit semantics.
    ///
    /// Expansion rules (PEG-style, applied left-to-right):
    ///   `(and)`         → emit `const #t`, return the register
    ///   `(and e)`       → compile e, return its register
    ///   `(and e1 e2 …)` → compile e1; if truthy, evaluate `(and e2 …)`; else `#f`
    ///
    /// The result register holds the value of the last evaluated sub-expression,
    /// or `#f` if any sub-expression was falsy.  This matches Scheme semantics.
    ///
    /// IIR pattern (mirrors `compile_if`):
    ///   `jmp_if_false cond, else_label`
    ///   then-path: compile rest, _move into dest, jmp end
    ///   else-path: label else_label; const #f, _move into dest
    ///   label end_label
    fn compile_and(&mut self, args: &[Expr], ctx: &mut FnCtx, loc: SourceLoc) -> Result<String, TwigCompileError> {
        match args {
            [] => {
                // (and) → #t
                let v = ctx.fresh_var("and");
                ctx.emit(IIRInstr::new("const", Some(v.clone()), vec![Operand::Bool(true)], "any"), loc);
                Ok(v)
            }
            [e] => {
                // (and e) → e
                self.compile_expr(e, ctx)
            }
            [first, rest @ ..] => {
                // (and e1 e2 …) → if e1 then (and e2 …) else #f
                let cond = self.compile_expr(first, ctx)?;
                let dest = ctx.fresh_var("and");
                let else_label = ctx.fresh_label("and_else");
                let end_label  = ctx.fresh_label("and_end");

                // jmp_if_false cond → else_label
                ctx.emit(IIRInstr::new("jmp_if_false", None,
                    vec![Operand::Var(cond), Operand::Var(else_label.clone())],
                    "void"), loc);

                // Then path: compile rest, copy to dest via typed `mov`,
                // jump to end.  Path-A increment 4.
                let then_val = self.compile_and(rest, ctx, loc)?;
                ctx.emit_move(&dest, &then_val, loc);
                ctx.emit(IIRInstr::new("jmp", None, vec![Operand::Var(end_label.clone())], "void"), loc);

                // Else path: dest ← #f (typed `mov` from a `const_bool`-
                // typed temporary so the dest carries `"bool"`).
                ctx.emit(IIRInstr::new("label", None, vec![Operand::Var(else_label)], "void"), loc);
                let false_tmp = ctx.fresh_var("f");
                // The false literal site uses the same typed `const`
                // shape that `Expr::BoolLit` emits — see compile_expr
                // path-A increment 1.
                ctx.emit(IIRInstr::new("const", Some(false_tmp.clone()),
                    vec![Operand::Bool(false)], "bool"), loc);
                ctx.record_type(&false_tmp, "bool");
                ctx.emit_move(&dest, &false_tmp, loc);

                ctx.emit(IIRInstr::new("label", None, vec![Operand::Var(end_label)], "void"), loc);
                Ok(dest)
            }
        }
    }

    /// Compile `(or args…)` with short-circuit semantics.
    ///
    /// Expansion rules:
    ///   `(or)`          → emit `const #f`, return the register
    ///   `(or e)`        → compile e, return its register
    ///   `(or e1 e2 …)`  → evaluate e1; if truthy return it, else `(or e2 …)`
    ///
    /// The result register holds the first truthy value, or the value of the
    /// last sub-expression if all were falsy.  This matches Scheme semantics.
    fn compile_or(&mut self, args: &[Expr], ctx: &mut FnCtx, loc: SourceLoc) -> Result<String, TwigCompileError> {
        match args {
            [] => {
                // (or) → #f
                let v = ctx.fresh_var("or");
                ctx.emit(IIRInstr::new("const", Some(v.clone()), vec![Operand::Bool(false)], "any"), loc);
                Ok(v)
            }
            [e] => {
                // (or e) → e
                self.compile_expr(e, ctx)
            }
            [first, rest @ ..] => {
                // (or e1 e2 …): if e1 is truthy return e1, else (or e2 …)
                let cond = self.compile_expr(first, ctx)?;
                let dest = ctx.fresh_var("or");
                let falsy_label = ctx.fresh_label("or_falsy");
                let end_label   = ctx.fresh_label("or_end");

                // jmp_if_false cond → falsy_label (i.e. if cond is FALSE, skip)
                ctx.emit(IIRInstr::new("jmp_if_false", None,
                    vec![Operand::Var(cond.clone()), Operand::Var(falsy_label.clone())],
                    "void"), loc);

                // Truthy path: dest ← cond via typed `mov`, jump to end.
                // Path-A increment 4.
                ctx.emit_move(&dest, &cond, loc);
                ctx.emit(IIRInstr::new("jmp", None, vec![Operand::Var(end_label.clone())], "void"), loc);

                // Falsy path: evaluate rest, copy via typed `mov`.
                ctx.emit(IIRInstr::new("label", None, vec![Operand::Var(falsy_label)], "void"), loc);
                let rest_val = self.compile_or(rest, ctx, loc)?;
                ctx.emit_move(&dest, &rest_val, loc);

                ctx.emit(IIRInstr::new("label", None, vec![Operand::Var(end_label)], "void"), loc);
                Ok(dest)
            }
        }
    }

    fn is_known_main_value_type(&self, name: &str, ctx: &FnCtx, ty: &str) -> bool {
        matches!(self.value_global_locals.get(name), Some(reg) if ctx.type_of(reg) == ty)
    }

    fn is_known_value_type(&self, name: &str, ctx: &FnCtx, ty: &str) -> bool {
        self.is_known_main_value_type(name, ctx, ty)
            || (ctx.locals.contains(name) && ctx.type_of(name) == ty)
    }

    fn can_compile_e4_string_expr(&self, expr: &Expr, ctx: &FnCtx) -> bool {
        match expr {
            Expr::StrLit(_) => true,
            Expr::VarRef(v) => self.is_known_value_type(&v.name, ctx, "str"),
            Expr::Apply(apply) => {
                if let Expr::VarRef(f) = apply.fn_expr.as_ref() {
                    match f.name.as_str() {
                        "string-append" => {
                            apply.args.len() == 2
                                && self.can_compile_e4_string_expr(&apply.args[0], ctx)
                                && self.can_compile_e4_string_expr(&apply.args[1], ctx)
                        }
                        "substring" => {
                            apply.args.len() == 3
                                && self.can_compile_e4_string_expr(&apply.args[0], ctx)
                                && self.can_compile_e4_index_expr(&apply.args[1], ctx)
                                && self.can_compile_e4_index_expr(&apply.args[2], ctx)
                        }
                        _ => false,
                    }
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn can_compile_e4_index_expr(&self, expr: &Expr, ctx: &FnCtx) -> bool {
        match expr {
            Expr::IntLit(_) => true,
            Expr::VarRef(v) => {
                self.is_known_value_type(&v.name, ctx, "i64")
                    || self.is_known_value_type(&v.name, ctx, "i32")
            }
            Expr::Apply(apply) => {
                let Expr::VarRef(f) = apply.fn_expr.as_ref() else {
                    return false;
                };
                match f.name.as_str() {
                    "string-length" => {
                        apply.args.len() == 1
                            && self.can_compile_e4_string_expr(&apply.args[0], ctx)
                    }
                    "+" | "-" | "*" | "/" => {
                        apply.args.len() >= 2
                            && apply
                                .args
                                .iter()
                                .all(|arg| self.can_compile_e4_index_expr(arg, ctx))
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    fn compile_e4_string_expr(&mut self, expr: &Expr, ctx: &mut FnCtx) -> Result<String, TwigCompileError> {
        let loc = SourceLoc::new(expr.pos().0, expr.pos().1);
        if let Expr::StrLit(StrLit { value, .. }) = expr {
            return Ok(ctx.emit_str_const(value, loc));
        }
        self.compile_expr(expr, ctx)
    }

    fn try_compile_e4_string_builtin(
        &mut self,
        name: &str,
        args: &[Expr],
        ctx: &mut FnCtx,
        loc: SourceLoc,
    ) -> Result<Option<String>, TwigCompileError> {
        match name {
            "string-length"
                if args.len() == 1 && self.can_compile_e4_string_expr(&args[0], ctx) =>
            {
                let string_reg = self.compile_e4_string_expr(&args[0], ctx)?;
                let dest = ctx.fresh_var("r");
                ctx.emit(
                    IIRInstr::new(
                        "str_len",
                        Some(dest.clone()),
                        vec![Operand::Var(string_reg)],
                        "i64",
                    ),
                    loc,
                );
                ctx.record_type(&dest, "i64");
                Ok(Some(dest))
            }
            "string-append"
                if args.len() == 2
                    && self.can_compile_e4_string_expr(&args[0], ctx)
                    && self.can_compile_e4_string_expr(&args[1], ctx) =>
            {
                let left = self.compile_e4_string_expr(&args[0], ctx)?;
                let right = self.compile_e4_string_expr(&args[1], ctx)?;
                let dest = ctx.fresh_var("s");
                ctx.emit(
                    IIRInstr::new(
                        "str_concat",
                        Some(dest.clone()),
                        vec![Operand::Var(left), Operand::Var(right)],
                        "str",
                    ),
                    loc,
                );
                ctx.record_type(&dest, "str");
                Ok(Some(dest))
            }
            "string=?"
                if args.len() == 2
                    && self.can_compile_e4_string_expr(&args[0], ctx)
                    && self.can_compile_e4_string_expr(&args[1], ctx) =>
            {
                let left = self.compile_e4_string_expr(&args[0], ctx)?;
                let right = self.compile_e4_string_expr(&args[1], ctx)?;
                let dest = ctx.fresh_var("r");
                ctx.emit(
                    IIRInstr::new(
                        "str_eq",
                        Some(dest.clone()),
                        vec![Operand::Var(left), Operand::Var(right)],
                        "i64",
                    ),
                    loc,
                );
                ctx.record_type(&dest, "i64");
                Ok(Some(dest))
            }
            "string<?" | "string>?"
                if args.len() == 2
                    && self.can_compile_e4_string_expr(&args[0], ctx)
                    && self.can_compile_e4_string_expr(&args[1], ctx) =>
            {
                let left = self.compile_e4_string_expr(&args[0], ctx)?;
                let right = self.compile_e4_string_expr(&args[1], ctx)?;
                let cmp = ctx.fresh_var("r");
                ctx.emit(
                    IIRInstr::new(
                        "str_cmp",
                        Some(cmp.clone()),
                        vec![Operand::Var(left), Operand::Var(right)],
                        "i64",
                    ),
                    loc,
                );
                ctx.record_type(&cmp, "i64");

                let zero = ctx.fresh_var("n");
                ctx.emit(
                    IIRInstr::new("const", Some(zero.clone()), vec![Operand::Int(0)], "i64"),
                    loc,
                );
                ctx.record_type(&zero, "i64");

                let dest = ctx.fresh_var("r");
                let op = if name == "string<?" { "cmp_lt" } else { "cmp_gt" };
                ctx.emit(
                    IIRInstr::new(
                        op,
                        Some(dest.clone()),
                        vec![Operand::Var(cmp), Operand::Var(zero)],
                        "i64",
                    ),
                    loc,
                );
                ctx.record_type(&dest, "bool");
                Ok(Some(dest))
            }
            "string-ref"
                if args.len() == 2
                    && self.can_compile_e4_string_expr(&args[0], ctx)
                    && self.can_compile_e4_index_expr(&args[1], ctx) =>
            {
                let string_reg = self.compile_e4_string_expr(&args[0], ctx)?;
                let idx_reg = self.compile_expr(&args[1], ctx)?;
                let dest = ctx.fresh_var("r");
                ctx.emit(
                    IIRInstr::new(
                        "str_index",
                        Some(dest.clone()),
                        vec![Operand::Var(string_reg), Operand::Var(idx_reg)],
                        "i64",
                    ),
                    loc,
                );
                ctx.record_type(&dest, "i64");
                Ok(Some(dest))
            }
            "substring"
                if args.len() == 3
                    && self.can_compile_e4_string_expr(&args[0], ctx)
                    && self.can_compile_e4_index_expr(&args[1], ctx)
                    && self.can_compile_e4_index_expr(&args[2], ctx) =>
            {
                let string_reg = self.compile_e4_string_expr(&args[0], ctx)?;
                let start_reg = self.compile_expr(&args[1], ctx)?;
                let end_reg = self.compile_expr(&args[2], ctx)?;
                let dest = ctx.fresh_var("s");
                ctx.emit(
                    IIRInstr::new(
                        "str_slice",
                        Some(dest.clone()),
                        vec![
                            Operand::Var(string_reg),
                            Operand::Var(start_reg),
                            Operand::Var(end_reg),
                        ],
                        "str",
                    ),
                    loc,
                );
                ctx.record_type(&dest, "str");
                Ok(Some(dest))
            }
            _ => Ok(None),
        }
    }

    fn compile_apply(&mut self, expr: &Apply, ctx: &mut FnCtx) -> Result<String, TwigCompileError> {
        let loc = SourceLoc::new(expr.line, expr.column);
        // ── LANG52: `and` / `or` short-circuit special forms ─────────────────
        //
        // `and` and `or` require short-circuit evaluation — the second argument
        // must NOT be evaluated when the first is sufficient.  We intercept them
        // at the apply site and lower them inline to `if` chains.  They do NOT
        // appear in BUILTINS and never reach the runtime.
        //
        // | Expression      | Expansion                         |
        // |-----------------|-----------------------------------|
        // | (and)           | #t                                |
        // | (and e)         | e                                 |
        // | (and e1 e2 …)   | (if e1 (and e2 …) #f)            |
        // | (or)            | #f                                |
        // | (or e)          | e                                 |
        // | (or e1 e2 …)    | emit cond-reg; if cond-reg return |
        //
        // The expansions are done recursively by re-entering compile_apply
        // so the depth counter naturally bounds them.
        if let Expr::VarRef(v) = expr.fn_expr.as_ref() {
            if v.name == "and" {
                return self.compile_and(&expr.args, ctx, loc);
            }
            if v.name == "or" {
                return self.compile_or(&expr.args, ctx, loc);
            }
        }

        // Direct call: fn is a VarRef whose name is a top-level
        // function or a builtin.  We materialise this decision at
        // compile time so the hot path stays a single `call`.
        if let Expr::VarRef(v) = expr.fn_expr.as_ref() {
            if self.fn_globals.contains(&v.name) {
                let return_type = self
                    .fn_return_types
                    .get(&v.name)
                    .cloned()
                    .unwrap_or_else(|| "any".to_string());
                let param_types = self
                    .fn_param_types
                    .get(&v.name)
                    .cloned()
                    .unwrap_or_default();
                let mut srcs: Vec<Operand> = vec![Operand::Var(v.name.clone())];
                for (idx, a) in expr.args.iter().enumerate() {
                    let r = if param_types.get(idx).map(|ty| ty.as_str()) == Some("str")
                        && self.can_compile_e4_string_expr(a, ctx)
                    {
                        self.compile_e4_string_expr(a, ctx)?
                    } else {
                        self.compile_expr(a, ctx)?
                    };
                    srcs.push(Operand::Var(r));
                }
                let dest = ctx.fresh_var("r");
                ctx.emit(IIRInstr::new(
                    "call",
                    Some(dest.clone()),
                    srcs,
                    return_type.as_str(),
                ), loc);
                if return_type != "any" {
                    ctx.record_type(&dest, return_type.as_str());
                }
                return Ok(dest);
            }

            if is_builtin(&v.name) {
                if let Some(result) =
                    self.try_compile_e4_string_builtin(&v.name, &expr.args, ctx, loc)?
                {
                    return Ok(result);
                }

                if v.name == "string-length" && expr.args.len() == 1 {
                    if let Expr::StrLit(StrLit { value, .. }) = &expr.args[0] {
                        let string_reg = ctx.fresh_var("s");
                        ctx.emit(IIRInstr::new(
                            "str_const",
                            Some(string_reg.clone()),
                            vec![Operand::Str(value.clone())],
                            "str",
                        ), loc);
                        ctx.var_types.insert(string_reg.clone(), "str".to_string());

                        let dest = ctx.fresh_var("r");
                        ctx.emit(IIRInstr::new(
                            "str_len",
                            Some(dest.clone()),
                            vec![Operand::Var(string_reg)],
                            "i64",
                        ), loc);
                        ctx.var_types.insert(dest.clone(), "i64".to_string());
                        return Ok(dest);
                    }
                    if let Expr::Apply(append) = &expr.args[0] {
                        if let Expr::VarRef(append_fn) = append.fn_expr.as_ref() {
                            if append_fn.name == "string-append" && append.args.len() == 2 {
                                if let (
                                    Expr::StrLit(StrLit { value: left, .. }),
                                    Expr::StrLit(StrLit { value: right, .. }),
                                ) = (&append.args[0], &append.args[1])
                                {
                                    let left_reg = ctx.fresh_var("s");
                                    ctx.emit(IIRInstr::new(
                                        "str_const",
                                        Some(left_reg.clone()),
                                        vec![Operand::Str(left.clone())],
                                        "str",
                                    ), loc);
                                    ctx.var_types.insert(left_reg.clone(), "str".to_string());

                                    let right_reg = ctx.fresh_var("s");
                                    ctx.emit(IIRInstr::new(
                                        "str_const",
                                        Some(right_reg.clone()),
                                        vec![Operand::Str(right.clone())],
                                        "str",
                                    ), loc);
                                    ctx.var_types.insert(right_reg.clone(), "str".to_string());

                                    let concat_reg = ctx.fresh_var("s");
                                    ctx.emit(IIRInstr::new(
                                        "str_concat",
                                        Some(concat_reg.clone()),
                                        vec![
                                            Operand::Var(left_reg),
                                            Operand::Var(right_reg),
                                        ],
                                        "str",
                                    ), loc);
                                    ctx.var_types.insert(concat_reg.clone(), "str".to_string());

                                    let dest = ctx.fresh_var("r");
                                    ctx.emit(IIRInstr::new(
                                        "str_len",
                                        Some(dest.clone()),
                                        vec![Operand::Var(concat_reg)],
                                        "i64",
                                    ), loc);
                                    ctx.var_types.insert(dest.clone(), "i64".to_string());
                                    return Ok(dest);
                                }
                            }
                        }
                    }
                }
                if v.name == "string=?" && expr.args.len() == 2 {
                    if let (
                        Expr::StrLit(StrLit { value: left, .. }),
                        Expr::StrLit(StrLit { value: right, .. }),
                    ) = (&expr.args[0], &expr.args[1])
                    {
                        let left_reg = ctx.fresh_var("s");
                        ctx.emit(IIRInstr::new(
                            "str_const",
                            Some(left_reg.clone()),
                            vec![Operand::Str(left.clone())],
                            "str",
                        ), loc);
                        ctx.var_types.insert(left_reg.clone(), "str".to_string());

                        let right_reg = ctx.fresh_var("s");
                        ctx.emit(IIRInstr::new(
                            "str_const",
                            Some(right_reg.clone()),
                            vec![Operand::Str(right.clone())],
                            "str",
                        ), loc);
                        ctx.var_types.insert(right_reg.clone(), "str".to_string());

                        let dest = ctx.fresh_var("r");
                        ctx.emit(IIRInstr::new(
                            "str_eq",
                            Some(dest.clone()),
                            vec![Operand::Var(left_reg), Operand::Var(right_reg)],
                            "i64",
                        ), loc);
                        ctx.var_types.insert(dest.clone(), "i64".to_string());
                        return Ok(dest);
                    }
                }
                if v.name == "string-ref" && expr.args.len() == 2 {
                    if let (
                        Expr::StrLit(StrLit { value, .. }),
                        Expr::IntLit(IntLit { value: idx, .. }),
                    ) = (&expr.args[0], &expr.args[1])
                    {
                        let string_reg = ctx.fresh_var("s");
                        ctx.emit(IIRInstr::new(
                            "str_const",
                            Some(string_reg.clone()),
                            vec![Operand::Str(value.clone())],
                            "str",
                        ), loc);
                        ctx.var_types.insert(string_reg.clone(), "str".to_string());

                        let idx_reg = ctx.fresh_var("i");
                        ctx.emit(IIRInstr::new(
                            "const",
                            Some(idx_reg.clone()),
                            vec![Operand::Int(*idx)],
                            "i64",
                        ), loc);
                        ctx.var_types.insert(idx_reg.clone(), "i64".to_string());

                        let dest = ctx.fresh_var("r");
                        ctx.emit(IIRInstr::new(
                            "str_index",
                            Some(dest.clone()),
                            vec![Operand::Var(string_reg), Operand::Var(idx_reg)],
                            "i64",
                        ), loc);
                        ctx.var_types.insert(dest.clone(), "i64".to_string());
                        return Ok(dest);
                    }
                }

                // Resolve every argument before deciding the lowering path.
                let arg_regs: Vec<String> = expr
                    .args
                    .iter()
                    .map(|a| self.compile_expr(a, ctx))
                    .collect::<Result<_, _>>()?;

                // ── Twig path-A increment 2: typed binary arithmetic / cmp ───
                //
                // When every resolved argument has a statically-known type
                // (recorded in `FnCtx::var_types` by literal-emission sites
                // and by previous typed builds-up such as earlier `add`
                // operations), and the builtin name is in the arithmetic /
                // comparison set, we can lower the call to a *typed CIR
                // mnemonic* (`add`, `cmp_lt`, …) that the IIR-to-* backends
                // accept directly — instead of the legacy
                // `call_builtin "<op>"` path which carries `type_hint = "any"`
                // and gets rejected at every backend validator.
                //
                // The pattern mirrors PR #3903's fix for Nib
                // (`compile_binary_chain` → typed CIR mnemonics).  See
                // `typed_arith_op_for` below for the symbol → mnemonic table.
                //
                // Scoped to binary forms (`(+ a b)`).  Variadic forms
                // (`(+ a b c)`), and any call where one arg has an unknown /
                // dynamic type, continue to use the `call_builtin "any"`
                // fallback — they remain rejected by the backend validators
                // until a later increment lowers variadic folds and / or
                // adds runtime type guards.
                if arg_regs.len() == 2 {
                    if let Some(typed_mnemonic) = typed_arith_op_for(&v.name) {
                        let lhs_ty = ctx.type_of(&arg_regs[0]).to_string();
                        let rhs_ty = ctx.type_of(&arg_regs[1]).to_string();
                        if lhs_ty == "i64" && rhs_ty == "i64" {
                            // Emit `<typed_mnemonic> dest = lhs, rhs [i64]`.
                            // Result type depends on the family:
                            //   add/sub/mul/div  → i64
                            //   cmp_* / =        → bool
                            let result_ty: &str =
                                if typed_mnemonic.starts_with("cmp_") {
                                    "bool"
                                } else {
                                    "i64"
                                };
                            let dest = ctx.fresh_var("r");
                            ctx.emit(IIRInstr::new(
                                typed_mnemonic,
                                Some(dest.clone()),
                                vec![
                                    Operand::Var(arg_regs[0].clone()),
                                    Operand::Var(arg_regs[1].clone()),
                                ],
                                result_ty,
                            ), loc);
                            ctx.record_type(&dest, result_ty);
                            return Ok(dest);
                        }
                    }
                }

                // ── Twig path-A increment 3 (TW1): typed *variadic* arithmetic ─
                //
                // Scheme arithmetic is variadic: `(+ a b c d)` means
                // `a + b + c + d`.  When every argument is statically `i64`
                // and the builtin is one of the four *arithmetic* operators,
                // we fold the call into a **left-associated chain of typed
                // binary CIR mnemonics** — the exact shape the `(+ a b)` case
                // above emits, repeated:
                //
                //     (+ a b c)   →   r1 = add a, b   [i64]
                //                     r2 = add r1, c  [i64]   ⇒ result r2
                //
                // Each step is a typed `add`/`sub`/`mul`/`div` the IIR-to-*
                // backends accept directly, so a variadic arithmetic call now
                // clears every backend validator instead of falling back to the
                // `call_builtin "<op>"` path (`type_hint = "any"`), which they
                // reject.  Comparisons are deliberately excluded: variadic
                // `(< a b c)` is a chained predicate (`a<b ∧ b<c`), not a fold,
                // so it stays on the dynamic path until a dedicated increment.
                // Unary / nullary forms (`(+)`, `(- a)`) likewise stay on the
                // fallback; this increment targets the `n ≥ 2` fold (the
                // `n == 2` arithmetic case is already handled above, so this
                // block effectively lights up `n ≥ 3`).
                if arg_regs.len() >= 2 {
                    if let Some(typed_mnemonic) = typed_arith_op_for(&v.name) {
                        if !typed_mnemonic.starts_with("cmp_")
                            && arg_regs.iter().all(|r| ctx.type_of(r) == "i64")
                        {
                            let mut acc = arg_regs[0].clone();
                            for rhs in &arg_regs[1..] {
                                let dest = ctx.fresh_var("r");
                                ctx.emit(IIRInstr::new(
                                    typed_mnemonic,
                                    Some(dest.clone()),
                                    vec![Operand::Var(acc), Operand::Var(rhs.clone())],
                                    "i64",
                                ), loc);
                                ctx.record_type(&dest, "i64");
                                acc = dest;
                            }
                            return Ok(acc);
                        }
                    }
                }

                // Fallback: legacy dynamic `call_builtin` path.
                let mut srcs: Vec<Operand> =
                    vec![Operand::Var(v.name.clone())];
                for r in arg_regs {
                    srcs.push(Operand::Var(r));
                }
                let dest = ctx.fresh_var("r");
                ctx.emit(IIRInstr::new(
                    "call_builtin",
                    Some(dest.clone()),
                    srcs,
                    "any",
                ), loc);
                return Ok(dest);
            }
        }

        // Indirect: compile the fn expression to a closure handle, then
        // invoke via `call_closure` (LANG34).
        //
        // Before LANG34 this emitted `call_builtin "apply_closure" handle args…`.
        // Now: the handle is srcs[0] directly — no leading name-string operand.
        let fn_handle = self.compile_expr(&expr.fn_expr, ctx)?;
        let mut srcs: Vec<Operand> = vec![Operand::Var(fn_handle)];
        for a in &expr.args {
            let r = self.compile_expr(a, ctx)?;
            srcs.push(Operand::Var(r));
        }
        let dest = ctx.fresh_var("r");
        ctx.emit(IIRInstr::new(
            "call_closure",
            Some(dest.clone()),
            srcs,
            "any",
        ), loc);
        Ok(dest)
    }

    // ------------------------------------------------------------------
    // TW05-A / LANG48 — match lowering
    // ------------------------------------------------------------------

    /// Lower a `(match scrutinee arm+)` expression to an if/let chain.
    ///
    /// Evaluation strategy:
    /// 1. Evaluate the scrutinee exactly once into `#matched_N`.
    /// 2. For each arm in order:
    ///    - **Variant arm** `(VarName b1 … bn) body+`:
    ///      Emit an `if` that tests `(= (car #matched_N) tag)`.
    ///      On true: bind fields via `(car (cdr …))` chains and evaluate body.
    ///    - **Binding arm** `name body+`:
    ///      Emit a `let`-style binding of `#matched_N` to `name`, evaluate body.
    ///    - **Wildcard arm** `_ body+`:
    ///      Evaluate body directly with no extra binding.
    /// 3. Fallthrough (no arm matched) → `nil`.
    ///
    /// Arms are chained in the else-branch of each if.  The innermost else
    /// produces a `make_nil` call so behaviour is deterministic when no arm
    /// matches (forward-compatible with exhaustiveness checking in TW05-B).
    fn compile_match(&mut self, m: &Match, ctx: &mut FnCtx) -> Result<String, TwigCompileError> {
        let loc = SourceLoc::new(m.line, m.column);

        // Evaluate the scrutinee once.
        // Path-A increment 5: convert all `call_builtin "_move"` sites
        // in compile_match to typed `mov` via `FnCtx::emit_move`.  The
        // `match` result's type is left as `"any"` for now — match
        // arms can produce mixed types (variant constructors vs. raw
        // values), so a consensus pass here is non-trivial.  Mechanical
        // conversion of the move shape alone is enough to unblock the
        // backend validators (which reject `call_builtin "_move"` but
        // accept untyped `mov [any]`).
        let scrutinee_reg = self.compile_expr(&m.scrutinee, ctx)?;
        // Bind to a fresh stable register so arms can reference it freely.
        let matched = ctx.fresh_var("matched");
        ctx.emit_move(&matched, &scrutinee_reg, loc);

        // Result register — each arm writes its value here.
        let result = ctx.fresh_var("match_result");
        // Initialise to nil (fallthrough value).
        //
        // Path A increment 6a: emit typed `const 0 [ref<LispyPair>]`
        // instead of `call_builtin "make_nil" [any]`.  emit_move then
        // propagates the `ref<LispyPair>` type onto `result`, so the
        // match expression's fallthrough is also typed.
        let nil_init = ctx.fresh_var("nil");
        ctx.emit(IIRInstr::new(
            "const",
            Some(nil_init.clone()),
            vec![Operand::Int(0)],
            "ref<LispyPair>",
        ), loc);
        ctx.record_type(&nil_init, "ref<LispyPair>");
        ctx.emit_move(&result, &nil_init, loc);

        // Generate labels for the chain.
        // We build: if (test0) { arm0 } else { if (test1) { arm1 } else { … } }
        // via explicit label/jmp/jmpif instructions.
        let end_label = ctx.fresh_label("match_end");

        for arm in &m.arms {
            let arm_loc = if arm.body.is_empty() { loc } else {
                let p = arm.body[0].pos();
                SourceLoc::new(p.0, p.1)
            };
            match &arm.pat {
                MatchPat::Variant { name, bindings } => {
                    // Look up the integer tag.  Unknown names are treated as
                    // binding patterns (forward-compatible with future types).
                    let tag = match self.variant_tags.get(name) {
                        Some(&t) => t,
                        None => {
                            // Unknown constructor — treat as bare binding.
                            let arm_result = self.compile_match_binding_arm(
                                &matched, name, &arm.body, ctx, arm_loc)?;
                            ctx.emit_move(&result, &arm_result, arm_loc);
                            ctx.emit(IIRInstr::new(
                                "jmp",
                                None,
                                vec![Operand::Var(end_label.clone())],
                                "void",
                            ), arm_loc);
                            continue;
                        }
                    };

                    // Emit: tag_reg = (car matched)
                    //
                    // Path A increment 6c: typed `field_load[0]` instead of
                    // `call_builtin "car"`.  The Phase 2 heap-lowering
                    // convention is `field_load [ref<any>]` (the loaded
                    // field can hold any Lisp value).
                    let tag_reg = ctx.fresh_var("tag");
                    ctx.emit(IIRInstr::new(
                        "field_load",
                        Some(tag_reg.clone()),
                        vec![Operand::Var(matched.clone()), Operand::Int(0)],
                        "ref<any>",
                    ), arm_loc);
                    ctx.record_type(&tag_reg, "ref<any>");

                    // Emit: tag_int = integer constant for this variant
                    let tag_int_reg = ctx.fresh_var("tag_val");
                    ctx.emit(IIRInstr::new(
                        "const",
                        Some(tag_int_reg.clone()),
                        vec![Operand::Int(tag as i64)],
                        "any",
                    ), arm_loc);

                    // Emit: cond = (= tag_reg tag_int)
                    let cond_reg = ctx.fresh_var("tag_eq");
                    ctx.emit(IIRInstr::new(
                        "call_builtin",
                        Some(cond_reg.clone()),
                        vec![
                            Operand::Var("=".into()),
                            Operand::Var(tag_reg),
                            Operand::Var(tag_int_reg),
                        ],
                        "any",
                    ), arm_loc);

                    // jmp_if_false cond → skip_label
                    //
                    // If the tag comparison is false (tag ≠ expected variant),
                    // jump over the arm body to the next arm.  When the condition
                    // is true the arm body falls through immediately.
                    //
                    // This mirrors `compile_if` (same two-operand jmp_if_false
                    // pattern) and is the correct IIR opcode recognised by the VM.
                    // The old three-operand `jmpif` was never added to the VM
                    // dispatch and produced UnsupportedOpcode at runtime (LANG57).
                    let skip_label = ctx.fresh_label("match_skip");
                    ctx.emit(IIRInstr::new(
                        "jmp_if_false",
                        None,
                        vec![
                            Operand::Var(cond_reg),
                            Operand::Var(skip_label.clone()),
                        ],
                        "void",
                    ), arm_loc);
                    // arm body follows immediately (fall-through when cond is true)

                    // Bind fields: field_i = (car (cdr^(i+1) matched))
                    //
                    // Path A increment 6c: typed `field_load[1]` (cdr) and
                    // `field_load[0]` (car) instead of `call_builtin "cdr"`
                    // and `call_builtin "car"`.  Phase 2 convention.
                    let mut added_names: Vec<String> = Vec::new();
                    let mut cur_cdr = matched.clone();
                    for binding in bindings {
                        // Advance one cdr step
                        let next_cdr = ctx.fresh_var("cdr");
                        ctx.emit(IIRInstr::new(
                            "field_load",
                            Some(next_cdr.clone()),
                            vec![Operand::Var(cur_cdr.clone()), Operand::Int(1)],
                            "ref<any>",
                        ), arm_loc);
                        ctx.record_type(&next_cdr, "ref<any>");
                        cur_cdr = next_cdr;

                        // Extract field: car of the cdr chain
                        let field_reg = ctx.fresh_var("field");
                        ctx.emit(IIRInstr::new(
                            "field_load",
                            Some(field_reg.clone()),
                            vec![Operand::Var(cur_cdr.clone()), Operand::Int(0)],
                            "ref<any>",
                        ), arm_loc);
                        ctx.record_type(&field_reg, "ref<any>");

                        // Bind field to name in ctx.locals via typed `mov`.
                        if ctx.locals.insert(binding.clone()) {
                            added_names.push(binding.clone());
                        }
                        ctx.emit_move(binding, &field_reg, arm_loc);
                    }

                    // Evaluate body
                    let mut body_result: Option<String> = None;
                    for e in &arm.body {
                        body_result = Some(self.compile_expr(e, ctx)?);
                    }
                    let body_v = body_result.expect("arm body non-empty (parser-enforced)");

                    // Copy to result register via typed `mov`.
                    ctx.emit_move(&result, &body_v, arm_loc);

                    // Unbind field names
                    for n in added_names {
                        ctx.locals.remove(&n);
                    }

                    // jmp to end
                    ctx.emit(IIRInstr::new(
                        "jmp",
                        None,
                        vec![Operand::Var(end_label.clone())],
                        "void",
                    ), arm_loc);

                    // skip_label: next arm or fallthrough
                    ctx.emit(IIRInstr::new(
                        "label",
                        None,
                        vec![Operand::Var(skip_label)],
                        "void",
                    ), arm_loc);
                }

                MatchPat::Binding(name) => {
                    let arm_result = self.compile_match_binding_arm(
                        &matched, name, &arm.body, ctx, arm_loc)?;
                    ctx.emit_move(&result, &arm_result, arm_loc);
                    // Binding arm always matches → jump to end immediately.
                    ctx.emit(IIRInstr::new(
                        "jmp",
                        None,
                        vec![Operand::Var(end_label.clone())],
                        "void",
                    ), arm_loc);
                }

                MatchPat::Wildcard => {
                    // No binding; evaluate body directly.
                    let mut body_result: Option<String> = None;
                    for e in &arm.body {
                        body_result = Some(self.compile_expr(e, ctx)?);
                    }
                    let body_v = body_result.expect("arm body non-empty");
                    ctx.emit_move(&result, &body_v, arm_loc);
                    // Wildcard always matches → jump to end.
                    ctx.emit(IIRInstr::new(
                        "jmp",
                        None,
                        vec![Operand::Var(end_label.clone())],
                        "void",
                    ), arm_loc);
                }
            }
        }

        // end_label: all paths converge here.
        ctx.emit(IIRInstr::new(
            "label",
            None,
            vec![Operand::Var(end_label)],
            "void",
        ), loc);

        Ok(result)
    }

    /// Helper: bind `matched_reg` to `name` in locals, evaluate `body`,
    /// then remove the binding.  Returns the body's result register.
    fn compile_match_binding_arm(
        &mut self,
        matched_reg: &str,
        name: &str,
        body: &[twig_parser::Expr],
        ctx: &mut FnCtx,
        loc: SourceLoc,
    ) -> Result<String, TwigCompileError> {
        let was_new = ctx.locals.insert(name.to_string());
        // Path-A increment 5: typed `mov` for the binding-arm helper.
        ctx.emit_move(name, matched_reg, loc);
        let mut last: Option<String> = None;
        for e in body {
            last = Some(self.compile_expr(e, ctx)?);
        }
        let last = last.expect("arm body non-empty (parser-enforced)");
        if was_new {
            ctx.locals.remove(name);
        }
        Ok(last)
    }

    // ------------------------------------------------------------------
    // TW05-A / LANG48 — record and union erasure
    // ------------------------------------------------------------------

    /// Erase a `(record Name field*)` declaration into IIR functions:
    ///
    /// - Constructor `Name(f0, f1, …, fn)` → `cons` chain ending in `nil`
    /// - Accessor `<lower(Name)>-<field>(r)` → `car` of the right `cdr` chain
    /// - Predicate `<lower(Name)>?(v)` → `(pair? v)`
    fn emit_record_def(&mut self, rec: &RecordDef) -> Result<(), TwigCompileError> {
        let loc = SourceLoc::SYNTHETIC;
        let prefix = rec.name.to_lowercase();

        // ── Constructor: Name(f0, f1, …, fn) ──────────────────────────
        // Builds (cons f0 (cons f1 (… (cons fn nil) …))).
        {
            let mut ctx = FnCtx::new();
            let params: Vec<String> = rec.fields.iter().map(|f| f.name.clone()).collect();
            for p in &params {
                ctx.locals.insert(p.clone());
            }

            // Start from the nil end and fold right.
            //
            // Path A increment 6a + 6b: typed `const 0 [ref<LispyPair>]`
            // initial tail, and each cons cell lowers to typed
            // `alloc [ref<LispyPair>]` + 2× `field_store [void]`
            // (Phase 2 heap-lowering convention).
            let nil_r = ctx.fresh_var("nil");
            ctx.emit(IIRInstr::new(
                "const",
                Some(nil_r.clone()),
                vec![Operand::Int(0)],
                "ref<LispyPair>",
            ), loc);
            ctx.record_type(&nil_r, "ref<LispyPair>");

            // Path A increment 6b: emit typed `alloc` + 2× `field_store`
            // instead of `call_builtin "cons" [any]`.  The Phase 2 heap-
            // lowering convention is:
            //   alloc cell [ref<LispyPair>]
            //   field_store cell, 0, head [void]    -- car
            //   field_store cell, 1, tail [void]    -- cdr
            // Every IIR-to-* backend accepts this triple for ref<LispyPair>.
            let mut tail = nil_r;
            for field_name in params.iter().rev() {
                let cell = ctx.fresh_var("cell");
                let mut alloc = IIRInstr::new(
                    "alloc",
                    Some(cell.clone()),
                    vec![],
                    "ref<LispyPair>",
                );
                alloc.may_alloc = true;
                ctx.emit(alloc, loc);
                ctx.record_type(&cell, "ref<LispyPair>");
                // field_store cell, 0, head — car
                ctx.emit(IIRInstr::new(
                    "field_store",
                    None,
                    vec![
                        Operand::Var(cell.clone()),
                        Operand::Int(0),
                        Operand::Var(field_name.clone()),
                    ],
                    // type_hint "void" matches iir-builtin-lowering's
                    // Phase 2 convention and the BEAM validator.  WASM,
                    // JVM, CLR accept it via their GC-op handlers.
                    "void",
                ), loc);
                // field_store cell, 1, tail — cdr
                ctx.emit(IIRInstr::new(
                    "field_store",
                    None,
                    vec![
                        Operand::Var(cell.clone()),
                        Operand::Int(1),
                        Operand::Var(tail),
                    ],
                    "void",
                ), loc);
                tail = cell;
            }
            ctx.emit(IIRInstr::new("ret", None, vec![Operand::Var(tail)], "ref<LispyPair>"), loc);

            self.functions.push(IIRFunction {
                name: rec.name.clone(),
                params: params.iter().map(|p| (p.clone(), "any".to_string())).collect(),
                return_type: "any".into(),
                register_count: count_registers(&ctx.instrs),
                instructions: ctx.instrs,
                type_status: FunctionTypeStatus::Untyped,
                call_count: 0,
                feedback_slots: HashMap::new(),
                source_map: ctx.source_map,
                param_refinements: Vec::new(),
                return_refinement: None,
            });
        }

        // ── Accessors: <prefix>-<field>(r) → car(cdr^i(r)) ───────────
        //
        // Path A increment 6c: typed `field_load[0]` / `field_load[1]`
        // instead of `call_builtin "car"` / `"cdr"`.  Phase 2 convention.
        for (i, field) in rec.fields.iter().enumerate() {
            let mut ctx = FnCtx::new();
            ctx.locals.insert("r".to_string());

            // Build the cdr chain: apply `cdr` i times.
            let mut cur = "r".to_string();
            for _ in 0..i {
                let next = ctx.fresh_var("cdr");
                ctx.emit(IIRInstr::new(
                    "field_load",
                    Some(next.clone()),
                    vec![Operand::Var(cur), Operand::Int(1)],
                    "ref<any>",
                ), loc);
                ctx.record_type(&next, "ref<any>");
                cur = next;
            }
            // Then take car.
            let field_val = ctx.fresh_var("fv");
            ctx.emit(IIRInstr::new(
                "field_load",
                Some(field_val.clone()),
                vec![Operand::Var(cur), Operand::Int(0)],
                "ref<any>",
            ), loc);
            ctx.record_type(&field_val, "ref<any>");
            ctx.emit(IIRInstr::new("ret", None, vec![Operand::Var(field_val)], "ref<any>"), loc);

            self.functions.push(IIRFunction {
                name: format!("{prefix}-{}", field.name),
                params: vec![("r".to_string(), "any".to_string())],
                return_type: "any".into(),
                register_count: count_registers(&ctx.instrs),
                instructions: ctx.instrs,
                type_status: FunctionTypeStatus::Untyped,
                call_count: 0,
                feedback_slots: HashMap::new(),
                source_map: ctx.source_map,
                param_refinements: Vec::new(),
                return_refinement: None,
            });
        }

        // ── Predicate: <prefix>?(v) → (pair? v) ───────────────────────
        {
            let mut ctx = FnCtx::new();
            ctx.locals.insert("v".to_string());
            let pred = ctx.fresh_var("pred");
            ctx.emit(IIRInstr::new(
                "call_builtin",
                Some(pred.clone()),
                vec![Operand::Var("pair?".into()), Operand::Var("v".to_string())],
                "any",
            ), loc);
            ctx.emit(IIRInstr::new("ret", None, vec![Operand::Var(pred)], "any"), loc);

            self.functions.push(IIRFunction {
                name: format!("{prefix}?"),
                params: vec![("v".to_string(), "any".to_string())],
                return_type: "any".into(),
                register_count: count_registers(&ctx.instrs),
                instructions: ctx.instrs,
                type_status: FunctionTypeStatus::Untyped,
                call_count: 0,
                feedback_slots: HashMap::new(),
                source_map: ctx.source_map,
                param_refinements: Vec::new(),
                return_refinement: None,
            });
        }
        Ok(())
    }

    /// Erase a `(union Name variant*)` declaration into IIR functions.
    ///
    /// Each variant at zero-based index `i` produces:
    /// - Constructor `VarName(f0, …, fn)` → `(cons i (cons f0 (… (cons fn nil) …)))`
    /// - Predicate `VarName?(v)` → `(= (car v) i)`
    /// - Accessor `<lower(VarName)>-<field>(v)` → `car(cdr^(k+1) v)` (k = field index)
    fn emit_union_def(&mut self, union: &UnionDef) -> Result<(), TwigCompileError> {
        let loc = SourceLoc::SYNTHETIC;

        for (tag, variant) in union.variants.iter().enumerate() {
            let vprefix = variant.name.to_lowercase();

            // ── Constructor: VarName(f0, …, fn) ───────────────────────
            // → (cons tag (cons f0 (cons f1 (… (cons fn nil) …))))
            {
                let mut ctx = FnCtx::new();
                let params: Vec<String> = variant.fields.iter().map(|f| f.name.clone()).collect();
                for p in &params {
                    ctx.locals.insert(p.clone());
                }

                // Start from nil, fold right over fields.
                //
                // Path A increment 6a: typed `const 0 [ref<LispyPair>]`
                // initial tail for the variant's cons chain.
                let nil_r = ctx.fresh_var("nil");
                ctx.emit(IIRInstr::new(
                    "const",
                    Some(nil_r.clone()),
                    vec![Operand::Int(0)],
                    "ref<LispyPair>",
                ), loc);
                ctx.record_type(&nil_r, "ref<LispyPair>");

                // Path A increment 6b: typed `alloc` + 2× `field_store`
                // for each variant field's cons cell (see record-constructor
                // site above for the convention).
                let mut tail = nil_r;
                for field_name in params.iter().rev() {
                    let cell = ctx.fresh_var("cell");
                    let mut alloc = IIRInstr::new(
                        "alloc",
                        Some(cell.clone()),
                        vec![],
                        "ref<LispyPair>",
                    );
                    alloc.may_alloc = true;
                    ctx.emit(alloc, loc);
                    ctx.record_type(&cell, "ref<LispyPair>");
                    ctx.emit(IIRInstr::new(
                        "field_store",
                        None,
                        vec![
                            Operand::Var(cell.clone()),
                            Operand::Int(0),
                            Operand::Var(field_name.clone()),
                        ],
                        "void",
                    ), loc);
                    ctx.emit(IIRInstr::new(
                        "field_store",
                        None,
                        vec![
                            Operand::Var(cell.clone()),
                            Operand::Int(1),
                            Operand::Var(tail),
                        ],
                        "void",
                    ), loc);
                    tail = cell;
                }

                // Prepend the integer tag — typed `const Int(tag) [i64]`
                // then typed `alloc` + `field_store` cons cell.
                let tag_reg = ctx.fresh_var("tag");
                ctx.emit(IIRInstr::new(
                    "const",
                    Some(tag_reg.clone()),
                    vec![Operand::Int(tag as i64)],
                    "i64",
                ), loc);
                ctx.record_type(&tag_reg, "i64");
                let head = ctx.fresh_var("head");
                let mut alloc_head = IIRInstr::new(
                    "alloc",
                    Some(head.clone()),
                    vec![],
                    "ref<LispyPair>",
                );
                alloc_head.may_alloc = true;
                ctx.emit(alloc_head, loc);
                ctx.record_type(&head, "ref<LispyPair>");
                ctx.emit(IIRInstr::new(
                    "field_store",
                    None,
                    vec![
                        Operand::Var(head.clone()),
                        Operand::Int(0),
                        Operand::Var(tag_reg),
                    ],
                    "void",
                ), loc);
                ctx.emit(IIRInstr::new(
                    "field_store",
                    None,
                    vec![
                        Operand::Var(head.clone()),
                        Operand::Int(1),
                        Operand::Var(tail),
                    ],
                    "void",
                ), loc);
                ctx.emit(IIRInstr::new("ret", None, vec![Operand::Var(head)], "ref<LispyPair>"), loc);

                self.functions.push(IIRFunction {
                    name: variant.name.clone(),
                    params: params.iter().map(|p| (p.clone(), "any".to_string())).collect(),
                    return_type: "any".into(),
                    register_count: count_registers(&ctx.instrs),
                    instructions: ctx.instrs,
                    type_status: FunctionTypeStatus::Untyped,
                    call_count: 0,
                    feedback_slots: HashMap::new(),
                    source_map: ctx.source_map,
                    param_refinements: Vec::new(),
                    return_refinement: None,
                });
            }

            // ── Predicate: VarName?(v) → (= (car v) tag) ──────────────
            {
                let mut ctx = FnCtx::new();
                ctx.locals.insert("v".to_string());

                // Path A increment 6c: typed `field_load[0]` instead of
                // `call_builtin "car"`.
                let car_v = ctx.fresh_var("hd");
                ctx.emit(IIRInstr::new(
                    "field_load",
                    Some(car_v.clone()),
                    vec![Operand::Var("v".to_string()), Operand::Int(0)],
                    "ref<any>",
                ), loc);
                ctx.record_type(&car_v, "ref<any>");

                let tag_reg = ctx.fresh_var("tag");
                ctx.emit(IIRInstr::new(
                    "const",
                    Some(tag_reg.clone()),
                    vec![Operand::Int(tag as i64)],
                    "any",
                ), loc);

                let result = ctx.fresh_var("pred");
                ctx.emit(IIRInstr::new(
                    "call_builtin",
                    Some(result.clone()),
                    vec![
                        Operand::Var("=".into()),
                        Operand::Var(car_v),
                        Operand::Var(tag_reg),
                    ],
                    "any",
                ), loc);
                ctx.emit(IIRInstr::new("ret", None, vec![Operand::Var(result)], "any"), loc);

                self.functions.push(IIRFunction {
                    name: format!("{}?", variant.name),
                    params: vec![("v".to_string(), "any".to_string())],
                    return_type: "any".into(),
                    register_count: count_registers(&ctx.instrs),
                    instructions: ctx.instrs,
                    type_status: FunctionTypeStatus::Untyped,
                    call_count: 0,
                    feedback_slots: HashMap::new(),
                    source_map: ctx.source_map,
                    param_refinements: Vec::new(),
                    return_refinement: None,
                });
            }

            // ── Accessors: <vprefix>-<field>(v) → car(cdr^(k+1) v) ────
            // Field k is at position k+1 (after the tag at cdr^0/car).
            //
            // Path A increment 6c: typed `field_load[1]` (cdr) and
            // `field_load[0]` (car) instead of `call_builtin`.
            for (k, field) in variant.fields.iter().enumerate() {
                let mut ctx = FnCtx::new();
                ctx.locals.insert("v".to_string());

                // cdr^(k+1) v: skip the tag (1 cdr) then k more for field index.
                let mut cur = "v".to_string();
                for _ in 0..=(k) {
                    let next = ctx.fresh_var("cdr");
                    ctx.emit(IIRInstr::new(
                        "field_load",
                        Some(next.clone()),
                        vec![Operand::Var(cur), Operand::Int(1)],
                        "ref<any>",
                    ), loc);
                    ctx.record_type(&next, "ref<any>");
                    cur = next;
                }
                let fval = ctx.fresh_var("fv");
                ctx.emit(IIRInstr::new(
                    "field_load",
                    Some(fval.clone()),
                    vec![Operand::Var(cur), Operand::Int(0)],
                    "ref<any>",
                ), loc);
                ctx.record_type(&fval, "ref<any>");
                ctx.emit(IIRInstr::new("ret", None, vec![Operand::Var(fval)], "ref<any>"), loc);

                self.functions.push(IIRFunction {
                    name: format!("{vprefix}-{}", field.name),
                    params: vec![("v".to_string(), "any".to_string())],
                    return_type: "any".into(),
                    register_count: count_registers(&ctx.instrs),
                    instructions: ctx.instrs,
                    type_status: FunctionTypeStatus::Untyped,
                    call_count: 0,
                    feedback_slots: HashMap::new(),
                    source_map: ctx.source_map,
                    param_refinements: Vec::new(),
                    return_refinement: None,
                });
            }
        }
        Ok(())
    }

    // ------------------------------------------------------------------
    // String-literal helper
    // ------------------------------------------------------------------

    /// Materialise a string literal into a fresh register and return
    /// the register's name.
    ///
    /// Used when a `call_builtin` needs a literal string as one of its
    /// runtime arguments — e.g. the `name` argument to `make_symbol`,
    /// `global_set`, `global_get`.  The string is emitted as a `const`
    /// instruction whose source is `Operand::Var(literal_text)`, which the
    /// twig-vm `const` handler interns as a symbol.
    ///
    /// **Note:** `alloc_closure` (LANG34) no longer uses `string_arg` — the
    /// function name is embedded inline as `Operand::Str` in the instruction
    /// itself.  `string_arg` is retained only for the operations above that
    /// still require the register-materialised convention.
    fn string_arg(&mut self, ctx: &mut FnCtx, literal: &str, loc: SourceLoc) -> String {
        let v = ctx.fresh_var("s");
        ctx.emit(IIRInstr::new(
            "const",
            Some(v.clone()),
            vec![Operand::Var(literal.to_string())],
            "any",
        ), loc);
        v
    }
}

impl Default for Compiler {
    fn default() -> Self {
        Compiler::new()
    }
}

// ---------------------------------------------------------------------------
// Register-count estimation
// ---------------------------------------------------------------------------
//
// `vm-core` allocates `register_count` slots per frame.  We count every
// distinct dest plus every Var operand that reads a name — the
// register file uses names as keys, so the count of distinct names is
// a tight upper bound.  +8 headroom matches the brainfuck-iir-compiler
// convention.

fn count_registers(instrs: &[IIRInstr]) -> usize {
    let mut names: HashSet<&str> = HashSet::new();
    for instr in instrs {
        if let Some(d) = instr.dest.as_deref() {
            names.insert(d);
        }
        for src in &instr.srcs {
            if let Operand::Var(s) = src {
                names.insert(s.as_str());
            }
        }
    }
    std::cmp::max(names.len() + 8, 16)
}
