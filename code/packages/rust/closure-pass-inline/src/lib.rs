//! Function-inlining pass for the Closure Compiler clone.
//!
//! Per [CLOC06](../../../specs/CLOC06-pass-interface-contract.md)'s
//! canonical pass set. Substitutes a callee's body at the call
//! site when doing so is cheaper than the call:
//!
//! ```js
//! // before
//! function double(x) { return x * 2; }
//! log(double(7));
//!
//! // after  (the call is replaced by the substituted body)
//! function double(x) { return x * 2; }   // now unreferenced …
//! log(7 * 2);                            // … and removed by the
//!                                        //     later remove-unused-vars
//!                                        //     / treeshake passes
//! ```
//!
//! # The two questions every inliner answers
//!
//! 1. **Is it safe?** A call can only be inlined if substituting
//!    the body doesn't change semantics. The hard cases:
//!    - `this` and `arguments` bindings: the inlined body sees
//!      different ones than it did when defined.
//!    - Captured variables from a different closure scope.
//!    - Recursive calls (inline once, sure — but where do you
//!      stop?).
//!    - Side-effecting argument expressions vs. parameters used
//!      multiple times in the body (you'd evaluate the arg twice).
//! 2. **Is it worth it?** Inlining a 1000-line function at 50
//!    call sites bloats output. Inlining a 3-line single-use
//!    helper shrinks it.
//!
//! # The provably-safe slice this pass implements
//!
//! Rather than answer the hard cases above with heuristics, the
//! current slice inlines only the subset where every one of them is
//! *structurally impossible*. A call `f(a₁, …, aₙ)` is inlined when
//! ALL of the following hold:
//!
//!   1. **`f` is a top-level `function` declaration.** Top-level so
//!      there is no enclosing scope whose variables the body could
//!      capture; a plain `function` (not generator / not `async`)
//!      so there is no `yield`/`await` state to preserve.
//!   2. **`f`'s body is exactly `{ return EXPR; }`.** One statement,
//!      a `return` with an argument. No locals, no control flow, no
//!      statements to splice — substitution is a pure
//!      expression-for-expression swap.
//!   3. **Every identifier in `EXPR` is one of `f`'s parameters.**
//!      This is the capture guard: with no *free* identifiers, the
//!      substituted expression can neither read a global that might
//!      be shadowed at the call site nor reference `f` itself
//!      (so recursion is excluded for free). `this` / `arguments`
//!      are identifiers too, so a body using them is rejected here.
//!   4. **`f`'s name is declared exactly once in the whole program.**
//!      No other binding (a `var f`, a parameter `f`, a second
//!      `function f`) anywhere shadows the name, so *every* use of
//!      the identifier `f` in the program resolves to this function
//!      — we can count and locate its call site by name without a
//!      full scope resolver. (Same self-contained philosophy as the
//!      `rename` pass.)
//!   5. **`f` is used exactly once, and that use is the call we are
//!      inlining**, with `arguments.len() == params.len()`.
//!      Single-use is the unambiguous size win (clone the body once,
//!      not N times) and sidesteps the multi-site budget decision.
//!   6. **Every argument is side-effect-free** — a literal or a bare
//!      identifier. Then substituting an argument for a parameter
//!      that the body uses zero, one, or many times can neither drop
//!      nor duplicate a side effect, so the argument-evaluation
//!      hazard vanishes.
//!
//! Everything outside this subset is left untouched (`changed`
//! stays `false`). Broader inlining — multi-use callees under a size
//! budget, function *expressions*, bodies with locals/branches —
//! is future work on the same walker.
//!
//! # Why this enables downstream folding
//!
//! Once the body is substituted at the call site, a later
//! `constant-fold` iteration sees concrete arguments instead of
//! parameter references. `double(7)` → `7 * 2` → `14`. The canonical
//! order runs fold *before* inline (so the inliner sees folded
//! arguments) and the size win is realised once fold runs again
//! under `IterationPolicy::FixedPoint`.
//!
//! # Where this pass sits in the canonical order
//!
//! CLOC06 §"Canonical pass set" pins:
//!
//! ```text
//! constant-fold → fold-control-flow → dce → inline → rename → ...
//! ```
//!
//! Inline runs **after DCE** so it doesn't bother inlining callees
//! that are about to be deleted, **before rename** so the heuristics
//! see meaningful names, and crucially **before remove-unused-vars /
//! treeshake**: once a single-use callee's only call is inlined, the
//! function declaration is unreferenced and those later passes
//! delete it. This pass deliberately leaves the now-dead declaration
//! in place rather than removing it itself — deletion is their job.

use std::collections::{HashMap, HashSet};

