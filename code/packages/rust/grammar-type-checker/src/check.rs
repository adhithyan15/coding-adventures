//! Core type-inference algorithm for the generic grammar type checker.
//!
//! The central function [`infer`] walks a [`GrammarASTNode`] tree recursively,
//! consulting the [`LanguageProfile`] to understand the semantic role of each
//! node.  It returns a fully-annotated [`AnnotatedNode`] tree where every
//! node carries its inferred [`KindDecl`].
//!
//! ## Priority order
//!
//! For every node `infer` tries, in order:
//!
//! 1. **Literal** → `profile.literal_kind(node)` → return immediately
//! 2. **Variable reference** → `profile.as_var_ref(node)` → scope/globals lookup
//! 3. **Apply** → `profile.as_apply(node)` → callee inference + arity check
//! 4. **Binder** → `profile.as_binder(node)` → push scope, infer body, pop
//! 5. **Match** → `profile.as_match(node)` → scrutinee + exhaustiveness + arms
//! 6. **Begin** → `profile.as_begin(node)` → infer all, return last kind
//! 7. **Fallback** → `profile.child_exprs(node)` → recurse, return last kind
//!
//! ## Annotation is always-on
//!
//! Even when `TypedModeDecl::Off`, the annotated tree is built.  The *kind*
//! on each node is always the best inferred kind (`Any` when unknown).
//! Whether errors are *emitted* depends on the mode (only Lenient/Strict emit
//! errors).

use std::collections::HashSet;

use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use type_checker_protocol::TypeErrorDiagnostic;
use type_declarations::{AnnotatedChild, AnnotatedNode, KindDecl, TypeDeclarations, TypedModeDecl};

use crate::profile::{ArmPattern, BinderKind, LanguageProfile};
use crate::scope::ScopeStack;

