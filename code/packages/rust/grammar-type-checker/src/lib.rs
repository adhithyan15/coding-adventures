//! # `grammar-type-checker`
//!
//! Generic type checker operating on raw [`GrammarASTNode`] trees from the
//! `parser` crate.  Type rules come from [`TypeDeclarations`] (emitted by any
//! language's parser); language-specific tree navigation is provided by a
//! [`LanguageProfile`] implementation.
//!
//! ## Why generic?
//!
//! The LANG49 `twig-type-checker` hardcoded knowledge of Twig's typed AST
//! (`Program`, `Expr`, `Form`).  Any new language would need its own checker.
//! This crate factors out the common algorithm — scope tracking, kind
//! inference, arity and exhaustiveness checking — so languages only need to
//! supply a [`LanguageProfile`] that maps their grammar rule names to the
//! semantic roles the algorithm understands.
//!
//! ## Types are not ornamental
//!
//! The checker returns [`TypeCheckResult<AnnotatedNode>`] rather than just
//! `ok/errors`.  The [`AnnotatedNode`] tree carries a [`KindDecl`] on every
//! node.  Downstream, `twig-ir-compiler::compile_annotated()` reads
//! `annotated.iir_hint()` to populate `type_hint` fields on IIR instructions
//! with concrete values (`"i64"`, `"bool"`, `"closure"`) instead of `"any"`.
//!
//! The JIT and AOT specialisers in this codebase prioritise `type_hint` over
//! runtime profiles, so fully-typed IIR reaches native-code quality with zero
//! warmup cost.
//!
//! ## Annotation is always-on
//!
//! Even in [`TypedModeDecl::Off`] the annotated tree is built — kinds are the
//! best available inference (`Any` where nothing better is known).  Annotation
//! always-on, enforcement mode-gated.
//!
//! ## Quick start
//!
//! ```rust,ignore
//! use grammar_type_checker::{check, LanguageProfile};
//! use type_declarations::{TypeDeclarations, TypedModeDecl};
//!
//! // MyProfile implements LanguageProfile for your grammar
//! let decls = TypeDeclarations::new("my-lang");
//! let result = check(&grammar_ast_root, &decls, &MyProfile);
//! assert!(result.ok);
//! let annotated_root = result.typed_ast;
//! // annotated_root.iir_hint() → "any" for the program root
//! // annotated_root.node_children() → annotated forms
//! ```

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use type_checker_protocol::{TypeCheckResult, TypeErrorDiagnostic};
use type_declarations::{
    AnnotatedChild, AnnotatedNode, KindDecl, TypeDeclarations, TypedModeDecl,
};

pub mod check;
pub mod profile;
pub mod scope;

