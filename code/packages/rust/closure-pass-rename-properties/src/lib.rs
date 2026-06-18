//! Aggressive property renaming pass for the Closure Compiler clone
//! (**ADVANCED**-only) — Closure Compiler's `RENAME_PROPERTIES` in
//! miniature. Consistently shortens program-private object **property
//! names** across the whole program:
//!
//! ```js
//! // before
//! obj.computeTotal = function () {};
//! widget.computeTotal();
//! var cfg = { renderMode: 1 };
//! use(cfg.renderMode);
//!
//! // after rename-properties (ADVANCED)
//! obj.a = function () {};
//! widget.a();
//! var cfg = { b: 1 };
//! use(cfg.b);
//! ```
//!
//! Property access is *by name*: renaming a property name at **every**
//! place it appears keeps the program's meaning, no matter which objects
//! carry it. So `computeTotal` → `a` is applied to every `.computeTotal`
//! dotted access and every unquoted `{ computeTotal: … }` key.
//!
//! # Soundness — the externs / quoting contract
//!
//! Property renaming is sound only under Closure's **externs contract**
//! (the property analogue of the contract the global renamer uses): every
//! property name reachable from *outside* this compilation — a DOM
//! property the browser calls (`onload`), a field another file reads, a
//! key accessed by a dynamically-built string — must be in the
//! do-not-rename boundary. Everything else is private and may be
//! shortened. A name is renamed only when ALL hold:
//!
//!   * **It appears in a renameable (dotted / unquoted-key) position.**
//!     `obj.x` and the key in `{ x: 1 }` are *identifier* positions.
//!   * **It is NOT quoted via a computed string member.** `obj["x"]` is a
//!     *string* position the bridge preserves; the author quoted it to
//!     signal "external / dynamic — leave it alone." A name quoted this
//!     way anywhere is declined everywhere (renaming the dotted
//!     occurrences would desync them from the quoted access we never
//!     touch). **Bridge limitation:** a *quoted object key*
//!     `{ "x": 1 }` is currently collapsed to an identifier key by the
//!     parser bridge, so it is NOT a usable quoting signal — protect such
//!     names via `--externs` instead.
//!   * **It is not a [`BUILTIN_PROPERTIES`]** (`length`, `prototype`,
//!     `toString`, `push`, …). closurec ships no browser/ECMAScript
//!     externs file, so this list is the default-externs substitute that
//!     keeps `arr.length` from becoming `arr.a`. It covers the common
//!     ECMAScript surface but **not the DOM/host** — host property names
//!     (`innerHTML`, `addEventListener`, …) must be supplied via
//!     `--externs`. The [`RenamePropertiesPass::new`] do-not-rename set
//!     extends the built-ins with the externs files' property names.
//!   * **It is longer than one character** (already minimal otherwise).
//!
//! **Dynamic computed access (`obj[expr]`) is the author's
//! responsibility**, exactly as in Closure: reaching a renameable
//! property through a runtime-built string requires quoting its
//! definitions (or listing it in externs). We cannot see the runtime
//! string, so this is a documented contract, not something we can check.
//!
//! Each distinct renameable property gets a **distinct** fresh name (no
//! reuse), so renamed properties never collide. Property names live in
//! their own namespace, so a property may be renamed to `a` even when a
//! *variable* `a` exists — the fresh name only avoids other property
//! names, the built-ins, and the do-not-rename set.
//!
//! # Why ADVANCED-only
//!
//! SIMPLE never renames property names (it cannot assume the quoting
//! contract holds). ADVANCED does, which is part of what makes ADVANCED
//! output smaller. This pass runs after the structural passes and the
//! variable renamers.

use std::collections::{HashMap, HashSet};

use coding_adventures_closure_pass_pipeline::{
    IterationPolicy, Pass, PassContext, PassError, PassOutput, PassStats,
};
use coding_adventures_javascript_ast::statement::TaggedStatement;
use coding_adventures_javascript_ast::{
    AssignmentTarget, Declaration, Expression, ForInit, Program, ProgramItem, PropertyKey,
    Statement,
};

/// `Pass::depends_on` value — empty. Property renaming is correct
/// standalone.
const DEPS: &[&str] = &[];

/// Reserved words we must never emit as a fresh short name.
const RESERVED: &[&str] = &["do", "if", "in", "of", "as", "is", "or"];

