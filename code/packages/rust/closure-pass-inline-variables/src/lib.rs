//! Constant-propagation pass for the Closure Compiler clone.
//!
//! Per [CLOC06](../../../specs/CLOC06-pass-interface-contract.md)'s
//! canonical pass set. Replaces references to a top-level `const`
//! binding whose value is a literal with the literal itself:
//!
//! ```js
//! // before
//! const RATE = 2;
//! total = base * RATE;
//!
//! // after inline-variables
//! const RATE = 2;          // now unreferenced …
//! total = base * 2;        // … and removed by remove-unused-vars,
//!                          //     after which constant-fold can fold
//!                          //     `base * 2` further if `base` is known
//! ```
//!
//! This is Closure Compiler's `InlineVariables` in miniature: pull a
//! constant's value to its use sites so the binding can be deleted and
//! downstream folding sees a concrete literal instead of a name.
//!
//! # Why this is safe — and why only `const`, only literals
//!
//! Propagating `X`'s value to a use site is sound only when that value
//! is the SAME at every use as it was at the declaration. Two things
//! guarantee that here:
//!
//! 1. **`const`, never `let`/`var`.** A `const` binding cannot be
//!    reassigned, so its value never changes after initialization. A
//!    `let`/`var` could be written between the declaration and a use
//!    (`let x = 1; … x = 2; … use(x)`), so propagating its initializer
//!    would be wrong. We touch only `const`.
//! 2. **A literal value, never an expression.** A literal
//!    (`5`, `"s"`, `true`, `null`, …) is immutable and has no
//!    sub-terms that could change. `const X = y;` is NOT propagated:
//!    `y` is a *variable* whose value at a later use could differ from
//!    its value when `X` was bound (copy propagation needs
//!    reaching-definitions analysis we don't do yet). `const X = o.p;`
//!    is not propagated either — reading `o.p` may trigger a getter.
//!
//! Add the same self-contained **shadow guard** the inline pass uses —
//! the name must be declared *exactly once in the whole program* — and
//! every occurrence of the identifier `X` provably refers to this one
//! `const`, so substituting the literal at each is a sound rewrite.
//!
//! # The temporal dead zone (why a use can throw)
//!
//! Scope resolution is not the whole story. A `const` has a *temporal
//! dead zone*: reading `X` before its declaration line executes throws
//! `ReferenceError`, even from a function whose body textually follows
//! the declaration but is *called* earlier. Replacing such a read with
//! the inert literal would erase a throw the original program performs —
//! unsound. Example:
//!
//! ```js
//! function read() { return X; }
//! const r = read();   // runs while X is in its TDZ → ReferenceError
//! const X = 5;
//! ```
//!
//! We guard against this conservatively: a `const X` is a candidate only
//! when **every top-level item before its declaration is inert** — a
//! function declaration (hoists, runs nothing) or a variable declaration
//! with only literal initializers (binds a constant, runs nothing). Then
//! no code executes before `X` initializes, so nothing can read it in
//! its TDZ. The common shape — a block of constants at the top of the
//! file, used by code and functions below — is fully covered; a `const`
//! sitting after any executable statement is left alone. (We also take
//! only single-declarator `const`s, so an earlier sibling in the same
//! declaration can't sneak a call in before `X`.)
//!
//! # Single-use vs. multi-use (the "is it worth it?" knob)
//!
//! Soundness aside, propagating a literal is a size win when the
//! literal isn't much larger than the name it replaces, *plus* the
//! whole `const X = …;` declaration disappears:
//!
//!   * **One use** → always propagate (the declaration is pure
//!     overhead once its single use is gone).
//!   * **N > 1 uses** → propagate only when the literal is short
//!     (`literal_cost <= MAX_MULTIUSE_LITERAL_LEN`), so duplicating it
//!     across the uses doesn't outweigh deleting the declaration. A
//!     long string constant used in twenty places stays a `const`.
//!
//! The pass itself only *propagates*; it leaves the now-unreferenced
//! `const` declaration in place. `remove-unused-vars` (which runs
//! after it and already deletes unreferenced top-level bindings with
//! literal initializers) removes the husk. Keeping the two concerns
//! separate mirrors how the inline pass leaves dead functions for
//! treeshake.
//!
//! # Where this pass sits
//!
//! `depends_on = ["constant-fold"]` so a folded initializer
//! (`const X = 1 + 2` → `const X = 3`) is a literal by the time this
//! pass looks at it. It runs before `remove-unused-vars` (which clears
//! the emptied declaration) and feeds `constant-fold` on the next
//! fixed-point sweep (`base * 2` with a now-literal operand).

use std::collections::HashMap;

use coding_adventures_closure_pass_pipeline::{
    IterationPolicy, Pass, PassContext, PassError, PassOutput, PassStats,
};
use coding_adventures_correlation_vector::Contribution;
use serde_json::json;
use coding_adventures_javascript_ast::statement::TaggedStatement;
use coding_adventures_javascript_ast::{
    ArrowBody, AssignmentTarget, BindingTarget, ClassMember, Declaration, Expression, ForInit,
    Program, ProgramItem, ObjectMember, PropertyKey, Statement, VarKind,
    VariableDeclaration,
};

/// `Pass::depends_on` value — constant-fold first, so a folded
/// initializer is already a literal when we scan for candidates.
const DEPS: &[&str] = &["constant-fold"];

/// A literal used at more than one site is propagated only when its
/// emitted form is at most this many bytes — short enough that
/// duplicating it across the uses is outweighed by deleting the whole
/// `const` declaration. Numbers, booleans, `null`, and short strings
/// clear this; long string/number constants used many times do not.
const MAX_MULTIUSE_LITERAL_LEN: usize = 8;

/// Constant-propagation pass. See crate-level docs for the exact
/// (provably-safe) slice it implements.
///
/// Zero-sized type: no per-instance state. Pass-internal state (the
/// candidate map, the per-name substitution) lives in pass-local maps
/// constructed inside [`Pass::run`] per CLOC06 §"Pass-internal state."
#[derive(Debug, Default, Clone, Copy)]
pub struct InlineVariablesPass;

impl InlineVariablesPass {
    /// Zero-arg constructor for ergonomic
    /// `PassPipeline::add(Box::new(InlineVariablesPass::new()))`.
    pub fn new() -> Self {
        Self
    }
}

impl Pass for InlineVariablesPass {
    fn name(&self) -> &'static str {
        "inline-variables"
    }

    fn depends_on(&self) -> &[&'static str] {
        // constant-fold first: a folded initializer (`const X = 1 + 2`
        // → `const X = 3`) is then a literal we can propagate.
        DEPS
    }

    fn iteration_policy(&self) -> IterationPolicy {
        // FixedPoint: propagating a constant exposes new folding
        // (`base * RATE` → `base * 2`) and the emptied declaration for
        // remove-unused-vars. Re-running converges — once a `const`'s
        // uses are gone the next pass finds zero sites and reports no
        // change.
        IterationPolicy::FixedPoint
    }

    fn cost(&self) -> u32 {
        // A shadow-count walk, a per-candidate use count, and a
        // substitution walk — comparable to DCE's mark+sweep.
        3
    }

    fn run(&self, ctx: PassContext<'_>) -> Result<PassOutput, PassError> {
        let mut program = ctx.program.clone();
        let mut nodes_touched: u32 = 1; // the program root
        let mut propagated: Vec<PropagatedConst> = Vec::new();
        let changed =
            inline_variables_program(&mut program, &mut nodes_touched, &mut propagated);

        // CV provenance (#89): record every constant we propagated as a
        // `propagated` contribution carrying `{name, value, sites}` — the
        // original `const` name, a compact rendering of its literal value,
        // and how many use sites the literal was substituted into.
        // Propagation *dissolves* the binding: its declaration becomes
        // unreferenced (remove-unused-vars deletes it) and the literal is
        // copied to each reader, so without this record the minified output
        // has no trace that a named constant ever stood there. The pipeline
        // attaches these to the program-root CV entry, so a
        // `--correlation_vector` consumer can map an inlined literal back to
        // the `const` it came from. Records come out in program (source)
        // order, one per propagated constant, so the emitted list is
        // deterministic run to run.
        //
        // This is the propagation *table* (name → value/site-count); tagging
        // each substituted literal's OWN CV id is a documented follow-up,
        // mirroring the inline / rename passes.
        let contributions: Vec<Contribution> = propagated
            .into_iter()
            .map(|p| Contribution {
                source: "inline-variables".to_string(),
                tag: "propagated".to_string(),
                meta: [
                    ("name".to_string(), json!(p.name)),
                    ("value".to_string(), json!(p.value)),
                    ("sites".to_string(), json!(p.sites)),
                ]
                .into_iter()
                .collect(),
            })
            .collect();

        Ok(PassOutput {
            program,
            contributions,
            changed,
            diagnostics: Vec::new(),
            stats: PassStats { nodes_touched },
        })
    }
}

