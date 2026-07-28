//! # Reading a parsed `type_expr` node into a generic [`TypeSpec`]
//!
//! `axiom-parser`'s `type_expr = NAME [ type_ctor_args ]` (MA-13c) gives
//! `axiom-runtime` a domain/category-shaped `GrammarASTNode` wherever MA13
//! §4's declaration (`:`), coercion (`::`), category-query (`has`), and
//! function-header type-annotation positions need one. This module's only
//! job is reading that generic tree shape into a plain `{ name, args }`
//! [`TypeSpec`] — it knows nothing about *which* names are valid built-in
//! domains/categories (that fixed table lives in `crate::domains`, per this
//! crate's own file-layout separation: this file is the "which built-in
//! names exist and how their surface AST reads into a spec" bridge,
//! `domains.rs` is "the fixed table those specs are checked against").
//!
//! ```text
//! type_expr = NAME [ type_ctor_args ]
//! type_ctor_args = LPAREN [ type_expr_list ] RPAREN
//!                | NAME                            -- paren-optional, ONE level
//! type_expr_list = type_expr { COMMA type_expr }
//! ```
//!
//! The paren-optional shorthand (`Fraction Integer`) is, per
//! `axiom.grammar`'s own comment, restricted to a single bare `NAME` — never
//! a further nested `type_expr` with its own arguments — so
//! [`parse_type_spec`] mirrors that restriction directly rather than
//! re-deriving it.

use crate::domains::TypeSpec;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};

/// Read a `type_expr` node into a [`TypeSpec`].
///
/// `node` may be the `type_expr` node itself, or any ancestor that has
/// exactly one `type_expr` descendant reachable by walking node children
/// (every call site in `crate::eval` hands this the direct `type_expr` child
/// it already found, so this is a thin, defensive convenience rather than a
/// general tree search).
pub fn parse_type_spec(node: &GrammarASTNode) -> TypeSpec {
    debug_assert_eq!(
        node.rule_name, "type_expr",
        "parse_type_spec expects a `type_expr` node, got `{}`",
        node.rule_name
    );

    let name = node
        .children
        .iter()
        .find_map(as_token)
        .map(|t| t.value.clone())
        .unwrap_or_default();

    let args = node
        .children
        .iter()
        .find_map(as_node)
        .filter(|n| n.rule_name == "type_ctor_args")
        .map(parse_type_ctor_args)
        .unwrap_or_default();

    TypeSpec { name, args }
}

/// `type_ctor_args = LPAREN [ type_expr_list ] RPAREN | NAME`.
fn parse_type_ctor_args(node: &GrammarASTNode) -> Vec<TypeSpec> {
    let has_lparen = node
        .children
        .iter()
        .any(|c| as_token(c).is_some_and(|t| t.effective_type_name() == "LPAREN"));

    if !has_lparen {
        // The bare-NAME paren-optional shorthand: a single NAME token, no
        // parens at all. `Fraction Integer`'s `Integer` becomes a single
        // zero-argument TypeSpec, mirroring the grammar's own restriction
        // that this shorthand never recurses into a further-parameterized
        // type.
        return node
            .children
            .iter()
            .find_map(as_token)
            .map(|t| {
                vec![TypeSpec {
                    name: t.value.clone(),
                    args: vec![],
                }]
            })
            .unwrap_or_default();
    }

    // The explicit-parens form: LPAREN [ type_expr_list ] RPAREN. An empty
    // `()` (no `type_expr_list` child at all) is zero arguments -- every
    // built-in constructor that takes parens also takes exactly one
    // argument, so `crate::domains::resolve_domain`/`resolve_category`
    // reject this by arity, not this function.
    node.children
        .iter()
        .find_map(as_node)
        .filter(|n| n.rule_name == "type_expr_list")
        .map(parse_type_expr_list)
        .unwrap_or_default()
}