/// Maximum recursion depth inside `infer`.
///
/// Guards against stack overflow on pathologically deep grammar trees.  The
/// grammar parser already enforces its own depth cap (64 for Twig parens),
/// so in practice depth never approaches 256.  This is a defence-in-depth.
pub const MAX_INFER_DEPTH: usize = 256;

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Infer the kind of `node`, build its [`AnnotatedNode`], and accumulate
/// any type errors into `errors`.
///
/// Callers should use the public API in [`crate`] rather than calling this
/// directly.
///
/// ## Parameters
///
/// - `node` — the raw grammar tree node to infer
/// - `decls` — parser-emitted type declarations (globals, named types, mode)
/// - `profile` — language-specific navigation (which rules mean what)
/// - `scope` — mutable lexical scope stack (caller provides, so binder forms
///   can push/pop without cloning the whole stack)
/// - `errors` — accumulated diagnostics (only written when mode ≠ Off)
/// - `mode` — enforcement mode (controls whether errors are emitted)
/// - `depth` — current recursion depth (starts at 0)
pub fn infer<P: LanguageProfile>(
    node: &GrammarASTNode,
    decls: &TypeDeclarations,
    profile: &P,
    scope: &mut ScopeStack,
    errors: &mut Vec<TypeErrorDiagnostic>,
    mode: &TypedModeDecl,
    depth: usize,
) -> AnnotatedNode {
    if depth > MAX_INFER_DEPTH {
        // Bail out — return Any to avoid stack overflow.
        return make_annotated(node, KindDecl::Any, vec![]);
    }

    // ── 1. Literal? ──────────────────────────────────────────────────────
    if let Some(kind) = profile.literal_kind(node) {
        let children = token_children(node);
        return make_annotated(node, kind, children);
    }

    // ── 2. Variable reference? ───────────────────────────────────────────
    if let Some(name) = profile.as_var_ref(node) {
        let kind = infer_var_ref(name, decls, scope, profile, node, errors, mode);
        let children = token_children(node);
        return make_annotated(node, kind, children);
    }

    // ── 3. Function application? ─────────────────────────────────────────
    if let Some(app) = profile.as_apply(node) {
        let callee_ann = infer(app.callee, decls, profile, scope, errors, mode, depth + 1);
        let callee_kind = decls.resolve(&callee_ann.kind);

        let mut arg_anns = Vec::with_capacity(app.args.len());
        for arg in &app.args {
            arg_anns.push(infer(arg, decls, profile, scope, errors, mode, depth + 1));
        }

        // Arity check when the callee resolves to a known function kind.
        if let KindDecl::Function { arity } = &callee_kind {
            let actual = app.args.len();
            if *arity != actual && *mode != TypedModeDecl::Off {
                let (line, col) = profile.position(node);
                let fn_name = profile.as_var_ref(app.callee).unwrap_or("<expr>");
                errors.push(TypeErrorDiagnostic {
                    message: format!(
                        "arity error: '{}' expects {} argument{}, got {}",
                        fn_name,
                        arity,
                        if *arity == 1 { "" } else { "s" },
                        actual
                    ),
                    line,
                    column: col,
                });
            }
        }

        // Result kind — call returns Any (static return-type tracking is TW05-C).
        let mut children = vec![AnnotatedChild::Node(callee_ann)];
        for ann in arg_anns {
            children.push(AnnotatedChild::Node(ann));
        }
        return make_annotated(node, KindDecl::Any, children);
    }

    // ── 4. Binder (let / lambda / define)? ───────────────────────────────
    if let Some(binder) = profile.as_binder(node) {
        return infer_binder(node, binder, decls, profile, scope, errors, mode, depth);
    }

    // ── 5. Match? ────────────────────────────────────────────────────────
    if let Some(m) = profile.as_match(node) {
        let scrutinee_ann =
            infer(m.scrutinee, decls, profile, scope, errors, mode, depth + 1);
        let scrutinee_kind = decls.resolve(&scrutinee_ann.kind);

        // Exhaustiveness check for union scrutinees.
        if let KindDecl::Named(ref union_name) = scrutinee_kind {
            if let Some(variants) = decls.union_variants(union_name) {
                if *mode != TypedModeDecl::Off {
                    check_exhaustiveness(union_name, &variants, &m.arms, profile, node, errors);
                }
            }
        }

        // Walk each arm.
        let mut arm_anns: Vec<AnnotatedChild> = vec![AnnotatedChild::Node(scrutinee_ann)];
        let mut last_kind = KindDecl::Any;
        for arm in &m.arms {
            scope.push();
            match &arm.pattern {
                ArmPattern::Binding(name) => {
                    scope.bind(name.clone(), scrutinee_kind.clone());
                }
                ArmPattern::Variant { bindings, .. } => {
                    for b in bindings {
                        scope.bind(b.clone(), KindDecl::Any);
                    }
                }
                ArmPattern::Wildcard => {}
            }
            let body_ann = infer(arm.body, decls, profile, scope, errors, mode, depth + 1);
            last_kind = body_ann.kind.clone();
            arm_anns.push(AnnotatedChild::Node(body_ann));
            scope.pop();
        }

        return make_annotated(node, last_kind, arm_anns);
    }

    // ── 6. Begin / sequence? ─────────────────────────────────────────────
    if let Some(exprs) = profile.as_begin(node) {
        let mut children = Vec::with_capacity(exprs.len());
        let mut last_kind = KindDecl::Nil; // begin with zero exprs → Nil
        for e in &exprs {
            let ann = infer(e, decls, profile, scope, errors, mode, depth + 1);
            last_kind = ann.kind.clone();
            children.push(AnnotatedChild::Node(ann));
        }
        return make_annotated(node, last_kind, children);
    }

    // ── 7. Fallback: recurse into child expressions ───────────────────────
    let child_nodes = profile.child_exprs(node);
    if child_nodes.is_empty() {
        // Leaf node with no expr children — token wrappers etc.
        let children = token_children(node);
        return make_annotated(node, KindDecl::Any, children);
    }

    let mut children = Vec::with_capacity(child_nodes.len());
    let mut last_kind = KindDecl::Any;
    for child in &child_nodes {
        let ann = infer(child, decls, profile, scope, errors, mode, depth + 1);
        last_kind = ann.kind.clone();
        children.push(AnnotatedChild::Node(ann));
    }
    make_annotated(node, last_kind, children)
}

// ---------------------------------------------------------------------------
// Variable reference helper
// ---------------------------------------------------------------------------

fn infer_var_ref<P: LanguageProfile>(
    name: &str,
    decls: &TypeDeclarations,
    scope: &ScopeStack,
    profile: &P,
    node: &GrammarASTNode,
    errors: &mut Vec<TypeErrorDiagnostic>,
    mode: &TypedModeDecl,
) -> KindDecl {
    // 1. Local scope (lambda params, let bindings)
    if let Some(k) = scope.lookup(name) {
        return k.clone();
    }
    // 2. Global declarations (top-level defines, pre-loaded before the walk)
    if let Some(k) = decls.globals.get(name) {
        return k.clone();
    }
    // 3. Not found — emit error in Lenient/Strict mode
    if *mode != TypedModeDecl::Off {
        let (line, col) = profile.position(node);
        errors.push(TypeErrorDiagnostic {
            message: format!("unresolved variable '{}'", name),
            line,
            column: col,
        });
    }
    KindDecl::Any
}

// ---------------------------------------------------------------------------
// Binder helper
// ---------------------------------------------------------------------------

