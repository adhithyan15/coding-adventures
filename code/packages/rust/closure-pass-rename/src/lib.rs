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
//! `RenamePass::run` is a real transform. v1 renames the
//! **parameters of leaf functions** — `function` declarations whose
//! body declares no nested function — to short names (`a`, `b`, …),
//! at the declaration and every use site. It conservatively never
//! touches:
//!
//! - module/global top-level names (they may be externally visible);
//! - free globals (`console`, `window`, …);
//! - property names (the `.x` of a non-computed member, a non-computed
//!   object-literal key);
//! - a parameter that is also declared `var`/`let`/`const` in the body
//!   (re-declared / block-shadowed — skipped rather than mis-renamed).
//!
//! Within that subset the rename is a provably-sound α-conversion (see
//! the `rename_leaf_params` safety argument). Broader renaming — locals,
//! nested non-leaf scopes, module-private top-level names once an
//! `external` marker exists — is future work built on the same walker.
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
    FunctionDeclaration, FunctionParam, Program, ProgramItem, PropertyKey, Statement,
    VariableDeclaration,
};

/// `Pass::depends_on` value. Empty in v1 — see crate-level docs
/// for why. Kept as a `const` so future tests/crates can refer to
/// it by reference rather than retyping.
const DEPS: &[&str] = &[];

/// Variable renaming pass — renames leaf-function parameters to short
/// names. See crate-level docs for the exact (conservative) scope.
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
        // v1 scope: rename the parameters of *leaf functions* (function
        // declarations whose body contains no nested function
        // declarations) to short names, conservatively. See
        // [`rename_program`] and the crate-level docs for the full
        // safety argument. Top-level / module-scope names are never
        // touched (they may be externally visible).
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
// Local-rename implementation (v1: leaf-function parameters)
// =========================================================================
//
// # What gets renamed, and why it is safe
//
// We rename the *parameters* of a **leaf function** — a
// `FunctionDeclaration` whose body declares no nested function — to short
// names (`a`, `b`, `c`, …). We never touch:
//
//   - module/global top-level names (they may be referenced by other
//     scripts / be the program's public surface);
//   - free globals (`console`, `window`, …);
//   - the `.name` side of a non-computed member access or the key of a
//     non-computed object literal (those are property names, not
//     bindings).
//
// **The safety argument** for a leaf function `f`:
//   1. `f` has no nested functions, so nothing inside `f` can capture or
//      re-scope `f`'s parameters under a different name.
//   2. We rename a parameter `p` only when `p` is NOT also declared as a
//      `var`/`let`/`const` anywhere in the body. With that excluded,
//      `p`'s *only* binding in the function is the parameter, so EVERY
//      identifier use of `p` in the body (other than property names)
//      resolves to that parameter. Rewriting all of them plus the
//      parameter declaration is therefore a sound α-rename.
//   3. The fresh name we pick avoids *every* identifier that appears
//      anywhere in the function, so it can neither collide with another
//      local nor accidentally capture a free global.
//
// Anything outside this provably-safe subset (non-leaf functions,
// shadowed/redeclared parameters, module-scope bindings) is left
// untouched — `changed` stays `false` for it. Broader renaming (locals,
// nested scopes) is future work built on the same walker.

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
        // No nested statements that could hold a function declaration.
        TaggedStatement::ExpressionStatement(_)
        | TaggedStatement::ReturnStatement(_)
        | TaggedStatement::ThrowStatement(_)
        | TaggedStatement::BreakStatement(_)
        | TaggedStatement::ContinueStatement(_)
        | TaggedStatement::EmptyStatement(_) => {}
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
        changed |= rename_leaf_params(fd);
    }
    changed
}

/// True if `block` contains a function declaration anywhere (recursively).
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
            TaggedStatement::ForStatement(fs) => stmt_has_function(&fs.body),
            TaggedStatement::LabeledStatement(ls) => stmt_has_function(&ls.body),
            TaggedStatement::SwitchStatement(ss) => ss
                .cases
                .iter()
                .any(|c| c.consequent.iter().any(stmt_has_function)),
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
            | TaggedStatement::EmptyStatement(_) => false,
        },
    }
}

