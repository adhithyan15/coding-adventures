//! # `TwigLanguageProfile` — Twig grammar tree navigation for the generic checker
//!
//! This module implements [`grammar_type_checker::LanguageProfile`] for the Twig
//! grammar, bridging the gap between raw `GrammarASTNode` rule names (produced by
//! `twig-parser`'s generic `GrammarParser`) and the semantic roles the
//! `grammar-type-checker` needs to understand.
//!
//! ## Twig grammar → semantic role mapping
//!
//! | Grammar rule       | Semantic role               | Profile method        |
//! |--------------------|-----------------------------|-----------------------|
//! | `atom` + INTEGER   | integer literal             | `literal_kind`        |
//! | `atom` + BOOL_TRUE | boolean literal             | `literal_kind`        |
//! | `atom` + BOOL_FALSE| boolean literal             | `literal_kind`        |
//! | `atom` + KEYWORD(nil) | nil literal              | `literal_kind`        |
//! | `quoted`           | symbol literal              | `literal_kind`        |
//! | `quote_form`       | symbol literal (long form)  | `literal_kind`        |
//! | `atom` + NAME      | variable reference          | `as_var_ref`          |
//! | `apply`            | function application        | `as_apply`            |
//! | `lambda_form`      | anonymous function          | `as_binder`           |
//! | `define` (fn form) | named function definition   | `as_binder`           |
//! | `let_form`         | local let bindings          | `as_binder`           |
//! | `match_form`       | pattern match               | `as_match`            |
//! | `begin_form`       | sequencing                  | `as_begin`            |
//! | `if_form`          | conditional (fallback)      | `child_exprs`         |
//! | `expr` / `compound`| transparent wrapper         | `child_exprs`         |
//! | `form`             | transparent top-level wrap  | `child_exprs`         |
//!
//! ## Wrapper transparency
//!
//! The Twig grammar has several "wrapper" rules:
//! - `expr = atom | quoted | compound`
//! - `compound = if_form | let_form | …`
//! - `form = define | type_alias | … | expr`
//!
//! These rules produce intermediate `GrammarASTNode` nodes that contain no
//! semantic content themselves.  Each profile method calls [`unwrap_expr`] to
//! transparently descend through these wrappers before dispatching on rule names.
//! This means `as_var_ref` works correctly when called on an `"expr"` node that
//! wraps an `"atom"` — the inferred kind propagates back to the wrapper.

use grammar_type_checker::{
    AppInfo, ArmPattern, BinderInfo, BinderKind, LanguageProfile, MatchArmInfo, MatchInfo,
};
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use type_declarations::KindDecl;

// ---------------------------------------------------------------------------
// TwigLanguageProfile
// ---------------------------------------------------------------------------

/// Twig-specific implementation of [`LanguageProfile`].
///
/// Construct one and pass it to [`grammar_type_checker::check`]:
///
/// ```no_run
/// use twig_type_checker::profile::TwigLanguageProfile;
/// use twig_parser::{parse_to_ast, emit_type_declarations, parse};
///
/// let raw = parse_to_ast("(define x 42)").unwrap();
/// let program = parse("(define x 42)").unwrap();
/// let decls = emit_type_declarations(&program);
/// let result = grammar_type_checker::check(&raw, &decls, &TwigLanguageProfile);
/// assert!(result.ok);
/// ```
pub struct TwigLanguageProfile;

impl TwigLanguageProfile {
    // ── Internal helper: descend through transparent wrapper nodes. ────────
    //
    // The Twig grammar nests semantic nodes inside wrappers:
    //   expr → compound → apply
    //   expr → atom
    //   form → expr → ...
    //
    // `unwrap_expr` follows the first child node through any chain of
    // `"expr"`, `"compound"`, or `"form"` wrappers until it reaches a node
    // with a semantic rule name (e.g. `"atom"`, `"apply"`, `"lambda_form"`).
    //
    // Lifetime: the returned reference borrows from the input tree so no
    // copies are needed.
    fn unwrap_expr<'a>(&self, node: &'a GrammarASTNode) -> &'a GrammarASTNode {
        match node.rule_name.as_str() {
            "expr" | "compound" | "form" => {
                // Find the first child GrammarASTNode and recurse.
                if let Some(child) = node.children.iter().find_map(|c| match c {
                    ASTNodeOrToken::Node(n) => Some(n),
                    ASTNodeOrToken::Token(_) => None,
                }) {
                    self.unwrap_expr(child)
                } else {
                    node // no child node — stay at current level
                }
            }
            _ => node,
        }
    }
}