// =========================================================================
// Constant-propagation implementation (top-level `const = literal`)
// =========================================================================

/// One propagatable constant: its name and a clone of the literal value
/// to substitute at each use site.
struct ConstCandidate {
    name: String,
    value: Expression,
}

/// One propagation event for CV provenance (#89): the original `const`
/// name, a compact rendering of its literal value, and how many use sites
/// the literal replaced. `run` turns each into a `propagated` contribution.
struct PropagatedConst {
    name: String,
    value: String,
    sites: usize,
}

/// A compact source-like rendering of a literal for the `value` meta field.
/// Covers exactly the variants [`is_literal`] admits; anything else is
/// unreachable here but rendered as `"?"` defensively.
fn literal_repr(expr: &Expression) -> String {
    match expr {
        Expression::NumericLiteral(n) => n.raw.clone(),
        Expression::StringLiteral(s) => format!("{:?}", s.value), // quoted
        Expression::BooleanLiteral(b) => b.value.to_string(),
        Expression::NullLiteral(_) => "null".to_string(),
        Expression::BigIntLiteral(b) => b.raw.clone(),
        Expression::UndefinedLiteral(_) => "undefined".to_string(),
        _ => "?".to_string(),
    }
}

/// Walk the whole program and propagate every qualifying top-level
/// `const = literal`. Returns whether anything changed. `propagated`
/// accumulates each propagation for CV provenance.
fn inline_variables_program(
    program: &mut Program,
    nodes_touched: &mut u32,
    propagated: &mut Vec<PropagatedConst>,
) -> bool {
    // Phase 1 — count how many times each name is declared as a binding
    // anywhere in the program (function names, parameters, var/let/const
    // targets). A candidate's name must be declared exactly once, so no
    // other scope shadows it and every use of the identifier resolves to
    // this `const`.
    let mut decl_counts: HashMap<String, usize> = HashMap::new();
    count_decl_names_program(program, &mut decl_counts, nodes_touched);

    // Phase 2 — collect candidates from top-level `const` declarators
    // whose initializer is a literal and whose name is declared once.
    let mut candidates: Vec<ConstCandidate> = Vec::new();
    for (idx, item) in program.body.iter().enumerate() {
        // The bridge emits a top-level declaration either as a bare
        // `ProgramItem::Declaration` or wrapped in
        // `ProgramItem::Statement(Statement::Declaration(..))`; handle
        // both shapes (same lesson as remove-unused-vars).
        let vd = match item {
            ProgramItem::Declaration(Declaration::VariableDeclaration(vd)) => vd,
            ProgramItem::Statement(Statement::Declaration(Declaration::VariableDeclaration(
                vd,
            ))) => vd,
            _ => continue,
        };
        if !matches!(vd.kind, VarKind::Const) {
            continue; // let/var can be reassigned — not safe to propagate
        }
        // Single declarator only. A multi-declarator `const A = f(), X = 2`
        // evaluates earlier siblings (which may call code that reads X)
        // before X initializes — the TDZ scan below only looks at whole
        // top-level items, not within a declaration, so we skip these.
        if vd.declarations.len() != 1 {
            continue;
        }
        let d = &vd.declarations[0];
        let BindingTarget::Identifier(id) = &d.id;
        if decl_counts.get(&id.name).copied().unwrap_or(0) != 1 {
            continue; // shadowed somewhere — can't resolve uses by name
        }
        let init = match &d.init {
            Some(i) => i,
            None => continue,
        };
        if !is_literal(init) {
            continue;
        }
        // Temporal-dead-zone guard. A top-level `const` cannot be read
        // before its declaration line runs — doing so throws
        // `ReferenceError`. If anything *executes* before this line (a
        // top-level call, or any statement that runs code), a function it
        // reaches could read the binding in its TDZ; propagating the
        // literal would erase that throw. We require every preceding
        // top-level item to be *inert* — a function declaration (hoisted,
        // doesn't run) or a variable declaration with only literal
        // initializers (binds a constant, runs no code). Then nothing has
        // read the binding by the time its declaration initializes it.
        if !prefix_is_inert(&program.body[..idx]) {
            continue;
        }
        candidates.push(ConstCandidate {
            name: id.name.clone(),
            value: init.clone(),
        });
    }
    if candidates.is_empty() {
        return false;
    }

    // Phase 3 — propagate. For each candidate, count its uses; propagate
    // when there is at least one use and either it is single-use or the
    // literal is short enough for the multi-use budget.
    let mut changed = false;
    for cand in &candidates {
        let uses = count_uses_program(program, &cand.name);
        if uses == 0 {
            continue; // nothing to propagate (remove-unused-vars will drop it)
        }
        if uses > 1 && literal_cost(&cand.value) > MAX_MULTIUSE_LITERAL_LEN {
            continue; // literal too large to duplicate across the uses
        }
        if propagate_all(program, cand) {
            changed = true;
            // CV: the literal was substituted into all `uses` sites (gate
            // above guarantees `uses > 0`), after which the `const`
            // declaration is unreferenced.
            propagated.push(PropagatedConst {
                name: cand.name.clone(),
                value: literal_repr(&cand.value),
                sites: uses,
            });
        }
    }
    changed
}

/// Are all of the top-level `items` *inert* — do they run no code that
/// could read a binding before it initializes?
///
/// An item is inert when it is a **function declaration** (hoisted; its
/// body doesn't execute until called) or a **variable declaration whose
/// every initializer is a literal** (binds a constant, runs nothing).
/// Anything else — an expression statement, an `if`/loop, a `var` with a
/// call/expression initializer — *executes*, and what it executes might
/// read a not-yet-initialized `const`, so it blocks propagation (the TDZ
/// guard). See the call site in [`inline_variables_program`].
fn prefix_is_inert(items: &[ProgramItem]) -> bool {
    items.iter().all(|item| match item {
        ProgramItem::Declaration(d) => decl_is_inert(d),
        ProgramItem::Statement(Statement::Declaration(d)) => decl_is_inert(d),
        // An expression statement, control-flow, etc. runs code.
        ProgramItem::Statement(_) => false,
    })
}

/// A declaration is inert (runs no code at the point it appears) when it
/// is a function declaration or a variable declaration with only literal
/// (or absent) initializers. See [`prefix_is_inert`].
fn decl_is_inert(decl: &Declaration) -> bool {
    match decl {
        // A function declaration only hoists here; its body runs when the
        // function is called, which (given every preceding item is inert)
        // cannot happen before a later `const` initializes.
        Declaration::FunctionDeclaration(_) => true,
        // A class declaration is NOT inert: evaluating its `extends` heritage
        // (`class C extends f() {}`) and creating the class runs code at the
        // declaration site — unlike a function declaration, which only hoists.
        // Conservatively (and correctly) treat it as running code.
        Declaration::ClassDeclaration(_) => false,
        // An import declaration runs the target module for its side effects, so
        // it is NOT inert — it can observe/mutate state before a later `const`.
        Declaration::ImportDeclaration(_) => false,
        Declaration::ExportNamedDeclaration(_) => false,
        Declaration::ExportDefaultDeclaration(_) => false,
        Declaration::ExportAllDeclaration(_) => false,
        Declaration::VariableDeclaration(vd) => vd.declarations.iter().all(|d| match &d.init {
            None => true,
            Some(init) => is_literal(init),
        }),
    }
}

/// Is `expr` an immutable literal we can safely duplicate?
fn is_literal(expr: &Expression) -> bool {
    matches!(
        expr,
        Expression::NumericLiteral(_)
            | Expression::StringLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_)
            | Expression::BigIntLiteral(_)
            | Expression::UndefinedLiteral(_)
    )
}

