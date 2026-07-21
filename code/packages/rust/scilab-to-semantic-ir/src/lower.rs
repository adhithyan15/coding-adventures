//! The lowering pass from `coding_adventures_scilab_parser`'s generic
//! [`GrammarASTNode`] CST → [`semantic_ir::Module`], **v0.1.0**.
//!
//! This crate's structure mirrors `matlab-to-semantic-ir` closely (per
//! [`MA10`](../../../specs/MA10-scilab-language.md) §5's "grammar shape is a
//! legitimate MATLAB-family inheritance" finding): `scilab.grammar`'s own
//! precedence-cascade rule names (`logical_or`, `bit_and`, `comparison`,
//! `colon_expr`, `additive`, `multiplicative`, `unary`, `power`, `postfix`,
//! `primary`, ...) are *identical* to `matlab.grammar`'s, since it was
//! forked from that grammar at the source level (MA10 §3). Where the two
//! languages' CSTs genuinely diverge, this file diverges too — see each
//! section below.
//!
//! # Scope
//!
//! **Supported** (MA10 §4's in-scope surface):
//! - Literals: `NUMBER` (int- or float-shaped by lexeme), `STRING` (single-
//!   or double-quoted — the same underlying type in Scilab, MA10 §3),
//!   matrix literals `[1 2; 3 4]` → [`Expr::ArrayLit`].
//! - The eight `%`-prefixed special constants (MA10 §3/§4) — see
//!   "`%`-constants: constant-folded, not a new SIR node" below.
//! - Variables (`NAME`), assignment (`x = expr`; first occurrence →
//!   `LetStarBinding`, later re-assignment → `Assign`).
//! - Arithmetic: `+`/`-` always lower to [`Expr::ElementwiseOp`] *unless
//!   both operands are provably scalar* (see "Scalar/array
//!   disambiguation" below), in which case a plain `BuiltinCall` is
//!   emitted instead. `.* ./ .^` likewise take the same scalar fast path;
//!   bare `*` disambiguates to [`Expr::MatMul`] vs. elementwise per the
//!   same rule. `\`/`.\ ` (left division) are handled **uniformly** as a
//!   broadcast reciprocal division — see "`\`/`.\ `: one divergence from
//!   the MATLAB template" below. Bare `/` (mrdivide) between two
//!   non-scalars is **unsupported**, mirroring `matlab-to-semantic-ir`
//!   exactly (no linear-solve kernel exists in `array-runtime`).
//! - Comparisons `== ~= <> < > <= >=` (both not-equal spellings, MA10 §1
//!   finding 6) — `==`/`~=`/`<>` lower to `BuiltinCall("=" | "!=", ...)`
//!   same as every ordering comparison; see "No arithmetic or ordering
//!   over string literals" below for the one extra guard this frontend
//!   adds beyond the MATLAB template.
//! - Logical `&& || & |` (short-circuit and elementwise forms are **not**
//!   distinguished — both lower to `LogicalAnd`/`LogicalOr`, the same
//!   disclosed simplification `matlab-to-semantic-ir` already makes), and
//!   unary `+ - ~`.
//! - Ranges `a:b` (as a value, and specialised for `for i = a:b`) →
//!   [`Expr::Range`].
//! - Transpose `'`/`.'` → [`Expr::Transpose`].
//! - Indexing `A(i, j, ...)` (read → [`Expr::IndexGet`], write →
//!   [`Stmt::IndexSet`]) with 1-based → 0-based translation at lowering
//!   time; `:` → [`IndexArg::Whole`].
//! - Control flow `if`/`elseif`/`else`, `while`, `for i = a:b` — the
//!   `then`/`do`/comma/newline linker keyword (MA10 §3's `stmt_sep`)
//!   needs no SIR representation at all; see "The `stmt_sep` linker"
//!   below.
//! - `select`/`case`/`else` — Scilab's own multi-way conditional,
//!   desugared into a nested `if`-chain; see "`select`/`case`: desugared,
//!   no new SIR node" below.
//! - Single- or zero-output function definitions
//!   (`function [out =] name(params) ... endfunction`) and calls to them
//!   ([`Expr::DirectCall`]).
//! - `disp(x)` — the one recognised builtin, mapped onto the shared SIR
//!   `print` builtin, mirroring `matlab-to-semantic-ir`'s identical
//!   choice for MATLAB's own `disp`.
//!
//! **Deliberately out of scope for v0.1.0** (each rejected with an explicit
//! [`ScilabLowerError`], not silently mis-lowered):
//! - **`$` (last-index)** — mirrors `matlab-to-semantic-ir`'s own
//!   `end`-relative-indexing exclusion: no `size`/`shape` builtin is wired
//!   up yet to resolve "the current indexing dimension's size" at
//!   lowering time (this is an ahead-of-time pass; `scilab-runtime`'s own
//!   `$` resolution is a *runtime* mechanism, per its `eval_call_args` doc
//!   comment, that this frontend has no equivalent of). Neither
//!   `apl-to-semantic-ir` nor `j-to-semantic-ir` has an analogous "current
//!   dimension size inside an index" construct to borrow a solved pattern
//!   from, so this mirrors MATLAB's own scope decision rather than
//!   inventing one.
//! - **`%i`** — complex numbers are not representable (`array-runtime` is
//!   real-`f64`-only), mirroring `scilab-runtime::builtins::percent_const`'s
//!   own clean `Err` for the identical reason.
//! - **Multi-output functions** (`[a, b] = f(...)`) — mirrors
//!   `matlab-to-semantic-ir`'s own explicit v0.1.0 scope exclusion; no
//!   sibling frontend (`apl-to-semantic-ir`, `j-to-semantic-ir`, or any
//!   other `-to-semantic-ir` crate in this repo) has since solved
//!   multi-output/multi-return lowering either, so there is no more
//!   mature pattern to mirror instead.
//! - **`break`/`continue`** — `semantic-ir` has no early-exit
//!   control-flow node at all yet (confirmed: no `Break`/`Continue`
//!   variant anywhere in `semantic-ir/src/nodes.rs`) — a whole-IR gap,
//!   not specific to this frontend, exactly as `matlab-to-semantic-ir`'s
//!   own doc comment states for MATLAB's identical `break`/`continue`.
//! - Stepped (`a:step:b`) or non-range (`for i = A`) `for` loops — only
//!   `for i = a:b` (unit step) is supported, mirroring
//!   `matlab-to-semantic-ir`'s identical `for`-loop scope limit.
//! - Matrix power (`A^2`/`A.^2` with a non-scalar base — `array-runtime`
//!   has no eigendecomposition kernel), matrix right division `/`
//!   (mrdivide) between two non-scalars.
//! - Nested function definitions, cell arrays / `list`/`tlist`/`mlist`
//!   (MA10 §4's own deferred aggregate-type system), field access
//!   (`.NAME`), auto-vivification on indexed assignment to an undeclared
//!   variable, and chained assignment (`a = b = c`).
//! - **Any operator over a directly-written string literal** beyond
//!   assignment/display/`==`/`~=`/`<>` — see the dedicated section below.
//!
//! # The `stmt_sep` linker: no SIR representation needed
//!
//! MA10 §3's `stmt_sep` production (`"then" | "do" | COMMA | NEWLINE`) is a
//! genuine new grammar rule with no MATLAB equivalent — but by the time
//! source reaches *this* lowering pass, it has already served its only
//! purpose (telling the parser where a header ends and a body begins) and
//! carries no information this frontend needs to preserve. Concretely,
//! `if_stmt`'s children are `[cond, stmt_sep, block_body, elseif_clause*,
//! else_clause?]` — one slot wider than `matlab-to-semantic-ir`'s own
//! `if_stmt` shape (`[cond, block_body, ...]`), so every control-flow
//! lowering function below indexes one position further in to skip over
//! the `stmt_sep` node, exactly mirroring how `scilab-runtime::eval::eval_if`
//! (MA-10d) already had to adjust its own child-indexing for the identical
//! reason — confirmed directly against that crate's own doc comment on
//! `eval_if`, which documents the exact same off-by-one-node correction.
//! This is precisely the outcome MA10 §5 predicted: "by lowering time,
//! `then`/`do`/comma/newline have already collapsed to 'which statements
//! are in this branch/body,' the identical shape an ordinary `if`/`while`
//! lowering already produces."
//!
//! # `select`/`case`: desugared, no new SIR node
//!
//! Mirroring how `scilab-runtime::eval::eval_select` evaluates `select` at
//! *runtime* (evaluate the selector once, then compare it against each
//! `case`'s value in turn, running the first match's body or `else`'s if
//! none match), this frontend desugars `select`/`case`/`else` into a nested
//! `if`-chain at *lowering* time — no new `Expr`/`Stmt` variant, per MA10
//! §5's own prediction for this construct. The one wrinkle a pure `if`-chain
//! doesn't handle for free: the selector must be evaluated **once**, not
//! once per `case` (re-evaluating a side-effecting selector expression,
//! e.g. a function call, once per case would be observably wrong — it
//! would call the function N times instead of once). So [`Lowerer::lower_select`]
//! first binds the selector to a fresh, compiler-generated local
//! (`__select_N`, uniquely numbered per `select` statement in the module)
//! via an ordinary `LetStarBinding`, then folds the `case`/`else` clauses
//! into an `if`-chain of `BuiltinCall("=", [VarRef(temp), case_value])`
//! conditions — the same equality [`crate::lower::values_equal`]-shaped
//! comparison `eval_select` itself uses, just built as IR instead of
//! executed directly. This is why `lower_select` returns `Vec<Lowered>`
//! (the temp binding *and* the if-chain) rather than the single `Expr`
//! `lower_if` returns — mirroring `apl-to-semantic-ir::lower_assignment_chain`'s
//! identical "one syntactic construct unrolls into several IR statements"
//! shape, the nearest existing precedent in this repo for a construct that
//! needs its own hoisted temporary.
//!
//! # `%`-constants: constant-folded, not a new SIR node
//!
//! MA10 §5 asks whether the eight `PERCENT_CONST` spellings need a
//! dedicated SIR node. They do not: every one of `%pi %e %inf %nan %eps
//! %t %f` has a value fixed at compile time (`std::f64::consts::PI`, etc.
//! — `%t`/`%f` are plain `1`/`0`, this repo's established "logicals are
//! ordinary 0/1 numeric values" convention, matching
//! `scilab-runtime::builtins::percent_const`'s identical choice), so this
//! frontend simply constant-folds each spelling directly into a plain
//! [`Expr::IntLit`]/[`Expr::FloatLit`] at the point it is lexed as a
//! `primary` — see [`Lowerer::percent_const_expr`]. `%i` is the one
//! exception: it has no representable value at all (no complex-number
//! type exists anywhere in this repo's array-family stack), so it is a
//! clean, honest [`ScilabLowerError`] instead of a guessed substitute,
//! mirroring `scilab-runtime::builtins::percent_const`'s own identical
//! choice for the identical reason.
//!
//! # `\`/`.\ `: one divergence from the MATLAB template
//!
//! `matlab-to-semantic-ir`'s own `build_multiplicative` treats bare `\`
//! (mldivide, a real matrix left-division/solve problem for non-scalar
//! operands) as **unsupported** outside the scalar case, while treating
//! `.\ ` (the explicitly-elementwise spelling) as an unconditional
//! broadcast reciprocal division. Scilab's own ground-truth interpreter,
//! `scilab-runtime::eval::apply_binop`, does **not** draw that distinction:
//! its own doc comment states plainly that "bare `\` and `.\ ` are both
//! treated as the elementwise reference operation (`y / x`, broadcasting
//! scalars) ... an honest, disclosed simplification for the general matrix
//! case" — i.e. the crate that is this repo's actual authority on what
//! Scilab's `\` computes has *already* made the "treat both spellings the
//! same, always" call, for both spellings uniformly. Lowering bare `\` the
//! way MATLAB's frontend does (rejecting it between two non-scalars) would
//! therefore make this frontend **stricter than the language's own
//! shipped, ground-truth interpreter** for no reason — the interpreter
//! already computes an answer for that shape. So this frontend mirrors
//! `scilab-runtime`'s own choice instead: both `\` and `.\ ` uniformly
//! lower to `BuiltinCall("/", [rhs, lhs])` when both operands are provably
//! scalar, or `Expr::ElementwiseOp { op: Div, lhs: rhs, rhs: lhs, .. }`
//! (broadcast, operands swapped) otherwise — see
//! [`Lowerer::build_multiplicative`]'s `"\\" | ".\\"` arm. This is the one
//! place this crate's lowering deliberately does **not** copy
//! `matlab-to-semantic-ir` verbatim, and the reason is traceable to a
//! specific, already-shipped decision in this language's own runtime
//! rather than an invented simplification of this crate's own.
//!
//! # No arithmetic or ordering over string literals
//!
//! MA10 §1 finding 1 — the decisive finding motivating this whole
//! language's existence as its own frontend — is that Scilab's `+` means
//! *concatenation* on strings where MATLAB's means ASCII-numeric addition.
//! MA10 §4 accordingly scopes strings down to assignment/display/equality
//! only, explicitly refusing to implement `+` (or any other operator) over
//! them this cut: "implementing `+` at all here, without the typed-dispatch
//! layer §2 defers, would risk landing on MATLAB's numeric-addition answer
//! by accident, which would be *worse* than simply not having the operator
//! yet." `matlab-to-semantic-ir`'s own scalar/array disambiguation
//! (`expr_is_known_scalar`) does not special-case a string operand at all —
//! a bare `Expr::StrLit` simply falls through to `_ => false` (not scalar),
//! so a literal string reaching `+`/`-`/`*`/... would silently take the
//! ordinary array-domain `ElementwiseOp` path there, exactly the silent
//! mis-lowering MA10 §1 finding 1 warns against. This frontend closes that
//! gap for the *directly-written-literal* case (see
//! [`Lowerer::reject_string_operand`]): every additive, multiplicative,
//! power, and ordering-comparison (`< <= > >=`) operator construction
//! site checks each already-lowered operand and rejects with a clean
//! [`ScilabLowerError`] if it is a bare `Expr::StrLit`. Equality (`==`/
//! `~=`/`<>`) is deliberately **not** guarded — MA10 §4 explicitly keeps
//! string equality in scope. This is a syntactic, non-evaluating check
//! (identical in spirit to `expr_is_known_scalar`'s own syntactic-only
//! reach): a *variable* merely known by the programmer to hold a string
//! (`s = "hi"; y = s + 1;`) is invisible to it, since this frontend has no
//! type inference at all — a real, disclosed limitation, not a full fix,
//! but one that catches the direct, obvious case MA10 §1 finding 1 is
//! actually about.
//!
//! # Scalar/array disambiguation
//!
//! Identical heuristic to `matlab-to-semantic-ir`'s own (see that crate's
//! module doc comment for the full rationale): an operand is "known
//! scalar" iff it is a bare `IntLit`/`FloatLit`, or a `BuiltinCall` of
//! `+ - * / neg` whose own arguments are (transitively) known-scalar.
//! Falling through to the array-domain node in an ambiguous case is
//! always semantically safe, just occasionally more conservative than a
//! full type-inference pass would be.