use coding_adventures_closure_pass_pipeline::{
    IterationPolicy, Pass, PassContext, PassError, PassOutput, PassStats,
};
use coding_adventures_javascript_ast::statement::TaggedStatement;
use coding_adventures_javascript_ast::{
    AssignmentTarget, BindingTarget, Declaration, Expression, ForInit, FunctionDeclaration,
    FunctionParam, Program, ProgramItem, PropertyKey, Statement, VariableDeclaration,
};

/// `Pass::depends_on` value. Kept as a `const` so future tests and
/// dependent crates can refer to it without retyping the pass name.
const DEPS: &[&str] = &["constant-fold"];

/// Function-inlining pass. See crate-level docs for the exact
/// (provably-safe) slice it implements.
///
/// Zero-sized type: no per-instance state. Pass-internal state
/// (the candidate map, the per-call substitution map) lives in
/// pass-local maps constructed inside [`Pass::run`] per CLOC06
/// §"Pass-internal state."
#[derive(Debug, Default, Clone, Copy)]
pub struct InlinePass;

impl InlinePass {
    /// Zero-arg constructor for ergonomic
    /// `PassPipeline::add(Box::new(InlinePass::new()))` registration.
    pub fn new() -> Self {
        Self
    }
}

impl Pass for InlinePass {
    fn name(&self) -> &'static str {
        "inline"
    }

    fn depends_on(&self) -> &[&'static str] {
        // CLOC06 canonical order: constant-fold first so the
        // inliner sees folded arguments at call sites. Folded
        // literals plug into parameters cleanly; unfolded
        // expressions would require carrying around argument
        // expression trees.
        DEPS
    }

    fn iteration_policy(&self) -> IterationPolicy {
        // Per CLOC06: inlining is canonically fixed-point.
        // Inlining `f(g(h(7)))` first inlines `f`, exposing the
        // call to `g` in the now-substituted body; the next
        // iteration can inline `g`, and so on. Each round strictly
        // removes a single-use callee's only reference, so the
        // candidate set shrinks monotonically and the fixed point
        // is reached in finitely many steps.
        IterationPolicy::FixedPoint
    }

    fn cost(&self) -> u32 {
        // Heavier than the folds and DCE:
        //   - Count every binding-name declaration once (shadow
        //     detection) and every use of each candidate name.
        //   - Clone-and-substitute the callee body at the call
        //     site. The clone-and-rewrite is the expensive step.
        4
    }

    fn run(&self, ctx: PassContext<'_>) -> Result<PassOutput, PassError> {
        // Real transform: inline single-use top-level leaf functions
        // whose body is `return EXPR` with no free identifiers. See
        // [`inline_program`] and the crate-level docs for the full
        // safety argument. An empty / construct-free program is left
        // untouched (`changed = false`, `nodes_touched = 1`).
        let mut program = ctx.program.clone();
        let mut nodes_touched: u32 = 1; // the program root
        let changed = inline_program(&mut program, &mut nodes_touched);

        Ok(PassOutput {
            program,
            contributions: Vec::new(),
            changed,
            diagnostics: Vec::new(),
            stats: PassStats { nodes_touched },
        })
    }
}

// =========================================================================
// Inlining implementation (single-use top-level leaf functions)
// =========================================================================

/// One inlinable function: its name, parameter names in order, and a
/// clone of the single `return` expression to substitute at the call
/// site.
struct InlineCandidate {
    name: String,
    params: Vec<String>,
    return_expr: Expression,
}

/// Walk the whole program and inline every qualifying single-use
/// top-level function. Returns whether anything changed.
fn inline_program(program: &mut Program, nodes_touched: &mut u32) -> bool {
    // Phase 1 — count how many times each *name* is declared as a
    // binding anywhere in the program (function names, parameters,
    // and `var`/`let`/`const` targets). A candidate's name must be
    // declared exactly once, which guarantees no other scope shadows
    // it and lets us resolve its uses by name alone.
    let mut decl_counts: HashMap<String, usize> = HashMap::new();
    count_decl_names_program(program, &mut decl_counts, nodes_touched);

    // Phase 2 — collect candidates from the top-level function
    // declarations. (Only top-level: a nested function could capture
    // its enclosing scope, which the free-identifier guard already
    // rejects, but restricting to top level keeps the first slice's
    // reasoning airtight.)
    let mut candidates: Vec<InlineCandidate> = Vec::new();
    for item in &program.body {
        if let ProgramItem::Declaration(Declaration::FunctionDeclaration(fd)) = item {
            if let Some(c) = candidate_from_function(fd, &decl_counts) {
                candidates.push(c);
            }
        }
    }
    if candidates.is_empty() {
        return false;
    }

    // Phase 3 — for each candidate, require exactly one use in the
    // program; then find that single call and substitute. Counting
    // and substituting on the (progressively mutated) program is
    // sound: an inlined body contains only the call's own simple
    // arguments, so it neither adds nor removes uses of any *other*
    // candidate's name.
    let mut changed = false;
    for cand in &candidates {
        if count_name_uses_program(program, &cand.name) != 1 {
            continue; // multi-use (budget decision) or a non-call value use
        }
        if inline_single_call(program, cand) {
            changed = true;
        }
    }
    changed
}