impl LanguageProfile for TwigLanguageProfile {
    // ── 1. Literal detection ──────────────────────────────────────────────
    //
    // Recognises integer, boolean, nil, and symbol literals from the `atom`
    // and `quoted` grammar rules.  The profile transparently descends through
    // `expr`/`compound` wrappers first.

    fn literal_kind(&self, node: &GrammarASTNode) -> Option<KindDecl> {
        let n = self.unwrap_expr(node);
        match n.rule_name.as_str() {
            "atom" => {
                // atom = INTEGER | BOOL_TRUE | BOOL_FALSE | "nil" | NAME
                // Literals are anything except NAME.
                for child in &n.children {
                    if let ASTNodeOrToken::Token(t) = child {
                        return match t.effective_type_name() {
                            "INTEGER" => Some(KindDecl::Int),
                            "BOOL_TRUE" | "BOOL_FALSE" => Some(KindDecl::Bool),
                            // "nil" is a KEYWORD token with value "nil".
                            "KEYWORD" if t.value == "nil" => Some(KindDecl::Nil),
                            // NAME token → variable reference, not a literal.
                            _ => None,
                        };
                    }
                }
                None
            }
            // quoted = QUOTE NAME  — always a symbol literal.
            "quoted" | "quote_form" => Some(KindDecl::Symbol),
            _ => None,
        }
    }

    // ── 2. Variable reference ─────────────────────────────────────────────
    //
    // Recognises `atom` nodes that carry a NAME token.  The first NAME token
    // in an atom is the variable name (there is exactly one).

