//! Aggressive top-level (global) renaming pass for the Closure Compiler
//! clone — the **ADVANCED**-level complement to
//! [`closure-pass-rename`](../closure-pass-rename), which only shortens
//! the *locals* of leaf functions.
//!
//! ```js
//! // before  (SIMPLE leaves these names alone — they might be external)
//! function computeTotal(items) { return items.length; }
//! var cachedResult = computeTotal(list);
//!
//! // after rename-globals  (ADVANCED, nothing external)
//! function a(items) { return items.length; }
//! var b = a(list);
//! ```
//!
//! # Why this is ADVANCED-only
//!
//! In a script, a top-level `function`/`var`/`let`/`const` is part of the
//! program's *public surface*: another script in the same realm can read
//! it off the global object (or the shared top-level lexical scope). So
//! renaming a top-level name is only safe under Closure's **whole-program
//! / `--externs` contract**: *everything externally visible is declared
//! in the externs*; anything else is private to this compilation and may
//! be shortened. SIMPLE makes no such assumption and never touches
//! top-level names; ADVANCED does, which is exactly why ADVANCED output
//! is smaller. This pass takes the externs boundary as a **do-not-rename
//! set** and renames the rest.
//!
//! # What gets renamed — and the soundness argument
//!
//! A top-level binding is renamed iff ALL of:
//!
//!   1. **It is declared at the top level** — a `function` declaration or
//!      a `var`/`let`/`const` declarator in program-body position. (A free
//!      global like `console` has no declaration here, so it is never a
//!      candidate and never renamed.)
//!   2. **Its name is declared exactly once in the whole program.** With a
//!      single declaration there is exactly one binding for the name, so
//!      *every* identifier use of it (other than property names) resolves
//!      to it — rewriting the declaration plus all uses is a provably
//!      sound α-conversion. A name also bound somewhere else (a parameter,
//!      a local `var`, a second declaration) is skipped: its uses could
//!      belong to distinct bindings we can't disambiguate without full
//!      scope resolution. (Same self-contained guard the `inline` and
//!      `inline-variables` passes use.)
//!   3. **It is not in the do-not-rename set** (the externs boundary).
//!   4. **Its name is longer than one character** — already minimal
//!      otherwise.
//!
//! The fresh short name we pick **avoids every identifier that appears
//! anywhere in the program** (declarations, uses, property names, free
//! globals) and every do-not-rename name, so a rename can neither collide
//! with another binding (including a function-local of the same letter)
//! nor capture a free global. Property names — the `.x` of a non-computed
//! member access and a non-computed object-literal key — are never
//! rewritten; a computed `obj[x]` is.
//!
//! # Composition with `closure-pass-rename`
//!
//! The two renamers operate on disjoint names: `rename` shortens
//! leaf-function *parameters and locals*; this pass shortens *top-level*
//! names. (A name bound both at the top level and as a local is declared
//! more than once, so neither pass renames it.) Running both in ADVANCED
//! shortens both layers.

use std::collections::{HashMap, HashSet};

use coding_adventures_closure_pass_pipeline::{
    IterationPolicy, Pass, PassContext, PassError, PassOutput, PassStats,
};
use coding_adventures_correlation_vector::Contribution;
use coding_adventures_javascript_ast::statement::TaggedStatement;
use serde_json::json;
use coding_adventures_javascript_ast::{
    ArrowBody, AssignmentTarget, BindingTarget, ClassMember, Declaration, Expression, ForInit,
    FunctionParam, ObjectMember, Program, ProgramItem, PropertyKey, Statement, VariableDeclaration,
};

/// `Pass::depends_on` value — empty. Global renaming is correct on its
/// own; it just produces less compression without the structural passes
/// having run first. Kept as a `const` for dependent tests.
const DEPS: &[&str] = &[];

/// Reserved words we must never emit as a fresh short name. Single
/// letters are all safe; only some two-letter combinations collide.
/// (Same list as `closure-pass-rename`.)
const RESERVED: &[&str] = &["do", "if", "in", "of", "as", "is", "or"];

/// Aggressive top-level (global) renaming pass. Holds the **do-not-rename
/// set** — the externs boundary — supplied at construction. See the
/// crate-level docs for the exact (provably-safe) slice it implements.
#[derive(Debug, Default, Clone)]
pub struct RenameGlobalsPass {
    /// Names that must NOT be renamed because they are externally visible
    /// (the union of the externs files' top-level names and any other
    /// caller-protected names). Everything else top-level is private.
    do_not_rename: HashSet<String>,
}

impl RenameGlobalsPass {
    /// Construct with the externs do-not-rename set.
    pub fn new(do_not_rename: HashSet<String>) -> Self {
        Self { do_not_rename }
    }

    /// Construct with an empty externs boundary — i.e. the pure
    /// whole-program assumption, where *every* top-level name is private
    /// and renameable. Convenience for callers that pass no `--externs`.
    pub fn with_no_externs() -> Self {
        Self {
            do_not_rename: HashSet::new(),
        }
    }
}

