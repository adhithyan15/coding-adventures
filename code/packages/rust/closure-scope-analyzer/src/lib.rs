//! Lexical scope and symbol-table analyzer for the Closure Compiler clone.
//!
//! ## Why this crate exists
//!
//! Five Phase-1 optimisation passes need to know *where* every name is
//! bound and *which* references resolve to which binding:
//!
//! | Pass                          | Needs from this crate                                                                                           |
//! |-------------------------------|-----------------------------------------------------------------------------------------------------------------|
//! | `closure-pass-rename`         | Every binding's containing scope (so the name-shortening assignment doesn't collide with a name in scope).      |
//! | `closure-pass-inline`         | Whether the callee's free variables are reachable from the call site.                                           |
//! | `closure-pass-treeshake`      | Which top-level declarations are unreferenced anywhere.                                                         |
//! | `closure-pass-collapse-properties` | Which property accesses can be safely flattened into a synthetic binding.                                   |
//! | `closure-pass-remove-unused-vars` | Which locals have zero use sites.                                                                          |
//!
//! Building this analysis once and reusing it across all five passes
//! beats every pass re-walking the AST to build its own ad-hoc symbol
//! table.  More importantly, it **unblocks the five passes as parallel
//! work streams** — each can land independently once the shared
//! contract here is stable.
//!
//! ## What the analysis produces
//!
//! ```text
//!     analyze(&Program) -> ScopeAnalysis
//!
//!     ScopeAnalysis
//!     ├── scopes:   Vec<Scope>           // every block / function / global scope
//!     ├── bindings: Vec<Binding>         // every declared name
//!     └── references: Vec<Reference>     // every Identifier use site
//! ```
//!
//! Scopes form a tree rooted at the global scope.  Bindings belong to
//! exactly one scope.  References point at exactly one binding (or are
//! marked `unresolved` when the lookup walks past the global scope —
//! that's how we detect references to free globals like `console`).
//!
//! ## What this crate does NOT do (yet)
//!
//! The v0.1.0 scaffold deliberately ships an **identity-style empty
//! `analyze`** that returns the global scope and nothing else.  The
//! types, the public surface, and the contract are stable; the
//! traversal-and-resolution body is the follow-up work tracked under
//! CLOC13.0.  This split is intentional — it lets the five consumer
//! passes (CLOC13.A through CLOC13.E) start their real-body work in
//! parallel against a frozen API, instead of every pass waiting on
//! the analyzer's full implementation.
//!
//! ## Identifier hygiene
//!
//! Scope IDs and binding IDs are newtype-wrapped `u32`s.  We don't use
//! pointer identity for two reasons:
//!
//! 1. We want the analysis to be serialisable (think: dumping it to a
//!    sidecar JSON for the CV pipeline).
//! 2. Pass crates shouldn't have to hold a `&Program` borrow for the
//!    entire pass — they should be able to walk the analysis and then
//!    walk the program afterward to apply changes.
//!
//! ## Per-CV correlation
//!
//! Each `Binding.declared_at` and `Reference.cv` is an
//! `Option<CvId>` so that downstream emitters and CV writers can
//! correlate a renamed identifier back to its source position.  When
//! CV tracing is off (the common production case), these stay `None`
//! and the per-node memory is just a word.

use coding_adventures_javascript_ast::statement::TaggedStatement;
use coding_adventures_javascript_ast::{
    ArrowBody, ArrowFunctionExpression,
    AssignmentTarget, BindingTarget, BlockStatement, ClassDeclaration, ClassMember, CvId, Declaration, Expression, ForInit,
    FunctionDeclaration, FunctionExpression, FunctionParam, ObjectMember, Program, ProgramItem, Property, PropertyKey, Statement,
    VarKind, VariableDeclaration,
};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------
// IDs — newtype wrappers around dense indices into ScopeAnalysis.scopes
// / ScopeAnalysis.bindings.
// ---------------------------------------------------------------------

/// An opaque handle into [`ScopeAnalysis::scopes`].  The global scope
/// is always [`ScopeId::GLOBAL`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ScopeId(pub u32);

impl ScopeId {
    /// The root scope of every program.  Reserved.
    pub const GLOBAL: ScopeId = ScopeId(0);
}

/// An opaque handle into [`ScopeAnalysis::bindings`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BindingId(pub u32);

// ---------------------------------------------------------------------
// Scope — one entry per lexical scope in the program
// ---------------------------------------------------------------------

/// One lexical scope.  Forms a parent-pointer tree rooted at
/// [`ScopeId::GLOBAL`].  Every binding belongs to exactly one scope;
/// nested scopes do NOT inherit their parent's bindings — name
/// resolution explicitly walks up the parent chain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scope {
    /// What kind of scope.  Block scopes hold `let` / `const`;
    /// function scopes also hold `var` and params.  Global is the
    /// outermost wrapper.
    pub kind: ScopeKind,
    /// The enclosing scope.  `None` only for [`ScopeId::GLOBAL`].
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub parent: Option<ScopeId>,
    /// Bindings declared directly in this scope.
    pub bindings: Vec<BindingId>,
}

/// What kind of scope.  Matches the three ECMAScript scope kinds we
/// need for Phase 1 passes.
///
/// `#[non_exhaustive]` so future variants (modules, `with` blocks if
/// we ever support them, catch-clause scopes) don't break exhaustive
/// matches in the five consumer pass crates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum ScopeKind {
    /// The program top level.  Bindings declared here are reachable
    /// from every nested scope.
    Global,
    /// A `function f(…) { … }` or `function() { … }` body.  Hosts
    /// `var` declarations (hoisted to the function's start) and the
    /// function's parameters.
    Function,
    /// A `{ … }` body anywhere a statement can appear.  Hosts `let`,
    /// `const`, and inner `function` declarations.
    Block,
}

// ---------------------------------------------------------------------
// Binding — one entry per declared name
// ---------------------------------------------------------------------

/// One declared name.  The pass crates look these up to decide:
///
/// - rename: can I shorten `name` without colliding?
/// - inline: does the callee reference any binding I can't reach?
/// - treeshake / remove-unused-vars: does any [`Reference`] point at me?
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Binding {
    /// The source-level identifier text.  Used for collision checks
    /// and for the rename pass's "don't replace exported names" rule.
    pub name: String,
    /// What kind of declaration introduced this binding.
    pub kind: BindingKind,
    /// The scope this binding lives in.
    pub scope: ScopeId,
    /// CV id of the declaration site, when tracing is on.  Lets the
    /// emitter and CV writer correlate renamed bindings back to source.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub declared_at: Option<CvId>,
}

/// What kind of declaration introduced a binding.  Determines scope
/// rules: `Var` / `Function` hoist to the enclosing function scope;
/// everything else stays in its declaring block.
///
/// `#[non_exhaustive]` so future variants (e.g., for-of loop
/// bindings, catch-clause bindings, import bindings when the
/// module-graph crate lands) don't break exhaustive matches in the
/// five consumer pass crates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum BindingKind {
    /// `var x = …`.  Function-scoped, hoisted to the top of its
    /// containing function.
    Var,
    /// `let x = …`.  Block-scoped, TDZ until the declaration.
    Let,
    /// `const x = …`.  Block-scoped, TDZ until the declaration,
    /// reassignment forbidden.
    Const,
    /// `function f() { … }`.  Function-scoped name binding in
    /// non-strict mode; block-scoped in strict mode (annex B).
    Function,
    /// `class C { … }`.  Block-scoped per spec.  Not in the v0.1.0
    /// AST yet, but reserved for the Phase 1.x extension.
    Class,
    /// A function parameter.  Lives in the function scope.
    Param,
}

// ---------------------------------------------------------------------
// Reference — one entry per Identifier use site
// ---------------------------------------------------------------------

/// One identifier use site, with its resolved binding (or `None` for
/// unresolved / global references).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reference {
    /// The source-level identifier text.  Same as the referenced
    /// binding's name (or the free-global name when unresolved).
    pub name: String,
    /// Which scope the reference is read FROM.  Resolution walks the
    /// parent chain starting here.
    pub from_scope: ScopeId,
    /// The binding this reference resolves to.  `None` means the
    /// lookup walked past the global scope without finding a match —
    /// the name refers to a free global (e.g. `console`, `window`).
    /// The treeshake / remove-unused-vars passes treat `None`-resolved
    /// references as "definitely used externally".
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub binding: Option<BindingId>,
    /// CV id of the reference site, when tracing is on.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub cv: Option<CvId>,
}

// ---------------------------------------------------------------------
// ScopeAnalysis — the public output of [`analyze`]
// ---------------------------------------------------------------------