use std::collections::HashSet;

use lexer::token::Token;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use semantic_ir::{
    Block, EffectSet, ElementwiseOpKind, Expr, Feature, FeatureManifest, Function, IndexArg,
    Metadata, Module, Param, ParamKind, Scope, Span, Stmt,
};

/// Maximum expression-nesting depth. Mirrors every other SIR frontend's
/// identically-named, identically-justified guard: turns pathologically
/// deep (but parseable) input into a clean [`ScilabLowerError`] instead of
/// a native (uncatchable) stack overflow.
const MAX_EXPR_DEPTH: usize = 256;

/// Maximum statement-block nesting depth (each `if`/`while`/`for`/`select`
/// body, or a `function` body, re-enters the block lowerer one level
/// deeper).
const MAX_BLOCK_DEPTH: usize = 256;

/// Synthetic file name used for all spans (the CST does not carry the
/// original path).
const FILE: &str = "<scilab>";

// ---------------------------------------------------------------------------
// Public error type
// ---------------------------------------------------------------------------

/// An error encountered during Scilab → SIR lowering.
///
/// Mirrors `MatlabLowerError`/`AplLowerError`'s shape exactly (`message` +
/// 1-based `line`/`column`) so tooling can treat every SIR frontend
/// uniformly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScilabLowerError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl std::fmt::Display for ScilabLowerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ScilabLowerError at {}:{}: {}",
            self.line, self.column, self.message
        )
    }
}

impl std::error::Error for ScilabLowerError {}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Lower a parsed Scilab CST (rooted at the `program` rule) into a SIR
/// module.
pub fn compile(tree: &GrammarASTNode, module_name: &str) -> Result<Module, ScilabLowerError> {
    Lowerer::new(module_name).lower_file(tree)
}

// ---------------------------------------------------------------------------
// The lowerer
// ---------------------------------------------------------------------------

/// One lowered top-level / body statement: either a `Stmt` or a bare
/// expression (an expression statement).
enum Lowered {
    Stmt(Box<Stmt>),
    Expr(Expr),
}

/// Per-function name-resolution context. Like MATLAB, Scilab scopes a
/// variable to its *whole enclosing function* (no block scoping) — a
/// function call gets a wholly fresh workspace (`scilab-runtime::eval::
/// Interpreter::call_user_function`'s own doc comment confirms this: "no
/// closures, no access to the caller's variables"). So, mirroring
/// `matlab-to-semantic-ir::FunctionCtx` exactly, `locals` simply
/// accumulates for the function's lifetime and is never rewound when
/// leaving an `if`/`while`/`for`/`select` body. The one place `locals` is
/// temporarily extended and then rewound is a `for`-loop variable, whose
/// scope genuinely is the loop.
struct FunctionCtx {
    params: HashSet<String>,
    locals: Vec<String>,
}

impl FunctionCtx {
    fn new(params: HashSet<String>) -> Self {
        Self {
            params,
            locals: Vec::new(),
        }
    }

    fn top_level() -> Self {
        Self::new(HashSet::new())
    }
}

struct Lowerer {
    module_name: String,
    /// Features observed while lowering, used to build the manifest so it
    /// declares *exactly* what the module emits.
    observed: FeatureManifest,
    /// Every top-level `function` name, collected in a first pass so a call
    /// to a function defined later in the file resolves as
    /// [`Expr::DirectCall`] regardless of textual order.
    function_names: HashSet<String>,
    /// The lowered top-level functions, in definition order. `main` is
    /// appended last by [`Self::lower_file`].
    functions: Vec<Function>,
    /// Counter for the compiler-generated `__select_N` temporaries
    /// [`Self::lower_select`] hoists the selector into — see this file's
    /// module doc comment, "`select`/`case`: desugared, no new SIR node".
    /// A single module-wide counter (not per-function) is simplest and
    /// still guarantees uniqueness: every `select` statement anywhere in
    /// the module gets its own number, so nested or sibling `select`s
    /// never collide.
    select_counter: usize,
}

impl Lowerer {
    fn new(module_name: &str) -> Self {
        Self {
            module_name: module_name.to_string(),
            observed: FeatureManifest::new(),
            function_names: HashSet::new(),
            functions: Vec::new(),
            select_counter: 0,
        }
    }

    // -------------------------------------------------------------------
    // for-loop variable scope: mark/rewind (the ONE place Scilab truly
    // does introduce a scope narrower than the whole function).
    // -------------------------------------------------------------------

    fn scope_mark(ctx: &FunctionCtx) -> usize {
        ctx.locals.len()
    }

    fn scope_rewind(ctx: &mut FunctionCtx, mark: usize) {
        ctx.locals.truncate(mark);
    }

    // -------------------------------------------------------------------
    // top level: `program` → collect function names, then lower
    // -------------------------------------------------------------------