pub use profile::{
    AppInfo, ArmPattern, BinderInfo, BinderKind, LanguageProfile, MatchArmInfo, MatchInfo,
};
pub use scope::ScopeStack;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Type-check a raw grammar AST using parser-emitted [`TypeDeclarations`] and
/// a language-specific [`LanguageProfile`].
///
/// # Returns
///
/// A [`TypeCheckResult`] where:
/// - `typed_ast` is the fully-annotated tree (kind on every node).
/// - `errors` holds any type violations found.
/// - `ok` is `false` only in [`TypedModeDecl::Strict`] when there are errors.
///
/// **Annotation is always built**, even in `Off` mode.  The annotations feed
/// compilation (IIR `type_hint` fields) regardless of enforcement.
///
/// # Example
///
/// ```rust,ignore
/// let result = grammar_type_checker::check(&raw_ast, &decls, &TwigLanguageProfile);
/// // result.typed_ast.iir_hint() → "any" (program root has no single value kind)
/// // result.errors.is_empty() on well-typed code in strict mode
/// ```
pub fn check<P: LanguageProfile>(
    root: &GrammarASTNode,
    decls: &TypeDeclarations,
    profile: &P,
) -> TypeCheckResult<AnnotatedNode> {
    let mode = decls
        .typed_mode
        .clone()
        .unwrap_or(TypedModeDecl::Off);

    // Seed the scope with all top-level globals so forward references work.
    let mut scope = ScopeStack::new();
    for (name, kind) in &decls.globals {
        scope.bind(name.clone(), kind.clone());
    }

    // Walk every form-level child of the program root.
    let mut errors: Vec<TypeErrorDiagnostic> = Vec::new();
    let mut annotated_children: Vec<AnnotatedChild> = Vec::new();

    for child in &root.children {
        if let ASTNodeOrToken::Node(n) = child {
            let ann =
                check::infer(n, decls, profile, &mut scope, &mut errors, &mode, 0);
            annotated_children.push(AnnotatedChild::Node(ann));
        } else if let ASTNodeOrToken::Token(t) = child {
            annotated_children.push(AnnotatedChild::Token {
                text: t.value.clone(),
                line: t.line,
                column: t.column,
            });
        }
    }

    // Build the annotated program root.
    let annotated_root = AnnotatedNode {
        rule_name: root.rule_name.clone(),
        kind: KindDecl::Any, // program root has no single value kind
        children: annotated_children,
        start_line: root.start_line,
        start_column: root.start_column,
        end_line: root.end_line,
        end_column: root.end_column,
    };

    let ok = match mode {
        TypedModeDecl::Strict => errors.is_empty(),
        _ => true,
    };

    TypeCheckResult {
        typed_ast: annotated_root,
        errors,
        ok,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
    use type_declarations::{
        KindDecl, NamedTypeDecl, TypeDeclarations, TypedModeDecl, VariantDecl,
    };

    // ── Test helpers ─────────────────────────────────────────────────────

    /// Minimal language profile for tests — recognises a handful of fictional
    /// rule names so we can exercise the checker without a real language.
    struct TestProfile;

    impl LanguageProfile for TestProfile {
        fn literal_kind(&self, node: &GrammarASTNode) -> Option<KindDecl> {
            match node.rule_name.as_str() {
                "int_lit" => Some(KindDecl::Int),
                "bool_lit" => Some(KindDecl::Bool),
                "nil_lit" => Some(KindDecl::Nil),
                "sym_lit" => Some(KindDecl::Symbol),
                _ => None,
            }
        }

        fn as_var_ref<'a>(&self, node: &'a GrammarASTNode) -> Option<&'a str> {
            if node.rule_name != "var_ref" {
                return None;
            }
            // First token child = variable name
            node.children.iter().find_map(|c| match c {
                ASTNodeOrToken::Token(t) => Some(t.value.as_str()),
                _ => None,
            })
        }

        fn as_apply<'a>(&self, node: &'a GrammarASTNode) -> Option<AppInfo<'a>> {
            if node.rule_name != "apply" {
                return None;
            }
            let expr_children: Vec<&GrammarASTNode> = node
                .children
                .iter()
                .filter_map(|c| match c {
                    ASTNodeOrToken::Node(n) => Some(n),
                    _ => None,
                })
                .collect();
            if expr_children.is_empty() {
                return None;
            }
            Some(AppInfo {
                callee: expr_children[0],
                args: expr_children[1..].to_vec(),
            })
        }

        fn as_binder<'a>(&self, node: &'a GrammarASTNode) -> Option<BinderInfo<'a>> {
            if node.rule_name != "let_expr" {
                return None;
            }
            // Fictional structure: [binding_pair*, body]
            let children: Vec<&GrammarASTNode> = node
                .children
                .iter()
                .filter_map(|c| match c {
                    ASTNodeOrToken::Node(n) => Some(n),
                    _ => None,
                })
                .collect();
            if children.len() < 2 {
                return None;
            }
            // Last child = body, everything before = bindings (name=rule_name)
            let body_node = children[children.len() - 1];
            let bindings: Vec<(String, &GrammarASTNode)> = children[..children.len() - 1]
                .iter()
                .map(|n| (n.rule_name.clone(), *n))
                .collect();
            Some(BinderInfo {
                is_global: false,
                kind: BinderKind::Let {
                    bindings,
                    body: vec![body_node],
                },
            })
        }

        fn as_match<'a>(&self, node: &'a GrammarASTNode) -> Option<MatchInfo<'a>> {
            if node.rule_name != "match_expr" {
                return None;
            }
            let children: Vec<&GrammarASTNode> = node
                .children
                .iter()
                .filter_map(|c| match c {
                    ASTNodeOrToken::Node(n) => Some(n),
                    _ => None,
                })
                .collect();
            if children.is_empty() {
                return None;
            }
            let scrutinee = children[0];
            let arms: Vec<MatchArmInfo<'_>> = children[1..]
                .iter()
                .map(|arm_node| {
                    let pat = if arm_node.rule_name == "_" {
                        ArmPattern::Wildcard
                    } else {
                        ArmPattern::Variant {
                            name: arm_node.rule_name.clone(),
                            bindings: vec![],
                        }
                    };
                    MatchArmInfo {
                        pattern: pat,
                        body: arm_node,
                    }
                })
                .collect();
            Some(MatchInfo { scrutinee, arms })
        }

        fn as_begin<'a>(&self, node: &'a GrammarASTNode) -> Option<Vec<&'a GrammarASTNode>> {
            if node.rule_name != "begin" {
                return None;
            }
            Some(
                node.children
                    .iter()
                    .filter_map(|c| match c {
                        ASTNodeOrToken::Node(n) => Some(n),
                        _ => None,
                    })
                    .collect(),
            )
        }

        fn child_exprs<'a>(&self, node: &'a GrammarASTNode) -> Vec<&'a GrammarASTNode> {
            node.children
                .iter()
                .filter_map(|c| match c {
                    ASTNodeOrToken::Node(n) => Some(n),
                    _ => None,
                })
                .collect()
        }
    }

    /// Build a minimal `GrammarASTNode` for a given rule name with no children.
    fn leaf(rule: &str) -> GrammarASTNode {
        GrammarASTNode {
            rule_name: rule.to_owned(),
            children: vec![],
            start_line: Some(1),
            start_column: Some(1),
            end_line: Some(1),
            end_column: Some(1),
        }
    }

    /// Build a `GrammarASTNode` whose only child is a token with the given text.
    fn token_node(rule: &str, token_text: &str) -> GrammarASTNode {
        use lexer::token::{Token, TokenType};
        let tok = Token {
            type_: TokenType::Name,
            value: token_text.to_owned(),
            line: 1,
            column: 1,
            type_name: None,
            flags: None,
        };
        GrammarASTNode {
            rule_name: rule.to_owned(),
            children: vec![ASTNodeOrToken::Token(tok)],
            start_line: Some(1),
            start_column: Some(1),
            end_line: Some(1),
            end_column: Some(1),
        }
    }

    /// Build a `GrammarASTNode` with node children.
    fn parent(rule: &str, children: Vec<GrammarASTNode>) -> GrammarASTNode {
        GrammarASTNode {
            rule_name: rule.to_owned(),
            children: children
                .into_iter()
                .map(ASTNodeOrToken::Node)
                .collect(),
            start_line: Some(1),
            start_column: Some(1),
            end_line: Some(1),
            end_column: Some(1),
        }
    }

    /// Wrap a single form child in a program root.
    fn program_of(form: GrammarASTNode) -> GrammarASTNode {
        parent("program", vec![form])
    }

    // ── Literal kind tests ────────────────────────────────────────────────

    #[test]
    fn generic_int_lit_kind() {
        let mut d = TypeDeclarations::new("test");
        d.typed_mode = Some(TypedModeDecl::Strict);
        let root = program_of(leaf("int_lit"));
        let result = check(&root, &d, &TestProfile);
        assert!(result.ok);
        // The single child annotation should have kind Int
        let form_ann = result.typed_ast.node_children();
        assert_eq!(form_ann[0].kind, KindDecl::Int);
    }

    #[test]
    fn generic_bool_lit_kind() {
        let mut d = TypeDeclarations::new("test");
        d.typed_mode = Some(TypedModeDecl::Strict);
        let root = program_of(leaf("bool_lit"));
        let result = check(&root, &d, &TestProfile);
        assert_eq!(result.typed_ast.node_children()[0].kind, KindDecl::Bool);
    }

    #[test]
    fn generic_nil_lit_kind() {
        let mut d = TypeDeclarations::new("test");
        d.typed_mode = Some(TypedModeDecl::Strict);
        let root = program_of(leaf("nil_lit"));
        let result = check(&root, &d, &TestProfile);
        assert_eq!(result.typed_ast.node_children()[0].kind, KindDecl::Nil);
    }

    // ── Variable reference tests ──────────────────────────────────────────

    #[test]
    fn generic_var_ref_resolved() {
        let mut d = TypeDeclarations::new("test");
        d.typed_mode = Some(TypedModeDecl::Strict);
        d.globals
            .insert("f".to_owned(), KindDecl::Function { arity: 1 });
        let root = program_of(token_node("var_ref", "f"));
        let result = check(&root, &d, &TestProfile);
        assert!(result.ok);
        assert!(result.errors.is_empty());
        assert_eq!(
            result.typed_ast.node_children()[0].kind,
            KindDecl::Function { arity: 1 }
        );
    }

    #[test]
    fn generic_var_ref_unresolved() {
        let mut d = TypeDeclarations::new("test");
        d.typed_mode = Some(TypedModeDecl::Strict);
        let root = program_of(token_node("var_ref", "unknown_name"));
        let result = check(&root, &d, &TestProfile);
        assert!(!result.ok);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].message.contains("unresolved variable"));
        assert!(result.errors[0].message.contains("unknown_name"));
    }

    #[test]
    fn generic_var_ref_unresolved_lenient_ok_true() {
        let mut d = TypeDeclarations::new("test");
        d.typed_mode = Some(TypedModeDecl::Lenient);
        let root = program_of(token_node("var_ref", "missing"));
        let result = check(&root, &d, &TestProfile);
        assert!(result.ok); // lenient: ok even with errors
        assert!(!result.errors.is_empty());
    }

    // ── Apply arity tests ─────────────────────────────────────────────────

    #[test]
    fn generic_apply_arity_correct() {
        let mut d = TypeDeclarations::new("test");
        d.typed_mode = Some(TypedModeDecl::Strict);
        d.globals
            .insert("f".to_owned(), KindDecl::Function { arity: 1 });
        // (apply (var_ref "f") (int_lit))
        let apply = parent("apply", vec![token_node("var_ref", "f"), leaf("int_lit")]);
        let root = program_of(apply);
        let result = check(&root, &d, &TestProfile);
        assert!(result.ok);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn generic_apply_arity_wrong() {
        let mut d = TypeDeclarations::new("test");
        d.typed_mode = Some(TypedModeDecl::Strict);
        d.globals
            .insert("f".to_owned(), KindDecl::Function { arity: 1 });
        // (apply (var_ref "f") (int_lit) (int_lit)) — too many args
        let apply = parent(
            "apply",
            vec![
                token_node("var_ref", "f"),
                leaf("int_lit"),
                leaf("int_lit"),
            ],
        );
        let root = program_of(apply);
        let result = check(&root, &d, &TestProfile);
        assert!(!result.ok);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].message.contains("arity error"));
    }

    // ── Typed mode tests ──────────────────────────────────────────────────

    #[test]
    fn generic_typed_off_no_errors() {
        // Off mode: even unresolved vars produce no errors
        let mut d = TypeDeclarations::new("test");
        d.typed_mode = Some(TypedModeDecl::Off);
        let root = program_of(token_node("var_ref", "does_not_exist"));
        let result = check(&root, &d, &TestProfile);
        assert!(result.ok);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn generic_typed_off_still_annotates() {
        // Annotation is always-on — even Off mode produces AnnotatedNode
        let mut d = TypeDeclarations::new("test");
        d.typed_mode = Some(TypedModeDecl::Off);
        let root = program_of(leaf("int_lit"));
        let result = check(&root, &d, &TestProfile);
        // Kind is inferred even in Off mode
        assert_eq!(result.typed_ast.node_children()[0].kind, KindDecl::Int);
    }

    #[test]
    fn generic_typed_strict_fails_on_error() {
        let mut d = TypeDeclarations::new("test");
        d.typed_mode = Some(TypedModeDecl::Strict);
        let root = program_of(token_node("var_ref", "not_defined"));
        let result = check(&root, &d, &TestProfile);
        assert!(!result.ok);
    }

    // ── AnnotatedNode propagation ─────────────────────────────────────────

    #[test]
    fn annotated_node_carries_kind() {
        let mut d = TypeDeclarations::new("test");
        d.typed_mode = Some(TypedModeDecl::Strict);
        let root = program_of(leaf("bool_lit"));
        let result = check(&root, &d, &TestProfile);
        let child = &result.typed_ast.node_children()[0];
        assert_eq!(child.kind, KindDecl::Bool);
        assert_eq!(child.iir_hint(), "bool");
    }

    #[test]
    fn annotated_children_propagate() {
        // A program with two literal forms — both should be annotated
        let mut d = TypeDeclarations::new("test");
        d.typed_mode = Some(TypedModeDecl::Strict);
        let root = parent(
            "program",
            vec![leaf("int_lit"), leaf("bool_lit")],
        );
        let result = check(&root, &d, &TestProfile);
        let children = result.typed_ast.node_children();
        assert_eq!(children[0].kind, KindDecl::Int);
        assert_eq!(children[1].kind, KindDecl::Bool);
    }

    // ── Match exhaustiveness ──────────────────────────────────────────────

    #[test]
    fn generic_match_non_exhaustive() {
        let mut d = TypeDeclarations::new("test");
        d.typed_mode = Some(TypedModeDecl::Strict);
        d.globals
            .insert("v".to_owned(), KindDecl::Named("Color".to_owned()));
        d.named_types.insert(
            "Color".to_owned(),
            NamedTypeDecl::Union {
                variants: vec![
                    VariantDecl { name: "Red".to_owned(), fields: vec![] },
                    VariantDecl { name: "Blue".to_owned(), fields: vec![] },
                ],
            },
        );

        // match_expr: scrutinee=var_ref("v"), one arm for "Red" only
        let scrutinee = token_node("var_ref", "v");
        // arm for Red only — Blue missing
        let red_arm = parent("Red", vec![leaf("int_lit")]);
        let match_node = parent("match_expr", vec![scrutinee, red_arm]);
        let root = program_of(match_node);

        let result = check(&root, &d, &TestProfile);
        assert!(!result.ok);
        assert!(result.errors[0].message.contains("non-exhaustive"));
        assert!(result.errors[0].message.contains("Blue"));
    }

    #[test]
    fn generic_match_exhaustive_all_variants() {
        let mut d = TypeDeclarations::new("test");
        d.typed_mode = Some(TypedModeDecl::Strict);
        d.globals
            .insert("v".to_owned(), KindDecl::Named("Coin".to_owned()));
        d.named_types.insert(
            "Coin".to_owned(),
            NamedTypeDecl::Union {
                variants: vec![
                    VariantDecl { name: "Heads".to_owned(), fields: vec![] },
                    VariantDecl { name: "Tails".to_owned(), fields: vec![] },
                ],
            },
        );

        let scrutinee = token_node("var_ref", "v");
        let heads_arm = parent("Heads", vec![leaf("int_lit")]);
        let tails_arm = parent("Tails", vec![leaf("int_lit")]);
        let match_node = parent("match_expr", vec![scrutinee, heads_arm, tails_arm]);
        let root = program_of(match_node);

        let result = check(&root, &d, &TestProfile);
        assert!(result.ok);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn generic_match_exhaustive_wildcard() {
        let mut d = TypeDeclarations::new("test");
        d.typed_mode = Some(TypedModeDecl::Strict);
        d.globals
            .insert("v".to_owned(), KindDecl::Named("Coin".to_owned()));
        d.named_types.insert(
            "Coin".to_owned(),
            NamedTypeDecl::Union {
                variants: vec![
                    VariantDecl { name: "Heads".to_owned(), fields: vec![] },
                    VariantDecl { name: "Tails".to_owned(), fields: vec![] },
                ],
            },
        );

        let scrutinee = token_node("var_ref", "v");
        // "_" is our fictional wildcard rule name
        let wildcard_arm = parent("_", vec![leaf("nil_lit")]);
        let match_node = parent("match_expr", vec![scrutinee, wildcard_arm]);
        let root = program_of(match_node);

        let result = check(&root, &d, &TestProfile);
        assert!(result.ok);
        assert!(result.errors.is_empty());
    }

    // ── KindDecl alias resolution ─────────────────────────────────────────

    #[test]
    fn kind_decl_resolve_alias_in_globals() {
        let mut d = TypeDeclarations::new("test");
        d.typed_mode = Some(TypedModeDecl::Strict);
        // Nat is an alias for Int
        d.named_types.insert(
            "Nat".to_owned(),
            NamedTypeDecl::Alias { target: KindDecl::Int },
        );
        d.globals
            .insert("n".to_owned(), KindDecl::Named("Nat".to_owned()));

        // Looking up "n" gives Named("Nat"), resolving gives Int
        let resolved = d.resolve(&KindDecl::Named("Nat".to_owned()));
        assert_eq!(resolved, KindDecl::Int);
    }

    // ── TypeDeclarations utilities ────────────────────────────────────────

    #[test]
    fn type_declarations_union_variants_lookup() {
        use type_declarations::VariantDecl;
        let mut d = TypeDeclarations::new("test");
        d.named_types.insert(
            "Shape".to_owned(),
            NamedTypeDecl::Union {
                variants: vec![
                    VariantDecl { name: "Circle".to_owned(), fields: vec![] },
                    VariantDecl { name: "Rect".to_owned(), fields: vec![] },
                ],
            },
        );
        let vs = d.union_variants("Shape").unwrap();
        assert_eq!(vs, vec!["Circle", "Rect"]);
    }

    #[test]
    fn type_declarations_empty_no_panic() {
        let d = TypeDeclarations::new("test");
        let root = parent("program", vec![]);
        let result = check(&root, &d, &TestProfile);
        assert!(result.ok);
        assert!(result.errors.is_empty());
    }
}
