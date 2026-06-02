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

use coding_adventures_javascript_ast::{
    BindingTarget, CvId, Declaration, Program, ProgramItem, VarKind,
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
///    [`Expression::Identifier`] node — should produce a
///    [`Reference`].  Today we emit zero references.  This is the
///    biggest remaining gap for the consumer passes;
///    `remove-unused-vars` and `inline` both gate on `uses == 0`
///    or `uses == 1`, and zero references means every binding
///    reads as "unused".
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
pub fn analyze(program: &Program) -> ScopeAnalysis {
    // CLOC13.0 algorithm — one walk of the top-level program body.
    //
    // Plain code beats a visitor-pattern here: the AST is shallow
    // (1 program → N items, each a Declaration or a Statement),
    // and we only care about Declarations in this PR.  A future
    // pass-internal walker can grow into a proper visitor.
    let mut analysis = ScopeAnalysis {
        scopes: vec![Scope {
            kind: ScopeKind::Global,
            parent: None,
            bindings: Vec::new(),
        }],
        bindings: Vec::new(),
        references: Vec::new(),
    };

    for item in &program.body {
        // Only Declarations at the top level produce bindings in
        // this PR.  Statement items are walked in CLOC13.0.1 when
        // we add the reference-collection pass.
        let decl = match item {
            ProgramItem::Declaration(d) => d,
            ProgramItem::Statement(_) => continue,
        };

        match decl {
            Declaration::VariableDeclaration(var_decl) => {
                // Map VarKind → BindingKind.  `var` is function-
                // scoped per ECMAScript; `let`/`const` are block-
                // scoped.  Since we're only emitting top-level
                // bindings in CLOC13.0, all three land in GLOBAL.
                // The function- vs block- distinction starts to
                // matter when CLOC13.0.1 introduces nested
                // scopes and var-hoisting.
                let kind = match var_decl.kind {
                    VarKind::Var => BindingKind::Var,
                    VarKind::Let => BindingKind::Let,
                    VarKind::Const => BindingKind::Const,
                };

                for declarator in &var_decl.declarations {
                    // Phase 1 only ships `BindingTarget::Identifier`.
                    // The match is exhaustive today; when CLOC09
                    // Phase 3 adds destructuring patterns
                    // (`ArrayPattern`, `ObjectPattern`), they'll
                    // need their own arms here that recursively
                    // collect every bound identifier.
                    let BindingTarget::Identifier(id) = &declarator.id;

                    let binding_id = BindingId(analysis.bindings.len() as u32);
                    analysis.bindings.push(Binding {
                        name: id.name.clone(),
                        kind,
                        scope: ScopeId::GLOBAL,
                        declared_at: id.cv.clone(),
                    });
                    analysis.scopes[ScopeId::GLOBAL.0 as usize]
                        .bindings
                        .push(binding_id);
                }
            }
            Declaration::FunctionDeclaration(fn_decl) => {
                // Emit the function name as a Function-kind binding
                // in the GLOBAL scope.  CLOC13.0.1 will also
                // create a child Function scope holding params +
                // nested decls; in CLOC13.0 we just surface the
                // name so treeshake / inline can see it.
                let binding_id = BindingId(analysis.bindings.len() as u32);
                analysis.bindings.push(Binding {
                    name: fn_decl.id.name.clone(),
                    kind: BindingKind::Function,
                    scope: ScopeId::GLOBAL,
                    declared_at: fn_decl.id.cv.clone(),
                });
                analysis.scopes[ScopeId::GLOBAL.0 as usize]
                    .bindings
                    .push(binding_id);
            }
        }
    }

    analysis
}

// ---------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------

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
        // Pin the deferred-to-CLOC13.0.1 contract.  Once references
        // start being collected, this test will fail loudly — which is
        // the right signal: it forces the contributor to update every
        // consumer pass's expectation that references == empty under
        // analyzer v0.2.0.
        let prog = program_with(vec![var_decl(VarKind::Let, &["x"])]);
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
}