    fn lower_file(&mut self, program: &GrammarASTNode) -> Result<Module, ScilabLowerError> {
        if program.rule_name != "program" {
            return Err(self.err_at(
                program,
                format!("expected `program` root, got `{}`", program.rule_name),
            ));
        }

        // Every value this frontend lowers has `sir_type: None` -- Scilab
        // has no static type declarations anywhere -- mirroring
        // `matlab-to-semantic-ir`'s identical observation.
        self.observed.add(Feature::DynamicTyping);

        self.collect_function_names(program)?;

        let mut ctx = FunctionCtx::top_level();
        let mut items: Vec<Lowered> = Vec::new();
        for stmt_line in child_nodes(program) {
            if stmt_line.rule_name != "statement_line" {
                continue;
            }
            let stmt = match self.first_child_named(stmt_line, "statement") {
                Some(s) => s,
                None => continue, // a bare terminator (blank line) -- nothing to lower
            };
            let inner = only_node(stmt, self)?;
            if inner.rule_name == "func_def" {
                let f = self.lower_func_def(inner)?;
                self.functions.push(f);
                continue;
            }
            items.extend(self.lower_statement_body_item(inner, &mut ctx, 0)?);
        }

        let span = Span::point(FILE, 1, 1);
        let main_body =
            assemble_stmts_only(items, Expr::NilLit { span: span.clone() }, span.clone());
        let main = Function {
            name: "main".to_string(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: main_body,
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: span.clone(),
        };

        let mut functions = std::mem::take(&mut self.functions);
        functions.push(main);

        let metadata = Metadata::new()
            .with_source_language("scilab")
            .with_sir_version(semantic_ir::CURRENT_SIR_VERSION);

        Ok(Module {
            name: self.module_name.clone(),
            manifest: self.observed.clone(),
            imports: vec![],
            exports: vec![],
            functions,
            globals: vec![],
            metadata,
            span,
        })
    }

    /// Pass 1: collect every top-level `function`'s name, so a call
    /// anywhere in the file — regardless of textual order — resolves as
    /// [`Expr::DirectCall`]. Nested function definitions are rejected (as
    /// an explicit error) when actually lowered, not here.
    fn collect_function_names(&mut self, program: &GrammarASTNode) -> Result<(), ScilabLowerError> {
        for stmt_line in child_nodes(program) {
            if stmt_line.rule_name != "statement_line" {
                continue;
            }
            let stmt = match self.first_child_named(stmt_line, "statement") {
                Some(s) => s,
                None => continue,
            };
            let inner = match only_node(stmt, self) {
                Ok(n) => n,
                Err(_) => continue,
            };
            if inner.rule_name == "func_def" {
                let name = self.func_def_name(inner)?;
                self.function_names.insert(name);
            }
        }
        Ok(())
    }

    // -------------------------------------------------------------------
    // function definitions
    // -------------------------------------------------------------------

    /// The function's own name: a bare `NAME` token directly under
    /// `func_def` (distinct from the *output variable*'s name, which lives
    /// one level deeper inside `func_returns` and is therefore invisible to
    /// this direct scan) — mirrors
    /// `matlab_to_semantic_ir::Lowerer::func_def_name` exactly.
    fn func_def_name(&self, def: &GrammarASTNode) -> Result<String, ScilabLowerError> {
        def.children
            .iter()
            .find_map(|c| match c {
                ASTNodeOrToken::Token(t) if t.effective_type_name() == "NAME" => {
                    Some(t.value.clone())
                }
                _ => None,
            })
            .ok_or_else(|| self.err_at(def, "malformed function definition: no name".to_string()))
    }

    /// Extract the declared output name, if any, from a `func_returns`
    /// node: `NAME EQ` (single output), `LBRACKET NAME RBRACKET EQ` (single
    /// output, explicit-bracket spelling), `LBRACKET RBRACKET EQ` (explicit
    /// zero-output bracket form), or a multi-name bracket list
    /// (unsupported — multi-output functions are out of scope, MA10 §4/this
    /// file's module doc comment).
    ///
    /// This is a strictly more complete reading of `func_returns` than
    /// `matlab-to-semantic-ir::lower_func_def`'s own inline handling (which
    /// treats a non-empty `name_list` as unconditionally multi-output and
    /// therefore cannot distinguish `[y] = f(x)` — a single name in
    /// brackets — from a genuine `[a, b] = f(x)`): mirrors
    /// `scilab-runtime::eval::Interpreter::register_function`'s own
    /// three-way handling instead, whose doc comment confirms "neither
    /// branch matching (`[] = f(...)`, an explicitly zero-output bracket
    /// form) leaves `returns` empty, which is exactly correct" — the
    /// ground-truth interpreter already solved this distinction correctly,
    /// so this frontend reuses that reading rather than the (slightly
    /// coarser) MATLAB-template one.
    fn lower_func_returns(&self, returns: &GrammarASTNode) -> Result<Option<String>, ScilabLowerError> {
        if let Some(name_list) = self.first_child_named(returns, "name_list") {
            let names: Vec<String> = name_list
                .children
                .iter()
                .filter_map(|c| match c {
                    ASTNodeOrToken::Token(t) if t.effective_type_name() == "NAME" => {
                        Some(t.value.clone())
                    }
                    _ => None,
                })
                .collect();
            return match names.len() {
                // `name_list = NAME { COMMA NAME }` requires at least one
                // NAME by construction; an empty vec here would mean the
                // grammar matched a `name_list` node with no NAME children
                // at all, which the parser cannot produce.
                0 => Err(self.err_at(name_list, "malformed output name list".to_string())),
                1 => Ok(Some(names.into_iter().next().expect("len checked above"))),
                _ => Err(self.err_at(
                    returns,
                    "unsupported: multiple output arguments (`[a, b] = f(...)`) are out of \
                     scope for v0.1.0"
                        .to_string(),
                )),
            };
        }
        // No `name_list` child: either the bare `NAME EQ` single-output
        // spelling, or the explicit, empty `[] = ` zero-output bracket
        // form -- a direct scan for a bare NAME token directly under
        // `func_returns` tells them apart.
        let bare = returns.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Token(t) if t.effective_type_name() == "NAME" => Some(t.value.clone()),
            _ => None,
        });
        Ok(bare)
    }

    /// Lower a `func_def` into a top-level [`Function`]. Scilab functions
    /// have no `return`-expression; the designated output variable's final
    /// value *is* the return value, so the body's trailing [`Block::value`]
    /// is synthesised as a `VarRef` to it (or `NilLit` for a function with
    /// no output) — identical to `matlab-to-semantic-ir::lower_func_def`.
    fn lower_func_def(&mut self, def: &GrammarASTNode) -> Result<Function, ScilabLowerError> {
        let span = self.span_of(def);
        let name = self.func_def_name(def)?;

        let mut output: Option<String> = None;
        if let Some(returns) = self.first_child_named(def, "func_returns") {
            output = self.lower_func_returns(returns)?;
        }

        let mut param_names: Vec<String> = Vec::new();
        if let Some(name_list) = self.first_child_named(def, "name_list") {
            param_names.extend(name_list.children.iter().filter_map(|c| match c {
                ASTNodeOrToken::Token(t) if t.effective_type_name() == "NAME" => {
                    Some(t.value.clone())
                }
                _ => None,
            }));
        }

        let body_node = self
            .first_child_named(def, "block_body")
            .ok_or_else(|| self.err_at(def, "malformed function: no body".to_string()))?;

        let mut ctx = FunctionCtx::new(param_names.iter().cloned().collect());
        let items = self.lower_body_items(body_node, &mut ctx, 0)?;

        let value = match &output {
            Some(out_name) => Expr::VarRef {
                name: out_name.clone(),
                scope: Scope::Local,
                span: span.clone(),
            },
            None => Expr::NilLit { span: span.clone() },
        };
        let body = assemble_stmts_only(items, value, span.clone());

        Ok(Function {
            name,
            params: param_names
                .into_iter()
                .map(|p| Param {
                    name: p,
                    sir_type: None,
                    kind: ParamKind::Required,
                    default: None,
                    span: span.clone(),
                })
                .collect(),
            return_type: None,
            captures: vec![],
            body,
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span,
        })
    }

    // -------------------------------------------------------------------
    // statement bodies (shared by `if`/`while`/`for`/`select`/`function`
    // bodies)
    // -------------------------------------------------------------------

    fn lower_body_items(
        &mut self,
        body_node: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Vec<Lowered>, ScilabLowerError> {
        let mut items = Vec::new();
        for stmt_line in child_nodes(body_node) {
            if stmt_line.rule_name == "statement_line" {
                items.extend(self.lower_statement_line(stmt_line, ctx, depth)?);
            }
        }
        Ok(items)
    }

    fn lower_block_body(
        &mut self,
        block_body: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Block, ScilabLowerError> {
        if depth > MAX_BLOCK_DEPTH {
            return Err(self.err_at(
                block_body,
                format!("control-flow nesting too deep (exceeds {MAX_BLOCK_DEPTH} levels)"),
            ));
        }
        let items = self.lower_body_items(block_body, ctx, depth)?;
        let span = self.span_of(block_body);
        Ok(assemble_stmts_only(
            items,
            Expr::NilLit { span: span.clone() },
            span,
        ))
    }

    fn lower_statement_line(
        &mut self,
        stmt_line: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Vec<Lowered>, ScilabLowerError> {
        let stmt = match self.first_child_named(stmt_line, "statement") {
            Some(s) => s,
            None => return Ok(Vec::new()), // a bare terminator (blank line)
        };
        let inner = only_node(stmt, self)?;
        self.lower_statement_body_item(inner, ctx, depth)
    }

    /// Dispatch one `statement` alternative that is *not* a top-level
    /// `func_def` (that case is handled directly by [`Self::lower_file`]).
    /// Reached here, `func_def` can only mean a *nested* definition, which
    /// this frontend does not support.
    ///
    /// Returns a `Vec` rather than an `Option` (unlike
    /// `matlab-to-semantic-ir::lower_statement_body_item`) because
    /// `select`/`case` desugars into *two* IR statements per syntactic
    /// statement (the hoisted selector binding, then the if-chain) — see
    /// [`Self::lower_select`]'s own doc comment, and
    /// `apl-to-semantic-ir::lower_top_level_statement`'s identical
    /// "one construct, several unrolled `Stmt`s" return shape for chained
    /// assignment.
    fn lower_statement_body_item(
        &mut self,
        inner: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Vec<Lowered>, ScilabLowerError> {
        if depth > MAX_BLOCK_DEPTH {
            return Err(self.err_at(
                inner,
                format!("control-flow nesting too deep (exceeds {MAX_BLOCK_DEPTH} levels)"),
            ));
        }
        match inner.rule_name.as_str() {
            "func_def" => Err(self.err_at(
                inner,
                "unsupported: nested function definitions are out of scope for v0.1.0".to_string(),
            )),
            "if_stmt" => Ok(vec![Lowered::Expr(self.lower_if(inner, ctx, depth)?)]),
            "select_stmt" => self.lower_select(inner, ctx, depth),
            "while_stmt" => Ok(vec![Lowered::Stmt(Box::new(
                self.lower_while(inner, ctx, depth)?,
            ))]),
            "for_stmt" => Ok(vec![Lowered::Stmt(Box::new(
                self.lower_for(inner, ctx, depth)?,
            ))]),
            "break_stmt" => Err(self.err_at(
                inner,
                "unsupported: `break` has no SIR equivalent yet (semantic-ir has no early-exit \
                 control-flow node at all -- a whole-IR gap, not specific to this frontend)"
                    .to_string(),
            )),
            "continue_stmt" => Err(self.err_at(
                inner,
                "unsupported: `continue` has no SIR equivalent yet (semantic-ir has no early-exit \
                 control-flow node at all -- a whole-IR gap, not specific to this frontend)"
                    .to_string(),
            )),
            _ => Ok(vec![self.lower_statement_expr(inner, ctx, depth)?]),
        }
    }

    // -------------------------------------------------------------------
    // control flow
    // -------------------------------------------------------------------

    /// Lower `if cond stmt_sep body { elseif_clause } [ else_clause ] end`.
    /// `node_children(if_stmt)` is `[cond, stmt_sep, body, elseif_clause*,
    /// else_clause?]` -- one slot wider than the MATLAB template, since
    /// `stmt_sep` is a real child node here (this file's module doc
    /// comment, "The `stmt_sep` linker"). `elseif_clause`'s own children
    /// are `[cond, stmt_sep, body]` for the identical reason; `else_clause`
    /// has no `stmt_sep` of its own (MA10 §3 never lists `else` among the
    /// six `stmt_sep` sites), so its shape (`[body]`) is unchanged from
    /// the MATLAB template.
    fn lower_if(
        &mut self,
        if_stmt: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Expr, ScilabLowerError> {
        struct Clause<'a> {
            cond: &'a GrammarASTNode,
            body: &'a GrammarASTNode,
        }
        let kids = child_nodes(if_stmt);
        if kids.len() < 3 {
            return Err(self.err_at(if_stmt, "malformed if: expected condition and body".to_string()));
        }
        let mut clauses: Vec<Clause> = vec![Clause {
            cond: kids[0],
            body: kids[2],
        }];
        let mut else_body: Option<&GrammarASTNode> = None;
        for rest in &kids[3..] {
            match rest.rule_name.as_str() {
                "elseif_clause" => match child_nodes(rest).as_slice() {
                    [c, _stmt_sep, b] => clauses.push(Clause { cond: c, body: b }),
                    _ => return Err(self.err_at(rest, "malformed elseif clause".to_string())),
                },
                "else_clause" => match child_nodes(rest).as_slice() {
                    [b] => else_body = Some(b),
                    _ => return Err(self.err_at(rest, "malformed else clause".to_string())),
                },
                other => {
                    return Err(self.err_at(
                        rest,
                        format!("unexpected `{other}` inside if statement"),
                    ))
                }
            }
        }

        let if_span = self.span_of(if_stmt);
        let mut else_branch: Block = match else_body {
            Some(b) => self.lower_block_body(b, ctx, depth + 1)?,
            None => empty_block(if_span.clone()),
        };
        for clause in clauses.into_iter().rev() {
            // Scilab truthiness ("nonzero is true", identical to MATLAB's
            // -- see `to_scilab_condition`'s doc comment).
            let cond = to_scilab_condition(self.lower_expr(clause.cond, ctx)?);
            let then_branch = self.lower_block_body(clause.body, ctx, depth + 1)?;
            let span = cond.span().clone();
            let folded = Expr::If {
                cond: Box::new(cond),
                then_branch: Box::new(then_branch),
                else_branch: Box::new(else_branch),
                span,
            };
            else_branch = value_block(folded);
        }
        match else_branch.value {
            Expr::If { .. } => Ok(else_branch.value),
            other => Ok(other),
        }
    }

    /// Lower `select selector stmt_sep { case_clause } [ else_clause ] end`
    /// into a nested `if`-chain — this file's module doc comment,
    /// "`select`/`case`: desugared, no new SIR node", explains the full
    /// design (why the selector must be hoisted into a fresh temp rather
    /// than re-lowered per case, and why this returns `Vec<Lowered>`
    /// rather than a single `Expr`).
    ///
    /// `node_children(select_stmt)` is `[selector, stmt_sep, case_clause*,
    /// else_clause?]`; each `case_clause`'s own children are `[value,
    /// stmt_sep, body]` (mirroring `if_stmt`'s `elseif_clause` shape
    /// exactly, since `case` is one of the six `stmt_sep` sites, MA10 §3).
    fn lower_select(
        &mut self,
        select_stmt: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Vec<Lowered>, ScilabLowerError> {
        struct Clause<'a> {
            value: &'a GrammarASTNode,
            body: &'a GrammarASTNode,
        }
        let kids = child_nodes(select_stmt);
        if kids.is_empty() {
            return Err(self.err_at(select_stmt, "malformed select: no selector".to_string()));
        }
        let selector_node = kids[0];
        let span = self.span_of(select_stmt);

        let selector = self.lower_expr(selector_node, ctx)?;
        let temp = format!("__select_{}", self.select_counter);
        self.select_counter += 1;
        ctx.locals.push(temp.clone());
        let mut stmts: Vec<Lowered> = vec![Lowered::Stmt(Box::new(Stmt::LetStarBinding {
            name: temp.clone(),
            sir_type: None,
            value: selector,
            span: span.clone(),
        }))];

        // `kids[1]` is `select`'s own `stmt_sep` (its header is one of the
        // six `stmt_sep` sites, MA10 §3) -- the case/else clause scan
        // starts at `kids[2]`, mirroring `scilab-runtime::eval::eval_select`'s
        // own `let mut i = 2;` exactly.
        let mut clauses: Vec<Clause> = Vec::new();
        let mut else_body: Option<&GrammarASTNode> = None;
        for rest in &kids[2..] {
            match rest.rule_name.as_str() {
                "case_clause" => match child_nodes(rest).as_slice() {
                    [v, _stmt_sep, b] => clauses.push(Clause { value: v, body: b }),
                    _ => return Err(self.err_at(rest, "malformed case clause".to_string())),
                },
                "else_clause" => match child_nodes(rest).as_slice() {
                    [b] => else_body = Some(b),
                    _ => return Err(self.err_at(rest, "malformed else clause".to_string())),
                },
                other => {
                    return Err(self.err_at(
                        rest,
                        format!("unexpected `{other}` inside select statement"),
                    ))
                }
            }
        }

        let mut else_branch: Block = match else_body {
            Some(b) => self.lower_block_body(b, ctx, depth + 1)?,
            None => empty_block(span.clone()),
        };
        for clause in clauses.into_iter().rev() {
            let case_value = self.lower_expr(clause.value, ctx)?;
            let cmp_span = case_value.span().clone();
            let eq = Expr::BuiltinCall {
                name: "=".to_string(),
                args: vec![
                    Expr::VarRef {
                        name: temp.clone(),
                        scope: Scope::Local,
                        span: cmp_span.clone(),
                    },
                    case_value,
                ],
                effects: EffectSet::PURE,
                span: cmp_span,
            };
            let cond = to_scilab_condition(eq);
            let then_branch = self.lower_block_body(clause.body, ctx, depth + 1)?;
            let if_span = cond.span().clone();
            let folded = Expr::If {
                cond: Box::new(cond),
                then_branch: Box::new(then_branch),
                else_branch: Box::new(else_branch),
                span: if_span,
            };
            else_branch = value_block(folded);
        }

        let final_expr = match else_branch.value {
            Expr::If { .. } => else_branch.value,
            other => other,
        };
        stmts.push(Lowered::Expr(final_expr));
        Ok(stmts)
    }

    /// `while cond stmt_sep body end`. `node_children(while_stmt)` is
    /// `[cond, stmt_sep, body]` -- kids[2], not kids[1], is the loop body
    /// (this file's module doc comment, "The `stmt_sep` linker").
    fn lower_while(
        &mut self,
        while_stmt: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Stmt, ScilabLowerError> {
        let (cond_node, body_node) = match child_nodes(while_stmt).as_slice() {
            [c, _stmt_sep, b] => (*c, *b),
            _ => {
                return Err(self.err_at(
                    while_stmt,
                    "malformed while: expected condition and body".to_string(),
                ))
            }
        };
        let cond = to_scilab_condition(self.lower_expr(cond_node, ctx)?);
        let body = self.lower_block_body(body_node, ctx, depth + 1)?;
        self.observed.add(Feature::Loops);
        Ok(Stmt::While {
            cond,
            body,
            span: self.span_of(while_stmt),
        })
    }

    /// Lower `for NAME = a:b stmt_sep body end` into [`Stmt::ForRange`].
    /// Only the unit-step, two-operand range form is supported (matches
    /// `matlab-to-semantic-ir`'s identical `for`-loop scope limit);
    /// `ForRange` is half-open, but Scilab's `a:b` is inclusive, so the
    /// exclusive bound is `b + 1` -- exact for any `a`/`b` precisely
    /// because the step is fixed at 1. `node_children(for_stmt)` is
    /// `[range, stmt_sep, body]` (the loop variable's `NAME`/`EQ` are bare
    /// tokens, not child nodes, extracted separately below) -- kids[2],
    /// not kids[1], is the loop body.
    fn lower_for(
        &mut self,
        for_stmt: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Stmt, ScilabLowerError> {
        let span = self.span_of(for_stmt);
        let var = for_stmt
            .children
            .iter()
            .find_map(|c| match c {
                ASTNodeOrToken::Token(t) if t.effective_type_name() == "NAME" => {
                    Some(t.value.clone())
                }
                _ => None,
            })
            .ok_or_else(|| self.err_at(for_stmt, "malformed for: no loop variable".to_string()))?;

        let (iter_node, body_node) = match child_nodes(for_stmt).as_slice() {
            [i, _stmt_sep, b] => (*i, *b),
            _ => {
                return Err(self.err_at(
                    for_stmt,
                    "malformed for: expected range and body".to_string(),
                ))
            }
        };

        let range_node = self.peel_to_named(iter_node, "colon_expr", 0);
        let (start_n, stop_n) = match range_node {
            Some(r) => match child_nodes(r).as_slice() {
                [s, e] => (*s, *e),
                _ => {
                    return Err(self.err_at(
                        r,
                        "unsupported: stepped for-loop ranges (`for i = a:step:b`) are out of \
                         scope for v0.1.0"
                            .to_string(),
                    ))
                }
            },
            None => {
                return Err(self.err_at(
                    iter_node,
                    "unsupported: `for` over a non-range expression is out of scope for v0.1.0 \
                     (only `for NAME = a:b` is supported)"
                        .to_string(),
                ))
            }
        };

        let start = self.lower_expr(start_n, ctx)?;
        let stop_val = self.lower_expr(stop_n, ctx)?;
        let stop_span = stop_val.span().clone();
        let stop = Expr::BuiltinCall {
            name: "+".to_string(),
            args: vec![
                stop_val,
                Expr::IntLit {
                    value: 1,
                    span: stop_span.clone(),
                },
            ],
            effects: EffectSet::PURE,
            span: stop_span,
        };
        let step = Expr::IntLit {
            value: 1,
            span: span.clone(),
        };

        self.observed.add(Feature::Loops);
        let mark = Self::scope_mark(ctx);
        ctx.locals.push(var.clone());
        let body = self.lower_block_body(body_node, ctx, depth + 1)?;
        Self::scope_rewind(ctx, mark);

        Ok(Stmt::ForRange {
            var,
            start,
            stop,
            step,
            body,
            span,
        })
    }

    // -------------------------------------------------------------------
    // assignment
    // -------------------------------------------------------------------

    /// Lower a `statement`'s `expr` alternative: either a value expression
    /// (a bare function call, e.g. `disp(x)`) or an assignment. Scilab's
    /// own grammar folds assignment *into* the expression precedence chain
    /// exactly like MATLAB's (`expr = assignment`, `assignment = logical_or
    /// [ EQ assignment ]`), so this peels down to the `assignment` rule
    /// specifically wherever it lands in the collapsed tree, mirroring
    /// `matlab_to_semantic_ir::lower_statement_expr` exactly.
    fn lower_statement_expr(
        &mut self,
        node: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Lowered, ScilabLowerError> {
        if depth > MAX_EXPR_DEPTH {
            return Err(self.err_at(
                node,
                format!("expression nesting too deep (exceeds {MAX_EXPR_DEPTH} levels)"),
            ));
        }
        if node.rule_name != "assignment" {
            return match child_nodes(node).as_slice() {
                [only] if node.children.len() == 1 => {
                    self.lower_statement_expr(only, ctx, depth + 1)
                }
                _ => {
                    let expr = self.lower_expr(node, ctx)?;
                    Ok(Lowered::Expr(expr))
                }
            };
        }
        match child_nodes(node).as_slice() {
            [lhs] if node.children.len() == 1 => {
                let expr = self.lower_expr(lhs, ctx)?;
                Ok(Lowered::Expr(expr))
            }
            [lhs, rhs] => {
                if rhs.rule_name == "assignment" && child_nodes(rhs).len() == 2 {
                    return Err(self.err_at(
                        rhs,
                        "unsupported: chained assignment (`a = b = c`) is out of scope for v0.1.0"
                            .to_string(),
                    ));
                }
                self.lower_assignment(node, lhs, rhs, ctx)
            }
            _ => Err(self.err_at(node, "malformed assignment".to_string())),
        }
    }

    fn lower_assignment(
        &mut self,
        assign_node: &GrammarASTNode,
        lhs: &GrammarASTNode,
        rhs: &GrammarASTNode,
        ctx: &mut FunctionCtx,
    ) -> Result<Lowered, ScilabLowerError> {
        let span = self.span_of(assign_node);

        if let Some(name) = self.bare_name(lhs) {
            let value = self.lower_expr(rhs, ctx)?;
            if ctx.locals.contains(&name) || ctx.params.contains(&name) {
                self.observed.add(Feature::MutableBindings);
                return Ok(Lowered::Stmt(Box::new(Stmt::Assign {
                    name,
                    scope: Scope::Local,
                    value,
                    span,
                })));
            }
            ctx.locals.push(name.clone());
            return Ok(Lowered::Stmt(Box::new(Stmt::LetStarBinding {
                name,
                sir_type: None,
                value,
                span,
            })));
        }

        if let Some((base_name, call_suffix)) = self.indexed_target(lhs) {
            if !(ctx.locals.contains(&base_name) || ctx.params.contains(&base_name)) {
                return Err(self.err_at(
                    lhs,
                    format!(
                        "cannot index-assign into `{base_name}`: not previously assigned \
                         (auto-vivification is out of scope for v0.1.0)"
                    ),
                ));
            }
            // A fresh statement's own expression tree gets its own
            // `MAX_EXPR_DEPTH` budget (depth 0) -- see
            // `Self::lower_index_args`'s doc comment for why *nested*
            // index/call positions instead thread the caller's depth.
            let indices = self.lower_index_args(call_suffix, ctx, 0)?;
            let value = self.lower_expr(rhs, ctx)?;
            return Ok(Lowered::Stmt(Box::new(Stmt::IndexSet {
                target: Box::new(Expr::VarRef {
                    name: base_name,
                    scope: Scope::Local,
                    span: span.clone(),
                }),
                indices,
                value: Box::new(value),
                span,
            })));
        }

        Err(self.err_at(
            lhs,
            "unsupported: assignment target is not a bare name or a simple index expression \
             (a multi-output destructuring target like `[a, b] = f(x)` is also rejected here, \
             since multi-output functions are out of scope for v0.1.0)"
                .to_string(),
        ))
    }

    // -------------------------------------------------------------------
    // expressions: precedence dispatch
    // -------------------------------------------------------------------

    fn lower_expr(&mut self, node: &GrammarASTNode, ctx: &mut FunctionCtx) -> Result<Expr, ScilabLowerError> {
        self.lower_expr_d(node, ctx, 0)
    }

    fn lower_expr_d(
        &mut self,
        node: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Expr, ScilabLowerError> {
        if depth > MAX_EXPR_DEPTH {
            return Err(self.err_at(
                node,
                format!("expression nesting too deep (exceeds {MAX_EXPR_DEPTH} levels)"),
            ));
        }
        match node.rule_name.as_str() {
            "logical_or" | "bit_or" => {
                if let Some(e) = self.try_logical(node, ctx, depth, true)? {
                    return Ok(e);
                }
            }
            "logical_and" | "bit_and" => {
                if let Some(e) = self.try_logical(node, ctx, depth, false)? {
                    return Ok(e);
                }
            }
            "comparison" => {
                if let Some(e) = self.try_comparison(node, ctx, depth)? {
                    return Ok(e);
                }
            }
            "colon_expr" => {
                if let Some(e) = self.lower_colon(node, ctx, depth)? {
                    return Ok(e);
                }
            }
            "additive" => {
                if let Some(e) = self.try_additive(node, ctx, depth)? {
                    return Ok(e);
                }
            }
            "multiplicative" => {
                if let Some(e) = self.try_multiplicative(node, ctx, depth)? {
                    return Ok(e);
                }
            }
            "unary" => {
                if let Some(e) = self.lower_unary(node, ctx, depth)? {
                    return Ok(e);
                }
            }
            "power" => {
                if let Some(e) = self.try_power(node, ctx, depth)? {
                    return Ok(e);
                }
            }
            "postfix" => {
                if let Some(e) = self.lower_postfix(node, ctx, depth)? {
                    return Ok(e);
                }
            }
            "assignment" => {
                // The RHS of a real assignment is itself grammatically
                // labelled `assignment` even when it carries no further
                // `=` -- see `matlab_to_semantic_ir`'s identical comment
                // for the full rationale.
                return match child_nodes(node).as_slice() {
                    [only] if node.children.len() == 1 => {
                        self.lower_expr_d(only, ctx, depth + 1)
                    }
                    _ => Err(self.err_at(
                        node,
                        "unsupported: assignment used as a value expression".to_string(),
                    )),
                };
            }
            _ => {}
        }
        match child_nodes(node).as_slice() {
            [only] if node.children.len() == 1 => self.lower_expr_d(only, ctx, depth + 1),
            _ => Err(self.err_at(
                node,
                format!("unsupported: `{}` (deferred)", node.rule_name),
            )),
        }
    }

    fn try_logical(
        &mut self,
        node: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
        is_or: bool,
    ) -> Result<Option<Expr>, ScilabLowerError> {
        let ops: &[&str] = if is_or { &["||", "|"] } else { &["&&", "&"] };
        let has_op = node
            .children
            .iter()
            .any(|c| matches!(c, ASTNodeOrToken::Token(t) if ops.contains(&t.value.as_str())));
        if !has_op {
            return Ok(None);
        }
        self.check_chain_length(node)?;
        // `Expr::LogicalAnd`/`Expr::LogicalOr` require `Feature::
        // ShortCircuit` in the manifest per the validator's own ground
        // truth -- see `matlab_to_semantic_ir::try_logical`'s identical
        // comment (and the confirmed bug that crate once shipped by
        // omitting this).
        self.observed.add(Feature::ShortCircuit);
        let mut acc: Option<Expr> = None;
        for child in &node.children {
            if let ASTNodeOrToken::Node(n) = child {
                // Short-circuit and elementwise `&&`/`&`, `||`/`|` are
                // deliberately NOT distinguished (this file's module doc
                // comment) -- every operand is forced to Scilab truthiness
                // before it can become the "deciding operand" a
                // short-circuit returns, mirroring the MATLAB template.
                let operand = to_scilab_condition(self.lower_expr_d(n, ctx, depth + 1)?);
                acc = Some(match acc.take() {
                    None => operand,
                    Some(lhs) => {
                        let span = lhs.span().clone();
                        if is_or {
                            Expr::LogicalOr {
                                lhs: Box::new(lhs),
                                rhs: Box::new(operand),
                                span,
                            }
                        } else {
                            Expr::LogicalAnd {
                                lhs: Box::new(lhs),
                                rhs: Box::new(operand),
                                span,
                            }
                        }
                    }
                });
            }
        }
        match acc {
            Some(e) => Ok(Some(e)),
            None => Err(self.err_at(node, "empty logical expression".to_string())),
        }
    }

    /// `colon_expr { (EQ_EQ|NE|NE_ALT|LE|GE|LT|GT) colon_expr }`. `<>` (MA10
    /// §1 finding 6, Scilab's own not-equal digraph) maps to the same
    /// `"!="` builtin as `~=` -- `scilab-parser`'s own grammar comment
    /// confirms both spellings collapse onto this one production, so the
    /// two-spellings-one-meaning collapse happens here, at lowering, not
    /// earlier.
    ///
    /// The four ORDERING operators (`< <= > >=`) reject a directly-written
    /// string-literal operand (see this file's module doc comment, "No
    /// arithmetic or ordering over string literals") -- MA10 §7's own
    /// citation restricts ordering comparisons to numeric/integer types.
    /// `=`/`!=` (equality) are deliberately NOT guarded: MA10 §4 keeps
    /// string equality in scope.
    fn try_comparison(
        &mut self,
        node: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Option<Expr>, ScilabLowerError> {
        const OPS: &[&str] = &["==", "~=", "<>", "<=", ">=", "<", ">"];
        let has_op = node
            .children
            .iter()
            .any(|c| matches!(c, ASTNodeOrToken::Token(t) if OPS.contains(&t.value.as_str())));
        if !has_op {
            return Ok(None);
        }
        self.check_chain_length(node)?;
        let mut acc: Option<Expr> = None;
        let mut pending: Option<String> = None;
        for child in &node.children {
            match child {
                ASTNodeOrToken::Token(t) if OPS.contains(&t.value.as_str()) => {
                    pending = Some(match t.value.as_str() {
                        "==" => "=".to_string(),
                        "~=" | "<>" => "!=".to_string(),
                        other => other.to_string(),
                    });
                }
                ASTNodeOrToken::Node(n) => {
                    let operand = self.lower_expr_d(n, ctx, depth + 1)?;
                    acc = Some(match (acc.take(), pending.take()) {
                        (None, _) => operand,
                        (Some(lhs), Some(op)) => {
                            if matches!(op.as_str(), "<" | "<=" | ">" | ">=") {
                                self.reject_string_operand(&lhs, node, "ordering comparisons")?;
                                self.reject_string_operand(&operand, node, "ordering comparisons")?;
                            }
                            let span = lhs.span().clone();
                            Expr::BuiltinCall {
                                name: op,
                                args: vec![lhs, operand],
                                effects: EffectSet::PURE,
                                span,
                            }
                        }
                        (Some(_), None) => {
                            return Err(self.err_at(node, "malformed comparison".to_string()))
                        }
                    });
                }
                ASTNodeOrToken::Token(_) => {}
            }
        }
        match acc {
            Some(e) => Ok(Some(e)),
            None => Err(self.err_at(node, "empty comparison".to_string())),
        }
    }

    fn lower_colon(
        &mut self,
        node: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Option<Expr>, ScilabLowerError> {
        if node.rule_name != "colon_expr" {
            return Ok(None);
        }
        let kids = child_nodes(node);
        let span = self.span_of(node);
        match kids.as_slice() {
            [only] => self.lower_expr_d(only, ctx, depth + 1).map(Some),
            [start, stop] => {
                self.observed.add(Feature::NDArrays);
                let start_e = self.lower_expr_d(start, ctx, depth + 1)?;
                let stop_e = self.lower_expr_d(stop, ctx, depth + 1)?;
                Ok(Some(Expr::Range {
                    start: Box::new(start_e),
                    step: None,
                    stop: Box::new(stop_e),
                    span,
                }))
            }
            [start, step, stop] => {
                self.observed.add(Feature::NDArrays);
                let start_e = self.lower_expr_d(start, ctx, depth + 1)?;
                let step_e = self.lower_expr_d(step, ctx, depth + 1)?;
                let stop_e = self.lower_expr_d(stop, ctx, depth + 1)?;
                Ok(Some(Expr::Range {
                    start: Box::new(start_e),
                    step: Some(Box::new(step_e)),
                    stop: Box::new(stop_e),
                    span,
                }))
            }
            _ => Err(self.err_at(node, "malformed range expression".to_string())),
        }
    }

    fn try_additive(
        &mut self,
        node: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Option<Expr>, ScilabLowerError> {
        let has_op = node
            .children
            .iter()
            .any(|c| matches!(c, ASTNodeOrToken::Token(t) if t.value == "+" || t.value == "-"));
        if !has_op {
            return Ok(None);
        }
        self.check_chain_length(node)?;
        // `acc` tracks the accumulated `Expr` *and* whether it is itself
        // known-scalar, updated incrementally (O(1) per fold step) --
        // see `matlab_to_semantic_ir::try_additive`'s identical comment for
        // why this must not be re-derived by re-walking the whole
        // accumulator on every step.
        let mut acc: Option<(Expr, bool)> = None;
        let mut pending: Option<String> = None;
        for child in &node.children {
            match child {
                ASTNodeOrToken::Token(t) if t.value == "+" || t.value == "-" => {
                    pending = Some(t.value.clone());
                }
                ASTNodeOrToken::Node(n) => {
                    let operand = self.lower_expr_d(n, ctx, depth + 1)?;
                    self.reject_string_operand(&operand, node, "arithmetic (`+`/`-`)")?;
                    let operand_scalar = expr_is_known_scalar(&operand);
                    acc = Some(match (acc.take(), pending.take()) {
                        (None, _) => (operand, operand_scalar),
                        (Some((lhs, lhs_scalar)), Some(op)) => {
                            self.build_additive(lhs, lhs_scalar, operand, operand_scalar, &op)
                        }
                        (Some(_), None) => {
                            return Err(
                                self.err_at(node, "malformed additive expression".to_string())
                            )
                        }
                    });
                }
                ASTNodeOrToken::Token(_) => {}
            }
        }
        match acc {
            Some((e, _)) => Ok(Some(e)),
            None => Err(self.err_at(node, "empty additive expression".to_string())),
        }
    }

    /// Combine one fold step of an additive chain -- mirrors
    /// `matlab_to_semantic_ir::build_additive` exactly (see that function's
    /// doc comment for why scalar-ness is threaded incrementally).
    fn build_additive(
        &mut self,
        lhs: Expr,
        lhs_scalar: bool,
        rhs: Expr,
        rhs_scalar: bool,
        op: &str,
    ) -> (Expr, bool) {
        let span = lhs.span().clone();
        if lhs_scalar && rhs_scalar {
            (
                Expr::BuiltinCall {
                    name: op.to_string(),
                    args: vec![lhs, rhs],
                    effects: EffectSet::PURE,
                    span,
                },
                true,
            )
        } else {
            self.observed.add(Feature::MatrixOps);
            self.observed.add(Feature::ArrayColumnMajor);
            let kind = if op == "+" {
                ElementwiseOpKind::Add
            } else {
                ElementwiseOpKind::Sub
            };
            (
                Expr::ElementwiseOp {
                    op: kind,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                    span,
                },
                false,
            )
        }
    }

    fn try_multiplicative(
        &mut self,
        node: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Option<Expr>, ScilabLowerError> {
        const OPS: &[&str] = &["*", "/", "\\", ".*", "./", ".\\"];
        let has_op = node
            .children
            .iter()
            .any(|c| matches!(c, ASTNodeOrToken::Token(t) if OPS.contains(&t.value.as_str())));
        if !has_op {
            return Ok(None);
        }
        self.check_chain_length(node)?;
        let mut acc: Option<(Expr, bool)> = None;
        let mut pending: Option<String> = None;
        for child in &node.children {
            match child {
                ASTNodeOrToken::Token(t) if OPS.contains(&t.value.as_str()) => {
                    pending = Some(t.value.clone());
                }
                ASTNodeOrToken::Node(n) => {
                    let operand = self.lower_expr_d(n, ctx, depth + 1)?;
                    self.reject_string_operand(&operand, node, "arithmetic (`* / \\`)")?;
                    let operand_scalar = expr_is_known_scalar(&operand);
                    acc = Some(match (acc.take(), pending.take()) {
                        (None, _) => (operand, operand_scalar),
                        (Some((lhs, lhs_scalar)), Some(op)) => self.build_multiplicative(
                            lhs,
                            lhs_scalar,
                            operand,
                            operand_scalar,
                            &op,
                            node,
                        )?,
                        (Some(_), None) => {
                            return Err(self.err_at(
                                node,
                                "malformed multiplicative expression".to_string(),
                            ))
                        }
                    });
                }
                ASTNodeOrToken::Token(_) => {}
            }
        }
        match acc {
            Some((e, _)) => Ok(Some(e)),
            None => Err(self.err_at(node, "empty multiplicative expression".to_string())),
        }
    }

    /// `lhs_scalar`/`rhs_scalar` are the caller's already-known scalar-ness
    /// of each operand -- see `matlab_to_semantic_ir::build_multiplicative`
    /// for the shared rationale. The `"\\" | ".\\"` arm is this crate's one
    /// deliberate divergence from that template -- see this file's module
    /// doc comment, "`\`/`.\ `: one divergence from the MATLAB template".
    fn build_multiplicative(
        &mut self,
        lhs: Expr,
        lhs_scalar: bool,
        rhs: Expr,
        rhs_scalar: bool,
        op: &str,
        node: &GrammarASTNode,
    ) -> Result<(Expr, bool), ScilabLowerError> {
        let span = lhs.span().clone();
        match op {
            ".*" => {
                if lhs_scalar && rhs_scalar {
                    Ok((
                        Expr::BuiltinCall {
                            name: "*".to_string(),
                            args: vec![lhs, rhs],
                            effects: EffectSet::PURE,
                            span,
                        },
                        true,
                    ))
                } else {
                    self.observed.add(Feature::MatrixOps);
                    self.observed.add(Feature::ArrayColumnMajor);
                    Ok((
                        Expr::ElementwiseOp {
                            op: ElementwiseOpKind::Mul,
                            lhs: Box::new(lhs),
                            rhs: Box::new(rhs),
                            span,
                        },
                        false,
                    ))
                }
            }
            "./" => {
                if lhs_scalar && rhs_scalar {
                    Ok((
                        Expr::BuiltinCall {
                            name: "/".to_string(),
                            args: vec![lhs, rhs],
                            effects: EffectSet::PURE,
                            span,
                        },
                        true,
                    ))
                } else {
                    self.observed.add(Feature::MatrixOps);
                    self.observed.add(Feature::ArrayColumnMajor);
                    Ok((
                        Expr::ElementwiseOp {
                            op: ElementwiseOpKind::Div,
                            lhs: Box::new(lhs),
                            rhs: Box::new(rhs),
                            span,
                        },
                        false,
                    ))
                }
            }
            // Both spellings treated UNIFORMLY as a broadcast reciprocal
            // division (`rhs / lhs`) -- see this file's module doc
            // comment for why this diverges from the MATLAB template's
            // asymmetric `\`-vs-`.\ ` treatment.
            "\\" | ".\\" => {
                if lhs_scalar && rhs_scalar {
                    Ok((
                        Expr::BuiltinCall {
                            name: "/".to_string(),
                            args: vec![rhs, lhs],
                            effects: EffectSet::PURE,
                            span,
                        },
                        true,
                    ))
                } else {
                    self.observed.add(Feature::MatrixOps);
                    self.observed.add(Feature::ArrayColumnMajor);
                    Ok((
                        Expr::ElementwiseOp {
                            op: ElementwiseOpKind::Div,
                            lhs: Box::new(rhs),
                            rhs: Box::new(lhs),
                            span,
                        },
                        false,
                    ))
                }
            }
            "*" => {
                if lhs_scalar && rhs_scalar {
                    Ok((
                        Expr::BuiltinCall {
                            name: "*".to_string(),
                            args: vec![lhs, rhs],
                            effects: EffectSet::PURE,
                            span,
                        },
                        true,
                    ))
                } else if lhs_scalar || rhs_scalar {
                    self.observed.add(Feature::MatrixOps);
                    self.observed.add(Feature::ArrayColumnMajor);
                    Ok((
                        Expr::ElementwiseOp {
                            op: ElementwiseOpKind::Mul,
                            lhs: Box::new(lhs),
                            rhs: Box::new(rhs),
                            span,
                        },
                        false,
                    ))
                } else {
                    self.observed.add(Feature::MatrixOps);
                    self.observed.add(Feature::ArrayColumnMajor);
                    Ok((
                        Expr::MatMul {
                            lhs: Box::new(lhs),
                            rhs: Box::new(rhs),
                            span,
                        },
                        false,
                    ))
                }
            }
            "/" => {
                if lhs_scalar && rhs_scalar {
                    Ok((
                        Expr::BuiltinCall {
                            name: "/".to_string(),
                            args: vec![lhs, rhs],
                            effects: EffectSet::PURE,
                            span,
                        },
                        true,
                    ))
                } else if lhs_scalar || rhs_scalar {
                    self.observed.add(Feature::MatrixOps);
                    self.observed.add(Feature::ArrayColumnMajor);
                    Ok((
                        Expr::ElementwiseOp {
                            op: ElementwiseOpKind::Div,
                            lhs: Box::new(lhs),
                            rhs: Box::new(rhs),
                            span,
                        },
                        false,
                    ))
                } else {
                    Err(self.err_at(
                        node,
                        "unsupported: matrix right division `/` (mrdivide) has no backend \
                         kernel yet"
                            .to_string(),
                    ))
                }
            }
            other => Err(self.err_at(node, format!("unsupported multiplicative operator `{other}`"))),
        }
    }

    fn lower_unary(
        &mut self,
        node: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Option<Expr>, ScilabLowerError> {
        if node.rule_name != "unary" || node.children.len() != 2 {
            return Ok(None);
        }
        let sign = node
            .children
            .iter()
            .find_map(|c| match c {
                ASTNodeOrToken::Token(t) => Some(t.value.clone()),
                ASTNodeOrToken::Node(_) => None,
            })
            .ok_or_else(|| self.err_at(node, "malformed unary expression".to_string()))?;
        let inner_node = *child_nodes(node)
            .first()
            .ok_or_else(|| self.err_at(node, "malformed unary expression: no operand".to_string()))?;
        let operand = self.lower_expr_d(inner_node, ctx, depth + 1)?;
        let span = operand.span().clone();
        let result = match sign.as_str() {
            "+" => operand,
            "-" => {
                self.reject_string_operand(&operand, node, "unary `-`")?;
                match operand {
                    Expr::IntLit { value, span } => Expr::IntLit {
                        value: value.wrapping_neg(),
                        span,
                    },
                    Expr::FloatLit { value, span } => Expr::FloatLit { value: -value, span },
                    other => Expr::BuiltinCall {
                        name: "neg".to_string(),
                        args: vec![other],
                        effects: EffectSet::PURE,
                        span,
                    },
                }
            }
            "~" => Expr::BuiltinCall {
                name: "not".to_string(),
                // Scilab truthiness ("nonzero is true"), identical to
                // MATLAB's -- see `to_scilab_condition`'s doc comment.
                args: vec![to_scilab_condition(operand)],
                effects: EffectSet::PURE,
                span,
            },
            other => return Err(self.err_at(node, format!("unsupported unary operator `{other}`"))),
        };
        Ok(Some(result))
    }

    /// `postfix [ (^|.^) unary ]`. Both spellings lower identically
    /// (elementwise power, scalar broadcasting handled by the runtime) --
    /// mirrors `matlab_to_semantic_ir::try_power` exactly, including its
    /// choice not to apply the scalar-fast-path `BuiltinCall` optimisation
    /// here (unlike `+`/`-`/`*`/`/`/`\`).
    fn try_power(
        &mut self,
        node: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Option<Expr>, ScilabLowerError> {
        if node.rule_name != "power" {
            return Ok(None);
        }
        let op = node.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Token(t) if t.value == "^" || t.value == ".^" => Some(t.value.clone()),
            _ => None,
        });
        let op = match op {
            Some(o) => o,
            None => return Ok(None),
        };
        let (base, exp) = match child_nodes(node).as_slice() {
            [b, e] => (*b, *e),
            _ => return Err(self.err_at(node, "malformed power expression".to_string())),
        };
        let _ = op; // both `^` and `.^` lower identically (see module scope note)
        let base_e = self.lower_expr_d(base, ctx, depth + 1)?;
        let exp_e = self.lower_expr_d(exp, ctx, depth + 1)?;
        self.reject_string_operand(&base_e, node, "power (`^`/`.^`)")?;
        self.reject_string_operand(&exp_e, node, "power (`^`/`.^`)")?;
        let span = base_e.span().clone();
        self.observed.add(Feature::MatrixOps);
        self.observed.add(Feature::ArrayColumnMajor);
        Ok(Some(Expr::ElementwiseOp {
            op: ElementwiseOpKind::Pow,
            lhs: Box::new(base_e),
            rhs: Box::new(exp_e),
            span,
        }))
    }

    // -------------------------------------------------------------------
    // postfix: transpose / call / index
    // -------------------------------------------------------------------

    fn lower_postfix(
        &mut self,
        node: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Option<Expr>, ScilabLowerError> {
        if node.rule_name != "postfix" {
            return Ok(None);
        }
        let kids = child_nodes(node);
        let (primary, suffixes) = match kids.split_first() {
            Some((p, rest)) => (*p, rest),
            None => return Err(self.err_at(node, "malformed postfix expression".to_string())),
        };
        if suffixes.is_empty() {
            return self.lower_primary(primary, ctx, depth + 1).map(Some);
        }

        let mut acc: Option<Expr> = None;
        let mut first = true;
        for suffix in suffixes {
            match suffix.rule_name.as_str() {
                "transpose_suffix" => {
                    let target = match acc.take() {
                        Some(e) => e,
                        None => self.lower_primary(primary, ctx, depth + 1)?,
                    };
                    let tok = suffix
                        .token()
                        .ok_or_else(|| self.err_at(suffix, "malformed transpose suffix".to_string()))?;
                    let conjugate = tok.value == "'";
                    let span = target.span().clone();
                    self.observed.add(Feature::MatrixOps);
                    self.observed.add(Feature::ArrayColumnMajor);
                    acc = Some(Expr::Transpose {
                        target: Box::new(target),
                        conjugate,
                        span,
                    });
                }
                "call_suffix" => {
                    if first {
                        let name = self.primary_bare_name(primary).ok_or_else(|| {
                            self.err_at(
                                primary,
                                "unsupported: call/index target is not a bare name".to_string(),
                            )
                        })?;
                        if ctx.locals.contains(&name) || ctx.params.contains(&name) {
                            let span = self.span_of(primary);
                            let indices = self.lower_index_args(suffix, ctx, depth + 1)?;
                            acc = Some(Expr::IndexGet {
                                target: Box::new(Expr::VarRef {
                                    name,
                                    scope: Scope::Local,
                                    span: span.clone(),
                                }),
                                indices,
                                span,
                            });
                        } else if name == "disp" {
                            // The one builtin this frontend recognises,
                            // mirroring `matlab-to-semantic-ir`'s identical
                            // `disp` -> `"print"` mapping.
                            let span = self.span_of(primary);
                            let args = self.lower_call_args(suffix, ctx, depth + 1)?;
                            if args.len() != 1 {
                                return Err(self.err_at(
                                    primary,
                                    "`disp` takes exactly one argument".to_string(),
                                ));
                            }
                            acc = Some(Expr::BuiltinCall {
                                name: "print".to_string(),
                                args,
                                effects: EffectSet::PURE,
                                span,
                            });
                        } else if self.function_names.contains(&name) {
                            let span = self.span_of(primary);
                            let args = self.lower_call_args(suffix, ctx, depth + 1)?;
                            acc = Some(Expr::DirectCall {
                                fn_name: name,
                                args,
                                effects: EffectSet::PURE,
                                span,
                            });
                        } else {
                            return Err(self.err_at(
                                primary,
                                format!(
                                    "unsupported: unknown identifier `{name}` (not a known \
                                     variable or user function -- only `disp` is recognised as \
                                     a builtin in this cut)"
                                ),
                            ));
                        }
                    } else {
                        let base = acc
                            .take()
                            .expect("acc is set after the first suffix in the fold");
                        let span = base.span().clone();
                        let indices = self.lower_index_args(suffix, ctx, depth + 1)?;
                        acc = Some(Expr::IndexGet {
                            target: Box::new(base),
                            indices,
                            span,
                        });
                    }
                }
                "cell_suffix" | "field_suffix" => {
                    return Err(self.err_at(
                        suffix,
                        format!("unsupported: `{}` is out of scope for v0.1.0", suffix.rule_name),
                    ))
                }
                other => return Err(self.err_at(suffix, format!("unsupported postfix suffix `{other}`"))),
            }
            first = false;
        }
        Ok(acc)
    }

    fn lower_primary(
        &mut self,
        node: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Expr, ScilabLowerError> {
        if depth > MAX_EXPR_DEPTH {
            return Err(self.err_at(
                node,
                format!("expression nesting too deep (exceeds {MAX_EXPR_DEPTH} levels)"),
            ));
        }
        if let Some(tok) = node.token() {
            let span = self.span_of(node);
            // Dispatch on `effective_type_name()`, NOT `tok.type_` --
            // unlike MATLAB's own lexer (where NUMBER/STRING/NAME map
            // directly onto core `TokenType` variants), Scilab's grammar-
            // specific token kinds (`PERCENT_CONST`, `DOLLAR`) do NOT map
            // onto any core `TokenType` variant, so they fall back to
            // `TokenType::Name` with `type_name` set to the real spelling
            // (see `lexer::token::Token::effective_type_name`'s own doc
            // comment). Matching on `tok.type_` alone -- the MATLAB
            // template's own approach -- would silently misread a `$` or
            // `%pi` token as an ordinary bare variable named `"$"`/`"%pi"`.
            // `scilab-runtime::eval::Interpreter::eval_primary` already
            // established this exact dispatch style for the identical
            // reason; mirrored here rather than the MATLAB template's.
            return match tok.effective_type_name() {
                "NUMBER" => Ok(self.number_literal_expr(tok, &span)),
                "STRING" => {
                    self.observed.add(Feature::Strings);
                    Ok(Expr::StrLit {
                        value: tok.value.clone(),
                        span,
                    })
                }
                "PERCENT_CONST" => self.percent_const_expr(tok, &span),
                "DOLLAR" => Err(self.err_at(
                    node,
                    "unsupported: `$` (last-index) is out of scope for v0.1.0 -- mirrors \
                     matlab-to-semantic-ir's own `end`-relative-indexing exclusion (no \
                     size/shape builtin is wired up yet to resolve \"the current indexing \
                     dimension's size\" at lowering time)"
                        .to_string(),
                )),
                "NAME" => {
                    let name = tok.value.clone();
                    if ctx.params.contains(&name) {
                        Ok(Expr::VarRef {
                            name,
                            scope: Scope::Param,
                            span,
                        })
                    } else if ctx.locals.contains(&name) {
                        Ok(Expr::VarRef {
                            name,
                            scope: Scope::Local,
                            span,
                        })
                    } else {
                        Err(self.err_at(
                            node,
                            format!("undefined variable `{name}` (not previously assigned)"),
                        ))
                    }
                }
                other => Err(self.err_at(node, format!("unsupported literal token `{other}`"))),
            };
        }
        let only = match child_nodes(node).as_slice() {
            [only] => *only,
            _ => return Err(self.err_at(node, "malformed primary expression".to_string())),
        };
        match only.rule_name.as_str() {
            "matrix_literal" => self.lower_matrix_literal(only, ctx, depth + 1),
            "cell_literal" => Err(self.err_at(
                only,
                "unsupported: cell arrays (`{ ... }`) are out of scope for v0.1.0 (MA10 §4 \
                 defers `list`/`tlist`/`mlist` entirely)"
                    .to_string(),
            )),
            "group" => match child_nodes(only).as_slice() {
                [inner] => self.lower_expr_d(inner, ctx, depth + 1),
                _ => Err(self.err_at(only, "malformed parenthesised expression".to_string())),
            },
            other => Err(self.err_at(only, format!("unsupported: `{other}` (deferred)"))),
        }
    }

    /// Resolve one of the eight fixed `PERCENT_CONST` spellings to a
    /// constant-folded `Expr` -- see this file's module doc comment,
    /// "`%`-constants: constant-folded, not a new SIR node".
    fn percent_const_expr(&mut self, tok: &Token, span: &Span) -> Result<Expr, ScilabLowerError> {
        match tok.value.as_str() {
            "%pi" => {
                self.observed.add(Feature::Floats);
                Ok(Expr::FloatLit {
                    value: std::f64::consts::PI,
                    span: span.clone(),
                })
            }
            "%e" => {
                self.observed.add(Feature::Floats);
                Ok(Expr::FloatLit {
                    value: std::f64::consts::E,
                    span: span.clone(),
                })
            }
            "%inf" => {
                self.observed.add(Feature::Floats);
                Ok(Expr::FloatLit {
                    value: f64::INFINITY,
                    span: span.clone(),
                })
            }
            "%nan" => {
                self.observed.add(Feature::Floats);
                Ok(Expr::FloatLit {
                    value: f64::NAN,
                    span: span.clone(),
                })
            }
            "%eps" => {
                self.observed.add(Feature::Floats);
                Ok(Expr::FloatLit {
                    value: f64::EPSILON,
                    span: span.clone(),
                })
            }
            // `%t`/`%f` are ordinary `1`/`0` -- this repo's established
            // "logicals are ordinary 0/1 numeric values" convention
            // (matches `scilab-runtime::builtins::percent_const`'s
            // identical choice); plain `IntLit`s need no extra feature.
            "%t" => Ok(Expr::IntLit {
                value: 1,
                span: span.clone(),
            }),
            "%f" => Ok(Expr::IntLit {
                value: 0,
                span: span.clone(),
            }),
            "%i" => Err(ScilabLowerError {
                message: "unsupported: `%i` (complex numbers) is out of scope for v0.1.0 -- \
                          array-runtime has no complex-number representation, mirroring \
                          scilab-runtime::builtins::percent_const's own clean error for the \
                          identical reason"
                    .to_string(),
                line: span.start_line,
                column: span.start_col,
            }),
            other => Err(ScilabLowerError {
                message: format!(
                    "unsupported special constant `{other}` (scilab-lexer's PERCENT_CONST \
                     pattern should only ever produce one of the eight fixed spellings)"
                ),
                line: span.start_line,
                column: span.start_col,
            }),
        }
    }

    fn lower_matrix_literal(
        &mut self,
        node: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Expr, ScilabLowerError> {
        if depth > MAX_BLOCK_DEPTH {
            return Err(self.err_at(
                node,
                format!("matrix literal nesting too deep (exceeds {MAX_BLOCK_DEPTH} levels)"),
            ));
        }
        self.observed.add(Feature::NDArrays);
        self.observed.add(Feature::ArrayColumnMajor);
        let span = self.span_of(node);
        let mut rows: Vec<Vec<Expr>> = Vec::new();
        if let Some(matrix_rows) = self.first_child_named(node, "matrix_rows") {
            for row in child_nodes(matrix_rows) {
                if row.rule_name == "matrix_row" {
                    let mut cells = Vec::new();
                    for cell in child_nodes(row) {
                        cells.push(self.lower_expr_d(cell, ctx, depth + 1)?);
                    }
                    rows.push(cells);
                }
            }
        }
        Ok(Expr::ArrayLit { rows, span })
    }

    // -------------------------------------------------------------------
    // indexing / call arguments
    // -------------------------------------------------------------------

    /// `depth` is the *enclosing expression's* depth, not a fresh count --
    /// see `matlab_to_semantic_ir::lower_index_args`'s identical doc
    /// comment for why this must be threaded rather than restarted.
    fn lower_index_args(
        &mut self,
        call_suffix: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Vec<IndexArg>, ScilabLowerError> {
        if depth > MAX_EXPR_DEPTH {
            return Err(self.err_at(
                call_suffix,
                format!("expression nesting too deep (exceeds {MAX_EXPR_DEPTH} levels)"),
            ));
        }
        self.observed.add(Feature::NDArrays);
        let arg_list = match self.first_child_named(call_suffix, "arg_list") {
            Some(a) => a,
            None => return Ok(vec![]),
        };
        let mut out = Vec::new();
        for arg in child_nodes(arg_list) {
            if arg.rule_name == "arg" {
                out.push(self.lower_one_index_arg(arg, ctx, depth + 1)?);
            }
        }
        Ok(out)
    }

    /// Lower one index-position argument, translating 1-based Scilab
    /// indexing to the IR's 0-based convention -- mirrors
    /// `matlab_to_semantic_ir::lower_one_index_arg` exactly. A range-valued
    /// index argument (`A(1:3)`) is not specially represented as
    /// [`IndexArg::Range`] here -- like the MATLAB template, this frontend
    /// always emits [`IndexArg::Scalar`] with a uniform `-1` shift; see
    /// this file's module doc comment for why that (inherited) gap is left
    /// as-is rather than fixed in this crate.
    fn lower_one_index_arg(
        &mut self,
        arg: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<IndexArg, ScilabLowerError> {
        if let Some(tok) = arg.token() {
            if tok.value == ":" {
                return Ok(IndexArg::Whole);
            }
        }
        let inner = match child_nodes(arg).as_slice() {
            [only] => *only,
            _ => return Err(self.err_at(arg, "malformed index argument".to_string())),
        };
        let idx = self.lower_expr_d(inner, ctx, depth)?;
        let span = idx.span().clone();
        let shifted = match idx {
            Expr::IntLit { value, .. } => Expr::IntLit {
                value: value - 1,
                span,
            },
            other => Expr::BuiltinCall {
                name: "-".to_string(),
                args: vec![
                    other,
                    Expr::IntLit {
                        value: 1,
                        span: span.clone(),
                    },
                ],
                effects: EffectSet::PURE,
                span,
            },
        };
        Ok(IndexArg::Scalar(Box::new(shifted)))
    }

    /// See [`Self::lower_index_args`] on why `depth` is threaded rather
    /// than restarted.
    fn lower_call_args(
        &mut self,
        call_suffix: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Vec<Expr>, ScilabLowerError> {
        if depth > MAX_EXPR_DEPTH {
            return Err(self.err_at(
                call_suffix,
                format!("expression nesting too deep (exceeds {MAX_EXPR_DEPTH} levels)"),
            ));
        }
        let arg_list = match self.first_child_named(call_suffix, "arg_list") {
            Some(a) => a,
            None => return Ok(vec![]),
        };
        let mut out = Vec::new();
        for arg in child_nodes(arg_list) {
            if arg.rule_name != "arg" {
                continue;
            }
            if let Some(tok) = arg.token() {
                if tok.value == ":" {
                    return Err(self.err_at(
                        arg,
                        "unsupported: `:` is not a valid function-call argument".to_string(),
                    ));
                }
            }
            let inner = match child_nodes(arg).as_slice() {
                [only] => *only,
                _ => return Err(self.err_at(arg, "malformed call argument".to_string())),
            };
            out.push(self.lower_expr_d(inner, ctx, depth + 1)?);
        }
        Ok(out)
    }

    // -------------------------------------------------------------------
    // target resolution (assignment LHS)
    // -------------------------------------------------------------------

    fn bare_name(&self, node: &GrammarASTNode) -> Option<String> {
        let postfix = self.peel_to_named(node, "postfix", 0)?;
        match child_nodes(postfix).as_slice() {
            [primary] => self.primary_bare_name(primary),
            _ => None,
        }
    }

    fn indexed_target<'a>(
        &self,
        node: &'a GrammarASTNode,
    ) -> Option<(String, &'a GrammarASTNode)> {
        let postfix = self.peel_to_named(node, "postfix", 0)?;
        match child_nodes(postfix).as_slice() {
            [primary, suffix] if suffix.rule_name == "call_suffix" => {
                self.primary_bare_name(primary).map(|name| (name, *suffix))
            }
            _ => None,
        }
    }

    fn primary_bare_name(&self, primary: &GrammarASTNode) -> Option<String> {
        let tok = primary.token()?;
        if tok.effective_type_name() == "NAME" {
            Some(tok.value.clone())
        } else {
            None
        }
    }

    // -------------------------------------------------------------------
    // small tree helpers
    // -------------------------------------------------------------------

    /// Peel through a chain of single-Node-child wrapper rules until
    /// reaching a node named `name`, or return `None` if the chain
    /// branches or runs out of depth first.
    fn peel_to_named<'a>(
        &self,
        node: &'a GrammarASTNode,
        name: &str,
        depth: usize,
    ) -> Option<&'a GrammarASTNode> {
        if depth > MAX_EXPR_DEPTH {
            return None;
        }
        if node.rule_name == name {
            return Some(node);
        }
        match child_nodes(node).as_slice() {
            [only] if node.children.len() == 1 => self.peel_to_named(only, name, depth + 1),
            _ => None,
        }
    }

    fn first_child_named<'a>(
        &self,
        node: &'a GrammarASTNode,
        kind: &str,
    ) -> Option<&'a GrammarASTNode> {
        child_nodes(node).into_iter().find(|n| n.rule_name == kind)
    }

    fn span_of(&self, node: &GrammarASTNode) -> Span {
        Span::point(
            FILE,
            node.start_line.unwrap_or(1),
            node.start_column.unwrap_or(1),
        )
    }

    fn err_at(&self, node: &GrammarASTNode, message: String) -> ScilabLowerError {
        ScilabLowerError {
            message,
            line: node.start_line.unwrap_or(1),
            column: node.start_column.unwrap_or(1),
        }
    }

    /// Err if `e` is a direct string literal -- see this file's module doc
    /// comment, "No arithmetic or ordering over string literals", for the
    /// full rationale (MA10 §1 finding 1) and the disclosed variable-blind-
    /// spot limitation.
    fn reject_string_operand(
        &self,
        e: &Expr,
        node: &GrammarASTNode,
        op_desc: &str,
    ) -> Result<(), ScilabLowerError> {
        if matches!(e, Expr::StrLit { .. }) {
            return Err(self.err_at(
                node,
                format!(
                    "unsupported: {op_desc} is not implemented over string operands in this cut \
                     (MA10 §4) -- Scilab's own `+` means concatenation, not numeric addition, \
                     and this frontend does not guess at string-operator semantics without a \
                     typed-dispatch layer"
                ),
            ));
        }
        Ok(())
    }

    /// A `NUMBER` lexeme is a float if it has a decimal point or exponent,
    /// otherwise an int; an integer lexeme too large for `i64` falls back
    /// to a float rather than silently truncating or erroring. Must be an
    /// instance method (not a free function) so every `FloatLit`-
    /// constructing branch can call `self.observed.add(Feature::Floats)`
    /// immediately -- see `matlab_to_semantic_ir::number_literal_expr`'s
    /// doc comment for the confirmed bug this discipline exists to avoid
    /// repeating.
    fn number_literal_expr(&mut self, tok: &Token, span: &Span) -> Expr {
        let text = &tok.value;
        if text.contains('.') || text.contains('e') || text.contains('E') {
            self.observed.add(Feature::Floats);
            Expr::FloatLit {
                value: text.parse::<f64>().unwrap_or(0.0),
                span: span.clone(),
            }
        } else {
            match text.parse::<i64>() {
                Ok(v) => Expr::IntLit {
                    value: v,
                    span: span.clone(),
                },
                Err(_) => {
                    self.observed.add(Feature::Floats);
                    Expr::FloatLit {
                        value: text.parse::<f64>().unwrap_or(0.0),
                        span: span.clone(),
                    }
                }
            }
        }
    }

    /// Reject a same-precedence operator chain with more than
    /// `MAX_EXPR_DEPTH` operands -- mirrors
    /// `matlab_to_semantic_ir::check_chain_length` exactly (see that
    /// function's own extensive doc comment for the full DoS rationale:
    /// Scilab's grammar collapses a flat run of `+`/`-`/`*`/... into one
    /// CST node with many children too, via the identical `{ x }`
    /// repetition shape, so the same unbounded-fold-depth hazard applies
    /// here verbatim).
    fn check_chain_length(&self, node: &GrammarASTNode) -> Result<(), ScilabLowerError> {
        let operand_count = node
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(_)))
            .count();
        if operand_count > MAX_EXPR_DEPTH {
            return Err(self.err_at(
                node,
                format!(
                    "expression chain too long ({operand_count} operands, exceeds \
                     {MAX_EXPR_DEPTH})"
                ),
            ));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Free helpers
// ---------------------------------------------------------------------------

/// Collect the *node* children of `node` (dropping tokens).
fn child_nodes(node: &GrammarASTNode) -> Vec<&GrammarASTNode> {
    node.children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Node(n) => Some(n),
            ASTNodeOrToken::Token(_) => None,
        })
        .collect()
}

/// The sole node child of `node`, erroring (via the lowerer's own
/// `err_at`) if there is not exactly one. Mirrors
/// `scilab-runtime::eval::only_node`'s "first Node child found" shape but
/// returns a proper [`ScilabLowerError`] instead of a bare `String`.
fn only_node<'a>(node: &'a GrammarASTNode, lowerer: &Lowerer) -> Result<&'a GrammarASTNode, ScilabLowerError> {
    match child_nodes(node).as_slice() {
        [only] => Ok(*only),
        _ => Err(lowerer.err_at(node, format!("malformed `{}` node", node.rule_name))),
    }
}

/// Is `e` provably a scalar? See this file's module doc comment's
/// "Scalar/array disambiguation" section -- mirrors
/// `matlab_to_semantic_ir::expr_is_known_scalar` exactly, including its
/// own depth-capped core (defense in depth against re-deriving scalar-ness
/// by re-walking a growing accumulator, which no call site in this file
/// does -- see `matlab_to_semantic_ir::expr_is_known_scalar_d`'s doc
/// comment for the full rationale).
fn expr_is_known_scalar(e: &Expr) -> bool {
    expr_is_known_scalar_d(e, 0)
}

fn expr_is_known_scalar_d(e: &Expr, depth: usize) -> bool {
    if depth > MAX_EXPR_DEPTH {
        return false;
    }
    match e {
        Expr::IntLit { .. } | Expr::FloatLit { .. } => true,
        Expr::BuiltinCall { name, args, .. }
            if matches!(name.as_str(), "+" | "-" | "*" | "/" | "neg") =>
        {
            args.iter().all(|a| expr_is_known_scalar_d(a, depth + 1))
        }
        _ => false,
    }
}

/// Coerce an already-lowered Scilab expression to genuine Scilab
/// truthiness at the point it reaches a boolean context: an `if`/`while`
/// condition (including a desugared `select`/`case` comparison), the
/// operand of unary `~`, or an operand of `&&`/`||`/`&`/`|`.
///
/// Scilab's own truthiness rule for numeric values is byte-for-byte
/// identical to MATLAB's ("nonzero is true" -- confirmed directly against
/// `scilab-runtime::value::ScilabValue::is_true`'s numeric arm, which is
/// the same rule `matlab_runtime::MatValue::is_true` uses). This function
/// therefore reuses the *exact* runtime intrinsic
/// `matlab_to_semantic_ir::to_matlab_condition` already established --
/// `BuiltinCall("matlab_truthy", [expr])`, which `semantic-ir-to-javascript`
/// already implements as `typeof x === "boolean" ? x : (numOf(x) !== 0)` --
/// rather than inventing a same-shaped `"scilab_truthy"` builtin the shared
/// JS backend would need a matching, currently-nonexistent implementation
/// for. Reusing the existing name is deliberate, not an oversight: this
/// crate's own `Cargo.toml` does not (and should not) depend on
/// `matlab-to-semantic-ir` -- `"matlab_truthy"` here is just a well-known
/// SIR *builtin name* string, the same kind of cross-frontend reuse this
/// repo already does for `"neg"`/`"print"`/etc. (`apl-to-semantic-ir`'s own
/// `apply_monadic_scalar` doc comment states the identical "a `BuiltinCall`
/// name is a generic operation any backend implements polymorphically"
/// convention).
///
/// One disclosed gap: Scilab's *string* truthiness ("a non-empty string is
/// true" -- `ScilabValue::is_true`'s `Str` arm) is **not** represented by
/// this reuse (`matlabTruthy`'s own runtime implementation only knows how
/// to coerce a *number* or an already-genuine boolean, never a string).
/// This is accepted because MA10 §4 never lists "a string used directly as
/// an `if`/`while`/`select` condition" as in-scope surface at all -- MA10
/// §4's string surface is explicitly "assignment, display, and equality
/// only" -- so there is no in-scope construct this gap could silently
/// mis-lower; it would only matter for a construct already outside this
/// cut's own stated scope.
fn to_scilab_condition(expr: Expr) -> Expr {
    let span = expr.span().clone();
    Expr::BuiltinCall {
        name: "matlab_truthy".to_string(),
        args: vec![expr],
        effects: EffectSet::PURE,
        span,
    }
}

/// An empty `Block` whose value is `NilLit`.
fn empty_block(span: Span) -> Block {
    Block {
        stmts: vec![],
        value: Expr::NilLit { span: span.clone() },
        span,
    }
}

/// A `Block` with no statements whose value is `expr`.
fn value_block(expr: Expr) -> Block {
    let span = expr.span().clone();
    Block {
        stmts: vec![],
        value: expr,
        span,
    }
}

/// Assemble a list of lowered items into a `Block` whose every item is a
/// statement (bare expressions wrapped as `ExprStmt`) and whose value is
/// always `value` -- unlike a script-oriented frontend, Scilab has no
/// "trailing expression is the result" convention at any body level.
fn assemble_stmts_only(items: Vec<Lowered>, value: Expr, span: Span) -> Block {
    let stmts: Vec<Stmt> = items
        .into_iter()
        .map(|item| match item {
            Lowered::Stmt(s) => *s,
            Lowered::Expr(expr) => {
                let s = expr.span().clone();
                Stmt::ExprStmt { expr, span: s }
            }
        })
        .collect();
    Block { stmts, value, span }
}