/// Approximate emitted byte length of a literal — the multi-use budget
/// gate. Over-estimating is safe (it just declines a borderline win).
fn literal_cost(expr: &Expression) -> usize {
    match expr {
        Expression::NumericLiteral(n) => n.raw.len().max(1),
        Expression::StringLiteral(s) => s.value.len() + 2, // surrounding quotes
        Expression::BooleanLiteral(b) => {
            if b.value {
                4
            } else {
                5
            }
        }
        Expression::NullLiteral(_) => 4,
        Expression::BigIntLiteral(b) => b.raw.len().max(1),
        Expression::UndefinedLiteral(_) => 9,
        // Non-literals never reach here (gated by `is_literal`), but be
        // safe: treat as "too large" so they are never multi-propagated.
        _ => usize::MAX,
    }
}

// ---- name-declaration counting (shadow detection) ------------------------

/// Count every binding-name *declaration* in the whole program —
/// function names, parameters, and `var`/`let`/`const` targets,
/// recursing into nested function bodies — accumulating occurrence
/// counts into `out`. `nodes_touched` is bumped per statement.
fn count_decl_names_program(
    program: &Program,
    out: &mut HashMap<String, usize>,
    nodes_touched: &mut u32,
) {
    for item in &program.body {
        match item {
            ProgramItem::Declaration(d) => count_decl_names_decl(d, out, nodes_touched),
            ProgramItem::Statement(s) => count_decl_names_stmt(s, out, nodes_touched),
        }
    }
}

fn count_decl_names_decl(
    decl: &Declaration,
    out: &mut HashMap<String, usize>,
    nodes_touched: &mut u32,
) {
    *nodes_touched += 1;
    match decl {
        Declaration::VariableDeclaration(vd) => count_decl_names_var(vd, out),
        Declaration::FunctionDeclaration(fd) => {
            *out.entry(fd.id.name.clone()).or_insert(0) += 1;
            for p in &fd.params {
                let id = p.binding_identifier();
                *out.entry(id.name.clone()).or_insert(0) += 1;
            }
            for s in &fd.body.body {
                count_decl_names_stmt(s, out, nodes_touched);
            }
        }
        // A class declaration binds its name (`cd.id`), and its method bodies
        // declare their own locals — count both, mirroring the function
        // declaration arm (name + body-declared names). Counting more names is
        // conservative: it only ever prevents an unsafe inline, never causes one.
        // An import declaration binds names but holds no expressions to
        // count/propagate through — nothing to do.
        Declaration::ImportDeclaration(_) => {}
        Declaration::ExportNamedDeclaration(_) => {}
        Declaration::ExportDefaultDeclaration(_) => {}
        Declaration::ExportAllDeclaration(_) => {}
        Declaration::ClassDeclaration(cd) => {
            *out.entry(cd.id.name.clone()).or_insert(0) += 1;
            for member in &cd.body {
                match member {
                    ClassMember::Method(m) => {
                        for s in &m.value.body.body {
                            count_decl_names_stmt(s, out, nodes_touched);
                        }
                    }
                    // A field initializer is an expression — it declares no
                    // statement-scope names at the class-body level.
                    ClassMember::Field(_) => {}
                    // A static-init block's statements declare their own locals —
                    // count them conservatively, mirroring the method-body arm
                    // (over-counting only ever prevents an unsafe inline).
                    ClassMember::StaticBlock(b) => {
                        for s in &b.body {
                            count_decl_names_stmt(s, out, nodes_touched);
                        }
                    }
                }
            }
        }
    }
}

fn count_decl_names_var(vd: &VariableDeclaration, out: &mut HashMap<String, usize>) {
    for d in &vd.declarations {
        let BindingTarget::Identifier(id) = &d.id;
        *out.entry(id.name.clone()).or_insert(0) += 1;
    }
}

fn count_decl_names_stmt(
    stmt: &Statement,
    out: &mut HashMap<String, usize>,
    nodes_touched: &mut u32,
) {
    *nodes_touched += 1;
    match stmt {
        Statement::Declaration(d) => count_decl_names_decl(d, out, nodes_touched),
        Statement::Tagged(t) => match t {
            TaggedStatement::BlockStatement(b) => {
                for s in &b.body {
                    count_decl_names_stmt(s, out, nodes_touched);
                }
            }
            TaggedStatement::IfStatement(is) => {
                count_decl_names_stmt(&is.consequent, out, nodes_touched);
                if let Some(alt) = &is.alternate {
                    count_decl_names_stmt(alt, out, nodes_touched);
                }
            }
            TaggedStatement::WhileStatement(ws) => {
                count_decl_names_stmt(&ws.body, out, nodes_touched)
            }
            TaggedStatement::WithStatement(ws) => {
                count_decl_names_stmt(&ws.body, out, nodes_touched)
            }
            TaggedStatement::DoWhileStatement(ds) => {
                count_decl_names_stmt(&ds.body, out, nodes_touched)
            }
            TaggedStatement::ForStatement(fs) => {
                if let Some(ForInit::VariableDeclaration(vd)) = &fs.init {
                    count_decl_names_var(vd, out);
                }
                count_decl_names_stmt(&fs.body, out, nodes_touched);
            }
            TaggedStatement::ForInStatement(fs) => {
                // The for-in `left`, when a declaration, binds the loop variable.
                if let ForInit::VariableDeclaration(vd) = &fs.left {
                    count_decl_names_var(vd, out);
                }
                count_decl_names_stmt(&fs.body, out, nodes_touched);
            }
            TaggedStatement::ForOfStatement(fs) => {
                // The for-in `left`, when a declaration, binds the loop variable.
                if let ForInit::VariableDeclaration(vd) = &fs.left {
                    count_decl_names_var(vd, out);
                }
                count_decl_names_stmt(&fs.body, out, nodes_touched);
            }
            TaggedStatement::LabeledStatement(ls) => {
                count_decl_names_stmt(&ls.body, out, nodes_touched)
            }
            TaggedStatement::SwitchStatement(ss) => {
                for c in &ss.cases {
                    for s in &c.consequent {
                        count_decl_names_stmt(s, out, nodes_touched);
                    }
                }
            }
            TaggedStatement::TryStatement(ts) => {
                // The catch `param` IS a binding — count it so it participates
                // in the shadow guard (a top-level const sharing its name is
                // not "uniquely declared"). Recurse into the three blocks.
                for s in &ts.block.body {
                    count_decl_names_stmt(s, out, nodes_touched);
                }
                if let Some(h) = &ts.handler {
                    if let Some(param) = &h.param {
                        *out.entry(param.name.clone()).or_insert(0) += 1;
                    }
                    for s in &h.body.body {
                        count_decl_names_stmt(s, out, nodes_touched);
                    }
                }
                if let Some(f) = &ts.finalizer {
                    for s in &f.body {
                        count_decl_names_stmt(s, out, nodes_touched);
                    }
                }
            }
            // Statements that introduce no binding in the Phase-1 AST.
            // Exhaustive on purpose: a future binding-introducing
            // statement (a `class` declaration) must be handled here so a
            // shadowing name can't slip past the shadow guard. The compiler
            // flags the omission.
            TaggedStatement::ExpressionStatement(_)
            | TaggedStatement::ReturnStatement(_)
            | TaggedStatement::ThrowStatement(_)
            | TaggedStatement::BreakStatement(_)
            | TaggedStatement::ContinueStatement(_)
            | TaggedStatement::EmptyStatement(_)
            | TaggedStatement::DebuggerStatement(_) => {}
        },
    }
}

// ---- use counting --------------------------------------------------------

/// Count the *uses* (binding-use-position occurrences) of `name` across
/// the whole program — the occurrences [`propagate_all`] will rewrite.
/// Declarations, property names, label names, and assignment *targets*
/// are not counted (a `const` is never an assignment target in valid
/// code, but we skip them defensively — you cannot assign to a literal).
fn count_uses_program(program: &Program, name: &str) -> usize {
    let mut count = 0;
    for item in &program.body {
        match item {
            ProgramItem::Declaration(d) => count_uses_decl(d, name, &mut count),
            ProgramItem::Statement(s) => count_uses_stmt(s, name, &mut count),
        }
    }
    count
}