/// The full lexical-scope analysis of a [`Program`].  Built by
/// [`analyze`]; consumed by the five Phase-1 optimisation passes.
///
/// Look up a scope by `analysis.scopes[id.0 as usize]`, a binding by
/// `analysis.bindings[id.0 as usize]`.  Scope IDs and binding IDs are
/// stable for the lifetime of the analysis.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopeAnalysis {
    pub scopes: Vec<Scope>,
    pub bindings: Vec<Binding>,
    pub references: Vec<Reference>,
}

impl ScopeAnalysis {
    /// Look up a binding by name starting from `from`, walking the
    /// parent chain.  Returns the first match (the innermost
    /// shadowing binding wins, per ECMAScript) or `None` if the name
    /// is a free global.
    ///
    /// This is a convenience for passes that want point lookups;
    /// the pre-resolved [`Reference`] list in
    /// [`ScopeAnalysis::references`] is the right tool when scanning
    /// every use site.
    pub fn resolve(&self, name: &str, from: ScopeId) -> Option<BindingId> {
        let mut current = Some(from);
        while let Some(scope_id) = current {
            let scope = &self.scopes[scope_id.0 as usize];
            for binding_id in &scope.bindings {
                if self.bindings[binding_id.0 as usize].name == name {
                    return Some(*binding_id);
                }
            }
            current = scope.parent;
        }
        None
    }
}

// ---------------------------------------------------------------------
// analyze — the entry point
// ---------------------------------------------------------------------

/// Build a [`ScopeAnalysis`] for a program.
///
/// **v0.2.0 — CLOC13.0 minimal body.**  Walks `program.body` and
/// surfaces *top-level* `var` / `let` / `const` / `function`
/// declarations as [`Binding`]s in the [`ScopeId::GLOBAL`] scope.
/// The signature still matches the v0.1.0 contract; CLOC13.A..E
/// consumers see exactly the same API surface.
///
/// ### What this body covers (v0.2.0)
///
/// - **Top-level `VariableDeclaration`** (`var` / `let` / `const`).
///   One [`Binding`] per [`VariableDeclarator`].  Destructuring
///   patterns are gated by [`BindingTarget`] — Phase 1 only ships
///   the `Identifier` variant, so the match is total today.
/// - **Top-level `FunctionDeclaration`**.  One [`Binding`] with
///   [`BindingKind::Function`].
/// - All bindings land in [`ScopeId::GLOBAL`].  No nested scope is
///   created yet — see "Deferred" below.
///
/// ### Deferred to CLOC13.0.1 (and tracked inline)
///
/// 1. **Function body scopes.**  A [`FunctionDeclaration`] should
///    create a [`ScopeKind::Function`] child scope holding its
///    [`FunctionParam`]s + nested `var`/`let`/`const`/function
///    decls.  Today we only emit the function's name binding in
///    GLOBAL.
/// 2. **Block scopes.**  `let` and `const` inside a
///    [`BlockStatement`] should land in a [`ScopeKind::Block`]
///    child of the enclosing scope.  Today we ignore nested
///    blocks entirely.
/// 3. **Var hoisting.**  A `var x` inside a block must bind in
///    the enclosing *function* scope, not the block.  Pattern when
///    block scopes land: pre-walk the function body to collect
///    `var` declarations, emit them against the function scope,
///    then walk normally.
/// 4. **`References`**.  Identifier use sites — every
///    [`Expression::Identifier`] node, plus assignment targets and
///    computed member keys — produce a [`Reference`], resolved to a
///    binding via the parent-chain walk (or `None` for free globals).
///    This is what lets `remove-unused-vars` / `inline` gate on
///    `uses == 0` or `uses == 1`.  (Implemented since CLOC13.0.1; the
///    earlier "emits zero references" gap is closed.)
/// 5. **Catch-clause scope** (not in Phase 1 AST yet).
/// 6. **Strict-mode binding semantics** (function-in-block scope).
///
/// ### Why this minimal split
///
/// The five CLOC13.A..E pass bodies all wired their candidate
/// scans to consume `bindings` (Const-only, Function/Class,
/// every-binding, etc.).  Returning a populated `bindings` vec —
/// even without references — activates the *kind-filter* half of
/// every wired pass.  Adding references is mechanical and lands
/// in CLOC13.0.1 as the second step.
///
/// **`changed` semantics in consumer passes is unchanged.**  The
/// CLOC13.A..E bodies still hard-pin `changed = false` until
/// *their* step-3 apply lands.  Lighting up real bindings doesn't
/// change that — it just makes their candidate scans non-empty.
///
/// The signature still matches the v0.1.0 contract; consumers
/// don't need to recompile against a new API.
/// A pending reference whose binding hasn't been resolved yet.
/// During the walk we know the name and from-scope but can't call
/// `analysis.resolve()` (which reads from `analysis.scopes` /
/// `analysis.bindings`) because we're still mutating them. We
/// collect pending references during the walk, then resolve them
/// all at the end.
struct PendingReference {
    name: String,
    from_scope: ScopeId,
    cv: Option<CvId>,
}

/// Walker state threaded through the recursive walk.
///
/// - `current`: the scope identifiers inside this lexical region
///   resolve from. Updated when we enter a Function or Block.
/// - `enclosing_function`: the nearest Function-kind ancestor (or
///   `GLOBAL` if we're at the top level / inside a top-level
///   block). `var` declarations hoist here.
#[derive(Copy, Clone)]
struct WalkCtx {
    current: ScopeId,
    enclosing_function: ScopeId,
}

pub fn analyze(program: &Program) -> ScopeAnalysis {
    // CLOC13.0.2 algorithm — single recursive walk over the AST
    // building scopes + bindings, with deferred reference
    // resolution.
    //
    // ## Scope rules
    //
    // - `var x`: binding hoists to the enclosing function scope
    //   (`WalkCtx.enclosing_function`). Visible from the start of
    //   that function (TDZ-free).
    // - `let x` / `const x`: binding lives in the *current* scope
    //   (`WalkCtx.current`). Block-scoped; TDZ until declaration
    //   (analyzer doesn't model TDZ — declaration order in the
    //   AST is enough for downstream passes).
    // - `function f` declaration: the *name* binding hoists to
    //   the enclosing scope (`WalkCtx.current`) as kind=Function.
    //   The function body gets a NEW Function scope as a child;
    //   that scope holds Param bindings + the function's own
    //   var/let/const/function declarations.
    // - `class C` declaration: name binding in `current` as
    //   kind=Class. Not in the AST yet (CLOC09 Phase 1.x).
    // - `BlockStatement` (other than a function body): creates a
    //   new Block scope as a child of `current`. `let`/`const`
    //   inside land there; `var` still hoists out to
    //   `enclosing_function`.
    //
    // ## What's NOT walked for references
    //
    // - Binding declaration sites (VariableDeclarator.id,
    //   FunctionDeclaration.id, FunctionParam).
    // - Non-computed MemberExpression `property` (a property
    //   name, not a binding lookup).
    // - Non-computed ObjectExpression `Property.key` (same).
    // - LabeledStatement.label / Break/Continue.label (label
    //   reference, not a binding lookup).
    //
    // ## Two-phase implementation
    //
    // We can't call `analysis.resolve()` mid-walk because that
    // reads from `analysis.scopes`/`analysis.bindings` which
    // we're still mutating. So:
    //
    //   Phase 1: walk recursively, push Bindings to
    //   `analysis.bindings` AND collect `PendingReference`s in
    //   a tmp Vec.
    //   Phase 2: for each pending ref, call
    //   `analysis.resolve(name, from_scope)` to get the binding,
    //   then push a final `Reference` to `analysis.references`.
    //
    // Resolution walks the parent chain, which is exactly what
    // we want for nested-scope lookups across function/block
    // boundaries.

    let mut analysis = ScopeAnalysis {
        scopes: vec![Scope {
            kind: ScopeKind::Global,
            parent: None,
            bindings: Vec::new(),
        }],
        bindings: Vec::new(),
        references: Vec::new(),
    };

    let mut pending: Vec<PendingReference> = Vec::new();

    let ctx = WalkCtx {
        current: ScopeId::GLOBAL,
        enclosing_function: ScopeId::GLOBAL,
    };

    // Walk every top-level item.
    for item in &program.body {
        walk_program_item(item, ctx, &mut analysis, &mut pending);
    }

    // Phase 2: resolve all pending references.
    for p in pending {
        let binding = analysis.resolve(&p.name, p.from_scope);
        analysis.references.push(Reference {
            name: p.name,
            from_scope: p.from_scope,
            binding,
            cv: p.cv,
        });
    }

    analysis
}