    fn as_var_ref<'a>(&self, node: &'a GrammarASTNode) -> Option<&'a str> {
        let n = self.unwrap_expr(node);
        if n.rule_name != "atom" {
            return None;
        }
        // Return the value of the NAME token, if present.
        n.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Token(t) if t.effective_type_name() == "NAME" => {
                Some(t.value.as_str())
            }
            _ => None,
        })
    }

    // ── 3. Function application ───────────────────────────────────────────
    //
    // `apply = LPAREN expr { expr } RPAREN`
    // The first `expr` child is the callee; the rest are the arguments.

    fn as_apply<'a>(&self, node: &'a GrammarASTNode) -> Option<AppInfo<'a>> {
        let n = self.unwrap_expr(node);
        if n.rule_name != "apply" {
            return None;
        }
        let exprs: Vec<&GrammarASTNode> = n
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(child) if child.rule_name == "expr" => Some(child),
                _ => None,
            })
            .collect();
        if exprs.is_empty() {
            return None;
        }
        Some(AppInfo {
            callee: exprs[0],
            args: exprs[1..].to_vec(),
        })
    }

    // ── 4. Binder (let / lambda / define-fn) ──────────────────────────────
    //
    // Three binder forms exist in Twig:
    //
    // a) `lambda_form = LPAREN "lambda" LPAREN { NAME } RPAREN expr { expr } RPAREN`
    //    → BinderKind::Lambda with params from the NAME tokens, body from expr children.
    //
    // b) `let_form = LPAREN "let" LPAREN { binding } RPAREN expr { expr } RPAREN`
    //    → BinderKind::Let with one (name, expr) pair per `binding` child.
    //
    // c) `define` with function-sugar: `LPAREN "define" LPAREN NAME { NAME } RPAREN expr+ RPAREN`
    //    → BinderKind::Lambda (is_global: true) — the function name is already
    //    in scope from TypeDeclarations; we only need to put parameters in scope
    //    while checking the body.

    fn as_binder<'a>(&self, node: &'a GrammarASTNode) -> Option<BinderInfo<'a>> {
        let n = self.unwrap_expr(node);
        match n.rule_name.as_str() {
            // ── lambda_form ───────────────────────────────────────────────
            "lambda_form" => {
                // Parameters are all NAME tokens that appear before the first
                // `expr` child node.  In the grammar, they sit between the
                // inner LPAREN/RPAREN pair, so we collect all NAME tokens from
                // the direct children (the grammar ensures they precede `expr`).
                let params: Vec<(String, KindDecl)> = n
                    .children
                    .iter()
                    .filter_map(|c| match c {
                        ASTNodeOrToken::Token(t) if t.effective_type_name() == "NAME" => {
                            Some((t.value.clone(), KindDecl::Any))
                        }
                        _ => None,
                    })
                    .collect();

                let body: Vec<&GrammarASTNode> = n
                    .children
                    .iter()
                    .filter_map(|c| match c {
                        ASTNodeOrToken::Node(child) if child.rule_name == "expr" => Some(child),
                        _ => None,
                    })
                    .collect();

                if body.is_empty() {
                    return None;
                }
                Some(BinderInfo {
                    is_global: false,
                    kind: BinderKind::Lambda { params, body },
                })
            }

            // ── let_form ──────────────────────────────────────────────────
            "let_form" => {
                // binding = LPAREN NAME expr RPAREN
                // Each `binding` child contributes (name, rhs_expr).
                // `expr` children at the `let_form` level are the body.
                let mut bindings: Vec<(String, &GrammarASTNode)> = Vec::new();
                let mut body: Vec<&GrammarASTNode> = Vec::new();

                for child in &n.children {
                    match child {
                        ASTNodeOrToken::Node(child_node) => {
                            if child_node.rule_name == "binding" {
                                // Extract the variable name and the RHS expression.
                                let name_opt =
                                    child_node.children.iter().find_map(|c| match c {
                                        ASTNodeOrToken::Token(t)
                                            if t.effective_type_name() == "NAME" =>
                                        {
                                            Some(t.value.clone())
                                        }
                                        _ => None,
                                    });
                                let rhs_opt = child_node.children.iter().find_map(|c| match c {
                                    ASTNodeOrToken::Node(e) if e.rule_name == "expr" => {
                                        Some(e as &GrammarASTNode)
                                    }
                                    _ => None,
                                });
                                if let (Some(name), Some(rhs)) = (name_opt, rhs_opt) {
                                    bindings.push((name, rhs));
                                }
                            } else if child_node.rule_name == "expr" {
                                body.push(child_node);
                            }
                        }
                        ASTNodeOrToken::Token(_) => {}
                    }
                }

                if body.is_empty() {
                    return None;
                }
                Some(BinderInfo {
                    is_global: false,
                    kind: BinderKind::Let { bindings, body },
                })
            }

            // ── define (function-sugar form) ──────────────────────────────
            //
            // `define = LPAREN "define" name_or_signature expr { expr } RPAREN`
            // `name_or_signature = LPAREN NAME { typed_param } ... RPAREN`
            //
            // Only the function-sugar form (sig has LPAREN) is a binder;
            // value-defines (sig has no LPAREN) just have a body expr and the
            // global is already registered in TypeDeclarations.
            "define" => {
                let sig_node =
                    n.children.iter().find_map(|c| match c {
                        ASTNodeOrToken::Node(n) if n.rule_name == "name_or_signature" => Some(n),
                        _ => None,
                    })?;

                // Function-sugar: sig has a LPAREN among its direct children.
                let is_fn_sugar = sig_node
                    .children
                    .iter()
                    .any(|c| matches!(c, ASTNodeOrToken::Token(t) if t.effective_type_name() == "LPAREN"));

                if !is_fn_sugar {
                    // Value-define: no new scope to push.  The global is already
                    // registered; let child_exprs handle the body expression.
                    return None;
                }

                // Collect parameter names.  The signature structure is:
                //   LPAREN  NAME(fn)  { NAME(param) | typed_param }  RPAREN
                // The first NAME is the function name (already in globals);
                // subsequent NAMEs (and typed_param children) are parameters.
                let mut first_name_seen = false;
                let mut params: Vec<(String, KindDecl)> = Vec::new();
                for child in &sig_node.children {
                    match child {
                        ASTNodeOrToken::Token(t) if t.effective_type_name() == "NAME" => {
                            if !first_name_seen {
                                first_name_seen = true; // function name — skip
                            } else {
                                params.push((t.value.clone(), KindDecl::Any));
                            }
                        }
                        ASTNodeOrToken::Node(tp_node) if tp_node.rule_name == "typed_param" => {
                            // typed_param = LPAREN NAME COLON type_annotation RPAREN | NAME
                            if let Some(t) = tp_node.children.iter().find_map(|c| match c {
                                ASTNodeOrToken::Token(t) if t.effective_type_name() == "NAME" => {
                                    Some(t)
                                }
                                _ => None,
                            }) {
                                params.push((t.value.clone(), KindDecl::Any));
                            }
                        }
                        _ => {}
                    }
                }

                // Body expressions are the `expr` children of the `define` node.
                let body: Vec<&GrammarASTNode> = n
                    .children
                    .iter()
                    .filter_map(|c| match c {
                        ASTNodeOrToken::Node(child) if child.rule_name == "expr" => Some(child),
                        _ => None,
                    })
                    .collect();

                if body.is_empty() {
                    return None;
                }

                Some(BinderInfo {
                    is_global: true, // function name already in globals scope
                    kind: BinderKind::Lambda { params, body },
                })
            }

            _ => None,
        }
    }

    // ── 5. Match expression ───────────────────────────────────────────────
    //
    // `match_form = LPAREN "match" expr { match_arm } RPAREN`
    // `match_arm  = LPAREN match_pat expr { expr } RPAREN`
    // `match_pat  = LPAREN NAME { NAME } RPAREN | NAME`

    fn as_match<'a>(&self, node: &'a GrammarASTNode) -> Option<MatchInfo<'a>> {
        let n = self.unwrap_expr(node);
        if n.rule_name != "match_form" {
            return None;
        }

        // First `expr` child is the scrutinee.
        let scrutinee = n.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Node(child) if child.rule_name == "expr" => {
                Some(child as &GrammarASTNode)
            }
            _ => None,
        })?;

        // All `match_arm` children are the arms.
        let arms: Vec<MatchArmInfo<'_>> = n
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(arm_node) if arm_node.rule_name == "match_arm" => {
                    extract_match_arm_info(arm_node)
                }
                _ => None,
            })
            .collect();

        Some(MatchInfo { scrutinee, arms })
    }

    // ── 6. Begin / sequence ───────────────────────────────────────────────
    //
    // `begin_form = LPAREN "begin" expr { expr } RPAREN`
    // Returns the `expr` children.  The begin form's value is the last expr.

    fn as_begin<'a>(&self, node: &'a GrammarASTNode) -> Option<Vec<&'a GrammarASTNode>> {
        let n = self.unwrap_expr(node);
        if n.rule_name != "begin_form" {
            return None;
        }
        let exprs: Vec<&GrammarASTNode> = n
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(child) if child.rule_name == "expr" => Some(child),
                _ => None,
            })
            .collect();
        if exprs.is_empty() {
            None
        } else {
            Some(exprs)
        }
    }

    // ── 7. Child expressions (fallback) ───────────────────────────────────
    //
    // For wrapper and structural nodes, returns the children that contain
    // expressions worth recursing into.

    fn child_exprs<'a>(&self, node: &'a GrammarASTNode) -> Vec<&'a GrammarASTNode> {
        match node.rule_name.as_str() {
            // Transparent single-child wrappers → return the child.
            "expr" | "compound" | "form" => node
                .children
                .iter()
                .filter_map(|c| match c {
                    ASTNodeOrToken::Node(n) => Some(n),
                    _ => None,
                })
                .take(1)
                .collect(),

            // `if_form = LPAREN "if" expr expr expr RPAREN`
            // → Three expression children (condition, then, else).
            "if_form" => node
                .children
                .iter()
                .filter_map(|c| match c {
                    ASTNodeOrToken::Node(child) if child.rule_name == "expr" => Some(child),
                    _ => None,
                })
                .collect(),

            // `define` (value form, i.e. not function-sugar) — walk the body expr.
            // For the function-sugar form, `as_binder` takes over; this fallback
            // handles `(define name expr)` where `as_binder` returns None.
            "define" => node
                .children
                .iter()
                .filter_map(|c| match c {
                    ASTNodeOrToken::Node(child) if child.rule_name == "expr" => Some(child),
                    _ => None,
                })
                .collect(),

            // `module_form`, `type_alias`, `record_def`, `union_def` —
            // no expression children to check.
            "module_form" | "type_alias" | "record_def" | "union_def" => vec![],

            // Generic fallback: return all child nodes.
            _ => node
                .children
                .iter()
                .filter_map(|c| match c {
                    ASTNodeOrToken::Node(n) => Some(n),
                    _ => None,
                })
                .collect(),
        }
    }

    // ── Position helper ───────────────────────────────────────────────────

    fn position(&self, node: &GrammarASTNode) -> (usize, usize) {
        (
            node.start_line.unwrap_or(1),
            node.start_column.unwrap_or(1),
        )
    }
}