fn count_uses_decl(decl: &Declaration, name: &str, count: &mut usize) {
    match decl {
        Declaration::VariableDeclaration(vd) => {
            for d in &vd.declarations {
                if let Some(init) = &d.init {
                    count_uses_expr(init, name, count);
                }
            }
        }
        Declaration::FunctionDeclaration(fd) => {
            for s in &fd.body.body {
                count_uses_stmt(s, name, count);
            }
        }
        // A class declaration can USE `name` in its `extends` operand
        // (`class C extends name {}`) and inside its method bodies. We MUST
        // count every such use — missing one would let the pass inline/remove
        // `name` while the class still references it (a miscompile). Mirrors the
        // `Expression::ClassExpression` arm of `count_uses_expr`.
        // An import declaration binds names but holds no expressions to
        // count/propagate through — nothing to do.
        Declaration::ImportDeclaration(_) => {}
        Declaration::ExportNamedDeclaration(_) => {}
        Declaration::ExportDefaultDeclaration(_) => {}
        Declaration::ExportAllDeclaration(_) => {}
        Declaration::ClassDeclaration(cd) => {
            if let Some(sup) = &cd.super_class {
                count_uses_expr(sup, name, count);
            }
            for member in &cd.body {
                match member {
                    ClassMember::Method(m) => {
                        for s in &m.value.body.body {
                            count_uses_stmt(s, name, count);
                        }
                    }
                    // A field initializer can USE `name` (`x = name`) — count
                    // it, else the pass could inline/remove a still-referenced
                    // binding (a miscompile).
                    ClassMember::Field(f) => {
                        if let Some(v) = &f.value {
                            count_uses_expr(v, name, count);
                        }
                    }
                    // SOUNDNESS: a static-init block's statements run at class-
                    // definition time and can USE `name` — count every such use,
                    // mirroring the method-body arm.
                    ClassMember::StaticBlock(b) => {
                        for s in &b.body {
                            count_uses_stmt(s, name, count);
                        }
                    }
                }
            }
        }
    }
}

fn count_uses_stmt(stmt: &Statement, name: &str, count: &mut usize) {
    match stmt {
        Statement::Declaration(d) => count_uses_decl(d, name, count),
        Statement::Tagged(t) => match t {
            TaggedStatement::ExpressionStatement(es) => {
                count_uses_expr(&es.expression, name, count)
            }
            TaggedStatement::BlockStatement(b) => {
                for s in &b.body {
                    count_uses_stmt(s, name, count);
                }
            }
            TaggedStatement::IfStatement(is) => {
                count_uses_expr(&is.test, name, count);
                count_uses_stmt(&is.consequent, name, count);
                if let Some(alt) = &is.alternate {
                    count_uses_stmt(alt, name, count);
                }
            }
            TaggedStatement::WhileStatement(ws) => {
                count_uses_expr(&ws.test, name, count);
                count_uses_stmt(&ws.body, name, count);
            }
            TaggedStatement::WithStatement(ws) => {
                count_uses_expr(&ws.object, name, count);
                count_uses_stmt(&ws.body, name, count);
            }
            TaggedStatement::DoWhileStatement(ds) => {
                count_uses_expr(&ds.test, name, count);
                count_uses_stmt(&ds.body, name, count);
            }
            TaggedStatement::ForStatement(fs) => {
                if let Some(init) = &fs.init {
                    match init {
                        ForInit::VariableDeclaration(vd) => {
                            for d in &vd.declarations {
                                if let Some(i) = &d.init {
                                    count_uses_expr(i, name, count);
                                }
                            }
                        }
                        ForInit::Expression(e) => count_uses_expr(e, name, count),
                    }
                }
                if let Some(test) = &fs.test {
                    count_uses_expr(test, name, count);
                }
                if let Some(update) = &fs.update {
                    count_uses_expr(update, name, count);
                }
                count_uses_stmt(&fs.body, name, count);
            }
            TaggedStatement::ForInStatement(fs) => {
                match &fs.left {
                    ForInit::VariableDeclaration(vd) => {
                        for d in &vd.declarations {
                            if let Some(i) = &d.init {
                                count_uses_expr(i, name, count);
                            }
                        }
                    }
                    ForInit::Expression(e) => count_uses_expr(e, name, count),
                }
                count_uses_expr(&fs.right, name, count);
                count_uses_stmt(&fs.body, name, count);
            }
            TaggedStatement::ForOfStatement(fs) => {
                match &fs.left {
                    ForInit::VariableDeclaration(vd) => {
                        for d in &vd.declarations {
                            if let Some(i) = &d.init {
                                count_uses_expr(i, name, count);
                            }
                        }
                    }
                    ForInit::Expression(e) => count_uses_expr(e, name, count),
                }
                count_uses_expr(&fs.right, name, count);
                count_uses_stmt(&fs.body, name, count);
            }
            TaggedStatement::ReturnStatement(rs) => {
                if let Some(a) = &rs.argument {
                    count_uses_expr(a, name, count);
                }
            }
            TaggedStatement::ThrowStatement(ts) => count_uses_expr(&ts.argument, name, count),
            TaggedStatement::LabeledStatement(ls) => count_uses_stmt(&ls.body, name, count),
            TaggedStatement::SwitchStatement(ss) => {
                count_uses_expr(&ss.discriminant, name, count);
                for c in &ss.cases {
                    if let Some(test) = &c.test {
                        count_uses_expr(test, name, count);
                    }
                    for s in &c.consequent {
                        count_uses_stmt(s, name, count);
                    }
                }
            }
            TaggedStatement::TryStatement(ts) => {
                // Count `name` uses inside the three blocks. The catch `param`
                // binding-site is not a use; references to it inside the body
                // are counted by recursion (and a candidate can't share its
                // name — the decl-count guard excludes that).
                for s in &ts.block.body {
                    count_uses_stmt(s, name, count);
                }
                if let Some(h) = &ts.handler {
                    for s in &h.body.body {
                        count_uses_stmt(s, name, count);
                    }
                }
                if let Some(f) = &ts.finalizer {
                    for s in &f.body {
                        count_uses_stmt(s, name, count);
                    }
                }
            }
            TaggedStatement::BreakStatement(_)
            | TaggedStatement::ContinueStatement(_)
            | TaggedStatement::EmptyStatement(_)
            | TaggedStatement::DebuggerStatement(_) => {}
        },
    }
}