/// `type_expr_list = type_expr { COMMA type_expr }`.
fn parse_type_expr_list(node: &GrammarASTNode) -> Vec<TypeSpec> {
    node.children
        .iter()
        .filter_map(as_node)
        .filter(|n| n.rule_name == "type_expr")
        .map(parse_type_spec)
        .collect()
}

fn as_node(child: &ASTNodeOrToken) -> Option<&GrammarASTNode> {
    match child {
        ASTNodeOrToken::Node(node) => Some(node),
        ASTNodeOrToken::Token(_) => None,
    }
}

fn as_token(child: &ASTNodeOrToken) -> Option<&lexer::token::Token> {
    match child {
        ASTNodeOrToken::Token(token) => Some(token),
        ASTNodeOrToken::Node(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_axiom_parser::parse_axiom;

    /// Parse `src` (a bare type-position expression, wrapped in a
    /// declaration so the grammar accepts it standalone) and return the
    /// `type_expr` node's [`TypeSpec`].
    fn spec_of(type_text: &str) -> TypeSpec {
        let src = format!("a : {type_text}");
        let ast = parse_axiom(&src);
        let type_expr = find_rule(&ast, "type_expr").expect("type_expr node");
        parse_type_spec(type_expr)
    }

    fn find_rule<'a>(node: &'a GrammarASTNode, name: &str) -> Option<&'a GrammarASTNode> {
        if node.rule_name == name {
            return Some(node);
        }
        for c in &node.children {
            if let ASTNodeOrToken::Node(n) = c {
                if let Some(found) = find_rule(n, name) {
                    return Some(found);
                }
            }
        }
        None
    }

    #[test]
    fn a_bare_name_has_no_args() {
        let spec = spec_of("Integer");
        assert_eq!(spec.name, "Integer");
        assert!(spec.args.is_empty());
    }

    #[test]
    fn explicit_parens_with_one_argument() {
        let spec = spec_of("Fraction(Integer)");
        assert_eq!(spec.name, "Fraction");
        assert_eq!(spec.args, vec![TypeSpec { name: "Integer".to_string(), args: vec![] }]);
    }

    #[test]
    fn paren_optional_shorthand_with_one_argument() {
        let spec = spec_of("Fraction Integer");
        assert_eq!(spec.name, "Fraction");
        assert_eq!(spec.args, vec![TypeSpec { name: "Integer".to_string(), args: vec![] }]);
    }

    #[test]
    fn deeply_nested_explicit_constructors_parse() {
        let spec = spec_of("List(Matrix(Polynomial(Integer)))");
        assert_eq!(spec.name, "List");
        assert_eq!(spec.args.len(), 1);
        assert_eq!(spec.args[0].name, "Matrix");
        assert_eq!(spec.args[0].args[0].name, "Polynomial");
        assert_eq!(spec.args[0].args[0].args[0].name, "Integer");
    }

    #[test]
    fn multiple_explicit_arguments_parse_in_order() {
        // Not a real built-in constructor shape, but the grammar's
        // `type_expr_list` accepts any comma-separated run -- exercised here
        // purely to confirm this module reads ALL of them, in order, not
        // just the first. `declared_define` requires a return-type
        // annotation (axiom-parser's own `declared_define_requires_a_return_type_annotation`
        // test), so this uses `Integer` as a throwaway return type.
        let src = "f(x: Fictional(Integer, Float)): Integer == x";
        let ast = parse_axiom(src);
        let type_expr = find_rule(&ast, "type_expr").expect("type_expr node");
        let spec = parse_type_spec(type_expr);
        assert_eq!(spec.name, "Fictional");
        assert_eq!(spec.args.len(), 2);
        assert_eq!(spec.args[0].name, "Integer");
        assert_eq!(spec.args[1].name, "Float");
    }

    #[test]
    fn zero_argument_explicit_parens() {
        let src = "f(x: Weird()): Integer == x";
        let ast = parse_axiom(src);
        let type_expr = find_rule(&ast, "type_expr").expect("type_expr node");
        let spec = parse_type_spec(type_expr);
        assert_eq!(spec.name, "Weird");
        assert!(spec.args.is_empty());
    }
}