// ---------------------------------------------------------------------------
// Helper: extract match arm info from a `match_arm` node
// ---------------------------------------------------------------------------

/// Extract a [`MatchArmInfo`] from a `match_arm` grammar node.
///
/// Grammar: `match_arm = LPAREN match_pat expr { expr } RPAREN`
///
/// The pattern is the single `match_pat` child; the body is the first `expr`
/// child (multi-expression arms use the first `expr` as the body node — the
/// checker recurses into it, and if it is a `begin_form` it gets the full
/// sequence).
fn extract_match_arm_info(arm_node: &GrammarASTNode) -> Option<MatchArmInfo<'_>> {
    // Pattern: first `match_pat` child.
    let pat_node = arm_node.children.iter().find_map(|c| match c {
        ASTNodeOrToken::Node(n) if n.rule_name == "match_pat" => Some(n),
        _ => None,
    })?;

    // Body: first `expr` child of the arm.
    // (If there are multiple exprs they form an implicit begin; we take the
    // first and let the profile's begin/fallback handling cover the rest.
    // In practice twig grammars produce a single expr per arm body unless
    // the user explicitly uses begin — the extractor already combines them.)
    let body = arm_node.children.iter().find_map(|c| match c {
        ASTNodeOrToken::Node(e) if e.rule_name == "expr" => Some(e as &GrammarASTNode),
        _ => None,
    })?;

    let pattern = extract_arm_pattern(pat_node);

    Some(MatchArmInfo { pattern, body })
}

