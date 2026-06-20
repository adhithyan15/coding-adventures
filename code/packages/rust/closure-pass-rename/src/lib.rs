//! Variable renaming pass for the Closure Compiler clone.
//!
//! Per [CLOC06](../../../specs/CLOC06-pass-interface-contract.md)'s
//! canonical pass set. Replaces non-exported binding names (local
//! variables, internal function names, private class members) with
//! short identifiers — typically `a`, `b`, `c`, ... — to reduce
//! output size.
//!
//! # The two kinds of renaming (and why this pass handles both)
//!
//! 1. **Local renaming.** Within a single scope, rename
//!    `let user_name = ...` → `let a = ...`. Safe because the name
//!    is only visible inside the scope; no external code can refer
//!    to it.
//! 2. **Module-scoped renaming.** Across a module, rename
//!    non-exported top-level bindings the same way. Safe because
//!    nothing outside the module imports them.
//!
//! Externally-visible names (`export`s, public class methods,
//! property keys on objects passed to external code) **must not be
//! renamed** — that would break the public contract. The pass
//! reads the type sidecar's `external` attribute and the AST's
//! export markers to decide what's off-limits.
//!
//! # Where this pass sits in the canonical order
//!
//! CLOC06 §"Canonical pass set" pins:
//!
//! ```text
//! constant-fold → fold-control-flow → dce → inline → rename → ...
//! ```
//!
//! Rename runs **late** — after dead code is gone, after inlining
//! has decided which functions stay and which fold into call
//! sites. Renaming before DCE would waste work renaming bindings
//! that will get deleted; renaming before inline would make the
//! inlining heuristic harder (names tell the heuristic something
//! about user intent).
//!
//! In v1 `depends_on` is left empty rather than declaring
//! `["dce", "inline"]`. Reasoning: rename is *correct* with or
//! without those earlier passes — it just produces less
//! compression when they don't run. The scheduler shouldn't reject
//! a pipeline that only contains `rename`. Once we add a hard
//! dependency (e.g., a `freeze-externals` pass that rename
//! genuinely cannot run without), it goes here.
//!
//! # Why `OneShot` and not `FixedPoint`?
//!
//! Unlike constant-fold or DCE, rename doesn't open new
//! opportunities for itself. After one walk, every renameable
//! binding has been renamed; running again would just be a no-op.
//! `OneShot` tells the scheduler exactly that.
//!
//! # Scope (current slice)
//!
//! `RenamePass::run` is a real transform. It renames the **uniquely-bound
//! names of leaf functions** — their parameters and their function-body
//! `var`/`let`/`const` declarations — to short names (`a`, `b`, …), at the
//! declaration and every use site. A *leaf function* is a `function`
//! declaration whose body declares no nested function. It conservatively
//! never touches:
//!
//! - module/global top-level names (they may be externally visible);
//! - free globals (`console`, `window`, …);
//! - property names (the `.x` of a non-computed member, a non-computed
//!   object-literal key);
//! - any name declared **more than once** in the function (a parameter
//!   also `var`'d, two block-scoped `let x`, …) — its uses could belong to
//!   distinct bindings we can't disambiguate without full scope
//!   resolution, so it is skipped rather than mis-renamed.
//!
//! The "declared exactly once" rule is the safety boundary: with a single
//! declaration there is exactly one binding for the name, so every in-body
//! use resolves to it and rewriting the declaration plus all uses is a
//! provably-sound α-conversion (see the `rename_leaf_bindings` argument).
//! Broader renaming — nested non-leaf scopes, module-private top-level
//! names once an `external` marker exists — is future work on the same
//! walker.
//!
//! The implementation is self-contained (its own scope-aware walk over
//! the Phase-1 AST); it does not yet consume `closure-scope-analyzer`,
//! which the broader renamer will use for cross-scope resolution.

use std::collections::{HashMap, HashSet};

use coding_adventures_closure_pass_pipeline::{
    IterationPolicy, Pass, PassContext, PassError, PassOutput, PassStats,
};
use coding_adventures_javascript_ast::statement::TaggedStatement;
use coding_adventures_javascript_ast::{
    AssignmentTarget, BindingTarget, BlockStatement, Declaration, Expression, ForInit,
    FunctionDeclaration, FunctionParam, Program, ProgramItem, PropertyKey, Statement, VarKind,
    VariableDeclaration,
};

/// `Pass::depends_on` value. Empty in v1 — see crate-level docs
/// for why. Kept as a `const` so future tests/crates can refer to
/// it by reference rather than retyping.
const DEPS: &[&str] = &[];

/// Variable renaming pass — renames the uniquely-bound names (parameters
/// and local var/let/const) of leaf functions to short names. See
/// crate-level docs for the exact (conservative) scope.
///
/// Zero-sized type: no per-instance state. Pass-internal state
/// (the binding → short-name map, the next-id counter, the
/// "do-not-rename" set seeded from `export`s and sidecar
/// `external` attributes) lives in pass-local maps constructed
/// inside [`Pass::run`] per CLOC06 §"Pass-internal state."
#[derive(Debug, Default, Clone, Copy)]
pub struct RenamePass;

impl RenamePass {
    /// Zero-arg constructor for ergonomic
    /// `PassPipeline::add(Box::new(RenamePass::new()))` registration.
    pub fn new() -> Self {
        Self
    }
}