impl Pass for RenameGlobalsPass {
    fn name(&self) -> &'static str {
        "rename-globals"
    }

    fn depends_on(&self) -> &[&'static str] {
        DEPS
    }

    fn iteration_policy(&self) -> IterationPolicy {
        // After one walk every renameable top-level binding has been
        // shortened; a second walk would do nothing. Like `rename`, this
        // pass doesn't open new opportunities for itself.
        IterationPolicy::OneShot
    }

    fn cost(&self) -> u32 {
        // Three whole-program walks: declaration counting (shadow
        // detection), the avoid-set collection, and the rewrite.
        3
    }

    fn run(&self, ctx: PassContext<'_>) -> Result<PassOutput, PassError> {
        let mut program = ctx.program.clone();
        let mut nodes_touched: u32 = 1; // the program root
        let (changed, renames) =
            rename_globals(&mut program, &self.do_not_rename, &mut nodes_touched);

        // CV provenance (#89): record every global rename as a `renamed`
        // contribution carrying `{from, to}`. The pipeline attaches these
        // to the program-root CV entry, so a `--correlation_vector`
        // consumer can map a minified global (`a`) back to its original
        // source name (`longName`) — provenance that rename otherwise
        // erased. Emitted only when the log is enabled matters at the
        // pipeline layer; here we always build the (cheap) list.
        //
        // This is the rename *table* (name → name), attached at the
        // program root. Attaching a contribution to each renamed
        // identifier's OWN CV id (per-output-span provenance) needs the
        // log threaded through the `rename_apply_*` recursion and is a
        // documented follow-up.
        let contributions: Vec<Contribution> = renames
            .into_iter()
            .map(|(from, to)| Contribution {
                source: "rename-globals".to_string(),
                tag: "renamed".to_string(),
                meta: [
                    ("from".to_string(), json!(from)),
                    ("to".to_string(), json!(to)),
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
// Implementation
// =========================================================================

/// Rename qualifying top-level bindings to fresh short names. Returns
/// whether anything changed.
/// Renames qualifying top-level bindings to fresh short names.
///
/// Returns `(changed, renames)` where `renames` is the applied rename
/// table as `(from, to)` pairs sorted by original name (deterministic
/// order for stable CV provenance). `renames` is empty exactly when
/// `changed` is `false`.
fn rename_globals(
    program: &mut Program,
    do_not_rename: &HashSet<String>,
    nodes_touched: &mut u32,
) -> (bool, Vec<(String, String)>) {
    // 1. Declaration counts across the whole program (the shadow guard).
    let mut decl_counts: HashMap<String, usize> = HashMap::new();
    count_decl_names_program(program, &mut decl_counts, nodes_touched);

    // 2. Top-level declared names, in source order (deterministic output).
    let mut top_level: Vec<String> = Vec::new();
    let mut seen_top: HashSet<String> = HashSet::new();
    for item in &program.body {
        match item {
            ProgramItem::Declaration(Declaration::FunctionDeclaration(fd)) => {
                if seen_top.insert(fd.id.name.clone()) {
                    top_level.push(fd.id.name.clone());
                }
            }
            ProgramItem::Declaration(Declaration::VariableDeclaration(vd))
            | ProgramItem::Statement(Statement::Declaration(Declaration::VariableDeclaration(
                vd,
            ))) => {
                for d in &vd.declarations {
                    let BindingTarget::Identifier(id) = &d.id;
                    if seen_top.insert(id.name.clone()) {
                        top_level.push(id.name.clone());
                    }
                }
            }
            // A nested function declared via the bridge's Statement shape
            // is still top-level scope-wise; cover it too.
            ProgramItem::Statement(Statement::Declaration(Declaration::FunctionDeclaration(
                fd,
            ))) => {
                if seen_top.insert(fd.id.name.clone()) {
                    top_level.push(fd.id.name.clone());
                }
            }
            _ => {}
        }
    }

    // 3. Avoid set — every identifier anywhere in the program, plus the
    // do-not-rename names. A fresh name drawn outside this set can neither
    // collide with another binding (top-level OR a function-local of the
    // same letter) nor capture a free global.
    let mut avoid: HashSet<String> = HashSet::new();
    collect_all_idents_program(program, &mut avoid);
    for n in do_not_rename {
        avoid.insert(n.clone());
    }

    // 4. Decide the renames, in declaration order for stable output.
    let mut map: HashMap<String, String> = HashMap::new();
    let mut gen = FreshNames::new();
    for name in &top_level {
        if do_not_rename.contains(name) {
            continue; // externally visible — must keep its name
        }
        if decl_counts.get(name.as_str()).copied().unwrap_or(0) != 1 {
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
        return (false, Vec::new());
    }

    // 5. Apply: rewrite declarations + every use across the whole program.
    for item in &mut program.body {
        rename_apply_item(item, &map);
    }

    // The rename table drives CV provenance (#89). Sort by original name
    // so the emitted contributions are deterministic run to run.
    let mut renames: Vec<(String, String)> =
        map.into_iter().map(|(from, to)| (from, to)).collect();
    renames.sort();
    (true, renames)
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

// ---- declaration-name counting (shadow detection) ------------------------

/// Count every binding-name *declaration* in the whole program — function
/// names, parameters, and `var`/`let`/`const` targets, recursing into
/// nested function bodies — into `out`. A top-level name with a count of
/// 1 has no shadowing binding and is safe to rewrite by name.
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
        // A class declaration binds its name (`cd.id`) as a global, and each
        // method's params + body-locals are declared names too. Counting the
        // method params here is what upholds the rename invariant used by
        // `rename_apply_decl`: a method param that shares a global name pushes
        // that name's count past 1, disqualifying it from renaming — so a
        // later body walk with the full map can never rewrite a shadowed use.
        Declaration::ClassDeclaration(cd) => {
            *out.entry(cd.id.name.clone()).or_insert(0) += 1;
            for member in &cd.body {
                match member {
                    ClassMember::Method(m) => {
                        for p in &m.value.params {
                            let FunctionParam::Identifier(id) = p;
                            *out.entry(id.name.clone()).or_insert(0) += 1;
                        }
                        for s in &m.value.body.body {
                            count_decl_names_stmt(s, out, nodes_touched);
                        }
                    }
                    // A field has no params, and its initializer declares no
                    // statement-scope names at the class-body level.
                    ClassMember::Field(_) => {}
                    // A static-init block has no params, but its statements
                    // declare their own locals — count them, upholding the same
                    // rename invariant as a method body.
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
                if let ForInit::VariableDeclaration(vd) = &fs.left {
                    count_decl_names_var(vd, out);
                }
                count_decl_names_stmt(&fs.body, out, nodes_touched);
            }
            TaggedStatement::ForOfStatement(fs) => {
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
                // Count the catch `param` as a declared binding: a top-level
                // global of the same name is then shadowed (count > 1) and the
                // shadow guard skips renaming it — sound. Recurse into the
                // three blocks for their declarations.
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
            // No binding introduced in the Phase-1 AST. Exhaustive on
            // purpose: a future binding-introducing statement must be
            // handled here so a shadowing name can't slip past the guard.
            TaggedStatement::ExpressionStatement(_)
            | TaggedStatement::ReturnStatement(_)
            | TaggedStatement::ThrowStatement(_)
            | TaggedStatement::BreakStatement(_)
            | TaggedStatement::ContinueStatement(_)
            | TaggedStatement::EmptyStatement(_) | TaggedStatement::DebuggerStatement(_) => {}
        },
    }
}

// ---- avoid-set collection (every identifier, anywhere) -------------------

fn collect_all_idents_program(program: &Program, out: &mut HashSet<String>) {
    for item in &program.body {
        match item {
            ProgramItem::Declaration(d) => collect_all_idents_decl(d, out),
            ProgramItem::Statement(s) => collect_all_idents_stmt(s, out),
        }
    }
}

fn collect_all_idents_decl(decl: &Declaration, out: &mut HashSet<String>) {
    match decl {
        Declaration::VariableDeclaration(vd) => {
            for d in &vd.declarations {
                let BindingTarget::Identifier(id) = &d.id;
                out.insert(id.name.clone());
                if let Some(init) = &d.init {
                    collect_all_idents_expr(init, out);
                }
            }
        }
        Declaration::ClassDeclaration(cd) => {
            // Mirror the `Expression::ClassExpression` arm of
            // `collect_all_idents_expr`: the class name, the heritage operand's
            // identifiers, and each method's key / value-name / params / body.
            out.insert(cd.id.name.clone());
            if let Some(sup) = &cd.super_class {
                collect_all_idents_expr(sup, out);
            }
            for member in &cd.body {
                match member {
                    ClassMember::Method(m) => {
                        if let PropertyKey::Identifier(id) = &m.key {
                            out.insert(id.name.clone());
                        }
                        if let Some(id) = &m.value.id {
                            out.insert(id.name.clone());
                        }
                        for p in &m.value.params {
                            let FunctionParam::Identifier(id) = p;
                            out.insert(id.name.clone());
                        }
                        for s in &m.value.body.body {
                            collect_all_idents_stmt(s, out);
                        }
                    }
                    // A field contributes its key ident + every identifier in a
                    // computed key and the initializer (over-collect for
                    // rename-collision safety).
                    ClassMember::Field(f) => {
                        if let PropertyKey::Identifier(id) = &f.key {
                            out.insert(id.name.clone());
                        }
                        if let PropertyKey::Expression(e) = &f.key {
                            collect_all_idents_expr(e, out);
                        }
                        if let Some(v) = &f.value {
                            collect_all_idents_expr(v, out);
                        }
                    }
                    // A static-init block has no key/name/params; over-collect
                    // every identifier in its statements for rename-collision
                    // safety, mirroring the method-body walk.
                    ClassMember::StaticBlock(b) => {
                        for s in &b.body {
                            collect_all_idents_stmt(s, out);
                        }
                    }
                }
            }
        }
        Declaration::FunctionDeclaration(fd) => {
            out.insert(fd.id.name.clone());
            for p in &fd.params {
                let FunctionParam::Identifier(id) = p;
                out.insert(id.name.clone());
            }
            for s in &fd.body.body {
                collect_all_idents_stmt(s, out);
            }
        }
    }
}

fn collect_all_idents_stmt(stmt: &Statement, out: &mut HashSet<String>) {
    match stmt {
        Statement::Declaration(d) => collect_all_idents_decl(d, out),
        Statement::Tagged(t) => match t {
            TaggedStatement::ExpressionStatement(es) => {
                collect_all_idents_expr(&es.expression, out)
            }
            TaggedStatement::BlockStatement(b) => {
                for s in &b.body {
                    collect_all_idents_stmt(s, out);
                }
            }
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
            TaggedStatement::ForInStatement(fs) => {
                match &fs.left {
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
                collect_all_idents_expr(&fs.right, out);
                collect_all_idents_stmt(&fs.body, out);
            }
            TaggedStatement::ForOfStatement(fs) => {
                match &fs.left {
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
                collect_all_idents_expr(&fs.right, out);
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
                // Add the catch `param` to the avoid set so a renamed global
                // never collides with it; recurse into the three blocks.
                if let Some(h) = &ts.handler {
                    if let Some(param) = &h.param {
                        out.insert(param.name.clone());
                    }
                    for s in &h.body.body {
                        collect_all_idents_stmt(s, out);
                    }
                }
                for s in &ts.block.body {
                    collect_all_idents_stmt(s, out);
                }
                if let Some(f) = &ts.finalizer {
                    for s in &f.body {
                        collect_all_idents_stmt(s, out);
                    }
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
        | Expression::RegExpLiteral(_)
        // `this` binds no global name — nothing to collect or rename.
        | Expression::ThisExpression(_)
        | Expression::Super(_)
        | Expression::NewTarget(_)
        | Expression::ImportMeta(_)
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
        Expression::UpdateExpression(ue) => collect_all_idents_expr(&ue.argument, out),
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
        Expression::NewExpression(ne) => {
            collect_all_idents_expr(&ne.callee, out);
            for a in &ne.arguments {
                collect_all_idents_expr(a, out);
            }
        }
        Expression::SequenceExpression(se) => {
            for e in &se.expressions {
                collect_all_idents_expr(e, out);
            }
        }
        Expression::MemberExpression(m) => {
            collect_all_idents_member(&m.object, &m.property, m.computed, out)
        }
        // `a?.b` / `a?.[k]` — collect idents in object and (computed) property
        // exactly as a plain member access.
        Expression::OptionalMemberExpression(m) => {
            collect_all_idents_member(&m.object, &m.property, m.computed, out)
        }
        // `a?.()` — collect idents in callee and each argument, as for a call.
        Expression::OptionalCallExpression(ce) => {
            collect_all_idents_expr(&ce.callee, out);
            for a in &ce.arguments {
                collect_all_idents_expr(a, out);
            }
        }
        // A chain expression transparently wraps its optional-chain spine —
        // descend into the inner expression.
        Expression::ChainExpression(c) => collect_all_idents_expr(&c.expression, out),
        Expression::ArrayExpression(ae) => {
            for el in ae.elements.iter().flatten() {
                collect_all_idents_expr(el, out);
            }
        }
        Expression::ObjectExpression(oe) => {
            for member in &oe.properties {
                match member {
                    ObjectMember::Property(prop) => {
                        match &prop.key {
                            PropertyKey::Identifier(id) => {
                                out.insert(id.name.clone());
                            }
                            PropertyKey::Expression(e) => collect_all_idents_expr(e, out),
                            _ => {}
                        }
                        collect_all_idents_expr(&prop.value, out);
                    }
                    // Object spread `...expr` — recurse into the spread
                    // argument the same way the property arm recurses into
                    // prop.value.
                    ObjectMember::Spread(s) => {
                        collect_all_idents_expr(&s.argument, out);
                    }
                }
            }
        }
        // A function *value* introduces its own name (if any) and params
        // as identifiers, and its body references more — record them all
        // so a renamed global never collides with any of them.
        Expression::FunctionExpression(fe) => {
            if let Some(id) = &fe.id {
                out.insert(id.name.clone());
            }
            for p in &fe.params {
                let FunctionParam::Identifier(id) = p;
                out.insert(id.name.clone());
            }
            for s in &fe.body.body {
                collect_all_idents_stmt(s, out);
            }
        }
        // A class *value* contributes: its optional class name, the idents in
        // the `extends` operand, and — for each method — the method value's
        // own name (if any), its params, and its body identifiers. Recording
        // them all keeps a renamed global from colliding with any of them,
        // exactly as the `FunctionExpression` arm does for a function value.
        // An identifier method KEY is a property name (like an object-literal
        // key), recorded here for collision-avoidance the same way the
        // `ObjectExpression` arm records `PropertyKey::Identifier`.
        Expression::ClassExpression(ce) => {
            if let Some(id) = &ce.id {
                out.insert(id.name.clone());
            }
            if let Some(sup) = &ce.super_class {
                collect_all_idents_expr(sup, out);
            }
            for member in &ce.body {
                match member {
                    ClassMember::Method(m) => {
                        if let PropertyKey::Identifier(id) = &m.key {
                            out.insert(id.name.clone());
                        }
                        if let Some(id) = &m.value.id {
                            out.insert(id.name.clone());
                        }
                        for p in &m.value.params {
                            let FunctionParam::Identifier(id) = p;
                            out.insert(id.name.clone());
                        }
                        for s in &m.value.body.body {
                            collect_all_idents_stmt(s, out);
                        }
                    }
                    // A field's key ident + computed-key / initializer
                    // identifiers, over-collected for collision-avoidance.
                    ClassMember::Field(f) => {
                        if let PropertyKey::Identifier(id) = &f.key {
                            out.insert(id.name.clone());
                        }
                        if let PropertyKey::Expression(e) = &f.key {
                            collect_all_idents_expr(e, out);
                        }
                        if let Some(v) = &f.value {
                            collect_all_idents_expr(v, out);
                        }
                    }
                    // A static-init block has no key/name/params; over-collect
                    // every identifier in its statements for collision-avoidance.
                    ClassMember::StaticBlock(b) => {
                        for s in &b.body {
                            collect_all_idents_stmt(s, out);
                        }
                    }
                }
            }
        }
        // An arrow value contributes its params and body identifiers (it
        // has no name), so a renamed global never collides with them.
        Expression::ArrowFunctionExpression(ae) => {
            for p in &ae.params {
                let FunctionParam::Identifier(id) = p;
                out.insert(id.name.clone());
            }
            match &ae.body {
                ArrowBody::Block(b) => {
                    for s in &b.body {
                        collect_all_idents_stmt(s, out);
                    }
                }
                ArrowBody::Expression(e) => collect_all_idents_expr(e, out),
            }
        }
        // A template literal contributes the identifiers used in its `${…}`
        // inserts (it introduces no names). Quasis are leaf strings — only
        // the insert expressions carry identifiers to record.
        Expression::TemplateLiteral(t) => {
            for e in &t.expressions {
                collect_all_idents_expr(e, out);
            }
        }
        // A tagged template carries identifiers in its tag callee and each
        // `${…}` insert. Quasis are leaf strings with nothing to record.
        Expression::TaggedTemplateExpression(t) => {
            collect_all_idents_expr(&t.tag, out);
            for e in &t.quasi.expressions {
                collect_all_idents_expr(e, out);
            }
        }
        // `...arg` — recurse into the spread argument to collect its idents.
        Expression::SpreadElement(s) => collect_all_idents_expr(&s.argument, out),
        Expression::YieldExpression(y) => { if let Some(a) = &y.argument { collect_all_idents_expr(a, out); } }
        Expression::AwaitExpression(a) => collect_all_idents_expr(&a.argument, out),
        Expression::ImportExpression(e) => collect_all_idents_expr(&e.source, out),
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
        // Property name — record for avoidance, but it is not a binding
        // use (the rewrite skips it).
        out.insert(id.name.clone());
    }
}

// ---- rewrite (declarations + every use) ----------------------------------

fn rename_apply_item(item: &mut ProgramItem, map: &HashMap<String, String>) {
    match item {
        ProgramItem::Declaration(d) => rename_apply_decl(d, map),
        ProgramItem::Statement(s) => rename_apply_stmt(s, map),
    }
}

fn rename_apply_decl(decl: &mut Declaration, map: &HashMap<String, String>) {
    match decl {
        Declaration::VariableDeclaration(vd) => {
            for d in &mut vd.declarations {
                // Rename the declared name itself (a top-level declarator
                // is a rename target) and its initializer (a use position).
                let BindingTarget::Identifier(id) = &mut d.id;
                if let Some(new) = map.get(&id.name) {
                    id.name = new.clone();
                }
                if let Some(init) = &mut d.init {
                    rename_apply_expr(init, map);
                }
            }
        }
        Declaration::FunctionDeclaration(fd) => {
            // Rename the function's own name (the binding) …
            if let Some(new) = map.get(&fd.id.name) {
                fd.id.name = new.clone();
            }
            // … and recurse into the body for uses of any renamed global.
            // Parameters are NOT renamed here — a parameter that shared a
            // top-level name would make that name declared-more-than-once,
            // so it would not be in `map`.
            for s in &mut fd.body.body {
                rename_apply_stmt(s, map);
            }
        }
        Declaration::ClassDeclaration(cd) => {
            // Rename the class's own name (a top-level binding, a rename
            // target) and the `extends` operand (a use position). Then recurse
            // each method body with the full `map` — mirroring the function
            // declaration arm. This is safe for the same reason: a method param
            // (or local) sharing a global name would be counted more than once
            // by `count_decl_names_decl` (which tallies method params), so that
            // name is excluded from `map` and a shadowed use is never rewritten.
            if let Some(new) = map.get(&cd.id.name) {
                cd.id.name = new.clone();
            }
            if let Some(sup) = &mut cd.super_class {
                rename_apply_expr(sup, map);
            }
            for member in &mut cd.body {
                match member {
                    ClassMember::Method(m) => {
                        for s in &mut m.value.body.body {
                            rename_apply_stmt(s, map);
                        }
                    }
                    // Rewrite renamed globals used in a field's computed key and
                    // initializer. The field *key* (an identifier name) is a
                    // property, not a variable, so it is not renamed here.
                    ClassMember::Field(f) => {
                        if let PropertyKey::Expression(e) = &mut f.key {
                            rename_apply_expr(e, map);
                        }
                        if let Some(v) = &mut f.value {
                            rename_apply_expr(v, map);
                        }
                    }
                    // Rewrite renamed globals used in the static-init block's
                    // statements, mirroring the method-body rewrite.
                    ClassMember::StaticBlock(b) => {
                        for s in &mut b.body {
                            rename_apply_stmt(s, map);
                        }
                    }
                }
            }
        }
    }
}

fn rename_apply_stmt(stmt: &mut Statement, map: &HashMap<String, String>) {
    match stmt {
        Statement::Declaration(d) => rename_apply_decl(d, map),
        Statement::Tagged(t) => rename_apply_tagged(t, map),
    }
}

fn rename_apply_tagged(t: &mut TaggedStatement, map: &HashMap<String, String>) {
    match t {
        TaggedStatement::ExpressionStatement(es) => rename_apply_expr(&mut es.expression, map),
        TaggedStatement::BlockStatement(b) => {
            for s in &mut b.body {
                rename_apply_stmt(s, map);
            }
        }
        TaggedStatement::IfStatement(is) => {
            rename_apply_expr(&mut is.test, map);
            rename_apply_stmt(&mut is.consequent, map);
            if let Some(alt) = &mut is.alternate {
                rename_apply_stmt(alt, map);
            }
        }
        TaggedStatement::WhileStatement(ws) => {
            rename_apply_expr(&mut ws.test, map);
            rename_apply_stmt(&mut ws.body, map);
        }
        TaggedStatement::DoWhileStatement(ds) => {
            rename_apply_expr(&mut ds.test, map);
            rename_apply_stmt(&mut ds.body, map);
        }
        TaggedStatement::ForStatement(fs) => {
            if let Some(init) = &mut fs.init {
                match init {
                    ForInit::VariableDeclaration(vd) => {
                        for d in &mut vd.declarations {
                            let BindingTarget::Identifier(id) = &mut d.id;
                            if let Some(new) = map.get(&id.name) {
                                id.name = new.clone();
                            }
                            if let Some(i) = &mut d.init {
                                rename_apply_expr(i, map);
                            }
                        }
                    }
                    ForInit::Expression(e) => rename_apply_expr(e, map),
                }
            }
            if let Some(test) = &mut fs.test {
                rename_apply_expr(test, map);
            }
            if let Some(update) = &mut fs.update {
                rename_apply_expr(update, map);
            }
            rename_apply_stmt(&mut fs.body, map);
        }
        TaggedStatement::ForInStatement(fs) => {
            match &mut fs.left {
                ForInit::VariableDeclaration(vd) => {
                    for d in &mut vd.declarations {
                        let BindingTarget::Identifier(id) = &mut d.id;
                        if let Some(new) = map.get(&id.name) {
                            id.name = new.clone();
                        }
                        if let Some(i) = &mut d.init {
                            rename_apply_expr(i, map);
                        }
                    }
                }
                ForInit::Expression(e) => rename_apply_expr(e, map),
            }
            rename_apply_expr(&mut fs.right, map);
            rename_apply_stmt(&mut fs.body, map);
        }
        TaggedStatement::ForOfStatement(fs) => {
            match &mut fs.left {
                ForInit::VariableDeclaration(vd) => {
                    for d in &mut vd.declarations {
                        let BindingTarget::Identifier(id) = &mut d.id;
                        if let Some(new) = map.get(&id.name) {
                            id.name = new.clone();
                        }
                        if let Some(i) = &mut d.init {
                            rename_apply_expr(i, map);
                        }
                    }
                }
                ForInit::Expression(e) => rename_apply_expr(e, map),
            }
            rename_apply_expr(&mut fs.right, map);
            rename_apply_stmt(&mut fs.body, map);
        }
        TaggedStatement::ReturnStatement(rs) => {
            if let Some(a) = &mut rs.argument {
                rename_apply_expr(a, map);
            }
        }
        TaggedStatement::ThrowStatement(ts) => rename_apply_expr(&mut ts.argument, map),
        TaggedStatement::LabeledStatement(ls) => rename_apply_stmt(&mut ls.body, map),
        TaggedStatement::SwitchStatement(ss) => {
            rename_apply_expr(&mut ss.discriminant, map);
            for c in &mut ss.cases {
                if let Some(test) = &mut c.test {
                    rename_apply_expr(test, map);
                }
                for s in &mut c.consequent {
                    rename_apply_stmt(s, map);
                }
            }
        }
        TaggedStatement::TryStatement(ts) => {
            // Rewrite renamed-global uses inside the three blocks. A global
            // shadowed by the catch `param` was skipped by the shadow guard
            // (not in `map`), so uses of the param resolve correctly and are
            // left untouched; the param name itself is never a map key.
            for s in &mut ts.block.body {
                rename_apply_stmt(s, map);
            }
            if let Some(h) = &mut ts.handler {
                for s in &mut h.body.body {
                    rename_apply_stmt(s, map);
                }
            }
            if let Some(f) = &mut ts.finalizer {
                for s in &mut f.body {
                    rename_apply_stmt(s, map);
                }
            }
        }
        // Labels live in a separate namespace from variables.
        TaggedStatement::BreakStatement(_)
        | TaggedStatement::ContinueStatement(_)
        | TaggedStatement::EmptyStatement(_) | TaggedStatement::DebuggerStatement(_) => {}
    }
}

fn rename_apply_expr(expr: &mut Expression, map: &HashMap<String, String>) {
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
        | Expression::RegExpLiteral(_)
        // `this` binds no global name — nothing to collect or rename.
        | Expression::ThisExpression(_)
        | Expression::Super(_)
        | Expression::NewTarget(_)
        | Expression::ImportMeta(_)
        | Expression::UndefinedLiteral(_) => {}
        Expression::BinaryExpression(be) => {
            rename_apply_expr(&mut be.left, map);
            rename_apply_expr(&mut be.right, map);
        }
        Expression::LogicalExpression(le) => {
            rename_apply_expr(&mut le.left, map);
            rename_apply_expr(&mut le.right, map);
        }
        Expression::UnaryExpression(ue) => rename_apply_expr(&mut ue.argument, map),
        Expression::UpdateExpression(ue) => rename_apply_expr(&mut ue.argument, map),
        Expression::AssignmentExpression(ae) => {
            match &mut ae.left {
                AssignmentTarget::Identifier(id) => {
                    // An assignment to a renamed top-level binding (e.g.
                    // `var x; x = 1`) must follow the rename.
                    if let Some(new) = map.get(&id.name) {
                        id.name = new.clone();
                    }
                }
                AssignmentTarget::MemberExpression(m) => {
                    rename_apply_member(&mut m.object, &mut m.property, m.computed, map)
                }
            }
            rename_apply_expr(&mut ae.right, map);
        }
        Expression::ConditionalExpression(ce) => {
            rename_apply_expr(&mut ce.test, map);
            rename_apply_expr(&mut ce.consequent, map);
            rename_apply_expr(&mut ce.alternate, map);
        }
        Expression::CallExpression(ce) => {
            rename_apply_expr(&mut ce.callee, map);
            for a in &mut ce.arguments {
                rename_apply_expr(a, map);
            }
        }
        Expression::NewExpression(ne) => {
            rename_apply_expr(&mut ne.callee, map);
            for a in &mut ne.arguments {
                rename_apply_expr(a, map);
            }
        }
        Expression::SequenceExpression(se) => {
            for e in &mut se.expressions {
                rename_apply_expr(e, map);
            }
        }
        Expression::MemberExpression(m) => {
            rename_apply_member(&mut m.object, &mut m.property, m.computed, map)
        }
        // `a?.b` / `a?.[k]` — rename in object and (computed) property exactly
        // as a plain member access.
        Expression::OptionalMemberExpression(m) => {
            rename_apply_member(&mut m.object, &mut m.property, m.computed, map)
        }
        // `a?.()` — rename in callee and each argument, as for a call.
        Expression::OptionalCallExpression(ce) => {
            rename_apply_expr(&mut ce.callee, map);
            for a in &mut ce.arguments {
                rename_apply_expr(a, map);
            }
        }
        // A chain expression transparently wraps its optional-chain spine —
        // descend into the inner expression.
        Expression::ChainExpression(c) => rename_apply_expr(&mut c.expression, map),
        Expression::ArrayExpression(ae) => {
            for el in ae.elements.iter_mut().flatten() {
                rename_apply_expr(el, map);
            }
        }
        Expression::ObjectExpression(oe) => {
            for member in &mut oe.properties {
                match member {
                    ObjectMember::Property(prop) => {
                        // Only a *computed* key `[expr]` is a use position; a plain
                        // identifier / string / number key is a property name.
                        if prop.computed {
                            if let PropertyKey::Expression(e) = &mut prop.key {
                                rename_apply_expr(e, map);
                            }
                        }
                        rename_apply_expr(&mut prop.value, map);
                    }
                    // Object spread `...expr` — recurse into the spread
                    // argument the same way the property arm recurses into
                    // prop.value.
                    ObjectMember::Spread(s) => {
                        rename_apply_expr(&mut s.argument, map);
                    }
                }
            }
        }
        // A function *value*'s own name (if named) and its params are
        // LOCAL bindings that shadow any global of the same spelling.
        // Recurse into the body with those names REMOVED from the active
        // map, so genuine global uses inside the body are renamed while a
        // shadowed use (referring to the param / self-name) is left
        // untouched — a self-contained soundness guarantee that does not
        // depend on the candidate-selection step having seen this
        // function expression.
        Expression::FunctionExpression(fe) => {
            let mut inner = map.clone();
            if let Some(id) = &fe.id {
                inner.remove(&id.name);
            }
            for p in &fe.params {
                let FunctionParam::Identifier(id) = p;
                inner.remove(&id.name);
            }
            for s in &mut fe.body.body {
                rename_apply_stmt(s, &inner);
            }
        }
        // A class expression. The `extends` operand is an ordinary
        // value-position expression at the class's OWN scope, so rename it
        // with the outer `map`. Each method's function *value* is its own
        // scope: like the `FunctionExpression` arm, clone the map and remove
        // the LOCAL bindings that shadow a same-spelled global — the class's
        // own name (a named class binds its name inside its body), the
        // method value's own name (if any), and the method params — before
        // recursing into the method body, so genuine global uses are renamed
        // while shadowed uses are left untouched. A method KEY is a property
        // name, NOT a variable use, so it is never renamed.
        Expression::ClassExpression(ce) => {
            if let Some(sup) = &mut ce.super_class {
                rename_apply_expr(sup, map);
            }
            // Names bound for the whole class body (the class's own name).
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
                            let FunctionParam::Identifier(id) = p;
                            inner.remove(&id.name);
                        }
                        for s in &mut m.value.body.body {
                            rename_apply_stmt(s, &inner);
                        }
                    }
                    // A field's computed key + initializer are value-position
                    // expressions in the class body — rename globals in them
                    // with `class_inner` (the class's own name, a local binding
                    // for a class *expression*, removed). The field key name is
                    // a property, not a variable, so it is not renamed.
                    ClassMember::Field(f) => {
                        if let PropertyKey::Expression(e) = &mut f.key {
                            rename_apply_expr(e, &class_inner);
                        }
                        if let Some(v) = &mut f.value {
                            rename_apply_expr(v, &class_inner);
                        }
                    }
                    // A static-init block's statements run at class-definition
                    // time with the class's own name in scope — rename globals in
                    // them with `class_inner`.
                    ClassMember::StaticBlock(b) => {
                        for s in &mut b.body {
                            rename_apply_stmt(s, &class_inner);
                        }
                    }
                }
            }
        }
        // An arrow's params are LOCAL bindings that shadow any global of
        // the same spelling; recurse into the body with those names
        // removed from the active map, so genuine global uses are renamed
        // while a shadowed param use is left untouched. (Arrows have no
        // self-name to shadow.)
        Expression::ArrowFunctionExpression(ae) => {
            let mut inner = map.clone();
            for p in &ae.params {
                let FunctionParam::Identifier(id) = p;
                inner.remove(&id.name);
            }
            match &mut ae.body {
                ArrowBody::Block(b) => {
                    for s in &mut b.body {
                        rename_apply_stmt(s, &inner);
                    }
                }
                ArrowBody::Expression(e) => rename_apply_expr(e, &inner),
            }
        }
        // A template literal binds nothing, so there is no shadowing to strip
        // — rename globals straight through each `${…}` insert. Quasis are
        // leaf strings and hold no renamable use.
        Expression::TemplateLiteral(t) => {
            for e in &mut t.expressions {
                rename_apply_expr(e, map);
            }
        }
        // Rename globals in the tag callee and each `${…}` insert.
        Expression::TaggedTemplateExpression(t) => {
            rename_apply_expr(&mut t.tag, map);
            for e in &mut t.quasi.expressions {
                rename_apply_expr(e, map);
            }
        }
        // `...arg` — recurse into the spread argument to rename globals through it.
        Expression::SpreadElement(s) => rename_apply_expr(&mut s.argument, map),
        Expression::YieldExpression(y) => { if let Some(a) = &mut y.argument { rename_apply_expr(a, map); } }
        Expression::AwaitExpression(a) => rename_apply_expr(&mut a.argument, map),
        Expression::ImportExpression(e) => rename_apply_expr(&mut e.source, map),
    }
}

fn rename_apply_member(
    object: &mut Expression,
    property: &mut Expression,
    computed: bool,
    map: &HashMap<String, String>,
) {
    rename_apply_expr(object, map);
    // The `.name` of a non-computed member access is a property name, NOT
    // a binding — only rewrite the property when it is computed (`o[x]`).
    if computed {
        rename_apply_expr(property, map);
    }
}

#[cfg(test)]
mod tests {
    //! Tests pin the public contract and the renaming behaviour, driven
    //! end-to-end through the real source → bridge → rename-globals → emit
    //! roundtrip so they exercise the exact AST shape the parser produces.
    use super::*;
    use coding_adventures_closure_emitter::{emit, EmitOptions};
    use coding_adventures_closure_pass_pipeline::PassContext;
    use coding_adventures_correlation_vector::CVLog;
    use coding_adventures_javascript_ast::{Program, SourceType};
    use coding_adventures_javascript_parser::{bridge, parse_javascript_typed};
    use coding_adventures_javascript_tokens::EsVersion;
    use coding_adventures_type_sidecar::Sidecar;

    fn program() -> Program {
        Program::new("prog.1".to_string(), EsVersion::Es2025, SourceType::Module)
    }

    /// Parse, bridge, run the pass with the given do-not-rename set, emit.
    fn rename_source_with(src: &str, externs: &[&str]) -> String {
        let es = EsVersion::Es2025;
        let node = parse_javascript_typed(src, es).expect("parse");
        let prog = bridge::grammar_to_program(&node, es).expect("bridge");

        let set: HashSet<String> = externs.iter().map(|s| s.to_string()).collect();
        let pass = RenameGlobalsPass::new(set);
        let sidecar = Sidecar::new();
        let mut cv = CVLog::new(false);
        let out = pass
            .run(PassContext {
                program: &prog,
                sidecar: &sidecar,
                cv: &mut cv,
            })
            .expect("rename-globals");

        let mut cv2 = CVLog::new(false);
        let opts = EmitOptions {
            source_map: false,
            ..Default::default()
        };
        emit(&out.program, &sidecar, &mut cv2, &opts)
            .expect("emit")
            .code
    }

    fn rename_source(src: &str) -> String {
        rename_source_with(src, &[])
    }

    /// Run the pass and return its CV contributions (the rename table).
    fn rename_contributions(src: &str) -> Vec<Contribution> {
        let es = EsVersion::Es2025;
        let node = parse_javascript_typed(src, es).expect("parse");
        let prog = bridge::grammar_to_program(&node, es).expect("bridge");
        let pass = RenameGlobalsPass::with_no_externs();
        let sidecar = Sidecar::new();
        let mut cv = CVLog::new(true);
        pass.run(PassContext {
            program: &prog,
            sidecar: &sidecar,
            cv: &mut cv,
        })
        .expect("rename-globals")
        .contributions
    }

    // ----- CV provenance (#89) -----

    #[test]
    fn emits_renamed_contribution_per_global() {
        // `function longName(){} longName();` — `longName` (declared once,
        // len>1, referenced) is renamed; the pass records a `renamed`
        // contribution mapping the original name to its short form.
        let contribs = rename_contributions("function longName(){} longName();");
        let renamed: Vec<_> = contribs
            .iter()
            .filter(|c| c.source == "rename-globals" && c.tag == "renamed")
            .collect();
        assert_eq!(
            renamed.len(),
            1,
            "expected exactly one renamed contribution; got {:?}",
            contribs
        );
        let c = renamed[0];
        assert_eq!(
            c.meta.get("from").and_then(|v| v.as_str()),
            Some("longName")
        );
        let to = c
            .meta
            .get("to")
            .and_then(|v| v.as_str())
            .expect("`to` present");
        assert!(
            to.len() < "longName".len(),
            "renamed to a shorter name; got {:?}",
            to
        );
    }

    #[test]
    fn no_contributions_when_nothing_renamed() {
        // `x();` — `x` is a free global (used, not declared here), so
        // there is nothing to rename and no contribution is emitted.
        let contribs = rename_contributions("x();");
        assert!(
            contribs.is_empty(),
            "expected no contributions; got {:?}",
            contribs
        );
    }

    // ----- metadata contract -----

    #[test]
    fn name_is_rename_globals() {
        assert_eq!(
            RenameGlobalsPass::with_no_externs().name(),
            "rename-globals"
        );
    }

    #[test]
    fn iteration_policy_is_one_shot() {
        assert_eq!(
            RenameGlobalsPass::with_no_externs().iteration_policy(),
            IterationPolicy::OneShot
        );
    }

    #[test]
    fn cost_is_three_pass_units() {
        assert_eq!(RenameGlobalsPass::with_no_externs().cost(), 3);
    }

    #[test]
    fn depends_on_is_empty() {
        assert!(RenameGlobalsPass::with_no_externs().depends_on().is_empty());
    }

    #[test]
    fn run_on_empty_program_is_identity() {
        let pass = RenameGlobalsPass::with_no_externs();
        let prog = program();
        let sidecar = Sidecar::new();
        let mut cv = CVLog::new(true);
        let out = pass
            .run(PassContext {
                program: &prog,
                sidecar: &sidecar,
                cv: &mut cv,
            })
            .expect("ok");
        assert!(!out.changed);
        assert_eq!(out.stats.nodes_touched, 1);
    }

    #[test]
    fn pass_is_default_and_clone() {
        let _a: RenameGlobalsPass = Default::default();
        let _b = RenameGlobalsPass::with_no_externs();
        let _c = _b.clone();
    }

    // =====================================================================
    // Renaming behaviour
    // =====================================================================

    #[test]
    fn renames_top_level_function_and_uses() {
        // The signature case: a top-level function and the top-level var
        // holding its result are both shortened (`computeTotal` → `a`,
        // `result` → `b`); the call site follows. `items` is a parameter
        // (this pass doesn't touch params) and `list` is a free global.
        assert_eq!(
            rename_source(
                "function computeTotal(items) { return items; } var result = computeTotal(list);"
            ),
            "function a(items){return items};var b=a(list);"
        );
    }

    #[test]
    fn renames_top_level_var() {
        assert_eq!(
            rename_source("var counter = 0; bump(counter);"),
            "var a=0;bump(a);"
        );
    }

    #[test]
    fn does_not_rename_free_globals() {
        // `console` / `window` are not declared in the program, so they are
        // never candidates. Only the declared `greet` is renamed.
        assert_eq!(
            rename_source("function greet() { console.log(window); } greet();"),
            "function a(){console.log(window)};a();"
        );
    }

    #[test]
    fn does_not_rename_names_in_externs() {
        // `apiHandler` is in the do-not-rename (externs) set → kept; the
        // private `helper` is renamed.
        assert_eq!(
            rename_source_with(
                "function apiHandler() { return helper(); } function helper() { return 1; }",
                &["apiHandler"]
            ),
            "function apiHandler(){return a()};function a(){return 1};"
        );
    }

    #[test]
    fn does_not_rename_property_names() {
        // `total` as a property of `obj` is not the top-level `total`. The
        // top-level `total` has zero binding-uses (only the property), so
        // it is still renamed at its declaration; the `.total` stays.
        assert_eq!(
            rename_source("var total = 1; read(obj.total);"),
            "var a=1;read(obj.total);"
        );
    }

    #[test]
    fn renames_computed_member_use() {
        // A computed `obj[key]` IS a use of the top-level `key`.
        assert_eq!(
            rename_source("var key = 2; read(obj[key]);"),
            "var a=2;read(obj[a]);"
        );
    }

    #[test]
    fn skips_single_char_names() {
        // Already minimal — nothing to gain, so the program is unchanged.
        assert_eq!(
            rename_source("function f() { return 1; } f();"),
            "function f(){return 1};f();"
        );
    }

    #[test]
    fn skips_name_shadowed_by_a_local() {
        // `helper` is declared both at top level AND as a parameter of
        // `other`, so it is declared more than once → skipped (its uses
        // could resolve to either binding). `other` is declared once → it
        // IS renamed.
        assert_eq!(
            rename_source(
                "function helper() { return 1; } function other(helper) { return helper; }"
            ),
            "function helper(){return 1};function a(helper){return helper};"
        );
    }

    #[test]
    fn fresh_name_avoids_a_local_of_the_same_letter() {
        // The body of `g` has a local `var a`. Renaming the top-level
        // `topName` must NOT pick `a` (that would be captured by the
        // local); it gets the next free name `b`.
        assert_eq!(
            rename_source("function topName() { return 0; } function g() { var a = 1; return a + topName(); }"),
            "function b(){return 0};function g(){var a=1;return a+b()};"
        );
    }

    #[test]
    fn renames_global_used_inside_a_function_body() {
        // A use of a renamed top-level name inside another function body is
        // rewritten (the walk recurses into function bodies).
        assert_eq!(
            rename_source("var SHARED = 5; function read() { return SHARED; } read();"),
            "var a=5;function b(){return a};b();"
        );
    }
}