/// Emit a binding into the given scope and update both the
/// flat `analysis.bindings` table and the per-scope binding list.
fn emit_binding(
    name: String,
    kind: BindingKind,
    scope: ScopeId,
    declared_at: Option<CvId>,
    analysis: &mut ScopeAnalysis,
) -> BindingId {
    let id = BindingId(analysis.bindings.len() as u32);
    analysis.bindings.push(Binding {
        name,
        kind,
        scope,
        declared_at,
    });
    analysis.scopes[scope.0 as usize].bindings.push(id);
    id
}

/// Allocate a new scope as a child of `parent`. Returns its id.
fn emit_scope(kind: ScopeKind, parent: ScopeId, analysis: &mut ScopeAnalysis) -> ScopeId {
    let id = ScopeId(analysis.scopes.len() as u32);
    analysis.scopes.push(Scope {
        kind,
        parent: Some(parent),
        bindings: Vec::new(),
    });
    id
}

fn walk_program_item(
    item: &ProgramItem,
    ctx: WalkCtx,
    analysis: &mut ScopeAnalysis,
    pending: &mut Vec<PendingReference>,
) {
    match item {
        ProgramItem::Statement(stmt) => walk_statement(stmt, ctx, analysis, pending),
        ProgramItem::Declaration(decl) => walk_declaration(decl, ctx, analysis, pending),
    }
}

fn walk_declaration(
    decl: &Declaration,
    ctx: WalkCtx,
    analysis: &mut ScopeAnalysis,
    pending: &mut Vec<PendingReference>,
) {
    match decl {
        Declaration::VariableDeclaration(vd) => walk_variable_declaration(vd, ctx, analysis, pending),
        Declaration::FunctionDeclaration(fd) => walk_function_declaration(fd, ctx, analysis, pending),
        Declaration::ClassDeclaration(cd) => walk_class_declaration(cd, ctx, analysis, pending),
    }
}

/// Walk a class *declaration* (`class C [extends S] { … }`). Two jobs, mirroring
/// the split between [`walk_function_declaration`] (the name binding) and the
/// `Expression::ClassExpression` arm of [`walk_expression`] (the body):
///
/// 1. **Bind the class name.** A class declaration introduces a
///    [`BindingKind::Class`] binding for `cd.id` in the current scope — the
///    lexical analogue of the `Function`-kind binding a function declaration
///    hoists. This is what lets a later reference to `C` resolve (and a renaming
///    pass rename the class consistently).
/// 2. **Resolve inside the body.** The `extends` heritage is an ordinary
///    expression evaluated in the enclosing scope; each method `value` is a
///    function expression walked as its own function scope — identical to the
///    class-expression handling.
fn walk_class_declaration(
    cd: &ClassDeclaration,
    ctx: WalkCtx,
    analysis: &mut ScopeAnalysis,
    pending: &mut Vec<PendingReference>,
) {
    emit_binding(
        cd.id.name.clone(),
        BindingKind::Class,
        ctx.current,
        cd.id.cv.clone(),
        analysis,
    );
    if let Some(sup) = &cd.super_class {
        walk_expression(sup, ctx, analysis, pending);
    }
    for member in &cd.body {
        match member {
            ClassMember::Method(m) => walk_function_expression(&m.value, ctx, analysis, pending),
            // A field initializer runs at construction in the class scope —
            // resolve references in it. The field *key* introduces no binding.
            ClassMember::Field(f) => {
                if let Some(v) = &f.value {
                    walk_expression(v, ctx, analysis, pending);
                }
            }
            // A static-init block runs at class-definition time and is its own
            // block scope — walk it as a free-standing block statement so its
            // local `let`/`const`/`var` land in a block scope and references
            // resolve. It introduces no member key or binding name.
            ClassMember::StaticBlock(b) => walk_block_statement(b, ctx, analysis, pending),
        }
    }
}

fn walk_variable_declaration(
    vd: &VariableDeclaration,
    ctx: WalkCtx,
    analysis: &mut ScopeAnalysis,
    pending: &mut Vec<PendingReference>,
) {
    // `var` hoists; `let`/`const` stay in the current scope.
    let (binding_kind, target_scope) = match vd.kind {
        VarKind::Var => (BindingKind::Var, ctx.enclosing_function),
        VarKind::Let => (BindingKind::Let, ctx.current),
        VarKind::Const => (BindingKind::Const, ctx.current),
    };

    for declarator in &vd.declarations {
        let BindingTarget::Identifier(id) = &declarator.id;
        emit_binding(id.name.clone(), binding_kind, target_scope, id.cv.clone(), analysis);
        if let Some(init) = &declarator.init {
            walk_expression(init, ctx, analysis, pending);
        }
    }
}

fn walk_function_declaration(
    fd: &FunctionDeclaration,
    ctx: WalkCtx,
    analysis: &mut ScopeAnalysis,
    pending: &mut Vec<PendingReference>,
) {
    // Function name hoists to the enclosing scope as a
    // Function-kind binding. The body gets its own Function
    // scope.
    emit_binding(
        fd.id.name.clone(),
        BindingKind::Function,
        ctx.current,
        fd.id.cv.clone(),
        analysis,
    );

    let function_scope = emit_scope(ScopeKind::Function, ctx.current, analysis);

    // Params become Param-kind bindings in the function scope.
    for param in &fd.params {
        let FunctionParam::Identifier(id) = param;
        emit_binding(
            id.name.clone(),
            BindingKind::Param,
            function_scope,
            id.cv.clone(),
            analysis,
        );
    }

    // Walk the body inside the new function scope. The body is a
    // BlockStatement, but per spec it's the function's own scope
    // — NOT a fresh Block child. So we walk the body's
    // statements directly rather than calling walk_block_statement
    // (which would create another Block scope).
    let inner_ctx = WalkCtx {
        current: function_scope,
        enclosing_function: function_scope,
    };
    for stmt in &fd.body.body {
        walk_statement(stmt, inner_ctx, analysis, pending);
    }
}

/// Walk a `FunctionExpression`. Mirrors [`walk_function_declaration`]
/// with one deliberate difference rooted in JS scoping semantics:
///
/// A function *declaration* hoists its name into the **enclosing**
/// scope. A named function *expression*'s name is **body-local** — it
/// binds only inside the function's own scope, so the function can refer
/// to itself for recursion (`var f = function rec(n){ return rec(n-1); }`)
/// without leaking `rec` outward. So we create the function scope first
/// and, when the expression is named, bind the name *inside* that scope
/// (not the enclosing one). Anonymous expressions bind no name at all.
///
/// Getting this right matters for renaming soundness: a body reference to
/// the function's own name must resolve to this local binding, never to a
/// free/global of the same spelling.
fn walk_function_expression(
    fe: &FunctionExpression,
    ctx: WalkCtx,
    analysis: &mut ScopeAnalysis,
    pending: &mut Vec<PendingReference>,
) {
    let function_scope = emit_scope(ScopeKind::Function, ctx.current, analysis);

    // Named function expression: the name is visible ONLY inside the
    // body, bound in the function's own scope.
    if let Some(id) = &fe.id {
        emit_binding(
            id.name.clone(),
            BindingKind::Function,
            function_scope,
            id.cv.clone(),
            analysis,
        );
    }

    for param in &fe.params {
        let FunctionParam::Identifier(id) = param;
        emit_binding(
            id.name.clone(),
            BindingKind::Param,
            function_scope,
            id.cv.clone(),
            analysis,
        );
    }

    let inner_ctx = WalkCtx {
        current: function_scope,
        enclosing_function: function_scope,
    };
    for stmt in &fe.body.body {
        walk_statement(stmt, inner_ctx, analysis, pending);
    }
}

/// Walk an arrow function, introducing its own function scope. Two
/// differences from [`walk_function_expression`]:
///
/// - **No name binding.** Arrows are always anonymous, so there is no
///   body-local `id` to bind (only the params).
/// - **A dual body.** A block body walks its statements; a concise
///   (expression) body walks its single expression. Both do so under the
///   arrow's own scope so a param reference resolves to the param, not to
///   an outer binding of the same name.
///
/// (Arrows do not bind their own `this`/`arguments`, but this analyzer
/// tracks *name* scopes — `var`/`let`/param bindings — for which an arrow
/// behaves exactly like any other function scope.)
fn walk_arrow_function_expression(
    ae: &ArrowFunctionExpression,
    ctx: WalkCtx,
    analysis: &mut ScopeAnalysis,
    pending: &mut Vec<PendingReference>,
) {
    let function_scope = emit_scope(ScopeKind::Function, ctx.current, analysis);

    for param in &ae.params {
        let FunctionParam::Identifier(id) = param;
        emit_binding(
            id.name.clone(),
            BindingKind::Param,
            function_scope,
            id.cv.clone(),
            analysis,
        );
    }

    let inner_ctx = WalkCtx {
        current: function_scope,
        enclosing_function: function_scope,
    };
    match &ae.body {
        ArrowBody::Block(b) => {
            for stmt in &b.body {
                walk_statement(stmt, inner_ctx, analysis, pending);
            }
        }
        ArrowBody::Expression(e) => walk_expression(e, inner_ctx, analysis, pending),
    }
}