/// Rename the parameters of a leaf function. Returns whether anything
/// changed.
fn rename_leaf_params(fd: &mut FunctionDeclaration) -> bool {
    if fd.params.is_empty() {
        return false;
    }

    // Names declared as var/let/const anywhere in the body. A parameter
    // sharing such a name is either the same binding (`var p`) or
    // block-shadowed (`let p`); either way we conservatively skip it.
    let mut decl_names: HashSet<String> = HashSet::new();
    collect_decl_names(&fd.body, &mut decl_names);

    // Every identifier that appears anywhere in the function (params +
    // body, including property names). A fresh name avoiding all of
    // these cannot collide with a local or capture a free global.
    let mut avoid: HashSet<String> = HashSet::new();
    for p in &fd.params {
        let FunctionParam::Identifier(id) = p;
        avoid.insert(id.name.clone());
    }
    collect_all_idents_block(&fd.body, &mut avoid);

    // Decide the renames.
    let mut map: HashMap<String, String> = HashMap::new();
    let mut gen = FreshNames::new();
    for p in &fd.params {
        let FunctionParam::Identifier(id) = p;
        if decl_names.contains(&id.name) {
            continue; // redeclared / shadowed — skip
        }
        if id.name.len() <= 1 {
            continue; // already minimal; renaming can't help
        }
        let fresh = gen.next(&avoid);
        avoid.insert(fresh.clone());
        map.insert(id.name.clone(), fresh);
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
    // … and every use inside the body.
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

/// Collect `var`/`let`/`const` and nested function-declaration names
/// declared anywhere in `block` (recursing through nested blocks/control
/// flow but NOT into nested function bodies — those are separate scopes).
fn collect_decl_names(block: &BlockStatement, out: &mut HashSet<String>) {
    for s in &block.body {
        collect_decl_names_stmt(s, out);
    }
}

fn collect_decl_names_stmt(stmt: &Statement, out: &mut HashSet<String>) {
    match stmt {
        Statement::Declaration(Declaration::VariableDeclaration(vd)) => {
            insert_var_names(vd, out);
        }
        Statement::Declaration(Declaration::FunctionDeclaration(fd)) => {
            out.insert(fd.id.name.clone());
            // Do NOT recurse into fd.body — separate scope.
        }
        Statement::Tagged(t) => match t {
            TaggedStatement::BlockStatement(b) => collect_decl_names(b, out),
            TaggedStatement::IfStatement(is) => {
                collect_decl_names_stmt(&is.consequent, out);
                if let Some(alt) = &is.alternate {
                    collect_decl_names_stmt(alt, out);
                }
            }
            TaggedStatement::WhileStatement(ws) => collect_decl_names_stmt(&ws.body, out),
            TaggedStatement::ForStatement(fs) => {
                if let Some(ForInit::VariableDeclaration(vd)) = &fs.init {
                    insert_var_names(vd, out);
                }
                collect_decl_names_stmt(&fs.body, out);
            }
            TaggedStatement::LabeledStatement(ls) => collect_decl_names_stmt(&ls.body, out),
            TaggedStatement::SwitchStatement(ss) => {
                for c in &ss.cases {
                    for s in &c.consequent {
                        collect_decl_names_stmt(s, out);
                    }
                }
            }
            // Statements that introduce no binding in the Phase-1 AST.
            // Exhaustive on purpose: a future binding-introducing
            // statement (a `try`/`catch`, a `class` declaration) must be
            // handled here, otherwise its name could shadow a parameter
            // without our noticing and we'd rename the parameter
            // unsoundly. The compiler will flag the omission.
            TaggedStatement::ExpressionStatement(_)
            | TaggedStatement::ReturnStatement(_)
            | TaggedStatement::ThrowStatement(_)
            | TaggedStatement::BreakStatement(_)
            | TaggedStatement::ContinueStatement(_)
            | TaggedStatement::EmptyStatement(_) => {}
        },
    }
}

fn insert_var_names(vd: &VariableDeclaration, out: &mut HashSet<String>) {
    for d in &vd.declarations {
        let BindingTarget::Identifier(id) = &d.id;
        out.insert(id.name.clone());
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
            TaggedStatement::EmptyStatement(_) => {}
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
                // A declarator id with a name in `map` cannot happen here:
                // we only map parameter names that are NOT redeclared as
                // var/let/const (see rename_leaf_params). So we rewrite
                // only the initializer (a use position).
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
        TaggedStatement::ForStatement(fs) => {
            if let Some(init) = &mut fs.init {
                match init {
                    ForInit::VariableDeclaration(vd) => {
                        for d in &mut vd.declarations {
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
        // Labels (break/continue) live in a separate label namespace, not
        // the variable namespace — never rewritten.
        TaggedStatement::BreakStatement(_)
        | TaggedStatement::ContinueStatement(_)
        | TaggedStatement::EmptyStatement(_) => {}
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
}
