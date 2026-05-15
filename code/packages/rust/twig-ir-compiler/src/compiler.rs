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
//!    Parameters lower 1-to-1 to typed `("any", name)` IIR params; the
//!    function body is the lowered body expressions plus a final `ret`.
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
//! Every emitted instruction carries `type_hint = "any"` because Twig is
//! dynamically typed — the function's `type_status` is therefore
//! `Untyped`.  The vm-core profiler will fill in observed types at
//! runtime; the JIT can specialise from those observations.
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
    RecordDef, StrLit, SymLit, TypeAnnotation, UnionDef, VarRef,
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
    var_counter: usize,
    label_counter: usize,
    /// Current AST-nesting depth.  Incremented on every entry to
    /// `compile_expr` and checked against [`MAX_COMPILE_DEPTH`] to
    /// guard against stack-overflow on adversarial input.
    depth: usize,
}

impl FnCtx {
    fn new() -> Self {
        FnCtx {
            instrs: Vec::new(),
            source_map: Vec::new(),
            locals: HashSet::new(),
            var_counter: 0,
            label_counter: 0,
            depth: 0,
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
    /// Names of top-level defines whose RHS is *not* a lambda — looked
    /// up through `global_get` at use sites.
    value_globals: HashSet<String>,
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
            value_globals: HashSet::new(),
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
                    // (define x value-expr) — evaluate at top level,
                    // store in globals.
                    let loc = SourceLoc::new(def.line, def.column);
                    let v = self.compile_expr(&def.expr, &mut main_ctx)?;
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
            main_ctx.emit(IIRInstr::new(
                "ret",
                None,
                vec![Operand::Var(reg)],
                "any",
            ), SourceLoc::SYNTHETIC);
        } else {
            // No final value-producing expression → return nil.
            let nil_var = main_ctx.fresh_var("nil");
            main_ctx.emit(IIRInstr::new(
                "call_builtin",
                Some(nil_var.clone()),
                vec![Operand::Var("make_nil".into())],
                "any",
            ), SourceLoc::SYNTHETIC);
            main_ctx.emit(IIRInstr::new(
                "ret",
                None,
                vec![Operand::Var(nil_var)],
                "any",
            ), SourceLoc::SYNTHETIC);
        }

        let main_fn = IIRFunction {
            name: "main".into(),
            params: vec![],
            return_type: "any".into(),
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
                    .map(|name| IIRExport::new(name))
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

    // ------------------------------------------------------------------
    // Top-level fn (define (name args...) body+)
    // ------------------------------------------------------------------

    fn compile_top_level_lambda(&mut self, name: &str, lam: &Lambda) -> Result<(), TwigCompileError> {
        let mut ctx = FnCtx::new();
        let lam_loc = SourceLoc::new(lam.line, lam.column);
        for p in &lam.params {
            ctx.locals.insert(p.clone());
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
        ctx.emit(
            IIRInstr::new("ret", None, vec![Operand::Var(last)], "any"),
            lam_loc,
        );

        let params = lam
            .params
            .iter()
            .map(|p| (p.clone(), "any".to_string()))
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
            params,
            return_type: "any".into(),
            register_count: count_registers(&ctx.instrs),
            instructions: ctx.instrs,
            type_status: FunctionTypeStatus::Untyped,
            call_count: 0,
            feedback_slots: std::collections::HashMap::new(),
            source_map: ctx.source_map,
            param_refinements,
            return_refinement,
        });
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
                ctx.emit(IIRInstr::new(
                    "const",
                    Some(v.clone()),
                    vec![Operand::Int(*value)],
                    "any",
                ), loc);
                Ok(v)
            }

            Expr::BoolLit(BoolLit { value, .. }) => {
                let v = ctx.fresh_var("b");
                ctx.emit(IIRInstr::new(
                    "const",
                    Some(v.clone()),
                    vec![Operand::Bool(*value)],
                    "any",
                ), loc);
                Ok(v)
            }

            Expr::NilLit(NilLit { .. }) => {
                let v = ctx.fresh_var("nil");
                ctx.emit(IIRInstr::new(
                    "call_builtin",
                    Some(v.clone()),
                    vec![Operand::Var("make_nil".into())],
                    "any",
                ), loc);
                Ok(v)
            }

            Expr::SymLit(SymLit { name, .. }) => {
                let name_reg = self.string_arg(ctx, name, loc);
                let v = ctx.fresh_var("sym");
                ctx.emit(IIRInstr::new(
                    "call_builtin",
                    Some(v.clone()),
                    vec![Operand::Var("make_symbol".into()), Operand::Var(name_reg)],
                    "any",
                ), loc);
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

        // Top-level value — look up via the host global table.
        if self.value_globals.contains(&v.name) {
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

        // Then branch — compile and copy into `result` via `_move`.
        let then_v = self.compile_expr(&expr.then_branch, ctx)?;
        let then_loc = SourceLoc::new(expr.then_branch.pos().0, expr.then_branch.pos().1);
        ctx.emit(IIRInstr::new(
            "call_builtin",
            Some(result.clone()),
            vec![Operand::Var("_move".into()), Operand::Var(then_v)],
            "any",
        ), then_loc);
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
        let else_loc = SourceLoc::new(expr.else_branch.pos().0, expr.else_branch.pos().1);
        ctx.emit(IIRInstr::new(
            "call_builtin",
            Some(result.clone()),
            vec![Operand::Var("_move".into()), Operand::Var(else_v)],
            "any",
        ), else_loc);

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
        // Compile RHSs in the OUTER scope (Scheme `let`, not `let*`).
        let mut binding_values: Vec<(String, String)> = Vec::new();
        for (name, rhs) in &expr.bindings {
            let v = self.compile_expr(rhs, ctx)?;
            binding_values.push((name.clone(), v));
        }

        // Bind each name into `locals_` via a `_move` copy so the
        // binding name exists as a named register in the frame.
        let mut added: Vec<String> = Vec::new();
        for (name, src) in &binding_values {
            if ctx.locals.insert(name.clone()) {
                added.push(name.clone());
            }
            ctx.emit(IIRInstr::new(
                "call_builtin",
                Some(name.clone()),
                vec![Operand::Var("_move".into()), Operand::Var(src.clone())],
                "any",
            ), loc);
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

        for (name, rhs) in &expr.bindings {
            // Compile the RHS in the current scope (which already includes
            // all prior let* bindings).
            let v = self.compile_expr(rhs, ctx)?;

            // Bind the name into locals BEFORE compiling the next binding.
            if ctx.locals.insert(name.clone()) {
                added.push(name.clone());
            }
            ctx.emit(IIRInstr::new(
                "call_builtin",
                Some(name.clone()),
                vec![Operand::Var("_move".into()), Operand::Var(v)],
                "any",
            ), loc);
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

                // Then path: compile rest, copy to dest, jump to end.
                let then_val = self.compile_and(rest, ctx, loc)?;
                ctx.emit(IIRInstr::new("call_builtin", Some(dest.clone()),
                    vec![Operand::Var("_move".into()), Operand::Var(then_val)], "any"), loc);
                ctx.emit(IIRInstr::new("jmp", None, vec![Operand::Var(end_label.clone())], "void"), loc);

                // Else path: dest ← #f
                ctx.emit(IIRInstr::new("label", None, vec![Operand::Var(else_label)], "void"), loc);
                let false_tmp = ctx.fresh_var("f");
                ctx.emit(IIRInstr::new("const", Some(false_tmp.clone()), vec![Operand::Bool(false)], "any"), loc);
                ctx.emit(IIRInstr::new("call_builtin", Some(dest.clone()),
                    vec![Operand::Var("_move".into()), Operand::Var(false_tmp)], "any"), loc);

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

                // Truthy path: dest ← cond, jump to end.
                ctx.emit(IIRInstr::new("call_builtin", Some(dest.clone()),
                    vec![Operand::Var("_move".into()), Operand::Var(cond)], "any"), loc);
                ctx.emit(IIRInstr::new("jmp", None, vec![Operand::Var(end_label.clone())], "void"), loc);

                // Falsy path: evaluate rest.
                ctx.emit(IIRInstr::new("label", None, vec![Operand::Var(falsy_label)], "void"), loc);
                let rest_val = self.compile_or(rest, ctx, loc)?;
                ctx.emit(IIRInstr::new("call_builtin", Some(dest.clone()),
                    vec![Operand::Var("_move".into()), Operand::Var(rest_val)], "any"), loc);

                ctx.emit(IIRInstr::new("label", None, vec![Operand::Var(end_label)], "void"), loc);
                Ok(dest)
            }
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
                let mut srcs: Vec<Operand> = vec![Operand::Var(v.name.clone())];
                for a in &expr.args {
                    let r = self.compile_expr(a, ctx)?;
                    srcs.push(Operand::Var(r));
                }
                let dest = ctx.fresh_var("r");
                ctx.emit(IIRInstr::new(
                    "call",
                    Some(dest.clone()),
                    srcs,
                    "any",
                ), loc);
                return Ok(dest);
            }

            if is_builtin(&v.name) {
                let mut srcs: Vec<Operand> = vec![Operand::Var(v.name.clone())];
                for a in &expr.args {
                    let r = self.compile_expr(a, ctx)?;
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
        let scrutinee_reg = self.compile_expr(&m.scrutinee, ctx)?;
        // Bind to a fresh stable register so arms can reference it freely.
        let matched = ctx.fresh_var("matched");
        ctx.emit(IIRInstr::new(
            "call_builtin",
            Some(matched.clone()),
            vec![Operand::Var("_move".into()), Operand::Var(scrutinee_reg)],
            "any",
        ), loc);

        // Result register — each arm writes its value here.
        let result = ctx.fresh_var("match_result");
        // Initialise to nil (fallthrough value).
        let nil_init = ctx.fresh_var("nil");
        ctx.emit(IIRInstr::new(
            "call_builtin",
            Some(nil_init.clone()),
            vec![Operand::Var("make_nil".into())],
            "any",
        ), loc);
        ctx.emit(IIRInstr::new(
            "call_builtin",
            Some(result.clone()),
            vec![Operand::Var("_move".into()), Operand::Var(nil_init)],
            "any",
        ), loc);

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
                            ctx.emit(IIRInstr::new(
                                "call_builtin",
                                Some(result.clone()),
                                vec![Operand::Var("_move".into()), Operand::Var(arm_result)],
                                "any",
                            ), arm_loc);
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
                    let tag_reg = ctx.fresh_var("tag");
                    ctx.emit(IIRInstr::new(
                        "call_builtin",
                        Some(tag_reg.clone()),
                        vec![Operand::Var("car".into()), Operand::Var(matched.clone())],
                        "any",
                    ), arm_loc);

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
                    let mut added_names: Vec<String> = Vec::new();
                    let mut cur_cdr = matched.clone();
                    for binding in bindings {
                        // Advance one cdr step
                        let next_cdr = ctx.fresh_var("cdr");
                        ctx.emit(IIRInstr::new(
                            "call_builtin",
                            Some(next_cdr.clone()),
                            vec![Operand::Var("cdr".into()), Operand::Var(cur_cdr.clone())],
                            "any",
                        ), arm_loc);
                        cur_cdr = next_cdr;

                        // Extract field: car of the cdr chain
                        let field_reg = ctx.fresh_var("field");
                        ctx.emit(IIRInstr::new(
                            "call_builtin",
                            Some(field_reg.clone()),
                            vec![Operand::Var("car".into()), Operand::Var(cur_cdr.clone())],
                            "any",
                        ), arm_loc);

                        // Bind field to name in ctx.locals
                        if ctx.locals.insert(binding.clone()) {
                            added_names.push(binding.clone());
                        }
                        ctx.emit(IIRInstr::new(
                            "call_builtin",
                            Some(binding.clone()),
                            vec![Operand::Var("_move".into()), Operand::Var(field_reg)],
                            "any",
                        ), arm_loc);
                    }

                    // Evaluate body
                    let mut body_result: Option<String> = None;
                    for e in &arm.body {
                        body_result = Some(self.compile_expr(e, ctx)?);
                    }
                    let body_v = body_result.expect("arm body non-empty (parser-enforced)");

                    // Copy to result register
                    ctx.emit(IIRInstr::new(
                        "call_builtin",
                        Some(result.clone()),
                        vec![Operand::Var("_move".into()), Operand::Var(body_v)],
                        "any",
                    ), arm_loc);

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
                    ctx.emit(IIRInstr::new(
                        "call_builtin",
                        Some(result.clone()),
                        vec![Operand::Var("_move".into()), Operand::Var(arm_result)],
                        "any",
                    ), arm_loc);
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
                    ctx.emit(IIRInstr::new(
                        "call_builtin",
                        Some(result.clone()),
                        vec![Operand::Var("_move".into()), Operand::Var(body_v)],
                        "any",
                    ), arm_loc);
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
        ctx.emit(IIRInstr::new(
            "call_builtin",
            Some(name.to_string()),
            vec![Operand::Var("_move".into()), Operand::Var(matched_reg.to_string())],
            "any",
        ), loc);
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
            let nil_r = ctx.fresh_var("nil");
            ctx.emit(IIRInstr::new(
                "call_builtin",
                Some(nil_r.clone()),
                vec![Operand::Var("make_nil".into())],
                "any",
            ), loc);

            let mut tail = nil_r;
            for field_name in params.iter().rev() {
                let cell = ctx.fresh_var("cell");
                ctx.emit(IIRInstr::new(
                    "call_builtin",
                    Some(cell.clone()),
                    vec![
                        Operand::Var("cons".into()),
                        Operand::Var(field_name.clone()),
                        Operand::Var(tail),
                    ],
                    "any",
                ), loc);
                tail = cell;
            }
            ctx.emit(IIRInstr::new("ret", None, vec![Operand::Var(tail)], "any"), loc);

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
        for (i, field) in rec.fields.iter().enumerate() {
            let mut ctx = FnCtx::new();
            ctx.locals.insert("r".to_string());

            // Build the cdr chain: apply `cdr` i times.
            let mut cur = "r".to_string();
            for _ in 0..i {
                let next = ctx.fresh_var("cdr");
                ctx.emit(IIRInstr::new(
                    "call_builtin",
                    Some(next.clone()),
                    vec![Operand::Var("cdr".into()), Operand::Var(cur)],
                    "any",
                ), loc);
                cur = next;
            }
            // Then take car.
            let field_val = ctx.fresh_var("fv");
            ctx.emit(IIRInstr::new(
                "call_builtin",
                Some(field_val.clone()),
                vec![Operand::Var("car".into()), Operand::Var(cur)],
                "any",
            ), loc);
            ctx.emit(IIRInstr::new("ret", None, vec![Operand::Var(field_val)], "any"), loc);

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
                let nil_r = ctx.fresh_var("nil");
                ctx.emit(IIRInstr::new(
                    "call_builtin",
                    Some(nil_r.clone()),
                    vec![Operand::Var("make_nil".into())],
                    "any",
                ), loc);

                let mut tail = nil_r;
                for field_name in params.iter().rev() {
                    let cell = ctx.fresh_var("cell");
                    ctx.emit(IIRInstr::new(
                        "call_builtin",
                        Some(cell.clone()),
                        vec![
                            Operand::Var("cons".into()),
                            Operand::Var(field_name.clone()),
                            Operand::Var(tail),
                        ],
                        "any",
                    ), loc);
                    tail = cell;
                }

                // Prepend the integer tag.
                let tag_reg = ctx.fresh_var("tag");
                ctx.emit(IIRInstr::new(
                    "const",
                    Some(tag_reg.clone()),
                    vec![Operand::Int(tag as i64)],
                    "any",
                ), loc);
                let head = ctx.fresh_var("head");
                ctx.emit(IIRInstr::new(
                    "call_builtin",
                    Some(head.clone()),
                    vec![
                        Operand::Var("cons".into()),
                        Operand::Var(tag_reg),
                        Operand::Var(tail),
                    ],
                    "any",
                ), loc);
                ctx.emit(IIRInstr::new("ret", None, vec![Operand::Var(head)], "any"), loc);

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

                let car_v = ctx.fresh_var("hd");
                ctx.emit(IIRInstr::new(
                    "call_builtin",
                    Some(car_v.clone()),
                    vec![Operand::Var("car".into()), Operand::Var("v".to_string())],
                    "any",
                ), loc);

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
            for (k, field) in variant.fields.iter().enumerate() {
                let mut ctx = FnCtx::new();
                ctx.locals.insert("v".to_string());

                // cdr^(k+1) v: skip the tag (1 cdr) then k more for field index.
                let mut cur = "v".to_string();
                for _ in 0..=(k) {
                    let next = ctx.fresh_var("cdr");
                    ctx.emit(IIRInstr::new(
                        "call_builtin",
                        Some(next.clone()),
                        vec![Operand::Var("cdr".into()), Operand::Var(cur)],
                        "any",
                    ), loc);
                    cur = next;
                }
                let fval = ctx.fresh_var("fv");
                ctx.emit(IIRInstr::new(
                    "call_builtin",
                    Some(fval.clone()),
                    vec![Operand::Var("car".into()), Operand::Var(cur)],
                    "any",
                ), loc);
                ctx.emit(IIRInstr::new("ret", None, vec![Operand::Var(fval)], "any"), loc);

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