fn count_uses_expr(expr: &Expression, name: &str, count: &mut usize) {
    match expr {
        Expression::Identifier(id) => {
            if id.name == name {
                *count += 1;
            }
        }
        Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::BigIntLiteral(_)
        | Expression::RegExpLiteral(_)
        // `this` binds no variable name — nothing to count or propagate into.
        | Expression::ThisExpression(_)
        | Expression::Super(_)
        | Expression::NewTarget(_)
        | Expression::ImportMeta(_)
        | Expression::UndefinedLiteral(_) => {}
        Expression::BinaryExpression(be) => {
            count_uses_expr(&be.left, name, count);
            count_uses_expr(&be.right, name, count);
        }
        Expression::LogicalExpression(le) => {
            count_uses_expr(&le.left, name, count);
            count_uses_expr(&le.right, name, count);
        }
        Expression::UnaryExpression(ue) => count_uses_expr(&ue.argument, name, count),
        Expression::UpdateExpression(ue) => count_uses_expr(&ue.argument, name, count),
        Expression::AssignmentExpression(ae) => {
            // The assignment TARGET is a write, not a use we propagate
            // into (you can't assign to a literal). Only the member-
            // object side of a member target, and the right-hand side,
            // are use positions.
            if let AssignmentTarget::MemberExpression(m) = &ae.left {
                count_uses_member(&m.object, &m.property, m.computed, name, count);
            }
            count_uses_expr(&ae.right, name, count);
        }
        Expression::ConditionalExpression(ce) => {
            count_uses_expr(&ce.test, name, count);
            count_uses_expr(&ce.consequent, name, count);
            count_uses_expr(&ce.alternate, name, count);
        }
        Expression::CallExpression(ce) => {
            count_uses_expr(&ce.callee, name, count);
            for a in &ce.arguments {
                count_uses_expr(a, name, count);
            }
        }
        Expression::NewExpression(ne) => {
            count_uses_expr(&ne.callee, name, count);
            for a in &ne.arguments {
                count_uses_expr(a, name, count);
            }
        }
        Expression::SequenceExpression(se) => {
            for e in &se.expressions {
                count_uses_expr(e, name, count);
            }
        }
        Expression::MemberExpression(m) => {
            count_uses_member(&m.object, &m.property, m.computed, name, count)
        }
        // `a?.b` / `a?.[k]` — count uses in object and property exactly as a
        // plain member access.
        Expression::OptionalMemberExpression(m) => {
            count_uses_member(&m.object, &m.property, m.computed, name, count)
        }
        // `a?.()` — count uses in callee and each argument, as for an
        // ordinary call.
        Expression::OptionalCallExpression(ce) => {
            count_uses_expr(&ce.callee, name, count);
            for a in &ce.arguments {
                count_uses_expr(a, name, count);
            }
        }
        // A chain expression transparently wraps its optional-chain spine —
        // descend into the inner expression.
        Expression::ChainExpression(c) => count_uses_expr(&c.expression, name, count),
        Expression::ArrayExpression(ae) => {
            for el in ae.elements.iter().flatten() {
                count_uses_expr(el, name, count);
            }
        }
        Expression::ObjectExpression(oe) => {
            for member in &oe.properties {
                match member {
                    ObjectMember::Property(prop) => {
                        if prop.computed {
                            if let PropertyKey::Expression(e) = &prop.key {
                                count_uses_expr(e, name, count);
                            }
                        }
                        count_uses_expr(&prop.value, name, count);
                    }
                    // Object spread `...expr` — recurse into the spread
                    // argument the same way the property arm recurses into
                    // prop.value.
                    ObjectMember::Spread(s) => {
                        count_uses_expr(&s.argument, name, count);
                    }
                }
            }
        }
        // Count uses of `name` inside a function *value*'s body, exactly
        // as the `FunctionDeclaration` arm above does. Over-counting
        // under shadowing (a param or the fn's own name equal to `name`)
        // is conservative — it only *prevents* an inline, never produces
        // a wrong one — matching the declaration's existing posture.
        Expression::FunctionExpression(fe) => {
            for s in &fe.body.body {
                count_uses_stmt(s, name, count);
            }
        }
        // Count uses of `name` inside a class expression: the `extends`
        // operand is an ordinary use-position expression, and each method's
        // function *value* body is counted exactly like the
        // `FunctionExpression` arm above (over-counting under a method-param
        // shadow is conservative — it only prevents an inline). The method
        // KEY is a property name, not a variable use, so it is not counted.
        Expression::ClassExpression(ce) => {
            if let Some(sup) = &ce.super_class {
                count_uses_expr(sup, name, count);
            }
            for member in &ce.body {
                match member {
                    ClassMember::Method(m) => {
                        for s in &m.value.body.body {
                            count_uses_stmt(s, name, count);
                        }
                    }
                    // A field initializer can use `name` — count it (soundness).
                    ClassMember::Field(f) => {
                        if let Some(v) = &f.value {
                            count_uses_expr(v, name, count);
                        }
                    }
                    // SOUNDNESS: a static-init block's statements can use `name` —
                    // count each, mirroring the method-body arm.
                    ClassMember::StaticBlock(b) => {
                        for s in &b.body {
                            count_uses_stmt(s, name, count);
                        }
                    }
                }
            }
        }
        // Count uses inside an arrow-value's body, same conservative
        // posture as the function arm (over-counting under a shadowing
        // param only prevents an inline, never produces a wrong one).
        Expression::ArrowFunctionExpression(ae) => match &ae.body {
            ArrowBody::Block(b) => {
                for s in &b.body {
                    count_uses_stmt(s, name, count);
                }
            }
            ArrowBody::Expression(e) => count_uses_expr(e, name, count),
        },
        // Count uses of `name` inside each `${…}` insert. Quasis are leaf
        // strings — only the insert expressions can reference `name`.
        Expression::TemplateLiteral(t) => {
            for e in &t.expressions {
                count_uses_expr(e, name, count);
            }
        }
        // Count uses of `name` in the tag callee and each `${…}` insert.
        Expression::TaggedTemplateExpression(t) => {
            count_uses_expr(&t.tag, name, count);
            for e in &t.quasi.expressions {
                count_uses_expr(e, name, count);
            }
        }
        // Count uses of `name` inside the spread argument (`...name`).
        Expression::SpreadElement(s) => count_uses_expr(&s.argument, name, count),
        Expression::YieldExpression(y) => { if let Some(a) = &y.argument { count_uses_expr(a, name, count); } }
        Expression::AwaitExpression(a) => count_uses_expr(&a.argument, name, count),
        Expression::ImportExpression(e) => count_uses_expr(&e.source, name, count),
    }
}

fn count_uses_member(
    object: &Expression,
    property: &Expression,
    computed: bool,
    name: &str,
    count: &mut usize,
) {
    count_uses_expr(object, name, count);
    // A non-computed `.name` is a property name, not a binding use.
    if computed {
        count_uses_expr(property, name, count);
    }
}

// ---- propagation (replace every use of the name with the literal) --------

/// Replace EVERY use of `cand.name` in the program with a clone of the
/// constant's literal value. Returns whether any replacement was made.
/// The walk does not short-circuit — it rewrites all use sites.
fn propagate_all(program: &mut Program, cand: &ConstCandidate) -> bool {
    let mut changed = false;
    for item in &mut program.body {
        changed |= match item {
            ProgramItem::Declaration(d) => propagate_in_decl(d, cand),
            ProgramItem::Statement(s) => propagate_in_stmt(s, cand),
        };
    }
    changed
}

fn propagate_in_decl(decl: &mut Declaration, cand: &ConstCandidate) -> bool {
    let mut changed = false;
    match decl {
        Declaration::VariableDeclaration(vd) => {
            for d in &mut vd.declarations {
                if let Some(init) = &mut d.init {
                    changed |= propagate_in_expr(init, cand);
                }
            }
        }
        Declaration::FunctionDeclaration(fd) => {
            for s in &mut fd.body.body {
                changed |= propagate_in_stmt(s, cand);
            }
        }
        // Substitute the const into the same positions `count_uses_decl`
        // inspects — the `extends` operand and each method body — so the count
        // and the rewrite stay in lockstep. Mirrors the
        // `Expression::ClassExpression` arm of `propagate_in_expr`.
        // An import declaration binds names but holds no expressions to
        // count/propagate through — nothing to do.
        Declaration::ImportDeclaration(_) => {}
        Declaration::ExportNamedDeclaration(_) => {}
        Declaration::ExportDefaultDeclaration(_) => {}
        Declaration::ExportAllDeclaration(_) => {}
        Declaration::ClassDeclaration(cd) => {
            if let Some(sup) = &mut cd.super_class {
                changed |= propagate_in_expr(sup, cand);
            }
            for member in &mut cd.body {
                match member {
                    ClassMember::Method(m) => {
                        for s in &mut m.value.body.body {
                            changed |= propagate_in_stmt(s, cand);
                        }
                    }
                    // Propagate the const into a field initializer, kept in
                    // lockstep with `count_uses_decl`.
                    ClassMember::Field(f) => {
                        if let Some(v) = &mut f.value {
                            changed |= propagate_in_expr(v, cand);
                        }
                    }
                    // Propagate the const into the static-init block's statements,
                    // kept in lockstep with `count_uses` (which counts them).
                    ClassMember::StaticBlock(b) => {
                        for s in &mut b.body {
                            changed |= propagate_in_stmt(s, cand);
                        }
                    }
                }
            }
        }
    }
    changed
}