/// Decide whether a top-level function declaration is an inline
/// candidate. Returns its (name, params, return-expression) when all
/// the structural safety conditions in the crate docs hold.
fn candidate_from_function(
    fd: &FunctionDeclaration,
    decl_counts: &HashMap<String, usize>,
) -> Option<InlineCandidate> {
    // (1) Plain function only — a generator / async function carries
    // resumable state that a straight expression swap would lose.
    if fd.generator || fd.is_async {
        return None;
    }

    // (4) The name must be declared exactly once in the whole program
    // (no shadowing), so every use of the identifier resolves here.
    if decl_counts.get(&fd.id.name).copied().unwrap_or(0) != 1 {
        return None;
    }

    // Parameter names must be distinct, or the substitution map would
    // be ambiguous. (`function f(a, a)` is a syntax error in strict
    // mode anyway, but we never assume the parser rejected it.)
    let mut params: Vec<String> = Vec::with_capacity(fd.params.len());
    let mut seen = HashSet::new();
    for p in &fd.params {
        let FunctionParam::Identifier(id) = p;
        if !seen.insert(id.name.clone()) {
            return None;
        }
        params.push(id.name.clone());
    }

    // (2) Body must be exactly `{ return EXPR; }`.
    if fd.body.body.len() != 1 {
        return None;
    }
    let return_expr = match &fd.body.body[0] {
        Statement::Tagged(TaggedStatement::ReturnStatement(rs)) => rs.argument.as_ref()?,
        _ => return None,
    };

    // (3) Capture guard: every identifier in EXPR must be a parameter.
    // No free identifiers ⇒ no global capture, no `this`/`arguments`,
    // and no self-reference (recursion excluded for free).
    let mut free = HashSet::new();
    collect_binding_idents_expr(return_expr, &mut free);
    let param_set: HashSet<&str> = params.iter().map(|s| s.as_str()).collect();
    if !free.iter().all(|n| param_set.contains(n.as_str())) {
        return None;
    }

    Some(InlineCandidate {
        name: fd.id.name.clone(),
        params,
        return_expr: return_expr.clone(),
    })
}

/// True for an argument expression that is safe to substitute for a
/// parameter no matter how many times (including zero) the parameter
/// is used in the body: a literal or a bare identifier — neither has
/// a side effect, so it can be dropped or duplicated freely.
fn is_simple_arg(expr: &Expression) -> bool {
    matches!(
        expr,
        Expression::Identifier(_)
            | Expression::NumericLiteral(_)
            | Expression::StringLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_)
            | Expression::BigIntLiteral(_)
            | Expression::UndefinedLiteral(_)
    )
}

// ---- name-declaration counting (shadow detection) ------------------------

/// Count every binding-name *declaration* in the whole program —
/// function names, parameters, and `var`/`let`/`const` targets,
/// recursing into nested function bodies — accumulating occurrence
/// counts into `out`. `nodes_touched` is bumped per statement for the
/// scheduler's cost accounting.
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
                let FunctionParam::Identifier(id) = p;
                *out.entry(id.name.clone()).or_insert(0) += 1;
            }
            for s in &fd.body.body {
                count_decl_names_stmt(s, out, nodes_touched);
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
            TaggedStatement::ForStatement(fs) => {
                if let Some(ForInit::VariableDeclaration(vd)) = &fs.init {
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
            // Statements that introduce no binding in the Phase-1 AST.
            // Exhaustive on purpose: a future binding-introducing
            // statement (a `try`/`catch`, a `class` declaration) must be
            // handled here so a shadowing name can't slip past the
            // shadow guard and make an unsound inline. The compiler
            // flags the omission.
            TaggedStatement::ExpressionStatement(_)
            | TaggedStatement::ReturnStatement(_)
            | TaggedStatement::ThrowStatement(_)
            | TaggedStatement::BreakStatement(_)
            | TaggedStatement::ContinueStatement(_)
            | TaggedStatement::EmptyStatement(_) => {}
        },
    }
}

// ---- binding-identifier collection (capture guard) -----------------------