fn infer_binder<P: LanguageProfile>(
    node: &GrammarASTNode,
    binder: crate::profile::BinderInfo<'_>,
    decls: &TypeDeclarations,
    profile: &P,
    scope: &mut ScopeStack,
    errors: &mut Vec<TypeErrorDiagnostic>,
    mode: &TypedModeDecl,
    depth: usize,
) -> AnnotatedNode {
    match binder.kind {
        BinderKind::Let { bindings, body } => {
            // Evaluate all RHS in the *outer* scope (Scheme `let` semantics —
            // none of the new names are visible to each other's RHS).
            let mut rhs_anns = Vec::with_capacity(bindings.len());
            let mut rhs_kinds = Vec::with_capacity(bindings.len());
            for (_, rhs_node) in &bindings {
                let ann = infer(rhs_node, decls, profile, scope, errors, mode, depth + 1);
                rhs_kinds.push(ann.kind.clone());
                rhs_anns.push(AnnotatedChild::Node(ann));
            }

            // Now push a new scope and bind all names.
            if !binder.is_global {
                scope.push();
            }
            for ((name, _), kind) in bindings.iter().zip(rhs_kinds.iter()) {
                scope.bind(name.clone(), kind.clone());
            }

            // Walk the body.
            let mut body_anns = Vec::with_capacity(body.len());
            let mut last_kind = KindDecl::Any;
            for b in &body {
                let ann = infer(b, decls, profile, scope, errors, mode, depth + 1);
                last_kind = ann.kind.clone();
                body_anns.push(AnnotatedChild::Node(ann));
            }

            if !binder.is_global {
                scope.pop();
            }

            let mut all_children = rhs_anns;
            all_children.extend(body_anns);
            make_annotated(node, last_kind, all_children)
        }

        BinderKind::Lambda { params, body } => {
            // Push scope, bind each param with its declared kind (or Any).
            let arity = params.len();
            scope.push();
            for (name, kind) in &params {
                scope.bind(name.clone(), kind.clone());
            }

            // Walk body.
            let mut body_anns = Vec::with_capacity(body.len());
            let mut last_kind = KindDecl::Any;
            for b in &body {
                let ann = infer(b, decls, profile, scope, errors, mode, depth + 1);
                last_kind = ann.kind.clone();
                body_anns.push(AnnotatedChild::Node(ann));
            }
            scope.pop();

            // The binder as a whole produces a Function value if anonymous
            // (lambda), or the last body kind if global (define-fn already
            // registered in globals).
            let result_kind = if binder.is_global {
                // For top-level define-fn the global is already registered;
                // the define form itself doesn't "produce" a value in the
                // program — its result is used for the body's side effects.
                last_kind
            } else {
                KindDecl::Function { arity }
            };

            make_annotated(node, result_kind, body_anns)
        }
    }
}

// ---------------------------------------------------------------------------
// Exhaustiveness checker
// ---------------------------------------------------------------------------

fn check_exhaustiveness<P: LanguageProfile>(
    union_name: &str,
    variants: &[String],
    arms: &[crate::profile::MatchArmInfo<'_>],
    profile: &P,
    node: &GrammarASTNode,
    errors: &mut Vec<TypeErrorDiagnostic>,
) {
    // A Wildcard or Binding arm makes the match trivially exhaustive.
    for arm in arms {
        match &arm.pattern {
            ArmPattern::Wildcard | ArmPattern::Binding(_) => return,
            _ => {}
        }
    }

    // Collect which variants are covered by Variant patterns.
    let covered: HashSet<&str> = arms
        .iter()
        .filter_map(|a| {
            if let ArmPattern::Variant { name, .. } = &a.pattern {
                Some(name.as_str())
            } else {
                None
            }
        })
        .collect();

    let missing: Vec<&str> = variants
        .iter()
        .filter(|v| !covered.contains(v.as_str()))
        .map(|v| v.as_str())
        .collect();

    if !missing.is_empty() {
        let (line, col) = profile.position(node);
        errors.push(TypeErrorDiagnostic {
            message: format!(
                "non-exhaustive match on union '{}': unmatched variants: '{}'",
                union_name,
                missing.join("', '")
            ),
            line,
            column: col,
        });
    }
}

// ---------------------------------------------------------------------------
// Annotation construction helpers
// ---------------------------------------------------------------------------

/// Build an [`AnnotatedNode`] from a raw grammar node + inferred kind.
fn make_annotated(
    node: &GrammarASTNode,
    kind: KindDecl,
    children: Vec<AnnotatedChild>,
) -> AnnotatedNode {
    AnnotatedNode {
        rule_name: node.rule_name.clone(),
        kind,
        children,
        start_line: node.start_line,
        start_column: node.start_column,
        end_line: node.end_line,
        end_column: node.end_column,
    }
}

/// Convert raw token children of `node` into [`AnnotatedChild::Token`]
/// entries.  Used for leaf nodes where no recursion is needed.
fn token_children(node: &GrammarASTNode) -> Vec<AnnotatedChild> {
    node.children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Token(t) => Some(AnnotatedChild::Token {
                text: t.value.clone(),
                line: t.line,
                column: t.column,
            }),
            ASTNodeOrToken::Node(_) => None,
        })
        .collect()
}