fn propagate_in_stmt(stmt: &mut Statement, cand: &ConstCandidate) -> bool {
    let mut changed = false;
    match stmt {
        Statement::Declaration(d) => changed |= propagate_in_decl(d, cand),
        Statement::Tagged(t) => match t {
            TaggedStatement::ExpressionStatement(es) => {
                changed |= propagate_in_expr(&mut es.expression, cand)
            }
            TaggedStatement::BlockStatement(b) => {
                for s in &mut b.body {
                    changed |= propagate_in_stmt(s, cand);
                }
            }
            TaggedStatement::IfStatement(is) => {
                changed |= propagate_in_expr(&mut is.test, cand);
                changed |= propagate_in_stmt(&mut is.consequent, cand);
                if let Some(alt) = &mut is.alternate {
                    changed |= propagate_in_stmt(alt, cand);
                }
            }
            TaggedStatement::WhileStatement(ws) => {
                changed |= propagate_in_expr(&mut ws.test, cand);
                changed |= propagate_in_stmt(&mut ws.body, cand);
            }
            TaggedStatement::WithStatement(ws) => {
                changed |= propagate_in_expr(&mut ws.object, cand);
                changed |= propagate_in_stmt(&mut ws.body, cand);
            }
            TaggedStatement::DoWhileStatement(ds) => {
                changed |= propagate_in_expr(&mut ds.test, cand);
                changed |= propagate_in_stmt(&mut ds.body, cand);
            }
            TaggedStatement::ForStatement(fs) => {
                if let Some(init) = &mut fs.init {
                    match init {
                        ForInit::VariableDeclaration(vd) => {
                            for d in &mut vd.declarations {
                                if let Some(i) = &mut d.init {
                                    changed |= propagate_in_expr(i, cand);
                                }
                            }
                        }
                        ForInit::Expression(e) => changed |= propagate_in_expr(e, cand),
                    }
                }
                if let Some(test) = &mut fs.test {
                    changed |= propagate_in_expr(test, cand);
                }
                if let Some(update) = &mut fs.update {
                    changed |= propagate_in_expr(update, cand);
                }
                changed |= propagate_in_stmt(&mut fs.body, cand);
            }
            TaggedStatement::ForInStatement(fs) => {
                match &mut fs.left {
                    ForInit::VariableDeclaration(vd) => {
                        for d in &mut vd.declarations {
                            if let Some(i) = &mut d.init {
                                changed |= propagate_in_expr(i, cand);
                            }
                        }
                    }
                    ForInit::Expression(e) => changed |= propagate_in_expr(e, cand),
                }
                changed |= propagate_in_expr(&mut fs.right, cand);
                changed |= propagate_in_stmt(&mut fs.body, cand);
            }
            TaggedStatement::ForOfStatement(fs) => {
                match &mut fs.left {
                    ForInit::VariableDeclaration(vd) => {
                        for d in &mut vd.declarations {
                            if let Some(i) = &mut d.init {
                                changed |= propagate_in_expr(i, cand);
                            }
                        }
                    }
                    ForInit::Expression(e) => changed |= propagate_in_expr(e, cand),
                }
                changed |= propagate_in_expr(&mut fs.right, cand);
                changed |= propagate_in_stmt(&mut fs.body, cand);
            }
            TaggedStatement::ReturnStatement(rs) => {
                if let Some(a) = &mut rs.argument {
                    changed |= propagate_in_expr(a, cand);
                }
            }
            TaggedStatement::ThrowStatement(ts) => {
                changed |= propagate_in_expr(&mut ts.argument, cand)
            }
            TaggedStatement::LabeledStatement(ls) => {
                changed |= propagate_in_stmt(&mut ls.body, cand)
            }
            TaggedStatement::SwitchStatement(ss) => {
                changed |= propagate_in_expr(&mut ss.discriminant, cand);
                for c in &mut ss.cases {
                    if let Some(test) = &mut c.test {
                        changed |= propagate_in_expr(test, cand);
                    }
                    for s in &mut c.consequent {
                        changed |= propagate_in_stmt(s, cand);
                    }
                }
            }
            TaggedStatement::TryStatement(ts) => {
                // Propagate into the three blocks. Sound because a candidate
                // shadowed by a catch `param` of the same name is excluded
                // upstream by the decl-count guard, so any `cand.name` use
                // reached here truly resolves to the candidate.
                for s in &mut ts.block.body {
                    changed |= propagate_in_stmt(s, cand);
                }
                if let Some(h) = &mut ts.handler {
                    for s in &mut h.body.body {
                        changed |= propagate_in_stmt(s, cand);
                    }
                }
                if let Some(f) = &mut ts.finalizer {
                    for s in &mut f.body {
                        changed |= propagate_in_stmt(s, cand);
                    }
                }
            }
            TaggedStatement::BreakStatement(_)
            | TaggedStatement::ContinueStatement(_)
            | TaggedStatement::EmptyStatement(_)
            | TaggedStatement::DebuggerStatement(_) => {}
        },
    }
    changed
}

fn propagate_in_expr(expr: &mut Expression, cand: &ConstCandidate) -> bool {
    // A bare identifier in use position that matches the constant's name
    // becomes the literal. This is the only place a replacement happens;
    // every other arm just recurses into use-position sub-expressions.
    if let Expression::Identifier(id) = expr {
        if id.name == cand.name {
            *expr = cand.value.clone();
            return true;
        }
        return false;
    }

    let mut changed = false;
    match expr {
        // Identifier handled above; literals have no sub-expressions.
        Expression::Identifier(_)
        | Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::BigIntLiteral(_)
        | Expression::RegExpLiteral(_)
        // `this` binds no variable name — nothing to count or propagate into.
        | Expression::ThisExpression(_)
        | Expression::Super(_)
        | Expression::NewTarget(_)
        | Expression::ImportMeta(_)
        | Expression::UndefinedLiteral(_) => {}
        Expression::BinaryExpression(be) => {
            changed |= propagate_in_expr(&mut be.left, cand);
            changed |= propagate_in_expr(&mut be.right, cand);
        }
        Expression::LogicalExpression(le) => {
            changed |= propagate_in_expr(&mut le.left, cand);
            changed |= propagate_in_expr(&mut le.right, cand);
        }
        Expression::UnaryExpression(ue) => changed |= propagate_in_expr(&mut ue.argument, cand),
        Expression::UpdateExpression(ue) => changed |= propagate_in_expr(&mut ue.argument, cand),
        Expression::AssignmentExpression(ae) => {
            // The target identifier is a write — never replaced (a
            // `const` can't be assigned, and you can't assign to a
            // literal). Only the member-object side and the RHS recurse.
            if let AssignmentTarget::MemberExpression(m) = &mut ae.left {
                changed |= propagate_in_member(m, cand);
            }
            changed |= propagate_in_expr(&mut ae.right, cand);
        }
        Expression::ConditionalExpression(ce) => {
            changed |= propagate_in_expr(&mut ce.test, cand);
            changed |= propagate_in_expr(&mut ce.consequent, cand);
            changed |= propagate_in_expr(&mut ce.alternate, cand);
        }
        Expression::CallExpression(ce) => {
            changed |= propagate_in_expr(&mut ce.callee, cand);
            for a in &mut ce.arguments {
                changed |= propagate_in_expr(a, cand);
            }
        }
        Expression::NewExpression(ne) => {
            changed |= propagate_in_expr(&mut ne.callee, cand);
            for a in &mut ne.arguments {
                changed |= propagate_in_expr(a, cand);
            }
        }
        Expression::SequenceExpression(se) => {
            for e in &mut se.expressions {
                changed |= propagate_in_expr(e, cand);
            }
        }
        Expression::MemberExpression(m) => changed |= propagate_in_member(m, cand),
        // `a?.b` / `a?.[k]` — propagate into the object and (computed only)
        // property exactly as `propagate_in_member` does for a plain member.
        Expression::OptionalMemberExpression(m) => {
            changed |= propagate_in_expr(&mut m.object, cand);
            // Only a computed property `o?.[expr]` is a use position; a
            // non-computed `?.name` is a property name.
            if m.computed {
                changed |= propagate_in_expr(&mut m.property, cand);
            }
        }
        // `a?.()` — propagate into callee and each argument, as for a call.
        Expression::OptionalCallExpression(ce) => {
            changed |= propagate_in_expr(&mut ce.callee, cand);
            for a in &mut ce.arguments {
                changed |= propagate_in_expr(a, cand);
            }
        }
        // A chain expression transparently wraps its optional-chain spine —
        // descend into the inner expression.
        Expression::ChainExpression(c) => changed |= propagate_in_expr(&mut c.expression, cand),
        Expression::ArrayExpression(ae) => {
            for el in ae.elements.iter_mut().flatten() {
                changed |= propagate_in_expr(el, cand);
            }
        }
        Expression::ObjectExpression(oe) => {
            for member in &mut oe.properties {
                match member {
                    ObjectMember::Property(prop) => {
                        // A computed key `[expr]` is a use position; a plain
                        // identifier / string / number key is a property name.
                        if prop.computed {
                            if let PropertyKey::Expression(e) = &mut prop.key {
                                changed |= propagate_in_expr(e, cand);
                            }
                        }
                        changed |= propagate_in_expr(&mut prop.value, cand);
                    }
                    // Object spread `...expr` — recurse into the spread
                    // argument the same way the property arm recurses into
                    // prop.value.
                    ObjectMember::Spread(s) => {
                        changed |= propagate_in_expr(&mut s.argument, cand);
                    }
                }
            }
        }
        // Propagate the candidate into a function *value*'s body,
        // mirroring the `FunctionDeclaration` arm in `propagate_in_decl`.
        // Kept consistent with `count_uses_expr` above so the use count
        // and the substitution walk cover the same positions.
        Expression::FunctionExpression(fe) => {
            for s in &mut fe.body.body {
                changed |= propagate_in_stmt(s, cand);
            }
        }
        // Propagate the candidate into a class expression, mirroring the
        // `FunctionExpression` arm and kept consistent with `count_uses_expr`
        // so the count and the substitution walk cover the same positions:
        // the `extends` operand is an ordinary use position, and each
        // method's function *value* body is walked like a function body. The
        // method KEY is a property name, never a substitutable use.
        Expression::ClassExpression(ce) => {
            if let Some(sup) = &mut ce.super_class {
                changed |= propagate_in_expr(sup, cand);
            }
            for member in &mut ce.body {
                match member {
                    ClassMember::Method(m) => {
                        for s in &mut m.value.body.body {
                            changed |= propagate_in_stmt(s, cand);
                        }
                    }
                    // Propagate the const into a field initializer.
                    ClassMember::Field(f) => {
                        if let Some(v) = &mut f.value {
                            changed |= propagate_in_expr(v, cand);
                        }
                    }
                    // Propagate the const into the static-init block's statements,
                    // kept in lockstep with `count_uses` (which counts them).
                    ClassMember::StaticBlock(b) => {
                        for s in &mut b.body {
                            changed |= propagate_in_stmt(s, cand);
                        }
                    }
                }
            }
        }
        // Propagate into an arrow-value's body, kept consistent with
        // `count_uses_expr` so the count and the substitution cover the
        // same positions.
        Expression::ArrowFunctionExpression(ae) => match &mut ae.body {
            ArrowBody::Block(b) => {
                for s in &mut b.body {
                    changed |= propagate_in_stmt(s, cand);
                }
            }
            ArrowBody::Expression(e) => changed |= propagate_in_expr(e, cand),
        },
        // Propagate into each `${…}` insert. Quasis are leaf strings and
        // hold no substitutable reference.
        Expression::TemplateLiteral(t) => {
            for e in &mut t.expressions {
                changed |= propagate_in_expr(e, cand);
            }
        }
        // Propagate into the tag callee and each `${…}` insert.
        Expression::TaggedTemplateExpression(t) => {
            changed |= propagate_in_expr(&mut t.tag, cand);
            for e in &mut t.quasi.expressions {
                changed |= propagate_in_expr(e, cand);
            }
        }
        // Propagate into the spread argument (`...name`).
        Expression::SpreadElement(s) => changed |= propagate_in_expr(&mut s.argument, cand),
        Expression::YieldExpression(y) => { if let Some(a) = &mut y.argument { changed |= propagate_in_expr(a, cand); } }
        Expression::AwaitExpression(a) => changed |= propagate_in_expr(&mut a.argument, cand),
        Expression::ImportExpression(e) => changed |= propagate_in_expr(&mut e.source, cand),
    }
    changed
}