fn walk_block_statement(
    block: &BlockStatement,
    ctx: WalkCtx,
    analysis: &mut ScopeAnalysis,
    pending: &mut Vec<PendingReference>,
) {
    // A free-standing BlockStatement (not a function body) gets
    // its own Block scope. `let`/`const` land here; `var`
    // continues to hoist out via WalkCtx.enclosing_function.
    let block_scope = emit_scope(ScopeKind::Block, ctx.current, analysis);
    let inner_ctx = WalkCtx {
        current: block_scope,
        enclosing_function: ctx.enclosing_function,
    };
    for stmt in &block.body {
        walk_statement(stmt, inner_ctx, analysis, pending);
    }
}

fn walk_statement(
    stmt: &Statement,
    ctx: WalkCtx,
    analysis: &mut ScopeAnalysis,
    pending: &mut Vec<PendingReference>,
) {
    match stmt {
        Statement::Tagged(t) => walk_tagged_statement(t, ctx, analysis, pending),
        Statement::Declaration(d) => walk_declaration(d, ctx, analysis, pending),
    }
}

fn walk_tagged_statement(
    stmt: &TaggedStatement,
    ctx: WalkCtx,
    analysis: &mut ScopeAnalysis,
    pending: &mut Vec<PendingReference>,
) {
    match stmt {
        TaggedStatement::ExpressionStatement(es) => {
            walk_expression(&es.expression, ctx, analysis, pending);
        }
        TaggedStatement::BlockStatement(b) => walk_block_statement(b, ctx, analysis, pending),
        TaggedStatement::IfStatement(is) => {
            walk_expression(&is.test, ctx, analysis, pending);
            walk_statement(&is.consequent, ctx, analysis, pending);
            if let Some(alt) = &is.alternate {
                walk_statement(alt, ctx, analysis, pending);
            }
        }
        TaggedStatement::WhileStatement(ws) => {
            walk_expression(&ws.test, ctx, analysis, pending);
            walk_statement(&ws.body, ctx, analysis, pending);
        }
        TaggedStatement::DoWhileStatement(ds) => {
            // A do-while introduces no new scope (same as while); walk the
            // body and the test in the current scope.
            walk_statement(&ds.body, ctx, analysis, pending);
            walk_expression(&ds.test, ctx, analysis, pending);
        }
        TaggedStatement::ForStatement(fs) => {
            if let Some(init) = &fs.init {
                match init {
                    ForInit::VariableDeclaration(vd) => {
                        walk_variable_declaration(vd, ctx, analysis, pending);
                    }
                    ForInit::Expression(e) => walk_expression(e, ctx, analysis, pending),
                }
            }
            if let Some(test) = &fs.test {
                walk_expression(test, ctx, analysis, pending);
            }
            if let Some(update) = &fs.update {
                walk_expression(update, ctx, analysis, pending);
            }
            walk_statement(&fs.body, ctx, analysis, pending);
        }
        TaggedStatement::ForInStatement(fs) => {
            // The for-in `left` declares (or targets) the loop variable; walk
            // it, then the enumerated `right`, then the body.
            match &fs.left {
                ForInit::VariableDeclaration(vd) => {
                    walk_variable_declaration(vd, ctx, analysis, pending);
                }
                ForInit::Expression(e) => walk_expression(e, ctx, analysis, pending),
            }
            walk_expression(&fs.right, ctx, analysis, pending);
            walk_statement(&fs.body, ctx, analysis, pending);
        }
        TaggedStatement::ForOfStatement(fs) => {
            // The for-in `left` declares (or targets) the loop variable; walk
            // it, then the enumerated `right`, then the body.
            match &fs.left {
                ForInit::VariableDeclaration(vd) => {
                    walk_variable_declaration(vd, ctx, analysis, pending);
                }
                ForInit::Expression(e) => walk_expression(e, ctx, analysis, pending),
            }
            walk_expression(&fs.right, ctx, analysis, pending);
            walk_statement(&fs.body, ctx, analysis, pending);
        }
        TaggedStatement::ReturnStatement(rs) => {
            if let Some(arg) = &rs.argument {
                walk_expression(arg, ctx, analysis, pending);
            }
        }
        TaggedStatement::LabeledStatement(ls) => {
            walk_statement(&ls.body, ctx, analysis, pending);
        }
        TaggedStatement::ThrowStatement(ts) => {
            walk_expression(&ts.argument, ctx, analysis, pending);
        }
        TaggedStatement::SwitchStatement(ss) => {
            // Visit the discriminant in the enclosing scope, then
            // each case's test expression and consequent. Per
            // ECMAScript §13.12, switch bodies share a single
            // lexical scope spanning all cases, so we keep `ctx`
            // unchanged across consequents.
            walk_expression(&ss.discriminant, ctx, analysis, pending);
            for case in &ss.cases {
                if let Some(test) = &case.test {
                    walk_expression(test, ctx, analysis, pending);
                }
                for s in &case.consequent {
                    walk_statement(s, ctx, analysis, pending);
                }
            }
        }
        TaggedStatement::TryStatement(ts) => {
            // `try { block } catch (param) { body } finally { fin }`.
            // The protected block and the finalizer are ordinary blocks. The
            // catch clause introduces a `param` binding scoped to the catch
            // body — modeled as a Block scope holding a (block-scoped) binding,
            // walked with the param in scope.
            walk_block_statement(&ts.block, ctx, analysis, pending);
            if let Some(handler) = &ts.handler {
                let catch_scope = emit_scope(ScopeKind::Block, ctx.current, analysis);
                if let Some(param) = &handler.param {
                    emit_binding(
                        param.name.clone(),
                        BindingKind::Let,
                        catch_scope,
                        param.cv.clone(),
                        analysis,
                    );
                }
                let catch_ctx = WalkCtx {
                    current: catch_scope,
                    enclosing_function: ctx.enclosing_function,
                };
                for stmt in &handler.body.body {
                    walk_statement(stmt, catch_ctx, analysis, pending);
                }
            }
            if let Some(finalizer) = &ts.finalizer {
                walk_block_statement(finalizer, ctx, analysis, pending);
            }
        }
        TaggedStatement::BreakStatement(_) => {}
        TaggedStatement::ContinueStatement(_) => {}
        TaggedStatement::EmptyStatement(_) => {}
        // `debugger;` has no children and binds nothing — nothing to analyze.
        TaggedStatement::DebuggerStatement(_) => {}
    }
}