/// Collect the names of every identifier appearing in *binding-use*
/// position inside `expr` — i.e. the names that actually reference a
/// variable. Property names (the `.x` of a non-computed member, a
/// non-computed object-literal key) are NOT bindings and are skipped,
/// matching the rewrite rules in [`substitute`].
fn collect_binding_idents_expr(expr: &Expression, out: &mut HashSet<String>) {
    match expr {
        Expression::Identifier(id) => {
            out.insert(id.name.clone());
        }
        Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::BigIntLiteral(_)
        | Expression::UndefinedLiteral(_) => {}
        Expression::BinaryExpression(be) => {
            collect_binding_idents_expr(&be.left, out);
            collect_binding_idents_expr(&be.right, out);
        }
        Expression::LogicalExpression(le) => {
            collect_binding_idents_expr(&le.left, out);
            collect_binding_idents_expr(&le.right, out);
        }
        Expression::UnaryExpression(ue) => collect_binding_idents_expr(&ue.argument, out),
        Expression::AssignmentExpression(ae) => {
            match &ae.left {
                AssignmentTarget::Identifier(id) => {
                    out.insert(id.name.clone());
                }
                AssignmentTarget::MemberExpression(m) => {
                    collect_binding_idents_member(&m.object, &m.property, m.computed, out)
                }
            }
            collect_binding_idents_expr(&ae.right, out);
        }
        Expression::ConditionalExpression(ce) => {
            collect_binding_idents_expr(&ce.test, out);
            collect_binding_idents_expr(&ce.consequent, out);
            collect_binding_idents_expr(&ce.alternate, out);
        }
        Expression::CallExpression(ce) => {
            collect_binding_idents_expr(&ce.callee, out);
            for a in &ce.arguments {
                collect_binding_idents_expr(a, out);
            }
        }
        Expression::MemberExpression(m) => {
            collect_binding_idents_member(&m.object, &m.property, m.computed, out)
        }
        Expression::ArrayExpression(ae) => {
            for el in ae.elements.iter().flatten() {
                collect_binding_idents_expr(el, out);
            }
        }
        Expression::ObjectExpression(oe) => {
            for prop in &oe.properties {
                // Only a *computed* key `[expr]` is a binding use; a
                // plain identifier / string / number key is a property
                // name.
                if prop.computed {
                    if let PropertyKey::Expression(e) = &prop.key {
                        collect_binding_idents_expr(e, out);
                    }
                }
                collect_binding_idents_expr(&prop.value, out);
            }
        }
    }
}

fn collect_binding_idents_member(
    object: &Expression,
    property: &Expression,
    computed: bool,
    out: &mut HashSet<String>,
) {
    collect_binding_idents_expr(object, out);
    // The `.name` of a non-computed member access is a property name,
    // NOT a binding — only the object (and a computed `[key]`) count.
    if computed {
        collect_binding_idents_expr(property, out);
    }
}

// ---- use counting --------------------------------------------------------