fn propagate_in_member(
    m: &mut coding_adventures_javascript_ast::MemberExpression,
    cand: &ConstCandidate,
) -> bool {
    let mut changed = propagate_in_expr(&mut m.object, cand);
    // Only a computed property `o[expr]` is a use position; a
    // non-computed `.name` is a property name.
    if m.computed {
        changed |= propagate_in_expr(&mut m.property, cand);
    }
    changed
}

#[cfg(test)]
mod tests {
    //! Tests pin the public contract (name, policy, cost, deps), the
    //! `PassPipeline` integration, and the propagation behaviour itself
    //! — driven end-to-end through the real source → bridge →
    //! inline-variables → emit roundtrip so they exercise the exact AST
    //! shape the parser produces.
    use super::*;
    use coding_adventures_closure_emitter::{emit, EmitOptions};
    use coding_adventures_closure_pass_constant_fold::ConstantFoldPass;
    use coding_adventures_closure_pass_pipeline::{PassContext, PassPipeline, PipelineOutput};
    use coding_adventures_correlation_vector::{CVLog, Contribution};
    use coding_adventures_javascript_ast::{Program, SourceType};
    use coding_adventures_javascript_parser::{bridge, parse_javascript_typed};
    use coding_adventures_javascript_tokens::EsVersion;
    use coding_adventures_type_sidecar::Sidecar;

    fn program() -> Program {
        Program::new("prog.1".to_string(), EsVersion::Es2025, SourceType::Module)
    }

    /// Parse `src`, bridge to a typed `Program`, run the pass, emit.
    fn propagate_source(src: &str) -> String {
        let es = EsVersion::Es2025;
        let node = parse_javascript_typed(src, es).expect("parse");
        let prog = bridge::grammar_to_program(&node, es).expect("bridge");

        let pass = InlineVariablesPass::new();
        let sidecar = Sidecar::new();
        let mut cv = CVLog::new(false);
        let out = pass
            .run(PassContext {
                program: &prog,
                sidecar: &sidecar,
                cv: &mut cv,
            })
            .expect("inline-variables");

        let mut cv2 = CVLog::new(false);
        let opts = EmitOptions {
            source_map: false,
            ..Default::default()
        };
        emit(&out.program, &sidecar, &mut cv2, &opts)
            .expect("emit")
            .code
    }

    /// Parse `src`, bridge, run the pass, and return its CV contributions —
    /// the propagation table (#89 provenance).
    fn propagate_contributions(src: &str) -> Vec<Contribution> {
        let es = EsVersion::Es2025;
        let node = parse_javascript_typed(src, es).expect("parse");
        let prog = bridge::grammar_to_program(&node, es).expect("bridge");
        let pass = InlineVariablesPass::new();
        let sidecar = Sidecar::new();
        let mut cv = CVLog::new(true);
        pass.run(PassContext {
            program: &prog,
            sidecar: &sidecar,
            cv: &mut cv,
        })
        .expect("inline-variables")
        .contributions
    }

    // ----- CV provenance (#89): `propagated` contributions -----

    #[test]
    fn single_use_propagation_records_value_and_one_site() {
        // `const N = 42; use(N);` → the literal is propagated to its lone use.
        let contribs = propagate_contributions("const N = 42; use(N);");
        assert_eq!(contribs.len(), 1, "one propagated const; got {contribs:?}");
        let c = &contribs[0];
        assert_eq!(c.source, "inline-variables");
        assert_eq!(c.tag, "propagated");
        assert_eq!(c.meta.get("name").and_then(|v| v.as_str()), Some("N"));
        assert_eq!(c.meta.get("value").and_then(|v| v.as_str()), Some("42"));
        assert_eq!(c.meta.get("sites").and_then(|v| v.as_u64()), Some(1));
    }

    #[test]
    fn multi_use_propagation_records_site_count() {
        // A short literal used twice is propagated to BOTH sites.
        let contribs = propagate_contributions("const K = 1; a(K); b(K);");
        assert_eq!(contribs.len(), 1, "one propagated const; got {contribs:?}");
        assert_eq!(
            contribs[0].meta.get("name").and_then(|v| v.as_str()),
            Some("K")
        );
        assert_eq!(
            contribs[0].meta.get("sites").and_then(|v| v.as_u64()),
            Some(2)
        );
    }

    #[test]
    fn no_propagation_emits_no_contributions() {
        // `let` is reassignable — never propagated, so the table is empty.
        let contribs = propagate_contributions("let x = 1; use(x);");
        assert!(
            contribs.is_empty(),
            "expected no contributions; got {contribs:?}"
        );
    }

    // ----- metadata contract -----