fn walk_expression(
    expr: &Expression,
    ctx: WalkCtx,
    analysis: &mut ScopeAnalysis,
    pending: &mut Vec<PendingReference>,
) {
    match expr {
        Expression::Identifier(id) => {
            pending.push(PendingReference {
                name: id.name.clone(),
                from_scope: ctx.current,
                cv: id.cv.clone(),
            });
        }
        Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::BigIntLiteral(_)
        | Expression::RegExpLiteral(_)
        // `this` binds no lexical name — it is resolved by the runtime, not by
        // scope analysis — so it introduces and references nothing.
        | Expression::ThisExpression(_)
        | Expression::Super(_)
        | Expression::NewTarget(_)
        | Expression::ImportMeta(_)
        | Expression::UndefinedLiteral(_) => {}
        Expression::BinaryExpression(be) => {
            walk_expression(&be.left, ctx, analysis, pending);
            walk_expression(&be.right, ctx, analysis, pending);
        }
        Expression::LogicalExpression(le) => {
            walk_expression(&le.left, ctx, analysis, pending);
            walk_expression(&le.right, ctx, analysis, pending);
        }
        Expression::UnaryExpression(ue) => {
            walk_expression(&ue.argument, ctx, analysis, pending);
        }
        // `++x` / `x++`: the operand is both read AND written, but for scope
        // resolution what matters is only that its identifier is *referenced*
        // in this scope — the same as any other operand walk.
        Expression::UpdateExpression(ue) => {
            walk_expression(&ue.argument, ctx, analysis, pending);
        }
        Expression::AssignmentExpression(ae) => {
            match &ae.left {
                AssignmentTarget::Identifier(id) => {
                    pending.push(PendingReference {
                        name: id.name.clone(),
                        from_scope: ctx.current,
                        cv: id.cv.clone(),
                    });
                }
                AssignmentTarget::MemberExpression(me) => {
                    walk_member_expression_inner(&me.object, &me.property, me.computed, ctx, analysis, pending);
                }
            }
            walk_expression(&ae.right, ctx, analysis, pending);
        }
        Expression::ConditionalExpression(ce) => {
            walk_expression(&ce.test, ctx, analysis, pending);
            walk_expression(&ce.consequent, ctx, analysis, pending);
            walk_expression(&ce.alternate, ctx, analysis, pending);
        }
        Expression::CallExpression(ce) => {
            walk_expression(&ce.callee, ctx, analysis, pending);
            for arg in &ce.arguments {
                walk_expression(arg, ctx, analysis, pending);
            }
        }
        Expression::NewExpression(ne) => {
            walk_expression(&ne.callee, ctx, analysis, pending);
            for arg in &ne.arguments {
                walk_expression(arg, ctx, analysis, pending);
            }
        }
        Expression::SequenceExpression(se) => {
            for e in &se.expressions {
                walk_expression(e, ctx, analysis, pending);
            }
        }
        Expression::MemberExpression(me) => {
            walk_member_expression_inner(&me.object, &me.property, me.computed, ctx, analysis, pending);
        }
        // `a?.b` / `a?.[k]` — the optional short-circuit changes runtime
        // behaviour but not name resolution, so it walks exactly like a plain
        // member access: resolve references in the object and (computed) property.
        Expression::OptionalMemberExpression(me) => {
            walk_member_expression_inner(&me.object, &me.property, me.computed, ctx, analysis, pending);
        }
        // `a?.()` — an optional call resolves references in its callee and each
        // argument, the same as an ordinary call.
        Expression::OptionalCallExpression(ce) => {
            walk_expression(&ce.callee, ctx, analysis, pending);
            for arg in &ce.arguments {
                walk_expression(arg, ctx, analysis, pending);
            }
        }
        // A chain expression is a transparent wrapper around an optional-chain
        // spine — it binds nothing, so walk straight into its inner expression.
        Expression::ChainExpression(c) => {
            walk_expression(&c.expression, ctx, analysis, pending);
        }
        Expression::ArrayExpression(ae) => {
            for el in &ae.elements {
                if let Some(e) = el {
                    walk_expression(e, ctx, analysis, pending);
                }
            }
        }
        Expression::ObjectExpression(oe) => {
            for member in &oe.properties {
                match member {
                    ObjectMember::Property(prop) => {
                        walk_property(prop, ctx, analysis, pending);
                    }
                    // An object spread `...expr` reads `expr` in the current
                    // scope (it binds nothing), so walk its argument like any
                    // other sub-expression.
                    ObjectMember::Spread(s) => {
                        walk_expression(&s.argument, ctx, analysis, pending);
                    }
                }
            }
        }
        Expression::FunctionExpression(fe) => {
            walk_function_expression(fe, ctx, analysis, pending);
        }
        // A class expression: resolve the `extends` operand in the current
        // scope, then walk each method's function value as its own nested
        // function scope (exactly like `FunctionExpression`). The optional
        // class name binding is body-local; not tracking it is conservative
        // (it is simply never chosen as a rename target).
        Expression::ClassExpression(ce) => {
            if let Some(sup) = &ce.super_class {
                walk_expression(sup, ctx, analysis, pending);
            }
            for member in &ce.body {
                match member {
                    ClassMember::Method(m) => {
                        walk_function_expression(&m.value, ctx, analysis, pending)
                    }
                    // A field initializer runs at construction in the class
                    // scope — resolve references in it; the key binds nothing.
                    ClassMember::Field(f) => {
                        if let Some(v) = &f.value {
                            walk_expression(v, ctx, analysis, pending);
                        }
                    }
                    // A static-init block is its own block scope — walk it as a
                    // free-standing block statement (mirrors the declaration form).
                    ClassMember::StaticBlock(b) => {
                        walk_block_statement(b, ctx, analysis, pending)
                    }
                }
            }
        }
        Expression::ArrowFunctionExpression(ae) => {
            walk_arrow_function_expression(ae, ctx, analysis, pending);
        }
        // A template literal introduces no scope or binding — walk each
        // `${…}` insert in the current scope. Quasis are leaf strings with
        // nothing to resolve.
        Expression::TemplateLiteral(t) => {
            for e in &t.expressions {
                walk_expression(e, ctx, analysis, pending);
            }
        }
        // A tagged template resolves references in its tag callee and in each
        // `${…}` insert. Quasis are leaf strings with nothing to resolve.
        Expression::TaggedTemplateExpression(t) => {
            walk_expression(&t.tag, ctx, analysis, pending);
            for e in &t.quasi.expressions {
                walk_expression(e, ctx, analysis, pending);
            }
        }
        // `...arg` — recurse into the spread argument to resolve references in it.
        Expression::SpreadElement(s) => walk_expression(&s.argument, ctx, analysis, pending),
        Expression::YieldExpression(y) => { if let Some(a) = &y.argument { walk_expression(a, ctx, analysis, pending); } }
        Expression::AwaitExpression(a) => walk_expression(&a.argument, ctx, analysis, pending),
        Expression::ImportExpression(e) => walk_expression(&e.source, ctx, analysis, pending),
    }
}

fn walk_member_expression_inner(
    object: &Expression,
    property: &Expression,
    computed: bool,
    ctx: WalkCtx,
    analysis: &mut ScopeAnalysis,
    pending: &mut Vec<PendingReference>,
) {
    walk_expression(object, ctx, analysis, pending);
    if computed {
        walk_expression(property, ctx, analysis, pending);
    }
}

fn walk_property(
    prop: &Property,
    ctx: WalkCtx,
    analysis: &mut ScopeAnalysis,
    pending: &mut Vec<PendingReference>,
) {
    if prop.computed {
        match &prop.key {
            PropertyKey::Identifier(id) => {
                pending.push(PendingReference {
                    name: id.name.clone(),
                    from_scope: ctx.current,
                    cv: id.cv.clone(),
                });
            }
            PropertyKey::StringLiteral(_) | PropertyKey::NumericLiteral(_) => {}
            PropertyKey::Expression(e) => walk_expression(e, ctx, analysis, pending),
        }
    }
    walk_expression(&prop.value, ctx, analysis, pending);
}