/// Count the *uses* (binding-use-position occurrences) of `name`
/// across the whole program. Declarations, property names, and label
/// names are not uses. Recurses into nested function bodies because a
/// call site can live anywhere.
fn count_name_uses_program(program: &Program, name: &str) -> usize {
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
            // Labels live in a separate namespace; break/continue/empty
            // hold no variable uses.
            TaggedStatement::BreakStatement(_)
            | TaggedStatement::ContinueStatement(_)
            | TaggedStatement::EmptyStatement(_) => {}
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
        Expression::AssignmentExpression(ae) => {
            match &ae.left {
                AssignmentTarget::Identifier(id) => {
                    if id.name == name {
                        *count += 1;
                    }
                }
                AssignmentTarget::MemberExpression(m) => {
                    count_uses_member(&m.object, &m.property, m.computed, name, count)
                }
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
        Expression::MemberExpression(m) => {
            count_uses_member(&m.object, &m.property, m.computed, name, count)
        }
        Expression::ArrayExpression(ae) => {
            for el in ae.elements.iter().flatten() {
                count_uses_expr(el, name, count);
            }
        }
        Expression::ObjectExpression(oe) => {
            for prop in &oe.properties {
                if prop.computed {
                    if let PropertyKey::Expression(e) = &prop.key {
                        count_uses_expr(e, name, count);
                    }
                }
                count_uses_expr(&prop.value, name, count);
            }
        }
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
    if computed {
        count_uses_expr(property, name, count);
    }
}

// ---- call substitution ---------------------------------------------------

/// Find the single call `cand.name(args)` in the program and, if its
/// arity matches and all arguments are side-effect-free, replace it
/// with the substituted callee body. Returns whether a replacement
/// was made. (The caller guarantees exactly one use of the name, so
/// there is at most one such call.)
fn inline_single_call(program: &mut Program, cand: &InlineCandidate) -> bool {
    for item in &mut program.body {
        let replaced = match item {
            ProgramItem::Declaration(d) => inline_in_decl(d, cand),
            ProgramItem::Statement(s) => inline_in_stmt(s, cand),
        };
        if replaced {
            return true;
        }
    }
    false
}

fn inline_in_decl(decl: &mut Declaration, cand: &InlineCandidate) -> bool {
    match decl {
        Declaration::VariableDeclaration(vd) => {
            for d in &mut vd.declarations {
                if let Some(init) = &mut d.init {
                    if inline_in_expr(init, cand) {
                        return true;
                    }
                }
            }
            false
        }
        Declaration::FunctionDeclaration(fd) => {
            for s in &mut fd.body.body {
                if inline_in_stmt(s, cand) {
                    return true;
                }
            }
            false
        }
    }
}

fn inline_in_stmt(stmt: &mut Statement, cand: &InlineCandidate) -> bool {
    match stmt {
        Statement::Declaration(d) => inline_in_decl(d, cand),
        Statement::Tagged(t) => match t {
            TaggedStatement::ExpressionStatement(es) => inline_in_expr(&mut es.expression, cand),
            TaggedStatement::BlockStatement(b) => {
                for s in &mut b.body {
                    if inline_in_stmt(s, cand) {
                        return true;
                    }
                }
                false
            }
            TaggedStatement::IfStatement(is) => {
                inline_in_expr(&mut is.test, cand)
                    || inline_in_stmt(&mut is.consequent, cand)
                    || is
                        .alternate
                        .as_mut()
                        .is_some_and(|alt| inline_in_stmt(alt, cand))
            }
            TaggedStatement::WhileStatement(ws) => {
                inline_in_expr(&mut ws.test, cand) || inline_in_stmt(&mut ws.body, cand)
            }
            TaggedStatement::ForStatement(fs) => {
                if let Some(init) = &mut fs.init {
                    let hit = match init {
                        ForInit::VariableDeclaration(vd) => vd
                            .declarations
                            .iter_mut()
                            .any(|d| d.init.as_mut().is_some_and(|i| inline_in_expr(i, cand))),
                        ForInit::Expression(e) => inline_in_expr(e, cand),
                    };
                    if hit {
                        return true;
                    }
                }
                if let Some(test) = &mut fs.test {
                    if inline_in_expr(test, cand) {
                        return true;
                    }
                }
                if let Some(update) = &mut fs.update {
                    if inline_in_expr(update, cand) {
                        return true;
                    }
                }
                inline_in_stmt(&mut fs.body, cand)
            }
            TaggedStatement::ReturnStatement(rs) => rs
                .argument
                .as_mut()
                .is_some_and(|a| inline_in_expr(a, cand)),
            TaggedStatement::ThrowStatement(ts) => inline_in_expr(&mut ts.argument, cand),
            TaggedStatement::LabeledStatement(ls) => inline_in_stmt(&mut ls.body, cand),
            TaggedStatement::SwitchStatement(ss) => {
                if inline_in_expr(&mut ss.discriminant, cand) {
                    return true;
                }
                for c in &mut ss.cases {
                    if let Some(test) = &mut c.test {
                        if inline_in_expr(test, cand) {
                            return true;
                        }
                    }
                    for s in &mut c.consequent {
                        if inline_in_stmt(s, cand) {
                            return true;
                        }
                    }
                }
                false
            }
            TaggedStatement::BreakStatement(_)
            | TaggedStatement::ContinueStatement(_)
            | TaggedStatement::EmptyStatement(_) => false,
        },
    }
}

fn inline_in_expr(expr: &mut Expression, cand: &InlineCandidate) -> bool {
    // If THIS node is the call we are inlining, replace it in place.
    if let Expression::CallExpression(ce) = expr {
        let is_target = matches!(&*ce.callee, Expression::Identifier(id) if id.name == cand.name)
            && ce.arguments.len() == cand.params.len()
            && ce.arguments.iter().all(is_simple_arg);
        if is_target {
            // Build an OWNED name → argument map (cloning the simple
            // args) so no borrow of `ce` outlives the `*expr = …`
            // overwrite below.
            let map: HashMap<String, Expression> = cand
                .params
                .iter()
                .cloned()
                .zip(ce.arguments.iter().cloned())
                .collect();
            let mut replacement = cand.return_expr.clone();
            substitute(&mut replacement, &map);
            *expr = replacement;
            return true;
        }
        // Not our call — recurse into the callee and arguments (the
        // target call might be nested, e.g. `outer(double(7))`).
        if inline_in_expr(&mut ce.callee, cand) {
            return true;
        }
        for a in &mut ce.arguments {
            if inline_in_expr(a, cand) {
                return true;
            }
        }
        return false;
    }

    match expr {
        Expression::Identifier(_)
        | Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::BigIntLiteral(_)
        | Expression::UndefinedLiteral(_) => false,
        Expression::BinaryExpression(be) => {
            inline_in_expr(&mut be.left, cand) || inline_in_expr(&mut be.right, cand)
        }
        Expression::LogicalExpression(le) => {
            inline_in_expr(&mut le.left, cand) || inline_in_expr(&mut le.right, cand)
        }
        Expression::UnaryExpression(ue) => inline_in_expr(&mut ue.argument, cand),
        Expression::AssignmentExpression(ae) => {
            let left_hit = match &mut ae.left {
                AssignmentTarget::Identifier(_) => false,
                AssignmentTarget::MemberExpression(m) => inline_in_member(m, cand),
            };
            left_hit || inline_in_expr(&mut ae.right, cand)
        }
        Expression::ConditionalExpression(ce) => {
            inline_in_expr(&mut ce.test, cand)
                || inline_in_expr(&mut ce.consequent, cand)
                || inline_in_expr(&mut ce.alternate, cand)
        }
        // CallExpression handled above.
        Expression::CallExpression(_) => unreachable!("CallExpression handled before this match"),
        Expression::MemberExpression(m) => inline_in_member(m, cand),
        Expression::ArrayExpression(ae) => ae
            .elements
            .iter_mut()
            .flatten()
            .any(|el| inline_in_expr(el, cand)),
        Expression::ObjectExpression(oe) => oe.properties.iter_mut().any(|prop| {
            // A computed key `[expr]` is a sub-expression to walk; a
            // plain identifier / string / number key is a property name.
            let key_hit = if prop.computed {
                if let PropertyKey::Expression(e) = &mut prop.key {
                    inline_in_expr(e, cand)
                } else {
                    false
                }
            } else {
                false
            };
            key_hit || inline_in_expr(&mut prop.value, cand)
        }),
    }
}

fn inline_in_member(
    m: &mut coding_adventures_javascript_ast::MemberExpression,
    cand: &InlineCandidate,
) -> bool {
    if inline_in_expr(&mut m.object, cand) {
        return true;
    }
    // Only a computed property `o[expr]` is a sub-expression to walk;
    // a non-computed `.name` is a property name.
    if m.computed {
        return inline_in_expr(&mut m.property, cand);
    }
    false
}

/// Substitute parameter identifiers with their argument expressions in
/// a clone of the callee body. A bare identifier whose name is in
/// `map` becomes the (cloned) argument; everything else recurses.
/// Property names (non-computed member `.x`, non-computed object key)
/// are never substituted — they aren't variable references.
fn substitute(expr: &mut Expression, map: &HashMap<String, Expression>) {
    match expr {
        Expression::Identifier(id) => {
            if let Some(arg) = map.get(&id.name) {
                *expr = arg.clone();
            }
        }
        Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::BigIntLiteral(_)
        | Expression::UndefinedLiteral(_) => {}
        Expression::BinaryExpression(be) => {
            substitute(&mut be.left, map);
            substitute(&mut be.right, map);
        }
        Expression::LogicalExpression(le) => {
            substitute(&mut le.left, map);
            substitute(&mut le.right, map);
        }
        Expression::UnaryExpression(ue) => substitute(&mut ue.argument, map),
        Expression::AssignmentExpression(ae) => {
            // The left side is an assignment *target*. In the safe slice
            // EXPR's only free identifiers are parameters; a parameter
            // appearing as a bare assignment target would write to the
            // substituted argument. We substitute the member-object side
            // and the right-hand side; a bare-identifier target is left
            // as-is (substituting a literal there is impossible and an
            // identifier target keeps the write well-defined).
            if let AssignmentTarget::MemberExpression(m) = &mut ae.left {
                substitute(&mut m.object, map);
                if m.computed {
                    substitute(&mut m.property, map);
                }
            }
            substitute(&mut ae.right, map);
        }
        Expression::ConditionalExpression(ce) => {
            substitute(&mut ce.test, map);
            substitute(&mut ce.consequent, map);
            substitute(&mut ce.alternate, map);
        }
        Expression::CallExpression(ce) => {
            substitute(&mut ce.callee, map);
            for a in &mut ce.arguments {
                substitute(a, map);
            }
        }
        Expression::MemberExpression(m) => {
            substitute(&mut m.object, map);
            if m.computed {
                substitute(&mut m.property, map);
            }
        }
        Expression::ArrayExpression(ae) => {
            for el in ae.elements.iter_mut().flatten() {
                substitute(el, map);
            }
        }
        Expression::ObjectExpression(oe) => {
            for prop in &mut oe.properties {
                if prop.computed {
                    if let PropertyKey::Expression(e) = &mut prop.key {
                        substitute(e, map);
                    }
                }
                substitute(&mut prop.value, map);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    //! Tests pin the public contract (name, policy, cost, deps), the
    //! `PassPipeline` integration, and the inlining behaviour itself —
    //! driven end-to-end through the real source → bridge → inline →
    //! emit roundtrip so they exercise the exact AST shape the parser
    //! produces.
    use super::*;
    use coding_adventures_closure_emitter::{emit, EmitOptions};
    use coding_adventures_closure_pass_constant_fold::ConstantFoldPass;
    use coding_adventures_closure_pass_pipeline::{PassContext, PassPipeline, PipelineOutput};
    use coding_adventures_correlation_vector::CVLog;
    use coding_adventures_javascript_ast::{Program, SourceType};
    use coding_adventures_javascript_parser::{bridge, parse_javascript_typed};
    use coding_adventures_javascript_tokens::EsVersion;
    use coding_adventures_type_sidecar::Sidecar;

    fn program() -> Program {
        Program::new("prog.1".to_string(), EsVersion::Es2025, SourceType::Module)
    }

    /// Parse `src`, bridge it to a typed `Program`, run `InlinePass`,
    /// and emit the result as minified JS — the same chain closurec's
    /// SIMPLE level uses. Returns the emitted string.
    fn inline_source(src: &str) -> String {
        let es = EsVersion::Es2025;
        let node = parse_javascript_typed(src, es).expect("parse");
        let prog = bridge::grammar_to_program(&node, es).expect("bridge");

        let pass = InlinePass::new();
        let sidecar = Sidecar::new();
        let mut cv = CVLog::new(false);
        let out = pass
            .run(PassContext {
                program: &prog,
                sidecar: &sidecar,
                cv: &mut cv,
            })
            .expect("inline");

        let mut cv2 = CVLog::new(false);
        let opts = EmitOptions {
            source_map: false,
            ..Default::default()
        };
        emit(&out.program, &sidecar, &mut cv2, &opts)
            .expect("emit")
            .code
    }

    // ----- metadata contract -----

    #[test]
    fn name_is_inline() {
        assert_eq!(InlinePass::new().name(), "inline");
    }

    #[test]
    fn iteration_policy_is_fixed_point() {
        assert_eq!(
            InlinePass::new().iteration_policy(),
            IterationPolicy::FixedPoint
        );
    }

    #[test]
    fn cost_is_four_pass_units() {
        assert_eq!(InlinePass::new().cost(), 4);
    }

    #[test]
    fn depends_on_constant_fold() {
        let p = InlinePass::new();
        assert_eq!(p.depends_on(), &["constant-fold"]);
    }

    #[test]
    fn invalidates_empty_in_v1() {
        assert!(InlinePass::new().invalidates().is_empty());
    }

    #[test]
    fn run_on_empty_program_returns_unchanged_identity() {
        let pass = InlinePass::new();
        let prog = program();
        let sidecar = Sidecar::new();
        let mut cv = CVLog::new(true);

        let ctx = PassContext {
            program: &prog,
            sidecar: &sidecar,
            cv: &mut cv,
        };
        let out = pass.run(ctx).expect("pass should succeed");

        assert_eq!(out.program.cv, prog.cv);
        assert_eq!(out.program.version, prog.version);
        assert_eq!(out.program.source_type, prog.source_type);
        assert!(!out.changed);
        assert!(out.contributions.is_empty());
        assert!(out.diagnostics.is_empty());
        assert_eq!(out.stats.nodes_touched, 1);
    }

    #[test]
    fn pipeline_orders_constant_fold_before_inline() {
        let mut pipeline = PassPipeline::new();
        pipeline.add(Box::new(InlinePass::new()));
        pipeline.add(Box::new(ConstantFoldPass::new()));

        let mut cv = CVLog::new(true);
        let out: PipelineOutput = pipeline
            .run(program(), &Sidecar::new(), &mut cv)
            .expect("pipeline should run cleanly");

        assert_eq!(
            out.execution_order,
            vec!["constant-fold".to_string(), "inline".to_string()],
            "inline must run after constant-fold per CLOC06 canonical order"
        );
        assert!(out.stats.contains_key("constant-fold"));
        assert!(out.stats.contains_key("inline"));
    }

    #[test]
    fn pipeline_runs_inline_as_solo_pass() {
        let mut pipeline = PassPipeline::new();
        pipeline.add(Box::new(InlinePass::new()));

        let mut cv = CVLog::new(true);
        let out = pipeline.run(program(), &Sidecar::new(), &mut cv).unwrap();
        assert_eq!(out.execution_order, vec!["inline".to_string()]);
        assert_eq!(out.stats["inline"].nodes_touched, 1);
        // The pipeline now iterates FixedPoint passes to a fixed point;
        // a non-changing solo pass converges in one sweep, so the old
        // "not-yet-iterated" limitation note is gone.
        assert!(
            !out.diagnostics
                .iter()
                .any(|d| d.group.0 == "pipeline.fixed-point-not-yet-iterated"),
            "the not-yet-iterated note must be gone now that the pipeline iterates; got {:?}",
            out.diagnostics
        );
    }

    #[test]
    fn pass_is_default_and_clone() {
        let _a: InlinePass = Default::default();
        let _b: InlinePass = InlinePass::new();
        let _c = _b;
        let _d = _c.clone();
    }

    // =====================================================================
    // Inlining behaviour (source → bridge → inline → emit)
    // =====================================================================
    //
    // NOTE on whitespace: these assert against raw `closure-emitter`
    // output (binary operators get spaces, function declarations get a
    // trailing `;`). The dead callee declaration is intentionally left
    // in place — remove-unused-vars / treeshake remove it downstream;
    // here we only check the call was substituted.

    #[test]
    fn inlines_single_use_double() {
        // The signature case: `double(7)` is replaced by `7 * 2`. The
        // (now dead) declaration stays — its removal is a later pass.
        assert_eq!(
            inline_source("function double(x) { return x * 2; } log(double(7));"),
            "function double(x){return x * 2};log(7 * 2);"
        );
    }

    #[test]
    fn inlines_identity_with_identifier_arg() {
        // A bare-identifier argument substitutes cleanly: `id(value)`
        // → `value`.
        assert_eq!(
            inline_source("function id(v) { return v; } print(id(value));"),
            "function id(v){return v};print(value);"
        );
    }

    #[test]
    fn inlines_two_param_function() {
        assert_eq!(
            inline_source("function add(a, b) { return a + b; } use(add(p, q));"),
            "function add(a,b){return a + b};use(p + q);"
        );
    }

    #[test]
    fn preserves_property_name_on_substitution() {
        // `o.x` with `o` the parameter: substitute `o` → `obj`, but the
        // `.x` property name must NOT be touched.
        assert_eq!(
            inline_source("function get(o) { return o.x; } use(get(obj));"),
            "function get(o){return o.x};use(obj.x);"
        );
    }

    #[test]
    fn inlines_computed_member() {
        // A computed `o[i]` IS a use position — both params substitute.
        assert_eq!(
            inline_source("function at(o, i) { return o[i]; } use(at(arr, idx));"),
            "function at(o,i){return o[i]};use(arr[idx]);"
        );
    }

    #[test]
    fn inlines_nested_call_argument() {
        // The call can be nested inside another call's arguments.
        assert_eq!(
            inline_source("function double(x) { return x * 2; } outer(inner(double(5)));"),
            "function double(x){return x * 2};outer(inner(5 * 2));"
        );
    }

    #[test]
    fn does_not_inline_multi_use_function() {
        // Two call sites → not the single-use slice. Left unchanged.
        assert_eq!(
            inline_source("function d(x) { return x * 2; } a(d(1)); b(d(2));"),
            "function d(x){return x * 2};a(d(1));b(d(2));"
        );
    }

    #[test]
    fn does_not_inline_recursive_function() {
        // `f` appears free in its own body (the inner call), so it is
        // not a candidate — recursion is excluded by the capture guard.
        assert_eq!(
            inline_source("function f(x) { return f(x); } g(f(1));"),
            "function f(x){return f(x)};g(f(1));"
        );
    }

    #[test]
    fn does_not_inline_body_with_free_global() {
        // `g` is a free identifier (not a parameter), so substituting
        // the body at the call site could capture a differently-scoped
        // `g`. Rejected.
        assert_eq!(
            inline_source("function f(x) { return x + g; } h(f(1));"),
            "function f(x){return x + g};h(f(1));"
        );
    }

    #[test]
    fn does_not_inline_shadowed_name() {
        // The name `f` is declared twice (the top-level function and a
        // parameter `f` of `uses`), so a use of `f` could resolve to
        // either binding. Rejected by the shadow guard.
        assert_eq!(
            inline_source("function f(x) { return x * 2; } function uses(f) { return f(1); }"),
            "function f(x){return x * 2};function uses(f){return f(1)};"
        );
    }

    #[test]
    fn does_not_inline_on_arity_mismatch() {
        // Call passes one argument to a two-parameter function — the
        // arity check fails, so the call is left intact.
        assert_eq!(
            inline_source("function add(a, b) { return a + b; } k(add(1));"),
            "function add(a,b){return a + b};k(add(1));"
        );
    }

    #[test]
    fn does_not_inline_side_effecting_argument() {
        // The argument `g()` has a side effect; substituting it for a
        // parameter could drop or duplicate that effect, so the call is
        // left intact. (`g` being free also makes the use-count 2 — `f`
        // plus `g` — but the simple-arg gate is the operative reason.)
        assert_eq!(
            inline_source("function f(x) { return x * 2; } m(f(g()));"),
            "function f(x){return x * 2};m(f(g()));"
        );
    }

    #[test]
    fn does_not_inline_non_call_value_use() {
        // `f` is used once, but as a *value* (passed to `h`), not
        // called. There is no call to substitute, so nothing changes.
        assert_eq!(
            inline_source("function f(x) { return x * 2; } h(f);"),
            "function f(x){return x * 2};h(f);"
        );
    }

    #[test]
    fn does_not_inline_multi_statement_body() {
        // Body has a local + a return — not the `{ return EXPR; }`
        // shape — so it is not a candidate.
        assert_eq!(
            inline_source("function f(x) { var t = x * 2; return t; } use(f(3));"),
            "function f(x){var t=x * 2;return t};use(f(3));"
        );
    }
}