/// Built-in property names that must never be renamed — the
/// default-externs substitute (closurec ships no browser/ECMAScript
/// externs file). Renaming any of these would break code that uses the
/// standard library (`arr.length`, `x.toString()`, `p.then(...)`, …). The
/// user's `--externs` files extend this boundary; this list covers the
/// common ECMAScript surface so the pass is safe by default.
///
/// It is intentionally conservative (over-protecting a user-defined
/// property that happens to share a built-in's name merely forgoes a
/// rename — never a miscompile).
const BUILTIN_PROPERTIES: &[&str] = &[
    // Object / function plumbing
    "prototype",
    "constructor",
    "__proto__",
    "name",
    "length",
    "arguments",
    "caller",
    "hasOwnProperty",
    "isPrototypeOf",
    "propertyIsEnumerable",
    "toString",
    "toLocaleString",
    "valueOf",
    "call",
    "apply",
    "bind",
    // Error
    "message",
    "stack",
    "cause",
    // Promise / iterator / generator
    "then",
    "catch",
    "finally",
    "next",
    "return",
    "throw",
    "value",
    "done",
    "resolve",
    "reject",
    "all",
    "race",
    "any",
    "allSettled",
    // Collections
    "size",
    "add",
    "has",
    "get",
    "set",
    "delete",
    "clear",
    "keys",
    "values",
    "entries",
    "forEach",
    // Array
    "push",
    "pop",
    "shift",
    "unshift",
    "slice",
    "splice",
    "concat",
    "join",
    "indexOf",
    "lastIndexOf",
    "includes",
    "find",
    "findIndex",
    "findLast",
    "findLastIndex",
    "filter",
    "map",
    "reduce",
    "reduceRight",
    "some",
    "every",
    "sort",
    "reverse",
    "fill",
    "flat",
    "flatMap",
    "copyWithin",
    "at",
    "from",
    "of",
    "isArray",
    // String
    "charAt",
    "charCodeAt",
    "codePointAt",
    "substring",
    "substr",
    "replace",
    "replaceAll",
    "split",
    "trim",
    "trimStart",
    "trimEnd",
    "toLowerCase",
    "toUpperCase",
    "toLocaleLowerCase",
    "toLocaleUpperCase",
    "padStart",
    "padEnd",
    "startsWith",
    "endsWith",
    "repeat",
    "normalize",
    "localeCompare",
    "match",
    "matchAll",
    "search",
    "fromCharCode",
    "fromCodePoint",
    "raw",
    // Number / Math / RegExp / JSON / Date
    "toFixed",
    "toPrecision",
    "toExponential",
    "test",
    "exec",
    "source",
    "flags",
    "global",
    "ignoreCase",
    "multiline",
    "lastIndex",
    "parse",
    "stringify",
    "now",
    "getTime",
    "getFullYear",
    "getMonth",
    "getDate",
    "getDay",
    "getHours",
    "getMinutes",
    "getSeconds",
    "getMilliseconds",
    "setFullYear",
    "toISOString",
    "toJSON",
    // Console / common host
    "log",
    "warn",
    "error",
    "info",
    "debug",
    "assert",
];

/// Aggressive property renaming pass. Holds the **do-not-rename set** of
/// property names supplied at construction (typically the property names
/// collected from `--externs` files); the built-in property list is
/// always protected on top of it.
#[derive(Debug, Default, Clone)]
pub struct RenamePropertiesPass {
    do_not_rename: HashSet<String>,
}

impl RenamePropertiesPass {
    /// Construct with an externs property do-not-rename set (extends the
    /// always-protected [`BUILTIN_PROPERTIES`]).
    pub fn new(do_not_rename: HashSet<String>) -> Self {
        Self { do_not_rename }
    }

    /// Construct protecting only the built-in properties (no extra
    /// externs property names).
    pub fn with_builtins_only() -> Self {
        Self {
            do_not_rename: HashSet::new(),
        }
    }
}