#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_javascript_ast::SourceType;
    use coding_adventures_javascript_tokens::EsVersion;

    fn empty_program() -> Program {
        Program::new_untraced(EsVersion::Es2025, SourceType::Module)
    }

    #[test]
    fn analyze_returns_global_scope_only() {
        let prog = empty_program();
        let analysis = analyze(&prog);
        assert_eq!(analysis.scopes.len(), 1);
        assert_eq!(analysis.scopes[0].kind, ScopeKind::Global);
        assert_eq!(analysis.scopes[0].parent, None);
        assert!(analysis.scopes[0].bindings.is_empty());
        assert!(analysis.bindings.is_empty());
        assert!(analysis.references.is_empty());
    }

    #[test]
    fn global_scope_id_is_zero() {
        // CLOC13.A through CLOC13.E pin to this constant; document it
        // here so a future refactor that changes it has to update the
        // test (and therefore notice the breaking change).
        assert_eq!(ScopeId::GLOBAL.0, 0);
    }

    #[test]
    fn resolve_in_empty_analysis_returns_none() {
        let prog = empty_program();
        let analysis = analyze(&prog);
        assert!(analysis.resolve("anything", ScopeId::GLOBAL).is_none());
    }

    #[test]
    fn resolve_walks_parent_chain_and_finds_outer_binding() {
        // Build a hand-rolled analysis: global scope with `x`, a child
        // block scope with nothing.  Lookup of `x` from the inner
        // scope should walk up and find the global binding.
        let analysis = ScopeAnalysis {
            scopes: vec![
                Scope {
                    kind: ScopeKind::Global,
                    parent: None,
                    bindings: vec![BindingId(0)],
                },
                Scope {
                    kind: ScopeKind::Block,
                    parent: Some(ScopeId::GLOBAL),
                    bindings: Vec::new(),
                },
            ],
            bindings: vec![Binding {
                name: "x".to_string(),
                kind: BindingKind::Let,
                scope: ScopeId::GLOBAL,
                declared_at: None,
            }],
            references: Vec::new(),
        };
        let inner = ScopeId(1);
        let resolved = analysis.resolve("x", inner);
        assert_eq!(resolved, Some(BindingId(0)));
    }

    #[test]
    fn resolve_innermost_shadow_wins() {
        // Global `x` is shadowed by an inner-block `x`.  Lookup from
        // the inner scope returns the inner binding.
        let analysis = ScopeAnalysis {
            scopes: vec![
                Scope {
                    kind: ScopeKind::Global,
                    parent: None,
                    bindings: vec![BindingId(0)],
                },
                Scope {
                    kind: ScopeKind::Block,
                    parent: Some(ScopeId::GLOBAL),
                    bindings: vec![BindingId(1)],
                },
            ],
            bindings: vec![
                Binding {
                    name: "x".to_string(),
                    kind: BindingKind::Let,
                    scope: ScopeId::GLOBAL,
                    declared_at: None,
                },
                Binding {
                    name: "x".to_string(),
                    kind: BindingKind::Let,
                    scope: ScopeId(1),
                    declared_at: None,
                },
            ],
            references: Vec::new(),
        };
        let inner = ScopeId(1);
        assert_eq!(analysis.resolve("x", inner), Some(BindingId(1)));
    }

    // ---------------------------------------------------------------
    // CLOC13.0 body tests — top-level declaration surfacing.
    // ---------------------------------------------------------------

    use coding_adventures_javascript_ast::{
        BindingTarget, BlockStatement, Declaration, FunctionDeclaration, Identifier,
        ProgramItem, VarKind, VariableDeclaration, VariableDeclarator,
    };

    fn ident(name: &str) -> Identifier {
        Identifier {
            cv: None,
            name: name.to_string(),
        }
    }

    fn program_with(items: Vec<ProgramItem>) -> Program {
        let mut p = empty_program();
        p.body = items;
        p
    }

    fn var_decl(kind: VarKind, names: &[&str]) -> ProgramItem {
        ProgramItem::Declaration(Declaration::VariableDeclaration(VariableDeclaration {
            cv: None,
            kind,
            declarations: names
                .iter()
                .map(|n| VariableDeclarator {
                    cv: None,
                    id: BindingTarget::Identifier(ident(n)),
                    init: None,
                })
                .collect(),
        }))
    }

    fn fn_decl(name: &str) -> ProgramItem {
        ProgramItem::Declaration(Declaration::FunctionDeclaration(FunctionDeclaration {
            cv: None,
            id: ident(name),
            params: Vec::new(),
            body: BlockStatement {
                cv: None,
                body: Vec::new(),
            },
            generator: false,
            is_async: false,
        }))
    }

    #[test]
    fn top_level_let_surfaces_as_binding_in_global() {
        // `let x = 1;` (init elided) — pin the Binding shape end-to-end.
        let prog = program_with(vec![var_decl(VarKind::Let, &["x"])]);
        let analysis = analyze(&prog);
        assert_eq!(analysis.bindings.len(), 1);
        assert_eq!(analysis.bindings[0].name, "x");
        assert_eq!(analysis.bindings[0].kind, BindingKind::Let);
        assert_eq!(analysis.bindings[0].scope, ScopeId::GLOBAL);
        // The global scope's bindings list mirrors the global table.
        assert_eq!(
            analysis.scopes[ScopeId::GLOBAL.0 as usize].bindings,
            vec![BindingId(0)]
        );
    }

    #[test]
    fn top_level_var_let_const_map_to_three_kinds() {
        let prog = program_with(vec![
            var_decl(VarKind::Var, &["a"]),
            var_decl(VarKind::Let, &["b"]),
            var_decl(VarKind::Const, &["c"]),
        ]);
        let analysis = analyze(&prog);
        let kinds: Vec<_> = analysis.bindings.iter().map(|b| b.kind).collect();
        assert_eq!(
            kinds,
            vec![BindingKind::Var, BindingKind::Let, BindingKind::Const],
        );
    }

    #[test]
    fn top_level_function_declaration_surfaces() {
        // `function f() {}` — kind == Function, scope == GLOBAL.
        let prog = program_with(vec![fn_decl("f")]);
        let analysis = analyze(&prog);
        assert_eq!(analysis.bindings.len(), 1);
        assert_eq!(analysis.bindings[0].name, "f");
        assert_eq!(analysis.bindings[0].kind, BindingKind::Function);
    }

    #[test]
    fn multi_declarator_emits_one_binding_per_declarator() {
        // `const a = 1, b = 2;` → two bindings, both Const.
        let prog = program_with(vec![var_decl(VarKind::Const, &["a", "b"])]);
        let analysis = analyze(&prog);
        assert_eq!(analysis.bindings.len(), 2);
        assert_eq!(analysis.bindings[0].name, "a");
        assert_eq!(analysis.bindings[1].name, "b");
        assert!(
            analysis
                .bindings
                .iter()
                .all(|b| b.kind == BindingKind::Const)
        );
    }

    #[test]
    fn binding_ids_are_dense_and_monotonic() {
        // 3 decls → BindingId(0), BindingId(1), BindingId(2). Pins the
        // contract that consumers can index bindings by their position.
        let prog = program_with(vec![
            var_decl(VarKind::Let, &["x"]),
            fn_decl("f"),
            var_decl(VarKind::Const, &["k"]),
        ]);
        let analysis = analyze(&prog);
        let scope_list = &analysis.scopes[ScopeId::GLOBAL.0 as usize].bindings;
        assert_eq!(*scope_list, vec![BindingId(0), BindingId(1), BindingId(2)]);
    }

    #[test]
    fn statement_items_are_skipped_for_now() {
        // ExpressionStatement at top level doesn't produce bindings in
        // CLOC13.0. Reference-collection lands in CLOC13.0.1. Pinning
        // this absence here so adding reference walks doesn't silently
        // break the skip-statements contract.
        use coding_adventures_javascript_ast::{
            ExpressionStatement, Expression, NumericLiteral, Statement,
        };
        let stmt = ProgramItem::Statement(Statement::expression_statement(
            ExpressionStatement {
                cv: None,
                expression: Expression::NumericLiteral(NumericLiteral {
                    cv: None,
                    value: 1.0,
                    raw: "1".to_string(),
                }),
            },
        ));
        let prog = program_with(vec![stmt]);
        let analysis = analyze(&prog);
        assert!(analysis.bindings.is_empty());
        assert!(analysis.references.is_empty());
    }

    #[test]
    fn references_are_empty_in_cloc13_0() {
        // Pinned by CLOC13.0; survives unchanged under CLOC13.0.1.
        // The fixture has no Identifier expressions — only a
        // `let x;` declarator with no init — so even with the
        // CLOC13.0.1 reference walker, zero references emerge.
        // The test now pins the *no-references-when-no-identifier-
        // expressions* contract, which is the more durable form
        // of the original CLOC13.0 promise.
        let prog = program_with(vec![var_decl(VarKind::Let, &["x"])]);
        let analysis = analyze(&prog);
        assert!(analysis.references.is_empty());
    }

    // ---------------------------------------------------------------
    // CLOC13.0.1 body tests — reference collection.
    //
    // These exercise the Phase-2 walker. Bindings come from Phase 1
    // (= CLOC13.0); references come from Phase 2 (= CLOC13.0.1).
    // ---------------------------------------------------------------

    use coding_adventures_javascript_ast::{
        BinaryExpression, BinaryOperator, CallExpression, ExpressionStatement,
        IfStatement, MemberExpression, NumericLiteral, Statement,
    };

    fn id_expr(name: &str) -> Expression {
        Expression::Identifier(ident(name))
    }

    fn num_expr(v: f64) -> Expression {
        Expression::NumericLiteral(NumericLiteral {
            cv: None,
            value: v,
            raw: v.to_string(),
        })
    }

    fn expr_stmt(expr: Expression) -> ProgramItem {
        ProgramItem::Statement(Statement::expression_statement(ExpressionStatement {
            cv: None,
            expression: expr,
        }))
    }

    #[test]
    fn bare_identifier_in_expression_statement_emits_reference() {
        // `x;` at the top level — a bare identifier read.
        let prog = program_with(vec![expr_stmt(id_expr("x"))]);
        let analysis = analyze(&prog);
        assert_eq!(analysis.references.len(), 1);
        assert_eq!(analysis.references[0].name, "x");
        assert_eq!(analysis.references[0].from_scope, ScopeId::GLOBAL);
        // No binding for `x` → unresolved (free global).
        assert!(analysis.references[0].binding.is_none());
    }

    #[test]
    fn reference_resolves_to_top_level_binding() {
        // `let x; x;` — the reference must resolve to the binding.
        let prog = program_with(vec![
            var_decl(VarKind::Let, &["x"]),
            expr_stmt(id_expr("x")),
        ]);
        let analysis = analyze(&prog);
        assert_eq!(analysis.bindings.len(), 1);
        assert_eq!(analysis.references.len(), 1);
        assert_eq!(analysis.references[0].binding, Some(BindingId(0)));
    }

    #[test]
    fn unresolved_reference_to_free_global() {
        // `console;` — `console` has no binding in this program,
        // so the reference is unresolved (binding == None). This
        // is how downstream passes detect free globals.
        let prog = program_with(vec![expr_stmt(id_expr("console"))]);
        let analysis = analyze(&prog);
        assert_eq!(analysis.references.len(), 1);
        assert!(analysis.references[0].binding.is_none());
    }

    #[test]
    fn binary_expression_collects_both_sides() {
        // `x + y;` — both `x` and `y` produce references.
        let prog = program_with(vec![expr_stmt(Expression::BinaryExpression(
            BinaryExpression {
                cv: None,
                operator: BinaryOperator::Add,
                left: Box::new(id_expr("x")),
                right: Box::new(id_expr("y")),
            },
        ))]);
        let analysis = analyze(&prog);
        let names: Vec<_> = analysis
            .references
            .iter()
            .map(|r| r.name.clone())
            .collect();
        assert_eq!(names, vec!["x".to_string(), "y".to_string()]);
    }

    #[test]
    fn call_expression_collects_callee_and_arguments() {
        // `f(x, y);` — callee + args all produce references.
        let prog = program_with(vec![expr_stmt(Expression::CallExpression(
            CallExpression {
                cv: None,
                callee: Box::new(id_expr("f")),
                arguments: vec![id_expr("x"), id_expr("y")],
            },
        ))]);
        let analysis = analyze(&prog);
        let names: Vec<_> = analysis
            .references
            .iter()
            .map(|r| r.name.clone())
            .collect();
        assert_eq!(names, vec!["f", "x", "y"]);
    }

    #[test]
    fn member_expression_collects_object_only_when_not_computed() {
        // `obj.prop;` — `obj` is a reference; `prop` is a property
        // *name*, NOT a binding lookup.
        let prog = program_with(vec![expr_stmt(Expression::MemberExpression(
            MemberExpression {
                cv: None,
                object: Box::new(id_expr("obj")),
                property: Box::new(id_expr("prop")),
                computed: false,
            },
        ))]);
        let analysis = analyze(&prog);
        assert_eq!(analysis.references.len(), 1);
        assert_eq!(analysis.references[0].name, "obj");
    }

    #[test]
    fn member_expression_computed_form_collects_property() {
        // `obj[name];` — `name` IS a reference (computed access).
        let prog = program_with(vec![expr_stmt(Expression::MemberExpression(
            MemberExpression {
                cv: None,
                object: Box::new(id_expr("obj")),
                property: Box::new(id_expr("name")),
                computed: true,
            },
        ))]);
        let analysis = analyze(&prog);
        let names: Vec<_> = analysis
            .references
            .iter()
            .map(|r| r.name.clone())
            .collect();
        assert_eq!(names, vec!["obj", "name"]);
    }

    #[test]
    fn variable_declaration_init_walks_for_references() {
        // `let y = x;` — `x` in the init produces a reference.
        // The binding `y` is NOT a reference (declaration site).
        let init = id_expr("x");
        let prog = program_with(vec![ProgramItem::Declaration(
            Declaration::VariableDeclaration(VariableDeclaration {
                cv: None,
                kind: VarKind::Let,
                declarations: vec![VariableDeclarator {
                    cv: None,
                    id: BindingTarget::Identifier(ident("y")),
                    init: Some(init),
                }],
            }),
        )]);
        let analysis = analyze(&prog);
        assert_eq!(analysis.bindings.len(), 1);
        assert_eq!(analysis.bindings[0].name, "y");
        assert_eq!(analysis.references.len(), 1);
        assert_eq!(analysis.references[0].name, "x");
    }

    #[test]
    fn function_body_walks_for_references() {
        // `function f() { x; }` — `x` in the body produces a
        // reference. The function name `f` is a binding only,
        // not a reference.
        //
        // CLOC13.0.2 update: the function body now lives in a
        // child Function scope (`ScopeId(1)`), not GLOBAL. The
        // reference's `from_scope` reflects this. The `x` is a
        // free global from inside the function — `analysis.resolve`
        // walks the parent chain (Function → GLOBAL) and finds
        // no binding, so `binding` is None.
        let body = BlockStatement {
            cv: None,
            body: vec![Statement::expression_statement(ExpressionStatement {
                cv: None,
                expression: id_expr("x"),
            })],
        };
        let prog = program_with(vec![ProgramItem::Declaration(
            Declaration::FunctionDeclaration(FunctionDeclaration {
                cv: None,
                id: ident("f"),
                params: Vec::new(),
                body,
                generator: false,
                is_async: false,
            }),
        )]);
        let analysis = analyze(&prog);
        assert_eq!(analysis.bindings.len(), 1);
        assert_eq!(analysis.bindings[0].name, "f");
        assert_eq!(analysis.bindings[0].scope, ScopeId::GLOBAL);
        assert_eq!(analysis.references.len(), 1);
        assert_eq!(analysis.references[0].name, "x");
        // The Function scope is scopes[1] (GLOBAL is scopes[0]).
        assert_eq!(analysis.references[0].from_scope, ScopeId(1));
        // `x` not bound anywhere → free global.
        assert!(analysis.references[0].binding.is_none());
    }

    #[test]
    fn if_statement_walks_test_and_branches() {
        // `if (cond) a; else b;` — `cond`, `a`, `b` all produce
        // references.
        let if_stmt = ProgramItem::Statement(Statement::if_statement(IfStatement {
            cv: None,
            test: id_expr("cond"),
            consequent: Box::new(Statement::expression_statement(ExpressionStatement {
                cv: None,
                expression: id_expr("a"),
            })),
            alternate: Some(Box::new(Statement::expression_statement(
                ExpressionStatement {
                    cv: None,
                    expression: id_expr("b"),
                },
            ))),
        }));
        let prog = program_with(vec![if_stmt]);
        let analysis = analyze(&prog);
        let names: Vec<_> = analysis
            .references
            .iter()
            .map(|r| r.name.clone())
            .collect();
        assert_eq!(names, vec!["cond", "a", "b"]);
    }

    #[test]
    fn literals_in_expression_statements_emit_no_references() {
        // `1; 2;` — pure literal statements. No references.
        let prog = program_with(vec![expr_stmt(num_expr(1.0)), expr_stmt(num_expr(2.0))]);
        let analysis = analyze(&prog);
        assert!(analysis.references.is_empty());
    }

    #[test]
    fn analysis_round_trips_via_serde() {
        let analysis = ScopeAnalysis {
            scopes: vec![Scope {
                kind: ScopeKind::Function,
                parent: Some(ScopeId::GLOBAL),
                bindings: vec![BindingId(0)],
            }],
            bindings: vec![Binding {
                name: "x".to_string(),
                kind: BindingKind::Var,
                scope: ScopeId::GLOBAL,
                declared_at: Some("cv.1".to_string()),
            }],
            references: vec![Reference {
                name: "x".to_string(),
                from_scope: ScopeId::GLOBAL,
                binding: Some(BindingId(0)),
                cv: Some("cv.2".to_string()),
            }],
        };
        let json = serde_json::to_string(&analysis).expect("serialize");
        let back: ScopeAnalysis = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(analysis, back);
    }

    // ---------------------------------------------------------------
    // CLOC13.0.2 — nested scope tests.
    //
    // These exercise the recursive scope-tree machinery: Function
    // bodies create Function scopes; BlockStatements create Block
    // scopes; var hoists to enclosing function; reference resolution
    // walks the parent chain.
    // ---------------------------------------------------------------

    fn fn_decl_with_body(name: &str, params: &[&str], body: Vec<Statement>) -> ProgramItem {
        ProgramItem::Declaration(Declaration::FunctionDeclaration(FunctionDeclaration {
            cv: None,
            id: ident(name),
            params: params
                .iter()
                .map(|n| FunctionParam::Identifier(ident(n)))
                .collect(),
            body: BlockStatement { cv: None, body },
            generator: false,
            is_async: false,
        }))
    }

    #[test]
    fn function_declaration_creates_child_function_scope() {
        // `function f() {}` — emits a Function scope (id 1) as
        // a child of GLOBAL. The function name `f` is in GLOBAL.
        let prog = program_with(vec![fn_decl("f")]);
        let analysis = analyze(&prog);
        assert_eq!(analysis.scopes.len(), 2);
        assert_eq!(analysis.scopes[1].kind, ScopeKind::Function);
        assert_eq!(analysis.scopes[1].parent, Some(ScopeId::GLOBAL));
        assert_eq!(analysis.bindings[0].name, "f");
        assert_eq!(analysis.bindings[0].scope, ScopeId::GLOBAL);
    }

    #[test]
    fn function_params_become_param_bindings_in_function_scope() {
        // `function f(a, b) {}` — `a` and `b` are Param-kind
        // bindings inside f's Function scope, not GLOBAL.
        let prog = program_with(vec![fn_decl_with_body("f", &["a", "b"], vec![])]);
        let analysis = analyze(&prog);
        assert_eq!(analysis.scopes.len(), 2);
        // 3 bindings: f (Function in GLOBAL), a (Param in f), b (Param in f)
        assert_eq!(analysis.bindings.len(), 3);
        assert_eq!(analysis.bindings[0].name, "f");
        assert_eq!(analysis.bindings[1].name, "a");
        assert_eq!(analysis.bindings[1].kind, BindingKind::Param);
        assert_eq!(analysis.bindings[1].scope, ScopeId(1));
        assert_eq!(analysis.bindings[2].name, "b");
        assert_eq!(analysis.bindings[2].kind, BindingKind::Param);
        assert_eq!(analysis.bindings[2].scope, ScopeId(1));
    }

    #[test]
    fn param_reference_resolves_inside_function() {
        // `function f(a) { a; }` — the `a` reference inside the body
        // resolves to the param binding via parent-chain lookup.
        let body = vec![Statement::expression_statement(ExpressionStatement {
            cv: None,
            expression: id_expr("a"),
        })];
        let prog = program_with(vec![fn_decl_with_body("f", &["a"], body)]);
        let analysis = analyze(&prog);
        assert_eq!(analysis.references.len(), 1);
        assert_eq!(analysis.references[0].name, "a");
        assert_eq!(analysis.references[0].from_scope, ScopeId(1));
        assert_eq!(analysis.references[0].binding, Some(BindingId(1)));
    }

    #[test]
    fn cross_scope_resolution_finds_outer_binding() {
        // `let x; function f() { x; }` — `x` inside f resolves
        // through the parent chain to the GLOBAL binding.
        let body = vec![Statement::expression_statement(ExpressionStatement {
            cv: None,
            expression: id_expr("x"),
        })];
        let prog = program_with(vec![
            var_decl(VarKind::Let, &["x"]),
            fn_decl_with_body("f", &[], body),
        ]);
        let analysis = analyze(&prog);
        // bindings: [x (Let in GLOBAL), f (Function in GLOBAL)]
        assert_eq!(analysis.bindings[0].name, "x");
        assert_eq!(analysis.references.len(), 1);
        assert_eq!(analysis.references[0].name, "x");
        assert_eq!(analysis.references[0].from_scope, ScopeId(1)); // f's Function scope
        assert_eq!(analysis.references[0].binding, Some(BindingId(0)));
    }

    #[test]
    fn block_statement_creates_block_scope() {
        // `{ let y; }` at the top level — a free-standing
        // BlockStatement creates a Block child of GLOBAL.
        let block = Statement::block_statement(BlockStatement {
            cv: None,
            body: vec![Statement::Declaration(Declaration::VariableDeclaration(
                VariableDeclaration {
                    cv: None,
                    kind: VarKind::Let,
                    declarations: vec![VariableDeclarator {
                        cv: None,
                        id: BindingTarget::Identifier(ident("y")),
                        init: None,
                    }],
                },
            ))],
        });
        let prog = program_with(vec![ProgramItem::Statement(block)]);
        let analysis = analyze(&prog);
        assert_eq!(analysis.scopes.len(), 2);
        assert_eq!(analysis.scopes[1].kind, ScopeKind::Block);
        assert_eq!(analysis.scopes[1].parent, Some(ScopeId::GLOBAL));
        // `y` lives in the Block scope, not GLOBAL.
        assert_eq!(analysis.bindings.len(), 1);
        assert_eq!(analysis.bindings[0].name, "y");
        assert_eq!(analysis.bindings[0].scope, ScopeId(1));
    }

    #[test]
    fn var_in_block_hoists_to_enclosing_function() {
        // `function f() { { var x; } }` — `var x` inside a
        // nested block hoists OUT to f's Function scope, NOT the
        // inner Block. This is the key hoisting test.
        let inner_var = Statement::Declaration(Declaration::VariableDeclaration(
            VariableDeclaration {
                cv: None,
                kind: VarKind::Var,
                declarations: vec![VariableDeclarator {
                    cv: None,
                    id: BindingTarget::Identifier(ident("x")),
                    init: None,
                }],
            },
        ));
        let inner_block = Statement::block_statement(BlockStatement {
            cv: None,
            body: vec![inner_var],
        });
        let prog = program_with(vec![fn_decl_with_body("f", &[], vec![inner_block])]);
        let analysis = analyze(&prog);
        // scopes: [GLOBAL, Function f, Block (inside f)]
        assert_eq!(analysis.scopes.len(), 3);
        assert_eq!(analysis.scopes[1].kind, ScopeKind::Function);
        assert_eq!(analysis.scopes[2].kind, ScopeKind::Block);
        assert_eq!(analysis.scopes[2].parent, Some(ScopeId(1)));
        // bindings: [f (Function in GLOBAL), x (Var in f-scope, NOT block-scope)]
        let x_binding = analysis
            .bindings
            .iter()
            .find(|b| b.name == "x")
            .expect("x binding");
        assert_eq!(x_binding.kind, BindingKind::Var);
        assert_eq!(x_binding.scope, ScopeId(1)); // f's Function scope, not the Block
    }

    #[test]
    fn let_in_block_stays_in_block_scope() {
        // `function f() { { let y; } }` — `let y` stays in the
        // Block scope (no hoisting), the opposite of `var x`.
        let inner_let = Statement::Declaration(Declaration::VariableDeclaration(
            VariableDeclaration {
                cv: None,
                kind: VarKind::Let,
                declarations: vec![VariableDeclarator {
                    cv: None,
                    id: BindingTarget::Identifier(ident("y")),
                    init: None,
                }],
            },
        ));
        let inner_block = Statement::block_statement(BlockStatement {
            cv: None,
            body: vec![inner_let],
        });
        let prog = program_with(vec![fn_decl_with_body("f", &[], vec![inner_block])]);
        let analysis = analyze(&prog);
        let y_binding = analysis
            .bindings
            .iter()
            .find(|b| b.name == "y")
            .expect("y binding");
        assert_eq!(y_binding.kind, BindingKind::Let);
        assert_eq!(y_binding.scope, ScopeId(2)); // the Block scope
    }

    #[test]
    fn nested_function_creates_nested_function_scope() {
        // `function outer() { function inner() {} }` — nested
        // Function-in-Function. Both get their own Function
        // scopes; inner's parent is outer's, outer's parent is
        // GLOBAL.
        let inner_decl = Statement::Declaration(Declaration::FunctionDeclaration(
            FunctionDeclaration {
                cv: None,
                id: ident("inner"),
                params: Vec::new(),
                body: BlockStatement {
                    cv: None,
                    body: Vec::new(),
                },
                generator: false,
                is_async: false,
            },
        ));
        let prog = program_with(vec![fn_decl_with_body("outer", &[], vec![inner_decl])]);
        let analysis = analyze(&prog);
        assert_eq!(analysis.scopes.len(), 3);
        assert_eq!(analysis.scopes[1].kind, ScopeKind::Function);
        assert_eq!(analysis.scopes[1].parent, Some(ScopeId::GLOBAL));
        assert_eq!(analysis.scopes[2].kind, ScopeKind::Function);
        assert_eq!(analysis.scopes[2].parent, Some(ScopeId(1)));
        // `inner` name binding lives in outer's Function scope.
        let inner_binding = analysis
            .bindings
            .iter()
            .find(|b| b.name == "inner")
            .expect("inner binding");
        assert_eq!(inner_binding.kind, BindingKind::Function);
        assert_eq!(inner_binding.scope, ScopeId(1));
    }

    #[test]
    fn empty_program_still_returns_global_scope_only() {
        // Identity check: the regression test from earlier still
        // passes — no items means no new scopes.
        let prog = empty_program();
        let analysis = analyze(&prog);
        assert_eq!(analysis.scopes.len(), 1);
        assert!(analysis.bindings.is_empty());
        assert!(analysis.references.is_empty());
    }
}