impl Pass for RenamePass {
    fn name(&self) -> &'static str {
        "rename"
    }

    fn depends_on(&self) -> &[&'static str] {
        // Empty in v1: rename is correct with or without earlier
        // passes — it just produces less compression without them.
        // Future hard dependencies (e.g., a `freeze-externals`
        // pass) would go here.
        DEPS
    }

    fn iteration_policy(&self) -> IterationPolicy {
        // After one walk, every renameable binding has been
        // renamed; a second walk would do nothing. Unlike
        // constant-fold and DCE which can cascade, rename doesn't
        // open new opportunities for itself.
        IterationPolicy::OneShot
    }

    fn cost(&self) -> u32 {
        // Two passes over the tree:
        //   1. Collect all bindings + figure out which are
        //      external (skip them).
        //   2. Walk again and substitute references.
        // Plus the name-allocator. Heavier than constant-fold's
        // single walk; comparable to DCE.
        3
    }

    fn run(&self, ctx: PassContext<'_>) -> Result<PassOutput, PassError> {
        // Scope: rename the uniquely-bound names of *leaf functions*
        // (function declarations whose body contains no nested function
        // declaration) — their parameters and their body's
        // `var`/`let`/`const` declarations — to short names,
        // conservatively. See [`rename_program`] / [`rename_leaf_bindings`]
        // and the crate-level docs for the full safety argument. Top-level
        // / module-scope names are never touched (they may be externally
        // visible).
        let mut program = ctx.program.clone();
        let mut nodes_touched: u32 = 1; // the program root
        let changed = rename_program(&mut program, &mut nodes_touched);

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
// Local-rename implementation (leaf-function parameters + locals)
// =========================================================================
//
// # What gets renamed, and why it is safe
//
// We rename the *uniquely-bound names* of a **leaf function** — a
// `FunctionDeclaration` whose body declares no nested function — to short
// names (`a`, `b`, `c`, …). A uniquely-bound name is one declared exactly
// once in the function: a parameter, or a body `var`/`let`/`const`. We
// never touch:
//
//   - module/global top-level names (they may be referenced by other
//     scripts / be the program's public surface);
//   - free globals (`console`, `window`, …);
//   - the `.name` side of a non-computed member access or the key of a
//     non-computed object literal (those are property names, not
//     bindings);
//   - any name declared MORE THAN ONCE in the function.
//
// **The safety argument** for a leaf function `f`:
//   1. `f` has no nested functions, so the only scope-introducers inside
//      it are `{}` blocks (which scope `let`/`const`). Nothing inside `f`
//      can capture or re-scope a name under a different binding except a
//      second declaration of that same name.
//   2. We rename a name only when it is declared EXACTLY ONCE in the
//      function (params + every var/let/const declarator are counted). A
//      single declaration means a single binding, so EVERY identifier use
//      of the name in the body (other than property names) resolves to it.
//      Rewriting all of them plus the declaration is a sound α-rename. A
//      name declared twice (a param also `var`'d, two block-scoped
//      `let x`) could have distinct bindings we cannot tell apart without
//      full scope resolution, so it is skipped.
//   3. The fresh name we pick avoids *every* identifier that appears
//      anywhere in the function, so it can neither collide with another
//      local nor accidentally capture a free global.
//
// Anything outside this provably-safe subset (non-leaf functions,
// multiply-declared names, module-scope bindings) is left untouched —
// `changed` stays `false` for it. Broader renaming (nested non-leaf
// scopes, module-private top-level names) is future work on the same
// walker.

/// Reserved words we must never emit as a fresh short name. Single
/// letters are all safe; only some two-letter combinations collide.
const RESERVED: &[&str] = &[
    "do", "if", "in", "of", "as", "is", "or", // 2-letter keywords/contextual
];

/// Walk the whole program and rename leaf-function parameters in place.
/// Returns whether anything changed. `nodes_touched` is bumped per
/// statement visited for the scheduler's cost accounting.
fn rename_program(program: &mut Program, nodes_touched: &mut u32) -> bool {
    let mut changed = false;
    for item in &mut program.body {
        if let ProgramItem::Declaration(Declaration::FunctionDeclaration(fd)) = item {
            changed |= process_function(fd, nodes_touched);
        } else if let ProgramItem::Statement(stmt) = item {
            changed |= process_stmt(stmt, nodes_touched);
        }
    }
    changed
}

/// Process one statement: recurse to find function declarations (which
/// may be nested in blocks / `if` / loops / `switch`), and rename the
/// parameters of any leaf function found.
fn process_stmt(stmt: &mut Statement, nodes_touched: &mut u32) -> bool {
    *nodes_touched += 1;
    match stmt {
        Statement::Declaration(Declaration::FunctionDeclaration(fd)) => {
            process_function(fd, nodes_touched)
        }
        Statement::Declaration(Declaration::VariableDeclaration(_)) => false,
        Statement::Tagged(t) => process_tagged(t, nodes_touched),
    }
}

/// Recurse into a tagged statement's child statements to find nested
/// function declarations. (No renaming happens here directly — that is
/// driven from [`process_function`].)
fn process_tagged(t: &mut TaggedStatement, nodes_touched: &mut u32) -> bool {
    let mut changed = false;
    match t {
        TaggedStatement::BlockStatement(b) => {
            for s in &mut b.body {
                changed |= process_stmt(s, nodes_touched);
            }
        }
        TaggedStatement::IfStatement(is) => {
            changed |= process_stmt(&mut is.consequent, nodes_touched);
            if let Some(alt) = &mut is.alternate {
                changed |= process_stmt(alt, nodes_touched);
            }
        }
        TaggedStatement::WhileStatement(ws) => {
            changed |= process_stmt(&mut ws.body, nodes_touched);
        }
        TaggedStatement::DoWhileStatement(ds) => {
            changed |= process_stmt(&mut ds.body, nodes_touched);
        }
        TaggedStatement::ForStatement(fs) => {
            changed |= process_stmt(&mut fs.body, nodes_touched);
        }
        TaggedStatement::LabeledStatement(ls) => {
            changed |= process_stmt(&mut ls.body, nodes_touched);
        }
        TaggedStatement::SwitchStatement(ss) => {
            for case in &mut ss.cases {
                for s in &mut case.consequent {
                    changed |= process_stmt(s, nodes_touched);
                }
            }
        }
        TaggedStatement::TryStatement(ts) => {
            // Drive leaf-function renaming into the three blocks so nested
            // functions inside try/catch/finally are processed. The catch
            // `param` is preserved.
            for s in &mut ts.block.body {
                changed |= process_stmt(s, nodes_touched);
            }
            if let Some(h) = &mut ts.handler {
                for s in &mut h.body.body {
                    changed |= process_stmt(s, nodes_touched);
                }
            }
            if let Some(f) = &mut ts.finalizer {
                for s in &mut f.body {
                    changed |= process_stmt(s, nodes_touched);
                }
            }
        }
        // No nested statements that could hold a function declaration.
        TaggedStatement::ExpressionStatement(_)
        | TaggedStatement::ReturnStatement(_)
        | TaggedStatement::ThrowStatement(_)
        | TaggedStatement::BreakStatement(_)
        | TaggedStatement::ContinueStatement(_)
        | TaggedStatement::EmptyStatement(_) | TaggedStatement::DebuggerStatement(_) => {}
    }
    changed
}

/// Recurse into a function's nested functions, then — if it is a leaf —
/// rename its renameable parameters.
fn process_function(fd: &mut FunctionDeclaration, nodes_touched: &mut u32) -> bool {
    // First handle any nested functions inside the body.
    let mut changed = false;
    for s in &mut fd.body.body {
        changed |= process_stmt(s, nodes_touched);
    }
    // A leaf function (no nested function declarations) is eligible for
    // parameter renaming.
    if !block_has_function(&fd.body) {
        changed |= rename_leaf_bindings(fd);
    }
    changed
}

/// True if `block` contains a function declaration anywhere (recursively).
///
/// This is the leaf-function test, and the whole rename soundness argument
/// rests on it: a "leaf" function has no nested function, so the only
/// scope-introducers inside it are `{}` blocks (which scope `let`/`const`).
///
/// **Phase-1 AST assumption — IMPORTANT.** This walks *statements* only. It
/// is sufficient TODAY because the Phase-1 AST has no function expressions,
/// arrow functions, `class` bodies, or `try`/`catch` — the only way to
/// introduce a nested function (and hence a closure scope) is a
/// `FunctionDeclaration` statement. If the AST ever grows a
/// function/arrow *expression* (which can appear anywhere an expression
/// can, e.g. a `var f = () => …`), this check would wrongly classify the
/// enclosing function as a leaf and the eligibility rule in
/// `rename_leaf_bindings` would become unsound (an inner closure could
/// capture/re-scope a name we rename). At that point this must also walk
/// expressions for nested functions. The exhaustive `match` in
/// `stmt_has_function` guards new *statement* kinds; new *expression*
/// kinds need this explicit reminder.
fn block_has_function(block: &BlockStatement) -> bool {
    block.body.iter().any(stmt_has_function)
}

fn stmt_has_function(stmt: &Statement) -> bool {
    match stmt {
        Statement::Declaration(Declaration::FunctionDeclaration(_)) => true,
        Statement::Declaration(Declaration::VariableDeclaration(_)) => false,
        Statement::Tagged(t) => match t {
            TaggedStatement::BlockStatement(b) => b.body.iter().any(stmt_has_function),
            TaggedStatement::IfStatement(is) => {
                stmt_has_function(&is.consequent)
                    || is.alternate.as_deref().is_some_and(stmt_has_function)
            }
            TaggedStatement::WhileStatement(ws) => stmt_has_function(&ws.body),
            TaggedStatement::DoWhileStatement(ds) => stmt_has_function(&ds.body),
            TaggedStatement::ForStatement(fs) => stmt_has_function(&fs.body),
            TaggedStatement::LabeledStatement(ls) => stmt_has_function(&ls.body),
            TaggedStatement::SwitchStatement(ss) => ss
                .cases
                .iter()
                .any(|c| c.consequent.iter().any(stmt_has_function)),
            TaggedStatement::TryStatement(ts) => {
                ts.block.body.iter().any(stmt_has_function)
                    || ts
                        .handler
                        .as_ref()
                        .is_some_and(|h| h.body.body.iter().any(stmt_has_function))
                    || ts
                        .finalizer
                        .as_ref()
                        .is_some_and(|f| f.body.iter().any(stmt_has_function))
            }
            // Statements that cannot (in the Phase-1 AST) contain a
            // function declaration. Listed EXHAUSTIVELY on purpose: when
            // the AST grows a new scope-introducing construct (a class
            // declaration, a `try`/`catch`, a function/arrow expression),
            // this match will fail to compile, forcing a maintainer to
            // decide whether it can hold a function — rather than this
            // silently classifying a non-leaf function as a leaf and
            // performing an unsound rename.
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

/// Rename the *uniquely-bound* names of a leaf function — its parameters
/// and its function-body `var`/`let`/`const` declarations — to short
/// names. Returns whether anything changed.
///
/// # Why "uniquely bound" is the safety boundary
///
/// A leaf function has no nested function, so the only scope-introducers
/// inside it are `{}` blocks (which scope `let`/`const`). We rename a name
/// only when it is *declared exactly once* in the whole function (counting
/// the parameter list and every `var`/`let`/`const` declarator). With a
/// single declaration there is exactly one binding for that name, so every
/// identifier use of it in the body (other than property names) resolves
/// to that binding — rewriting the declaration and all uses is a sound
/// α-rename. A name declared two or more times (a parameter also `var`'d,
/// two block-scoped `let x` in sibling blocks, …) could have *distinct*
/// bindings whose uses we cannot tell apart without full scope resolution,
/// so we conservatively skip it entirely.
fn rename_leaf_bindings(fd: &mut FunctionDeclaration) -> bool {
    // The declared names, in deterministic source order (params first,
    // then body declarators), WITH duplicates — each tagged with whether
    // its *scope* makes it safe to rename:
    //
    //   - a parameter or a `var` is function-scoped, so all in-body uses
    //     of its (uniquely-declared) name resolve to it → eligible;
    //   - a `let`/`const` is block-scoped. It is eligible ONLY when
    //     declared at the function-body top level (its block is then the
    //     whole body, so every in-body use resolves to it). A `let`/`const`
    //     nested inside an inner `{}`/`if`/loop/`switch`/`for`-init binds
    //     only within that inner block, while the same identifier used
    //     OUTSIDE the block resolves to an outer/global binding — renaming
    //     "every use" would corrupt that outer use, so it is NOT eligible.
    //
    // We still record duplicates so a name declared more than once (any
    // kind) is detected and skipped.
    let mut decl_order: Vec<(String, bool)> = Vec::new();
    for p in &fd.params {
        let FunctionParam::Identifier(id) = p;
        decl_order.push((id.name.clone(), true)); // params: function-scoped
    }
    collect_decl_occurrences(&fd.body, &mut decl_order, false);

    // Occurrence counts: a name declared exactly once is uniquely bound.
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for (name, _) in &decl_order {
        *counts.entry(name.as_str()).or_insert(0) += 1;
    }

    // Fresh names must avoid every identifier in the function (declarations,
    // uses, and property names), so a rename can neither collide with a
    // local nor capture a free global.
    let mut avoid: HashSet<String> = HashSet::new();
    for p in &fd.params {
        let FunctionParam::Identifier(id) = p;
        avoid.insert(id.name.clone());
    }
    collect_all_idents_block(&fd.body, &mut avoid);

    // Decide the renames, in declaration order for deterministic output.
    let mut map: HashMap<String, String> = HashMap::new();
    let mut gen = FreshNames::new();
    for (name, eligible) in &decl_order {
        if !eligible {
            continue; // block-scoped (nested let/const) — uses outside its
                      // block bind elsewhere; not safe to rename "all uses"
        }
        if map.contains_key(name) {
            continue; // already decided
        }
        if counts.get(name.as_str()).copied().unwrap_or(0) != 1 {
            continue; // declared more than once → ambiguous binding, skip
        }
        if name.len() <= 1 {
            continue; // already minimal
        }
        let fresh = gen.next(&avoid);
        avoid.insert(fresh.clone());
        map.insert(name.clone(), fresh);
    }

    if map.is_empty() {
        return false;
    }

    // Apply: rewrite the parameter declarations …
    for p in &mut fd.params {
        let FunctionParam::Identifier(id) = p;
        if let Some(new) = map.get(&id.name) {
            id.name = new.clone();
        }
    }
    // … and every declaration + use inside the body.
    rewrite_uses_block(&mut fd.body, &map);
    true
}

/// Generates `a`, `b`, …, `z`, `aa`, `ab`, … skipping reserved words and
/// (via the caller's `avoid` set) any name already in use.
struct FreshNames {
    counter: usize,
}

impl FreshNames {
    fn new() -> Self {
        FreshNames { counter: 0 }
    }

    fn next(&mut self, avoid: &HashSet<String>) -> String {
        loop {
            let name = encode(self.counter);
            self.counter += 1;
            if !RESERVED.contains(&name.as_str()) && !avoid.contains(&name) {
                return name;
            }
        }
    }
}

/// Bijective base-26 encoding into lowercase identifiers: 0→a, 25→z,
/// 26→aa, 27→ab, …
fn encode(mut n: usize) -> String {
    let mut s = Vec::new();
    loop {
        s.push(b'a' + (n % 26) as u8);
        if n < 26 {
            break;
        }
        n = n / 26 - 1;
    }
    s.reverse();
    String::from_utf8(s).expect("ascii")
}

// ---- name collection -----------------------------------------------------

/// Collect every `var`/`let`/`const` (and nested function-declaration)
/// name declared anywhere in `block`, IN SOURCE ORDER and WITH duplicates,
/// each tagged `(name, eligible)`. `eligible` is true when the declaration
/// is function-scoped (`var`, or any param at the call site) or a
/// top-level `let`/`const`; it is false for a `let`/`const` nested inside
/// an inner block, `if`, loop, `switch`, or `for`-init (block-scoped, so
/// uses outside that block bind elsewhere). `nested` is true once we have
/// descended below the function-body top level. We do NOT recurse into
/// nested function bodies (separate scopes). Duplicates let the caller
/// count occurrences (a name declared twice is ambiguous → skipped).
fn collect_decl_occurrences(block: &BlockStatement, out: &mut Vec<(String, bool)>, nested: bool) {
    for s in &block.body {
        collect_decl_occurrences_stmt(s, out, nested);
    }
}

fn collect_decl_occurrences_stmt(stmt: &Statement, out: &mut Vec<(String, bool)>, nested: bool) {
    match stmt {
        Statement::Declaration(Declaration::VariableDeclaration(vd)) => {
            push_var_occurrences(vd, out, nested);
        }
        Statement::Declaration(Declaration::FunctionDeclaration(fd)) => {
            // A nested function name (a leaf function has none, so this is
            // defensive). Mark ineligible — renaming a function name needs
            // its own care.
            out.push((fd.id.name.clone(), false));
            // Do NOT recurse into fd.body — separate scope.
        }
        Statement::Tagged(t) => match t {
            // Anything below this point is inside an inner block → nested.
            TaggedStatement::BlockStatement(b) => collect_decl_occurrences(b, out, true),
            TaggedStatement::IfStatement(is) => {
                collect_decl_occurrences_stmt(&is.consequent, out, true);
                if let Some(alt) = &is.alternate {
                    collect_decl_occurrences_stmt(alt, out, true);
                }
            }
            TaggedStatement::WhileStatement(ws) => {
                collect_decl_occurrences_stmt(&ws.body, out, true)
            }
            TaggedStatement::DoWhileStatement(ds) => {
                collect_decl_occurrences_stmt(&ds.body, out, true)
            }
            TaggedStatement::ForStatement(fs) => {
                // A `for`-init `let`/`const` is scoped to the loop (never the
                // whole body), so it is block-scoped → pass `nested = true`.
                if let Some(ForInit::VariableDeclaration(vd)) = &fs.init {
                    push_var_occurrences(vd, out, true);
                }
                collect_decl_occurrences_stmt(&fs.body, out, true);
            }
            // A label does not introduce a variable scope; keep `nested`.
            TaggedStatement::LabeledStatement(ls) => {
                collect_decl_occurrences_stmt(&ls.body, out, nested)
            }
            TaggedStatement::SwitchStatement(ss) => {
                for c in &ss.cases {
                    for s in &c.consequent {
                        collect_decl_occurrences_stmt(s, out, true);
                    }
                }
            }
            TaggedStatement::TryStatement(ts) => {
                // The catch `param` is a block-scoped binding (like a nested
                // `let`): record it as INELIGIBLE so it is never renamed
                // function-wide, and so a function-scoped `var` of the same
                // name becomes a duplicate occurrence → skipped (ambiguous),
                // which is the sound conservative outcome. Recurse into the
                // three blocks as nested scopes (their `var`s remain eligible
                // via `push_var_occurrences`; their `let`/`const` are
                // block-scoped → ineligible).
                collect_decl_occurrences(&ts.block, out, true);
                if let Some(h) = &ts.handler {
                    if let Some(param) = &h.param {
                        out.push((param.name.clone(), false));
                    }
                    collect_decl_occurrences(&h.body, out, true);
                }
                if let Some(f) = &ts.finalizer {
                    collect_decl_occurrences(f, out, true);
                }
            }
            // Statements that introduce no binding in the Phase-1 AST.
            // Exhaustive on purpose: a future binding-introducing
            // statement (a `class` declaration) must be handled here,
            // otherwise its name could shadow a local without our noticing
            // and we'd rename unsoundly. The compiler will flag the omission.
            TaggedStatement::ExpressionStatement(_)
            | TaggedStatement::ReturnStatement(_)
            | TaggedStatement::ThrowStatement(_)
            | TaggedStatement::BreakStatement(_)
            | TaggedStatement::ContinueStatement(_)
            | TaggedStatement::EmptyStatement(_) | TaggedStatement::DebuggerStatement(_) => {}
        },
    }
}

fn push_var_occurrences(vd: &VariableDeclaration, out: &mut Vec<(String, bool)>, nested: bool) {
    // `var` is function-scoped (eligible regardless of nesting); `let`/
    // `const` is block-scoped (eligible only at the function-body top
    // level, i.e. when not nested).
    let eligible = matches!(vd.kind, VarKind::Var) || !nested;
    for d in &vd.declarations {
        let BindingTarget::Identifier(id) = &d.id;
        out.push((id.name.clone(), eligible));
    }
}

/// Collect EVERY identifier name appearing anywhere in `block`
/// (declarations, uses, and property names) — used to pick collision-free
/// fresh names. Over-inclusive on purpose: avoiding a name that only
/// appears as a property key is harmless, and never missing a real use is
/// what keeps the rename sound.
fn collect_all_idents_block(block: &BlockStatement, out: &mut HashSet<String>) {
    for s in &block.body {
        collect_all_idents_stmt(s, out);
    }
}

fn collect_all_idents_stmt(stmt: &Statement, out: &mut HashSet<String>) {
    match stmt {
        Statement::Declaration(Declaration::VariableDeclaration(vd)) => {
            for d in &vd.declarations {
                let BindingTarget::Identifier(id) = &d.id;
                out.insert(id.name.clone());
                if let Some(init) = &d.init {
                    collect_all_idents_expr(init, out);
                }
            }
        }
        Statement::Declaration(Declaration::FunctionDeclaration(fd)) => {
            out.insert(fd.id.name.clone());
            for p in &fd.params {
                let FunctionParam::Identifier(id) = p;
                out.insert(id.name.clone());
            }
            collect_all_idents_block(&fd.body, out);
        }
        Statement::Tagged(t) => match t {
            TaggedStatement::ExpressionStatement(es) => {
                collect_all_idents_expr(&es.expression, out)
            }
            TaggedStatement::BlockStatement(b) => collect_all_idents_block(b, out),
            TaggedStatement::IfStatement(is) => {
                collect_all_idents_expr(&is.test, out);
                collect_all_idents_stmt(&is.consequent, out);
                if let Some(alt) = &is.alternate {
                    collect_all_idents_stmt(alt, out);
                }
            }
            TaggedStatement::WhileStatement(ws) => {
                collect_all_idents_expr(&ws.test, out);
                collect_all_idents_stmt(&ws.body, out);
            }
            TaggedStatement::DoWhileStatement(ds) => {
                collect_all_idents_expr(&ds.test, out);
                collect_all_idents_stmt(&ds.body, out);
            }
            TaggedStatement::ForStatement(fs) => {
                if let Some(init) = &fs.init {
                    match init {
                        ForInit::VariableDeclaration(vd) => {
                            for d in &vd.declarations {
                                let BindingTarget::Identifier(id) = &d.id;
                                out.insert(id.name.clone());
                                if let Some(i) = &d.init {
                                    collect_all_idents_expr(i, out);
                                }
                            }
                        }
                        ForInit::Expression(e) => collect_all_idents_expr(e, out),
                    }
                }
                if let Some(test) = &fs.test {
                    collect_all_idents_expr(test, out);
                }
                if let Some(update) = &fs.update {
                    collect_all_idents_expr(update, out);
                }
                collect_all_idents_stmt(&fs.body, out);
            }
            TaggedStatement::ReturnStatement(rs) => {
                if let Some(a) = &rs.argument {
                    collect_all_idents_expr(a, out);
                }
            }
            TaggedStatement::ThrowStatement(ts) => collect_all_idents_expr(&ts.argument, out),
            TaggedStatement::LabeledStatement(ls) => {
                out.insert(ls.label.name.clone());
                collect_all_idents_stmt(&ls.body, out);
            }
            TaggedStatement::SwitchStatement(ss) => {
                collect_all_idents_expr(&ss.discriminant, out);
                for c in &ss.cases {
                    if let Some(test) = &c.test {
                        collect_all_idents_expr(test, out);
                    }
                    for s in &c.consequent {
                        collect_all_idents_stmt(s, out);
                    }
                }
            }
            TaggedStatement::BreakStatement(b) => {
                if let Some(l) = &b.label {
                    out.insert(l.name.clone());
                }
            }
            TaggedStatement::ContinueStatement(c) => {
                if let Some(l) = &c.label {
                    out.insert(l.name.clone());
                }
            }
            TaggedStatement::TryStatement(ts) => {
                // Add the catch `param` to the avoid set so no renamed local
                // can collide with it (it is itself left unrenamed). Recurse
                // into the three blocks to collect every other identifier.
                collect_all_idents_block(&ts.block, out);
                if let Some(h) = &ts.handler {
                    if let Some(param) = &h.param {
                        out.insert(param.name.clone());
                    }
                    collect_all_idents_block(&h.body, out);
                }
                if let Some(f) = &ts.finalizer {
                    collect_all_idents_block(f, out);
                }
            }
            TaggedStatement::EmptyStatement(_) | TaggedStatement::DebuggerStatement(_) => {}
        },
    }
}

fn collect_all_idents_expr(expr: &Expression, out: &mut HashSet<String>) {
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
            collect_all_idents_expr(&be.left, out);
            collect_all_idents_expr(&be.right, out);
        }
        Expression::LogicalExpression(le) => {
            collect_all_idents_expr(&le.left, out);
            collect_all_idents_expr(&le.right, out);
        }
        Expression::UnaryExpression(ue) => collect_all_idents_expr(&ue.argument, out),
        Expression::AssignmentExpression(ae) => {
            match &ae.left {
                AssignmentTarget::Identifier(id) => {
                    out.insert(id.name.clone());
                }
                AssignmentTarget::MemberExpression(m) => {
                    collect_all_idents_member(&m.object, &m.property, m.computed, out)
                }
            }
            collect_all_idents_expr(&ae.right, out);
        }
        Expression::ConditionalExpression(ce) => {
            collect_all_idents_expr(&ce.test, out);
            collect_all_idents_expr(&ce.consequent, out);
            collect_all_idents_expr(&ce.alternate, out);
        }
        Expression::CallExpression(ce) => {
            collect_all_idents_expr(&ce.callee, out);
            for a in &ce.arguments {
                collect_all_idents_expr(a, out);
            }
        }
        Expression::MemberExpression(m) => {
            collect_all_idents_member(&m.object, &m.property, m.computed, out)
        }
        Expression::ArrayExpression(ae) => {
            for el in ae.elements.iter().flatten() {
                collect_all_idents_expr(el, out);
            }
        }
        Expression::ObjectExpression(oe) => {
            for prop in &oe.properties {
                match &prop.key {
                    PropertyKey::Identifier(id) => {
                        out.insert(id.name.clone());
                    }
                    PropertyKey::Expression(e) => collect_all_idents_expr(e, out),
                    _ => {}
                }
                collect_all_idents_expr(&prop.value, out);
            }
        }
    }
}

fn collect_all_idents_member(
    object: &Expression,
    property: &Expression,
    computed: bool,
    out: &mut HashSet<String>,
) {
    collect_all_idents_expr(object, out);
    if computed {
        collect_all_idents_expr(property, out);
    } else if let Expression::Identifier(id) = property {
        // Property name — record it for fresh-name avoidance, but it is
        // not a binding use (rewrite_uses skips it).
        out.insert(id.name.clone());
    }
}

// ---- use rewriting -------------------------------------------------------

fn rewrite_uses_block(block: &mut BlockStatement, map: &HashMap<String, String>) {
    for s in &mut block.body {
        rewrite_uses_stmt(s, map);
    }
}

fn rewrite_uses_stmt(stmt: &mut Statement, map: &HashMap<String, String>) {
    match stmt {
        Statement::Declaration(Declaration::VariableDeclaration(vd)) => {
            for d in &mut vd.declarations {
                // Rewrite the declared name itself (a `var`/`let`/`const`
                // declarator is a rename target now that locals are
                // renamed, not only parameters) …
                let BindingTarget::Identifier(id) = &mut d.id;
                if let Some(new) = map.get(&id.name) {
                    id.name = new.clone();
                }
                // … and its initializer (a use position).
                if let Some(init) = &mut d.init {
                    rewrite_uses_expr(init, map);
                }
            }
        }
        // A leaf function has no nested function declarations, so this arm
        // is unreachable in practice; leave nested functions untouched.
        Statement::Declaration(Declaration::FunctionDeclaration(_)) => {}
        Statement::Tagged(t) => rewrite_uses_tagged(t, map),
    }
}

fn rewrite_uses_tagged(t: &mut TaggedStatement, map: &HashMap<String, String>) {
    match t {
        TaggedStatement::ExpressionStatement(es) => rewrite_uses_expr(&mut es.expression, map),
        TaggedStatement::BlockStatement(b) => rewrite_uses_block(b, map),
        TaggedStatement::IfStatement(is) => {
            rewrite_uses_expr(&mut is.test, map);
            rewrite_uses_stmt(&mut is.consequent, map);
            if let Some(alt) = &mut is.alternate {
                rewrite_uses_stmt(alt, map);
            }
        }
        TaggedStatement::WhileStatement(ws) => {
            rewrite_uses_expr(&mut ws.test, map);
            rewrite_uses_stmt(&mut ws.body, map);
        }
        TaggedStatement::DoWhileStatement(ds) => {
            rewrite_uses_expr(&mut ds.test, map);
            rewrite_uses_stmt(&mut ds.body, map);
        }
        TaggedStatement::ForStatement(fs) => {
            if let Some(init) = &mut fs.init {
                match init {
                    ForInit::VariableDeclaration(vd) => {
                        for d in &mut vd.declarations {
                            // Rewrite the declared name (a `for (var x …)`
                            // binding is a rename target) and its init.
                            let BindingTarget::Identifier(id) = &mut d.id;
                            if let Some(new) = map.get(&id.name) {
                                id.name = new.clone();
                            }
                            if let Some(i) = &mut d.init {
                                rewrite_uses_expr(i, map);
                            }
                        }
                    }
                    ForInit::Expression(e) => rewrite_uses_expr(e, map),
                }
            }
            if let Some(test) = &mut fs.test {
                rewrite_uses_expr(test, map);
            }
            if let Some(update) = &mut fs.update {
                rewrite_uses_expr(update, map);
            }
            rewrite_uses_stmt(&mut fs.body, map);
        }
        TaggedStatement::ReturnStatement(rs) => {
            if let Some(a) = &mut rs.argument {
                rewrite_uses_expr(a, map);
            }
        }
        TaggedStatement::ThrowStatement(ts) => rewrite_uses_expr(&mut ts.argument, map),
        TaggedStatement::LabeledStatement(ls) => rewrite_uses_stmt(&mut ls.body, map),
        TaggedStatement::SwitchStatement(ss) => {
            rewrite_uses_expr(&mut ss.discriminant, map);
            for c in &mut ss.cases {
                if let Some(test) = &mut c.test {
                    rewrite_uses_expr(test, map);
                }
                for s in &mut c.consequent {
                    rewrite_uses_stmt(s, map);
                }
            }
        }
        TaggedStatement::TryStatement(ts) => {
            // Rewrite renamed-binding uses inside the three blocks. The catch
            // `param` is never in `map` (it is not collected as a renameable),
            // so its binding site and its uses are left untouched.
            rewrite_uses_block(&mut ts.block, map);
            if let Some(h) = &mut ts.handler {
                rewrite_uses_block(&mut h.body, map);
            }
            if let Some(f) = &mut ts.finalizer {
                rewrite_uses_block(f, map);
            }
        }
        // Labels (break/continue) live in a separate label namespace, not
        // the variable namespace — never rewritten.
        TaggedStatement::BreakStatement(_)
        | TaggedStatement::ContinueStatement(_)
        | TaggedStatement::EmptyStatement(_) | TaggedStatement::DebuggerStatement(_) => {}
    }
}

fn rewrite_uses_expr(expr: &mut Expression, map: &HashMap<String, String>) {
    match expr {
        Expression::Identifier(id) => {
            if let Some(new) = map.get(&id.name) {
                id.name = new.clone();
            }
        }
        Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::BigIntLiteral(_)
        | Expression::UndefinedLiteral(_) => {}
        Expression::BinaryExpression(be) => {
            rewrite_uses_expr(&mut be.left, map);
            rewrite_uses_expr(&mut be.right, map);
        }
        Expression::LogicalExpression(le) => {
            rewrite_uses_expr(&mut le.left, map);
            rewrite_uses_expr(&mut le.right, map);
        }
        Expression::UnaryExpression(ue) => rewrite_uses_expr(&mut ue.argument, map),
        Expression::AssignmentExpression(ae) => {
            match &mut ae.left {
                AssignmentTarget::Identifier(id) => {
                    if let Some(new) = map.get(&id.name) {
                        id.name = new.clone();
                    }
                }
                AssignmentTarget::MemberExpression(m) => {
                    rewrite_uses_member(&mut m.object, &mut m.property, m.computed, map)
                }
            }
            rewrite_uses_expr(&mut ae.right, map);
        }
        Expression::ConditionalExpression(ce) => {
            rewrite_uses_expr(&mut ce.test, map);
            rewrite_uses_expr(&mut ce.consequent, map);
            rewrite_uses_expr(&mut ce.alternate, map);
        }
        Expression::CallExpression(ce) => {
            rewrite_uses_expr(&mut ce.callee, map);
            for a in &mut ce.arguments {
                rewrite_uses_expr(a, map);
            }
        }
        Expression::MemberExpression(m) => {
            rewrite_uses_member(&mut m.object, &mut m.property, m.computed, map)
        }
        Expression::ArrayExpression(ae) => {
            for el in ae.elements.iter_mut().flatten() {
                rewrite_uses_expr(el, map);
            }
        }
        Expression::ObjectExpression(oe) => {
            for prop in &mut oe.properties {
                // Only a *computed* key `[expr]` is a use position; a
                // plain identifier / string / number key is a property
                // name, never rewritten.
                if prop.computed {
                    if let PropertyKey::Expression(e) = &mut prop.key {
                        rewrite_uses_expr(e, map);
                    }
                }
                rewrite_uses_expr(&mut prop.value, map);
            }
        }
    }
}

fn rewrite_uses_member(
    object: &mut Expression,
    property: &mut Expression,
    computed: bool,
    map: &HashMap<String, String>,
) {
    rewrite_uses_expr(object, map);
    // The `.name` of a non-computed member access is a property name, NOT
    // a binding — only rewrite the property when it is computed (`o[x]`).
    if computed {
        rewrite_uses_expr(property, map);
    }
}

#[cfg(test)]
mod tests {
    //! These tests pin the public contract (name, policy, cost,
    //! deps), the `PassPipeline` integration, and the renaming
    //! behavior itself — driven end-to-end through the real
    //! source → bridge → rename → emit roundtrip so the tests
    //! exercise the exact AST shape the parser produces.
    use super::*;
    use coding_adventures_closure_emitter::{emit, EmitOptions};
    use coding_adventures_closure_pass_pipeline::{PassContext, PassPipeline, PipelineOutput};
    use coding_adventures_correlation_vector::CVLog;
    use coding_adventures_javascript_ast::{Program, SourceType};
    use coding_adventures_javascript_parser::{bridge, parse_javascript_typed};
    use coding_adventures_javascript_tokens::EsVersion;
    use coding_adventures_type_sidecar::Sidecar;

    fn program() -> Program {
        Program::new("prog.1".to_string(), EsVersion::Es2025, SourceType::Module)
    }

    /// Parse `src`, bridge it to a typed `Program`, run `RenamePass`,
    /// and emit the result as minified JS — the same chain closurec's
    /// SIMPLE level uses. Returns the emitted string.
    fn rename_source(src: &str) -> String {
        let es = EsVersion::Es2025;
        let node = parse_javascript_typed(src, es).expect("parse");
        let prog = bridge::grammar_to_program(&node, es).expect("bridge");

        let pass = RenamePass::new();
        let sidecar = Sidecar::new();
        let mut cv = CVLog::new(false);
        let out = pass
            .run(PassContext {
                program: &prog,
                sidecar: &sidecar,
                cv: &mut cv,
            })
            .expect("rename");

        let mut cv2 = CVLog::new(false);
        let opts = EmitOptions {
            source_map: false,
            ..Default::default()
        };
        emit(&out.program, &sidecar, &mut cv2, &opts)
            .expect("emit")
            .code
    }

    #[test]
    fn name_is_rename() {
        // The name is the public handle: `--disable=rename`,
        // `out.stats["rename"]`, etc. Drift here is a breaking
        // change.
        assert_eq!(RenamePass::new().name(), "rename");
    }

    #[test]
    fn iteration_policy_is_one_shot() {
        // Unlike fold/DCE, one pass converges. OneShot tells the
        // scheduler not to bother re-running.
        assert_eq!(
            RenamePass::new().iteration_policy(),
            IterationPolicy::OneShot
        );
    }

    #[test]
    fn cost_is_three_pass_units() {
        // Two-pass walk + name allocator. Heavier than constant-
        // fold's single walk.
        assert_eq!(RenamePass::new().cost(), 3);
    }

    #[test]
    fn depends_on_is_empty_in_v1() {
        // Empty in v1: rename is correct standalone. See
        // crate-level docs.
        let p = RenamePass::new();
        assert!(p.depends_on().is_empty());
    }

    #[test]
    fn invalidates_empty_in_v1() {
        // CLOC06 Open Question 1: invalidates() is informational
        // only in v0.1.0. Empty avoids over-committing.
        assert!(RenamePass::new().invalidates().is_empty());
    }

    #[test]
    fn run_on_empty_program_returns_unchanged_identity() {
        // Identity check: same CV, version, source_type; no
        // contributions, no diagnostics, changed=false,
        // nodes_touched=1.
        let pass = RenamePass::new();
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
    fn pipeline_runs_rename_as_solo_pass() {
        // Rename in a pipeline alone: should produce
        // execution_order=["rename"], stats["rename"], and — since
        // rename is OneShot, NOT FixedPoint — no "not yet iterated"
        // diagnostic from the scheduler.
        let mut pipeline = PassPipeline::new();
        pipeline.add(Box::new(RenamePass::new()));

        let mut cv = CVLog::new(true);
        let out: PipelineOutput = pipeline
            .run(program(), &Sidecar::new(), &mut cv)
            .expect("pipeline should run cleanly");

        assert_eq!(out.execution_order, vec!["rename".to_string()]);
        assert_eq!(out.stats["rename"].nodes_touched, 1);
        // OneShot ≠ FixedPoint: no fixed-point-deferred note.
        assert!(
            !out.diagnostics
                .iter()
                .any(|d| d.group.0 == "pipeline.fixed-point-not-yet-iterated"),
            "OneShot should NOT trigger the FixedPoint note; got {:?}",
            out.diagnostics
        );
    }

    #[test]
    fn pass_is_default_and_clone() {
        // ZST + Default + Copy + Clone keeps registration
        // ergonomic and avoids ownership thinking at call sites.
        let _a: RenamePass = Default::default();
        let _b: RenamePass = RenamePass::new();
        let _c = _b;
        let _d = _c.clone();
    }

    // =====================================================================
    // Renaming behavior (source → bridge → rename → emit)
    // =====================================================================

    // NOTE on whitespace: these assert against the raw
    // `closure-emitter` output (binary operators get spaces, function
    // declarations get a trailing `;`). closurec's pipeline emits the
    // same; the WHITESPACE_ONLY-style tightening is a separate concern.
    // What matters here is WHICH identifiers were renamed.

    #[test]
    fn renames_leaf_function_param() {
        // The signature case: a leaf function's parameter is renamed to
        // a short name, at both the declaration and every use site.
        assert_eq!(
            rename_source("function f(longName) { return longName + 1; }"),
            "function f(a){return a + 1};"
        );
    }

    // ---- catch-param soundness (CLOC19) -----------------------

    #[test]
    fn rewrites_param_use_inside_catch_body() {
        // The param `longName` is used inside the catch handler. Renaming
        // must reach into the catch body and rewrite the use, while the
        // catch binding `e` is preserved verbatim (catch params are never
        // in the local-rename set).
        assert_eq!(
            rename_source(
                "function f(longName) { try { risky(); } catch (e) { return longName; } }"
            ),
            "function f(a){try{risky()}catch(e){return a}};"
        );
    }

    #[test]
    fn does_not_rename_catch_param_itself() {
        // A long catch-binding name is NOT shortened — catch params are
        // bindings the renamer treats as reserved, not locals to compress.
        assert_eq!(
            rename_source(
                "function f(p) { try { risky(); } catch (longError) { report(longError); } }"
            ),
            // `p` is already 1 char (no byte savings), so it stays; the
            // catch binding `longError` is reserved and stays verbatim too.
            "function f(p){try{risky()}catch(longError){report(longError)}};"
        );
    }

    #[test]
    fn fresh_name_avoids_colliding_with_catch_param() {
        // The killer case: the catch param is literally `a`, the name the
        // allocator would otherwise hand to the function's own param. The
        // soundness guard adds the catch param to the avoid set, so
        // `longName` must become `b` (NOT `a`) — otherwise the renamed
        // param would alias the caught value and miscompile `use(a, …)`.
        assert_eq!(
            rename_source(
                "function f(longName) { try { risky(); } catch (a) { use(a, longName); } }"
            ),
            "function f(b){try{risky()}catch(a){use(a,b)}};"
        );
    }

    #[test]
    fn renames_multiple_params_distinctly() {
        assert_eq!(
            rename_source("function f(first, second) { return first * second; }"),
            "function f(a,b){return a * b};"
        );
    }

    #[test]
    fn does_not_rename_property_access() {
        // `obj.longName` — `longName` here is a property name, NOT a use
        // of the parameter, so it must NOT be renamed. The parameter
        // `obj` IS renamed; the `.longName` member stays.
        assert_eq!(
            rename_source("function f(obj) { return obj.longName; }"),
            "function f(a){return a.longName};"
        );
    }

    #[test]
    fn does_not_rename_object_literal_key() {
        // A non-computed object key is a property name, never renamed.
        // The param `val` is renamed; the key `keyName` stays.
        assert_eq!(
            rename_source("function f(val) { return { keyName: val }; }"),
            "function f(a){return {keyName:a}};"
        );
    }

    #[test]
    fn renames_computed_member_and_key() {
        // A computed member `obj[idx]` IS a use position — the param
        // `idx` must be renamed there too.
        assert_eq!(
            rename_source("function f(obj, idx) { return obj[idx]; }"),
            "function f(a,b){return a[b]};"
        );
    }

    #[test]
    fn skips_param_redeclared_as_local() {
        // `p` is also declared `var p` — same function-scope binding.
        // We conservatively skip renaming it (would need to rewrite the
        // declaration too).
        assert_eq!(
            rename_source("function f(p) { var p = 2; return p; }"),
            "function f(p){var p=2;return p};"
        );
    }

    #[test]
    fn does_not_rename_non_leaf_function_params() {
        // `outer` has a nested function `inner`, so it is NOT a leaf —
        // its parameter `param` is left alone (renaming non-leaf
        // functions is future work). `inner` IS a leaf, so ITS param
        // `q` is renamed.
        assert_eq!(
            rename_source(
                "function outer(param) { function inner(innerArg) { return innerArg; } return inner(param); }"
            ),
            "function outer(param){function inner(a){return a};return inner(param)};"
        );
    }

    #[test]
    fn does_not_rename_top_level_or_globals() {
        // Top-level function name `f` and free global `console` are
        // never renamed; only the parameter `message` is.
        assert_eq!(
            rename_source("function f(message) { console.log(message); }"),
            "function f(a){console.log(a)};"
        );
    }

    #[test]
    fn fresh_name_avoids_referenced_global() {
        // The body references a free global `a`. The parameter must NOT
        // be renamed to `a` (that would capture the global); it gets the
        // next free name `b`.
        assert_eq!(
            rename_source("function f(longName) { return a + longName; }"),
            "function f(b){return a + b};"
        );
    }

    #[test]
    fn renames_param_used_in_nested_block() {
        // Uses inside a nested `if` block are rewritten. (Bare
        // assignment statements like `x = …;` are not in the Phase-1
        // grammar, so this exercises nested-block uses via `return`.)
        assert_eq!(
            rename_source(
                "function f(counter) { if (counter > 0) { return counter; } return 0; }"
            ),
            "function f(a){if(a > 0){return a}return 0};"
        );
    }

    #[test]
    fn single_char_param_left_alone() {
        // A one-character parameter is already minimal; renaming can't
        // shrink it, so nothing changes.
        assert_eq!(
            rename_source("function f(x) { return x + 1; }"),
            "function f(x){return x + 1};"
        );
    }

    // ----- local variable renaming (var/let/const) -----

    #[test]
    fn renames_local_var() {
        // A function-body `var` declared once is uniquely bound — its
        // declaration and its use are both rewritten.
        assert_eq!(
            rename_source("function f() { var counter = 0; return counter + 1; }"),
            "function f(){var a=0;return a + 1};"
        );
    }

    #[test]
    fn renames_local_const_and_let() {
        assert_eq!(
            rename_source("function f() { const total = 1; let partial = 2; return total + partial; }"),
            "function f(){const a=1;let b=2;return a + b};"
        );
    }

    #[test]
    fn renames_param_and_local_together() {
        // Parameter and local are both uniquely bound; both shortened, in
        // declaration order (param first).
        assert_eq!(
            rename_source("function f(input) { var doubled = input * 2; return doubled; }"),
            "function f(a){var b=a * 2;return b};"
        );
    }

    #[test]
    fn skips_name_declared_twice() {
        // `total` is declared as BOTH a function-scope `var` and a
        // block-scope `let` — two distinct bindings. Renaming "every use
        // of total" would conflate them, so we skip the name entirely
        // (the safety boundary of the uniquely-bound rule). `keep` is
        // declared once, so it IS renamed.
        assert_eq!(
            rename_source(
                "function f() { var total = 1; { let total = 2; sink(total); } var keep = total; return keep; }"
            ),
            "function f(){var total=1;{let total=2;sink(total)}var a=total;return a};"
        );
    }

    #[test]
    fn renames_for_loop_var() {
        // A `for (var i …)` binding is uniquely bound; the declaration,
        // the test, and body uses are all rewritten. (Single-char `i`
        // would be skipped, so use a longer name. The update slot is
        // omitted — the Phase-1 grammar doesn't parse a bare assignment
        // expression there.)
        assert_eq!(
            rename_source(
                "function f() { for (var index = 0; index < 3; ) { sink(index); } }"
            ),
            "function f(){for(var a=0;a < 3;){sink(a)}};"
        );
    }

    #[test]
    fn skips_nested_block_scoped_let_used_outside_its_block() {
        // SOUNDNESS regression (caught in security review): `shadowed` is
        // a single `let` inside an `if` block, but it is ALSO used at the
        // function-body top level (`use(shadowed)`) where it resolves to
        // an OUTER/global `shadowed`. The block-scoped `let` is declared
        // exactly once, but renaming "every use" would corrupt the outer
        // use — so a nested `let`/`const` is NOT eligible. The parameter
        // `cond` (function-scoped) is still renamed.
        assert_eq!(
            rename_source(
                "function f(cond) { use(shadowed); if (cond) { let shadowed = 1; sink(shadowed); } }"
            ),
            "function f(a){use(shadowed);if(a){let shadowed=1;sink(shadowed)}};"
        );
    }

    #[test]
    fn renames_nested_var_because_function_scoped() {
        // A `var` is function-scoped even when written inside a block, so
        // a use outside the block (`return hoisted`) resolves to it — it
        // IS eligible (contrast the `let` case above). `cond` → a,
        // `hoisted` → b.
        assert_eq!(
            rename_source(
                "function f(cond) { if (cond) { var hoisted = 1; } return hoisted; }"
            ),
            "function f(a){if(a){var b=1;}return b};"
        );
    }

    #[test]
    fn whitespace_only_keeps_local_names() {
        // Companion — under WHITESPACE_ONLY the local keeps its full name.
        let es = EsVersion::Es2025;
        let src = "function f() { var counter = 0; return counter + 1; }";
        let node = parse_javascript_typed(src, es).expect("parse");
        let prog = bridge::grammar_to_program(&node, es).expect("bridge");
        // (Emit the BRIDGED program directly without the rename pass to
        // mirror the WHITESPACE_ONLY contrast.)
        let mut cv = CVLog::new(false);
        let opts = EmitOptions {
            source_map: false,
            ..Default::default()
        };
        let out = emit(&prog, &Sidecar::new(), &mut cv, &opts).expect("emit").code;
        assert_eq!(out, "function f(){var counter=0;return counter + 1};");
    }
}
