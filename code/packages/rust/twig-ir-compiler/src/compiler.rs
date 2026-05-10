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
//! | Anything else (locals etc.) | `call_builtin "apply_closure", h, ...args`  |
//!
//! Top-level recursion stays on the fast `call` path; only locals
//! holding closures pay the indirect cost.
//!
//! ## Encoding string operands
//!
//! `interpreter_ir::Operand` doesn't have a dedicated `String` variant —
//! the four variants are `Var`, `Int`, `Float`, `Bool`.  Where the IR
//! semantically needs a string literal (e.g. the function name passed
//! to `make_closure`, or the global key passed to `global_set`), we
//! materialise it via a `const` instruction whose source operand is a
//! `Operand::Var(literal_text)`.  The `vm-core` `const` handler stores
//! the literal verbatim — Python's `IIRInstr("const", v, ["text"])`
//! and Rust's `IIRInstr("const", Some(v), [Operand::Var("text")])`
//! round-trip identically through the runtime.  The *destination*
//! variable then carries the string, and downstream `call_builtin`
//! ops resolve it through the frame in the normal way.

use std::collections::HashSet;

use interpreter_ir::{
    function::{FunctionTypeStatus, IIRFunction},
    instr::{IIRInstr, Operand},
    module::IIRModule,
    SourceLoc,
};
use lang_refined_types::{Kind, Predicate, RefinedType};

use twig_parser::{
    Apply, Begin, BoolLit, Expr, Form, If, IntLit, Lambda, Let, NilLit, Program, SymLit,
    TypeAnnotation, VarRef,
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
    // Arithmetic / comparison
    "+", "-", "*", "/", "=", "<", ">",
    // Cons cells
    "cons", "car", "cdr",
    // Predicates
    "null?", "pair?", "number?", "symbol?",
    // I/O
    "print",
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
}

impl Compiler {
    pub fn new() -> Self {
        Compiler {
            fn_globals: HashSet::new(),
            value_globals: HashSet::new(),
            functions: Vec::new(),
            lambda_counter: 0,
        }
    }

    // ------------------------------------------------------------------
    // Top-level driver
    // ------------------------------------------------------------------

    /// Compile a [`Program`] into an [`IIRModule`].  Consumes `self`.
    pub fn compile(mut self, program: &Program, module_name: &str) -> Result<IIRModule, TwigCompileError> {
        // ── Pre-pass: classify top-level defines ─────────────────────
        // Free-variable analysis at lambda sites needs to know which
        // names are globals (and therefore *not* free) before we walk
        // any bodies, so we do this in one pre-pass.
        for form in &program.forms {
            if let Form::Define(def) = form {
                if matches!(def.expr, Expr::Lambda(_)) {
                    self.fn_globals.insert(def.name.clone());
                } else {
                    self.value_globals.insert(def.name.clone());
                }
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

        Ok(IIRModule {
            name: module_name.to_string(),
            functions: self.functions,
            entry_point: Some("main".to_string()),
            language: "twig".to_string(),
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

        // 3. Emit `make_closure` at the call site.
        // The fn_name is itself a string literal — we materialise it
        // via `const` so it survives the runtime's frame resolution
        // (see the module-level "Encoding string operands" comment).
        let fn_name_reg = self.string_arg(outer, &fn_name, lam_loc);
        let dest = outer.fresh_var("clos");
        let mut srcs: Vec<Operand> = vec![
            Operand::Var("make_closure".into()),
            Operand::Var(fn_name_reg),
        ];
        for c in &captures {
            srcs.push(Operand::Var(c.clone()));
        }
        outer.emit(
            IIRInstr::new("call_builtin", Some(dest.clone()), srcs, "any"),
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

            Expr::Lambda(l) => self.compile_anonymous_lambda(l, ctx),

            Expr::Apply(a) => self.compile_apply(a, ctx),
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
        if self.fn_globals.contains(&v.name) {
            let name_reg = self.string_arg(ctx, &v.name, loc);
            let dest = ctx.fresh_var("fnref");
            ctx.emit(IIRInstr::new(
                "call_builtin",
                Some(dest.clone()),
                vec![Operand::Var("make_closure".into()), Operand::Var(name_reg)],
                "any",
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

    fn compile_apply(&mut self, expr: &Apply, ctx: &mut FnCtx) -> Result<String, TwigCompileError> {
        let loc = SourceLoc::new(expr.line, expr.column);
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
        // route through the `apply_closure` builtin.
        let fn_handle = self.compile_expr(&expr.fn_expr, ctx)?;
        let mut srcs: Vec<Operand> = vec![
            Operand::Var("apply_closure".into()),
            Operand::Var(fn_handle),
        ];
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
        Ok(dest)
    }

    // ------------------------------------------------------------------
    // String-literal helper
    // ------------------------------------------------------------------

    /// Materialise a string literal into a fresh register and return
    /// the register's name.
    ///
    /// Used when a `call_builtin` needs a literal string as one of its
    /// runtime arguments — e.g. the `name` argument to `make_closure`,
    /// `make_symbol`, `global_set`, `global_get`.  See the module-
    /// level "Encoding string operands" comment for why we pass the
    /// string through `Operand::Var` rather than via a dedicated
    /// `Operand::Str` variant (which would require modifying the
    /// shared `interpreter-ir` crate).
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