    #[test]
    fn name_is_inline_variables() {
        assert_eq!(InlineVariablesPass::new().name(), "inline-variables");
    }

    #[test]
    fn iteration_policy_is_fixed_point() {
        assert_eq!(
            InlineVariablesPass::new().iteration_policy(),
            IterationPolicy::FixedPoint
        );
    }

    #[test]
    fn cost_is_three_pass_units() {
        assert_eq!(InlineVariablesPass::new().cost(), 3);
    }

    #[test]
    fn depends_on_constant_fold() {
        assert_eq!(InlineVariablesPass::new().depends_on(), &["constant-fold"]);
    }

    #[test]
    fn invalidates_empty_in_v1() {
        assert!(InlineVariablesPass::new().invalidates().is_empty());
    }

    #[test]
    fn run_on_empty_program_returns_unchanged_identity() {
        let pass = InlineVariablesPass::new();
        let prog = program();
        let sidecar = Sidecar::new();
        let mut cv = CVLog::new(true);
        let out = pass
            .run(PassContext {
                program: &prog,
                sidecar: &sidecar,
                cv: &mut cv,
            })
            .expect("pass should succeed");
        assert_eq!(out.program.cv, prog.cv);
        assert!(!out.changed);
        assert!(out.contributions.is_empty());
        assert_eq!(out.stats.nodes_touched, 1);
    }

    #[test]
    fn pipeline_orders_constant_fold_before_inline_variables() {
        let mut pipeline = PassPipeline::new();
        pipeline.add(Box::new(InlineVariablesPass::new()));
        pipeline.add(Box::new(ConstantFoldPass::new()));
        let mut cv = CVLog::new(true);
        let out: PipelineOutput = pipeline
            .run(program(), &Sidecar::new(), &mut cv)
            .expect("pipeline should run cleanly");
        assert_eq!(
            out.execution_order,
            vec!["constant-fold".to_string(), "inline-variables".to_string()],
            "inline-variables must run after constant-fold"
        );
    }

    #[test]
    fn pass_is_default_and_clone() {
        let _a: InlineVariablesPass = Default::default();
        let _b: InlineVariablesPass = InlineVariablesPass::new();
        let _c = _b;
        let _d = _c;
    }

    // =====================================================================
    // Propagation behaviour (source → bridge → inline-variables → emit)
    // =====================================================================
    //
    // NOTE: the pass only PROPAGATES — it leaves the now-unreferenced
    // `const` declaration in place (remove-unused-vars removes it
    // downstream). These tests assert the value reached the use sites;
    // the husk `const X=…;` is expected to remain at the pass level.

    #[test]
    fn propagates_single_use_const() {
        assert_eq!(
            propagate_source("const RATE = 2; use(RATE);"),
            "const RATE=2;use(2);"
        );
    }

    #[test]
    fn propagates_const_into_expression() {
        assert_eq!(
            propagate_source("const RATE = 2; total(base * RATE);"),
            "const RATE=2;total(base*2);"
        );
    }

    #[test]
    fn propagates_short_literal_at_multiple_sites() {
        assert_eq!(
            propagate_source("const N = 3; a(N); b(N); c(N);"),
            "const N=3;a(3);b(3);c(3);"
        );
    }

    #[test]
    fn propagates_boolean_and_null_literals() {
        // Booleans render via closure-emitter's Closure-style shorthand
        // (`true` → `!0`); `null` is unchanged. The propagation itself — both
        // the declaration and the propagated use carrying the literal — is
        // what this test exercises.
        assert_eq!(
            propagate_source("const ON = true; const NONE = null; f(ON, NONE);"),
            "const ON=!0;const NONE=null;f(!0,null);"
        );
    }

    #[test]
    fn does_not_propagate_long_literal_at_multiple_sites() {
        // The string is longer than the multi-use budget, so duplicating
        // it across two sites is not worth deleting the declaration.
        assert_eq!(
            propagate_source("const MSG = \"a long message value\"; a(MSG); b(MSG);"),
            "const MSG=\"a long message value\";a(MSG);b(MSG);"
        );
    }

    #[test]
    fn propagates_long_literal_at_a_single_site() {
        // A single use is always worth it — the whole declaration goes.
        assert_eq!(
            propagate_source("const MSG = \"a long message value\"; a(MSG);"),
            "const MSG=\"a long message value\";a(\"a long message value\");"
        );
    }

    #[test]
    fn does_not_propagate_let_or_var() {
        // `let`/`var` can be reassigned — never propagated.
        assert_eq!(propagate_source("let X = 5; use(X);"), "let X=5;use(X);");
        assert_eq!(propagate_source("var Y = 5; use(Y);"), "var Y=5;use(Y);");
    }

    #[test]
    fn does_not_propagate_non_literal_value() {
        // `const X = expr` where expr is not a literal is left alone:
        // an identifier value could be reassigned; a call/member could
        // have side effects.
        assert_eq!(
            propagate_source("const X = other; use(X);"),
            "const X=other;use(X);"
        );
        assert_eq!(
            propagate_source("const X = make(); use(X);"),
            "const X=make();use(X);"
        );
    }

    #[test]
    fn does_not_propagate_shadowed_name() {
        // `RATE` is declared twice (the const and a parameter of `f`), so
        // a use of `RATE` could resolve to either binding. Declined.
        assert_eq!(
            propagate_source("const RATE = 2; function f(RATE) { return RATE; }"),
            "const RATE=2;function f(RATE){return RATE};"
        );
    }

    #[test]
    fn does_not_replace_property_name() {
        // `obj.RATE` — `RATE` here is a property name, not a use of the
        // const, so it is not replaced and the const has zero uses.
        assert_eq!(
            propagate_source("const RATE = 2; use(obj.RATE);"),
            "const RATE=2;use(obj.RATE);"
        );
    }

    #[test]
    fn replaces_computed_member() {
        // A computed `obj[RATE]` IS a use position — replaced.
        assert_eq!(
            propagate_source("const RATE = 2; use(obj[RATE]);"),
            "const RATE=2;use(obj[2]);"
        );
    }

    // ----- temporal dead zone (TDZ) guard -----

    #[test]
    fn propagates_through_an_inert_const_block() {
        // Every item before `X` is inert (a literal-valued `const`), so
        // nothing runs before `X` initializes — both constants propagate.
        assert_eq!(
            propagate_source("const A = 1; const X = 2; f(A, X);"),
            "const A=1;const X=2;f(1,2);"
        );
    }

    #[test]
    fn propagates_into_a_function_declared_before_the_const() {
        // `helper` is declared before `X` but a function declaration only
        // hoists — it can't run until called, and the only call
        // (`run(helper)`) is after `X` initializes. So reading `X` inside
        // `helper` is never in the TDZ, and propagation is sound.
        assert_eq!(
            propagate_source("function helper() { return X; } const X = 5; run(helper);"),
            "function helper(){return 5};const X=5;run(helper);"
        );
    }

    #[test]
    fn does_not_propagate_when_code_runs_before_the_declaration() {
        // SOUNDNESS (TDZ): a top-level call `g()` runs before `const X`
        // initializes. `g` (or anything it reaches) could read `X` in its
        // temporal dead zone and throw `ReferenceError`; propagating the
        // literal would erase that throw. The prefix is not inert, so `X`
        // is declined and left untouched.
        assert_eq!(
            propagate_source("g(); const X = 5; use(X);"),
            "g();const X=5;use(X);"
        );
    }

    #[test]
    fn does_not_propagate_a_const_after_a_non_literal_initializer() {
        // `const SETUP = init();` runs `init()` before `const X` — not an
        // inert prefix, so `X` is declined (TDZ guard). `SETUP` itself is
        // not a candidate (its initializer is a call, not a literal).
        assert_eq!(
            propagate_source("const SETUP = init(); const X = 5; use(X);"),
            "const SETUP=init();const X=5;use(X);"
        );
    }

    #[test]
    fn does_not_propagate_multi_declarator_const() {
        // A multi-declarator `const A = 1, X = 2` is conservatively
        // skipped (an earlier sibling could be a call the TDZ scan, which
        // works at item granularity, would not see).
        assert_eq!(
            propagate_source("const A = 1, X = 2; f(A, X);"),
            "const A=1,X=2;f(A,X);"
        );
    }
}