impl Pass for RenamePropertiesPass {
    fn name(&self) -> &'static str {
        "rename-properties"
    }

    fn depends_on(&self) -> &[&'static str] {
        DEPS
    }

    fn iteration_policy(&self) -> IterationPolicy {
        // After one whole-program walk every renameable property has been
        // shortened; re-running does nothing.
        IterationPolicy::OneShot
    }

    fn cost(&self) -> u32 {
        // Two whole-program walks: classify (dotted vs quoted) + rewrite.
        3
    }

    fn run(&self, ctx: PassContext<'_>) -> Result<PassOutput, PassError> {
        let mut program = ctx.program.clone();
        let mut nodes_touched: u32 = 1;
        let changed = rename_properties(&mut program, &self.do_not_rename, &mut nodes_touched);

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
// Implementation
// =========================================================================

/// The per-name evidence gathered in the classification walk.
#[derive(Default)]
struct Classify {
    /// Names seen in a renameable (dotted / unquoted-key) position, in
    /// first-seen source order (deterministic fresh-name assignment).
    dotted_order: Vec<String>,
    dotted_seen: HashSet<String>,
    /// Names seen in a quoted position (`obj["x"]` / `{ "x": 1 }`) — these
    /// are off-limits and disable the name everywhere.
    quoted: HashSet<String>,
}

impl Classify {
    fn see_dotted(&mut self, name: &str) {
        if self.dotted_seen.insert(name.to_string()) {
            self.dotted_order.push(name.to_string());
        }
    }
    fn see_quoted(&mut self, name: &str) {
        self.quoted.insert(name.to_string());
    }
}

fn rename_properties(
    program: &mut Program,
    do_not_rename: &HashSet<String>,
    nodes_touched: &mut u32,
) -> bool {
    // 1. Classify every property occurrence as dotted (renameable shape)
    //    or quoted (off-limits).
    let mut cls = Classify::default();
    for item in &program.body {
        classify_item(item, &mut cls, nodes_touched);
    }

    // 2. Decide the renames. A property is renameable when it appears
    //    dotted, never quoted, is not a built-in, is not in the externs
    //    do-not-rename set, and is longer than one character.
    let builtins: HashSet<&str> = BUILTIN_PROPERTIES.iter().copied().collect();
    // Fresh names avoid every property name in the program plus the
    // built-ins and externs set (property namespace only — variable names
    // are irrelevant).
    let mut avoid: HashSet<String> = HashSet::new();
    avoid.extend(cls.dotted_seen.iter().cloned());
    avoid.extend(cls.quoted.iter().cloned());
    avoid.extend(BUILTIN_PROPERTIES.iter().map(|s| s.to_string()));
    avoid.extend(do_not_rename.iter().cloned());

    let mut map: HashMap<String, String> = HashMap::new();
    let mut gen = FreshNames::new();
    for name in &cls.dotted_order {
        if cls.quoted.contains(name)
            || builtins.contains(name.as_str())
            || do_not_rename.contains(name)
            || name.len() <= 1
        {
            continue;
        }
        let fresh = gen.next(&avoid);
        avoid.insert(fresh.clone());
        map.insert(name.clone(), fresh);
    }

    if map.is_empty() {
        return false;
    }

    // 3. Rewrite every dotted / unquoted-key occurrence of a renamed name.
    for item in &mut program.body {
        rewrite_item(item, &map);
    }
    true
}

/// Collect **every property name that appears anywhere** in `program` —
/// the property-namespace boundary an externs file declares.
///
/// This is the property-renaming analogue of collecting an externs file's
/// top-level variable/function names (the *value*-namespace boundary). A
/// driver that wants to feed an externs file's properties into a
/// [`RenamePropertiesPass`] `do_not_rename` set walks each externs program
/// through this function and unions the results.
///
/// We return the **union of dotted and quoted occurrences**, deliberately
/// over-collecting:
///
/// * `el.innerHTML` (dotted)        → `innerHTML`
/// * `obj["data-id"]` (quoted)      → `data-id`
/// * `{ onload: f }` (unquoted key) → `onload`
/// * `{ "aria-label": s }` (quoted) → `aria-label`
///
/// Why every occurrence, not just the renameable (dotted) ones? Because an
/// externs file is a *declaration of the external boundary*: any property
/// it names is part of the host/library contract and must be preserved in
/// the program being compiled. Including quoted names too only ever
/// *protects more* — forgoing a rename is never a miscompile, whereas
/// renaming a genuinely external property is. (Computed dynamic keys like
/// `obj[runtimeExpr]` contribute nothing — there is no static name to
/// protect; that access is the author's own contract, exactly as in the
/// pass itself.)
///
/// ```
/// use coding_adventures_closure_pass_rename_properties::collect_property_names;
/// use coding_adventures_javascript_ast::{Program, SourceType};
/// use coding_adventures_javascript_tokens::EsVersion;
/// // An empty externs program declares no property boundary.
/// let empty = Program::new("ext.1".to_string(), EsVersion::Es2025, SourceType::Module);
/// assert!(collect_property_names(&empty).is_empty());
/// ```
pub fn collect_property_names(program: &Program) -> HashSet<String> {
    let mut cls = Classify::default();
    let mut nodes_touched: u32 = 0;
    for item in &program.body {
        classify_item(item, &mut cls, &mut nodes_touched);
    }
    // The union of both buckets: dotted (renameable-shape) and quoted
    // (off-limits-shape) occurrences. As an externs boundary, both kinds
    // are equally external and must be protected.
    let mut names = cls.dotted_seen;
    names.extend(cls.quoted);
    names
}

/// Generates `a`, `b`, …, `z`, `aa`, … skipping reserved words and the
/// caller's `avoid` set.
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

/// Bijective base-26 encoding: 0→a, 25→z, 26→aa, …
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

// ---- classification ------------------------------------------------------

fn classify_item(item: &ProgramItem, cls: &mut Classify, nodes_touched: &mut u32) {
    match item {
        ProgramItem::Declaration(d) => classify_decl(d, cls, nodes_touched),
        ProgramItem::Statement(s) => classify_stmt(s, cls, nodes_touched),
    }
}

fn classify_decl(decl: &Declaration, cls: &mut Classify, nodes_touched: &mut u32) {
    *nodes_touched += 1;
    match decl {
        Declaration::VariableDeclaration(vd) => {
            for d in &vd.declarations {
                if let Some(init) = &d.init {
                    classify_expr(init, cls);
                }
            }
        }
        Declaration::FunctionDeclaration(fd) => {
            for s in &fd.body.body {
                classify_stmt(s, cls, nodes_touched);
            }
        }
    }
}

fn classify_stmt(stmt: &Statement, cls: &mut Classify, nodes_touched: &mut u32) {
    *nodes_touched += 1;
    match stmt {
        Statement::Declaration(d) => classify_decl(d, cls, nodes_touched),
        Statement::Tagged(t) => match t {
            TaggedStatement::ExpressionStatement(es) => classify_expr(&es.expression, cls),
            TaggedStatement::BlockStatement(b) => {
                for s in &b.body {
                    classify_stmt(s, cls, nodes_touched);
                }
            }
            TaggedStatement::IfStatement(is) => {
                classify_expr(&is.test, cls);
                classify_stmt(&is.consequent, cls, nodes_touched);
                if let Some(alt) = &is.alternate {
                    classify_stmt(alt, cls, nodes_touched);
                }
            }
            TaggedStatement::WhileStatement(ws) => {
                classify_expr(&ws.test, cls);
                classify_stmt(&ws.body, cls, nodes_touched);
            }
            TaggedStatement::ForStatement(fs) => {
                if let Some(init) = &fs.init {
                    match init {
                        ForInit::VariableDeclaration(vd) => {
                            for d in &vd.declarations {
                                if let Some(i) = &d.init {
                                    classify_expr(i, cls);
                                }
                            }
                        }
                        ForInit::Expression(e) => classify_expr(e, cls),
                    }
                }
                if let Some(test) = &fs.test {
                    classify_expr(test, cls);
                }
                if let Some(update) = &fs.update {
                    classify_expr(update, cls);
                }
                classify_stmt(&fs.body, cls, nodes_touched);
            }
            TaggedStatement::ReturnStatement(rs) => {
                if let Some(a) = &rs.argument {
                    classify_expr(a, cls);
                }
            }
            TaggedStatement::ThrowStatement(ts) => classify_expr(&ts.argument, cls),
            TaggedStatement::LabeledStatement(ls) => classify_stmt(&ls.body, cls, nodes_touched),
            TaggedStatement::SwitchStatement(ss) => {
                classify_expr(&ss.discriminant, cls);
                for c in &ss.cases {
                    if let Some(test) = &c.test {
                        classify_expr(test, cls);
                    }
                    for s in &c.consequent {
                        classify_stmt(s, cls, nodes_touched);
                    }
                }
            }
            TaggedStatement::BreakStatement(_)
            | TaggedStatement::ContinueStatement(_)
            | TaggedStatement::EmptyStatement(_) => {}
        },
    }
}

fn classify_expr(expr: &Expression, cls: &mut Classify) {
    match expr {
        Expression::Identifier(_)
        | Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::BigIntLiteral(_)
        | Expression::UndefinedLiteral(_) => {}
        Expression::BinaryExpression(be) => {
            classify_expr(&be.left, cls);
            classify_expr(&be.right, cls);
        }
        Expression::LogicalExpression(le) => {
            classify_expr(&le.left, cls);
            classify_expr(&le.right, cls);
        }
        Expression::UnaryExpression(ue) => classify_expr(&ue.argument, cls),
        Expression::AssignmentExpression(ae) => {
            if let AssignmentTarget::MemberExpression(m) = &ae.left {
                classify_member(&m.object, &m.property, m.computed, cls);
            }
            classify_expr(&ae.right, cls);
        }
        Expression::ConditionalExpression(ce) => {
            classify_expr(&ce.test, cls);
            classify_expr(&ce.consequent, cls);
            classify_expr(&ce.alternate, cls);
        }
        Expression::CallExpression(ce) => {
            classify_expr(&ce.callee, cls);
            for a in &ce.arguments {
                classify_expr(a, cls);
            }
        }
        Expression::MemberExpression(m) => classify_member(&m.object, &m.property, m.computed, cls),
        Expression::ArrayExpression(ae) => {
            for el in ae.elements.iter().flatten() {
                classify_expr(el, cls);
            }
        }
        Expression::ObjectExpression(oe) => {
            for prop in &oe.properties {
                if prop.computed {
                    // `{ [expr]: v }` — Phase-1 rejects this at the bridge,
                    // but be defensive: recurse the key, record nothing.
                    if let PropertyKey::Expression(e) = &prop.key {
                        classify_expr(e, cls);
                    }
                } else {
                    match &prop.key {
                        // `{ x: v }` — a renameable dotted key.
                        PropertyKey::Identifier(id) => cls.see_dotted(&id.name),
                        // `{ "x": v }` — a quoted key disables `x`.
                        PropertyKey::StringLiteral(s) => cls.see_quoted(&s.value),
                        // Numeric keys / others — not renameable identifiers.
                        _ => {}
                    }
                }
                classify_expr(&prop.value, cls);
            }
        }
    }
}

fn classify_member(object: &Expression, property: &Expression, computed: bool, cls: &mut Classify) {
    classify_expr(object, cls);
    if computed {
        // `obj["x"]` — a quoted access disables `x`. Any other computed
        // key is a dynamic access (the author's contract responsibility);
        // we still recurse it for nested property accesses.
        if let Expression::StringLiteral(s) = property {
            cls.see_quoted(&s.value);
        } else {
            classify_expr(property, cls);
        }
    } else if let Expression::Identifier(id) = property {
        // `obj.x` — a renameable dotted access.
        cls.see_dotted(&id.name);
    }
}

// ---- rewrite -------------------------------------------------------------

fn rewrite_item(item: &mut ProgramItem, map: &HashMap<String, String>) {
    match item {
        ProgramItem::Declaration(d) => rewrite_decl(d, map),
        ProgramItem::Statement(s) => rewrite_stmt(s, map),
    }
}

fn rewrite_decl(decl: &mut Declaration, map: &HashMap<String, String>) {
    match decl {
        Declaration::VariableDeclaration(vd) => {
            for d in &mut vd.declarations {
                if let Some(init) = &mut d.init {
                    rewrite_expr(init, map);
                }
            }
        }
        Declaration::FunctionDeclaration(fd) => {
            for s in &mut fd.body.body {
                rewrite_stmt(s, map);
            }
        }
    }
}

fn rewrite_stmt(stmt: &mut Statement, map: &HashMap<String, String>) {
    match stmt {
        Statement::Declaration(d) => rewrite_decl(d, map),
        Statement::Tagged(t) => match t {
            TaggedStatement::ExpressionStatement(es) => rewrite_expr(&mut es.expression, map),
            TaggedStatement::BlockStatement(b) => {
                for s in &mut b.body {
                    rewrite_stmt(s, map);
                }
            }
            TaggedStatement::IfStatement(is) => {
                rewrite_expr(&mut is.test, map);
                rewrite_stmt(&mut is.consequent, map);
                if let Some(alt) = &mut is.alternate {
                    rewrite_stmt(alt, map);
                }
            }
            TaggedStatement::WhileStatement(ws) => {
                rewrite_expr(&mut ws.test, map);
                rewrite_stmt(&mut ws.body, map);
            }
            TaggedStatement::ForStatement(fs) => {
                if let Some(init) = &mut fs.init {
                    match init {
                        ForInit::VariableDeclaration(vd) => {
                            for d in &mut vd.declarations {
                                if let Some(i) = &mut d.init {
                                    rewrite_expr(i, map);
                                }
                            }
                        }
                        ForInit::Expression(e) => rewrite_expr(e, map),
                    }
                }
                if let Some(test) = &mut fs.test {
                    rewrite_expr(test, map);
                }
                if let Some(update) = &mut fs.update {
                    rewrite_expr(update, map);
                }
                rewrite_stmt(&mut fs.body, map);
            }
            TaggedStatement::ReturnStatement(rs) => {
                if let Some(a) = &mut rs.argument {
                    rewrite_expr(a, map);
                }
            }
            TaggedStatement::ThrowStatement(ts) => rewrite_expr(&mut ts.argument, map),
            TaggedStatement::LabeledStatement(ls) => rewrite_stmt(&mut ls.body, map),
            TaggedStatement::SwitchStatement(ss) => {
                rewrite_expr(&mut ss.discriminant, map);
                for c in &mut ss.cases {
                    if let Some(test) = &mut c.test {
                        rewrite_expr(test, map);
                    }
                    for s in &mut c.consequent {
                        rewrite_stmt(s, map);
                    }
                }
            }
            TaggedStatement::BreakStatement(_)
            | TaggedStatement::ContinueStatement(_)
            | TaggedStatement::EmptyStatement(_) => {}
        },
    }
}

fn rewrite_expr(expr: &mut Expression, map: &HashMap<String, String>) {
    match expr {
        Expression::Identifier(_)
        | Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::BigIntLiteral(_)
        | Expression::UndefinedLiteral(_) => {}
        Expression::BinaryExpression(be) => {
            rewrite_expr(&mut be.left, map);
            rewrite_expr(&mut be.right, map);
        }
        Expression::LogicalExpression(le) => {
            rewrite_expr(&mut le.left, map);
            rewrite_expr(&mut le.right, map);
        }
        Expression::UnaryExpression(ue) => rewrite_expr(&mut ue.argument, map),
        Expression::AssignmentExpression(ae) => {
            if let AssignmentTarget::MemberExpression(m) = &mut ae.left {
                rewrite_member(&mut m.object, &mut m.property, m.computed, map);
            }
            rewrite_expr(&mut ae.right, map);
        }
        Expression::ConditionalExpression(ce) => {
            rewrite_expr(&mut ce.test, map);
            rewrite_expr(&mut ce.consequent, map);
            rewrite_expr(&mut ce.alternate, map);
        }
        Expression::CallExpression(ce) => {
            rewrite_expr(&mut ce.callee, map);
            for a in &mut ce.arguments {
                rewrite_expr(a, map);
            }
        }
        Expression::MemberExpression(m) => {
            rewrite_member(&mut m.object, &mut m.property, m.computed, map)
        }
        Expression::ArrayExpression(ae) => {
            for el in ae.elements.iter_mut().flatten() {
                rewrite_expr(el, map);
            }
        }
        Expression::ObjectExpression(oe) => {
            for prop in &mut oe.properties {
                if prop.computed {
                    if let PropertyKey::Expression(e) = &mut prop.key {
                        rewrite_expr(e, map);
                    }
                } else if let PropertyKey::Identifier(id) = &mut prop.key {
                    // Rewrite a renameable unquoted key; a `StringLiteral`
                    // (quoted) key is never in `map`, so it is left alone.
                    if let Some(new) = map.get(&id.name) {
                        id.name = new.clone();
                    }
                }
                rewrite_expr(&mut prop.value, map);
            }
        }
    }
}

fn rewrite_member(
    object: &mut Expression,
    property: &mut Expression,
    computed: bool,
    map: &HashMap<String, String>,
) {
    rewrite_expr(object, map);
    if computed {
        // A quoted `obj["x"]` is never renamed (the name was disabled at
        // classification); a dynamic key is recursed for nested accesses.
        rewrite_expr(property, map);
    } else if let Expression::Identifier(id) = property {
        if let Some(new) = map.get(&id.name) {
            id.name = new.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    //! Source → bridge → rename-properties → emit roundtrips, plus the
    //! metadata contract.
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

    fn rename_with(src: &str, externs: &[&str]) -> String {
        let es = EsVersion::Es2025;
        let node = parse_javascript_typed(src, es).expect("parse");
        let prog = bridge::grammar_to_program(&node, es).expect("bridge");
        let set: HashSet<String> = externs.iter().map(|s| s.to_string()).collect();
        let pass = RenamePropertiesPass::new(set);
        let sidecar = Sidecar::new();
        let mut cv = CVLog::new(false);
        let out = pass
            .run(PassContext {
                program: &prog,
                sidecar: &sidecar,
                cv: &mut cv,
            })
            .expect("rename-properties");
        let mut cv2 = CVLog::new(false);
        let opts = EmitOptions {
            source_map: false,
            ..Default::default()
        };
        emit(&out.program, &sidecar, &mut cv2, &opts)
            .expect("emit")
            .code
    }

    fn rename(src: &str) -> String {
        rename_with(src, &[])
    }

    // ----- metadata -----

    #[test]
    fn name_is_rename_properties() {
        assert_eq!(
            RenamePropertiesPass::with_builtins_only().name(),
            "rename-properties"
        );
    }

    #[test]
    fn iteration_policy_is_one_shot() {
        assert_eq!(
            RenamePropertiesPass::with_builtins_only().iteration_policy(),
            IterationPolicy::OneShot
        );
    }

    #[test]
    fn cost_is_three() {
        assert_eq!(RenamePropertiesPass::with_builtins_only().cost(), 3);
    }

    #[test]
    fn depends_on_is_empty() {
        assert!(RenamePropertiesPass::with_builtins_only()
            .depends_on()
            .is_empty());
    }

    #[test]
    fn run_on_empty_program_is_identity() {
        let pass = RenamePropertiesPass::with_builtins_only();
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
        let _a: RenamePropertiesPass = Default::default();
        let _b = RenamePropertiesPass::with_builtins_only();
        let _c = _b.clone();
    }

    // ----- behaviour -----

    // NOTE on inputs: a member-ASSIGNMENT statement (`obj.x = 1;`) is not
    // in the Phase-1 grammar, so these tests drive property accesses via
    // member READS (call args / initializers) and object literals.

    #[test]
    fn renames_a_dotted_property_consistently() {
        // `renderMode` appears dotted on two different objects and as an
        // unquoted key; all three become the same fresh name.
        assert_eq!(
            rename("read(a.renderMode); read(b.renderMode); var c = { renderMode: 3 };"),
            "read(a.a);read(b.a);var c={a:3};"
        );
    }

    #[test]
    fn does_not_rename_a_property_quoted_via_computed_member() {
        // `mode` appears quoted (`other["mode"]`, a computed string member
        // — which the bridge preserves as a StringLiteral). That disables
        // `mode` everywhere, even at the dotted `obj.mode` (renaming would
        // desync it from the quoted access we never touch).
        assert_eq!(
            rename("read(obj.mode); read(other[\"mode\"]);"),
            "read(obj.mode);read(other[\"mode\"]);"
        );
    }

    #[test]
    fn does_not_rename_builtins() {
        // `length` / `push` / `toString` are built-ins → never renamed.
        // The user property `tally` (an object key) IS renamed.
        assert_eq!(
            rename("var n = arr.length; list.push(x); s.toString(); var o = { tally: 1 };"),
            "var n=arr.length;list.push(x);s.toString();var o={a:1};"
        );
    }

    #[test]
    fn does_not_rename_externs_property() {
        // `apiField` is supplied as an externs property → kept; the
        // private `helperField` is renamed.
        assert_eq!(
            rename_with("read(obj.apiField); read(obj.helperField);", &["apiField"]),
            "read(obj.apiField);read(obj.a);"
        );
    }

    #[test]
    fn renames_a_computed_member_object_but_not_the_dynamic_key() {
        // `obj[idx]` — `idx` is a variable (dynamic), not a property name,
        // so it is left alone; the dotted `obj.field` IS renamed. (`idx`
        // never appears dotted, so it is not a property at all.)
        assert_eq!(
            rename("var v = obj[idx]; read(obj.field);"),
            "var v=obj[idx];read(obj.a);"
        );
    }

    #[test]
    fn skips_single_char_property() {
        // Already minimal.
        assert_eq!(
            rename("read(obj.x); read(obj.x);"),
            "read(obj.x);read(obj.x);"
        );
    }

    #[test]
    fn renames_nested_property_chain() {
        // Both links of `a.outerField.innerField` are renameable; the
        // outer object's property (`outerField`) is seen first → `a`,
        // then `innerField` → `b`.
        assert_eq!(rename("read(a.outerField.innerField);"), "read(a.a.b);");
    }

    // ----- collect_property_names (externs property boundary) -----

    /// Parse `src` and collect its property names — the helper a driver
    /// uses to turn an externs file into a `do_not_rename` set.
    fn collect(src: &str) -> HashSet<String> {
        let es = EsVersion::Es2025;
        let node = parse_javascript_typed(src, es).expect("parse");
        let prog = bridge::grammar_to_program(&node, es).expect("bridge");
        collect_property_names(&prog)
    }

    #[test]
    fn collect_empty_program_is_empty() {
        assert!(collect_property_names(&program()).is_empty());
    }

    #[test]
    fn collect_dotted_member_read() {
        // `el.innerHTML` — a dotted access names `innerHTML` as external.
        let names = collect("read(el.innerHTML);");
        assert!(names.contains("innerHTML"));
    }

    #[test]
    fn collect_quoted_member_read() {
        // `obj["data-id"]` — a quoted access still names `data-id`. As an
        // externs boundary we protect it (over-collecting is always safe).
        let names = collect("read(obj[\"data-id\"]);");
        assert!(names.contains("data-id"));
    }

    #[test]
    fn collect_unquoted_object_key() {
        // `{ onload: f }` — an unquoted key names `onload`.
        let names = collect("var handlers = { onload: cb };");
        assert!(names.contains("onload"));
    }

    #[test]
    fn collect_quoted_object_key() {
        // `{ "aria-label": s }` — a quoted key names `aria-label`.
        let names = collect("var attrs = { \"aria-label\": label };");
        assert!(names.contains("aria-label"));
    }

    #[test]
    fn collect_unions_multiple_occurrences() {
        // Dotted + quoted + object-key occurrences all land in one set.
        let names =
            collect("read(el.innerHTML); read(node[\"textContent\"]); var o = { onclick: h };");
        assert!(names.contains("innerHTML"));
        assert!(names.contains("textContent"));
        assert!(names.contains("onclick"));
    }

    #[test]
    fn collect_ignores_dynamic_computed_key() {
        // `obj[runtimeKey]` has no static name — there is nothing to
        // protect, so it contributes nothing to the boundary. (`prefix`
        // is still collected from the dotted access.)
        let names = collect("read(obj[runtimeKey]); read(obj.prefix);");
        assert!(names.contains("prefix"));
        assert!(!names.contains("runtimeKey"));
    }

    #[test]
    fn collect_walks_into_function_bodies() {
        // Property accesses nested inside a function declaration are still
        // part of the externs boundary.
        let names = collect("function api(x){ return x.payload; }");
        assert!(names.contains("payload"));
    }

    #[test]
    fn collected_externs_protect_a_property_from_rename() {
        // End-to-end intent: feeding collected externs names into the pass
        // keeps those properties while still renaming program-private ones.
        let externs = collect("read(boundary.innerHTML);");
        let externs_vec: Vec<&str> = externs.iter().map(|s| s.as_str()).collect();
        // `innerHTML` is in the boundary → kept; `secretField` is private
        // → renamed to a short name.
        let out = rename_with(
            "read(node.innerHTML); read(node.secretField); read(node.secretField);",
            &externs_vec,
        );
        assert!(out.contains(".innerHTML"));
        assert!(!out.contains("secretField"));
    }
}