/// Convert a `match_pat` node to an [`ArmPattern`].
///
/// Grammar: `match_pat = LPAREN NAME { NAME } RPAREN | NAME`
fn extract_arm_pattern(pat_node: &GrammarASTNode) -> ArmPattern {
    let has_paren = pat_node.children.iter().any(|c| match c {
        ASTNodeOrToken::Token(t) => t.effective_type_name() == "LPAREN",
        _ => false,
    });

    let name_tokens: Vec<&str> = pat_node
        .children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Token(t) if t.effective_type_name() == "NAME" => {
                Some(t.value.as_str())
            }
            _ => None,
        })
        .collect();

    if has_paren {
        // Variant pattern: (VariantName b1 b2 ...)
        match name_tokens.as_slice() {
            [] => ArmPattern::Wildcard,
            [variant, bindings @ ..] => ArmPattern::Variant {
                name: (*variant).to_owned(),
                bindings: bindings.iter().map(|s: &&str| s.to_string()).collect(),
            },
        }
    } else {
        // Bare NAME: wildcard `_` or binding `varname`.
        match name_tokens.first().copied() {
            Some("_") => ArmPattern::Wildcard,
            Some(n) => ArmPattern::Binding((*n).to_owned()),
            None => ArmPattern::Wildcard,
        }
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use twig_parser::parse_to_ast;
    use type_declarations::{KindDecl, TypeDeclarations, TypedModeDecl};

    fn decls_strict() -> TypeDeclarations {
        let mut d = TypeDeclarations::new("twig");
        d.typed_mode = Some(TypedModeDecl::Strict);
        d
    }

    // ── Literal detection ─────────────────────────────────────────────────

    #[test]
    fn integer_atom_is_int_literal() {
        let raw = parse_to_ast("42").unwrap();
        let decls = decls_strict();
        let result = grammar_type_checker::check(&raw, &decls, &TwigLanguageProfile);
        // "program" has a "form" child whose last annotated child is Int.
        // Verify top-level result is ok (no strict errors on well-typed source).
        assert!(result.ok);
    }

    #[test]
    fn bool_literal_kind() {
        let raw = parse_to_ast("#t").unwrap();
        let result = grammar_type_checker::check(&raw, &decls_strict(), &TwigLanguageProfile);
        assert!(result.ok);
    }

    #[test]
    fn nil_literal_kind() {
        let raw = parse_to_ast("nil").unwrap();
        let result = grammar_type_checker::check(&raw, &decls_strict(), &TwigLanguageProfile);
        assert!(result.ok);
    }

    #[test]
    fn quoted_symbol_kind() {
        let raw = parse_to_ast("'foo").unwrap();
        let result = grammar_type_checker::check(&raw, &decls_strict(), &TwigLanguageProfile);
        assert!(result.ok);
    }

    // ── Variable reference ────────────────────────────────────────────────

    #[test]
    fn known_var_ref_no_error() {
        let raw = parse_to_ast("x").unwrap();
        let mut decls = decls_strict();
        decls.globals.insert("x".to_owned(), KindDecl::Int);
        let result = grammar_type_checker::check(&raw, &decls, &TwigLanguageProfile);
        assert!(result.ok);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn unknown_var_ref_emits_error_strict() {
        let raw = parse_to_ast("undefined_var").unwrap();
        let result = grammar_type_checker::check(&raw, &decls_strict(), &TwigLanguageProfile);
        // strict mode → ok:false because of the unresolved variable error
        assert!(!result.ok);
        assert!(!result.errors.is_empty());
    }

    // ── Apply ─────────────────────────────────────────────────────────────

    #[test]
    fn apply_correct_arity() {
        let raw = parse_to_ast("(f 1 2)").unwrap();
        let mut decls = decls_strict();
        decls.globals.insert("f".to_owned(), KindDecl::Function { arity: 2 });
        let result = grammar_type_checker::check(&raw, &decls, &TwigLanguageProfile);
        assert!(result.ok);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn apply_wrong_arity_emits_error() {
        let raw = parse_to_ast("(f 1 2 3)").unwrap();
        let mut decls = decls_strict();
        decls.globals.insert("f".to_owned(), KindDecl::Function { arity: 2 });
        let result = grammar_type_checker::check(&raw, &decls, &TwigLanguageProfile);
        assert!(!result.ok);
        assert!(!result.errors.is_empty());
        assert!(result.errors[0].message.contains("arity"));
    }

    // ── Lambda binder ─────────────────────────────────────────────────────

    #[test]
    fn lambda_params_in_scope() {
        // The body references the parameter `x`; no unresolved-variable error.
        let raw = parse_to_ast("(lambda (x) x)").unwrap();
        let result = grammar_type_checker::check(&raw, &decls_strict(), &TwigLanguageProfile);
        assert!(result.ok);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn lambda_with_no_params() {
        let raw = parse_to_ast("(lambda () 42)").unwrap();
        let result = grammar_type_checker::check(&raw, &decls_strict(), &TwigLanguageProfile);
        assert!(result.ok);
    }

    // ── Let binder ────────────────────────────────────────────────────────

    #[test]
    fn let_binding_in_scope() {
        let raw = parse_to_ast("(let ((x 1)) x)").unwrap();
        let result = grammar_type_checker::check(&raw, &decls_strict(), &TwigLanguageProfile);
        assert!(result.ok);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn let_binding_not_visible_in_sibling_rhs() {
        // Scheme `let`: (let ((x 1) (y x)) y) — `x` is not in scope for `y`'s RHS.
        let raw = parse_to_ast("(let ((x 1) (y x)) y)").unwrap();
        let result = grammar_type_checker::check(&raw, &decls_strict(), &TwigLanguageProfile);
        // `x` is unresolved in the RHS of `y` — strict mode emits an error.
        assert!(!result.ok);
        assert!(!result.errors.is_empty());
    }

    // ── Define (function-sugar) ───────────────────────────────────────────

    #[test]
    fn define_fn_params_in_scope() {
        // `(define (double x) (+ x x))` — `x` and `+` must be in scope.
        let raw = parse_to_ast("(define (double x) (+ x x))").unwrap();
        let mut decls = decls_strict();
        // `double` is registered as a global by emit_type_declarations.
        decls.globals.insert("double".to_owned(), KindDecl::Function { arity: 1 });
        decls.globals.insert("+".to_owned(), KindDecl::Function { arity: 2 });
        let result = grammar_type_checker::check(&raw, &decls, &TwigLanguageProfile);
        assert!(result.ok);
        assert!(result.errors.is_empty());
    }

    // ── Begin ─────────────────────────────────────────────────────────────

    #[test]
    fn begin_form_checked() {
        let raw = parse_to_ast("(begin 1 2 3)").unwrap();
        let result = grammar_type_checker::check(&raw, &decls_strict(), &TwigLanguageProfile);
        assert!(result.ok);
    }

    // ── Off mode — no errors even on unresolved var ───────────────────────

    #[test]
    fn off_mode_no_errors() {
        let raw = parse_to_ast("undefined_var").unwrap();
        let decls = TypeDeclarations::new("twig"); // no mode → Off
        let result = grammar_type_checker::check(&raw, &decls, &TwigLanguageProfile);
        assert!(result.ok);
        assert!(result.errors.is_empty());
    }
}
