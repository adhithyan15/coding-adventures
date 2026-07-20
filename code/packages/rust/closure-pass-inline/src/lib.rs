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
//!   5. **Every use of `f` is an inlinable call** with
//!      `arguments.len() == params.len()` — i.e. there is no use of
//!      `f` as a *value* (`g(f)`) and no call with the wrong arity or
//!      side-effecting arguments. Then inlining *all* the calls leaves
//!      `f` unreferenced so the later passes delete it; if even one
//!      use is not an inlinable call we decline the whole function
//!      (partial inlining would duplicate the body *and* keep the
//!      declaration — usually a net loss).
//!   6. **Every argument is side-effect-free** — a literal or a bare
//!      identifier. Then substituting an argument for a parameter
//!      that the body uses zero, one, or many times can neither drop
//!      nor duplicate a side effect, so the argument-evaluation
//!      hazard vanishes.
//!
//! # Single-use vs. multi-use (the only "is it worth it?" knob)
//!
//! All of the above is about *soundness*. The single remaining
//! question — *is it worth it?* — splits on the number of call sites:
//!
//!   * **One call site** → always inline. One substitution, the
//!     declaration removed: a strict size win.
//!   * **N > 1 call sites** → inline only when the body fits a
//!     conservative size budget — `expr_node_count(body) <= 2 +
//!     params.len()`, i.e. the substituted body is no larger than the
//!     call it replaces (see [`multiuse_budget_ok`]). Then duplicating
//!     it across the sites never grows the output, and removing the
//!     declaration is a pure saving. A body too large to duplicate is
//!     left alone.
//!
//! Everything outside this subset is left untouched (`changed` stays
//! `false`). Broader inlining — function *expressions*, bodies with
//! locals/branches, multi-use bodies above the budget — is future work
//! on the same walker.
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
use coding_adventures_correlation_vector::Contribution;
use serde_json::json;
use coding_adventures_javascript_ast::statement::{ReturnStatement, TaggedStatement};
use coding_adventures_javascript_ast::{
    ArrowBody, AssignmentExpression, AssignmentOperator, AssignmentTarget, BindingTarget,
    BlockStatement,
    CallExpression, ClassMember, Declaration, Expression, ExpressionStatement, ForInit,
    FunctionDeclaration,
    Identifier, IfStatement, NullLiteral, Program, ProgramItem, ObjectMember, PropertyKey,
    Statement, VarKind, VariableDeclaration, VariableDeclarator,
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
        // Real transform: inline top-level leaf functions whose body is
        // `return EXPR` with no free identifiers — single-use always,
        // multi-use under a size budget. See
        // [`inline_program`] and the crate-level docs for the full
        // safety argument. An empty / construct-free program is left
        // untouched (`changed = false`, `nodes_touched = 1`).
        let mut program = ctx.program.clone();
        let mut nodes_touched: u32 = 1; // the program root
        let mut inlined: Vec<InlineRecord> = Vec::new();
        let changed = inline_program(&mut program, &mut nodes_touched, &mut inlined);

        // CV provenance (#89): record every function we inlined as an
        // `inlined` contribution carrying `{name, sites}` — the original
        // source name of the helper and how many call sites its body was
        // substituted into. Inlining *dissolves* a function: its
        // declaration becomes unreferenced (later removed) and its body is
        // copied into each caller, so without this record the minified
        // output has no trace that a `helper(x)` call ever existed. The
        // pipeline attaches these to the program-root CV entry, so a
        // `--correlation_vector` consumer can map inlined code back to the
        // helper it came from. Records come out in program (source) order,
        // one per inlined function, so the emitted list is deterministic
        // run to run.
        //
        // This is the inline *table* (name → site-count), attached at the
        // program root. Tagging each substituted body's OWN CV id
        // (per-output-span provenance) needs the log threaded through the
        // clone-and-substitute recursion and is a documented follow-up,
        // mirroring the rename passes' coarse-table-first approach.
        let contributions: Vec<Contribution> = inlined
            .into_iter()
            .map(|rec| Contribution {
                source: "inline".to_string(),
                tag: "inlined".to_string(),
                meta: [
                    ("name".to_string(), json!(rec.name)),
                    ("sites".to_string(), json!(rec.sites)),
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
// Inlining implementation (top-level leaf functions, single- and multi-use)
// =========================================================================

/// One inlinable function: its name, parameter names in order, and a
/// clone of the single `return` expression to substitute at the call
/// site.
struct InlineCandidate {
    name: String,
    params: Vec<String>,
    return_expr: Expression,
}

/// One inlining *event* for CV provenance (#89): the original source name of
/// a function whose body was substituted into its call site(s), and how many
/// sites were rewritten. `run` turns each record into an `inlined`
/// contribution `{name, sites}`. The expression inliner (Phase 3) may rewrite
/// several sites at once; the statement-helper inliners (Phases 4 and 5) each
/// fire on a single-use helper, so their `sites` is always 1.
struct InlineRecord {
    name: String,
    sites: usize,
}

/// Walk the whole program and inline every qualifying top-level
/// function (single-use always; multi-use under the size budget).
/// Returns whether anything changed.
fn inline_program(
    program: &mut Program,
    nodes_touched: &mut u32,
    inlined: &mut Vec<InlineRecord>,
) -> bool {
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
    // NOTE: no early return on an empty expression-candidate set — Phase 4
    // (void statement-helper inlining) runs independently of the
    // expression candidates, so the empty Phase-3 loop below just falls
    // through to it.

    // Phase 3 — for each candidate, decide whether to inline ALL its
    // call sites. We `tally` two numbers in one walk:
    //   * `uses`      — every binding-use of the name;
    //   * `inlinable` — calls `name(args)` with matching arity and
    //                   side-effect-free arguments.
    //
    // We inline only when `uses == inlinable` (and `uses > 0`): every
    // use is an inlinable call, so after substituting them all the
    // function declaration is unreferenced and the later
    // remove-unused-vars / treeshake passes delete it. If even one use
    // is a non-call value (`g(f)`) or a non-inlinable call (wrong
    // arity / side-effecting argument), we skip the whole function —
    // partial inlining would duplicate the body *and* keep the
    // declaration, usually a net loss.
    //
    //   * 1 call site  → always inline (a strict win — one substitution,
    //     the declaration removed).
    //   * N>1 sites    → inline only when the body fits the size budget
    //     (see [`multiuse_budget_ok`]) so duplicating it across the
    //     sites never grows the output.
    //
    // Counting + substituting on the progressively-mutated program is
    // sound: an inlined body contains only the call's own simple
    // arguments, so it neither adds nor removes uses of any *other*
    // candidate's name.
    let mut changed = false;
    for cand in &candidates {
        let tally = tally_program(program, cand);
        if tally.uses == 0 || tally.uses != tally.inlinable {
            continue; // unused, a non-call value use, or a non-inlinable call
        }
        if tally.uses > 1 && !multiuse_budget_ok(cand) {
            continue; // multi-use body too large to duplicate — net loss
        }
        if inline_all_calls(program, cand) {
            changed = true;
            // CV: the body was substituted into all `tally.uses` call sites
            // (gate above guarantees `uses == inlinable`), after which the
            // declaration is unreferenced.
            inlined.push(InlineRecord {
                name: cand.name.clone(),
                sites: tally.uses,
            });
        }
    }

    // Phase 4 — CLOC15 PR-1: inline single-use *void multi-statement*
    // helpers by splicing their (parameter-substituted, locals-renamed)
    // body statements at the call site. This is the statement-level
    // counterpart to the expression-swap above; see
    // [`inline_void_statement_helpers`] and the CLOC15 spec for the full
    // soundness argument. It runs after the expression inliner because
    // the two operate on disjoint function shapes (`{ return EXPR; }` vs.
    // a multi-statement void body), so neither perturbs the other's
    // candidate set, and the declaration-count map stays valid (inlining
    // removes call sites, never declarations).
    changed |= inline_void_statement_helpers(program, &decl_counts, nodes_touched, inlined);

    // Phase 5 — CLOC15 PR-3/PR-5: inline a single-use multi-statement helper
    // whose RESULT IS USED, by hoisting its body before the enclosing
    // statement and consuming the tail-return value at the call site. The
    // sound value positions are:
    //   - PR-3: the call is the entire initializer of a single-declarator
    //     `var`/`let`/`const` (`const r = f(x)`), captured into a fresh temp;
    //   - PR-5: the call is the entire argument of a `return` (`return f(x)`),
    //     re-emitted as the caller's own `return E` — no temp, since the
    //     value flows straight out and `return` is a terminator.
    // Both reject the call appearing under a short-circuit / conditional
    // operator (`a && f(x)`, `c ? f(x) : y`), where hoisting would change
    // evaluation. See [`inline_valued_statement_helpers`]. Runs after Phase 4
    // because the void pass consumes the discarded-statement uses first, so
    // this pass only ever sees the value-position use.
    changed |= inline_valued_statement_helpers(program, &decl_counts, nodes_touched, inlined);

    changed
}

/// The counts [`inline_program`] needs per candidate, gathered in a single
/// walk: total binding-uses of the name; how many are *simple-arg* inlinable
/// calls (the expression inliner's gate); and how many are name+arity calls
/// regardless of argument simplicity (the statement inliner's gate — CLOC15
/// PR-4a hoists non-simple arguments into temps, so it does not require them
/// to be simple).
#[derive(Default)]
struct Tally {
    uses: usize,
    inlinable: usize,
    arity_calls: usize,
}

/// Is `ce` a call we can inline for `cand` — `cand.name(args)` with the
/// right number of side-effect-free arguments?
fn is_inlinable_call(ce: &CallExpression, cand: &InlineCandidate) -> bool {
    is_name_arity_call(ce, cand) && ce.arguments.iter().all(is_simple_arg)
}

/// Is `ce` a call to `cand.name` with the right ARITY (any arguments)? The
/// statement-inlining paths match on this and materialise non-simple
/// arguments into temps; the expression inliner additionally requires
/// [`is_simple_arg`] via [`is_inlinable_call`].
fn is_name_arity_call(ce: &CallExpression, cand: &InlineCandidate) -> bool {
    matches!(&*ce.callee, Expression::Identifier(id) if id.name == cand.name)
        && ce.arguments.len() == cand.params.len()
}

/// Conservative size budget for inlining a callee used at MORE THAN ONE
/// site. A call `f(a₁, …, aₙ)` is a tree of `2 + n` nodes (the call
/// node, the callee identifier, and one node per side-effect-free
/// argument). Substituting the body replaces each param (1 node) with
/// its argument (also 1 node — a literal or identifier), so the
/// substituted body has exactly `expr_node_count(return_expr)` nodes.
/// Requiring that to be `<= 2 + params.len()` guarantees the
/// replacement is no larger than the call it replaces — so inlining at
/// every site never grows the output, and removing the now-dead
/// declaration is a pure saving.
///
/// Single-use inlining needs no budget (one substitution, declaration
/// gone — always a win); this gate applies only to N>1 sites. The
/// post-inline `constant-fold` sweep often shrinks literal-argument
/// results further (`f(7)` → `7 * 2` → `14`), but we don't lean on
/// that here.
fn multiuse_budget_ok(cand: &InlineCandidate) -> bool {
    expr_node_count(&cand.return_expr) <= 2 + cand.params.len()
}

/// Structural node count of an expression tree (every [`Expression`]
/// node counts as 1, plus its children). Used by [`multiuse_budget_ok`].
fn expr_node_count(expr: &Expression) -> usize {
    1 + match expr {
        Expression::Identifier(_)
        | Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::BigIntLiteral(_)
        // A regex literal is an inert leaf (no sub-expression, references no
        // identifier) — it counts like a StringLiteral: 0 sub-nodes.
        | Expression::RegExpLiteral(_)
        // `this` is a single leaf keyword — it contributes one node like the
        // other leaves (which this arm counts as 0 sub-nodes).
        | Expression::ThisExpression(_)
        | Expression::Super(_)
        | Expression::NewTarget(_)
        | Expression::ImportMeta(_)
        | Expression::UndefinedLiteral(_) => 0,
        Expression::BinaryExpression(be) => expr_node_count(&be.left) + expr_node_count(&be.right),
        Expression::LogicalExpression(le) => expr_node_count(&le.left) + expr_node_count(&le.right),
        Expression::UnaryExpression(ue) => expr_node_count(&ue.argument),
        Expression::UpdateExpression(ue) => expr_node_count(&ue.argument),
        Expression::AssignmentExpression(ae) => {
            let left = match &ae.left {
                AssignmentTarget::Identifier(_) => 1,
                AssignmentTarget::MemberExpression(m) => {
                    1 + expr_node_count(&m.object) + expr_node_count(&m.property)
                }
            };
            left + expr_node_count(&ae.right)
        }
        Expression::ConditionalExpression(ce) => {
            expr_node_count(&ce.test)
                + expr_node_count(&ce.consequent)
                + expr_node_count(&ce.alternate)
        }
        Expression::CallExpression(ce) => {
            expr_node_count(&ce.callee) + ce.arguments.iter().map(expr_node_count).sum::<usize>()
        }
        Expression::NewExpression(ne) => {
            expr_node_count(&ne.callee) + ne.arguments.iter().map(expr_node_count).sum::<usize>()
        }
        Expression::SequenceExpression(se) => se.expressions.iter().map(expr_node_count).sum(),
        Expression::MemberExpression(m) => {
            expr_node_count(&m.object) + expr_node_count(&m.property)
        }
        // `a?.b` / `a?.[k]` — same child shape as a plain member access.
        Expression::OptionalMemberExpression(m) => {
            expr_node_count(&m.object) + expr_node_count(&m.property)
        }
        // `a?.()` — same child shape as an ordinary call.
        Expression::OptionalCallExpression(ce) => {
            expr_node_count(&ce.callee) + ce.arguments.iter().map(expr_node_count).sum::<usize>()
        }
        // A chain expression transparently wraps one inner expression.
        Expression::ChainExpression(c) => expr_node_count(&c.expression),
        Expression::ArrayExpression(ae) => ae.elements.iter().flatten().map(expr_node_count).sum(),
        Expression::ObjectExpression(oe) => oe
            .properties
            .iter()
            .map(|member| match member {
                ObjectMember::Property(p) => {
                    let key = match &p.key {
                        PropertyKey::Expression(e) => expr_node_count(e),
                        _ => 1,
                    };
                    key + expr_node_count(&p.value)
                }
                // A spread `...expr` has no key — count one node for the
                // spread itself plus the node count of its argument.
                ObjectMember::Spread(s) => 1 + expr_node_count(&s.argument),
            })
            .sum(),
        // A function *value* is heavy: count its params and one unit per
        // body statement. This is only a size heuristic for the multi-use
        // budget (never a correctness input), and counting the body keeps
        // a candidate whose value embeds a function from looking
        // deceptively cheap.
        Expression::FunctionExpression(fe) => fe.params.len() + fe.body.body.len(),
        // A class *value* is heavy: weigh its `extends` operand plus, for each
        // method, its params and one unit per body statement — the same size
        // heuristic the `FunctionExpression` arm applies to a function value.
        // This is only a budget heuristic (never a correctness input); counting
        // the method bodies keeps a candidate embedding a class from looking
        // deceptively cheap.
        Expression::ClassExpression(ce) => {
            ce.super_class.as_ref().map_or(0, |s| expr_node_count(s))
                + ce.body
                    .iter()
                    .map(|m| match m {
                        ClassMember::Method(md) => {
                            md.value.params.len() + md.value.body.body.len()
                        }
                        // A field weighs its initializer's node count (a bare
                        // field `x;` weighs 0) — the same size heuristic applied
                        // to any expression value.
                        ClassMember::Field(fd) => {
                            fd.value.as_ref().map_or(0, expr_node_count)
                        }
                        // A static-init block weighs one unit per body statement,
                        // the same size heuristic a function body uses.
                        ClassMember::StaticBlock(b) => b.body.len(),
                    })
                    .sum::<usize>()
        }
        // Same size heuristic for an arrow: params plus its body weight —
        // one unit per statement for a block, or the node count of the
        // concise expression.
        Expression::ArrowFunctionExpression(ae) => {
            ae.params.len()
                + match &ae.body {
                    ArrowBody::Block(b) => b.body.len(),
                    ArrowBody::Expression(e) => expr_node_count(e),
                }
        }
        // A template literal weighs one unit per quasi (leaf strings, nothing
        // to recurse) plus the node weight of each `${…}` insert expression.
        Expression::TemplateLiteral(t) => {
            t.quasis.len() + t.expressions.iter().map(expr_node_count).sum::<usize>()
        }
        // A tagged template weighs its tag callee plus the template it applies
        // (quasis + each `${…}` insert expression).
        Expression::TaggedTemplateExpression(t) => {
            expr_node_count(&t.tag)
                + t.quasi.quasis.len()
                + t.quasi.expressions.iter().map(expr_node_count).sum::<usize>()
        }
        // `...arg` — the spread weighs exactly its single inner argument.
        Expression::SpreadElement(s) => expr_node_count(&s.argument),
        Expression::YieldExpression(y) => y.argument.as_ref().map_or(0, |a| expr_node_count(a)),
        Expression::AwaitExpression(a) => expr_node_count(&a.argument),
        Expression::ImportExpression(e) => expr_node_count(&e.source),
    }
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

    // A default parameter (`function f(a = expr)`) can't be inlined by simple
    // positional argument binding: when a call omits the argument or passes
    // `undefined`, the default value applies — semantics a plain `const a = arg`
    // substitution does not reproduce. Decline the whole candidate; the function
    // is left intact for the other passes. (A rest param is fine — it binds a
    // name with no default expression.)
    if fd.params.iter().any(|p| p.default_value().is_some()) {
        return None;
    }

    // Parameter names must be distinct, or the substitution map would
    // be ambiguous. (`function f(a, a)` is a syntax error in strict
    // mode anyway, but we never assume the parser rejected it.)
    let mut params: Vec<String> = Vec::with_capacity(fd.params.len());
    let mut seen = HashSet::new();
    for p in &fd.params {
        let id = p.binding_identifier();
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
                let id = p.binding_identifier();
                *out.entry(id.name.clone()).or_insert(0) += 1;
            }
            for s in &fd.body.body {
                count_decl_names_stmt(s, out, nodes_touched);
            }
        }
        // A class declaration binds its name, and each method's params + locals
        // are declared names — count all, mirroring the function arm. Counting
        // method params keeps inline-collision detection conservative.
        // An import declaration has no inlinable body and binds foreign-linked
        // names — leave it untouched.
        Declaration::ImportDeclaration(_) => {}
        Declaration::ExportNamedDeclaration(_) => {}
        Declaration::ExportDefaultDeclaration(_) => {}
        Declaration::ExportAllDeclaration(_) => {}
        Declaration::ClassDeclaration(cd) => {
            *out.entry(cd.id.name.clone()).or_insert(0) += 1;
            for member in &cd.body {
                match member {
                    // A method contributes its params + body-declared locals.
                    ClassMember::Method(m) => {
                        for p in &m.value.params {
                            let id = p.binding_identifier();
                            *out.entry(id.name.clone()).or_insert(0) += 1;
                        }
                        for s in &m.value.body.body {
                            count_decl_names_stmt(s, out, nodes_touched);
                        }
                    }
                    // A field declares no statement-scope name — its key is a
                    // property name, not a binding, and its initializer
                    // introduces no declarations.
                    ClassMember::Field(_) => {}
                    // A static-init block's inner statements declare their own
                    // locals — count them conservatively (mirroring the method
                    // body) so inline-collision detection stays sound.
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
                // Count the catch `param` as a program-wide binding so the
                // exact-decl-count shadow guard (CLOC16 Slice B1) stays exact:
                // a free body identifier resolving to a name ALSO bound by a
                // catch param is multiply-declared, so it is not treated as
                // unshadowable. Recurse into the three blocks.
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
            // shadowing name can't slip past the shadow guard and make an
            // unsound inline. The compiler flags the omission.
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
        // A regex literal is an inert leaf (no sub-expression, references no
        // identifier) — these traversals treat it exactly like a StringLiteral.
        | Expression::RegExpLiteral(_)
        // `this` is a leaf keyword — it binds/references no identifier and has
        // no sub-expression, so these traversals do nothing for it. (It is
        // deliberately NOT treated as a freely-substitutable primary: `this` is
        // bound at the call site, so the inliner's triv-/pure-expression
        // predicates leave it conservative.)
        | Expression::ThisExpression(_)
        | Expression::Super(_)
        | Expression::NewTarget(_)
        | Expression::ImportMeta(_)
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
        Expression::UpdateExpression(ue) => collect_binding_idents_expr(&ue.argument, out),
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
        Expression::NewExpression(ne) => {
            collect_binding_idents_expr(&ne.callee, out);
            for a in &ne.arguments {
                collect_binding_idents_expr(a, out);
            }
        }
        Expression::SequenceExpression(se) => {
            for e in &se.expressions {
                collect_binding_idents_expr(e, out);
            }
        }
        Expression::MemberExpression(m) => {
            collect_binding_idents_member(&m.object, &m.property, m.computed, out)
        }
        // `a?.b` / `a?.[k]` — same as a plain member access.
        Expression::OptionalMemberExpression(m) => {
            collect_binding_idents_member(&m.object, &m.property, m.computed, out)
        }
        // `a?.()` — same as an ordinary call.
        Expression::OptionalCallExpression(ce) => {
            collect_binding_idents_expr(&ce.callee, out);
            for a in &ce.arguments {
                collect_binding_idents_expr(a, out);
            }
        }
        // A chain expression transparently wraps its optional-chain spine.
        Expression::ChainExpression(c) => collect_binding_idents_expr(&c.expression, out),
        Expression::ArrayExpression(ae) => {
            for el in ae.elements.iter().flatten() {
                collect_binding_idents_expr(el, out);
            }
        }
        Expression::ObjectExpression(oe) => {
            for member in &oe.properties {
                match member {
                    ObjectMember::Property(prop) => {
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
                    // Object spread `...expr` — recurse into the spread
                    // argument the same way the property arm recurses into
                    // prop.value.
                    ObjectMember::Spread(s) => {
                        collect_binding_idents_expr(&s.argument, out);
                    }
                }
            }
        }
        // A function *value* binds its own name (if named) and its params
        // at its boundary; record them so an inline never captures or
        // collides with them. (Its body's own `var`/`let`/`const` live in
        // a nested scope that this top-level-helper inliner does not
        // substitute into.)
        Expression::FunctionExpression(fe) => {
            if let Some(id) = &fe.id {
                out.insert(id.name.clone());
            }
            for p in &fe.params {
                out.insert(p.binding_identifier().name.clone());
                // A default's `right` reads names too (`a = SOME_NAME`); over-
                // collect them so a fresh name the void splice mints can never
                // collide with a name a nested default reads.
                if let Some(def) = p.default_value() {
                    collect_binding_idents_expr(def, out);
                }
            }
        }
        // A class *value* binds its own name (if named) and each method
        // value's name (if named) and params at their boundaries; record
        // them so an inline never captures or collides with them — mirroring
        // the `FunctionExpression` arm, which records boundary bindings but
        // does NOT recurse into the (nested-scope) function bodies. The
        // `extends` operand, however, is evaluated in the ENCLOSING scope
        // (like a callee), so recurse into it as this walk does for other
        // operand-position sub-expressions. A method KEY is a property name,
        // not a binding.
        Expression::ClassExpression(ce) => {
            if let Some(id) = &ce.id {
                out.insert(id.name.clone());
            }
            if let Some(sup) = &ce.super_class {
                collect_binding_idents_expr(sup, out);
            }
            for member in &ce.body {
                match member {
                    ClassMember::Method(m) => {
                        if let Some(id) = &m.value.id {
                            out.insert(id.name.clone());
                        }
                        for p in &m.value.params {
                            out.insert(p.binding_identifier().name.clone());
                            if let Some(def) = p.default_value() {
                                collect_binding_idents_expr(def, out);
                            }
                        }
                    }
                    // A field's initializer is evaluated at construction; the
                    // computed key is evaluated in the enclosing scope. Over-
                    // collect any binding idents inside either so an inline
                    // never captures or collides with them. The field KEY name
                    // itself is a property name, not a binding.
                    ClassMember::Field(f) => {
                        if let PropertyKey::Expression(e) = &f.key {
                            collect_binding_idents_expr(e, out);
                        }
                        if let Some(v) = &f.value {
                            collect_binding_idents_expr(v, out);
                        }
                    }
                    // A static-init block's statements run at construction; over-
                    // collect every identifier they touch (via the broader
                    // `collect_used_idents_stmt`) so an inline never captures or
                    // collides with a static-block-local binding. Over-collecting
                    // into this avoid-set only makes the pass more conservative.
                    ClassMember::StaticBlock(b) => {
                        for s in &b.body {
                            collect_used_idents_stmt(s, out);
                        }
                    }
                }
            }
        }
        // An arrow binds its params at its boundary (it has no name);
        // record them so an inline never captures or collides with them.
        Expression::ArrowFunctionExpression(ae) => {
            for p in &ae.params {
                out.insert(p.binding_identifier().name.clone());
                if let Some(def) = p.default_value() {
                    collect_binding_idents_expr(def, out);
                }
            }
        }
        // A template literal introduces NO boundary bindings; its quasis are
        // leaf strings. Mirror the arrow arm's non-recursion into sub-bodies
        // (it only records params, and a template has none) — nothing to do.
        Expression::TemplateLiteral(_) => {}
        // A tagged template introduces no boundary bindings either (the tag is
        // a callee, the quasi a template) — mirror the template arm.
        Expression::TaggedTemplateExpression(_) => {}
        // `...arg` — recurse into the spread argument to collect its bindings.
        Expression::SpreadElement(s) => collect_binding_idents_expr(&s.argument, out),
        Expression::YieldExpression(y) => { if let Some(a) = &y.argument { collect_binding_idents_expr(a, out); } }
        Expression::AwaitExpression(a) => collect_binding_idents_expr(&a.argument, out),
        Expression::ImportExpression(e) => collect_binding_idents_expr(&e.source, out),
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

// ---- use + inlinable-call counting ---------------------------------------

/// Walk the whole program once and tally, for `cand`: every binding-use
/// of its name, and how many of those are inlinable calls. Declarations,
/// property names, and label names are not uses. Recurses into nested
/// function bodies because a call site can live anywhere.
fn tally_program(program: &Program, cand: &InlineCandidate) -> Tally {
    let mut t = Tally::default();
    for item in &program.body {
        match item {
            ProgramItem::Declaration(d) => tally_decl(d, cand, &mut t),
            ProgramItem::Statement(s) => tally_stmt(s, cand, &mut t),
        }
    }
    t
}

fn tally_decl(decl: &Declaration, cand: &InlineCandidate, t: &mut Tally) {
    match decl {
        Declaration::VariableDeclaration(vd) => {
            for d in &vd.declarations {
                if let Some(init) = &d.init {
                    tally_expr(init, cand, t);
                }
            }
        }
        Declaration::FunctionDeclaration(fd) => {
            for s in &fd.body.body {
                tally_stmt(s, cand, t);
            }
        }
        // Tally candidate uses inside a class declaration's heritage operand
        // and method bodies — missing one would let the pass inline a callee
        // that is still called from inside the class (a miscompile). Mirrors the
        // `Expression::ClassExpression` arm of `tally_expr`.
        // An import declaration has no inlinable body and binds foreign-linked
        // names — leave it untouched.
        Declaration::ImportDeclaration(_) => {}
        Declaration::ExportNamedDeclaration(_) => {}
        Declaration::ExportDefaultDeclaration(_) => {}
        Declaration::ExportAllDeclaration(_) => {}
        Declaration::ClassDeclaration(cd) => {
            if let Some(sup) = &cd.super_class {
                tally_expr(sup, cand, t);
            }
            for member in &cd.body {
                match member {
                    ClassMember::Method(m) => {
                        for s in &m.value.body.body {
                            tally_stmt(s, cand, t);
                        }
                    }
                    // SOUNDNESS: a candidate use inside a field initializer
                    // (or a computed key) is a real use — miss it and the pass
                    // would inline a callee still called at class construction.
                    ClassMember::Field(f) => {
                        if let PropertyKey::Expression(e) = &f.key {
                            tally_expr(e, cand, t);
                        }
                        if let Some(v) = &f.value {
                            tally_expr(v, cand, t);
                        }
                    }
                    // SOUNDNESS: a candidate use inside a static-init block's
                    // statements is a real use — they run at class construction.
                    ClassMember::StaticBlock(b) => {
                        for s in &b.body {
                            tally_stmt(s, cand, t);
                        }
                    }
                }
            }
        }
    }
}

fn tally_stmt(stmt: &Statement, cand: &InlineCandidate, t: &mut Tally) {
    match stmt {
        Statement::Declaration(d) => tally_decl(d, cand, t),
        Statement::Tagged(tagged) => match tagged {
            TaggedStatement::ExpressionStatement(es) => tally_expr(&es.expression, cand, t),
            TaggedStatement::BlockStatement(b) => {
                for s in &b.body {
                    tally_stmt(s, cand, t);
                }
            }
            TaggedStatement::IfStatement(is) => {
                tally_expr(&is.test, cand, t);
                tally_stmt(&is.consequent, cand, t);
                if let Some(alt) = &is.alternate {
                    tally_stmt(alt, cand, t);
                }
            }
            TaggedStatement::WhileStatement(ws) => {
                tally_expr(&ws.test, cand, t);
                tally_stmt(&ws.body, cand, t);
            }
            TaggedStatement::WithStatement(ws) => {
                tally_expr(&ws.object, cand, t);
                tally_stmt(&ws.body, cand, t);
            }
            TaggedStatement::DoWhileStatement(ds) => {
                tally_expr(&ds.test, cand, t);
                tally_stmt(&ds.body, cand, t);
            }
            TaggedStatement::ForStatement(fs) => {
                if let Some(init) = &fs.init {
                    match init {
                        ForInit::VariableDeclaration(vd) => {
                            for d in &vd.declarations {
                                if let Some(i) = &d.init {
                                    tally_expr(i, cand, t);
                                }
                            }
                        }
                        ForInit::Expression(e) => tally_expr(e, cand, t),
                    }
                }
                if let Some(test) = &fs.test {
                    tally_expr(test, cand, t);
                }
                if let Some(update) = &fs.update {
                    tally_expr(update, cand, t);
                }
                tally_stmt(&fs.body, cand, t);
            }
            TaggedStatement::ForInStatement(fs) => {
                match &fs.left {
                    ForInit::VariableDeclaration(vd) => {
                        for d in &vd.declarations {
                            if let Some(i) = &d.init {
                                tally_expr(i, cand, t);
                            }
                        }
                    }
                    ForInit::Expression(e) => tally_expr(e, cand, t),
                }
                tally_expr(&fs.right, cand, t);
                tally_stmt(&fs.body, cand, t);
            }
            TaggedStatement::ForOfStatement(fs) => {
                match &fs.left {
                    ForInit::VariableDeclaration(vd) => {
                        for d in &vd.declarations {
                            if let Some(i) = &d.init {
                                tally_expr(i, cand, t);
                            }
                        }
                    }
                    ForInit::Expression(e) => tally_expr(e, cand, t),
                }
                tally_expr(&fs.right, cand, t);
                tally_stmt(&fs.body, cand, t);
            }
            TaggedStatement::ReturnStatement(rs) => {
                if let Some(a) = &rs.argument {
                    tally_expr(a, cand, t);
                }
            }
            TaggedStatement::ThrowStatement(ts) => tally_expr(&ts.argument, cand, t),
            TaggedStatement::LabeledStatement(ls) => tally_stmt(&ls.body, cand, t),
            TaggedStatement::SwitchStatement(ss) => {
                tally_expr(&ss.discriminant, cand, t);
                for c in &ss.cases {
                    if let Some(test) = &c.test {
                        tally_expr(test, cand, t);
                    }
                    for s in &c.consequent {
                        tally_stmt(s, cand, t);
                    }
                }
            }
            TaggedStatement::TryStatement(ts) => {
                // Tally candidate uses / inlinable calls inside the three
                // blocks — a call site can live in any of them.
                for s in &ts.block.body {
                    tally_stmt(s, cand, t);
                }
                if let Some(h) = &ts.handler {
                    for s in &h.body.body {
                        tally_stmt(s, cand, t);
                    }
                }
                if let Some(f) = &ts.finalizer {
                    for s in &f.body {
                        tally_stmt(s, cand, t);
                    }
                }
            }
            // Labels live in a separate namespace; break/continue/empty
            // hold no variable uses.
            TaggedStatement::BreakStatement(_)
            | TaggedStatement::ContinueStatement(_)
            | TaggedStatement::EmptyStatement(_)
            | TaggedStatement::DebuggerStatement(_) => {}
        },
    }
}

fn tally_expr(expr: &Expression, cand: &InlineCandidate, t: &mut Tally) {
    match expr {
        Expression::Identifier(id) => {
            if id.name == cand.name {
                t.uses += 1;
            }
        }
        Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::BigIntLiteral(_)
        // A regex literal is an inert leaf (no sub-expression, references no
        // identifier) — these traversals treat it exactly like a StringLiteral.
        | Expression::RegExpLiteral(_)
        // `this` is a leaf keyword — it binds/references no identifier and has
        // no sub-expression, so these traversals do nothing for it. (It is
        // deliberately NOT treated as a freely-substitutable primary: `this` is
        // bound at the call site, so the inliner's triv-/pure-expression
        // predicates leave it conservative.)
        | Expression::ThisExpression(_)
        | Expression::Super(_)
        | Expression::NewTarget(_)
        | Expression::ImportMeta(_)
        | Expression::UndefinedLiteral(_) => {}
        Expression::BinaryExpression(be) => {
            tally_expr(&be.left, cand, t);
            tally_expr(&be.right, cand, t);
        }
        Expression::LogicalExpression(le) => {
            tally_expr(&le.left, cand, t);
            tally_expr(&le.right, cand, t);
        }
        Expression::UnaryExpression(ue) => tally_expr(&ue.argument, cand, t),
        Expression::UpdateExpression(ue) => tally_expr(&ue.argument, cand, t),
        Expression::AssignmentExpression(ae) => {
            match &ae.left {
                AssignmentTarget::Identifier(id) => {
                    if id.name == cand.name {
                        t.uses += 1;
                    }
                }
                AssignmentTarget::MemberExpression(m) => {
                    tally_member(&m.object, &m.property, m.computed, cand, t)
                }
            }
            tally_expr(&ae.right, cand, t);
        }
        Expression::ConditionalExpression(ce) => {
            tally_expr(&ce.test, cand, t);
            tally_expr(&ce.consequent, cand, t);
            tally_expr(&ce.alternate, cand, t);
        }
        Expression::CallExpression(ce) => {
            // A call whose callee is our name, with the right arity and
            // side-effect-free args, is an inlinable call (the expression
            // inliner's gate). A name+arity match regardless of arg
            // simplicity is an `arity_call` (the statement inliner's gate —
            // it materialises non-simple args into temps). Both counts are
            // gathered here. (The callee identifier is also counted as a use
            // when we recurse — so `uses == inlinable`/`arity_calls` exactly
            // when every use is such a call.)
            if is_name_arity_call(ce, cand) {
                t.arity_calls += 1;
                if ce.arguments.iter().all(is_simple_arg) {
                    t.inlinable += 1;
                }
            }
            tally_expr(&ce.callee, cand, t);
            for a in &ce.arguments {
                tally_expr(a, cand, t);
            }
        }
        // A `new X(args)` is a *construction*, not a function call the inliner
        // can substitute — it never adds to `inlinable` / `arity_calls`. Its
        // callee and arguments are still ordinary uses of the candidate, so
        // recurse (mirrors the CallExpression tail without the call gate).
        Expression::NewExpression(ne) => {
            tally_expr(&ne.callee, cand, t);
            for a in &ne.arguments {
                tally_expr(a, cand, t);
            }
        }
        Expression::SequenceExpression(se) => {
            for e in &se.expressions {
                tally_expr(e, cand, t);
            }
        }
        Expression::MemberExpression(m) => {
            tally_member(&m.object, &m.property, m.computed, cand, t)
        }
        // `a?.b` / `a?.[k]` — same as a plain member access.
        Expression::OptionalMemberExpression(m) => {
            tally_member(&m.object, &m.property, m.computed, cand, t)
        }
        // `a?.()` — an optional call is NOT a plain-call inline site (the
        // inliner only substitutes `CallExpression`, and `is_name_arity_call`
        // is typed to `&CallExpression`). Its callee and arguments are still
        // ordinary uses of the candidate, so recurse without the call gate —
        // mirroring the `NewExpression` tail above.
        Expression::OptionalCallExpression(ce) => {
            tally_expr(&ce.callee, cand, t);
            for a in &ce.arguments {
                tally_expr(a, cand, t);
            }
        }
        // A chain expression transparently wraps its optional-chain spine.
        Expression::ChainExpression(c) => tally_expr(&c.expression, cand, t),
        Expression::ArrayExpression(ae) => {
            for el in ae.elements.iter().flatten() {
                tally_expr(el, cand, t);
            }
        }
        Expression::ObjectExpression(oe) => {
            for member in &oe.properties {
                match member {
                    ObjectMember::Property(prop) => {
                        if prop.computed {
                            if let PropertyKey::Expression(e) = &prop.key {
                                tally_expr(e, cand, t);
                            }
                        }
                        tally_expr(&prop.value, cand, t);
                    }
                    // Object spread `...expr` — recurse into the spread
                    // argument the same way the property arm recurses into
                    // prop.value.
                    ObjectMember::Spread(s) => {
                        tally_expr(&s.argument, cand, t);
                    }
                }
            }
        }
        // Count uses of the candidate inside a function *value*'s body —
        // a closure over the candidate is still a use. Mirrors the
        // `FunctionDeclaration` arm in `tally_decl`; over-counting under
        // shadowing only makes the pass decline to inline, never wrong.
        Expression::FunctionExpression(fe) => {
            for s in &fe.body.body {
                tally_stmt(s, cand, t);
            }
        }
        // A use of the candidate inside a class is still a use: the `extends`
        // operand is an ordinary use position, and a closure over the
        // candidate inside a method body counts too — mirror the
        // `FunctionExpression` arm for each method value's body.
        // Over-counting under method-param shadowing only makes the pass
        // decline to inline, never wrong. A method KEY is a property name.
        Expression::ClassExpression(ce) => {
            if let Some(sup) = &ce.super_class {
                tally_expr(sup, cand, t);
            }
            for member in &ce.body {
                match member {
                    ClassMember::Method(m) => {
                        for s in &m.value.body.body {
                            tally_stmt(s, cand, t);
                        }
                    }
                    // SOUNDNESS: a candidate use inside a field initializer (or
                    // a computed key) is a real use — count it, mirroring the
                    // `ClassDeclaration` arm of `tally_decl`.
                    ClassMember::Field(f) => {
                        if let PropertyKey::Expression(e) = &f.key {
                            tally_expr(e, cand, t);
                        }
                        if let Some(v) = &f.value {
                            tally_expr(v, cand, t);
                        }
                    }
                    // SOUNDNESS: candidate uses inside a static-init block's
                    // statements count too — mirror `tally_decl`'s arm.
                    ClassMember::StaticBlock(b) => {
                        for s in &b.body {
                            tally_stmt(s, cand, t);
                        }
                    }
                }
            }
        }
        // A closure over the candidate inside an arrow body is still a
        // use — mirror the function arm.
        Expression::ArrowFunctionExpression(ae) => match &ae.body {
            ArrowBody::Block(b) => {
                for s in &b.body {
                    tally_stmt(s, cand, t);
                }
            }
            ArrowBody::Expression(e) => tally_expr(e, cand, t),
        },
        // A use of the candidate inside a `${…}` insert is still a use.
        // Quasis are leaf strings — only the insert expressions recurse.
        Expression::TemplateLiteral(t2) => {
            for e in &t2.expressions {
                tally_expr(e, cand, t);
            }
        }
        // A use of the candidate can hide in the tag callee or inside a `${…}`
        // insert of the applied template.
        Expression::TaggedTemplateExpression(t2) => {
            tally_expr(&t2.tag, cand, t);
            for e in &t2.quasi.expressions {
                tally_expr(e, cand, t);
            }
        }
        // `...arg` — recurse into the spread argument to tally candidate uses.
        Expression::SpreadElement(s) => tally_expr(&s.argument, cand, t),
        Expression::YieldExpression(y) => { if let Some(a) = &y.argument { tally_expr(a, cand, t); } }
        Expression::AwaitExpression(a) => tally_expr(&a.argument, cand, t),
        Expression::ImportExpression(e) => tally_expr(&e.source, cand, t),
    }
}

fn tally_member(
    object: &Expression,
    property: &Expression,
    computed: bool,
    cand: &InlineCandidate,
    t: &mut Tally,
) {
    tally_expr(object, cand, t);
    if computed {
        tally_expr(property, cand, t);
    }
}

// ---- call substitution ---------------------------------------------------

/// Replace EVERY inlinable call `cand.name(args)` in the program with
/// the substituted callee body. Returns whether any replacement was
/// made. The caller has already verified (via [`tally_program`]) that
/// every use of the name is such a call, so after this the function is
/// unreferenced. Unlike the single-use case there may be several sites,
/// so the walk does not short-circuit — it visits and rewrites them all.
fn inline_all_calls(program: &mut Program, cand: &InlineCandidate) -> bool {
    let mut changed = false;
    for item in &mut program.body {
        changed |= match item {
            ProgramItem::Declaration(d) => inline_in_decl(d, cand),
            ProgramItem::Statement(s) => inline_in_stmt(s, cand),
        };
    }
    changed
}

fn inline_in_decl(decl: &mut Declaration, cand: &InlineCandidate) -> bool {
    let mut changed = false;
    match decl {
        Declaration::VariableDeclaration(vd) => {
            for d in &mut vd.declarations {
                if let Some(init) = &mut d.init {
                    changed |= inline_in_expr(init, cand);
                }
            }
        }
        Declaration::FunctionDeclaration(fd) => {
            for s in &mut fd.body.body {
                changed |= inline_in_stmt(s, cand);
            }
        }
        // Perform the inline substitution inside a class declaration's heritage
        // operand and method bodies, kept in lockstep with `tally_decl` above.
        // Mirrors the `Expression::ClassExpression` arm of `inline_in_expr`.
        // An import declaration has no inlinable body and binds foreign-linked
        // names — leave it untouched.
        Declaration::ImportDeclaration(_) => {}
        Declaration::ExportNamedDeclaration(_) => {}
        Declaration::ExportDefaultDeclaration(_) => {}
        Declaration::ExportAllDeclaration(_) => {}
        Declaration::ClassDeclaration(cd) => {
            if let Some(sup) = &mut cd.super_class {
                changed |= inline_in_expr(sup, cand);
            }
            for member in &mut cd.body {
                match member {
                    ClassMember::Method(m) => {
                        for s in &mut m.value.body.body {
                            changed |= inline_in_stmt(s, cand);
                        }
                    }
                    // Lockstep with `tally_decl`: substitute inside a field's
                    // initializer and computed key, the same use positions that
                    // arm counted.
                    ClassMember::Field(f) => {
                        if let PropertyKey::Expression(e) = &mut f.key {
                            changed |= inline_in_expr(e, cand);
                        }
                        if let Some(v) = &mut f.value {
                            changed |= inline_in_expr(v, cand);
                        }
                    }
                    // Lockstep with `tally`: substitute inside the static-init
                    // block's statements, the same positions that arm counted.
                    ClassMember::StaticBlock(b) => {
                        for s in &mut b.body {
                            changed |= inline_in_stmt(s, cand);
                        }
                    }
                }
            }
        }
    }
    changed
}

fn inline_in_stmt(stmt: &mut Statement, cand: &InlineCandidate) -> bool {
    let mut changed = false;
    match stmt {
        Statement::Declaration(d) => changed |= inline_in_decl(d, cand),
        Statement::Tagged(t) => match t {
            TaggedStatement::ExpressionStatement(es) => {
                changed |= inline_in_expr(&mut es.expression, cand)
            }
            TaggedStatement::BlockStatement(b) => {
                for s in &mut b.body {
                    changed |= inline_in_stmt(s, cand);
                }
            }
            TaggedStatement::IfStatement(is) => {
                changed |= inline_in_expr(&mut is.test, cand);
                changed |= inline_in_stmt(&mut is.consequent, cand);
                if let Some(alt) = &mut is.alternate {
                    changed |= inline_in_stmt(alt, cand);
                }
            }
            TaggedStatement::WhileStatement(ws) => {
                changed |= inline_in_expr(&mut ws.test, cand);
                changed |= inline_in_stmt(&mut ws.body, cand);
            }
            TaggedStatement::WithStatement(ws) => {
                changed |= inline_in_expr(&mut ws.object, cand);
                changed |= inline_in_stmt(&mut ws.body, cand);
            }
            TaggedStatement::DoWhileStatement(ds) => {
                changed |= inline_in_expr(&mut ds.test, cand);
                changed |= inline_in_stmt(&mut ds.body, cand);
            }
            TaggedStatement::ForStatement(fs) => {
                if let Some(init) = &mut fs.init {
                    match init {
                        ForInit::VariableDeclaration(vd) => {
                            for d in &mut vd.declarations {
                                if let Some(i) = &mut d.init {
                                    changed |= inline_in_expr(i, cand);
                                }
                            }
                        }
                        ForInit::Expression(e) => changed |= inline_in_expr(e, cand),
                    }
                }
                if let Some(test) = &mut fs.test {
                    changed |= inline_in_expr(test, cand);
                }
                if let Some(update) = &mut fs.update {
                    changed |= inline_in_expr(update, cand);
                }
                changed |= inline_in_stmt(&mut fs.body, cand);
            }
            TaggedStatement::ForInStatement(fs) => {
                match &mut fs.left {
                    ForInit::VariableDeclaration(vd) => {
                        for d in &mut vd.declarations {
                            if let Some(i) = &mut d.init {
                                changed |= inline_in_expr(i, cand);
                            }
                        }
                    }
                    ForInit::Expression(e) => changed |= inline_in_expr(e, cand),
                }
                changed |= inline_in_expr(&mut fs.right, cand);
                changed |= inline_in_stmt(&mut fs.body, cand);
            }
            TaggedStatement::ForOfStatement(fs) => {
                match &mut fs.left {
                    ForInit::VariableDeclaration(vd) => {
                        for d in &mut vd.declarations {
                            if let Some(i) = &mut d.init {
                                changed |= inline_in_expr(i, cand);
                            }
                        }
                    }
                    ForInit::Expression(e) => changed |= inline_in_expr(e, cand),
                }
                changed |= inline_in_expr(&mut fs.right, cand);
                changed |= inline_in_stmt(&mut fs.body, cand);
            }
            TaggedStatement::ReturnStatement(rs) => {
                if let Some(a) = &mut rs.argument {
                    changed |= inline_in_expr(a, cand);
                }
            }
            TaggedStatement::ThrowStatement(ts) => {
                changed |= inline_in_expr(&mut ts.argument, cand)
            }
            TaggedStatement::LabeledStatement(ls) => changed |= inline_in_stmt(&mut ls.body, cand),
            TaggedStatement::SwitchStatement(ss) => {
                changed |= inline_in_expr(&mut ss.discriminant, cand);
                for c in &mut ss.cases {
                    if let Some(test) = &mut c.test {
                        changed |= inline_in_expr(test, cand);
                    }
                    for s in &mut c.consequent {
                        changed |= inline_in_stmt(s, cand);
                    }
                }
            }
            TaggedStatement::TryStatement(ts) => {
                // Expression-inline call sites inside the three blocks.
                for s in &mut ts.block.body {
                    changed |= inline_in_stmt(s, cand);
                }
                if let Some(h) = &mut ts.handler {
                    for s in &mut h.body.body {
                        changed |= inline_in_stmt(s, cand);
                    }
                }
                if let Some(f) = &mut ts.finalizer {
                    for s in &mut f.body {
                        changed |= inline_in_stmt(s, cand);
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

fn inline_in_expr(expr: &mut Expression, cand: &InlineCandidate) -> bool {
    // If THIS node is an inlinable call, replace it in place. The
    // substituted body contains only this call's own simple arguments
    // (no call to `cand.name`), so there is nothing further to inline
    // inside the replacement — we return without recursing into it. We
    // do NOT short-circuit at the sibling/parent level: a multi-use
    // callee has several call sites and every one must be rewritten.
    if let Expression::CallExpression(ce) = expr {
        if is_inlinable_call(ce, cand) {
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
        // Not our call — recurse into the callee and every argument (the
        // target call might be nested, e.g. `outer(double(7))`).
        let mut changed = inline_in_expr(&mut ce.callee, cand);
        for a in &mut ce.arguments {
            changed |= inline_in_expr(a, cand);
        }
        return changed;
    }

    let mut changed = false;
    match expr {
        Expression::Identifier(_)
        | Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::BigIntLiteral(_)
        // A regex literal is an inert leaf (no sub-expression, references no
        // identifier) — these traversals treat it exactly like a StringLiteral.
        | Expression::RegExpLiteral(_)
        // `this` is a leaf keyword — it binds/references no identifier and has
        // no sub-expression, so these traversals do nothing for it. (It is
        // deliberately NOT treated as a freely-substitutable primary: `this` is
        // bound at the call site, so the inliner's triv-/pure-expression
        // predicates leave it conservative.)
        | Expression::ThisExpression(_)
        | Expression::Super(_)
        | Expression::NewTarget(_)
        | Expression::ImportMeta(_)
        | Expression::UndefinedLiteral(_) => {}
        Expression::BinaryExpression(be) => {
            changed |= inline_in_expr(&mut be.left, cand);
            changed |= inline_in_expr(&mut be.right, cand);
        }
        Expression::LogicalExpression(le) => {
            changed |= inline_in_expr(&mut le.left, cand);
            changed |= inline_in_expr(&mut le.right, cand);
        }
        Expression::UnaryExpression(ue) => changed |= inline_in_expr(&mut ue.argument, cand),
        Expression::UpdateExpression(ue) => changed |= inline_in_expr(&mut ue.argument, cand),
        Expression::AssignmentExpression(ae) => {
            if let AssignmentTarget::MemberExpression(m) = &mut ae.left {
                changed |= inline_in_member(m, cand);
            }
            changed |= inline_in_expr(&mut ae.right, cand);
        }
        Expression::ConditionalExpression(ce) => {
            changed |= inline_in_expr(&mut ce.test, cand);
            changed |= inline_in_expr(&mut ce.consequent, cand);
            changed |= inline_in_expr(&mut ce.alternate, cand);
        }
        // CallExpression handled above.
        Expression::CallExpression(_) => unreachable!("CallExpression handled before this match"),
        // `new X(args)` is not an inlinable function call — recurse into the
        // callee and arguments so a nested inlinable call is still reached.
        Expression::NewExpression(ne) => {
            changed |= inline_in_expr(&mut ne.callee, cand);
            for a in &mut ne.arguments {
                changed |= inline_in_expr(a, cand);
            }
        }
        Expression::SequenceExpression(se) => {
            for e in &mut se.expressions {
                changed |= inline_in_expr(e, cand);
            }
        }
        Expression::MemberExpression(m) => changed |= inline_in_member(m, cand),
        // `a?.b` / `a?.[k]` — recurse into object and (computed) property
        // exactly as `inline_in_member` does for a plain member.
        Expression::OptionalMemberExpression(m) => {
            changed |= inline_in_expr(&mut m.object, cand);
            // Only a computed property `o?.[expr]` is a sub-expression to walk;
            // a non-computed `?.name` is a property name.
            if m.computed {
                changed |= inline_in_expr(&mut m.property, cand);
            }
        }
        // `a?.()` is not a plain-call inline site (only `CallExpression` is
        // substituted, handled before this match) — recurse into callee and
        // arguments so a nested inlinable call is still reached, mirroring
        // `NewExpression`.
        Expression::OptionalCallExpression(ce) => {
            changed |= inline_in_expr(&mut ce.callee, cand);
            for a in &mut ce.arguments {
                changed |= inline_in_expr(a, cand);
            }
        }
        // A chain expression transparently wraps its optional-chain spine.
        Expression::ChainExpression(c) => changed |= inline_in_expr(&mut c.expression, cand),
        Expression::ArrayExpression(ae) => {
            for el in ae.elements.iter_mut().flatten() {
                changed |= inline_in_expr(el, cand);
            }
        }
        Expression::ObjectExpression(oe) => {
            for member in &mut oe.properties {
                match member {
                    ObjectMember::Property(prop) => {
                        // A computed key `[expr]` is a sub-expression to walk; a
                        // plain identifier / string / number key is a property
                        // name.
                        if prop.computed {
                            if let PropertyKey::Expression(e) = &mut prop.key {
                                changed |= inline_in_expr(e, cand);
                            }
                        }
                        changed |= inline_in_expr(&mut prop.value, cand);
                    }
                    // Object spread `...expr` — recurse into the spread
                    // argument the same way the property arm recurses into
                    // prop.value.
                    ObjectMember::Spread(s) => {
                        changed |= inline_in_expr(&mut s.argument, cand);
                    }
                }
            }
        }
        // Inline candidate calls that appear inside a function *value*'s
        // body too, mirroring the `FunctionDeclaration` arm in
        // `inline_in_decl`.
        Expression::FunctionExpression(fe) => {
            for s in &mut fe.body.body {
                changed |= inline_in_stmt(s, cand);
            }
        }
        // Inline candidate calls inside a class too: the `extends` operand is
        // an ordinary sub-expression, and each method body is walked like a
        // `FunctionExpression` body. A method KEY is a property name, not a
        // call site.
        Expression::ClassExpression(ce) => {
            if let Some(sup) = &mut ce.super_class {
                changed |= inline_in_expr(sup, cand);
            }
            for member in &mut ce.body {
                match member {
                    ClassMember::Method(m) => {
                        for s in &mut m.value.body.body {
                            changed |= inline_in_stmt(s, cand);
                        }
                    }
                    // Lockstep with `tally_expr`: substitute inside a field's
                    // initializer and computed key.
                    ClassMember::Field(f) => {
                        if let PropertyKey::Expression(e) = &mut f.key {
                            changed |= inline_in_expr(e, cand);
                        }
                        if let Some(v) = &mut f.value {
                            changed |= inline_in_expr(v, cand);
                        }
                    }
                    // Lockstep with `tally`: substitute inside the static-init
                    // block's statements, the same positions that arm counted.
                    ClassMember::StaticBlock(b) => {
                        for s in &mut b.body {
                            changed |= inline_in_stmt(s, cand);
                        }
                    }
                }
            }
        }
        // Inline candidate calls inside an arrow body too, mirroring the
        // function arm.
        Expression::ArrowFunctionExpression(ae) => match &mut ae.body {
            ArrowBody::Block(b) => {
                for s in &mut b.body {
                    changed |= inline_in_stmt(s, cand);
                }
            }
            ArrowBody::Expression(e) => changed |= inline_in_expr(e, cand),
        },
        // Inline candidate calls inside a `${…}` insert too. Quasis are leaf
        // strings — only the insert expressions recurse.
        Expression::TemplateLiteral(t) => {
            for e in &mut t.expressions {
                changed |= inline_in_expr(e, cand);
            }
        }
        // Inline candidate calls in the tag callee and each `${…}` insert.
        Expression::TaggedTemplateExpression(t) => {
            changed |= inline_in_expr(&mut t.tag, cand);
            for e in &mut t.quasi.expressions {
                changed |= inline_in_expr(e, cand);
            }
        }
        // `...arg` — recurse into the spread argument to inline candidate calls.
        Expression::SpreadElement(s) => changed |= inline_in_expr(&mut s.argument, cand),
        Expression::YieldExpression(y) => { if let Some(a) = &mut y.argument { changed |= inline_in_expr(a, cand); } }
        Expression::AwaitExpression(a) => changed |= inline_in_expr(&mut a.argument, cand),
        Expression::ImportExpression(e) => changed |= inline_in_expr(&mut e.source, cand),
    }
    changed
}

fn inline_in_member(
    m: &mut coding_adventures_javascript_ast::MemberExpression,
    cand: &InlineCandidate,
) -> bool {
    let mut changed = inline_in_expr(&mut m.object, cand);
    // Only a computed property `o[expr]` is a sub-expression to walk;
    // a non-computed `.name` is a property name.
    if m.computed {
        changed |= inline_in_expr(&mut m.property, cand);
    }
    changed
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
        // A regex literal is an inert leaf (no sub-expression, references no
        // identifier) — these traversals treat it exactly like a StringLiteral.
        | Expression::RegExpLiteral(_)
        // `this` is a leaf keyword — it binds/references no identifier and has
        // no sub-expression, so these traversals do nothing for it. (It is
        // deliberately NOT treated as a freely-substitutable primary: `this` is
        // bound at the call site, so the inliner's triv-/pure-expression
        // predicates leave it conservative.)
        | Expression::ThisExpression(_)
        | Expression::Super(_)
        | Expression::NewTarget(_)
        | Expression::ImportMeta(_)
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
        Expression::UpdateExpression(ue) => substitute(&mut ue.argument, map),
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
        Expression::NewExpression(ne) => {
            substitute(&mut ne.callee, map);
            for a in &mut ne.arguments {
                substitute(a, map);
            }
        }
        Expression::SequenceExpression(se) => {
            for e in &mut se.expressions {
                substitute(e, map);
            }
        }
        Expression::MemberExpression(m) => {
            substitute(&mut m.object, map);
            if m.computed {
                substitute(&mut m.property, map);
            }
        }
        // `a?.b` / `a?.[k]` — substitute in object and (computed) property
        // exactly as a plain member access.
        Expression::OptionalMemberExpression(m) => {
            substitute(&mut m.object, map);
            if m.computed {
                substitute(&mut m.property, map);
            }
        }
        // `a?.()` — substitute in callee and each argument, as for a call.
        Expression::OptionalCallExpression(ce) => {
            substitute(&mut ce.callee, map);
            for a in &mut ce.arguments {
                substitute(a, map);
            }
        }
        // A chain expression transparently wraps its optional-chain spine.
        Expression::ChainExpression(c) => substitute(&mut c.expression, map),
        Expression::ArrayExpression(ae) => {
            for el in ae.elements.iter_mut().flatten() {
                substitute(el, map);
            }
        }
        Expression::ObjectExpression(oe) => {
            for member in &mut oe.properties {
                match member {
                    ObjectMember::Property(prop) => {
                        if prop.computed {
                            if let PropertyKey::Expression(e) = &mut prop.key {
                                substitute(e, map);
                            }
                        }
                        substitute(&mut prop.value, map);
                    }
                    // Object spread `...expr` — recurse into the spread
                    // argument the same way the property arm recurses into
                    // prop.value.
                    ObjectMember::Spread(s) => {
                        substitute(&mut s.argument, map);
                    }
                }
            }
        }
        // Substitute param→arg inside a function *value*'s body, but a
        // param or the fn's own name SHADOWS the substituted parameter of
        // the same spelling — remove those keys before recursing so a
        // shadowed reference is left untouched.
        Expression::FunctionExpression(fe) => {
            let mut inner = map.clone();
            if let Some(id) = &fe.id {
                inner.remove(&id.name);
            }
            for p in &fe.params {
                let id = p.binding_identifier();
                inner.remove(&id.name);
            }
            // A default parameter's `right` runs in the nested scope and can
            // reference a substituted outer parameter (`return function(a = b){}`
            // inside `f(a, b)`); substitute through it with the shadow-stripped
            // `inner`, exactly like the body.
            for p in &mut fe.params {
                if let Some(def) = p.default_value_mut() {
                    substitute(def, &inner);
                }
            }
            for s in &mut fe.body.body {
                substitute_in_stmt(s, &inner);
            }
        }
        // Substitute param→arg inside a class *value*. The `extends` operand
        // is evaluated in the ENCLOSING scope, so substitute through it with
        // the outer `map`. Each method body is a nested scope where the
        // class's own name, the method value's own name, and the method
        // params SHADOW a substituted parameter of the same spelling — remove
        // those keys before recursing, exactly as the `FunctionExpression`
        // arm does. A method KEY is a property name, never a substitutable
        // identifier.
        Expression::ClassExpression(ce) => {
            if let Some(sup) = &mut ce.super_class {
                substitute(sup, map);
            }
            let mut class_inner = map.clone();
            if let Some(id) = &ce.id {
                class_inner.remove(&id.name);
            }
            for member in &mut ce.body {
                match member {
                    ClassMember::Method(m) => {
                        let mut inner = class_inner.clone();
                        if let Some(id) = &m.value.id {
                            inner.remove(&id.name);
                        }
                        for p in &m.value.params {
                            let id = p.binding_identifier();
                            inner.remove(&id.name);
                        }
                        // Default-param `right` expressions — see the
                        // `FunctionExpression` arm.
                        for p in &mut m.value.params {
                            if let Some(def) = p.default_value_mut() {
                                substitute(def, &inner);
                            }
                        }
                        for s in &mut m.value.body.body {
                            substitute_in_stmt(s, &inner);
                        }
                    }
                    // A field's initializer runs at construction with the
                    // class's own name in scope (but no method params), so
                    // substitute through it with `class_inner`. The computed
                    // key is likewise an enclosing-scope sub-expression.
                    ClassMember::Field(f) => {
                        if let PropertyKey::Expression(e) = &mut f.key {
                            substitute(e, &class_inner);
                        }
                        if let Some(v) = &mut f.value {
                            substitute(v, &class_inner);
                        }
                    }
                    // A static-init block's statements run at construction with
                    // the class's own name in scope (no method params) —
                    // substitute through them with `class_inner`.
                    ClassMember::StaticBlock(b) => {
                        for s in &mut b.body {
                            substitute_in_stmt(s, &class_inner);
                        }
                    }
                }
            }
        }
        // An arrow's params SHADOW the substituted parameter of the same
        // spelling — remove those keys before recursing so a shadowed
        // reference is left untouched. (Arrows have no self-name.)
        Expression::ArrowFunctionExpression(ae) => {
            let mut inner = map.clone();
            for p in &ae.params {
                let id = p.binding_identifier();
                inner.remove(&id.name);
            }
            // Default-param `right` expressions — see the `FunctionExpression` arm.
            for p in &mut ae.params {
                if let Some(def) = p.default_value_mut() {
                    substitute(def, &inner);
                }
            }
            match &mut ae.body {
                ArrowBody::Block(b) => {
                    for s in &mut b.body {
                        substitute_in_stmt(s, &inner);
                    }
                }
                ArrowBody::Expression(e) => substitute(e, &inner),
            }
        }
        // A template literal binds nothing, so there is no shadowing to strip
        // — substitute straight through each `${…}` insert. Quasis are leaf
        // strings and never contain a substitutable identifier.
        Expression::TemplateLiteral(t) => {
            for e in &mut t.expressions {
                substitute(e, map);
            }
        }
        // A tagged template binds nothing — substitute through the tag callee
        // and each `${…}` insert of the applied template.
        Expression::TaggedTemplateExpression(t) => {
            substitute(&mut t.tag, map);
            for e in &mut t.quasi.expressions {
                substitute(e, map);
            }
        }
        // `...arg` — recurse into the spread argument to substitute through it.
        Expression::SpreadElement(s) => substitute(&mut s.argument, map),
        Expression::YieldExpression(y) => { if let Some(a) = &mut y.argument { substitute(a, map); } }
        Expression::AwaitExpression(a) => substitute(&mut a.argument, map),
        Expression::ImportExpression(e) => substitute(&mut e.source, map),
    }
}

// =========================================================================
// CLOC15 PR-1 — void multi-statement statement-helper inlining
// =========================================================================
//
// The expression inliner above handles only the `{ return EXPR; }` shape:
// a single call *expression* is swapped for a single *expression*. A real
// helper, though, is usually several statements:
//
// ```js
// function track(name, value) {
//   const event = name + ":" + value;   // a local binding
//   metrics.push(event);                 // a free global (`metrics`)
// }
// track("click", 1);                     // result discarded
// ```
//
// Inlining `track("click", 1)` means replacing the ONE call statement with
// the TWO body statements — a 1 → N statement splice. That needs a walker
// that can see the enclosing statement *list*, which the expression
// inliner (threading `&mut Expression`) structurally cannot. This is that
// walker, restricted to the provably-safe first slice from the CLOC15
// spec.
//
// # The slice (every condition is a hard reject — declining is never wrong)
//
//   1. **Single-use, single-declaration.** The helper's name is declared
//      exactly once in the whole program (no shadowing — same guard the
//      expression inliner uses) and used exactly once.
//   2. **The one use is a discarded statement call** — the call is the
//      entire expression of an `ExpressionStatement` (`track(…);`), not a
//      value (`x = track(…)`, `log(track(…))`). A discarded result means
//      there is no return value to *capture* (capturing a used result is
//      PR-3); a discarded TAIL return, however, can be inlined (PR-2).
//   3. **The body is straight-line to an optional tail `return`.** Each
//      statement is an `ExpressionStatement` or a `let` / `const`
//      `VariableDeclaration`, plus an OPTIONAL `return` as the FINAL
//      statement — nothing else (no *early* `return`, no `if`, loops,
//      `var`, nested blocks, …). No mid-body control construct means a
//      flat splice cannot mis-scope control flow; no `var` means no
//      function-scoped hoisting to reason about. Because the call site
//      discards the result, a tail `return E` is normalized by
//      [`normalize_tail_return`]: dropped when `E` is provably inert (a
//      literal or a bare parameter/local read), else kept as `E;` for its
//      side effects.
//   4. **No `this` / `arguments`.** Their meaning is bound by the callee's
//      own call frame; splicing into the caller would silently rebind
//      them. Rejected explicitly.
//   5. **Callee locals are alpha-renamed to program-fresh names** before
//      splicing, so a spliced `let event` can never collide with — or
//      shadow — a binding already live at the call site.
//   6. **Free identifiers must be true globals.** A body identifier that
//      is neither a parameter nor a callee-local (`metrics`, `console`)
//      must be a name that is **never declared as a binding anywhere in
//      the program**. Such a name has no declaration to be shadowed by, so
//      it resolves to the same global at the definition site and at every
//      possible splice site — soundness without a scope analyzer. (This is
//      the conservative bootstrap the spec's Open Question 1 sanctions; a
//      later slice can widen it to "global-and-unshadowed-here" using
//      `closure-scope-analyzer`. It declines, e.g., a helper that calls
//      another top-level function — sound, just not yet capable.)
//   7. **Arguments are side-effect-free** — literals or bare identifiers
//      (the existing [`is_simple_arg`] gate). Then substituting an
//      argument for a parameter used zero, one, or many times neither
//      drops nor duplicates a side effect, and evaluation order is moot.
//   8. **No capture through substitution** — guaranteed by (5) + (7): the
//      locals an argument identifier could be captured by are all fresh.
//
// Everything outside this subset is left untouched. Broader shapes
// (a tail return whose result is *captured* into a hoisted temp, `var`
// locals, `if`, non-simple arguments, multi-use under a budget) are the
// later CLOC15 slices on this same walker.

/// One inlinable void multi-statement helper: its name, parameter names in
/// order, the body statements to splice, and the set of local binding
/// names the body declares (to be alpha-renamed at the splice site).
struct VoidStmtCandidate {
    name: String,
    params: Vec<String>,
    body: Vec<Statement>,
    locals: Vec<String>,
    /// Free identifiers in the body that resolve to a **top-level program
    /// declaration** (a sibling `function`, a top-level `var`/`let`/`const`)
    /// rather than a true global (CLOC16 Slice A). When non-empty, the
    /// candidate may be spliced **only at a direct `program.body` site**,
    /// where the program scope guarantees those names resolve to the same
    /// top-level binding they did in the helper — no intervening scope can
    /// shadow them. At any nested site a local of the same name could capture
    /// the reference, so the splice is declined there. Empty ⇒ no such
    /// obligation (every free ident is a true global or there are none), and
    /// the candidate splices anywhere exactly as before.
    free_top_level: HashSet<String>,
    /// Parameters the body **reassigns** (`x = …`, `x += …`, or nested). Such
    /// a parameter cannot be substituted by its argument expression (you
    /// cannot reassign a literal, and a captured value would read the
    /// pre-assignment argument). CLOC18 instead **materialises** each into a
    /// fresh mutable local seeded from the argument (`let <fresh> = <arg>;`)
    /// and routes the parameter through the rename map. Empty ⇒ all parameters
    /// are pure values, substituted directly as before.
    mutated_params: HashSet<String>,
}

/// Find every qualifying void statement-helper and splice its single call.
/// Returns whether anything changed.
fn inline_void_statement_helpers(
    program: &mut Program,
    decl_counts: &HashMap<String, usize>,
    nodes_touched: &mut u32,
    inlined: &mut Vec<InlineRecord>,
) -> bool {
    // Collect candidates from the top-level function declarations (top
    // level only, mirroring the expression inliner: no enclosing scope to
    // capture, and the free-identifier guard is reasoned against the whole
    // program).
    let top_level_decls = collect_top_level_decl_names(program);
    let mut candidates: Vec<VoidStmtCandidate> = Vec::new();
    for item in &program.body {
        if let ProgramItem::Declaration(Declaration::FunctionDeclaration(fd)) = item {
            if let Some(c) = void_candidate_from_function(fd, decl_counts, &top_level_decls) {
                candidates.push(c);
            }
        }
    }
    if candidates.is_empty() {
        return false;
    }

    // The fresh-name avoidance set: every variable identifier (declaration
    // OR use) anywhere in the program. A callee local renamed to a name
    // outside this set cannot collide with a binding live at the splice
    // site (it is in no declaration) nor shadow a global the body reads (it
    // is in no use) — the property that makes condition 5 sound without a
    // scope resolver. Computed once; splicing introduces only fresh names,
    // so it never goes stale in a way that matters.
    let mut avoid: HashSet<String> = decl_counts.keys().cloned().collect();
    collect_used_idents_program(program, &mut avoid);

    let mut changed = false;
    for cand in &candidates {
        // Gate on the use shape before touching the tree. `uses` counts
        // every binding-use of the name; `inlinable` counts calls with the
        // right arity and side-effect-free arguments (in any position).
        // A valid candidate's body never references its own name (that
        // would be a declared free identifier, rejected at candidate time),
        // so all uses are external. We require exactly one use that is an
        // inlinable call; the splice walker then confirms it sits at
        // statement position (result discarded) and rewrites it.
        let (uses, arity_calls) = name_use_and_arity_calls(program, &cand.name, cand.params.len());
        if uses != 1 || arity_calls != 1 {
            continue;
        }
        if splice_void_call_program(program, cand, &mut avoid, nodes_touched) {
            changed = true;
            // CV: exactly one call site (the `uses == 1 && arity_calls == 1`
            // gate above), spliced in as statements.
            inlined.push(InlineRecord {
                name: cand.name.clone(),
                sites: 1,
            });
        }
    }
    changed
}

/// Collect the names declared at **program scope** — the direct members of
/// `program.body` (CLOC16). These are the only declarations a free identifier
/// can resolve to that are unshadowable at a *top-level* splice site, so a
/// `free_top_level` candidate (Slice A) may reference them.
///
/// We count a top-level `function`, and a top-level `var`/`let`/`const` of any
/// kind. A declaration nested inside a top-level *block* (`{ let x = … }`) is
/// block-scoped, NOT program-scope, so it is intentionally excluded — walking
/// only `program.body`'s direct items gives exactly the program scope. A
/// top-level variable declaration may bridge as either a
/// `ProgramItem::Declaration` or a `ProgramItem::Statement`, so both forms are
/// gathered.
fn collect_top_level_decl_names(program: &Program) -> HashSet<String> {
    let mut out = HashSet::new();
    let add_decl = |d: &Declaration, out: &mut HashSet<String>| match d {
        Declaration::FunctionDeclaration(fd) => {
            out.insert(fd.id.name.clone());
        }
        // A top-level `class C {}` binds `C` in the program scope, exactly like
        // a top-level function name.
        // An import declaration has no inlinable body and binds foreign-linked
        // names — leave it untouched.
        Declaration::ImportDeclaration(_) => {}
        Declaration::ExportNamedDeclaration(_) => {}
        Declaration::ExportDefaultDeclaration(_) => {}
        Declaration::ExportAllDeclaration(_) => {}
        Declaration::ClassDeclaration(cd) => {
            out.insert(cd.id.name.clone());
        }
        Declaration::VariableDeclaration(vd) => {
            for decl in &vd.declarations {
                let BindingTarget::Identifier(id) = &decl.id;
                out.insert(id.name.clone());
            }
        }
    };
    for item in &program.body {
        match item {
            ProgramItem::Declaration(d) => add_decl(d, &mut out),
            ProgramItem::Statement(Statement::Declaration(d)) => add_decl(d, &mut out),
            ProgramItem::Statement(_) => {}
        }
    }
    out
}

/// Collect the subset of `params` that `body` **reassigns** (`x = …`,
/// `x += …`, or nested forms like `y = (x = 5)`, `f(x = 5)`, `c ? (x = 5) :
/// 0`). Walks every expression position. Only `AssignmentTarget::Identifier`
/// counts: a member-target whose *base* is a parameter (`x.k = 5`) mutates a
/// property of the argument, not the parameter binding, and stays substitutable
/// — so it is NOT collected. CLOC18 materialises each collected parameter into
/// a fresh mutable local; the result is empty for the common all-pure-params
/// case (the fast substitution path).
fn collect_mutated_params(body: &[Statement], params: &HashSet<String>) -> HashSet<String> {
    let mut out = HashSet::new();
    for s in body {
        stmt_collect_mutated_params(s, params, &mut out);
    }
    out
}

fn stmt_collect_mutated_params(
    stmt: &Statement,
    params: &HashSet<String>,
    out: &mut HashSet<String>,
) {
    match stmt {
        Statement::Declaration(Declaration::VariableDeclaration(vd)) => {
            for d in &vd.declarations {
                if let Some(e) = &d.init {
                    expr_collect_mutated_params(e, params, out);
                }
            }
        }
        Statement::Declaration(_) => {}
        Statement::Tagged(t) => match t {
            TaggedStatement::ExpressionStatement(es) => {
                expr_collect_mutated_params(&es.expression, params, out)
            }
            TaggedStatement::ReturnStatement(rs) => {
                if let Some(a) = &rs.argument {
                    expr_collect_mutated_params(a, params, out);
                }
            }
            TaggedStatement::IfStatement(is) => {
                expr_collect_mutated_params(&is.test, params, out);
                stmt_collect_mutated_params(&is.consequent, params, out);
                if let Some(a) = &is.alternate {
                    stmt_collect_mutated_params(a, params, out);
                }
            }
            TaggedStatement::BlockStatement(b) => {
                for s in &b.body {
                    stmt_collect_mutated_params(s, params, out);
                }
            }
            TaggedStatement::ThrowStatement(ts) => {
                expr_collect_mutated_params(&ts.argument, params, out)
            }
            // The remaining forms never appear in an admitted candidate body
            // (the shape filter rejects loops/switch/labeled/break/continue
            // before this runs). If the shape filter ever widens, this must
            // widen with it.
            _ => {}
        },
    }
}

fn expr_collect_mutated_params(
    expr: &Expression,
    params: &HashSet<String>,
    out: &mut HashSet<String>,
) {
    match expr {
        Expression::AssignmentExpression(ae) => {
            match &ae.left {
                AssignmentTarget::Identifier(id) if params.contains(&id.name) => {
                    out.insert(id.name.clone());
                }
                AssignmentTarget::MemberExpression(m) => {
                    expr_collect_mutated_params(&m.object, params, out);
                    if m.computed {
                        expr_collect_mutated_params(&m.property, params, out);
                    }
                }
                AssignmentTarget::Identifier(_) => {}
            }
            expr_collect_mutated_params(&ae.right, params, out);
        }
        Expression::BinaryExpression(be) => {
            expr_collect_mutated_params(&be.left, params, out);
            expr_collect_mutated_params(&be.right, params, out);
        }
        Expression::LogicalExpression(le) => {
            expr_collect_mutated_params(&le.left, params, out);
            expr_collect_mutated_params(&le.right, params, out);
        }
        Expression::UnaryExpression(ue) => expr_collect_mutated_params(&ue.argument, params, out),
        Expression::UpdateExpression(ue) => expr_collect_mutated_params(&ue.argument, params, out),
        Expression::ConditionalExpression(ce) => {
            expr_collect_mutated_params(&ce.test, params, out);
            expr_collect_mutated_params(&ce.consequent, params, out);
            expr_collect_mutated_params(&ce.alternate, params, out);
        }
        Expression::CallExpression(ce) => {
            expr_collect_mutated_params(&ce.callee, params, out);
            for a in &ce.arguments {
                expr_collect_mutated_params(a, params, out);
            }
        }
        Expression::NewExpression(ne) => {
            expr_collect_mutated_params(&ne.callee, params, out);
            for a in &ne.arguments {
                expr_collect_mutated_params(a, params, out);
            }
        }
        Expression::SequenceExpression(se) => {
            for e in &se.expressions {
                expr_collect_mutated_params(e, params, out);
            }
        }
        Expression::MemberExpression(m) => {
            expr_collect_mutated_params(&m.object, params, out);
            if m.computed {
                expr_collect_mutated_params(&m.property, params, out);
            }
        }
        // `a?.b` / `a?.[k]` — recurse into object and (computed) property
        // exactly as a plain member access.
        Expression::OptionalMemberExpression(m) => {
            expr_collect_mutated_params(&m.object, params, out);
            if m.computed {
                expr_collect_mutated_params(&m.property, params, out);
            }
        }
        // `a?.()` — recurse into callee and each argument, as for a call.
        Expression::OptionalCallExpression(ce) => {
            expr_collect_mutated_params(&ce.callee, params, out);
            for a in &ce.arguments {
                expr_collect_mutated_params(a, params, out);
            }
        }
        // A chain expression transparently wraps its optional-chain spine.
        Expression::ChainExpression(c) => {
            expr_collect_mutated_params(&c.expression, params, out)
        }
        // Array/object literals: recurse every contained expression. The typed
        // AST has no spread element (`[...e]` / `{...e}` are Phase 2) and no
        // function-expression variant, so a parameter assignment cannot hide in
        // a spread argument or a getter/method body — `prop.value` is always a
        // plain sub-expression, and an unrepresentable form makes the whole
        // program bridge as unsupported (never reaching the inliner). If either
        // becomes representable, add a recursion arm here.
        Expression::ArrayExpression(ae) => {
            for el in ae.elements.iter().flatten() {
                expr_collect_mutated_params(el, params, out);
            }
        }
        Expression::ObjectExpression(oe) => {
            for member in &oe.properties {
                match member {
                    ObjectMember::Property(prop) => {
                        if prop.computed {
                            if let PropertyKey::Expression(e) = &prop.key {
                                expr_collect_mutated_params(e, params, out);
                            }
                        }
                        expr_collect_mutated_params(&prop.value, params, out);
                    }
                    // Object spread `...expr` — recurse into the spread
                    // argument the same way the property arm recurses into
                    // prop.value.
                    ObjectMember::Spread(s) => {
                        expr_collect_mutated_params(&s.argument, params, out);
                    }
                }
            }
        }
        // An assignment to an outer param INSIDE a function value's body
        // (a closure mutating the param) counts as a mutation. Recurse via
        // the statement helper. Over-detection only makes the pass decline
        // to inline (a param treated as mutated is not substituted).
        Expression::FunctionExpression(fe) => {
            for s in &fe.body.body {
                stmt_collect_mutated_params(s, params, out);
            }
        }
        // A closure inside a class can mutate an outer param too: the
        // `extends` operand is an ordinary sub-expression, and each method
        // body is walked like a `FunctionExpression` body. Over-detection
        // only makes the pass decline to inline (a mutated param is not
        // substituted). A method KEY is a property name, never an assignment
        // target.
        Expression::ClassExpression(ce) => {
            if let Some(sup) = &ce.super_class {
                expr_collect_mutated_params(sup, params, out);
            }
            for member in &ce.body {
                match member {
                    ClassMember::Method(m) => {
                        for s in &m.value.body.body {
                            stmt_collect_mutated_params(s, params, out);
                        }
                    }
                    // A field's initializer (or computed key) can mutate an
                    // outer param — recurse. Over-detection only makes the pass
                    // decline to inline, never wrong.
                    ClassMember::Field(f) => {
                        if let PropertyKey::Expression(e) = &f.key {
                            expr_collect_mutated_params(e, params, out);
                        }
                        if let Some(v) = &f.value {
                            expr_collect_mutated_params(v, params, out);
                        }
                    }
                    // A static-init block's statements can mutate an outer param
                    // — recurse. Over-detection only makes the pass decline to
                    // inline, never wrong.
                    ClassMember::StaticBlock(b) => {
                        for s in &b.body {
                            stmt_collect_mutated_params(s, params, out);
                        }
                    }
                }
            }
        }
        // A closure inside an arrow body mutating an outer param counts
        // as a mutation too — recurse. Over-detection only makes the pass
        // decline to inline (a mutated param is not substituted).
        Expression::ArrowFunctionExpression(ae) => match &ae.body {
            ArrowBody::Block(b) => {
                for s in &b.body {
                    stmt_collect_mutated_params(s, params, out);
                }
            }
            ArrowBody::Expression(e) => expr_collect_mutated_params(e, params, out),
        },
        // A `${…}` insert can mutate an outer param — recurse into each.
        // Quasis are leaf strings and never mutate anything.
        Expression::TemplateLiteral(t) => {
            for e in &t.expressions {
                expr_collect_mutated_params(e, params, out);
            }
        }
        // The tag callee or a `${…}` insert can mutate an outer param.
        Expression::TaggedTemplateExpression(t) => {
            expr_collect_mutated_params(&t.tag, params, out);
            for e in &t.quasi.expressions {
                expr_collect_mutated_params(e, params, out);
            }
        }
        // `...arg` — recurse into the spread argument to find mutated params.
        Expression::SpreadElement(s) => expr_collect_mutated_params(&s.argument, params, out),
        Expression::YieldExpression(y) => { if let Some(a) = &y.argument { expr_collect_mutated_params(a, params, out); } }
        Expression::AwaitExpression(a) => expr_collect_mutated_params(&a.argument, params, out),
        Expression::ImportExpression(e) => expr_collect_mutated_params(&e.source, params, out),
        Expression::Identifier(_)
        | Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::BigIntLiteral(_)
        // A regex literal is an inert leaf (no sub-expression, references no
        // identifier) — these traversals treat it exactly like a StringLiteral.
        | Expression::RegExpLiteral(_)
        // `this` is a leaf keyword — it binds/references no identifier and has
        // no sub-expression, so these traversals do nothing for it. (It is
        // deliberately NOT treated as a freely-substitutable primary: `this` is
        // bound at the call site, so the inliner's triv-/pure-expression
        // predicates leave it conservative.)
        | Expression::ThisExpression(_)
        | Expression::Super(_)
        | Expression::NewTarget(_)
        | Expression::ImportMeta(_)
        | Expression::UndefinedLiteral(_) => {}
    }
}

/// Decide whether a top-level function declaration is a void
/// statement-helper candidate. Returns its name, params, body statements,
/// declared local names, and the free identifiers that resolve to top-level
/// declarations (Slice A) when every structural condition holds.
///
/// `top_level_decls` is the program-scope declaration set from
/// [`collect_top_level_decl_names`].
fn void_candidate_from_function(
    fd: &FunctionDeclaration,
    decl_counts: &HashMap<String, usize>,
    top_level_decls: &HashSet<String>,
) -> Option<VoidStmtCandidate> {
    // (3a) Plain function only — generators / async carry resumable state.
    if fd.generator || fd.is_async {
        return None;
    }

    // (1) The name must be declared exactly once in the whole program, so
    // every use of the identifier resolves to this function.
    if decl_counts.get(&fd.id.name).copied().unwrap_or(0) != 1 {
        return None;
    }

    // A default parameter can't be inlined by positional argument binding (an
    // omitted/`undefined` argument triggers the default) — decline, exactly as
    // the return-expression candidate builder does.
    if fd.params.iter().any(|p| p.default_value().is_some()) {
        return None;
    }

    // Parameter names must be distinct (unambiguous substitution map).
    let mut params: Vec<String> = Vec::with_capacity(fd.params.len());
    let mut param_set: HashSet<String> = HashSet::new();
    for p in &fd.params {
        let id = p.binding_identifier();
        if !param_set.insert(id.name.clone()) {
            return None;
        }
        params.push(id.name.clone());
    }

    // (3) Body shape: each statement is an `ExpressionStatement`, a
    // `let`/`const` `VariableDeclaration`, or — CLOC15 PR-2 — an optional
    // `return` as the FINAL statement. No early return, no other control
    // flow, no `var`, no nested blocks. We also collect the local binding
    // names.
    //
    // A tail `return E` is sound here precisely because the call site
    // discards the result (condition 2): the returned value is never read,
    // so the splice can drop it (or keep `E;` for its side effects — see
    // [`normalize_tail_return`]). A `return` anywhere but the last position
    // would change control flow when spliced (the caller's following
    // statements would still run), so it is rejected.
    let mut locals: Vec<String> = Vec::new();
    let mut local_set: HashSet<String> = HashSet::new();
    let last_index = fd.body.body.len().wrapping_sub(1);
    for (i, stmt) in fd.body.body.iter().enumerate() {
        match stmt {
            Statement::Tagged(TaggedStatement::ExpressionStatement(_)) => {}
            Statement::Declaration(Declaration::VariableDeclaration(vd)) => {
                // (3) `var` / `let` / `const` locals are all admitted (CLOC15
                // Open Q3). Each declared name is collected so the splice
                // alpha-renames it to a program-fresh name (condition 5). A
                // `var` is function-scoped and hoists to the top of the
                // *caller's* function on a flat splice, whereas `let`/`const`
                // stay block-scoped — but once the binding is renamed to a name
                // that appears nowhere else in the program (it is not in
                // `avoid`), the hoist is observationally inert: nothing reads or
                // writes the fresh name except the spliced body, in source
                // order. (The bridge desugars `var t = E` into `var t; t = E`,
                // so an admitted `var`-local body contains an assignment to that
                // local — sound under renaming, and not flagged by the
                // parameter-mutation guard below since the target is a local,
                // not a parameter.)
                for d in &vd.declarations {
                    let BindingTarget::Identifier(id) = &d.id;
                    locals.push(id.name.clone());
                    local_set.insert(id.name.clone());
                }
            }
            // A `return` is admitted ONLY in the final (tail) position;
            // anywhere earlier it would alter control flow on splice.
            Statement::Tagged(TaggedStatement::ReturnStatement(_)) if i == last_index => {}
            // CLOC15 PR-4b — an `if` with no early exit. Each branch must be
            // an `ExpressionStatement` or a block of `ExpressionStatement`s
            // (no `return`/`break`/`continue`, which would change control
            // flow on splice; and no nested declarations, which would
            // introduce block-scoped locals the name-based renamer cannot
            // shadow-correctly). So an admitted `if` declares NO locals and
            // contains NO early exit — splicing it is observationally inert.
            Statement::Tagged(TaggedStatement::IfStatement(is)) if is_inlinable_if(is) => {}
            // Anything else (early `return`, a richer `if`, while, for,
            // throw, break, continue, switch, labeled, empty, a nested block,
            // a nested function declaration) is outside this slice.
            _ => return None,
        }
    }

    // Defense in depth: a `let`/`const` local that shares a parameter's
    // spelling is a SyntaxError in conformant JS, so a faithful parser
    // never produces it. But the name-based alpha-renamer is not
    // scope-aware — it would rename *every* occurrence of the shared name,
    // including ones the parameter-substitution step expects to replace
    // with an argument. Rather than rely on the input being well-formed,
    // decline outright when params and locals collide. (Declining is never
    // a miscompile.)
    if params.iter().any(|p| local_set.contains(p)) {
        return None;
    }

    // (4b) Parameters the body REASSIGNS (`x = …`, `x += …`) cannot be
    // substituted by their argument expression — substituting a non-lvalue
    // argument would target a literal, and a captured value would read the
    // pre-assignment argument (the `function f(x){ x = x+1; return x; }` ⇒
    // `g = 7` instead of `8` miscompile). CLOC18 admits them anyway by
    // MATERIALISING each into a fresh mutable local seeded from the argument
    // (`let <fresh> = <arg>;`) and routing the parameter through the rename map
    // — exactly a real call's binding semantics. We record the set here; the
    // materialisation happens in `materialize_args` + the splice builders.
    // (Reachable only since assignment statements parse, CLOC17. Mutation via
    // `++`/`--` is not reachable: the typed AST has no `UpdateExpression`.)
    let mutated_params = collect_mutated_params(&fd.body.body, &param_set);

    // (4) + (6) Walk every body expression's binding-use identifiers. Each
    // must be a parameter, a callee-local, a true global (never declared
    // anywhere), or — CLOC16 Slice A — a top-level program declaration (which
    // imposes the top-level-only splice obligation, recorded in
    // `free_top_level`). `this` / `arguments` are rejected outright.
    let mut used: HashSet<String> = HashSet::new();
    for stmt in &fd.body.body {
        collect_used_idents_stmt(stmt, &mut used);
    }
    let mut free_top_level: HashSet<String> = HashSet::new();
    for name in &used {
        if name == "this" || name == "arguments" {
            return None; // (4) frame-bound meaning would change on splice
        }
        if param_set.contains(name) || local_set.contains(name) {
            continue; // a parameter or a callee-local — handled by splicing
        }
        // (6) a free identifier. Sound cases below; anything else is rejected.
        if top_level_decls.contains(name) {
            // Resolves to a top-level declaration. Two sub-cases by how many
            // times the name is declared **program-wide** (`count_decl_names_*`
            // counts every binding at every depth, so the count is exact):
            if decl_counts.get(name).copied().unwrap_or(0) == 1 {
                // CLOC16 Slice B (uniqueness gate): declared EXACTLY ONCE, and
                // that declaration is the top-level one. No other binding of
                // the name exists anywhere in the program, so NO scope — at
                // any splice site, nested or not — can shadow it. It therefore
                // behaves like a true global for splice-location purposes: no
                // obligation, splices everywhere. (A scope walk would be
                // needed only to admit the multiply-declared case below.)
            } else {
                // CLOC16 Slice A: the name is ALSO declared elsewhere (a local
                // in some other scope could shadow it). Sound to splice ONLY at
                // a direct `program.body` site, where program scope guarantees
                // the same resolution. Record the obligation; the splice walker
                // enforces the top-level-only restriction.
                free_top_level.insert(name.clone());
            }
        } else if decl_counts.get(name).copied().unwrap_or(0) == 0 {
            // A true global — declared nowhere, so unshadowable everywhere.
            // No obligation (unchanged behaviour).
        } else {
            // Declared somewhere, but NOT at program scope — i.e. bound only
            // inside some other function/block. We cannot prove what it
            // resolves to at an arbitrary splice site, so decline (CLOC16
            // proof obligation 4; declining is never a miscompile).
            return None;
        }
    }

    Some(VoidStmtCandidate {
        name: fd.id.name.clone(),
        params,
        body: fd.body.body.clone(),
        locals,
        free_top_level,
        mutated_params,
    })
}

/// CLOC15 PR-4b: is `is` an `if` we can splice into a straight-line body?
/// Admitted only when every branch is "control-flow-inert and
/// declaration-free" — an `ExpressionStatement` or a block of
/// `ExpressionStatement`s. That excludes (a) `return`/`break`/`continue`,
/// which would alter control flow once spliced into the caller, and (b)
/// nested `let`/`const`/`var` declarations, whose block-scoped locals the
/// name-based alpha-renamer could not shadow-correctly. So an admitted `if`
/// introduces no new local and no early exit; splicing it is inert. The
/// test expression is unrestricted — its identifiers are vetted by the
/// normal free-identifier walk.
fn is_inlinable_if(is: &IfStatement) -> bool {
    is_inlinable_if_branch(&is.consequent)
        && is.alternate.as_deref().is_none_or(is_inlinable_if_branch)
}

/// One `if` branch: a bare `ExpressionStatement`, or a `BlockStatement`
/// whose every statement is an `ExpressionStatement`.
fn is_inlinable_if_branch(stmt: &Statement) -> bool {
    match stmt {
        Statement::Tagged(TaggedStatement::ExpressionStatement(_)) => true,
        Statement::Tagged(TaggedStatement::BlockStatement(b)) => b.body.iter().all(|s| {
            matches!(
                s,
                Statement::Tagged(TaggedStatement::ExpressionStatement(_))
            )
        }),
        _ => false,
    }
}

/// Count, for `name`, every binding-use and how many of those are
/// name+arity calls (any arguments — the statement-inlining gate), reusing
/// the shared [`tally_program`]. The probe's `return_expr` is a placeholder
/// — `tally_program` reads only the name and parameter count.
fn name_use_and_arity_calls(program: &Program, name: &str, arity: usize) -> (usize, usize) {
    let probe = InlineCandidate {
        name: name.to_string(),
        params: vec![String::new(); arity],
        return_expr: Expression::NullLiteral(NullLiteral { cv: None }),
    };
    let t = tally_program(program, &probe);
    // The statement-inlining paths gate on name+arity matches (not simple-arg
    // inlinability): PR-4a materialises non-simple arguments into temps.
    (t.uses, t.arity_calls)
}

/// Is `ce` the discardable statement call we splice — `name(args)` with the
/// right arity and side-effect-free arguments?
fn is_void_target_call(ce: &CallExpression, cand: &VoidStmtCandidate) -> bool {
    // Name + arity only: PR-4a materialises non-simple arguments into temps
    // (see [`materialize_args`]), so the simple-arg requirement is lifted for
    // the statement-inlining paths.
    matches!(&*ce.callee, Expression::Identifier(id) if id.name == cand.name)
        && ce.arguments.len() == cand.params.len()
}

/// Walk the program's statement structure and replace the single
/// statement-position call `cand.name(args);` with the spliced body.
/// Returns whether the splice happened (false if the sole call sits in a
/// value position, which this slice declines).
fn splice_void_call_program(
    program: &mut Program,
    cand: &VoidStmtCandidate,
    avoid: &mut HashSet<String>,
    nodes_touched: &mut u32,
) -> bool {
    // The top level is a `Vec<ProgramItem>`. Splicing here means rewriting
    // a `ProgramItem::Statement(ExpressionStatement(call))` into the body
    // statements. We rebuild the vector so a 1 → N expansion is natural.
    //
    // CLOC16 Slice A: a candidate that references a top-level declaration
    // (`!free_top_level.is_empty()`) is sound to splice ONLY at a direct
    // `program.body` site — exactly the `try_splice_statement` branch below.
    // Descending into a nested statement / declaration body would reach a
    // site where a local could shadow the referenced top-level name, so for
    // such candidates we skip the recursion (the call is left intact —
    // declining is never a miscompile). A candidate with no top-level free
    // idents (`free_top_level` empty) recurses everywhere, exactly as before.
    let top_level_only = !cand.free_top_level.is_empty();
    let mut changed = false;
    let mut new_items: Vec<ProgramItem> = Vec::with_capacity(program.body.len());
    for item in std::mem::take(&mut program.body) {
        match item {
            ProgramItem::Statement(stmt) => {
                if let Some(spliced) = try_splice_statement(&stmt, cand, avoid, nodes_touched) {
                    for s in spliced {
                        new_items.push(ProgramItem::Statement(s));
                    }
                    changed = true;
                } else {
                    let mut stmt = stmt;
                    if !top_level_only {
                        changed |= splice_void_in_stmt(&mut stmt, cand, avoid, nodes_touched);
                    }
                    new_items.push(ProgramItem::Statement(stmt));
                }
            }
            ProgramItem::Declaration(mut d) => {
                if !top_level_only {
                    changed |= splice_void_in_decl(&mut d, cand, avoid, nodes_touched);
                }
                new_items.push(ProgramItem::Declaration(d));
            }
        }
    }
    program.body = new_items;
    changed
}

/// If `stmt` is exactly `ExpressionStatement(cand.name(args))`, build and
/// return the spliced replacement statements; otherwise `None`.
fn try_splice_statement(
    stmt: &Statement,
    cand: &VoidStmtCandidate,
    avoid: &mut HashSet<String>,
    nodes_touched: &mut u32,
) -> Option<Vec<Statement>> {
    if let Statement::Tagged(TaggedStatement::ExpressionStatement(es)) = stmt {
        if let Expression::CallExpression(ce) = &es.expression {
            if is_void_target_call(ce, cand) {
                return Some(build_spliced_body(
                    cand,
                    &ce.arguments,
                    avoid,
                    nodes_touched,
                ));
            }
        }
    }
    None
}

/// Recurse into a statement, splicing the target call in any nested
/// statement *list* (block / switch-case) or single-statement *slot*
/// (`if`/loop/labeled body). For a single slot we wrap the spliced
/// statements in a fresh `BlockStatement`: the body is straight-line
/// `let`/`const` + expressions with no control flow, so block-scoping the
/// (already-fresh) locals is observationally inert and keeps an
/// unbraced `if (c) f();` correct.
fn splice_void_in_stmt(
    stmt: &mut Statement,
    cand: &VoidStmtCandidate,
    avoid: &mut HashSet<String>,
    nodes_touched: &mut u32,
) -> bool {
    *nodes_touched += 1;
    match stmt {
        Statement::Declaration(d) => splice_void_in_decl(d, cand, avoid, nodes_touched),
        Statement::Tagged(t) => match t {
            TaggedStatement::BlockStatement(b) => {
                splice_void_in_stmt_vec(&mut b.body, cand, avoid, nodes_touched)
            }
            TaggedStatement::IfStatement(is) => {
                let mut changed =
                    splice_void_in_slot(&mut is.consequent, cand, avoid, nodes_touched);
                if let Some(alt) = &mut is.alternate {
                    changed |= splice_void_in_slot(alt, cand, avoid, nodes_touched);
                }
                changed
            }
            TaggedStatement::WhileStatement(ws) => {
                splice_void_in_slot(&mut ws.body, cand, avoid, nodes_touched)
            }
            TaggedStatement::WithStatement(ws) => {
                splice_void_in_slot(&mut ws.body, cand, avoid, nodes_touched)
            }
            TaggedStatement::DoWhileStatement(ds) => {
                splice_void_in_slot(&mut ds.body, cand, avoid, nodes_touched)
            }
            TaggedStatement::ForStatement(fs) => {
                splice_void_in_slot(&mut fs.body, cand, avoid, nodes_touched)
            }
            TaggedStatement::ForInStatement(fs) => {
                splice_void_in_slot(&mut fs.body, cand, avoid, nodes_touched)
            }
            TaggedStatement::ForOfStatement(fs) => {
                splice_void_in_slot(&mut fs.body, cand, avoid, nodes_touched)
            }
            TaggedStatement::LabeledStatement(ls) => {
                splice_void_in_slot(&mut ls.body, cand, avoid, nodes_touched)
            }
            TaggedStatement::SwitchStatement(ss) => {
                let mut changed = false;
                for c in &mut ss.cases {
                    changed |=
                        splice_void_in_stmt_vec(&mut c.consequent, cand, avoid, nodes_touched);
                }
                changed
            }
            TaggedStatement::TryStatement(ts) => {
                // Splice the void target call in any of the three blocks'
                // statement lists. Fresh-renamed locals avoid the catch param
                // (it is in the `avoid` set via `collect_used_idents`).
                let mut changed =
                    splice_void_in_stmt_vec(&mut ts.block.body, cand, avoid, nodes_touched);
                if let Some(h) = &mut ts.handler {
                    changed |=
                        splice_void_in_stmt_vec(&mut h.body.body, cand, avoid, nodes_touched);
                }
                if let Some(f) = &mut ts.finalizer {
                    changed |= splice_void_in_stmt_vec(&mut f.body, cand, avoid, nodes_touched);
                }
                changed
            }
            // Leaf / expression-only statements hold no nested statement
            // list to splice into. (A target call inside one of these — an
            // `ExpressionStatement` whose expression is NOT the bare call,
            // a `return`/`throw` argument, etc. — is a value position this
            // slice declines.)
            TaggedStatement::ExpressionStatement(_)
            | TaggedStatement::ReturnStatement(_)
            | TaggedStatement::ThrowStatement(_)
            | TaggedStatement::BreakStatement(_)
            | TaggedStatement::ContinueStatement(_)
            | TaggedStatement::EmptyStatement(_)
            | TaggedStatement::DebuggerStatement(_) => false,
        },
    }
}

/// Splice within a `Vec<Statement>` (block body, switch case): rebuild the
/// list, expanding a matched call statement into the body statements.
fn splice_void_in_stmt_vec(
    list: &mut Vec<Statement>,
    cand: &VoidStmtCandidate,
    avoid: &mut HashSet<String>,
    nodes_touched: &mut u32,
) -> bool {
    let mut changed = false;
    let mut out: Vec<Statement> = Vec::with_capacity(list.len());
    for stmt in std::mem::take(list) {
        if let Some(spliced) = try_splice_statement(&stmt, cand, avoid, nodes_touched) {
            out.extend(spliced);
            changed = true;
        } else {
            let mut stmt = stmt;
            changed |= splice_void_in_stmt(&mut stmt, cand, avoid, nodes_touched);
            out.push(stmt);
        }
    }
    *list = out;
    changed
}

/// Splice into a single-statement *slot* (the body of an `if`/loop/labeled
/// statement). If the slot itself is the matched call, replace it with a
/// `BlockStatement` holding the spliced statements; otherwise recurse.
fn splice_void_in_slot(
    slot: &mut Statement,
    cand: &VoidStmtCandidate,
    avoid: &mut HashSet<String>,
    nodes_touched: &mut u32,
) -> bool {
    if let Some(spliced) = try_splice_statement(slot, cand, avoid, nodes_touched) {
        *slot = Statement::Tagged(TaggedStatement::BlockStatement(BlockStatement {
            cv: None,
            body: spliced,
        }));
        true
    } else {
        splice_void_in_stmt(slot, cand, avoid, nodes_touched)
    }
}

fn splice_void_in_decl(
    decl: &mut Declaration,
    cand: &VoidStmtCandidate,
    avoid: &mut HashSet<String>,
    nodes_touched: &mut u32,
) -> bool {
    match decl {
        // A function body is a `Vec<Statement>` the call may live in.
        Declaration::FunctionDeclaration(fd) => {
            splice_void_in_stmt_vec(&mut fd.body.body, cand, avoid, nodes_touched)
        }
        // Each class method body is a `Vec<Statement>` a void call may live in
        // — splice into every method, mirroring the function-body arm.
        // An import declaration has no inlinable body and binds foreign-linked
        // names — leave it untouched.
        Declaration::ImportDeclaration(_) => false,
        Declaration::ExportNamedDeclaration(_) => false,
        Declaration::ExportDefaultDeclaration(_) => false,
        Declaration::ExportAllDeclaration(_) => false,
        Declaration::ClassDeclaration(cd) => {
            let mut changed = false;
            for member in &mut cd.body {
                match member {
                    // A method body is a `Vec<Statement>` a void call may live in.
                    ClassMember::Method(m) => {
                        changed |= splice_void_in_stmt_vec(
                            &mut m.value.body.body,
                            cand,
                            avoid,
                            nodes_touched,
                        );
                    }
                    // A field's initializer is an expression (a value position),
                    // so a void statement cannot be spliced there.
                    ClassMember::Field(_) => {}
                    // A static-init block's body IS a `Vec<Statement>` — splice
                    // into it like a method body.
                    ClassMember::StaticBlock(b) => {
                        changed |=
                            splice_void_in_stmt_vec(&mut b.body, cand, avoid, nodes_touched);
                    }
                }
            }
            changed
        }
        // Variable initializers are expressions — a call there is a value
        // position, declined by this slice.
        Declaration::VariableDeclaration(_) => false,
    }
}

/// Build the statement list to splice in for one call site: a clone of the
/// body with (0) the tail `return` normalized (the result is discarded),
/// (1) arguments materialised (non-simple ones hoisted into temps via
/// [`materialize_args`], CLOC15 PR-4a), then (a) callee locals alpha-renamed
/// to program-fresh names, then (b) parameters substituted by their argument
/// (or its temp). The argument-temp prelude is prepended so the arguments
/// evaluate left-to-right, once each, before the body. Newly minted fresh
/// names are added to `avoid` so a second splice cannot reuse them.
fn build_spliced_body(
    cand: &VoidStmtCandidate,
    args: &[Expression],
    avoid: &mut HashSet<String>,
    nodes_touched: &mut u32,
) -> Vec<Statement> {
    let mut body = cand.body.clone();

    // (0) Normalize a tail `return E` for the discarded-result call site.
    // Done first, while the names are still the callee's own, so the
    // membership test in [`is_drop_safe_return`] sees the original param /
    // local names; the resulting `E;` (if kept) is then renamed and
    // substituted by the steps below like any other expression statement.
    normalize_tail_return(&mut body, cand);

    // (1) Materialise arguments. With all-simple args this is the previous
    // direct substitution (no prelude). With any non-simple arg, every arg
    // is hoisted into a fresh `const` temp (in source order) and the param
    // map sends each parameter to its temp — so a non-simple argument is
    // evaluated exactly once regardless of how often its parameter is used.
    // Done BEFORE local renaming so the arg temps occupy `avoid` first.
    let (prelude, param_map, mutated_rename) = materialize_args(cand, args, avoid, nodes_touched);

    // (a) Alpha-rename callee locals → fresh, AND route each materialised
    // mutated parameter (CLOC18) through the same rename so both its reads and
    // its assignment targets become the fresh `let` temp (the target-aware
    // `rename` walk does this; `substitute` deliberately does not). Renaming a
    // binding and every in-body use of it makes a spliced `let event`
    // collision-proof against the call-site scope (and against the temps
    // already in `avoid`).
    let mut rename: HashMap<String, String> = mutated_rename;
    if !cand.locals.is_empty() {
        let mut gen = FreshNames::new();
        for local in &cand.locals {
            let fresh = gen.next(avoid);
            avoid.insert(fresh.clone());
            rename.insert(local.clone(), fresh);
        }
    }
    if !rename.is_empty() {
        for stmt in &mut body {
            rename_in_stmt(stmt, &rename);
        }
    }

    // (b) Substitute pure parameters → arguments (or their temps). Because the
    // locals are now fresh, an identifier argument can never be captured by
    // a callee local.
    if !param_map.is_empty() {
        for stmt in &mut body {
            substitute_in_stmt(stmt, &param_map);
        }
    }

    *nodes_touched += body.len() as u32;

    // The arg-temp prelude (if any) runs before the body.
    let mut out = prelude;
    out.extend(body);
    out
}

/// Materialise a call's arguments for splicing.
///
/// - **All arguments simple** (literal / bare identifier): the previous
///   behaviour — no prelude, and the param map substitutes each parameter
///   directly by its argument. A simple argument has no side effect and its
///   value cannot change before the body runs, so duplicating it across N
///   parameter uses is sound; keeping this path byte-for-byte preserves the
///   existing single-pass output (no fixture churn).
/// - **Any argument non-simple**: hoist EVERY argument into a fresh `const`
///   temp, in source order, and map each parameter to its temp identifier.
///   This evaluates all arguments left-to-right exactly once before the body
///   (JS call semantics) and captures their values, so a parameter used many
///   times reads the captured value rather than re-evaluating the argument.
///   The redundant temps on the simple arguments are removed downstream by
///   `inline-variables` + `constant-fold`.
///
/// Temps are program-fresh (minted from `avoid`, each added as minted). The
/// caller mints these BEFORE any callee-local fresh names, so the two name
/// spaces are disjoint.
fn materialize_args(
    cand: &VoidStmtCandidate,
    args: &[Expression],
    avoid: &mut HashSet<String>,
    nodes_touched: &mut u32,
) -> (
    Vec<Statement>,
    HashMap<String, Expression>,
    HashMap<String, String>,
) {
    // Fast path: every argument is simple AND no parameter is reassigned. Then
    // each parameter substitutes directly by its argument — the previous
    // behaviour, byte-for-byte (no prelude, no rename). A mutated parameter
    // disqualifies this path even with a simple argument: you cannot reassign a
    // substituted literal or a caller-scope identifier (CLOC18).
    if cand.mutated_params.is_empty() && args.iter().all(is_simple_arg) {
        let map: HashMap<String, Expression> = cand
            .params
            .iter()
            .cloned()
            .zip(args.iter().cloned())
            .collect();
        return (Vec::new(), map, HashMap::new());
    }

    // Otherwise materialise EVERY argument into a fresh temp, in source order,
    // so the arguments evaluate left-to-right exactly once before the body.
    // - A **mutated** parameter (CLOC18) becomes a `let <fresh> = <arg>;`
    //   (reassignable) and is routed through `mutated_rename` — the
    //   target-aware `rename` walk rewrites both its reads and its assignment
    //   targets, which `substitute` deliberately does not.
    // - A **pure** parameter becomes a `const <fresh> = <arg>;` and substitutes
    //   by its temp identifier, as before.
    let mut prelude: Vec<Statement> = Vec::with_capacity(args.len());
    let mut subst_map: HashMap<String, Expression> = HashMap::new();
    let mut mutated_rename: HashMap<String, String> = HashMap::new();
    let mut gen = FreshNames::new();
    for (param, arg) in cand.params.iter().zip(args.iter()) {
        let temp = gen.next(avoid);
        avoid.insert(temp.clone());
        let is_mutated = cand.mutated_params.contains(param);
        prelude.push(Statement::Declaration(Declaration::VariableDeclaration(
            VariableDeclaration {
                cv: None,
                // `let` for a mutated param (it gets reassigned in the body),
                // `const` for a pure one.
                kind: if is_mutated {
                    VarKind::Let
                } else {
                    VarKind::Const
                },
                declarations: vec![VariableDeclarator {
                    cv: None,
                    id: BindingTarget::Identifier(Identifier {
                        cv: None,
                        name: temp.clone(),
                    }),
                    init: Some(arg.clone()),
                }],
            },
        )));
        if is_mutated {
            mutated_rename.insert(param.clone(), temp);
        } else {
            subst_map.insert(
                param.clone(),
                Expression::Identifier(Identifier {
                    cv: None,
                    name: temp,
                }),
            );
        }
    }
    *nodes_touched += prelude.len() as u32;
    (prelude, subst_map, mutated_rename)
}

/// Normalize a tail `return` for a discarded-result splice (CLOC15 PR-2).
/// The call site discards the value, so the returned expression is never
/// read:
///
/// - `return;` (no argument) → dropped entirely (a no-op).
/// - `return E;` where `E` is **provably inert** (a literal, or a bare read
///   of a parameter / callee-local — a binding that always exists, so the
///   read neither throws nor has a side effect) → dropped.
/// - `return E;` otherwise → rewritten to `E;` (an `ExpressionStatement`),
///   so `E` is still evaluated for its side effects and the value is
///   discarded — exactly what the original function did before returning.
///
/// A bare *global* identifier is deliberately NOT dropped: reading an
/// undeclared global throws `ReferenceError`, which we must preserve.
fn normalize_tail_return(body: &mut Vec<Statement>, cand: &VoidStmtCandidate) {
    let is_tail_return = matches!(
        body.last(),
        Some(Statement::Tagged(TaggedStatement::ReturnStatement(_)))
    );
    if !is_tail_return {
        return;
    }
    // Pop the tail return; conditionally push back its argument as a plain
    // expression statement (kept for side effects) unless it is droppable.
    let Some(Statement::Tagged(TaggedStatement::ReturnStatement(rs))) = body.pop() else {
        return;
    };
    match rs.argument {
        None => {}                                     // bare return: drop
        Some(e) if is_drop_safe_return(&e, cand) => {} // inert value: drop
        Some(e) => body.push(Statement::Tagged(TaggedStatement::ExpressionStatement(
            ExpressionStatement {
                cv: None,
                expression: e,
            },
        ))),
    }
}

/// Is the discarded return value `e` provably inert — safe to drop rather
/// than keep as `E;`? True for a literal (no side effect, never throws) or
/// a bare read of a parameter / callee-local (the binding always exists, so
/// the read neither throws a `ReferenceError` nor has a side effect). A free
/// global identifier is NOT inert (an undeclared read throws), nor is any
/// member access / call / operator (possible getter / throw / side effect).
fn is_drop_safe_return(e: &Expression, cand: &VoidStmtCandidate) -> bool {
    match e {
        Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::BigIntLiteral(_)
        | Expression::UndefinedLiteral(_) => true,
        Expression::Identifier(id) => {
            cand.params.contains(&id.name) || cand.locals.contains(&id.name)
        }
        _ => false,
    }
}

// =========================================================================
// CLOC15 PR-3 — result-used helpers, captured into a hoisted temp
// =========================================================================
//
// PR-1/PR-2 inline a helper only when its result is DISCARDED (the call is a
// statement). When the result is USED, we cannot swap statements in place —
// there is a value to thread back. The transform:
//
// ```js
// function compute(a) { const t = a + 1; return t * 2; }
// var x = compute(5);
// // ⇒
// const u = 5 + 1;     // body, locals alpha-renamed, params substituted
// const v = u * 2;     // the tail-return value captured into a fresh temp
// var x = v;           // the call replaced by the temp
// ```
//
// i.e. the body is HOISTED to before the enclosing statement and the tail
// `return E` becomes `const <temp> = E;`, then the call expression is
// replaced by `<temp>`.
//
// # The soundness crux: hoisting must not reorder evaluation
//
// Hoisting the body to *before* the enclosing statement runs it before
// anything else that statement evaluates. That is sound only when nothing
// in the enclosing statement is evaluated before the call. The airtight
// subset this slice admits — the call is the ENTIRE initializer of a
// SINGLE-declarator `var`/`let`/`const`:
//
//   `var x = compute(5);`        ← admitted (the call is the whole init)
//   `var x = a + compute(5);`    ← declined (`a` is evaluated first)
//   `var x = f(), y = compute(5);`← declined (multi-declarator order)
//   `x = compute(5);`            ← declined (assignment target; later slice)
//   `return compute(5);`         ← declined (later slice)
//   `a && compute(5)`            ← declined (conditional / short-circuit)
//   `for (var x = compute(5);;)` ← declined (not a statement-list element)
//
// In the admitted shape the declaration's only job is to bind its name to
// the init's value; there is no sibling sub-expression whose evaluation a
// hoist could reorder, and a `var`/`let`/`const` initializer is always
// evaluated (never short-circuited). All of PR-1/PR-2's guards still apply
// (single-use, no `this`/`arguments`, locals alpha-renamed, free idents are
// true globals, simple args); additionally the body MUST end in a tail
// `return E` (the value to capture). Broader value positions (assignment
// targets, `return` arguments, and reordering-safe operand positions) are
// later slices on this same machinery.

/// Find every result-used helper whose single call is a capturable
/// single-declarator initializer, and inline it by hoisting its body and
/// capturing the tail-return value into a fresh temp. Returns whether
/// anything changed.
fn inline_valued_statement_helpers(
    program: &mut Program,
    decl_counts: &HashMap<String, usize>,
    nodes_touched: &mut u32,
    inlined: &mut Vec<InlineRecord>,
) -> bool {
    // Candidates share PR-1/PR-2's body shape (straight-line + optional tail
    // return), but here the tail `return E` MUST be present with an argument
    // — that is the value we capture.
    let top_level_decls = collect_top_level_decl_names(program);
    let mut candidates: Vec<VoidStmtCandidate> = Vec::new();
    for item in &program.body {
        if let ProgramItem::Declaration(Declaration::FunctionDeclaration(fd)) = item {
            if let Some(c) = void_candidate_from_function(fd, decl_counts, &top_level_decls) {
                if candidate_has_tail_return_value(&c) {
                    candidates.push(c);
                }
            }
        }
    }
    if candidates.is_empty() {
        return false;
    }

    let mut avoid: HashSet<String> = decl_counts.keys().cloned().collect();
    collect_used_idents_program(program, &mut avoid);

    let mut changed = false;
    for cand in &candidates {
        let (uses, arity_calls) = name_use_and_arity_calls(program, &cand.name, cand.params.len());
        if uses != 1 || arity_calls != 1 {
            continue;
        }
        if splice_valued_call_program(program, cand, &mut avoid, nodes_touched) {
            changed = true;
            // CV: exactly one call site (gate above), value-captured.
            inlined.push(InlineRecord {
                name: cand.name.clone(),
                sites: 1,
            });
        }
    }
    changed
}

/// Does the candidate body end in a `return E` with an argument? (The value
/// PR-3 captures.) A void body — no return, or a bare `return;` — is not a
/// PR-3 candidate; it is PR-1/PR-2's job.
fn candidate_has_tail_return_value(cand: &VoidStmtCandidate) -> bool {
    matches!(
        cand.body.last(),
        Some(Statement::Tagged(TaggedStatement::ReturnStatement(rs))) if rs.argument.is_some()
    )
}

/// Walk the program's statement structure and rewrite the single
/// capturable initializer call. Handles top-level items (a top-level
/// variable declaration may bridge as either a `ProgramItem::Declaration`
/// or a `ProgramItem::Statement`) and recurses into nested statement lists.
fn splice_valued_call_program(
    program: &mut Program,
    cand: &VoidStmtCandidate,
    avoid: &mut HashSet<String>,
    nodes_touched: &mut u32,
) -> bool {
    // CLOC16 Slice A (same rule as the void path): a `free_top_level`
    // candidate may be captured ONLY at a direct `program.body` site — the
    // `try_capture_in_stmt` / top-level `capture_splice_for_vardecl` branches
    // below (a top-level `const r = f(x);`). A `return f(x)` is never valid at
    // program scope, so for these candidates the return-capture path is simply
    // unreachable here. Descending into nested statement / function bodies
    // would reach a site where a local could shadow a referenced top-level
    // name, so the recursion is skipped for them (the call is left intact).
    let top_level_only = !cand.free_top_level.is_empty();
    let mut changed = false;
    let mut new_items: Vec<ProgramItem> = Vec::with_capacity(program.body.len());
    for item in std::mem::take(&mut program.body) {
        match item {
            ProgramItem::Statement(stmt) => {
                if let Some(spliced) = try_capture_in_stmt(&stmt, cand, avoid, nodes_touched) {
                    for s in spliced {
                        new_items.push(ProgramItem::Statement(s));
                    }
                    changed = true;
                } else {
                    let mut stmt = stmt;
                    if !top_level_only {
                        changed |= splice_valued_in_stmt(&mut stmt, cand, avoid, nodes_touched);
                    }
                    new_items.push(ProgramItem::Statement(stmt));
                }
            }
            ProgramItem::Declaration(d) => {
                if let Declaration::VariableDeclaration(vd) = &d {
                    if let Some(spliced) =
                        capture_splice_for_vardecl(vd, cand, avoid, nodes_touched)
                    {
                        for s in spliced {
                            new_items.push(ProgramItem::Statement(s));
                        }
                        changed = true;
                        continue;
                    }
                }
                let mut d = d;
                if !top_level_only {
                    changed |= splice_valued_in_decl(&mut d, cand, avoid, nodes_touched);
                }
                new_items.push(ProgramItem::Declaration(d));
            }
        }
    }
    program.body = new_items;
    changed
}

/// If `stmt` is a capturable single-declarator variable declaration whose
/// initializer is exactly the target call, return its hoisted-and-captured
/// replacement statements; else `None`.
fn try_capture_in_stmt(
    stmt: &Statement,
    cand: &VoidStmtCandidate,
    avoid: &mut HashSet<String>,
    nodes_touched: &mut u32,
) -> Option<Vec<Statement>> {
    match stmt {
        // `const r = f(x);` / `let r = f(x);` — PR-3 value capture into the
        // declared binding via a hoisted temp.
        Statement::Declaration(Declaration::VariableDeclaration(vd)) => {
            capture_splice_for_vardecl(vd, cand, avoid, nodes_touched)
        }
        // `return f(x);` — PR-5 value capture in return-argument position.
        Statement::Tagged(TaggedStatement::ReturnStatement(rs)) => {
            capture_splice_for_return(rs, cand, avoid, nodes_touched)
        }
        // `g = f(x);` — PR-6 value capture in assignment-target position
        // (CLOC15 Open Question 2's last case). Reachable only now that
        // assignment-expression statements parse (CLOC17).
        Statement::Tagged(TaggedStatement::ExpressionStatement(es)) => {
            capture_splice_for_assignment(es, cand, avoid, nodes_touched)
        }
        _ => None,
    }
}

/// The return-position capture core (CLOC15 PR-5): a `return` statement
/// qualifies iff its argument is **exactly** `cand.name(args)` (matching
/// arity, the call is the entire argument — not `return a + f(x)` nor
/// `return c ? f(x) : y`, which are not `CallExpression`s and so are
/// declined). The replacement is the hoisted body with the callee's tail
/// `return E` re-emitted as *this* function's `return E` — no temp, because
/// the value flows straight out.
///
/// Soundness: `return` is a terminator, so the single `return f(x)` statement
/// is the last reachable statement on its path; replacing it with
/// `body…; return E` runs the body's effects (exactly as they ran inside the
/// callee before its own return) and then returns the same value. Anything
/// textually after `return f(x)` was dead before and remains dead after.
fn capture_splice_for_return(
    rs: &ReturnStatement,
    cand: &VoidStmtCandidate,
    avoid: &mut HashSet<String>,
    nodes_touched: &mut u32,
) -> Option<Vec<Statement>> {
    let arg = rs.argument.as_ref()?;
    let Expression::CallExpression(ce) = arg else {
        return None; // the call must be the ENTIRE return argument
    };
    if !is_void_target_call(ce, cand) {
        return None;
    }
    Some(build_captured_body(
        cand,
        &ce.arguments,
        CaptureTail::AsReturn,
        avoid,
        nodes_touched,
    ))
}

/// The assignment-target capture core (CLOC15 PR-6 / Open Question 2): an
/// expression statement qualifies iff it is a **simple** assignment
/// (`g = …`, operator `=`) to a **bare identifier** whose right-hand side is
/// **exactly** `cand.name(args)` (the call is the entire RHS). The replacement
/// is the hoisted body with the callee's tail `return E` re-emitted as
/// `g = E;` — no temp, because the value flows straight into the assignment
/// target.
///
/// Soundness — why only `=` to a bare identifier:
/// - `g = f(x)` evaluates the LHS *reference* to `g` (trivial, no side effects
///   for a bare identifier), then evaluates `f(x)` (the body), then assigns.
///   Splicing `body…; g = E` runs the body's effects and assigns `E` to `g` in
///   exactly that order — observationally identical. The assignment
///   expression's *result value* is discarded (it is an expression statement),
///   so nothing downstream depends on it.
/// - **Compound** assignment (`g += f(x)`) is declined: `g += f(x)` reads the
///   OLD `g` *before* `f(x)` runs, but `body…; g += E` would read `g` *after*
///   the body — if the body mutates `g`, the two differ. Reordering would
///   miscompile.
/// - **Member** targets (`obj.k = f(x)`) are declined: the reference to
///   `obj.k`'s base (`obj`) is evaluated *before* `f(x)`; hoisting the body
///   ahead of it could reorder observable effects (a getter on `obj`, or the
///   body mutating `obj`).
fn capture_splice_for_assignment(
    es: &ExpressionStatement,
    cand: &VoidStmtCandidate,
    avoid: &mut HashSet<String>,
    nodes_touched: &mut u32,
) -> Option<Vec<Statement>> {
    let Expression::AssignmentExpression(ae) = &es.expression else {
        return None;
    };
    if ae.operator != AssignmentOperator::Eq {
        return None; // compound assignment reads the target before the call
    }
    let AssignmentTarget::Identifier(target) = &ae.left else {
        return None; // member targets evaluate their base before the call
    };
    let Expression::CallExpression(ce) = ae.right.as_ref() else {
        return None; // the call must be the ENTIRE right-hand side
    };
    if !is_void_target_call(ce, cand) {
        return None;
    }
    Some(build_captured_body(
        cand,
        &ce.arguments,
        CaptureTail::IntoAssignment(&target.name),
        avoid,
        nodes_touched,
    ))
}

/// The capture core: a `VariableDeclaration` qualifies iff it has EXACTLY
/// one declarator whose initializer is exactly `cand.name(args)` (matching
/// arity, side-effect-free args). The replacement is the hoisted body (with
/// the tail return captured into a fresh temp) followed by the original
/// declaration with its initializer rewritten to that temp.
fn capture_splice_for_vardecl(
    vd: &VariableDeclaration,
    cand: &VoidStmtCandidate,
    avoid: &mut HashSet<String>,
    nodes_touched: &mut u32,
) -> Option<Vec<Statement>> {
    if vd.declarations.len() != 1 {
        return None; // multi-declarator: later declarators' order would shift
    }
    let init = vd.declarations[0].init.as_ref()?;
    let Expression::CallExpression(ce) = init else {
        return None; // the call must be the ENTIRE initializer
    };
    if !is_void_target_call(ce, cand) {
        return None;
    }

    // Mint the capture temp first (added to `avoid`), so the callee-local
    // fresh names minted inside `build_captured_body` cannot collide with it.
    let temp = {
        let mut gen = FreshNames::new();
        let t = gen.next(avoid);
        avoid.insert(t.clone());
        t
    };

    let mut out = build_captured_body(
        cand,
        &ce.arguments,
        CaptureTail::IntoTemp(&temp),
        avoid,
        nodes_touched,
    );

    // The original declaration, initializer rewritten to the temp.
    let mut new_vd = vd.clone();
    new_vd.declarations[0].init = Some(Expression::Identifier(Identifier {
        cv: None,
        name: temp,
    }));
    out.push(Statement::Declaration(Declaration::VariableDeclaration(
        new_vd,
    )));
    Some(out)
}

/// How a captured tail-return value is consumed at the splice site. The
/// rename/substitute/arg-materialisation work is identical; only the final
/// statement differs, so the two value-capture paths share
/// [`build_captured_body`] and vary only this tail.
enum CaptureTail<'a> {
    /// Bind the value into a fresh `const <temp>;`. Used when the call was a
    /// variable initializer (`const r = f(x)`) or expression-statement result:
    /// a later statement reads the value through `<temp>` (CLOC15 PR-3).
    IntoTemp(&'a str),
    /// The call was itself the argument of a `return` statement
    /// (`return f(x)`). Emit `return <value>;` directly — no temp is needed
    /// because the value flows straight out as the enclosing function's return
    /// value. Sound because `return` is a terminator: nothing in the caller's
    /// statement list runs after it, so splicing `body; return E` in place of
    /// `return f(x)` preserves both the effects and the returned value
    /// (CLOC15 PR-5).
    AsReturn,
    /// The call was the entire right-hand side of a simple assignment to a
    /// bare identifier (`g = f(x)`). Emit `g = <value>;` directly — no temp is
    /// needed because the value flows straight into the assignment target. The
    /// caller (`capture_splice_for_assignment`) has already verified the
    /// operator is `=` and the target is a bare identifier, the two conditions
    /// that make hoisting the body ahead of the assignment observationally
    /// inert (CLOC15 PR-6).
    IntoAssignment(&'a str),
}

/// Build the hoisted body for a captured call: the callee body with its
/// tail `return E` removed, locals alpha-renamed, params substituted, and a
/// trailing statement consuming the (renamed/substituted) return value per
/// `tail` — either `const <temp> = E;` ([`CaptureTail::IntoTemp`]) or
/// `return E;` ([`CaptureTail::AsReturn`]).
fn build_captured_body(
    cand: &VoidStmtCandidate,
    args: &[Expression],
    tail: CaptureTail<'_>,
    avoid: &mut HashSet<String>,
    nodes_touched: &mut u32,
) -> Vec<Statement> {
    let mut body = cand.body.clone();

    // Pop the tail `return E` (guaranteed present by the candidate filter).
    let mut return_value: Expression = match body.pop() {
        Some(Statement::Tagged(TaggedStatement::ReturnStatement(rs))) => rs
            .argument
            .expect("PR-3 candidate has a tail return with an argument"),
        Some(other) => {
            body.push(other);
            unreachable!("PR-3 candidate's last statement is a return with an argument")
        }
        None => unreachable!("PR-3 candidate body is non-empty"),
    };

    // (1) Materialise arguments (CLOC15 PR-4a): non-simple args hoisted into
    // fresh temps before the body, in source order; the param map sends each
    // parameter to its argument (or temp). Minted BEFORE local renaming so
    // the arg temps occupy `avoid` first. The capture temp (`temp`) was
    // already minted by the caller and is in `avoid`, so it cannot collide.
    let (prelude, param_map, mutated_rename) = materialize_args(cand, args, avoid, nodes_touched);

    // (a) Alpha-rename callee locals → fresh, AND route each materialised
    // mutated parameter (CLOC18) through the same rename — applied to the body
    // AND the captured return expression (so `return x` for a mutated `x`
    // captures the post-assignment `<fresh>`, the fix the materialisation
    // exists for).
    let mut rename: HashMap<String, String> = mutated_rename;
    if !cand.locals.is_empty() {
        let mut gen = FreshNames::new();
        for local in &cand.locals {
            let fresh = gen.next(avoid);
            avoid.insert(fresh.clone());
            rename.insert(local.clone(), fresh);
        }
    }
    if !rename.is_empty() {
        for stmt in &mut body {
            rename_in_stmt(stmt, &rename);
        }
        rename_in_expr(&mut return_value, &rename);
    }

    // (b) Substitute pure parameters → arguments (or their temps), in the body
    // AND the captured return expression.
    if !param_map.is_empty() {
        for stmt in &mut body {
            substitute_in_stmt(stmt, &param_map);
        }
        substitute(&mut return_value, &param_map);
    }

    // Consume the (renamed/substituted) return value per the requested tail.
    match tail {
        // `const <temp> = E;` — a later statement reads it through `<temp>`.
        CaptureTail::IntoTemp(temp) => {
            body.push(Statement::Declaration(Declaration::VariableDeclaration(
                VariableDeclaration {
                    cv: None,
                    kind: VarKind::Const,
                    declarations: vec![VariableDeclarator {
                        cv: None,
                        id: BindingTarget::Identifier(Identifier {
                            cv: None,
                            name: temp.to_string(),
                        }),
                        init: Some(return_value),
                    }],
                },
            )));
        }
        // `return E;` — the value flows straight out (no temp).
        CaptureTail::AsReturn => {
            body.push(Statement::Tagged(TaggedStatement::ReturnStatement(
                ReturnStatement {
                    cv: None,
                    argument: Some(return_value),
                },
            )));
        }
        // `g = E;` — the value flows straight into the assignment target
        // (no temp). A bare-identifier simple-assignment expression statement.
        CaptureTail::IntoAssignment(target) => {
            body.push(Statement::Tagged(TaggedStatement::ExpressionStatement(
                ExpressionStatement {
                    cv: None,
                    expression: Expression::AssignmentExpression(AssignmentExpression {
                        cv: None,
                        operator: AssignmentOperator::Eq,
                        left: AssignmentTarget::Identifier(Identifier {
                            cv: None,
                            name: target.to_string(),
                        }),
                        right: Box::new(return_value),
                    }),
                },
            )));
        }
    }

    *nodes_touched += body.len() as u32;

    // The arg-temp prelude (if any) runs before the hoisted body + capture.
    let mut out = prelude;
    out.extend(body);
    out
}

/// Recurse a statement, splicing a capturable initializer in any nested
/// statement LIST (block / switch case / function body). A capturable
/// declaration is always a list element — a lexical/`var` declaration is
/// never the unbraced body of an `if`/loop — so single-statement slots are
/// only recursed into (never spliced at).
fn splice_valued_in_stmt(
    stmt: &mut Statement,
    cand: &VoidStmtCandidate,
    avoid: &mut HashSet<String>,
    nodes_touched: &mut u32,
) -> bool {
    *nodes_touched += 1;
    match stmt {
        Statement::Declaration(d) => splice_valued_in_decl(d, cand, avoid, nodes_touched),
        Statement::Tagged(t) => match t {
            TaggedStatement::BlockStatement(b) => {
                splice_valued_in_stmt_vec(&mut b.body, cand, avoid, nodes_touched)
            }
            TaggedStatement::IfStatement(is) => {
                let mut changed =
                    splice_valued_in_stmt(&mut is.consequent, cand, avoid, nodes_touched);
                if let Some(alt) = &mut is.alternate {
                    changed |= splice_valued_in_stmt(alt, cand, avoid, nodes_touched);
                }
                changed
            }
            TaggedStatement::WhileStatement(ws) => {
                splice_valued_in_stmt(&mut ws.body, cand, avoid, nodes_touched)
            }
            TaggedStatement::WithStatement(ws) => {
                splice_valued_in_stmt(&mut ws.body, cand, avoid, nodes_touched)
            }
            TaggedStatement::DoWhileStatement(ds) => {
                splice_valued_in_stmt(&mut ds.body, cand, avoid, nodes_touched)
            }
            TaggedStatement::ForStatement(fs) => {
                splice_valued_in_stmt(&mut fs.body, cand, avoid, nodes_touched)
            }
            TaggedStatement::ForInStatement(fs) => {
                splice_valued_in_stmt(&mut fs.body, cand, avoid, nodes_touched)
            }
            TaggedStatement::ForOfStatement(fs) => {
                splice_valued_in_stmt(&mut fs.body, cand, avoid, nodes_touched)
            }
            TaggedStatement::LabeledStatement(ls) => {
                splice_valued_in_stmt(&mut ls.body, cand, avoid, nodes_touched)
            }
            TaggedStatement::SwitchStatement(ss) => {
                let mut changed = false;
                for c in &mut ss.cases {
                    changed |=
                        splice_valued_in_stmt_vec(&mut c.consequent, cand, avoid, nodes_touched);
                }
                changed
            }
            TaggedStatement::TryStatement(ts) => {
                // Value-capture splice within any of the three blocks.
                let mut changed =
                    splice_valued_in_stmt_vec(&mut ts.block.body, cand, avoid, nodes_touched);
                if let Some(h) = &mut ts.handler {
                    changed |=
                        splice_valued_in_stmt_vec(&mut h.body.body, cand, avoid, nodes_touched);
                }
                if let Some(f) = &mut ts.finalizer {
                    changed |=
                        splice_valued_in_stmt_vec(&mut f.body, cand, avoid, nodes_touched);
                }
                changed
            }
            TaggedStatement::ExpressionStatement(_)
            | TaggedStatement::ReturnStatement(_)
            | TaggedStatement::ThrowStatement(_)
            | TaggedStatement::BreakStatement(_)
            | TaggedStatement::ContinueStatement(_)
            | TaggedStatement::EmptyStatement(_)
            | TaggedStatement::DebuggerStatement(_) => false,
        },
    }
}

/// Splice within a `Vec<Statement>` (block body, switch case, function
/// body): rebuild the list, expanding a matched capturable declaration into
/// its hoisted body + rewritten declaration.
fn splice_valued_in_stmt_vec(
    list: &mut Vec<Statement>,
    cand: &VoidStmtCandidate,
    avoid: &mut HashSet<String>,
    nodes_touched: &mut u32,
) -> bool {
    let mut changed = false;
    let mut out: Vec<Statement> = Vec::with_capacity(list.len());
    for stmt in std::mem::take(list) {
        if let Some(spliced) = try_capture_in_stmt(&stmt, cand, avoid, nodes_touched) {
            out.extend(spliced);
            changed = true;
        } else {
            let mut stmt = stmt;
            changed |= splice_valued_in_stmt(&mut stmt, cand, avoid, nodes_touched);
            out.push(stmt);
        }
    }
    *list = out;
    changed
}

fn splice_valued_in_decl(
    decl: &mut Declaration,
    cand: &VoidStmtCandidate,
    avoid: &mut HashSet<String>,
    nodes_touched: &mut u32,
) -> bool {
    match decl {
        Declaration::FunctionDeclaration(fd) => {
            splice_valued_in_stmt_vec(&mut fd.body.body, cand, avoid, nodes_touched)
        }
        // Each class method body is a `Vec<Statement>` a valued call may live
        // in — splice into every method, mirroring the function-body arm.
        // An import declaration has no inlinable body and binds foreign-linked
        // names — leave it untouched.
        Declaration::ImportDeclaration(_) => false,
        Declaration::ExportNamedDeclaration(_) => false,
        Declaration::ExportDefaultDeclaration(_) => false,
        Declaration::ExportAllDeclaration(_) => false,
        Declaration::ClassDeclaration(cd) => {
            let mut changed = false;
            for member in &mut cd.body {
                match member {
                    // A method body is a `Vec<Statement>` a valued call may live in.
                    ClassMember::Method(m) => {
                        changed |= splice_valued_in_stmt_vec(
                            &mut m.value.body.body,
                            cand,
                            avoid,
                            nodes_touched,
                        );
                    }
                    // A field's initializer is a value position — a valued call
                    // cannot be spliced there.
                    ClassMember::Field(_) => {}
                    // A static-init block's body IS a `Vec<Statement>` — splice
                    // into it like a method body.
                    ClassMember::StaticBlock(b) => {
                        changed |=
                            splice_valued_in_stmt_vec(&mut b.body, cand, avoid, nodes_touched);
                    }
                }
            }
            changed
        }
        // A variable initializer is a value position; the call must be the
        // ENTIRE init (handled at the list level by `try_capture_in_stmt` /
        // `capture_splice_for_vardecl`), so there is nothing to descend into
        // here (a call nested inside a larger init is declined).
        Declaration::VariableDeclaration(_) => false,
    }
}

// ---- statement-level rename (callee-local alpha-renaming) -----------------

/// Rename binding identifiers in a body statement per `map` — both the
/// declared name of a `let`/`const` and every use of it in expressions.
/// Handles the statement shapes the candidate admits (`ExpressionStatement`,
/// `let`/`const` `VariableDeclaration`, and — CLOC15 PR-4b — a control-flow-
/// inert `IfStatement` / `BlockStatement` of those); any other shape is left
/// untouched (it cannot occur in a valid body).
fn rename_in_stmt(stmt: &mut Statement, map: &HashMap<String, String>) {
    match stmt {
        Statement::Tagged(TaggedStatement::ExpressionStatement(es)) => {
            rename_in_expr(&mut es.expression, map)
        }
        Statement::Declaration(Declaration::VariableDeclaration(vd)) => {
            for d in &mut vd.declarations {
                let BindingTarget::Identifier(id) = &mut d.id;
                if let Some(fresh) = map.get(&id.name) {
                    id.name = fresh.clone();
                }
                if let Some(init) = &mut d.init {
                    rename_in_expr(init, map);
                }
            }
        }
        // PR-4b: an admitted `if` (test + control-flow-inert branches) and
        // the blocks its branches may be. The branch restriction guarantees
        // these contain only `ExpressionStatement`s — no nested declaration
        // re-binds a renamed name, so the name-based rename stays correct.
        Statement::Tagged(TaggedStatement::IfStatement(is)) => {
            rename_in_expr(&mut is.test, map);
            rename_in_stmt(&mut is.consequent, map);
            if let Some(alt) = &mut is.alternate {
                rename_in_stmt(alt, map);
            }
        }
        Statement::Tagged(TaggedStatement::BlockStatement(b)) => {
            for s in &mut b.body {
                rename_in_stmt(s, map);
            }
        }
        _ => {}
    }
}

/// Rename binding-use identifiers in an expression per `map`. Mirrors
/// [`collect_binding_idents_expr`] / [`substitute`]: property names (a
/// non-computed member `.x`, a non-computed object key) are never renamed.
fn rename_in_expr(expr: &mut Expression, map: &HashMap<String, String>) {
    match expr {
        Expression::Identifier(id) => {
            if let Some(fresh) = map.get(&id.name) {
                id.name = fresh.clone();
            }
        }
        Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::BigIntLiteral(_)
        // A regex literal is an inert leaf (no sub-expression, references no
        // identifier) — these traversals treat it exactly like a StringLiteral.
        | Expression::RegExpLiteral(_)
        // `this` is a leaf keyword — it binds/references no identifier and has
        // no sub-expression, so these traversals do nothing for it. (It is
        // deliberately NOT treated as a freely-substitutable primary: `this` is
        // bound at the call site, so the inliner's triv-/pure-expression
        // predicates leave it conservative.)
        | Expression::ThisExpression(_)
        | Expression::Super(_)
        | Expression::NewTarget(_)
        | Expression::ImportMeta(_)
        | Expression::UndefinedLiteral(_) => {}
        Expression::BinaryExpression(be) => {
            rename_in_expr(&mut be.left, map);
            rename_in_expr(&mut be.right, map);
        }
        Expression::LogicalExpression(le) => {
            rename_in_expr(&mut le.left, map);
            rename_in_expr(&mut le.right, map);
        }
        Expression::UnaryExpression(ue) => rename_in_expr(&mut ue.argument, map),
        Expression::UpdateExpression(ue) => rename_in_expr(&mut ue.argument, map),
        Expression::AssignmentExpression(ae) => {
            match &mut ae.left {
                AssignmentTarget::Identifier(id) => {
                    if let Some(fresh) = map.get(&id.name) {
                        id.name = fresh.clone();
                    }
                }
                AssignmentTarget::MemberExpression(m) => {
                    rename_in_expr(&mut m.object, map);
                    if m.computed {
                        rename_in_expr(&mut m.property, map);
                    }
                }
            }
            rename_in_expr(&mut ae.right, map);
        }
        Expression::ConditionalExpression(ce) => {
            rename_in_expr(&mut ce.test, map);
            rename_in_expr(&mut ce.consequent, map);
            rename_in_expr(&mut ce.alternate, map);
        }
        Expression::CallExpression(ce) => {
            rename_in_expr(&mut ce.callee, map);
            for a in &mut ce.arguments {
                rename_in_expr(a, map);
            }
        }
        Expression::NewExpression(ne) => {
            rename_in_expr(&mut ne.callee, map);
            for a in &mut ne.arguments {
                rename_in_expr(a, map);
            }
        }
        Expression::SequenceExpression(se) => {
            for e in &mut se.expressions {
                rename_in_expr(e, map);
            }
        }
        Expression::MemberExpression(m) => {
            rename_in_expr(&mut m.object, map);
            if m.computed {
                rename_in_expr(&mut m.property, map);
            }
        }
        // `a?.b` / `a?.[k]` — rename in object and (computed) property exactly
        // as a plain member access.
        Expression::OptionalMemberExpression(m) => {
            rename_in_expr(&mut m.object, map);
            if m.computed {
                rename_in_expr(&mut m.property, map);
            }
        }
        // `a?.()` — rename in callee and each argument, as for a call.
        Expression::OptionalCallExpression(ce) => {
            rename_in_expr(&mut ce.callee, map);
            for a in &mut ce.arguments {
                rename_in_expr(a, map);
            }
        }
        // A chain expression transparently wraps its optional-chain spine.
        Expression::ChainExpression(c) => rename_in_expr(&mut c.expression, map),
        Expression::ArrayExpression(ae) => {
            for el in ae.elements.iter_mut().flatten() {
                rename_in_expr(el, map);
            }
        }
        Expression::ObjectExpression(oe) => {
            for member in &mut oe.properties {
                match member {
                    ObjectMember::Property(prop) => {
                        if prop.computed {
                            if let PropertyKey::Expression(e) = &mut prop.key {
                                rename_in_expr(e, map);
                            }
                        }
                        rename_in_expr(&mut prop.value, map);
                    }
                    // Object spread `...expr` — recurse into the spread
                    // argument the same way the property arm recurses into
                    // prop.value.
                    ObjectMember::Spread(s) => {
                        rename_in_expr(&mut s.argument, map);
                    }
                }
            }
        }
        // Alpha-rename uses inside a function *value*'s body, but its own
        // name and params SHADOW any outer name being renamed — drop those
        // keys before recursing so a shadowed use keeps its (inner) name.
        Expression::FunctionExpression(fe) => {
            let mut inner = map.clone();
            if let Some(id) = &fe.id {
                inner.remove(&id.name);
            }
            for p in &fe.params {
                let id = p.binding_identifier();
                inner.remove(&id.name);
            }
            // A default's `right` can reference an outer name being alpha-renamed
            // (`(x = loc) => …` spliced where `loc` is renamed); rename it with
            // the shadow-stripped `inner`, exactly like the body.
            for p in &mut fe.params {
                if let Some(def) = p.default_value_mut() {
                    rename_in_expr(def, &inner);
                }
            }
            for s in &mut fe.body.body {
                rename_in_stmt(s, &inner);
            }
        }
        // Alpha-rename uses inside a class *value*. The `extends` operand is
        // evaluated in the ENCLOSING scope, so rename it with the outer `map`.
        // Each method body is a nested scope where the class's own name, the
        // method value's own name, and the method params SHADOW an outer name
        // being renamed — drop those keys before recursing so a shadowed use
        // keeps its (inner) name, exactly as the `FunctionExpression` arm
        // does. A method KEY is a property name, never a renamed use.
        Expression::ClassExpression(ce) => {
            if let Some(sup) = &mut ce.super_class {
                rename_in_expr(sup, map);
            }
            let mut class_inner = map.clone();
            if let Some(id) = &ce.id {
                class_inner.remove(&id.name);
            }
            for member in &mut ce.body {
                match member {
                    ClassMember::Method(m) => {
                        let mut inner = class_inner.clone();
                        if let Some(id) = &m.value.id {
                            inner.remove(&id.name);
                        }
                        for p in &m.value.params {
                            let id = p.binding_identifier();
                            inner.remove(&id.name);
                        }
                        // Default-param `right` expressions — see the
                        // `FunctionExpression` arm.
                        for p in &mut m.value.params {
                            if let Some(def) = p.default_value_mut() {
                                rename_in_expr(def, &inner);
                            }
                        }
                        for s in &mut m.value.body.body {
                            rename_in_stmt(s, &inner);
                        }
                    }
                    // A field's initializer and computed key are renamed with
                    // `class_inner` (the class's own name in scope, no method
                    // params). The field KEY name is a property name, never a
                    // renamed use.
                    ClassMember::Field(f) => {
                        if let PropertyKey::Expression(e) = &mut f.key {
                            rename_in_expr(e, &class_inner);
                        }
                        if let Some(v) = &mut f.value {
                            rename_in_expr(v, &class_inner);
                        }
                    }
                    // A static-init block's statements are alpha-renamed with
                    // `class_inner` (the class's own name in scope, no method
                    // params).
                    ClassMember::StaticBlock(b) => {
                        for s in &mut b.body {
                            rename_in_stmt(s, &class_inner);
                        }
                    }
                }
            }
        }
        // An arrow's params SHADOW any outer name being alpha-renamed —
        // drop those keys before recursing so a shadowed use keeps its
        // (inner) name. (Arrows have no self-name.)
        Expression::ArrowFunctionExpression(ae) => {
            let mut inner = map.clone();
            for p in &ae.params {
                let id = p.binding_identifier();
                inner.remove(&id.name);
            }
            // Default-param `right` expressions — see the `FunctionExpression` arm.
            for p in &mut ae.params {
                if let Some(def) = p.default_value_mut() {
                    rename_in_expr(def, &inner);
                }
            }
            match &mut ae.body {
                ArrowBody::Block(b) => {
                    for s in &mut b.body {
                        rename_in_stmt(s, &inner);
                    }
                }
                ArrowBody::Expression(e) => rename_in_expr(e, &inner),
            }
        }
        // A template literal binds nothing, so there is no shadowing to strip
        // — alpha-rename straight through each `${…}` insert. Quasis are leaf
        // strings and contain no identifier uses.
        Expression::TemplateLiteral(t) => {
            for e in &mut t.expressions {
                rename_in_expr(e, map);
            }
        }
        // A tagged template binds nothing — alpha-rename through the tag callee
        // and each `${…}` insert of the applied template.
        Expression::TaggedTemplateExpression(t) => {
            rename_in_expr(&mut t.tag, map);
            for e in &mut t.quasi.expressions {
                rename_in_expr(e, map);
            }
        }
        // `...arg` — recurse into the spread argument to alpha-rename through it.
        Expression::SpreadElement(s) => rename_in_expr(&mut s.argument, map),
        Expression::YieldExpression(y) => { if let Some(a) = &mut y.argument { rename_in_expr(a, map); } }
        Expression::AwaitExpression(a) => rename_in_expr(&mut a.argument, map),
        Expression::ImportExpression(e) => rename_in_expr(&mut e.source, map),
    }
}

/// Apply parameter→argument [`substitute`] across a body statement (the
/// admitted shapes, including the PR-4b `if` / block).
fn substitute_in_stmt(stmt: &mut Statement, map: &HashMap<String, Expression>) {
    match stmt {
        Statement::Tagged(TaggedStatement::ExpressionStatement(es)) => {
            substitute(&mut es.expression, map)
        }
        Statement::Declaration(Declaration::VariableDeclaration(vd)) => {
            for d in &mut vd.declarations {
                if let Some(init) = &mut d.init {
                    substitute(init, map);
                }
            }
        }
        // PR-4b: an admitted `if` and the blocks its branches may be.
        Statement::Tagged(TaggedStatement::IfStatement(is)) => {
            substitute(&mut is.test, map);
            substitute_in_stmt(&mut is.consequent, map);
            if let Some(alt) = &mut is.alternate {
                substitute_in_stmt(alt, map);
            }
        }
        Statement::Tagged(TaggedStatement::BlockStatement(b)) => {
            for s in &mut b.body {
                substitute_in_stmt(s, map);
            }
        }
        _ => {}
    }
}

// ---- used-identifier collection (free-var guard + fresh-name avoidance) ---

/// Collect every binding-use identifier name in the program into `out`,
/// recursing into nested function bodies. Used both to vet a candidate's
/// free identifiers and to build the fresh-name avoidance set. Property
/// names are excluded (handled by [`collect_binding_idents_expr`]).
fn collect_used_idents_program(program: &Program, out: &mut HashSet<String>) {
    for item in &program.body {
        match item {
            ProgramItem::Declaration(d) => collect_used_idents_decl(d, out),
            ProgramItem::Statement(s) => collect_used_idents_stmt(s, out),
        }
    }
}

fn collect_used_idents_decl(decl: &Declaration, out: &mut HashSet<String>) {
    match decl {
        Declaration::VariableDeclaration(vd) => {
            for d in &vd.declarations {
                if let Some(init) = &d.init {
                    collect_binding_idents_expr(init, out);
                }
            }
        }
        Declaration::FunctionDeclaration(fd) => {
            for s in &fd.body.body {
                collect_used_idents_stmt(s, out);
            }
        }
        // Collect identifiers used in a class declaration's heritage operand and
        // method bodies, so an inline never mints a fresh name that collides
        // with one referenced inside the class.
        // An import declaration has no inlinable body and binds foreign-linked
        // names — leave it untouched.
        Declaration::ImportDeclaration(_) => {}
        Declaration::ExportNamedDeclaration(_) => {}
        Declaration::ExportDefaultDeclaration(_) => {}
        Declaration::ExportAllDeclaration(_) => {}
        Declaration::ClassDeclaration(cd) => {
            if let Some(sup) = &cd.super_class {
                collect_binding_idents_expr(sup, out);
            }
            for member in &cd.body {
                match member {
                    ClassMember::Method(m) => {
                        for s in &m.value.body.body {
                            collect_used_idents_stmt(s, out);
                        }
                    }
                    // Over-collect identifiers referenced in a field's
                    // initializer and computed key, so an inline never mints a
                    // fresh name that collides with one used there.
                    ClassMember::Field(f) => {
                        if let PropertyKey::Expression(e) = &f.key {
                            collect_binding_idents_expr(e, out);
                        }
                        if let Some(v) = &f.value {
                            collect_binding_idents_expr(v, out);
                        }
                    }
                    // Over-collect identifiers referenced in the static-init
                    // block's statements, so an inline never mints a fresh name
                    // that collides with one used there.
                    ClassMember::StaticBlock(b) => {
                        for s in &b.body {
                            collect_used_idents_stmt(s, out);
                        }
                    }
                }
            }
        }
    }
}

fn collect_used_idents_stmt(stmt: &Statement, out: &mut HashSet<String>) {
    match stmt {
        Statement::Declaration(d) => collect_used_idents_decl(d, out),
        Statement::Tagged(t) => match t {
            TaggedStatement::ExpressionStatement(es) => {
                collect_binding_idents_expr(&es.expression, out)
            }
            TaggedStatement::BlockStatement(b) => {
                for s in &b.body {
                    collect_used_idents_stmt(s, out);
                }
            }
            TaggedStatement::IfStatement(is) => {
                collect_binding_idents_expr(&is.test, out);
                collect_used_idents_stmt(&is.consequent, out);
                if let Some(alt) = &is.alternate {
                    collect_used_idents_stmt(alt, out);
                }
            }
            TaggedStatement::WhileStatement(ws) => {
                collect_binding_idents_expr(&ws.test, out);
                collect_used_idents_stmt(&ws.body, out);
            }
            TaggedStatement::WithStatement(ws) => {
                collect_binding_idents_expr(&ws.object, out);
                collect_used_idents_stmt(&ws.body, out);
            }
            TaggedStatement::DoWhileStatement(ds) => {
                collect_binding_idents_expr(&ds.test, out);
                collect_used_idents_stmt(&ds.body, out);
            }
            TaggedStatement::ForStatement(fs) => {
                if let Some(init) = &fs.init {
                    match init {
                        ForInit::VariableDeclaration(vd) => {
                            for d in &vd.declarations {
                                if let Some(i) = &d.init {
                                    collect_binding_idents_expr(i, out);
                                }
                            }
                        }
                        ForInit::Expression(e) => collect_binding_idents_expr(e, out),
                    }
                }
                if let Some(test) = &fs.test {
                    collect_binding_idents_expr(test, out);
                }
                if let Some(update) = &fs.update {
                    collect_binding_idents_expr(update, out);
                }
                collect_used_idents_stmt(&fs.body, out);
            }
            TaggedStatement::ForInStatement(fs) => {
                match &fs.left {
                    ForInit::VariableDeclaration(vd) => {
                        for d in &vd.declarations {
                            if let Some(i) = &d.init {
                                collect_binding_idents_expr(i, out);
                            }
                        }
                    }
                    ForInit::Expression(e) => collect_binding_idents_expr(e, out),
                }
                collect_binding_idents_expr(&fs.right, out);
                collect_used_idents_stmt(&fs.body, out);
            }
            TaggedStatement::ForOfStatement(fs) => {
                match &fs.left {
                    ForInit::VariableDeclaration(vd) => {
                        for d in &vd.declarations {
                            if let Some(i) = &d.init {
                                collect_binding_idents_expr(i, out);
                            }
                        }
                    }
                    ForInit::Expression(e) => collect_binding_idents_expr(e, out),
                }
                collect_binding_idents_expr(&fs.right, out);
                collect_used_idents_stmt(&fs.body, out);
            }
            TaggedStatement::ReturnStatement(rs) => {
                if let Some(a) = &rs.argument {
                    collect_binding_idents_expr(a, out);
                }
            }
            TaggedStatement::ThrowStatement(ts) => collect_binding_idents_expr(&ts.argument, out),
            TaggedStatement::LabeledStatement(ls) => collect_used_idents_stmt(&ls.body, out),
            TaggedStatement::SwitchStatement(ss) => {
                collect_binding_idents_expr(&ss.discriminant, out);
                for c in &ss.cases {
                    if let Some(test) = &c.test {
                        collect_binding_idents_expr(test, out);
                    }
                    for s in &c.consequent {
                        collect_used_idents_stmt(s, out);
                    }
                }
            }
            TaggedStatement::TryStatement(ts) => {
                // The catch `param` MUST join the avoid set: a callee local
                // alpha-renamed to a fresh name at a splice site inside the
                // catch body must not collide with `param`. Recurse into the
                // three blocks for every other used identifier.
                for s in &ts.block.body {
                    collect_used_idents_stmt(s, out);
                }
                if let Some(h) = &ts.handler {
                    if let Some(param) = &h.param {
                        out.insert(param.name.clone());
                    }
                    for s in &h.body.body {
                        collect_used_idents_stmt(s, out);
                    }
                }
                if let Some(f) = &ts.finalizer {
                    for s in &f.body {
                        collect_used_idents_stmt(s, out);
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

// ---- fresh-name generation (callee-local alpha-renaming) ------------------

/// Reserved words a fresh local name must never collide with. Kept local
/// to this crate (the `rename` family keeps its own private copy); a
/// shared primitive is a reasonable future extraction, but duplicating a
/// short, stable list here avoids a premature cross-crate coupling.
const RESERVED: &[&str] = &[
    "do",
    "if",
    "in",
    "for",
    "let",
    "new",
    "try",
    "var",
    "case",
    "else",
    "enum",
    "eval",
    "null",
    "this",
    "true",
    "void",
    "with",
    "break",
    "catch",
    "class",
    "const",
    "false",
    "super",
    "throw",
    "while",
    "yield",
    "delete",
    "export",
    "import",
    "public",
    "return",
    "static",
    "switch",
    "typeof",
    "default",
    "extends",
    "finally",
    "package",
    "private",
    "continue",
    "debugger",
    "function",
    "arguments",
    "interface",
    "protected",
    "implements",
    "instanceof",
];

/// Base-26 fresh-name generator (`a`, `b`, …, `z`, `aa`, …) that skips
/// reserved words and any name in the caller-supplied avoidance set.
/// Mirrors the generator the `rename` passes use; here it produces names
/// guaranteed not to clash with anything in the program.
struct FreshNames {
    counter: usize,
}

impl FreshNames {
    fn new() -> Self {
        FreshNames { counter: 0 }
    }

    /// Yield the next name not in `avoid` and not reserved.
    fn next(&mut self, avoid: &HashSet<String>) -> String {
        loop {
            let name = Self::encode(self.counter);
            self.counter += 1;
            if !RESERVED.contains(&name.as_str()) && !avoid.contains(&name) {
                return name;
            }
        }
    }

    /// Encode `n` as a lowercase base-26 identifier: 0→`a`, 25→`z`,
    /// 26→`aa`, … (bijective base-26, so every n maps to a distinct name).
    fn encode(mut n: usize) -> String {
        let mut chars = Vec::new();
        loop {
            chars.push((b'a' + (n % 26) as u8) as char);
            if n < 26 {
                break;
            }
            n = n / 26 - 1;
        }
        chars.iter().rev().collect()
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
    use coding_adventures_correlation_vector::{CVLog, Contribution};
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

    /// Parse `src`, bridge, run `InlinePass`, and return its CV
    /// contributions — the `inlined` table (#89 provenance).
    fn inline_contributions(src: &str) -> Vec<Contribution> {
        let es = EsVersion::Es2025;
        let node = parse_javascript_typed(src, es).expect("parse");
        let prog = bridge::grammar_to_program(&node, es).expect("bridge");
        let pass = InlinePass::new();
        let sidecar = Sidecar::new();
        let mut cv = CVLog::new(true);
        pass.run(PassContext {
            program: &prog,
            sidecar: &sidecar,
            cv: &mut cv,
        })
        .expect("inline")
        .contributions
    }

    // ----- CV provenance (#89): `inlined` contributions -----

    #[test]
    fn single_use_expression_inline_records_one_site() {
        // `id` is inlined at its lone call site; the record names it with
        // `sites: 1`.
        let contribs = inline_contributions("function id(x){return x;} id(7);");
        assert_eq!(contribs.len(), 1, "one inlined function; got {contribs:?}");
        let c = &contribs[0];
        assert_eq!(c.source, "inline");
        assert_eq!(c.tag, "inlined");
        assert_eq!(c.meta.get("name").and_then(|v| v.as_str()), Some("id"));
        assert_eq!(c.meta.get("sites").and_then(|v| v.as_u64()), Some(1));
    }

    #[test]
    fn multi_use_expression_inline_records_site_count() {
        // A tiny body used twice is inlined at BOTH sites under the size
        // budget; `sites` reflects the count.
        let contribs = inline_contributions("function d(x){return x+x;} d(1); d(2);");
        assert_eq!(contribs.len(), 1, "one inlined function; got {contribs:?}");
        assert_eq!(
            contribs[0].meta.get("name").and_then(|v| v.as_str()),
            Some("d")
        );
        assert_eq!(
            contribs[0].meta.get("sites").and_then(|v| v.as_u64()),
            Some(2)
        );
    }

    #[test]
    fn no_inline_emits_no_contributions() {
        // `g(f)` passes `f` as a value — not an inlinable call — so nothing
        // is inlined and the table is empty.
        let contribs = inline_contributions("function f(x){return x;} g(f);");
        assert!(
            contribs.is_empty(),
            "expected no contributions; got {contribs:?}"
        );
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
        let _d = _c;
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
            "function double(x){return x*2}log(7*2);"
        );
    }

    #[test]
    fn inlines_identity_with_identifier_arg() {
        // A bare-identifier argument substitutes cleanly: `id(value)`
        // → `value`.
        assert_eq!(
            inline_source("function id(v) { return v; } print(id(value));"),
            "function id(v){return v}print(value);"
        );
    }

    #[test]
    fn inlines_two_param_function() {
        assert_eq!(
            inline_source("function add(a, b) { return a + b; } use(add(p, q));"),
            "function add(a,b){return a+b}use(p+q);"
        );
    }

    #[test]
    fn preserves_property_name_on_substitution() {
        // `o.x` with `o` the parameter: substitute `o` → `obj`, but the
        // `.x` property name must NOT be touched.
        assert_eq!(
            inline_source("function get(o) { return o.x; } use(get(obj));"),
            "function get(o){return o.x}use(obj.x);"
        );
    }

    #[test]
    fn inlines_computed_member() {
        // A computed `o[i]` IS a use position — both params substitute.
        assert_eq!(
            inline_source("function at(o, i) { return o[i]; } use(at(arr, idx));"),
            "function at(o,i){return o[i]}use(arr[idx]);"
        );
    }

    #[test]
    fn inlines_nested_call_argument() {
        // The call can be nested inside another call's arguments.
        assert_eq!(
            inline_source("function double(x) { return x * 2; } outer(inner(double(5)));"),
            "function double(x){return x*2}outer(inner(5*2));"
        );
    }

    #[test]
    fn inlines_multi_use_small_body() {
        // Two call sites of a tiny body (`x * 2` is 3 nodes, the budget
        // for one param is 2 + 1 = 3) → both sites are inlined. The dead
        // declaration is left for the downstream passes to remove.
        assert_eq!(
            inline_source("function d(x) { return x * 2; } a(d(1)); b(d(2));"),
            "function d(x){return x*2}a(1*2);b(2*2);"
        );
    }

    #[test]
    fn does_not_inline_multi_use_large_body() {
        // `x * x * x` is 5 nodes; the budget for one param is 2 + 1 = 3.
        // Duplicating a 5-node body across two sites could grow the
        // output, so multi-use inlining is declined. (A single call site
        // WOULD still be inlined — see the single-use tests.)
        assert_eq!(
            inline_source("function cube(x) { return x * x * x; } a(cube(p)); b(cube(q));"),
            "function cube(x){return x*x*x}a(cube(p));b(cube(q));"
        );
    }

    #[test]
    fn does_not_inline_when_one_of_several_uses_is_not_a_call() {
        // `f` is used 3 times: two inlinable calls and one value use
        // (`keep(f)`). Inlining the calls would leave `f` still
        // referenced, so the declaration couldn't be removed — a likely
        // net loss. We decline the whole function (uses != inlinable).
        assert_eq!(
            inline_source("function f(x) { return x * 2; } a(f(1)); b(f(2)); keep(f);"),
            "function f(x){return x*2}a(f(1));b(f(2));keep(f);"
        );
    }

    #[test]
    fn does_not_inline_multi_use_when_one_call_has_bad_args() {
        // Two calls, but one passes a side-effecting argument `g()`.
        // That call is not inlinable, so uses != inlinable → the whole
        // function is declined (no partial inlining).
        assert_eq!(
            inline_source("function d(x) { return x * 2; } a(d(1)); b(d(g()));"),
            "function d(x){return x*2}a(d(1));b(d(g()));"
        );
    }

    #[test]
    fn does_not_inline_recursive_function() {
        // `f` appears free in its own body (the inner call), so it is
        // not a candidate — recursion is excluded by the capture guard.
        assert_eq!(
            inline_source("function f(x) { return f(x); } g(f(1));"),
            "function f(x){return f(x)}g(f(1));"
        );
    }

    #[test]
    fn does_not_inline_body_with_free_global() {
        // `g` is a free identifier (not a parameter), so substituting
        // the body at the call site could capture a differently-scoped
        // `g`. Rejected.
        assert_eq!(
            inline_source("function f(x) { return x + g; } h(f(1));"),
            "function f(x){return x+g}h(f(1));"
        );
    }

    #[test]
    fn does_not_inline_shadowed_name() {
        // The name `f` is declared twice (the top-level function and a
        // parameter `f` of `uses`), so a use of `f` could resolve to
        // either binding. Rejected by the shadow guard.
        assert_eq!(
            inline_source("function f(x) { return x * 2; } function uses(f) { return f(1); }"),
            "function f(x){return x*2}function uses(f){return f(1)};"
        );
    }

    #[test]
    fn does_not_inline_on_arity_mismatch() {
        // Call passes one argument to a two-parameter function — the
        // arity check fails, so the call is left intact.
        assert_eq!(
            inline_source("function add(a, b) { return a + b; } k(add(1));"),
            "function add(a,b){return a+b}k(add(1));"
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
            "function f(x){return x*2}m(f(g()));"
        );
    }

    #[test]
    fn does_not_inline_non_call_value_use() {
        // `f` is used once, but as a *value* (passed to `h`), not
        // called. There is no call to substitute, so nothing changes.
        assert_eq!(
            inline_source("function f(x) { return x * 2; } h(f);"),
            "function f(x){return x*2}h(f);"
        );
    }

    #[test]
    fn does_not_inline_multi_statement_body_with_return() {
        // Body has a local + a return — neither the `{ return EXPR; }`
        // expression shape nor the no-return void shape (CLOC15 PR-1
        // forbids `return`), and it is used as a value — so it is not a
        // candidate for either inliner.
        assert_eq!(
            inline_source("function f(x) { var t = x * 2; return t; } use(f(3));"),
            "function f(x){var t=x*2;return t}use(f(3));"
        );
    }

    // =====================================================================
    // CLOC15 PR-1: void multi-statement statement-helper inlining
    // =====================================================================

    #[test]
    fn inlines_void_helper_with_local_and_free_global() {
        // The signature case: a single-use void helper with a local
        // (`e`) and a free global (`metrics`), called as a statement, is
        // replaced by its body. The local is alpha-renamed to a fresh
        // name and the parameters are substituted by the (simple) args.
        // The dead declaration is left for downstream passes.
        assert_eq!(
            inline_source(
                "function track(n, v) { const e = n+v; metrics.push(e); } track(a, b);"
            ),
            "function track(n,v){const e=n+v;metrics.push(e)}const c=a+b;metrics.push(c);"
        );
    }

    #[test]
    fn inlines_void_helper_no_locals() {
        // No locals, only free globals + a param: both call sites of the
        // body run with the argument substituted in.
        assert_eq!(
            inline_source("function log2(x) { console.log(x); console.log(x); } log2(v);"),
            "function log2(x){console.log(x);console.log(x)}console.log(v);console.log(v);"
        );
    }

    #[test]
    fn alpha_renames_local_that_would_collide_with_argument() {
        // The body's local `c` would collide with the argument `c` once
        // substituted. Alpha-renaming the local to a program-fresh name
        // keeps them distinct — the soundness crux of statement splicing.
        assert_eq!(
            inline_source("function f(x) { const c = x; sink(c); } f(c);"),
            "function f(x){const c=x;sink(c)}const a=c;sink(a);"
        );
    }

    #[test]
    fn inlines_empty_void_helper_drops_the_call() {
        // An empty body splices nothing — the call statement disappears.
        // Sound because the (simple) argument has no side effect to drop.
        assert_eq!(
            inline_source("function noop(x) {} noop(v);"),
            "function noop(x){};"
        );
    }

    #[test]
    fn wraps_spliced_body_in_block_at_unbraced_if() {
        // The single call sits in an unbraced `if` consequent. Splicing
        // two statements there must wrap them in a block, or only the
        // first would be guarded by the condition.
        assert_eq!(
            inline_source("function f() { a(); b(); } if (c) f();"),
            "function f(){a();b()}if(c){a();b()}"
        );
    }

    #[test]
    fn does_not_inline_void_helper_used_as_value() {
        // The sole use is a value position (`var x = f()`), not a
        // discarded statement call. This slice declines it.
        assert_eq!(
            inline_source("function f() { sink(1); } var x = f();"),
            "function f(){sink(1)}var x=f();"
        );
    }

    #[test]
    fn inlines_void_helper_with_var_local() {
        // CLOC15 Open Q3: a `var` local is now admitted. The bridge desugars
        // `var t = x` into `var t; t = x`, and the local `t` is alpha-renamed
        // to a program-fresh name (`b`) — so the hoisted `var b` is inert
        // (nothing else references `b`). Previously this slice declined `var`.
        assert_eq!(
            inline_source("function f(x) { var t = x; sink(t); } f(a);"),
            "function f(x){var t=x;sink(t)}var b=a;sink(b);"
        );
    }

    // ----- CLOC15 PR-2: tail `return` with a discarded result -----

    #[test]
    fn inlines_tail_return_literal_dropped() {
        // The call discards the result, so a tail `return <literal>` is
        // provably inert and dropped entirely; the rest splices as usual.
        assert_eq!(
            inline_source("function f(x) { sink(x); return 1; } f(a);"),
            "function f(x){sink(x);return 1}sink(a);"
        );
    }

    #[test]
    fn inlines_tail_bare_return_dropped() {
        // A bare `return;` is a no-op once the value is discarded — dropped.
        assert_eq!(
            inline_source("function f(x) { sink(x); return; } f(a);"),
            "function f(x){sink(x);return}sink(a);"
        );
    }

    #[test]
    fn inlines_tail_return_param_identifier_dropped() {
        // `return x` reads a parameter — a binding that always exists, so
        // the read neither throws nor has a side effect; dropped.
        assert_eq!(
            inline_source("function f(x) { sink(x); return x; } f(a);"),
            "function f(x){sink(x);return x}sink(a);"
        );
    }

    #[test]
    fn inlines_tail_return_effectful_kept_as_statement() {
        // `return setup()` may have a side effect, so the value is dropped
        // but the call is KEPT as a statement (`setup();`) for its effect.
        assert_eq!(
            inline_source("function f(x) { log(x); return setup(); } f(a);"),
            "function f(x){log(x);return setup()}log(a);setup();"
        );
    }

    #[test]
    fn inlines_single_tail_return_with_free_global() {
        // Body is a single tail `return g()` where `g` is a free global —
        // the expression inliner (all-idents-must-be-params) cannot touch
        // this, but the discarded-result statement splice can: `g();`.
        assert_eq!(
            inline_source("function f() { return setup(); } f();"),
            "function f(){return setup()}setup();"
        );
    }

    #[test]
    fn inlines_tail_return_free_global_identifier_kept() {
        // `return glob` reads a free global, which can throw `ReferenceError`
        // if undeclared — so the read is preserved as `glob;`, not dropped.
        assert_eq!(
            inline_source("function f(x) { sink(x); return glob; } f(a);"),
            "function f(x){sink(x);return glob}sink(a);glob;"
        );
    }

    #[test]
    fn does_not_inline_void_helper_with_early_return() {
        // A `return` that is NOT the final statement would change control
        // flow on a flat splice (the following statements would still run),
        // so the candidate is declined.
        assert_eq!(
            inline_source("function f(x) { return; sink(x); } f(a);"),
            "function f(x){return;sink(x)}f(a);"
        );
    }

    #[test]
    fn does_not_inline_void_helper_referencing_arguments() {
        // `arguments` is bound by the callee's own frame; splicing would
        // rebind it. Rejected.
        assert_eq!(
            inline_source("function f() { sink(arguments); } f();"),
            "function f(){sink(arguments)}f();"
        );
    }

    // ----- CLOC16 Slice A: free idents resolving to top-level decls -----

    #[test]
    fn inlines_top_level_helper_referencing_top_level_const() {
        // CLOC16 Slice A: the body reads a top-level `const K` — a free
        // identifier that resolves to a PROGRAM-SCOPE declaration. The single
        // call `f()` is a direct `program.body` statement, so at that scope
        // `K` resolves to the same top-level binding it did in the helper
        // (nothing can shadow it at program scope). The body is spliced and
        // `K` is preserved. (Previously declined by the conservative
        // global-only rule; this is the intended Slice A behaviour change. The
        // now-dead `function f` declaration is removed downstream by
        // remove-unused-vars / treeshake, not by this pass.)
        assert_eq!(
            inline_source("const K = 5; function f() { sink(K); } f();"),
            "const K=5;function f(){sink(K)}sink(K);"
        );
    }

    #[test]
    fn inlines_top_level_helper_referencing_sibling_function() {
        // The common case: a helper that calls another top-level function.
        // `dep` is program-scope, the call site is top level → spliced. `dep`
        // is kept multi-use AND multi-statement so it is NOT itself inlined —
        // it survives as a genuine free top-level reference inside the spliced
        // body.
        assert_eq!(
            inline_source(
                "function dep(x) { trace(x); return x*2; } dep(0); function f(p) { log(p); use(dep(p)); } f(5);"
            ),
            "function dep(x){trace(x);return x*2}dep(0);function f(p){log(p);use(dep(p))}log(5);use(dep(5));"
        );
    }

    // ----- CLOC16 Slice B: nested sites for UNIQUELY-declared top-level refs

    #[test]
    fn inlines_unique_top_level_ref_at_nested_site() {
        // CLOC16 Slice B (uniqueness gate): `K` is a top-level `const`
        // declared EXACTLY ONCE program-wide, so no other binding of `K`
        // exists anywhere — it cannot be shadowed at any site. The helper `f`
        // therefore carries no top-level-only obligation and inlines even at a
        // NESTED call site (inside `main`). (`main` is multi-use, so it is not
        // itself inlined — the call stays nested.) Previously declined by
        // Slice A's blanket top-level-only rule; admitting it is the intended
        // Slice B behaviour change.
        assert_eq!(
            inline_source(
                "const K = 5; function f() { sink(K); } function main() { f(); } main(); main();"
            ),
            "const K=5;function f(){sink(K)}function main(){sink(K)}main();main();"
        );
    }

    #[test]
    fn inlines_unique_top_level_ref_in_block() {
        // Likewise inside a top-level block: `K` declared once ⇒ unshadowable,
        // so the splice is sound there too.
        assert_eq!(
            inline_source("const K = 5; function f() { sink(K); } { f(); }"),
            "const K=5;function f(){sink(K)}{sink(K)}"
        );
    }

    #[test]
    fn does_not_inline_multiply_declared_top_level_ref_at_nested_site() {
        // The name is declared TWICE — top-level `function dep` AND a local
        // `let dep` in `g`. `decl_counts[dep] == 2`, so the uniqueness gate
        // does NOT apply and `f` keeps the Slice A top-level-only obligation.
        // Its single call sits inside `g` (a nested site that DOES shadow
        // `dep`), so the splice is declined — preventing the miscompile where
        // `use(dep)` would read `g`'s local. `dep` is multi-use+multi-statement
        // so it is not itself inlined; `g` is multi-use so it stays.
        assert_eq!(
            inline_source(
                "function dep() { keep(); return 1; } dep(); function f() { log(0); use(dep); } function g() { let dep = 99; f(); } g(); g();"
            ),
            "function dep(){keep();return 1}dep();function f(){log(0);use(dep)}function g(){let dep=99;f()}g();g();"
        );
    }

    #[test]
    fn inlines_multiply_declared_top_level_ref_at_top_level_site() {
        // The SAME multiply-declared `dep`, but `f`'s single call is at the
        // top level — a direct `program.body` site where program scope cannot
        // shadow `dep` (the other `dep` is a local in `other`, out of scope
        // here). Slice A admits it. This keeps the Slice A top-level path
        // exercised now that the uniqueness gate handles the single-decl case.
        assert_eq!(
            inline_source(
                "function dep() { keep(); return 1; } dep(); function f() { log(0); use(dep); } f(); function other() { let dep = 5; return dep; } other(); other();"
            ),
            "function dep(){keep();return 1}dep();function f(){log(0);use(dep)}log(0);use(dep);function other(){let dep=5;return dep}other();other();"
        );
    }

    #[test]
    fn does_not_inline_when_nested_function_shadows_top_level_ref() {
        // The soundness linchpin for the uniqueness gate: `count_decl_names_*`
        // counts NESTED function-declaration names too, so the top-level
        // `function dep` plus the nested `function dep` inside `g` make
        // `decl_counts[dep] == 2`. `f` therefore keeps the top-level-only
        // obligation, and its single (nested) call inside `g` is declined —
        // preventing the miscompile where the spliced `use(dep)` would capture
        // `g`'s nested `dep` (99) instead of the top-level one.
        assert_eq!(
            inline_source(
                "function dep() { return 1; } dep(); function f() { log(0); use(dep); } function g() { function dep() { return 99; } f(); dep(); } g(); g();"
            ),
            "function dep(){return 1}dep();function f(){log(0);use(dep)}function g(){function dep(){return 99}f();dep()}g();g();"
        );
    }

    #[test]
    fn does_not_inline_free_name_declared_only_in_other_function() {
        // The body's free `q` is declared ONLY inside an unrelated function
        // (never at program scope). We cannot prove what it resolves to, so
        // the candidate is rejected outright — not even a top-level call site
        // makes it sound (CLOC16 proof obligation 4). `other` is multi-use AND
        // multi-statement so it is not itself inlined (keeping the test focused
        // on `f`).
        assert_eq!(
            inline_source(
                "function other(q) { trace(q); return q; } other(1); other(2); function f() { sink(q); } f();"
            ),
            "function other(q){trace(q);return q}other(1);other(2);function f(){sink(q)}f();"
        );
    }

    // ----- CLOC15 PR-4a: non-simple arguments via per-argument temps -----

    #[test]
    fn inlines_side_effecting_argument_via_temp() {
        // A side-effecting argument `g()` is hoisted into a temp evaluated
        // exactly once, then the parameter reads the temp — so the call
        // inlines without dropping or duplicating the side effect. (PR-1..3
        // declined this; PR-4a admits it.)
        assert_eq!(
            inline_source("function f(x) { sink(x); } f(g());"),
            "function f(x){sink(x)}const a=g();sink(a);"
        );
    }

    #[test]
    fn inlines_member_argument_via_temp_used_twice() {
        // A non-simple member-access arg used by a parameter referenced
        // twice: the temp captures the read once, both uses read the temp.
        assert_eq!(
            inline_source("function f(p) { sink(p); use(p); } f(obj.x);"),
            "function f(p){sink(p);use(p)}const a=obj.x;sink(a);use(a);"
        );
    }

    #[test]
    fn temps_all_args_left_to_right_when_any_non_simple() {
        // Mixed simple + non-simple args: when ANY arg is non-simple, ALL
        // are hoisted in source order, preserving left-to-right evaluation.
        assert_eq!(
            inline_source("function f(p, q) { sink(p, q); } f(5, obj.x);"),
            "function f(p,q){sink(p,q)}const a=5;const b=obj.x;sink(a,b);"
        );
    }

    #[test]
    fn captures_non_simple_argument_in_value_position() {
        // PR-4a composes with PR-3: a result-used helper called with a
        // non-simple argument hoists the arg temp before the captured body.
        assert_eq!(
            inline_source("function f(p) { g(); return p + 1; } var x = f(obj.y);"),
            "function f(p){g();return p+1}const b=obj.y;g();const a=b+1;var x=a;"
        );
    }

    #[test]
    fn all_simple_args_still_substitute_directly_no_temp() {
        // The all-simple path is unchanged (no temps), so existing output is
        // preserved byte-for-byte — the no-churn guarantee.
        assert_eq!(
            inline_source("function f(p) { sink(p); use(p); } f(v);"),
            "function f(p){sink(p);use(p)}sink(v);use(v);"
        );
    }

    // ----- CLOC15 PR-4b: `if` without an early exit in the body -----

    #[test]
    fn inlines_if_with_unbraced_branches() {
        // An `if` whose branches are bare expression statements is spliced
        // verbatim with the parameter substituted in.
        assert_eq!(
            inline_source("function f(x) { if (x > 0) log(x); else warn(x); } f(v);"),
            "function f(x){if(x>0)log(x);else warn(x);}if(v>0)log(v);else warn(v);"
        );
    }

    #[test]
    fn inlines_if_with_block_branch() {
        // A block branch of expression statements is spliced as a block.
        assert_eq!(
            inline_source("function f(x) { if (x) { a(x); b(x); } } f(v);"),
            "function f(x){if(x){a(x);b(x)}}if(v){a(v);b(v)}"
        );
    }

    #[test]
    fn inlines_if_whose_test_reads_a_renamed_local() {
        // A local declared before the `if` is alpha-renamed; the `if` test
        // and branch that read it are renamed consistently.
        assert_eq!(
            inline_source("function f(x) { const t = x > 0; if (t) sink(x); } f(v);"),
            "function f(x){const t=x>0;if(t)sink(x);}const a=v>0;if(a)sink(v);"
        );
    }

    #[test]
    fn inlines_if_with_non_simple_argument() {
        // PR-4b composes with PR-4a: a non-simple argument is hoisted into a
        // temp once, and the `if` reads the temp.
        assert_eq!(
            inline_source("function f(p) { if (p) sink(p); } f(g());"),
            "function f(p){if(p)sink(p);}const a=g();if(a)sink(a);"
        );
    }

    #[test]
    fn does_not_inline_if_with_early_return() {
        // A `return` inside an `if` branch is a control-flow exit that a flat
        // splice would mis-scope — the whole helper is declined.
        assert_eq!(
            inline_source("function f(x) { if (x) return; sink(x); } f(v);"),
            "function f(x){if(x)return;sink(x)}f(v);"
        );
    }

    #[test]
    fn does_not_inline_if_with_nested_declaration() {
        // A `let` inside an `if` block introduces a block-scoped local the
        // name-based renamer cannot shadow-correctly — declined.
        assert_eq!(
            inline_source("function f(x) { if (x) { let t = 1; sink(t); } } f(v);"),
            "function f(x){if(x){let t=1;sink(t)}}f(v);"
        );
    }

    #[test]
    fn does_not_inline_nested_if() {
        // A nested `if` is not a bare expression statement, so the outer
        // branch fails the restriction — declined (kept for a later slice).
        assert_eq!(
            inline_source("function f(x) { if (x) { if (y) a(); } } f(v);"),
            "function f(x){if(x){if(y)a();}}f(v);"
        );
    }

    #[test]
    fn if_body_spliced_into_unbraced_slot_is_block_wrapped() {
        // Soundness guard against dangling-else capture: a helper whose body
        // ends in an else-less `if`, called from a braceless `if`-consequent
        // that has a caller `else`. The single-statement-slot splice wraps
        // the body in a block, so the caller's `else` stays bound to the
        // OUTER `if` (it must NOT capture the inner else-less `if`).
        assert_eq!(
            inline_source("function g(x) { if (x) a(x); } if (c) g(v); else other();"),
            "function g(x){if(x)a(x);}if(c){if(v)a(v);}else other();"
        );
    }

    #[test]
    fn does_not_inline_multi_use_void_helper() {
        // Two call sites — statement splicing of a multi-use body is a
        // separate, budgeted concern (a non-goal for PR-1). Declined.
        assert_eq!(
            inline_source("function f(x) { sink(x); } f(a); f(b);"),
            "function f(x){sink(x)}f(a);f(b);"
        );
    }

    #[test]
    fn does_not_inline_void_helper_with_param_local_name_collision() {
        // A `let` local sharing a parameter's name is illegal JS (a
        // faithful parser never emits it), but the name-based alpha-renamer
        // is not scope-aware, so we decline outright rather than risk a
        // mis-rename. Defense in depth against a non-conformant parser.
        assert_eq!(
            inline_source("function f(x) { const x = 1; sink(x); } f(a);"),
            "function f(x){const x=1;sink(x)}f(a);"
        );
    }

    #[test]
    fn does_not_inline_recursive_void_helper() {
        // `f` appears free in its own body (a declared name), so it is not
        // a candidate — recursion excluded, and the sole-external-use
        // invariant preserved.
        assert_eq!(
            inline_source("function f(x) { if (x) f(x); } g(f);"),
            "function f(x){if(x)f(x);}g(f);"
        );
    }

    // =====================================================================
    // CLOC15 PR-3: result-used helper captured into a hoisted temp
    // =====================================================================

    #[test]
    fn captures_result_used_helper_into_temp() {
        // The signature case: a multi-statement helper whose result is used
        // as a variable initializer. The body is hoisted (local `t`
        // alpha-renamed, param `a` substituted by `5`), the tail return
        // captured into a fresh temp, and the call replaced by that temp.
        assert_eq!(
            inline_source(
                "function compute(a) { const t = a+1; return t*2; } var x = compute(5);"
            ),
            "function compute(a){const t=a+1;return t*2}const c=5+1;const b=c*2;var x=b;"
        );
    }

    #[test]
    fn captures_with_free_global_side_effect() {
        // A side-effecting body statement (a free global call) is hoisted
        // and runs before the binding; the returned value is captured.
        assert_eq!(
            inline_source("function make(a) { setup(a); return build(a); } var r = make(x);"),
            "function make(a){setup(a);return build(a)}setup(x);const b=build(x);var r=b;"
        );
    }

    #[test]
    fn captures_into_let_binding() {
        // The captured-temp machinery works for a `let` binding too; the
        // tail `return a` (a parameter) is captured after substitution.
        assert_eq!(
            inline_source("function f(a) { g(); return a; } let x = f(7);"),
            "function f(a){g();return a}g();const b=7;let x=b;"
        );
    }

    #[test]
    fn does_not_capture_when_call_is_not_the_whole_initializer() {
        // The call is nested inside a larger initializer (`k + f(1)`), so
        // hoisting its body before the statement would run it before `k` is
        // read — declined.
        assert_eq!(
            inline_source("function f(p) { g(); return p; } var x = k + f(1);"),
            "function f(p){g();return p}var x=k+f(1);"
        );
    }

    #[test]
    fn does_not_capture_nested_call_argument_initializer() {
        // `f(2)` is an argument to `h(...)`, not the whole initializer —
        // declined (the call is not the initializer's top expression).
        assert_eq!(
            inline_source("function f(a) { g(); return a; } var x = h(f(2));"),
            "function f(a){g();return a}var x=h(f(2));"
        );
    }

    #[test]
    fn does_not_capture_multi_declarator() {
        // A second declarator (`y = 2`) is evaluated after the first; a
        // flat hoist of the body before the whole declaration would not
        // preserve that ordering — declined.
        assert_eq!(
            inline_source("function f(a) { g(); return a; } var x = f(1), y = 2;"),
            "function f(a){g();return a}var x=f(1),y=2;"
        );
    }

    #[test]
    fn does_not_capture_void_body_used_as_value() {
        // The helper has no tail-return value (a bare `return;`), so there
        // is nothing to capture; using it as an initializer yields
        // `undefined`, which this slice does not synthesize — declined.
        assert_eq!(
            inline_source("function f(a) { g(); return; } var x = f(1);"),
            "function f(a){g();return}var x=f(1);"
        );
    }

    // ---- CLOC15 PR-5: value capture in `return`-argument position --------

    #[test]
    fn captures_helper_in_return_position() {
        // The signature PR-5 case: `return f(x)` where `f` is a single-use
        // multi-statement helper. The body is hoisted into the caller and the
        // callee's tail `return a` (param substituted by `7`) becomes the
        // caller's own `return 7` — no temp, the value flows straight out.
        assert_eq!(
            inline_source("function f(a) { g(); return a; } function main() { return f(7); }"),
            "function f(a){g();return a}function main(){g();return 7};"
        );
    }

    #[test]
    fn captures_return_position_with_local_and_nonsimple_arg() {
        // Return-position capture composes with local alpha-renaming AND the
        // PR-4a per-argument temp: the non-simple argument `compute()` is
        // materialised once into a fresh temp before the body, the callee
        // local `t` is renamed program-fresh, and the tail expression becomes
        // the caller's return value.
        assert_eq!(
            inline_source(
                "function f(p) { const t = p+1; return t; } function main() { return f(compute()); }"
            ),
            "function f(p){const t=p+1;return t}function main(){const a=compute();const b=a+1;return b};"
        );
    }

    #[test]
    fn does_not_capture_return_under_short_circuit() {
        // `return cond && f(x)` — the call is not the *entire* return argument
        // (it is the right operand of `&&`), so hoisting the body before the
        // `return` would run it unconditionally, changing semantics. The
        // argument is a `LogicalExpression`, not a `CallExpression`, so it is
        // declined.
        assert_eq!(
            inline_source(
                "function f(a) { g(); return a; } function main() { return cond&&f(7); }"
            ),
            "function f(a){g();return a}function main(){return cond&&f(7)};"
        );
    }

    #[test]
    fn does_not_capture_return_conditional() {
        // `return c ? f(x) : y` — same reasoning: the return argument is a
        // `ConditionalExpression`, not a bare call, so the body must not be
        // hoisted out of the conditional. Declined.
        assert_eq!(
            inline_source(
                "function f(a) { g(); return a; } function main() { return c ? f(7) : 0; }"
            ),
            "function f(a){g();return a}function main(){return c?f(7):0};"
        );
    }

    #[test]
    fn does_not_capture_void_helper_in_return_position() {
        // A helper with no tail-return *value* (bare `return;`) is a void
        // candidate, not a valued one; `return f(x)` would return its
        // `undefined`, which the valued path does not synthesize — so the
        // call is left intact rather than mis-spliced.
        assert_eq!(
            inline_source("function f(a) { g(); return; } function main() { return f(1); }"),
            "function f(a){g();return}function main(){return f(1)};"
        );
    }

    // ===== CLOC15 PR-6 — assignment-target value capture (`g = f(x)`) =====

    #[test]
    fn captures_helper_in_assignment_position() {
        // The signature PR-6 case: `h = f(7)` where `f` is a single-use
        // multi-statement helper. The body is hoisted before the assignment
        // and the callee's tail `return a` (param substituted by `7`) becomes
        // the caller's own `h = 7` — no temp, the value flows straight into the
        // assignment target. Reachable only now that assignment-expression
        // statements parse (CLOC17).
        assert_eq!(
            inline_source("function f(a) { g(); return a; } var h; h = f(7);"),
            "function f(a){g();return a}var h;g();h=7;"
        );
    }

    #[test]
    fn captures_assignment_position_with_local_and_nonsimple_arg() {
        // Assignment-position capture composes with local alpha-renaming AND
        // the per-argument temp: the non-simple argument `compute()` is
        // materialised once into a fresh temp before the body, the callee local
        // `t` is renamed program-fresh, and the tail expression becomes the
        // assignment's right-hand side.
        assert_eq!(
            inline_source("function f(p) { const t = p + 1; return t; } var h; h = f(compute());"),
            "function f(p){const t=p+1;return t}var h;const a=compute();const b=a+1;h=b;"
        );
    }

    #[test]
    fn does_not_capture_compound_assignment() {
        // `h += f(7)` reads the OLD `h` *before* `f(7)` runs; hoisting the body
        // ahead (`g(); h += 7`) would read `h` *after* the body's effects. If
        // the body mutated `h` the two would differ, so compound assignment is
        // declined (the call is left intact).
        assert_eq!(
            inline_source("function f(a) { g(); return a; } var h = 0; h += f(7);"),
            "function f(a){g();return a}var h=0;h+=f(7);"
        );
    }

    #[test]
    fn does_not_capture_member_assignment_target() {
        // `o.k = f(7)` evaluates the reference to `o.k`'s base (`o`) *before*
        // `f(7)`; hoisting the body ahead could reorder observable effects (an
        // `o` getter, or the body mutating `o`). Member targets are declined.
        assert_eq!(
            inline_source("function f(a) { g(); return a; } var o = {}; o.k = f(7);"),
            "function f(a){g();return a}var o={};o.k=f(7);"
        );
    }

    #[test]
    fn does_not_capture_assignment_when_call_is_not_entire_rhs() {
        // `h = f(7) + 1` — the call is the left operand of `+`, not the entire
        // right-hand side (the RHS is a `BinaryExpression`, not a
        // `CallExpression`). Hoisting the body would be wrong, so it is
        // declined.
        assert_eq!(
            inline_source("function f(a) { g(); return a; } var h; h = f(7) + 1;"),
            "function f(a){g();return a}var h;h=f(7)+1;"
        );
    }

    #[test]
    fn does_not_capture_void_helper_in_assignment_position() {
        // A helper with no tail-return *value* (bare `return;`) is a void
        // candidate, not a valued one; `h = f(1)` would assign its `undefined`,
        // which the valued path does not synthesize — so the call is left
        // intact rather than mis-spliced.
        assert_eq!(
            inline_source("function f(a) { g(); return; } var h; h = f(1);"),
            "function f(a){g();return}var h;h=f(1);"
        );
    }

    // ===== CLOC18 — parameter-mutation materialization =====
    // A helper that REASSIGNS a parameter cannot substitute the parameter by
    // its argument expression. CLOC18 materialises each mutated parameter into
    // a fresh mutable local seeded from the argument (`let <fresh> = <arg>`) and
    // routes it through the rename map — exactly a real call's binding. These
    // were decline tests in 0.13.1 (#6272); they now inline correctly.

    #[test]
    fn materializes_reassigned_param() {
        // `function f(x){ x = x + 1; return x; }` returns 8 for `f(7)`. The
        // parameter `x` is materialised as `let b = 7`; `x = x + 1` becomes
        // `b = b + 1` (b == 8), captured into the temp `a`, so `var g = 8`.
        // (This was the #6272 miscompile `g = 7`; now correct.)
        // (The `function f` declaration is kept by the inline pass alone;
        // `remove-unused-vars` deletes it in the full SIMPLE pipeline, leaving
        // just `var g = 8;`.)
        assert_eq!(
            inline_source("function f(x) { x = x + 1; return x; } var g = f(7);"),
            "function f(x){x=x+1;return x}let b=7;b=b+1;const a=b;var g=a;"
        );
    }

    #[test]
    fn materializes_compound_reassigned_param() {
        // Compound assignment (`x += 1`) mutates the parameter too; same
        // materialisation, with the compound operator preserved.
        assert_eq!(
            inline_source("function f(x) { x += 1; return x; } var g = f(7);"),
            "function f(x){x+=1;return x}let b=7;b+=1;const a=b;var g=a;"
        );
    }

    #[test]
    fn materializes_nested_param_assignment() {
        // The mutation need not be a top-level statement: `y = (x = 5)` mutates
        // `x` inside a larger expression. The collector recurses every
        // expression position, so `x` is materialised (`let b = 7`) and the
        // nested `x = 5` becomes `b = 5`. (`y` is a callee local renamed to
        // `c`.) `f(7)` returns 5, so `var g = 5`.
        assert_eq!(
            inline_source("function f(x) { var y; y = (x = 5); return y; } var g = f(7);"),
            "function f(x){var y;y=x=5;return y}let b=7;var c;c=b=5;const a=c;var g=a;"
        );
    }

    #[test]
    fn materializes_only_the_mutated_param_in_a_mixed_helper() {
        // Two parameters, only `y` reassigned. A non-simple argument is present,
        // so both args materialise in source order: pure `x` into a `const`
        // (substituted), mutated `y` into a `let` (renamed). The capture temp
        // `a` is minted first, then arg temps `b` (x), `c` (y); `y = y + x`
        // becomes `c = c + b`. `f(p(), 1)` ⇒ `g = 1 + p()`.
        assert_eq!(
            inline_source("function f(x, y) { y = y + x; return y; } var g = f(p(), 1);"),
            "function f(x,y){y=y+x;return y}const b=p();let c=1;c=c+b;const a=c;var g=a;"
        );
    }

    #[test]
    fn materializes_param_with_side_effecting_argument_once() {
        // A side-effecting argument to a mutated parameter is evaluated exactly
        // once, into the `let`. `side()` must not be duplicated across the two
        // reads of `x` in `x + x`.
        assert_eq!(
            inline_source("function f(x) { x = x + 1; return x + x; } var g = f(side());"),
            "function f(x){x=x+1;return x+x}let b=side();b=b+1;const a=b+b;var g=a;"
        );
    }

    #[test]
    fn still_inlines_helper_that_assigns_a_free_variable() {
        // Assigning a FREE (non-parameter) variable is sound — it is not
        // substituted, so the spliced body writes the same binding the helper
        // did. Only parameter mutation is unsound, so this helper still inlines:
        // `glob = 7` is spliced and the tail value captured into a temp that
        // the declaration reads (`const a = 7; var g = a`). (The inline pass
        // runs alone here; constant-fold/propagate would later collapse the
        // temp to `var g = 7` in the full SIMPLE pipeline.)
        assert_eq!(
            inline_source("function f(x) { glob = x; return x; } var g = f(7);"),
            "function f(x){glob=x;return x}glob=7;const a=7;var g=a;"
        );
    }

    // ===== CLOC15 Open Q3 — `var` locals admitted (alpha-renamed) =====

    #[test]
    fn inlines_helper_with_var_local_value_capture() {
        // A `var` local in a valued helper. The bridge desugars `var t = x + 1`
        // into `var t; t = x + 1`; the local `t` is alpha-renamed to a fresh
        // `b`, params substituted, and the tail captured into the temp `a`. The
        // hoisted `var b` is inert because `b` appears nowhere else.
        assert_eq!(
            inline_source("function f(x) { var t = x + 1; return t * 2; } var g = f(7);"),
            "function f(x){var t=x+1;return t*2}var b=7+1;const a=b*2;var g=a;"
        );
    }

    #[test]
    fn inlines_helper_that_reassigns_a_var_local() {
        // Reassigning a *local* (not a parameter) is sound — the local is
        // renamed, so both the declaration and the `t = t + 1` assignment
        // target become the fresh name. (Contrast the parameter-reassignment
        // guard above, which declines mutating a *parameter*.)
        assert_eq!(
            inline_source("function f(x) { var t = x; t = t + 1; return t; } var g = f(7);"),
            "function f(x){var t=x;t=t+1;return t}var b=7;b=b+1;const a=b;var g=a;"
        );
    }

    #[test]
    fn var_local_is_renamed_away_from_a_colliding_caller_binding() {
        // The soundness crux: the caller has its OWN `t`. The helper's `var t`
        // must be renamed to a fresh name so the splice cannot capture or
        // clobber the caller's `t`. Here `f`'s `t` becomes `b`; the caller's
        // `t` (declared `var t = 9` → kept as `var t=9`) is untouched.
        assert_eq!(
            inline_source("var t = 9; function f(x) { var t = x; return t; } var g = f(5);"),
            "var t=9;function f(x){var t=x;return t}var b=5;const a=b;var g=a;"
        );
    }

    // -------------------------------------------------------------------
    // CLOC12.191 PR1 (security-review regression) — when the inliner splices a
    // function body that contains a NESTED function/arrow value with a default
    // parameter, the body-rewriters (`substitute` param→arg, `rename_in_expr`
    // alpha-rename) must rewrite that nested default too — else a name it reads
    // (an outer param being substituted, or an outer local being renamed) is
    // left dangling. The bridge does not yet produce defaults (PR2), so these
    // call the rewriters directly on hand-built AST.
    // -------------------------------------------------------------------
    #[test]
    fn substitute_reaches_nested_default_parameter() {
        use coding_adventures_javascript_ast::statement::ReturnStatement;
        use coding_adventures_javascript_ast::{
            AssignmentPattern, BlockStatement, Expression, FunctionExpression, FunctionParam,
            Identifier, NumericLiteral, Statement,
        };

        let id = |n: &str| Identifier {
            cv: None,
            name: n.to_string(),
        };
        // `function(a = b){ return a; }` — the default reads `b`.
        let mut expr = Expression::FunctionExpression(FunctionExpression {
            cv: None,
            id: None,
            params: vec![FunctionParam::AssignmentPattern(AssignmentPattern {
                cv: None,
                left: id("a"),
                right: Expression::Identifier(id("b")),
            })],
            body: BlockStatement {
                cv: None,
                body: vec![Statement::return_statement(ReturnStatement {
                    cv: None,
                    argument: Some(Expression::Identifier(id("a"))),
                })],
            },
            generator: false,
            is_async: false,
        });

        // Substituting `b -> 2` must reach the default `= b`.
        let mut map = HashMap::new();
        map.insert(
            "b".to_string(),
            Expression::NumericLiteral(NumericLiteral {
                cv: None,
                value: 2.0,
                raw: "2".to_string(),
            }),
        );
        substitute(&mut expr, &map);

        let Expression::FunctionExpression(fe) = &expr else {
            panic!("expected a function expression");
        };
        match fe.params[0].default_value() {
            Some(Expression::NumericLiteral(n)) => assert_eq!(n.value, 2.0),
            other => panic!("nested default `= b` was not substituted: {other:?}"),
        }
    }

    #[test]
    fn rename_in_expr_reaches_nested_arrow_default() {
        use coding_adventures_javascript_ast::{
            ArrowBody, ArrowFunctionExpression, AssignmentPattern, Expression, FunctionParam,
            Identifier,
        };

        let id = |n: &str| Identifier {
            cv: None,
            name: n.to_string(),
        };
        // `(x = loc) => x` — the default reads the outer local `loc`.
        let mut expr = Expression::ArrowFunctionExpression(ArrowFunctionExpression {
            cv: None,
            params: vec![FunctionParam::AssignmentPattern(AssignmentPattern {
                cv: None,
                left: id("x"),
                right: Expression::Identifier(id("loc")),
            })],
            body: ArrowBody::Expression(Box::new(Expression::Identifier(id("x")))),
            is_async: false,
        });

        // Alpha-renaming `loc -> _a` must reach the default `= loc`.
        let mut map = HashMap::new();
        map.insert("loc".to_string(), "_a".to_string());
        rename_in_expr(&mut expr, &map);

        let Expression::ArrowFunctionExpression(ae) = &expr else {
            panic!("expected an arrow expression");
        };
        match ae.params[0].default_value() {
            Some(Expression::Identifier(idref)) => assert_eq!(
                idref.name, "_a",
                "nested arrow default `= loc` was not alpha-renamed"
            ),
            other => panic!("expected an identifier default, got {other:?}"),
        }
    }
}
